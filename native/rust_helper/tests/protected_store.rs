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
