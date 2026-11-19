//! 編集時の版移動。呼出し側が登録、承認、永続監査と復旧記録を管理する。
use super::{read_options, regular, same_file, Dir, ReadError, WorkspaceReader};
use sha2::{Digest, Sha256};
use std::io::Read;

#[derive(Debug, PartialEq, Eq)]
pub enum EditMoveError {
    BeforeMove(ReadError),
    AfterMove(ReadError),
}

impl WorkspaceReader {
    /// 既知の版を、既存fileを置換せず別名へ移す。
    /// AfterMoveでは呼出し側が移動先を保持し、復旧対象として扱う。
    pub fn move_edit_version(
        &self,
        path: &str,
        expected_hash: &str,
        destination: &Self,
        destination_path: &str,
    ) -> Result<(), EditMoveError> {
        use EditMoveError::{AfterMove, BeforeMove};
        if self.root_device != destination.root_device
            || !expected_hash.strip_prefix("sha256:").is_some_and(|hash| {
                hash.len() == 64 && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
            })
        {
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
