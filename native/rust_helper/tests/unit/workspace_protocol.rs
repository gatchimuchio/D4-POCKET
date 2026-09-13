use super::*;
use serde_json::json;
use std::{fs, path::PathBuf};
const REGISTERED_RUNTIME: &str = "gui_shell_rust_broker";

struct Fixture {broker: Option<Broker>, root: PathBuf}
impl Drop for Fixture {
    fn drop(&mut self) {self.broker.take();let _ = fs::remove_dir_all(&self.root);}
}
fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!("gui-shell-workspace-broker-{}", crate::broker::dialogue::識別子生成().unwrap()));
    fs::create_dir_all(root.join("workspace")).unwrap();
    fs::write(root.join("workspace/private-document.txt"), "内部本文\r\n").unwrap();
    fs::write(root.join("workspace/.env"), "秘密の資格").unwrap();
    fs::write(root.join("workspace/data.bin"), [0, 255, 1]).unwrap();
    let mut broker = Broker::new_persistent("workspace-test", root.join("store")).unwrap();
    broker.current_epoch_seconds_override = Some(1789280000);
    let dir = cap_std::fs::Dir::open_ambient_dir(root.join("workspace"),cap_std::ambient_authority()).unwrap();
    broker.作業領域登録(REGISTERED_RUNTIME,"workspace-a",dir,&[]).unwrap();
    Fixture {broker:Some(broker),root}
}
fn request(b: &mut Broker, op: &str, payload: Value, owner: bool) -> BrokerResponse {
    let id = crate::broker::dialogue::識別子生成().unwrap();
    let value = json!({"request_id":id,"nonce":id,"session_id":"workspace-test","operation":op,
        "issued_at":epoch_seconds_to_rfc3339(b.current_epoch_seconds()),"metadata":{},"payload_hash":canonical_payload_hash(Some(&payload)),"payload":payload});
    if owner {b.owner要求処理(&value.to_string())} else {b.handle_json(&value.to_string())}
}
fn registration(b: &mut Broker) -> Value {
    let r = request(b,"作業領域一覧",json!({}),false);
    assert_eq!(r.status,BrokerStatus::Accepted);
    r.body.unwrap()["作業領域"][0]["登録hash"].clone()
}
fn approve(b: &mut Broker, visibility: &str) -> Value {
    let hash = registration(b);
    let result = request(b,"作業領域承認",json!({"作業領域ID":"workspace-a","登録hash":hash,"表示範囲":visibility}),true);
    assert_eq!(result.status,BrokerStatus::Accepted);
    result.body.unwrap()
}
fn read(b: &mut Broker, path: &str) -> BrokerResponse {
    request(b,"作業領域読取",json!({"作業領域ID":"workspace-a","相対path":path}),false)
}

#[test]
fn workspace_requires_current_owner_grant_and_binds_real_read_to_durable_audit() {
    let mut f=fixture();let b=f.broker.as_mut().unwrap();
    assert_ne!(read(b,"private-document.txt").status,BrokerStatus::Accepted);
    let hash=registration(b);
    let p=json!({"作業領域ID":"workspace-a","登録hash":hash,"表示範囲":"full"});
    assert_eq!(request(b,"作業領域承認",p.clone(),false).error.unwrap().code,"権限拒否");
    let mut wrong=p.clone();wrong["登録hash"]=json!("sha256:".to_string()+&"0".repeat(64));
    assert_ne!(request(b,"作業領域承認",wrong,true).status,BrokerStatus::Accepted);
    let mut injected=p;injected["root"]=json!(f.root);
    assert_ne!(request(b,"作業領域承認",injected,true).status,BrokerStatus::Accepted);
    let grant=approve(b,"full");
    let r=read(b,"private-document.txt");
    assert_eq!(r.status,BrokerStatus::Accepted);assert_eq!(r.evidence_source,"LIVE_RUNTIME");
    assert_eq!(r.body.as_ref().unwrap()["projection"]["text"],"内部本文\r\n");
    assert_eq!(r.body.as_ref().unwrap()["approval_id"],grant["approval_id"]);
    let audit_id=r.audit_event_id.clone();
    let tree=request(b,"作業領域ツリー",json!({"作業領域ID":"workspace-a","相対path":""}),false);
    assert_eq!(tree.status,BrokerStatus::Accepted);assert!(!tree.to_json_string().unwrap().contains(".env"));
    for path in ["../private-document.txt",".env","","private-document.txt:stream"] {
        assert_ne!(read(b,path).status,BrokerStatus::Accepted);
    }
    let revoke=json!({"作業領域ID":"workspace-a","登録hash":registration(b)});
    assert_ne!(request(b,"作業領域失効",revoke.clone(),false).status,BrokerStatus::Accepted);
    assert_eq!(request(b,"作業領域失効",revoke,true).status,BrokerStatus::Accepted);
    assert_ne!(read(b,"private-document.txt").status,BrokerStatus::Accepted);
    let (_,state)=BrokerPersistentStore::open_or_create(f.root.join("store"),"audit-reopen").unwrap();
    let event=state.audit_log.events().iter().find(|e|e.event_id==audit_id).unwrap();
    assert_eq!(event.payload_hash,sha256_tagged(r.body.unwrap().to_string().as_bytes()));
    let raw=fs::read_to_string(f.root.join("store/audit.jsonl")).unwrap();
    for text in ["内部本文","秘密の資格","private-document.txt",".env"] {assert!(!raw.contains(text));}
}

#[test]
fn workspace_visibility_binary_expiry_and_clock_rollback_do_not_leak_or_restore_grants() {
    let mut f=fixture();let b=f.broker.as_mut().unwrap();
    for visibility in ["none","summary","redacted","hash_only","full"] {
        approve(b,visibility);
        let r=read(b,"private-document.txt");assert_eq!(r.status,BrokerStatus::Accepted);
        let body=r.body.unwrap();
        assert_eq!(body.to_string().contains("内部本文"),visibility=="full");
        if visibility!="full" {assert!(!body.to_string().contains("private-document.txt"));}
        if visibility=="none" {assert!(body["projection"].is_null());}
        if visibility=="hash_only" {assert_eq!(body["projection"].as_object().unwrap().len(),1);}
        assert_ne!(read(b,"").status,BrokerStatus::Accepted);
        assert_ne!(read(b,".env").status,BrokerStatus::Accepted);
    }
    let binary=read(b,"data.bin").body.unwrap();
    assert_eq!(binary["projection"]["binary"],true);assert!(binary["projection"]["text"].is_null());
    b.current_epoch_seconds_override=Some(1789280300);
    assert_ne!(read(b,"private-document.txt").status,BrokerStatus::Accepted);
    b.current_epoch_seconds_override=Some(1789280000);
    assert_ne!(read(b,"private-document.txt").status,BrokerStatus::Accepted);
    b.current_epoch_seconds_override=Some(1789280300);
    assert_ne!(read(b,"private-document.txt").status,BrokerStatus::Accepted);
    approve(b,"full");assert_eq!(read(b,"private-document.txt").status,BrokerStatus::Accepted);
    let restarted=Broker::new_persistent("workspace-test",f.root.join("store")).unwrap();
    *b=restarted;
    assert_ne!(read(b,"private-document.txt").status,BrokerStatus::Accepted);
}

#[test]
fn workspace_audit_failure_drops_all_grants_and_roots_before_any_content_response() {
    let mut f=fixture();let b=f.broker.as_mut().unwrap();approve(b,"full");
    let audit=f.root.join("store/audit.jsonl");let saved=f.root.join("store/audit.saved");
    fs::rename(&audit,&saved).unwrap();fs::create_dir(&audit).unwrap();
    let r=read(b,"private-document.txt");assert_ne!(r.status,BrokerStatus::Accepted);assert!(r.body.is_none());
    fs::remove_dir(&audit).unwrap();fs::rename(&saved,&audit).unwrap();
    assert_ne!(read(b,"private-document.txt").status,BrokerStatus::Accepted);
    let list=request(b,"作業領域一覧",json!({}),false);assert_eq!(list.body.unwrap()["作業領域"],json!([]));
}

#[test]
fn workspace_registration_rejects_unknown_runtime_duplicate_and_nonpersistent_store() {
    let mut f=fixture();let b=f.broker.as_mut().unwrap();
    let open=||cap_std::fs::Dir::open_ambient_dir(f.root.join("workspace"),cap_std::ambient_authority()).unwrap();
    assert!(b.作業領域登録("unknown","workspace-b",open(),&[]).is_err());
    assert!(b.作業領域登録(BROKER_ID,"workspace-b",open(),&[]).is_err());
    assert!(b.作業領域登録(REGISTERED_RUNTIME,"workspace-a",open(),&[]).is_err());
    assert!(Broker::new("skeleton").作業領域登録(REGISTERED_RUNTIME,"workspace-a",open(),&[]).is_err());
}

#[test]
fn workspace_duplicate_fields_in_raw_json_are_rejected_before_permission() {
    let mut f=fixture();let b=f.broker.as_mut().unwrap();approve(b,"full");
    let payload=json!({"作業領域ID":"workspace-a","相対path":"private-document.txt"});
    let value=json!({"request_id":"duplicate","nonce":"duplicate","session_id":"workspace-test","operation":"作業領域読取",
        "issued_at":epoch_seconds_to_rfc3339(b.current_epoch_seconds()),"metadata":{},"payload_hash":canonical_payload_hash(Some(&payload)),"payload":payload});
    let raw=value.to_string().replace("\"相対path\":\"private-document.txt\"", "\"相対path\":\".env\",\"相対path\":\"private-document.txt\"");
    let result=b.handle_json(&raw);
    assert_eq!(result.status,BrokerStatus::Rejected);assert!(result.body.is_none());
    assert_eq!(result.error.unwrap().code,"broker_request_malformed");
    assert_eq!(b.audit_events().last().unwrap().payload_hash,sha256_tagged(raw.as_bytes()));
    let result=b.owner要求処理(&raw);
    assert_eq!(result.status,BrokerStatus::Rejected);
    assert_eq!(b.audit_events().last().unwrap().payload_hash,sha256_tagged(raw.as_bytes()));
}
