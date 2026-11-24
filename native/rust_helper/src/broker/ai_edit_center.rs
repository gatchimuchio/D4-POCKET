//! Owner/Developerが明示開始したGUI Shell編集提案のBroker境界。
//!
//! この経路は提案を受理して審査待ちReceiptを返すだけであり、Repositoryの
//! file、process、network、credential、Permission、Approvalを変更しない。
#![allow(non_snake_case)]

use super::protocol::{Broker, BrokerResponse, BrokerStatus, EVIDENCE_SOURCE_INTERNAL_STATE};
use serde::Deserialize;
use serde_json::{json, Value};

const VERSION: u64 = 1;
const OPERATION: &str = "GUI Shell編集提案";
pub(super) const REVIEW_OPERATION: &str = "GUI Shell編集提案審査";
pub(super) const APPLY_TARGET_OPERATION: &str = "GUI Shell編集適用先照合";
pub(super) const APPLY_OPERATION: &str = "GUI Shell編集適用";
pub(super) const ROLLBACK_OPERATION: &str = "GUI Shell編集Rollback";
pub(super) const ROLLBACK_CANDIDATES_OPERATION: &str = "GUI Shell編集Rollback候補一覧";

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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReviewRequest {
    version: u64,
    pub(super) proposal: Value,
    pub(super) proposal_audit_id: String,
    pub(super) workspace_id: String,
    pub(super) registration_hash: String,
    pub(super) baseline_hash: String,
    pub(super) approval_id: String,
}

impl ReviewRequest {
    pub(super) fn parse(payload: &Value) -> Result<Self, &'static str> {
        let request: Self =
            serde_json::from_value(payload.clone()).map_err(|_| "編集提案審査の入力が不正")?;
        if request.version != VERSION
            || !valid_identifier(&request.proposal_audit_id)
            || !super::workspace::identifier(&request.workspace_id)
            || !valid_identifier(&request.approval_id)
            || !valid_hash(&request.registration_hash)
            || !valid_hash(&request.baseline_hash)
        {
            return Err("編集提案審査の版、作業領域、承認または基準点が不正");
        }
        Ok(request)
    }

    pub(super) fn proposal_scope(&self) -> Result<(String, Vec<String>), &'static str> {
        let proposal = parse_proposal(&self.proposal).map_err(|_| "審査対象の提案が不正")?;
        Ok((proposal.edit_id, proposal.target_paths))
    }

    pub(super) fn verify_receipt(&self, broker: &Broker) -> Result<(), &'static str> {
        self.proposal_scope()?;
        let hash = super::protocol::canonical_payload_hash(Some(&self.proposal));
        let event = current_host_receipt(broker, &self.proposal_audit_id)?;
        if event.operation != OPERATION
            || event.decision != "accepted"
            || event.evidence_source != EVIDENCE_SOURCE_INTERNAL_STATE
            || event.payload_hash != hash
        {
            return Err("提案受付の監査と現在の提案が不一致");
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ApplyTargetRequest {
    version: u64,
    pub(super) review_audit_id: String,
    review_binding: Value,
    pub(super) workspace_id: String,
    pub(super) registration_hash: String,
    pub(super) approval_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ApplyBinding {
    pub(super) proposal_id: String,
    pub(super) diff_hash: String,
    pub(super) workspace_id: String,
    pub(super) registration_hash: String,
    pub(super) approval_id: String,
    pub(super) expires_at: i64,
    pub(super) baseline_hash: String,
    pub(super) files: Vec<ApplyFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ApplyFile {
    pub(super) path: String,
    pub(super) before_hash: Option<String>,
    pub(super) after_hash: Option<String>,
}

impl ApplyTargetRequest {
    pub(super) fn parse(payload: &Value) -> Result<Self, &'static str> {
        let request: Self =
            serde_json::from_value(payload.clone()).map_err(|_| "編集適用先照合の入力が不正")?;
        if request.version != VERSION
            || !valid_identifier(&request.review_audit_id)
            || !super::workspace::identifier(&request.workspace_id)
            || !valid_hash(&request.registration_hash)
            || !valid_identifier(&request.approval_id)
        {
            return Err("編集適用先照合の審査参照または登録が不正");
        }
        Ok(request)
    }

    pub(super) fn binding(&self) -> Result<ApplyBinding, &'static str> {
        let value: ApplyBinding = serde_json::from_value(self.review_binding.clone())
            .map_err(|_| "編集審査の結合が不正")?;
        if value.files.is_empty() || value.files.len() > 32 {
            return Err("編集審査の対象件数が不正");
        }
        Ok(value)
    }

    pub(super) fn verify_receipt(&self, broker: &Broker) -> Result<(), &'static str> {
        let event = current_host_receipt(broker, &self.review_audit_id)?;
        if event.operation != REVIEW_OPERATION
            || event.decision != "accepted"
            || event.evidence_source != super::protocol::EVIDENCE_SOURCE_LIVE_RUNTIME
            || event.reason != review_binding_reason(&self.review_binding)
        {
            return Err("編集審査の永続監査と適用先照合の入力が不一致");
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ApplyRequest {
    version: u64,
    pub(super) target_audit_id: String,
    pub(super) target_request: Value,
}

impl ApplyRequest {
    pub(super) fn parse(
        payload: &Value,
    ) -> Result<(Self, ApplyTargetRequest, ApplyBinding), &'static str> {
        let request: Self =
            serde_json::from_value(payload.clone()).map_err(|_| "編集適用の入力が不正")?;
        if request.version != VERSION || !valid_identifier(&request.target_audit_id) {
            return Err("編集適用の版または適用先監査参照が不正");
        }
        let target = ApplyTargetRequest::parse(&request.target_request)?;
        let binding = target.binding()?;
        if !valid_identifier(&binding.proposal_id)
            || !valid_hash(&binding.diff_hash)
            || binding.files.iter().any(|file| {
                file.path.len() > 200
                    || crate::workspace_reader::WorkspaceReader::validate_registered_secret_path(
                        &file.path,
                    )
                    .is_err()
                    || file
                        .path
                        .chars()
                        .any(|c| matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
                    || file.before_hash == file.after_hash
                    || [file.before_hash.as_deref(), file.after_hash.as_deref()]
                        .into_iter()
                        .flatten()
                        .any(|hash| !valid_hash(hash))
            })
        {
            return Err("編集適用の差分結合または対象版が不正");
        }
        Ok((request, target, binding))
    }
}

/// 既に完了した適用だけを、別の現在Owner確認で巻き戻す要求。
/// 埋め込みの適用要求とhashは対象特定・監査相関にだけ使い、旧Approvalを再利用しない。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RollbackRequest {
    version: u64,
    pub(super) transaction_id: String,
    pub(super) applied_request_hash: String,
    pub(super) applied_audit_id: String,
    pub(super) applied_request: Value,
    pub(super) workspace_id: String,
    pub(super) registration_hash: String,
    pub(super) approval_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RollbackCandidatesRequest {
    version: u64,
    pub(super) workspace_id: String,
    pub(super) registration_hash: String,
    pub(super) approval_id: String,
    #[serde(default, deserialize_with = "deserialize_cursor")]
    pub(super) cursor: Option<String>,
}

fn deserialize_cursor<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer).map(Some)
}

impl RollbackCandidatesRequest {
    pub(super) fn parse(payload: &Value) -> Result<Self, &'static str> {
        let request: Self =
            serde_json::from_value(payload.clone()).map_err(|_| "Rollback候補一覧の入力が不正")?;
        if request.version != VERSION
            || !super::workspace::identifier(&request.workspace_id)
            || !valid_hash(&request.registration_hash)
            || !valid_identifier(&request.approval_id)
            || request
                .cursor
                .as_deref()
                .is_some_and(|cursor| !valid_transaction_id(cursor))
        {
            return Err("Rollback候補一覧の版、Workspace、Permissionまたはcursorが不正");
        }
        Ok(request)
    }
}

impl RollbackRequest {
    pub(super) fn parse(payload: &Value) -> Result<Self, &'static str> {
        let request: Self =
            serde_json::from_value(payload.clone()).map_err(|_| "編集Rollback要求の形式が不正")?;
        let (_, target, _) = ApplyRequest::parse(&request.applied_request)?;
        if request.version != VERSION
            || !valid_transaction_id(&request.transaction_id)
            || !valid_hash(&request.applied_request_hash)
            || !valid_identifier(&request.applied_audit_id)
            || !super::workspace::identifier(&request.workspace_id)
            || !valid_hash(&request.registration_hash)
            || !valid_identifier(&request.approval_id)
            || target.workspace_id != request.workspace_id
            || super::protocol::canonical_payload_hash(Some(&request.applied_request))
                != request.applied_request_hash
        {
            return Err("編集Rollbackの適用記録、Workspaceまたは現在Approvalが不正");
        }
        Ok(request)
    }

    pub(super) fn applied_context(
        &self,
    ) -> Result<(ApplyTargetRequest, ApplyBinding), &'static str> {
        let (_, target, binding) = ApplyRequest::parse(&self.applied_request)?;
        Ok((target, binding))
    }

    pub(super) fn verify_applied_receipt(&self, broker: &Broker) -> Result<(), &'static str> {
        let (target, binding) = self.applied_context()?;
        let event = current_host_receipt(broker, &self.applied_audit_id)?;
        let expected = applied_receipt_body(
            &self.applied_request_hash,
            &target.workspace_id,
            &target.review_audit_id,
            &binding.diff_hash,
            &self.transaction_id,
            binding.files.len(),
        );
        if event.operation != APPLY_OPERATION
            || event.decision != "accepted"
            || event.evidence_source != super::protocol::EVIDENCE_SOURCE_LIVE_RUNTIME
            || event.reason != "編集適用と復旧記録の耐久確定"
            || event.payload_hash != super::protocol::canonical_payload_hash(Some(&expected))
        {
            return Err("巻戻し対象の適用Audit receiptが現在Host・Journalと一致しない");
        }
        Ok(())
    }
}

pub(crate) fn rollback_confirmation_summary(payload: &Value, payload_hash: &str) -> Option<String> {
    let request = RollbackRequest::parse(payload).ok()?;
    if !valid_hash(payload_hash)
        || super::protocol::canonical_payload_hash(Some(payload)) != payload_hash
    {
        return None;
    }
    let (_, binding) = request.applied_context().ok()?;
    let files = binding
        .files
        .iter()
        .map(|file| {
            let effect = match (&file.before_hash, &file.after_hash) {
                (None, Some(_)) => "追加版を削除",
                (Some(_), None) => "削除前の版を復元",
                (Some(_), Some(_)) => "変更前の版を復元",
                (None, None) => return None,
            };
            Some(format!("{effect}  {}", file.path))
        })
        .collect::<Option<Vec<_>>>()?
        .join("\n");
    Some(format!(
        "別の現在Owner確認で、完了済みのGUI Shell編集を巻き戻します。\nWorkspace: {}\n現在の登録hash: {}\n現在のPermission ID: {}\nTransaction: {}\n適用Audit: {}\n適用要求hash: {}\n差分hash: {}\n対象: {}件\n{}\nRollback要求hash: {}\n\nBrokerは現在のWorkspace登録・Permission・root実体・耐久Journal・各file hashを再確認します。元の適用Approvalは再利用しません。",
        request.workspace_id,
        request.registration_hash,
        request.approval_id,
        request.transaction_id,
        request.applied_audit_id,
        request.applied_request_hash,
        binding.diff_hash,
        binding.files.len(),
        files,
        payload_hash,
    ))
}

pub(crate) fn apply_confirmation_summary(payload: &Value, request_hash: &str) -> Option<String> {
    let (_, target, binding) = ApplyRequest::parse(payload).ok()?;
    if !valid_hash(request_hash) {
        return None;
    }
    let files = binding
        .files
        .iter()
        .map(|file| {
            let effect = match (&file.before_hash, &file.after_hash) {
                (None, _) => "追加",
                (_, None) => "削除",
                _ => "置換",
            };
            format!("{effect}  {}", file.path)
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!(
        "審査済み差分を作業領域へ一回適用します。\n作業領域 {}\n提案 {}\n差分hash {}\n対象 {}件\n{}\n要求hash {}",
        target.workspace_id, binding.proposal_id, binding.diff_hash, binding.files.len(), files, request_hash,
    ))
}

pub(super) fn review_binding(body: &Value, changes: &[Value]) -> Value {
    let review = &body["projection"];
    let files: Vec<Value> = changes
        .iter()
        .map(|change| {
            json!({
                "path": change["path"],
                "before_hash": change["diff"]["before"]["sha256"],
                "after_hash": change["diff"]["after"]["sha256"],
            })
        })
        .collect();
    json!({
        "proposal_id": review["proposal_id"], "diff_hash": review["diff_hash"],
        "workspace_id": body["作業領域ID"], "registration_hash": body["登録hash"],
        "approval_id": body["approval_id"], "expires_at": body["有効期限"],
        "baseline_hash": review["基準点hash"], "files": files
    })
}

pub(super) fn review_binding_reason(binding: &Value) -> String {
    format!(
        "編集提案審査の適用用結合hash {}",
        super::protocol::canonical_payload_hash(Some(binding))
    )
}

pub(super) fn current_host_receipt(
    broker: &Broker,
    audit_id: &str,
) -> Result<super::audit::BrokerAuditEvent, &'static str> {
    let log = broker
        .state_store
        .verified_audit_log()
        .map_err(|_| "編集の永続監査を検証できない")?
        .ok_or("編集の永続監査がない")?;
    current_host_receipt_in_log(&log, audit_id)
}

pub(super) fn current_host_receipt_in_log(
    log: &super::audit::BrokerAuditLog,
    audit_id: &str,
) -> Result<super::audit::BrokerAuditEvent, &'static str> {
    current_host_events(log)
        .iter()
        .find(|event| event.event_id == audit_id)
        .cloned()
        .ok_or("現在Hostに一致する編集監査がない")
}

pub(super) fn find_applied_receipt(
    log: &super::audit::BrokerAuditLog,
    request_hash: &str,
    workspace_id: &str,
    review_audit_id: &str,
    diff_hash: &str,
    transaction_id: &str,
    changed_files: usize,
) -> Result<super::audit::BrokerAuditEvent, &'static str> {
    let expected_hash = super::protocol::canonical_payload_hash(Some(&applied_receipt_body(
        request_hash,
        workspace_id,
        review_audit_id,
        diff_hash,
        transaction_id,
        changed_files,
    )));
    let matches = current_host_events(log)
        .iter()
        .filter(|event| {
            event.operation == APPLY_OPERATION
                && event.decision == "accepted"
                && event.evidence_source == super::protocol::EVIDENCE_SOURCE_LIVE_RUNTIME
                && event.reason == "編集適用と復旧記録の耐久確定"
                && event.payload_hash == expected_hash
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [event] => Ok((*event).clone()),
        [] => Err("現在Hostに適用完了Audit receiptがない"),
        _ => Err("同じ編集適用に複数のAudit receiptがある"),
    }
}

fn current_host_events(log: &super::audit::BrokerAuditLog) -> &[super::audit::BrokerAuditEvent] {
    let start = log
        .events()
        .iter()
        .rposition(|event| event.operation == "Host切替" && event.decision == "accepted")
        .map_or(0, |index| index + 1);
    &log.events()[start..]
}

fn applied_receipt_body(
    request_hash: &str,
    workspace_id: &str,
    review_audit_id: &str,
    diff_hash: &str,
    transaction_id: &str,
    changed_files: usize,
) -> Value {
    json!({
        "version": 1,
        "operation": APPLY_OPERATION,
        "request_hash": request_hash,
        "workspace_id": workspace_id,
        "review_audit_id": review_audit_id,
        "diff_hash": diff_hash,
        "transaction_id": transaction_id,
        "changed_files": changed_files,
        "status": "applied"
    })
}

fn valid_hash(hash: &str) -> bool {
    hash.strip_prefix("sha256:").is_some_and(|value| {
        value.len() == 64
            && value
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    })
}

fn valid_transaction_id(id: &str) -> bool {
    id.len() == 64
        && id
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

pub(crate) fn owner_confirmation_summary(payload: &Value, payload_hash: &str) -> Option<String> {
    let proposal = parse_proposal(payload).ok()?;
    let scope = match proposal.scope.as_str() {
        "composition" => "構成",
        "ui" => "画面",
        "contract" => "契約",
        _ => return None,
    };
    Some(format!(
        "このGUI Shell編集提案を審査待ちとして記録しますか？\n\n提案ID {}\n範囲 {scope}\n対象ファイル {} 件\n予定変更 {} 件\n要求hash {payload_hash}\n\n指示と対象は元の入力画面で確認してください。Taskの開始、ファイルの変更、PermissionやApprovalの発行は行いません。",
        proposal.edit_id,
        proposal.target_paths.len(),
        proposal.expected_changes.len(),
    ))
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
    let duplicate_target_path = proposal
        .target_paths
        .iter()
        .enumerate()
        .any(|(index, path)| proposal.target_paths[..index].contains(path));
    if proposal.version != VERSION
        || !valid_identifier(&proposal.edit_id)
        || !["composition", "ui", "contract"].contains(&proposal.scope.as_str())
        || proposal.instruction.trim().is_empty()
        || proposal.instruction.chars().count() > 2000
        || proposal.target_paths.is_empty()
        || proposal.target_paths.len() > 32
        || duplicate_target_path
        || proposal
            .target_paths
            .iter()
            .any(|path| path.chars().count() > 200)
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
        return Err(
            "編集提案はowner規約確認済み、自己承認なし、proposal_onlyだけを受け付ける".to_string(),
        );
    }
    if proposal.instruction.contains('\0')
        || proposal.expected_changes.iter().any(|change| {
            change.trim().is_empty() || change.chars().count() > 240 || change.contains('\0')
        })
        || proposal
            .target_paths
            .iter()
            .any(|path| !valid_target_path(path))
    {
        return Err("編集提案の指示、変更予定、対象pathが許可境界外".to_string());
    }
    Ok(proposal)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= 64
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
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
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && value.chars().all(|character| !character.is_control())
        && !forbidden
            .iter()
            .any(|token| value.to_ascii_lowercase().contains(token))
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
        assert!(!body
            .to_string()
            .contains("構成Previewへ対象platformを表示する"));
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

    #[test]
    fn 編集提案は重複と長さ超過の対象pathを拒否する() {
        let mut broker = Broker::new("session-1");

        let mut duplicate_paths = proposal();
        duplicate_paths["target_paths"] = json!([
            "apps/desktop_flutter/lib/screens/settings.dart",
            "apps/desktop_flutter/lib/screens/settings.dart"
        ]);
        let raw = owner_request_json(&mut broker, duplicate_paths);
        assert_eq!(broker.owner要求処理(&raw).status, BrokerStatus::Rejected);

        let too_long_path = format!("apps/desktop_flutter/{}", "a".repeat(180));
        assert_eq!(too_long_path.chars().count(), 201);
        let mut oversized_path = proposal();
        oversized_path["target_paths"] = json!([too_long_path]);
        let raw = owner_request_json(&mut broker, oversized_path);
        assert_eq!(broker.owner要求処理(&raw).status, BrokerStatus::Rejected);
    }
}
