//! C10 運用プロファイルのBroker経路。
//!
//! Profileは再利用可能な要求設定だけを保持する。Profileの内容、適用要求、
//! 永続化状態はPermission、Approval、Authority、Credentialを生成または変更しない。
#![allow(non_snake_case)]

use super::protocol::{
    Broker, BrokerOperation, BrokerResponse, BrokerStatus, EVIDENCE_SOURCE_INTERNAL_STATE,
};
use super::store::{BrokerPersistentStore, BrokerStoreError};
use crate::audit_hash::sha256_tagged;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const VERSION: u64 = 1;
const MAX_PROFILES: usize = 128;
const OP_CREATE: &str = "プロファイル作成";
const OP_COPY: &str = "プロファイル複製";
const OP_APPLY: &str = "プロファイル適用要求";
const OP_DELETE: &str = "プロファイル削除";
const OP_EXPORT: &str = "プロファイルexport";
const OP_IMPORT: &str = "プロファイルimport";
const OP_LIST: &str = "プロファイル一覧";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProfileDocument {
    #[serde(rename = "版")]
    pub(super) version: u64,
    #[serde(rename = "ProfileID")]
    pub(super) profile_id: String,
    #[serde(rename = "表示名")]
    pub(super) display_name: String,
    #[serde(rename = "Runtime")]
    pub(super) runtime: String,
    #[serde(rename = "Adapter")]
    pub(super) adapter: String,
    #[serde(rename = "要求Capability")]
    pub(super) requested_capabilities: Vec<String>,
    #[serde(rename = "Content Exposure")]
    pub(super) content_exposure: ContentExposure,
    #[serde(rename = "network exposure")]
    pub(super) network_exposure: String,
    #[serde(rename = "resource limit")]
    pub(super) resource_limit: ResourceLimit,
    #[serde(rename = "UI preference")]
    pub(super) ui_preference: UiPreference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ContentExposure {
    #[serde(rename = "visibility")]
    visibility: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ResourceLimit {
    #[serde(rename = "cpu_millis")]
    cpu_millis: u64,
    #[serde(rename = "memory_bytes")]
    memory_bytes: u64,
    #[serde(rename = "max_processes")]
    max_processes: u32,
    #[serde(rename = "max_output_bytes")]
    max_output_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UiPreference {
    #[serde(rename = "theme")]
    theme: String,
    #[serde(rename = "density")]
    density: String,
    #[serde(rename = "locale")]
    locale: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CopyRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "source_ProfileID")]
    source_profile_id: String,
    #[serde(rename = "ProfileID")]
    profile_id: String,
    #[serde(rename = "表示名")]
    display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileIDRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "ProfileID")]
    profile_id: String,
    #[serde(rename = "profile_hash")]
    profile_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplyRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "ProfileID")]
    profile_id: String,
    #[serde(rename = "profile_hash")]
    profile_hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "Profile")]
    profile: ProfileDocument,
}

pub(super) fn load_persistent_profiles(
    store: &BrokerPersistentStore,
) -> Result<BTreeMap<String, Value>, BrokerStoreError> {
    let state = store.load_profile_state()?;
    decode_state(&state).map_err(BrokerStoreError::MalformedProfileState)
}

fn decode_state(state: &Value) -> Result<BTreeMap<String, Value>, String> {
    let object = state
        .as_object()
        .ok_or_else(|| "profile stateはobjectでなければならない".to_string())?;
    if object.get("版") != Some(&Value::from(VERSION)) {
        return Err("profile stateの版が不正".to_string());
    }
    if object.keys().any(|key| key != "版" && key != "profiles") {
        return Err("profile stateに未知fieldがある".to_string());
    }
    let profiles = object
        .get("profiles")
        .and_then(Value::as_array)
        .ok_or_else(|| "profile stateのprofilesがarrayではない".to_string())?;
    if profiles.len() > MAX_PROFILES {
        return Err("profile stateの件数上限を超過".to_string());
    }
    let mut result = BTreeMap::new();
    for value in profiles {
        let profile = parse_profile(value)?;
        if result
            .insert(profile.profile_id.clone(), value.clone())
            .is_some()
        {
            return Err("profile stateに重複ProfileIDがある".to_string());
        }
    }
    Ok(result)
}

fn state_value(profiles: &BTreeMap<String, Value>) -> Value {
    json!({"版": VERSION, "profiles": profiles.values().cloned().collect::<Vec<_>>()})
}

fn persist(broker: &Broker) -> Result<(), String> {
    broker
        .state_store
        .write_profile_state(&state_value(&broker.profiles))
        .map_err(|error| error.message())
}

pub(super) fn dispatch(
    broker: &mut Broker,
    operation: BrokerOperation,
    payload: &Value,
    _owner: bool,
    request_id: &str,
    payload_hash: &str,
) -> BrokerResponse {
    let operation_name = operation.as_str();
    match operation_name {
        OP_CREATE => create(broker, payload, request_id, payload_hash),
        OP_COPY => copy(broker, payload, request_id, payload_hash),
        OP_APPLY => apply(broker, payload, request_id, payload_hash),
        OP_DELETE => delete(broker, payload, request_id, payload_hash),
        OP_EXPORT => export(broker, payload, request_id, payload_hash),
        OP_IMPORT => import(broker, payload, request_id, payload_hash),
        OP_LIST => list(broker, payload, request_id, payload_hash),
        _ => broker.reject_with_payload_hash(
            request_id,
            operation_name,
            "profile_operation_unknown",
            "Profile operationが未定義",
            true,
            payload_hash,
        ),
    }
}

fn create(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    let profile = match parse_profile(payload) {
        Ok(profile) => profile,
        Err(reason) => {
            return reject(
                broker,
                request_id,
                OP_CREATE,
                "profile_invalid",
                &reason,
                hash,
            )
        }
    };
    if broker.profiles.len() >= MAX_PROFILES {
        return reject(
            broker,
            request_id,
            OP_CREATE,
            "profile_limit_exceeded",
            "Profile件数上限を超過",
            hash,
        );
    }
    if broker.profiles.contains_key(&profile.profile_id) {
        return reject(
            broker,
            request_id,
            OP_CREATE,
            "profile_duplicate",
            "同じProfileIDは作成できない",
            hash,
        );
    }
    mutate_and_receipt(broker, OP_CREATE, profile, request_id, hash, "created")
}

fn copy(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    let request: CopyRequest = match serde_json::from_value::<CopyRequest>(payload.clone()) {
        Ok(request) if request.version == VERSION => request,
        _ => {
            return reject(
                broker,
                request_id,
                OP_COPY,
                "profile_copy_invalid",
                "Profile複製要求の構造が不正",
                hash,
            )
        }
    };
    if broker.profiles.contains_key(&request.profile_id) {
        return reject(
            broker,
            request_id,
            OP_COPY,
            "profile_duplicate",
            "同じProfileIDは複製できない",
            hash,
        );
    }
    let source = match broker.profiles.get(&request.source_profile_id) {
        Some(value) => value.clone(),
        None => {
            return reject(
                broker,
                request_id,
                OP_COPY,
                "profile_not_found",
                "複製元Profileがない",
                hash,
            )
        }
    };
    let mut profile: ProfileDocument = match serde_json::from_value(source) {
        Ok(profile) => profile,
        Err(_) => {
            return reject(
                broker,
                request_id,
                OP_COPY,
                "profile_state_invalid",
                "複製元Profileの状態が不正",
                hash,
            )
        }
    };
    profile.profile_id = request.profile_id;
    profile.display_name = request.display_name;
    mutate_and_receipt(broker, OP_COPY, profile, request_id, hash, "copied")
}

fn apply(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    let request: ApplyRequest = match serde_json::from_value::<ApplyRequest>(payload.clone()) {
        Ok(request) if request.version == VERSION && valid_hash(&request.profile_hash) => request,
        _ => {
            return reject(
                broker,
                request_id,
                OP_APPLY,
                "profile_apply_invalid",
                "Profile適用要求の構造が不正",
                hash,
            )
        }
    };
    let profile = match broker.profiles.get(&request.profile_id) {
        Some(profile) => profile,
        None => {
            return reject(
                broker,
                request_id,
                OP_APPLY,
                "profile_not_found",
                "適用対象Profileがない",
                hash,
            )
        }
    };
    let actual_hash = crate::broker::protocol::canonical_payload_hash(Some(profile));
    if actual_hash != request.profile_hash {
        return reject(
            broker,
            request_id,
            OP_APPLY,
            "profile_stale",
            "Profileが更新されているため適用要求を拒否",
            hash,
        );
    }
    let body = receipt_body(
        OP_APPLY,
        profile.clone(),
        "apply_requested",
        &actual_hash,
        None,
    );
    accepted(
        broker,
        OP_APPLY,
        request_id,
        body,
        hash,
        "Profile適用要求を記録。PermissionとAuthorityは変更しない",
    )
}

fn delete(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    let request: ProfileIDRequest =
        match serde_json::from_value::<ProfileIDRequest>(payload.clone()) {
            Ok(request) if request.version == VERSION => request,
            _ => {
                return reject(
                    broker,
                    request_id,
                    OP_DELETE,
                    "profile_delete_invalid",
                    "Profile削除要求の構造が不正",
                    hash,
                )
            }
        };
    let current = match broker.profiles.get(&request.profile_id) {
        Some(profile) => profile.clone(),
        None => {
            return reject(
                broker,
                request_id,
                OP_DELETE,
                "profile_not_found",
                "削除対象Profileがない",
                hash,
            )
        }
    };
    if let Some(expected) = request.profile_hash.as_deref() {
        if !valid_hash(expected)
            || crate::broker::protocol::canonical_payload_hash(Some(&current)) != expected
        {
            return reject(
                broker,
                request_id,
                OP_DELETE,
                "profile_stale",
                "Profile hashが一致しない",
                hash,
            );
        }
    }
    broker.profiles.remove(&request.profile_id);
    if let Err(reason) = persist(broker) {
        broker.profiles.insert(request.profile_id.clone(), current);
        return broker.audit_store_failed_response(
            request_id,
            OP_DELETE,
            "profile_persistence_failed",
            &reason,
        );
    }
    let body = receipt_body(
        OP_DELETE,
        Value::Null,
        "deleted",
        "sha256:deleted",
        Some(request.profile_id),
    );
    accepted(
        broker,
        OP_DELETE,
        request_id,
        body,
        hash,
        "Profileを削除。PermissionとAuthorityは変更しない",
    )
}

fn export(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    let request: ProfileIDRequest =
        match serde_json::from_value::<ProfileIDRequest>(payload.clone()) {
            Ok(request) if request.version == VERSION => request,
            _ => {
                return reject(
                    broker,
                    request_id,
                    OP_EXPORT,
                    "profile_export_invalid",
                    "Profile export要求の構造が不正",
                    hash,
                )
            }
        };
    let profile = match broker.profiles.get(&request.profile_id) {
        Some(profile) => profile.clone(),
        None => {
            return reject(
                broker,
                request_id,
                OP_EXPORT,
                "profile_not_found",
                "export対象Profileがない",
                hash,
            )
        }
    };
    let profile_hash = crate::broker::protocol::canonical_payload_hash(Some(&profile));
    let body = receipt_body(OP_EXPORT, profile, "exported", &profile_hash, None);
    accepted(
        broker,
        OP_EXPORT,
        request_id,
        body,
        hash,
        "Profile設定だけをexport。CredentialとAuthorityは含めない",
    )
}

fn import(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    let request: ImportRequest = match serde_json::from_value::<ImportRequest>(payload.clone()) {
        Ok(request) if request.version == VERSION => request,
        _ => {
            return reject(
                broker,
                request_id,
                OP_IMPORT,
                "profile_import_invalid",
                "Profile import要求の構造が不正",
                hash,
            )
        }
    };
    let profile = match validate_profile(request.profile) {
        Ok(profile) => profile,
        Err(reason) => {
            return reject(
                broker,
                request_id,
                OP_IMPORT,
                "profile_invalid",
                &reason,
                hash,
            )
        }
    };
    if broker.profiles.len() >= MAX_PROFILES || broker.profiles.contains_key(&profile.profile_id) {
        return reject(
            broker,
            request_id,
            OP_IMPORT,
            "profile_duplicate",
            "import先ProfileIDが使用済みまたは件数上限",
            hash,
        );
    }
    mutate_and_receipt(broker, OP_IMPORT, profile, request_id, hash, "imported")
}

fn list(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    if payload != &json!({"版": VERSION}) {
        return reject(
            broker,
            request_id,
            OP_LIST,
            "profile_list_invalid",
            "Profile一覧は版だけを受け付ける",
            hash,
        );
    }
    let profiles: Vec<Value> = broker.profiles.values().cloned().collect();
    let body = json!({
        "版": VERSION,
        "Profiles": profiles,
        "件数": broker.profiles.len(),
        "公開範囲": "configuration_only",
        "権限生成": "なし",
        "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
    });
    accepted(
        broker,
        OP_LIST,
        request_id,
        body,
        hash,
        "Profile設定一覧を返却。PermissionとAuthorityは返却・生成しない",
    )
}

fn mutate_and_receipt(
    broker: &mut Broker,
    operation: &str,
    profile: ProfileDocument,
    request_id: &str,
    hash: &str,
    state: &str,
) -> BrokerResponse {
    let profile_id = profile.profile_id.clone();
    let value = match serde_json::to_value(&profile) {
        Ok(value) => value,
        Err(_) => {
            return reject(
                broker,
                request_id,
                operation,
                "profile_serialize_failed",
                "Profileをserializeできない",
                hash,
            )
        }
    };
    if broker
        .append_audit(
            request_id,
            operation,
            "received",
            "Profile設定を受信。権限fieldとCredential実値は受け付けない",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            operation,
            "profile_audit_append_failed",
            "Profile受信Auditを確定できない",
        );
    }
    broker.profiles.insert(profile_id.clone(), value.clone());
    if let Err(reason) = persist(broker) {
        broker.profiles.remove(&profile_id);
        return broker.audit_store_failed_response(
            request_id,
            operation,
            "profile_persistence_failed",
            &reason,
        );
    }
    let profile_hash = crate::broker::protocol::canonical_payload_hash(Some(&value));
    let body = receipt_body(operation, value, state, &profile_hash, None);
    accepted(
        broker,
        operation,
        request_id,
        body,
        hash,
        "Profile設定を確定。PermissionとAuthorityは変更しない",
    )
}

fn receipt_body(
    operation: &str,
    profile: Value,
    state: &str,
    profile_hash: &str,
    id: Option<String>,
) -> Value {
    let profile_id = id.or_else(|| {
        profile
            .get("ProfileID")
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    json!({
        "版": VERSION,
        "操作": operation,
        "ProfileID": profile_id.unwrap_or_default(),
        "Profile": profile,
        "状態": state,
        "profile_hash": profile_hash,
        "権限生成": "なし",
        "公開範囲": "configuration_only",
        "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        "復旧ID": "recover-profile-state",
    })
}

fn accepted(
    broker: &mut Broker,
    operation: &str,
    request_id: &str,
    body: Value,
    hash: &str,
    reason: &str,
) -> BrokerResponse {
    let event = match broker.append_audit(
        request_id,
        operation,
        "accepted",
        reason,
        EVIDENCE_SOURCE_INTERNAL_STATE,
        &sha256_tagged(body.to_string().as_bytes()),
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                operation,
                "profile_audit_append_failed",
                "Profile結果Auditを確定できない",
            )
        }
    };
    let _ = hash;
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
    reason: &str,
    hash: &str,
) -> BrokerResponse {
    broker.reject_with_payload_hash(request_id, operation, code, reason, true, hash)
}

fn parse_profile(value: &Value) -> Result<ProfileDocument, String> {
    let profile: ProfileDocument = serde_json::from_value(value.clone())
        .map_err(|_| "Profileの構造が不正または禁止fieldがある".to_string())?;
    validate_profile(profile)
}

fn validate_profile(profile: ProfileDocument) -> Result<ProfileDocument, String> {
    if profile.version != VERSION
        || !valid_identifier(&profile.profile_id)
        || profile.display_name.trim().is_empty()
        || profile.display_name.chars().count() > 128
    {
        return Err("ProfileID、版、表示名が不正".to_string());
    }
    if profile.runtime.trim().is_empty()
        || profile.runtime.chars().count() > 128
        || profile.adapter.trim().is_empty()
        || profile.adapter.chars().count() > 128
    {
        return Err("RuntimeまたはAdapterが不正".to_string());
    }
    if profile.requested_capabilities.len() > 64
        || profile
            .requested_capabilities
            .iter()
            .any(|value| value.trim().is_empty() || value.chars().count() > 128)
    {
        return Err("要求Capabilityが不正".to_string());
    }
    if !["none", "hash_only", "summary", "redacted", "full"]
        .contains(&profile.content_exposure.visibility.as_str())
    {
        return Err("Content Exposureのvisibilityが不正".to_string());
    }
    if !["none", "loopback", "outbound"].contains(&profile.network_exposure.as_str()) {
        return Err("network exposureが不正".to_string());
    }
    if profile.resource_limit.cpu_millis > 1_000_000
        || profile.resource_limit.memory_bytes > 1 << 40
        || profile.resource_limit.max_processes > 4096
        || profile.resource_limit.max_output_bytes > 1 << 34
    {
        return Err("resource limitが上限を超過".to_string());
    }
    if !["system", "light", "dark"].contains(&profile.ui_preference.theme.as_str())
        || !["compact", "comfortable"].contains(&profile.ui_preference.density.as_str())
        || profile.ui_preference.locale.is_empty()
        || profile.ui_preference.locale.chars().count() > 16
    {
        return Err("UI preferenceが不正".to_string());
    }
    Ok(profile)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn valid_hash(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::protocol::{Broker, BrokerOperation, BrokerRequestEnvelope, BrokerStatus};
    use serde_json::json;

    fn profile(id: &str) -> Value {
        json!({
            "版": 1, "ProfileID": id, "表示名": "開発用", "Runtime": "runtime.local", "Adapter": "adapter.local",
            "要求Capability": ["filesystem.read"],
            "Content Exposure": {"visibility": "redacted"}, "network exposure": "loopback",
            "resource limit": {"cpu_millis": 1000, "memory_bytes": 1048576, "max_processes": 2, "max_output_bytes": 1048576},
            "UI preference": {"theme": "system", "density": "comfortable", "locale": "ja-JP"}
        })
    }

    fn call(
        broker: &mut Broker,
        operation: BrokerOperation,
        payload: Value,
    ) -> super::super::protocol::BrokerResponse {
        let mut request = BrokerRequestEnvelope::command_envelope_at(
            "profile-test",
            "session-1",
            &format!("nonce-{}", broker.audit_events().len()),
            &BrokerRequestEnvelope::current_issued_at(),
        );
        request.operation = Some(operation);
        request.payload = Some(payload);
        request.refresh_payload_hash();
        broker.handle(request)
    }

    #[test]
    fn profile_lifecycle_does_not_create_permission() {
        let mut broker = Broker::new("session-1");
        assert_eq!(
            call(
                &mut broker,
                BrokerOperation::プロファイル作成,
                profile("dev")
            )
            .status,
            BrokerStatus::Accepted
        );
        let created = broker.profiles.get("dev").cloned().unwrap();
        let hash = crate::broker::protocol::canonical_payload_hash(Some(&created));
        let applied = call(
            &mut broker,
            BrokerOperation::プロファイル適用要求,
            json!({"版": 1, "ProfileID": "dev", "profile_hash": hash}),
        );
        assert_eq!(applied.status, BrokerStatus::Accepted);
        assert_eq!(applied.body.unwrap()["権限生成"], "なし");
        assert_eq!(broker.profiles.len(), 1);
    }

    #[test]
    fn profile_rejects_authority_and_credential_fields() {
        let mut payload = profile("bad");
        payload
            .as_object_mut()
            .unwrap()
            .insert("Permission".into(), json!("grant"));
        assert_eq!(
            call(
                &mut Broker::new("session-1"),
                BrokerOperation::プロファイル作成,
                payload
            )
            .status,
            BrokerStatus::Rejected
        );
    }

    #[test]
    fn profile_persists_and_reloads() {
        let root = std::env::temp_dir().join(format!("gui-shell-profile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut broker = Broker::new_persistent("session-1", &root).unwrap();
        assert_eq!(
            call(
                &mut broker,
                BrokerOperation::プロファイル作成,
                profile("persistent")
            )
            .status,
            BrokerStatus::Accepted
        );
        let recovered = Broker::new_persistent("session-2", &root).unwrap();
        assert!(recovered.profiles.contains_key("persistent"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
