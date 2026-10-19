//! 資格情報保存先だけを分離する。Permission、Approval、用途、Auditは保管庫に残す。
use zeroize::Zeroizing;

#[derive(Debug)]
pub(super) enum 保存失敗 {
    保管拒否,
    #[cfg_attr(windows, allow(dead_code))]
    回収未成立,
    #[cfg(target_os = "macos")]
    署名identity未成立,
}

pub(super) trait 資格情報保存先 {
    fn 登録(&self, id: &str, plaintext: &[u8]) -> Result<String, 保存失敗>;
    fn 点検(&self, id: &str) -> Result<Option<(String, u64)>, ()>;
    fn 読取(&self, id: &str, hash: &str) -> Result<Zeroizing<Vec<u8>>, ()>;
    fn 新規破棄(&self, id: &str, hash: &str) -> bool;
}

#[cfg(windows)]
impl 資格情報保存先 for crate::protected_store::ProtectedStore {
    fn 登録(&self, id: &str, plaintext: &[u8]) -> Result<String, 保存失敗> {
        self.create(crate::protected_store::Purpose::Credential, id, plaintext)
            .map_err(|_| 保存失敗::保管拒否)
    }
    fn 点検(&self, id: &str) -> Result<Option<(String, u64)>, ()> {
        self.inspect(crate::protected_store::Purpose::Credential, id)
            .map_err(|_| ())
    }
    fn 読取(&self, id: &str, hash: &str) -> Result<Zeroizing<Vec<u8>>, ()> {
        self.read(crate::protected_store::Purpose::Credential, id, hash)
            .map(|secret| Zeroizing::new(secret.as_bytes().to_vec()))
            .map_err(|_| ())
    }
    fn 新規破棄(&self, id: &str, hash: &str) -> bool {
        self.prepare_delete(crate::protected_store::Purpose::Credential, id, hash)
            .ok()
            .and_then(|prepared| prepared.commit().ok())
            .is_some()
    }
}

impl super::protocol::Broker {
    #[cfg(target_os = "linux")]
    pub(super) fn linux資格情報保存先を初期化(
        &mut self,
        root: &std::path::Path,
    ) -> Result<(), &'static str> {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let checked = || -> Option<String> {
            let meta = std::fs::symlink_metadata(root).ok()?;
            if !meta.is_dir()
                || meta.file_type().is_symlink()
                || meta.permissions().mode() & 0o077 != 0
                || meta.uid() != rustix::process::geteuid().as_raw()
            {
                return None;
            }
            let path = root.canonicalize().ok()?;
            let hash = crate::audit_hash::sha256_tagged(path.to_str()?.as_bytes());
            Some(hash.strip_prefix("sha256:")?.to_owned())
        };
        self.linux_credential_store = Some(LinuxCredentialStore::new(
            checked().ok_or("資格情報名前空間の検証が未成立")?,
        ));
        Ok(())
    }
    #[cfg(target_os = "macos")]
    pub(super) fn macos資格情報保存先を初期化(
        &mut self,
        root: &std::path::Path,
    ) -> Result<(), &'static str> {
        use std::os::unix::fs::PermissionsExt;
        let checked = || -> Option<String> {
            let meta = std::fs::symlink_metadata(root).ok()?;
            let parent = root.parent()?;
            let parent_meta = std::fs::symlink_metadata(parent).ok()?;
            if !meta.is_dir()
                || meta.file_type().is_symlink()
                || !parent_meta.is_dir()
                || parent_meta.file_type().is_symlink()
                || parent_meta.permissions().mode() & 0o077 != 0
            {
                return None;
            }
            let path = root.canonicalize().ok()?;
            let hash = crate::audit_hash::sha256_tagged(path.to_str()?.as_bytes());
            Some(hash.strip_prefix("sha256:")?.to_owned())
        };
        self.macos_credential_store = Some(
            MacOSCredentialStore::new(checked().ok_or("資格情報名前空間の検証が未成立")?)
                .map_err(|_| "資格情報保存先が未成立")?,
        );
        Ok(())
    }
    pub(super) fn 資格情報保存先(&self) -> Option<&dyn 資格情報保存先> {
        #[cfg(windows)]
        {
            self.protected_store
                .as_ref()
                .map(|v| v as &dyn 資格情報保存先)
        }
        #[cfg(target_os = "macos")]
        {
            self.macos_credential_store
                .as_ref()
                .map(|v| v as &dyn 資格情報保存先)
        }
        #[cfg(target_os = "linux")]
        {
            self.linux_credential_store
                .as_ref()
                .map(|v| v as &dyn 資格情報保存先)
        }
    }
}

#[cfg(target_os = "linux")]
pub(super) struct LinuxCredentialStore {
    namespace: String,
}

#[cfg(target_os = "linux")]
impl std::fmt::Debug for LinuxCredentialStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinuxCredentialStore")
            .finish_non_exhaustive()
    }
}

#[cfg(target_os = "linux")]
impl LinuxCredentialStore {
    const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);
    const MAX_SECRET_BYTES: usize = 65_536;
    const HEADER: &'static [u8; 5] = b"D4SS1";

    pub(super) fn new(namespace: String) -> Self {
        Self { namespace }
    }

    fn lookup_key(&self, id: &str, part: &str) -> String {
        crate::audit_hash::sha256_tagged(
            format!("D4Pocket:credential:v1:{}:{id}:{part}", self.namespace).as_bytes(),
        )
    }

    fn attributes<'a>(&'a self, key: &'a str) -> std::collections::HashMap<&'a str, &'a str> {
        std::collections::HashMap::from([
            ("application", "d4-pocket"),
            ("namespace", self.namespace.as_str()),
            ("record", key),
        ])
    }

    fn context(&self, id: &str) -> Vec<u8> {
        format!("D4Pocket:credential:v1:{}:{id}", self.namespace).into_bytes()
    }

    fn seal(
        &self,
        id: &str,
        secret: &[u8],
    ) -> Result<(Zeroizing<Vec<u8>>, Zeroizing<[u8; 32]>), 保存失敗> {
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
        if secret.is_empty() || secret.len() > Self::MAX_SECRET_BYTES {
            return Err(保存失敗::保管拒否);
        }
        let mut key = Zeroizing::new([0_u8; 32]);
        let mut nonce = [0_u8; 12];
        getrandom::getrandom(key.as_mut()).map_err(|_| 保存失敗::保管拒否)?;
        getrandom::getrandom(&mut nonce).map_err(|_| 保存失敗::保管拒否)?;
        let cipher = LessSafeKey::new(
            UnboundKey::new(&AES_256_GCM, key.as_ref()).map_err(|_| 保存失敗::保管拒否)?,
        );
        let mut sealed = Zeroizing::new(secret.to_vec());
        cipher
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(self.context(id)),
                &mut *sealed,
            )
            .map_err(|_| 保存失敗::保管拒否)?;
        let mut ciphertext = Zeroizing::new(Vec::with_capacity(17 + sealed.len()));
        ciphertext.extend_from_slice(Self::HEADER);
        ciphertext.extend_from_slice(&nonce);
        ciphertext.extend_from_slice(&sealed);
        Ok((ciphertext, key))
    }

    fn open_envelope(
        &self,
        id: &str,
        bytes: &[u8],
        key: &[u8],
        hash: &str,
    ) -> Result<Zeroizing<Vec<u8>>, ()> {
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
        if key.len() != 32
            || bytes.len() < 33
            || bytes.len() > 33 + Self::MAX_SECRET_BYTES
            || !bytes.starts_with(Self::HEADER)
            || crate::audit_hash::sha256_tagged(bytes) != hash
        {
            return Err(());
        }
        let nonce: [u8; 12] = bytes[5..17].try_into().map_err(|_| ())?;
        let cipher = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, key).map_err(|_| ())?);
        let mut ciphertext = Zeroizing::new(bytes[17..].to_vec());
        let plaintext = cipher
            .open_in_place(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(self.context(id)),
                &mut ciphertext,
            )
            .map_err(|_| ())?;
        Ok(Zeroizing::new(plaintext.to_vec()))
    }

    async fn delete_exact_item(
        collection: &secret_service::Collection<'_>,
        attributes: std::collections::HashMap<&str, &str>,
        expected: &[u8],
    ) -> Result<bool, ()> {
        let items = collection.search_items(attributes).await.map_err(|_| ())?;
        if items.is_empty() {
            return Ok(true);
        }
        if items.len() != 1 || items[0].is_locked().await.map_err(|_| ())? {
            return Ok(false);
        }
        let stored = Zeroizing::new(items[0].get_secret().await.map_err(|_| ())?);
        if stored.as_slice() != expected {
            return Ok(false);
        }
        Ok(items[0].delete().await.is_ok())
    }
}

#[cfg(target_os = "linux")]
impl 資格情報保存先 for LinuxCredentialStore {
    fn 登録(&self, id: &str, plaintext: &[u8]) -> Result<String, 保存失敗> {
        use secret_service::{EncryptionType, SecretService};
        let (ciphertext, key_bytes) = self.seal(id, plaintext)?;
        let key_lookup = self.lookup_key(id, "key");
        let ciphertext_lookup = self.lookup_key(id, "ciphertext");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| 保存失敗::保管拒否)?;
        runtime.block_on(async {
            tokio::time::timeout(Self::TIMEOUT, async {
                let service = SecretService::connect(EncryptionType::Dh)
                    .await
                    .map_err(|_| 保存失敗::保管拒否)?;
                let collection = service
                    .get_default_collection()
                    .await
                    .map_err(|_| 保存失敗::保管拒否)?;
                if collection
                    .is_locked()
                    .await
                    .map_err(|_| 保存失敗::保管拒否)?
                {
                    return Err(保存失敗::保管拒否);
                }
                let key_attributes = self.attributes(&key_lookup);
                let ciphertext_attributes = self.attributes(&ciphertext_lookup);
                if !collection
                    .search_items(key_attributes.clone())
                    .await
                    .map_err(|_| 保存失敗::保管拒否)?
                    .is_empty()
                    || !collection
                        .search_items(ciphertext_attributes.clone())
                        .await
                        .map_err(|_| 保存失敗::保管拒否)?
                        .is_empty()
                {
                    return Err(保存失敗::保管拒否);
                }
                if collection
                    .create_item(
                        "D4 Pocket credential key",
                        key_attributes.clone(),
                        key_bytes.as_ref(),
                        false,
                        "application/octet-stream",
                    )
                    .await
                    .is_err()
                {
                    let removed =
                        Self::delete_exact_item(&collection, key_attributes, key_bytes.as_ref())
                            .await;
                    return Err(if removed == Ok(true) {
                        保存失敗::保管拒否
                    } else {
                        保存失敗::回収未成立
                    });
                }
                if collection
                    .create_item(
                        "D4 Pocket credential ciphertext",
                        ciphertext_attributes.clone(),
                        &ciphertext,
                        false,
                        "application/octet-stream",
                    )
                    .await
                    .is_err()
                {
                    let ciphertext_removed =
                        Self::delete_exact_item(&collection, ciphertext_attributes, &ciphertext)
                            .await;
                    let key_removed =
                        Self::delete_exact_item(&collection, key_attributes, key_bytes.as_ref())
                            .await;
                    return Err(
                        if ciphertext_removed == Ok(true) && key_removed == Ok(true) {
                            保存失敗::保管拒否
                        } else {
                            保存失敗::回収未成立
                        },
                    );
                }
                let key_items = collection
                    .search_items(key_attributes.clone())
                    .await
                    .map_err(|_| 保存失敗::回収未成立)?;
                let ciphertext_items = collection
                    .search_items(ciphertext_attributes.clone())
                    .await
                    .map_err(|_| 保存失敗::回収未成立)?;
                let verified = if key_items.len() == 1
                    && ciphertext_items.len() == 1
                    && !key_items[0]
                        .is_locked()
                        .await
                        .map_err(|_| 保存失敗::回収未成立)?
                    && !ciphertext_items[0]
                        .is_locked()
                        .await
                        .map_err(|_| 保存失敗::回収未成立)?
                {
                    let stored_key = Zeroizing::new(
                        key_items[0]
                            .get_secret()
                            .await
                            .map_err(|_| 保存失敗::回収未成立)?,
                    );
                    let stored_ciphertext = Zeroizing::new(
                        ciphertext_items[0]
                            .get_secret()
                            .await
                            .map_err(|_| 保存失敗::回収未成立)?,
                    );
                    stored_key.as_slice() == key_bytes.as_ref()
                        && stored_ciphertext.as_slice() == ciphertext.as_slice()
                } else {
                    false
                };
                if !verified {
                    let ciphertext_removed =
                        Self::delete_exact_item(&collection, ciphertext_attributes, &ciphertext)
                            .await;
                    let key_removed =
                        Self::delete_exact_item(&collection, key_attributes, key_bytes.as_ref())
                            .await;
                    return Err(
                        if ciphertext_removed == Ok(true) && key_removed == Ok(true) {
                            保存失敗::保管拒否
                        } else {
                            保存失敗::回収未成立
                        },
                    );
                }
                Ok(crate::audit_hash::sha256_tagged(&ciphertext))
            })
            .await
            .map_err(|_| 保存失敗::回収未成立)?
        })
    }

    fn 点検(&self, id: &str) -> Result<Option<(String, u64)>, ()> {
        use secret_service::{EncryptionType, SecretService};
        let key_lookup = self.lookup_key(id, "key");
        let ciphertext_lookup = self.lookup_key(id, "ciphertext");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| ())?;
        runtime.block_on(async {
            tokio::time::timeout(Self::TIMEOUT, async {
                let service = SecretService::connect(EncryptionType::Dh)
                    .await
                    .map_err(|_| ())?;
                let collection = service.get_default_collection().await.map_err(|_| ())?;
                if collection.is_locked().await.map_err(|_| ())? {
                    return Err(());
                }
                let key_items = collection
                    .search_items(self.attributes(&key_lookup))
                    .await
                    .map_err(|_| ())?;
                let ciphertext_items = collection
                    .search_items(self.attributes(&ciphertext_lookup))
                    .await
                    .map_err(|_| ())?;
                if key_items.is_empty() && ciphertext_items.is_empty() {
                    return Ok(None);
                }
                if key_items.len() != 1
                    || ciphertext_items.len() != 1
                    || key_items[0].is_locked().await.map_err(|_| ())?
                    || ciphertext_items[0].is_locked().await.map_err(|_| ())?
                {
                    return Err(());
                }
                let key_bytes = Zeroizing::new(key_items[0].get_secret().await.map_err(|_| ())?);
                let ciphertext =
                    Zeroizing::new(ciphertext_items[0].get_secret().await.map_err(|_| ())?);
                if key_bytes.len() != 32
                    || ciphertext.len() < 33
                    || ciphertext.len() > 33 + Self::MAX_SECRET_BYTES
                    || !ciphertext.starts_with(Self::HEADER)
                    || self
                        .open_envelope(
                            id,
                            &ciphertext,
                            &key_bytes,
                            &crate::audit_hash::sha256_tagged(&ciphertext),
                        )
                        .is_err()
                {
                    return Err(());
                }
                Ok(Some((
                    crate::audit_hash::sha256_tagged(&ciphertext),
                    ciphertext.len() as u64,
                )))
            })
            .await
            .map_err(|_| ())?
        })
    }

    fn 読取(&self, id: &str, hash: &str) -> Result<Zeroizing<Vec<u8>>, ()> {
        use secret_service::{EncryptionType, SecretService};
        let key_lookup = self.lookup_key(id, "key");
        let ciphertext_lookup = self.lookup_key(id, "ciphertext");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| ())?;
        runtime.block_on(async {
            tokio::time::timeout(Self::TIMEOUT, async {
                let service = SecretService::connect(EncryptionType::Dh)
                    .await
                    .map_err(|_| ())?;
                let collection = service.get_default_collection().await.map_err(|_| ())?;
                if collection.is_locked().await.map_err(|_| ())? {
                    return Err(());
                }
                let key_items = collection
                    .search_items(self.attributes(&key_lookup))
                    .await
                    .map_err(|_| ())?;
                let ciphertext_items = collection
                    .search_items(self.attributes(&ciphertext_lookup))
                    .await
                    .map_err(|_| ())?;
                if key_items.len() != 1
                    || ciphertext_items.len() != 1
                    || key_items[0].is_locked().await.map_err(|_| ())?
                    || ciphertext_items[0].is_locked().await.map_err(|_| ())?
                {
                    return Err(());
                }
                let key_bytes = Zeroizing::new(key_items[0].get_secret().await.map_err(|_| ())?);
                let ciphertext =
                    Zeroizing::new(ciphertext_items[0].get_secret().await.map_err(|_| ())?);
                self.open_envelope(id, &ciphertext, &key_bytes, hash)
            })
            .await
            .map_err(|_| ())?
        })
    }

    fn 新規破棄(&self, id: &str, hash: &str) -> bool {
        use secret_service::{EncryptionType, SecretService};
        let key_lookup = self.lookup_key(id, "key");
        let ciphertext_lookup = self.lookup_key(id, "ciphertext");
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return false;
        };
        runtime.block_on(async {
            tokio::time::timeout(Self::TIMEOUT, async {
                let service = SecretService::connect(EncryptionType::Dh)
                    .await
                    .map_err(|_| ())?;
                let collection = service.get_default_collection().await.map_err(|_| ())?;
                if collection.is_locked().await.map_err(|_| ())? {
                    return Err(());
                }
                let key_items = collection
                    .search_items(self.attributes(&key_lookup))
                    .await
                    .map_err(|_| ())?;
                let ciphertext_items = collection
                    .search_items(self.attributes(&ciphertext_lookup))
                    .await
                    .map_err(|_| ())?;
                if key_items.len() != 1
                    || ciphertext_items.len() != 1
                    || key_items[0].is_locked().await.map_err(|_| ())?
                    || ciphertext_items[0].is_locked().await.map_err(|_| ())?
                {
                    return Err(());
                }
                let key_bytes = Zeroizing::new(key_items[0].get_secret().await.map_err(|_| ())?);
                let ciphertext =
                    Zeroizing::new(ciphertext_items[0].get_secret().await.map_err(|_| ())?);
                if crate::audit_hash::sha256_tagged(&ciphertext) != hash
                    || self
                        .open_envelope(id, &ciphertext, &key_bytes, hash)
                        .is_err()
                {
                    return Err(());
                }
                ciphertext_items[0].delete().await.map_err(|_| ())?;
                key_items[0].delete().await.map_err(|_| ())
            })
            .await
            .is_ok_and(|result| result.is_ok())
        })
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod linux_storage_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn linux_secret_service_envelope_is_randomized_bounded_and_hash_bound() {
        let store = LinuxCredentialStore::new("a".repeat(64));
        let id = "b".repeat(32);
        let secret = b"synthetic-linux-credential-not-operational";
        let (first, first_key) = store.seal(&id, secret).unwrap();
        let (second, _) = store.seal(&id, secret).unwrap();
        assert_ne!(first.as_slice(), second.as_slice());
        assert!(!first.windows(secret.len()).any(|part| part == secret));
        let hash = crate::audit_hash::sha256_tagged(&first);
        assert_eq!(
            store
                .open_envelope(&id, &first, first_key.as_ref(), &hash)
                .unwrap()
                .as_slice(),
            secret
        );
        assert!(store
            .open_envelope(
                &id,
                &first,
                first_key.as_ref(),
                &crate::audit_hash::sha256_tagged(&second)
            )
            .is_err());
        assert!(store.seal(&id, &[]).is_err());
        assert!(store
            .seal(&id, &vec![b'x'; LinuxCredentialStore::MAX_SECRET_BYTES + 1])
            .is_err());
        assert!(store
            .open_envelope(&"c".repeat(32), &first, first_key.as_ref(), &hash)
            .is_err());
        let lookup = store.lookup_key(&id, "ciphertext");
        let attributes = store.attributes(&lookup);
        assert_eq!(attributes.get("application"), Some(&"d4-pocket"));
        assert!(!attributes.values().any(|value| value.contains(&id)));
        assert_ne!(store.lookup_key(&id, "key"), lookup);
    }

    #[test]
    #[ignore = "requires an isolated, unlocked user-session Secret Service"]
    fn linux_secret_service_actual_store_is_create_only_namespaced_and_cleaned() {
        let mut random = [0_u8; 32];
        getrandom::getrandom(&mut random).unwrap();
        let namespace = crate::audit_hash::sha256_tagged(&random)[7..].to_owned();
        let store = LinuxCredentialStore::new(namespace.clone());
        let other = LinuxCredentialStore::new("f".repeat(64));
        let id = &namespace[..32];
        let secret = Zeroizing::new(b"synthetic-linux-secret-not-operational".to_vec());
        let hash = store
            .登録(id, &secret)
            .expect("Secret Service APIへの新規item");
        struct Cleanup<'a>(&'a LinuxCredentialStore, &'a str, &'a str, bool);
        impl Cleanup<'_> {
            fn clean(&mut self) -> bool {
                let cleaned = self.0.新規破棄(self.1, self.2);
                self.3 = !cleaned;
                cleaned
            }
        }
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                if self.3 {
                    let _ = self.0.新規破棄(self.1, self.2);
                }
            }
        }
        let mut cleanup = Cleanup(&store, id, &hash, true);
        assert_eq!(store.点検(id).unwrap().unwrap().0, hash);
        assert_eq!(store.読取(id, &hash).unwrap().as_slice(), secret.as_slice());
        assert!(store.登録(id, &secret).is_err(), "同じIDを置換しない");
        assert!(other.点検(id).unwrap().is_none(), "他namespaceを検索しない");
        assert!(store
            .読取(id, &format!("sha256:{}", "0".repeat(64)))
            .is_err());
        assert!(!store.新規破棄(id, &format!("sha256:{}", "0".repeat(64))));
        assert!(cleanup.clean(), "合成keyとciphertextだけを回収");
        assert!(store.点検(id).unwrap().is_none());

        let broker_store = std::env::temp_dir().join(format!(
            "d4-linux-credential-broker-{}-{}",
            std::process::id(),
            hex::encode(random)
        ));
        std::fs::create_dir(&broker_store).unwrap();
        std::fs::set_permissions(&broker_store, std::fs::Permissions::from_mode(0o700)).unwrap();
        let session = "linux-credential-installed-path";
        let mut broker = crate::broker::protocol::Broker::new_persistent(session, &broker_store)
            .expect("isolated persistent Broker");
        broker
            .linux資格情報保存先を初期化(&broker_store)
            .expect("private Broker store namespace");
        let broker_id_text = hex::encode(random);
        let broker_id = &broker_id_text[..32];
        let broker_secret = "synthetic-linux-owner-gated-secret-not-operational";
        let registration = serde_json::json!({
            "版":1,"操作":"追加","資格情報ID":broker_id,"用途":"mcp_transport",
            "接続対象":"synthetic-server","種類":"api_key",
            "保管方式":"linux_secret_service","登録者種別":"owner",
            "登録経路":"owner_control","秘密値":broker_secret
        });
        let request = serde_json::json!({
            "request_id":"linux-credential-owner-registered",
            "session_id":session,
            "operation":"資格情報登録",
            "payload_hash":crate::broker::protocol::canonical_payload_hash(Some(&registration)),
            "nonce":"linux-credential-owner-once",
            "issued_at":crate::broker::protocol::BrokerRequestEnvelope::current_issued_at(),
            "metadata":{"client":"desktop_flutter"},
            "payload":registration
        });
        let response = broker.desktop_owner_operation_json(&request.to_string());
        assert_eq!(
            response.status,
            crate::broker::protocol::BrokerStatus::Accepted
        );
        let receipt = response.body.expect("metadata-only receipt");
        let public_response = serde_json::to_string(&receipt).unwrap();
        assert!(!public_response.contains(broker_secret));
        assert_eq!(receipt["保管方式"], "linux_secret_service");
        let audit = serde_json::to_string(&broker.audit_events()).unwrap();
        assert!(!audit.contains(broker_secret), "秘密を永続Auditへ出さない");
        let broker_ciphertext_hash = receipt["暗号文hash"].as_str().unwrap();
        let broker_root = broker_store.canonicalize().unwrap();
        let broker_namespace =
            crate::audit_hash::sha256_tagged(broker_root.to_str().unwrap().as_bytes())[7..]
                .to_owned();
        let broker_credential_store = LinuxCredentialStore::new(broker_namespace);
        let mut broker_cleanup = Cleanup(
            &broker_credential_store,
            broker_id,
            broker_ciphertext_hash,
            true,
        );
        assert_eq!(
            broker_credential_store
                .読取(broker_id, broker_ciphertext_hash)
                .unwrap()
                .as_slice(),
            broker_secret.as_bytes()
        );

        let mcp_workspace = broker_store.join("mcp-workspace");
        std::fs::create_dir(&mcp_workspace).unwrap();
        std::fs::set_permissions(&mcp_workspace, std::fs::Permissions::from_mode(0o700)).unwrap();
        let server = "synthetic-server";
        let script_path = mcp_workspace.join("mcp-server.sh");
        let discovery = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "resultType": "complete",
                "supportedVersions": [crate::mcp::MODERN_PROTOCOL_VERSION],
                "capabilities": {}
            }
        });
        let script = format!(
            "#!/bin/sh\n[ \"$MCP_TEST_CREDENTIAL\" = \"{broker_secret}\" ] || exit 41\nIFS= read -r request || exit 42\ncase \"$request\" in *server/discover*) ;; *) exit 43 ;; esac\nprintf '%s\\n' '{}'\nexec /bin/sleep 60\n",
            discovery
        );
        std::fs::write(&script_path, script).unwrap();
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let connect_payload = serde_json::json!({
            "版":1,
            "操作":"接続",
            "ServerID":server,
            "実行file":"/bin/sh",
            "引数":[script_path.to_string_lossy()],
            "workspace":mcp_workspace.to_string_lossy(),
            "Transport":"stdio",
            "Credential ref":{
                "credential_id":broker_id,
                "purpose":"mcp_transport",
                "target":server,
                "required":true,
                "status":"configured",
                "environment_variable":"MCP_TEST_CREDENTIAL"
            }
        });
        let connect_hash = crate::broker::protocol::canonical_payload_hash(Some(&connect_payload));
        let connected = crate::broker::mcp_center::connect(
            &mut broker,
            "linux-mcp-secret-service-connect",
            &connect_payload,
            true,
            &connect_hash,
        );
        assert_eq!(
            connected.status,
            crate::broker::protocol::BrokerStatus::Accepted,
            "合成Secret Service値だけを一致対象のMCP childへ渡す: {connected:?}"
        );
        let connected_public = serde_json::to_string(&connected).unwrap();
        assert!(!connected_public.contains(broker_secret));
        let disconnected = crate::broker::mcp_center::disconnect(
            &mut broker,
            "linux-mcp-secret-service-disconnect",
            &serde_json::json!({"版":1,"操作":"切断","ServerID":server}),
            true,
            &crate::broker::protocol::canonical_payload_hash(None),
        );
        assert_eq!(
            disconnected.status,
            crate::broker::protocol::BrokerStatus::Accepted,
            "所有process groupを停止して切断: {disconnected:?}"
        );
        let complete_audit = serde_json::to_string(&broker.audit_events()).unwrap();
        assert!(!complete_audit.contains(broker_secret));
        drop(broker);
        assert!(broker_cleanup.clean(), "Broker試験が作成したitemだけ回収");
        assert!(broker_credential_store.点検(broker_id).unwrap().is_none());
        std::fs::remove_dir_all(broker_store).unwrap();
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    #[test]
    fn macos_keychain_actual_ciphertext_and_namespace_boundary() {
        let mut random = [0u8; 32];
        getrandom::getrandom(&mut random).unwrap();
        let namespace = crate::audit_hash::sha256_tagged(&random)[7..].to_owned();
        let store = MacOSCredentialStore::new(namespace.clone()).unwrap();
        let id = &namespace[..32];
        let secret = Zeroizing::new(random.to_vec());
        let hash = store.登録(id, &secret).expect("Keychain実API追加");
        struct Cleanup<'a>(&'a MacOSCredentialStore, &'a str, &'a str);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                assert!(self.0.新規破棄(self.1, self.2), "今回の合成itemだけを回収");
            }
        }
        let _cleanup = Cleanup(&store, id, &hash);
        assert!(store
            .点検(id)
            .unwrap()
            .is_some_and(|(actual, _)| actual == hash));
        assert!(
            store.読取(id, &hash).unwrap().as_slice() == secret.as_slice(),
            "復号値は内部比較だけ"
        );
        assert!(store
            .読取(id, &format!("sha256:{}", "0".repeat(64)))
            .is_err());
        assert!(store.登録(id, &secret).is_err(), "重複で置換しない");
        let foreign = MacOSCredentialStore::new("f".repeat(64)).unwrap();
        assert!(foreign.点検(id).unwrap().is_none(), "別storeの探索をしない");
        let encrypted = store
            .os
            .read(id, gui_shell_macos_keychain::Part::Ciphertext)
            .unwrap();
        assert!(
            !encrypted
                .windows(secret.len())
                .any(|part| part == secret.as_slice()),
            "公開hash対象は実暗号文"
        );
    }
}

#[cfg(target_os = "macos")]
pub(super) struct MacOSCredentialStore {
    os: gui_shell_macos_keychain::Store,
    namespace: String,
}

#[cfg(target_os = "macos")]
impl std::fmt::Debug for MacOSCredentialStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacOSCredentialStore")
            .finish_non_exhaustive()
    }
}

#[cfg(target_os = "macos")]
impl MacOSCredentialStore {
    pub(super) fn new(namespace: String) -> Result<Self, ()> {
        Ok(Self {
            os: gui_shell_macos_keychain::Store::new(&namespace).map_err(|_| ())?,
            namespace,
        })
    }
    fn context(&self, id: &str) -> Vec<u8> {
        format!("D4Pocket:credential:v1:{}:{id}", self.namespace).into_bytes()
    }
}

#[cfg(target_os = "macos")]
impl 資格情報保存先 for MacOSCredentialStore {
    fn 登録(&self, id: &str, plaintext: &[u8]) -> Result<String, 保存失敗> {
        use gui_shell_macos_keychain::Part;
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
        let rejected = |e| match e {
            gui_shell_macos_keychain::Error::IdentityRequired => {
                保存失敗::署名identity未成立
            }
            _ => 保存失敗::保管拒否,
        };
        if plaintext.is_empty() || plaintext.len() > 65536 {
            return Err(保存失敗::保管拒否);
        }
        let mut key = Zeroizing::new([0u8; 32]);
        let mut nonce = [0u8; 12];
        getrandom::getrandom(key.as_mut()).map_err(|_| 保存失敗::保管拒否)?;
        getrandom::getrandom(&mut nonce).map_err(|_| 保存失敗::保管拒否)?;
        let cipher = LessSafeKey::new(
            UnboundKey::new(&AES_256_GCM, key.as_ref()).map_err(|_| 保存失敗::保管拒否)?,
        );
        let mut sealed = Zeroizing::new(plaintext.to_vec());
        cipher
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(self.context(id)),
                &mut *sealed,
            )
            .map_err(|_| 保存失敗::保管拒否)?;
        let mut bytes = Vec::with_capacity(17 + sealed.len());
        bytes.extend_from_slice(b"D4KV1");
        bytes.extend_from_slice(&nonce);
        bytes.extend_from_slice(&sealed);
        self.os.add(id, Part::Key, key.as_ref()).map_err(rejected)?;
        if self.os.add(id, Part::Ciphertext, &bytes).is_err() {
            // 今回作成した鍵だけを回収する。失敗を成功へ変換しない。
            return Err(if self.os.delete(id, Part::Key).is_ok() {
                保存失敗::保管拒否
            } else {
                保存失敗::回収未成立
            });
        }
        Ok(crate::audit_hash::sha256_tagged(&bytes))
    }
    fn 点検(&self, id: &str) -> Result<Option<(String, u64)>, ()> {
        use gui_shell_macos_keychain::{Error, Part};
        let bytes = match self.os.read(id, Part::Ciphertext) {
            Ok(bytes) => bytes,
            Err(Error::Missing) => return Ok(None),
            Err(_) => return Err(()),
        };
        if !self.os.exists(id, Part::Key).map_err(|_| ())?
            || bytes.len() < 34
            || bytes.len() > 65569
            || !bytes.starts_with(b"D4KV1")
        {
            return Err(());
        }
        Ok(Some((
            crate::audit_hash::sha256_tagged(&bytes),
            bytes.len() as u64,
        )))
    }
    fn 読取(&self, id: &str, hash: &str) -> Result<Zeroizing<Vec<u8>>, ()> {
        use gui_shell_macos_keychain::Part;
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
        let mut bytes = self.os.read(id, Part::Ciphertext).map_err(|_| ())?;
        if bytes.len() < 34
            || bytes.len() > 65569
            || !bytes.starts_with(b"D4KV1")
            || crate::audit_hash::sha256_tagged(&bytes) != hash
        {
            return Err(());
        }
        let nonce: [u8; 12] = bytes[5..17].try_into().map_err(|_| ())?;
        let key = self.os.read(id, Part::Key).map_err(|_| ())?;
        let cipher = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, &key).map_err(|_| ())?);
        let plaintext = cipher
            .open_in_place(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(self.context(id)),
                &mut bytes[17..],
            )
            .map_err(|_| ())?;
        Ok(Zeroizing::new(plaintext.to_vec()))
    }
    fn 新規破棄(&self, id: &str, hash: &str) -> bool {
        use gui_shell_macos_keychain::Part;
        if !matches!(self.点検(id), Ok(Some((actual, _))) if actual == hash) {
            return false;
        }
        // 未公開の新規IDだけを呼出し側が渡す。通常の論理失効はこの操作を使わない。
        self.os.delete(id, Part::Ciphertext).is_ok() && self.os.delete(id, Part::Key).is_ok()
    }
}
