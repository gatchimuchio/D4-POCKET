//! Linux native秘密入力を既存Owner receiverへ渡す。Dart／通常IPCへ秘密を出さない。
use crate::broker::ipc_server::DesktopOwnerOperationRequest;
use crate::broker::protocol::{canonical_payload_hash, request_issued_at_is_current};
use crate::broker::{BrokerEndpoint, BrokerRequestEnvelope};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    io,
    sync::mpsc::{self, SyncSender},
    time::Duration,
};
use zeroize::{Zeroize, Zeroizing};

pub(super) const INPUT_REPLY_LIMIT: usize = 32_900;
const REQUEST_LIMIT: usize = 8192;
const SUMMARY_LIMIT: usize = 16 * 1024;

pub(super) trait LinuxCredentialUi {
    fn enter_secret(
        &mut self,
        challenge: &str,
        summary: &str,
    ) -> io::Result<Option<Zeroizing<String>>>;
    fn confirm_owner(&mut self, summary: &str) -> io::Result<bool>;
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    client: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    #[serde(rename = "版")]
    version: u8,
    #[serde(rename = "資格情報ID")]
    id: String,
    #[serde(rename = "用途")]
    purpose: String,
    #[serde(rename = "接続対象")]
    target: String,
    #[serde(rename = "種類")]
    kind: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicRequest {
    request_id: String,
    operation: String,
    payload_hash: String,
    nonce: String,
    issued_at: String,
    metadata: Metadata,
    payload: Payload,
}

fn valid_text(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

pub(super) fn valid_credential_input(bytes: &[u8]) -> bool {
    if bytes.is_empty()
        || bytes.len() > REQUEST_LIMIT
        || bytes.contains(&b'\n')
        || bytes.contains(&b'\r')
    {
        return false;
    }
    let Ok(request) = serde_json::from_slice::<PublicRequest>(bytes) else {
        return false;
    };
    let id = &request.payload.id;
    request.operation == "資格情報登録"
        && request.metadata.client == "desktop_flutter"
        && valid_text(&request.request_id, 256)
        && valid_text(&request.nonce, 256)
        && request.issued_at.len() == 20
        && request.issued_at.ends_with('Z')
        && request.payload_hash.len() == 71
        && request.payload_hash.starts_with("sha256:")
        && request.payload_hash[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && request.payload.version == 1
        && id.len() == 32
        && id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && matches!(
            request.payload.purpose.as_str(),
            "provider_api_key" | "mcp_transport"
        )
        && valid_text(&request.payload.target, 256)
        && request.payload.kind == "api_key"
}

pub(super) fn input_prompt(challenge: &str, summary: &str) -> Option<String> {
    if challenge.len() != 32
        || !challenge
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || summary.is_empty()
        || summary.len() > SUMMARY_LIMIT
        || summary.chars().any(|character| {
            character == '\0' || (character.is_control() && character != '\n' && character != '\t')
        })
    {
        return None;
    }
    let frame = format!("D4CRED1 {challenge} {}\n", hex::encode(summary));
    (frame.len() <= INPUT_REPLY_LIMIT).then_some(frame)
}

pub(super) fn input_reply_secret(reply: &[u8], challenge: &str) -> Option<Zeroizing<String>> {
    if reply.len() > INPUT_REPLY_LIMIT {
        return None;
    }
    let reply = std::str::from_utf8(reply).ok()?;
    let mut fields = reply.split(' ');
    if fields.next()? != "D4CRED1" || fields.next()? != challenge || fields.next()? != "ENTERED" {
        return None;
    }
    let encoded = fields.next()?;
    if fields.next().is_some()
        || encoded.is_empty()
        || encoded.len() > 32_768
        || encoded.len() % 2 != 0
    {
        return None;
    }
    if !encoded
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    let mut bytes = Zeroizing::new(hex::decode(encoded).ok()?);
    if bytes.is_empty() || bytes.len() > 16_384 {
        return None;
    }
    let secret = String::from_utf8(bytes.to_vec()).ok()?;
    bytes.zeroize();
    if secret.chars().any(char::is_control) {
        let mut secret = Zeroizing::new(secret);
        secret.zeroize();
        return None;
    }
    Some(Zeroizing::new(secret))
}

pub(super) fn dispatch_native(
    frame: &[u8],
    endpoint: &BrokerEndpoint,
    sender: &SyncSender<DesktopOwnerOperationRequest>,
    seen: &mut BTreeSet<String>,
    ui: &mut impl LinuxCredentialUi,
) -> io::Result<Option<Vec<u8>>> {
    if !valid_credential_input(frame) {
        return Ok(None);
    }
    let envelope = BrokerRequestEnvelope::from_json_str(
        std::str::from_utf8(frame).map_err(|_| input_error())?,
    )
    .map_err(|_| input_error())?;
    let hash = canonical_payload_hash(envelope.payload.as_ref());
    if envelope.session_id.is_some()
        || envelope.payload_hash.as_deref() != Some(&hash)
        || !envelope
            .issued_at
            .as_deref()
            .is_some_and(request_issued_at_is_current)
        || seen.len() >= 64
        || !seen.insert(envelope.nonce.clone().ok_or_else(input_error)?)
    {
        return Ok(None);
    }
    let payload = envelope.payload.as_ref().ok_or_else(input_error)?;
    let id = payload["資格情報ID"].as_str().ok_or_else(input_error)?;
    let purpose = payload["用途"].as_str().ok_or_else(input_error)?;
    let target = payload["接続対象"].as_str().ok_or_else(input_error)?;
    let mut challenge_bytes = [0_u8; 16];
    getrandom::getrandom(&mut challenge_bytes).map_err(|_| input_error())?;
    let challenge = hex::encode(challenge_bytes);
    let input_summary = format!(
        "D4 Pocketへ資格情報を入力します。\nID: {id}\n用途: {purpose}\n接続対象: {target}\n\n入力値はFlutter・Audit・通常応答へ送られず、続けて別個のOwner確認を行います。"
    );
    let mut secret = match ui.enter_secret(&challenge, &input_summary)? {
        Some(secret) => secret,
        None => return relay_denied(frame, endpoint),
    };
    if !envelope
        .issued_at
        .as_deref()
        .is_some_and(request_issued_at_is_current)
    {
        secret.zeroize();
        return relay_denied(frame, endpoint);
    }
    let owner_summary = format!(
        "資格情報をこのユーザーのLinux Secret Service default collectionへ登録します。\nID: {id}\n用途: {purpose}\n接続対象: {target}\n種類: api_key\n公開要求hash: {hash}\n\n秘密値は表示しません。Permission・Approval・Trust・接続を付与しません。登録内容はBrokerが現在状態とAuditを再評価します。"
    );
    if !ui.confirm_owner(&owner_summary)?
        || !envelope
            .issued_at
            .as_deref()
            .is_some_and(request_issued_at_is_current)
    {
        secret.zeroize();
        return relay_denied(frame, endpoint);
    }

    let mut request: Value = serde_json::from_slice(frame).map_err(|_| input_error())?;
    request["payload"] = json!({
        "版": 1,
        "操作": "追加",
        "資格情報ID": id,
        "用途": purpose,
        "接続対象": target,
        "種類": "api_key",
        "保管方式": "linux_secret_service",
        "登録者種別": "owner",
        "登録経路": "owner_control",
        "秘密値": secret.as_str(),
    });
    request["session_id"] = json!(endpoint.session_id);
    request["payload_hash"] = json!(canonical_payload_hash(request.get("payload")));
    let encoded = Zeroizing::new(request.to_string());
    if let Some(Value::String(value)) = request["payload"].get_mut("秘密値") {
        value.zeroize();
    }
    secret.zeroize();
    let (reply, response) = mpsc::sync_channel(1);
    let request = DesktopOwnerOperationRequest {
        request_json: encoded.to_string(),
        download_confirmation: None,
        apply_confirmation: None,
        activation_confirmation: None,
        product_repair_confirmation: None,
        product_uninstall_confirmation: None,
        reply,
    };
    if let Err(error) = sender.try_send(request) {
        let mut request = match error {
            mpsc::TrySendError::Full(request) | mpsc::TrySendError::Disconnected(request) => {
                request
            }
        };
        request.request_json.zeroize();
        return Err(input_error());
    }
    let response = response
        .recv_timeout(Duration::from_secs(8))
        .map_err(|_| input_error())?;
    response
        .to_json_string()
        .map(|value| Some(value.into_bytes()))
        .map_err(|_| input_error())
}

fn relay_denied(frame: &[u8], endpoint: &BrokerEndpoint) -> io::Result<Option<Vec<u8>>> {
    super::macos_desktop_worker::relay(
        &super::macos_desktop_worker::normalize(frame, &endpoint.session_id),
        endpoint,
    )
    .map(Some)
}

fn input_error() -> io::Error {
    io::Error::other("Linux native資格情報入力を安全に完了できません")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::protocol::{BrokerResponse, BrokerStatus};
    use std::thread;

    fn public_request() -> Value {
        json!({
            "request_id": "linux-credential",
            "operation": "資格情報登録",
            "payload_hash": format!("sha256:{}", "a".repeat(64)),
            "nonce": "once",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"},
            "payload": {"版":1,"資格情報ID":"b".repeat(32),"用途":"mcp_transport","接続対象":"linux-server","種類":"api_key"}
        })
    }

    fn endpoint() -> BrokerEndpoint {
        BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 1,
            session_id: "linux-owner-session".into(),
            session_secret: "synthetic-session-secret".into(),
            credential_role: crate::broker::BrokerCredentialRole::Normal,
            transport: "authenticated_loopback_tcp".into(),
            max_request_bytes: REQUEST_LIMIT,
        }
    }

    fn request_bytes() -> Vec<u8> {
        let mut request = public_request();
        request["payload_hash"] = json!(canonical_payload_hash(request.get("payload")));
        serde_json::to_vec(&request).unwrap()
    }

    struct TestUi {
        secret: Option<Zeroizing<String>>,
        owner_confirmed: bool,
        input_summary: String,
        owner_summary: String,
    }

    impl LinuxCredentialUi for TestUi {
        fn enter_secret(
            &mut self,
            _challenge: &str,
            summary: &str,
        ) -> io::Result<Option<Zeroizing<String>>> {
            self.input_summary = summary.to_owned();
            Ok(self.secret.take())
        }

        fn confirm_owner(&mut self, summary: &str) -> io::Result<bool> {
            self.owner_summary = summary.to_owned();
            Ok(self.owner_confirmed)
        }
    }

    #[test]
    fn linux_credential_input_rejects_public_secret_authority_and_duplicate_fields() {
        let value = public_request();
        assert!(valid_credential_input(value.to_string().as_bytes()));
        for field in [
            "秘密値",
            "承認",
            "Permission",
            "保管方式",
            "登録者種別",
            "登録経路",
        ] {
            let mut bad = value.clone();
            bad["payload"][field] = json!("injected");
            assert!(!valid_credential_input(bad.to_string().as_bytes()));
        }
        let duplicate = value
            .to_string()
            .replacen("\"版\":1", "\"版\":1,\"版\":1", 1);
        assert!(!valid_credential_input(duplicate.as_bytes()));
        assert!(!valid_credential_input(&[b'x'; REQUEST_LIMIT + 1]));
    }

    #[test]
    fn linux_credential_input_prompt_and_reply_are_challenge_bound_and_bounded() {
        let challenge = "a".repeat(32);
        let prompt = input_prompt(&challenge, "公開metadataだけ").unwrap();
        assert!(prompt.starts_with(&format!("D4CRED1 {challenge} ")));
        let secret = b"synthetic-secret-not-operational";
        let reply = format!("D4CRED1 {challenge} ENTERED {}", hex::encode(secret));
        assert_eq!(
            input_reply_secret(reply.as_bytes(), &challenge)
                .unwrap()
                .as_bytes(),
            secret
        );
        assert!(input_reply_secret(reply.as_bytes(), &"b".repeat(32)).is_none());
        assert!(input_reply_secret(
            b"D4CRED1 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa CANCELLED",
            &challenge
        )
        .is_none());
        assert!(input_prompt(&challenge, &"x".repeat(SUMMARY_LIMIT + 1)).is_none());
    }

    #[test]
    fn linux_credential_input_requires_separate_owner_confirmation_and_cannot_replay() {
        let bytes = request_bytes();
        let secret_value = "synthetic-linux-secret-not-operational";
        let (sender, receiver) = mpsc::sync_channel::<DesktopOwnerOperationRequest>(1);
        let worker = thread::spawn(move || {
            let mut request = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
            let mut value: Value = serde_json::from_str(&request.request_json).unwrap();
            assert_eq!(value["session_id"], "linux-owner-session");
            assert_eq!(value["operation"], "資格情報登録");
            assert_eq!(value["payload"]["保管方式"], "linux_secret_service");
            assert_eq!(value["payload"]["秘密値"], secret_value);
            if let Some(Value::String(secret)) = value["payload"].get_mut("秘密値") {
                secret.zeroize();
            }
            request.request_json.zeroize();
            request
                .reply
                .send(BrokerResponse {
                    request_id: "linux-credential".into(),
                    operation: "資格情報登録".into(),
                    status: BrokerStatus::Accepted,
                    evidence_source: "INTERNAL_STATE".into(),
                    audit_event_id: "linux-credential-audit".into(),
                    error: None,
                    health: None,
                    body: Some(json!({
                        "保管方式":"linux_secret_service",
                        "公開範囲":"metadata_only"
                    })),
                    shutdown_requested: false,
                })
                .unwrap();
            assert!(receiver.try_recv().is_err());
        });
        let mut ui = TestUi {
            secret: Some(Zeroizing::new(secret_value.to_owned())),
            owner_confirmed: true,
            input_summary: String::new(),
            owner_summary: String::new(),
        };
        let mut seen = BTreeSet::new();
        let response = dispatch_native(&bytes, &endpoint(), &sender, &mut seen, &mut ui)
            .unwrap()
            .unwrap();
        assert!(!String::from_utf8_lossy(&response).contains(secret_value));
        assert!(ui.input_summary.contains("linux-server"));
        assert!(!ui.input_summary.contains(secret_value));
        assert!(ui.owner_summary.contains("Permission・Approval・Trust"));
        assert!(!ui.owner_summary.contains(secret_value));
        assert!(
            dispatch_native(&bytes, &endpoint(), &sender, &mut seen, &mut ui)
                .unwrap()
                .is_none()
        );
        worker.join().unwrap();
    }

    #[test]
    fn linux_credential_input_rejects_stale_request_before_showing_ui() {
        let mut request: Value = serde_json::from_slice(&request_bytes()).unwrap();
        request["issued_at"] = json!("2000-01-01T00:00:00Z");
        let payload = request["payload"].clone();
        request["payload_hash"] = json!(canonical_payload_hash(Some(&payload)));
        let bytes = serde_json::to_vec(&request).unwrap();
        let (sender, receiver) = mpsc::sync_channel::<DesktopOwnerOperationRequest>(1);
        let mut ui = TestUi {
            secret: Some(Zeroizing::new("synthetic-secret".into())),
            owner_confirmed: true,
            input_summary: String::new(),
            owner_summary: String::new(),
        };
        assert!(
            dispatch_native(&bytes, &endpoint(), &sender, &mut BTreeSet::new(), &mut ui)
                .unwrap()
                .is_none()
        );
        assert!(ui.input_summary.is_empty());
        assert!(receiver.try_recv().is_err());
    }
}
