use super::*;
use crate::broker::dialogue::{対話要求, 実行結果};
use serde_json::json;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

struct Adapter { large: bool }
impl 実行系Adapter for Adapter {
    fn 接続対象(&self) -> String { "fixture".into() }
    fn 応答(&self, r: &対話要求, _: &AtomicBool, _: Instant, raw: &mut Vec<Vec<u8>>) -> Result<実行結果, 対話失敗> {
        raw.push(b"synthetic-raw".to_vec());
        Ok(実行結果 {対話セッションID:r.対話セッションID.clone(),本文:if self.large {"a".repeat(65536)} else {"試験の応答本文".into()},参照:vec![],能力:vec!["fixture".into()],経路:"fixture".into(),追跡ID:"a".repeat(32),追跡hash:sha256_tagged(b"fixture"),保留:false,生応答:b"synthetic-raw".to_vec()})
    }
}

fn call(b: &mut Broker, op: BrokerOperation, payload: Value, owner: bool) -> BrokerResponse {
    let id = crate::broker::dialogue::識別子生成().unwrap();
    let mut r = BrokerRequestEnvelope::command_envelope(&id, "session-1", &id);
    r.operation = Some(op); r.payload = Some(payload); r.refresh_payload_hash();
    b.処理(r, owner)
}
fn accepted(r: BrokerResponse) -> Value { assert_eq!(r.status,BrokerStatus::Accepted, "{:?}",r.error); r.body.unwrap() }
fn prepare(b: &mut Broker, large: bool) -> Value {
    b.実行系登録("fixture",Arc::new(Adapter {large})).unwrap();
    let s = accepted(call(b,BrokerOperation::対話開始,json!({"実行系ID":"fixture"}),false));
    let p = accepted(call(b,BrokerOperation::対話送信,json!({"対話セッションID":s["対話セッションID"],"入力":"試験の入力本文"}),false));
    accepted(call(b,BrokerOperation::対話承認,json!({"要求ID":p["要求ID"],"要求hash":p["要求hash"],"表示範囲":"full"}),true));
    for _ in 0..200 {
        let r=accepted(call(b,BrokerOperation::対話取得,json!({"要求ID":p["要求ID"]}),false));
        if r["状態"]=="完了" { return json!({"要求ID":p["要求ID"],"要求hash":p["要求hash"]}); }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("完了期限超過")
}

#[test]
fn explicit_save_encrypts_and_binds_audits_without_overwrite() {
    let root=super::tests::temp_store_dir("content-save");
    let vault=root.join("vault"); std::fs::create_dir(&vault).unwrap();
    let mut b=super::tests::persistent_test_broker(&root.join("audit"));
    let select=prepare(&mut b, false);
    assert_eq!(call(&mut b,BrokerOperation::対話内容保存,select.clone(),true).error.unwrap().code,"保管未登録");
    b.保管先起動登録(&vault,true,&[root.join("audit")]).unwrap();
    assert_eq!(call(&mut b,BrokerOperation::対話内容保存,select.clone(),false).status,BrokerStatus::Rejected);
    let receipt=accepted(call(&mut b,BrokerOperation::対話内容保存,select.clone(),true));
    let target=select["要求ID"].as_str().unwrap();
    let cipher_path=vault.join(format!("history-{target}.dpapi"));
    let cipher=std::fs::read(&cipher_path).unwrap();
    assert_eq!(receipt["暗号文hash"],sha256_tagged(&cipher));
    let secret=b.protected_store.as_ref().unwrap().read(crate::protected_store::Purpose::History,target,receipt["暗号文hash"].as_str().unwrap()).unwrap();
    let content:Value=serde_json::from_slice(secret.as_bytes()).unwrap();
    assert_eq!(content["要求"]["入力"],"試験の入力本文");
    assert_eq!(content["結果"]["本文"],"試験の応答本文");
    assert_eq!(content["要求hash"],select["要求hash"]);
    assert_eq!(content["実行記録"]["終了監査ID"],receipt["終了監査ID"]);
    assert!(!String::from_utf8_lossy(&cipher).contains("試験の入力本文"));
    let disk=b.state_store.persistent_store.as_ref().unwrap().verified_audit_log().unwrap();
    assert_eq!(disk,b.audit_log);
    let audit=serde_json::to_string(disk.events()).unwrap();
    assert!(!audit.contains("試験の入力本文") && !audit.contains("試験の応答本文"));
    assert_eq!(call(&mut b,BrokerOperation::対話内容保存,select.clone(),true).status,BrokerStatus::Rejected);
    assert_eq!(std::fs::read(&cipher_path).unwrap(),cipher);
    std::fs::remove_file(&cipher_path).unwrap();
    assert_eq!(call(&mut b,BrokerOperation::対話内容保存,select.clone(),true).error.unwrap().code,"保存済み");
    assert!(!cipher_path.exists());
    std::fs::write(&cipher_path,&cipher).unwrap();
    // 保存後段の実監査確定関数へI/O失敗を注入する。成功metadataを公開しない。
    let audit_path=root.join("audit/audit.jsonl");
    std::fs::remove_file(&audit_path).unwrap(); std::fs::create_dir(&audit_path).unwrap();
    let response=b.対話内容保存確定("fixture-finalization",receipt);
    assert_eq!(response.status,BrokerStatus::Suspended); assert!(response.body.is_none());
    assert_eq!(std::fs::read(&cipher_path).unwrap(),cipher);
    drop(secret); drop(b); std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn tampered_anchor_prevents_any_content_write() {
    let root=super::tests::temp_store_dir("content-tamper");
    let vault=root.join("vault"); std::fs::create_dir(&vault).unwrap();
    let mut b=super::tests::persistent_test_broker(&root.join("audit"));
    b.保管先起動登録(&vault,true,&[root.join("audit")]).unwrap();
    let select=prepare(&mut b, false);
    std::fs::write(root.join("audit/audit_anchor.json"),b"{}").unwrap();
    let r=call(&mut b,BrokerOperation::対話内容保存,select,true);
    assert_eq!(r.status,BrokerStatus::Suspended); assert!(r.body.is_none());
    assert_eq!(std::fs::read_dir(&vault).unwrap().count(),0);
    drop(b); std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn oversized_content_is_rejected_before_save_approval() {
    let root=super::tests::temp_store_dir("content-large");
    let vault=root.join("vault"); std::fs::create_dir(&vault).unwrap();
    let mut b=super::tests::persistent_test_broker(&root.join("audit"));
    b.保管先起動登録(&vault,true,&[root.join("audit")]).unwrap();
    let select=prepare(&mut b, true);
    let r=call(&mut b,BrokerOperation::対話内容保存,select,true);
    assert_eq!(r.status,BrokerStatus::Rejected); assert_eq!(r.error.unwrap().code,"保管上限超過");
    assert_eq!(std::fs::read_dir(&vault).unwrap().count(),0);
    assert!(!b.audit_log.events().iter().any(|e|e.reason.starts_with("対話内容保存承認 ")));
    drop(b); std::fs::remove_dir_all(root).unwrap();
}
