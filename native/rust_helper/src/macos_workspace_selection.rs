//! OSの選択だけを既存private receiverへ結合する。Owner承認とAuthorityを生成しない。
use crate::broker::ipc_server::DesktopOwnerOperationRequest;
use crate::broker::protocol::{canonical_payload_hash, request_issued_at_is_current};
use crate::broker::{BrokerEndpoint, BrokerRequestEnvelope};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::io;
use std::path::Path;
use std::sync::mpsc::{self, SyncSender};
use std::time::Duration;

const OPERATION: &str = "作業領域OS選択";
const SCOPE_LIMIT: usize = 8;

fn candidate(frame: &[u8], endpoint: &BrokerEndpoint) -> Option<Value> {
    if frame.len() >= endpoint.max_request_bytes {
        return None;
    }
    let text = std::str::from_utf8(frame).ok()?;
    let envelope = BrokerRequestEnvelope::from_json_str(text).ok()?;
    if envelope.operation?.as_str() != OPERATION
        || envelope.session_id.is_some()
        || !envelope
            .request_id
            .as_deref()
            .is_some_and(|v| !v.is_empty() && v.len() <= 256)
        || !envelope
            .nonce
            .as_deref()
            .is_some_and(|v| !v.is_empty() && v.len() <= 256)
        || !envelope
            .issued_at
            .as_deref()
            .is_some_and(request_issued_at_is_current)
        || envelope.metadata.len() != 1
        || envelope.metadata[0].key != "client"
        || envelope.metadata[0].value != "desktop_flutter"
        || envelope.payload != Some(json!({"version":1}))
        || envelope.payload_hash.as_deref()
            != Some(&canonical_payload_hash(envelope.payload.as_ref()))
    {
        return None;
    }
    serde_json::from_slice(frame).ok()
}

pub(super) fn dispatch<T: AsRef<Path>>(
    frame: &[u8],
    endpoint: &BrokerEndpoint,
    sender: &SyncSender<DesktopOwnerOperationRequest>,
    scopes: &mut Vec<T>,
    seen: &mut BTreeSet<String>,
    choose: &mut impl FnMut() -> Result<Option<T>, &'static str>,
) -> io::Result<Option<Vec<u8>>> {
    let Some(mut request) = candidate(frame, endpoint) else {
        return Ok(None);
    };
    let nonce = request["nonce"].as_str().unwrap().to_owned();
    if seen.len() >= 64 || !seen.insert(nonce) || scopes.len() >= SCOPE_LIMIT {
        return Ok(None);
    }
    let original_hash = request["payload_hash"].clone();
    let selection = choose();
    let (state, selected) = match selection {
        Ok(Some(scope))
            if scope.as_ref().to_str().is_some_and(|p| {
                scope.as_ref().is_absolute() && p.len() <= 1024 && !p.chars().any(char::is_control)
            }) =>
        {
            ("selected", Some(scope))
        }
        Ok(None) => ("cancelled", None),
        _ => ("failed", None),
    };
    request["session_id"] = json!(endpoint.session_id);
    request["payload"] = json!({
        "version":1, "selection_status":state,
        "workspace_root": selected.as_ref().and_then(|s| s.as_ref().to_str()),
        "selection_request_hash": original_hash,
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
        .map_err(|_| io::Error::other("OS選択のBroker配送が不能"))?;
    let response = receiver
        .recv_timeout(Duration::from_secs(4))
        .map_err(|_| io::Error::other("OS選択のBroker応答が未確定。再送しない"))?;
    if response.status == crate::broker::BrokerStatus::Accepted {
        if let Some(selected) = selected {
            scopes.push(selected);
        }
    }
    Ok(Some(
        response
            .to_json_string()
            .map_err(io::Error::other)?
            .into_bytes(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_selection_cancel_is_audited_and_never_replayed() {
        let endpoint = BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 1,
            transport: "authenticated_loopback_tcp".into(),
            session_id: "selection-session".into(),
            session_secret: "fixture-only".into(),
            credential_role: crate::broker::BrokerCredentialRole::Normal,
            max_request_bytes: 65536,
        };
        let request=json!({"request_id":"cancel-selection", "operation":OPERATION,
            "nonce":"cancel-selection-nonce", "issued_at":BrokerRequestEnvelope::current_issued_at(),
            "metadata":{"client":"desktop_flutter"}, "payload":{"version":1},
            "payload_hash":canonical_payload_hash(Some(&json!({"version":1})))}).to_string();
        let (sender, receiver) = mpsc::sync_channel::<DesktopOwnerOperationRequest>(1);
        let worker = std::thread::spawn(move || {
            let mut broker = crate::broker::Broker::new("selection-session");
            broker.set_desktop_setup_doctor_runtime_evidence(true, false, true);
            let message = receiver.recv().unwrap();
            let response = broker.desktop_owner_operation_json(&message.request_json);
            message.reply.send(response).unwrap();
            broker.audit_events().len()
        });
        let mut scopes = Vec::<std::path::PathBuf>::new();
        let mut seen = BTreeSet::new();
        let mut calls = 0;
        let mut choose = || {
            calls += 1;
            Ok(None)
        };
        let response = dispatch(
            request.as_bytes(),
            &endpoint,
            &sender,
            &mut scopes,
            &mut seen,
            &mut choose,
        )
        .unwrap()
        .unwrap();
        let response: Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(response["status"], "accepted");
        assert_eq!(response["body"]["selection_status"], "cancelled");
        assert!(response["body"]["workspace_root"].is_null());
        assert!(!response["audit_event_id"].as_str().unwrap().is_empty());
        assert!(dispatch(
            request.as_bytes(),
            &endpoint,
            &sender,
            &mut scopes,
            &mut seen,
            &mut choose
        )
        .unwrap()
        .is_none());
        assert_eq!(calls, 1);
        assert!(scopes.is_empty());
        assert_eq!(worker.join().unwrap(), 1);
    }
    #[test]
    fn workspace_selection_broker_projection_is_not_owner_authority() {
        use crate::broker::{Broker, BrokerStatus};
        let mut broker = Broker::new("selection-session");
        broker.set_desktop_setup_doctor_runtime_evidence(true, false, true);
        let payload = json!({"version":1,"selection_status":"selected",
            "workspace_root":"/external-fixture-workspace",
            "selection_request_hash":canonical_payload_hash(Some(&json!({"version":1})))});
        let request = json!({"request_id":"selection", "operation":OPERATION,
            "session_id":"selection-session", "nonce":"selection-broker-nonce",
            "issued_at":BrokerRequestEnvelope::current_issued_at(),
            "metadata":{"client":"desktop_flutter"}, "payload_hash":canonical_payload_hash(Some(&payload)),
            "payload":payload});
        assert_eq!(
            broker.handle_json(&request.to_string()).status,
            BrokerStatus::Rejected
        );
        assert_eq!(
            broker.owner要求処理(&request.to_string()).status,
            BrokerStatus::Rejected
        );
        let response = broker.desktop_owner_operation_json(&request.to_string());
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.unwrap();
        assert_eq!(body["workspace_root"], "/external-fixture-workspace");
        for field in [
            "permission_generated",
            "approval_generated",
            "registration_generated",
        ] {
            assert_eq!(body[field], false);
        }
        assert_eq!(
            broker
                .desktop_owner_operation_json(&request.to_string())
                .status,
            BrokerStatus::Rejected
        );
        let mut cancel = request.clone();
        cancel["nonce"] = json!("cancel-nonce");
        cancel["payload"]["selection_status"] = json!("cancelled");
        cancel["payload"]["workspace_root"] = Value::Null;
        cancel["payload_hash"] = json!(canonical_payload_hash(Some(&cancel["payload"])));
        assert_eq!(
            broker
                .desktop_owner_operation_json(&cancel.to_string())
                .status,
            BrokerStatus::Accepted
        );
        cancel["nonce"] = json!("injection-nonce");
        cancel["payload"]["approval"] = json!(true);
        cancel["payload_hash"] = json!(canonical_payload_hash(Some(&cancel["payload"])));
        assert_eq!(
            broker
                .desktop_owner_operation_json(&cancel.to_string())
                .status,
            BrokerStatus::Rejected
        );
        let audit = serde_json::to_string(broker.audit_events()).unwrap();
        assert!(!audit.contains("external-fixture-workspace"));
    }
    #[test]
    fn workspace_selection_rejects_ui_scope_and_authority_injection() {
        let endpoint = BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 1,
            transport: "authenticated_loopback_tcp".into(),
            session_id: "session-test".into(),
            session_secret: "not-a-production-secret".into(),
            credential_role: crate::broker::BrokerCredentialRole::Normal,
            max_request_bytes: 65536,
        };
        let base = json!({"request_id":"selection-test", "operation":OPERATION,
            "nonce":"selection-nonce", "issued_at":BrokerRequestEnvelope::current_issued_at(),
            "metadata":{"client":"desktop_flutter"}, "payload":{"version":1},
            "payload_hash":canonical_payload_hash(Some(&json!({"version":1})))});
        assert!(candidate(base.to_string().as_bytes(), &endpoint).is_some());
        for key in [
            "workspace_root",
            "permission",
            "approval",
            "bookmark",
            "credential",
        ] {
            let mut injected = base.clone();
            injected["payload"][key] = json!("attacker");
            injected["payload_hash"] = json!(canonical_payload_hash(Some(&injected["payload"])));
            assert!(candidate(injected.to_string().as_bytes(), &endpoint).is_none());
        }
        let mut injected = base.clone();
        injected["session_id"] = json!("session-test");
        assert!(candidate(injected.to_string().as_bytes(), &endpoint).is_none());
    }
}
