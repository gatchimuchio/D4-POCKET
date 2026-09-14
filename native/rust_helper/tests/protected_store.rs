#![cfg(windows)]
#![forbid(unsafe_code)]
use cap_std::{ambient_authority, fs::Dir};
use gui_shell_rust_helper::protected_store::{ProtectedStore, Purpose};

#[test]
fn 暗号文保存と再読取と転用拒否を実fileで確認する() {
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
    for path in [&source, &copied, &different] {
        std::fs::remove_file(path).unwrap();
    }
    std::fs::remove_dir(&root).unwrap();
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
