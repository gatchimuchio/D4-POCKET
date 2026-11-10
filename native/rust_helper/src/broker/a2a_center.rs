//! C16 A2A接続センターのBroker経路。
//!
//! Agent Cardをowner controlから取得し、metadata-onlyの接続receiptへ射影する。
//! A2AのTask送信、Message送信、Artifact本文、Stream購読、Credential注入は未接続とする。
#![allow(non_snake_case)]

use super::protocol::{Broker, BrokerResponse, BrokerStatus, EVIDENCE_SOURCE_INTERNAL_STATE};
use super::store::{BrokerPersistentStore, BrokerStoreError};
use crate::a2a::{fetch_agent_card, A2aError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const OP_CONNECT: &str = "A2A接続";
const OP_LIST: &str = "A2A接続一覧";
const VERSION: u64 = 1;
const PROTOCOL_VERSION: &str = "1.0";
const MAX_CONNECTIONS: usize = 64;

pub(super) fn load_persistent_connections(
    store: &BrokerPersistentStore,
) -> Result<BTreeMap<String, Value>, BrokerStoreError> {
    let state = store.load_a2a_state()?;
    decode_state(&state).map_err(BrokerStoreError::MalformedA2aState)
}

fn decode_state(state: &Value) -> Result<BTreeMap<String, Value>, String> {
    let object = state
        .as_object()
        .ok_or_else(|| "A2A接続stateはobjectでなければならない".to_string())?;
    if object.keys().any(|key| key != "版" && key != "connections") {
        return Err("A2A接続stateに未知fieldがある".to_string());
    }
    if object.get("版") != Some(&Value::from(VERSION)) {
        return Err("A2A接続stateの版が不正".to_string());
    }
    let values = object
        .get("connections")
        .and_then(Value::as_array)
        .ok_or_else(|| "A2A接続stateのconnectionsがarrayではない".to_string())?;
    if values.len() > MAX_CONNECTIONS {
        return Err("A2A接続stateの件数上限を超過".to_string());
    }
    let mut result = BTreeMap::new();
    for value in values {
        let mut restored = normalize_legacy_receipt(value)?;
        let agent_id = validate_persisted_receipt(&restored)?;
        let restored_object = restored
            .as_object_mut()
            .ok_or_else(|| "A2A接続receiptがobjectではない".to_string())?;
        restored_object.insert(
            "接続状態".to_string(),
            Value::String("restored_pending_review".to_string()),
        );
        restored_object.insert(
            "証拠種別".to_string(),
            Value::String(EVIDENCE_SOURCE_INTERNAL_STATE.to_string()),
        );
        restored_object.insert(
            "承認状態".to_string(),
            Value::String("owner_reapproval_required".to_string()),
        );
        if result.insert(agent_id, restored).is_some() {
            return Err("A2A接続stateに重複AgentIDがある".to_string());
        }
    }
    Ok(result)
}

fn normalize_legacy_receipt(value: &Value) -> Result<Value, String> {
    let mut normalized = value.clone();
    let Some(interfaces) = normalized
        .get_mut("Agent Card")
        .and_then(Value::as_object_mut)
        .and_then(|card| card.get_mut("supported_interfaces"))
        .and_then(Value::as_array_mut)
    else {
        return Ok(normalized);
    };
    for interface in interfaces {
        let Some(object) = interface.as_object_mut() else {
            continue;
        };
        let legacy_keys = ["binding", "transport", "status", "reason"];
        let is_legacy = object.len() == legacy_keys.len()
            && object.keys().all(|key| legacy_keys.contains(&key.as_str()));
        if !is_legacy {
            continue;
        }
        let valid = object
            .get("binding")
            .and_then(Value::as_str)
            .is_some_and(|value| ["jsonrpc", "grpc", "http_rest", "unknown"].contains(&value))
            && object
                .get("transport")
                .and_then(Value::as_str)
                .is_some_and(|value| ["http", "https", "unknown"].contains(&value))
            && object
                .get("status")
                .and_then(Value::as_str)
                .is_some_and(|value| ["supported", "unsupported", "unknown"].contains(&value))
            && object
                .get("reason")
                .and_then(Value::as_str)
                .is_some_and(|value| {
                    !value.is_empty()
                        && value.chars().count() <= 512
                        && !value.chars().any(char::is_control)
                });
        if !valid {
            return Err("旧版A2A interface metadataが不正".to_string());
        }
        object.insert(
            "protocol_version".to_string(),
            Value::String("unknown".to_string()),
        );
        object.insert("interface_hash".to_string(), Value::Null);
        object.insert("status".to_string(), Value::String("unknown".to_string()));
        object.insert(
            "reason".to_string(),
            Value::String(
                "旧版保存metadataにはinterface URLがないため、endpoint identityと実行状態はunknown"
                    .to_string(),
            ),
        );
    }
    Ok(normalized)
}

fn state_value(connections: &BTreeMap<String, Value>) -> Value {
    json!({
        "版": VERSION,
        "connections": connections.values().cloned().collect::<Vec<_>>(),
    })
}

fn persist(broker: &Broker) -> Result<(), String> {
    broker
        .state_store
        .write_a2a_state(&state_value(&broker.a2a_connections))
        .map_err(|error| error.message())
}

fn is_restored_pending_review(receipt: &Value) -> bool {
    receipt.get("接続状態").and_then(Value::as_str) == Some("restored_pending_review")
        && receipt.get("証拠種別").and_then(Value::as_str) == Some(EVIDENCE_SOURCE_INTERNAL_STATE)
        && receipt.get("承認状態").and_then(Value::as_str) == Some("owner_reapproval_required")
}

fn validate_persisted_receipt(value: &Value) -> Result<String, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "A2A接続receiptはobjectでなければならない".to_string())?;
    const KEYS: [&str; 21] = [
        "版",
        "契約種別",
        "protocol_version",
        "AgentID",
        "Agent Card",
        "Task",
        "Message",
        "Artifact",
        "Stream",
        "Trust",
        "Capability diff",
        "権限生成",
        "authority_strip",
        "公開範囲",
        "証拠種別",
        "接続状態",
        "能力ID",
        "権限ID",
        "承認状態",
        "復旧ID",
        "接続監査ID",
    ];
    if object.len() != KEYS.len() || object.keys().any(|key| !KEYS.contains(&key.as_str())) {
        return Err("A2A接続receiptに未知または欠落fieldがある".to_string());
    }
    if object.get("版") != Some(&Value::from(VERSION))
        || object.get("契約種別") != Some(&Value::from("A2A外部概念射影"))
        || object.get("protocol_version") != Some(&Value::from(PROTOCOL_VERSION))
        || object.get("権限生成") != Some(&Value::from("なし"))
        || object.get("authority_strip") != Some(&Value::Bool(true))
        || object.get("公開範囲") != Some(&Value::from("metadata_only"))
        || object.get("能力ID") != Some(&Value::from("a2a.connection.connect"))
        || object.get("権限ID") != Some(&Value::from("permission.a2a.connection.connect"))
        || object.get("復旧ID") != Some(&Value::from("recover-a2a-connection"))
    {
        return Err("A2A接続receiptの固定fieldが不正".to_string());
    }
    let agent_id = object
        .get("AgentID")
        .and_then(Value::as_str)
        .filter(|value| valid_agent_id(value))
        .ok_or_else(|| "A2A接続receiptのAgentIDが不正".to_string())?
        .to_string();
    let evidence = object
        .get("証拠種別")
        .and_then(Value::as_str)
        .ok_or_else(|| "A2A接続receiptの証拠種別が不正".to_string())?;
    let connection_state = object
        .get("接続状態")
        .and_then(Value::as_str)
        .ok_or_else(|| "A2A接続receiptの接続状態が不正".to_string())?;
    let approval = object
        .get("承認状態")
        .and_then(Value::as_str)
        .ok_or_else(|| "A2A接続receiptの承認状態が不正".to_string())?;
    if !matches!(
        (evidence, connection_state, approval),
        ("LIVE_RUNTIME", "connected", "owner_control_approved")
            | (
                "INTERNAL_STATE",
                "restored_pending_review",
                "owner_reapproval_required"
            )
    ) {
        return Err("A2A接続receiptの証拠・状態・承認の組合せが不正".to_string());
    }
    let trust = object
        .get("Trust")
        .and_then(Value::as_object)
        .ok_or_else(|| "A2A Trustがobjectではない".to_string())?;
    if trust.len() != 4
        || trust.keys().any(|key| {
            ![
                "state",
                "evidence_source",
                "reason",
                "requires_operator_review",
            ]
            .contains(&key.as_str())
        })
        || trust.get("state") != Some(&Value::from("pending_review"))
        || trust.get("evidence_source") != Some(&Value::from("LIVE_RUNTIME"))
        || trust.get("requires_operator_review") != Some(&Value::Bool(true))
        || trust
            .get("reason")
            .and_then(Value::as_str)
            .is_none_or(|value| value.is_empty() || value.chars().count() > 512)
    {
        return Err("A2A Trustのmetadataが不正".to_string());
    }
    let capability_diff = object
        .get("Capability diff")
        .and_then(Value::as_object)
        .ok_or_else(|| "A2A Capability diffがobjectではない".to_string())?;
    if capability_diff.len() != 6
        || capability_diff.keys().any(|key| {
            ![
                "status",
                "added",
                "removed",
                "changed",
                "requires_operator_review",
                "evidence_source",
            ]
            .contains(&key.as_str())
        })
        || capability_diff.get("status") != Some(&Value::from("not_evaluated"))
        || capability_diff.get("requires_operator_review") != Some(&Value::Bool(true))
        || capability_diff.get("evidence_source") != Some(&Value::from("LIVE_RUNTIME"))
    {
        return Err("A2A Capability diffのmetadataが不正".to_string());
    }
    for field in ["added", "removed", "changed"] {
        let values = capability_diff
            .get(field)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("A2A Capability diffの{field}がarrayではない"))?;
        if values.len() > 128
            || values
                .iter()
                .any(|value| value.as_str().is_none_or(|item| !valid_agent_id(item)))
        {
            return Err(format!("A2A Capability diffの{field}が不正"));
        }
    }
    validate_card(object.get("Agent Card"), &agent_id)?;
    for key in ["Task", "Message", "Artifact", "Stream"] {
        let array = object
            .get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("A2A接続receiptの{key}がarrayではない"))?;
        let max = match key {
            "Task" => 128,
            "Message" | "Artifact" => 256,
            _ => 128,
        };
        if array.len() > max {
            return Err(format!("A2A接続receiptの{key}件数上限を超過"));
        }
    }
    for stream in object["Stream"]
        .as_array()
        .expect("A2A Streamは先にarray検証済み")
    {
        let stream = stream
            .as_object()
            .ok_or_else(|| "A2A Streamがobjectではない".to_string())?;
        if stream.len() != 7
            || stream.keys().any(|key| {
                ![
                    "stream_id",
                    "task_id",
                    "mode",
                    "status",
                    "event_types",
                    "resumable",
                    "authority_strip",
                ]
                .contains(&key.as_str())
            })
            || stream
                .get("stream_id")
                .and_then(Value::as_str)
                .is_none_or(|value| !valid_agent_id(value))
            || stream
                .get("task_id")
                .and_then(Value::as_str)
                .is_none_or(|value| !valid_agent_id(value))
            || stream.get("resumable").and_then(Value::as_bool).is_none()
            || stream.get("authority_strip") != Some(&Value::Bool(true))
        {
            return Err("A2A Streamのmetadataが不正".to_string());
        }
        let status = stream
            .get("status")
            .and_then(Value::as_object)
            .ok_or_else(|| "A2A Stream statusがobjectではない".to_string())?;
        if status.len() != 2
            || status
                .keys()
                .any(|key| !["status", "reason"].contains(&key.as_str()))
            || status.get("status").and_then(Value::as_str).is_none()
            || status.get("reason").and_then(Value::as_str).is_none()
        {
            return Err("A2A Stream statusのmetadataが不正".to_string());
        }
        let event_types = stream
            .get("event_types")
            .and_then(Value::as_array)
            .ok_or_else(|| "A2A Stream event_typesがarrayではない".to_string())?;
        if event_types.len() > 16
            || event_types
                .iter()
                .any(|value| value.as_str().is_none_or(|item| item.is_empty()))
        {
            return Err("A2A Stream event_typesのmetadataが不正".to_string());
        }
    }
    if !object["Task"].as_array().is_some_and(Vec::is_empty)
        || !object["Message"].as_array().is_some_and(Vec::is_empty)
        || !object["Artifact"].as_array().is_some_and(Vec::is_empty)
    {
        return Err("C16のA2A接続receiptへTask、Message、Artifactを保存できない".to_string());
    }
    reject_sensitive_keys(value)?;
    let audit_id = object
        .get("接続監査ID")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 256)
        .ok_or_else(|| "A2A接続receiptの接続監査IDが不正".to_string())?;
    if audit_id.contains("http://") || audit_id.contains("https://") {
        return Err("A2A接続receiptの接続監査IDへendpointを含められない".to_string());
    }
    Ok(agent_id)
}

fn validate_card(value: Option<&Value>, agent_id: &str) -> Result<(), String> {
    let card = value
        .and_then(Value::as_object)
        .ok_or_else(|| "A2A接続receiptのAgent Cardがobjectではない".to_string())?;
    const KEYS: [&str; 11] = [
        "agent_id",
        "display_name",
        "description_summary",
        "version",
        "endpoint_hash",
        "supported_interfaces",
        "capabilities",
        "skills",
        "authentication",
        "origin",
        "status",
    ];
    if card.keys().any(|key| !KEYS.contains(&key.as_str())) || card.len() != KEYS.len() {
        return Err("A2A Agent Cardに未知または欠落fieldがある".to_string());
    }
    if card.get("agent_id").and_then(Value::as_str) != Some(agent_id)
        || card.get("origin") != Some(&Value::from("live_runtime"))
        || card.get("status") != Some(&Value::from("discovered"))
        || !card
            .get("endpoint_hash")
            .and_then(Value::as_str)
            .is_some_and(valid_hash)
    {
        return Err("A2A Agent Cardのidentityまたはendpoint hashが不正".to_string());
    }
    for field in ["display_name", "description_summary", "version"] {
        let valid = card
            .get(field)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty() && value.chars().count() <= 1024);
        if !valid {
            return Err(format!("A2A Agent Cardの{field}が不正"));
        }
    }
    let interfaces = card
        .get("supported_interfaces")
        .and_then(Value::as_array)
        .ok_or_else(|| "A2A Agent Cardのinterfacesが不正".to_string())?;
    if interfaces.is_empty() || interfaces.len() > 8 {
        return Err("A2A Agent Cardのinterfaces件数が不正".to_string());
    }
    for interface in interfaces {
        let object = interface
            .as_object()
            .ok_or_else(|| "A2A interfaceがobjectではない".to_string())?;
        if object.len() != 6
            || object.keys().any(|key| {
                ![
                    "binding",
                    "protocol_version",
                    "interface_hash",
                    "transport",
                    "status",
                    "reason",
                ]
                .contains(&key.as_str())
            })
            || !object
                .get("binding")
                .and_then(Value::as_str)
                .is_some_and(|value| ["jsonrpc", "grpc", "http_rest", "unknown"].contains(&value))
            || !object
                .get("protocol_version")
                .and_then(Value::as_str)
                .is_some_and(|value| {
                    !value.is_empty()
                        && value.chars().count() <= 128
                        && !value.chars().any(char::is_control)
                })
            || !match (
                object.get("interface_hash"),
                object.get("status").and_then(Value::as_str),
            ) {
                (Some(Value::String(hash)), Some("unsupported" | "unknown")) => valid_hash(hash),
                (Some(Value::Null), Some("unknown")) => true,
                _ => false,
            }
            || !object
                .get("transport")
                .and_then(Value::as_str)
                .is_some_and(|value| ["http", "https", "unknown"].contains(&value))
            || !object
                .get("status")
                .and_then(Value::as_str)
                .is_some_and(|value| ["unsupported", "unknown"].contains(&value))
            || !object
                .get("reason")
                .and_then(Value::as_str)
                .is_some_and(|value| {
                    !value.is_empty()
                        && value.chars().count() <= 512
                        && !value.chars().any(char::is_control)
                })
        {
            return Err("A2A interfaceのmetadataが不正".to_string());
        }
    }
    let capabilities = card
        .get("capabilities")
        .and_then(Value::as_object)
        .ok_or_else(|| "A2A capabilitiesがobjectではない".to_string())?;
    if capabilities.len() != 4
        || capabilities.keys().any(|key| {
            ![
                "streaming",
                "push_notifications",
                "state_transition_history",
                "extended_agent_card",
            ]
            .contains(&key.as_str())
        })
        || capabilities.values().any(|value| !value.is_boolean())
    {
        return Err("A2A capabilitiesのmetadataが不正".to_string());
    }
    let skills = card
        .get("skills")
        .and_then(Value::as_array)
        .ok_or_else(|| "A2A skillsがarrayではない".to_string())?;
    if skills.len() > 128 {
        return Err("A2A skills件数の上限を超過".to_string());
    }
    for skill in skills {
        let skill = skill
            .as_object()
            .ok_or_else(|| "A2A skillがobjectではない".to_string())?;
        if skill.len() != 6
            || skill.keys().any(|key| {
                ![
                    "skill_id",
                    "name",
                    "description_summary",
                    "tags",
                    "input_modes",
                    "output_modes",
                ]
                .contains(&key.as_str())
            })
        {
            return Err("A2A skillのmetadataが不正".to_string());
        }
        for field in ["skill_id", "name", "description_summary"] {
            if skill
                .get(field)
                .and_then(Value::as_str)
                .is_none_or(|value| value.is_empty() || value.chars().count() > 1024)
            {
                return Err(format!("A2A skillの{field}が不正"));
            }
        }
        for field in ["tags", "input_modes", "output_modes"] {
            let values = skill
                .get(field)
                .and_then(Value::as_array)
                .ok_or_else(|| format!("A2A skillの{field}がarrayではない"))?;
            if values.len() > 32
                || values.iter().any(|value| {
                    value
                        .as_str()
                        .is_none_or(|item| item.is_empty() || item.chars().count() > 1024)
                })
            {
                return Err(format!("A2A skillの{field}が不正"));
            }
        }
    }
    let authentication = card
        .get("authentication")
        .and_then(Value::as_object)
        .ok_or_else(|| "A2A authenticationがobjectではない".to_string())?;
    if authentication.len() != 3
        || authentication.keys().any(|key| {
            !["schemes", "credential_ref", "secret_value_present"].contains(&key.as_str())
        })
        || authentication.get("secret_value_present") != Some(&Value::Bool(false))
    {
        return Err("A2A authenticationのmetadataが不正".to_string());
    }
    let schemes = authentication
        .get("schemes")
        .and_then(Value::as_array)
        .ok_or_else(|| "A2A authentication schemesがarrayではない".to_string())?;
    if schemes.len() > 16
        || schemes.iter().any(|value| {
            value
                .as_str()
                .is_none_or(|scheme| scheme.is_empty() || scheme.chars().count() > 128)
        })
    {
        return Err("A2A authentication schemesのmetadataが不正".to_string());
    }
    let credential = authentication
        .get("credential_ref")
        .and_then(Value::as_object)
        .ok_or_else(|| "A2A credential refがobjectではない".to_string())?;
    if credential.keys().any(|key| {
        !["credential_id", "purpose", "target", "required", "status"].contains(&key.as_str())
    }) || credential.len() != 5
        || credential.get("required") != Some(&Value::Bool(false))
        || credential.get("status") != Some(&Value::from("missing"))
        || credential
            .get("credential_id")
            .and_then(Value::as_str)
            .is_none_or(|value| value.len() != 32 || !value.chars().all(|c| c.is_ascii_hexdigit()))
    {
        return Err("A2A credential refのmetadataが不正".to_string());
    }
    for field in ["purpose", "target"] {
        if credential
            .get(field)
            .and_then(Value::as_str)
            .is_none_or(|value| value.is_empty() || value.chars().count() > 256)
        {
            return Err(format!("A2A credential refの{field}が不正"));
        }
    }
    Ok(())
}

fn reject_sensitive_keys(value: &Value) -> Result<(), String> {
    const FORBIDDEN: [&str; 15] = [
        "endpoint",
        "endpoint_uri",
        "uri",
        "secret",
        "secret_value",
        "token",
        "password",
        "credential_value",
        "raw_content",
        "payload",
        "body",
        "authority",
        "authority_context",
        "permission",
        "approval",
    ];
    match value {
        Value::Object(object) => {
            if object.keys().any(|key| FORBIDDEN.contains(&key.as_str())) {
                return Err(
                    "A2A接続stateへendpointまたは秘密・raw contentを保存できない".to_string(),
                );
            }
            for child in object.values() {
                reject_sensitive_keys(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                reject_sensitive_keys(child)?;
            }
        }
        Value::String(string) if string.contains("://") => {
            return Err("A2A接続stateへURI実値を保存できない".to_string())
        }
        _ => {}
    }
    Ok(())
}

fn valid_hash(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.chars().all(|character| character.is_ascii_hexdigit())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "操作")]
    operation: String,
    #[serde(rename = "AgentID")]
    agent_id: String,
    #[serde(rename = "Agent Card URI")]
    agent_card_uri: String,
    #[serde(rename = "protocol_version")]
    protocol_version: String,
    #[serde(rename = "Transport")]
    transport: String,
    #[serde(rename = "Credential ref")]
    credential_ref: Value,
}

pub(super) fn connect(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    owner: bool,
    payload_hash: &str,
) -> BrokerResponse {
    if !owner {
        return broker.reject_with_payload_hash(
            request_id,
            OP_CONNECT,
            "a2a_owner_required",
            "A2A接続にはowner controlが必要",
            true,
            payload_hash,
        );
    }
    if !broker.state_store.persistence_ready() {
        return broker.reject_with_payload_hash(
            request_id,
            OP_CONNECT,
            "broker_persistence_unavailable",
            "A2A接続には永続Auditが必要",
            true,
            payload_hash,
        );
    }
    let request = match parse_request(payload) {
        Ok(request) => request,
        Err(error) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_CONNECT,
                error.code,
                &error.message,
                true,
                payload_hash,
            )
        }
    };
    let previous_receipt = match broker.a2a_connections.get(&request.agent_id) {
        Some(receipt) if is_restored_pending_review(receipt) => Some(receipt.clone()),
        Some(_) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_CONNECT,
                "a2a_connection_duplicate",
                "現在有効な同じA2A Agent IDを重複登録できない",
                true,
                payload_hash,
            )
        }
        None => None,
    };
    let is_revalidation = previous_receipt.is_some();
    if broker.a2a_connections.len() >= MAX_CONNECTIONS && !is_revalidation {
        return broker.reject_with_payload_hash(
            request_id,
            OP_CONNECT,
            "a2a_connection_bound_exceeded",
            "A2A接続数のbounded上限を超過",
            true,
            payload_hash,
        );
    }
    if broker
        .append_audit(
            request_id,
            OP_CONNECT,
            "received",
            if is_revalidation {
                "owner controlによる再確認後、復元済みA2A metadata receiptの再検証要求を受信。URI実値とCredential実値はAuditへ保存しない"
            } else {
                "owner controlからA2A Agent Card取得要求を受信。URI実値とCredential実値はAuditへ保存しない"
            },
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_CONNECT,
            "a2a_audit_append_failed",
            "A2A接続の受信Auditを確定できない",
        );
    }

    let projection = match fetch_agent_card(
        &request.agent_card_uri,
        &request.agent_id,
        &request.protocol_version,
        &request.credential_ref,
    ) {
        Ok(projection) => projection,
        Err(error) => return reject_a2a_error(broker, request_id, error, payload_hash),
    };
    let mut receipt = projection;
    {
        let object = receipt.as_object_mut().expect("A2A射影はobject");
        object.insert(
            "AgentID".to_string(),
            Value::String(request.agent_id.clone()),
        );
        object.insert(
            "接続状態".to_string(),
            Value::String("connected".to_string()),
        );
        object.insert(
            "能力ID".to_string(),
            Value::String("a2a.connection.connect".to_string()),
        );
        object.insert(
            "権限ID".to_string(),
            Value::String("permission.a2a.connection.connect".to_string()),
        );
        object.insert(
            "承認状態".to_string(),
            Value::String("owner_control_approved".to_string()),
        );
        object.insert(
            "復旧ID".to_string(),
            Value::String("recover-a2a-connection".to_string()),
        );
    }
    let body_hash = super::protocol::canonical_payload_hash(Some(&receipt));
    let accepted = match broker.append_audit(
        request_id,
        OP_CONNECT,
        "accepted",
        &format!(
            "A2A Agent Card metadataを{}。AgentID={}",
            if is_revalidation {
                "fresh owner controlにより再検証"
            } else {
                "新規受理"
            },
            request.agent_id
        ),
        EVIDENCE_SOURCE_INTERNAL_STATE,
        &body_hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_CONNECT,
                "a2a_audit_append_failed",
                "A2A接続の受理Auditを確定できない。接続を破棄した",
            )
        }
    };
    receipt
        .as_object_mut()
        .expect("A2A projectionはobject")
        .insert(
            "接続監査ID".to_string(),
            Value::String(accepted.event_id.clone()),
        );
    broker
        .a2a_connections
        .insert(request.agent_id.clone(), receipt.clone());
    if let Err(reason) = persist(broker) {
        if let Some(previous_receipt) = previous_receipt {
            broker
                .a2a_connections
                .insert(request.agent_id.clone(), previous_receipt);
        } else {
            broker.a2a_connections.remove(&request.agent_id);
        }
        return broker.audit_store_failed_response(
            request_id,
            OP_CONNECT,
            "a2a_persistence_failed",
            &reason,
        );
    }
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OP_CONNECT.to_string(),
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
            "a2a_normal_channel_required",
            "A2A接続一覧は通常IPCだけが参照できる",
            true,
            payload_hash,
        );
    }
    if payload != &json!({ "版": VERSION }) {
        return broker.reject_with_payload_hash(
            request_id,
            OP_LIST,
            "a2a_list_request_invalid",
            "A2A接続一覧は版だけを受け付ける",
            true,
            payload_hash,
        );
    }
    if broker.a2a_connections.len() > MAX_CONNECTIONS {
        return broker.reject_with_payload_hash(
            request_id,
            OP_LIST,
            "a2a_connection_bound_exceeded",
            "A2A接続一覧のbounded上限を超過",
            true,
            payload_hash,
        );
    }
    if broker
        .append_audit(
            request_id,
            OP_LIST,
            "received",
            "A2A接続metadata一覧を要求",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_LIST,
            "a2a_audit_append_failed",
            "A2A接続一覧の受信Auditを確定できない",
        );
    }
    let connections: Vec<Value> = broker.a2a_connections.values().cloned().collect();
    let body = json!({
        "版": VERSION,
        "A2A接続一覧": connections,
        "件数": broker.a2a_connections.len(),
        "公開範囲": "metadata_only",
        "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
    });
    let event = match broker.append_audit(
        request_id,
        OP_LIST,
        "accepted",
        "A2A接続metadata一覧を返却。Credential実値とTask本文は返さない",
        EVIDENCE_SOURCE_INTERNAL_STATE,
        &super::protocol::canonical_payload_hash(Some(&body)),
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_LIST,
                "a2a_audit_append_failed",
                "A2A接続一覧の結果Auditを確定できない",
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

fn parse_request(payload: &Value) -> Result<ConnectionRequest, A2aError> {
    let request: ConnectionRequest = serde_json::from_value(payload.clone()).map_err(|_| {
        A2aError::new(
            "a2a_connection_request_invalid",
            "A2A接続payloadの構造が不正",
        )
    })?;
    if request.version != VERSION
        || request.operation != "接続"
        || !valid_agent_id(&request.agent_id)
        || request.protocol_version != PROTOCOL_VERSION
        || request.transport != "http"
    {
        return Err(A2aError::new(
            "a2a_connection_request_invalid",
            "A2A接続payloadの値または現行transport境界が不正",
        ));
    }
    Ok(request)
}

pub(crate) fn owner_confirmation_summary(payload: &Value) -> Result<(String, String), String> {
    let request = parse_request(payload).map_err(|error| error.message)?;
    let target = crate::a2a::owner_confirmation_target(
        &request.agent_card_uri,
        &request.credential_ref,
        &request.agent_id,
    )
    .map_err(|error| error.message)?;
    Ok((request.agent_id, target))
}

fn valid_agent_id(value: &str) -> bool {
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

fn reject_a2a_error(
    broker: &mut Broker,
    request_id: &str,
    error: A2aError,
    payload_hash: &str,
) -> BrokerResponse {
    broker.reject_with_payload_hash(
        request_id,
        OP_CONNECT,
        error.code,
        &error.message,
        true,
        payload_hash,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestStore(std::path::PathBuf);

    impl TestStore {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("gui-shell-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for TestStore {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn spawn_agent_card_fixture(
        status: u16,
        display_name: &'static str,
    ) -> (String, std::thread::JoinHandle<Result<(), String>>) {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback Agent Card待受");
        let address = listener.local_addr().expect("Agent Card接続先");
        let uri = format!("http://{address}/.well-known/agent-card.json");
        let body = if status == 200 {
            json!({
                "name": display_name,
                "description": "A2A receipt revalidation fixture",
                "version": "1.0.0",
                "supportedInterfaces": [{
                    "url": format!("http://{address}/rpc"),
                    "protocolBinding": "JSONRPC",
                    "protocolVersion": "1.0"
                }],
                "capabilities": {
                    "streaming": false,
                    "pushNotifications": false,
                    "stateTransitionHistory": false,
                    "extendedAgentCard": false
                },
                "securitySchemes": {},
                "skills": []
            })
        } else {
            json!({"error": "fixture not found"})
        };
        let body = serde_json::to_vec(&body).expect("Agent Card試験用JSON");
        let worker = std::thread::spawn(move || -> Result<(), String> {
            let (mut stream, peer) = listener
                .accept()
                .map_err(|error| format!("Agent Card fixture accept失敗: {error}"))?;
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .map_err(|error| format!("peer={peer}; read timeout設定失敗: {error}"))?;
            stream
                .set_write_timeout(Some(std::time::Duration::from_secs(10)))
                .map_err(|error| format!("peer={peer}; write timeout設定失敗: {error}"))?;
            let mut request = Vec::new();
            let mut chunk = [0u8; 512];
            while !request.windows(4).any(|value| value == b"\r\n\r\n") {
                let size = stream
                    .read(&mut chunk)
                    .map_err(|error| format!("peer={peer}; HTTP request読取失敗: {error}"))?;
                if size == 0 || request.len().saturating_add(size) >= 8192 {
                    return Err(format!(
                        "peer={peer}; HTTP requestがheader前に終了または上限超過"
                    ));
                }
                request.extend_from_slice(&chunk[..size]);
            }
            let status_text = if status == 200 {
                "200 OK"
            } else {
                "404 Not Found"
            };
            let mut response = format!(
                "HTTP/1.1 {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .into_bytes();
            response.extend_from_slice(&body);
            stream
                .write_all(&response)
                .map_err(|error| format!("peer={peer}; HTTP response write失敗: {error}"))?;
            stream
                .flush()
                .map_err(|error| format!("peer={peer}; HTTP response flush失敗: {error}"))?;
            stream
                .shutdown(std::net::Shutdown::Write)
                .map_err(|error| format!("peer={peer}; HTTP response shutdown失敗: {error}"))?;
            let mut client_close = [0; 1];
            match stream.read(&mut client_close) {
                Ok(0) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => Ok(()),
                Ok(size) => Err(format!("peer={peer}; 予期しない追加request bytes={size}")),
                Err(error) => Err(format!("peer={peer}; client close観測失敗: {error}")),
            }
        });
        (uri, worker)
    }

    fn valid_payload() -> Value {
        json!({
            "版": 1,
            "操作": "接続",
            "AgentID": "remote-agent-example",
            "Agent Card URI": "http://127.0.0.1:9080/.well-known/agent-card.json",
            "protocol_version": "1.0",
            "Transport": "http",
            "Credential ref": {
                "credential_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "purpose": "A2A接続",
                "target": "remote-agent-example",
                "required": false,
                "status": "missing"
            }
        })
    }

    #[test]
    fn A2A接続要求の正常値を受理できる() {
        let request = parse_request(&valid_payload()).expect("A2A接続要求の正常値");
        assert_eq!(request.agent_id, "remote-agent-example");
        assert_eq!(request.transport, "http");
    }

    #[test]
    fn A2A接続要求のAuthorityとHTTPSを拒否する() {
        let mut authority = valid_payload();
        authority["AgentID"] = Value::String("agent/authority".to_string());
        assert_eq!(
            parse_request(&authority).expect_err("AgentID不正").code,
            "a2a_connection_request_invalid"
        );
        let mut https = valid_payload();
        https["Transport"] = Value::String("https".to_string());
        assert_eq!(
            parse_request(&https).expect_err("未対応transport").code,
            "a2a_connection_request_invalid"
        );
    }

    #[test]
    fn owner接続をBrokerで受理し通常IPC一覧へbounded射影する() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;

        let store =
            std::env::temp_dir().join(format!("gui-shell-a2a-center-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&store);
        let mut broker = Broker::new_persistent("a2a-center-test", &store).expect("永続Broker");
        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback Agent Card待受");
        let address = listener.local_addr().expect("接続先");
        let response_body = serde_json::to_vec(&json!({
            "name": "外部Agent例",
            "description": "接続試験用Agent Card",
            "version": "1.0.0",
            "supportedInterfaces": [{"url": format!("http://{address}/rpc"), "protocolBinding": "JSONRPC", "protocolVersion": "1.0"}],
            "capabilities": {"streaming": false, "pushNotifications": false, "stateTransitionHistory": false, "extendedAgentCard": false},
            "securitySchemes": {},
            "skills": []
        }))
        .expect("Agent Card JSON");
        let response_length = response_body.len();
        let worker = thread::spawn(move || -> Result<String, String> {
            let (mut stream, peer) = listener
                .accept()
                .map_err(|error| format!("Agent Card要求受付失敗: {error}"))?;
            let local = stream
                .local_addr()
                .map_err(|error| format!("Agent Card待受取得失敗: {error}"))?;
            // 全suite負荷時にもfixture serverがclient要求の到着前に閉じないようにする。
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .map_err(|error| {
                    format!("peer={peer}; local={local}; read timeout設定失敗: {error}")
                })?;
            stream
                .set_write_timeout(Some(std::time::Duration::from_secs(10)))
                .map_err(|error| {
                    format!("peer={peer}; local={local}; write timeout設定失敗: {error}")
                })?;
            let mut request = Vec::new();
            let mut chunk = [0u8; 512];
            while !request.windows(4).any(|value| value == b"\r\n\r\n") {
                let size = stream.read(&mut chunk).map_err(|error| {
                    format!("peer={peer}; local={local}; HTTP要求読取失敗: {error}")
                })?;
                if size == 0 {
                    return Err(format!(
                        "peer={peer}; local={local}; header前に要求終了; received_bytes={}",
                        request.len()
                    ));
                }
                request.extend_from_slice(&chunk[..size]);
                if request.len() >= 8192 {
                    return Err(format!(
                        "peer={peer}; local={local}; HTTP要求header上限超過; received_bytes={}",
                        request.len()
                    ));
                }
            }
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {response_length}\r\nConnection: close\r\n\r\n"
            );
            let mut response = header.into_bytes();
            response.extend_from_slice(&response_body);
            stream.write_all(&response).map_err(|error| {
                format!(
                    "peer={peer}; local={local}; 応答write_all失敗; response_bytes={}: {error}",
                    response.len()
                )
            })?;
            stream.flush().map_err(|error| {
                format!(
                    "peer={peer}; local={local}; 応答flush失敗; response_bytes={}: {error}",
                    response.len()
                )
            })?;
            stream
                .shutdown(std::net::Shutdown::Write)
                .map_err(|error| {
                    format!("peer={peer}; local={local}; 応答shutdown失敗: {error}")
                })?;
            let mut client_close = [0; 1];
            let peer_close = match stream.read(&mut client_close) {
                Ok(0) => "peer_closed",
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {
                    "reset_after_response"
                }
                Ok(size) => {
                    return Err(format!(
                        "peer={peer}; local={local}; 応答後の予期しないdata; bytes={size}"
                    ));
                }
                Err(error) => {
                    return Err(format!(
                        "peer={peer}; local={local}; peer close観測失敗: {error}"
                    ));
                }
            };
            Ok(format!(
                "peer={peer}; local={local}; request_bytes={}; response_bytes={}; response=written; peer_close={peer_close}",
                request.len(),
                response.len()
            ))
        });
        let mut payload = valid_payload();
        payload["Agent Card URI"] =
            Value::String(format!("http://{address}/.well-known/agent-card.json"));
        let connected = connect(
            &mut broker,
            "a2a-connect-test",
            &payload,
            true,
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        let server_observation = worker.join().expect("Agent Card試験worker panic");
        assert!(
            server_observation.is_ok(),
            "Agent Card fixture observation={server_observation:?}; Broker response={:?}",
            connected.error
        );
        assert_eq!(
            connected.status,
            BrokerStatus::Accepted,
            "Broker応答={:?}。Agent Card試験用serverの観測={server_observation:?}",
            connected.error
        );
        assert_eq!(
            connected.body.as_ref().unwrap()["公開範囲"],
            "metadata_only"
        );

        let listed = list(
            &mut broker,
            "a2a-list-test",
            &json!({"版": VERSION}),
            false,
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        assert_eq!(listed.status, BrokerStatus::Accepted);
        assert_eq!(listed.body.as_ref().unwrap()["件数"], 1);
        let owner_list = list(
            &mut broker,
            "a2a-owner-list-test",
            &json!({"版": VERSION}),
            true,
            "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        );
        assert_eq!(owner_list.status, BrokerStatus::Rejected);
        drop(broker);
        let mut restarted =
            Broker::new_persistent("a2a-center-restarted", &store).expect("再起動後の永続Broker");
        assert_eq!(restarted.a2a_connections.len(), 1);
        let restored = list(
            &mut restarted,
            "a2a-restarted-list-test",
            &json!({"版": VERSION}),
            false,
            "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        );
        assert_eq!(restored.status, BrokerStatus::Accepted);
        let restored_receipt = &restored.body.as_ref().unwrap()["A2A接続一覧"][0];
        assert_eq!(restored_receipt["接続状態"], "restored_pending_review");
        assert_eq!(restored_receipt["承認状態"], "owner_reapproval_required");
        assert_eq!(restored_receipt["証拠種別"], EVIDENCE_SOURCE_INTERNAL_STATE);
        let persisted = restarted
            .state_store
            .load_a2a_state()
            .expect("A2A接続state読取")
            .expect("永続storeのA2A接続state");
        assert!(!persisted.to_string().contains("http://127.0.0.1"));
        let _ = std::fs::remove_dir_all(store);
    }

    #[test]
    fn 再起動後pending_receiptだけを新Owner確認で再検証し失敗時は保持する() {
        let store = TestStore::new("a2a-revalidation");
        let mut broker =
            Broker::new_persistent("a2a-revalidation-initial", &store.0).expect("初回永続Broker");
        let (initial_uri, initial_worker) = spawn_agent_card_fixture(200, "初回Agent");
        let mut payload = valid_payload();
        payload["Agent Card URI"] = Value::String(initial_uri);
        let initial = connect(
            &mut broker,
            "a2a-revalidation-initial-connect",
            &payload,
            true,
            "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        );
        assert!(initial_worker.join().expect("初回fixture worker").is_ok());
        assert_eq!(initial.status, BrokerStatus::Accepted);
        let initial_receipt = initial.body.expect("初回接続receipt");
        assert_eq!(initial_receipt["接続状態"], "connected");
        drop(broker);

        let mut restarted =
            Broker::new_persistent("a2a-revalidation-restarted", &store.0).expect("再起動Broker");
        let pending_receipt = restarted
            .a2a_connections
            .get("remote-agent-example")
            .expect("復元receipt")
            .clone();
        assert!(is_restored_pending_review(&pending_receipt));

        let denied_without_owner = connect(
            &mut restarted,
            "a2a-revalidation-no-owner",
            &valid_payload(),
            false,
            "sha256:2222222222222222222222222222222222222222222222222222222222222222",
        );
        assert_eq!(denied_without_owner.status, BrokerStatus::Rejected);
        assert_eq!(
            restarted.a2a_connections.get("remote-agent-example"),
            Some(&pending_receipt)
        );

        let (failed_uri, failed_worker) = spawn_agent_card_fixture(404, "失敗Agent");
        let mut failed_payload = valid_payload();
        failed_payload["Agent Card URI"] = Value::String(failed_uri);
        let fetch_failed = connect(
            &mut restarted,
            "a2a-revalidation-fetch-failed",
            &failed_payload,
            true,
            "sha256:3333333333333333333333333333333333333333333333333333333333333333",
        );
        assert!(failed_worker.join().expect("404応答を返す試験処理").is_ok());
        assert_eq!(fetch_failed.status, BrokerStatus::Rejected);
        assert_eq!(
            restarted.a2a_connections.get("remote-agent-example"),
            Some(&pending_receipt)
        );

        let state_path = store.0.join("a2a_connections.json");
        let preserved_path = store.0.join("a2a_connections.revalidation-preserved");
        let persisted_before_failure = std::fs::read(&state_path).expect("既存永続receipt読取");
        std::fs::rename(&state_path, &preserved_path).expect("既存永続receipt保全");
        std::fs::create_dir(&state_path).expect("永続先を一時的にdirectory化");
        let (persistence_uri, persistence_worker) = spawn_agent_card_fixture(200, "永続失敗Agent");
        let mut persistence_payload = valid_payload();
        persistence_payload["Agent Card URI"] = Value::String(persistence_uri);
        let persistence_failed = connect(
            &mut restarted,
            "a2a-revalidation-persist-failed",
            &persistence_payload,
            true,
            "sha256:4444444444444444444444444444444444444444444444444444444444444444",
        );
        assert!(persistence_worker
            .join()
            .expect("永続化失敗時の応答試験処理")
            .is_ok());
        assert_eq!(persistence_failed.status, BrokerStatus::Suspended);
        assert_eq!(
            persistence_failed
                .error
                .as_ref()
                .map(|error| error.code.as_str()),
            Some("a2a_persistence_failed")
        );
        assert_eq!(
            restarted.a2a_connections.get("remote-agent-example"),
            Some(&pending_receipt)
        );
        let temp_path = state_path.with_extension("tmp");
        assert!(!temp_path.exists(), "永続失敗時のatomic temp残留");
        std::fs::remove_dir(&state_path).expect("一時directory cleanup");
        std::fs::rename(&preserved_path, &state_path).expect("既存receipt復元");
        assert_eq!(
            std::fs::read(&state_path).expect("保全receipt再読取"),
            persisted_before_failure
        );
        drop(restarted);

        let mut retried =
            Broker::new_persistent("a2a-revalidation-retry", &store.0).expect("再試行Broker");
        let (fresh_uri, fresh_worker) = spawn_agent_card_fixture(200, "再検証済みAgent");
        let mut fresh_payload = valid_payload();
        fresh_payload["Agent Card URI"] = Value::String(fresh_uri.clone());
        let revalidated = connect(
            &mut retried,
            "a2a-revalidation-success",
            &fresh_payload,
            true,
            "sha256:5555555555555555555555555555555555555555555555555555555555555555",
        );
        assert!(fresh_worker.join().expect("再検証応答試験処理").is_ok());
        assert_eq!(revalidated.status, BrokerStatus::Accepted);
        let fresh_receipt = revalidated.body.expect("再検証receipt");
        assert_eq!(fresh_receipt["接続状態"], "connected");
        assert_eq!(fresh_receipt["証拠種別"], "LIVE_RUNTIME");
        assert_eq!(fresh_receipt["承認状態"], "owner_control_approved");
        assert_ne!(fresh_receipt["接続監査ID"], initial_receipt["接続監査ID"]);
        assert_eq!(
            retried.a2a_connections.get("remote-agent-example"),
            Some(&fresh_receipt)
        );

        let active_duplicate = connect(
            &mut retried,
            "a2a-revalidation-active-duplicate",
            &fresh_payload,
            true,
            "sha256:6666666666666666666666666666666666666666666666666666666666666666",
        );
        assert_eq!(active_duplicate.status, BrokerStatus::Rejected);
        assert_eq!(
            active_duplicate
                .error
                .as_ref()
                .map(|error| error.code.as_str()),
            Some("a2a_connection_duplicate")
        );
        assert_eq!(
            retried.a2a_connections.get("remote-agent-example"),
            Some(&fresh_receipt)
        );
    }

    #[test]
    fn malformedなA2A接続stateは再起動時にfail_closedになる() {
        let store =
            std::env::temp_dir().join(format!("gui-shell-a2a-malformed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&store);
        let broker = Broker::new_persistent("a2a-malformed-test", &store).expect("永続Broker");
        broker
            .state_store
            .write_a2a_state(&json!({"版": 1, "connections": [{}]}))
            .expect("malformed state書込試験");
        drop(broker);
        let error = Broker::new_persistent("a2a-malformed-restart", &store)
            .expect_err("malformed stateを拒否");
        assert!(matches!(error, BrokerStoreError::MalformedA2aState(_)));
        let _ = std::fs::remove_dir_all(store);
    }

    #[test]
    fn 旧版interface保存stateはAuthorityを継承せずunknownへ復元する() {
        let mut legacy: Value = serde_json::from_str(include_str!(
            "../../../../examples/contracts/a2a_connection_receipt.valid.json"
        ))
        .expect("旧版receipt fixture");
        let interface = &mut legacy["Agent Card"]["supported_interfaces"][0];
        interface
            .as_object_mut()
            .unwrap()
            .remove("protocol_version");
        interface.as_object_mut().unwrap().remove("interface_hash");
        interface["status"] = Value::String("supported".to_string());
        interface["reason"] = Value::String("旧版Agent Card宣言".to_string());
        assert!(validate_persisted_receipt(&legacy).is_err());

        let state = json!({ "版": VERSION, "connections": [legacy] });
        let restored = decode_state(&state).expect("旧版stateの保守的projection");
        let receipt = &restored["remote-agent-example"];
        let projected = &receipt["Agent Card"]["supported_interfaces"][0];
        assert_eq!(projected["protocol_version"], "unknown");
        assert_eq!(projected["interface_hash"], Value::Null);
        assert_eq!(projected["status"], "unknown");
        assert_eq!(receipt["承認状態"], "owner_reapproval_required");
        assert_eq!(receipt["接続状態"], "restored_pending_review");
        assert_eq!(receipt["証拠種別"], EVIDENCE_SOURCE_INTERNAL_STATE);
    }
}
