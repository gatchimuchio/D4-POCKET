//! 検証済み監査から過去の状態遷移だけを取り出す。実行権限を所有しない。
use super::audit::BrokerAuditLog;
use crate::audit_hash::sha256_tagged;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Query {
    pub after: usize,
    pub limit: usize,
}

pub(crate) fn page(log: &BrokerAuditLog, query: Query) -> Result<Value, &'static str> {
    let events = log.events();
    if query.limit == 0 || query.limit > 100 || query.after > events.len() {
        return Err("履歴範囲が不正");
    }
    let mut entries = Vec::new();
    let mut next = query.after;
    let mut more = false;
    for (index, event) in events.iter().enumerate().skip(query.after) {
        let Some(body) = event.reason.strip_prefix("対話実行記録:") else {
            next = index + 1;
            continue;
        };
        if entries.len() == query.limit {
            more = true;
            break;
        }
        if event.decision != "recorded"
            || event.evidence_source != "INTERNAL_STATE"
            || ![
                "実行系列挙",
                "対話開始",
                "対話送信",
                "対話取得",
                "対話中止",
                "対話終了",
                "対話承認",
                "対話承認待ち",
            ]
            .contains(&event.operation.as_str())
            || body.len() > 16384
            || sha256_tagged(body.as_bytes()) != event.payload_hash
        {
            return Err("履歴hashが不正");
        }
        let value: Value = super::json_input::read_unique(body).map_err(|_| "履歴形式が不正")?;
        let object = value.as_object().ok_or("履歴形式が不正")?;
        if object.len() != 4
            || ["版", "状態", "失敗分類", "実行記録"]
                .iter()
                .any(|k| !object.contains_key(*k))
            || value["版"] != 1
            || !["承認待ち", "実行中", "成功", "保留", "失敗", "中止"]
                .contains(&value["状態"].as_str().unwrap_or(""))
            || !(value["失敗分類"].is_null()
                || [
                    "要求不正",
                    "実行系不在",
                    "権限拒否",
                    "セッション不一致",
                    "通信失敗",
                    "期限超過",
                    "応答不正",
                    "監査失敗",
                    "取消",
                ]
                .contains(&value["失敗分類"].as_str().unwrap_or("")))
        {
            return Err("履歴形式が不正");
        }
        let record = value["実行記録"].as_object().ok_or("実行記録が不正")?;
        let keys = [
            "要求ID",
            "実行系ID",
            "対話セッションID",
            "作成時刻",
            "開始時刻",
            "終了時刻",
            "作成監査ID",
            "開始監査ID",
            "終了監査ID",
        ];
        if record.len() != keys.len()
            || keys.iter().any(|k| !record.contains_key(*k))
            || record["要求ID"] != event.request_id
        {
            return Err("履歴の要求対応が不正");
        }
        let started = record["開始時刻"].is_i64();
        let ended = record["終了時刻"].is_i64();
        let failed = value["失敗分類"].as_str();
        let consistent = match value["状態"].as_str() {
            Some("承認待ち") => !started && !ended && failed.is_none(),
            Some("実行中") => started && !ended && failed.is_none(),
            Some("成功" | "保留") => started && ended && failed.is_none(),
            Some("失敗") => ended && failed.is_some() && failed != Some("取消"),
            Some("中止") => failed == Some("取消"),
            _ => false,
        };
        if !consistent {
            return Err("履歴状態と実行記録が矛盾");
        }
        for key in ["要求ID", "対話セッションID"] {
            let id = record[key].as_str().ok_or("履歴IDが不正")?;
            if id.len() != 32
                || !id
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            {
                return Err("履歴IDが不正");
            }
        }
        let runtime = record["実行系ID"].as_str().ok_or("履歴の実行系が不正")?;
        if runtime.is_empty()
            || runtime.len() > 128
            || !runtime.as_bytes()[0].is_ascii_alphanumeric()
            || !runtime
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
        {
            return Err("履歴の実行系が不正");
        }
        for (stage, reasons) in [
            ("作成", &["対話承認待ち作成"][..]),
            ("開始", &["対話送信承認"][..]),
            (
                "終了",
                &["対話完了", "対話中止", "対話期限超過", "対話worker起動失敗"][..],
            ),
        ] {
            let time = &record[&format!("{stage}時刻")];
            let audit = &record[&format!("{stage}監査ID")];
            if stage != "作成" && time.is_null() && audit.is_null() {
                continue;
            }
            if !time.is_i64() {
                return Err("履歴時刻が不正");
            }
            let id = audit.as_str().ok_or("履歴監査参照が不正")?;
            let referenced = events[..index]
                .iter()
                .find(|v| v.event_id == id)
                .ok_or("履歴監査参照が不在")?;
            if referenced.request_id != event.request_id
                || referenced.decision != "recorded"
                || referenced.evidence_source != "INTERNAL_STATE"
                || !reasons.iter().any(|r| referenced.reason.starts_with(r))
            {
                return Err("履歴監査参照が不一致");
            }
        }
        entries.push(
            json!({"audit_event_id":event.event_id,"event_hash":event.event_hash,"record":value}),
        );
        next = index + 1;
    }
    Ok(
        json!({"version":1,"entries":entries,"next_cursor":next,"has_more":more,"head_hash":events.last().map(|v| &v.event_hash)}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 状態と開始終了と失敗分類の関係を検査する() {
        let cases = [
            ("承認待ち", false, false, None, true),
            ("承認待ち", true, false, None, false),
            ("実行中", true, false, None, true),
            ("実行中", true, true, None, false),
            ("成功", true, true, None, true),
            ("成功", true, false, None, false),
            ("成功", true, true, Some("通信失敗"), false),
            ("保留", true, true, None, true),
            ("保留", false, false, None, false),
            ("失敗", false, true, Some("期限超過"), true),
            ("失敗", true, true, Some("通信失敗"), true),
            ("失敗", true, true, None, false),
            ("失敗", true, false, Some("通信失敗"), false),
            ("失敗", true, true, Some("取消"), false),
            ("中止", false, false, Some("取消"), true),
            ("中止", true, true, Some("取消"), true),
            ("中止", true, true, None, false),
        ];
        for (state, started, ended, failure, expected) in cases {
            let mut log = BrokerAuditLog::default();
            let request = "a".repeat(32);
            let created = log.append(
                &request,
                "対話送信",
                "recorded",
                "対話承認待ち作成",
                "INTERNAL_STATE",
                &sha256_tagged(b"input"),
            );
            let start = started.then(|| {
                log.append(
                    &request,
                    "対話承認",
                    "recorded",
                    "対話送信承認",
                    "INTERNAL_STATE",
                    &sha256_tagged(b"input"),
                )
                .event_id
            });
            let end = ended.then(|| {
                log.append(
                    &request,
                    "対話取得",
                    "recorded",
                    "対話完了",
                    "INTERNAL_STATE",
                    &sha256_tagged(b"result"),
                )
                .event_id
            });
            let record = json!({"版":1,"状態":state,"失敗分類":failure,"実行記録":{
                "要求ID":request,"実行系ID":"local","対話セッションID":"b".repeat(32),"作成時刻":100,
                "開始時刻":started.then_some(110),"終了時刻":ended.then_some(120),"作成監査ID":created.event_id,"開始監査ID":start,"終了監査ID":end}});
            let body = record.to_string();
            log.append(
                &request,
                "対話取得",
                "recorded",
                &format!("対話実行記録:{body}"),
                "INTERNAL_STATE",
                &sha256_tagged(body.as_bytes()),
            );
            assert_eq!(
                page(
                    &log,
                    Query {
                        after: 0,
                        limit: 100
                    }
                )
                .is_ok(),
                expected,
                "{state} 開始={started} 終了={ended} 失敗={failure:?}"
            );
        }
    }

    #[test]
    fn chain内でも不正形式と別要求参照を履歴として返さない() {
        for case in 0..10 {
            let mut log = BrokerAuditLog::default();
            let request = "a".repeat(32);
            let created = log.append(
                &request,
                "対話送信",
                "recorded",
                "対話承認待ち作成",
                "INTERNAL_STATE",
                &sha256_tagged(b"input"),
            );
            let mut record = json!({"版":1,"状態":"承認待ち","失敗分類":null,"実行記録":{
                "要求ID":request,"実行系ID":"local","対話セッションID":"b".repeat(32),"作成時刻":100,
                "開始時刻":null,"終了時刻":null,"作成監査ID":created.event_id,"開始監査ID":null,"終了監査ID":null}});
            match case {
                1 => record["実行記録"]["要求ID"] = json!("c".repeat(32)),
                2 => record["実行記録"]["作成監査ID"] = json!("absent"),
                3 => {
                    record.as_object_mut().unwrap().remove("失敗分類");
                    record["unknown"] = Value::Null;
                }
                4 => record["実行記録"]["開始監査ID"] = json!(created.event_id),
                5 => record["版"] = json!(2),
                9 => record["状態"] = json!("成功"),
                _ => (),
            }
            let body = if case == 8 {
                record.to_string().replacen('{', "{\"版\":1,", 1)
            } else {
                record.to_string()
            };
            log.append(
                &request,
                if case == 7 { "other" } else { "対話送信" },
                "recorded",
                &format!("対話実行記録:{body}"),
                "INTERNAL_STATE",
                &sha256_tagged(if case == 6 { b"wrong" } else { body.as_bytes() }),
            );
            let result = page(&log, Query { after: 0, limit: 1 });
            assert_eq!(result.is_ok(), case == 0);
            if case == 0 {
                let value = result.unwrap();
                assert_eq!(value["entries"].as_array().unwrap().len(), 1);
                assert_eq!(value["next_cursor"], 2);
                assert_eq!(value["has_more"], false);
                assert_eq!(
                    page(&log, Query { after: 2, limit: 1 }).unwrap()["entries"],
                    json!([])
                );
            }
        }
    }
}
