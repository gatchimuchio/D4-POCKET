use super::*;
#[test]
fn failed_grant_audit_never_enables_read_and_failed_result_audit_returns_no_body() {
    let path=std::env::temp_dir().join(format!("gui-shell-workspace-audit-{}",識別子生成().unwrap()));
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("file.txt"),"監査後だけ返す本文").unwrap();
    let root=Dir::open_ambient_dir(&path,cap_std::ambient_authority()).unwrap();
    let mut registry=WorkspaceRegistry::default();
    registry.register("runtime-a","workspace-a",root,&[],&mut |_,_|Ok(())).unwrap();
    let hash=registry.entries["workspace-a"].registration_hash.clone();
    let approval=json!({"作業領域ID":"workspace-a","登録hash":hash,"表示範囲":"full"});
    assert!(registry.operate("作業領域承認",&approval,true,100,&mut |_,_|Err("試験監査障害")).is_err());
    let read=json!({"作業領域ID":"workspace-a","相対path":"file.txt"});
    assert!(registry.operate("作業領域読取",&read,false,100,&mut |_,_|panic!("未承認で取得へ到達")).is_err());
    registry.operate("作業領域承認",&approval,true,100,&mut |_,_|Ok(())).unwrap();
    let mut result_seen=false;
    let result=registry.operate("作業領域読取",&read,false,100,&mut |reason,hash| {
        assert!(hash.starts_with("sha256:"));
        if reason=="作業領域の取得projectionを確定" {result_seen=true;Err("試験監査障害")} else {Ok(())}
    });
    assert!(result_seen);assert_eq!(result,Err("試験監査障害"));
    registry.entries.get_mut("workspace-a").unwrap().grant.as_mut().unwrap().deadline=Instant::now()+Duration::from_millis(100);
    let result=registry.operate("作業領域読取",&read,false,100,&mut |reason,_| {
        if reason=="作業領域の取得projectionを確定" {std::thread::sleep(Duration::from_millis(150));}
        Ok(())
    });
    assert_eq!(result,Err("取得中に読取承認が期限超過"));
    drop(registry);std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn baseline_capture_audit_failure_and_expiry_never_publish_old_content() {
    let path=std::env::temp_dir().join(format!("gui-shell-baseline-{}",識別子生成().unwrap()));
    std::fs::create_dir(&path).unwrap();std::fs::write(path.join("file.txt"),"比較前").unwrap();
    let mut registry=WorkspaceRegistry::default();
    registry.register("runtime-a","workspace-a",Dir::open_ambient_dir(&path,cap_std::ambient_authority()).unwrap(),&[],&mut |_,_|Ok(())).unwrap();
    let hash=registry.entries["workspace-a"].registration_hash.clone();
    let grant=json!({"作業領域ID":"workspace-a","登録hash":hash,"表示範囲":"full"});
    registry.operate("作業領域承認",&grant,true,100,&mut |_,_|Ok(())).unwrap();
    let capture=json!({"作業領域ID":"workspace-a","登録hash":hash,"相対paths":["file.txt"]});
    assert!(registry.operate("作業領域基準点保存",&capture,true,100,&mut |reason,_|if reason=="指定範囲の基準点を確定" {Err("監査障害")}else{Ok(())}).is_err());
    assert!(registry.entries["workspace-a"].baseline.is_none());
    let receipt=registry.operate("作業領域基準点保存",&capture,true,100,&mut |_,_|Ok(())).unwrap();
    let diff=json!({"作業領域ID":"workspace-a","相対path":"file.txt","基準点hash":receipt["projection"]["基準点hash"]});
    std::fs::write(path.join("file.txt"),b"\0binary").unwrap();
    let binary=registry.operate("作業領域差分",&diff,false,100,&mut |_,_|Ok(())).unwrap();
    assert_eq!(binary["projection"]["diff"]["kind"],"binary");assert!(binary["projection"]["diff"]["unified"].is_null());
    std::fs::write(path.join("file.txt"),vec![b'x';65_537]).unwrap();
    let large=registry.operate("作業領域差分",&diff,false,100,&mut |_,_|Ok(())).unwrap();
    assert_eq!(large["projection"]["diff"]["kind"],"oversized");
    assert!(large["projection"]["diff"]["unified"].is_null());
    std::fs::OpenOptions::new().write(true).open(path.join("file.txt")).unwrap().set_len(crate::workspace_reader::MAX_COMPARISON_BYTES+1).unwrap();
    assert!(registry.operate("作業領域基準点保存",&capture,true,100,&mut |_,_|Ok(())).is_err());
    assert!(registry.entries["workspace-a"].baseline.is_none());
    std::fs::write(path.join("file.txt"),b"small").unwrap();
    registry.operate("作業領域基準点保存",&capture,true,100,&mut |_,_|Ok(())).unwrap();
    assert!(registry.operate("作業領域差分",&diff,false,400,&mut |_,_|Ok(())).is_err());
    assert!(registry.operate("作業領域差分",&diff,false,100,&mut |_,_|Ok(())).is_err());
    assert!(registry.entries["workspace-a"].baseline.is_none());
    drop(registry);std::fs::remove_dir_all(path).unwrap();
}
