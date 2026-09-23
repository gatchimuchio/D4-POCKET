//! C19 Adapter管理のBroker経路。
//!
//! このmoduleはAdapter manifestのboundedなcatalogと状態遷移を管理する。
//! 導入・更新・削除は外部filesystemやprocessを暗黙に操作せず、owner control
//! からmanifestを登録する。通常IPCの管理要求はowner再承認待ちで停止する。
//! Adapter metadata、署名済みmanifest、過去状態からPermission、Approval、Authority
//! を生成しない。検証済みAdapterだけを有効化でき、隔離状態はRuntime操作から再利用しない。
#![allow(non_snake_case)]

use super::protocol::{
    Broker, BrokerOperation, BrokerResponse, BrokerStatus, EVIDENCE_SOURCE_INTERNAL_STATE,
};
use super::store::{BrokerPersistentStore, BrokerStoreError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const VERSION: u64 = 1;
const MAX_ADAPTERS: usize = 64;
const MAX_LIST: usize = 64;
const OP_LIST: &str = "アダプター一覧";
const OP_INSTALL: &str = "アダプター導入";
const OP_VERIFY: &str = "アダプター検証";
const OP_ENABLE: &str = "アダプター有効化";
const OP_DISABLE: &str = "アダプター無効化";
const OP_QUARANTINE: &str = "アダプター隔離";
const OP_UPDATE: &str = "アダプター更新";
const OP_REMOVE: &str = "アダプター削除";
const RECOVERY_ID: &str = "recover-adapter-management";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AdapterManifest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "Adapter ID")]
    adapter_id: String,
    #[serde(rename = "Runtime ID")]
    runtime_id: String,
    #[serde(rename = "発行者")]
    publisher: String,
    source: String,
    #[serde(rename = "version")]
    adapter_version: String,
    transport: String,
    #[serde(rename = "Content Exposure")]
    content_exposure: String,
    #[serde(rename = "要求Capability")]
    requested_capabilities: Vec<String>,
    #[serde(rename = "許可差分")]
    permission_diff: Vec<String>,
    #[serde(rename = "既知の危険")]
    known_risks: Vec<String>,
    #[serde(rename = "互換性")]
    compatibility: String,
    authority_strip: bool,
    signed_manifest: bool,
    #[serde(rename = "署名対象")]
    signed_bytes_hex: String,
    #[serde(rename = "署名")]
    signature_hex: String,
    #[serde(rename = "署名者fingerprint")]
    signer_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdapterRecord {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "マニフェスト")]
    manifest: AdapterManifest,
    hash: String,
    #[serde(rename = "署名状態")]
    signature_status: String,
    #[serde(rename = "検証状態")]
    verification_status: String,
    #[serde(rename = "管理状態")]
    management_state: String,
    #[serde(rename = "有効状態")]
    active_state: String,
    #[serde(rename = "更新可能")]
    update_available: bool,
    #[serde(rename = "最終検証")]
    last_verified: Option<String>,
    #[serde(rename = "監査ID")]
    audit_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdapterManagementRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "操作")]
    operation: String,
    #[serde(rename = "Adapter ID")]
    adapter_id: Option<String>,
    #[serde(rename = "Adapter hash")]
    adapter_hash: Option<String>,
    #[serde(rename = "Manifest")]
    manifest: Option<AdapterManifest>,
}

pub(super) fn load_persistent_adapters(
    store: &BrokerPersistentStore,
) -> Result<BTreeMap<String, Value>, BrokerStoreError> {
    let state = store.load_adapter_state()?;
    decode_state(&state).map_err(BrokerStoreError::MalformedAdapterState)
}

fn decode_state(state: &Value) -> Result<BTreeMap<String, Value>, String> {
    let object = state
        .as_object()
        .ok_or_else(|| "Adapter stateはobjectでなければならない".to_string())?;
    if object.keys().any(|key| key != "版" && key != "adapters") {
        return Err("Adapter stateに未知fieldがある".to_string());
    }
    if object.get("版") != Some(&Value::from(VERSION)) {
        return Err("Adapter stateの版が不正".to_string());
    }
    let values = object
        .get("adapters")
        .and_then(Value::as_array)
        .ok_or_else(|| "Adapter stateのadaptersがarrayではない".to_string())?;
    if values.len() > MAX_ADAPTERS {
        return Err("Adapter stateの件数上限を超過".to_string());
    }
    let mut result = BTreeMap::new();
    for value in values {
        let record: AdapterRecord = serde_json::from_value(value.clone())
            .map_err(|_| "Adapter stateのrecord構造が不正".to_string())?;
        validate_record(&record)?;
        if result
            .insert(record.manifest.adapter_id.clone(), value.clone())
            .is_some()
        {
            return Err("Adapter stateに重複Adapter IDがある".to_string());
        }
    }
    Ok(result)
}

fn state_value(adapters: &BTreeMap<String, Value>) -> Value {
    json!({"版": VERSION, "adapters": adapters.values().cloned().collect::<Vec<_>>()})
}

fn persist(broker: &Broker) -> Result<(), String> {
    broker
        .state_store
        .write_adapter_state(&state_value(&broker.adapters))
        .map_err(|error| error.message())
}

pub(super) fn dispatch(
    broker: &mut Broker,
    operation: BrokerOperation,
    payload: &Value,
    owner: bool,
    request_id: &str,
    payload_hash: &str,
) -> BrokerResponse {
    match operation.as_str() {
        OP_LIST => list(broker, payload, owner, request_id, payload_hash),
        OP_INSTALL | OP_VERIFY | OP_ENABLE | OP_DISABLE | OP_QUARANTINE | OP_UPDATE | OP_REMOVE => {
            mutate(
                broker,
                operation.as_str(),
                payload,
                owner,
                request_id,
                payload_hash,
            )
        }
        _ => broker.reject_with_payload_hash(
            request_id,
            operation.as_str(),
            "adapter_operation_unknown",
            "Adapter管理操作が未定義",
            true,
            payload_hash,
        ),
    }
}

fn list(
    broker: &mut Broker,
    payload: &Value,
    owner: bool,
    request_id: &str,
    hash: &str,
) -> BrokerResponse {
    if payload != &json!({"版": VERSION}) || broker.adapters.len() > MAX_LIST {
        return reject(
            broker,
            request_id,
            OP_LIST,
            "adapter_list_invalid",
            "Adapter一覧は版だけを受け付け、件数はboundedでなければならない",
            hash,
        );
    }
    let records = broker
        .adapters
        .values()
        .map(|value| {
            serde_json::from_value::<AdapterRecord>(value.clone())
                .map_err(|_| "Adapter一覧の内部状態が不正")
        })
        .collect::<Result<Vec<_>, _>>();
    let records = match records {
        Ok(records) => records,
        Err(reason) => {
            return reject(
                broker,
                request_id,
                OP_LIST,
                "adapter_state_invalid",
                reason,
                hash,
            )
        }
    };
    let body = json!({
        "版": VERSION,
        "Adapter一覧": records.iter().map(receipt).collect::<Vec<_>>(),
        "件数": broker.adapters.len(),
        "公開範囲": "metadata_only",
        "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        "権限生成": "なし",
        "authority_strip": true,
    });
    let reason = if owner {
        "Adapter metadata一覧をowner controlへ返却"
    } else {
        "Adapter metadata一覧を通常IPCへ返却"
    };
    accepted(broker, OP_LIST, request_id, body, hash, reason)
}

fn mutate(
    broker: &mut Broker,
    operation: &str,
    payload: &Value,
    owner: bool,
    request_id: &str,
    hash: &str,
) -> BrokerResponse {
    let request: AdapterManagementRequest =
        match serde_json::from_value::<AdapterManagementRequest>(payload.clone()) {
            Ok(request)
                if request.version == VERSION
                    && request.operation == operation_label(operation) =>
            {
                request
            }
            _ => {
                return reject(
                    broker,
                    request_id,
                    operation,
                    "adapter_request_invalid",
                    "Adapter管理要求の構造または操作名が不正",
                    hash,
                )
            }
        };
    if !owner {
        return suspended(
            broker,
            operation,
            request_id,
            json!({
                "版": VERSION,
                "操作": request.operation,
                "Adapter ID": request.adapter_id.or_else(|| request.manifest.as_ref().map(|manifest| manifest.adapter_id.clone())).unwrap_or_default(),
                "実行状態": "suspended",
                "承認状態": "owner_reapproval_required",
                "権限生成": "なし",
                "authority_strip": true,
                "公開範囲": "metadata_only",
                "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
                "復旧ID": RECOVERY_ID,
            }),
            hash,
            "Adapter管理はowner control承認なしに状態を変更しない",
        );
    }
    match operation {
        OP_INSTALL => install(broker, request.manifest, request_id, hash),
        OP_UPDATE => update(broker, request.manifest, request_id, hash),
        OP_VERIFY => verify(
            broker,
            request.adapter_id,
            request.adapter_hash,
            request_id,
            hash,
        ),
        OP_ENABLE => enable(
            broker,
            request.adapter_id,
            request.adapter_hash,
            request_id,
            hash,
        ),
        OP_DISABLE => disable(
            broker,
            request.adapter_id,
            request.adapter_hash,
            request_id,
            hash,
        ),
        OP_QUARANTINE => quarantine(
            broker,
            request.adapter_id,
            request.adapter_hash,
            request_id,
            hash,
        ),
        OP_REMOVE => remove(
            broker,
            request.adapter_id,
            request.adapter_hash,
            request_id,
            hash,
        ),
        _ => reject(
            broker,
            request_id,
            operation,
            "adapter_operation_unknown",
            "Adapter管理操作が未定義",
            hash,
        ),
    }
}

fn operation_label(operation: &str) -> &'static str {
    match operation {
        OP_INSTALL => "導入",
        OP_VERIFY => "検証",
        OP_ENABLE => "有効化",
        OP_DISABLE => "無効化",
        OP_QUARANTINE => "隔離",
        OP_UPDATE => "更新",
        OP_REMOVE => "削除",
        _ => "",
    }
}

fn install(
    broker: &mut Broker,
    manifest: Option<AdapterManifest>,
    request_id: &str,
    hash: &str,
) -> BrokerResponse {
    let Some(manifest) = manifest else {
        return reject(
            broker,
            request_id,
            OP_INSTALL,
            "adapter_manifest_missing",
            "導入にはManifestが必要",
            hash,
        );
    };
    if let Err(reason) = validate_manifest(&manifest) {
        return reject(
            broker,
            request_id,
            OP_INSTALL,
            "adapter_manifest_invalid",
            reason,
            hash,
        );
    }
    if broker.adapters.contains_key(&manifest.adapter_id) {
        return reject(
            broker,
            request_id,
            OP_INSTALL,
            "adapter_duplicate",
            "同じAdapter IDは導入できない",
            hash,
        );
    }
    let record = new_record(manifest);
    let adapter_id = record.manifest.adapter_id.clone();
    let mut record = record;
    record.audit_id = broker.audit_log.next_event_id();
    let body = receipt(&record);
    broker.adapters.insert(
        adapter_id,
        serde_json::to_value(&record).unwrap_or(Value::Null),
    );
    if let Err(reason) = persist(broker) {
        broker.adapters.remove(&record.manifest.adapter_id);
        return broker.audit_store_failed_response(
            request_id,
            OP_INSTALL,
            "adapter_persistence_failed",
            &reason,
        );
    }
    accepted(
        broker,
        OP_INSTALL,
        request_id,
        operation_receipt(body, "導入", "accepted"),
        hash,
        "Adapter manifestをcatalogへ導入登録。filesystem/processは実行しない",
    )
}

fn update(
    broker: &mut Broker,
    manifest: Option<AdapterManifest>,
    request_id: &str,
    hash: &str,
) -> BrokerResponse {
    let Some(manifest) = manifest else {
        return reject(
            broker,
            request_id,
            OP_UPDATE,
            "adapter_manifest_missing",
            "更新にはManifestが必要",
            hash,
        );
    };
    if let Err(reason) = validate_manifest(&manifest) {
        return reject(
            broker,
            request_id,
            OP_UPDATE,
            "adapter_manifest_invalid",
            reason,
            hash,
        );
    }
    let Some(previous) = broker.adapters.get(&manifest.adapter_id).cloned() else {
        return reject(
            broker,
            request_id,
            OP_UPDATE,
            "adapter_not_found",
            "更新対象Adapterがない",
            hash,
        );
    };
    let previous_record: AdapterRecord = match serde_json::from_value(previous.clone()) {
        Ok(record) => record,
        Err(_) => {
            return reject(
                broker,
                request_id,
                OP_UPDATE,
                "adapter_state_invalid",
                "Adapter状態が不正",
                hash,
            )
        }
    };
    let mut record = new_record(manifest);
    record.audit_id = broker.audit_log.next_event_id();
    let body = receipt(&record);
    broker.adapters.insert(
        record.manifest.adapter_id.clone(),
        serde_json::to_value(&record).unwrap_or(Value::Null),
    );
    if let Err(reason) = persist(broker) {
        broker
            .adapters
            .insert(previous_record.manifest.adapter_id, previous);
        return broker.audit_store_failed_response(
            request_id,
            OP_UPDATE,
            "adapter_persistence_failed",
            &reason,
        );
    }
    accepted(
        broker,
        OP_UPDATE,
        request_id,
        operation_receipt(body, "更新", "accepted"),
        hash,
        "Adapter manifestをcatalog上で更新登録。外部download/installは実行しない",
    )
}

fn verify(
    broker: &mut Broker,
    adapter_id: Option<String>,
    adapter_hash: Option<String>,
    request_id: &str,
    hash: &str,
) -> BrokerResponse {
    let Some((id, expected_hash)) = checked_id_hash(adapter_id, adapter_hash) else {
        return reject(
            broker,
            request_id,
            OP_VERIFY,
            "adapter_request_invalid",
            "検証対象IDまたはhashが不正",
            hash,
        );
    };
    let Some(value) = broker.adapters.get(&id).cloned() else {
        return reject(
            broker,
            request_id,
            OP_VERIFY,
            "adapter_not_found",
            "検証対象Adapterがない",
            hash,
        );
    };
    let mut record: AdapterRecord = match serde_json::from_value(value) {
        Ok(record) => record,
        Err(_) => {
            return reject(
                broker,
                request_id,
                OP_VERIFY,
                "adapter_state_invalid",
                "Adapter状態が不正",
                hash,
            )
        }
    };
    if record.hash != expected_hash {
        return reject(
            broker,
            request_id,
            OP_VERIFY,
            "adapter_stale",
            "Adapter hashが一致しない",
            hash,
        );
    }
    let signed_bytes = match hex::decode(&record.manifest.signed_bytes_hex) {
        Ok(bytes) => bytes,
        Err(_) => {
            return reject(
                broker,
                request_id,
                OP_VERIFY,
                "adapter_signature_invalid",
                "署名対象hexが不正",
                hash,
            )
        }
    };
    if signed_bytes != signed_payload_bytes(&record.manifest) {
        return reject(
            broker,
            request_id,
            OP_VERIFY,
            "adapter_signed_payload_mismatch",
            "署名対象がmanifestの正本byteと一致しない",
            hash,
        );
    }
    if let Err(reason) = super::update_center::verify_bytes_with_trust(
        broker.update_trust.as_ref(),
        &record.manifest.adapter_id,
        &record.manifest.signed_bytes_hex,
        &record.manifest.signature_hex,
        &record.manifest.signer_fingerprint,
    ) {
        record.signature_status = if broker.update_trust.is_some() {
            "invalid".into()
        } else {
            "unconfigured".into()
        };
        record.verification_status = "pending_review".into();
        record.audit_id = broker.audit_log.next_event_id();
        broker
            .adapters
            .insert(id, serde_json::to_value(&record).unwrap_or(Value::Null));
        if let Err(persist_reason) = persist(broker) {
            return broker.audit_store_failed_response(
                request_id,
                OP_VERIFY,
                "adapter_persistence_failed",
                &persist_reason,
            );
        }
        return reject(
            broker,
            request_id,
            OP_VERIFY,
            "adapter_signature_unverified",
            reason,
            hash,
        );
    }
    record.signature_status = "verified".into();
    record.verification_status = "verified".into();
    record.management_state = "verified".into();
    record.last_verified = Some("Broker所有Ed25519 trust".into());
    record.audit_id = broker.audit_log.next_event_id();
    let body = receipt(&record);
    broker
        .adapters
        .insert(id, serde_json::to_value(&record).unwrap_or(Value::Null));
    if let Err(reason) = persist(broker) {
        return broker.audit_store_failed_response(
            request_id,
            OP_VERIFY,
            "adapter_persistence_failed",
            &reason,
        );
    }
    accepted(
        broker,
        OP_VERIFY,
        request_id,
        operation_receipt(body, "検証", "accepted"),
        hash,
        "Broker所有trustでAdapter署名と正本byteを検証",
    )
}

fn enable(
    broker: &mut Broker,
    adapter_id: Option<String>,
    adapter_hash: Option<String>,
    request_id: &str,
    hash: &str,
) -> BrokerResponse {
    transition(
        broker,
        OP_ENABLE,
        "有効化",
        adapter_id,
        adapter_hash,
        request_id,
        hash,
        |record| {
            if record.verification_status != "verified" || record.signature_status != "verified" {
                return Err("署名検証済みAdapterだけを有効化できる");
            }
            record.management_state = "enabled".into();
            record.active_state = "active".into();
            Ok(())
        },
    )
}

fn disable(
    broker: &mut Broker,
    adapter_id: Option<String>,
    adapter_hash: Option<String>,
    request_id: &str,
    hash: &str,
) -> BrokerResponse {
    transition(
        broker,
        OP_DISABLE,
        "無効化",
        adapter_id,
        adapter_hash,
        request_id,
        hash,
        |record| {
            if record.management_state == "quarantined" {
                return Err("隔離済みAdapterは無効化操作で状態を置換できない");
            }
            record.management_state = "disabled".into();
            record.active_state = "inactive".into();
            Ok(())
        },
    )
}

fn quarantine(
    broker: &mut Broker,
    adapter_id: Option<String>,
    adapter_hash: Option<String>,
    request_id: &str,
    hash: &str,
) -> BrokerResponse {
    transition(
        broker,
        OP_QUARANTINE,
        "隔離",
        adapter_id,
        adapter_hash,
        request_id,
        hash,
        |record| {
            record.management_state = "quarantined".into();
            record.active_state = "blocked".into();
            Ok(())
        },
    )
}

fn remove(
    broker: &mut Broker,
    adapter_id: Option<String>,
    adapter_hash: Option<String>,
    request_id: &str,
    hash: &str,
) -> BrokerResponse {
    let Some((id, expected_hash)) = checked_id_hash(adapter_id, adapter_hash) else {
        return reject(
            broker,
            request_id,
            OP_REMOVE,
            "adapter_request_invalid",
            "削除対象IDまたはhashが不正",
            hash,
        );
    };
    let Some(value) = broker.adapters.get(&id).cloned() else {
        return reject(
            broker,
            request_id,
            OP_REMOVE,
            "adapter_not_found",
            "削除対象Adapterがない",
            hash,
        );
    };
    let record: AdapterRecord = match serde_json::from_value(value.clone()) {
        Ok(record) => record,
        Err(_) => {
            return reject(
                broker,
                request_id,
                OP_REMOVE,
                "adapter_state_invalid",
                "Adapter状態が不正",
                hash,
            )
        }
    };
    if record.hash != expected_hash {
        return reject(
            broker,
            request_id,
            OP_REMOVE,
            "adapter_stale",
            "Adapter hashが一致しない",
            hash,
        );
    }
    if record.management_state == "enabled" {
        return reject(
            broker,
            request_id,
            OP_REMOVE,
            "adapter_enabled",
            "有効化中のAdapterは先に無効化が必要",
            hash,
        );
    }
    broker.adapters.remove(&id);
    if let Err(reason) = persist(broker) {
        broker.adapters.insert(id, value);
        return broker.audit_store_failed_response(
            request_id,
            OP_REMOVE,
            "adapter_persistence_failed",
            &reason,
        );
    }
    accepted(
        broker,
        OP_REMOVE,
        request_id,
        json!({"版": VERSION, "操作": "削除", "Adapter ID": record.manifest.adapter_id, "実行状態": "accepted", "管理状態": "removed", "権限生成": "なし", "authority_strip": true, "公開範囲": "metadata_only", "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE, "復旧ID": RECOVERY_ID}),
        hash,
        "Adapter catalog recordを削除。外部artifactのfilesystem削除は実行しない",
    )
}

fn transition<F>(
    broker: &mut Broker,
    operation: &str,
    action: &str,
    adapter_id: Option<String>,
    adapter_hash: Option<String>,
    request_id: &str,
    hash: &str,
    change: F,
) -> BrokerResponse
where
    F: FnOnce(&mut AdapterRecord) -> Result<(), &'static str>,
{
    let Some((id, expected_hash)) = checked_id_hash(adapter_id, adapter_hash) else {
        return reject(
            broker,
            request_id,
            operation,
            "adapter_request_invalid",
            "対象Adapter IDまたはhashが不正",
            hash,
        );
    };
    let Some(value) = broker.adapters.get(&id).cloned() else {
        return reject(
            broker,
            request_id,
            operation,
            "adapter_not_found",
            "対象Adapterがない",
            hash,
        );
    };
    let mut record: AdapterRecord = match serde_json::from_value(value.clone()) {
        Ok(record) => record,
        Err(_) => {
            return reject(
                broker,
                request_id,
                operation,
                "adapter_state_invalid",
                "Adapter状態が不正",
                hash,
            )
        }
    };
    if record.hash != expected_hash {
        return reject(
            broker,
            request_id,
            operation,
            "adapter_stale",
            "Adapter hashが一致しない",
            hash,
        );
    }
    if let Err(reason) = change(&mut record) {
        return reject(
            broker,
            request_id,
            operation,
            "adapter_transition_denied",
            reason,
            hash,
        );
    }
    record.audit_id = broker.audit_log.next_event_id();
    let body = operation_receipt(receipt(&record), action, "accepted");
    broker
        .adapters
        .insert(id, serde_json::to_value(&record).unwrap_or(Value::Null));
    if let Err(reason) = persist(broker) {
        broker
            .adapters
            .insert(record.manifest.adapter_id.clone(), value);
        return broker.audit_store_failed_response(
            request_id,
            operation,
            "adapter_persistence_failed",
            &reason,
        );
    }
    accepted(
        broker,
        operation,
        request_id,
        body,
        hash,
        "Adapter管理状態をBroker内部で更新。Permission、Approval、Authorityは変更しない",
    )
}

fn checked_id_hash(
    adapter_id: Option<String>,
    adapter_hash: Option<String>,
) -> Option<(String, String)> {
    let id = adapter_id.filter(|value| valid_identifier(value))?;
    let hash = adapter_hash.filter(|value| valid_hash(value))?;
    Some((id, hash))
}

fn new_record(manifest: AdapterManifest) -> AdapterRecord {
    let manifest_value = serde_json::to_value(&manifest).unwrap_or(Value::Null);
    let hash = crate::broker::protocol::canonical_payload_hash(Some(&manifest_value));
    AdapterRecord {
        version: VERSION,
        manifest,
        hash,
        signature_status: "unverified".into(),
        verification_status: "pending_review".into(),
        management_state: "installed".into(),
        active_state: "inactive".into(),
        update_available: false,
        last_verified: None,
        audit_id: "pending".into(),
    }
}

fn receipt(record: &AdapterRecord) -> Value {
    json!({
        "版": VERSION,
        "Adapter ID": record.manifest.adapter_id,
        "Runtime ID": record.manifest.runtime_id,
        "発行者": record.manifest.publisher,
        "source": record.manifest.source,
        "version": record.manifest.adapter_version,
        "transport": record.manifest.transport,
        "Content Exposure": record.manifest.content_exposure,
        "要求Capability": record.manifest.requested_capabilities,
        "許可差分": record.manifest.permission_diff,
        "既知の危険": record.manifest.known_risks,
        "互換性": record.manifest.compatibility,
        "hash": record.hash,
        "署名状態": record.signature_status,
        "検証状態": record.verification_status,
        "管理状態": record.management_state,
        "有効状態": record.active_state,
        "更新可能": record.update_available,
        "公開範囲": "metadata_only",
        "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        "権限生成": "なし",
        "authority_strip": true,
        "最終検証": record.last_verified,
        "監査ID": record.audit_id,
        "復旧ID": RECOVERY_ID,
    })
}

fn operation_receipt(mut body: Value, action: &str, execution_state: &str) -> Value {
    if let Some(object) = body.as_object_mut() {
        object.insert("操作".into(), Value::String(action.into()));
        object.insert("実行状態".into(), Value::String(execution_state.into()));
        object.insert(
            "承認状態".into(),
            Value::String("owner_control_approved".into()),
        );
    }
    body
}

fn validate_manifest(manifest: &AdapterManifest) -> Result<(), &'static str> {
    if manifest.version != VERSION
        || !valid_identifier(&manifest.adapter_id)
        || !valid_identifier(&manifest.runtime_id)
        || manifest.publisher.trim().is_empty()
        || manifest.publisher.chars().count() > 256
        || !matches!(
            manifest.source.as_str(),
            "owner_manifest" | "local_bundle" | "builtin"
        )
        || manifest.adapter_version.trim().is_empty()
        || manifest.adapter_version.chars().count() > 128
        || !matches!(
            manifest.transport.as_str(),
            "http" | "stdio" | "ipc" | "mock"
        )
        || !matches!(
            manifest.content_exposure.as_str(),
            "none" | "hash_only" | "summary" | "redacted" | "full"
        )
        || manifest.requested_capabilities.len() > 64
        || manifest.permission_diff.len() > 64
        || manifest.known_risks.len() > 64
        || !matches!(
            manifest.compatibility.as_str(),
            "compatible" | "incompatible" | "unknown"
        )
        || !manifest.authority_strip
        || !manifest.signed_manifest
        || !valid_hex(&manifest.signed_bytes_hex)
        || manifest.signed_bytes_hex.len() > 131072
        || !valid_hex(&manifest.signature_hex)
        || manifest.signature_hex.len() != 128
        || !valid_hash(&manifest.signer_fingerprint)
        || manifest
            .requested_capabilities
            .iter()
            .any(|value| value.trim().is_empty())
        || manifest
            .permission_diff
            .iter()
            .any(|value| value.trim().is_empty())
        || manifest
            .known_risks
            .iter()
            .any(|value| value.trim().is_empty())
    {
        return Err("Adapter manifestの値またはauthority境界が不正");
    }
    Ok(())
}

fn validate_record(record: &AdapterRecord) -> Result<(), String> {
    validate_manifest(&record.manifest).map_err(str::to_string)?;
    if record.version != VERSION
        || !valid_hash(&record.hash)
        || !matches!(
            record.signature_status.as_str(),
            "verified" | "unconfigured" | "invalid" | "unverified"
        )
        || !matches!(
            record.verification_status.as_str(),
            "verified" | "pending_review" | "rejected" | "unverified"
        )
        || !matches!(
            record.management_state.as_str(),
            "installed" | "verified" | "enabled" | "disabled" | "quarantined"
        )
        || !matches!(
            record.active_state.as_str(),
            "inactive" | "active" | "blocked"
        )
        || record.audit_id.trim().is_empty()
    {
        return Err("Adapter recordの状態が不正".to_string());
    }
    Ok(())
}

fn signed_payload_bytes(manifest: &AdapterManifest) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "版": manifest.version,
        "Adapter ID": manifest.adapter_id,
        "Runtime ID": manifest.runtime_id,
        "発行者": manifest.publisher,
        "source": manifest.source,
        "version": manifest.adapter_version,
        "transport": manifest.transport,
        "Content Exposure": manifest.content_exposure,
        "要求Capability": manifest.requested_capabilities,
        "許可差分": manifest.permission_diff,
        "既知の危険": manifest.known_risks,
        "互換性": manifest.compatibility,
        "authority_strip": true,
        "signed_manifest": true,
    }))
    .unwrap_or_default()
}

fn valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && chars.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
        })
        && value.chars().count() <= 128
}

fn valid_hash(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && valid_hex(hex))
}

fn valid_hex(value: &str) -> bool {
    !value.is_empty()
        && value.len() % 2 == 0
        && value
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
}

pub(super) fn runtime_is_quarantined(broker: &Broker, runtime_id: &str) -> bool {
    broker.adapters.values().any(|value| {
        serde_json::from_value::<AdapterRecord>(value.clone())
            .ok()
            .is_some_and(|record| {
                record.manifest.runtime_id == runtime_id && record.management_state == "quarantined"
            })
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
        hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                operation,
                "adapter_audit_append_failed",
                "Adapter管理の監査を確定できない",
            )
        }
    };
    BrokerResponse {
        request_id: request_id.into(),
        operation: operation.into(),
        status: BrokerStatus::Accepted,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.into(),
        audit_event_id: event.event_id,
        error: None,
        health: None,
        body: Some(body),
        shutdown_requested: broker.shutdown_requested,
    }
}

fn suspended(
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
        "suspended",
        reason,
        EVIDENCE_SOURCE_INTERNAL_STATE,
        hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                operation,
                "adapter_audit_append_failed",
                "Adapter管理の監査を確定できない",
            )
        }
    };
    BrokerResponse {
        request_id: request_id.into(),
        operation: operation.into(),
        status: BrokerStatus::Suspended,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.into(),
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
    use crate::audit_hash::sha256_tagged;
    use ring::{
        rand::SystemRandom,
        signature::{Ed25519KeyPair, KeyPair},
    };

    fn hash() -> &'static str {
        "sha256:0000000000000000000000000000000000000000000000000000000000000000"
    }

    fn manifest() -> AdapterManifest {
        let mut manifest = AdapterManifest {
            version: VERSION,
            adapter_id: "fixture_adapter".into(),
            runtime_id: "fixture_runtime".into(),
            publisher: "fixture".into(),
            source: "owner_manifest".into(),
            adapter_version: "1.0.0".into(),
            transport: "mock".into(),
            content_exposure: "redacted".into(),
            requested_capabilities: vec!["runtime.read".into()],
            permission_diff: vec!["none".into()],
            known_risks: vec!["fixture".into()],
            compatibility: "compatible".into(),
            authority_strip: true,
            signed_manifest: true,
            signed_bytes_hex: String::new(),
            signature_hex: "00".repeat(64),
            signer_fingerprint: "sha256:".to_string() + &"0".repeat(64),
        };
        manifest.signed_bytes_hex = hex::encode(signed_payload_bytes(&manifest));
        manifest
    }

    fn response(
        broker: &mut Broker,
        operation: BrokerOperation,
        payload: Value,
        owner: bool,
    ) -> BrokerResponse {
        dispatch(broker, operation, &payload, owner, "adapter-test", hash())
    }

    #[test]
    fn normal_channel_cannot_mutate_adapter_catalog() {
        let mut broker = Broker::new("adapter-session");
        let payload = json!({"版": VERSION, "操作": "導入", "Manifest": manifest()});
        let result = response(&mut broker, BrokerOperation::アダプター導入, payload, false);
        assert_eq!(result.status, BrokerStatus::Suspended);
        assert!(broker.adapters.is_empty());
    }

    #[test]
    fn owner_install_is_metadata_only_and_requires_verification_before_enable() {
        let mut broker = Broker::new("adapter-session");
        let install = response(
            &mut broker,
            BrokerOperation::アダプター導入,
            json!({"版": VERSION, "操作": "導入", "Manifest": manifest()}),
            true,
        );
        assert_eq!(install.status, BrokerStatus::Accepted);
        let record = broker.adapters.get("fixture_adapter").unwrap();
        assert_eq!(record["管理状態"], "installed");
        assert_eq!(record["マニフェスト"]["authority_strip"], true);
        let receipt = install.body.unwrap();
        assert_eq!(receipt["管理状態"], "installed");
        assert_eq!(receipt["authority_strip"], true);
        assert_eq!(receipt["権限生成"], "なし");
        let hash_value = serde_json::from_value::<AdapterRecord>(record.clone())
            .unwrap()
            .hash;
        let enabled = response(
            &mut broker,
            BrokerOperation::アダプター有効化,
            json!({"版": VERSION, "操作": "有効化", "Adapter ID": "fixture_adapter", "Adapter hash": hash_value}),
            true,
        );
        assert_eq!(enabled.status, BrokerStatus::Rejected);
    }

    #[test]
    fn quarantine_blocks_runtime_reuse_and_remove_requires_disabled_state() {
        let mut broker = Broker::new("adapter-session");
        response(
            &mut broker,
            BrokerOperation::アダプター導入,
            json!({"版": VERSION, "操作": "導入", "Manifest": manifest()}),
            true,
        );
        let hash_value = serde_json::from_value::<AdapterRecord>(
            broker.adapters.get("fixture_adapter").unwrap().clone(),
        )
        .unwrap()
        .hash;
        let quarantine = response(
            &mut broker,
            BrokerOperation::アダプター隔離,
            json!({"版": VERSION, "操作": "隔離", "Adapter ID": "fixture_adapter", "Adapter hash": hash_value}),
            true,
        );
        assert_eq!(quarantine.status, BrokerStatus::Accepted);
        assert!(runtime_is_quarantined(&broker, "fixture_runtime"));
        let current_hash = serde_json::from_value::<AdapterRecord>(
            broker.adapters.get("fixture_adapter").unwrap().clone(),
        )
        .unwrap()
        .hash;
        let remove = response(
            &mut broker,
            BrokerOperation::アダプター削除,
            json!({"版": VERSION, "操作": "削除", "Adapter ID": "fixture_adapter", "Adapter hash": current_hash}),
            true,
        );
        assert_eq!(remove.status, BrokerStatus::Accepted);
        assert!(!runtime_is_quarantined(&broker, "fixture_runtime"));
    }

    #[test]
    fn broker_owned_signature_verification_is_required_before_enable() {
        let key = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        let pair = Ed25519KeyPair::from_pkcs8(key.as_ref()).unwrap();
        let public_key_der = [
            0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
        ]
        .into_iter()
        .chain(pair.public_key().as_ref().iter().copied())
        .collect::<Vec<_>>();
        let fingerprint = sha256_tagged(pair.public_key().as_ref());
        let mut signed = manifest();
        signed.signed_bytes_hex = hex::encode(signed_payload_bytes(&signed));
        signed.signature_hex = hex::encode(
            pair.sign(&hex::decode(&signed.signed_bytes_hex).unwrap())
                .as_ref(),
        );
        signed.signer_fingerprint = fingerprint.clone();
        let mut broker = Broker::new("adapter-signature-session");
        broker.update_trust = Some(super::super::update_center::UpdateTrust {
            version: VERSION,
            algorithm: "Ed25519".into(),
            public_key_der_hex: hex::encode(public_key_der),
            public_key_fingerprint: fingerprint,
        });
        let installed = response(
            &mut broker,
            BrokerOperation::アダプター導入,
            json!({"版": VERSION, "操作": "導入", "Manifest": signed}),
            true,
        );
        assert_eq!(installed.status, BrokerStatus::Accepted);
        let hash_value = serde_json::from_value::<AdapterRecord>(
            broker.adapters.get("fixture_adapter").unwrap().clone(),
        )
        .unwrap()
        .hash;
        let verified = response(
            &mut broker,
            BrokerOperation::アダプター検証,
            json!({"版": VERSION, "操作": "検証", "Adapter ID": "fixture_adapter", "Adapter hash": hash_value}),
            true,
        );
        assert_eq!(verified.status, BrokerStatus::Accepted);
        let hash_value = serde_json::from_value::<AdapterRecord>(
            broker.adapters.get("fixture_adapter").unwrap().clone(),
        )
        .unwrap()
        .hash;
        let enabled = response(
            &mut broker,
            BrokerOperation::アダプター有効化,
            json!({"版": VERSION, "操作": "有効化", "Adapter ID": "fixture_adapter", "Adapter hash": hash_value}),
            true,
        );
        assert_eq!(enabled.status, BrokerStatus::Accepted);
        assert_eq!(enabled.body.unwrap()["管理状態"], "enabled");
    }

    #[test]
    fn persistent_adapter_catalog_reloads_after_broker_restart() {
        let root = std::env::temp_dir().join(format!(
            "gui-shell-adapter-state-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut broker = Broker::new_persistent("adapter-persistent-session", &root).unwrap();
        let installed = response(
            &mut broker,
            BrokerOperation::アダプター導入,
            json!({"版": VERSION, "操作": "導入", "Manifest": manifest()}),
            true,
        );
        assert_eq!(installed.status, BrokerStatus::Accepted);
        drop(broker);
        let restarted = Broker::new_persistent("adapter-persistent-restart", &root).unwrap();
        assert!(restarted.adapters.contains_key("fixture_adapter"));
        let _ = std::fs::remove_dir_all(root);
    }
}
