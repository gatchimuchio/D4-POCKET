use gui_shell_rust_helper::checkpoint::*;
use gui_shell_rust_helper::audit_hash::sha256_tagged;
use gui_shell_rust_helper::broker::audit::BrokerAuditLog;
use ring::{rand::SystemRandom, signature::{Ed25519KeyPair, KeyPair}};

fn key() -> (Ed25519KeyPair, Vec<u8>, Trust) {
    // 実owner鍵ではない。試験processのメモリ外へ秘密鍵を出さない。
    let doc = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
    let key = Ed25519KeyPair::from_pkcs8(doc.as_ref()).unwrap();
    let mut der = hex::decode("302a300506032b6570032100").unwrap();
    der.extend_from_slice(key.public_key().as_ref());
    let trust = Trust {version:1,algorithm:"Ed25519".into(),public_key_fingerprint:Some(sha256_tagged(key.public_key().as_ref()))};
    (key,der,trust)
}
fn installation(root: &std::path::Path) {
    for path in ["app/gui_shell_desktop.exe", "app/data/app.so", "app/flutter_windows.dll", "broker/gui_shell_rust_helper.exe", "gui_shell_desktop_launcher.exe"] {
        let file = root.join(path); std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, path.as_bytes()).unwrap();
    }
}
fn checkpoint() -> Checkpoint {
    Checkpoint {format:"gui-shell-audit-checkpoint".into(),version:2,audit_chain_head:Some(sha256_tagged(b"head")),
        audit_log_sha256:sha256_tagged(b"log"),audit_anchor_sha256:sha256_tagged(b"anchor"),source_commit:"a".repeat(40),
        installed_artifact_sha256:sha256_tagged(b"exe"),generated_at:1000,sequence:1,previous_checkpoint_hash:None}
}
#[test]
fn 正常署名と継続と同一最新証拠を受理する() {
    let (key,der,trust)=key(); let cp=checkpoint(); let b=canonical(&cp).unwrap(); let s=key.sign(&b);
    let zero=TrustedHead{version:1,sequence:0,signed_checkpoint_hash:None};
    let one=verify(&b,s.as_ref(),&der,&trust,&zero,None,&cp,1000).unwrap();
    verify(&b,s.as_ref(),&der,&trust,&one,None,&cp,1000).unwrap();
    let mut next=cp.clone(); next.sequence=2; next.previous_checkpoint_hash=one.signed_checkpoint_hash.clone();
    let nb=canonical(&next).unwrap();let ns=key.sign(&nb);
    verify(&nb,ns.as_ref(),&der,&trust,&one,Some((&b,s.as_ref())),&next,1000).unwrap();
}
#[test]
fn 不正署名と別鍵と未設定信頼とcanonical改変を拒否する() {
    let (key,der,trust)=key();let cp=checkpoint();let b=canonical(&cp).unwrap();let s=key.sign(&b);
    assert!(signature_check(&b,&[],&der,&trust).is_err());
    let mut bad=s.as_ref().to_vec();bad[0]^=1;
    assert!(signature_check(&b,&bad,&der,&trust).is_err());
    let (other,other_der,_)=self::key();
    assert!(signature_check(&b,other.sign(&b).as_ref(),&other_der,&trust).is_err());
    let empty=Trust{version:1,algorithm:"Ed25519".into(),public_key_fingerprint:None};
    assert!(signature_check(&b,s.as_ref(),&der,&empty).is_err());
    let mut altered=b.clone();altered[0]=b'[';
    assert!(signature_check(&altered,s.as_ref(),&der,&trust).is_err());
    let spaced=[b.as_slice(),b"\n"].concat();
    assert!(signature_check(&spaced,key.sign(&spaced).as_ref(),&der,&trust).is_err());
    let duplicated=String::from_utf8(b.clone()).unwrap().replacen("{", "{\"version\":1,",1);
    assert!(signature_check(duplicated.as_bytes(),key.sign(duplicated.as_bytes()).as_ref(),&der,&trust).is_err());
}
#[test]
fn 現在logとanchorとheadとsourceとartifactの不一致をすべて拒否する() {
    let (key,der,trust)=key();let cp=checkpoint();let b=canonical(&cp).unwrap();let s=key.sign(&b);
    let zero=TrustedHead{version:1,sequence:0,signed_checkpoint_hash:None};
    for field in 0..5 {
        let mut current=cp.clone();
        match field {0=>current.audit_log_sha256=sha256_tagged(b"altered"),1=>current.audit_anchor_sha256=sha256_tagged(b"altered"),
            2=>current.audit_chain_head=Some(sha256_tagged(b"altered")),3=>current.source_commit="b".repeat(40),_=>current.installed_artifact_sha256=sha256_tagged(b"altered")};
        assert!(verify(&b,s.as_ref(),&der,&trust,&zero,None,&current,1000).is_err(),"field {field}");
    }
    assert!(verify(&b,s.as_ref(),&der,&trust,&zero,None,&cp,90000).is_err());
}
#[test]
fn 前checkpoint不一致とsequence巻戻しと同番号置換を拒否する() {
    let (key,der,trust)=key();let cp=checkpoint();let b=canonical(&cp).unwrap();let s=key.sign(&b);
    let one=TrustedHead{version:1,sequence:1,signed_checkpoint_hash:Some(signed_hash(&b,s.as_ref()))};
    let mut next=cp.clone();next.sequence=2;next.previous_checkpoint_hash=Some(sha256_tagged(b"wrong"));
    let nb=canonical(&next).unwrap();let ns=key.sign(&nb);
    assert!(verify(&nb,ns.as_ref(),&der,&trust,&one,Some((&b,s.as_ref())),&next,1000).is_err());
    let newer=TrustedHead{version:1,sequence:2,signed_checkpoint_hash:Some(sha256_tagged(b"newer"))};
    assert!(verify(&b,s.as_ref(),&der,&trust,&newer,None,&cp,1000).is_err());
    let mut replacement=cp.clone();replacement.generated_at=1001;
    let rb=canonical(&replacement).unwrap();
    assert!(verify(&rb,key.sign(&rb).as_ref(),&der,&trust,&one,None,&replacement,1001).is_err());
}
#[test]
fn 実fileの改変と不正chainを検出する() {
    let mut id=[0u8;16];getrandom::getrandom(&mut id).unwrap();
    let root=std::env::temp_dir().join(format!("gui-shell-checkpoint-{}",hex::encode(id)));
    std::fs::create_dir(&root).unwrap();
    let mut log=BrokerAuditLog::default();let event=log.append("test","health","accepted","試験","FIXTURE",&sha256_tagged(b"null"));
    let log_bytes=serde_json::to_vec(&event).unwrap();
    let anchor=serde_json::json!({"version":1,"event_count":1,"head_event_hash":event.event_hash,"anchor_hmac":sha256_tagged(b"fixture")});
    std::fs::write(root.join("audit.jsonl"),&log_bytes).unwrap();
    std::fs::write(root.join("audit_anchor.json"),serde_json::to_vec(&anchor).unwrap()).unwrap();
    installation(&root.join("installed"));
    let cp=measure(&root,&root.join("installed"),&"a".repeat(40),1,None,1000).unwrap();
    let (key,der,trust)=key();let b=canonical(&cp).unwrap();let sig=key.sign(&b);let zero=TrustedHead{version:1,sequence:0,signed_checkpoint_hash:None};
    std::fs::write(root.join("audit.jsonl"),[log_bytes.as_slice(),b"\n"].concat()).unwrap();
    let changed=measure(&root,&root.join("installed"),&"a".repeat(40),1,None,1000).unwrap();
    assert!(verify(&b,sig.as_ref(),&der,&trust,&zero,None,&changed,1000).is_err());
    std::fs::write(root.join("audit.jsonl"),&log_bytes).unwrap();
    std::fs::write(root.join("audit_anchor.json"),serde_json::to_vec_pretty(&anchor).unwrap()).unwrap();
    let changed=measure(&root,&root.join("installed"),&"a".repeat(40),1,None,1000).unwrap();
    assert!(verify(&b,sig.as_ref(),&der,&trust,&zero,None,&changed,1000).is_err());
    std::fs::write(root.join("audit.jsonl"),b"{\"event_hash\":\"forged\"}").unwrap();
    assert!(measure(&root,&root.join("installed"),&"a".repeat(40),1,None,1000).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[cfg(windows)]
fn 実コマンドと収集器と再検証を接続し改変を拒否する() {
    use std::process::Command;
    use std::path::Path;
    let repository=Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let mut id=[0u8;16];getrandom::getrandom(&mut id).unwrap();
    let root=std::env::temp_dir().join(format!("gui-shell-checkpoint-integration-{}",hex::encode(id)));
    let store=root.join("installed/runtime/store"); let bundle=root.join("bundle");
    for dir in [&store,&bundle,&root.join("config"),&root.join("installer/windows"),&root.join("native/rust_helper/target/debug")] {std::fs::create_dir_all(dir).unwrap();}
    let binary=root.join("native/rust_helper/target/debug/gui_shell_rust_helper.exe");
    std::fs::copy(env!("CARGO_BIN_EXE_gui_shell_rust_helper"),&binary).unwrap();
    std::fs::copy(repository.join("installer/windows/collect_audit_anchor_proof.ps1"),root.join("installer/windows/collect_audit_anchor_proof.ps1")).unwrap();
    let git=Command::new("git").args(["rev-parse","HEAD"]).current_dir(repository).output().unwrap();assert!(git.status.success());
    let source=String::from_utf8(git.stdout).unwrap().trim().to_owned();
    installation(&root.join("installed"));
    let artifact=root.join("installed/app/gui_shell_desktop.exe");std::fs::write(&artifact,b"fixture installed artifact").unwrap();
    let mut log=BrokerAuditLog::default();let event=log.append("integration","health","accepted","試験","FIXTURE",&sha256_tagged(b"null"));
    std::fs::write(store.join("audit.jsonl"),serde_json::to_vec(&event).unwrap()).unwrap();
    std::fs::write(store.join("audit_anchor.json"),serde_json::to_vec(&serde_json::json!({"version":1,"event_count":1,"head_event_hash":event.event_hash,"anchor_hmac":sha256_tagged(b"fixture")})).unwrap()).unwrap();
    std::fs::write(store.join("audit_anchor.key"),b"fixture only; not a signing key").unwrap();
    std::fs::write(root.join("installed/installed_manifest.json"),serde_json::to_vec(&serde_json::json!({"source_commit":source,"source_worktree_clean":true,"app_exe":artifact,"app_artifact_sha256":sha256_tagged(b"fixture installed artifact")})).unwrap()).unwrap();
    let floor=root.join("trusted-head.json");std::fs::write(&floor,b"{\"version\":1,\"sequence\":0,\"signed_checkpoint_hash\":null}").unwrap();
    let prepared=Command::new(&binary).args(["監査チェックポイント","prepare"]).arg(&store).arg(root.join("installed")).arg(&source).arg(&floor).arg(bundle.join("checkpoint.json")).output().unwrap();
    assert!(prepared.status.success(),"{}",String::from_utf8_lossy(&prepared.stderr));
    let (key,der,trust)=key();let bytes=std::fs::read(bundle.join("checkpoint.json")).unwrap();
    std::fs::write(bundle.join("signature.bin"),key.sign(&bytes).as_ref()).unwrap();
    std::fs::write(bundle.join("public-key.der"),der).unwrap();
    let standard=Command::new("openssl").args(["pkeyutl","-verify","-pubin","-keyform","DER","-rawin","-inkey"]).arg(bundle.join("public-key.der"))
        .arg("-in").arg(bundle.join("checkpoint.json")).arg("-sigfile").arg(bundle.join("signature.bin")).output().unwrap();
    assert!(standard.status.success(),"{}",String::from_utf8_lossy(&standard.stderr));
    std::fs::write(root.join("config/audit_signing_trust.json"),serde_json::to_vec(&serde_json::json!({"version":1,"algorithm":"Ed25519","public_key_fingerprint":trust.public_key_fingerprint})).unwrap()).unwrap();
    let proof=root.join("proof.json");
    let result=Command::new("pwsh").args(["-NoProfile","-File"]).arg(root.join("installer/windows/collect_audit_anchor_proof.ps1"))
        .arg("-InstalledRoot").arg(root.join("installed")).arg("-AuditDir").arg(&store).arg("-OutputPath").arg(&proof)
        .arg("-CheckpointBundle").arg(&bundle).arg("-TrustedHeadPath").arg(&floor)
        .env("GIT_DIR",repository.join(".git")).output().unwrap();
    assert!(result.status.success(),"{}",String::from_utf8_lossy(&result.stderr));
    let report:serde_json::Value=serde_json::from_slice(&std::fs::read(&proof).unwrap()).unwrap();
    assert_eq!(report["status"],"passed","{report}");assert_eq!(report["signed_evidence_verified"],true);assert_eq!(report["key_anchor_log_same_user_rewrite_mitigated"],true);
    let script="import sys,json;from pathlib import Path;sys.path.insert(0,sys.argv[1]);from tooling import audit_checkpoint_verifier as v;v.ROOT=Path(sys.argv[2]);v.verify_collected(json.loads(Path(sys.argv[3]).read_text(encoding='utf-8-sig')))";
    let run=||Command::new("python").args(["-c",script]).arg(repository).arg(&root).arg(&proof).env("GIT_DIR",repository.join(".git")).env("GUI_SHELL_AUDIT_TRUSTED_HEAD",&floor).output().unwrap();
    let verified=run();assert!(verified.status.success(),"{}",String::from_utf8_lossy(&verified.stderr));
    // runner exeは変更せず、AOTだけの改変をrelease再検証で拒否する。
    std::fs::write(root.join("installed/app/data/app.so"),b"tampered after collection").unwrap();
    assert!(!run().status.success());
    std::fs::remove_dir_all(root).unwrap();
}


#[test]
fn 配布物の変更追加削除とpath変更と旧署名を拒否する() {
    let mut id=[0u8;16];getrandom::getrandom(&mut id).unwrap();
    let root=std::env::temp_dir().join(format!("gui-shell-artifact-{}",hex::encode(id)));
    installation(&root);
    let initial=gui_shell_rust_helper::installed_artifact::canonical(&root).unwrap();
    let (key,der,trust)=key(); let mut cp=checkpoint(); cp.installed_artifact_sha256=sha256_tagged(&initial);
    let bytes=canonical(&cp).unwrap(); let sig=key.sign(&bytes);let floor=TrustedHead{version:1,sequence:0,signed_checkpoint_hash:None};
    let rejects=|| {
        if let Ok(current)=gui_shell_rust_helper::installed_artifact::canonical(&root) {
            let mut measured=cp.clone();measured.installed_artifact_sha256=sha256_tagged(&current);
            assert!(verify(&bytes,sig.as_ref(),&der,&trust,&floor,None,&measured,1000).is_err());
        }
    };
    for path in ["app/data/app.so","app/flutter_windows.dll","broker/gui_shell_rust_helper.exe","gui_shell_desktop_launcher.exe"] {
        let p=root.join(path);let before=std::fs::read(&p).unwrap();
        std::fs::write(&p,b"altered").unwrap();rejects();std::fs::write(&p,&before).unwrap();
        std::fs::remove_file(&p).unwrap();rejects();std::fs::write(&p,&before).unwrap();
    }
    let plugin=root.join("app/plugin.dll");std::fs::write(&plugin,b"added").unwrap();rejects();std::fs::remove_file(&plugin).unwrap();
    std::fs::rename(root.join("app/data/app.so"),root.join("app/data/renamed.so")).unwrap();rejects();
    std::fs::rename(root.join("app/data/renamed.so"),root.join("app/data/app.so")).unwrap();
    std::fs::write(root.join("outside.dll"),b"extra root").unwrap();rejects();std::fs::remove_file(root.join("outside.dll")).unwrap();
    std::fs::create_dir(root.join("runtime")).unwrap();std::fs::write(root.join("runtime/state"),b"mutable").unwrap();
    assert_eq!(gui_shell_rust_helper::installed_artifact::canonical(&root).unwrap(),initial);
    let old=String::from_utf8(bytes.clone()).unwrap().replacen("\"version\":2", "\"version\":1", 1).into_bytes();
    assert!(signature_check(&old,key.sign(&old).as_ref(),&der,&trust).is_err());
    assert!(gui_shell_rust_helper::installed_artifact::canonical(&root.join("app/gui_shell_desktop.exe")).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn 配布物のlinkと規約外pathを拒否する() {
    let mut id=[0u8;16];getrandom::getrandom(&mut id).unwrap();
    let root=std::env::temp_dir().join(format!("gui-shell-artifact-path-{}",hex::encode(id)));installation(&root);
    let link=root.join("app").join("link");let target=root.join("broker");
    #[cfg(unix)] std::os::unix::fs::symlink(&target,&link).unwrap();
    #[cfg(windows)] {
        let out=std::process::Command::new("cmd").args(["/c","mklink","/J"]).arg(&link).arg(&target).output().unwrap();
        assert!(out.status.success(),"junction作成失敗: {:?} {} {}",out.status,String::from_utf8_lossy(&out.stdout),String::from_utf8_lossy(&out.stderr));
    }
    assert!(gui_shell_rust_helper::installed_artifact::canonical(&root).is_err());
    #[cfg(unix)] std::fs::remove_file(&link).unwrap();
    #[cfg(windows)] std::fs::remove_dir(&link).unwrap();
    let invalid=root.join("app/規約外");std::fs::write(&invalid,b"x").unwrap();
    assert!(gui_shell_rust_helper::installed_artifact::canonical(&root).is_err());std::fs::remove_file(invalid).unwrap();
    // OS名からfilesystemの大小文字規則を推定しない。
    std::fs::write(root.join("app/CASE"),b"a").unwrap();std::fs::write(root.join("app/case"),b"b").unwrap();
    let count=std::fs::read_dir(root.join("app")).unwrap().filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().eq_ignore_ascii_case("case")).count();
    match count {
        2 => assert!(gui_shell_rust_helper::installed_artifact::canonical(&root).is_err()),
        1 => {
            let bytes=gui_shell_rust_helper::installed_artifact::canonical(&root).unwrap();
            let manifest:serde_json::Value=serde_json::from_slice(&bytes).unwrap();
            let entries:Vec<_>=manifest["entries"].as_array().unwrap().iter().filter(|e| e["path"].as_str().unwrap().eq_ignore_ascii_case("app/case")).collect();
            assert_eq!(entries.len(),1);
            assert_eq!(entries[0]["sha256"],sha256_tagged(b"b"));
        },
        _ => panic!("試験fileのentry数が不正"),
    }
    std::fs::remove_dir_all(root).unwrap();
}
