//! 監査と現在fileの照合。結果は観測であり操作承認ではない。
use super::*;

impl Broker {
    pub(super) fn 保管状態処理(
        &mut self,
        id: &str,
        payload: &Value,
        owner: bool,
        hash: &str,
    ) -> BrokerResponse {
        let op = "対話保管状態";
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
            let log = match self
                .state_store
                .persistent_store
                .as_ref()
                .and_then(|s| s.verified_audit_log().ok())
            {
                Some(v) if v == self.audit_log => v,
                _ => return self.audit_store_failed_response(id, op, "監査失敗", "保管監査再確認"),
            };
            if self.append_audit(id,op,"received","Capability=対話保管状態 Permission=指定要求の独立保管先 Approval=現在owner照会 RecoveryAction=保管監査再確認",EVIDENCE_SOURCE_INTERNAL_STATE,hash).is_err() {
                return self.audit_store_failed_response(id,op,"監査失敗","保管監査再確認");
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
            match observe(&log, store, payload, self.current_epoch_seconds()) {
                Ok(body) => self.保管状態確定(id, body),
                Err(reason) => {
                    self.reject_with_payload_hash(id, op, "保管状態確認失敗", reason, true, hash)
                }
            }
        }
    }
    #[cfg(windows)]
    pub(super) fn 保管状態確定(&mut self, id: &str, body: Value) -> BrokerResponse {
        let op = "対話保管状態";
        match self.append_audit(
            id,
            op,
            "accepted",
            "現在fileと監査の照合。現在の操作権限を生成しない",
            "LIVE_RUNTIME",
            &sha256_tagged(body.to_string().as_bytes()),
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
            Err(_) => self.audit_store_failed_response(id, op, "監査失敗", "保管監査再確認"),
        }
    }
}

#[cfg(windows)]
pub(super) fn observe(
    log: &super::super::audit::BrokerAuditLog,
    store: &crate::protected_store::ProtectedStore,
    payload: &Value,
    now: i64,
) -> Result<Value, &'static str> {
    use super::super::execution_history::{page, Query};
    use serde::Deserialize;
    use serde_json::json;
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Select {
        #[serde(rename = "要求ID")]
        id: String,
        #[serde(rename = "要求hash")]
        hash: String,
    }
    let p: Select = serde_json::from_value(payload.clone()).map_err(|_| "照会対象形式が不正")?;
    if now < 0 {
        return Err("観測時刻が不正");
    }
    let page = page(
        log,
        Query {
            after: 0,
            limit: 1,
            latest_per_request: true,
            include_result_evidence: true,
            include_content_receipt: true,
            filter: [("要求ID".into(), p.id.clone())].into(),
            ..Default::default()
        },
    )?;
    let entry = page["entries"]
        .as_array()
        .and_then(|v| v.first())
        .ok_or("要求が不在")?;
    if entry["result_evidence"]["要求hash"] != p.hash
        || entry["result_evidence"]["表示範囲"] != "full"
    {
        return Err("全文結果証跡と要求hashが不一致");
    }
    let saved = &entry["content_receipt"];
    let attempt =
        super::super::execution_history::latest_save_attempt(log, &entry["record"], &p.hash)?;
    let (discard_intent, discard_result, discard_reconciled) =
        super::content_discard::markers(log, entry, attempt.as_deref())?;

    let mut intent: Option<(String, String, Value)> = None;
    let mut committed: Option<String> = None;
    let mut reconciled: Option<String> = None;
    for event in log.events().iter().filter(|e| {
        (e.operation == "対話内容削除" && matches!(e.decision.as_str(), "recorded" | "accepted"))
            || (e.operation == "対話削除中断確認" && e.decision == "accepted")
    }) {
        if event.operation == "対話削除中断確認" {
            let raw = event
                .reason
                .strip_prefix("対話削除中断照合記録:")
                .ok_or("復旧照合の形式が不正")?;
            let v: Value =
                super::super::json_input::read_unique(raw).map_err(|_| "復旧照合の形式が不正")?;
            if event.payload_hash != sha256_tagged(raw.as_bytes())
                || event.evidence_source != "LIVE_RUNTIME"
            {
                return Err("復旧照合のhashが不正");
            }
            if v["要求ID"] != p.id {
                continue;
            }
            if !exact(
                &v,
                &[
                    "版",
                    "要求ID",
                    "要求hash",
                    "削除承認監査ID",
                    "観測監査head",
                    "観測時刻",
                    "状態",
                    "証拠種別",
                ],
            ) || v["版"] != 1
                || v["要求hash"] != p.hash
                || v["状態"] != "削除中断照合済み"
                || v["証拠種別"] != "LIVE_RUNTIME"
                || v["観測時刻"].as_u64().is_none()
                || committed.is_some()
                || reconciled.is_some()
                || !intent.as_ref().is_some_and(|i| v["削除承認監査ID"] == i.0)
            {
                return Err("復旧照合と未確定削除の対応が不正");
            }
            let before: Vec<_> = log
                .events()
                .iter()
                .take_while(|e| e.event_id != event.event_id)
                .collect();
            if !before.iter().any(|e| v["観測監査head"] == e.event_hash) {
                return Err("復旧観測の監査headが不正");
            }
            let request =
                json!({"要求ID":p.id,"要求hash":p.hash,"削除承認監査ID":v["削除承認監査ID"]});
            let hash = sha256_tagged(request.to_string().as_bytes());
            if !before.iter().any(|e| {
                e.operation == event.operation
                    && e.request_id == event.request_id
                    && e.decision == "received"
                    && e.payload_hash == hash
                    && e.evidence_source == "INTERNAL_STATE"
            }) {
                return Err("復旧照合の受信対応が不正");
            }
            reconciled = Some(event.event_id.clone());
            continue;
        }
        let prefix = if event.decision == "recorded" {
            "対話内容削除承認:"
        } else {
            "対話内容削除記録:"
        };
        let raw = event
            .reason
            .strip_prefix(prefix)
            .ok_or("削除監査の形式が不正")?;
        let value: Value =
            super::super::json_input::read_unique(raw).map_err(|_| "削除監査の形式が不正")?;
        if event.payload_hash != sha256_tagged(raw.as_bytes())
            || event.evidence_source != "INTERNAL_STATE"
        {
            return Err("削除監査のhashが不正");
        }
        if value["要求ID"] != p.id {
            continue;
        }
        if !log
            .events()
            .iter()
            .take_while(|e| e.event_id != event.event_id)
            .any(|e| saved["audit_event_id"] == e.event_id && saved["event_hash"] == e.event_hash)
        {
            return Err("削除監査より前の保存記録がない");
        }
        if saved.is_null()
            || value["保存監査ID"] != saved["audit_event_id"]
            || value["保存監査hash"] != saved["event_hash"]
            || value["暗号文hash"] != saved["receipt"]["暗号文hash"]
        {
            return Err("削除監査の保存参照が不一致");
        }
        if event.decision == "recorded" {
            if !exact(
                &value,
                &["要求ID", "保存監査ID", "保存監査hash", "暗号文hash"],
            ) {
                return Err("削除承認の構造が不正");
            }
            let request = json!({"要求ID":p.id,"保存監査ID":value["保存監査ID"],"保存監査hash":value["保存監査hash"]});
            let request_hash = sha256_tagged(request.to_string().as_bytes());
            if !log
                .events()
                .iter()
                .take_while(|e| e.event_id != event.event_id)
                .any(|e| {
                    e.operation == event.operation
                        && e.request_id == event.request_id
                        && e.decision == "received"
                        && e.payload_hash == request_hash
                        && e.evidence_source == "INTERNAL_STATE"
                })
            {
                return Err("削除受信と承認の対応が不正");
            }
            intent = Some((event.event_id.clone(), event.request_id.clone(), value));
            committed = None;
            reconciled = None;
        } else {
            if !exact(
                &value,
                &[
                    "版",
                    "要求ID",
                    "保存監査ID",
                    "保存監査hash",
                    "暗号文hash",
                    "削除承認監査ID",
                    "状態",
                    "証拠種別",
                ],
            ) || value["版"] != 1
                || value["状態"] != "削除確定"
                || value["証拠種別"] != "INTERNAL_STATE"
            {
                return Err("削除結果の構造が不正");
            }
            let prior = intent.as_ref().ok_or("削除結果に先行承認がない")?;
            if value["削除承認監査ID"] != prior.0
                || event.request_id != prior.1
                || committed.is_some()
            {
                return Err("削除結果と承認が不一致");
            }
            committed = Some(event.event_id.clone());
        }
    }
    let file = store
        .inspect(crate::protected_store::Purpose::History, &p.id)
        .map_err(|_| "file観測失敗。欠落と推定しない")?;
    let state = if discard_intent.is_some() {
        if file.is_some() {
            "部分保存破棄承認あり・file残存"
        } else if discard_result.is_some() {
            "部分保存破棄済み・file不在"
        } else if discard_reconciled.is_some() {
            "部分破棄中断・復旧照合済み"
        } else {
            "部分保存破棄・結果未確定"
        }
    } else if intent.is_some() {
        if file.is_some() {
            "削除承認あり・file残存"
        } else if committed.is_some() {
            "削除確定・file不在"
        } else if reconciled.is_some() {
            "削除中断・復旧照合済み"
        } else {
            "削除承認あり・file不在・結果未確定"
        }
    } else if saved.is_null() {
        if file.is_some() {
            "保存記録なし・fileあり"
        } else {
            "未保存・file不在"
        }
    } else {
        match &file {
            None => "保存記録あり・file欠落",
            Some((hash, _)) if saved["receipt"]["暗号文hash"] == *hash => "保存済み・hash一致",
            _ => "保存記録あり・hash不一致",
        }
    };
    Ok(
        json!({"版":1,"要求ID":p.id,"要求hash":p.hash,"観測監査head":page["head_hash"],"観測時刻":now,"状態":state,"file存在":file.is_some(),"暗号文hash":file.as_ref().map(|v|&v.0),"bytes":file.as_ref().map(|v|v.1),"保存監査ID":saved["audit_event_id"],"削除承認監査ID":intent.as_ref().map(|v|&v.0),"削除結果監査ID":committed,"復旧照合監査ID":reconciled,"保存試行監査ID":attempt,"部分保存破棄承認監査ID":discard_intent,"部分保存破棄結果監査ID":discard_result,"部分破棄復旧照合監査ID":discard_reconciled,"証拠種別":"LIVE_RUNTIME"}),
    )
}
#[cfg(windows)]
fn exact(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|v| v.len() == keys.len() && keys.iter().all(|k| v.contains_key(*k)))
}
