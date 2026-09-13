//! 検証済み監査から過去の状態遷移だけを取り出す。実行権限を所有しない。
use super::audit::BrokerAuditLog;
use crate::audit_hash::sha256_tagged;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct Query {
    pub after: usize,
    pub limit: usize,
    #[serde(default)]
    pub latest_per_request: bool,
    #[serde(default)]
    pub include_audit_context: bool,
    #[serde(default)]
    pub include_result_evidence: bool,
    #[serde(default)]
    pub filter: std::collections::BTreeMap<String, String>,
}

pub(crate) fn page(log: &BrokerAuditLog, query: Query) -> Result<Value, &'static str> {
    let events = log.events();
    if query.limit == 0 || query.limit > 100 || query.after > events.len() {
        return Err("履歴範囲が不正");
    }
    for (key, value) in &query.filter {
        let valid = match key.as_str() {
            "要求ID" | "対話セッションID" => {
                value.len() == 32
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            }
            "実行系ID" => {
                !value.is_empty()
                    && value.len() <= 128
                    && value.as_bytes()[0].is_ascii_alphanumeric()
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            }
            "状態" => {
                ["承認待ち", "実行中", "成功", "保留", "失敗", "中止"].contains(&value.as_str())
            }
            _ => false,
        };
        if !valid {
            return Err("履歴検索条件が不正");
        }
    }
    let mut latest = std::collections::HashMap::new();
    if query.latest_per_request {
        for (index, event) in events.iter().enumerate() {
            if event.reason.starts_with("対話実行記録:") {
                latest.insert(event.request_id.as_str(), index);
            }
        }
    }
    let mut entries = Vec::new();
    let mut next = query.after;
    let mut more = false;
    for (index, event) in events.iter().enumerate().skip(query.after) {
        let Some(body) = event.reason.strip_prefix("対話実行記録:") else {
            next = index + 1;
            continue;
        };
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
        if query.latest_per_request && latest.get(event.request_id.as_str()) != Some(&index) {
            next = index + 1;
            continue;
        }
        if !query.filter.iter().all(|(key, expected)| {
            let actual = if key == "状態" {
                &value["状態"]
            } else {
                &record[key]
            };
            actual.as_str() == Some(expected.as_str())
        }) {
            next = index + 1;
            continue;
        }
        if entries.len() == query.limit {
            more = true;
            break;
        }
        let mut entry =
            json!({"audit_event_id":event.event_id,"event_hash":event.event_hash,"record":value});
        if query.include_audit_context {
            entry["audit_context"] = audit_context(&events[..index], &entry["record"]["実行記録"])?;
        }
        if query.include_result_evidence {
            entry["result_evidence"] = result_evidence(&events[..index], &entry["record"])?;
        }
        entries.push(entry);
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
    fn 要求集約は最後の観測を選んでから検索しページを継続する() {
        let mut log = BrokerAuditLog::default();
        let mut created = std::collections::HashMap::new();
        for (key, finished) in [('a', false), ('b', false), ('a', true), ('b', true)] {
            let request = key.to_string().repeat(32);
            let id = created.entry(key).or_insert_with(|| {
                log.append(
                    &request,
                    "対話送信",
                    "recorded",
                    "対話承認待ち作成",
                    "INTERNAL_STATE",
                    &sha256_tagged(b"input"),
                )
                .event_id
            });
            let body=json!({"版":1,"状態":if finished{"中止"}else{"承認待ち"},"失敗分類":if finished{Some("取消")}else{None},"実行記録":{
                "要求ID":request,"実行系ID":"local","対話セッションID":"c".repeat(32),"作成時刻":100,"開始時刻":null,"終了時刻":null,"作成監査ID":id,"開始監査ID":null,"終了監査ID":null}}).to_string();
            log.append(
                &request,
                "対話中止",
                "recorded",
                &format!("対話実行記録:{body}"),
                "INTERNAL_STATE",
                &sha256_tagged(body.as_bytes()),
            );
        }
        assert_eq!(
            page(
                &log,
                Query {
                    limit: 100,
                    ..Default::default()
                }
            )
            .unwrap()["entries"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
        let first = page(
            &log,
            Query {
                limit: 1,
                latest_per_request: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            first["entries"][0]["record"]["実行記録"]["要求ID"],
            "a".repeat(32)
        );
        assert_eq!(first["entries"][0]["record"]["状態"], "中止");
        assert_eq!(first["has_more"], true);
        let second = page(
            &log,
            Query {
                after: first["next_cursor"].as_u64().unwrap() as usize,
                limit: 1,
                latest_per_request: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            second["entries"][0]["record"]["実行記録"]["要求ID"],
            "b".repeat(32)
        );
        assert_eq!(second["has_more"], false);
        assert_eq!(
            page(
                &log,
                Query {
                    limit: 100,
                    latest_per_request: true,
                    filter: [("状態".into(), "承認待ち".into())].into(),
                    ..Default::default()
                }
            )
            .unwrap()["entries"],
            json!([])
        );
    }

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
                        limit: 100,
                        ..Default::default()
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
            let result = page(
                &log,
                Query {
                    after: 0,
                    limit: 1,
                    ..Default::default()
                },
            );
            assert_eq!(result.is_ok(), case == 0);
            let excluded = page(
                &log,
                Query {
                    after: 0,
                    limit: 1,
                    filter: [("状態".into(), "失敗".into())].into(),
                    ..Default::default()
                },
            );
            assert_eq!(excluded.is_ok(), case == 0);
            if case == 0 {
                assert_eq!(excluded.unwrap()["entries"], json!([]));
            }

            if case == 0 {
                let value = result.unwrap();
                assert_eq!(value["entries"].as_array().unwrap().len(), 1);
                assert_eq!(value["next_cursor"], 2);
                assert_eq!(value["has_more"], false);
                assert_eq!(
                    page(
                        &log,
                        Query {
                            after: 2,
                            limit: 1,
                            ..Default::default()
                        }
                    )
                    .unwrap()["entries"],
                    json!([])
                );
            }
        }
    }
}

fn audit_context(
    events: &[super::audit::BrokerAuditEvent],
    record: &Value,
) -> Result<Value, &'static str> {
    let find = |key: &str| {
        events
            .iter()
            .find(|e| Some(e.event_id.as_str()) == record[key].as_str())
            .ok_or("監査文脈の参照が不在")
    };
    let created = find("作成監査ID")?;
    if created.operation != "対話送信" || created.reason != "対話承認待ち作成" {
        return Err("監査文脈の作成経路が不正");
    }
    let mut approval = Value::Null;
    let mut capabilities = Vec::<&str>::new();
    let mut recovery = None;
    if !record["開始監査ID"].is_null() {
        let start = find("開始監査ID")?;
        if start.operation != "対話承認" || start.payload_hash != created.payload_hash {
            return Err("過去承認の要求hashが不一致");
        }
        let prefix = format!(
            "対話送信承認 Capability=対話送信 Permission={}:",
            record["実行系ID"].as_str().ok_or("実行系が不正")?
        );
        let rest = start
            .reason
            .strip_prefix(&prefix)
            .ok_or("過去承認の実行系が不一致")?;
        let (endpoint, authority) = rest
            .split_once(" Approval=")
            .ok_or("過去承認の理由が不正")?;
        if endpoint.is_empty() || endpoint.len() > 256 || endpoint.chars().any(char::is_whitespace)
        {
            return Err("過去承認の接続範囲が不正");
        }
        let scope = authority
            .strip_prefix(&format!("{} 表示範囲=", created.payload_hash))
            .and_then(|s| s.strip_suffix(" RecoveryAction=接続再確認"))
            .ok_or("過去承認の対応が不正")?;
        if !["none", "hash_only", "summary", "redacted", "full"].contains(&scope) {
            return Err("過去承認の表示範囲が不正");
        }
        approval = json!({"監査ID":start.event_id,"操作":start.operation,"要求hash":start.payload_hash,"内容表示範囲":scope});
        capabilities.push("対話送信");
        recovery = Some("接続再確認");
    }
    Ok(
        json!({"版":1,"要求hash":created.payload_hash,"作成操作":created.operation,"承認":approval,"承認能力":capabilities,"復旧対応":recovery,"現在権限":false}),
    )
}

#[test]
fn 過去承認文脈は要求と実行系と表示範囲を結合する() {
    for case in 0..6 {
        let mut log = BrokerAuditLog::default();
        let hash = sha256_tagged(b"request");
        let created = log.append(
            "a",
            "対話送信",
            "recorded",
            "対話承認待ち作成",
            "INTERNAL_STATE",
            &hash,
        );
        let record = json!({"実行系ID":"local","作成監査ID":created.event_id,"開始監査ID":null});
        let pending = audit_context(log.events(), &record).unwrap();
        assert!(pending["承認"].is_null());
        assert_eq!(pending["承認能力"], json!([]));
        let reason = format!("対話送信承認 Capability=対話送信 Permission=local:127.0.0.1:9 Approval={hash} 表示範囲=hash_only RecoveryAction=接続再確認");
        let reason = match case {
            1 => reason.replace("local:", "other:"),
            2 => reason.replace(&hash, &sha256_tagged(b"other")),
            3 => reason.replace("hash_only", "unknown"),
            4 => reason.replace("接続再確認", "arbitrary"),
            _ => reason,
        };
        let start = log.append(
            "a",
            "対話承認",
            "recorded",
            &reason,
            "INTERNAL_STATE",
            if case == 5 { "wrong" } else { &hash },
        );
        let mut record = record;
        record["開始監査ID"] = json!(start.event_id);
        let result = audit_context(log.events(), &record);
        if case == 0 {
            let context = result.unwrap();
            assert_eq!(context["承認"]["要求hash"], hash);
            assert_eq!(context["承認"]["内容表示範囲"], "hash_only");
            assert_eq!(context["現在権限"], false);
            assert!(!context.to_string().contains("127.0.0.1"));
        } else {
            assert!(result.is_err(), "case={case}");
        }
    }
}


fn result_evidence(events: &[super::audit::BrokerAuditEvent], archive: &Value) -> Result<Value, &'static str> {
    if !matches!(archive["状態"].as_str(), Some("成功" | "保留")) { return Ok(Value::Null); }
    let record = &archive["実行記録"];
    let matching: Vec<_> = events.iter().enumerate().filter(|(_, e)|
        Some(e.request_id.as_str()) == record["要求ID"].as_str() && e.reason.starts_with("対話結果証跡:")).collect();
    if matching.is_empty() { return Ok(Value::Null); }
    if matching.len() != 1 { return Err("結果証跡が重複"); }
    let (index, event) = matching[0];
    let body = event.reason.strip_prefix("対話結果証跡:").ok_or("結果証跡が不正")?;
    if body.len() > 16384 || sha256_tagged(body.as_bytes()) != event.payload_hash
        || event.decision != "recorded" || event.evidence_source != "INTERNAL_STATE"
        || !["実行系列挙", "対話開始", "対話送信", "対話取得", "対話中止", "対話終了", "対話承認", "対話承認待ち"].contains(&event.operation.as_str()) {
        return Err("結果証跡の監査が不正");
    }
    let proof: Value = super::json_input::read_unique(body).map_err(|_| "結果証跡形式が不正")?;
    let object = proof.as_object().ok_or("結果証跡形式が不正")?;
    let keys = ["版", "要求ID", "対話セッションID", "実行系ID", "要求hash", "終了監査ID", "表示範囲", "応答hash", "能力申告hash", "経路申告hash", "追跡参照hash", "証拠種別"];
    if object.len() != keys.len() || keys.iter().any(|k| !object.contains_key(*k))
        || proof["版"] != 1 || proof["証拠種別"] != "INTERNAL_STATE" {
        return Err("結果証跡形式が不正");
    }
    for key in ["要求ID", "対話セッションID", "実行系ID", "終了監査ID"] {
        if proof[key] != record[key] { return Err("結果証跡の要求対応が不一致"); }
    }
    let context = audit_context(&events[..index], record)?;
    let scope = proof["表示範囲"].as_str().ok_or("結果証跡の表示範囲が不正")?;
    if !["full", "hash_only", "summary", "redacted"].contains(&scope)
        || proof["要求hash"] != context["要求hash"]
        || proof["表示範囲"] != context["承認"]["内容表示範囲"] {
        return Err("結果証跡の過去承認が不一致");
    }
    let ended = events[..index].iter().find(|e| Some(e.event_id.as_str()) == proof["終了監査ID"].as_str()).ok_or("結果証跡の終了監査が不在")?;
    if ended.request_id != event.request_id || ended.reason != "対話完了"
        || ended.decision != "recorded" || ended.evidence_source != "INTERNAL_STATE"
        {
        return Err("結果証跡の終了監査が不一致");
    }
    let valid_hash = |v: &Value| v.as_str().and_then(|s| s.strip_prefix("sha256:")).is_some_and(|s| s.len() == 64 && s.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)));
    if !valid_hash(&proof["要求hash"]) || !valid_hash(&proof["応答hash"]) { return Err("結果証跡hash形式が不正"); }
    for key in ["能力申告hash", "経路申告hash", "追跡参照hash"] {
        if if scope == "full" { !valid_hash(&proof[key]) } else { !proof[key].is_null() } {
            return Err("結果証跡の内容範囲が不正");
        }
    }
    Ok(proof)
}


#[test]
fn 結果証跡読取は改変と別要求と表示昇格と重複を拒否する() {
    for case in 0..12 {
        let mut log = BrokerAuditLog::default();
        let request = "a".repeat(32); let hash = sha256_tagged(b"request");
        let created = log.append(&request, "対話送信", "recorded", "対話承認待ち作成", "INTERNAL_STATE", &hash);
        let scope = if case == 1 { "hash_only" } else { "full" };
        let start = log.append(&request, "対話承認", "recorded", &format!("対話送信承認 Capability=対話送信 Permission=local:127.0.0.1:9 Approval={hash} 表示範囲={scope} RecoveryAction=接続再確認"), "INTERNAL_STATE", &hash);
        let end = log.append(&request, "対話取得", "recorded", "対話完了", "INTERNAL_STATE", &sha256_tagged(b"receipt"));
        let archive = json!({"状態":"成功", "実行記録":{"要求ID":request,"対話セッションID":"b".repeat(32),"実行系ID":"local","作成監査ID":created.event_id,"開始監査ID":start.event_id,"終了監査ID":end.event_id}});
        assert!(result_evidence(log.events(), &archive).unwrap().is_null());
        let mut proof = json!({"版":1,"要求ID":request,"対話セッションID":"b".repeat(32),"実行系ID":"local","要求hash":hash,"終了監査ID":end.event_id,"表示範囲":scope,"応答hash":hash,"能力申告hash":hash,"経路申告hash":hash,"追跡参照hash":hash,"証拠種別":"INTERNAL_STATE"});
        if case == 1 { for k in ["能力申告hash", "経路申告hash", "追跡参照hash"] { proof[k] = Value::Null; } }
        match case {
            2 => proof["要求ID"] = json!("c".repeat(32)),
            3 => proof["対話セッションID"] = json!("c".repeat(32)),
            4 => proof["実行系ID"] = json!("other"),
            5 => proof["要求hash"] = json!(sha256_tagged(b"other")),
            6 => proof["終了監査ID"] = json!(start.event_id),
            7 => proof["表示範囲"] = json!("hash_only"),
            8 => proof["応答hash"] = json!("invalid"),
            9 => proof["本文"] = json!("非公開"),
            _ => (),
        }
        let body = proof.to_string();
        let payload = if case == 10 { sha256_tagged(b"wrong") } else { sha256_tagged(body.as_bytes()) };
        log.append(&request, "対話取得", "recorded", &format!("対話結果証跡:{body}"), "INTERNAL_STATE", &payload);
        if case == 11 { log.append(&request, "対話取得", "recorded", &format!("対話結果証跡:{body}"), "INTERNAL_STATE", &payload); }
        let result = result_evidence(log.events(), &archive);
        if case < 2 { assert_eq!(result.unwrap(), proof); } else { assert!(result.is_err(), "case={case}"); }
    }
}
