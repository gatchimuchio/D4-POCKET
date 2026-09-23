//! 監査結果から生成する、権限を持たない通知センター。
//!
//! 通知を外部callerが登録する操作は提供しない。通知本文はBrokerが保持する監査eventの
//! source、decision、event id/hashだけから限定射影し、reason、payload、metadataを表示しない。
#![allow(non_snake_case)]

use super::protocol::{
    canonical_payload_hash, Broker, BrokerOperation, BrokerResponse, BrokerStatus,
    EVIDENCE_SOURCE_INTERNAL_STATE,
};
use super::store::{BrokerPersistentStore, BrokerStoreError};
use crate::broker::audit::BrokerAuditEvent;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const VERSION: u64 = 1;
const MAX_AUDIT_SCAN: usize = 4096;
const MAX_NOTIFICATIONS: usize = 256;
const MAX_PERSISTED_STATES: usize = 4096;
const OP_LIST: &str = "通知一覧";
const OP_READ: &str = "通知既読";
const OP_DISMISS: &str = "通知破棄";
const OP_READ_ALL: &str = "通知全既読";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NotificationState {
    #[serde(rename = "通知ID")]
    pub(super) notification_id: String,
    #[serde(rename = "通知hash")]
    pub(super) notification_hash: String,
    #[serde(rename = "状態")]
    pub(super) state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NotificationRecord {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "通知ID")]
    notification_id: String,
    #[serde(rename = "source")]
    source: String,
    #[serde(rename = "severity")]
    severity: String,
    #[serde(rename = "タイトル")]
    title: String,
    #[serde(rename = "概要")]
    summary: String,
    #[serde(rename = "作成順")]
    created_order: u64,
    #[serde(rename = "関連監査ID")]
    related_audit_id: String,
    #[serde(rename = "関連監査hash")]
    related_audit_hash: String,
    #[serde(rename = "遷移先")]
    navigation_target: String,
    #[serde(rename = "状態")]
    state: String,
    #[serde(rename = "通知hash")]
    notification_hash: String,
    #[serde(rename = "表示範囲")]
    display_range: String,
    #[serde(rename = "権限生成")]
    authority_generation: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "未読のみ", default)]
    unread_only: bool,
    #[serde(rename = "上限", default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActionRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "通知ID")]
    notification_id: String,
    #[serde(rename = "通知hash")]
    notification_hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadAllRequest {
    #[serde(rename = "版")]
    version: u64,
}

pub(super) fn load_persistent_states(
    store: &BrokerPersistentStore,
) -> Result<BTreeMap<String, NotificationState>, BrokerStoreError> {
    let state = store.load_notification_state()?;
    decode_state(&state).map_err(BrokerStoreError::MalformedNotificationState)
}

fn decode_state(state: &Value) -> Result<BTreeMap<String, NotificationState>, String> {
    let object = state
        .as_object()
        .ok_or_else(|| "notification stateはobjectでなければならない".to_string())?;
    if object.keys().any(|key| key != "版" && key != "states") {
        return Err("notification stateに未知fieldがある".to_string());
    }
    if object.get("版") != Some(&Value::from(VERSION)) {
        return Err("notification stateの版が不正".to_string());
    }
    let states = object
        .get("states")
        .and_then(Value::as_array)
        .ok_or_else(|| "notification stateのstatesがarrayではない".to_string())?;
    if states.len() > MAX_PERSISTED_STATES {
        return Err("notification stateの件数上限を超過".to_string());
    }
    let mut result = BTreeMap::new();
    for value in states {
        let state: NotificationState = serde_json::from_value(value.clone())
            .map_err(|_| "notification stateのrecord構造が不正".to_string())?;
        validate_state(&state)?;
        if result
            .insert(state.notification_id.clone(), state)
            .is_some()
        {
            return Err("notification stateに重複通知IDがある".to_string());
        }
    }
    Ok(result)
}

fn state_value(states: &BTreeMap<String, NotificationState>) -> Value {
    json!({
        "版": VERSION,
        "states": states.values().collect::<Vec<_>>(),
    })
}

fn persist(broker: &Broker) -> Result<(), String> {
    broker
        .state_store
        .write_notification_state(&state_value(&broker.notification_states))
        .map_err(|error| error.message())
}

pub(super) fn dispatch(
    broker: &mut Broker,
    operation: BrokerOperation,
    payload: &Value,
    request_id: &str,
    payload_hash: &str,
) -> BrokerResponse {
    match operation.as_str() {
        OP_LIST => list(broker, payload, request_id, payload_hash),
        OP_READ => action(broker, payload, request_id, payload_hash, "read"),
        OP_DISMISS => action(broker, payload, request_id, payload_hash, "dismissed"),
        OP_READ_ALL => read_all(broker, payload, request_id, payload_hash),
        _ => broker.reject_with_payload_hash(
            request_id,
            operation.as_str(),
            "notification_operation_unknown",
            "通知操作が未定義",
            true,
            payload_hash,
        ),
    }
}

fn list(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    let request: ListRequest = match serde_json::from_value::<ListRequest>(payload.clone()) {
        Ok(request) if request.version == VERSION => request,
        _ => {
            return reject(
                broker,
                request_id,
                OP_LIST,
                "notification_list_invalid",
                "通知一覧の要求が不正",
                hash,
            )
        }
    };
    let limit = request.limit.unwrap_or(MAX_NOTIFICATIONS);
    if !(1..=MAX_NOTIFICATIONS).contains(&limit) {
        return reject(
            broker,
            request_id,
            OP_LIST,
            "notification_limit_invalid",
            "通知一覧の上限は1から256までである",
            hash,
        );
    }
    let mut notifications = project_notifications(broker);
    if request.unread_only {
        notifications.retain(|item| item.state == "unread");
    }
    notifications.truncate(limit);
    let unread_count = notifications
        .iter()
        .filter(|item| item.state == "unread")
        .count();
    let critical_count = notifications
        .iter()
        .filter(|item| item.severity == "critical")
        .count();
    accepted(
        broker,
        OP_LIST,
        request_id,
        json!({
            "版": VERSION,
            "通知一覧": notifications,
            "件数": notifications.len(),
            "未読件数": unread_count,
            "重大件数": critical_count,
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
            "表示範囲": "summary",
            "権限生成": "なし",
            "操作": "navigation_only",
        }),
        hash,
        "監査eventから通知summaryを射影。通知は権限を生成しない",
    )
}

fn action(
    broker: &mut Broker,
    payload: &Value,
    request_id: &str,
    hash: &str,
    state: &str,
) -> BrokerResponse {
    let operation = if state == "read" { OP_READ } else { OP_DISMISS };
    let request: ActionRequest = match serde_json::from_value::<ActionRequest>(payload.clone()) {
        Ok(request) if request.version == VERSION => request,
        _ => {
            return reject(
                broker,
                request_id,
                operation,
                "notification_action_invalid",
                "通知操作の要求が不正",
                hash,
            )
        }
    };
    let current = project_notifications(broker)
        .into_iter()
        .find(|item| item.notification_id == request.notification_id);
    let Some(current) = current else {
        return reject(
            broker,
            request_id,
            operation,
            "notification_not_found",
            "対象通知が現在の監査射影に存在しない",
            hash,
        );
    };
    if current.notification_hash != request.notification_hash {
        return reject(
            broker,
            request_id,
            operation,
            "notification_stale_hash",
            "通知hashが現在の通知射影と一致しない",
            hash,
        );
    }
    if broker
        .append_audit(
            request_id,
            operation,
            "received",
            "通知状態操作を受信。通知は権限を生成しない",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            operation,
            "notification_audit_append_failed",
            "通知状態操作の受信Auditを確定できない",
        );
    }
    let previous = broker.notification_states.insert(
        request.notification_id.clone(),
        NotificationState {
            notification_id: request.notification_id.clone(),
            notification_hash: request.notification_hash.clone(),
            state: state.to_string(),
        },
    );
    if let Err(reason) = persist(broker) {
        match previous {
            Some(value) => {
                broker
                    .notification_states
                    .insert(request.notification_id.clone(), value);
            }
            None => {
                broker
                    .notification_states
                    .remove(&request.notification_id);
            }
        }
        return broker.audit_store_failed_response(
            request_id,
            operation,
            "notification_persistence_failed",
            &reason,
        );
    }
    accepted(
        broker,
        operation,
        request_id,
        json!({
            "版": VERSION,
            "通知ID": request.notification_id,
            "状態": state,
            "通知hash": request.notification_hash,
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
            "権限生成": "なし",
            "操作": "navigation_only",
        }),
        hash,
        "通知表示状態を確定。Approval、Permission、Authorityを変更しない",
    )
}

fn read_all(
    broker: &mut Broker,
    payload: &Value,
    request_id: &str,
    hash: &str,
) -> BrokerResponse {
    let request: ReadAllRequest = match serde_json::from_value::<ReadAllRequest>(payload.clone()) {
        Ok(request) if request.version == VERSION => request,
        _ => {
            return reject(
                broker,
                request_id,
                OP_READ_ALL,
                "notification_read_all_invalid",
                "通知全既読の要求が不正",
                hash,
            )
        }
    };
    let _ = request;
    let previous = broker.notification_states.clone();
    for current in project_notifications(broker) {
        broker.notification_states.insert(
            current.notification_id.clone(),
            NotificationState {
                notification_id: current.notification_id,
                notification_hash: current.notification_hash,
                state: "read".to_string(),
            },
        );
    }
    if broker
        .append_audit(
            request_id,
            OP_READ_ALL,
            "received",
            "通知全既読を受信。通知は権限を生成しない",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            hash,
        )
        .is_err()
    {
        broker.notification_states = previous;
        return broker.audit_store_failed_response(
            request_id,
            OP_READ_ALL,
            "notification_audit_append_failed",
            "通知全既読の受信Auditを確定できない",
        );
    }
    if let Err(reason) = persist(broker) {
        broker.notification_states = previous;
        return broker.audit_store_failed_response(
            request_id,
            OP_READ_ALL,
            "notification_persistence_failed",
            &reason,
        );
    }
    accepted(
        broker,
        OP_READ_ALL,
        request_id,
        json!({
            "版": VERSION,
            "状態": "read",
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
            "権限生成": "なし",
            "操作": "navigation_only",
        }),
        hash,
        "表示中の通知を既読化。Approval、Permission、Authorityを変更しない",
    )
}

fn project_notifications(broker: &Broker) -> Vec<NotificationRecord> {
    let mut result = Vec::new();
    for event in broker
        .audit_log
        .events()
        .iter()
        .rev()
        .take(MAX_AUDIT_SCAN)
    {
        let Some((source, target)) = source_for_operation(&event.operation) else {
            continue;
        };
        let notification_id = format!("notification-{}", event.event_id);
        let state = broker
            .notification_states
            .get(&notification_id)
            .filter(|state| state.notification_hash == notification_hash(event, source, target))
            .map(|state| state.state.as_str())
            .unwrap_or("unread");
        if state == "dismissed" {
            continue;
        }
        let mut record = build_record(event, source, target);
        record.state = state.to_string();
        result.push(record);
        if result.len() >= MAX_NOTIFICATIONS {
            break;
        }
    }
    result
}

fn build_record(event: &BrokerAuditEvent, source: &str, target: &str) -> NotificationRecord {
    let severity = severity_for(source, &event.decision);
    let title = format!("{source}の状態更新");
    let summary = match event.decision.as_str() {
        "accepted" => format!("{source}操作を受け付けました。"),
        "rejected" => format!("{source}操作が拒否されました。監査ビューアーで確認してください。"),
        "suspended" => format!("{source}操作を保留しました。復旧手順を確認してください。"),
        _ => format!("{source}操作の結果を確認してください。"),
    };
    let created_order = event
        .event_id
        .strip_prefix("broker-audit-")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let mut record = NotificationRecord {
        version: VERSION,
        notification_id: format!("notification-{}", event.event_id),
        source: source.to_string(),
        severity: severity.to_string(),
        title,
        summary,
        created_order,
        related_audit_id: event.event_id.clone(),
        related_audit_hash: event.event_hash.clone(),
        navigation_target: target.to_string(),
        state: "unread".to_string(),
        notification_hash: String::new(),
        display_range: "summary".to_string(),
        authority_generation: "なし".to_string(),
    };
    record.notification_hash = notification_hash(event, source, target);
    record
}

fn notification_hash(event: &BrokerAuditEvent, source: &str, target: &str) -> String {
    let record = build_hash_projection(event, source, target);
    canonical_payload_hash(Some(&record))
}

fn build_hash_projection(event: &BrokerAuditEvent, source: &str, target: &str) -> Value {
    let severity = severity_for(source, &event.decision);
    let summary = match event.decision.as_str() {
        "accepted" => format!("{source}操作を受け付けました。"),
        "rejected" => format!("{source}操作が拒否されました。監査ビューアーで確認してください。"),
        "suspended" => format!("{source}操作を保留しました。復旧手順を確認してください。"),
        _ => format!("{source}操作の結果を確認してください。"),
    };
    json!({
        "版": VERSION,
        "通知ID": format!("notification-{}", event.event_id),
        "source": source,
        "severity": severity,
        "タイトル": format!("{source}の状態更新"),
        "概要": summary,
        "作成順": event.event_id.strip_prefix("broker-audit-").and_then(|value| value.parse::<u64>().ok()).unwrap_or(0),
        "関連監査ID": event.event_id,
        "関連監査hash": event.event_hash,
        "遷移先": target,
        "表示範囲": "summary",
        "権限生成": "なし",
    })
}

fn source_for_operation(operation: &str) -> Option<(&'static str, &'static str)> {
    if operation.starts_with("通知") {
        return None;
    }
    if operation.starts_with("端末") {
        return Some(("Device", "device"));
    }
    if operation.starts_with("MCP") {
        return Some(("MCP", "agent"));
    }
    if operation.starts_with("A2A") {
        return Some(("A2A", "agent"));
    }
    if operation.starts_with("更新") {
        return Some(("Update", "settings"));
    }
    if operation.starts_with("評価") || operation.starts_with("回帰") {
        return Some(("Evaluation", "evaluation"));
    }
    if operation.contains("承認") || operation.starts_with("approval") {
        return Some(("Approval", "approval"));
    }
    if operation.contains("復旧") || operation.contains("回復") {
        return Some(("Recovery", "recovery"));
    }
    if operation == "audit_verify" || operation.contains("監査") {
        return Some(("Audit", "audit"));
    }
    if operation.starts_with("実行系")
        || operation.starts_with("対話")
        || operation.starts_with("runtime")
        || operation.starts_with("adapter")
    {
        return Some(("Runtime", "runtime"));
    }
    None
}

fn severity_for(source: &str, decision: &str) -> &'static str {
    match (source, decision) {
        ("Audit", "rejected" | "suspended") => "critical",
        (_, "suspended") => "error",
        (_, "rejected") => "warning",
        _ => "info",
    }
}

fn validate_state(state: &NotificationState) -> Result<(), String> {
    if state.notification_id.is_empty()
        || state.notification_id.len() > 256
        || !state.notification_id.starts_with("notification-broker-audit-")
        || state.notification_hash.len() != 71
        || !state.notification_hash.starts_with("sha256:")
        || !state.notification_hash[7..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || !matches!(state.state.as_str(), "read" | "dismissed")
    {
        return Err("notification stateの値が不正".to_string());
    }
    Ok(())
}

fn accepted(
    broker: &mut Broker,
    operation: &str,
    request_id: &str,
    body: Value,
    request_hash: &str,
    reason: &str,
) -> BrokerResponse {
    let event = match broker.append_audit(
        request_id,
        operation,
        "accepted",
        reason,
        EVIDENCE_SOURCE_INTERNAL_STATE,
        &canonical_payload_hash(Some(&body)),
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                operation,
                "notification_audit_append_failed",
                "通知結果Auditを確定できない",
            )
        }
    };
    let _ = request_hash;
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: operation.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
        audit_event_id: event.event_id,
        error: None,
        health: None,
        body: Some(body),
        shutdown_requested: broker.shutdown_requested,
    }
}

fn reject(
    broker: &mut Broker,
    request_id: &str,
    operation: &str,
    code: &str,
    message: &str,
    hash: &str,
) -> BrokerResponse {
    broker.reject_with_payload_hash(request_id, operation, code, message, true, hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 監査eventからsummaryだけを生成しraw_reasonを含めない() {
        let mut broker = Broker::new("session");
        broker
            .append_audit(
                "request",
                "更新確認",
                "rejected",
                "secret-reason-must-not-leak",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .expect("audit");
        let records = project_notifications(&broker);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].source, "Update");
        assert_eq!(records[0].severity, "warning");
        assert!(!serde_json::to_string(&records[0]).unwrap().contains("secret-reason"));
    }

    #[test]
    fn 通知操作自身は通知を生成しない() {
        let mut broker = Broker::new("session");
        broker
            .append_audit(
                "request",
                OP_READ,
                "accepted",
                "通知状態を記録",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .expect("audit");
        assert!(project_notifications(&broker).is_empty());
    }

    #[test]
    fn audit_failure_is_critical() {
        assert_eq!(severity_for("Audit", "rejected"), "critical");
        assert_eq!(severity_for("Runtime", "suspended"), "error");
    }

    #[test]
    fn malformed_state_is_rejected() {
        let result = decode_state(&json!({
            "版": 1,
            "states": [{
                "通知ID": "notification-broker-audit-1",
                "通知hash": "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
                "状態": "read"
            }]
        }));
        assert!(result.is_err());
    }

    #[test]
    fn 既読状態は再起動後に再読込される() {
        let root = std::env::temp_dir().join(format!(
            "gui-shell-notification-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut broker = Broker::new_persistent("session", &root).expect("永続Broker");
        broker
            .append_audit(
                "request",
                "更新確認",
                "accepted",
                "内部理由は通知へ出さない",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .expect("audit");
        let notification = project_notifications(&broker).pop().expect("notification");
        let response = action(
            &mut broker,
            &json!({
                "版": VERSION,
                "通知ID": notification.notification_id,
                "通知hash": notification.notification_hash,
            }),
            "read-request",
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "read",
        );
        assert_eq!(response.status, BrokerStatus::Accepted);
        drop(broker);

        let reopened = Broker::new_persistent("session-2", &root).expect("Broker再起動");
        let notifications = project_notifications(&reopened);
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].state, "read");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn 永続状態の不正値は再起動を停止する() {
        let root = std::env::temp_dir().join(format!(
            "gui-shell-notification-malformed-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let (store, _) = BrokerPersistentStore::open_or_create(&root, "session").expect("保管庫");
        store
            .write_notification_state(&json!({
                "版": VERSION,
                "states": [{
                    "通知ID": "notification-broker-audit-1",
                    "通知hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "状態": "unknown"
                }]
            }))
            .expect("不正状態の書込み");
        assert!(matches!(
            Broker::new_persistent("session-2", &root),
            Err(BrokerStoreError::MalformedNotificationState(_))
        ));
        let _ = std::fs::remove_dir_all(root);
    }
}
