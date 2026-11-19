//! 編集時の版移動。呼出し側が登録、承認、永続監査と復旧記録を管理する。
use super::{read_options, regular, same_file, Dir, OpenOptions, ReadError, WorkspaceReader};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};

#[derive(Debug, PartialEq, Eq)]
pub enum EditMoveError {
    BeforeMove(ReadError),
    AfterMove(ReadError),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EditFileVersion {
    pub(crate) hash: String,
    pub(crate) file_id: u64,
}

impl WorkspaceReader {
    pub(crate) fn edit_root_identity(&self) -> Result<(u64, u64), ReadError> {
        let metadata = self
            .root
            .dir_metadata()
            .map_err(|_| ReadError::Unavailable)?;
        let file_id = cap_fs_ext::MetadataExt::ino(&metadata);
        if !metadata.is_dir() || super::reparse(&metadata) || file_id == 0 {
            return Err(ReadError::UnsafeFile);
        }
        Ok((self.root_device, file_id))
    }

    pub(crate) fn edit_file_version(
        &self,
        path: &str,
        budget: u64,
    ) -> Result<Option<(EditFileVersion, u64)>, ReadError> {
        Ok(self.read_comparison_version(path, budget)?.map(|observed| {
            (
                EditFileVersion {
                    hash: observed.sha256,
                    file_id: observed.file_id,
                },
                observed.bytes as u64,
            )
        }))
    }

    /// 適用先に書き込む前に、隔離先へ全byteを記録して同じ版か検証する。
    pub(crate) fn stage_edit_version(
        &self,
        path: &str,
        expected_hash: &str,
        vault: &Self,
        slot: &str,
        budget: u64,
    ) -> Result<(EditFileVersion, u64), ReadError> {
        if vault.allowed(slot)?.len() != 1 || !valid_hash(expected_hash) {
            return Err(ReadError::InvalidPath);
        }
        self.allowed(path)?;
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut staged = vault
            .root
            .open_with(slot, &options)
            .map_err(|_| ReadError::Unavailable)?;
        let read = self.read_file_to(
            path,
            budget,
            || {},
            |chunk| staged.write_all(chunk).map_err(|_| ReadError::Unavailable),
        )?;
        if read.sha256 != expected_hash {
            return Err(ReadError::Changed);
        }
        staged.sync_all().map_err(|_| ReadError::Unavailable)?;
        let staged_id =
            cap_fs_ext::MetadataExt::ino(&staged.metadata().map_err(|_| ReadError::Unavailable)?);
        drop(staged);
        let (version, bytes) = vault
            .edit_file_version(slot, budget)?
            .ok_or(ReadError::Changed)?;
        if version.hash != expected_hash || version.file_id != staged_id || staged_id == 0 {
            return Err(ReadError::Changed);
        }
        Ok((version, bytes))
    }

    /// 既知の版を、既存fileを置換せず別名へ移す。
    /// AfterMoveでは呼出し側が移動先を保持し、復旧対象として扱う。
    pub fn move_edit_version(
        &self,
        path: &str,
        expected_hash: &str,
        destination: &Self,
        destination_path: &str,
    ) -> Result<(), EditMoveError> {
        self.move_edit_file(
            path,
            expected_hash,
            None,
            destination,
            destination_path,
            || true,
        )
    }

    pub(crate) fn move_planned_edit_version(
        &self,
        path: &str,
        version: &EditFileVersion,
        destination: &Self,
        destination_path: &str,
        current: impl FnOnce() -> bool,
    ) -> Result<(), EditMoveError> {
        self.move_edit_file(
            path,
            &version.hash,
            Some(version.file_id),
            destination,
            destination_path,
            current,
        )
    }

    fn move_edit_file(
        &self,
        path: &str,
        expected_hash: &str,
        expected_id: Option<u64>,
        destination: &Self,
        destination_path: &str,
        current: impl FnOnce() -> bool,
    ) -> Result<(), EditMoveError> {
        use EditMoveError::{AfterMove, BeforeMove};
        if self.root_device != destination.root_device || !valid_hash(expected_hash) {
            return Err(BeforeMove(ReadError::InvalidPath));
        }
        let parts = self.allowed(path).map_err(BeforeMove)?;
        let (&name, parent_parts) = parts
            .split_last()
            .ok_or(BeforeMove(ReadError::InvalidPath))?;
        let target_parts = destination.allowed(destination_path).map_err(BeforeMove)?;
        let (&target_name, target_parent_parts) = target_parts
            .split_last()
            .ok_or(BeforeMove(ReadError::InvalidPath))?;
        let parent = self.directory(parent_parts).map_err(BeforeMove)?;
        let target_parent = destination
            .directory(target_parent_parts)
            .map_err(BeforeMove)?;
        #[cfg(windows)]
        let moving = gui_shell_windows_protection::file_move::MoveFile::open(&parent, name)
            .map_err(|_| BeforeMove(ReadError::UnsafeFile))?;
        #[cfg(windows)]
        let mut file = moving.file();
        #[cfg(not(windows))]
        let mut file = parent
            .open_with(name, &read_options())
            .map_err(|_| BeforeMove(ReadError::UnsafeFile))?;
        let before = file
            .metadata()
            .map_err(|_| BeforeMove(ReadError::Unavailable))?;
        if !regular(&before) || cap_fs_ext::MetadataExt::dev(&before) != self.root_device {
            return Err(BeforeMove(ReadError::UnsafeFile));
        }
        if expected_id.is_some_and(|id| id == 0 || id != cap_fs_ext::MetadataExt::ino(&before)) {
            return Err(BeforeMove(ReadError::Changed));
        }
        if before.len() > super::MAX_COMPARISON_BYTES {
            return Err(BeforeMove(ReadError::TooLarge(before.len())));
        }
        let mut hash = Sha256::new();
        let mut remaining = (&mut file).take(before.len() + 1);
        let mut count = 0;
        let mut buffer = [0; 65_536];
        loop {
            let read = remaining
                .read(&mut buffer)
                .map_err(|_| BeforeMove(ReadError::Unavailable))?;
            if read == 0 {
                break;
            }
            count += read as u64;
            hash.update(&buffer[..read]);
        }
        let observed_hash = format!("sha256:{}", hex::encode(hash.finalize()));
        let after = file
            .metadata()
            .map_err(|_| BeforeMove(ReadError::Unavailable))?;
        if observed_hash != expected_hash
            || count != before.len()
            || !same_version(&before, &after)
            || !current_parent(self, parent_parts, &parent)
            || !current_parent(destination, target_parent_parts, &target_parent)
        {
            return Err(BeforeMove(ReadError::Changed));
        }
        if !current() {
            return Err(BeforeMove(ReadError::Unavailable));
        }
        #[cfg(not(windows))]
        {
            let named = parent
                .symlink_metadata(name)
                .map_err(|_| BeforeMove(ReadError::Changed))?;
            if !same_version(&before, &named) {
                return Err(BeforeMove(ReadError::Changed));
            }
            rustix::fs::renameat_with(
                &parent,
                name,
                &target_parent,
                target_name,
                rustix::fs::RenameFlags::NOREPLACE,
            )
            .map_err(|_| BeforeMove(ReadError::Unavailable))?;
        }
        #[cfg(windows)]
        {
            moving
                .move_to(&target_parent, target_name)
                .map_err(|_| BeforeMove(ReadError::Unavailable))?;
            drop(moving);
        }
        #[cfg(not(windows))]
        drop(file);
        let moved = target_parent
            .open_with(target_name, &read_options())
            .and_then(|entry| entry.metadata())
            .map_err(|_| AfterMove(ReadError::Changed))?;
        if !same_version(&before, &moved)
            || !current_parent(self, parent_parts, &parent)
            || !current_parent(destination, target_parent_parts, &target_parent)
            || !self.is_missing_checked(path, || {}).map_err(AfterMove)?
            || destination
                .read_comparison_version(destination_path, before.len())
                .map_err(AfterMove)?
                .as_ref()
                .map(|version| version.sha256.as_str())
                != Some(expected_hash)
        {
            return Err(AfterMove(ReadError::Changed));
        }
        Ok(())
    }
}

fn valid_hash(hash: &str) -> bool {
    hash.strip_prefix("sha256:").is_some_and(|value| {
        value.len() == 64
            && value
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    })
}

fn current_parent(reader: &WorkspaceReader, parts: &[&str], original: &Dir) -> bool {
    let Ok(original_metadata) = original.dir_metadata() else {
        return false;
    };
    reader
        .directory(parts)
        .and_then(|parent| parent.dir_metadata().map_err(|_| ReadError::Unavailable))
        .is_ok_and(|current| same_file(&original_metadata, &current))
}

fn same_version(first: &cap_std::fs::Metadata, current: &cap_std::fs::Metadata) -> bool {
    regular(current)
        && same_file(first, current)
        && first.len() == current.len()
        && first
            .modified()
            .ok()
            .is_some_and(|modified| current.modified().ok() == Some(modified))
}
