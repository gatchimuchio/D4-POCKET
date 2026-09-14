//! 部分破棄中断の現在不在を再観測する。元の削除結果は作り直さない。
use super::*;
impl Broker {
    pub(super) fn 部分破棄中断確認処理(
        &mut self,
        id: &str,
        payload: &Value,
        owner: bool,
        hash: &str,
    ) -> BrokerResponse {
        let op = "対話部分破棄中断確認";
        if !owner || !self.state_store.persistence_ready() {
            return self.reject_with_payload_hash(
                id,
                op,
                "権限拒否",
                "現在owner制御資格と永続監査が必要",
                true,
                hash,
            );
        }
        #[cfg(not(windows))]
        {
            let _ = payload;
            self.reject_with_payload_hash(
                id,
                op,
                "保管未対応",
                "このOSの安全保管は未対応",
                true,
                hash,
            )
        }
        #[cfg(windows)]
        {
            self.内容閲覧.revoke();
            let log = match self
                .state_store
                .persistent_store
                .as_ref()
                .and_then(|s| s.verified_audit_log().ok())
            {
                Some(v) if v == self.audit_log => v,
                _ => return self.audit_store_failed_response(id, op, "監査失敗", "保管監査再確認"),
            };
            if self.append_audit(id,op,"received","Capability=対話部分破棄中断確認 Permission=指定未確定部分破棄一件 Approval=現在owner照合要求 RecoveryAction=保管監査再確認",EVIDENCE_SOURCE_INTERNAL_STATE,hash).is_err() {return self.audit_store_failed_response(id,op,"監査失敗","保管監査再確認");}
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Select {
                #[serde(rename = "要求ID")]
                id: String,
                #[serde(rename = "要求hash")]
                hash: String,
                #[serde(rename = "部分保存破棄承認監査ID")]
                approval: String,
            }
            let p: Select = match serde_json::from_value(payload.clone()) {
                Ok(v) => v,
                Err(_) => {
                    return self.reject_with_payload_hash(
                        id,
                        op,
                        "復旧対象拒否",
                        "入力形式が不正",
                        true,
                        hash,
                    )
                }
            };
            if p.approval.is_empty() || p.approval.len() > 256 {
                return self.reject_with_payload_hash(
                    id,
                    op,
                    "復旧対象拒否",
                    "部分破棄承認参照が不正",
                    true,
                    hash,
                );
            }
            let Some(store) = self.protected_store.as_ref() else {
                return self.reject_with_payload_hash(
                    id,
                    op,
                    "保管未登録",
                    "独立保管先の登録が必要",
                    true,
                    hash,
                );
            };
            let select = serde_json::json!({"要求ID":p.id,"要求hash":p.hash});
            let current = match super::content_inventory::observe(
                &log,
                store,
                &select,
                self.current_epoch_seconds(),
            ) {
                Ok(v) => v,
                Err(reason) => {
                    return self.reject_with_payload_hash(
                        id,
                        op,
                        "復旧照合失敗",
                        reason,
                        true,
                        hash,
                    )
                }
            };
            if current["状態"] != "部分保存破棄・結果未確定"
                || current["部分保存破棄承認監査ID"] != p.approval
            {
                return self.reject_with_payload_hash(
                    id,
                    op,
                    "復旧対象拒否",
                    "現在不在の未確定部分破棄と指定承認が一致しない",
                    true,
                    hash,
                );
            }
            let body = serde_json::json!({"版":1,"要求ID":p.id,"要求hash":p.hash,"部分保存破棄承認監査ID":p.approval,"観測監査head":current["観測監査head"],"観測時刻":current["観測時刻"],"状態":"部分破棄中断照合済み","証拠種別":"LIVE_RUNTIME"});
            self.部分破棄中断照合確定(id, body)
        }
    }
    #[cfg(windows)]
    pub(super) fn 部分破棄中断照合確定(&mut self, id: &str, body: Value) -> BrokerResponse {
        let op = "対話部分破棄中断確認";
        let encoded = body.to_string();
        match self.append_audit(
            id,
            op,
            "accepted",
            &format!("対話部分破棄中断照合記録:{encoded}"),
            "LIVE_RUNTIME",
            &sha256_tagged(encoded.as_bytes()),
        ) {
            Ok(v) => BrokerResponse {
                request_id: id.into(),
                operation: op.into(),
                status: BrokerStatus::Accepted,
                evidence_source: "LIVE_RUNTIME".into(),
                audit_event_id: v.event_id,
                error: None,
                health: None,
                body: Some(body),
                shutdown_requested: false,
            },
            Err(_) => self.audit_store_failed_response(
                id,
                op,
                "監査失敗",
                "復旧照合未確定。保管監査再確認",
            ),
        }
    }
}
