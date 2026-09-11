//! ownerが署名する前後のrelease用操作。秘密鍵の引数・読取りはない。
use gui_shell_rust_helper::checkpoint::*;
use std::{path::Path, time::{SystemTime, UNIX_EPOCH}};

pub fn run(args: &[String]) -> Result<(), String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| "時計不正")?.as_secs();
    if args.first().map(String::as_str) == Some("artifact-manifest") && args.len() == 2 {
        use std::io::Write;
        let bytes = gui_shell_rust_helper::installed_artifact::canonical(Path::new(&args[1]))?;
        std::io::stdout().write_all(&bytes).map_err(|_| "成果物一覧出力失敗")?;
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("fingerprint") && args.len() == 2 {
        let der = read(Path::new(&args[1]), 44)?;
        println!("{}", gui_shell_rust_helper::audit_hash::sha256_tagged(public_key(&der)?));
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("prepare") && args.len() == 6 {
        let head: TrustedHead = serde_json::from_slice(&read(Path::new(&args[4]), 4096)?).map_err(|_| "継続性記録不正")?;
        if head.version != 1 || (head.sequence == 0) != head.signed_checkpoint_hash.is_none() { return Err("継続性記録不正".into()); }
        let cp = measure(Path::new(&args[1]), Path::new(&args[2]), &args[3], head.sequence.checked_add(1).ok_or("sequence上限")?, head.signed_checkpoint_hash, now)?;
        use std::io::Write;
        let mut out = std::fs::OpenOptions::new().write(true).create_new(true).open(&args[5]).map_err(|_| "新規checkpoint fileを作成できない（既存fileは上書きしない）")?;
        out.write_all(&canonical(&cp)?).and_then(|_| out.sync_all()).map_err(|_| "checkpoint書込み失敗")?;
        println!("未署名checkpointを生成。署名・release承認は未成立。");
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("verify") && args.len() == 8 {
        let trust: Trust = serde_json::from_slice(&read(Path::new(&args[4]), 4096)?).map_err(|_| "署名信頼設定不正")?;
        let head: TrustedHead = serde_json::from_slice(&read(Path::new(&args[5]), 4096)?).map_err(|_| "継続性記録不正")?;
        let bundle = Path::new(&args[6]);
        let bytes = read(&bundle.join("checkpoint.json"), 16384)?;
        let sig = read(&bundle.join("signature.bin"), 64)?;
        let der = read(&bundle.join("public-key.der"), 44)?;
        let cp = parse(&bytes)?;
        let previous = if args[7] == "-" {None} else {
            let dir = Path::new(&args[7]);
            Some((read(&dir.join("checkpoint.json"), 16384)?, read(&dir.join("signature.bin"), 64)?))
        };
        let measured = measure(Path::new(&args[1]), Path::new(&args[2]), &args[3], cp.sequence, cp.previous_checkpoint_hash.clone(), now)?;
        let accepted = verify(&bytes, &sig, &der, &trust, &head, previous.as_ref().map(|(b,s)| (b.as_slice(),s.as_slice())), &measured, now)?;
        println!("{}", serde_json::json!({"status":"passed","verification_kind":"offline_ed25519_checkpoint_v2",
            "signer_public_key_fingerprint":trust.public_key_fingerprint,"signed_checkpoint_hash":accepted.signed_checkpoint_hash,
            "sequence":accepted.sequence,"previous_checkpoint_hash":cp.previous_checkpoint_hash,
            "source_commit":cp.source_commit,"installed_artifact_sha256":cp.installed_artifact_sha256,
            "audit_chain_head":cp.audit_chain_head,"audit_log_sha256":cp.audit_log_sha256,"audit_anchor_sha256":cp.audit_anchor_sha256,
            "administrator_root_resistance_claimed":false}));
        return Ok(());
    }
    Err("使用法: 監査チェックポイント artifact-manifest <installed root> | fingerprint <public-key.der> | prepare <store> <installed root> <source commit> <trusted-head.json> <新規checkpoint.json> | verify <store> <installed root> <source commit> <Repository信頼設定> <owner継続性記録> <署名bundle> <直前bundleまたは->".into())
}
