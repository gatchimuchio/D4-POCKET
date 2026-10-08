//! macOSの限定Owner入口。UIデータを承認へ昇格せず、既存Brokerへ現在要求を渡す。
use crate::broker::ipc_server::DesktopOwnerOperationRequest;
use crate::broker::protocol::{canonical_payload_hash, request_issued_at_is_current};
use crate::broker::{adapter_center, BrokerEndpoint, BrokerRequestEnvelope};
use std::sync::mpsc::{self, SyncSender};
use std::time::Duration;

struct Candidate {
    request: String,
    summary: String,
    response_timeout: Duration,
}

fn candidate(frame: &[u8], endpoint: &BrokerEndpoint) -> Option<Candidate> {
    if frame.len() >= endpoint.max_request_bytes {
        return None;
    }
    let raw = std::str::from_utf8(frame).ok()?;
    let envelope = BrokerRequestEnvelope::from_json_str(raw).ok()?;
    if envelope.session_id.is_some()
        || envelope.request_id.as_deref().is_none_or(str::is_empty)
        || envelope.nonce.as_deref().is_none_or(str::is_empty)
        || !envelope
            .issued_at
            .as_deref()
            .is_some_and(request_issued_at_is_current)
        || envelope.metadata.len() != 1
        || envelope.metadata[0].key != "client"
        || envelope.metadata[0].value != "desktop_flutter"
    {
        return None;
    }
    let hash = canonical_payload_hash(envelope.payload.as_ref());
    if envelope.payload_hash.as_deref() != Some(&hash) {
        return None;
    }
    let operation = envelope.operation?.as_str();
    let payload = envelope.payload.as_ref()?;
    let registration = operation == "AgentCLI実行系作業領域登録";
    let text = if registration {
        let summary = crate::broker::protocol::macos_agent_registration_summary(payload)?;
        format!("{summary}\n要求hash: {hash}")
    } else if operation == "資格情報失効" {
        let p = payload.as_object()?;
        let fields = [
            "版",
            "資格情報ID",
            "用途",
            "接続対象",
            "暗号文hash",
            "作成監査ID",
        ];
        let text = |s: &str| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control);
        let id = p.get("資格情報ID")?.as_str()?;
        let ciphertext_hash = p.get("暗号文hash")?.as_str()?;
        if p.len() != fields.len()
            || !fields.iter().all(|f| p.contains_key(*f))
            || p.get("版")? != &serde_json::json!(1)
            || id.len() != 32
            || !id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || ciphertext_hash.len() != 71
            || !ciphertext_hash.starts_with("sha256:")
            || !["用途", "接続対象", "作成監査ID"].iter().all(|f| {
                p.get(*f)
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(text)
            })
        {
            return None;
        }
        format!("資格情報を論理失効します。取消できません。\nID: {id}\n用途: {}\n接続対象: {}\n暗号文hash: {ciphertext_hash}\n作成監査ID: {}\n要求hash: {hash}\nBrokerが現在metadataを再照合します。秘密値は表示・物理削除しません。",
            p["用途"].as_str()?, p["接続対象"].as_str()?, p["作成監査ID"].as_str()?)
    } else if let Some(summary) =
        adapter_center::owner_confirmation_summary(operation.strip_prefix("アダプター")?, payload)
    {
        format!(
            "Adapter {}をこのBrokerのcatalogへ要求します。\nAdapter: {}\n現在Adapter hash: {}\n要求hash: {}\n\nBrokerが現在のrecord、hash、署名と状態条件を再評価します。外部codeの起動・Permission・Approval・Credential・trustの付与は行いません。削除はcatalogのrecordだけが対象です。",
            summary.operation, summary.adapter_id, summary.adapter_hash, hash,
        )
    } else {
        let summary = adapter_center::owner_manifest_confirmation_summary(
            operation.strip_prefix("アダプター")?,
            payload,
        )?;
        format!(
        "Adapter {}をこのBrokerのcatalogへ登録します。\nAdapter: {}\nRuntime: {}\n発行者: {}\n版: {}\n接続: {}\n内容露出: {}\n要求Capability: {}\n許可差分（要求のみ）: {}\n既知の危険: {}\n互換性: {}\n署名者: {}\n署名対象hash: {}\n現在Adapter hash: {}\n要求hash: {}\n\n外部codeの起動・Permission・Approval・Credential・trustの付与は行いません。署名の検証・有効化は別操作です。",
        summary.operation, summary.adapter_id, summary.runtime_id, summary.publisher,
        summary.adapter_version, summary.transport, summary.content_exposure,
        summary.requested_capabilities.join(" / "), summary.permission_diff.join(" / "),
        summary.known_risks.join(" / "), summary.compatibility, summary.signer_fingerprint,
        summary.signed_bytes_hash, summary.current_adapter_hash.as_deref().unwrap_or("新規"), hash,
        )
    };
    let normalized = super::macos_desktop_worker::normalize(frame, &endpoint.session_id);
    Some(Candidate {
        request: String::from_utf8(normalized).ok()?,
        summary: text,
        // 登録は既存5秒probeを2回実行する。OS確認300秒とは別の限定待機。
        response_timeout: Duration::from_secs(if registration { 15 } else { 4 }),
    })
}

/// Noneはnormal IPCへ戻す。戻った要求は既存Brokerが拒否・監査し、承認にならない。
pub(super) fn dispatch(
    frame: &[u8],
    endpoint: &BrokerEndpoint,
    sender: &SyncSender<DesktopOwnerOperationRequest>,
    confirm: &mut impl FnMut(&str) -> bool,
) -> std::io::Result<Option<Vec<u8>>> {
    let Some(candidate) = candidate(frame, endpoint) else {
        return Ok(None);
    };
    if !confirm(&candidate.summary) {
        return Ok(None);
    }
    let (reply, response) = mpsc::sync_channel(1);
    sender
        .try_send(DesktopOwnerOperationRequest {
            request_json: candidate.request,
            download_confirmation: None,
            apply_confirmation: None,
            activation_confirmation: None,
            product_repair_confirmation: None,
            product_uninstall_confirmation: None,
            reply,
        })
        .map_err(|_| std::io::Error::other("Owner操作のBroker配送が不能"))?;
    let response = response
        .recv_timeout(candidate.response_timeout)
        .map_err(|_| std::io::Error::other("Owner操作の応答が未確定。再送しない"))?;
    Ok(Some(
        response
            .to_json_string()
            .map_err(std::io::Error::other)?
            .into_bytes(),
    ))
}
