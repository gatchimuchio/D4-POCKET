//! C9 MCP接続センターのBroker経路。
//!
//! 現行単位はowner起動設定からstdio Serverを接続し、discoveryとTool/Resource/Prompt
//! catalogを検証してmetadata-only receiptへ射影する。Tool実行とWindows stdio childへの
//! Credential実値注入は、それぞれのnative Owner確認とBroker統治に限定する。OAuthと
//! Streamable HTTPはこの単位では接続しない。
#![allow(non_snake_case)]

use super::protocol::{
    Broker, BrokerResponse, BrokerStatus, EVIDENCE_SOURCE_INTERNAL_STATE,
    EVIDENCE_SOURCE_LIVE_RUNTIME,
};
use crate::adapters::mcp_stdio::McpStdioConnection;
use crate::mcp::McpError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

const OP_CONNECT: &str = "MCP接続";
const OP_DISCONNECT: &str = "MCP切断";
const OP_LIST: &str = "MCP接続一覧";
const OP_TOOL_CALL: &str = "MCP Tool実行";
const VERSION: u64 = 1;
const MAX_ARGUMENTS: usize = 32;
const MAX_ARGUMENT_BYTES: usize = 1024;

#[derive(Debug)]
pub(super) struct McpConnectionEntry {
    pub(super) connection: McpStdioConnection,
    pub(super) projection: Value,
    pub(super) quarantined: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CredentialReference {
    credential_id: String,
    purpose: String,
    target: String,
    required: bool,
    status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    environment_variable: Option<String>,
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
    credential_ref: CredentialReference,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DisconnectRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "操作")]
    operation: String,
    #[serde(rename = "ServerID")]
    server_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolCallRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "操作")]
    operation: String,
    #[serde(rename = "ServerID")]
    server_id: String,
    #[serde(rename = "ToolID")]
    tool_id: String,
    #[serde(rename = "名前")]
    name: String,
    #[serde(rename = "arguments")]
    arguments: Value,
}

struct OneShotMcpPermission {
    id: String,
    server_id: String,
    tool_id: String,
    arguments_hash: String,
    consumed: bool,
}

impl OneShotMcpPermission {
    fn consume(&mut self, request: &ToolCallRequest, arguments_hash: &str) -> bool {
        if self.consumed
            || self.server_id != request.server_id
            || self.tool_id != request.tool_id
            || self.arguments_hash != arguments_hash
        {
            return false;
        }
        self.consumed = true;
        true
    }
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

    let mut credential_environment = Vec::new();
    if request.credential_ref.status == "configured" {
        #[cfg(windows)]
        {
            let secret = match broker.資格情報MCP使用処理(
                request_id,
                &request.credential_ref.credential_id,
                &request.server_id,
                payload_hash,
            ) {
                Ok(secret) => secret,
                Err(response) => return response,
            };
            let secret_value = match String::from_utf8(secret.as_bytes().to_vec()) {
                Ok(value) => zeroize::Zeroizing::new(value),
                Err(error) => {
                    let _invalid_secret_bytes = zeroize::Zeroizing::new(error.into_bytes());
                    return broker.reject_with_payload_hash(
                        request_id,
                        OP_CONNECT,
                        "mcp_credential_encoding_invalid",
                        "保管Credentialの文字encodingが不正",
                        true,
                        payload_hash,
                    );
                }
            };
            credential_environment.push((
                request
                    .credential_ref
                    .environment_variable
                    .as_deref()
                    .expect("configured Credentialは環境変数名検証済み")
                    .to_string(),
                secret_value,
            ));
        }
        #[cfg(not(windows))]
        {
            return broker.reject_with_payload_hash(
                request_id,
                OP_CONNECT,
                "credential_platform_unsupported",
                "MCP Credential注入はWindows DPAPI環境だけに対応しています",
                true,
                payload_hash,
            );
        }
    }

    let mut connection = match McpStdioConnection::connect_with_environment(
        Path::new(&request.executable),
        &request.arguments,
        Path::new(&request.workspace),
        &credential_environment,
    ) {
        Ok(connection) => connection,
        Err(error) => return reject_mcp_error(broker, request_id, error, payload_hash),
    };
    if request.credential_ref.status == "configured" {
        #[cfg(windows)]
        if broker
            .資格情報MCP使用確定処理(
                request_id,
                &request.credential_ref.credential_id,
                &request.server_id,
                request
                    .credential_ref
                    .environment_variable
                    .as_deref()
                    .expect("configured Credentialは環境変数名検証済み"),
                payload_hash,
            )
            .is_err()
        {
            let _ = connection.terminate();
            return broker.audit_store_failed_response(
                request_id,
                OP_CONNECT,
                "credential_audit_append_failed",
                "MCP Credential使用Auditを確定できず、process群を停止しました",
            );
        }
    }
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
        serde_json::to_value(&request.credential_ref)
            .expect("検証済みCredential refはJSON値へ変換できる"),
    ) {
        Ok(value) => value,
        Err(error) => return reject_mcp_error(broker, request_id, error, payload_hash),
    };
    {
        let object = projection.as_object_mut().expect("MCP projectionはobject");
        // 発見時の外部metadataではなく、Brokerが保存する接続receiptへ分類する。
        object.insert("証拠種別".to_string(), Value::String(EVIDENCE_SOURCE_INTERNAL_STATE.to_string()));
        object.insert(
            "接続状態".to_string(),
            Value::String("connected".to_string()),
        );
        object.insert("実行状態".to_string(), Value::String("ready".to_string()));
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
            quarantined: false,
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
        .any(|entry| !entry.quarantined && !entry.connection.is_alive())
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
        .map(|entry| {
            let mut projection = entry.projection.clone();
            projection
                .as_object_mut()
                .expect("MCP接続projectionはobject")
                .insert(
                    "証拠種別".to_string(),
                    Value::String(EVIDENCE_SOURCE_INTERNAL_STATE.to_string()),
                );
            projection
                .as_object_mut()
                .expect("MCP接続projectionはobject")
                .insert(
                    "実行状態".to_string(),
                    Value::String(
                        if entry.quarantined {
                            "quarantined"
                        } else {
                            "ready"
                        }
                        .to_string(),
                    ),
                );
            projection
        })
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

#[cfg(not(any(windows, target_os = "macos")))]
pub(super) fn call_tool(
    broker: &mut Broker,
    request_id: &str,
    _payload: &Value,
    _owner: bool,
    payload_hash: &str,
) -> BrokerResponse {
    broker.reject_with_payload_hash(
        request_id,
        OP_TOOL_CALL,
        "mcp_tool_call_platform_unsupported",
        "MCP Tool実行はWindows native Owner確認経路だけに対応しています",
        true,
        payload_hash,
    )
}

#[cfg(any(windows, target_os = "macos"))]
pub(super) fn call_tool(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    owner: bool,
    payload_hash: &str,
) -> BrokerResponse {
    if !owner {
        return broker.reject_with_payload_hash(
            request_id,
            OP_TOOL_CALL,
            "mcp_owner_required",
            "MCP Tool実行にはDesktop native Owner確認が必要",
            true,
            payload_hash,
        );
    }
    if !broker.state_store.persistence_ready() {
        return broker.reject_with_payload_hash(
            request_id,
            OP_TOOL_CALL,
            "broker_persistence_unavailable",
            "MCP Tool実行には永続Auditが必要",
            true,
            payload_hash,
        );
    }
    let request = match parse_tool_call_request(payload) {
        Ok(request) => request,
        Err(error) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_TOOL_CALL,
                error.code,
                error.message,
                true,
                payload_hash,
            )
        }
    };
    if broker
        .append_audit(
            request_id,
            OP_TOOL_CALL,
            "received",
            "Rust起動器のnative Owner確認後にTool要求を受信。引数本文は記録しない",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_TOOL_CALL,
            "mcp_audit_append_failed",
            "MCP Tool要求の受信Auditを確定できない",
        );
    }

    let arguments_hash = crate::audit_hash::sha256_tagged(
        &serde_json::to_vec(&request.arguments).expect("JSON Valueは直列化可能"),
    );
    let preflight = match broker.mcp_connections.get_mut(&request.server_id) {
        None => Err(McpError::new(
            "mcp_connection_unknown",
            "指定MCP Serverの接続が存在しない",
        )),
        Some(entry) => {
            if entry.quarantined {
                Err(McpError::new(
                    "mcp_connection_quarantined",
                    "MCP接続は隔離中のためTool実行できない。ownerが切断・再接続する",
                ))
            } else if !entry.connection.is_alive() {
                quarantine(entry);
                Err(McpError::new(
                    "mcp_server_unavailable",
                    "MCP Serverが終了したためTool実行できない",
                ))
            } else {
                entry.connection.catalog().validate_tool_identity_and_call(
                    &request.tool_id,
                    &request.name,
                    &request.arguments,
                )
            }
        }
    };
    if let Err(error) = preflight {
        return broker.reject_with_payload_hash(
            request_id,
            OP_TOOL_CALL,
            error.code,
            error.message,
            true,
            payload_hash,
        );
    }

    let mut permission_random = [0u8; 16];
    if getrandom::getrandom(&mut permission_random).is_err() {
        return broker.reject_with_payload_hash(
            request_id,
            OP_TOOL_CALL,
            "mcp_permission_unavailable",
            "一回限りPermissionの識別値を生成できないため送信しない",
            true,
            payload_hash,
        );
    }
    let mut permission = OneShotMcpPermission {
        id: format!("mcp-tool-{}", hex::encode(permission_random)),
        server_id: request.server_id.clone(),
        tool_id: request.tool_id.clone(),
        arguments_hash: arguments_hash.clone(),
        consumed: false,
    };
    let approval_event = match broker.append_audit(
        request_id,
        OP_TOOL_CALL,
        "approved",
        &format!(
            "Capability=mcp.tool.call Permission={} Approval=Rust Desktop native Ownerが現在のServer／Tool／arguments hashを確認 RecoveryAction=inspect-mcp-tool-side-effect; ServerID={} ToolID={} arguments_hash={}",
            permission.id, request.server_id, request.tool_id, arguments_hash
        ),
        EVIDENCE_SOURCE_INTERNAL_STATE,
        payload_hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_TOOL_CALL,
                "mcp_audit_append_failed",
                "MCP ToolのApproval／Permission Auditを確定できないため送信しない",
            )
        }
    };
    if !permission.consume(&request, &arguments_hash) {
        return broker.reject_with_payload_hash(
            request_id,
            OP_TOOL_CALL,
            "mcp_permission_binding_invalid",
            "一回限りPermissionが現在のServer／Tool／argumentsへ結合されていない",
            true,
            payload_hash,
        );
    }
    if broker
        .append_audit(
            request_id,
            OP_TOOL_CALL,
            "started",
            &format!(
                "Capability=mcp.tool.call Permission={} consumed=true Approval={} RecoveryAction=inspect-mcp-tool-side-effect; MCP tools/callを同一接続へ一度だけ送信",
                permission.id, approval_event.event_id
            ),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_TOOL_CALL,
            "mcp_audit_append_failed",
            "MCP Tool実行開始Auditを確定できないため送信しない",
        );
    }

    let call_result = broker
        .mcp_connections
        .get_mut(&request.server_id)
        .expect("事前検査済みのMCP接続")
        .connection
        .call_tool(&request.tool_id, &request.name, &request.arguments);
    let summary = match call_result {
        Ok(summary) => summary,
        Err(_error) => {
            if let Some(entry) = broker.mcp_connections.get_mut(&request.server_id) {
                quarantine(entry);
            }
            return broker.reject_with_payload_hash(
                request_id,
                OP_TOOL_CALL,
                "mcp_tool_result_unknown",
                "送信後の結果を確定できない。接続を隔離し、外部副作用をownerが確認してから切断・再接続する",
                true,
                payload_hash,
            );
        }
    };
    let connection_alive = broker
        .mcp_connections
        .get_mut(&request.server_id)
        .is_some_and(|entry| entry.connection.is_alive());
    if !connection_alive {
        if let Some(entry) = broker.mcp_connections.get_mut(&request.server_id) {
            quarantine(entry);
        }
    }
    let connection_state = if connection_alive {
        "connected"
    } else {
        "quarantined"
    };
    let content_count = summary.content_types.len();
    let content_types = summary.content_types.clone();
    let result_hash = summary.result_hash.clone();
    let is_error = summary.is_error;
    let body = json!({
        "版": VERSION,
        "契約種別": "MCP Tool実行receipt",
        "ServerID": request.server_id,
        "ToolID": request.tool_id,
        "名前": request.name,
        "arguments_hash": arguments_hash,
        "result_hash": result_hash.clone(),
        "Tool error": is_error,
        "content_count": content_count,
        "content_types": content_types,
        "接続状態": connection_state,
        "能力ID": "mcp.tool.call",
        "権限ID": "permission.mcp.tool.call.one_shot",
        "承認状態": "native_owner_confirmed",
        "承認監査ID": approval_event.event_id.clone(),
        "復旧ID": "inspect-mcp-tool-side-effect",
        "権限生成": "Broker内一回限りPermissionを消費",
        "公開範囲": "hash_only",
        "証拠種別": EVIDENCE_SOURCE_LIVE_RUNTIME,
    });
    let accepted = match broker.append_audit(
        request_id,
        OP_TOOL_CALL,
        "accepted",
        &format!(
            "Capability=mcp.tool.call Permission={} consumed=true Approval={} RecoveryAction=inspect-mcp-tool-side-effect; 結果本文を保持せずhash-only receiptを確定; result_hash={} content_count={} isError={}",
            permission.id,
            approval_event.event_id,
            result_hash,
            content_count,
            is_error
        ),
        EVIDENCE_SOURCE_LIVE_RUNTIME,
        &result_hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            if let Some(entry) = broker.mcp_connections.get_mut(&request.server_id) {
                quarantine(entry);
            }
            return broker.audit_store_failed_response(
                request_id,
                OP_TOOL_CALL,
                "mcp_audit_append_failed",
                "Tool応答後のAuditを確定できない。接続を隔離し結果を未確定としてowner確認を要求する",
            );
        }
    };
    let mut body = body;
    body.as_object_mut()
        .expect("MCP Tool実行receiptはobject")
        .insert(
            "監査ID".to_string(),
            Value::String(accepted.event_id.clone()),
        );
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OP_TOOL_CALL.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: EVIDENCE_SOURCE_LIVE_RUNTIME.to_string(),
        audit_event_id: accepted.event_id,
        error: None,
        health: None,
        body: Some(body),
        shutdown_requested: broker.shutdown_requested,
    }
}

#[cfg(any(windows, target_os = "macos"))]
fn quarantine(entry: &mut McpConnectionEntry) {
    entry.quarantined = true;
    entry.projection["接続状態"] = Value::String("quarantined".to_string());
    entry.projection["実行状態"] = Value::String("quarantined".to_string());
}

pub(super) fn disconnect(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    owner: bool,
    payload_hash: &str,
) -> BrokerResponse {
    if !owner {
        return broker.reject_with_payload_hash(
            request_id,
            OP_DISCONNECT,
            "mcp_owner_required",
            "MCP切断にはowner controlが必要",
            true,
            payload_hash,
        );
    }
    if !broker.state_store.persistence_ready() {
        return broker.reject_with_payload_hash(
            request_id,
            OP_DISCONNECT,
            "broker_persistence_unavailable",
            "MCP切断には永続Auditが必要",
            true,
            payload_hash,
        );
    }
    let request = match parse_disconnect_request(payload) {
        Ok(request) => request,
        Err(error) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_DISCONNECT,
                error.code,
                error.message,
                true,
                payload_hash,
            )
        }
    };
    if !broker.mcp_connections.contains_key(&request.server_id) {
        return broker.reject_with_payload_hash(
            request_id,
            OP_DISCONNECT,
            "mcp_connection_unknown",
            "指定MCP Server IDの接続が存在しない",
            true,
            payload_hash,
        );
    }
    if broker
        .append_audit(
            request_id,
            OP_DISCONNECT,
            "received",
            &format!(
                "Capability=mcp.connection.disconnect Permission=permission.mcp.connection.disconnect Approval=Ownerの明示切断 RecoveryAction=retry-mcp-disconnect; ServerID={}",
                request.server_id
            ),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_DISCONNECT,
            "mcp_audit_append_failed",
            "MCP切断の受信Auditを確定できない",
        );
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        return broker.reject_with_payload_hash(
            request_id,
            OP_DISCONNECT,
            "mcp_process_tree_supervision_unsupported",
            "現行環境ではMCP process群の停止を保証できないため切断を拒否し、接続記録を保持する",
            true,
            payload_hash,
        );
    }

    #[cfg(any(windows, target_os = "macos"))]
    {
        let termination = broker
            .mcp_connections
            .get_mut(&request.server_id)
            .expect("MCP接続の存在を直前に確認済み")
            .connection
            .terminate();
        if let Err(error) = termination {
            return broker.reject_with_payload_hash(
                request_id,
                OP_DISCONNECT,
                error.code,
                "MCP process群の終了を確認できない。接続状態を保持しowner切断の再試行を要求する",
                true,
                payload_hash,
            );
        }

        let mut body = json!({
            "版": VERSION,
            "ServerID": request.server_id,
            "接続状態": "disconnected",
            "能力ID": "mcp.connection.disconnect",
            "権限ID": "permission.mcp.connection.disconnect",
            "承認状態": "owner_control_approved",
            "復旧ID": "retry-mcp-disconnect",
            "権限生成": "なし",
            "公開範囲": "metadata_only",
            "証拠種別": EVIDENCE_SOURCE_LIVE_RUNTIME,
        });
        let accepted = match broker.append_audit(
            request_id,
            OP_DISCONNECT,
            "accepted",
            &format!(
                "所有OS process群の終了確認後にMCP切断。ServerID={}",
                request.server_id
            ),
            EVIDENCE_SOURCE_LIVE_RUNTIME,
            &super::protocol::canonical_payload_hash(Some(&body)),
        ) {
            Ok(event) => event,
            Err(_) => {
                return broker.audit_store_failed_response(
                    request_id,
                    OP_DISCONNECT,
                    "mcp_audit_append_failed",
                    "切断後の結果Auditを確定できない。接続記録を保持してowner再確認を要求する",
                )
            }
        };
        body.as_object_mut()
            .expect("MCP切断receiptはobject")
            .insert(
                "切断監査ID".to_string(),
                Value::String(accepted.event_id.clone()),
            );
        broker.mcp_connections.remove(&request.server_id);
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: OP_DISCONNECT.to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_LIVE_RUNTIME.to_string(),
            audit_event_id: accepted.event_id,
            error: None,
            health: None,
            body: Some(body),
            shutdown_requested: broker.shutdown_requested,
        }
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
        || request.server_id.chars().any(char::is_control)
        || request.executable.is_empty()
        || request.executable.as_bytes().len() > 1024
        || request.executable.chars().any(char::is_control)
        || !Path::new(&request.executable).is_absolute()
        || request.workspace.is_empty()
        || request.workspace.as_bytes().len() > 1024
        || request.workspace.chars().any(char::is_control)
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
    if request.credential_ref.credential_id.len() != 32
        || !request
            .credential_ref
            .credential_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || request.credential_ref.purpose != "mcp_transport"
        || request.credential_ref.target != request.server_id
    {
        return Err(McpError::new(
            "mcp_credential_ref_invalid",
            "MCP Credential refの識別または対象が不正",
        ));
    }
    match request.credential_ref.status.as_str() {
        "missing" if request.credential_ref.required => {
            return Err(McpError::new(
                "mcp_credential_unavailable",
                "必須MCP Credentialが未構成のためstdio childを起動しない",
            ));
        }
        "missing"
            if request.credential_ref.environment_variable.is_some()
                || request.credential_ref.required =>
        {
            return Err(McpError::new(
                "mcp_credential_ref_invalid",
                "missing Credential refに環境変数名またはrequiredを指定できない",
            ));
        }
        "configured"
            if !request.credential_ref.required
                || request
                    .credential_ref
                    .environment_variable
                    .as_deref()
                    .is_none_or(|name| !safe_credential_environment_name(name)) =>
        {
            return Err(McpError::new(
                "mcp_credential_ref_invalid",
                "MCP Credential refの状態または子process環境変数名が不正",
            ));
        }
        "missing" | "configured" => {}
        _ => {
            return Err(McpError::new(
                "mcp_credential_ref_invalid",
                "MCP Credential refの状態が未対応",
            ));
        }
    }
    Ok(request)
}

pub(super) fn safe_credential_environment_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == b'_')
        || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        || name.len() > 128
    {
        return false;
    }
    let normalized = name.to_ascii_uppercase();
    !matches!(
        normalized.as_str(),
        "PATH"
            | "SYSTEMROOT"
            | "WINDIR"
            | "TEMP"
            | "TMP"
            | "USERPROFILE"
            | "HOME"
            | "APPDATA"
            | "LOCALAPPDATA"
            | "PROGRAMDATA"
            | "SYSTEMDRIVE"
            | "COMSPEC"
            | "PATHEXT"
            | "PSMODULEPATH"
    ) && !normalized.starts_with("GUI_SHELL_")
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    use super::super::protocol::{BrokerOperation, BrokerRequestEnvelope};
    use super::*;

    #[cfg(windows)]
    struct TestTempRoot(std::path::PathBuf);

    #[cfg(windows)]
    impl Drop for TestTempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn request(credential_ref: Value) -> Value {
        json!({
            "版": 1,
            "操作": "接続",
            "ServerID": "fixture-mcp-server",
            "実行file": std::env::current_exe().expect("試験用実行ファイルの取得").to_string_lossy(),
            "引数": [],
            "workspace": std::env::temp_dir().to_string_lossy(),
            "Transport": "stdio",
            "Credential ref": credential_ref,
        })
    }

    fn missing_reference() -> Value {
        json!({
            "credential_id": "00000000000000000000000000000000",
            "purpose": "mcp_transport",
            "target": "fixture-mcp-server",
            "required": false,
            "status": "missing",
        })
    }

    #[test]
    fn macos_mcp_owner_summary_接続範囲だけを表示し秘密と権限注入を拒否する() {
        let mut payload = request(missing_reference());
        payload["引数"] = json!(["公開fixture引数"]);
        let summary = macos_owner_summary(OP_CONNECT, &payload).unwrap();
        assert!(summary.contains("fixture-mcp-server"));
        assert!(!summary.contains("公開fixture引数"));
        assert!(summary.contains("Credential注入: なし"));
        let mut extra = payload.clone();
        extra["approval"] = json!(true);
        assert!(macos_owner_summary(OP_CONNECT, &extra).is_none());
        let mut reference = payload.clone();
        reference["Credential ref"]["status"] = json!("configured");
        reference["Credential ref"]["required"] = json!(true);
        reference["Credential ref"]["environment_variable"] = json!("MCP_API_KEY");
        assert!(macos_owner_summary(OP_CONNECT, &reference).is_none());
        let call = json!({"版":1,"操作":"実行","ServerID":"fixture-mcp-server",
            "ToolID": format!("tool-{}", "a".repeat(142)), "名前":"echo",
            "arguments":{"text":"公開fixture本文"}});
        let summary = macos_owner_summary(OP_TOOL_CALL, &call).unwrap();
        assert!(!summary.contains("公開fixture本文"));
        assert!(summary.contains("一回Permission"));
        let stop = json!({"版":1,"操作":"切断","ServerID":"fixture-mcp-server"});
        assert!(macos_owner_summary(OP_DISCONNECT, &stop).unwrap().contains("所有process group"));
        assert!(macos_owner_summary("未知操作", &stop).is_none());
    }

    #[test]
    fn mcp_credential_reference_requires_exact_target_and_safe_child_environment_name() {
        let configured = json!({
            "credential_id": "0123456789abcdef0123456789abcdef",
            "purpose": "mcp_transport",
            "target": "fixture-mcp-server",
            "required": true,
            "status": "configured",
            "environment_variable": "MCP_API_KEY",
        });
        assert!(parse_request(&request(configured.clone())).is_ok());
        assert!(parse_request(&request(missing_reference())).is_ok());

        for name in [
            "PATH",
            "SystemRoot",
            "GUI_SHELL_BROKER_CHANNEL_PIPE",
            "MCP-KEY",
            "1TOKEN",
            "",
            "秘密",
        ] {
            assert!(!safe_credential_environment_name(name), "{name:?}");
            let mut invalid = configured.clone();
            invalid["environment_variable"] = Value::String(name.to_string());
            assert!(parse_request(&request(invalid)).is_err(), "{name:?}");
        }

        let mut cross_target = configured.clone();
        cross_target["target"] = json!("different-server");
        assert!(parse_request(&request(cross_target)).is_err());

        let mut optional_configured = configured;
        optional_configured["required"] = json!(false);
        assert!(parse_request(&request(optional_configured)).is_err());

        let mut missing_with_variable = missing_reference();
        missing_with_variable["environment_variable"] = json!("MCP_API_KEY");
        assert!(parse_request(&request(missing_with_variable)).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn broker_mcp_credential_to_audited_tool_call_uses_live_stdio_and_hash_only_result() {
        use std::time::{SystemTime, UNIX_EPOCH};

        const SECRET: &str = "synthetic-p6-mcp-secret-not-real";
        const OUTPUT: &str = "synthetic-p6-mcp-result-not-for-projection";
        const SERVER: &str = "p6-integrated-mcp-server";

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let temp_root = TestTempRoot(std::env::temp_dir().join(format!(
            "gui-shell-p6-mcp-e2e-{}-{nonce}",
            std::process::id()
        )));
        let root = &temp_root.0;
        let audit_root = root.join("audit");
        let vault_root = root.join("vault");
        let workspace = root.join("workspace");
        std::fs::create_dir_all(&vault_root).expect("資格情報保管先を作成");
        std::fs::create_dir_all(&workspace).expect("MCP作業領域を作成");
        let mut broker =
            Broker::new_persistent("session-1", &audit_root).expect("永続Brokerを作成");
        broker.current_epoch_seconds_override = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("現在時刻を取得")
                .as_secs() as i64,
        );
        broker
            .保管先起動登録(&vault_root, true, std::slice::from_ref(&audit_root))
            .expect("DPAPI保管先を登録");

        let credential_id = "a".repeat(32);
        let registration = json!({
            "版": 1,
            "操作": "追加",
            "資格情報ID": credential_id,
            "用途": "mcp_transport",
            "接続対象": SERVER,
            "種類": "api_key",
            "保管方式": "windows_dpapi",
            "登録者種別": "owner",
            "登録経路": "owner_control",
            "秘密値": SECRET,
        });
        let add = broker_test_operation(
            &mut broker,
            BrokerOperation::資格情報登録,
            registration,
            true,
        );
        assert_eq!(add.status, BrokerStatus::Accepted, "{add:?}");
        let add_json = serde_json::to_string(&add).expect("公開登録応答を直列化");
        assert!(!add_json.contains(SECRET));

        let script_path = workspace.join("mcp-server.cmd");
        let script = r#"@echo off
if not "%MCP_TEST_CREDENTIAL%"=="__SECRET__" exit /b 41
setlocal EnableDelayedExpansion
set /p request=
echo !request! | findstr /c:"server/discover" > nul
if errorlevel 1 exit /b 42
echo {"jsonrpc":"2.0","id":1,"result":{"resultType":"complete","supportedVersions":["__PROTOCOL_VERSION__"],"capabilities":{"tools":{}}},"_meta":{"io.modelcontextprotocol/serverInfo":{"name":"p6-e2e-fixture","version":"1"}}}
set /p request=
echo !request! | findstr /c:"tools/list" > nul
if errorlevel 1 exit /b 43
echo {"jsonrpc":"2.0","id":2,"result":{"resultType":"complete","tools":[{"name":"echo","inputSchema":{"type":"object","properties":{"text":{"type":"string"}},"required":["text"],"additionalProperties":false}}]}}
set /p request=
echo !request! | findstr /c:"tools/call" > nul
if errorlevel 1 exit /b 44
echo {"jsonrpc":"2.0","id":3,"result":{"resultType":"complete","content":[{"type":"text","text":"__OUTPUT__"}],"isError":false}}
"%SystemRoot%\System32\ping.exe" -t 127.0.0.1 > nul
"#
            .replace("__SECRET__", SECRET)
            .replace("__OUTPUT__", OUTPUT)
            .replace("__PROTOCOL_VERSION__", crate::mcp::MODERN_PROTOCOL_VERSION);
        std::fs::write(&script_path, script).expect("MCP fixture scriptを書き込む");
        let executable = std::env::var_os("SystemRoot")
            .map(std::path::PathBuf::from)
            .expect("Windows環境根を取得")
            .join("System32")
            .join("cmd.exe");
        let connect = json!({
            "版": 1,
            "操作": "接続",
            "ServerID": SERVER,
            "実行file": executable.to_string_lossy(),
            "引数": ["/D", "/C", script_path.to_string_lossy()],
            "workspace": workspace.to_string_lossy(),
            "Transport": "stdio",
            "Credential ref": {
                "credential_id": credential_id,
                "purpose": "mcp_transport",
                "target": SERVER,
                "required": true,
                "status": "configured",
                "environment_variable": "MCP_TEST_CREDENTIAL"
            }
        });
        let connected = broker_test_operation(&mut broker, BrokerOperation::MCP接続, connect, true);
        assert_eq!(connected.status, BrokerStatus::Accepted, "{connected:?}");
        assert_eq!(connected.evidence_source, EVIDENCE_SOURCE_INTERNAL_STATE);
        assert_eq!(connected.body.as_ref().unwrap()["証拠種別"], EVIDENCE_SOURCE_INTERNAL_STATE,
            "接続receiptは現行SchemaとUIが要求する保存済みmetadataの証拠種別へ射影する");
        let connect_json = serde_json::to_string(&connected).expect("接続応答を直列化");
        assert!(!connect_json.contains(SECRET));
        assert_eq!(
            connected.body.as_ref().unwrap()["Trust"]["state"],
            "unverified"
        );
        assert_eq!(connected.body.as_ref().unwrap()["権限生成"], "なし");

        let listed = broker_test_operation(
            &mut broker,
            BrokerOperation::MCP接続一覧,
            json!({"版": 1}),
            false,
        );
        assert_eq!(listed.status, BrokerStatus::Accepted, "{listed:?}");
        let listed_json = serde_json::to_string(&listed).expect("metadata-only Tool一覧を直列化");
        assert!(!listed_json.contains(SECRET));
        let tool = &listed.body.as_ref().unwrap()["MCP接続一覧"][0]["Tool"][0];
        assert_eq!(tool["name"], "echo");
        let tool_id = tool["tool_id"].as_str().expect("Broker発行Tool ID");

        let tool_request = json!({
            "版": 1,
            "操作": "実行",
            "ServerID": SERVER,
            "ToolID": tool_id,
            "名前": "echo",
            "arguments": {"text": "synthetic public input"}
        });
        let unconfirmed = broker_test_operation(
            &mut broker,
            BrokerOperation::MCPTool実行,
            tool_request.clone(),
            true,
        );
        assert_eq!(unconfirmed.status, BrokerStatus::Rejected);
        assert_eq!(
            unconfirmed.error.as_ref().map(|error| error.code.as_str()),
            Some("desktop_native_owner_confirmation_required")
        );
        let call = broker_test_native_owner_operation(
            &mut broker,
            BrokerOperation::MCPTool実行,
            tool_request,
        );
        assert_eq!(call.status, BrokerStatus::Accepted, "{call:?}");
        assert_eq!(call.evidence_source, EVIDENCE_SOURCE_LIVE_RUNTIME);
        let call_json = serde_json::to_string(&call).expect("hash-only呼出し記録を直列化");
        assert!(!call_json.contains(SECRET));
        assert!(!call_json.contains(OUTPUT));
        assert!(call.body.as_ref().unwrap()["result_hash"]
            .as_str()
            .is_some_and(|hash| hash.starts_with("sha256:")));
        assert_eq!(call.body.as_ref().unwrap()["公開範囲"], "hash_only");

        let audit_json = serde_json::to_string(&broker.audit_events()).expect("永続Auditを直列化");
        assert!(!audit_json.contains(SECRET));
        assert!(!audit_json.contains(OUTPUT));
        assert!(broker.audit_events().iter().any(|event| {
            event.operation == "MCP Tool実行"
                && event.decision == "approved"
                && event.reason.contains("arguments_hash=")
        }));
        assert!(broker.audit_events().iter().any(|event| {
            event.operation == "MCP Tool実行"
                && event.decision == "accepted"
                && event.evidence_source == EVIDENCE_SOURCE_LIVE_RUNTIME
        }));

        let disconnected = broker_test_operation(
            &mut broker,
            BrokerOperation::MCP切断,
            json!({"版": 1, "操作": "切断", "ServerID": SERVER}),
            true,
        );
        assert_eq!(
            disconnected.status,
            BrokerStatus::Accepted,
            "{disconnected:?}"
        );
        assert!(broker.mcp_connections.is_empty());
        drop(broker);
        drop(temp_root);
    }

    #[cfg(windows)]
    fn broker_test_operation(
        broker: &mut Broker,
        operation: BrokerOperation,
        payload: Value,
        owner: bool,
    ) -> BrokerResponse {
        let request_id = format!("p6-mcp-e2e-{}", broker.audit_events().len());
        let mut envelope = BrokerRequestEnvelope::command_envelope_at(
            &request_id,
            "session-1",
            &format!("nonce-{request_id}"),
            &BrokerRequestEnvelope::current_issued_at(),
        );
        envelope.operation = Some(operation);
        envelope.payload = Some(payload);
        envelope.refresh_payload_hash();
        broker.処理(envelope, owner)
    }

    #[cfg(windows)]
    fn broker_test_native_owner_operation(
        broker: &mut Broker,
        operation: BrokerOperation,
        payload: Value,
    ) -> BrokerResponse {
        let request_id = format!("p6-mcp-native-{}", broker.audit_events().len());
        let payload_hash = super::super::protocol::canonical_payload_hash(Some(&payload));
        let input = json!({
            "request_id": request_id,
            "session_id": "session-1",
            "operation": operation.as_str(),
            "payload": payload,
            "payload_hash": payload_hash,
            "nonce": format!("nonce-{request_id}"),
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        broker.desktop_owner_operation_json(&input.to_string())
    }
}

fn parse_disconnect_request(payload: &Value) -> Result<DisconnectRequest, McpError> {
    let request: DisconnectRequest = serde_json::from_value(payload.clone()).map_err(|_| {
        McpError::new(
            "mcp_disconnect_request_invalid",
            "MCP切断payloadの構造が不正",
        )
    })?;
    if request.version != VERSION
        || request.operation != "切断"
        || request.server_id.is_empty()
        || request.server_id.as_bytes().len() > 128
        || request.server_id.chars().any(char::is_control)
    {
        return Err(McpError::new(
            "mcp_disconnect_request_invalid",
            "MCP切断payloadの値が不正",
        ));
    }
    Ok(request)
}

fn parse_tool_call_request(payload: &Value) -> Result<ToolCallRequest, McpError> {
    let request: ToolCallRequest = serde_json::from_value(payload.clone()).map_err(|_| {
        McpError::new(
            "mcp_tool_call_request_invalid",
            "MCP Tool実行payloadの構造が不正",
        )
    })?;
    if request.version != VERSION
        || request.operation != "実行"
        || request.server_id.is_empty()
        || request.server_id.as_bytes().len() > 128
        || request.server_id.chars().any(char::is_control)
        || request.tool_id.len() != 147
        || !request.tool_id.starts_with("tool-")
        || !request.tool_id[5..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || request.name.is_empty()
        || request.name.as_bytes().len() > 256
        || request.name.chars().any(char::is_control)
        || !request.arguments.is_object()
    {
        return Err(McpError::new(
            "mcp_tool_call_request_invalid",
            "MCP Tool実行payloadの識別子または値が不正",
        ));
    }
    Ok(request)
}

/// Mac native確認の公開表示。承認・catalog適合・実行資格は既存Brokerが別途再評価する。
#[cfg(any(target_os = "macos", test))]
pub(crate) fn macos_owner_summary(operation: &str, payload: &Value) -> Option<String> {
    match operation {
        OP_CONNECT => {
            let request = parse_request(payload).ok()?;
            if request.credential_ref.status != "missing" { return None; }
            let hash = super::protocol::canonical_payload_hash(Some(&serde_json::to_value(&request.arguments).ok()?));
            Some(format!("このstdio Serverを起動して接続します。\nServer ID: {}\n実行file: {}\nWorkspace: {}\n引数: {}項目\n引数hash: {hash}\nCredential注入: なし\nApp Sandboxを継承し、所有process groupを明示切断します。外部Server metadataはTrust・Permissionではありません。引数本文はここへ表示しません。",
                request.server_id, request.executable, request.workspace, request.arguments.len()))
        },
        OP_DISCONNECT => {
            let request = parse_disconnect_request(payload).ok()?;
            Some(format!("このMCP Serverの所有process groupを停止して切断します。\nServer ID: {}\n現在接続と停止・AuditをBrokerが再評価します。第三者の別groupへの離脱や外部副作用の取消を保証しません。", request.server_id))
        },
        OP_TOOL_CALL => {
            let request = parse_tool_call_request(payload).ok()?;
            let hash = super::protocol::canonical_payload_hash(Some(&request.arguments));
            Some(format!("このToolを現在Serverへ一回だけ実行します。\nServer ID: {}\nTool: {} ({})\n引数: {}項目\n引数hash: {hash}\n前画面のJSON argumentsを確認してください。秘密値を含めないでください。Brokerが現在Catalog／Schemaを再検査し、一回PermissionとAuditを確定します。結果本文は公開せずhashだけを返します。外部副作用は取消できず、結果不明時は隔離・自動再送なしです。",
                request.server_id, request.name, request.tool_id, request.arguments.as_object()?.len()))
        },
        _ => None,
    }
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
