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
        let agent_id = validate_persisted_receipt(value)?;
        let mut restored = value.clone();
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
        if object.len() != 4
            || object
                .keys()
                .any(|key| !["binding", "transport", "status", "reason"].contains(&key.as_str()))
            || object.get("binding").and_then(Value::as_str).is_none()
            || object.get("transport").and_then(Value::as_str).is_none()
            || object.get("status").and_then(Value::as_str).is_none()
            || object.get("reason").and_then(Value::as_str).is_none()
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
    if broker.a2a_connections.contains_key(&request.agent_id) {
        return broker.reject_with_payload_hash(
            request_id,
            OP_CONNECT,
            "a2a_connection_duplicate",
            "同じA2A Agent IDを再接続できない",
            true,
            payload_hash,
        );
    }
    if broker.a2a_connections.len() >= MAX_CONNECTIONS {
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
            "owner controlからA2A Agent Card取得要求を受信。URI実値とCredential実値はAuditへ保存しない",
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
            "A2A Agent Card metadataを受理。AgentID={}",
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
        broker.a2a_connections.remove(&request.agent_id);
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
            "supportedInterfaces": [{"protocolBinding": "JSONRPC", "protocolVersion": "1.0"}],
            "capabilities": {"streaming": false, "pushNotifications": false, "stateTransitionHistory": false, "extendedAgentCard": false},
            "securitySchemes": {},
            "skills": []
        }))
        .expect("Agent Card JSON");
        let response_length = response_body.len();
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("Agent Card要求");
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .expect("要求読取期限");
            let mut request = Vec::new();
            let mut chunk = [0u8; 512];
            while !request.windows(4).any(|value| value == b"\r\n\r\n") {
                let size = stream.read(&mut chunk).expect("要求読取");
                assert!(size > 0);
                request.extend_from_slice(&chunk[..size]);
                assert!(request.len() < 8192);
            }
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {response_length}\r\nConnection: close\r\n\r\n"
            );
            let mut response = header.into_bytes();
            response.extend_from_slice(&response_body);
            stream.write_all(&response).expect("Agent Card応答");
            stream.flush().expect("応答flush");
            stream
                .shutdown(std::net::Shutdown::Write)
                .expect("応答側shutdown");
            let mut client_close = [0; 1];
            assert_eq!(stream.read(&mut client_close).expect("相手側終了"), 0);
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
        worker.join().expect("Agent Card試験worker");
        assert_eq!(
            connected.status,
            BrokerStatus::Accepted,
            "{:?}",
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
}
