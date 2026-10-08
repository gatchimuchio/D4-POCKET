//! native秘密入力だけを既存Owner receiverへ結ぶ。公開要求は秘密・承認・保管先を持たない。
use crate::broker::ipc_server::DesktopOwnerOperationRequest;
use crate::broker::protocol::{canonical_payload_hash, request_issued_at_is_current};
use crate::broker::{BrokerEndpoint, BrokerRequestEnvelope};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    io,
    sync::mpsc::{self, SyncSender},
    time::Duration,
};
use zeroize::{Zeroize, Zeroizing};

pub(super) fn dispatch_native(
    frame: &[u8],
    endpoint: &BrokerEndpoint,
    sender: &SyncSender<DesktopOwnerOperationRequest>,
    seen: &mut BTreeSet<String>,
    confirm: &mut impl FnMut(&str) -> bool,
) -> io::Result<Option<Vec<u8>>> {
    // raw Valueをparseし秘密の余分なcopyを作らず、private識別子だけで分岐する。
    if !frame.starts_with(b"{\"native_credential_input\":") {
        return Ok(None);
    }
    let private: gui_shell_macos_owner::CredentialPrivateFrame =
        serde_json::from_slice(frame).map_err(|_| io::Error::other("資格情報private形状が不正"))?;
    let input = &private.native_credential_input;
    let raw = input.request_json.as_bytes();
    if !gui_shell_macos_owner::valid_credential_input(raw) {
        return Err(io::Error::other("資格情報の公開要求が不正"));
    }
    let envelope = BrokerRequestEnvelope::from_json_str(&input.request_json)
        .map_err(|_| io::Error::other("資格情報の元要求が不正"))?;
    if envelope.session_id.is_some()
        || envelope.payload_hash.as_deref()
            != Some(&canonical_payload_hash(envelope.payload.as_ref()))
        || !envelope
            .issued_at
            .as_deref()
            .is_some_and(request_issued_at_is_current)
        || seen.len() >= 64
        || !seen.insert(
            envelope
                .nonce
                .clone()
                .ok_or_else(|| io::Error::other("nonceがない"))?,
        )
    {
        return Err(io::Error::other("資格情報の要求期限または再送が不正"));
    }
    let mut request: Value =
        serde_json::from_slice(raw).map_err(|_| io::Error::other("資格情報要求が不正"))?;
    let payload = request["payload"].clone();
    match (input.state.as_str(), input.secret.as_deref()) {
        ("cancelled" | "invalid", None) => return denied(raw, endpoint),
        ("entered", Some(secret))
            if !secret.is_empty()
                && secret.len() <= 16384
                && !secret.chars().any(char::is_control) => {}
        _ => return Err(io::Error::other("資格情報のnative入力状態が不正")),
    }
    let summary = format!("資格情報をこのBrokerのKeychainへ登録します。\nID: {}\n用途: {}\n接続対象: {}\n種類: api_key\n公開要求hash: {}\n秘密値は表示しません。Permission・Approval・接続を付与しません。",
        payload["資格情報ID"].as_str().unwrap(), payload["用途"].as_str().unwrap(), payload["接続対象"].as_str().unwrap(), envelope.payload_hash.as_deref().unwrap());
    if !confirm(&summary) {
        return denied(raw, endpoint);
    }
    // 入力とOwner確認で期限を消費しても古い要求を更新して再承認しない。
    if !envelope
        .issued_at
        .as_deref()
        .is_some_and(request_issued_at_is_current)
    {
        return denied(raw, endpoint);
    }
    request["payload"] = json!({"版":1,"操作":"追加", "資格情報ID":payload["資格情報ID"], "用途":payload["用途"],"接続対象":payload["接続対象"], "種類":"api_key", "保管方式":"macos_keychain", "登録者種別":"owner", "登録経路":"owner_control", "秘密値":input.secret.as_deref().unwrap()});
    request["session_id"] = json!(endpoint.session_id);
    request["payload_hash"] = json!(canonical_payload_hash(request.get("payload")));
    let encoded = Zeroizing::new(request.to_string());
    if let Some(Value::String(s)) = request["payload"].get_mut("秘密値") {
        s.zeroize();
    }
    let (reply, response) = mpsc::sync_channel(1);
    sender
        .try_send(DesktopOwnerOperationRequest {
            request_json: encoded.to_string(),
            download_confirmation: None,
            apply_confirmation: None,
            activation_confirmation: None,
            product_repair_confirmation: None,
            product_uninstall_confirmation: None,
            reply,
        })
        .map_err(|_| io::Error::other("資格情報Owner配送が未成立"))?;
    let response = response
        .recv_timeout(Duration::from_secs(8))
        .map_err(|_| io::Error::other("資格情報結果が未確定。自動再送しない"))?;
    response
        .to_json_string()
        .map(|v| Some(v.into_bytes()))
        .map_err(|_| io::Error::other("資格情報応答が不正"))
}
fn denied(raw: &[u8], endpoint: &BrokerEndpoint) -> io::Result<Option<Vec<u8>>> {
    // 秘密なしの公開要求を通常Brokerへ返してowner_required拒否とAuditを得る。
    super::macos_desktop_worker::relay(
        &super::macos_desktop_worker::normalize(raw, &endpoint.session_id),
        endpoint,
    )
    .map(Some)
}
