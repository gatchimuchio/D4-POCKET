//! Linux GTKファイル選択画面の応答は選択内容だけを渡す入力であり、権限の根拠にはなりません。
use crate::broker::ipc_server::DesktopOwnerOperationRequest;
use crate::broker::protocol::{canonical_payload_hash, request_issued_at_is_current};
use crate::broker::{BrokerEndpoint, BrokerRequestEnvelope};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::time::Duration;

const OPERATION: &str = "作業領域OS選択";
const PICKER_REPLY_LIMIT: usize = 2 * 1024 + 128;

pub(super) enum NativeSelection {
    Selected(PathBuf),
    Cancelled,
    Failed,
}

pub(super) fn dispatch_native(
    frame: &[u8],
    endpoint: &BrokerEndpoint,
    sender: &SyncSender<DesktopOwnerOperationRequest>,
    seen: &mut BTreeSet<String>,
    choose: &mut impl FnMut(&str) -> io::Result<NativeSelection>,
) -> io::Result<Option<Vec<u8>>> {
    let Some(mut request) = candidate(frame, endpoint) else {
        return Ok(None);
    };
    let nonce = request["nonce"].as_str().unwrap().to_owned();
    if seen.len() >= 64 || !seen.insert(nonce) {
        return Ok(None);
    }

    let mut random = [0_u8; 16];
    getrandom::getrandom(&mut random).map_err(|_| failure())?;
    let challenge = hex::encode(random);
    let choice = choose(&challenge)?;
    let (selection_status, workspace_root) = match choice {
        NativeSelection::Selected(path) if valid_path(&path) => {
            ("selected", path.to_str().map(str::to_owned))
        }
        NativeSelection::Selected(_) => ("failed_path", None),
        NativeSelection::Cancelled => ("cancelled", None),
        NativeSelection::Failed => ("failed_native", None),
    };

    let selection_request_hash = request["payload_hash"].clone();
    request["session_id"] = json!(endpoint.session_id);
    request["payload"] = json!({
        "version": 1,
        "selection_status": selection_status,
        "workspace_root": workspace_root,
        "selection_request_hash": selection_request_hash,
    });
    request["payload_hash"] = json!(canonical_payload_hash(Some(&request["payload"])));

    let (reply, receiver) = mpsc::sync_channel(1);
    sender
        .try_send(DesktopOwnerOperationRequest {
            request_json: request.to_string(),
            download_confirmation: None,
            apply_confirmation: None,
            activation_confirmation: None,
            product_repair_confirmation: None,
            product_uninstall_confirmation: None,
            reply,
        })
        .map_err(|_| failure())?;
    let response = receiver
        .recv_timeout(Duration::from_secs(4))
        .map_err(|_| failure())?;
    Ok(Some(
        response
            .to_json_string()
            .map_err(|_| failure())?
            .into_bytes(),
    ))
}

pub(super) fn parse_picker_reply(challenge: &str, frame: &[u8]) -> io::Result<NativeSelection> {
    if !valid_challenge(challenge) || frame.len() > PICKER_REPLY_LIMIT {
        return Err(failure());
    }
    let text = std::str::from_utf8(frame).map_err(|_| failure())?;
    let prefix = format!("D4PICK1 {challenge} ");
    let response = text.strip_prefix(&prefix).ok_or_else(failure)?;
    if response == "CANCELLED" {
        return Ok(NativeSelection::Cancelled);
    }
    if response == "FAILED" {
        return Ok(NativeSelection::Failed);
    }
    let encoded = response.strip_prefix("SELECTED ").ok_or_else(failure)?;
    if encoded.is_empty() || encoded.len() > 2048 || encoded.len() % 2 != 0 {
        return Err(failure());
    }
    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    for pair in encoded.as_bytes().chunks_exact(2) {
        let high = lower_hex_nibble(pair[0]).ok_or_else(failure)?;
        let low = lower_hex_nibble(pair[1]).ok_or_else(failure)?;
        bytes.push((high << 4) | low);
    }
    if bytes.len() > 1024 {
        return Err(failure());
    }
    let path = String::from_utf8(bytes).map_err(|_| failure())?;
    let path = PathBuf::from(path);
    if !valid_path(&path) {
        return Err(failure());
    }
    Ok(NativeSelection::Selected(path))
}

fn candidate(frame: &[u8], endpoint: &BrokerEndpoint) -> Option<Value> {
    if frame.len() >= endpoint.max_request_bytes {
        return None;
    }
    let text = std::str::from_utf8(frame).ok()?;
    let envelope = BrokerRequestEnvelope::from_json_str(text).ok()?;
    if envelope.operation.as_ref().map(|value| value.as_str()) != Some(OPERATION)
        || envelope.session_id.is_some()
        || !envelope
            .request_id
            .as_deref()
            .is_some_and(|value| !value.is_empty() && value.len() <= 256)
        || !envelope
            .nonce
            .as_deref()
            .is_some_and(|value| !value.is_empty() && value.len() <= 256)
        || !envelope
            .issued_at
            .as_deref()
            .is_some_and(request_issued_at_is_current)
        || envelope.metadata.len() != 1
        || envelope.metadata[0].key != "client"
        || envelope.metadata[0].value != "desktop_flutter"
        || envelope.payload != Some(json!({"version": 1}))
        || envelope.payload_hash.as_deref()
            != Some(&canonical_payload_hash(envelope.payload.as_ref()))
    {
        return None;
    }
    serde_json::from_slice(frame).ok()
}

fn valid_path(path: &std::path::Path) -> bool {
    path.is_absolute()
        && path
            .to_str()
            .is_some_and(|value| value.len() <= 1024 && !value.chars().any(char::is_control))
}

fn valid_challenge(challenge: &str) -> bool {
    challenge.len() == 32
        && challenge
            .bytes()
            .all(|byte| lower_hex_nibble(byte).is_some())
}

fn lower_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn failure() -> io::Error {
    io::Error::other("Linux作業領域OS選択を確認できません")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::{Broker, BrokerCredentialRole, BrokerStatus};

    fn endpoint() -> BrokerEndpoint {
        BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 1,
            transport: "authenticated_loopback_tcp".into(),
            session_id: "linux-selection-test".into(),
            session_secret: "fixture-only".into(),
            credential_role: BrokerCredentialRole::Normal,
            max_request_bytes: 65536,
        }
    }

    fn request(nonce: &str) -> Value {
        let payload = json!({"version": 1});
        json!({
            "request_id": "linux-selection-request",
            "operation": OPERATION,
            "nonce": nonce,
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"},
            "payload": payload,
            "payload_hash": canonical_payload_hash(Some(&json!({"version": 1}))),
        })
    }

    #[test]
    fn linux_chooser_selection_and_cancel_are_audited_without_authority() {
        let endpoint = endpoint();
        let (sender, receiver) = mpsc::sync_channel::<DesktopOwnerOperationRequest>(1);
        let worker = std::thread::spawn(move || {
            let mut broker = Broker::new("linux-selection-test");
            broker.set_desktop_setup_doctor_runtime_evidence(true, false, true);
            let request = receiver.recv().unwrap();
            let response = broker.desktop_owner_operation_json(&request.request_json);
            request.reply.send(response).unwrap();
            serde_json::to_string(broker.audit_events()).unwrap()
        });
        let frame = request("linux-selection-once").to_string();
        let mut seen = BTreeSet::new();
        let mut calls = 0;
        let response =
            dispatch_native(frame.as_bytes(), &endpoint, &sender, &mut seen, &mut |_| {
                calls += 1;
                Ok(NativeSelection::Selected(PathBuf::from(
                    "/synthetic/linux-workspace",
                )))
            })
            .unwrap()
            .unwrap();
        let response: Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(response["status"], "accepted");
        assert_eq!(
            response["body"]["workspace_root"],
            "/synthetic/linux-workspace"
        );
        assert_eq!(response["body"]["scope_lifetime"], "none");
        for key in [
            "permission_generated",
            "approval_generated",
            "registration_generated",
        ] {
            assert_eq!(response["body"][key], false);
        }
        assert!(
            dispatch_native(frame.as_bytes(), &endpoint, &sender, &mut seen, &mut |_| {
                calls += 1;
                Ok(NativeSelection::Cancelled)
            },)
            .unwrap()
            .is_none()
        );
        assert_eq!(calls, 1);

        let cancel = request("linux-selection-cancel").to_string();
        let mut broker = Broker::new("linux-selection-test");
        broker.set_desktop_setup_doctor_runtime_evidence(true, false, true);
        let payload = json!({
            "version": 1,
            "selection_status": "cancelled",
            "workspace_root": null,
            "selection_request_hash": canonical_payload_hash(Some(&json!({"version": 1}))),
        });
        let mut cancel_value: Value = serde_json::from_str(&cancel).unwrap();
        cancel_value["session_id"] = json!(endpoint.session_id);
        cancel_value["payload"] = payload;
        cancel_value["payload_hash"] = json!(canonical_payload_hash(cancel_value.get("payload")));
        let response = broker.desktop_owner_operation_json(&cancel_value.to_string());
        assert_eq!(response.status, BrokerStatus::Accepted);
        assert_eq!(response.body.unwrap()["selection_status"], "cancelled");
        let audit = format!(
            "{}{}",
            worker.join().unwrap(),
            serde_json::to_string(broker.audit_events()).unwrap()
        );
        assert!(!audit.contains("/synthetic/linux-workspace"));
    }

    #[test]
    fn linux_picker_reply_is_challenge_bound_and_bounded() {
        let challenge = "0123456789abcdef0123456789abcdef";
        assert!(matches!(
            parse_picker_reply(challenge, b"D4PICK1 0123456789abcdef0123456789abcdef SELECTED 2f746d70").unwrap(),
            NativeSelection::Selected(path) if path == PathBuf::from("/tmp")
        ));
        assert!(matches!(
            parse_picker_reply(
                challenge,
                b"D4PICK1 0123456789abcdef0123456789abcdef CANCELLED"
            )
            .unwrap(),
            NativeSelection::Cancelled
        ));
        for invalid in [
            b"D4PICK1 ffffffffffffffffffffffffffffffff SELECTED 2f746d70".as_slice(),
            b"D4PICK1 0123456789abcdef0123456789abcdef SELECTED 2f6261640a70617468".as_slice(),
            b"D4PICK1 0123456789abcdef0123456789abcdef SELECTED ff".as_slice(),
        ] {
            assert!(parse_picker_reply(challenge, invalid).is_err());
        }
    }

    #[test]
    fn linux_selection_request_rejects_ui_path_and_authority_injection() {
        let endpoint = endpoint();
        let base = request("linux-selection-candidate");
        assert!(candidate(base.to_string().as_bytes(), &endpoint).is_some());
        for key in ["workspace_root", "permission", "approval", "credential"] {
            let mut injected = base.clone();
            injected["payload"][key] = json!("injected");
            injected["payload_hash"] = json!(canonical_payload_hash(injected.get("payload")));
            assert!(candidate(injected.to_string().as_bytes(), &endpoint).is_none());
        }
        let mut owner = base;
        owner["operation"] = json!("AgentTaskOwnerApprovalGrant");
        assert!(candidate(owner.to_string().as_bytes(), &endpoint).is_none());
    }

    #[test]
    fn linux_workspace_selection_source_is_private_and_cannot_grant_authority() {
        let mut broker = Broker::new("linux-selection-private-source");
        broker.set_desktop_setup_doctor_runtime_evidence(true, false, true);
        let payload = json!({
            "version": 1,
            "selection_status": "selected",
            "workspace_root": "/synthetic/private-workspace",
            "selection_request_hash": canonical_payload_hash(Some(&json!({"version": 1}))),
        });
        let request = json!({
            "request_id": "linux-private-source",
            "operation": OPERATION,
            "session_id": "linux-selection-private-source",
            "nonce": "linux-private-source-once",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"},
            "payload_hash": canonical_payload_hash(Some(&payload)),
            "payload": payload,
        });
        assert_eq!(
            broker.handle_json(&request.to_string()).status,
            BrokerStatus::Rejected
        );
        assert_eq!(
            broker.owner要求処理(&request.to_string()).status,
            BrokerStatus::Rejected
        );
        let selected = broker.desktop_owner_operation_json(&request.to_string());
        assert_eq!(selected.status, BrokerStatus::Accepted);
        let body = selected.body.unwrap();
        assert_eq!(body["scope_lifetime"], "none");
        for key in [
            "permission_generated",
            "approval_generated",
            "registration_generated",
        ] {
            assert_eq!(body[key], false);
        }
        assert_eq!(
            broker
                .desktop_owner_operation_json(&request.to_string())
                .status,
            BrokerStatus::Rejected
        );
        assert!(!serde_json::to_string(broker.audit_events())
            .unwrap()
            .contains("private-workspace"));
    }
}
