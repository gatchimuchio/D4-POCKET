#![cfg(windows)]
#![forbid(unsafe_code)]
use cap_std::{ambient_authority, fs::Dir};
use gui_shell_rust_helper::protected_store::{ProtectedStore, Purpose};

#[test]
fn 暗号文保存と再読取と目的間転用拒否を実fileで確認する() {
    let mut random = [0u8; 16];
    getrandom::getrandom(&mut random).unwrap();
    let root = std::env::temp_dir().join(format!("gui-shell-protected-{}", hex::encode(random)));
    std::fs::create_dir(&root).unwrap();
    let store = ProtectedStore::new(Dir::open_ambient_dir(&root, ambient_authority()).unwrap());
    let id = "a".repeat(32);
    let plain = b"synthetic protected history";
    let hash = store.create(Purpose::History, &id, plain).unwrap();
    let source = root.join(format!("history-{id}.dpapi"));
    let encrypted = std::fs::read(&source).unwrap();
    assert!(encrypted.windows(plain.len()).all(|w| w != plain));
    assert_eq!(
        store.read(Purpose::History, &id, &hash).unwrap().as_bytes(),
        plain
    );
    assert!(store.create(Purpose::History, &id, b"replacement").is_err());
    assert_eq!(std::fs::read(&source).unwrap(), encrypted);
    assert!(store
        .read(Purpose::History, &id, &format!("sha256:{}", "0".repeat(64)))
        .is_err());
    assert!(store.create(Purpose::History, "../escape", plain).is_err());
    let other = "b".repeat(32);
    let copied = root.join(format!("history-{other}.dpapi"));
    std::fs::copy(&source, &copied).unwrap();
    assert!(store.read(Purpose::History, &other, &hash).is_err());
    let different = root.join(format!("credential-{id}.dpapi"));
    std::fs::copy(&source, &different).unwrap();
    assert!(store.read(Purpose::Credential, &id, &hash).is_err());
    let evaluation = root.join(format!("evaluation-{id}.dpapi"));
    std::fs::copy(&source, &evaluation).unwrap();
    assert!(store.read(Purpose::Evaluation, &id, &hash).is_err());
    let alias = root.join("alias");
    std::fs::hard_link(&source, &alias).unwrap();
    assert!(store.read(Purpose::History, &id, &hash).is_err());
    std::fs::remove_file(&alias).unwrap();
    assert_eq!(
        store.read(Purpose::History, &id, &hash).unwrap().as_bytes(),
        plain
    );
    let reopened = ProtectedStore::new(Dir::open_ambient_dir(&root, ambient_authority()).unwrap());
    assert_eq!(
        reopened
            .read(Purpose::History, &id, &hash)
            .unwrap()
            .as_bytes(),
        plain
    );
    drop(reopened);
    std::fs::write(&source, b"corrupt").unwrap();
    assert!(store.read(Purpose::History, &id, &hash).is_err());
    drop(store);
    for path in [&source, &copied, &different, &evaluation] {
        std::fs::remove_file(path).unwrap();
    }
    std::fs::remove_dir(&root).unwrap();
}

#[test]
fn evaluationは固有の暗号文とentropyを使い全目的間の転用を拒否する() {
    let mut random = [0u8; 16];
    getrandom::getrandom(&mut random).unwrap();
    let root = std::env::temp_dir().join(format!("gui-shell-evaluation-{}", hex::encode(random)));
    std::fs::create_dir(&root).unwrap();
    let store = ProtectedStore::new(Dir::open_ambient_dir(&root, ambient_authority()).unwrap());
    let plaintext = b"synthetic private evaluation payload";
    let sources = [
        (Purpose::History, "history", "a".repeat(32)),
        (Purpose::Credential, "credential", "b".repeat(32)),
        (Purpose::Evaluation, "evaluation", "c".repeat(32)),
    ];
    let mut hashes = Vec::with_capacity(sources.len());

    for (purpose, prefix, id) in &sources {
        let hash = store.create(*purpose, id, plaintext).unwrap();
        let path = root.join(format!("{prefix}-{id}.dpapi"));
        let ciphertext = std::fs::read(&path).unwrap();
        assert!(
            ciphertext.windows(plaintext.len()).all(|window| window != plaintext),
            "{prefix} purposeの同一路にplaintextを保存してはならない"
        );
        assert_eq!(
            store.read(*purpose, id, &hash).unwrap().as_bytes(),
            plaintext,
            "{prefix} purposeは自身のentropyでだけ復号できる"
        );
        hashes.push(hash);
    }

    let copy_ids = [
        "10000000000000000000000000000000",
        "20000000000000000000000000000000",
        "30000000000000000000000000000000",
        "40000000000000000000000000000000",
        "50000000000000000000000000000000",
        "60000000000000000000000000000000",
    ];
    let mut copy_index = 0;
    for (source_index, (_, source_prefix, source_id)) in sources.iter().enumerate() {
        let source_path = root.join(format!("{source_prefix}-{source_id}.dpapi"));
        for (target_index, (target_purpose, target_prefix, _)) in sources.iter().enumerate() {
            if source_index == target_index {
                continue;
            }
            let target_id = copy_ids[copy_index];
            copy_index += 1;
            let target_path = root.join(format!("{target_prefix}-{target_id}.dpapi"));
            std::fs::copy(&source_path, &target_path).unwrap();
            assert!(
                store
                    .read(*target_purpose, target_id, &hashes[source_index])
                    .is_err(),
                "{source_prefix}暗号文を{target_prefix} purposeで読ませてはならない"
            );
        }
    }
    assert_eq!(copy_index, copy_ids.len());

    let evaluation_id = &sources[2].2;
    let evaluation_path = root.join(format!("evaluation-{evaluation_id}.dpapi"));
    store
        .prepare_delete(Purpose::Evaluation, evaluation_id, &hashes[2])
        .unwrap()
        .commit()
        .unwrap();
    assert!(!evaluation_path.exists());

    drop(store);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn 削除準備は同一fileを排他保持しcommitだけが削除する() {
    use sha2::{Digest, Sha256};
    let mut random = [0u8; 16];
    getrandom::getrandom(&mut random).unwrap();
    let root = std::env::temp_dir().join(format!("gui-shell-delete-{}", hex::encode(random)));
    std::fs::create_dir(&root).unwrap();
    let store = ProtectedStore::new(Dir::open_ambient_dir(&root, ambient_authority()).unwrap());
    let id = "c".repeat(32);
    let hash = store
        .create(Purpose::History, &id, b"synthetic delete target")
        .unwrap();
    let path = root.join(format!("history-{id}.dpapi"));
    let original = std::fs::read(&path).unwrap();
    assert!(store
        .prepare_delete(Purpose::History, "../escape", &hash)
        .is_err());
    assert!(store
        .prepare_delete(Purpose::History, &id, "invalid")
        .is_err());
    assert!(store
        .prepare_delete(Purpose::History, &id, &format!("sha256:{}", "0".repeat(64)))
        .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let alias = root.join("alias");
    std::fs::hard_link(&path, &alias).unwrap();
    assert!(store.prepare_delete(Purpose::History, &id, &hash).is_err());
    std::fs::remove_file(&alias).unwrap();
    let reader = std::fs::File::open(&path).unwrap();
    assert!(store.prepare_delete(Purpose::History, &id, &hash).is_err());
    drop(reader);
    let prepared = store.prepare_delete(Purpose::History, &id, &hash).unwrap();
    assert!(std::fs::write(&path, b"replacement").is_err());
    assert!(std::fs::rename(&path, &alias).is_err());
    assert!(std::fs::remove_file(&path).is_err());
    drop(prepared);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    store
        .prepare_delete(Purpose::History, &id, &hash)
        .unwrap()
        .commit()
        .unwrap();
    assert!(!path.exists());
    assert!(store.prepare_delete(Purpose::History, &id, &hash).is_err());
    // 部分保存の空bytesと復号不能bytesも、明示hashとの一致だけを低位で検査する。
    for bytes in [&b""[..], &b"partial ciphertext"[..]] {
        std::fs::write(&path, bytes).unwrap();
        let partial_hash = format!("sha256:{}", hex::encode(Sha256::digest(bytes)));
        store
            .prepare_delete(Purpose::History, &id, &partial_hash)
            .unwrap()
            .commit()
            .unwrap();
        assert!(!path.exists());
    }
    let oversized = vec![0u8; gui_shell_windows_protection::MAX_CIPHERTEXT + 1];
    std::fs::write(&path, &oversized).unwrap();
    let large_hash = format!("sha256:{}", hex::encode(Sha256::digest(&oversized)));
    assert!(store
        .prepare_delete(Purpose::History, &id, &large_hash)
        .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), oversized);
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(store.prepare_delete(Purpose::History, &id, &hash).is_err());
    std::fs::remove_dir(&path).unwrap();
    drop(store);
    std::fs::remove_dir(&root).unwrap();
}
