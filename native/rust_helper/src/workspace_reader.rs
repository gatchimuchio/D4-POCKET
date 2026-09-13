//! 登録済みdirectory handleだけを使う内部取得器。Brokerの権限評価を代替しない。
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
#[cfg(windows)]
use cap_std::fs::MetadataExt;
use cap_std::fs::{Dir, Metadata, OpenOptions};
use std::io::Read;
use unicode_normalization::UnicodeNormalization;

const MAX_BYTES: u64 = 65_536;
const MAX_ENTRIES: usize = 1024;

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
    pub(crate) fn validate_relative_path(&self, path: &str, allow_root: bool) -> Result<(), ReadError> {
        if path.is_empty() && allow_root {return Ok(());}
        self.allowed(path).map(|_| ())
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
        Ok(Self {
            root,
            secrets: normalized,
        })
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
            if !metadata.is_dir() || reparse(&metadata) {
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
            let metadata = if metadata.is_file() {
                let file = dir
                    .open_with(name, &read_options())
                    .map_err(|_| ReadError::Changed)?;
                file.metadata().map_err(|_| ReadError::Changed)?
            } else {
                metadata
            };
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

    pub fn read(&self, path: &str) -> Result<Vec<u8>, ReadError> {
        self.read_checked(path, || {})
    }

    fn read_checked(&self, path: &str, after_open: impl FnOnce()) -> Result<Vec<u8>, ReadError> {
        let parts = self.allowed(path)?;
        let parent = self.directory(&parts[..parts.len() - 1])?;
        let name = parts[parts.len() - 1];
        let options = read_options();
        let mut file = parent
            .open_with(name, &options)
            .map_err(|_| ReadError::UnsafeFile)?;
        let before = file.metadata().map_err(|_| ReadError::Unavailable)?;
        if !regular(&before) {
            return Err(ReadError::UnsafeFile);
        }
        if before.len() > MAX_BYTES {
            return Err(ReadError::TooLarge(before.len()));
        }
        after_open();
        let mut bytes = Vec::new();
        (&mut file)
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ReadError::Unavailable)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(ReadError::TooLarge(bytes.len() as u64));
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
            || before.len() != bytes.len() as u64
            || after.len() != before.len()
            || before.modified().ok().is_none()
            || before.modified().ok() != after.modified().ok()
            || current.modified().ok() != after.modified().ok()
        {
            return Err(ReadError::Changed);
        }
        Ok(bytes)
    }
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
    }
}
