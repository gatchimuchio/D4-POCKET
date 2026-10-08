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
