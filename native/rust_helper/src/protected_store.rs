//! 検証済みdirectory capability内で不変の暗号文を扱う。承認・監査は呼出し側が担う。
use crate::audit_hash::sha256_tagged;
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, MetadataExt, OpenOptions};
use gui_shell_windows_protection::{protect, unprotect, Secret, MAX_CIPHERTEXT};
use std::io::{Read, Write};

#[derive(Clone, Copy, Debug)]
pub enum Purpose {
    History,
    Credential,
}
impl Purpose {
    fn label(self) -> &'static str {
        match self {
            Self::History => "history",
            Self::Credential => "credential",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum StoreError {
    InvalidReference,
    Protection,
    Io,
    Changed,
}

pub struct ProtectedStore {
    directory: Dir,
}
impl ProtectedStore {
    /// directory取得の権限・link境界は呼出し側で検証済みでなければならない。
    pub fn new(directory: Dir) -> Self {
        Self { directory }
    }

    pub fn create(
        &self,
        purpose: Purpose,
        id: &str,
        plaintext: &[u8],
    ) -> Result<String, StoreError> {
        let (name, context) = reference(purpose, id)?;
        let ciphertext =
            protect(context.as_bytes(), plaintext).map_err(|_| StoreError::Protection)?;
        let hash = sha256_tagged(&ciphertext);
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No);
        let mut file = self
            .directory
            .open_with(name, &options)
            .map_err(|_| StoreError::Io)?;
        file.write_all(&ciphertext).map_err(|_| StoreError::Io)?;
        file.sync_all().map_err(|_| StoreError::Io)?;
        Ok(hash)
    }

    pub fn read(
        &self,
        purpose: Purpose,
        id: &str,
        expected_hash: &str,
    ) -> Result<Secret, StoreError> {
        let (name, context) = reference(purpose, id)?;
        if !expected_hash
            .strip_prefix("sha256:")
            .is_some_and(|s| s.len() == 64 && lower_hex(s))
        {
            return Err(StoreError::InvalidReference);
        }
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let file = self
            .directory
            .open_with(name, &options)
            .map_err(|_| StoreError::Io)?;
        let metadata = file.metadata().map_err(|_| StoreError::Io)?;
        if !metadata.is_file()
            || metadata.file_attributes() & 0x400 != 0
            || cap_fs_ext::MetadataExt::nlink(&metadata) != 1
            || metadata.len() == 0
            || metadata.len() > MAX_CIPHERTEXT as u64
        {
            return Err(StoreError::Changed);
        }
        let mut ciphertext = Vec::new();
        file.take(MAX_CIPHERTEXT as u64 + 1)
            .read_to_end(&mut ciphertext)
            .map_err(|_| StoreError::Io)?;
        if ciphertext.len() != metadata.len() as usize
            || sha256_tagged(&ciphertext) != expected_hash
        {
            return Err(StoreError::Changed);
        }
        unprotect(context.as_bytes(), &ciphertext).map_err(|_| StoreError::Protection)
    }
}

fn lower_hex(s: &str) -> bool {
    s.bytes()
        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn reference(purpose: Purpose, id: &str) -> Result<(String, String), StoreError> {
    if id.len() != 32 || !lower_hex(id) {
        return Err(StoreError::InvalidReference);
    }
    Ok((
        format!("{}-{id}.dpapi", purpose.label()),
        format!("GUI-Shell:protected:v1:{}:{id}", purpose.label()),
    ))
}
