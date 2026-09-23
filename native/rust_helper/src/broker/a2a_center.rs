//! C16 A2A接続センターのBroker経路。
//!
//! Agent Cardをowner controlから取得し、metadata-onlyの接続receiptへ射影する。
//! A2AのTask送信、Message送信、Artifact本文、Stream購読、Credential注入は未接続とする。
#![allow(non_snake_case)]

use super::protocol::{Broker, BrokerResponse, BrokerStatus, EVIDENCE_SOURCE_INTERNAL_STATE};
use crate::a2a::{fetch_agent_card, A2aError};
use serde::Deserialize;
use serde_json::{json, Value};

const OP_CONNECT: &str = "A2A接続";
const OP_LIST: &str = "A2A接続一覧";
const VERSION: u64 = 1;
const PROTOCOL_VERSION: &str = "1.0";
const MAX_CONNECTIONS: usize = 64;

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
        .insert(request.agent_id, receipt.clone());
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
        use std::net::{Shutdown, TcpListener};
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
            stream.write_all(header.as_bytes()).expect("応答header");
            stream.write_all(&response_body).expect("応答body");
            stream.flush().expect("応答flush");
            stream.shutdown(Shutdown::Write).expect("応答終了");
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
        let _ = std::fs::remove_dir_all(store);
    }
}
