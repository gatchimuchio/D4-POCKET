//! 登録済みdirectory handleだけを使う内部取得器。Brokerの権限評価を代替しない。
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
#[cfg(windows)]
use cap_std::fs::MetadataExt;
use cap_std::fs::{Dir, Metadata, OpenOptions};
use std::io::Read;
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

const MAX_BYTES: u64 = 65_536;
pub const MAX_COMPARISON_BYTES: u64 = 128 * 1024 * 1024;

/// 本文を保持しない大fileも、全体取得済みのmetadataを持つ。
pub struct ComparedFile {
    pub bytes: usize,
    pub sha256: String,
    pub(crate) content: Option<Vec<u8>>,
}
/// 比較対象の探索結果。secret subtreeの内部件数や名前は含めない。
#[derive(Debug, PartialEq, Eq)]
pub struct ComparisonInventory {
    pub files: Vec<String>,
    pub excluded_secrets: usize,
}
pub const MAX_INVENTORY_ENTRIES: usize = 4096;
const MAX_ENTRIES: usize = 1024;
const MAX_SECRET_TREE_DEPTH: usize = 64;

#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
    InvalidPath,
    SecretPath,
    UnsafeFile,
    Unavailable,
    Changed,
    TooLarge(u64),
    TooManyEntries,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
}

#[derive(Debug, PartialEq, Eq)]
pub struct WorkspaceEntry {
    pub path: String,
    pub kind: EntryKind,
    pub bytes: Option<u64>,
}

pub struct WorkspaceReader {
    root: Dir,
    secrets: Vec<String>,
    root_device: u64,
}

pub(crate) fn validate_root_part(part: &str) -> Result<(), ReadError> {
    if path_parts(part)?.len()!=1 || secret_part(part) {return Err(ReadError::SecretPath);}
    Ok(())
}

fn path_parts(path: &str) -> Result<Vec<&str>, ReadError> {
    if path.is_empty() || path.len() > 1024 || path.nfc().collect::<String>() != path {
        return Err(ReadError::InvalidPath);
    }
    let parts: Vec<_> = path.split('/').collect();
    if parts.len() > 64 {
        return Err(ReadError::InvalidPath);
    }
    for part in &parts {
        let stem = part
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$")
            || ["COM", "LPT"].iter().any(|prefix| {
                stem.strip_prefix(prefix).is_some_and(|tail| {
                    matches!(
                        tail,
                        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                    )
                })
            });
        if part.is_empty()
            || *part == "."
            || *part == ".."
            || part.len() > 255
            || part.ends_with([' ', '.'])
            || part.chars().any(|c| {
                c.is_control() || matches!(c, '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*' | '~')
            })
            || reserved
        {
            return Err(ReadError::InvalidPath);
        }
    }
    Ok(parts)
}

fn secret_part(part: &str) -> bool {
    let part = part.to_lowercase();
    matches!(
        part.as_str(),
        ".env" | ".ssh" | ".gnupg" | ".git" | "secrets" | "credentials"
    ) || part.starts_with(".env.")
        || [".pem", ".key", ".p12", ".pfx"]
            .iter()
            .any(|suffix| part.ends_with(suffix))
}

fn regular(metadata: &Metadata) -> bool {
    metadata.is_file() && !reparse(metadata) && links(metadata) == Some(1)
}
#[cfg(windows)]
fn reparse(metadata: &Metadata) -> bool {
    metadata.file_attributes() & 0x400 != 0
}
#[cfg(not(windows))]
fn reparse(metadata: &Metadata) -> bool {
    metadata.file_type().is_symlink()
}
fn links(metadata: &Metadata) -> Option<u64> {
    Some(cap_fs_ext::MetadataExt::nlink(metadata))
}
fn same_file(a: &Metadata, b: &Metadata) -> bool {
    cap_fs_ext::MetadataExt::ino(a) != 0
        && cap_fs_ext::MetadataExt::dev(a) == cap_fs_ext::MetadataExt::dev(b)
        && cap_fs_ext::MetadataExt::ino(a) == cap_fs_ext::MetadataExt::ino(b)
}

fn read_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    options
}

impl WorkspaceReader {
    pub(crate) fn registered_secret_paths(&self) -> &[String] {
        &self.secrets
    }

    pub(crate) fn validate_relative_path(&self, path: &str, allow_root: bool) -> Result<(), ReadError> {
        if path.is_empty() && allow_root {return Ok(());}
        self.allowed(path).map(|_| ())
    }

    pub(crate) fn validate_registered_secret_path(path: &str) -> Result<(), ReadError> {
        path_parts(path).map(|_| ())
    }
    /// dirはowner登録経路が開いたhandle。通常要求からambient pathを開かない。
    pub fn from_registered_dir(root: Dir, secrets: &[String]) -> Result<Self, ReadError> {
        if secrets.len() > 256 {
            return Err(ReadError::InvalidPath);
        }
        let mut normalized = Vec::new();
        for secret in secrets {
            path_parts(secret)?;
            normalized.push(secret.to_lowercase());
        }
        let metadata=root.dir_metadata().map_err(|_| ReadError::Unavailable)?;
        if !metadata.is_dir() || reparse(&metadata) {return Err(ReadError::UnsafeFile);}
        let root_device=cap_fs_ext::MetadataExt::dev(&metadata);
        let reader = Self {
            root,
            secrets: normalized,
            root_device,
        };
        reader.validate_registered_secret_aliases()?;
        Ok(reader)
    }

    fn validate_registered_secret_aliases(&self) -> Result<(), ReadError> {
        let mut inspected_entries = 0usize;
        'secrets: for secret in &self.secrets {
            let parts = path_parts(secret)?;
            let mut parent = self.root.try_clone().map_err(|_| ReadError::Unavailable)?;
            for part in &parts[..parts.len() - 1] {
                match parent.open_dir_nofollow(part) {
                    Ok(next) => parent = next,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        continue 'secrets;
                    }
                    Err(_) => return Err(ReadError::UnsafeFile),
                }
            }
            let name = *parts.last().ok_or(ReadError::InvalidPath)?;
            let observed = match parent.symlink_metadata(name) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    continue 'secrets;
                }
                Err(_) => return Err(ReadError::Unavailable),
            };
            if reparse(&observed)
                || cap_fs_ext::MetadataExt::dev(&observed) != self.root_device
            {
                return Err(ReadError::UnsafeFile);
            }
            if observed.is_file() {
                let file = parent
                    .open_with(name, &read_options())
                    .map_err(|_| ReadError::Changed)?;
                let opened = file.metadata().map_err(|_| ReadError::Changed)?;
                if !same_file(&observed, &opened) {
                    return Err(ReadError::Changed);
                }
                if !regular(&opened) {
                    return Err(ReadError::UnsafeFile);
                }
            } else if observed.is_dir() {
                let directory = parent
                    .open_dir_nofollow(name)
                    .map_err(|_| ReadError::Changed)?;
                let opened = directory.dir_metadata().map_err(|_| ReadError::Changed)?;
                if !same_file(&observed, &opened)
                    || !opened.is_dir()
                    || reparse(&opened)
                    || cap_fs_ext::MetadataExt::dev(&opened) != self.root_device
                {
                    return Err(ReadError::Changed);
                }
                validate_secret_tree(
                    &directory,
                    self.root_device,
                    0,
                    &mut inspected_entries,
                )?;
            } else {
                return Err(ReadError::UnsafeFile);
            }
        }
        Ok(())
    }

    fn allowed<'a>(&self, path: &'a str) -> Result<Vec<&'a str>, ReadError> {
        let parts = path_parts(path)?;
        let folded = path.to_lowercase();
        if parts.iter().any(|part| secret_part(part))
            || self
                .secrets
                .iter()
                .any(|secret| folded == *secret || folded.starts_with(&format!("{secret}/")))
        {
            return Err(ReadError::SecretPath);
        }
        Ok(parts)
    }

    fn directory(&self, parts: &[&str]) -> Result<Dir, ReadError> {
        let mut dir = self.root.try_clone().map_err(|_| ReadError::Unavailable)?;
        for part in parts {
            let next = dir
                .open_dir_nofollow(part)
                .map_err(|_| ReadError::UnsafeFile)?;
            let metadata = next.dir_metadata().map_err(|_| ReadError::Unavailable)?;
            if !metadata.is_dir() || reparse(&metadata) || cap_fs_ext::MetadataExt::dev(&metadata)!=self.root_device {
                return Err(ReadError::UnsafeFile);
            }
            dir = next;
        }
        Ok(dir)
    }

    /// 1階層だけを列挙する。secret名・link・特殊fileを結果へ含めない。
    pub fn list(&self, path: &str) -> Result<Vec<WorkspaceEntry>, ReadError> {
        let parts = if path.is_empty() {
            vec![]
        } else {
            self.allowed(path)?
        };
        let dir = self.directory(&parts)?;
        let mut entries = Vec::new();
        for (count, entry) in dir
            .entries()
            .map_err(|_| ReadError::Unavailable)?
            .enumerate()
        {
            if count >= MAX_ENTRIES {
                return Err(ReadError::TooManyEntries);
            }
            let entry = entry.map_err(|_| ReadError::Unavailable)?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let relative = if path.is_empty() {
                name.to_owned()
            } else {
                format!("{path}/{name}")
            };
            if self.allowed(&relative).is_err() {
                continue;
            }
            let metadata = dir.symlink_metadata(name).map_err(|_| ReadError::Changed)?;
            if reparse(&metadata) {
                continue;
            }
            if !metadata.is_file() && !metadata.is_dir() {continue;}
            let metadata = if metadata.is_file() {
                let file = dir
                    .open_with(name, &read_options())
                    .map_err(|_| ReadError::Changed)?;
                file.metadata().map_err(|_| ReadError::Changed)?
            } else if metadata.is_dir() {
                dir.open_dir_nofollow(name).map_err(|_| ReadError::Changed)?.dir_metadata().map_err(|_| ReadError::Changed)?
            } else {
                metadata
            };
            if cap_fs_ext::MetadataExt::dev(&metadata)!=self.root_device {continue;}
            let kind = if metadata.is_dir() {
                EntryKind::Directory
            } else if regular(&metadata) {
                EntryKind::File
            } else {
                continue;
            };
            entries.push(WorkspaceEntry {
                path: relative,
                bytes: if metadata.is_file() {
                    Some(metadata.len())
                } else {
                    None
                },
                kind,
            });
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    /// 全体比較用。secret以外の省略を許可せず、部分結果を返さない。
    pub fn comparison_inventory(&self) -> Result<ComparisonInventory, ReadError> {
        self.inventory_checked(MAX_INVENTORY_ENTRIES, || {})
    }

    fn inventory_checked(&self, budget: usize, after_walk: impl FnOnce()) -> Result<ComparisonInventory, ReadError> {
        let root_metadata=self.root.dir_metadata().map_err(|_| ReadError::Unavailable)?;
        let mut pending=vec![(String::new(),root_metadata)];
        let mut directories=Vec::new();
        let mut files=Vec::new();
        let mut excluded_secrets=0;
        let mut total=0usize;
        while let Some((path, expected))=pending.pop() {
            let parts=if path.is_empty() {vec![]} else {self.allowed(&path)?};
            let dir=self.directory(&parts)?;
            let before=dir.dir_metadata().map_err(|_| ReadError::Unavailable)?;
            if !same_file(&expected,&before) || expected.modified().ok().is_none()
                || expected.modified().ok()!=before.modified().ok() {return Err(ReadError::Changed);}
            for (count,entry) in dir.entries().map_err(|_| ReadError::Unavailable)?.enumerate() {
                if count>=MAX_ENTRIES || total>=budget.min(MAX_INVENTORY_ENTRIES) {return Err(ReadError::TooManyEntries);}
                total+=1;
                let entry=entry.map_err(|_| ReadError::Unavailable)?;
                let name=entry.file_name();
                let name=name.to_str().ok_or(ReadError::InvalidPath)?;
                let relative=if path.is_empty() {name.to_owned()} else {format!("{path}/{name}")};
                match self.allowed(&relative) {
                    Err(ReadError::SecretPath)=>{excluded_secrets+=1;continue;},
                    Err(error)=>return Err(error),
                    Ok(_)=>{},
                }
                let observed=dir.symlink_metadata(name).map_err(|_| ReadError::Changed)?;
                if reparse(&observed) || cap_fs_ext::MetadataExt::dev(&observed)!=self.root_device {
                    return Err(ReadError::UnsafeFile);
                }
                if observed.is_dir() {
                    let child=dir.open_dir_nofollow(name).map_err(|_| ReadError::Changed)?;
                    let opened=child.dir_metadata().map_err(|_| ReadError::Changed)?;
                    if !same_file(&observed,&opened) || reparse(&opened) || !opened.is_dir() {return Err(ReadError::Changed);}
                    pending.push((relative,opened));
                } else {
                    if !regular(&observed) {return Err(ReadError::UnsafeFile);}
                    let file=dir.open_with(name,&read_options()).map_err(|_| ReadError::Changed)?;
                    let opened=file.metadata().map_err(|_| ReadError::Changed)?;
                    if !same_file(&observed,&opened) || !regular(&opened) {return Err(ReadError::Changed);}
                    files.push(relative);
                }
            }
            directories.push((path,before));
        }
        after_walk();
        for (path,before) in directories {
            let parts=if path.is_empty() {vec![]} else {self.allowed(&path)?};
            let current=self.directory(&parts).map_err(|_| ReadError::Changed)?;
            let after=current.dir_metadata().map_err(|_| ReadError::Changed)?;
            if !same_file(&before,&after) || before.modified().ok().is_none()
                || before.modified().ok()!=after.modified().ok() {return Err(ReadError::Changed);}
        }
        files.sort();
        Ok(ComparisonInventory {files,excluded_secrets})
    }

    /// 差分用の存在状態。拒否・競合・linkをfile不在へ変換しない。
    pub fn read_version(&self, path: &str) -> Result<Option<Vec<u8>>, ReadError> {
        self.read_version_checked(path, || {})
    }

    fn read_version_checked(&self, path: &str, missing_observed: impl FnOnce()) -> Result<Option<Vec<u8>>, ReadError> {
        if self.is_missing_checked(path, missing_observed)? {return Ok(None);}
        self.read(path).map(Some)
    }

    pub fn read_comparison_version(&self, path: &str, budget: u64) -> Result<Option<ComparedFile>, ReadError> {
        if self.is_missing_checked(path, || {})? {return Ok(None);}
        self.read_file_checked(path, budget.min(MAX_COMPARISON_BYTES), || {}).map(Some)
    }

    fn is_missing_checked(&self, path: &str, missing_observed: impl FnOnce()) -> Result<bool, ReadError> {
        let parts = self.allowed(path)?;
        let mut dir = self.root.try_clone().map_err(|_| ReadError::Unavailable)?;
        for (index, part) in parts.iter().enumerate() {
            let metadata = match dir.symlink_metadata(part) {
                Ok(value) => value,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    missing_observed();
                    let current = self.directory(&parts[..index]).map_err(|_| ReadError::Changed)?;
                    let original_metadata = dir.dir_metadata().map_err(|_| ReadError::Changed)?;
                    let current_metadata = current.dir_metadata().map_err(|_| ReadError::Changed)?;
                    if !same_file(&original_metadata, &current_metadata) {return Err(ReadError::Changed);}
                    return match current.symlink_metadata(part) {
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
                        _ => Err(ReadError::Changed),
                    };
                },
                Err(_) => return Err(ReadError::Unavailable),
            };
            if reparse(&metadata) {return Err(ReadError::UnsafeFile);}
            if index + 1 == parts.len() {
                if !regular(&metadata) {return Err(ReadError::UnsafeFile);}
                // 存在確認後の消失も、既存取得器の競合・取得拒否として扱う。
                return Ok(false);
            }
            if !metadata.is_dir() {return Err(ReadError::UnsafeFile);}
            let next = dir.open_dir_nofollow(part).map_err(|_| ReadError::Changed)?;
            let opened = next.dir_metadata().map_err(|_| ReadError::Unavailable)?;
            if !opened.is_dir() || reparse(&opened) || cap_fs_ext::MetadataExt::dev(&opened) != self.root_device {
                return Err(ReadError::UnsafeFile);
            }
            dir = next;
        }
        Err(ReadError::InvalidPath)
    }

    pub fn read(&self, path: &str) -> Result<Vec<u8>, ReadError> {
        self.read_checked(path, || {})
    }

    fn read_checked(&self, path: &str, after_open: impl FnOnce()) -> Result<Vec<u8>, ReadError> {
        let result=self.read_file_checked(path,MAX_BYTES,after_open)?;
        result.content.ok_or(ReadError::TooLarge(result.bytes as u64))
    }

    fn read_file_checked(&self, path: &str, limit: u64, after_open: impl FnOnce()) -> Result<ComparedFile, ReadError> {
        let parts = self.allowed(path)?;
        let parent = self.directory(&parts[..parts.len() - 1])?;
        let name = parts[parts.len() - 1];
        let options = read_options();
        let mut file = parent
            .open_with(name, &options)
            .map_err(|_| ReadError::UnsafeFile)?;
        let before = file.metadata().map_err(|_| ReadError::Unavailable)?;
        if !regular(&before) || cap_fs_ext::MetadataExt::dev(&before)!=self.root_device {
            return Err(ReadError::UnsafeFile);
        }
        if before.len() > limit {
            return Err(ReadError::TooLarge(before.len()));
        }
        after_open();
        let mut content=Some(Vec::new());
        let mut count=0u64;
        let mut hasher=Sha256::new();
        let mut buffer=[0u8;65_536];
        let mut limited=(&mut file).take(limit+1);
        loop {
            let read=limited.read(&mut buffer).map_err(|_| ReadError::Unavailable)?;
            if read==0 {break;}
            count+=read as u64;
            if count > limit {return Err(ReadError::TooLarge(count));}
            hasher.update(&buffer[..read]);
            if count <= MAX_BYTES {
                if let Some(bytes)=content.as_mut() {bytes.extend_from_slice(&buffer[..read]);}
            } else {content=None;}
        }
        let after = file.metadata().map_err(|_| ReadError::Unavailable)?;
        let current_parent = self.directory(&parts[..parts.len() - 1])?;
        // path経由のmetadataはWindowsでfile IDが欠け得るため、再openしたhandleで照合する。
        let current = current_parent
            .open_with(name, &options)
            .map_err(|_| ReadError::Changed)?;
        let current = current.metadata().map_err(|_| ReadError::Changed)?;
        if !regular(&after)
            || !regular(&current)
            || !same_file(&before, &after)
            || !same_file(&after, &current)
            || before.len() != count
            || after.len() != before.len()
            || current.len() != after.len()
            || before.modified().ok().is_none()
            || before.modified().ok() != after.modified().ok()
            || current.modified().ok() != after.modified().ok()
        {
            return Err(ReadError::Changed);
        }
        Ok(ComparedFile {bytes:count as usize,sha256:format!("sha256:{}",hex::encode(hasher.finalize())),content})
    }
}

fn validate_secret_tree(
    directory: &Dir,
    root_device: u64,
    depth: usize,
    inspected_entries: &mut usize,
) -> Result<(), ReadError> {
    if depth >= MAX_SECRET_TREE_DEPTH {
        return Err(ReadError::TooManyEntries);
    }
    let before = directory
        .dir_metadata()
        .map_err(|_| ReadError::Unavailable)?;
    if !before.is_dir()
        || reparse(&before)
        || cap_fs_ext::MetadataExt::dev(&before) != root_device
    {
        return Err(ReadError::UnsafeFile);
    }
    for entry in directory
        .entries()
        .map_err(|_| ReadError::Unavailable)?
    {
        *inspected_entries = inspected_entries
            .checked_add(1)
            .ok_or(ReadError::TooManyEntries)?;
        if *inspected_entries > MAX_INVENTORY_ENTRIES {
            return Err(ReadError::TooManyEntries);
        }
        let entry = entry.map_err(|_| ReadError::Changed)?;
        let name = entry.file_name();
        let name = name.to_str().ok_or(ReadError::InvalidPath)?;
        let observed = directory
            .symlink_metadata(name)
            .map_err(|_| ReadError::Changed)?;
        if reparse(&observed) || cap_fs_ext::MetadataExt::dev(&observed) != root_device {
            return Err(ReadError::UnsafeFile);
        }
        if observed.is_dir() {
            let child = directory
                .open_dir_nofollow(name)
                .map_err(|_| ReadError::Changed)?;
            let opened = child.dir_metadata().map_err(|_| ReadError::Changed)?;
            if !same_file(&observed, &opened) || !opened.is_dir() || reparse(&opened) {
                return Err(ReadError::Changed);
            }
            validate_secret_tree(&child, root_device, depth + 1, inspected_entries)?;
        } else if observed.is_file() {
            let file = directory
                .open_with(name, &read_options())
                .map_err(|_| ReadError::Changed)?;
            let opened = file.metadata().map_err(|_| ReadError::Changed)?;
            if !same_file(&observed, &opened) {
                return Err(ReadError::Changed);
            }
            if !regular(&opened) {
                return Err(ReadError::UnsafeFile);
            }
        } else {
            return Err(ReadError::UnsafeFile);
        }
    }
    let after = directory
        .dir_metadata()
        .map_err(|_| ReadError::Changed)?;
    if !same_file(&before, &after) || before.modified().ok() != after.modified().ok() {
        return Err(ReadError::Changed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let mut random = [0u8; 16];
            getrandom::getrandom(&mut random).unwrap();
            let path =
                std::env::temp_dir().join(format!("gui-shell-reader-test-{}", hex::encode(random)));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn reader(&self) -> WorkspaceReader {
            WorkspaceReader::from_registered_dir(
                Dir::open_ambient_dir(&self.0, cap_std::ambient_authority()).unwrap(),
                &["private-data".into()],
            )
            .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn inventory_recurses_and_excludes_only_declared_secrets() {
        let f=Fixture::new();let reader=f.reader();
        fs::create_dir_all(f.0.join("src/deep")).unwrap();
        fs::create_dir_all(f.0.join("private-data/nested")).unwrap();
        fs::write(f.0.join("private-data/nested/hidden"),b"secret").unwrap();
        fs::write(f.0.join(".env"),b"secret").unwrap();
        fs::write(f.0.join("z"),b"").unwrap();
        fs::write(f.0.join("src/deep/a"),b"file").unwrap();
        assert_eq!(reader.comparison_inventory(),Ok(ComparisonInventory {
            files:vec!["src/deep/a".into(),"z".into()],excluded_secrets:2}));
        assert_eq!(reader.inventory_checked(5,||{}),Err(ReadError::TooManyEntries));
        assert_eq!(reader.inventory_checked(6,||{}).unwrap().files.len(),2);
        fs::hard_link(f.0.join("z"),f.0.join("alias")).unwrap();
        assert_eq!(reader.comparison_inventory(),Err(ReadError::UnsafeFile));
    }

    #[test]
    fn inventory_rejects_directory_mutation_and_never_returns_partial_paths() {
        let f=Fixture::new();let reader=f.reader();
        fs::create_dir(f.0.join("nested")).unwrap();
        fs::write(f.0.join("nested/old"),b"old").unwrap();
        assert_eq!(reader.inventory_checked(MAX_INVENTORY_ENTRIES,|| {
            fs::remove_file(f.0.join("nested/old")).unwrap();
            fs::remove_dir(f.0.join("nested")).unwrap();
        }),Err(ReadError::Changed));
    }

    #[cfg(unix)]
    #[test]
    fn inventory_rejects_invalid_names_instead_of_omitting_them() {
        let f=Fixture::new();let reader=f.reader();
        fs::write(f.0.join("bad:name"),b"").unwrap();
        assert_eq!(reader.comparison_inventory(),Err(ReadError::InvalidPath));
    }

    #[test]
    fn comparison_hashes_all_bytes_without_retaining_large_content() {
        let f=Fixture::new();let reader=f.reader();
        let bytes=vec![b'x';65_537];fs::write(f.0.join("large"),&bytes).unwrap();
        let value=reader.read_comparison_version("large",MAX_COMPARISON_BYTES).unwrap().unwrap();
        assert_eq!(value.bytes,bytes.len());assert_eq!(value.sha256,crate::audit_hash::sha256_tagged(&bytes));
        assert!(value.content.is_none());
        assert_eq!(reader.read("large"),Err(ReadError::TooLarge(65_537)));
        assert!(matches!(reader.read_comparison_version("large",65_536),Err(ReadError::TooLarge(65_537))));
        fs::write(f.0.join("empty"),b"").unwrap();
        let empty=reader.read_comparison_version("empty",0).unwrap().unwrap();
        assert_eq!(empty.content,Some(vec![]));assert_eq!(empty.sha256,crate::audit_hash::sha256_tagged(b""));
        assert!(reader.read_comparison_version("missing",0).unwrap().is_none());
    }

    #[test]
    fn comparison_rejects_growth_and_truncation_without_partial_hash() {
        let f=Fixture::new();let reader=f.reader();
        fs::write(f.0.join("file"),vec![b'a';65_537]).unwrap();
        assert!(matches!(reader.read_file_checked("file",MAX_COMPARISON_BYTES, || {
            fs::write(f.0.join("file"),vec![b'b';65_538]).unwrap();
        }),Err(ReadError::Changed)));
        assert!(matches!(reader.read_file_checked("file",MAX_COMPARISON_BYTES, || {
            fs::write(f.0.join("file"),b"short").unwrap();
        }),Err(ReadError::Changed)));
    }

    #[test]
    fn absence_recheck_rejects_created_file_and_replaced_parent() {
        let f=Fixture::new();
        fs::create_dir(f.0.join("parent")).unwrap();
        let reader=f.reader();
        assert_eq!(reader.read_version_checked("parent/file", || {
            fs::write(f.0.join("parent/file"),b"appeared").unwrap();
        }),Err(ReadError::Changed));
        fs::remove_file(f.0.join("parent/file")).unwrap();
        let replaced=reader.read_version_checked("parent/file", || {
            let rename=fs::rename(f.0.join("parent"),f.0.join("old-parent"));
            #[cfg(windows)]
            assert_eq!(rename.unwrap_err().raw_os_error(),Some(32));
            #[cfg(not(windows))]
            {
                rename.unwrap();
                fs::create_dir(f.0.join("parent")).unwrap();
            }
        });
        #[cfg(windows)]
        assert_eq!(replaced,Ok(None)); // 開いたdirectoryの改名自体をOSが拒否する。
        #[cfg(not(windows))]
        assert_eq!(replaced,Err(ReadError::Changed));
        assert_eq!(reader.read_version("parent/file"),Ok(None));
    }

    #[test]
    fn real_files_are_read_and_listing_excludes_secret_names() {
        let f = Fixture::new();
        fs::create_dir(f.0.join("src")).unwrap();
        fs::write(f.0.join("src/日本語.txt"), "本文\r\n").unwrap();
        fs::write(f.0.join(".ENV.production"), "secret").unwrap();
        fs::create_dir(f.0.join("private-data")).unwrap();
        let reader = f.reader();
        assert_eq!(
            reader.read("src/日本語.txt").unwrap(),
            "本文\r\n".as_bytes()
        );
        assert_eq!(
            reader.list("").unwrap(),
            vec![WorkspaceEntry {
                path: "src".into(),
                kind: EntryKind::Directory,
                bytes: None
            }]
        );
        assert_eq!(reader.list("src").unwrap()[0].path, "src/日本語.txt");
    }

    #[test]
    fn traversal_aliases_and_secret_paths_are_denied_before_io() {
        let f = Fixture::new();
        let reader = f.reader();
        for path in [
            "/outside",
            "../outside",
            "src/../file",
            "a//b",
            "a\\b",
            "C:/file",
            "a:stream",
            "CON",
            "nul.txt",
            "a.",
            "a ",
            "a\n",
            "e\u{301}",
        ] {
            assert_eq!(reader.read(path), Err(ReadError::InvalidPath), "{path:?}");
        }
        for path in [
            ".env",
            "x/.ENV.prod",
            ".git/config",
            "Private-Data/a",
            "key.pem",
            "x/credentials/a",
        ] {
            assert_eq!(reader.read(path), Err(ReadError::SecretPath), "{path:?}");
        }
    }

    #[test]
    fn hardlinks_and_oversized_files_do_not_return_partial_content() {
        let f = Fixture::new();
        let outside = Fixture::new();
        fs::write(outside.0.join("secret"), "not for workspace").unwrap();
        fs::hard_link(outside.0.join("secret"), f.0.join("alias")).unwrap();
        let reader = f.reader();
        assert_eq!(reader.read("alias"), Err(ReadError::UnsafeFile));
        fs::write(f.0.join("large"), vec![65; MAX_BYTES as usize + 1]).unwrap();
        assert_eq!(
            reader.read("large"),
            Err(ReadError::TooLarge(MAX_BYTES + 1))
        );
    }

    #[test]
    fn registered_secret_file_or_tree_with_hardlink_alias_is_rejected() {
        let file_fixture = Fixture::new();
        fs::create_dir(file_fixture.0.join("private")).unwrap();
        fs::write(file_fixture.0.join("private/secret.txt"), b"synthetic secret").unwrap();
        fs::hard_link(
            file_fixture.0.join("private/secret.txt"),
            file_fixture.0.join("public-alias.txt"),
        )
        .unwrap();
        let file_root = Dir::open_ambient_dir(&file_fixture.0, cap_std::ambient_authority()).unwrap();
        assert!(matches!(
            WorkspaceReader::from_registered_dir(file_root, &["private/secret.txt".into()]),
            Err(ReadError::UnsafeFile)
        ));

        let tree_fixture = Fixture::new();
        fs::create_dir_all(tree_fixture.0.join("private/secrets/nested")).unwrap();
        fs::write(tree_fixture.0.join("private/secrets/nested/token"), b"synthetic secret")
            .unwrap();
        fs::hard_link(
            tree_fixture.0.join("private/secrets/nested/token"),
            tree_fixture.0.join("public-alias.txt"),
        )
        .unwrap();
        let tree_root = Dir::open_ambient_dir(&tree_fixture.0, cap_std::ambient_authority()).unwrap();
        assert!(matches!(
            WorkspaceReader::from_registered_dir(tree_root, &["private/secrets".into()]),
            Err(ReadError::UnsafeFile)
        ));
    }

    #[test]
    fn missing_registered_secret_path_remains_valid() {
        let fixture = Fixture::new();
        let root = Dir::open_ambient_dir(&fixture.0, cap_std::ambient_authority()).unwrap();
        assert!(WorkspaceReader::from_registered_dir(
            root,
            &["future/private/secret.txt".into()]
        )
        .is_ok());
    }

    #[test]
    fn replacement_and_growth_after_open_are_rejected() {
        let f = Fixture::new();
        fs::write(f.0.join("file"), "original").unwrap();
        let reader = f.reader();
        assert_eq!(
            reader.read_checked("file", || {
                fs::rename(f.0.join("file"), f.0.join("old")).unwrap();
                fs::write(f.0.join("file"), "replaced").unwrap();
            }),
            Err(ReadError::Changed)
        );
        assert!(matches!(
            reader.read_checked("file", || {
                fs::write(f.0.join("file"), vec![65; MAX_BYTES as usize + 1]).unwrap();
            }),
            Err(ReadError::TooLarge(_))
        ));
    }

    #[test]
    fn entry_limit_and_dos_short_name_alias_are_rejected() {
        let f = Fixture::new();
        let reader = f.reader();
        assert_eq!(reader.read("PRIVAT~1/file"), Err(ReadError::InvalidPath));
        for i in 0..=MAX_ENTRIES {
            fs::write(f.0.join(format!("file-{i}")), b"").unwrap();
        }
        assert_eq!(reader.list(""), Err(ReadError::TooManyEntries));
        assert_eq!(reader.comparison_inventory(),Err(ReadError::TooManyEntries));
    }
}
