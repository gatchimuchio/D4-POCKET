//! Compile-time Module selection enforced at the authenticated Broker IPC boundary.
//!
//! This gate prevents an excluded optional surface from invoking its Broker
//! operation through an alternate client. The optional implementation code is
//! not yet conditionally omitted from the Rust binary; build evidence must keep
//! `binary_pruning_verified` false until that source-level pruning is complete.

use serde_json::Value;

use super::protocol::{Broker, BrokerOperation, BrokerRequestEnvelope, BrokerResponse};

pub(super) fn reject_if_pruned(broker: &mut Broker, request_json: &str) -> Option<BrokerResponse> {
    let envelope = BrokerRequestEnvelope::from_json_str(request_json).ok()?;
    let operation = envelope.operation.as_ref()?;
    if envelope.request_id.as_deref().unwrap_or("").is_empty()
        || envelope.payload_hash.as_deref().unwrap_or("").is_empty()
        || envelope.issued_at.as_deref().unwrap_or("").is_empty()
        || envelope.nonce.as_deref().unwrap_or("").is_empty()
        || !envelope.metadata_present
    {
        return None;
    }
    let module_id = module_for_operation(operation, envelope.payload.as_ref())?;
    if module_enabled(module_id) {
        return None;
    }
    let request_id = envelope
        .request_id
        .as_deref()
        .filter(|value| !value.is_empty())?;
    let payload_hash = crate::audit_hash::sha256_tagged(request_json.as_bytes());
    Some(broker.reject_with_payload_hash(
        request_id,
        operation.as_str(),
        "optional_module_not_included",
        "この独立Appに含まれないModuleのBroker操作を拒否しました。",
        true,
        &payload_hash,
    ))
}

fn module_for_operation(
    operation: &BrokerOperation,
    payload: Option<&Value>,
) -> Option<&'static str> {
    use BrokerOperation::*;

    match operation {
        SetupDoctor報告取得 => Some("shell.setup_doctor"),
        対話履歴承認
        | 対話履歴失効
        | 対話履歴閲覧状態
        | 対話履歴閲覧
        | AgentTask履歴閲覧
        | 対話再実行
        | 対話分岐
        | 対話履歴一覧 => Some("shell.history"),
        評価Dataset登録
        | 評価Dataset一覧
        | 評価実験開始
        | 評価実験状態
        | 評価比較
        | 回帰Case登録
        | 回帰Case一覧
        | 回帰Case削除
        | 回帰Case削除中断確認 => Some("shell.evaluation_lab"),
        ホスト能力 => Some("shell.host_capabilities"),
        通知一覧 | 通知既読 | 通知破棄 | 通知全既読 => Some("shell.notifications"),
        観測一覧 if payload.is_some_and(|body| body.get("TraceID").is_some()) => {
            Some("shell.trace_inspector")
        }
        観測一覧 => Some("shell.observability"),
        Host登録 | Host切替 => Some("shell.host_operations"),
        // Host一覧 is part of the shared Broker startup projection, not the
        // optional Host Operations surface.
        _ => None,
    }
}

fn module_enabled(module_id: &str) -> bool {
    match module_id {
        "shell.setup_doctor" => cfg!(feature = "module_setup_doctor"),
        "shell.history" => cfg!(feature = "module_history"),
        "shell.evaluation_lab" => cfg!(feature = "module_evaluation_lab"),
        "shell.host_capabilities" => cfg!(feature = "module_host_capabilities"),
        "shell.notifications" => cfg!(feature = "module_notifications"),
        "shell.observability" => cfg!(feature = "module_observability"),
        "shell.trace_inspector" => cfg!(feature = "module_trace_inspector"),
        "shell.host_operations" => cfg!(feature = "module_host_operations"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{module_enabled, module_for_operation, reject_if_pruned};
    use crate::broker::{Broker, BrokerOperation, BrokerStatus};
    use serde_json::json;

    #[test]
    fn optional_operations_bind_to_their_catalog_module() {
        assert_eq!(
            module_for_operation(&BrokerOperation::SetupDoctor報告取得, None),
            Some("shell.setup_doctor")
        );
        assert_eq!(
            module_for_operation(&BrokerOperation::AgentTask履歴閲覧, None),
            Some("shell.history")
        );
        assert_eq!(
            module_for_operation(&BrokerOperation::評価Dataset登録, None),
            Some("shell.evaluation_lab")
        );
        assert_eq!(
            module_for_operation(&BrokerOperation::通知一覧, None),
            Some("shell.notifications")
        );
        assert_eq!(
            module_for_operation(
                &BrokerOperation::観測一覧,
                Some(&json!({"TraceID": "trace-1"}))
            ),
            Some("shell.trace_inspector")
        );
        assert_eq!(
            module_for_operation(&BrokerOperation::観測一覧, Some(&json!({"limit": 10}))),
            Some("shell.observability")
        );
        assert_eq!(
            module_for_operation(&BrokerOperation::Host切替, None),
            Some("shell.host_operations")
        );
    }

    #[test]
    fn shared_core_operations_are_not_classified_as_optional() {
        assert_eq!(module_for_operation(&BrokerOperation::Health, None), None);
        assert_eq!(module_for_operation(&BrokerOperation::Host一覧, None), None);
        assert_eq!(
            module_for_operation(&BrokerOperation::作業領域一覧, None),
            None
        );
    }

    #[test]
    fn cargo_feature_selection_controls_each_optional_gate() {
        assert_eq!(
            module_enabled("shell.setup_doctor"),
            cfg!(feature = "module_setup_doctor")
        );
        assert_eq!(
            module_enabled("shell.history"),
            cfg!(feature = "module_history")
        );
        assert_eq!(
            module_enabled("shell.evaluation_lab"),
            cfg!(feature = "module_evaluation_lab")
        );
        assert_eq!(
            module_enabled("shell.host_capabilities"),
            cfg!(feature = "module_host_capabilities")
        );
        assert_eq!(
            module_enabled("shell.notifications"),
            cfg!(feature = "module_notifications")
        );
        assert_eq!(
            module_enabled("shell.observability"),
            cfg!(feature = "module_observability")
        );
        assert_eq!(
            module_enabled("shell.trace_inspector"),
            cfg!(feature = "module_trace_inspector")
        );
        assert_eq!(
            module_enabled("shell.host_operations"),
            cfg!(feature = "module_host_operations")
        );
        assert!(!module_enabled("shell.unknown"));
    }

    #[test]
    fn excluded_broker_operation_is_rejected_with_an_audit_receipt() {
        let mut broker = Broker::new("module-pruning-test-session");
        let request = json!({
            "request_id": "module-pruning-test-request",
            "operation": "Setup Doctor報告取得",
            "payload_hash": "sha256:test-payload-hash",
            "nonce": "module-pruning-test-nonce",
            "issued_at": "2026-11-16T00:00:00Z",
            "metadata": {},
            "payload": {"version": 1}
        })
        .to_string();

        let response = reject_if_pruned(&mut broker, &request);
        if cfg!(feature = "module_setup_doctor") {
            assert!(response.is_none());
        } else {
            let response = response.expect("excluded optional operation must be rejected");
            assert_eq!(response.status, BrokerStatus::Rejected);
            assert_eq!(response.operation, "Setup Doctor報告取得");
            assert!(!response.audit_event_id.is_empty());
            assert_eq!(
                response.error.as_ref().map(|value| value.code.as_str()),
                Some("optional_module_not_included")
            );
        }
    }
}
