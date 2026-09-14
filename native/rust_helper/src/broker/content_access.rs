//! 保存承認とは独立した、現在の一件の内容閲覧承認。権限を永続化しない。
use super::{audit::BrokerAuditLog, dialogue::{対話要求, 識別子生成, 要求hash}, execution_history::{page, Query}};
use crate::audit_hash::sha256_tagged;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

#[derive(Default, Debug)]
pub(crate) struct ContentAccess { grant: Option<Grant>, last_now: Option<i64> }
#[derive(Debug)]
struct Grant { id: String, target: Target, expires: i64, deadline: Instant }
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    #[serde(rename="要求ID")] request: String,
    #[serde(rename="保存監査ID")] audit_id: String,
    #[serde(rename="保存監査hash")] audit_hash: String,
}

impl ContentAccess {
    pub(crate) fn revoke(&mut self) { self.grant=None; }
    fn refresh(&mut self, now: i64) {
        if self.last_now.is_some_and(|last| now<last) || self.grant.as_ref().is_some_and(|g|now>=g.expires || Instant::now()>=g.deadline) { self.revoke(); }
        self.last_now=Some(self.last_now.unwrap_or(now).max(now));
    }
    fn grant_body(&self) -> Value {
        self.grant.as_ref().map(|g|json!({"approval_id":g.id,"要求ID":g.target.request,"保存監査ID":g.target.audit_id,"保存監査hash":g.target.audit_hash,"expires_at":g.expires})).unwrap_or(Value::Null)
    }
    pub(crate) fn current(&mut self, body: &Value, now: i64) -> bool {
        self.refresh(now);
        self.grant.as_ref().is_some_and(|g|body["grant"]["approval_id"]==g.id)
    }
    pub(crate) fn operate(&mut self, op: &str, payload: &Value, owner: bool, now: i64, log: &BrokerAuditLog,
        audit: &mut dyn FnMut(&str,&str)->Result<(), &'static str>,
        load: &mut dyn FnMut(&str,&str)->Result<Vec<u8>, &'static str>) -> Result<Value, &'static str> {
        self.refresh(now);
        if matches!(op,"対話内容承認"|"対話内容失効") && !owner { return Err("owner制御資格が必要"); }
        let mut content=Value::Null;
        match op {
            "対話内容承認" => {
                self.revoke();
                let target:Target=serde_json::from_value(payload.clone()).map_err(|_|"内容承認形式が不正")?;
                if self.last_now.is_some_and(|last| now<last) { return Err("時計後退中は承認できない"); }
                target_entry(log,&target)?;
                let id=識別子生成().map_err(|_|"承認ID生成失敗")?;
                let expires=now.checked_add(60).ok_or("内容承認期限が不正")?;
                let body=json!({"approval_id":id,"要求ID":target.request,"保存監査ID":target.audit_id,"保存監査hash":target.audit_hash,"expires_at":expires});
                audit("Capability=対話内容閲覧 Permission=一件の保存参照 Approval=owner期限付き承認 RecoveryAction=保管監査再確認",&sha256_tagged(body.to_string().as_bytes()))?;
                self.grant=Some(Grant{id,target,expires,deadline:Instant::now()+Duration::from_secs(60)});
            }
            "対話内容失効" | "対話内容閲覧状態" => {
                if !payload.as_object().is_some_and(|v|v.is_empty()) { return Err("内容操作の入力が不正"); }
                if op=="対話内容失効" { self.revoke(); audit("現在の内容閲覧承認を失効",&sha256_tagged(b"{}"))?; }
            }
            "対話内容閲覧" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Read { approval_id:String }
                let p:Read=serde_json::from_value(payload.clone()).map_err(|_|"内容閲覧形式が不正")?;
                let g=self.grant.as_ref().ok_or("現在の内容閲覧承認が必要")?;
                if p.approval_id!=g.id { return Err("内容閲覧承認が不一致"); }
                let entry=target_entry(log,&g.target)?;
                let receipt=&entry["content_receipt"]["receipt"];
                let bytes=load(&g.target.request,receipt["暗号文hash"].as_str().ok_or("暗号文hash不在")?)?;
                content=validate_content(&bytes,&entry)?;
            }
            _=>return Err("未知の内容操作"),
        }
        Ok(json!({"grant":self.grant_body(),"content":content}))
    }
}

#[cfg(windows)]
pub(crate) fn deletion_entry(log: &BrokerAuditLog, payload: &Value) -> Result<Value, &'static str> {
    let target: Target = serde_json::from_value(payload.clone()).map_err(|_| "削除対象形式が不正")?;
    saved_entry(log, &target)
}

fn target_entry(log:&BrokerAuditLog,target:&Target)->Result<Value,&'static str> {
    let entry = saved_entry(log, target)?;
    for event in log.events().iter().filter(|e|e.operation=="対話内容削除" && e.decision=="recorded") {
        let raw=event.reason.strip_prefix("対話内容削除承認:").ok_or("削除監査形式が不正")?;
        let intent:Value=super::json_input::read_unique(raw).map_err(|_|"削除監査形式が不正")?;
        if !exact(&intent,&["要求ID","保存監査ID","保存監査hash","暗号文hash"])
            || event.payload_hash!=sha256_tagged(raw.as_bytes()) || event.evidence_source!="INTERNAL_STATE" {
            return Err("削除監査形式が不正");
        }
        if intent["要求ID"]==target.request {return Err("削除承認済み内容は復元しない。保管監査再確認が必要");}
    }
    Ok(entry)
}

fn saved_entry(log:&BrokerAuditLog,target:&Target)->Result<Value,&'static str> {
    if target.audit_id.is_empty() || target.audit_id.len()>256 {return Err("保存監査IDが不正");}
    let result=page(log,Query{after:0,limit:1,latest_per_request:true,include_result_evidence:true,include_content_receipt:true,filter:[("要求ID".into(),target.request.clone())].into(),..Default::default()})?;
    let entry=result["entries"].as_array().and_then(|v|v.first()).ok_or("保存対象が不在")?;
    let saved=&entry["content_receipt"];
    if saved.is_null() || saved["audit_event_id"]!=target.audit_id || saved["event_hash"]!=target.audit_hash { return Err("保存監査参照が不一致"); }
    Ok(entry.clone())
}

fn exact(v:&Value,keys:&[&str])->bool { v.as_object().is_some_and(|m|m.len()==keys.len() && keys.iter().all(|k|m.contains_key(*k))) }
fn validate_content(bytes:&[u8],entry:&Value)->Result<Value,&'static str> {
    if bytes.is_empty() || bytes.len()>65536 {return Err("内容保管の上限が不正");}
    let text=std::str::from_utf8(bytes).map_err(|_|"内容文字列が不正")?;
    let v:Value=super::json_input::read_unique(text).map_err(|_|"保管内容形式が不正")?;
    if !exact(&v,&["版","要求","要求hash","結果","実行記録","結果証跡"]) || v["版"]!=1 {return Err("内容保管形式が不正");}
    let request:対話要求=serde_json::from_value(v["要求"].clone()).map_err(|_|"保管要求が不正")?;
    let record=&entry["record"]["実行記録"];
    if request.入力.is_empty() || request.入力.chars().count()>4096 || v["実行記録"]!=*record || v["結果証跡"]!=entry["result_evidence"]
        || v["要求hash"]!=要求hash(&request) || v["要求hash"]!=entry["content_receipt"]["receipt"]["要求hash"] {
        return Err("保管内容と監査が不一致");
    }
    for key in ["要求ID","実行系ID","対話セッションID"] {
        if v["要求"][key]!=record[key] || v["結果"][key]!=record[key] {return Err("保管結果の対象が不一致");}
    }
    let r=&v["結果"];
    if !exact(r,&["要求ID","実行系ID","対話セッションID","状態","表示範囲","本文","参照","能力","経路","追跡ID","追跡hash","応答hash","失敗分類","復旧"])
        || r["状態"]!=entry["record"]["状態"] || r["表示範囲"]!="full" || r["失敗分類"]!="" || r["復旧"]!=""
        || !r["本文"].as_str().is_some_and(|s|s.chars().count()<=65536)
        || !r["経路"].as_str().is_some_and(|s|s.chars().count()<=256) { return Err("保管結果の形式が不正"); }
    for (key,limit) in [("参照",2048),("能力",256)] {
        if !r[key].as_array().is_some_and(|a|a.len()<=64 && a.iter().all(|s|s.as_str().is_some_and(|s|s.chars().count()<=limit))) {return Err("保管結果の配列が不正");}
    }
    let hex=|s:&str,n:usize|s.len()==n && s.bytes().all(|c|c.is_ascii_digit() || (b'a'..=b'f').contains(&c));
    if !r["追跡ID"].as_str().is_some_and(|s|hex(s,32)) || !r["追跡hash"].as_str().and_then(|s|s.strip_prefix("sha256:")).is_some_and(|s|hex(s,64)) {
        return Err("保管結果の追跡参照が不正");
    }
    let proof=&v["結果証跡"];
    let hash=|v:Value|sha256_tagged(v.to_string().as_bytes());
    if r["応答hash"]!=proof["応答hash"] || hash(r["能力"].clone())!=proof["能力申告hash"]
        || hash(r["経路"].clone())!=proof["経路申告hash"]
        || hash(json!({"追跡ID":r["追跡ID"],"追跡hash":r["追跡hash"]}))!=proof["追跡参照hash"] {
        return Err("保管結果の証跡が不一致");
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decrypted_content_rejects_unknown_fields_and_cross_record_substitution() {
        let mut original:Value=serde_json::from_str(include_str!("../../../../examples/contracts/runtime_content_archive.valid.json")).unwrap();
        let request:対話要求=serde_json::from_value(original["要求"].clone()).unwrap();
        original["要求hash"]=json!(要求hash(&request));
        original["結果"]["追跡ID"]=json!("a".repeat(32));
        original["結果"]["追跡hash"]=json!(sha256_tagged(b"trace"));
        original["結果証跡"]["要求hash"]=original["要求hash"].clone();
        original["結果証跡"]["能力申告hash"]=json!(sha256_tagged(original["結果"]["能力"].to_string().as_bytes()));
        original["結果証跡"]["経路申告hash"]=json!(sha256_tagged(original["結果"]["経路"].to_string().as_bytes()));
        original["結果証跡"]["追跡参照hash"]=json!(sha256_tagged(json!({"追跡ID":original["結果"]["追跡ID"],"追跡hash":original["結果"]["追跡hash"]}).to_string().as_bytes()));
        let entry=json!({"record":{"状態":"成功","実行記録":original["実行記録"]},"result_evidence":original["結果証跡"],"content_receipt":{"receipt":{"要求hash":original["要求hash"]}}});
        assert_eq!(validate_content(original.to_string().as_bytes(),&entry).unwrap(),original);
        for case in 0..12 {
            let mut content=original.clone();
            match case {
                0=>content["権限"]=json!(true),
                1=>content["版"]=json!(2),
                2=>content["要求"]["入力"]=json!("別内容"),
                3=>content["要求"]["権限"]=json!(true),
                4=>content["実行記録"]["終了監査ID"]=json!("other"),
                5=>content["結果証跡"]["要求ID"]=json!("b".repeat(32)),
                6=>content["結果"]["表示範囲"]=json!("hash_only"),
                7=>content["結果"]["要求ID"]=json!("b".repeat(32)),
                8=>content["結果"]["能力"]=json!(["other"]),
                9=>content["結果"]["本文"]=json!(null),
                10=>content["結果"]["参照"]=json!([123]),
                _=>content["結果"]["追跡ID"]=json!("invalid"),
            }
            assert!(validate_content(content.to_string().as_bytes(),&entry).is_err(),"case={case}");
        }
    }
    #[test]
    fn current_grant_expires_on_both_clocks_and_never_resurrects() {
        let mut a=ContentAccess::default();
        a.grant=Some(Grant{id:"fixture".into(),target:Target{request:"a".repeat(32),audit_id:"audit".into(),audit_hash:sha256_tagged(b"fixture")},expires:160,deadline:Instant::now()+Duration::from_secs(60)});
        let body=json!({"grant":a.grant_body(),"content":null});
        assert!(a.current(&body,100)); assert!(!a.current(&body,99)); assert!(!a.current(&body,101));
        a.grant=Some(Grant{id:"fixture".into(),target:Target{request:"a".repeat(32),audit_id:"audit".into(),audit_hash:sha256_tagged(b"fixture")},expires:160,deadline:Instant::now()});
        assert!(!a.current(&body,102));
        a.grant=Some(Grant{id:"fixture".into(),target:Target{request:"a".repeat(32),audit_id:"audit".into(),audit_hash:sha256_tagged(b"fixture")},expires:160,deadline:Instant::now()+Duration::from_secs(60)});
        assert!(a.current(&body,159)); assert!(!a.current(&body,160));
        let log=BrokerAuditLog::default();
        let mut audit=|_:&str,_:&str|Ok(());
        let mut no_read=|_:&str,_:&str|->Result<Vec<u8>,&'static str>{panic!("承認前に内容を読まない")};
        assert!(a.operate("対話内容閲覧",&json!({"approval_id":"fixture"}),false,160,&log,&mut audit,&mut no_read).is_err());
        assert!(a.operate("対話内容承認",&json!({}),false,160,&log,&mut audit,&mut no_read).is_err());
        assert!(a.operate("対話内容閲覧状態",&json!({}),false,160,&log,&mut audit,&mut no_read).unwrap()["grant"].is_null());
        a.revoke();
    }
}
