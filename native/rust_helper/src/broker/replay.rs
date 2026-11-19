//! 検証済みの過去要求を参照し、新Sessionの承認待ち要求を作る。
use super::super::dialogue::{対話要求, 要求hash};
use super::*;
use crate::broker::audit::BrokerAuditLog;
use crate::broker::execution_history::{agent_task_page, AgentTaskHistoryQuery};
use serde::Deserialize;
use serde_json::json;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    approval_id: String,
    #[serde(rename = "実行系ID")]
    runtime: String,
    #[serde(rename = "参照監査ID")]
    audit: String,
    #[serde(rename = "参照event_hash")]
    event_hash: String,
    #[serde(rename = "入力")]
    input: String,
    #[serde(default)]
    source_task_id: Option<String>,
}

fn agent_task_history_source(
    log: &BrokerAuditLog,
    runtime_id: &str,
    task_id: &str,
    audit_id: &str,
    event_hash: &str,
    replay_instruction_hash: Option<&str>,
) -> Result<serde_json::Value, &'static str> {
    if task_id.len() != 32
        || !task_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("Agent Task履歴のTask IDが不正");
    }
    let after = log
        .events()
        .iter()
        .position(|event| event.event_id == audit_id && event.event_hash == event_hash)
        .ok_or("Agent Task履歴のAudit参照が不一致")?;
    let page = agent_task_page(log, runtime_id, AgentTaskHistoryQuery { after, limit: 50 })?;
    let entries = page["entries"]
        .as_array()
        .ok_or("Agent Task履歴pageが不正")?;
    let selected = entries
        .iter()
        .find(|entry| entry["audit_event_id"] == audit_id && entry["event_hash"] == event_hash)
        .ok_or("Agent Task履歴が古いか現在の履歴にありません")?;
    let record = &selected["record"];
    if record["task_id"] != task_id
        || record["runtime_id"] != runtime_id
        || !["completed", "failed", "cancelled", "suspended"]
            .contains(&record["status"].as_str().unwrap_or(""))
    {
        return Err("Agent Task履歴の対象・Runtimeまたは終端状態が不一致");
    }
    if let Some(expected) = replay_instruction_hash {
        if record["instruction_hash"].as_str() != Some(expected) {
            return Err("再実行の指示が元Taskの指示hashと一致しません");
        }
    }
    Ok(selected.clone())
}

impl Broker {
    pub(super) fn 履歴再要求(
        &mut self,
        id: &str,
        op: &str,
        payload: &Value,
        hash: &str,
    ) -> BrokerResponse {
        if self.append_audit(id,op,"received","Capability=対話送信 Permission=新要求で再評価 Approval=新規承認待ち Recovery=新規要求の再確認",EVIDENCE_SOURCE_INTERNAL_STATE,hash).is_err() {
            return self.audit_store_failed_response(id,op,"監査失敗","履歴再要求を開始できない");
        }
        let p: Selection = match serde_json::from_value(payload.clone()) {
            Ok(v) => v,
            Err(_) => {
                return self.reject_with_payload_hash(
                    id,
                    op,
                    "要求不正",
                    "履歴再要求の形式が不正",
                    true,
                    hash,
                )
            }
        };
        if let Some(source_task_id) = p.source_task_id.as_deref() {
            return self.AgentTask履歴再要求(id, op, &p, source_task_id, hash);
        }
        if p.input.trim().is_empty() || p.input.chars().count() > 4096 {
            return self.reject_with_payload_hash(id, op, "要求不正", "入力範囲が不正", true, hash);
        }
        let index = match self
            .audit_log
            .events()
            .iter()
            .position(|e| e.event_id == p.audit && e.event_hash == p.event_hash)
        {
            Some(v) => v,
            None => {
                return self.reject_with_payload_hash(
                    id,
                    op,
                    "要求不正",
                    "参照履歴が一致しない",
                    true,
                    hash,
                )
            }
        };
        let selection = json!({"approval_id":p.approval_id,"query":{"after":index,"limit":1,"filter":{"実行系ID":p.runtime}}});
        let mut observed = self.履歴閲覧処理(
            id,
            "対話履歴閲覧",
            &selection,
            false,
            &sha256_tagged(selection.to_string().as_bytes()),
        );
        if observed.status != BrokerStatus::Accepted {
            observed.operation = op.into();
            return observed;
        }
        let access = observed.body.take().unwrap();
        let parent = &access["page"]["entries"][0];
        if parent["audit_event_id"] != p.audit || parent["event_hash"] != p.event_hash {
            return self.reject_with_payload_hash(
                id,
                op,
                "権限拒否",
                "参照履歴が現在承認範囲と一致しない",
                true,
                hash,
            );
        }
        let record = &parent["record"]["実行記録"];
        let old = 対話要求 {
            要求ID: record["要求ID"].as_str().unwrap().into(),
            実行系ID: p.runtime.clone(),
            対話セッションID: record["対話セッションID"].as_str().unwrap().into(),
            入力: p.input.clone(),
        };
        if op == "対話再実行"
            && !self.audit_log.events().iter().any(|e| {
                Some(e.event_id.as_str()) == record["作成監査ID"].as_str()
                    && e.payload_hash == 要求hash(&old)
            })
        {
            return self.reject_with_payload_hash(
                id,
                op,
                "要求不正",
                "再実行入力が過去要求と一致しない",
                true,
                hash,
            );
        }
        if !self.対話.登録済み(&p.runtime) {
            return self.reject_with_payload_hash(
                id,
                op,
                "実行系不在",
                "現在登録された同一Runtimeが必要",
                true,
                hash,
            );
        }
        let now = self.current_epoch_seconds();
        if !self.履歴閲覧.current(&access, now) {
            return self.reject_with_payload_hash(
                id,
                op,
                "期限超過",
                "履歴承認の期限超過",
                true,
                hash,
            );
        }
        let start = json!({"実行系ID":p.runtime});
        let mut started = self.対話要求処理(
            id,
            BrokerOperation::対話開始,
            &start,
            false,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &sha256_tagged(start.to_string().as_bytes()),
        );
        if started.status != BrokerStatus::Accepted {
            started.operation = op.into();
            return started;
        }
        let session = started.body.unwrap()["対話セッションID"]
            .as_str()
            .unwrap()
            .to_owned();
        let send = json!({"対話セッションID":session,"入力":p.input});
        let mut created = self.対話要求処理(
            id,
            BrokerOperation::対話送信,
            &send,
            false,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &sha256_tagged(send.to_string().as_bytes()),
        );
        if created.status != BrokerStatus::Accepted {
            self.対話.資格隔離(&[session]);
            created.operation = op.into();
            return created;
        }
        let mut body = created.body.take().unwrap();
        body["対話セッションID"] = json!(session);
        body["実行系ID"] = json!(p.runtime);
        body["参照監査ID"] = json!(p.audit);
        body["参照event_hash"] = json!(p.event_hash);
        body["種別"] = json!(op);
        let reason = format!("履歴参照再要求:{body}");
        match self.append_audit(
            id,
            op,
            "accepted",
            &reason,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &sha256_tagged(body.to_string().as_bytes()),
        ) {
            Err(_) => {
                self.対話.資格隔離(&[session]);
                self.audit_store_failed_response(
                    id,
                    op,
                    "監査失敗",
                    "参照要求を隔離したため監査修復後に再確認",
                )
            }
            Ok(event) => {
                let now = self.current_epoch_seconds();
                if !self.履歴閲覧.current(&access, now) {
                    self.対話.資格隔離(&[session]);
                    return self.reject_with_payload_hash(
                        id,
                        op,
                        "期限超過",
                        "新要求を隔離したため再承認が必要",
                        true,
                        hash,
                    );
                }
                BrokerResponse {
                    request_id: id.into(),
                    operation: op.into(),
                    status: BrokerStatus::Accepted,
                    evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.into(),
                    audit_event_id: event.event_id,
                    error: None,
                    health: None,
                    body: Some(body),
                    shutdown_requested: false,
                }
            }
        }
    }

    fn AgentTask履歴再要求(
        &mut self,
        id: &str,
        op: &str,
        selection: &Selection,
        source_task_id: &str,
        request_hash: &str,
    ) -> BrokerResponse {
        if selection.input.trim().is_empty() || selection.input.chars().count() > 32768 {
            return self.reject_with_payload_hash(
                id,
                op,
                "要求不正",
                "新しいTask指示は1〜32768文字で指定してください",
                true,
                request_hash,
            );
        }
        let request = json!({
            "approval_id": selection.approval_id,
            "query": {"after": 0, "limit": 50}
        });
        let mut history = self.履歴閲覧処理(
            id,
            "AgentTask履歴閲覧",
            &request,
            false,
            &sha256_tagged(request.to_string().as_bytes()),
        );
        if history.status != BrokerStatus::Accepted {
            history.operation = op.into();
            return history;
        }
        let access = history.body.take().unwrap_or(Value::Null);
        let Some(grant) = access["grant"].as_object() else {
            return self.reject_with_payload_hash(
                id,
                op,
                "権限拒否",
                "現在のAgent Task履歴閲覧Grantがありません",
                true,
                request_hash,
            );
        };
        if grant["approval_id"] != selection.approval_id || grant["runtime_id"] != selection.runtime
        {
            return self.reject_with_payload_hash(
                id,
                op,
                "権限拒否",
                "Agent Task履歴のRuntimeまたはGrantが一致しません",
                true,
                request_hash,
            );
        }
        let instruction_hash = sha256_tagged(selection.input.as_bytes());
        let replay = op == "対話再実行";
        let source = match agent_task_history_source(
            &self.audit_log,
            &selection.runtime,
            source_task_id,
            &selection.audit,
            &selection.event_hash,
            replay.then_some(instruction_hash.as_str()),
        ) {
            Ok(entry) => entry,
            Err(reason) => {
                return self.reject_with_payload_hash(
                    id,
                    op,
                    "要求不正",
                    reason,
                    true,
                    request_hash,
                )
            }
        };
        let record = &source["record"];
        if source["audit_event_id"] != selection.audit
            || source["event_hash"] != selection.event_hash
        {
            return self.reject_with_payload_hash(
                id,
                op,
                "要求不正",
                "選択したAgent Task履歴が現在のAudit headと一致しません",
                true,
                request_hash,
            );
        }
        let workspace_id = match record["workspace_id"].as_str() {
            Some(value) if !value.is_empty() && value.len() <= 128 => value.to_owned(),
            _ => {
                return self.reject_with_payload_hash(
                    id,
                    op,
                    "要求不正",
                    "Agent Task履歴のWorkspace IDが不正です",
                    true,
                    request_hash,
                )
            }
        };
        if !self.対話.登録済み(&selection.runtime) {
            return self.reject_with_payload_hash(
                id,
                op,
                "実行系不在",
                "現在登録された同一Runtimeが必要です",
                true,
                request_hash,
            );
        }
        let now = self.current_epoch_seconds();
        if !self.履歴閲覧.current(&access, now) {
            return self.reject_with_payload_hash(
                id,
                op,
                "期限超過",
                "Agent Task履歴承認の期限超過",
                true,
                request_hash,
            );
        }
        let expires = match grant["expires_at"].as_i64() {
            Some(value) if value > now => value,
            _ => {
                return self.reject_with_payload_hash(
                    id,
                    op,
                    "期限超過",
                    "Agent Task履歴承認の期限超過",
                    true,
                    request_hash,
                )
            }
        };
        let start_request = json!({
            "実行系ID": selection.runtime,
            "作業領域ID": workspace_id
        });
        let mut started = self.対話要求処理(
            id,
            BrokerOperation::対話開始,
            &start_request,
            false,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &sha256_tagged(start_request.to_string().as_bytes()),
        );
        if started.status != BrokerStatus::Accepted {
            started.operation = op.into();
            return started;
        }
        let source_session_id = record["session_id"].as_str().unwrap_or("");
        let session_id = match started
            .body
            .as_ref()
            .and_then(|body| body["対話セッションID"].as_str())
        {
            Some(value) if value != source_session_id => value.to_owned(),
            _ => {
                if let Some(value) = started
                    .body
                    .as_ref()
                    .and_then(|body| body["対話セッションID"].as_str())
                    .filter(|value| *value != source_session_id)
                {
                    self.対話.資格隔離(&[value.to_owned()]);
                }
                return self.reject_with_payload_hash(
                    id,
                    op,
                    "応答不正",
                    "新しいAgent Task Sessionを確認できません",
                    true,
                    request_hash,
                );
            }
        };
        if !self.履歴閲覧.current(&access, self.current_epoch_seconds()) {
            self.対話.資格隔離(std::slice::from_ref(&session_id));
            return self.reject_with_payload_hash(
                id,
                op,
                "期限超過",
                "履歴承認が失効したため新Sessionを隔離しました",
                true,
                request_hash,
            );
        }
        let audit_record = json!({
            "source_task_id": source_task_id,
            "source_audit_event_id": selection.audit,
            "source_event_hash": selection.event_hash,
            "runtime_id": selection.runtime,
            "workspace_id": workspace_id,
            "new_session_id": session_id,
            "new_instruction_hash": instruction_hash
        });
        let action = if replay { "再実行" } else { "分岐" };
        let reason = format!("AgentTask履歴{action}:{audit_record}");
        let event = match self.append_audit(
            id,
            op,
            "accepted",
            &reason,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &sha256_tagged(audit_record.to_string().as_bytes()),
        ) {
            Ok(event) => event,
            Err(_) => {
                self.対話.資格隔離(std::slice::from_ref(&session_id));
                return self.audit_store_failed_response(
                    id,
                    op,
                    "監査失敗",
                    "新Task Sessionを隔離したため監査修復後に再確認",
                );
            }
        };
        if !self.履歴閲覧.current(&access, self.current_epoch_seconds()) {
            self.対話.資格隔離(std::slice::from_ref(&session_id));
            return self.reject_with_payload_hash(
                id,
                op,
                "期限超過",
                "新Task Sessionを隔離したため履歴承認の再確認が必要",
                true,
                request_hash,
            );
        }
        let body = json!({
            "要求ID": id,
            "要求hash": instruction_hash,
            "状態": "Task開始前",
            "期限": expires,
            "対話セッションID": session_id,
            "実行系ID": selection.runtime,
            "参照監査ID": selection.audit,
            "参照event_hash": selection.event_hash,
            "種別": format!("AgentTask{action}"),
            "source_task_id": source_task_id,
            "作業領域ID": workspace_id
        });
        BrokerResponse {
            request_id: id.into(),
            operation: op.into(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.into(),
            audit_event_id: event.event_id,
            error: None,
            health: None,
            body: Some(body),
            shutdown_requested: false,
        }
    }
}

#[cfg(test)]
mod agent_task_branch_tests {
    use super::*;
    use crate::audit_hash::sha256_tagged;
    use crate::broker::execution_history::{encode_agent_task_audit, AgentTaskHistoryAuditRecord};

    fn task_record(
        stage: &str,
        start_audit_event_id: Option<String>,
    ) -> AgentTaskHistoryAuditRecord {
        AgentTaskHistoryAuditRecord {
            version: 1,
            stage: stage.into(),
            task_id: "a".repeat(32),
            runtime_id: "runtime-a".into(),
            session_id: "b".repeat(32),
            workspace_id: "workspace-a".into(),
            instruction_hash: sha256_tagged(b"instruction"),
            request_hash: sha256_tagged(b"request"),
            created_at: 100,
            status: if stage == "start" {
                "running"
            } else {
                "completed"
            }
            .into(),
            updated_at: if stage == "start" { 100 } else { 101 },
            result_hash: (stage == "terminal").then(|| sha256_tagged(b"result")),
            failure_class: None,
            start_audit_event_id,
            buffer_intervention_record_hash: None,
            buffer_intervention_status: None,
            buffer_intervention_audit_event_id: None,
        }
    }

    fn completed_task_log() -> (BrokerAuditLog, String, String) {
        let mut log = BrokerAuditLog::default();
        let (reason, digest) = encode_agent_task_audit(&task_record("start", None)).unwrap();
        let start = log.append(
            &"a".repeat(32),
            "AgentTask実行",
            "recorded",
            &reason,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &digest,
        );
        let (reason, digest) =
            encode_agent_task_audit(&task_record("terminal", Some(start.event_id))).unwrap();
        let terminal = log.append(
            &"a".repeat(32),
            "AgentTask状態",
            "recorded",
            &reason,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &digest,
        );
        (log, terminal.event_id, terminal.event_hash)
    }

    #[test]
    fn Task分岐元は現在の終端AuditとTask結合を必要とする() {
        let (log, audit_id, event_hash) = completed_task_log();
        let selected = agent_task_history_source(
            &log,
            "runtime-a",
            &"a".repeat(32),
            &audit_id,
            &event_hash,
            None,
        )
        .unwrap();
        assert_eq!(selected["record"]["status"], "completed");
        assert_eq!(
            agent_task_history_source(
                &log,
                "runtime-a",
                &"a".repeat(32),
                &audit_id,
                &sha256_tagged(b"stale"),
                None,
            )
            .unwrap_err(),
            "Agent Task履歴のAudit参照が不一致"
        );
        assert!(agent_task_history_source(
            &log,
            "runtime-b",
            &"a".repeat(32),
            &audit_id,
            &event_hash,
            None,
        )
        .is_err());
        assert!(agent_task_history_source(
            &log,
            "runtime-a",
            &"c".repeat(32),
            &audit_id,
            &event_hash,
            None,
        )
        .is_err());
    }

    #[test]
    fn Task再実行は現在の終端Auditの指示hashに一致する場合だけ認める() {
        let (log, audit_id, event_hash) = completed_task_log();
        let task_id = "a".repeat(32);
        let verify = |instruction: &str| {
            agent_task_history_source(
                &log,
                "runtime-a",
                &task_id,
                &audit_id,
                &event_hash,
                Some(&sha256_tagged(instruction.as_bytes())),
            )
        };
        assert!(verify("instruction").is_ok());
        assert_eq!(
            verify("instruction ").unwrap_err(),
            "再実行の指示が元Taskの指示hashと一致しません"
        );
    }

    #[test]
    fn active_Taskや古い開始Auditから分岐できない() {
        let mut log = BrokerAuditLog::default();
        let (reason, digest) = encode_agent_task_audit(&task_record("start", None)).unwrap();
        let start = log.append(
            &"a".repeat(32),
            "AgentTask実行",
            "recorded",
            &reason,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &digest,
        );
        assert!(agent_task_history_source(
            &log,
            "runtime-a",
            &"a".repeat(32),
            &start.event_id,
            &start.event_hash,
            None,
        )
        .is_err());

        let (log, _, _) = completed_task_log();
        let start = &log.events()[0];
        assert!(agent_task_history_source(
            &log,
            "runtime-a",
            &"a".repeat(32),
            &start.event_id,
            &start.event_hash,
            None,
        )
        .is_err());
    }
}
