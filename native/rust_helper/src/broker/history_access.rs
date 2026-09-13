//! 現在のowner承認に限定した履歴metadata閲覧。再起動時に復元しない。
use super::{
    audit::BrokerAuditLog,
    dialogue::識別子生成,
    execution_history::{page, Query},
};
use crate::audit_hash::sha256_tagged;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub(crate) struct HistoryAccess {
    grant: Option<Grant>,
    last_now: Option<i64>,
}
#[derive(Debug)]
struct Grant {
    id: String,
    runtime: String,
    expires: i64,
    deadline: Instant,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Approval {
    #[serde(rename = "実行系ID")]
    runtime: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    approval_id: String,
    query: Query,
}

impl HistoryAccess {
    fn refresh(&mut self, now: i64) {
        if self.last_now.is_some_and(|last| now < last)
            || self
                .grant
                .as_ref()
                .is_some_and(|g| now >= g.expires || Instant::now() >= g.deadline)
        {
            self.grant = None;
        }
        self.last_now = Some(self.last_now.unwrap_or(now).max(now));
    }
    pub(crate) fn current(&mut self, body: &Value, now: i64) -> bool {
        self.refresh(now);
        self.grant
            .as_ref()
            .is_some_and(|g| body["grant"]["approval_id"] == g.id)
    }
    pub(crate) fn operate(
        &mut self,
        op: &str,
        payload: &Value,
        owner: bool,
        now: i64,
        log: &BrokerAuditLog,
        audit: &mut dyn FnMut(&str, &str) -> Result<(), &'static str>,
    ) -> Result<Value, &'static str> {
        self.refresh(now);
        if matches!(op, "対話履歴承認" | "対話履歴失効") && !owner {
            return Err("owner制御資格が必要");
        }
        let mut result_page = Value::Null;
        match op {
            "対話履歴承認" => {
                let p: Approval =
                    serde_json::from_value(payload.clone()).map_err(|_| "履歴承認形式が不正")?;
                if p.runtime.is_empty()
                    || p.runtime.len() > 128
                    || !p.runtime.as_bytes()[0].is_ascii_alphanumeric()
                    || !p
                        .runtime
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
                {
                    return Err("実行系IDが不正");
                }
                if self.last_now.is_some_and(|last| now < last) {
                    return Err("時計後退中は再承認できない");
                }
                self.grant = None;
                let id = 識別子生成().map_err(|_| "承認ID生成失敗")?;
                let expires = now.checked_add(300).ok_or("承認期限が不正")?;
                let body = json!({"approval_id":id,"runtime_id":p.runtime,"expires_at":expires});
                audit("Capability=dialogue.history.inspect Permission=実行系metadata Approval=owner要求 Recovery=再承認", &sha256_tagged(body.to_string().as_bytes()))?;
                self.grant = Some(Grant {
                    id,
                    runtime: p.runtime,
                    expires,
                    deadline: Instant::now() + Duration::from_secs(300),
                });
            }
            "対話履歴失効" | "対話履歴閲覧状態" => {
                if !payload.as_object().is_some_and(|v| v.is_empty()) {
                    return Err("履歴操作の入力が不正");
                }
                if op == "対話履歴失効" {
                    self.grant = None;
                    audit(
                        "履歴metadataのPermissionとApprovalを失効",
                        &sha256_tagged(payload.to_string().as_bytes()),
                    )?;
                }
            }
            "対話履歴閲覧" => {
                let p: Read =
                    serde_json::from_value(payload.clone()).map_err(|_| "履歴閲覧形式が不正")?;
                let g = self.grant.as_ref().ok_or("現在の履歴閲覧承認が必要")?;
                if p.approval_id != g.id || p.query.filter.get("実行系ID") != Some(&g.runtime) {
                    return Err("履歴閲覧の承認または実行系が不一致");
                }
                result_page = page(log, p.query)?;
            }
            _ => return Err("未知の履歴閲覧操作"),
        }
        let grant = self
            .grant
            .as_ref()
            .map(|g| json!({"approval_id":g.id,"runtime_id":g.runtime,"expires_at":g.expires}));
        Ok(json!({"grant":grant,"page":result_page}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn 期限と時計後退と監査失敗は閲覧を復活させない() {
        let mut access = HistoryAccess::default();
        let log = BrokerAuditLog::default();
        let mut ok = |_: &str, _: &str| Ok(());
        assert!(access
            .operate(
                "対話履歴承認",
                &json!({"実行系ID":"local"}),
                false,
                100,
                &log,
                &mut ok
            )
            .is_err());
        let a = access
            .operate(
                "対話履歴承認",
                &json!({"実行系ID":"local"}),
                true,
                100,
                &log,
                &mut ok,
            )
            .unwrap();
        assert!(access.current(&a, 399));
        assert!(!access.current(&a, 400));
        assert!(!access.current(&a, 100));
        assert!(access
            .operate(
                "対話履歴承認",
                &json!({"実行系ID":"local"}),
                true,
                100,
                &log,
                &mut ok
            )
            .is_err());
        let b = access
            .operate(
                "対話履歴承認",
                &json!({"実行系ID":"local"}),
                true,
                401,
                &log,
                &mut ok,
            )
            .unwrap();
        assert!(!access.current(&a, 401));
        assert!(access.current(&b, 401));
        access.grant.as_mut().unwrap().deadline = Instant::now();
        assert!(!access.current(&b, 401));
        assert!(access
            .operate(
                "対話履歴承認",
                &json!({"実行系ID":"local"}),
                true,
                402,
                &log,
                &mut |_, _| Err("監査失敗")
            )
            .is_err());
        assert!(access.grant.is_none());
        let c = access
            .operate(
                "対話履歴承認",
                &json!({"実行系ID":"local"}),
                true,
                403,
                &log,
                &mut ok,
            )
            .unwrap();
        assert!(access
            .operate(
                "対話履歴失効",
                &json!({}),
                true,
                403,
                &log,
                &mut |_, _| Err("監査失敗")
            )
            .is_err());
        assert!(!access.current(&c, 403));
    }
}
