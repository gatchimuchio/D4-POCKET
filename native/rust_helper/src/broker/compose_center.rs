//! D4 PocketのGUI Shell構成Manifestを生成するBroker経路。
//!
//! この段階はManifest-onlyであり、build、独立App identity、Credential、
//! Permission、Approval、Audit chainの継承を行わない。構成入力はBrokerで
//! Schema相当の構造と境界を再検証し、受理結果だけを監査へ記録する。
#![allow(non_snake_case)]

use super::protocol::{
    Broker, BrokerResponse, BrokerStatus, EVIDENCE_SOURCE_INTERNAL_STATE,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const VERSION: u64 = 1;
const OP_COMPOSE: &str = "GUI Shell構成";
const OP_PREVIEW: &str = "GUI Shell構成Preview";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Theme {
    theme_id: String,
    mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Settings {
    locale: String,
    density: String,
    content_visibility: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InheritancePolicy {
    authority: String,
    permission: String,
    approval: String,
    credential: String,
    audit_chain: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ComposeManifest {
    version: u64,
    compose_id: String,
    display_name: String,
    runtime_ids: Vec<String>,
    agent_ids: Vec<String>,
    tool_ids: Vec<String>,
    mcp_connection_ids: Vec<String>,
    theme: Theme,
    capability_requirements: Vec<String>,
    settings: Settings,
    inheritance_policy: InheritancePolicy,
    output_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreviewRequest {
    version: u64,
    current_manifest: Value,
    candidate_manifest: ComposeManifest,
    build_target: String,
    preview_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreviewInput {
    current_manifest: Option<ComposeManifest>,
    candidate_manifest: ComposeManifest,
    build_target: String,
}

pub(super) fn compose(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    _owner: bool,
    payload_hash: &str,
) -> BrokerResponse {
    let manifest = match parse_manifest(payload) {
        Ok(manifest) => manifest,
        Err(reason) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_COMPOSE,
                "gui_shell_compose_invalid",
                &reason,
                true,
                payload_hash,
            )
        }
    };

    let manifest_value = match serde_json::to_value(&manifest) {
        Ok(value) => value,
        Err(_) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_COMPOSE,
                "gui_shell_compose_serialize_failed",
                "構成Manifestを正規化できない",
                true,
                payload_hash,
            )
        }
    };
    let audit = match broker.append_audit(
        request_id,
        OP_COMPOSE,
        "accepted",
        "構成Manifestだけを生成。build、App identity、Credential、Permission、Approval、Audit chainを継承しない",
        EVIDENCE_SOURCE_INTERNAL_STATE,
        payload_hash,
    ) {
        Ok(event) => event,
        Err(error) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_COMPOSE,
                "broker_audit_append_failed",
                &error.message(),
            )
        }
    };

    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OP_COMPOSE.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
        audit_event_id: audit.event_id.clone(),
        error: None,
        health: None,
        body: Some(json!({
            "version": VERSION,
            "operation": OP_COMPOSE,
            "status": "accepted",
            "compose_manifest": manifest_value,
            "build_status": "not_started",
            "app_identity_status": "not_generated",
            "permission_generated": false,
            "authority_strip": true,
            "inheritance_policy": {
                "authority": "none",
                "permission": "none",
                "approval": "none",
                "credential": "none",
                "audit_chain": "none"
            },
            "evidence_source": EVIDENCE_SOURCE_INTERNAL_STATE,
            "audit_id": audit.event_id,
        })),
        shutdown_requested: broker.shutdown_requested,
    }
}

pub(super) fn preview(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    _owner: bool,
    payload_hash: &str,
) -> BrokerResponse {
    let input = match parse_preview(payload) {
        Ok(input) => input,
        Err(reason) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_PREVIEW,
                "gui_shell_preview_invalid",
                &reason,
                true,
                payload_hash,
            )
        }
    };
    let candidate_value = match serde_json::to_value(&input.candidate_manifest) {
        Ok(value) => value,
        Err(_) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_PREVIEW,
                "gui_shell_preview_serialize_failed",
                "Preview対象Manifestを正規化できない",
                true,
                payload_hash,
            )
        }
    };
    let current_value = input
        .current_manifest
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .ok()
        .flatten()
        .unwrap_or(Value::Null);
    let audit = match broker.append_audit(
        request_id,
        OP_PREVIEW,
        "accepted",
        "構成、機能要件、差分、版rollbackをPreview。build、Export、rollback、権限生成は実行しない",
        EVIDENCE_SOURCE_INTERNAL_STATE,
        payload_hash,
    ) {
        Ok(event) => event,
        Err(error) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_PREVIEW,
                "broker_audit_append_failed",
                &error.message(),
            )
        }
    };
    let permission_requirements: Vec<Value> = input
        .candidate_manifest
        .capability_requirements
        .iter()
        .map(|capability| {
            json!({
                "capability": capability,
                "permission_status": "not_generated",
                "approval_status": "not_requested"
            })
        })
        .collect();
    let diff = manifest_diff(
        input.current_manifest.as_ref(),
        &input.candidate_manifest,
    );
    let current_version = input.current_manifest.as_ref().map(|manifest| manifest.version);
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OP_PREVIEW.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
        audit_event_id: audit.event_id.clone(),
        error: None,
        health: None,
        body: Some(json!({
            "version": VERSION,
            "operation": OP_PREVIEW,
            "status": "accepted",
            "current_manifest": current_value,
            "candidate_manifest": candidate_value,
            "build_target": input.build_target,
            "preview_mode": "version_rollback",
            "diff": diff,
            "permission_requirements": permission_requirements,
            "version_preview": {
                "current_version": current_version,
                "candidate_version": VERSION,
                "rollback_available": false,
                "rollback_status": "not_executable",
                "rollback_reason": "Previewはrollback要求を実行しない"
            },
            "build_status": "not_started",
            "export_status": "not_started",
            "authority_strip": true,
            "inheritance_policy": {
                "authority": "none",
                "permission": "none",
                "approval": "none",
                "credential": "none",
                "audit_chain": "none"
            },
            "evidence_source": EVIDENCE_SOURCE_INTERNAL_STATE,
            "audit_id": audit.event_id,
        })),
        shutdown_requested: broker.shutdown_requested,
    }
}

pub(super) fn parse_manifest(value: &Value) -> Result<ComposeManifest, String> {
    let manifest: ComposeManifest = serde_json::from_value(value.clone())
        .map_err(|_| "GUI Shell構成の構造が不正または禁止fieldがある".to_string())?;
    validate_manifest(manifest)
}

fn parse_preview(value: &Value) -> Result<PreviewInput, String> {
    let request: PreviewRequest = serde_json::from_value(value.clone())
        .map_err(|_| "GUI Shell構成Previewの構造が不正または禁止fieldがある".to_string())?;
    if request.version != VERSION {
        return Err("GUI Shell構成Previewの版が不正".to_string());
    }
    if request.preview_mode != "version_rollback" {
        return Err("Preview modeはversion_rollbackだけを受け付ける".to_string());
    }
    if !["windows", "desktop", "mobile"].contains(&request.build_target.as_str()) {
        return Err("Preview対象platformが不正".to_string());
    }
    let current_manifest = if request.current_manifest.is_null() {
        None
    } else {
        Some(validate_manifest(
            serde_json::from_value(request.current_manifest)
                .map_err(|_| "現在の構成Manifestが不正".to_string())?,
        )?)
    };
    Ok(PreviewInput {
        current_manifest,
        candidate_manifest: validate_manifest(request.candidate_manifest)?,
        build_target: request.build_target,
    })
}

fn manifest_diff(base: Option<&ComposeManifest>, candidate: &ComposeManifest) -> Value {
    let (runtime_added, runtime_removed) = list_delta(
        base.map(|manifest| manifest.runtime_ids.as_slice()),
        candidate.runtime_ids.as_slice(),
    );
    let (agent_added, agent_removed) = list_delta(
        base.map(|manifest| manifest.agent_ids.as_slice()),
        candidate.agent_ids.as_slice(),
    );
    let (tool_added, tool_removed) = list_delta(
        base.map(|manifest| manifest.tool_ids.as_slice()),
        candidate.tool_ids.as_slice(),
    );
    let (mcp_added, mcp_removed) = list_delta(
        base.map(|manifest| manifest.mcp_connection_ids.as_slice()),
        candidate.mcp_connection_ids.as_slice(),
    );
    let (capability_added, capability_removed) = list_delta(
        base.map(|manifest| manifest.capability_requirements.as_slice()),
        candidate.capability_requirements.as_slice(),
    );
    let mut changed_fields = Vec::new();
    match base {
        None => changed_fields.push("initial_configuration".to_string()),
        Some(base) => {
            if base.compose_id != candidate.compose_id {
                changed_fields.push("compose_id".to_string());
            }
            if base.display_name != candidate.display_name {
                changed_fields.push("display_name".to_string());
            }
            if base.theme != candidate.theme {
                changed_fields.push("theme".to_string());
            }
            if base.settings != candidate.settings {
                changed_fields.push("settings".to_string());
            }
        }
    }
    json!({
        "runtime_added": runtime_added,
        "runtime_removed": runtime_removed,
        "agent_added": agent_added,
        "agent_removed": agent_removed,
        "tool_added": tool_added,
        "tool_removed": tool_removed,
        "mcp_added": mcp_added,
        "mcp_removed": mcp_removed,
        "capability_added": capability_added,
        "capability_removed": capability_removed,
        "changed_fields": changed_fields,
    })
}

fn list_delta(base: Option<&[String]>, candidate: &[String]) -> (Vec<String>, Vec<String>) {
    let old: BTreeSet<&String> = base
        .unwrap_or(&[])
        .iter()
        .collect();
    let new: BTreeSet<&String> = candidate.iter().collect();
    (
        new.difference(&old).map(|value| (*value).clone()).collect(),
        old.difference(&new).map(|value| (*value).clone()).collect(),
    )
}

fn validate_manifest(manifest: ComposeManifest) -> Result<ComposeManifest, String> {
    if manifest.version != VERSION
        || !valid_identifier(&manifest.compose_id)
        || manifest.display_name.trim().is_empty()
        || manifest.display_name.chars().count() > 128
    {
        return Err("構成ID、版、表示名が不正".to_string());
    }
    for (label, values, limit) in [
        ("実行基盤", &manifest.runtime_ids, 32usize),
        ("エージェント", &manifest.agent_ids, 32usize),
        ("ツール", &manifest.tool_ids, 64usize),
        ("MCP接続", &manifest.mcp_connection_ids, 32usize),
        ("機能要件", &manifest.capability_requirements, 64usize),
    ] {
        if values.len() > limit
            || values.iter().any(|value| {
                value.trim().is_empty() || value.chars().count() > 128
            })
            || values.iter().collect::<BTreeSet<_>>().len() != values.len()
        {
            return Err(format!("{label}選択が不正または重複している"));
        }
    }
    if manifest.theme.theme_id.trim().is_empty()
        || manifest.theme.theme_id.chars().count() > 64
        || !["system", "light", "dark"].contains(&manifest.theme.mode.as_str())
    {
        return Err("Theme選択が不正".to_string());
    }
    if !valid_locale(&manifest.settings.locale)
        || !["compact", "comfortable"].contains(&manifest.settings.density.as_str())
        || !["none", "hash_only", "summary", "redacted", "full"]
            .contains(&manifest.settings.content_visibility.as_str())
    {
        return Err("Settings選択が不正".to_string());
    }
    let policy = &manifest.inheritance_policy;
    if [
        &policy.authority,
        &policy.permission,
        &policy.approval,
        &policy.credential,
        &policy.audit_chain,
    ]
    .iter()
    .any(|value| value.as_str() != "none")
    {
        return Err("Credential、Permission、Approval、Authority、Audit chainの継承を拒否".to_string());
    }
    if manifest.output_mode != "manifest_only" {
        return Err("構成出力はmanifest_onlyだけを受け付ける".to_string());
    }
    Ok(manifest)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= 64
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
}

fn valid_locale(value: &str) -> bool {
    let mut parts = value.split('-');
    let language = parts.next().unwrap_or_default();
    let region = parts.next().unwrap_or_default();
    parts.next().is_none()
        && language.len() == 2
        && region.len() == 2
        && language.chars().all(|character| character.is_ascii_alphabetic())
        && region.chars().all(|character| character.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::protocol::{Broker, BrokerOperation, BrokerRequestEnvelope, BrokerStatus};

    fn manifest() -> Value {
        json!({
            "version": 1,
            "compose_id": "d4-pocket-local",
            "display_name": "D4 Pocket ローカル構成",
            "runtime_ids": ["gui_shell_rust_broker"],
            "agent_ids": ["codex"],
            "tool_ids": [],
            "mcp_connection_ids": [],
            "theme": {"theme_id": "d4-pocket", "mode": "system"},
            "capability_requirements": ["runtime.read", "agent.metadata"],
            "settings": {"locale": "ja-JP", "density": "comfortable", "content_visibility": "summary"},
            "inheritance_policy": {
                "authority": "none", "permission": "none", "approval": "none",
                "credential": "none", "audit_chain": "none"
            },
            "output_mode": "manifest_only"
        })
    }

    fn call(broker: &mut Broker, payload: Value) -> super::super::protocol::BrokerResponse {
        let mut request = BrokerRequestEnvelope::command_envelope_at(
            "compose-test",
            "session-1",
            &format!("nonce-{}", broker.audit_events().len()),
            &BrokerRequestEnvelope::current_issued_at(),
        );
        request.operation = Some(BrokerOperation::GuiShell構成);
        request.payload = Some(payload);
        request.refresh_payload_hash();
        broker.handle(request)
    }

    fn preview_payload(current: Value, candidate: Value) -> Value {
        json!({
            "version": 1,
            "current_manifest": current,
            "candidate_manifest": candidate,
            "build_target": "windows",
            "preview_mode": "version_rollback"
        })
    }

    fn call_preview(broker: &mut Broker, payload: Value) -> super::super::protocol::BrokerResponse {
        let mut request = BrokerRequestEnvelope::command_envelope_at(
            "preview-test",
            "session-1",
            &format!("preview-nonce-{}", broker.audit_events().len()),
            &BrokerRequestEnvelope::current_issued_at(),
        );
        request.operation = Some(BrokerOperation::GuiShell構成Preview);
        request.payload = Some(payload);
        request.refresh_payload_hash();
        broker.handle(request)
    }

    #[test]
    fn 構成はmanifestだけを生成し権限を継承しない() {
        let mut broker = Broker::new("session-1");
        let response = call(&mut broker, manifest());
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.unwrap();
        assert_eq!(body["build_status"], "not_started");
        assert_eq!(body["app_identity_status"], "not_generated");
        assert_eq!(body["permission_generated"], false);
        assert_eq!(body["authority_strip"], true);
        assert_eq!(body["inheritance_policy"]["credential"], "none");
        assert_eq!(body["compose_manifest"]["output_mode"], "manifest_only");
    }

    #[test]
    fn 構成は権限継承と未知fieldを拒否する() {
        let mut broker = Broker::new("session-1");
        let mut authority = manifest();
        authority["inheritance_policy"]["permission"] = Value::from("permission.read");
        assert_eq!(call(&mut broker, authority).status, BrokerStatus::Rejected);

        let mut unknown = manifest();
        unknown["credential"] = Value::from("secret");
        assert_eq!(call(&mut broker, unknown).status, BrokerStatus::Rejected);
    }

    #[test]
    fn 構成のlocaleと重複選択を検査する() {
        let mut broker = Broker::new("session-1");
        let mut invalid_locale = manifest();
        invalid_locale["settings"]["locale"] = Value::from("ja");
        assert_eq!(call(&mut broker, invalid_locale).status, BrokerStatus::Rejected);

        let mut duplicate = manifest();
        duplicate["agent_ids"] = json!(["codex", "codex"]);
        assert_eq!(call(&mut broker, duplicate).status, BrokerStatus::Rejected);
    }

    #[test]
    fn 構成のpayload_hashが監査へ結合される() {
        let mut broker = Broker::new("session-1");
        let payload = manifest();
        let response = call(&mut broker, payload.clone());
        let event = broker.audit_events().last().unwrap();
        assert_eq!(event.operation, OP_COMPOSE);
        assert_eq!(
            event.payload_hash,
            crate::broker::protocol::canonical_payload_hash(Some(&payload))
        );
        assert_eq!(response.audit_event_id, event.event_id);
    }

    #[test]
    fn Previewは差分と機能要件を表示するがbuildとrollbackを実行しない() {
        let mut broker = Broker::new("session-1");
        let response = call_preview(&mut broker, preview_payload(Value::Null, manifest()));
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.unwrap();
        assert_eq!(body["build_status"], "not_started");
        assert_eq!(body["export_status"], "not_started");
        assert_eq!(body["version_preview"]["rollback_available"], false);
        assert_eq!(body["diff"]["runtime_added"][0], "gui_shell_rust_broker");
        assert_eq!(body["permission_requirements"][0]["permission_status"], "not_generated");
    }

    #[test]
    fn Previewは現在構成との差分を決定論的に返す() {
        let mut broker = Broker::new("session-1");
        let current = manifest();
        let mut candidate = manifest();
        candidate["display_name"] = Value::from("変更後構成");
        candidate["agent_ids"] = json!(["codex", "claude"]);
        candidate["capability_requirements"] = json!(["runtime.read"]);
        let response = call_preview(&mut broker, preview_payload(current, candidate));
        let body = response.body.unwrap();
        assert_eq!(body["diff"]["agent_added"][0], "claude");
        assert_eq!(body["diff"]["capability_removed"][0], "agent.metadata");
        assert_eq!(body["diff"]["changed_fields"][0], "display_name");
    }

    #[test]
    fn Previewは権限継承と未知fieldを拒否する() {
        let mut broker = Broker::new("session-1");
        let mut invalid = preview_payload(Value::Null, manifest());
        invalid["candidate_manifest"]["inheritance_policy"]["permission"] = Value::from("permission.read");
        assert_eq!(call_preview(&mut broker, invalid).status, BrokerStatus::Rejected);

        let mut unknown = preview_payload(Value::Null, manifest());
        unknown["candidate_manifest"]["secret"] = Value::from("credential");
        assert_eq!(call_preview(&mut broker, unknown).status, BrokerStatus::Rejected);
    }
}
