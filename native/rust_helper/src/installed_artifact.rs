//! release専用のWindows配置成果物測定。runtime状態・秘密鍵は走査しない。
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, io::Read, path::Path};

const ROOTS: [&str; 4] = [
    "app",
    "broker",
    "GUI-Shell.brokered.cmd",
    "GUI-Shell.brokered.ps1",
];
const REQUIRED: [&str; 6] = [
    "app/gui_shell_desktop.exe",
    "app/data/app.so",
    "app/flutter_windows.dll",
    "broker/gui_shell_rust_helper.exe",
    "GUI-Shell.brokered.cmd",
    "GUI-Shell.brokered.ps1",
];
const FILE_LIMIT: u64 = 1024 * 1024 * 1024;

#[derive(Serialize, PartialEq, Eq)]
struct Entry {
    path: String,
    kind: &'static str,
    size: u64,
    sha256: Option<String>,
}
#[derive(Serialize)]
struct Manifest {
    format: &'static str,
    version: u32,
    entries: Vec<Entry>,
}

fn metadata(path: &Path) -> Result<fs::Metadata, String> {
    let m = fs::symlink_metadata(path).map_err(|_| "成果物metadata取得失敗")?;
    if m.file_type().is_symlink() {
        return Err("成果物linkを拒否".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_attributes() & 0x400 != 0 {
            return Err("成果物reparse pointを拒否".into());
        }
    }
    if !m.is_dir() && !m.is_file() {
        return Err("成果物はdirectory/通常fileに限定する".into());
    }
    Ok(m)
}
fn component(name: &str) -> Result<(), String> {
    let base = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with(['.', ' '])
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b" ._-()".contains(&b))
        || ["CON", "PRN", "AUX", "NUL"].contains(&base.as_str())
        || (base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && (b'1'..=b'9').contains(&base.as_bytes()[3]))
    {
        return Err("成果物path component不正".into());
    }
    Ok(())
}
fn walk(
    root: &Path,
    relative: &str,
    entries: &mut Vec<Entry>,
    seen: &mut BTreeSet<String>,
    total: &mut u64,
) -> Result<(), String> {
    if relative.split('/').count() > 32 || seen.len() >= 100000 {
        return Err("成果物走査上限超過".into());
    }
    if !seen.insert(relative.to_ascii_lowercase()) {
        return Err("成果物path大小文字衝突".into());
    }
    let path = root.join(relative);
    let m = metadata(&path)?;
    if m.is_dir() {
        entries.push(Entry {
            path: relative.into(),
            kind: "directory",
            size: 0,
            sha256: None,
        });
        for item in fs::read_dir(&path).map_err(|_| "成果物directory読取り失敗")? {
            let item = item.map_err(|_| "成果物entry読取り失敗")?;
            let name = item
                .file_name()
                .into_string()
                .map_err(|_| "成果物path文字列不正")?;
            component(&name)?;
            walk(root, &format!("{relative}/{name}"), entries, seen, total)?;
        }
    } else {
        if m.len() > FILE_LIMIT {
            return Err("成果物file上限超過".into());
        }
        let mut file = fs::File::open(&path)
            .map_err(|_| "成果物file読取り失敗")?
            .take(FILE_LIMIT + 1);
        let mut digest = Sha256::new();
        let mut size = 0;
        let mut buffer = [0u8; 65536];
        loop {
            let n = file.read(&mut buffer).map_err(|_| "成果物byte読取り失敗")?;
            if n == 0 {
                break;
            }
            size += n as u64;
            if size > FILE_LIMIT {
                return Err("成果物file上限超過".into());
            }
            digest.update(&buffer[..n]);
        }
        *total = total.checked_add(size).ok_or("成果物合計上限超過")?;
        if *total > 4 * FILE_LIMIT || size != m.len() {
            return Err("成果物上限超過・採取中の変更".into());
        }
        entries.push(Entry {
            path: relative.into(),
            kind: "file",
            size,
            sha256: Some(format!("sha256:{}", hex::encode(digest.finalize()))),
        });
    }
    let after = metadata(&path)?;
    if m.is_dir() != after.is_dir()
        || m.len() != after.len()
        || m.modified().ok() != after.modified().ok()
    {
        return Err("採取中に成果物metadataが変化".into());
    }
    Ok(())
}
fn scan(root: &Path) -> Result<Vec<u8>, String> {
    if !metadata(root)?.is_dir() {
        return Err("成果物引数はinstalled rootに限定する".into());
    }
    for item in fs::read_dir(root).map_err(|_| "配置root読取り失敗")? {
        let name = item.map_err(|_| "配置entry読取り失敗")?.file_name();
        let name = name.to_str().ok_or("配置entry文字列不正")?;
        if !ROOTS.contains(&name) && !["runtime", "installed_manifest.json"].contains(&name) {
            return Err("配置rootに範囲外entry".into());
        }
    }
    if !metadata(&root.join("app"))?.is_dir() || !metadata(&root.join("broker"))?.is_dir() {
        return Err("app/broker directoryが必要".into());
    }
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    let mut total = 0;
    for relative in ROOTS {
        walk(root, relative, &mut entries, &mut seen, &mut total)?;
    }
    for path in REQUIRED {
        if !entries.iter().any(|e| e.path == path && e.kind == "file") {
            return Err("必須実行成果物がない".into());
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    serde_json::to_vec(&Manifest {
        format: "gui-shell-windows-artifact",
        version: 1,
        entries,
    })
    .map_err(|_| "成果物正本化失敗".into())
}
pub fn canonical(root: &Path) -> Result<Vec<u8>, String> {
    let first = scan(root)?;
    if scan(root)? != first {
        return Err("採取中に配置成果物が変化".into());
    }
    Ok(first)
}
