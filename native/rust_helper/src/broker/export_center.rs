//! Windows向けGUI Shell書出しの独立manifestを生成するBroker境界。
//!
//! 現行単位は新規identity、監査store、設定、Runtime／Adapter manifestと配布
//! metadataの生成までであり、実build、installer、process、filesystem書込み、
//! Credential／Permission／Approval／Audit chain継承は行わない。
#![allow(non_snake_case)]

use super::compose_center;
use super::dialogue::識別子生成;
use super::protocol::{
    Broker, BrokerResponse, BrokerStatus, EVIDENCE_SOURCE_INTERNAL_STATE,
};
use serde::Deserialize;
use serde_json::{json, Value};

const VERSION: u64 = 1;
const OPERATION: &str = "GUI Shell書出し";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportRequest {
    version: u64,
    export_id: String,
    compose_manifest: Value,
    target_platform: String,
    export_mode: String,
    distribution_channel: String,
}

pub(super) fn export(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    owner: bool,
    payload_hash: &str,
) -> BrokerResponse {
    if !owner {
        return broker.reject_with_payload_hash(
            request_id,
            OPERATION,
            "owner_required",
            "GUI Shell書出しはowner制御資格が必要",
            true,
            payload_hash,
        );
    }
    let request = match parse_request(payload) {
        Ok(request) => request,
        Err(reason) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_export_invalid",
                &reason,
                true,
                payload_hash,
            )
        }
    };
    let manifest = match compose_center::parse_manifest(&request.compose_manifest) {
        Ok(manifest) => manifest,
        Err(reason) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_export_manifest_invalid",
                &reason,
                true,
                payload_hash,
            )
        }
    };
    let manifest_value = match serde_json::to_value(manifest) {
        Ok(value) => value,
        Err(_) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_export_serialize_failed",
                "書出し元Manifestを正規化できない",
                true,
                payload_hash,
            )
        }
    };
    let app_id = match 識別子生成() {
        Ok(id) => format!("d4-pocket-app-{id}"),
        Err(_) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_export_identity_failed",
                "新規App identityを生成できない",
                true,
                payload_hash,
            )
        }
    };
    let audit_store_id = match 識別子生成() {
        Ok(id) => format!("audit-store-{id}"),
        Err(_) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_export_audit_store_failed",
                "新規監査store identityを生成できない",
                true,
                payload_hash,
            )
        }
    };
    let audit = match broker.append_audit(
        request_id,
        OPERATION,
        "accepted",
        "Windows向け独立manifestを生成。Credential、Permission、Approval、Audit chainは継承せず、buildとinstallerは開始しない",
        EVIDENCE_SOURCE_INTERNAL_STATE,
        payload_hash,
    ) {
        Ok(event) => event,
        Err(error) => {
            return broker.audit_store_failed_response(
                request_id,
                OPERATION,
                "broker_audit_append_failed",
                &error.message(),
            )
        }
    };
    let display_name = manifest_value["display_name"].clone();
    let settings = manifest_value["settings"].clone();
    let runtime_ids = manifest_value["runtime_ids"].clone();
    let agent_ids = manifest_value["agent_ids"].clone();
    let tool_ids = manifest_value["tool_ids"].clone();
    let mcp_connection_ids = manifest_value["mcp_connection_ids"].clone();
    let capability_requirements = manifest_value["capability_requirements"].clone();
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OPERATION.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
        audit_event_id: audit.event_id.clone(),
        error: None,
        health: None,
        body: Some(json!({
            "version": VERSION,
            "operation": OPERATION,
            "status": "accepted",
            "export_id": request.export_id,
            "export_manifest": {
                "app_identity": {"app_id": app_id, "display_name": display_name, "target_platform": "windows"},
                "audit_store": {"store_id": audit_store_id, "chain_status": "new", "inherited": false},
                "settings": settings,
                "runtime_manifest": {"runtime_ids": runtime_ids},
                "adapter_configuration": {"agent_ids": agent_ids, "tool_ids": tool_ids, "mcp_connection_ids": mcp_connection_ids},
                "capability_requirements": capability_requirements,
                "distribution_metadata": {"target_platform": "windows", "artifact_status": "not_built", "installer_status": "not_started", "signed": false, "channel": request.distribution_channel},
                "inheritance_policy": {"authority": "none", "permission": "none", "approval": "none", "credential": "none", "audit_chain": "none"}
            },
            "build_status": "not_started",
            "artifact_status": "not_built",
            "credential_inherited": false,
            "permission_inherited": false,
            "approval_inherited": false,
            "audit_chain_inherited": false,
            "authority_strip": true,
            "evidence_source": EVIDENCE_SOURCE_INTERNAL_STATE,
            "audit_id": audit.event_id,
        })),
        shutdown_requested: broker.shutdown_requested,
    }
}

fn parse_request(value: &Value) -> Result<ExportRequest, String> {
    let request: ExportRequest = serde_json::from_value(value.clone())
        .map_err(|_| "GUI Shell書出しの構造が不正または禁止fieldがある".to_string())?;
    if request.version != VERSION
        || !valid_identifier(&request.export_id)
        || request.target_platform != "windows"
        || request.export_mode != "manifest_only"
        || !["local", "installer_candidate"].contains(&request.distribution_channel.as_str())
    {
        return Err("書出し版、識別子、対象platform、出力mode、配布channelが不正".to_string());
    }
    Ok(request)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= 64
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::protocol::{Broker, BrokerOperation, BrokerRequestEnvelope, BrokerStatus};

    fn manifest() -> Value {
        json!({
            "version": 1, "compose_id": "d4-pocket-local", "display_name": "D4 Pocket ローカル構成",
            "runtime_ids": ["gui_shell_rust_broker"], "agent_ids": ["codex"], "tool_ids": [], "mcp_connection_ids": [],
            "theme": {"theme_id": "d4-pocket", "mode": "system"}, "capability_requirements": ["runtime.read"],
            "settings": {"locale": "ja-JP", "density": "comfortable", "content_visibility": "summary"},
            "inheritance_policy": {"authority": "none", "permission": "none", "approval": "none", "credential": "none", "audit_chain": "none"},
            "output_mode": "manifest_only"
        })
    }

    fn payload() -> Value {
        json!({"version": 1, "export_id": "export-test", "compose_manifest": manifest(), "target_platform": "windows", "export_mode": "manifest_only", "distribution_channel": "local"})
    }

    fn owner_request(broker: &mut Broker, payload: Value) -> String {
        let nonce = format!("owner-export-nonce-{}", broker.audit_events().len());
        let payload_hash = crate::broker::protocol::canonical_payload_hash(Some(&payload));
        json!({"request_id": "export-owner-test", "session_id": "session-1", "operation": OPERATION, "payload": payload, "payload_hash": payload_hash, "nonce": nonce, "issued_at": BrokerRequestEnvelope::current_issued_at(), "metadata": {}}).to_string()
    }

    #[test]
    fn 書出しは新規identityと監査storeを生成するが権限を継承しない() {
        let mut broker = Broker::new("session-1");
        let raw = owner_request(&mut broker, payload());
        let response = broker.owner要求処理(&raw);
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.unwrap();
        assert!(body["export_manifest"]["app_identity"]["app_id"].as_str().unwrap().starts_with("d4-pocket-app-"));
        assert_eq!(body["export_manifest"]["audit_store"]["inherited"], false);
        assert_eq!(body["build_status"], "not_started");
        assert_eq!(body["credential_inherited"], false);
        assert_eq!(body["permission_inherited"], false);
        assert_eq!(body["approval_inherited"], false);
        assert_eq!(body["audit_chain_inherited"], false);
    }

    #[test]
    fn 書出しは通常資格と継承要求を拒否する() {
        let mut broker = Broker::new("session-1");
        let mut normal = BrokerRequestEnvelope::command_envelope_at("export-normal", "session-1", "export-normal-nonce", &BrokerRequestEnvelope::current_issued_at());
        normal.operation = Some(BrokerOperation::GuiShell書出し);
        normal.payload = Some(payload());
        normal.refresh_payload_hash();
        assert_eq!(broker.handle(normal).status, BrokerStatus::Rejected);

        let mut invalid = payload();
        invalid["compose_manifest"]["inheritance_policy"]["credential"] = Value::from("credential.value");
        let raw = owner_request(&mut broker, invalid);
        assert_eq!(broker.owner要求処理(&raw).status, BrokerStatus::Rejected);
    }
}
