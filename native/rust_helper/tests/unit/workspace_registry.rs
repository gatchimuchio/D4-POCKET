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
