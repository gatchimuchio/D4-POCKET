//! Owner/Developerが明示開始したGUI Shell編集提案のBroker境界。
//!
//! この経路は提案を受理して審査待ちReceiptを返すだけであり、Repositoryの
//! file、process、network、credential、Permission、Approvalを変更しない。
#![allow(non_snake_case)]

use super::protocol::{
    Broker, BrokerResponse, BrokerStatus, EVIDENCE_SOURCE_INTERNAL_STATE,
};
use serde::Deserialize;
use serde_json::{json, Value};

const VERSION: u64 = 1;
const OPERATION: &str = "GUI Shell編集提案";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct EditProposal {
    version: u64,
    edit_id: String,
    scope: String,
    instruction: String,
    target_paths: Vec<String>,
    expected_changes: Vec<String>,
    source: String,
    repository_rules_acknowledged: bool,
    self_approval: bool,
    execution_mode: String,
}

pub(super) fn propose(
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
            "GUI Shell編集提案はowner／developer明示開始が必要",
            true,
            payload_hash,
        );
    }
    let proposal = match parse_proposal(payload) {
        Ok(proposal) => proposal,
        Err(reason) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_edit_proposal_invalid",
                &reason,
                true,
                payload_hash,
            )
        }
    };
    let audit = match broker.append_audit(
        request_id,
        OPERATION,
        "accepted",
        "owner／developer明示の編集提案を審査待ちで記録。自動apply、自己承認、権限変更は行わない",
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
            "edit_id": proposal.edit_id,
            "scope": proposal.scope,
            "target_paths": proposal.target_paths,
            "expected_change_count": proposal.expected_changes.len(),
            "execution_mode": "proposal_only",
            "review_required": true,
            "apply_status": "not_started",
            "files_written": false,
            "permission_generated": false,
            "approval_state": "owner_review_required",
            "authority_strip": true,
            "evidence_source": EVIDENCE_SOURCE_INTERNAL_STATE,
            "instruction_hash": payload_hash,
            "audit_id": audit.event_id,
        })),
        shutdown_requested: broker.shutdown_requested,
    }
}

fn parse_proposal(value: &Value) -> Result<EditProposal, String> {
    let proposal: EditProposal = serde_json::from_value(value.clone())
        .map_err(|_| "GUI Shell編集提案の構造が不正または禁止fieldがある".to_string())?;
    if proposal.version != VERSION
        || !valid_identifier(&proposal.edit_id)
        || !["composition", "ui", "contract"].contains(&proposal.scope.as_str())
        || proposal.instruction.trim().is_empty()
        || proposal.instruction.chars().count() > 2000
        || proposal.target_paths.is_empty()
        || proposal.target_paths.len() > 32
        || proposal.expected_changes.is_empty()
        || proposal.expected_changes.len() > 32
    {
        return Err("編集提案の版、識別子、範囲、指示、対象、変更予定が不正".to_string());
    }
    if proposal.source != "owner_directed_agent"
        || !proposal.repository_rules_acknowledged
        || proposal.self_approval
        || proposal.execution_mode != "proposal_only"
    {
        return Err("編集提案はowner規約確認済み、自己承認なし、proposal_onlyだけを受け付ける".to_string());
    }
    if proposal.instruction.contains('\0')
        || proposal.expected_changes.iter().any(|change| {
            change.trim().is_empty() || change.chars().count() > 240 || change.contains('\0')
        })
        || proposal.target_paths.iter().any(|path| !valid_target_path(path))
    {
        return Err("編集提案の指示、変更予定、対象pathが許可境界外".to_string());
    }
    Ok(proposal)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= 64
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
}

fn valid_target_path(value: &str) -> bool {
    let allowed_root = [
        "apps/desktop_flutter/",
        "packages/",
        "specs/",
        "docs/",
        "tooling/",
        "native/rust_helper/src/",
        "規定/",
    ]
    .iter()
    .any(|root| value.starts_with(root));
    let forbidden = [
        ".git",
        "MANIFEST.sha256.json",
        "release_evidence",
        "secret",
        "credential",
        "private",
        "signing",
    ];
    allowed_root
        && !value.contains('\\')
        && !value.contains(':')
        && value.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
        && value.chars().all(|character| !character.is_control())
        && !forbidden.iter().any(|token| value.to_ascii_lowercase().contains(token))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::protocol::{Broker, BrokerOperation, BrokerRequestEnvelope, BrokerStatus};

    fn proposal() -> Value {
        json!({
            "version": 1,
            "edit_id": "edit-compose-preview",
            "scope": "composition",
            "instruction": "構成Previewへ対象platformを表示する",
            "target_paths": ["apps/desktop_flutter/lib/screens/settings.dart", "docs/specs/gui-shell-preview.md"],
            "expected_changes": ["Preview結果に対象platformを表示する"],
            "source": "owner_directed_agent",
            "repository_rules_acknowledged": true,
            "self_approval": false,
            "execution_mode": "proposal_only"
        })
    }

    fn request(broker: &mut Broker, payload: Value) -> BrokerRequestEnvelope {
        let mut request = BrokerRequestEnvelope::command_envelope_at(
            "edit-test",
            "session-1",
            &format!("edit-nonce-{}", broker.audit_events().len()),
            &BrokerRequestEnvelope::current_issued_at(),
        );
        request.operation = Some(BrokerOperation::GuiShell編集提案);
        request.payload = Some(payload);
        request.refresh_payload_hash();
        request
    }

    fn owner_request_json(broker: &mut Broker, payload: Value) -> String {
        let nonce = format!("owner-edit-nonce-{}", broker.audit_events().len());
        let payload_hash = crate::broker::protocol::canonical_payload_hash(Some(&payload));
        json!({
            "request_id": "edit-owner-test",
            "session_id": "session-1",
            "operation": "GUI Shell編集提案",
            "payload": payload,
            "payload_hash": payload_hash,
            "nonce": nonce,
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {}
        })
        .to_string()
    }

    #[test]
    fn 編集提案はowner審査待ちで記録しfileを書かない() {
        let mut broker = Broker::new("session-1");
        let raw = owner_request_json(&mut broker, proposal());
        let response = broker.owner要求処理(&raw);
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.unwrap();
        assert_eq!(body["review_required"], true);
        assert_eq!(body["apply_status"], "not_started");
        assert_eq!(body["files_written"], false);
        assert_eq!(body["permission_generated"], false);
        assert_eq!(body["approval_state"], "owner_review_required");
        assert!(!body.to_string().contains("構成Previewへ対象platformを表示する"));
    }

    #[test]
    fn 編集提案は通常資格と自己承認と秘密pathを拒否する() {
        let mut broker = Broker::new("session-1");
        let normal_request = request(&mut broker, proposal());
        assert_eq!(broker.handle(normal_request).status, BrokerStatus::Rejected);

        let mut self_approval = proposal();
        self_approval["self_approval"] = Value::from(true);
        let raw = owner_request_json(&mut broker, self_approval);
        assert_eq!(broker.owner要求処理(&raw).status, BrokerStatus::Rejected);

        let mut secret_path = proposal();
        secret_path["target_paths"] = json!(["docs/credential-notes.md"]);
        let raw = owner_request_json(&mut broker, secret_path);
        assert_eq!(broker.owner要求処理(&raw).status, BrokerStatus::Rejected);
    }
}
