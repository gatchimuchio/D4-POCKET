//! C11 更新センターのBroker経路。
//!
//! 更新候補の表示・署名検査・延期・適用要求を扱う。候補に含まれる公開鍵、
//! metadata、Profile、履歴は信頼源にならない。署名鍵はBroker所有の永続設定
//! だけから読み、download / install / process / rollbackの実行は現段階では
//! 常にsuspendedとする。
#![allow(non_snake_case)]

use super::protocol::{
    Broker, BrokerError, BrokerOperation, BrokerResponse, BrokerStatus,
    EVIDENCE_SOURCE_INTERNAL_STATE,
};
use super::store::{BrokerPersistentStore, BrokerStoreError};
use crate::audit_hash::sha256_tagged;
use crate::update_verification::{verify_signed_update_signature, SignedUpdateCandidate};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const VERSION: u64 = 1;
const CANDIDATE_VERSION: u64 = 2;
const MAX_UPDATES: usize = 64;
const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_PACKAGE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const OP_LIST: &str = "更新一覧";
const OP_CHECK: &str = "更新確認";
const OP_VERIFY: &str = "更新署名検査";
const OP_DOWNLOAD: &str = "更新download要求";
const OP_APPLY: &str = "更新適用要求";
const OP_DEFER: &str = "更新延期";
const OP_ROLLBACK: &str = "更新rollback要求";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateCandidateDocument {
    #[serde(rename = "版")]
    pub(super) version: u64,
    #[serde(rename = "更新ID")]
    pub(super) update_id: String,
    #[serde(rename = "現行版")]
    pub(super) current_version: String,
    #[serde(rename = "提供版")]
    pub(super) offered_version: String,
    #[serde(rename = "channel")]
    pub(super) channel: String,
    #[serde(rename = "内容概要")]
    pub(super) summary: String,
    #[serde(
        rename = "package_sha256",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub(super) package_sha256: Option<String>,
    #[serde(
        rename = "package_size_bytes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub(super) package_size_bytes: Option<u64>,
    #[serde(rename = "署名対象")]
    pub(super) signed_bytes_hex: String,
    #[serde(rename = "署名")]
    pub(super) signature_hex: String,
    #[serde(rename = "署名者fingerprint")]
    pub(super) signer_fingerprint: String,
    #[serde(rename = "rollback可能")]
    pub(super) rollback_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateRecord {
    #[serde(flatten)]
    pub(super) candidate: UpdateCandidateDocument,
    #[serde(rename = "署名状態")]
    pub(super) signature_status: String,
    #[serde(rename = "候補hash")]
    pub(super) candidate_hash: String,
    #[serde(rename = "延期期限")]
    pub(super) deferred_until: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateTrust {
    #[serde(rename = "版")]
    pub(super) version: u64,
    pub(super) algorithm: String,
    pub(super) public_key_der_hex: String,
    pub(super) public_key_fingerprint: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "候補")]
    candidate: UpdateCandidateDocument,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "候補")]
    candidates: Vec<UpdateCandidateDocument>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateIDRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "更新ID")]
    update_id: String,
    #[serde(rename = "候補hash")]
    candidate_hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeferRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "更新ID")]
    update_id: String,
    #[serde(rename = "候補hash")]
    candidate_hash: String,
    #[serde(rename = "延期期限")]
    deferred_until: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SignedManifest<'a> {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "更新ID")]
    update_id: &'a str,
    #[serde(rename = "現行版")]
    current_version: &'a str,
    #[serde(rename = "提供版")]
    offered_version: &'a str,
    #[serde(rename = "channel")]
    channel: &'a str,
    #[serde(rename = "内容概要")]
    summary: &'a str,
    #[serde(rename = "package_sha256")]
    package_sha256: &'a str,
    #[serde(rename = "package_size_bytes")]
    package_size_bytes: u64,
    #[serde(rename = "rollback可能")]
    rollback_available: bool,
}

pub(super) fn load_persistent_updates(
    store: &BrokerPersistentStore,
) -> Result<BTreeMap<String, Value>, BrokerStoreError> {
    let state = store.load_update_state()?;
    decode_state(&state).map_err(BrokerStoreError::MalformedUpdateState)
}

pub(super) fn load_persistent_trust(
    store: &BrokerPersistentStore,
) -> Result<Option<UpdateTrust>, BrokerStoreError> {
    let Some(value) = store.load_update_trust()? else {
        return Ok(None);
    };
    let trust: UpdateTrust = serde_json::from_value(value).map_err(|_| {
        BrokerStoreError::MalformedUpdateTrust(
            "broker update trustのfieldまたは版が不正である".to_string(),
        )
    })?;
    validate_trust(&trust).map_err(BrokerStoreError::MalformedUpdateTrust)?;
    Ok(Some(trust))
}

fn decode_state(state: &Value) -> Result<BTreeMap<String, Value>, String> {
    let object = state
        .as_object()
        .ok_or_else(|| "update stateはobjectでなければならない".to_string())?;
    if object.keys().any(|key| key != "版" && key != "updates") {
        return Err("update stateに未知fieldがある".to_string());
    }
    if object.get("版") != Some(&Value::from(VERSION)) {
        return Err("update stateの版が不正".to_string());
    }
    let values = object
        .get("updates")
        .and_then(Value::as_array)
        .ok_or_else(|| "update stateのupdatesがarrayではない".to_string())?;
    if values.len() > MAX_UPDATES {
        return Err("update stateの件数上限を超過".to_string());
    }
    let mut result = BTreeMap::new();
    for value in values {
        let record: UpdateRecord = serde_json::from_value(value.clone())
            .map_err(|_| "update stateのrecord構造が不正".to_string())?;
        validate_record(&record)?;
        if result
            .insert(record.candidate.update_id.clone(), value.clone())
            .is_some()
        {
            return Err("update stateに重複更新IDがある".to_string());
        }
    }
    Ok(result)
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
        OP_CHECK => check(broker, payload, request_id, payload_hash),
        OP_VERIFY => verify(broker, payload, request_id, payload_hash),
        OP_DOWNLOAD => execution_request(broker, OP_DOWNLOAD, payload, request_id, payload_hash),
        OP_APPLY => execution_request(broker, OP_APPLY, payload, request_id, payload_hash),
        OP_DEFER => defer(broker, payload, request_id, payload_hash),
        OP_ROLLBACK => execution_request(broker, OP_ROLLBACK, payload, request_id, payload_hash),
        _ => broker.reject_with_payload_hash(
            request_id,
            operation.as_str(),
            "update_operation_unknown",
            "更新操作が未定義",
            true,
            payload_hash,
        ),
    }
}

fn list(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    if payload != &json!({"版": VERSION}) {
        return reject(
            broker,
            request_id,
            OP_LIST,
            "update_list_invalid",
            "更新一覧は版だけを受け付ける",
            hash,
        );
    }
    let updates: Vec<Value> = broker.updates.values().map(project_update_record).collect();
    accepted(
        broker,
        OP_LIST,
        request_id,
        json!({
            "版": VERSION,
            "更新一覧": updates,
            "件数": broker.updates.len(),
            "署名信頼設定": if broker.update_trust.is_some() { "configured" } else { "unconfigured" },
            "download実行": "suspended",
            "適用実行": "suspended",
            "rollback実行": "suspended",
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        }),
        hash,
        "更新候補一覧を返却。実行系操作はsuspended",
    )
}

fn project_update_record(value: &Value) -> Value {
    let mut projected = value.clone();
    if projected.get("版").and_then(Value::as_u64) == Some(1) {
        if let Some(object) = projected.as_object_mut() {
            object.insert("署名状態".into(), Value::String("legacy_unbound".into()));
        }
    }
    projected
}

fn verify(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    let request: CandidateRequest =
        match serde_json::from_value::<CandidateRequest>(payload.clone()) {
            Ok(request) if request.version == VERSION => request,
            _ => {
                return reject(
                    broker,
                    request_id,
                    OP_VERIFY,
                    "update_candidate_invalid",
                    "更新候補の構造が不正",
                    hash,
                )
            }
        };
    let record = match verify_candidate(broker, request.candidate) {
        Ok(record) => record,
        Err((code, message)) => return reject(broker, request_id, OP_VERIFY, code, message, hash),
    };
    accepted(
        broker,
        OP_VERIFY,
        request_id,
        serde_json::to_value(record).unwrap_or(Value::Null),
        hash,
        "更新候補のEd25519署名を検査",
    )
}

fn check(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    let request: CheckRequest = match serde_json::from_value::<CheckRequest>(payload.clone()) {
        Ok(request) if request.version == VERSION && !request.candidates.is_empty() => request,
        _ => {
            return reject(
                broker,
                request_id,
                OP_CHECK,
                "update_check_invalid",
                "更新確認の構造が不正",
                hash,
            )
        }
    };
    if request.candidates.len() > MAX_UPDATES {
        return reject(
            broker,
            request_id,
            OP_CHECK,
            "update_limit_exceeded",
            "更新候補件数上限を超過",
            hash,
        );
    }
    let mut records = Vec::with_capacity(request.candidates.len());
    for candidate in request.candidates {
        match verify_candidate(broker, candidate) {
            Ok(record) => records.push(record),
            Err((code, message)) => {
                return reject(broker, request_id, OP_CHECK, code, message, hash)
            }
        }
    }
    let previous = broker.updates.clone();
    for record in &records {
        let value = match serde_json::to_value(record) {
            Ok(value) => value,
            Err(_) => {
                return reject(
                    broker,
                    request_id,
                    OP_CHECK,
                    "update_serialize_failed",
                    "更新候補を保存形式へ変換できない",
                    hash,
                )
            }
        };
        broker
            .updates
            .insert(record.candidate.update_id.clone(), value);
    }
    if broker.updates.len() > MAX_UPDATES {
        broker.updates = previous;
        return reject(
            broker,
            request_id,
            OP_CHECK,
            "update_limit_exceeded",
            "更新候補件数上限を超過",
            hash,
        );
    }
    if let Err(reason) = persist(broker) {
        broker.updates = previous;
        return broker.audit_store_failed_response(
            request_id,
            OP_CHECK,
            "update_persistence_failed",
            &reason,
        );
    }
    accepted(
        broker,
        OP_CHECK,
        request_id,
        json!({"版": VERSION, "更新一覧": records, "件数": records.len(), "署名状態": "verified", "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE}),
        hash,
        "更新候補を検査し永続化。未署名・未信頼候補は保存しない",
    )
}

fn defer(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    let request: DeferRequest = match serde_json::from_value::<DeferRequest>(payload.clone()) {
        Ok(request) if request.version == VERSION && !request.deferred_until.trim().is_empty() => {
            request
        }
        _ => {
            return reject(
                broker,
                request_id,
                OP_DEFER,
                "update_defer_invalid",
                "更新延期の構造が不正",
                hash,
            )
        }
    };
    let Some(current) = broker.updates.get(&request.update_id).cloned() else {
        return reject(
            broker,
            request_id,
            OP_DEFER,
            "update_not_found",
            "延期対象の更新がない",
            hash,
        );
    };
    let mut record: UpdateRecord = match serde_json::from_value(current.clone()) {
        Ok(record) => record,
        Err(_) => {
            return reject(
                broker,
                request_id,
                OP_DEFER,
                "update_state_invalid",
                "更新状態が不正",
                hash,
            )
        }
    };
    if record.candidate_hash != request.candidate_hash {
        return reject(
            broker,
            request_id,
            OP_DEFER,
            "update_stale",
            "更新候補hashが一致しない",
            hash,
        );
    }
    record.deferred_until = Some(request.deferred_until);
    let previous = broker.updates.insert(
        request.update_id.clone(),
        serde_json::to_value(&record).unwrap_or(Value::Null),
    );
    if let Err(reason) = persist(broker) {
        if let Some(previous) = previous {
            broker.updates.insert(request.update_id, previous);
        }
        return broker.audit_store_failed_response(
            request_id,
            OP_DEFER,
            "update_persistence_failed",
            &reason,
        );
    }
    accepted(
        broker,
        OP_DEFER,
        request_id,
        project_update_record(&serde_json::to_value(record).unwrap_or(Value::Null)),
        hash,
        "更新延期を永続化。インストールは実行しない",
    )
}

fn execution_request(
    broker: &mut Broker,
    operation: &str,
    payload: &Value,
    request_id: &str,
    hash: &str,
) -> BrokerResponse {
    let request: UpdateIDRequest = match serde_json::from_value::<UpdateIDRequest>(payload.clone())
    {
        Ok(request) if request.version == VERSION && valid_hash(&request.candidate_hash) => request,
        _ => {
            return reject(
                broker,
                request_id,
                operation,
                "update_request_invalid",
                "更新実行要求の構造が不正",
                hash,
            )
        }
    };
    let Some(value) = broker.updates.get(&request.update_id) else {
        return reject(
            broker,
            request_id,
            operation,
            "update_not_found",
            "対象の更新がない",
            hash,
        );
    };
    let record: UpdateRecord = match serde_json::from_value(value.clone()) {
        Ok(record) => record,
        Err(_) => {
            return reject(
                broker,
                request_id,
                operation,
                "update_state_invalid",
                "更新状態が不正",
                hash,
            )
        }
    };
    if record.candidate_hash != request.candidate_hash {
        return reject(
            broker,
            request_id,
            operation,
            "update_stale",
            "更新候補hashが一致しない",
            hash,
        );
    }
    if record.signature_status != "verified" {
        return reject(
            broker,
            request_id,
            operation,
            "update_signature_required",
            "署名検査済みの更新だけを要求できる",
            hash,
        );
    }
    if record.candidate.version != CANDIDATE_VERSION {
        return reject(
            broker,
            request_id,
            operation,
            "update_package_binding_required",
            "配布packageのhashとbyte長へ署名が結合していない旧候補は実行できない",
            hash,
        );
    }
    suspended(
        broker,
        operation,
        request_id,
        json!({"版": VERSION, "更新ID": request.update_id, "候補hash": request.candidate_hash, "実行状態": "suspended", "復旧ID": "recover-update-execution", "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE}),
        hash,
        "外部download、install、process、rollback実行経路は未接続のためsuspended",
    )
}

fn verify_candidate(
    broker: &Broker,
    candidate: UpdateCandidateDocument,
) -> Result<UpdateRecord, (&'static str, &'static str)> {
    match candidate.version {
        1 => {
            return Err((
                "update_package_binding_required",
                "新しい更新候補には配布packageのhashとbyte長を署名へ含める必要がある",
            ));
        }
        CANDIDATE_VERSION => {}
        _ => return Err(("update_candidate_invalid", "更新候補の版が未対応")),
    }
    validate_candidate(&candidate)
        .map_err(|_| ("update_candidate_invalid", "更新候補の値が不正"))?;
    let Some(trust) = broker.update_trust.as_ref() else {
        return Err((
            "update_trust_unconfigured",
            "Broker所有の更新署名信頼設定が未構成",
        ));
    };
    let public_key_der = hex::decode(&trust.public_key_der_hex)
        .map_err(|_| ("update_trusted_key_invalid", "更新公開鍵のhexが不正"))?;
    let signed_bytes = hex::decode(&candidate.signed_bytes_hex)
        .map_err(|_| ("update_candidate_invalid", "署名対象hexが不正"))?;
    let signature = hex::decode(&candidate.signature_hex)
        .map_err(|_| ("update_candidate_invalid", "署名hexが不正"))?;
    let signed_manifest = SignedManifest {
        version: candidate.version,
        update_id: &candidate.update_id,
        current_version: &candidate.current_version,
        offered_version: &candidate.offered_version,
        channel: &candidate.channel,
        summary: &candidate.summary,
        package_sha256: candidate
            .package_sha256
            .as_deref()
            .ok_or(("update_candidate_invalid", "package SHA-256が必要"))?,
        package_size_bytes: candidate
            .package_size_bytes
            .ok_or(("update_candidate_invalid", "package byte長が必要"))?,
        rollback_available: candidate.rollback_available,
    };
    let canonical_bytes = serde_json::to_vec(&signed_manifest)
        .map_err(|_| ("update_candidate_invalid", "署名対象の正本化に失敗"))?;
    if signed_bytes != canonical_bytes || signed_bytes.len() > MAX_MANIFEST_BYTES {
        return Err((
            "update_signed_manifest_mismatch",
            "署名対象が候補metadataの正本byteと一致しない",
        ));
    }
    let response = verify_signed_update_signature(
        &SignedUpdateCandidate {
            update_id: candidate.update_id.clone(),
            signed_bytes,
            signature: Some(signature),
            signer_fingerprint: candidate.signer_fingerprint.clone(),
        },
        &public_key_der,
        &trust.public_key_fingerprint,
    );
    if !response.ok {
        let error = response
            .error
            .ok_or(("update_signature_invalid", "更新署名検査に失敗"))?;
        return Err(match error.code.as_str() {
            "update_trusted_key_invalid" => ("update_trusted_key_invalid", "更新公開鍵が不正"),
            "update_signer_untrusted" => (
                "update_signer_untrusted",
                "更新署名者が信頼設定と一致しない",
            ),
            "update_signature_required" => ("update_signature_required", "更新署名が必要"),
            _ => ("update_signature_invalid", "更新署名検査に失敗"),
        });
    }
    let candidate_value = serde_json::to_value(&candidate)
        .map_err(|_| ("update_candidate_invalid", "候補hashの計算に失敗"))?;
    Ok(UpdateRecord {
        candidate,
        signature_status: "verified".to_string(),
        candidate_hash: crate::broker::protocol::canonical_payload_hash(Some(&candidate_value)),
        deferred_until: None,
    })
}

fn persist(broker: &Broker) -> Result<(), String> {
    broker
        .state_store
        .write_update_state(&json!({"版": VERSION, "updates": broker.updates.values().cloned().collect::<Vec<_>>() }))
        .map_err(|error| error.message())
}

fn validate_candidate(candidate: &UpdateCandidateDocument) -> Result<(), ()> {
    let package_binding_valid = match candidate.version {
        1 => candidate.package_sha256.is_none() && candidate.package_size_bytes.is_none(),
        CANDIDATE_VERSION => {
            candidate
                .package_sha256
                .as_deref()
                .is_some_and(valid_package_sha256)
                && candidate
                    .package_size_bytes
                    .is_some_and(|size| (1..=MAX_PACKAGE_BYTES).contains(&size))
        }
        _ => false,
    };
    if !package_binding_valid
        || !valid_identifier(&candidate.update_id)
        || candidate.current_version.trim().is_empty()
        || candidate.offered_version.trim().is_empty()
        || candidate.current_version.len() > 128
        || candidate.offered_version.len() > 128
        || !["stable", "beta", "nightly"].contains(&candidate.channel.as_str())
        || candidate.summary.trim().is_empty()
        || candidate.summary.chars().count() > 1024
        || !valid_hex(&candidate.signed_bytes_hex)
        || candidate.signed_bytes_hex.len() > MAX_MANIFEST_BYTES * 2
        || !valid_hex(&candidate.signature_hex)
        || candidate.signature_hex.len() != 128
        || !valid_hash(&candidate.signer_fingerprint)
    {
        return Err(());
    }
    Ok(())
}

fn valid_package_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn validate_record(record: &UpdateRecord) -> Result<(), String> {
    validate_candidate(&record.candidate).map_err(|_| "update recordの候補が不正".to_string())?;
    if record.signature_status != "verified" || !valid_hash(&record.candidate_hash) {
        return Err("update recordの署名状態または候補hashが不正".to_string());
    }
    if record.deferred_until.as_deref().is_some_and(str::is_empty) {
        return Err("update recordの延期期限が空である".to_string());
    }
    Ok(())
}

fn validate_trust(trust: &UpdateTrust) -> Result<(), String> {
    let key =
        hex::decode(&trust.public_key_der_hex).map_err(|_| "更新公開鍵hexが不正".to_string())?;
    let public_key =
        crate::checkpoint::public_key(&key).map_err(|_| "更新公開鍵DERが不正".to_string())?;
    if trust.version != VERSION
        || trust.algorithm != "Ed25519"
        || trust.public_key_fingerprint != sha256_tagged(public_key)
        || !valid_hash(&trust.public_key_fingerprint)
    {
        return Err("更新署名信頼設定が不正".to_string());
    }
    Ok(())
}

pub(super) fn verify_bytes_with_trust(
    trust: Option<&UpdateTrust>,
    update_id: &str,
    signed_bytes_hex: &str,
    signature_hex: &str,
    signer_fingerprint: &str,
) -> Result<(), &'static str> {
    let Some(trust) = trust else {
        return Err("broker所有の署名信頼設定が未構成");
    };
    let public_key_der =
        hex::decode(&trust.public_key_der_hex).map_err(|_| "broker所有の署名公開鍵が不正")?;
    let signed_bytes = hex::decode(signed_bytes_hex).map_err(|_| "署名対象hexが不正")?;
    let signature = hex::decode(signature_hex).map_err(|_| "署名hexが不正")?;
    let result = verify_signed_update_signature(
        &SignedUpdateCandidate {
            update_id: update_id.to_string(),
            signed_bytes,
            signature: Some(signature),
            signer_fingerprint: signer_fingerprint.to_string(),
        },
        &public_key_der,
        &trust.public_key_fingerprint,
    );
    if result.ok {
        Ok(())
    } else {
        Err("Adapter署名の検証に失敗")
    }
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
                "update_audit_append_failed",
                "更新操作の監査を確定できない",
            )
        }
    };
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
                "update_audit_append_failed",
                "更新操作の監査を確定できない",
            )
        }
    };
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: operation.to_string(),
        status: BrokerStatus::Suspended,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
        audit_event_id: event.event_id,
        error: Some(BrokerError {
            code: "update_execution_suspended".to_string(),
            message: reason.to_string(),
            recoverable: true,
            audit_event_required: true,
            fail_closed: true,
        }),
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

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn valid_hex(value: &str) -> bool {
    !value.is_empty()
        && value.len().is_multiple_of(2)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_hash(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::protocol::{Broker, BrokerOperation, BrokerRequestEnvelope, BrokerStatus};
    use ring::{
        rand::SystemRandom,
        signature::{Ed25519KeyPair, KeyPair},
    };

    fn trust_and_candidate() -> (UpdateTrust, UpdateCandidateDocument) {
        let key = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        let pair = Ed25519KeyPair::from_pkcs8(key.as_ref()).unwrap();
        let der = [
            0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
        ]
        .into_iter()
        .chain(pair.public_key().as_ref().iter().copied())
        .collect::<Vec<_>>();
        let signed = serde_json::to_vec(&SignedManifest {
            version: CANDIDATE_VERSION,
            update_id: "update-1",
            current_version: "1.0.0",
            offered_version: "1.1.0",
            channel: "stable",
            summary: "安全更新",
            package_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            package_size_bytes: 1024,
            rollback_available: true,
        })
        .unwrap();
        let fingerprint = sha256_tagged(pair.public_key().as_ref());
        let signature = pair.sign(&signed);
        (
            UpdateTrust {
                version: 1,
                algorithm: "Ed25519".into(),
                public_key_der_hex: hex::encode(der),
                public_key_fingerprint: fingerprint.clone(),
            },
            UpdateCandidateDocument {
                version: CANDIDATE_VERSION,
                update_id: "update-1".into(),
                current_version: "1.0.0".into(),
                offered_version: "1.1.0".into(),
                channel: "stable".into(),
                summary: "安全更新".into(),
                package_sha256: Some(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                ),
                package_size_bytes: Some(1024),
                signed_bytes_hex: hex::encode(signed),
                signature_hex: hex::encode(signature.as_ref()),
                signer_fingerprint: fingerprint,
                rollback_available: true,
            },
        )
    }

    fn call(broker: &mut Broker, operation: BrokerOperation, payload: Value) -> BrokerResponse {
        let mut request = BrokerRequestEnvelope::command_envelope_at(
            "update-test",
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
    fn update_candidate_requires_broker_owned_trust() {
        let (_, candidate) = trust_and_candidate();
        let mut broker = Broker::new("session-1");
        let response = call(
            &mut broker,
            BrokerOperation::更新確認,
            json!({"版": 1, "候補": [candidate]}),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "update_trust_unconfigured");
    }

    #[test]
    fn verified_candidate_is_persisted_only_after_signature_check() {
        let (trust, candidate) = trust_and_candidate();
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        let response = call(
            &mut broker,
            BrokerOperation::更新確認,
            json!({"版": 1, "候補": [candidate]}),
        );
        assert_eq!(response.status, BrokerStatus::Accepted);
        assert_eq!(broker.updates.len(), 1);
        let update = broker.updates.values().next().unwrap();
        assert_eq!(update["署名状態"], "verified");
        assert_eq!(
            update["package_sha256"],
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(update["package_size_bytes"], 1024);
    }

    #[test]
    fn update_signature_binds_package_digest_and_exact_size() {
        let (trust, mut candidate) = trust_and_candidate();
        candidate.package_sha256 =
            Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into());
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        let response = call(
            &mut broker,
            BrokerOperation::更新確認,
            json!({"版": 1, "候補": [candidate]}),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "update_signed_manifest_mismatch"
        );
        assert!(broker.updates.is_empty());

        let (_, mut candidate) = trust_and_candidate();
        candidate.package_size_bytes = Some(MAX_PACKAGE_BYTES + 1);
        assert!(validate_candidate(&candidate).is_err());

        let (_, mut candidate) = trust_and_candidate();
        candidate.package_sha256 = Some("A".repeat(64));
        assert!(validate_candidate(&candidate).is_err());

        let (trust, mut candidate) = trust_and_candidate();
        candidate.package_size_bytes = Some(1025);
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        let response = call(
            &mut broker,
            BrokerOperation::更新確認,
            json!({"版": 1, "候補": [candidate]}),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "update_signed_manifest_mismatch"
        );
        assert!(broker.updates.is_empty());
    }

    #[test]
    fn legacy_candidate_cannot_be_accepted_as_a_new_update() {
        let (trust, mut candidate) = trust_and_candidate();
        candidate.version = 1;
        candidate.package_sha256 = None;
        candidate.package_size_bytes = None;
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        let response = call(
            &mut broker,
            BrokerOperation::更新確認,
            json!({"版": 1, "候補": [candidate]}),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "update_package_binding_required"
        );
        assert!(broker.updates.is_empty());
    }

    #[test]
    fn legacy_unbound_candidate_is_preserved_but_not_actionable() {
        let (_, mut candidate) = trust_and_candidate();
        candidate.version = 1;
        candidate.package_sha256 = None;
        candidate.package_size_bytes = None;
        let record = UpdateRecord {
            candidate,
            signature_status: "verified".into(),
            candidate_hash:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            deferred_until: None,
        };
        let record_value = serde_json::to_value(record).unwrap();
        let decoded = decode_state(&json!({"版": 1, "updates": [record_value.clone()]}));
        assert!(decoded.is_ok(), "旧状態はBroker起動を妨げず保持できる");

        let mut broker = Broker::new("session-1");
        broker.updates.insert("update-1".into(), record_value);
        let listing = call(&mut broker, BrokerOperation::更新一覧, json!({"版": 1}));
        assert_eq!(
            listing.body.unwrap()["更新一覧"][0]["署名状態"],
            "legacy_unbound"
        );
        let deferred = call(
            &mut broker,
            BrokerOperation::更新延期,
            json!({
                "版": 1,
                "更新ID": "update-1",
                "候補hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "延期期限": "2030-01-01T00:00:00Z"
            }),
        );
        assert_eq!(deferred.status, BrokerStatus::Accepted);
        assert_eq!(deferred.body.unwrap()["署名状態"], "legacy_unbound");
        let response = call(
            &mut broker,
            BrokerOperation::更新適用要求,
            json!({
                "版": 1,
                "更新ID": "update-1",
                "候補hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "update_package_binding_required"
        );
    }

    #[test]
    fn execution_requests_remain_suspended_and_audited() {
        let (trust, candidate) = trust_and_candidate();
        let candidate_value = serde_json::to_value(&candidate).unwrap();
        let candidate_hash =
            crate::broker::protocol::canonical_payload_hash(Some(&candidate_value));
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        assert_eq!(
            call(
                &mut broker,
                BrokerOperation::更新確認,
                json!({"版": 1, "候補": [candidate]})
            )
            .status,
            BrokerStatus::Accepted
        );
        let response = call(
            &mut broker,
            BrokerOperation::更新適用要求,
            json!({"版": 1, "更新ID": "update-1", "候補hash": candidate_hash}),
        );
        assert_eq!(response.status, BrokerStatus::Suspended);
        assert_eq!(response.error.unwrap().code, "update_execution_suspended");
    }

    #[test]
    fn verified_update_state_reloads_without_promoting_trust() {
        let (trust, candidate) = trust_and_candidate();
        let root =
            std::env::temp_dir().join(format!("gui-shell-update-center-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        {
            let mut broker = Broker::new_persistent("session-1", &root).unwrap();
            broker.update_trust = Some(trust);
            assert_eq!(
                call(
                    &mut broker,
                    BrokerOperation::更新確認,
                    json!({"版": 1, "候補": [candidate]})
                )
                .status,
                BrokerStatus::Accepted
            );
        }
        let restarted = Broker::new_persistent("session-1", &root).unwrap();
        assert_eq!(restarted.updates.len(), 1);
        assert!(restarted.update_trust.is_none());
        assert!(root.join("updates.json").exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
