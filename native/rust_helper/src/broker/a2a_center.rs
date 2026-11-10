//! C16 A2A接続センターのBroker経路。
//!
//! Agent Card discoveryと、無認証A2A v1.0 JSON-RPC SendMessage／GetTaskのBroker経路。
//! Credential注入、Task／Artifact本文、Stream購読は未接続とする。
#![allow(non_snake_case)]

use super::protocol::{Broker, BrokerResponse, BrokerStatus, EVIDENCE_SOURCE_INTERNAL_STATE};
use super::store::{BrokerPersistentStore, BrokerStoreError};
use crate::a2a::{fetch_agent_card, A2aError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const OP_CONNECT: &str = "A2A接続";
const OP_LIST: &str = "A2A接続一覧";
const OP_SEND_MESSAGE: &str = "A2Aメッセージ送信";
const OP_GET_TASK: &str = "A2Aタスク状態取得";
const VERSION: u64 = 1;
const PROTOCOL_VERSION: &str = "1.0";
const MAX_CONNECTIONS: usize = 64;
const MAX_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_MESSAGE_CHARS: usize = 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SendMessageRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "操作")]
    operation: String,
    #[serde(rename = "AgentID")]
    agent_id: String,
    #[serde(rename = "Interface URI")]
    interface_uri: String,
    #[serde(rename = "Message")]
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GetTaskRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "操作")]
    operation: String,
    #[serde(rename = "AgentID")]
    agent_id: String,
    #[serde(rename = "Interface URI")]
    interface_uri: String,
    #[serde(rename = "TaskID")]
    task_id: String,
    #[serde(rename = "ArtifactID")]
    artifact_id: Option<String>,
    #[serde(rename = "Artifact hash")]
    artifact_hash: Option<String>,
}

struct OneShotA2aPermission {
    permission_id: String,
    request_id: String,
    agent_id: String,
    interface_hash: String,
    binding_hash: String,
    expires_at: std::time::Instant,
    consumed: bool,
}

impl OneShotA2aPermission {
    fn issue(
        operation_prefix: &str,
        request_id: &str,
        agent_id: &str,
        interface_hash: &str,
        binding_hash: &str,
    ) -> Option<Self> {
        let mut nonce = [0u8; 16];
        getrandom::getrandom(&mut nonce).ok()?;
        Some(Self {
            permission_id: format!("{operation_prefix}{}", hex::encode(nonce)),
            request_id: request_id.to_string(),
            agent_id: agent_id.to_string(),
            interface_hash: interface_hash.to_string(),
            binding_hash: binding_hash.to_string(),
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(15),
            consumed: false,
        })
    }

    fn consume(
        &mut self,
        request_id: &str,
        agent_id: &str,
        interface_hash: &str,
        binding_hash: &str,
    ) -> bool {
        if self.consumed
            || std::time::Instant::now() >= self.expires_at
            || self.request_id != request_id
            || self.agent_id != agent_id
            || self.interface_hash != interface_hash
            || self.binding_hash != binding_hash
        {
            return false;
        }
        self.consumed = true;
        true
    }
}

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
                (Some(Value::String(hash)), Some("supported" | "unsupported" | "unknown")) => {
                    valid_hash(hash)
                }
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
                .is_some_and(|value| ["supported", "unsupported", "unknown"].contains(&value))
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
    if !(3..=4).contains(&authentication.len())
        || authentication.keys().any(|key| {
            ![
                "schemes",
                "credential_ref",
                "secret_value_present",
                "security_requirements_present",
            ]
            .contains(&key.as_str())
        })
        || authentication.get("secret_value_present") != Some(&Value::Bool(false))
        || authentication
            .get("security_requirements_present")
            .is_some_and(|value| !value.is_boolean())
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
    for interface in interfaces {
        let interface = interface.as_object().expect("A2A interfaceは先行検査済み");
        if interface.get("status").and_then(Value::as_str) == Some("supported")
            && (interface.get("binding").and_then(Value::as_str) != Some("jsonrpc")
                || interface.get("protocol_version").and_then(Value::as_str)
                    != Some(PROTOCOL_VERSION)
                || !schemes.is_empty()
                || authentication.get("security_requirements_present") != Some(&Value::Bool(false))
                || interface.get("reason").and_then(Value::as_str)
                    != Some("認証情報なしのA2A v1.0 JSON-RPC SendMessage consumerを接続済み"))
        {
            return Err(
                "A2A supported interfaceは無認証JSON-RPC v1.0 SendMessage consumerだけを表す"
                    .to_string(),
            );
        }
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

fn parse_send_message_request(payload: &Value) -> Result<SendMessageRequest, A2aError> {
    let request: SendMessageRequest = serde_json::from_value(payload.clone()).map_err(|_| {
        A2aError::new(
            "a2a_message_request_invalid",
            "A2A message payloadの構造が不正",
        )
    })?;
    if request.version != VERSION
        || request.operation != "送信"
        || !valid_agent_id(&request.agent_id)
        || request.message.trim().is_empty()
        || request.message.len() > MAX_MESSAGE_BYTES
        || request.message.chars().count() > MAX_MESSAGE_CHARS
        || request.message.chars().any(|character| {
            (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
                || matches!(character, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
    {
        return Err(A2aError::new(
            "a2a_message_request_invalid",
            "A2A message payloadの値またはbounded本文が不正",
        ));
    }
    crate::a2a::transport_name(&request.interface_uri)?;
    Ok(request)
}

fn validate_send_message_target(
    broker: &Broker,
    request: &SendMessageRequest,
) -> Result<String, A2aError> {
    validate_registered_interface(broker, &request.agent_id, &request.interface_uri)
}

fn validate_registered_interface(
    broker: &Broker,
    agent_id: &str,
    interface_uri: &str,
) -> Result<String, A2aError> {
    let receipt = broker
        .a2a_connections
        .get(agent_id)
        .ok_or_else(|| A2aError::new("a2a_connection_missing", "A2A接続receiptがない"))?;
    if receipt.get("接続状態").and_then(Value::as_str) != Some("connected")
        || receipt.get("証拠種別").and_then(Value::as_str) != Some("LIVE_RUNTIME")
        || receipt.get("承認状態").and_then(Value::as_str) != Some("owner_control_approved")
        || receipt
            .get("Trust")
            .and_then(|value| value.get("state"))
            .and_then(Value::as_str)
            != Some("pending_review")
    {
        return Err(A2aError::new(
            "a2a_connection_revalidation_required",
            "A2A接続receiptは再確認またはOperator reviewが必要",
        ));
    }
    let card = receipt
        .get("Agent Card")
        .and_then(Value::as_object)
        .ok_or_else(|| A2aError::new("a2a_connection_invalid", "A2A Agent Card receiptが不正"))?;
    let authentication = card
        .get("authentication")
        .and_then(Value::as_object)
        .ok_or_else(|| A2aError::new("a2a_connection_invalid", "A2A認証metadataが不正"))?;
    if authentication
        .get("schemes")
        .and_then(Value::as_array)
        .is_none_or(|items| !items.is_empty())
        || authentication.get("security_requirements_present") != Some(&Value::Bool(false))
        || authentication.get("secret_value_present") != Some(&Value::Bool(false))
    {
        return Err(A2aError::new(
            "a2a_authentication_unsupported",
            "現行A2A Message送信はCredentialを使用しないinterfaceに限定",
        ));
    }
    let (transport, interface_hash) = crate::a2a::interface_identity(interface_uri)?;
    let interfaces = card
        .get("supported_interfaces")
        .and_then(Value::as_array)
        .ok_or_else(|| A2aError::new("a2a_connection_invalid", "A2A interface receiptが不正"))?;
    let matched = interfaces.iter().any(|interface| {
        interface.get("binding").and_then(Value::as_str) == Some("jsonrpc")
            && interface.get("protocol_version").and_then(Value::as_str) == Some(PROTOCOL_VERSION)
            && interface.get("transport").and_then(Value::as_str) == Some(transport.as_str())
            && interface.get("status").and_then(Value::as_str) == Some("supported")
            && interface.get("interface_hash").and_then(Value::as_str)
                == Some(interface_hash.as_str())
    });
    if !matched {
        return Err(A2aError::new(
            "a2a_interface_not_supported",
            "現在のAgent Cardに一致する無認証A2A v1.0 JSON-RPC interfaceがない",
        ));
    }
    Ok(interface_hash)
}

fn parse_get_task_request(payload: &Value) -> Result<GetTaskRequest, A2aError> {
    let request: GetTaskRequest = serde_json::from_value(payload.clone()).map_err(|_| {
        A2aError::new(
            "a2a_task_request_invalid",
            "A2A Task取得payloadの構造が不正",
        )
    })?;
    if request.version != VERSION
        || request.operation != "状態取得"
        || !valid_agent_id(&request.agent_id)
        || !crate::a2a::valid_task_id(&request.task_id)
        || request
            .artifact_id
            .as_deref()
            .is_some_and(|value| !crate::a2a::valid_task_id(value))
        || request
            .artifact_hash
            .as_deref()
            .is_some_and(|value| !valid_sha256_hash(value))
        || request.artifact_id.is_some() != request.artifact_hash.is_some()
    {
        return Err(A2aError::new(
            "a2a_task_request_invalid",
            "A2A Task取得payloadの値が不正",
        ));
    }
    crate::a2a::transport_name(&request.interface_uri)?;
    Ok(request)
}

fn valid_sha256_hash(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value.as_bytes()[7..]
            .iter()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(super) fn send_message(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    owner: bool,
    payload_hash: &str,
) -> BrokerResponse {
    if !owner {
        return broker.reject_with_payload_hash(
            request_id,
            OP_SEND_MESSAGE,
            "a2a_native_owner_required",
            "A2A Message送信は各送信内容に対するnative Owner確認が必要",
            true,
            payload_hash,
        );
    }
    if !broker.state_store.persistence_ready() {
        return broker.reject_with_payload_hash(
            request_id,
            OP_SEND_MESSAGE,
            "broker_persistence_unavailable",
            "A2A Message送信には永続Auditが必要",
            true,
            payload_hash,
        );
    }
    let request = match parse_send_message_request(payload) {
        Ok(request) => request,
        Err(error) => return reject_send_message(broker, request_id, error, payload_hash),
    };
    let interface_hash = match validate_send_message_target(broker, &request) {
        Ok(hash) => hash,
        Err(error) => return reject_send_message(broker, request_id, error, payload_hash),
    };
    let message_hash = crate::audit_hash::sha256_tagged(request.message.as_bytes());
    if broker
        .append_audit(
            request_id,
            OP_SEND_MESSAGE,
            "received",
            &format!(
                "A2A SendMessage要求を受信。AgentID={} interface_hash={} message_hash={}。本文・URI・CredentialはAuditへ保存しない",
                request.agent_id, interface_hash, message_hash
            ),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_SEND_MESSAGE,
            "a2a_audit_append_failed",
            "A2A Message送信の受信Auditを確定できない",
        );
    }
    if broker
        .append_audit(
            request_id,
            OP_SEND_MESSAGE,
            "owner_confirmed",
            "native Owner確認が現在の要求hash・対象Agent・本文hashに一致。再利用しない",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_SEND_MESSAGE,
            "a2a_audit_append_failed",
            "A2A Message送信のOwner確認Auditを確定できない",
        );
    }
    let mut permission = match OneShotA2aPermission::issue(
        "a2a-message-",
        request_id,
        &request.agent_id,
        &interface_hash,
        &message_hash,
    ) {
        Some(permission) => permission,
        None => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_SEND_MESSAGE,
                "a2a_permission_unavailable",
                "一回限りA2A Message Permissionを発行できない",
                true,
                payload_hash,
            )
        }
    };
    if broker
        .append_audit(
            request_id,
            OP_SEND_MESSAGE,
            "permission_issued",
            &format!(
                "一回限りPermissionを発行。permission_id={} AgentID={} interface_hash={} message_hash={}。永続化せず、再利用しない",
                permission.permission_id, request.agent_id, interface_hash, message_hash
            ),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_SEND_MESSAGE,
            "a2a_audit_append_failed",
            "A2A Message PermissionのAuditを確定できない",
        );
    }
    if !permission.consume(
        request_id,
        &request.agent_id,
        &interface_hash,
        &message_hash,
    ) {
        return broker.reject_with_payload_hash(
            request_id,
            OP_SEND_MESSAGE,
            "a2a_permission_binding_invalid",
            "A2A Message Permissionの対象が一致しない",
            false,
            payload_hash,
        );
    }
    if broker
        .append_audit(
            request_id,
            OP_SEND_MESSAGE,
            "started",
            "一回限りPermissionを消費。A2A SendMessageを一回だけ実行し、応答不明時は再送しない",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_SEND_MESSAGE,
            "a2a_audit_append_failed",
            "A2A Message送信の開始Auditを確定できず、外部作用を行わない",
        );
    }
    let result = crate::a2a::send_message(
        &request.interface_uri,
        &request.message,
        std::time::Instant::now() + std::time::Duration::from_secs(15),
    );
    match result {
        Ok(result) => {
            let result_hash = result
                .get("result_hash")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let body = json!({
                "版": VERSION,
                "AgentID": request.agent_id,
                "interface_hash": interface_hash,
                "status": "accepted",
                "message_hash": message_hash,
                "result": result,
                "Trust": "pending_review",
                "権限生成": "なし",
                "authority_strip": true,
                "公開範囲": "metadata_only",
                "再送可能": false
            });
            let event = match broker.append_audit(
                request_id,
                OP_SEND_MESSAGE,
                "accepted",
                &format!(
                    "A2A SendMessage応答を受領。AgentID={} result_hash={}。外部応答本文は保存・投影せず、Task完了とは扱わない",
                    request.agent_id, result_hash
                ),
                "LIVE_RUNTIME",
                payload_hash,
            ) {
                Ok(event) => event,
                Err(_) => {
                    return broker.audit_store_failed_response(
                        request_id,
                        OP_SEND_MESSAGE,
                        "a2a_audit_append_failed",
                        "外部応答後のAuditを確定できない。再送せず結果不明とする",
                    )
                }
            };
            BrokerResponse {
                request_id: request_id.to_string(),
                operation: OP_SEND_MESSAGE.to_string(),
                status: BrokerStatus::Accepted,
                evidence_source: "LIVE_RUNTIME".to_string(),
                audit_event_id: event.event_id,
                error: None,
                health: None,
                body: Some(body),
                shutdown_requested: broker.shutdown_requested,
            }
        }
        Err(error) => unknown_send_message(
            broker,
            request_id,
            &request.agent_id,
            &interface_hash,
            &message_hash,
            payload_hash,
            error.code,
        ),
    }
}

pub(super) fn get_task(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    owner: bool,
    payload_hash: &str,
) -> BrokerResponse {
    if !owner {
        return broker.reject_with_payload_hash(
            request_id,
            OP_GET_TASK,
            "a2a_native_owner_required",
            "A2A Task状態取得は各要求に対するnative Owner確認が必要",
            true,
            payload_hash,
        );
    }
    if !broker.state_store.persistence_ready() {
        return broker.reject_with_payload_hash(
            request_id,
            OP_GET_TASK,
            "broker_persistence_unavailable",
            "A2A Task状態取得には永続Auditが必要",
            true,
            payload_hash,
        );
    }
    let request = match parse_get_task_request(payload) {
        Ok(request) => request,
        Err(error) => return reject_get_task(broker, request_id, error, payload_hash),
    };
    let interface_hash =
        match validate_registered_interface(broker, &request.agent_id, &request.interface_uri) {
            Ok(hash) => hash,
            Err(error) => return reject_get_task(broker, request_id, error, payload_hash),
        };
    let task_id_hash = crate::audit_hash::sha256_tagged(request.task_id.as_bytes());
    let artifact_id_hash = request
        .artifact_id
        .as_deref()
        .map(|value| crate::audit_hash::sha256_tagged(value.as_bytes()));
    if broker
        .append_audit(
            request_id,
            OP_GET_TASK,
            "received",
            &format!(
                "A2A GetTask要求を受信。AgentID={} interface_hash={} task_id_hash={} artifact_id_hash={}。Task／Artifact ID実値・URI・本文はAuditへ保存しない",
                request.agent_id,
                interface_hash,
                task_id_hash,
                artifact_id_hash.as_deref().unwrap_or("none")
            ),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_GET_TASK,
            "a2a_audit_append_failed",
            "A2A Task取得の受信Auditを確定できない",
        );
    }
    if broker
        .append_audit(
            request_id,
            OP_GET_TASK,
            "owner_confirmed",
            "native Owner確認が現在の要求hash・対象Agent・Task ID hashに一致。再利用しない",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_GET_TASK,
            "a2a_audit_append_failed",
            "A2A Task取得のOwner確認Auditを確定できない",
        );
    }
    let mut permission = match OneShotA2aPermission::issue(
        "a2a-task-get-",
        request_id,
        &request.agent_id,
        &interface_hash,
        payload_hash,
    ) {
        Some(permission) => permission,
        None => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_GET_TASK,
                "a2a_permission_unavailable",
                "一回限りA2A Task取得Permissionを発行できない",
                true,
                payload_hash,
            )
        }
    };
    if broker
        .append_audit(
            request_id,
            OP_GET_TASK,
            "permission_issued",
            &format!(
                "一回限りPermissionを発行。permission_id={} AgentID={} interface_hash={} task_id_hash={}。永続化せず、再利用しない",
                permission.permission_id, request.agent_id, interface_hash, task_id_hash
            ),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_GET_TASK,
            "a2a_audit_append_failed",
            "A2A Task取得PermissionのAuditを確定できない",
        );
    }
    if !permission.consume(request_id, &request.agent_id, &interface_hash, payload_hash) {
        return broker.reject_with_payload_hash(
            request_id,
            OP_GET_TASK,
            "a2a_permission_binding_invalid",
            "A2A Task取得Permissionの対象が一致しない",
            false,
            payload_hash,
        );
    }
    if broker
        .append_audit(
            request_id,
            OP_GET_TASK,
            "started",
            if request.artifact_id.is_some() {
                "一回限りPermissionを消費。A2A GetTaskからhash一致するtext Artifactだけを一回取得し、bounded full表示へ渡す"
            } else {
                "一回限りPermissionを消費。A2A GetTaskを一回だけ実行し、historyLength=0でbounded metadataを受け取る"
            },
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_GET_TASK,
            "a2a_audit_append_failed",
            "A2A Task取得の開始Auditを確定できず、外部読取を行わない",
        );
    }
    match crate::a2a::get_task(
        &request.interface_uri,
        &request.task_id,
        request
            .artifact_id
            .as_deref()
            .zip(request.artifact_hash.as_deref()),
        std::time::Instant::now() + std::time::Duration::from_secs(15),
    ) {
        Ok(result) => {
            let result_hash = result
                .get("result_hash")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let selected_artifact = result.get("selected_artifact");
            let selected_artifact_hash = selected_artifact
                .and_then(|value| value.get("artifact_hash"))
                .and_then(Value::as_str)
                .unwrap_or("none");
            let selected_content_hash = selected_artifact
                .and_then(|value| value.get("content_hash"))
                .and_then(Value::as_str)
                .unwrap_or("none");
            let selected_byte_length = selected_artifact
                .and_then(|value| value.get("byte_length"))
                .and_then(Value::as_u64)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string());
            let body = json!({
                "版": VERSION,
                "AgentID": request.agent_id,
                "interface_hash": interface_hash,
                "status": "accepted",
                "result": result,
                "Trust": "pending_review",
                "権限生成": "なし",
                "authority_strip": true,
                "公開範囲": if request.artifact_id.is_some() { "full" } else { "metadata_only" },
                "履歴公開": false,
                "Credential使用": false
            });
            let event = match broker.append_audit(
                request_id,
                OP_GET_TASK,
                "accepted",
                &format!(
                    "A2A GetTask結果を受領。AgentID={} interface_hash={} task_id_hash={} artifact_id_hash={} result_hash={} selected_artifact_hash={} content_hash={} byte_length={}。Artifact本文はAuditへ保存しない",
                    request.agent_id,
                    interface_hash,
                    task_id_hash,
                    artifact_id_hash.as_deref().unwrap_or("none"),
                    result_hash,
                    selected_artifact_hash,
                    selected_content_hash,
                    selected_byte_length
                ),
                "LIVE_RUNTIME",
                payload_hash,
            ) {
                Ok(event) => event,
                Err(_) => {
                    return broker.audit_store_failed_response(
                        request_id,
                        OP_GET_TASK,
                        "a2a_audit_append_failed",
                        "A2A Task応答後のAuditを確定できない。metadataを成功として返さない",
                    )
                }
            };
            BrokerResponse {
                request_id: request_id.to_string(),
                operation: OP_GET_TASK.to_string(),
                status: BrokerStatus::Accepted,
                evidence_source: "LIVE_RUNTIME".to_string(),
                audit_event_id: event.event_id,
                error: None,
                health: None,
                body: Some(body),
                shutdown_requested: broker.shutdown_requested,
            }
        }
        Err(error) if error.code.starts_with("a2a_artifact_") => {
            reject_get_task(broker, request_id, error, payload_hash)
        }
        Err(error) if error.code == "a2a_remote_task_rejected" => broker.reject_with_payload_hash(
            request_id,
            OP_GET_TASK,
            "a2a_remote_task_rejected",
            "外部AgentがTask状態取得を拒否した。外部応答本文は表示しない",
            true,
            payload_hash,
        ),
        Err(error) => unknown_get_task(
            broker,
            request_id,
            &request.agent_id,
            &interface_hash,
            &task_id_hash,
            payload_hash,
            error.code,
        ),
    }
}

fn reject_get_task(
    broker: &mut Broker,
    request_id: &str,
    error: A2aError,
    payload_hash: &str,
) -> BrokerResponse {
    broker.reject_with_payload_hash(
        request_id,
        OP_GET_TASK,
        error.code,
        &error.message,
        true,
        payload_hash,
    )
}

fn unknown_get_task(
    broker: &mut Broker,
    request_id: &str,
    agent_id: &str,
    interface_hash: &str,
    task_id_hash: &str,
    payload_hash: &str,
    failure_code: &str,
) -> BrokerResponse {
    let body = json!({
        "版": VERSION,
        "AgentID": agent_id,
        "interface_hash": interface_hash,
        "status": "unknown",
        "result": Value::Null,
        "Trust": "pending_review",
        "権限生成": "なし",
        "authority_strip": true,
        "公開範囲": "metadata_only",
        "履歴公開": false,
        "Credential使用": false
    });
    let event = match broker.append_audit(
        request_id,
        OP_GET_TASK,
        "unknown",
        &format!(
            "A2A GetTask結果不明。failure_code={} AgentID={} interface_hash={} task_id_hash={}。自動再取得せず、手動再取得にも新Owner確認と一回Permissionを要求",
            failure_code, agent_id, interface_hash, task_id_hash
        ),
        "LIVE_RUNTIME",
        payload_hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_GET_TASK,
                "a2a_audit_append_failed",
                "A2A Task取得の結果不明Auditを確定できない",
            )
        }
    };
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OP_GET_TASK.to_string(),
        status: BrokerStatus::Suspended,
        evidence_source: "LIVE_RUNTIME".to_string(),
        audit_event_id: event.event_id,
        error: Some(super::protocol::BrokerError {
            code: "a2a_task_query_outcome_unknown".to_string(),
            message: "A2A Taskの状態を確認できませんでした。自動再取得はせず、再試行には新しいOwner確認が必要です。".to_string(),
            recoverable: true,
            audit_event_required: true,
            fail_closed: true,
        }),
        health: None,
        body: Some(body),
        shutdown_requested: broker.shutdown_requested,
    }
}

fn reject_send_message(
    broker: &mut Broker,
    request_id: &str,
    error: A2aError,
    payload_hash: &str,
) -> BrokerResponse {
    broker.reject_with_payload_hash(
        request_id,
        OP_SEND_MESSAGE,
        error.code,
        &error.message,
        true,
        payload_hash,
    )
}

fn unknown_send_message(
    broker: &mut Broker,
    request_id: &str,
    agent_id: &str,
    interface_hash: &str,
    message_hash: &str,
    payload_hash: &str,
    failure_code: &str,
) -> BrokerResponse {
    let body = json!({
        "版": VERSION,
        "AgentID": agent_id,
        "interface_hash": interface_hash,
        "message_hash": message_hash,
        "status": "unknown",
        "result": Value::Null,
        "Trust": "pending_review",
        "権限生成": "なし",
        "authority_strip": true,
        "公開範囲": "metadata_only",
        "再送可能": false
    });
    let event = match broker.append_audit(
        request_id,
        OP_SEND_MESSAGE,
        "unknown",
        &format!(
            "A2A SendMessageの外部結果は不明。failure_code={} AgentID={} interface_hash={} message_hash={}。再送禁止",
            failure_code, agent_id, interface_hash, message_hash
        ),
        "LIVE_RUNTIME",
        payload_hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_SEND_MESSAGE,
                "a2a_audit_append_failed",
                "A2A Message送信の結果不明Auditを確定できない。再送禁止",
            )
        }
    };
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OP_SEND_MESSAGE.to_string(),
        status: BrokerStatus::Suspended,
        evidence_source: "LIVE_RUNTIME".to_string(),
        audit_event_id: event.event_id,
        error: Some(super::protocol::BrokerError {
            code: "a2a_message_outcome_unknown".to_string(),
            message: "外部AgentへのMessage送信結果は不明です。二重送信防止のため自動再送しません。"
                .to_string(),
            recoverable: false,
            audit_event_required: true,
            fail_closed: true,
        }),
        health: None,
        body: Some(body),
        shutdown_requested: broker.shutdown_requested,
    }
}

pub(crate) fn recover_interrupted_sends(broker: &mut Broker) -> Result<(), BrokerStoreError> {
    let mut started = BTreeMap::<String, String>::new();
    let mut terminal = std::collections::BTreeSet::<String>::new();
    for event in broker.audit_log.events() {
        if event.operation != OP_SEND_MESSAGE {
            continue;
        }
        if event.decision == "started" {
            started.insert(event.request_id.clone(), event.payload_hash.clone());
        } else if ["accepted", "unknown", "rejected", "suspended"]
            .contains(&event.decision.as_str())
        {
            terminal.insert(event.request_id.clone());
        }
    }
    for (request_id, payload_hash) in started {
        if terminal.contains(&request_id) {
            continue;
        }
        broker.append_audit(
            &request_id,
            OP_SEND_MESSAGE,
            "unknown",
            "Broker再起動時にA2A SendMessageの終端Auditがないため結果不明として隔離。自動再送しない",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &payload_hash,
        )?;
    }
    Ok(())
}

pub(crate) fn recover_interrupted_get_tasks(broker: &mut Broker) -> Result<(), BrokerStoreError> {
    let mut started = BTreeMap::<String, String>::new();
    let mut terminal = std::collections::BTreeSet::<String>::new();
    for event in broker.audit_log.events() {
        if event.operation != OP_GET_TASK {
            continue;
        }
        if event.decision == "started" {
            started.insert(event.request_id.clone(), event.payload_hash.clone());
        } else if ["accepted", "unknown", "rejected", "suspended"]
            .contains(&event.decision.as_str())
        {
            terminal.insert(event.request_id.clone());
        }
    }
    for (request_id, payload_hash) in started {
        if terminal.contains(&request_id) {
            continue;
        }
        broker.append_audit(
            &request_id,
            OP_GET_TASK,
            "unknown",
            "Broker再起動時にA2A GetTaskの終端Auditがないため結果不明として隔離。自動再取得しない",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &payload_hash,
        )?;
    }
    Ok(())
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
        || crate::a2a::transport_name(&request.agent_card_uri).ok()
            != Some(request.transport.as_str())
    {
        return Err(A2aError::new(
            "a2a_connection_request_invalid",
            "A2A接続payloadの値またはURIとtransportの対応が不正",
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

pub(crate) fn owner_send_message_confirmation_summary(
    payload: &Value,
) -> Result<(String, String, String, String), String> {
    let request = parse_send_message_request(payload).map_err(|error| error.message)?;
    crate::a2a::transport_name(&request.interface_uri).map_err(|error| error.message)?;
    let message_hash = crate::audit_hash::sha256_tagged(request.message.as_bytes());
    Ok((
        request.agent_id,
        request.interface_uri,
        request.message,
        message_hash,
    ))
}

pub(crate) fn owner_get_task_confirmation_summary(
    payload: &Value,
) -> Result<(String, String, String, Option<String>, Option<String>), String> {
    let request = parse_get_task_request(payload).map_err(|error| error.message)?;
    crate::a2a::transport_name(&request.interface_uri).map_err(|error| error.message)?;
    Ok((
        request.agent_id,
        request.interface_uri,
        request.task_id,
        request.artifact_id,
        request.artifact_hash,
    ))
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

    #[test]
    fn A2A_GetTask再起動時startedだけならunknownへ一度だけ閉じる() {
        let store = TestStore::new("a2a-get-task-recovery");
        let request_id = "a2a-get-task-interrupted";
        let payload_hash =
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let mut broker =
            Broker::new_persistent("a2a-get-task-before-restart", &store.0).expect("永続Broker");
        broker
            .append_audit(
                request_id,
                OP_GET_TASK,
                "started",
                "fixture started task_id_hash=sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                payload_hash,
            )
            .expect("GetTask開始Audit");
        drop(broker);

        let recovered =
            Broker::new_persistent("a2a-get-task-after-restart", &store.0).expect("再起動後の復旧");
        let recovery_events: Vec<_> = recovered
            .audit_events()
            .iter()
            .filter(|event| {
                event.request_id == request_id
                    && event.operation == OP_GET_TASK
                    && event.decision == "unknown"
            })
            .collect();
        assert_eq!(recovery_events.len(), 1);
        assert!(!recovery_events[0].reason.contains("fixture-task-id"));
        drop(recovered);

        let reopened = Broker::new_persistent("a2a-get-task-after-recovery", &store.0)
            .expect("recovery idempotence");
        assert_eq!(
            reopened
                .audit_events()
                .iter()
                .filter(|event| {
                    event.request_id == request_id
                        && event.operation == OP_GET_TASK
                        && event.decision == "unknown"
                })
                .count(),
            1
        );
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
    fn A2A接続要求はURIと一致するtransportだけを受理する() {
        let mut authority = valid_payload();
        authority["AgentID"] = Value::String("agent/authority".to_string());
        assert_eq!(
            parse_request(&authority).expect_err("AgentID不正").code,
            "a2a_connection_request_invalid"
        );
        let mut https = valid_payload();
        https["Agent Card URI"] =
            Value::String("https://agent.example/.well-known/agent-card.json".to_string());
        https["Transport"] = Value::String("https".to_string());
        assert_eq!(parse_request(&https).unwrap().transport, "https");
        https["Transport"] = Value::String("http".to_string());
        assert_eq!(
            parse_request(&https)
                .expect_err("URIと不一致のtransport")
                .code,
            "a2a_connection_request_invalid"
        );
    }

    #[test]
    fn A2A_send_messageはnative_owner必須でAuditが本文を保存しない() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let store = TestStore::new("a2a-send-positive");
        let mut broker = Broker::new_persistent("a2a-send-positive", &store.0).expect("永続Broker");
        let listener = TcpListener::bind("127.0.0.1:0").expect("Agent Card／Message fixture");
        let address = listener.local_addr().expect("fixture address");
        let card_uri = format!("http://{address}/.well-known/agent-card.json");
        let interface_uri = format!("http://{address}/rpc");
        let remote_artifact = json!({
            "artifactId": "fixture-artifact-1",
            "parts": [{"kind": "text", "text": "private-get-task-marker"}],
            "metadata": {"private": "artifact-metadata-marker"}
        });
        let remote_artifact_hash = crate::audit_hash::sha256_tagged(
            &serde_json::to_vec(&remote_artifact).expect("fixture Artifact serialization"),
        );
        let worker_artifact = remote_artifact.clone();
        let worker = std::thread::spawn(move || -> Result<(Value, Value, Value, Value), String> {
            fn read_request(stream: &mut std::net::TcpStream) -> Result<(String, Vec<u8>), String> {
                let mut bytes = Vec::new();
                let mut chunk = [0u8; 1024];
                let header_end = loop {
                    let size = stream.read(&mut chunk).map_err(|error| error.to_string())?;
                    if size == 0 || bytes.len().saturating_add(size) > 20 * 1024 {
                        return Err("HTTP request header/body truncated or oversized".into());
                    }
                    bytes.extend_from_slice(&chunk[..size]);
                    if let Some(position) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                        break position + 4;
                    }
                };
                let header = std::str::from_utf8(&bytes[..header_end])
                    .map_err(|error| error.to_string())?
                    .to_owned();
                let length = header
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("Content-Length: ")
                            .and_then(|value| value.parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                while bytes.len() < header_end + length {
                    let size = stream.read(&mut chunk).map_err(|error| error.to_string())?;
                    if size == 0 {
                        return Err("HTTP request body truncated".into());
                    }
                    bytes.extend_from_slice(&chunk[..size]);
                }
                if bytes.len() != header_end + length {
                    return Err("HTTP request contains trailing bytes".into());
                }
                Ok((header, bytes[header_end..].to_vec()))
            }
            fn respond(stream: &mut std::net::TcpStream, body: &[u8]) -> Result<(), String> {
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream
                    .write_all(header.as_bytes())
                    .map_err(|error| error.to_string())?;
                stream.write_all(body).map_err(|error| error.to_string())?;
                stream.flush().map_err(|error| error.to_string())?;
                stream
                    .shutdown(std::net::Shutdown::Write)
                    .map_err(|error| error.to_string())?;
                let mut close = [0u8; 1];
                match stream.read(&mut close) {
                    Ok(0) => Ok(()),
                    Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => Ok(()),
                    Ok(_) => Err("client sent unexpected data after response".into()),
                    Err(error) => Err(error.to_string()),
                }
            }

            let (mut card_stream, _) = listener.accept().map_err(|error| error.to_string())?;
            card_stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            card_stream
                .set_write_timeout(Some(std::time::Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            let (card_header, card_body) = read_request(&mut card_stream)?;
            if !card_header.starts_with("GET /.well-known/agent-card.json HTTP/1.1\r\n")
                || !card_body.is_empty()
            {
                return Err("Agent Card GET mismatch".into());
            }
            let card = serde_json::to_vec(&json!({
                "name":"A2A Send fixture", "description":"bounded fixture", "version":"1.0.0",
                "supportedInterfaces":[{"url":format!("http://{address}/rpc"),"protocolBinding":"JSONRPC","protocolVersion":"1.0"}],
                "capabilities":{"streaming":false,"pushNotifications":false,"stateTransitionHistory":false,"extendedAgentCard":false},
                "securitySchemes":{},"skills":[]
            }))
            .map_err(|error| error.to_string())?;
            respond(&mut card_stream, &card)?;

            let (mut send_stream, _) = listener.accept().map_err(|error| error.to_string())?;
            send_stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            send_stream
                .set_write_timeout(Some(std::time::Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            let (send_header, send_body) = read_request(&mut send_stream)?;
            if !send_header.starts_with("POST /rpc HTTP/1.1\r\n")
                || !send_header
                    .lines()
                    .any(|line| line.eq_ignore_ascii_case("A2A-Version: 1.0"))
                || !send_header
                    .lines()
                    .any(|line| line.eq_ignore_ascii_case("Content-Type: application/json"))
            {
                return Err("A2A v1.0 SendMessage header mismatch".into());
            }
            let request: Value =
                serde_json::from_slice(&send_body).map_err(|error| error.to_string())?;
            if request["method"] != "SendMessage"
                || request["params"]["message"]["parts"][0]["text"] != "fixture user message"
            {
                return Err("SendMessage method or message mismatch".into());
            }
            let response = serde_json::to_vec(&json!({
                "jsonrpc":"2.0", "id":request["id"], "result":{"task":{
                    "kind":"task", "id":"fixture-task-1", "contextId":"fixture-context-1",
                    "status":{"state":"TASK_STATE_SUBMITTED","message":{"kind":"message","messageId":"fixture-response","role":"ROLE_AGENT","parts":[{"kind":"text","text":"private-response-marker"}]}},
                    "artifacts":[],"history":[]
                }}
            }))
            .map_err(|error| error.to_string())?;
            respond(&mut send_stream, &response)?;

            let (mut unknown_stream, _) = listener.accept().map_err(|error| error.to_string())?;
            unknown_stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            unknown_stream
                .set_write_timeout(Some(std::time::Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            let (unknown_header, unknown_body) = read_request(&mut unknown_stream)?;
            if !unknown_header.starts_with("POST /rpc HTTP/1.1\r\n") {
                return Err("uncertain SendMessage HTTP method mismatch".into());
            }
            let unknown_request: Value =
                serde_json::from_slice(&unknown_body).map_err(|error| error.to_string())?;
            if unknown_request["params"]["message"]["parts"][0]["text"]
                != "uncertain fixture message"
            {
                return Err("uncertain SendMessage body mismatch".into());
            }
            let stale_response = serde_json::to_vec(&json!({
                "jsonrpc":"2.0", "id":"stale-request-id", "result":{"message":{
                    "kind":"message", "messageId":"stale-response",
                    "role":"ROLE_AGENT", "parts":[{"kind":"text","text":"uncertain-response-private"}]
                }}
            }))
            .map_err(|error| error.to_string())?;
            respond(&mut unknown_stream, &stale_response)?;
            let (mut task_stream, _) = listener.accept().map_err(|error| error.to_string())?;
            task_stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            task_stream
                .set_write_timeout(Some(std::time::Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            let (task_header, task_body) = read_request(&mut task_stream)?;
            if !task_header.starts_with("POST /rpc HTTP/1.1\r\n") {
                return Err("A2A GetTask HTTP method mismatch".into());
            }
            let task_request: Value =
                serde_json::from_slice(&task_body).map_err(|error| error.to_string())?;
            if task_request["method"] != "GetTask"
                || task_request["params"]["id"] != "fixture-task-1"
                || task_request["params"]["historyLength"] != 0
            {
                return Err("A2A GetTask parameters mismatch".into());
            }
            let task_response = serde_json::to_vec(&json!({
                "jsonrpc":"2.0", "id":task_request["id"], "result":{
                    "kind":"task", "id":"fixture-task-1", "contextId":"fixture-context-1",
                    "status":{"state":"TASK_STATE_COMPLETED","message":{"parts":[{"text":"private-get-task-marker"}]}},
                    "artifacts":[worker_artifact.clone()], "history":[]
                }
            })).map_err(|error| error.to_string())?;
            respond(&mut task_stream, &task_response)?;

            let (mut selected_stream, _) = listener.accept().map_err(|error| error.to_string())?;
            selected_stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            selected_stream
                .set_write_timeout(Some(std::time::Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            let (selected_header, selected_body) = read_request(&mut selected_stream)?;
            if !selected_header.starts_with("POST /rpc HTTP/1.1\r\n") {
                return Err("selected A2A GetTask HTTP method mismatch".into());
            }
            let selected_task_request: Value =
                serde_json::from_slice(&selected_body).map_err(|error| error.to_string())?;
            if selected_task_request["method"] != "GetTask"
                || selected_task_request["params"]["id"] != "fixture-task-1"
                || selected_task_request["params"]["historyLength"] != 0
            {
                return Err("selected A2A GetTask parameters mismatch".into());
            }
            let selected_response = serde_json::to_vec(&json!({
                "jsonrpc":"2.0", "id":selected_task_request["id"], "result":{
                    "kind":"task", "id":"fixture-task-1", "contextId":"fixture-context-1",
                    "status":{"state":"TASK_STATE_COMPLETED"},
                    "artifacts":[worker_artifact], "history":[]
                }
            }))
            .map_err(|error| error.to_string())?;
            respond(&mut selected_stream, &selected_response)?;
            Ok((
                request,
                unknown_request,
                task_request,
                selected_task_request,
            ))
        });

        let mut connection_payload = valid_payload();
        connection_payload["Agent Card URI"] = Value::String(card_uri);
        let connected = connect(
            &mut broker,
            "a2a-send-connect",
            &connection_payload,
            true,
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        assert_eq!(
            connected.status,
            BrokerStatus::Accepted,
            "{:?}",
            connected.error
        );
        let receipt = broker
            .a2a_connections
            .get("remote-agent-example")
            .expect("connected receipt");
        assert_eq!(
            receipt["Agent Card"]["supported_interfaces"][0]["status"],
            "supported"
        );
        assert_eq!(receipt["Trust"]["state"], "pending_review");

        let secret_message = "fixture user message";
        let send_payload = json!({
            "版": 1,
            "操作": "送信",
            "AgentID": "remote-agent-example",
            "Interface URI": interface_uri,
            "Message": secret_message
        });
        let response = send_message(
            &mut broker,
            "a2a-send-positive-request",
            &send_payload,
            true,
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        let mut uncertain_payload = send_payload.clone();
        uncertain_payload["Message"] = Value::String("uncertain fixture message".into());
        let uncertain = send_message(
            &mut broker,
            "a2a-send-unknown-request",
            &uncertain_payload,
            true,
            "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        );
        let task_payload = json!({
            "版": 1,
            "操作": "状態取得",
            "AgentID": "remote-agent-example",
            "Interface URI": interface_uri,
            "TaskID": "fixture-task-1"
        });
        let task_result = get_task(
            &mut broker,
            "a2a-get-task-positive-request",
            &task_payload,
            true,
            "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        );
        let artifact_payload = json!({
            "版": 1,
            "操作": "状態取得",
            "AgentID": "remote-agent-example",
            "Interface URI": interface_uri,
            "TaskID": "fixture-task-1",
            "ArtifactID": "fixture-artifact-1",
            "Artifact hash": remote_artifact_hash
        });
        let artifact_result = get_task(
            &mut broker,
            "a2a-get-task-artifact-positive-request",
            &artifact_payload,
            true,
            "sha256:abababababababababababababababababababababababababababababababab",
        );
        let (request, uncertain_request, task_request, selected_task_request) = worker
            .join()
            .expect("SendMessage fixture thread")
            .expect("fixture requests");
        assert_eq!(request["params"]["message"]["role"], "ROLE_USER");
        assert_eq!(
            response.status,
            BrokerStatus::Accepted,
            "{:?}",
            response.error
        );
        let body = response.body.expect("bounded Message result");
        assert_eq!(body["status"], "accepted");
        assert_eq!(body["Trust"], "pending_review");
        assert_eq!(body["権限生成"], "なし");
        assert_eq!(body["再送可能"], false);
        assert_eq!(body["result"]["kind"], "task");
        assert_eq!(body["result"]["task_id"], "fixture-task-1");
        assert!(!body.to_string().contains(secret_message));
        assert!(!body.to_string().contains("private-response-marker"));
        assert_eq!(uncertain_request["method"], "SendMessage");
        assert_eq!(uncertain.status, BrokerStatus::Suspended);
        assert_eq!(
            uncertain.error.as_ref().map(|error| error.code.as_str()),
            Some("a2a_message_outcome_unknown")
        );
        let uncertain_body = uncertain.body.expect("unknown Message result");
        assert_eq!(uncertain_body["status"], "unknown");
        assert_eq!(uncertain_body["result"], Value::Null);
        assert_eq!(uncertain_body["再送可能"], false);
        assert!(!uncertain_body
            .to_string()
            .contains("uncertain fixture message"));
        assert!(!uncertain_body
            .to_string()
            .contains("uncertain-response-private"));
        assert_eq!(task_result.status, BrokerStatus::Accepted);
        assert_eq!(task_request["method"], "GetTask");
        let task_body = task_result.body.expect("bounded GetTask response");
        assert_eq!(task_body["status"], "accepted");
        assert_eq!(task_body["result"]["task_id"], "fixture-task-1");
        assert_eq!(task_body["result"]["state"], "completed");
        assert_eq!(task_body["result"]["history_count"], 0);
        assert_eq!(task_body["履歴公開"], false);
        assert_eq!(task_body["Credential使用"], false);
        assert!(!task_body.to_string().contains("private-get-task-marker"));
        assert!(!task_body.to_string().contains("artifact-metadata-marker"));
        assert_eq!(
            task_body["result"]["artifact_manifest"][0]["artifact_id"],
            "fixture-artifact-1"
        );
        assert_eq!(task_body["result"]["selected_artifact"], Value::Null);
        assert_eq!(selected_task_request["method"], "GetTask");
        assert_eq!(artifact_result.status, BrokerStatus::Accepted);
        let artifact_body = artifact_result.body.expect("explicit selected Artifact");
        assert_eq!(artifact_body["公開範囲"], "full");
        assert_eq!(
            artifact_body["result"]["selected_artifact"]["parts"][0],
            "private-get-task-marker"
        );
        assert_eq!(
            artifact_body["result"]["selected_artifact"]["artifact_hash"],
            remote_artifact_hash
        );
        assert!(!artifact_body
            .to_string()
            .contains("artifact-metadata-marker"));
        assert!(broker.audit_events().iter().all(|event| {
            !event.reason.contains(secret_message)
                && !event.reason.contains("uncertain fixture message")
                && !event.reason.contains("private-response-marker")
                && !event.reason.contains("uncertain-response-private")
                && !event.reason.contains("private-get-task-marker")
                && !event.reason.contains("artifact-metadata-marker")
                && !event.reason.contains("fixture-task-1")
                && !event.reason.contains(&interface_uri)
        }));
        for decision in [
            "received",
            "owner_confirmed",
            "permission_issued",
            "started",
            "accepted",
            "unknown",
        ] {
            assert!(
                broker.audit_events().iter().any(|event| {
                    event.operation == OP_SEND_MESSAGE && event.decision == decision
                }),
                "missing A2A send Audit decision: {decision}"
            );
        }
        assert!(broker
            .audit_events()
            .iter()
            .any(|event| { event.operation == OP_GET_TASK && event.decision == "accepted" }));
        assert_eq!(
            broker.a2a_connections["remote-agent-example"]["Trust"]["state"],
            "pending_review"
        );

        let denied_task = get_task(
            &mut broker,
            "a2a-get-task-no-owner",
            &task_payload,
            false,
            "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        );
        assert_eq!(denied_task.status, BrokerStatus::Rejected);
        assert_eq!(
            denied_task.error.as_ref().map(|error| error.code.as_str()),
            Some("a2a_native_owner_required")
        );

        let denied = send_message(
            &mut broker,
            "a2a-send-no-owner",
            &send_payload,
            false,
            "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        );
        assert_eq!(denied.status, BrokerStatus::Rejected);
        assert_eq!(
            denied.error.as_ref().map(|error| error.code.as_str()),
            Some("a2a_native_owner_required")
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
