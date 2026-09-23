//! C17 複数Hostのmetadata registryをBrokerへ接続する。
//!
//! Host登録はowner controlだけが行い、通常IPCはmetadata一覧だけを参照する。
//! Host identity、Trust、Runtime summaryは観測または構成metadataであり、
//! Permission、Approval、Authority、Credential、接続実行資格を生成しない。
#![allow(non_snake_case)]

use super::protocol::{Broker, BrokerResponse, BrokerStatus, EVIDENCE_SOURCE_INTERNAL_STATE};
use super::store::{BrokerPersistentStore, BrokerStoreError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const OP_REGISTER: &str = "Host登録";
const OP_LIST: &str = "Host一覧";
const VERSION: u64 = 1;
const MAX_HOSTS: usize = 64;
const MAX_RUNTIME_COUNT: u64 = 256;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HostRegistrationRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "操作")]
    operation: String,
    #[serde(rename = "Host ID")]
    host_id: String,
    #[serde(rename = "表示名")]
    display_name: String,
    #[serde(rename = "Platform")]
    platform: String,
    #[serde(rename = "接続状態")]
    connection_state: String,
    #[serde(rename = "Trust")]
    trust: String,
    #[serde(rename = "証明書/identity")]
    identity: HostIdentity,
    #[serde(rename = "Runtime summary")]
    runtime_summary: RuntimeSummary,
    #[serde(rename = "最終接続")]
    last_connection: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HostIdentity {
    #[serde(rename = "種別")]
    kind: String,
    hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeSummary {
    runtime_count: u64,
    agent_count: u64,
    evidence_source: String,
}

pub(super) fn load_persistent_hosts(
    store: &BrokerPersistentStore,
) -> Result<BTreeMap<String, Value>, BrokerStoreError> {
    let state = store.load_host_state()?;
    decode_state(&state).map_err(BrokerStoreError::MalformedHostState)
}

fn decode_state(state: &Value) -> Result<BTreeMap<String, Value>, String> {
    let object = state
        .as_object()
        .ok_or_else(|| "Host registry stateはobjectでなければならない".to_string())?;
    if object.keys().any(|key| key != "版" && key != "hosts") {
        return Err("Host registry stateに未知fieldがある".to_string());
    }
    if object.get("版") != Some(&Value::from(VERSION)) {
        return Err("Host registry stateの版が不正".to_string());
    }
    let values = object
        .get("hosts")
        .and_then(Value::as_array)
        .ok_or_else(|| "Host registry stateのhostsがarrayではない".to_string())?;
    if values.len() > MAX_HOSTS {
        return Err("Host registry stateの件数上限を超過".to_string());
    }
    let mut result = BTreeMap::new();
    for value in values {
        let host_id = validate_persisted_receipt(value)?;
        if result.insert(host_id, value.clone()).is_some() {
            return Err("Host registry stateに重複Host IDがある".to_string());
        }
    }
    Ok(result)
}

fn state_value(hosts: &BTreeMap<String, Value>) -> Value {
    json!({
        "版": VERSION,
        "hosts": hosts.values().cloned().collect::<Vec<_>>(),
    })
}

fn persist(broker: &Broker) -> Result<(), String> {
    broker
        .state_store
        .write_host_state(&state_value(&broker.hosts))
        .map_err(|error| error.message())
}

fn parse_request(payload: &Value) -> Result<HostRegistrationRequest, String> {
    let request: HostRegistrationRequest = serde_json::from_value(payload.clone())
        .map_err(|_| "Host登録payloadの構造が不正".to_string())?;
    if request.version != VERSION
        || request.operation != "登録"
        || !valid_host_id(&request.host_id)
        || request.display_name.trim().is_empty()
        || request.display_name.chars().count() > 256
        || !valid_platform(&request.platform)
        || request.connection_state != "pending_review"
        || request.trust != "pending_review"
        || request.last_connection.is_some()
        || !valid_identity(&request.identity)
        || !valid_runtime_summary(&request.runtime_summary)
    {
        return Err("Host登録payloadの値またはTrust境界が不正".to_string());
    }
    Ok(request)
}

fn valid_platform(value: &str) -> bool {
    matches!(
        value,
        "windows" | "linux" | "macos" | "android" | "ios" | "unknown"
    )
}

fn valid_host_id(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && chars.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | ':' | '-')
        })
        && value.chars().count() <= 128
}

fn valid_hash(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.chars().all(|character| character.is_ascii_hexdigit())
}

fn valid_identity(identity: &HostIdentity) -> bool {
    matches!(identity.kind.as_str(), "certificate_hash" | "identity_hash")
        && valid_hash(&identity.hash)
}

fn valid_runtime_summary(summary: &RuntimeSummary) -> bool {
    summary.runtime_count <= MAX_RUNTIME_COUNT
        && summary.agent_count <= MAX_RUNTIME_COUNT
        && summary.evidence_source == EVIDENCE_SOURCE_INTERNAL_STATE
}

fn validate_identity(value: &Value) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "Host identityがobjectではない".to_string())?;
    if object.len() != 2
        || object
            .keys()
            .any(|key| !["種別", "hash"].contains(&key.as_str()))
    {
        return Err("Host identityに未知または欠落fieldがある".to_string());
    }
    let kind = object
        .get("種別")
        .and_then(Value::as_str)
        .ok_or_else(|| "Host identityの種別が不正".to_string())?;
    let hash = object
        .get("hash")
        .and_then(Value::as_str)
        .ok_or_else(|| "Host identityのhashが不正".to_string())?;
    if !matches!(kind, "certificate_hash" | "identity_hash") || !valid_hash(hash) {
        return Err("Host identityはhash-onlyでなければならない".to_string());
    }
    Ok(())
}

fn validate_runtime_summary(value: &Value) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "Host Runtime summaryがobjectではない".to_string())?;
    if object.len() != 3
        || object
            .keys()
            .any(|key| !["runtime_count", "agent_count", "evidence_source"].contains(&key.as_str()))
    {
        return Err("Host Runtime summaryに未知または欠落fieldがある".to_string());
    }
    let runtime_count = object
        .get("runtime_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Host runtime_countが不正".to_string())?;
    let agent_count = object
        .get("agent_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Host agent_countが不正".to_string())?;
    if runtime_count > MAX_RUNTIME_COUNT
        || agent_count > MAX_RUNTIME_COUNT
        || object.get("evidence_source") != Some(&Value::from(EVIDENCE_SOURCE_INTERNAL_STATE))
    {
        return Err("Host Runtime summaryはbounded INTERNAL_STATEでなければならない".to_string());
    }
    Ok(())
}

fn validate_persisted_receipt(value: &Value) -> Result<String, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "Host receiptはobjectでなければならない".to_string())?;
    const KEYS: [&str; 18] = [
        "版",
        "Host ID",
        "表示名",
        "Platform",
        "接続状態",
        "Trust",
        "証明書/identity",
        "Runtime summary",
        "最終接続",
        "公開範囲",
        "証拠種別",
        "権限生成",
        "authority_strip",
        "能力ID",
        "権限ID",
        "承認状態",
        "復旧ID",
        "登録監査ID",
    ];
    if object.len() != KEYS.len() || object.keys().any(|key| !KEYS.contains(&key.as_str())) {
        return Err("Host receiptに未知または欠落fieldがある".to_string());
    }
    if object.get("版") != Some(&Value::from(VERSION))
        || object.get("公開範囲") != Some(&Value::from("metadata_only"))
        || object.get("証拠種別") != Some(&Value::from(EVIDENCE_SOURCE_INTERNAL_STATE))
        || object.get("権限生成") != Some(&Value::from("なし"))
        || object.get("authority_strip") != Some(&Value::Bool(true))
        || object.get("能力ID") != Some(&Value::from("host.registry.register"))
        || object.get("権限ID") != Some(&Value::from("permission.host.registry.register"))
        || object.get("承認状態") != Some(&Value::from("owner_control_approved"))
        || object.get("復旧ID") != Some(&Value::from("recover-host-registration"))
        || object.get("最終接続") != Some(&Value::Null)
        || object.get("接続状態") != Some(&Value::from("pending_review"))
    {
        return Err("Host receiptの固定Authority境界が不正".to_string());
    }
    let host_id = object
        .get("Host ID")
        .and_then(Value::as_str)
        .filter(|value| valid_host_id(value))
        .ok_or_else(|| "Host receiptのHost IDが不正".to_string())?
        .to_string();
    let display_name = object
        .get("表示名")
        .and_then(Value::as_str)
        .ok_or_else(|| "Host receiptの表示名が不正".to_string())?;
    if display_name.trim().is_empty() || display_name.chars().count() > 256 {
        return Err("Host receiptの表示名が不正".to_string());
    }
    let platform = object
        .get("Platform")
        .and_then(Value::as_str)
        .ok_or_else(|| "Host receiptのPlatformが不正".to_string())?;
    if !valid_platform(platform) {
        return Err("Host receiptのPlatformが不正".to_string());
    }
    let trust = object
        .get("Trust")
        .and_then(Value::as_object)
        .ok_or_else(|| "Host receiptのTrustがobjectではない".to_string())?;
    if trust.len() != 3
        || trust.keys().any(|key| {
            !["state", "evidence_source", "requires_operator_review"].contains(&key.as_str())
        })
        || trust.get("state") != Some(&Value::from("pending_review"))
        || trust.get("evidence_source") != Some(&Value::from(EVIDENCE_SOURCE_INTERNAL_STATE))
        || trust.get("requires_operator_review") != Some(&Value::Bool(true))
    {
        return Err("Host receiptのTrust境界が不正".to_string());
    }
    validate_identity(object.get("証明書/identity").unwrap())?;
    validate_runtime_summary(object.get("Runtime summary").unwrap())?;
    let audit_id = object
        .get("登録監査ID")
        .and_then(Value::as_str)
        .ok_or_else(|| "Host receiptの登録監査IDが不正".to_string())?;
    if audit_id.is_empty() || audit_id.chars().count() > 256 {
        return Err("Host receiptの登録監査IDが不正".to_string());
    }
    Ok(host_id)
}

pub(super) fn register(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    owner: bool,
    payload_hash: &str,
) -> BrokerResponse {
    if !owner {
        return broker.reject_with_payload_hash(
            request_id,
            OP_REGISTER,
            "host_owner_required",
            "Host登録にはowner controlが必要",
            true,
            payload_hash,
        );
    }
    if !broker.state_store.persistence_ready() {
        return broker.reject_with_payload_hash(
            request_id,
            OP_REGISTER,
            "broker_persistence_unavailable",
            "Host登録には永続Auditとstateが必要",
            true,
            payload_hash,
        );
    }
    let request = match parse_request(payload) {
        Ok(request) => request,
        Err(message) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_REGISTER,
                "host_registration_invalid",
                &message,
                true,
                payload_hash,
            )
        }
    };
    if broker.hosts.contains_key(&request.host_id) {
        return broker.reject_with_payload_hash(
            request_id,
            OP_REGISTER,
            "host_registration_duplicate",
            "同じHost IDを再登録できない",
            true,
            payload_hash,
        );
    }
    if broker.hosts.len() >= MAX_HOSTS {
        return broker.reject_with_payload_hash(
            request_id,
            OP_REGISTER,
            "host_registry_bound_exceeded",
            "Host registryの件数上限を超過",
            true,
            payload_hash,
        );
    }
    if broker
        .append_audit(
            request_id,
            OP_REGISTER,
            "received",
            "owner controlからHost metadata登録要求を受信。identity実値と権限fieldは保存しない",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_REGISTER,
            "host_audit_append_failed",
            "Host登録の受信Auditを確定できない",
        );
    }

    let mut receipt = json!({
        "版": VERSION,
        "Host ID": request.host_id,
        "表示名": request.display_name,
        "Platform": request.platform,
        "接続状態": "pending_review",
        "Trust": {
            "state": "pending_review",
            "evidence_source": EVIDENCE_SOURCE_INTERNAL_STATE,
            "requires_operator_review": true
        },
        "証明書/identity": {
            "種別": request.identity.kind,
            "hash": request.identity.hash
        },
        "Runtime summary": {
            "runtime_count": request.runtime_summary.runtime_count,
            "agent_count": request.runtime_summary.agent_count,
            "evidence_source": EVIDENCE_SOURCE_INTERNAL_STATE
        },
        "最終接続": Value::Null,
        "公開範囲": "metadata_only",
        "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        "権限生成": "なし",
        "authority_strip": true,
        "能力ID": "host.registry.register",
        "権限ID": "permission.host.registry.register",
        "承認状態": "owner_control_approved",
        "復旧ID": "recover-host-registration"
    });
    let body_hash = super::protocol::canonical_payload_hash(Some(&receipt));
    let accepted = match broker.append_audit(
        request_id,
        OP_REGISTER,
        "accepted",
        &format!("Host metadataを受理。Host ID={}", request.host_id),
        EVIDENCE_SOURCE_INTERNAL_STATE,
        &body_hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_REGISTER,
                "host_audit_append_failed",
                "Host登録の受理Auditを確定できない。登録を破棄した",
            )
        }
    };
    receipt
        .as_object_mut()
        .expect("Host receiptはobject")
        .insert(
            "登録監査ID".to_string(),
            Value::String(accepted.event_id.clone()),
        );
    if let Err(reason) = validate_persisted_receipt(&receipt) {
        return broker.audit_store_failed_response(
            request_id,
            OP_REGISTER,
            "host_receipt_invalid",
            &reason,
        );
    }
    broker.hosts.insert(request.host_id, receipt.clone());
    if let Err(reason) = persist(broker) {
        broker.hosts.remove(
            receipt
                .get("Host ID")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        );
        return broker.audit_store_failed_response(
            request_id,
            OP_REGISTER,
            "host_persistence_failed",
            &reason,
        );
    }
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OP_REGISTER.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
        audit_event_id: accepted.event_id,
        error: None,
        health: None,
        body: Some(receipt),
        shutdown_requested: broker.shutdown_requested,
    }
}

pub(super) fn list(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    owner: bool,
    payload_hash: &str,
) -> BrokerResponse {
    if owner {
        return broker.reject_with_payload_hash(
            request_id,
            OP_LIST,
            "host_normal_channel_required",
            "Host一覧は通常IPCだけが参照できる",
            true,
            payload_hash,
        );
    }
    if payload != &json!({ "版": VERSION }) {
        return broker.reject_with_payload_hash(
            request_id,
            OP_LIST,
            "host_list_request_invalid",
            "Host一覧は版だけを受け付ける",
            true,
            payload_hash,
        );
    }
    if broker.hosts.len() > MAX_HOSTS {
        return broker.reject_with_payload_hash(
            request_id,
            OP_LIST,
            "host_registry_bound_exceeded",
            "Host一覧のbounded上限を超過",
            true,
            payload_hash,
        );
    }
    if broker
        .append_audit(
            request_id,
            OP_LIST,
            "received",
            "Host metadata一覧を要求",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_LIST,
            "host_audit_append_failed",
            "Host一覧の受信Auditを確定できない",
        );
    }
    let body = json!({
        "版": VERSION,
        "Host一覧": broker.hosts.values().cloned().collect::<Vec<_>>(),
        "件数": broker.hosts.len(),
        "公開範囲": "metadata_only",
        "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
    });
    let event = match broker.append_audit(
        request_id,
        OP_LIST,
        "accepted",
        "Host metadata一覧を返却。identity実値、Permission、Approval、Credentialは返さない",
        EVIDENCE_SOURCE_INTERNAL_STATE,
        &super::protocol::canonical_payload_hash(Some(&body)),
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_LIST,
                "host_audit_append_failed",
                "Host一覧の結果Auditを確定できない",
            )
        }
    };
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OP_LIST.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
        audit_event_id: event.event_id,
        error: None,
        health: None,
        body: Some(body),
        shutdown_requested: broker.shutdown_requested,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_payload(host_id: &str) -> Value {
        json!({
            "版": 1,
            "操作": "登録",
            "Host ID": host_id,
            "表示名": "開発Host",
            "Platform": "windows",
            "接続状態": "pending_review",
            "Trust": "pending_review",
            "証明書/identity": {"種別": "certificate_hash", "hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
            "Runtime summary": {"runtime_count": 0, "agent_count": 0, "evidence_source": "INTERNAL_STATE"},
            "最終接続": null
        })
    }

    #[test]
    fn Host登録payloadは権限fieldとlive状態を拒否する() {
        let mut authority = valid_payload("host-authority");
        authority["permission_id"] = Value::String("permission.other-host".to_string());
        assert!(parse_request(&authority).is_err());
        let mut live = valid_payload("host-live");
        live["接続状態"] = Value::String("connected".to_string());
        assert!(parse_request(&live).is_err());
    }

    #[test]
    fn owner登録と通常一覧はHost間でmetadataだけを保持する() {
        let store =
            std::env::temp_dir().join(format!("gui-shell-host-center-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&store);
        let mut broker = Broker::new_persistent("host-center-test", &store).expect("永続Broker");
        let first = register(
            &mut broker,
            "host-register-a",
            &valid_payload("host-a"),
            true,
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        assert_eq!(first.status, BrokerStatus::Accepted);
        let second = register(
            &mut broker,
            "host-register-b",
            &valid_payload("host-b"),
            true,
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        assert_eq!(second.status, BrokerStatus::Accepted);
        let listed = list(
            &mut broker,
            "host-list",
            &json!({"版": 1}),
            false,
            "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        );
        assert_eq!(listed.status, BrokerStatus::Accepted);
        assert_eq!(listed.body.as_ref().unwrap()["件数"], 2);
        assert!(!listed
            .body
            .as_ref()
            .unwrap()
            .to_string()
            .contains("permission_id"));
        let owner_list = list(
            &mut broker,
            "host-owner-list",
            &json!({"版": 1}),
            true,
            "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        );
        assert_eq!(owner_list.status, BrokerStatus::Rejected);
        let persisted = broker
            .state_store
            .load_host_state()
            .expect("Host state読取")
            .expect("永続Host state");
        let persisted_text = persisted.to_string();
        assert!(persisted_text.contains("certificate_hash"));
        assert!(!persisted_text.contains("certificate-value"));
        drop(broker);
        let restarted =
            Broker::new_persistent("host-center-restarted", &store).expect("Host state復元");
        assert_eq!(restarted.hosts.len(), 2);
        let _ = std::fs::remove_dir_all(store);
    }

    #[test]
    fn malformedなHost_stateと過去のverified状態は再起動時にfail_closedになる() {
        let store =
            std::env::temp_dir().join(format!("gui-shell-host-malformed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&store);
        let broker = Broker::new_persistent("host-malformed-test", &store).expect("永続Broker");
        let mut receipt = valid_payload("host-invalid");
        receipt["接続状態"] = Value::String("connected".to_string());
        broker
            .state_store
            .write_host_state(&json!({"版": 1, "hosts": [receipt]}))
            .expect("malformed state書込");
        drop(broker);
        let error = Broker::new_persistent("host-malformed-restart", &store)
            .expect_err("malformed Host stateを拒否");
        assert!(matches!(error, BrokerStoreError::MalformedHostState(_)));
        let _ = std::fs::remove_dir_all(store);
    }
}
