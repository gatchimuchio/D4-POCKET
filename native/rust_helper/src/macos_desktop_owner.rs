//! macOSの限定Owner入口。UIデータを承認へ昇格せず、既存Brokerへ現在要求を渡す。
use crate::broker::ipc_server::DesktopOwnerOperationRequest;
use crate::broker::protocol::{canonical_payload_hash, request_issued_at_is_current};
use crate::broker::{adapter_center, BrokerEndpoint, BrokerRequestEnvelope};
use std::sync::mpsc::{self, SyncSender};
use std::time::Duration;

struct Candidate {
    request: String,
    summary: String,
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
    let operation = envelope.operation?.as_str().strip_prefix("アダプター")?;
    let payload = envelope.payload.as_ref()?;
    let text = if let Some(summary) = adapter_center::owner_confirmation_summary(operation, payload)
    {
        format!(
            "Adapter {}をこのBrokerのcatalogへ要求します。\nAdapter: {}\n現在Adapter hash: {}\n要求hash: {}\n\nBrokerが現在のrecord、hash、署名と状態条件を再評価します。外部codeの起動・Permission・Approval・Credential・trustの付与は行いません。削除はcatalogのrecordだけが対象です。",
            summary.operation, summary.adapter_id, summary.adapter_hash, hash,
        )
    } else {
        let summary = adapter_center::owner_manifest_confirmation_summary(operation, payload)?;
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
        .recv_timeout(Duration::from_secs(4))
        .map_err(|_| std::io::Error::other("Owner操作の応答が未確定。再送しない"))?;
    Ok(Some(
        response
            .to_json_string()
            .map_err(std::io::Error::other)?
            .into_bytes(),
    ))
}
