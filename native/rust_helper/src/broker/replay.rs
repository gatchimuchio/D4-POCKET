//! 検証済みの過去要求を参照し、新Sessionの承認待ち要求を作る。
use super::super::dialogue::{対話要求, 要求hash};
use super::*;
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
}
