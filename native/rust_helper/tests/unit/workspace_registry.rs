use super::*;
fn open_scoped(path:&std::path::Path)->(Dir,Vec<super::super::workspace_root::DirectoryIdentity>) {
    let (root,_,ancestry)=super::super::workspace_root::open_isolated_root_with_ancestry(path,&[]).unwrap();
    (root,ancestry)
}

#[test]
fn dialogue_binding_exposes_only_registered_workspace_for_exact_runtime() {
    let path = std::env::temp_dir().join(format!(
        "gui-shell-dialogue-binding-{}",
        識別子生成().unwrap()
    ));
    std::fs::create_dir(&path).unwrap();
    let mut registry = WorkspaceRegistry::default();
    let secret_paths = vec!["private/credential-backup.txt".to_owned()];
    registry
        .register(
            "runtime-a",
            "workspace-a",
            Dir::open_ambient_dir(&path, cap_std::ambient_authority()).unwrap(),
            &secret_paths,
            None,
            &mut |_, _| Ok(()),
        )
        .unwrap();
    let binding = registry
        .dialogue_binding("runtime-a", "workspace-a")
        .expect("登録済みの同一Runtime");
    assert!(binding.matches("runtime-a", "workspace-a"));
    assert_eq!(
        binding.secret_paths(),
        &["private/credential-backup.txt".to_owned()]
    );
    assert!(!binding.matches("runtime-b", "workspace-a"));
    assert!(registry
        .dialogue_binding("runtime-b", "workspace-a")
        .is_none());
    assert!(registry
        .dialogue_binding("runtime-a", "unknown-workspace")
        .is_none());
    assert!(binding.audit_hash("session-a").starts_with("sha256:"));
    drop(registry);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn 別Agentの同一rootと親子rootを拒否し独立rootを許可する() {
    let shared=std::env::temp_dir().join(format!("gui-shell-agent-shared-root-{}",識別子生成().unwrap()));
    let child=shared.join("child");
    let isolated=std::env::temp_dir().join(format!("gui-shell-agent-isolated-root-{}",識別子生成().unwrap()));
    std::fs::create_dir_all(&child).unwrap();
    std::fs::create_dir(&isolated).unwrap();
    let mut registry=WorkspaceRegistry::default();
    let mut audit_calls=0;
    let (shared_root,shared_ancestry)=open_scoped(&shared);
    registry.register(
        "agent-runtime-a",
        "workspace-a",
        shared_root,
        &[],
        Some(shared_ancestry),
        &mut |_,_| {audit_calls+=1;Ok(())},
    ).unwrap();
    let (same_root,same_ancestry)=open_scoped(&shared);
    assert_eq!(
        registry.register(
            "agent-runtime-b",
            "workspace-b",
            same_root,
            &[],
            Some(same_ancestry),
            &mut |_,_| {audit_calls+=1;Ok(())},
        ),
        Err("別Agentとの同一物理作業領域共有を拒否"),
    );
    let (child_root,child_ancestry)=open_scoped(&child);
    assert_eq!(
        registry.register(
            "agent-runtime-b",
            "workspace-b",
            child_root,
            &[],
            Some(child_ancestry),
            &mut |_,_| {audit_calls+=1;Ok(())},
        ),
        Err("別Agentとの親子Workspace範囲重複を拒否"),
    );
    assert_eq!(audit_calls,1,"成功登録として記録するcallbackは拒否時に呼ばれない");
    let (isolated_root,isolated_ancestry)=open_scoped(&isolated);
    registry.register(
        "agent-runtime-b",
        "workspace-b",
        isolated_root,
        &[],
        Some(isolated_ancestry),
        &mut |_,_| {audit_calls+=1;Ok(())},
    ).unwrap();
    assert_eq!(registry.entries.len(),2);
    drop(registry);

    let mut reverse=WorkspaceRegistry::default();
    let (child_root,child_ancestry)=open_scoped(&child);
    reverse.register("agent-runtime-a","child-first",child_root,&[],Some(child_ancestry),&mut |_,_|Ok(())).unwrap();
    let (parent_root,parent_ancestry)=open_scoped(&shared);
    assert_eq!(
        reverse.register("agent-runtime-b","parent-second",parent_root,&[],Some(parent_ancestry),&mut |_,_|Ok(())),
        Err("別Agentとの親子Workspace範囲重複を拒否"),
    );
    drop(reverse);

    let mut malformed=WorkspaceRegistry::default();
    let (root_handle,root_ancestry)=open_scoped(&isolated);
    let root_identity=*root_ancestry.last().unwrap();
    assert_eq!(
        malformed.register("agent-runtime-a","root-only",root_handle,&[],Some(vec![root_identity]),&mut |_,_|Ok(())),
        Err("作業領域の親子識別範囲が不正または未観測"),
    );
    drop(malformed);

    let mut incomplete=WorkspaceRegistry::default();
    incomplete.register(
        "agent-runtime-a","handle-only",Dir::open_ambient_dir(&isolated,cap_std::ambient_authority()).unwrap(),&[],None,&mut |_,_|Ok(()),
    ).unwrap();
    let (independent_root,independent_ancestry)=open_scoped(&shared);
    assert_eq!(
        incomplete.register("agent-runtime-b","cannot-prove",independent_root,&[],Some(independent_ancestry),&mut |_,_|Ok(())),
        Err("別AgentとのWorkspace分離範囲を確認できないため登録を拒否"),
    );
    drop(incomplete);
    std::fs::remove_dir_all(shared).unwrap();
    std::fs::remove_dir_all(isolated).unwrap();
}

#[test]
fn failed_grant_audit_never_enables_read_and_failed_result_audit_returns_no_body() {
    let path=std::env::temp_dir().join(format!("gui-shell-workspace-audit-{}",識別子生成().unwrap()));
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("file.txt"),"監査後だけ返す本文").unwrap();
    let root=Dir::open_ambient_dir(&path,cap_std::ambient_authority()).unwrap();
    let mut registry=WorkspaceRegistry::default();
    registry.register("runtime-a","workspace-a",root,&[],None,&mut |_,_|Ok(())).unwrap();
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
    registry.register("runtime-a","workspace-a",Dir::open_ambient_dir(&path,cap_std::ambient_authority()).unwrap(),&[],None,&mut |_,_|Ok(())).unwrap();
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

#[test]
fn whole_baseline_handles_empty_scope_bounds_retention_and_drops_failed_capture() {
    let path=std::env::temp_dir().join(format!("gui-shell-whole-{}",識別子生成().unwrap()));
    std::fs::create_dir(&path).unwrap();
    let mut registry=WorkspaceRegistry::default();
    registry.register("runtime-a","workspace-a",Dir::open_ambient_dir(&path,cap_std::ambient_authority()).unwrap(),&[],None,&mut |_,_|Ok(())).unwrap();
    let hash=registry.entries["workspace-a"].registration_hash.clone();
    registry.operate("作業領域承認",&json!({"作業領域ID":"workspace-a","登録hash":hash,"表示範囲":"full"}),true,100,&mut |_,_|Ok(())).unwrap();
    let capture=json!({"作業領域ID":"workspace-a","登録hash":hash});
    let empty=registry.operate("作業領域全体基準点保存",&capture,true,100,&mut |_,_|Ok(())).unwrap();
    assert_eq!(empty["projection"]["対象数"],0);
    let selection=json!({"作業領域ID":"workspace-a","相対path":"","基準点hash":empty["projection"]["基準点hash"]});
    assert_eq!(registry.operate("作業領域変更一覧",&selection,false,100,&mut |_,_|Ok(())).unwrap()["projection"]["changes"],json!([]));
    for i in 0..129 {std::fs::write(path.join(format!("file-{i:03}")),vec![b'a';65536]).unwrap();}
    registry.operate("作業領域全体基準点保存",&capture,true,100,&mut |_,_|Ok(())).unwrap();
    let baseline=registry.entries["workspace-a"].baseline.as_ref().unwrap();
    assert_eq!(baseline.files.len(),129);
    let retained:usize=baseline.files.values().map(|v|v.as_ref().unwrap().content.as_ref().map(|b|b.len()).unwrap_or(0)).sum();
    assert_eq!(retained,8*1024*1024);
    assert!(registry.operate("作業領域全体基準点保存",&capture,true,100,&mut |reason,_| {
        if reason=="指定範囲の基準点を確定" {Err("試験監査障害")} else {Ok(())}
    }).is_err());
    assert!(registry.entries["workspace-a"].baseline.is_none());
    drop(registry);std::fs::remove_dir_all(path).unwrap();
}
