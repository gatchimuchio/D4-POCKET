//! A2A Agent Cardのowner制御取得とmetadata-only射影。
//!
//! C16ではAgent Cardの取得・検証だけを行う。認証credentialの注入、Task送信、
//! Message送信、Artifact本文、Stream購読はBroker接続センターへ持ち込まない。
#![allow(non_snake_case)]

use crate::audit_hash::sha256_tagged;
use serde_json::{json, Map, Value};
use std::io::{Read, Write};
use std::net::{IpAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

const MAX_URI_BYTES: usize = 2048;
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_STRING_CHARS: usize = 1024;

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
    validate_credential_ref(credential_ref)?;
    let raw = http_get(&endpoint)?;
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
    if scheme == "https" {
        return Err(A2aError::new(
            "a2a_https_unavailable",
            "C16の現行Rust helperはhttps Agent Card取得を未接続とする",
        ));
    }
    if remainder.contains('@') || remainder.contains('#') {
        return Err(A2aError::new(
            "a2a_endpoint_invalid",
            "A2A Agent Card URIへuserinfoまたはfragmentを含められない",
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
        (authority.to_string(), 80)
    };
    if port == 0 || host.is_empty() || host.len() > 255 || host.contains(':') || path.len() > 1024 {
        return Err(A2aError::new(
            "a2a_endpoint_invalid",
            "A2A Agent Card URIのhost、port、pathが不正",
        ));
    }
    let ip = host.parse::<IpAddr>().map_err(|_| {
        A2aError::new(
            "a2a_http_non_loopback",
            "httpのA2A接続はloopback IPへ限定する",
        )
    })?;
    if !ip.is_loopback() {
        return Err(A2aError::new(
            "a2a_http_non_loopback",
            "httpのA2A接続はloopbackへ限定する",
        ));
    }
    Ok(Endpoint {
        scheme,
        host,
        port,
        authority: authority.to_string(),
        path,
    })
}

fn validate_credential_ref(value: &Value) -> Result<(), A2aError> {
    let object = value.as_object().ok_or_else(|| {
        A2aError::new(
            "a2a_credential_ref_invalid",
            "A2A Credential refがobjectではない",
        )
    })?;
    if object
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        || object.get("status").and_then(Value::as_str) != Some("missing")
    {
        return Err(A2aError::new(
            "a2a_credential_unavailable",
            "C16ではA2A credential実値注入を行わない",
        ));
    }
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
    Ok(())
}

fn http_get(endpoint: &Endpoint) -> Result<Vec<u8>, A2aError> {
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
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\nConnection: close\r\nUser-Agent: D4-Pocket-A2A/1\r\n\r\n",
        endpoint.path, endpoint.authority
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|_| A2aError::new("a2a_connection_failed", "A2A Agent Card要求を送信できない"))?;
    stream
        .flush()
        .map_err(|_| A2aError::new("a2a_connection_failed", "A2A Agent Card要求をflushできない"))?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    let (body_start, body_length) = loop {
        if Instant::now() >= deadline {
            return Err(A2aError::new(
                "a2a_timeout",
                "A2A Agent Card応答が期限を超過",
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
            Err(_) => {
                return Err(A2aError::new(
                    "a2a_connection_failed",
                    "A2A Agent Card応答を読めない",
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
            for line in lines {
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
                "A2A Agent Card bodyが期限を超過",
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
            Err(_) => {
                return Err(A2aError::new(
                    "a2a_connection_failed",
                    "A2A Agent Card bodyを読めない",
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
        projected_interfaces.push(json!({
            "binding": binding, "transport": endpoint.scheme, "status": "supported",
            "reason": "A2A Agent Cardのinterface宣言を確認"
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
    let schemes = match object.get("securitySchemes").and_then(Value::as_object) {
        Some(value) => {
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
        None => Vec::new(),
    };
    let streams = if streaming {
        vec![json!({
            "stream_id": format!("{agent_id}-stream"), "task_id": format!("{agent_id}-card"), "mode": "sse",
            "status": {"status": "supported", "reason": "Agent Cardがstreamingを宣言。実購読は未開始"},
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
            "skills": skills, "authentication": {"schemes": schemes, "credential_ref": credential_ref, "secret_value_present": false},
            "origin": "live_runtime", "status": "discovered"
        },
        "Task": [], "Message": [], "Artifact": [], "Stream": streams,
        "Trust": {"state": "pending_review", "evidence_source": "LIVE_RUNTIME", "reason": "Agent Cardは宣言情報であり、owner review前のTrustを生成しない", "requires_operator_review": true},
        "Capability diff": {"status": "not_evaluated", "added": [], "removed": [], "changed": [], "requires_operator_review": true, "evidence_source": "LIVE_RUNTIME"},
        "権限生成": "なし", "authority_strip": true, "公開範囲": "metadata_only", "証拠種別": "LIVE_RUNTIME"
    }))
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
        json!({"name": "外部Agent例", "description": "外部Agentの宣言情報", "version": "1.0.0", "supportedInterfaces": [{"protocolBinding": "JSONRPC", "protocolVersion": "1.0"}], "capabilities": {"streaming": true, "pushNotifications": false, "stateTransitionHistory": true, "extendedAgentCard": false}, "securitySchemes": {"oauth2": {}}, "skills": [{"id": "summarize", "name": "要約", "description": "要約", "tags": ["summary"], "inputModes": ["text/plain"], "outputModes": ["text/plain"]}]})
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
            projected["Agent Card"]["authentication"]["secret_value_present"],
            false
        );
    }
    #[test]
    fn httpsと非loopbackとuserinfoを現行境界で拒否する() {
        assert_eq!(
            parse_endpoint("https://example.com/card").unwrap_err().code,
            "a2a_https_unavailable"
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
    }

    #[test]
    fn loopback_HTTPからAgent_Cardを取得してmetadata_onlyへ射影する() {
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback待受");
        let address = listener.local_addr().expect("接続先");
        let response_body = serde_json::to_vec(&card()).expect("Agent CardのJSON");
        let response_length = response_body.len();
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("Agent Card要求");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("要求読取期限");
            let mut request = Vec::new();
            let mut chunk = [0u8; 512];
            while !request.windows(4).any(|value| value == b"\r\n\r\n") {
                let size = stream.read(&mut chunk).expect("要求読取");
                assert!(size > 0, "header前に要求が終了");
                request.extend_from_slice(&chunk[..size]);
                assert!(request.len() < 8192, "要求header上限");
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
        let uri = format!("http://{address}/.well-known/agent-card.json");
        let projected = fetch_agent_card(&uri, "remote-agent-example", "1.0", &credential());
        let worker_result = worker.join();
        worker_result.expect("Agent Card試験worker");
        let projected = projected.expect("loopback Agent Card射影");
        assert_eq!(projected["証拠種別"], "LIVE_RUNTIME");
        assert_eq!(projected["公開範囲"], "metadata_only");
        assert_eq!(projected["Agent Card"]["origin"], "live_runtime");
        assert!(projected["Agent Card"].get("endpoint").is_none());
    }

    #[test]
    fn credential実値をAgent_Card取得へ持ち込めない() {
        let mut credential = credential();
        credential["required"] = Value::Bool(true);
        let error = validate_credential_ref(&credential).expect_err("credential required拒否");
        assert_eq!(error.code, "a2a_credential_unavailable");
    }
}
