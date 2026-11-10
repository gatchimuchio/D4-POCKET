//! A2A v1.0 Agent Card取得、JSON-RPC SendMessage／GetTaskのbounded transport。
//! 認証credential、Task本文、Artifact本文、Stream購読は扱わない。
#![allow(non_snake_case)]

use crate::audit_hash::sha256_tagged;
use crate::broker::update_download::{fetch_a2a_agent_card_document, DownloadError};
use serde_json::{json, Map, Value};
use std::io::{Read, Write};
use std::net::{IpAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

const MAX_URI_BYTES: usize = 2048;
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_STRING_CHARS: usize = 1024;
const MAX_SEND_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_JSON_RPC_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct A2aError {
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

impl A2aError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Endpoint {
    scheme: String,
    host: String,
    port: u16,
    authority: String,
    path: String,
}

pub(crate) fn fetch_agent_card(
    uri: &str,
    agent_id: &str,
    protocol_version: &str,
    credential_ref: &Value,
) -> Result<Value, A2aError> {
    let endpoint = parse_endpoint(uri)?;
    validate_credential_ref(credential_ref, agent_id)?;
    let raw = if endpoint.scheme == "http" {
        http_get(&endpoint)?
    } else {
        fetch_a2a_agent_card_document(uri, Instant::now() + Duration::from_secs(8))
            .map_err(map_https_error)?
    };
    let card_text = std::str::from_utf8(&raw)
        .map_err(|_| A2aError::new("a2a_card_invalid_json", "A2A Agent CardがUTF-8ではない"))?;
    let card: Value = crate::broker::json_input::read_unique(card_text).map_err(|_| {
        A2aError::new(
            "a2a_card_invalid_json",
            "A2A Agent CardのJSONまたは重複fieldが不正",
        )
    })?;
    project_agent_card(&endpoint, &card, agent_id, protocol_version, credential_ref)
}

fn parse_endpoint(uri: &str) -> Result<Endpoint, A2aError> {
    if uri.is_empty() || uri.len() > MAX_URI_BYTES || uri.chars().any(char::is_control) {
        return Err(A2aError::new(
            "a2a_endpoint_invalid",
            "A2A Agent Card URIの長さまたは文字が不正",
        ));
    }
    let (scheme, remainder) = uri
        .split_once("://")
        .ok_or_else(|| A2aError::new("a2a_endpoint_invalid", "A2A Agent Card URIのschemeが不正"))?;
    let scheme = scheme.to_ascii_lowercase();
    if !matches!(scheme.as_str(), "http" | "https") {
        return Err(A2aError::new(
            "a2a_transport_unsupported",
            "A2A接続はhttpまたはhttpsだけを受け付ける",
        ));
    }
    if remainder.contains('@') || remainder.contains('#') || remainder.contains('?') {
        return Err(A2aError::new(
            "a2a_endpoint_invalid",
            "A2A Agent Card URIへuserinfo、query、fragmentを含められない",
        ));
    }
    let (authority, path) = match remainder.split_once('/') {
        Some((authority, path)) => (authority, format!("/{path}")),
        None => (remainder, "/".to_string()),
    };
    let (host, port) = if let Some((host, port)) = authority.rsplit_once(':') {
        if host.is_empty() || host.contains(':') {
            return Err(A2aError::new(
                "a2a_endpoint_invalid",
                "A2A Agent Card URIのhostが不正",
            ));
        }
        (
            host.to_string(),
            port.parse::<u16>().map_err(|_| {
                A2aError::new("a2a_endpoint_invalid", "A2A Agent Card URIのportが不正")
            })?,
        )
    } else {
        (
            authority.to_string(),
            if scheme == "https" { 443 } else { 80 },
        )
    };
    if port == 0 || host.is_empty() || host.len() > 255 || host.contains(':') || path.len() > 1024 {
        return Err(A2aError::new(
            "a2a_endpoint_invalid",
            "A2A Agent Card URIのhost、port、pathが不正",
        ));
    }
    if !path.is_ascii() || path.bytes().any(|byte| !(b'!'..=b'~').contains(&byte)) {
        return Err(A2aError::new(
            "a2a_endpoint_invalid",
            "A2A Agent Card URIのpathは空白のないASCII URI表記が必要",
        ));
    }
    match scheme.as_str() {
        "http" => {
            let ip = host.parse::<IpAddr>().map_err(|_| {
                A2aError::new(
                    "a2a_http_non_loopback",
                    "httpのA2A接続はloopback IPへ限定する",
                )
            })?;
            if !ip.is_loopback() {
                return Err(A2aError::new(
                    "a2a_http_non_loopback",
                    "httpのA2A接続はloopback IPへ限定する",
                ));
            }
        }
        "https" if valid_https_host(&host) => {}
        "https" => {
            return Err(A2aError::new(
                "a2a_endpoint_invalid",
                "httpsのA2A接続先host表記が不正",
            ));
        }
        _ => unreachable!("schemeは先行検査済み"),
    }
    Ok(Endpoint {
        scheme,
        host,
        port,
        authority: authority.to_string(),
        path,
    })
}

fn valid_https_host(host: &str) -> bool {
    if host.parse::<IpAddr>().is_ok() {
        return host.is_ascii();
    }
    host.is_ascii()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

pub(crate) fn transport_name(uri: &str) -> Result<&'static str, A2aError> {
    let endpoint = parse_endpoint(uri)?;
    Ok(if endpoint.scheme == "https" {
        "https"
    } else {
        "http"
    })
}

pub(crate) fn interface_identity(uri: &str) -> Result<(String, String), A2aError> {
    hash_interface_endpoint(uri)
}

pub(crate) fn send_message(
    uri: &str,
    message_text: &str,
    deadline: Instant,
) -> Result<Value, A2aError> {
    let endpoint = parse_endpoint(uri)?;
    if message_text.is_empty()
        || message_text.len() > MAX_SEND_MESSAGE_BYTES
        || message_text.chars().any(|character| character == '\0')
        || Instant::now() >= deadline
    {
        return Err(A2aError::new(
            "a2a_message_invalid",
            "A2A messageが空、上限超過または期限切れ",
        ));
    }
    let mut request_id_bytes = [0u8; 16];
    let mut message_id_bytes = [0u8; 16];
    getrandom::getrandom(&mut request_id_bytes)
        .and_then(|_| getrandom::getrandom(&mut message_id_bytes))
        .map_err(|_| A2aError::new("a2a_message_id_unavailable", "A2A識別値を生成できない"))?;
    let request_id = format!("d4p-{}", hex::encode(request_id_bytes));
    let message_id = format!("d4p-{}", hex::encode(message_id_bytes));
    let request = json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "SendMessage",
        "params": {
            "message": {
                "kind": "message",
                "messageId": message_id,
                "role": "ROLE_USER",
                "parts": [{ "kind": "text", "text": message_text }]
            }
        }
    });
    let request_bytes = serde_json::to_vec(&request)
        .map_err(|_| A2aError::new("a2a_message_invalid", "A2A messageを構成できない"))?;
    if request_bytes.len() > MAX_JSON_RPC_BYTES {
        return Err(A2aError::new(
            "a2a_message_invalid",
            "A2A JSON-RPC requestの上限を超過",
        ));
    }
    let raw = if endpoint.scheme == "http" {
        http_exchange(&endpoint, "POST", &request_bytes)?
    } else {
        crate::broker::update_download::send_a2a_json_rpc_document(uri, &request_bytes, deadline)
            .map_err(map_https_error)?
    };
    project_send_message_response(&raw, &request_id)
}

pub(crate) fn get_task(uri: &str, task_id: &str, deadline: Instant) -> Result<Value, A2aError> {
    let endpoint = parse_endpoint(uri)?;
    if !valid_task_id(task_id) || Instant::now() >= deadline {
        return Err(A2aError::new(
            "a2a_task_request_invalid",
            "A2A Task IDまたは要求期限が不正",
        ));
    }
    let mut request_id_bytes = [0u8; 16];
    getrandom::getrandom(&mut request_id_bytes)
        .map_err(|_| A2aError::new("a2a_task_id_unavailable", "A2A識別値を生成できない"))?;
    let request_id = format!("d4p-get-task-{}", hex::encode(request_id_bytes));
    let request = json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "GetTask",
        "params": { "id": task_id, "historyLength": 0 }
    });
    let request_bytes = serde_json::to_vec(&request)
        .map_err(|_| A2aError::new("a2a_task_request_invalid", "A2A要求を構成できない"))?;
    if request_bytes.len() > MAX_JSON_RPC_BYTES {
        return Err(A2aError::new(
            "a2a_task_request_invalid",
            "A2A JSON-RPC requestの上限を超過",
        ));
    }
    let raw = if endpoint.scheme == "http" {
        http_exchange(&endpoint, "POST", &request_bytes)?
    } else {
        crate::broker::update_download::send_a2a_json_rpc_document(uri, &request_bytes, deadline)
            .map_err(map_https_error)?
    };
    project_get_task_response(&raw, &request_id, task_id)
}

pub(crate) fn valid_task_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= 256
        && !value.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '\u{061c}'
                        | '\u{200e}'
                        | '\u{200f}'
                        | '\u{2028}'
                        | '\u{2029}'
                        | '\u{202a}'..='\u{202e}'
                        | '\u{2066}'..='\u{2069}'
                )
        })
}

fn project_get_task_response(
    raw: &[u8],
    request_id: &str,
    expected_task_id: &str,
) -> Result<Value, A2aError> {
    let raw_text = std::str::from_utf8(raw)
        .map_err(|_| A2aError::new("a2a_response_invalid", "A2A応答がUTF-8ではない"))?;
    let response: Value = crate::broker::json_input::read_unique(raw_text)
        .map_err(|_| A2aError::new("a2a_response_invalid", "A2A JSON-RPC応答が不正"))?;
    let object = response
        .as_object()
        .ok_or_else(|| A2aError::new("a2a_response_invalid", "A2A JSON-RPC応答がobjectではない"))?;
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
        || object.get("id").and_then(Value::as_str) != Some(request_id)
    {
        return Err(A2aError::new(
            "a2a_response_invalid",
            "A2A JSON-RPC応答のversionまたはrequest IDが一致しない",
        ));
    }
    if object.contains_key("error") {
        if object.len() != 3 || object.get("result").is_some() {
            return Err(A2aError::new(
                "a2a_response_invalid",
                "A2A JSON-RPC error応答のfieldが不正",
            ));
        }
        return Err(A2aError::new(
            "a2a_remote_task_rejected",
            "外部AgentがA2A Task取得を拒否した",
        ));
    }
    if object.len() != 3
        || object
            .keys()
            .any(|key| !["jsonrpc", "id", "result"].contains(&key.as_str()))
    {
        return Err(A2aError::new(
            "a2a_response_invalid",
            "A2A JSON-RPC応答に未知または欠落fieldがある",
        ));
    }
    let response_hash = sha256_tagged(raw);
    let task = object
        .get("result")
        .ok_or_else(|| A2aError::new("a2a_response_invalid", "A2A Task resultがない"))?;
    project_task_metadata(task, &response_hash, Some(expected_task_id), true)
}

fn project_send_message_response(raw: &[u8], request_id: &str) -> Result<Value, A2aError> {
    let raw_text = std::str::from_utf8(raw)
        .map_err(|_| A2aError::new("a2a_response_invalid", "A2A応答がUTF-8ではない"))?;
    let response: Value = crate::broker::json_input::read_unique(raw_text)
        .map_err(|_| A2aError::new("a2a_response_invalid", "A2A JSON-RPC応答が不正"))?;
    let object = response
        .as_object()
        .ok_or_else(|| A2aError::new("a2a_response_invalid", "A2A JSON-RPC応答がobjectではない"))?;
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
        || object.get("id").and_then(Value::as_str) != Some(request_id)
    {
        return Err(A2aError::new(
            "a2a_response_invalid",
            "A2A JSON-RPC応答のversionまたはrequest IDが一致しない",
        ));
    }
    if object.contains_key("error") {
        if object.len() != 3 || object.get("result").is_some() {
            return Err(A2aError::new(
                "a2a_response_invalid",
                "A2A JSON-RPC error応答のfieldが不正",
            ));
        }
        return Err(A2aError::new(
            "a2a_remote_rejected",
            "外部AgentがA2A messageを拒否した",
        ));
    }
    if object.len() != 3
        || object
            .keys()
            .any(|key| !["jsonrpc", "id", "result"].contains(&key.as_str()))
    {
        return Err(A2aError::new(
            "a2a_response_invalid",
            "A2A JSON-RPC応答に未知または欠落fieldがある",
        ));
    }
    let result = object
        .get("result")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            A2aError::new(
                "a2a_response_invalid",
                "A2A SendMessage resultがobjectではない",
            )
        })?;
    if result.len() != 1
        || result
            .keys()
            .any(|key| !["task", "message"].contains(&key.as_str()))
    {
        return Err(A2aError::new(
            "a2a_response_invalid",
            "A2A SendMessage resultはTaskまたはMessageの一方だけを含む必要がある",
        ));
    }
    let response_hash = sha256_tagged(raw);
    if let Some(task) = result.get("task") {
        project_task_metadata(task, &response_hash, None, false)
    } else {
        let message = result["message"].as_object().ok_or_else(|| {
            A2aError::new("a2a_response_invalid", "A2A Message resultがobjectではない")
        })?;
        if message.keys().any(|key| {
            ![
                "kind",
                "messageId",
                "contextId",
                "taskId",
                "role",
                "parts",
                "metadata",
                "extensions",
                "referenceTaskIds",
            ]
            .contains(&key.as_str())
        }) {
            return Err(A2aError::new(
                "a2a_response_invalid",
                "A2A Message resultに未知fieldがある",
            ));
        }
        if message.get("kind").and_then(Value::as_str) != Some("message") {
            return Err(A2aError::new(
                "a2a_response_invalid",
                "A2A Message kindが不正",
            ));
        }
        let message_id = bounded_external_id(message.get("messageId"), "Message messageId")?;
        let context_id = optional_external_id(message.get("contextId"), "Message contextId")?;
        let task_id = optional_external_id(message.get("taskId"), "Message taskId")?;
        let role = message
            .get("role")
            .and_then(Value::as_str)
            .filter(|value| matches!(*value, "ROLE_AGENT" | "ROLE_USER"))
            .ok_or_else(|| A2aError::new("a2a_response_invalid", "A2A Message roleが不正"))?;
        let part_count = bounded_optional_array_len(message.get("parts"), "parts", 64)?;
        if part_count == 0 {
            return Err(A2aError::new(
                "a2a_response_invalid",
                "A2A Message partsが空",
            ));
        }
        Ok(json!({
            "kind": "message",
            "message_id": message_id,
            "context_id": context_id,
            "task_id": task_id,
            "role": role,
            "part_count": part_count,
            "result_hash": response_hash,
        }))
    }
}

fn project_task_metadata(
    task_value: &Value,
    response_hash: &str,
    expected_task_id: Option<&str>,
    require_empty_history: bool,
) -> Result<Value, A2aError> {
    let task = task_value
        .as_object()
        .ok_or_else(|| A2aError::new("a2a_response_invalid", "A2A Task resultがobjectではない"))?;
    if task.keys().any(|key| {
        ![
            "kind",
            "id",
            "contextId",
            "status",
            "artifacts",
            "history",
            "metadata",
        ]
        .contains(&key.as_str())
    }) {
        return Err(A2aError::new(
            "a2a_response_invalid",
            "A2A Task resultに未知fieldがある",
        ));
    }
    if task.get("kind").and_then(Value::as_str) != Some("task") {
        return Err(A2aError::new("a2a_response_invalid", "A2A Task kindが不正"));
    }
    let task_id = bounded_external_id(task.get("id"), "Task id")?;
    if expected_task_id.is_some_and(|expected| expected != task_id) {
        return Err(A2aError::new(
            "a2a_response_invalid",
            "A2A Task response IDが要求IDと一致しない",
        ));
    }
    let context_id = optional_external_id(task.get("contextId"), "Task contextId")?;
    let status = task
        .get("status")
        .and_then(Value::as_object)
        .ok_or_else(|| A2aError::new("a2a_response_invalid", "A2A Task statusがobjectではない"))?;
    if status
        .keys()
        .any(|key| !["state", "message", "timestamp"].contains(&key.as_str()))
    {
        return Err(A2aError::new(
            "a2a_response_invalid",
            "A2A Task statusに未知fieldがある",
        ));
    }
    let state = project_task_state(status.get("state"))?;
    let artifact_count = bounded_optional_array_len(task.get("artifacts"), "artifacts", 64)?;
    let history_count = bounded_optional_array_len(task.get("history"), "history", 64)?;
    if require_empty_history && history_count != 0 {
        return Err(A2aError::new(
            "a2a_response_invalid",
            "A2A GetTaskはhistoryLength=0を超える履歴を返した",
        ));
    }
    Ok(json!({
        "kind": "task",
        "task_id": task_id,
        "context_id": context_id,
        "state": state,
        "artifact_count": artifact_count,
        "history_count": history_count,
        "result_hash": response_hash,
    }))
}

fn bounded_external_id(value: Option<&Value>, field: &str) -> Result<String, A2aError> {
    value
        .and_then(Value::as_str)
        .filter(|value| {
            !value.is_empty()
                && value.chars().count() <= 256
                && !value.chars().any(|character| {
                    character.is_control()
                        || matches!(
                            character,
                            '\u{061c}'
                                | '\u{200e}'
                                | '\u{200f}'
                                | '\u{2028}'
                                | '\u{2029}'
                                | '\u{202a}'..='\u{202e}'
                                | '\u{2066}'..='\u{2069}'
                        )
                })
        })
        .map(str::to_owned)
        .ok_or_else(|| A2aError::new("a2a_response_invalid", format!("A2A {field}が不正")))
}

fn optional_external_id(value: Option<&Value>, field: &str) -> Result<Option<String>, A2aError> {
    match value {
        None => Ok(None),
        Some(value) => bounded_external_id(Some(value), field).map(Some),
    }
}

fn bounded_optional_array_len(
    value: Option<&Value>,
    field: &str,
    max: usize,
) -> Result<usize, A2aError> {
    match value {
        None => Ok(0),
        Some(Value::Array(values)) if values.len() <= max => Ok(values.len()),
        _ => Err(A2aError::new(
            "a2a_response_invalid",
            format!("A2A {field}の件数が上限外"),
        )),
    }
}

fn project_task_state(value: Option<&Value>) -> Result<&'static str, A2aError> {
    let state = value
        .and_then(Value::as_str)
        .ok_or_else(|| A2aError::new("a2a_response_invalid", "A2A Task stateが不正"))?;
    match state {
        "TASK_STATE_UNSPECIFIED" | "unspecified" => Ok("unknown"),
        "TASK_STATE_SUBMITTED" | "submitted" => Ok("submitted"),
        "TASK_STATE_WORKING" | "working" => Ok("working"),
        "TASK_STATE_COMPLETED" | "completed" => Ok("completed"),
        "TASK_STATE_FAILED" | "failed" => Ok("failed"),
        "TASK_STATE_CANCELED" | "canceled" => Ok("canceled"),
        "TASK_STATE_INPUT_REQUIRED" | "input_required" => Ok("input_required"),
        "TASK_STATE_REJECTED" | "rejected" => Ok("rejected"),
        "TASK_STATE_AUTH_REQUIRED" | "auth_required" => Ok("auth_required"),
        _ => Ok("unknown"),
    }
}

fn map_https_error(error: DownloadError) -> A2aError {
    match error {
        DownloadError::NonPublicAddress => A2aError::new(
            "a2a_https_non_public_address",
            "https A2A接続先がpublic IP範囲ではない",
        ),
        DownloadError::TimedOut => A2aError::new("a2a_timeout", "https Agent Card取得が期限を超過"),
        DownloadError::DocumentTooLarge
        | DownloadError::InvalidHeaders
        | DownloadError::RedirectOrUnexpectedStatus
        | DownloadError::SizeMismatch => A2aError::new(
            "a2a_response_invalid",
            "https Agent Card応答のstatus、headerまたはsizeが不正",
        ),
        DownloadError::InvalidRequest => {
            A2aError::new("a2a_endpoint_invalid", "https Agent Card接続先が不正")
        }
        _ => A2aError::new("a2a_connection_failed", "https Agent Cardへ接続できない"),
    }
}

pub(crate) fn owner_confirmation_target(
    uri: &str,
    credential_ref: &Value,
    agent_id: &str,
) -> Result<String, A2aError> {
    let endpoint = parse_endpoint(uri)?;
    validate_credential_ref(credential_ref, agent_id)?;
    Ok(format!(
        "{}://{}{}",
        endpoint.scheme, endpoint.authority, endpoint.path
    ))
}

fn validate_credential_ref(value: &Value, agent_id: &str) -> Result<(), A2aError> {
    let object = value.as_object().ok_or_else(|| {
        A2aError::new(
            "a2a_credential_ref_invalid",
            "A2A Credential refがobjectではない",
        )
    })?;
    for key in [
        "secret",
        "secret_value",
        "token",
        "password",
        "credential_value",
    ] {
        if object.contains_key(key) {
            return Err(A2aError::new(
                "a2a_credential_secret_received",
                "A2A Credential refへ秘密値を含められない",
            ));
        }
    }
    if object.get("required").and_then(Value::as_bool) != Some(false)
        || object.get("status").and_then(Value::as_str) != Some("missing")
    {
        return Err(A2aError::new(
            "a2a_credential_unavailable",
            "現行A2A接続ではCredential注入を行わない",
        ));
    }
    let credential_id = object.get("credential_id").and_then(Value::as_str);
    let purpose = object.get("purpose").and_then(Value::as_str);
    let target = object.get("target").and_then(Value::as_str);
    if object.len() != 5
        || object.keys().any(|key| {
            !["credential_id", "purpose", "target", "required", "status"].contains(&key.as_str())
        })
        || credential_id.is_none_or(|id| {
            id.len() != 32
                || !id
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
        || purpose != Some("A2A接続")
        || target != Some(agent_id)
    {
        return Err(A2aError::new(
            "a2a_credential_ref_invalid",
            "A2A Credential refの識別・用途・対象が不正",
        ));
    }
    Ok(())
}

fn http_get(endpoint: &Endpoint) -> Result<Vec<u8>, A2aError> {
    http_exchange(endpoint, "GET", &[])
}

fn http_exchange(endpoint: &Endpoint, method: &str, body: &[u8]) -> Result<Vec<u8>, A2aError> {
    if !matches!(method, "GET" | "POST") || body.len() > MAX_JSON_RPC_BYTES {
        return Err(A2aError::new(
            "a2a_request_invalid",
            "A2A HTTP要求の形式が不正",
        ));
    }
    let socket = format!("{}:{}", endpoint.host, endpoint.port)
        .to_socket_addrs()
        .map_err(|_| {
            A2aError::new(
                "a2a_connection_failed",
                "A2A Agent Card接続先を解決できない",
            )
        })?
        .next()
        .ok_or_else(|| A2aError::new("a2a_connection_failed", "A2A Agent Card接続先がない"))?;
    let mut stream = TcpStream::connect_timeout(&socket, Duration::from_secs(2))
        .map_err(|_| A2aError::new("a2a_connection_failed", "A2A Agent Cardへ接続できない"))?;
    stream
        .set_read_timeout(Some(Duration::from_millis(250)))
        .map_err(|_| A2aError::new("a2a_connection_failed", "A2A受信期限を設定できない"))?;
    stream
        .set_write_timeout(Some(Duration::from_millis(250)))
        .map_err(|_| A2aError::new("a2a_connection_failed", "A2A送信期限を設定できない"))?;
    let protocol_headers = if method == "POST" {
        "Content-Type: application/json\r\nA2A-Version: 1.0\r\n"
    } else {
        ""
    };
    let request = format!(
        "{method} {} HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\nAccept-Encoding: identity\r\n{protocol_headers}Content-Length: {}\r\nConnection: close\r\nUser-Agent: D4-Pocket-A2A/1\r\n\r\n",
        endpoint.path,
        endpoint.authority,
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|_| A2aError::new("a2a_connection_failed", "A2A Agent Card要求を送信できない"))?;
    stream
        .flush()
        .map_err(|_| A2aError::new("a2a_connection_failed", "A2A Agent Card要求をflushできない"))?;
    if !body.is_empty() {
        stream
            .write_all(body)
            .map_err(|_| A2aError::new("a2a_connection_failed", "A2A message要求を送信できない"))?;
        stream.flush().map_err(|_| {
            A2aError::new("a2a_connection_failed", "A2A message要求をflushできない")
        })?;
    }
    let response_started_at = Instant::now();
    let deadline = response_started_at + Duration::from_secs(5);
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    let (body_start, body_length) = loop {
        if Instant::now() >= deadline {
            return Err(A2aError::new(
                "a2a_timeout",
                format!(
                    "A2A Agent Card応答が期限を超過; stage=headers; received_bytes={}; elapsed_ms={}",
                    bytes.len(),
                    response_started_at.elapsed().as_millis()
                ),
            ));
        }
        match stream.read(&mut chunk) {
            Ok(0) => {
                return Err(A2aError::new(
                    "a2a_response_invalid",
                    "A2A Agent Card応答が途中で終了",
                ))
            }
            Ok(size) => bytes.extend_from_slice(&chunk[..size]),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::Interrupted
                ) =>
            {
                continue
            }
            Err(error) => {
                return Err(A2aError::new(
                    "a2a_connection_failed",
                    format!(
                        "A2A Agent Card応答を読めない: {:?}; stage=headers; received_bytes={}",
                        error.kind(),
                        bytes.len()
                    ),
                ))
            }
        }
        if let Some(position) = bytes.windows(4).position(|value| value == b"\r\n\r\n") {
            if position > MAX_HEADER_BYTES {
                return Err(A2aError::new(
                    "a2a_response_invalid",
                    "A2A Agent Card headerが大きすぎる",
                ));
            }
            let header = std::str::from_utf8(&bytes[..position]).map_err(|_| {
                A2aError::new(
                    "a2a_response_invalid",
                    "A2A Agent Card headerがUTF-8ではない",
                )
            })?;
            let mut lines = header.split("\r\n");
            let mut status = lines.next().unwrap_or_default().split_whitespace();
            if !matches!(status.next(), Some("HTTP/1.0" | "HTTP/1.1"))
                || status.next() != Some("200")
            {
                return Err(A2aError::new(
                    "a2a_connection_failed",
                    "A2A Agent Card HTTP statusが200ではない",
                ));
            }
            let mut length = None;
            let mut content_type = None;
            let mut header_count = 0usize;
            for line in lines {
                header_count += 1;
                if header_count > 64 {
                    return Err(A2aError::new(
                        "a2a_response_invalid",
                        "A2A HTTP header field数が多すぎる",
                    ));
                }
                let (name, value) = line.split_once(':').ok_or_else(|| {
                    A2aError::new("a2a_response_invalid", "A2A HTTP headerが不正")
                })?;
                match name.to_ascii_lowercase().as_str() {
                    "content-length" => {
                        if length.is_some() {
                            return Err(A2aError::new(
                                "a2a_response_invalid",
                                "A2A Content-Lengthが重複",
                            ));
                        }
                        length = Some(value.trim().parse::<usize>().map_err(|_| {
                            A2aError::new("a2a_response_invalid", "A2A Content-Lengthが不正")
                        })?);
                    }
                    "content-type" => {
                        if content_type.is_some() {
                            return Err(A2aError::new(
                                "a2a_response_invalid",
                                "A2A Content-Typeが重複",
                            ));
                        }
                        content_type = Some(value.trim().to_ascii_lowercase())
                    }
                    "transfer-encoding" | "content-encoding" | "location" => {
                        return Err(A2aError::new(
                            "a2a_response_invalid",
                            "A2A HTTP転送境界が不正",
                        ));
                    }
                    _ => {}
                }
            }
            let length = length
                .ok_or_else(|| A2aError::new("a2a_response_invalid", "A2A Content-Lengthがない"))?;
            if length == 0
                || length > MAX_RESPONSE_BYTES
                || !content_type
                    .is_some_and(|v| v.split(';').next().map(str::trim) == Some("application/json"))
            {
                return Err(A2aError::new(
                    "a2a_response_invalid",
                    "A2A Agent Card JSON応答の境界が不正",
                ));
            }
            break (position + 4, length);
        }
        if bytes.len() > MAX_HEADER_BYTES {
            return Err(A2aError::new(
                "a2a_response_invalid",
                "A2A Agent Card headerが大きすぎる",
            ));
        }
    };
    while bytes.len() < body_start + body_length {
        if Instant::now() >= deadline {
            return Err(A2aError::new(
                "a2a_timeout",
                format!(
                    "A2A Agent Card bodyが期限を超過; stage=body; received_bytes={}; expected_bytes={}; elapsed_ms={}",
                    bytes.len().saturating_sub(body_start),
                    body_length,
                    response_started_at.elapsed().as_millis()
                ),
            ));
        }
        let remaining = body_start + body_length - bytes.len();
        let read_size = chunk.len().min(remaining);
        match stream.read(&mut chunk[..read_size]) {
            Ok(0) => {
                return Err(A2aError::new(
                    "a2a_response_invalid",
                    "A2A Agent Card bodyが途中で終了",
                ))
            }
            Ok(size) => bytes.extend_from_slice(&chunk[..size]),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::Interrupted
                ) =>
            {
                continue
            }
            Err(error) => {
                return Err(A2aError::new(
                    "a2a_connection_failed",
                    format!(
                        "A2A Agent Card bodyを読めない: {:?}; stage=body; received_bytes={}",
                        error.kind(),
                        bytes.len()
                    ),
                ))
            }
        }
    }
    if bytes.len() != body_start + body_length {
        return Err(A2aError::new(
            "a2a_response_invalid",
            "A2A Agent Card bodyがContent-Lengthと一致しない",
        ));
    }
    Ok(bytes[body_start..].to_vec())
}

fn project_agent_card(
    endpoint: &Endpoint,
    card: &Value,
    agent_id: &str,
    protocol_version: &str,
    credential_ref: &Value,
) -> Result<Value, A2aError> {
    let object = card
        .as_object()
        .ok_or_else(|| A2aError::new("a2a_card_invalid", "A2A Agent Cardがobjectではない"))?;
    let name = bounded_string(object.get("name"), "name", 256)?;
    let description = bounded_string(object.get("description"), "description", 1024)?;
    let version = bounded_string(object.get("version"), "version", 128)?;
    let schemes = match object.get("securitySchemes") {
        Some(Value::Object(value)) => {
            if value.len() > 16 {
                return Err(A2aError::new(
                    "a2a_card_invalid",
                    "A2A Agent Card security scheme数が多すぎる",
                ));
            }
            let mut names = Vec::with_capacity(value.len());
            for name in value.keys() {
                if name.is_empty() || name.chars().count() > 128 {
                    return Err(A2aError::new(
                        "a2a_card_invalid",
                        "A2A Agent Card security scheme名の長さが不正",
                    ));
                }
                names.push(name.clone());
            }
            names
        }
        Some(_) => {
            return Err(A2aError::new(
                "a2a_card_invalid",
                "A2A Agent Card securitySchemesがobjectではない",
            ));
        }
        None => Vec::new(),
    };
    let security_requirements_present = card_has_security_requirements(object);
    let unauthenticated = schemes.is_empty() && !security_requirements_present;
    let interfaces = object
        .get("supportedInterfaces")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            A2aError::new(
                "a2a_card_invalid",
                "A2A Agent Card supportedInterfacesがない",
            )
        })?;
    if interfaces.is_empty() || interfaces.len() > 8 {
        return Err(A2aError::new(
            "a2a_card_invalid",
            "A2A Agent Card interface数が不正",
        ));
    }
    let mut projected_interfaces = Vec::new();
    for interface in interfaces {
        let interface = interface
            .as_object()
            .ok_or_else(|| A2aError::new("a2a_card_invalid", "A2A interfaceがobjectではない"))?;
        let interface_url = bounded_string(interface.get("url"), "supportedInterfaces.url", 2048)?;
        let protocol_version = bounded_string(
            interface.get("protocolVersion"),
            "supportedInterfaces.protocolVersion",
            128,
        )?;
        let (transport, interface_hash) = hash_interface_endpoint(&interface_url)?;
        let binding = interface
            .get("protocolBinding")
            .and_then(Value::as_str)
            .ok_or_else(|| A2aError::new("a2a_card_invalid", "A2A protocolBindingがない"))?;
        let binding = match binding.to_ascii_lowercase().as_str() {
            "jsonrpc" => "jsonrpc",
            "grpc" => "grpc",
            "http+json" | "http_rest" => "http_rest",
            _ => "unknown",
        };
        let executable = binding == "jsonrpc"
            && protocol_version == "1.0"
            && unauthenticated
            && parse_endpoint(&interface_url).is_ok();
        let (status, reason) = if executable {
            (
                "supported",
                "認証情報なしのA2A v1.0 JSON-RPC SendMessage consumerを接続済み",
            )
        } else {
            (
                "unsupported",
                "A2A v1.0 JSON-RPC SendMessage条件（interface、認証、接続先）を満たさない",
            )
        };
        projected_interfaces.push(json!({
            "binding": binding,
            "protocol_version": protocol_version,
            "interface_hash": interface_hash,
            "transport": transport,
            "status": status,
            "reason": reason
        }));
    }
    let capabilities = object
        .get("capabilities")
        .and_then(Value::as_object)
        .ok_or_else(|| A2aError::new("a2a_card_invalid", "A2A Agent Card capabilitiesがない"))?;
    let streaming = bool_field(capabilities, "streaming");
    let push_notifications =
        bool_field_alias(capabilities, "pushNotifications", "push_notifications");
    let state_transition_history = bool_field_alias(
        capabilities,
        "stateTransitionHistory",
        "state_transition_history",
    );
    let extended_agent_card =
        bool_field_alias(capabilities, "extendedAgentCard", "extended_agent_card");
    let skills = project_skills(object.get("skills"))?;
    let streams = if streaming {
        vec![json!({
            "stream_id": format!("{agent_id}-stream"), "task_id": format!("{agent_id}-card"), "mode": "sse",
            "status": {"status": "unsupported", "reason": "Agent Cardはstreamingを宣言するが、実購読consumerは未接続"},
            "event_types": ["task_status_update", "artifact_update"], "resumable": false, "authority_strip": true
        })]
    } else {
        Vec::new()
    };
    Ok(json!({
        "版": 1, "契約種別": "A2A外部概念射影", "protocol_version": protocol_version,
        "Agent Card": {
            "agent_id": agent_id, "display_name": name, "description_summary": description, "version": version,
            "endpoint_hash": sha256_tagged(format!("{}://{}{}", endpoint.scheme, endpoint.authority, endpoint.path).as_bytes()),
            "supported_interfaces": projected_interfaces,
            "capabilities": {"streaming": streaming, "push_notifications": push_notifications, "state_transition_history": state_transition_history, "extended_agent_card": extended_agent_card},
            "skills": skills, "authentication": {"schemes": schemes, "credential_ref": credential_ref, "secret_value_present": false, "security_requirements_present": security_requirements_present},
            "origin": "live_runtime", "status": "discovered"
        },
        "Task": [], "Message": [], "Artifact": [], "Stream": streams,
        "Trust": {"state": "pending_review", "evidence_source": "LIVE_RUNTIME", "reason": "Agent Cardは宣言情報であり、owner review前のTrustを生成しない", "requires_operator_review": true},
        "Capability diff": {"status": "not_evaluated", "added": [], "removed": [], "changed": [], "requires_operator_review": true, "evidence_source": "LIVE_RUNTIME"},
        "権限生成": "なし", "authority_strip": true, "公開範囲": "metadata_only", "証拠種別": "LIVE_RUNTIME"
    }))
}

fn security_requirement_field_present(value: Option<&Value>) -> bool {
    match value {
        None => false,
        Some(Value::Array(requirements)) => !requirements.is_empty(),
        Some(_) => true,
    }
}

fn card_has_security_requirements(card: &Map<String, Value>) -> bool {
    if security_requirement_field_present(card.get("securityRequirements"))
        || security_requirement_field_present(card.get("security"))
    {
        return true;
    }
    match card.get("skills") {
        Some(Value::Array(skills)) => skills.iter().any(|skill| {
            skill.as_object().is_none_or(|skill| {
                security_requirement_field_present(skill.get("securityRequirements"))
            })
        }),
        // project_skills rejects these shapes. Keep eligibility fail-closed as well.
        Some(_) | None => true,
    }
}

fn hash_interface_endpoint(uri: &str) -> Result<(String, String), A2aError> {
    let invalid = || {
        A2aError::new(
            "a2a_interface_url_invalid",
            "A2A supported interface URLの形式が不正",
        )
    };
    if uri.is_empty()
        || uri.len() > MAX_URI_BYTES
        || !uri.is_ascii()
        || uri.bytes().any(|byte| !(b'!'..=b'~').contains(&byte))
        || uri
            .chars()
            .any(|value| matches!(value, '@' | '?' | '#' | '\\'))
    {
        return Err(invalid());
    }
    let (scheme, remainder) = uri.split_once("://").ok_or_else(invalid)?;
    let transport = match scheme.to_ascii_lowercase().as_str() {
        "http" => "http",
        "https" => "https",
        _ => return Err(invalid()),
    };
    let (authority, path) = match remainder.split_once('/') {
        Some((authority, path)) => (authority, format!("/{path}")),
        None => (remainder, String::new()),
    };
    if authority.is_empty() || path.len() > 1024 || !path.is_ascii() {
        return Err(invalid());
    }
    if path
        .bytes()
        .any(|byte| !(b'!'..=b'~').contains(&byte) || matches!(byte, b'?' | b'#' | b'\\'))
    {
        return Err(invalid());
    }

    let valid_host = |host: &str| {
        if host.parse::<IpAddr>().is_ok() {
            return true;
        }
        host.len() <= 255
            && !host.is_empty()
            && host.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                    && label
                        .as_bytes()
                        .first()
                        .is_some_and(|byte| byte.is_ascii_alphanumeric())
                    && label
                        .as_bytes()
                        .last()
                        .is_some_and(|byte| byte.is_ascii_alphanumeric())
            })
    };
    let valid_port = |port: &str| port.parse::<u16>().is_ok_and(|value| value != 0);
    let valid_authority = if let Some(rest) = authority.strip_prefix('[') {
        let (address, suffix) = rest.split_once(']').ok_or_else(invalid)?;
        if address.parse::<std::net::Ipv6Addr>().is_err() {
            false
        } else if suffix.is_empty() {
            true
        } else {
            suffix.strip_prefix(':').is_some_and(valid_port)
        }
    } else if let Some((host, port)) = authority.rsplit_once(':') {
        !host.contains(':') && valid_host(host) && valid_port(port)
    } else {
        valid_host(authority)
    };
    if !valid_authority {
        return Err(invalid());
    }

    Ok((transport.to_string(), sha256_tagged(uri.as_bytes())))
}

fn bounded_string(value: Option<&Value>, field: &str, max: usize) -> Result<String, A2aError> {
    let value = value.and_then(Value::as_str).ok_or_else(|| {
        A2aError::new("a2a_card_invalid", format!("A2A Agent Card {field}がない"))
    })?;
    if value.is_empty() || value.chars().count() > max {
        return Err(A2aError::new(
            "a2a_card_invalid",
            format!("A2A Agent Card {field}の長さが不正"),
        ));
    }
    Ok(value.to_string())
}

fn bool_field(object: &Map<String, Value>, key: &str) -> bool {
    object.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn bool_field_alias(object: &Map<String, Value>, camel: &str, snake: &str) -> bool {
    object
        .get(camel)
        .or_else(|| object.get(snake))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn project_skills(value: Option<&Value>) -> Result<Vec<Value>, A2aError> {
    let skills = value
        .and_then(Value::as_array)
        .ok_or_else(|| A2aError::new("a2a_card_invalid", "A2A Agent Card skillsがない"))?;
    if skills.len() > 128 {
        return Err(A2aError::new(
            "a2a_card_invalid",
            "A2A Agent Card skillsが多すぎる",
        ));
    }
    skills.iter().map(|skill| {
        let skill = skill.as_object().ok_or_else(|| A2aError::new("a2a_card_invalid", "A2A skillがobjectではない"))?;
        Ok(json!({
            "skill_id": bounded_string(skill.get("id"), "skill.id", 128)?,
            "name": bounded_string(skill.get("name"), "skill.name", 128)?,
            "description_summary": bounded_string(skill.get("description"), "skill.description", 1024)?,
            "tags": string_array(skill.get("tags"), 32)?,
            "input_modes": string_array(skill.get("inputModes"), 16)?,
            "output_modes": string_array(skill.get("outputModes"), 16)?
        }))
    }).collect()
}

fn string_array(value: Option<&Value>, max: usize) -> Result<Vec<String>, A2aError> {
    let values = value
        .and_then(Value::as_array)
        .ok_or_else(|| A2aError::new("a2a_card_invalid", "A2A Agent Cardの配列fieldがない"))?;
    if values.len() > max {
        return Err(A2aError::new(
            "a2a_card_invalid",
            "A2A Agent Cardの配列fieldが大きすぎる",
        ));
    }
    values
        .iter()
        .map(|value| {
            let value = value
                .as_str()
                .ok_or_else(|| A2aError::new("a2a_card_invalid", "A2A配列要素が文字列ではない"))?;
            if value.is_empty() || value.chars().count() > MAX_STRING_CHARS {
                return Err(A2aError::new("a2a_card_invalid", "A2A配列要素長が不正"));
            }
            Ok(value.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn credential() -> Value {
        json!({"credential_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "purpose": "A2A接続", "target": "remote-agent-example", "required": false, "status": "missing"})
    }
    fn card() -> Value {
        json!({"name": "外部Agent例", "description": "外部Agentの宣言情報", "version": "1.0.0", "supportedInterfaces": [{"url": "http://127.0.0.1:8090/rpc", "protocolBinding": "JSONRPC", "protocolVersion": "1.0"}], "capabilities": {"streaming": true, "pushNotifications": false, "stateTransitionHistory": true, "extendedAgentCard": false}, "securitySchemes": {"oauth2": {}}, "skills": [{"id": "summarize", "name": "要約", "description": "要約", "tags": ["summary"], "inputModes": ["text/plain"], "outputModes": ["text/plain"]}]})
    }
    #[test]
    fn Agent_Cardをmetadata_onlyへ射影し秘密とendpointを返さない() {
        let endpoint = parse_endpoint("http://127.0.0.1:8080/.well-known/agent-card.json").unwrap();
        let projected = project_agent_card(
            &endpoint,
            &card(),
            "remote-agent-example",
            "1.0",
            &credential(),
        )
        .unwrap();
        assert_eq!(projected["権限生成"], "なし");
        assert_eq!(projected["公開範囲"], "metadata_only");
        assert_eq!(projected["Trust"]["state"], "pending_review");
        assert!(projected["Agent Card"].get("endpoint").is_none());
        assert_eq!(
            projected["Agent Card"]["supported_interfaces"][0]["transport"],
            "http"
        );
        assert_eq!(
            projected["Agent Card"]["supported_interfaces"][0]["protocol_version"],
            "1.0"
        );
        assert_eq!(
            projected["Agent Card"]["supported_interfaces"][0]["status"],
            "unsupported"
        );
        assert_eq!(
            projected["Agent Card"]["supported_interfaces"][0]["interface_hash"],
            sha256_tagged(b"http://127.0.0.1:8090/rpc")
        );
        assert!(!projected.to_string().contains("127.0.0.1:8090/rpc"));
        assert_eq!(projected["Stream"][0]["status"]["status"], "unsupported");
        assert_eq!(
            projected["Agent Card"]["authentication"]["secret_value_present"],
            false
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn A2A_send_securityRequirementsがrootとskillで認証要件を拒否する() {
        let endpoint = parse_endpoint("http://127.0.0.1:8080/card").unwrap();
        let mut unauthenticated = card();
        unauthenticated["securitySchemes"] = json!({});
        let projected = project_agent_card(
            &endpoint,
            &unauthenticated,
            "remote-agent-example",
            "1.0",
            &credential(),
        )
        .unwrap();
        assert_eq!(
            projected["Agent Card"]["authentication"]["security_requirements_present"],
            false
        );
        assert_eq!(
            projected["Agent Card"]["supported_interfaces"][0]["status"],
            "supported"
        );

        for skill_level in [false, true] {
            let mut protected = unauthenticated.clone();
            let requirements = json!([{ "oauth2": ["send"] }]);
            if skill_level {
                protected["skills"][0]["securityRequirements"] = requirements;
            } else {
                protected["securityRequirements"] = requirements;
            }
            let projected = project_agent_card(
                &endpoint,
                &protected,
                "remote-agent-example",
                "1.0",
                &credential(),
            )
            .unwrap();
            assert_eq!(
                projected["Agent Card"]["authentication"]["security_requirements_present"], true,
                "skill_level={skill_level}"
            );
            assert_eq!(
                projected["Agent Card"]["supported_interfaces"][0]["status"], "unsupported",
                "skill_level={skill_level}"
            );
        }

        for malformed in [Value::String("invalid".into()), Value::Null] {
            let mut protected = unauthenticated.clone();
            protected["securityRequirements"] = malformed;
            let projected = project_agent_card(
                &endpoint,
                &protected,
                "remote-agent-example",
                "1.0",
                &credential(),
            )
            .unwrap();
            assert_eq!(
                projected["Agent Card"]["supported_interfaces"][0]["status"],
                "unsupported"
            );
        }
    }

    #[test]
    fn interface_URLに秘密や危険なauthorityを含めない() {
        for uri in [
            "https://agent.example/rpc?token=secret-marker",
            "https://user@agent.example/rpc",
            "https://agent.example/rpc#fragment",
            "file://agent.example/rpc",
            "https://-invalid.example/rpc",
        ] {
            let mut value = card();
            value["supportedInterfaces"][0]["url"] = Value::String(uri.to_string());
            let error = project_agent_card(
                &parse_endpoint("http://127.0.0.1:8080/card").unwrap(),
                &value,
                "remote-agent-example",
                "1.0",
                &credential(),
            )
            .unwrap_err();
            assert_eq!(error.code, "a2a_interface_url_invalid");
            assert!(!error.message.contains("secret-marker"));
        }
    }
    #[test]
    fn httpsを許可し_HTTP外部hostと不正hostとuserinfoを拒否する() {
        let https = parse_endpoint("https://agent.example/card").expect("HTTPS endpoint");
        assert_eq!(https.scheme, "https");
        assert_eq!(https.port, 443);
        assert_eq!(
            transport_name("https://agent.example:8443/card").unwrap(),
            "https"
        );
        assert_eq!(
            parse_endpoint("http://example.com/card").unwrap_err().code,
            "a2a_http_non_loopback"
        );
        assert_eq!(
            parse_endpoint("http://user@127.0.0.1/card")
                .unwrap_err()
                .code,
            "a2a_endpoint_invalid"
        );
        for uri in [
            "https://-invalid.example/card",
            "https://invalid-.example/card",
            "https://bad_host.example/card",
            "https://agent.example/card?token=secret-marker",
            "https://agent.example/card#fragment",
        ] {
            assert_eq!(
                parse_endpoint(uri).unwrap_err().code,
                "a2a_endpoint_invalid"
            );
        }
        for uri in [
            "http://127.0.0.1/card?token=secret-marker",
            "http://127.0.0.1/card#fragment",
            "http://127.0.0.1/card with-space",
            "http://127.0.0.1/card\u{3000}wide-space",
        ] {
            assert_eq!(
                parse_endpoint(uri).unwrap_err().code,
                "a2a_endpoint_invalid"
            );
        }
    }

    #[test]
    fn A2A_send応答はTaskとMessageをbounded_projectionし本文を返さない() {
        let sentinel = "remote-private-body-marker";
        let task = json!({
            "jsonrpc":"2.0", "id":"request-1", "result":{"task":{
                "kind":"task", "id":"task-1", "contextId":"context-1",
                "status":{"state":"TASK_STATE_WORKING", "message":{"kind":"message", "messageId":"m-1", "role":"ROLE_AGENT", "parts":[{"kind":"text","text":sentinel}]}},
                "artifacts":[{"artifactId":"a-1","parts":[{"kind":"text","text":sentinel}]}],
                "history":[{"kind":"message","messageId":"m-2","role":"ROLE_AGENT","parts":[{"kind":"text","text":sentinel}]}],
                "metadata":{"private":sentinel}
            }}
        });
        let raw = serde_json::to_vec(&task).unwrap();
        let projected = project_send_message_response(&raw, "request-1").unwrap();
        assert_eq!(projected["kind"], "task");
        assert_eq!(projected["task_id"], "task-1");
        assert_eq!(projected["state"], "working");
        assert_eq!(projected["artifact_count"], 1);
        assert_eq!(projected["history_count"], 1);
        assert!(!projected.to_string().contains(sentinel));

        let message = json!({
            "jsonrpc":"2.0", "id":"request-2", "result":{"message":{
                "kind":"message", "messageId":"message-1", "contextId":"context-2",
                "role":"ROLE_AGENT", "parts":[{"kind":"text","text":sentinel}],
                "metadata":{"private":sentinel}
            }}
        });
        let projected =
            project_send_message_response(&serde_json::to_vec(&message).unwrap(), "request-2")
                .unwrap();
        assert_eq!(projected["kind"], "message");
        assert_eq!(projected["message_id"], "message-1");
        assert_eq!(projected["part_count"], 1);
        assert!(!projected.to_string().contains(sentinel));
    }

    #[test]
    fn A2A_send応答のstale_id_errorと曖昧な結果を拒否する() {
        let response = json!({"jsonrpc":"2.0","id":"other-request","result":{"message":{
            "kind":"message","messageId":"m-1","role":"ROLE_AGENT","parts":[{"kind":"text","text":"response-secret-marker"}]
        }}});
        let error =
            project_send_message_response(&serde_json::to_vec(&response).unwrap(), "request-1")
                .expect_err("stale request IDを拒否");
        assert_eq!(error.code, "a2a_response_invalid");
        assert!(!error.message.contains("response-secret-marker"));

        let ambiguous = json!({"jsonrpc":"2.0","id":"request-1","result":{
            "task":{"kind":"task","id":"t-1","status":{"state":"working"}},
            "message":{"kind":"message","messageId":"m-1","role":"ROLE_AGENT","parts":[{"kind":"text","text":"response-secret-marker"}]}
        }});
        assert_eq!(
            project_send_message_response(&serde_json::to_vec(&ambiguous).unwrap(), "request-1")
                .unwrap_err()
                .code,
            "a2a_response_invalid"
        );

        let rejected = json!({"jsonrpc":"2.0","id":"request-1","error":{"code":-32000,"message":"response-secret-marker"}});
        let error =
            project_send_message_response(&serde_json::to_vec(&rejected).unwrap(), "request-1")
                .expect_err("JSON-RPC errorを拒否");
        assert_eq!(error.code, "a2a_remote_rejected");
        assert!(!error.message.contains("response-secret-marker"));
    }

    #[test]
    fn A2A_GetTaskは応答IDと要求TaskIDを照合し履歴と本文を返さない() {
        let sentinel = "remote-task-body-secret";
        let response = json!({
            "jsonrpc":"2.0", "id":"request-get-1", "result":{
                "kind":"task", "id":"task-exact", "contextId":"context-1",
                "status":{"state":"TASK_STATE_WORKING","message":{"parts":[{"text":sentinel}]}},
                "artifacts":[{"parts":[{"text":sentinel}]}], "history":[],
                "metadata":{"secret":sentinel}
            }
        });
        let projected = project_get_task_response(
            &serde_json::to_vec(&response).unwrap(),
            "request-get-1",
            "task-exact",
        )
        .expect("GetTask bounded projection");
        assert_eq!(projected["task_id"], "task-exact");
        assert_eq!(projected["state"], "working");
        assert_eq!(projected["artifact_count"], 1);
        assert_eq!(projected["history_count"], 0);
        assert!(!projected.to_string().contains(sentinel));

        let mut stale_task = response.clone();
        stale_task["result"]["id"] = json!("other-task");
        assert_eq!(
            project_get_task_response(
                &serde_json::to_vec(&stale_task).unwrap(),
                "request-get-1",
                "task-exact",
            )
            .unwrap_err()
            .code,
            "a2a_response_invalid"
        );
        let mut unexpected_history = response.clone();
        unexpected_history["result"]["history"] = json!([{"text":sentinel}]);
        let error = project_get_task_response(
            &serde_json::to_vec(&unexpected_history).unwrap(),
            "request-get-1",
            "task-exact",
        )
        .unwrap_err();
        assert_eq!(error.code, "a2a_response_invalid");
        assert!(!error.message.contains(sentinel));
        assert!(!valid_task_id("task\u{202e}spoof"));
        let mut bidi_id = response.clone();
        bidi_id["result"]["id"] = json!("task\u{202e}spoof");
        assert_eq!(
            project_get_task_response(
                &serde_json::to_vec(&bidi_id).unwrap(),
                "request-get-1",
                "task\u{202e}spoof",
            )
            .unwrap_err()
            .code,
            "a2a_response_invalid"
        );
    }

    #[test]
    fn loopback_A2A_GetTaskはhistoryLength_zeroを送りTask状態だけを返す() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback GetTask待受");
        let address = listener.local_addr().expect("loopback address");
        let server = thread::spawn(move || -> Result<Value, String> {
            let (mut stream, _) = listener.accept().map_err(|error| error.to_string())?;
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            let mut request = Vec::new();
            let mut chunk = [0u8; 1024];
            let header_end = loop {
                let size = stream.read(&mut chunk).map_err(|error| error.to_string())?;
                if size == 0 || request.len() + size > MAX_JSON_RPC_BYTES + MAX_HEADER_BYTES {
                    return Err("GetTask request truncated or oversized".into());
                }
                request.extend_from_slice(&chunk[..size]);
                if let Some(position) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                    break position + 4;
                }
            };
            let header =
                std::str::from_utf8(&request[..header_end]).map_err(|error| error.to_string())?;
            if !header.starts_with("POST /rpc HTTP/1.1\r\n")
                || !header
                    .lines()
                    .any(|line| line.eq_ignore_ascii_case("A2A-Version: 1.0"))
            {
                return Err("GetTask header mismatch".into());
            }
            let content_length = header
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("Content-Length")
                        .then(|| value.trim().parse::<usize>().ok())?
                })
                .ok_or_else(|| "Content-Length missing".to_string())?;
            while request.len() < header_end + content_length {
                let size = stream.read(&mut chunk).map_err(|error| error.to_string())?;
                if size == 0 {
                    return Err("GetTask body truncated".into());
                }
                request.extend_from_slice(&chunk[..size]);
            }
            let body: Value =
                serde_json::from_slice(&request[header_end..header_end + content_length])
                    .map_err(|error| error.to_string())?;
            let response_body = serde_json::to_vec(&json!({
                "jsonrpc":"2.0", "id":body["id"], "result":{
                    "kind":"task", "id":"task-query-fixture", "contextId":"context-1",
                    "status":{"state":"TASK_STATE_COMPLETED","message":{"parts":[{"text":"private-task-body"}]}},
                    "artifacts":[], "history":[]
                }
            })).map_err(|error| error.to_string())?;
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", response_body.len());
            stream
                .write_all(response.as_bytes())
                .map_err(|error| error.to_string())?;
            stream
                .write_all(&response_body)
                .map_err(|error| error.to_string())?;
            stream.flush().map_err(|error| error.to_string())?;
            Ok(body)
        });
        let projected = get_task(
            &format!("http://{address}/rpc"),
            "task-query-fixture",
            Instant::now() + Duration::from_secs(5),
        )
        .expect("loopback GetTask bounded result");
        let request = server
            .join()
            .expect("GetTask server panic")
            .expect("request valid");
        assert_eq!(request["method"], "GetTask");
        assert_eq!(request["params"]["id"], "task-query-fixture");
        assert_eq!(request["params"]["historyLength"], 0);
        assert_eq!(projected["state"], "completed");
        assert_eq!(projected["history_count"], 0);
        assert!(!projected.to_string().contains("private-task-body"));
    }

    #[test]
    fn loopback_A2A_sendはv1_headerを送り応答をbounded_projectionする() {
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback SendMessage待受");
        let address = listener.local_addr().expect("loopback address");
        let server = thread::spawn(move || -> Result<Value, String> {
            let (mut stream, _) = listener.accept().map_err(|error| error.to_string())?;
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            let mut request = Vec::new();
            let mut chunk = [0u8; 1024];
            let header_end = loop {
                let size = stream.read(&mut chunk).map_err(|error| error.to_string())?;
                if size == 0
                    || request.len().saturating_add(size) > MAX_JSON_RPC_BYTES + MAX_HEADER_BYTES
                {
                    return Err("request header/body truncated or oversized".into());
                }
                request.extend_from_slice(&chunk[..size]);
                if let Some(position) = request.windows(4).position(|window| window == b"\r\n\r\n")
                {
                    break position + 4;
                }
            };
            let header =
                std::str::from_utf8(&request[..header_end]).map_err(|error| error.to_string())?;
            if !header.starts_with("POST /rpc HTTP/1.1\r\n")
                || !header
                    .lines()
                    .any(|line| line.eq_ignore_ascii_case("A2A-Version: 1.0"))
                || !header
                    .lines()
                    .any(|line| line.eq_ignore_ascii_case("Content-Type: application/json"))
            {
                return Err("SendMessage HTTP method or required headers mismatch".into());
            }
            let content_length = header
                .lines()
                .find_map(|line| {
                    line.strip_prefix("Content-Length: ")
                        .and_then(|value| value.parse::<usize>().ok())
                })
                .ok_or_else(|| "Content-Length missing".to_string())?;
            while request.len() < header_end + content_length {
                let size = stream.read(&mut chunk).map_err(|error| error.to_string())?;
                if size == 0 {
                    return Err("SendMessage body truncated".into());
                }
                request.extend_from_slice(&chunk[..size]);
            }
            if request.len() != header_end + content_length {
                return Err("unexpected request tail".into());
            }
            let body: Value = serde_json::from_slice(&request[header_end..])
                .map_err(|error| error.to_string())?;
            if body["method"] != "SendMessage"
                || body["params"]["message"]["parts"][0]["text"] != "fixture-user-message"
            {
                return Err("A2A SendMessage JSON-RPC request mismatch".into());
            }
            let response_body = serde_json::to_vec(&json!({
                "jsonrpc":"2.0", "id":body["id"], "result":{"task":{
                    "kind":"task", "id":"fixture-task", "contextId":"fixture-context",
                    "status":{"state":"TASK_STATE_SUBMITTED","message":{"kind":"message","messageId":"fixture-response","role":"ROLE_AGENT","parts":[{"kind":"text","text":"fixture-response-secret"}]}},
                    "artifacts":[], "history":[]
                }}
            })).map_err(|error| error.to_string())?;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                response_body.len()
            );
            stream
                .write_all(response.as_bytes())
                .map_err(|error| error.to_string())?;
            stream
                .write_all(&response_body)
                .map_err(|error| error.to_string())?;
            stream.flush().map_err(|error| error.to_string())?;
            Ok(body)
        });
        let uri = format!("http://{address}/rpc");
        let projected = send_message(
            &uri,
            "fixture-user-message",
            Instant::now() + Duration::from_secs(5),
        )
        .expect("loopback SendMessage response");
        let request = server
            .join()
            .expect("loopback server thread")
            .expect("valid SendMessage request");
        assert_eq!(request["params"]["message"]["role"], "ROLE_USER");
        assert_eq!(projected["kind"], "task");
        assert_eq!(projected["task_id"], "fixture-task");
        assert!(!projected.to_string().contains("fixture-response-secret"));
    }

    #[test]
    fn loopback_HTTPからAgent_Cardを取得してmetadata_onlyへ射影する() {
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback待受");
        let address = listener.local_addr().expect("接続先");
        let response_body = serde_json::to_vec(&card()).expect("Agent CardのJSON");
        let response_length = response_body.len();
        let worker = thread::spawn(move || -> Result<String, String> {
            let started_at = std::time::Instant::now();
            let (mut stream, peer) = listener
                .accept()
                .map_err(|error| format!("Agent Card要求受付失敗: {error}"))?;
            let accepted_at = std::time::Instant::now();
            let local = stream
                .local_addr()
                .map_err(|error| format!("Agent Card待受取得失敗: {error}"))?;
            // 全suite負荷時にもfixture serverがclient要求の到着前に閉じないようにする。
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .map_err(|error| {
                    format!("peer={peer}; local={local}; 要求読取期限設定失敗: {error}")
                })?;
            stream
                .set_write_timeout(Some(Duration::from_secs(10)))
                .map_err(|error| {
                    format!("peer={peer}; local={local}; 応答書込期限設定失敗: {error}")
                })?;
            let mut request = Vec::new();
            let mut chunk = [0u8; 512];
            while !request.windows(4).any(|value| value == b"\r\n\r\n") {
                let size = stream.read(&mut chunk).map_err(|error| {
                    format!("peer={peer}; local={local}; 要求読取失敗: {error}")
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
                        "peer={peer}; local={local}; 要求header上限超過; received_bytes={}",
                        request.len()
                    ));
                }
            }
            let request_received_at = std::time::Instant::now();
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
                    format!("peer={peer}; local={local}; 応答側shutdown失敗: {error}")
                })?;
            let response_written_at = std::time::Instant::now();
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
                        "peer={peer}; local={local}; 応答後peer close観測失敗: {error}"
                    ));
                }
            };
            let peer_closed_at = std::time::Instant::now();
            Ok(format!(
                "peer={peer}; local={local}; request_bytes={}; response_bytes={}; response=written; peer_close={peer_close}; accepted_after_ms={}; request_received_after_ms={}; response_written_after_ms={}; peer_closed_after_ms={}",
                request.len(),
                response.len(),
                accepted_at.duration_since(started_at).as_millis(),
                request_received_at.duration_since(started_at).as_millis(),
                response_written_at.duration_since(started_at).as_millis(),
                peer_closed_at.duration_since(started_at).as_millis()
            ))
        });
        let uri = format!("http://{address}/.well-known/agent-card.json");
        let client_started_at = std::time::Instant::now();
        let projected = fetch_agent_card(&uri, "remote-agent-example", "1.0", &credential());
        let client_elapsed_ms = client_started_at.elapsed().as_millis();
        let worker_result = worker.join().expect("Agent Card試験worker panic");
        let server_observation = worker_result.unwrap_or_else(|error| {
            panic!("Agent Card試験server失敗: {error}; client={projected:?}")
        });
        let projected = projected.unwrap_or_else(|error| {
            panic!(
                "loopback Agent Card射影失敗: {error:?}; client_elapsed_ms={client_elapsed_ms}; server={server_observation}"
            )
        });
        assert_eq!(projected["証拠種別"], "LIVE_RUNTIME");
        assert_eq!(projected["公開範囲"], "metadata_only");
        assert_eq!(projected["Agent Card"]["origin"], "live_runtime");
        assert!(projected["Agent Card"].get("endpoint").is_none());
    }

    #[test]
    fn credential実値をAgent_Card取得へ持ち込めない() {
        let mut credential = credential();
        credential["required"] = Value::Bool(true);
        let error = validate_credential_ref(&credential, "remote-agent-example")
            .expect_err("credential required拒否");
        assert_eq!(error.code, "a2a_credential_unavailable");
    }

    #[test]
    fn A2A_Credential参照の未知fieldと別Agent対象を拒否する() {
        let mut with_secret = credential();
        with_secret["secret_value"] = Value::String("secret-marker".into());
        assert_eq!(
            validate_credential_ref(&with_secret, "remote-agent-example")
                .unwrap_err()
                .code,
            "a2a_credential_secret_received"
        );

        let mut wrong_target = credential();
        wrong_target["target"] = Value::String("other-agent".into());
        assert_eq!(
            validate_credential_ref(&wrong_target, "remote-agent-example")
                .unwrap_err()
                .code,
            "a2a_credential_ref_invalid"
        );

        let mut unknown_field = credential();
        unknown_field["extra"] = Value::Bool(true);
        assert_eq!(
            validate_credential_ref(&unknown_field, "remote-agent-example")
                .unwrap_err()
                .code,
            "a2a_credential_ref_invalid"
        );
    }
}
