//! C9 MCP接続センターのBroker経路。
//!
//! 現行単位はowner起動設定からstdio Serverを接続し、discoveryとTool/Resource/Prompt
//! catalogを検証してmetadata-only receiptへ射影する。Tool実行、Credential実値注入、
//! OAuth、Streamable HTTPはこの単位では接続しない。
#![allow(non_snake_case)]

use super::protocol::{Broker, BrokerResponse, BrokerStatus, EVIDENCE_SOURCE_INTERNAL_STATE};
use crate::adapters::mcp_stdio::McpStdioConnection;
use crate::mcp::McpError;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

const OP_CONNECT: &str = "MCP接続";
const OP_LIST: &str = "MCP接続一覧";
const VERSION: u64 = 1;
const MAX_ARGUMENTS: usize = 32;
const MAX_ARGUMENT_BYTES: usize = 1024;

#[derive(Debug)]
pub(super) struct McpConnectionEntry {
    pub(super) connection: McpStdioConnection,
    pub(super) projection: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "操作")]
    operation: String,
    #[serde(rename = "ServerID")]
    server_id: String,
    #[serde(rename = "実行file")]
    executable: String,
    #[serde(rename = "引数")]
    arguments: Vec<String>,
    #[serde(rename = "workspace")]
    workspace: String,
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
            "mcp_owner_required",
            "MCP接続にはowner controlが必要",
            true,
            payload_hash,
        );
    }
    if !broker.state_store.persistence_ready() {
        return broker.reject_with_payload_hash(
            request_id,
            OP_CONNECT,
            "broker_persistence_unavailable",
            "MCP接続には永続Auditが必要",
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
                error.message,
                true,
                payload_hash,
            )
        }
    };
    if broker.mcp_connections.contains_key(&request.server_id) {
        return broker.reject_with_payload_hash(
            request_id,
            OP_CONNECT,
            "mcp_connection_duplicate",
            "同じMCP Server IDを再接続できない",
            true,
            payload_hash,
        );
    }
    if broker
        .append_audit(
            request_id,
            OP_CONNECT,
            "received",
            "owner controlからMCP接続要求を受信。実行file、引数、Credential実値はAuditへ保存しない",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_CONNECT,
            "mcp_audit_append_failed",
            "MCP接続の受信Auditを確定できない",
        );
    }

    let connection = match McpStdioConnection::connect(
        Path::new(&request.executable),
        &request.arguments,
        Path::new(&request.workspace),
    ) {
        Ok(connection) => connection,
        Err(error) => return reject_mcp_error(broker, request_id, error, payload_hash),
    };
    let endpoint_hash = crate::audit_hash::sha256_tagged(
        format!(
            "{}\n{}\n{}",
            request.executable,
            request.workspace,
            request.arguments.join("\n")
        )
        .as_bytes(),
    );
    let mut projection = match connection.catalog().to_metadata_projection(
        &request.server_id,
        &request.transport,
        &endpoint_hash,
        request.credential_ref.clone(),
    ) {
        Ok(value) => value,
        Err(error) => return reject_mcp_error(broker, request_id, error, payload_hash),
    };
    {
        let object = projection.as_object_mut().expect("MCP projectionはobject");
        object.insert(
            "接続状態".to_string(),
            Value::String("connected".to_string()),
        );
        object.insert(
            "能力ID".to_string(),
            Value::String("mcp.connection.connect".to_string()),
        );
        object.insert(
            "権限ID".to_string(),
            Value::String("permission.mcp.connection.connect".to_string()),
        );
        object.insert(
            "承認状態".to_string(),
            Value::String("owner_control_approved".to_string()),
        );
        object.insert(
            "復旧ID".to_string(),
            Value::String("recover-mcp-connection".to_string()),
        );
    }
    let body = projection.clone();
    let accepted = match broker.append_audit(
        request_id,
        OP_CONNECT,
        "accepted",
        &format!("MCP接続metadataを受理。ServerID={}", request.server_id),
        EVIDENCE_SOURCE_INTERNAL_STATE,
        &super::protocol::canonical_payload_hash(Some(&body)),
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_CONNECT,
                "mcp_audit_append_failed",
                "MCP接続の受理Auditを確定できない。接続を破棄した",
            )
        }
    };
    projection
        .as_object_mut()
        .expect("MCP projectionはobject")
        .insert(
            "接続監査ID".to_string(),
            Value::String(accepted.event_id.clone()),
        );
    broker.mcp_connections.insert(
        request.server_id,
        McpConnectionEntry {
            connection,
            projection: projection.clone(),
        },
    );
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OP_CONNECT.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
        audit_event_id: accepted.event_id,
        error: None,
        health: None,
        body: Some(projection),
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
            "mcp_normal_channel_required",
            "MCP接続一覧は通常IPCだけが参照できる",
            true,
            payload_hash,
        );
    }
    if payload != &json!({"版": VERSION}) {
        return broker.reject_with_payload_hash(
            request_id,
            OP_LIST,
            "mcp_list_request_invalid",
            "MCP接続一覧は版だけを受け付ける",
            true,
            payload_hash,
        );
    }
    if broker
        .append_audit(
            request_id,
            OP_LIST,
            "received",
            "MCP接続metadata一覧を要求",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_LIST,
            "mcp_audit_append_failed",
            "MCP接続一覧の受信Auditを確定できない",
        );
    }
    if broker
        .mcp_connections
        .values_mut()
        .any(|entry| !entry.connection.is_alive())
    {
        return broker.reject_with_payload_hash(
            request_id,
            OP_LIST,
            "mcp_server_unavailable",
            "MCP Serverの接続状態を確認できないため一覧を返さない",
            true,
            payload_hash,
        );
    }
    let connections: Vec<Value> = broker
        .mcp_connections
        .values()
        .map(|entry| entry.projection.clone())
        .collect();
    let body = json!({
        "版": VERSION,
        "MCP接続一覧": connections,
        "件数": broker.mcp_connections.len(),
        "公開範囲": "metadata_only",
        "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
    });
    let event = match broker.append_audit(
        request_id,
        OP_LIST,
        "accepted",
        "MCP接続metadata一覧を返却。Tool実行資格やCredential実値は返さない",
        EVIDENCE_SOURCE_INTERNAL_STATE,
        &super::protocol::canonical_payload_hash(Some(&body)),
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_LIST,
                "mcp_audit_append_failed",
                "MCP接続一覧の結果Auditを確定できない",
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

fn parse_request(payload: &Value) -> Result<ConnectionRequest, McpError> {
    let request: ConnectionRequest = serde_json::from_value(payload.clone()).map_err(|_| {
        McpError::new(
            "mcp_connection_request_invalid",
            "MCP接続payloadの構造が不正",
        )
    })?;
    if request.version != VERSION
        || request.operation != "接続"
        || request.server_id.is_empty()
        || request.server_id.as_bytes().len() > 128
        || request.executable.is_empty()
        || !Path::new(&request.executable).is_absolute()
        || request.workspace.is_empty()
        || !Path::new(&request.workspace).is_absolute()
        || request.transport != "stdio"
        || request.arguments.len() > MAX_ARGUMENTS
        || request.arguments.iter().any(|value| {
            value.is_empty()
                || value.as_bytes().len() > MAX_ARGUMENT_BYTES
                || value.chars().any(char::is_control)
        })
    {
        return Err(McpError::new(
            "mcp_connection_request_invalid",
            "MCP接続payloadの値またはstdio境界が不正",
        ));
    }
    let credential = request.credential_ref.as_object().ok_or_else(|| {
        McpError::new(
            "mcp_credential_ref_invalid",
            "MCP Credential refがobjectではない",
        )
    })?;
    let required = credential
        .get("required")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            McpError::new(
                "mcp_credential_ref_invalid",
                "MCP Credential ref requiredが不正",
            )
        })?;
    let status = credential
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            McpError::new(
                "mcp_credential_ref_invalid",
                "MCP Credential ref statusが不正",
            )
        })?;
    if required || status != "missing" {
        return Err(McpError::new(
            "mcp_credential_unavailable",
            "現行C9 stdio接続はCredential実値注入を未接続のため資格情報必須Serverを接続しない",
        ));
    }
    Ok(request)
}

fn reject_mcp_error(
    broker: &mut Broker,
    request_id: &str,
    error: McpError,
    payload_hash: &str,
) -> BrokerResponse {
    broker.reject_with_payload_hash(
        request_id,
        OP_CONNECT,
        error.code,
        error.message,
        true,
        payload_hash,
    )
}
