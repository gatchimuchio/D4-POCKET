//! 保存済み一件の削除。owner承認監査を実file削除より前へ置く。
use super::*;

impl Broker {
    pub(super) fn 内容削除処理(
        &mut self,
        id: &str,
        payload: &Value,
        owner: bool,
        hash: &str,
    ) -> BrokerResponse {
        let op = "対話内容削除";
        if !owner || !self.state_store.persistence_ready() {
            return self.reject_with_payload_hash(
                id,
                op,
                "権限拒否",
                "owner制御資格と永続監査が必要",
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
            if self.append_audit(id,op,"received","Capability=対話内容削除 Permission=保存参照一件 Approval=現在owner操作 RecoveryAction=保管監査再確認",EVIDENCE_SOURCE_INTERNAL_STATE,hash).is_err() {
                return self.audit_store_failed_response(id,op,"監査失敗","保管監査再確認");
            }
            let entry = match super::super::content_access::deletion_entry(&log, payload) {
                Ok(v) => v,
                Err(reason) => {
                    return self.reject_with_payload_hash(
                        id,
                        op,
                        "削除対象拒否",
                        reason,
                        true,
                        hash,
                    )
                }
            };
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
            let target = entry["record"]["実行記録"]["要求ID"]
                .as_str()
                .expect("検証済み要求ID");
            let cipher = entry["content_receipt"]["receipt"]["暗号文hash"]
                .as_str()
                .expect("検証済み暗号文hash");
            let prepared = match store.prepare_delete(
                crate::protected_store::Purpose::History,
                target,
                cipher,
            ) {
                Ok(v) => v,
                Err(_) => {
                    return self.reject_with_payload_hash(
                        id,
                        op,
                        "削除準備失敗",
                        "対象を変更せず保管監査再確認",
                        true,
                        hash,
                    )
                }
            };
            let intent = serde_json::json!({"要求ID":target,"保存監査ID":payload["保存監査ID"],"保存監査hash":payload["保存監査hash"],"暗号文hash":cipher});
            self.内容削除実行(id, intent, prepared, hash)
        }
    }
    #[cfg(windows)]
    pub(super) fn 内容削除実行(
        &mut self,
        id: &str,
        intent: Value,
        prepared: crate::protected_store::PreparedDelete,
        hash: &str,
    ) -> BrokerResponse {
        let op = "対話内容削除";
        let encoded = intent.to_string();
        let approval = match self.append_audit(
            id,
            op,
            "recorded",
            &format!("対話内容削除承認:{encoded}"),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &sha256_tagged(encoded.as_bytes()),
        ) {
            Ok(v) => v.event_id,
            Err(_) => {
                return self.audit_store_failed_response(
                    id,
                    op,
                    "監査失敗",
                    "削除を確定せず保管監査再確認",
                )
            }
        };
        if prepared.commit().is_err() {
            return self.reject_with_payload_hash(
                id,
                op,
                "削除確定失敗",
                "内容閲覧を停止して保管監査再確認",
                true,
                hash,
            );
        }
        let body = serde_json::json!({"版":1,"要求ID":intent["要求ID"],"保存監査ID":intent["保存監査ID"],"保存監査hash":intent["保存監査hash"],"暗号文hash":intent["暗号文hash"],"削除承認監査ID":approval,"状態":"削除確定","証拠種別":"INTERNAL_STATE"});
        self.内容削除確定(id, body)
    }
    #[cfg(windows)]
    pub(super) fn 内容削除確定(&mut self, id: &str, body: Value) -> BrokerResponse {
        let op = "対話内容削除";
        let encoded = body.to_string();
        match self.append_audit(
            id,
            op,
            "accepted",
            &format!("対話内容削除記録:{encoded}"),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &sha256_tagged(encoded.as_bytes()),
        ) {
            Ok(v) => BrokerResponse {
                request_id: id.into(),
                operation: op.into(),
                status: BrokerStatus::Accepted,
                evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.into(),
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
                "削除後の結果監査未確定。保管監査再確認",
            ),
        }
    }
}
