//! 保存完了を持たない、検証済み保存試行に限る現在owner破棄。
use super::*;
impl Broker {
    pub(super) fn 部分保存破棄処理(
        &mut self,
        id: &str,
        payload: &Value,
        owner: bool,
        hash: &str,
    ) -> BrokerResponse {
        let op = "対話部分保存破棄";
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
            if self.append_audit(id,op,"received","Capability=対話部分保存破棄 Permission=保存試行と暗号文hash一件 Approval=現在owner破棄 RecoveryAction=保管監査再確認",EVIDENCE_SOURCE_INTERNAL_STATE,hash).is_err() {return self.audit_store_failed_response(id,op,"監査失敗","保管監査再確認");}
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Select {
                #[serde(rename = "要求ID")]
                id: String,
                #[serde(rename = "要求hash")]
                hash: String,
                #[serde(rename = "保存試行監査ID")]
                attempt: String,
                #[serde(rename = "暗号文hash")]
                cipher: String,
            }
            let p: Select = match serde_json::from_value(payload.clone()) {
                Ok(v) => v,
                Err(_) => {
                    return self.reject_with_payload_hash(
                        id,
                        op,
                        "破棄対象拒否",
                        "入力形式が不正",
                        true,
                        hash,
                    )
                }
            };
            if p.attempt.is_empty() || p.attempt.len() > 256 {
                return self.reject_with_payload_hash(
                    id,
                    op,
                    "破棄対象拒否",
                    "保存試行参照が不正",
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
                Err(e) => {
                    return self.reject_with_payload_hash(id, op, "破棄対象拒否", e, true, hash)
                }
            };
            if !["保存記録なし・fileあり", "部分保存破棄承認あり・file残存"]
                .contains(&current["状態"].as_str().unwrap_or(""))
                || !current["保存監査ID"].is_null()
                || current["保存試行監査ID"] != p.attempt
                || current["暗号文hash"] != p.cipher
            {
                return self.reject_with_payload_hash(
                    id,
                    op,
                    "破棄対象拒否",
                    "現在の未確定保存試行とfile hashが一致しない",
                    true,
                    hash,
                );
            }
            let prepared = match store.prepare_delete(
                crate::protected_store::Purpose::History,
                &p.id,
                &p.cipher,
            ) {
                Ok(v) => v,
                Err(_) => {
                    return self.reject_with_payload_hash(
                        id,
                        op,
                        "破棄準備失敗",
                        "現在fileを変更せず保管監査再確認",
                        true,
                        hash,
                    )
                }
            };
            self.部分保存破棄実行(id, payload.clone(), prepared, hash)
        }
    }
    #[cfg(windows)]
    pub(super) fn 部分保存破棄実行(
        &mut self,
        id: &str,
        intent: Value,
        prepared: crate::protected_store::PreparedDelete,
        hash: &str,
    ) -> BrokerResponse {
        let op = "対話部分保存破棄";
        let encoded = intent.to_string();
        let approval = match self.append_audit(
            id,
            op,
            "recorded",
            &format!("対話部分保存破棄承認:{encoded}"),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &sha256_tagged(encoded.as_bytes()),
        ) {
            Ok(v) => v.event_id,
            Err(_) => {
                return self.audit_store_failed_response(
                    id,
                    op,
                    "監査失敗",
                    "fileを残して保管監査再確認",
                )
            }
        };
        if prepared.commit().is_err() {
            return self.reject_with_payload_hash(
                id,
                op,
                "破棄確定失敗",
                "再保存せず保管監査再確認",
                true,
                hash,
            );
        }
        let body = serde_json::json!({"版":1,"要求ID":intent["要求ID"],"要求hash":intent["要求hash"],"保存試行監査ID":intent["保存試行監査ID"],"暗号文hash":intent["暗号文hash"],"破棄承認監査ID":approval,"状態":"部分保存破棄確認","証拠種別":"INTERNAL_STATE"});
        self.部分保存破棄確定(id, body)
    }
    #[cfg(windows)]
    pub(super) fn 部分保存破棄確定(&mut self, id: &str, body: Value) -> BrokerResponse {
        let op = "対話部分保存破棄";
        let encoded = body.to_string();
        match self.append_audit(
            id,
            op,
            "accepted",
            &format!("対話部分保存破棄記録:{encoded}"),
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
                "破棄後の結果監査未確定。保管監査再確認",
            ),
        }
    }
}
#[cfg(windows)]
pub(super) fn blocked(
    log: &super::super::audit::BrokerAuditLog,
    target: &str,
) -> Result<bool, &'static str> {
    for e in log
        .events()
        .iter()
        .filter(|e| e.operation == "対話部分保存破棄" && e.decision == "recorded")
    {
        let raw = e
            .reason
            .strip_prefix("対話部分保存破棄承認:")
            .ok_or("破棄監査形式が不正")?;
        let v: Value =
            super::super::json_input::read_unique(raw).map_err(|_| "破棄監査形式が不正")?;
        if !exact(&v, &["要求ID", "要求hash", "保存試行監査ID", "暗号文hash"])
            || e.payload_hash != sha256_tagged(raw.as_bytes())
            || e.evidence_source != "INTERNAL_STATE"
        {
            return Err("破棄監査の構造/hashが不正");
        }
        if v["要求ID"] == target {
            return Ok(true);
        }
    }
    Ok(false)
}
#[cfg(windows)]
pub(super) fn markers(
    log: &super::super::audit::BrokerAuditLog,
    entry: &Value,
    attempt: Option<&str>,
) -> Result<(Option<String>, Option<String>), &'static str> {
    let target = &entry["record"]["実行記録"]["要求ID"];
    let mut intent: Option<(String, String, Value)> = None;
    let mut result = None;
    for (index, e) in log.events().iter().enumerate().filter(|(_, e)| {
        e.operation == "対話部分保存破棄" && matches!(e.decision.as_str(), "recorded" | "accepted")
    }) {
        let prefix = if e.decision == "recorded" {
            "対話部分保存破棄承認:"
        } else {
            "対話部分保存破棄記録:"
        };
        let raw = e
            .reason
            .strip_prefix(prefix)
            .ok_or("破棄監査の形式が不正")?;
        let v: Value =
            super::super::json_input::read_unique(raw).map_err(|_| "破棄監査の形式が不正")?;
        if e.payload_hash != sha256_tagged(raw.as_bytes()) || e.evidence_source != "INTERNAL_STATE"
        {
            return Err("破棄監査hashが不正");
        }
        if v["要求ID"] != *target {
            continue;
        }
        if !entry["content_receipt"].is_null()
            || v["要求hash"] != entry["result_evidence"]["要求hash"]
            || attempt.is_none()
            || v["保存試行監査ID"].as_str() != attempt
        {
            return Err("破棄対象と保存試行が不一致");
        }
        if !v["暗号文hash"]
            .as_str()
            .and_then(|s| s.strip_prefix("sha256:"))
            .is_some_and(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            })
        {
            return Err("破棄hashが不正");
        }
        let before = &log.events()[..index];
        if !before.iter().any(|e| Some(e.event_id.as_str()) == attempt) {
            return Err("破棄前の保存試行がない");
        }
        if e.decision == "recorded" {
            if !exact(&v, &["要求ID", "要求hash", "保存試行監査ID", "暗号文hash"]) {
                return Err("破棄承認の構造が不正");
            }
            if !before.iter().any(|r| {
                r.request_id == e.request_id
                    && r.operation == e.operation
                    && r.decision == "received"
                    && r.payload_hash == e.payload_hash
                    && r.evidence_source == "INTERNAL_STATE"
            }) {
                return Err("破棄承認前の受信が不一致");
            }
            intent = Some((e.event_id.clone(), e.request_id.clone(), v));
            result = None;
        } else {
            let previous = intent.as_ref().ok_or("破棄結果に先行承認がない")?;
            if !exact(
                &v,
                &[
                    "版",
                    "要求ID",
                    "要求hash",
                    "保存試行監査ID",
                    "暗号文hash",
                    "破棄承認監査ID",
                    "状態",
                    "証拠種別",
                ],
            ) || v["版"] != 1
                || v["状態"] != "部分保存破棄確認"
                || v["証拠種別"] != "INTERNAL_STATE"
                || v["破棄承認監査ID"] != previous.0
                || e.request_id != previous.1
                || v["暗号文hash"] != previous.2["暗号文hash"]
                || result.is_some()
            {
                return Err("破棄結果と承認の対応が不正");
            }
            result = Some(e.event_id.clone());
        }
    }
    Ok((intent.map(|v| v.0), result))
}
#[cfg(windows)]
fn exact(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|v| v.len() == keys.len() && keys.iter().all(|k| v.contains_key(*k)))
}
