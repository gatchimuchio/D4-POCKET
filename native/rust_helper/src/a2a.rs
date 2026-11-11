//! A2A v1.0 Agent Card取得、JSON-RPC SendMessage／GetTask／SendStreamingMessageのbounded transport。
//! 認証credentialと外部raw contentの保存は扱わない。
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
const MAX_TASK_ARTIFACTS: usize = 32;
const MAX_ARTIFACT_PARTS: usize = 32;
const MAX_ARTIFACT_TEXT_BYTES: usize = 64 * 1024;
const MAX_A2A_STREAM_EVENTS: usize = 32;

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

pub(crate) fn send_stream(
    uri: &str,
    message_text: &str,
    deadline: Instant,
) -> Result<Value, A2aError> {
    let _endpoint = parse_endpoint(uri)?;
    if message_text.trim().is_empty()
        || message_text.len() > MAX_SEND_MESSAGE_BYTES
        || message_text.chars().any(|character| {
            character == '\0'
                || (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
                || matches!(character, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        || Instant::now() >= deadline
    {
        return Err(A2aError::new(
            "a2a_stream_request_invalid",
            "A2A Stream要求の本文または期限が不正",
        ));
    }
    let mut request_id_bytes = [0u8; 16];
    let mut message_id_bytes = [0u8; 16];
    getrandom::getrandom(&mut request_id_bytes)
        .and_then(|_| getrandom::getrandom(&mut message_id_bytes))
        .map_err(|_| A2aError::new("a2a_stream_id_unavailable", "A2A識別値を生成できない"))?;
    let request_id = format!("d4p-stream-{}", hex::encode(request_id_bytes));
    let message_id = format!("d4p-stream-msg-{}", hex::encode(message_id_bytes));
    let request = json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "SendStreamingMessage",
        "params": {
            "message": {
                "messageId": message_id,
                "role": "ROLE_USER",
                "parts": [{ "text": message_text }]
            }
        }
    });
    let request_bytes = serde_json::to_vec(&request)
        .map_err(|_| A2aError::new("a2a_stream_request_invalid", "A2A Stream要求を構成できない"))?;
    if request_bytes.len() > MAX_JSON_RPC_BYTES {
        return Err(A2aError::new(
            "a2a_stream_request_invalid",
            "A2A Stream JSON-RPC requestの上限を超過",
        ));
    }
    let raw = crate::broker::update_download::send_a2a_json_rpc_stream_document(
        uri,
        &request_bytes,
        deadline,
    )
    .map_err(map_a2a_stream_transport_error)?;
    project_stream_response(&raw, &request_id)
}

pub(crate) fn get_task(
    uri: &str,
    task_id: &str,
    artifact_selection: Option<(&str, &str)>,
    deadline: Instant,
) -> Result<Value, A2aError> {
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
    project_get_task_response_with_artifact(&raw, &request_id, task_id, artifact_selection)
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
    project_get_task_response_with_artifact(raw, request_id, expected_task_id, None)
}

fn project_get_task_response_with_artifact(
    raw: &[u8],
    request_id: &str,
    expected_task_id: &str,
    artifact_selection: Option<(&str, &str)>,
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
    project_task_metadata(
        task,
        &response_hash,
        Some(expected_task_id),
        true,
        true,
        artifact_selection,
    )
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
        project_task_metadata(task, &response_hash, None, false, false, None)
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

fn map_a2a_stream_transport_error(error: DownloadError) -> A2aError {
    match error {
        DownloadError::TimedOut => A2aError::new("a2a_timeout", "A2A Streamが15秒の期限を超過"),
        DownloadError::DocumentTooLarge => {
            A2aError::new("a2a_stream_too_large", "A2A Streamが64 KiB上限を超過")
        }
        DownloadError::NonPublicAddress => A2aError::new(
            "a2a_https_non_public_address",
            "https A2A接続先がpublic IP範囲ではない",
        ),
        DownloadError::InvalidRequest => {
            A2aError::new("a2a_endpoint_invalid", "A2A Stream接続先が不正")
        }
        DownloadError::InvalidHeaders | DownloadError::RedirectOrUnexpectedStatus => {
            A2aError::new("a2a_stream_response_invalid", "A2A Stream応答headerが不正")
        }
        _ => A2aError::new(
            "a2a_stream_transport_failed",
            "A2A Stream通信を完了できない",
        ),
    }
}

fn project_stream_response(raw: &[u8], request_id: &str) -> Result<Value, A2aError> {
    if raw.is_empty() || raw.len() > 64 * 1024 {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Streamの応答長が不正",
        ));
    }
    let text = std::str::from_utf8(raw)
        .map_err(|_| A2aError::new("a2a_stream_response_invalid", "A2A StreamがUTF-8ではない"))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut events = Vec::new();
    let mut data_lines = Vec::<&str>::new();
    for raw_line in text.split_terminator('\n') {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.contains('\r') {
            return Err(A2aError::new(
                "a2a_stream_response_invalid",
                "A2A Streamの行区切りが不正",
            ));
        }
        if line.is_empty() {
            if !data_lines.is_empty() {
                if events.len() >= MAX_A2A_STREAM_EVENTS {
                    return Err(A2aError::new(
                        "a2a_stream_event_limit",
                        "A2A Streamが32 event上限を超過",
                    ));
                }
                events.push(data_lines.join("\n"));
                data_lines.clear();
            }
            continue;
        }
        if line.starts_with(':') {
            continue;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        if field != "data" {
            return Err(A2aError::new(
                "a2a_stream_response_invalid",
                "A2A Streamの未対応SSE fieldを拒否",
            ));
        }
        data_lines.push(value.strip_prefix(' ').unwrap_or(value));
    }
    if !data_lines.is_empty() {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Streamの最終eventが区切られていない",
        ));
    }
    if events.is_empty() {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Streamにeventがない",
        ));
    }

    let mut projections = Vec::with_capacity(events.len());
    let mut stream_kind = None;
    let mut terminal_state = None;
    let mut task_identity: Option<(String, Option<String>)> = None;
    for (index, event_data) in events.iter().enumerate() {
        if terminal_state.is_some() {
            return Err(A2aError::new(
                "a2a_stream_response_invalid",
                "A2A Streamが終端event後に追加eventを含む",
            ));
        }
        let response: Value = crate::broker::json_input::read_unique(event_data).map_err(|_| {
            A2aError::new(
                "a2a_stream_response_invalid",
                "A2A Stream JSON-RPC eventが不正",
            )
        })?;
        let response = response.as_object().ok_or_else(|| {
            A2aError::new(
                "a2a_stream_response_invalid",
                "A2A Stream eventがobjectではない",
            )
        })?;
        if response.len() != 3
            || response.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
            || response.get("id").and_then(Value::as_str) != Some(request_id)
            || response
                .keys()
                .any(|key| !["jsonrpc", "id", "result"].contains(&key.as_str()))
        {
            return Err(A2aError::new(
                "a2a_stream_response_invalid",
                "A2A Stream JSON-RPC version、request ID、またはfieldが不正",
            ));
        }
        let result = response
            .get("result")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                A2aError::new(
                    "a2a_stream_response_invalid",
                    "A2A Stream resultがobjectではない",
                )
            })?;
        if result.len() != 1 {
            return Err(A2aError::new(
                "a2a_stream_response_invalid",
                "A2A StreamResponseは一つのvariantだけを要求",
            ));
        }
        let event_hash = sha256_tagged(event_data.as_bytes());
        let (variant, value) = result.iter().next().expect("one result variant");
        let (projection, kind, terminal) = match variant.as_str() {
            "task" if index == 0 => {
                let (projection, task_id, context_id, state) =
                    project_stream_task(value, &event_hash)?;
                let terminal = is_terminal_task_state(state);
                task_identity = Some((task_id, context_id));
                (projection, "task", terminal.then_some(state.to_string()))
            }
            "message" if index == 0 && events.len() == 1 => {
                (project_stream_message(value, &event_hash)?, "message", None)
            }
            "statusUpdate" if index > 0 && stream_kind_is_task(&projections) => {
                let (projection, state, event_task_id, event_context_id) =
                    project_stream_status_update(value, &event_hash)?;
                ensure_stream_task_identity(
                    task_identity.as_ref(),
                    &event_task_id,
                    event_context_id.as_deref(),
                )?;
                let terminal = is_terminal_task_state(&state).then_some(state);
                (projection, "task", terminal)
            }
            "artifactUpdate" if index > 0 && stream_kind_is_task(&projections) => {
                let (projection, event_task_id, event_context_id) =
                    project_stream_artifact_update(value, &event_hash)?;
                ensure_stream_task_identity(
                    task_identity.as_ref(),
                    &event_task_id,
                    event_context_id.as_deref(),
                )?;
                (projection, "task", None)
            }
            _ => {
                return Err(A2aError::new(
                    "a2a_stream_response_invalid",
                    "A2A StreamResponse variantが未対応",
                ))
            }
        };
        projections.push(projection);
        stream_kind = Some(kind);
        terminal_state = terminal;
    }
    let stream_kind = stream_kind
        .ok_or_else(|| A2aError::new("a2a_stream_response_invalid", "A2A Streamにeventがない"))?;
    if stream_kind == "task" && terminal_state.is_none() {
        return Err(A2aError::new(
            "a2a_stream_nonterminal",
            "A2A StreamはTaskの終端状態を確認できないため結果不明",
        ));
    }
    Ok(json!({
        "stream_kind": stream_kind,
        "state": terminal_state,
        "event_count": projections.len(),
        "events": projections,
        "result_hash": sha256_tagged(raw),
    }))
}

fn stream_kind_is_task(projections: &[Value]) -> bool {
    projections
        .first()
        .and_then(|event| event.get("kind"))
        .and_then(Value::as_str)
        == Some("task")
}

fn is_terminal_task_state(state: &str) -> bool {
    matches!(state, "completed" | "failed" | "canceled" | "rejected")
}

fn ensure_stream_task_identity(
    task_identity: Option<&(String, Option<String>)>,
    event_task_id: &str,
    event_context_id: Option<&str>,
) -> Result<(), A2aError> {
    let Some((task_id, context_id)) = task_identity else {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Stream eventに先行Taskがない",
        ));
    };
    if task_id != event_task_id || context_id.as_deref() != event_context_id {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Stream eventが先行Taskの識別子と一致しない",
        ));
    }
    Ok(())
}

fn project_stream_task(
    value: &Value,
    event_hash: &str,
) -> Result<(Value, String, Option<String>, &'static str), A2aError> {
    let task = value.as_object().ok_or_else(|| {
        A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Stream Taskがobjectではない",
        )
    })?;
    if task.keys().any(|key| {
        ![
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
            "a2a_stream_response_invalid",
            "A2A Stream Taskに未知fieldがある",
        ));
    }
    let task_id = bounded_external_id(task.get("id"), "Stream Task id")?;
    let context_id = optional_external_id(task.get("contextId"), "Stream Task contextId")?;
    let status = task
        .get("status")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            A2aError::new(
                "a2a_stream_response_invalid",
                "A2A Stream Task statusが不正",
            )
        })?;
    if status
        .keys()
        .any(|key| !["state", "message", "timestamp"].contains(&key.as_str()))
        || status
            .get("message")
            .is_some_and(|message| !message.is_object())
        || status
            .get("timestamp")
            .is_some_and(|timestamp| timestamp.as_str().is_none())
        || task
            .get("metadata")
            .is_some_and(|metadata| !metadata.is_object())
    {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Stream Task statusまたはmetadataが不正",
        ));
    }
    let state = project_task_state(status.get("state"))?;
    let artifacts =
        bounded_optional_array_len(task.get("artifacts"), "artifacts", MAX_TASK_ARTIFACTS)?;
    if let Some(values) = task.get("artifacts").and_then(Value::as_array) {
        for artifact in values {
            validate_stream_artifact(artifact)?;
        }
    }
    let history = bounded_optional_array_len(task.get("history"), "history", 64)?;
    if let Some(values) = task.get("history").and_then(Value::as_array) {
        for message in values {
            project_stream_message(message, event_hash)?;
        }
    }
    Ok((
        json!({
            "kind": "task",
            "task_id_hash": sha256_tagged(task_id.as_bytes()),
            "context_id_hash": context_id.as_deref().map(|id| sha256_tagged(id.as_bytes())),
            "state": state,
            "artifact_count": artifacts,
            "history_count": history,
            "event_hash": event_hash,
        }),
        task_id,
        context_id,
        state,
    ))
}

fn project_stream_message(message: &Value, event_hash: &str) -> Result<Value, A2aError> {
    let message = message.as_object().ok_or_else(|| {
        A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Stream Messageがobjectではない",
        )
    })?;
    if message.keys().any(|key| {
        ![
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
            "a2a_stream_response_invalid",
            "A2A Stream Message fieldまたはkindが不正",
        ));
    }
    let message_id = bounded_external_id(message.get("messageId"), "Message messageId")?;
    let task_id = optional_external_id(message.get("taskId"), "Message taskId")?;
    let context_id = optional_external_id(message.get("contextId"), "Message contextId")?;
    let role = message
        .get("role")
        .and_then(Value::as_str)
        .filter(|value| matches!(*value, "ROLE_AGENT" | "ROLE_USER"))
        .ok_or_else(|| {
            A2aError::new(
                "a2a_stream_response_invalid",
                "A2A Stream Message roleが不正",
            )
        })?;
    let parts = message
        .get("parts")
        .and_then(Value::as_array)
        .ok_or_else(|| A2aError::new("a2a_stream_response_invalid", "Message partsが不正"))?;
    let part_count = validate_stream_parts(parts, 64)?.len();
    if part_count == 0 {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Stream Message partsが空",
        ));
    }
    if message
        .get("metadata")
        .is_some_and(|metadata| !metadata.is_object())
        || message.get("extensions").is_some_and(|extensions| {
            extensions.as_array().is_none_or(|items| {
                items.len() > 64
                    || items
                        .iter()
                        .any(|item| item.as_str().is_none_or(|text| text.len() > 2048))
            })
        })
        || message.get("referenceTaskIds").is_some_and(|ids| {
            ids.as_array().is_none_or(|items| {
                items.len() > 64
                    || items
                        .iter()
                        .any(|id| id.as_str().is_none_or(|text| text.len() > 256))
            })
        })
    {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Stream Message metadataまたはreferenceTaskIdsが不正",
        ));
    }
    Ok(json!({
        "kind": "message",
        "message_id_hash": sha256_tagged(message_id.as_bytes()),
        "task_id_hash": task_id.map(|id| sha256_tagged(id.as_bytes())),
        "context_id_hash": context_id.map(|id| sha256_tagged(id.as_bytes())),
        "role": role,
        "part_count": part_count,
        "event_hash": event_hash,
    }))
}

fn project_stream_status_update(
    update: &Value,
    event_hash: &str,
) -> Result<(Value, String, String, Option<String>), A2aError> {
    let update = update.as_object().ok_or_else(|| {
        A2aError::new(
            "a2a_stream_response_invalid",
            "A2A statusUpdateがobjectではない",
        )
    })?;
    if update
        .keys()
        .any(|key| !["taskId", "contextId", "status", "metadata"].contains(&key.as_str()))
    {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A statusUpdateに未知fieldがある",
        ));
    }
    let task_id = bounded_external_id(update.get("taskId"), "statusUpdate taskId")?;
    let context_id = bounded_external_id(update.get("contextId"), "statusUpdate contextId")?;
    let status = update
        .get("status")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            A2aError::new(
                "a2a_stream_response_invalid",
                "A2A statusUpdate statusが不正",
            )
        })?;
    if status
        .keys()
        .any(|key| !["state", "message", "timestamp"].contains(&key.as_str()))
    {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A statusUpdate statusに未知fieldがある",
        ));
    }
    let state = project_task_state(status.get("state"))?;
    if status
        .get("message")
        .is_some_and(|message| !message.is_object())
        || update
            .get("metadata")
            .is_some_and(|metadata| !metadata.is_object())
    {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A statusUpdateのMessageまたはmetadataが不正",
        ));
    }
    Ok((
        json!({
            "kind": "statusUpdate",
            "task_id_hash": sha256_tagged(task_id.as_bytes()),
            "context_id_hash": sha256_tagged(context_id.as_bytes()),
            "state": state,
            "event_hash": event_hash,
        }),
        state.to_string(),
        task_id,
        Some(context_id),
    ))
}

fn project_stream_artifact_update(
    update: &Value,
    event_hash: &str,
) -> Result<(Value, String, Option<String>), A2aError> {
    let update = update.as_object().ok_or_else(|| {
        A2aError::new(
            "a2a_stream_response_invalid",
            "A2A artifactUpdateがobjectではない",
        )
    })?;
    if update.keys().any(|key| {
        ![
            "taskId",
            "contextId",
            "artifact",
            "append",
            "lastChunk",
            "metadata",
        ]
        .contains(&key.as_str())
    }) {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A artifactUpdateに未知fieldがある",
        ));
    }
    let task_id = bounded_external_id(update.get("taskId"), "artifactUpdate taskId")?;
    let context_id = bounded_external_id(update.get("contextId"), "artifactUpdate contextId")?;
    let artifact = update
        .get("artifact")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            A2aError::new(
                "a2a_stream_response_invalid",
                "A2A artifactUpdate artifactが不正",
            )
        })?;
    validate_stream_artifact(&Value::Object(artifact.clone()))?;
    if artifact.keys().any(|key| {
        ![
            "artifactId",
            "name",
            "description",
            "parts",
            "metadata",
            "extensions",
        ]
        .contains(&key.as_str())
    }) {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A stream Artifactに未知fieldがある",
        ));
    }
    let artifact_id =
        bounded_external_id(artifact.get("artifactId"), "stream Artifact artifactId")?;
    let parts = artifact
        .get("parts")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            A2aError::new(
                "a2a_stream_response_invalid",
                "A2A stream Artifact partsが不正",
            )
        })?;
    if parts.is_empty() || parts.len() > MAX_ARTIFACT_PARTS {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A stream Artifact part件数が上限外",
        ));
    }
    let part_kinds = validate_stream_parts(parts, MAX_ARTIFACT_PARTS)?;
    let append = optional_bool_field(update, "append", false)?;
    let last_chunk = optional_bool_field(update, "lastChunk", false)?;
    if artifact
        .get("metadata")
        .is_some_and(|metadata| !metadata.is_object())
        || artifact
            .get("extensions")
            .is_some_and(|extensions| !extensions.is_array())
        || update
            .get("metadata")
            .is_some_and(|metadata| !metadata.is_object())
    {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Artifact metadataまたはextensionsが不正",
        ));
    }
    Ok((
        json!({
            "kind": "artifactUpdate",
            "task_id_hash": sha256_tagged(task_id.as_bytes()),
            "context_id_hash": sha256_tagged(context_id.as_bytes()),
            "artifact_id_hash": sha256_tagged(artifact_id.as_bytes()),
            "part_count": parts.len(),
            "part_kinds": part_kinds,
            "append": append,
            "last_chunk": last_chunk,
            "event_hash": event_hash,
        }),
        task_id,
        Some(context_id),
    ))
}

fn validate_stream_artifact(value: &Value) -> Result<(), A2aError> {
    let artifact = value.as_object().ok_or_else(|| {
        A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Stream Artifactがobjectではない",
        )
    })?;
    if artifact.keys().any(|key| {
        ![
            "artifactId",
            "name",
            "description",
            "parts",
            "metadata",
            "extensions",
        ]
        .contains(&key.as_str())
    }) {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Stream Artifactに未知fieldがある",
        ));
    }
    bounded_external_id(artifact.get("artifactId"), "Stream Artifact artifactId")?;
    let parts = artifact
        .get("parts")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            A2aError::new(
                "a2a_stream_response_invalid",
                "A2A Stream Artifact partsが不正",
            )
        })?;
    validate_stream_parts(parts, MAX_ARTIFACT_PARTS)?;
    let valid_string = |field: &str, max: usize| {
        artifact
            .get(field)
            .is_none_or(|value| value.as_str().is_some_and(|text| text.len() <= max))
    };
    if !valid_string("name", 1024)
        || !valid_string("description", 2048)
        || artifact
            .get("metadata")
            .is_some_and(|metadata| !metadata.is_object())
        || artifact.get("extensions").is_some_and(|extensions| {
            extensions.as_array().is_none_or(|items| {
                items.len() > 64
                    || items
                        .iter()
                        .any(|item| item.as_str().is_none_or(|text| text.len() > 2048))
            })
        })
    {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Stream Artifact metadataが不正",
        ));
    }
    Ok(())
}

fn optional_bool_field(
    object: &Map<String, Value>,
    field: &str,
    default: bool,
) -> Result<bool, A2aError> {
    match object.get(field) {
        None => Ok(default),
        Some(value) => value.as_bool().ok_or_else(|| {
            A2aError::new(
                "a2a_stream_response_invalid",
                format!("A2A {field}がboolではない"),
            )
        }),
    }
}

fn validate_stream_parts(parts: &[Value], maximum: usize) -> Result<Vec<&'static str>, A2aError> {
    if parts.is_empty() || parts.len() > maximum {
        return Err(A2aError::new(
            "a2a_stream_response_invalid",
            "A2A Stream partsの件数が上限外",
        ));
    }
    let mut kinds = Vec::with_capacity(parts.len());
    for part in parts {
        let part = part.as_object().ok_or_else(|| {
            A2aError::new(
                "a2a_stream_response_invalid",
                "A2A Stream Partがobjectではない",
            )
        })?;
        if part.keys().any(|key| {
            ![
                "text",
                "raw",
                "url",
                "data",
                "metadata",
                "filename",
                "mediaType",
            ]
            .contains(&key.as_str())
        }) {
            return Err(A2aError::new(
                "a2a_stream_response_invalid",
                "A2A Stream Partに未知fieldがある",
            ));
        }
        let variants = ["text", "raw", "url", "data"]
            .into_iter()
            .filter(|key| part.contains_key(*key))
            .collect::<Vec<_>>();
        if variants.len() != 1
            || part
                .get("metadata")
                .is_some_and(|metadata| !metadata.is_object())
            || part
                .get("filename")
                .is_some_and(|value| value.as_str().is_none_or(|text| text.len() > 512))
            || part
                .get("mediaType")
                .is_some_and(|value| value.as_str().is_none_or(|text| text.len() > 256))
        {
            return Err(A2aError::new(
                "a2a_stream_response_invalid",
                "A2A Stream Partのvariantまたはmetadataが不正",
            ));
        }
        let kind = match variants[0] {
            "text" if part["text"].is_string() => "text",
            "raw" if part["raw"].is_string() => "raw",
            "url" if part["url"].is_string() => "url",
            "data" => "data",
            _ => {
                return Err(A2aError::new(
                    "a2a_stream_response_invalid",
                    "A2A Stream Partの型が不正",
                ))
            }
        };
        kinds.push(kind);
    }
    Ok(kinds)
}

fn project_task_metadata(
    task_value: &Value,
    response_hash: &str,
    expected_task_id: Option<&str>,
    require_empty_history: bool,
    include_artifact_manifest: bool,
    artifact_selection: Option<(&str, &str)>,
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
    let (artifact_manifest, selected_artifact) = if include_artifact_manifest {
        project_task_artifacts(task, state, artifact_selection)?
    } else {
        (None, None)
    };
    let mut projection = json!({
        "kind": "task",
        "task_id": task_id,
        "context_id": context_id,
        "state": state,
        "artifact_count": artifact_count,
        "history_count": history_count,
        "result_hash": response_hash,
    });
    if let Some(manifest) = artifact_manifest {
        projection["artifact_manifest"] = manifest;
        projection["selected_artifact"] = selected_artifact.unwrap_or(Value::Null);
    }
    Ok(projection)
}

fn project_task_artifacts(
    task: &Map<String, Value>,
    state: &'static str,
    selection: Option<(&str, &str)>,
) -> Result<(Option<Value>, Option<Value>), A2aError> {
    let artifacts = task
        .get("artifacts")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    if artifacts.len() > MAX_TASK_ARTIFACTS {
        return Err(A2aError::new(
            "a2a_response_invalid",
            "A2A Task Artifact件数が上限を超過",
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut manifest = Vec::with_capacity(artifacts.len());
    let mut selected = None;
    for artifact in artifacts {
        let object = artifact
            .as_object()
            .ok_or_else(|| A2aError::new("a2a_response_invalid", "A2A Artifactがobjectではない"))?;
        if object.keys().any(|key| {
            ![
                "artifactId",
                "name",
                "description",
                "parts",
                "metadata",
                "extensions",
            ]
            .contains(&key.as_str())
        }) {
            return Err(A2aError::new(
                "a2a_response_invalid",
                "A2A Artifactに未知fieldがある",
            ));
        }
        let artifact_id = bounded_external_id(object.get("artifactId"), "Artifact artifactId")?;
        if !ids.insert(artifact_id.clone()) {
            return Err(A2aError::new(
                "a2a_response_invalid",
                "A2A Taskに重複Artifact IDがある",
            ));
        }
        let parts = object
            .get("parts")
            .and_then(Value::as_array)
            .filter(|parts| !parts.is_empty() && parts.len() <= MAX_ARTIFACT_PARTS)
            .ok_or_else(|| {
                A2aError::new(
                    "a2a_response_invalid",
                    "A2A Artifact partsが不正または上限超過",
                )
            })?;
        let serialized = serde_json::to_vec(artifact)
            .map_err(|_| A2aError::new("a2a_response_invalid", "A2A Artifactをhash化できない"))?;
        let artifact_hash = sha256_tagged(&serialized);
        let text_parts = text_artifact_parts(parts);
        let (text_only, text_bytes) = match &text_parts {
            Some(parts) => {
                let bytes = parts.iter().map(|part| part.len()).sum::<usize>();
                if bytes <= MAX_ARTIFACT_TEXT_BYTES {
                    (true, Some(bytes))
                } else {
                    (false, None)
                }
            }
            None => (false, None),
        };
        manifest.push(json!({
            "artifact_id": artifact_id.clone(),
            "artifact_hash": artifact_hash.clone(),
            "part_count": parts.len(),
            "text_only": text_only,
            "text_bytes": text_bytes,
        }));
        if let Some((selected_id, selected_hash)) = selection {
            if artifact_id == selected_id {
                if artifact_hash != selected_hash {
                    return Err(A2aError::new(
                        "a2a_artifact_changed",
                        "A2A Artifact hashが一覧取得時から変化",
                    ));
                }
                if state != "completed" {
                    return Err(A2aError::new(
                        "a2a_artifact_not_ready",
                        "A2A Artifact全文表示はTask完了後に限る",
                    ));
                }
                let text_parts = text_parts
                    .filter(|parts| {
                        parts.iter().map(|part| part.len()).sum::<usize>()
                            <= MAX_ARTIFACT_TEXT_BYTES
                            && parts.iter().all(|part| safe_artifact_text(part))
                    })
                    .ok_or_else(|| {
                        A2aError::new(
                            "a2a_artifact_unsupported",
                            "A2A Artifactは上限内の安全なtext partsだけを表示できる",
                        )
                    })?;
                let content_bytes = text_parts.iter().map(|part| part.len()).sum::<usize>();
                let content_hash =
                    sha256_tagged(&serde_json::to_vec(&text_parts).map_err(|_| {
                        A2aError::new("a2a_response_invalid", "A2A Artifact本文hashを計算できない")
                    })?);
                selected = Some(json!({
                    "artifact_id": artifact_id,
                    "artifact_hash": artifact_hash,
                    "content_hash": content_hash,
                    "byte_length": content_bytes,
                    "parts": text_parts,
                }));
            }
        }
    }
    if let Some((selected_id, _)) = selection {
        if selected.is_none() || !ids.contains(selected_id) {
            return Err(A2aError::new(
                "a2a_artifact_not_found",
                "現在のA2A Taskに選択Artifactがない",
            ));
        }
    }
    Ok((Some(Value::Array(manifest)), selected))
}

fn text_artifact_parts(parts: &[Value]) -> Option<Vec<String>> {
    let mut result = Vec::with_capacity(parts.len());
    for part in parts {
        let object = part.as_object()?;
        if object.get("kind").and_then(Value::as_str) != Some("text")
            || object
                .keys()
                .any(|key| !["kind", "text", "metadata", "extensions"].contains(&key.as_str()))
        {
            return None;
        }
        let text = object.get("text")?.as_str()?;
        if !safe_artifact_text(text) {
            return None;
        }
        result.push(text.to_string());
    }
    Some(result)
}

fn safe_artifact_text(text: &str) -> bool {
    text.chars().all(|character| {
        (!character.is_control() || matches!(character, '\n' | '\r' | '\t'))
            && !matches!(
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
                "artifacts":[{"artifactId":"artifact-1","parts":[{"kind":"text","text":sentinel}],"metadata":{"secret":sentinel}}], "history":[],
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
        assert_eq!(
            projected["artifact_manifest"][0]["artifact_id"],
            "artifact-1"
        );
        assert_eq!(projected["artifact_manifest"][0]["text_only"], true);
        assert_eq!(projected["selected_artifact"], Value::Null);
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
    fn A2A_GetTask選択Artifactは完了textだけhash一致後に全文を返す() {
        let text = "bounded external result";
        let artifact = json!({
            "artifactId":"artifact-exact",
            "name":"ignored untrusted display metadata",
            "parts":[{"kind":"text","text":text}],
            "metadata":{"secret":"must-not-project"}
        });
        let artifact_hash =
            sha256_tagged(&serde_json::to_vec(&artifact).expect("canonical Artifact bytes"));
        let response = json!({
            "jsonrpc":"2.0", "id":"request-artifact-1", "result":{
                "kind":"task", "id":"task-exact", "contextId":"context-1",
                "status":{"state":"TASK_STATE_COMPLETED"},
                "artifacts":[artifact], "history":[],
                "metadata":{"secret":"must-not-project"}
            }
        });
        let raw = serde_json::to_vec(&response).unwrap();
        let metadata = project_get_task_response(&raw, "request-artifact-1", "task-exact")
            .expect("Artifact manifest only");
        assert_eq!(
            metadata["artifact_manifest"][0]["artifact_hash"],
            artifact_hash
        );
        assert_eq!(metadata["selected_artifact"], Value::Null);
        assert!(!metadata.to_string().contains(text));
        assert!(!metadata.to_string().contains("must-not-project"));

        let selected = project_get_task_response_with_artifact(
            &raw,
            "request-artifact-1",
            "task-exact",
            Some(("artifact-exact", artifact_hash.as_str())),
        )
        .expect("explicit full content projection");
        assert_eq!(selected["selected_artifact"]["parts"][0], text);
        assert_eq!(
            selected["selected_artifact"]["content_hash"],
            sha256_tagged(&serde_json::to_vec(&vec![text]).unwrap())
        );
        assert_eq!(selected["selected_artifact"]["byte_length"], text.len());
        assert!(!selected["selected_artifact"]
            .to_string()
            .contains("must-not-project"));

        assert_eq!(
            project_get_task_response_with_artifact(
                &raw,
                "request-artifact-1",
                "task-exact",
                Some((
                    "artifact-exact",
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                )),
            )
            .unwrap_err()
            .code,
            "a2a_artifact_changed"
        );

        let file_artifact = json!({
            "artifactId":"artifact-file",
            "parts":[{"kind":"file","file":{"uri":"file:///secret"}}]
        });
        let file_hash = sha256_tagged(&serde_json::to_vec(&file_artifact).unwrap());
        let mut file_response = response.clone();
        file_response["result"]["artifacts"] = json!([file_artifact]);
        let error = project_get_task_response_with_artifact(
            &serde_json::to_vec(&file_response).unwrap(),
            "request-artifact-1",
            "task-exact",
            Some(("artifact-file", &file_hash)),
        )
        .unwrap_err();
        assert_eq!(error.code, "a2a_artifact_unsupported");
        assert!(!error.message.contains("file:///secret"));

        let mut working = response;
        working["result"]["status"]["state"] = json!("TASK_STATE_WORKING");
        let error = project_get_task_response_with_artifact(
            &serde_json::to_vec(&working).unwrap(),
            "request-artifact-1",
            "task-exact",
            Some(("artifact-exact", &artifact_hash)),
        )
        .unwrap_err();
        assert_eq!(error.code, "a2a_artifact_not_ready");
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
            None,
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

    #[test]
    fn A2A_Streamは終端状態までmetadataだけをbounded投影する() {
        let events = [
            json!({
                "jsonrpc":"2.0", "id":"stream-fixture",
                "result":{"task":{
                    "id":"task-private-id", "contextId":"context-private-id",
                    "status":{"state":"TASK_STATE_WORKING"}, "artifacts":[], "history":[]
                }}
            }),
            json!({
                "jsonrpc":"2.0", "id":"stream-fixture",
                "result":{"artifactUpdate":{
                    "taskId":"task-private-id", "contextId":"context-private-id",
                    "artifact":{"artifactId":"artifact-private-id","parts":[{"text":"private-stream-body"}]},
                    "append":false, "lastChunk":true
                }}
            }),
            json!({
                "jsonrpc":"2.0", "id":"stream-fixture",
                "result":{"statusUpdate":{
                    "taskId":"task-private-id", "contextId":"context-private-id",
                    "status":{"state":"TASK_STATE_COMPLETED"}
                }}
            }),
        ];
        let raw = events
            .iter()
            .map(|event| format!("data: {}\n\n", event))
            .collect::<String>();
        let projected = project_stream_response(raw.as_bytes(), "stream-fixture")
            .expect("bounded terminal stream projection");
        assert_eq!(projected["stream_kind"], "task");
        assert_eq!(projected["state"], "completed");
        assert_eq!(projected["event_count"], 3);
        assert_eq!(projected["events"][1]["kind"], "artifactUpdate");
        assert_eq!(projected["events"][1]["part_kinds"][0], "text");
        assert_eq!(projected["events"][2]["kind"], "statusUpdate");
        for private_value in [
            "private-stream-body",
            "artifact-private-id",
            "task-private-id",
            "context-private-id",
        ] {
            assert!(!projected.to_string().contains(private_value));
        }
    }

    #[test]
    fn A2A_Streamは非終端_request不一致variant超過をunknownに閉じる() {
        let task = json!({
            "jsonrpc":"2.0", "id":"stream-fixture",
            "result":{"task":{"id":"task", "contextId":"context", "status":{"state":"working"}, "artifacts":[], "history":[]}}
        });
        let nonterminal = json!({
            "jsonrpc":"2.0", "id":"stream-fixture",
            "result":{"statusUpdate":{
                "taskId":"task", "contextId":"context", "status":{"state":"working"}
            }}
        });
        let raw = format!("data: {task}\n\ndata: {nonterminal}\n\n");
        assert_eq!(
            project_stream_response(raw.as_bytes(), "stream-fixture")
                .expect_err("nonterminal stream must not become accepted")
                .code,
            "a2a_stream_nonterminal"
        );

        let mismatch = format!(
            "data: {}\n\n",
            json!({
                "jsonrpc":"2.0", "id":"different-request",
                "result":{"statusUpdate":{
                    "taskId":"task", "contextId":"context", "status":{"state":"completed"}
                }}
            })
        );
        assert_eq!(
            project_stream_response(mismatch.as_bytes(), "stream-fixture")
                .expect_err("stale request ID")
                .code,
            "a2a_stream_response_invalid"
        );

        let duplicate_variant = format!(
            "data: {}\n\n",
            json!({
                "jsonrpc":"2.0", "id":"stream-fixture",
                "result":{
                    "message":{"messageId":"m","role":"ROLE_AGENT","parts":[{"text":"private"}]},
                    "task":{"kind":"task","id":"t","status":{"state":"completed"}}
                }
            })
        );
        assert_eq!(
            project_stream_response(duplicate_variant.as_bytes(), "stream-fixture")
                .expect_err("oneof duplicate")
                .code,
            "a2a_stream_response_invalid"
        );

        let too_many = (0..=MAX_A2A_STREAM_EVENTS)
            .map(|_| format!("data: {nonterminal}\n\n"))
            .collect::<String>();
        assert_eq!(
            project_stream_response(too_many.as_bytes(), "stream-fixture")
                .expect_err("event count bound")
                .code,
            "a2a_stream_event_limit"
        );
    }

    #[test]
    fn A2A_Stream_message_onlyは本文を除外して一eventで閉じる() {
        let response = json!({
            "jsonrpc":"2.0", "id":"stream-fixture",
            "result":{"message":{
                "messageId":"private-message-id", "role":"ROLE_AGENT",
                "parts":[{"text":"private-message-body"}]
            }}
        });
        let raw = format!("data: {response}\n\n");
        let projected = project_stream_response(raw.as_bytes(), "stream-fixture")
            .expect("message-only streaming response");
        assert_eq!(projected["stream_kind"], "message");
        assert!(projected["state"].is_null());
        assert_eq!(projected["event_count"], 1);
        for secret in ["private-message-id", "private-message-body"] {
            assert!(!projected.to_string().contains(secret));
        }
        let extra = format!("{raw}data: {response}\n\n");
        assert_eq!(
            project_stream_response(extra.as_bytes(), "stream-fixture")
                .expect_err("message-only stream must contain one event")
                .code,
            "a2a_stream_response_invalid"
        );
    }

    #[test]
    fn loopback_A2A_Streamは一回SendStreamingMessageを実行しmetadataだけを返す() {
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").expect("stream loopback listener");
        let address = listener.local_addr().expect("stream listener address");
        let server = thread::spawn(move || -> Result<Value, String> {
            let (mut stream, _) = listener.accept().map_err(|error| error.to_string())?;
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            let mut request = Vec::new();
            let mut chunk = [0u8; 2048];
            let header_end = loop {
                let size = stream.read(&mut chunk).map_err(|error| error.to_string())?;
                if size == 0
                    || request.len().saturating_add(size) > MAX_JSON_RPC_BYTES + MAX_HEADER_BYTES
                {
                    return Err("A2A Stream request header/body truncated or oversized".into());
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
                    .any(|line| line.eq_ignore_ascii_case("accept: text/event-stream"))
                || !header
                    .lines()
                    .any(|line| line.eq_ignore_ascii_case("A2A-Version: 1.0"))
                || !header
                    .lines()
                    .any(|line| line.eq_ignore_ascii_case("Content-Type: application/json"))
            {
                return Err("A2A Stream request method or protocol headers mismatch".into());
            }
            let content_length = header
                .lines()
                .find_map(|line| {
                    line.split_once(':').and_then(|(name, value)| {
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                })
                .ok_or_else(|| "A2A Stream Content-Length missing".to_string())?;
            while request.len() < header_end + content_length {
                let size = stream.read(&mut chunk).map_err(|error| error.to_string())?;
                if size == 0 {
                    return Err("A2A Stream request body truncated".into());
                }
                request.extend_from_slice(&chunk[..size]);
            }
            let body: Value =
                serde_json::from_slice(&request[header_end..header_end + content_length])
                    .map_err(|error| error.to_string())?;
            let message = &body["params"]["message"];
            let part = &message["parts"][0];
            if body["method"] != "SendStreamingMessage"
                || body["jsonrpc"] != "2.0"
                || message.get("kind").is_some()
                || part.get("kind").is_some()
                || body["params"]["message"]["parts"][0]["text"] != "stream-fixture-message"
            {
                return Err("A2A SendStreamingMessage request mismatch".into());
            }
            let initial_data = json!({
                "jsonrpc":"2.0", "id":body["id"],
                "result":{"task":{"id":"fixture-private-task", "contextId":"fixture-private-context",
                    "status":{"state":"TASK_STATE_WORKING"}, "artifacts":[], "history":[]}}
            })
            .to_string();
            let terminal_data = json!({
                "jsonrpc":"2.0", "id":body["id"],
                "result":{"statusUpdate":{
                    "taskId":"fixture-private-task", "contextId":"fixture-private-context",
                    "status":{"state":"TASK_STATE_COMPLETED"}
                }}
            })
            .to_string();
            let response_body = format!("data: {initial_data}\n\ndata: {terminal_data}\n\n");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                response_body.len()
            );
            stream
                .write_all(response.as_bytes())
                .map_err(|error| error.to_string())?;
            stream
                .write_all(response_body.as_bytes())
                .map_err(|error| error.to_string())?;
            stream.flush().map_err(|error| error.to_string())?;
            Ok(body)
        });
        let projected = send_stream(
            &format!("http://{address}/rpc"),
            "stream-fixture-message",
            Instant::now() + Duration::from_secs(15),
        )
        .expect("loopback A2A Stream response");
        let request = server
            .join()
            .expect("A2A Stream server thread")
            .expect("valid A2A Stream request");
        assert_eq!(request["params"]["message"]["role"], "ROLE_USER");
        assert_eq!(projected["stream_kind"], "task");
        assert_eq!(projected["state"], "completed");
        assert_eq!(projected["event_count"], 2);
        assert!(!projected.to_string().contains("fixture-private-task"));
    }
}
