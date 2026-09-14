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
    let saved_event=b.audit_events().last().unwrap().clone();
    let approval_target=json!({"要求ID":select["要求ID"],"保存監査ID":saved_event.event_id,"保存監査hash":saved_event.event_hash});
    assert_eq!(call(&mut b,BrokerOperation::対話内容承認,approval_target.clone(),false).status,BrokerStatus::Rejected);
    let grant=accepted(call(&mut b,BrokerOperation::対話内容承認,approval_target.clone(),true));
    let read=json!({"approval_id":grant["grant"]["approval_id"]});
    let opened=accepted(call(&mut b,BrokerOperation::対話内容閲覧,read.clone(),false));
    assert_eq!(opened["content"]["要求"]["入力"],"試験の入力本文");
    assert_eq!(opened["content"]["結果"]["本文"],"試験の応答本文");
    accepted(call(&mut b,BrokerOperation::対話内容失効,json!({}),true));
    assert!(call(&mut b,BrokerOperation::対話内容閲覧,read,false).body.is_none());
    let target=select["要求ID"].as_str().unwrap();
    let cipher_path=vault.join(format!("history-{target}.dpapi"));
    let cipher=std::fs::read(&cipher_path).unwrap();
    let grant=accepted(call(&mut b,BrokerOperation::対話内容承認,approval_target.clone(),true));
    let mut corrupt=cipher.clone(); let last=corrupt.len()-1; corrupt[last]^=1;
    std::fs::write(&cipher_path,&corrupt).unwrap();
    let denied=call(&mut b,BrokerOperation::対話内容閲覧,json!({"approval_id":grant["grant"]["approval_id"]}),false);
    assert!(denied.body.is_none());
    std::fs::write(&cipher_path,&cipher).unwrap();
    assert!(accepted(call(&mut b,BrokerOperation::対話内容閲覧状態,json!({}),false))["grant"].is_null());
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
    let denied=b.内容閲覧確定("fixture-content-finalization","対話内容閲覧",opened,&sha256_tagged(b"fixture"));
    assert_eq!(denied.status,BrokerStatus::Suspended); assert!(denied.body.is_none());
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


#[test]
fn explicit_delete_binds_saved_receipt_and_never_revives_restored_ciphertext() {
    let root=super::tests::temp_store_dir("content-delete");
    let vault=root.join("vault");std::fs::create_dir(&vault).unwrap();
    let audit=root.join("audit");
    let mut b=super::tests::persistent_test_broker(&audit);
    b.保管先起動登録(&vault,true,&[audit.clone()]).unwrap();
    let select=prepare(&mut b,false);
    let receipt=accepted(call(&mut b,BrokerOperation::対話内容保存,select.clone(),true));
    let saved=b.audit_events().last().unwrap().clone();
    let target=json!({"要求ID":select["要求ID"],"保存監査ID":saved.event_id,"保存監査hash":saved.event_hash});
    let path=vault.join(format!("history-{}.dpapi",select["要求ID"].as_str().unwrap()));
    let cipher=std::fs::read(&path).unwrap();
    assert!(call(&mut b,BrokerOperation::対話内容削除,target.clone(),false).body.is_none());
    for key in ["要求ID","保存監査ID","保存監査hash","extra"] {
        let mut wrong=target.clone();wrong[key]=json!("wrong");
        assert!(call(&mut b,BrokerOperation::対話内容削除,wrong,true).body.is_none());
        assert_eq!(std::fs::read(&path).unwrap(),cipher);
    }
    std::fs::write(&path,b"tampered").unwrap();
    assert!(call(&mut b,BrokerOperation::対話内容削除,target.clone(),true).body.is_none());
    assert_eq!(std::fs::read(&path).unwrap(),b"tampered");
    std::fs::write(&path,&cipher).unwrap();
    let grant=accepted(call(&mut b,BrokerOperation::対話内容承認,target.clone(),true));
    let result=accepted(call(&mut b,BrokerOperation::対話内容削除,target.clone(),true));
    assert_eq!(result["暗号文hash"],receipt["暗号文hash"]);
    assert_eq!(result["状態"],"削除確定");
    assert!(!path.exists());
    assert!(call(&mut b,BrokerOperation::対話内容閲覧,json!({"approval_id":grant["grant"]["approval_id"]}),false).body.is_none());
    assert!(call(&mut b,BrokerOperation::対話内容削除,target.clone(),true).body.is_none());
    let events=b.audit_events();
    let approval_index=events.iter().position(|e|e.event_id==result["削除承認監査ID"]).unwrap();
    let committed_index=events.iter().position(|e|e.reason.starts_with("対話内容削除記録:")).unwrap();
    assert!(approval_index<committed_index);
    assert_eq!(events[approval_index].decision,"recorded");
    assert_eq!(b.state_store.persistent_store.as_ref().unwrap().verified_audit_log().unwrap(),b.audit_log);
    std::fs::write(&path,&cipher).unwrap();
    assert!(call(&mut b,BrokerOperation::対話内容承認,target.clone(),true).body.is_none());
    drop(b);
    let mut b=super::tests::persistent_test_broker(&audit);
    b.保管先起動登録(&vault,true,&[audit.clone()]).unwrap();
    assert!(call(&mut b,BrokerOperation::対話内容承認,target.clone(),true).body.is_none());
    assert_eq!(std::fs::read(&path).unwrap(),cipher);
    // 後段監査の実関数でI/O失敗を発生させ、削除成功の推定を公開しない。
    let audit_file=audit.join("audit.jsonl");
    std::fs::remove_file(&audit_file).unwrap();std::fs::create_dir(&audit_file).unwrap();
    let denied=b.内容削除確定("delete-finalization-failure",result);
    assert_eq!(denied.status,BrokerStatus::Suspended);assert!(denied.body.is_none());
    assert!(call(&mut b,BrokerOperation::対話内容削除,target,true).body.is_none());
    assert_eq!(std::fs::read(&path).unwrap(),cipher);
    drop(b);std::fs::remove_dir_all(root).unwrap();
}


#[test]
fn delete_approval_audit_failure_drops_prepared_handle_without_deleting() {
    let root=super::tests::temp_store_dir("delete-approval-failure");
    let vault=root.join("vault");std::fs::create_dir(&vault).unwrap();
    let audit=root.join("audit");
    let mut b=super::tests::persistent_test_broker(&audit);
    b.保管先起動登録(&vault,true,&[audit.clone()]).unwrap();
    let select=prepare(&mut b,false);
    let receipt=accepted(call(&mut b,BrokerOperation::対話内容保存,select.clone(),true));
    let saved=b.audit_events().last().unwrap().clone();
    let target=select["要求ID"].as_str().unwrap();
    let cipher_hash=receipt["暗号文hash"].as_str().unwrap();
    let path=vault.join(format!("history-{target}.dpapi"));
    let cipher=std::fs::read(&path).unwrap();
    let prepared=b.protected_store.as_ref().unwrap().prepare_delete(crate::protected_store::Purpose::History,target,cipher_hash).unwrap();
    let intent=json!({"要求ID":target,"保存監査ID":saved.event_id,"保存監査hash":saved.event_hash,"暗号文hash":cipher_hash});
    let audit_file=audit.join("audit.jsonl");std::fs::remove_file(&audit_file).unwrap();std::fs::create_dir(&audit_file).unwrap();
    let denied=b.内容削除実行("delete-approval-failure",intent,prepared,&sha256_tagged(b"fixture"));
    assert_eq!(denied.status,BrokerStatus::Suspended);assert!(denied.body.is_none());
    assert_eq!(std::fs::read(&path).unwrap(),cipher);
    drop(b);std::fs::remove_dir_all(root).unwrap();
}


#[test]
fn owner_inventory_distinguishes_actual_missing_partial_tampered_and_deleted_files() {
    let root=super::tests::temp_store_dir("content-inventory");
    let vault=root.join("vault");std::fs::create_dir(&vault).unwrap();
    let audit=root.join("audit");let mut b=super::tests::persistent_test_broker(&audit);
    let select=prepare(&mut b,false);
    assert!(call(&mut b,BrokerOperation::対話保管状態,select.clone(),true).body.is_none());
    b.保管先起動登録(&vault,true,&[audit.clone()]).unwrap();
    assert!(call(&mut b,BrokerOperation::対話保管状態,select.clone(),false).body.is_none());
    for key in ["要求ID","要求hash","extra"] {
        let mut wrong=select.clone();wrong[key]=json!("wrong");
        assert!(call(&mut b,BrokerOperation::対話保管状態,wrong,true).body.is_none());
    }
    let state=|b:&mut Broker|accepted(call(b,BrokerOperation::対話保管状態,select.clone(),true));
    assert_eq!(state(&mut b)["状態"],"未保存・file不在");
    let path=vault.join(format!("history-{}.dpapi",select["要求ID"].as_str().unwrap()));
    std::fs::write(&path,b"").unwrap();
    let partial=state(&mut b);assert_eq!(partial["状態"],"保存記録なし・fileあり");assert_eq!(partial["bytes"],0);assert_eq!(partial["暗号文hash"],sha256_tagged(b""));
    std::fs::remove_file(&path).unwrap();
    accepted(call(&mut b,BrokerOperation::対話内容保存,select.clone(),true));
    let saved=b.audit_events().last().unwrap().clone();
    let target=json!({"要求ID":select["要求ID"],"保存監査ID":saved.event_id,"保存監査hash":saved.event_hash});
    let cipher=std::fs::read(&path).unwrap();
    let normal=state(&mut b);assert_eq!(normal["状態"],"保存済み・hash一致");assert_eq!(normal["暗号文hash"],sha256_tagged(&cipher));
    let reader=std::fs::File::open(&path).unwrap();
    assert!(call(&mut b,BrokerOperation::対話保管状態,select.clone(),true).body.is_none());drop(reader);
    let alias=vault.join("alias");std::fs::hard_link(&path,&alias).unwrap();
    assert!(call(&mut b,BrokerOperation::対話保管状態,select.clone(),true).body.is_none());std::fs::remove_file(&alias).unwrap();
    std::fs::write(&path,b"partial").unwrap();assert_eq!(state(&mut b)["状態"],"保存記録あり・hash不一致");
    std::fs::remove_file(&path).unwrap();assert_eq!(state(&mut b)["状態"],"保存記録あり・file欠落");
    std::fs::write(&path,&cipher).unwrap();
    accepted(call(&mut b,BrokerOperation::対話内容削除,target.clone(),true));
    let deleted=state(&mut b);assert_eq!(deleted["状態"],"削除確定・file不在");assert!(!deleted["削除結果監査ID"].is_null());
    std::fs::write(&path,&cipher).unwrap();assert_eq!(state(&mut b)["状態"],"削除承認あり・file残存");
    assert_eq!(std::fs::read(&path).unwrap(),cipher);
    // 削除承認を永続化した後、fileを削除して結果監査前で停止した状態を構成する。
    let request_id="fixture-interrupted-delete";
    let request_hash=sha256_tagged(target.to_string().as_bytes());
    b.append_audit(request_id,"対話内容削除","received","試験受信",EVIDENCE_SOURCE_INTERNAL_STATE,&request_hash).unwrap();
    let intent=json!({"要求ID":select["要求ID"],"保存監査ID":saved.event_id,"保存監査hash":saved.event_hash,"暗号文hash":sha256_tagged(&cipher)});
    let encoded=intent.to_string();
    b.append_audit(request_id,"対話内容削除","recorded",&format!("対話内容削除承認:{encoded}"),EVIDENCE_SOURCE_INTERNAL_STATE,&sha256_tagged(encoded.as_bytes())).unwrap();
    b.protected_store.as_ref().unwrap().prepare_delete(crate::protected_store::Purpose::History,select["要求ID"].as_str().unwrap(),&sha256_tagged(&cipher)).unwrap().commit().unwrap();
    assert_eq!(state(&mut b)["状態"],"削除承認あり・file不在・結果未確定");
    drop(b);let mut b=super::tests::persistent_test_broker(&audit);b.保管先起動登録(&vault,true,&[audit.clone()]).unwrap();
    assert_eq!(state(&mut b)["状態"],"削除承認あり・file不在・結果未確定");
    let pending=state(&mut b);
    let recovery=json!({"要求ID":select["要求ID"],"要求hash":select["要求hash"],"削除承認監査ID":pending["削除承認監査ID"]});
    assert!(call(&mut b,BrokerOperation::対話削除中断確認,recovery.clone(),false).body.is_none());
    for key in ["要求ID","要求hash","削除承認監査ID","extra"] {
        let mut wrong=recovery.clone();wrong[key]=json!("wrong");
        assert!(call(&mut b,BrokerOperation::対話削除中断確認,wrong,true).body.is_none());
    }
    std::fs::write(&path,&cipher).unwrap();
    assert!(call(&mut b,BrokerOperation::対話削除中断確認,recovery.clone(),true).body.is_none());
    assert_eq!(std::fs::read(&path).unwrap(),cipher);std::fs::remove_file(&path).unwrap();
    let confirmed=accepted(call(&mut b,BrokerOperation::対話削除中断確認,recovery.clone(),true));
    assert_eq!(confirmed["状態"],"削除中断照合済み");
    let current=state(&mut b);assert_eq!(current["状態"],"削除中断・復旧照合済み");assert!(current["削除結果監査ID"].is_null());assert!(!current["復旧照合監査ID"].is_null());
    assert!(call(&mut b,BrokerOperation::対話削除中断確認,recovery.clone(),true).body.is_none());
    drop(b);let mut b=super::tests::persistent_test_broker(&audit);b.保管先起動登録(&vault,true,&[audit.clone()]).unwrap();
    assert_eq!(state(&mut b)["状態"],"削除中断・復旧照合済み");
    std::fs::write(&path,&cipher).unwrap();assert_eq!(state(&mut b)["状態"],"削除承認あり・file残存");
    assert!(call(&mut b,BrokerOperation::対話内容承認,target.clone(),true).body.is_none());
    let counts=b.audit_events().iter().filter(|e|e.operation=="対話内容削除" && e.decision=="accepted").count();assert_eq!(counts,1);
    let audit_file=audit.join("audit.jsonl");std::fs::remove_file(&audit_file).unwrap();std::fs::create_dir(&audit_file).unwrap();
    let recovery_denied=b.削除中断照合確定("recovery-finalization-failure",confirmed);assert_eq!(recovery_denied.status,BrokerStatus::Suspended);assert!(recovery_denied.body.is_none());
    let denied=b.保管状態確定("inventory-finalization-failure",normal);assert_eq!(denied.status,BrokerStatus::Suspended);assert!(denied.body.is_none());
    assert!(call(&mut b,BrokerOperation::対話保管状態,select,true).body.is_none());
    drop(b);std::fs::remove_dir_all(root).unwrap();
}


#[test]
fn partial_discard_requires_latest_save_attempt_and_current_hash() {
    let root=super::tests::temp_store_dir("partial-discard");let vault=root.join("vault");std::fs::create_dir(&vault).unwrap();let audit=root.join("audit");
    let mut b=super::tests::persistent_test_broker(&audit);b.保管先起動登録(&vault,true,&[audit.clone()]).unwrap();
    let select=prepare(&mut b,false);let target=select["要求ID"].as_str().unwrap();let path=vault.join(format!("history-{target}.dpapi"));
    std::fs::write(&path,b"").unwrap();
    let state=|b:&mut Broker|accepted(call(b,BrokerOperation::対話保管状態,select.clone(),true));
    assert!(state(&mut b)["保存試行監査ID"].is_null());
    let mut payload=json!({"要求ID":target,"要求hash":select["要求hash"],"保存試行監査ID":"unknown","暗号文hash":sha256_tagged(b"")});
    assert!(call(&mut b,BrokerOperation::対話部分保存破棄,payload.clone(),true).body.is_none());assert!(path.exists());
    assert!(call(&mut b,BrokerOperation::対話内容保存,select.clone(),true).body.is_none());
    payload["保存試行監査ID"]=state(&mut b)["保存試行監査ID"].clone();assert!(!payload["保存試行監査ID"].is_null());
    assert!(call(&mut b,BrokerOperation::対話部分保存破棄,payload.clone(),false).body.is_none());
    for key in ["要求ID","要求hash","保存試行監査ID","暗号文hash","extra"] {
        let mut wrong=payload.clone();wrong[key]=json!("wrong");assert!(call(&mut b,BrokerOperation::対話部分保存破棄,wrong,true).body.is_none());assert_eq!(std::fs::read(&path).unwrap(),b"");
    }
    assert!(call(&mut b,BrokerOperation::対話内容保存,select.clone(),true).body.is_none());
    assert!(call(&mut b,BrokerOperation::対話部分保存破棄,payload.clone(),true).body.is_none());
    payload["保存試行監査ID"]=state(&mut b)["保存試行監査ID"].clone();
    std::fs::write(&path,b"partial").unwrap();assert!(call(&mut b,BrokerOperation::対話部分保存破棄,payload.clone(),true).body.is_none());assert_eq!(std::fs::read(&path).unwrap(),b"partial");
    payload["暗号文hash"]=state(&mut b)["暗号文hash"].clone();
    let result=accepted(call(&mut b,BrokerOperation::対話部分保存破棄,payload.clone(),true));assert_eq!(result["状態"],"部分保存破棄確認");assert!(!path.exists());assert_eq!(state(&mut b)["状態"],"部分保存破棄済み・file不在");
    assert!(call(&mut b,BrokerOperation::対話部分保存破棄,payload.clone(),true).body.is_none());
    assert_eq!(call(&mut b,BrokerOperation::対話内容保存,select.clone(),true).error.unwrap().code,"破棄承認済み");
    drop(b);let mut b=super::tests::persistent_test_broker(&audit);b.保管先起動登録(&vault,true,&[audit.clone()]).unwrap();
    assert_eq!(state(&mut b)["状態"],"部分保存破棄済み・file不在");std::fs::write(&path,b"partial").unwrap();assert_eq!(state(&mut b)["状態"],"部分保存破棄承認あり・file残存");
    // 準備後・承認監査前のI/O障害は、同じproduction関数でfileを保持する。
    let prepared=b.protected_store.as_ref().unwrap().prepare_delete(crate::protected_store::Purpose::History,target,&sha256_tagged(b"partial")).unwrap();
    let audit_file=audit.join("audit.jsonl");std::fs::remove_file(&audit_file).unwrap();std::fs::create_dir(&audit_file).unwrap();
    let denied=b.部分保存破棄実行("partial-approval-failure",payload.clone(),prepared,&sha256_tagged(payload.to_string().as_bytes()));assert_eq!(denied.status,BrokerStatus::Suspended);assert!(denied.body.is_none());assert_eq!(std::fs::read(&path).unwrap(),b"partial");
    let denied=b.部分保存破棄確定("partial-result-failure",result);assert_eq!(denied.status,BrokerStatus::Suspended);assert!(denied.body.is_none());
    drop(b);std::fs::remove_dir_all(root).unwrap();
}


#[test]
fn partial_discard_interruption_reconciles_current_absence_only() {
    let root=super::tests::temp_store_dir("partial-recovery");let vault=root.join("vault");std::fs::create_dir(&vault).unwrap();let audit=root.join("audit");
    let mut b=super::tests::persistent_test_broker(&audit);b.保管先起動登録(&vault,true,&[audit.clone()]).unwrap();
    let select=prepare(&mut b,false);let target=select["要求ID"].as_str().unwrap();let path=vault.join(format!("history-{target}.dpapi"));
    std::fs::write(&path,b"partial").unwrap();assert!(call(&mut b,BrokerOperation::対話内容保存,select.clone(),true).body.is_none());
    let state=|b:&mut Broker|accepted(call(b,BrokerOperation::対話保管状態,select.clone(),true));
    let current=state(&mut b);
    let intent=json!({"要求ID":target,"要求hash":select["要求hash"],"保存試行監査ID":current["保存試行監査ID"],"暗号文hash":current["暗号文hash"]});
    let encoded=intent.to_string();let hash=sha256_tagged(encoded.as_bytes());
    // 実承認監査とfile削除を行い、結果監査なしの中断状態を構成する。process強制終了証拠ではない。
    b.append_audit("partial-interrupted","対話部分保存破棄","received","Capability=対話部分保存破棄 Permission=保存試行と暗号文hash一件 Approval=現在owner破棄 RecoveryAction=保管監査再確認","INTERNAL_STATE",&hash).unwrap();
    let approval=b.append_audit("partial-interrupted","対話部分保存破棄","recorded",&format!("対話部分保存破棄承認:{encoded}"),"INTERNAL_STATE",&hash).unwrap().event_id;
    let query=json!({"要求ID":target,"要求hash":select["要求hash"],"部分保存破棄承認監査ID":approval});
    assert!(call(&mut b,BrokerOperation::対話部分破棄中断確認,query.clone(),true).body.is_none());
    b.protected_store.as_ref().unwrap().prepare_delete(crate::protected_store::Purpose::History,target,current["暗号文hash"].as_str().unwrap()).unwrap().commit().unwrap();
    drop(b);let mut b=super::tests::persistent_test_broker(&audit);b.保管先起動登録(&vault,true,&[audit.clone()]).unwrap();
    assert_eq!(state(&mut b)["状態"],"部分保存破棄・結果未確定");
    assert!(call(&mut b,BrokerOperation::対話部分破棄中断確認,query.clone(),false).body.is_none());
    for key in ["要求ID","要求hash","部分保存破棄承認監査ID","extra"] {let mut wrong=query.clone();wrong[key]=json!("wrong");assert!(call(&mut b,BrokerOperation::対話部分破棄中断確認,wrong,true).body.is_none());}
    let result=accepted(call(&mut b,BrokerOperation::対話部分破棄中断確認,query.clone(),true));assert_eq!(result["状態"],"部分破棄中断照合済み");
    assert_eq!(state(&mut b)["状態"],"部分破棄中断・復旧照合済み");assert!(state(&mut b)["部分保存破棄結果監査ID"].is_null());
    assert!(call(&mut b,BrokerOperation::対話部分破棄中断確認,query.clone(),true).body.is_none());
    drop(b);let mut b=super::tests::persistent_test_broker(&audit);b.保管先起動登録(&vault,true,&[audit.clone()]).unwrap();
    assert_eq!(state(&mut b)["状態"],"部分破棄中断・復旧照合済み");
    std::fs::write(&path,b"partial").unwrap();assert_eq!(state(&mut b)["状態"],"部分保存破棄承認あり・file残存");
    assert!(call(&mut b,BrokerOperation::対話部分破棄中断確認,query,true).body.is_none());
    assert_eq!(b.audit_events().iter().filter(|e|e.operation=="対話部分保存破棄" && e.decision=="accepted").count(),0);
    let audit_file=audit.join("audit.jsonl");std::fs::remove_file(&audit_file).unwrap();std::fs::create_dir(&audit_file).unwrap();
    let denied=b.部分破棄中断照合確定("partial-recovery-failure",result);assert_eq!(denied.status,BrokerStatus::Suspended);assert!(denied.body.is_none());
    drop(b);std::fs::remove_dir_all(root).unwrap();
}
