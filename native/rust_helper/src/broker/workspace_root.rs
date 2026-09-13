//! owner起動設定のrootを開き、対応filesystemと内部資格との分離を確認する。
use cap_fs_ext::DirExt;
use cap_std::fs::Dir;
use serde::{Deserialize, Serialize};
use std::{io::Read, path::{Component, Path, PathBuf}};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StartupConfig {
    pub version: u8,
    pub workspaces: Vec<WorkspaceStartup>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceStartup {
    pub runtime_id: String,
    pub workspace_id: String,
    pub root_path: String,
    pub secret_paths: Vec<String>,
}

pub(crate) fn read_config(path: &Path) -> Result<StartupConfig, &'static str> {
    let mut options=std::fs::OpenOptions::new();options.read(true);
    #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    let file=options.open(path).map_err(|_| "作業領域設定を開けない")?;
    if !file.metadata().map_err(|_| "設定metadataを確認できない")?.is_file() {return Err("作業領域設定は通常fileに限る");}
    let mut bytes=Vec::new();
    file.take(65_537).read_to_end(&mut bytes).map_err(|_| "作業領域設定を読めない")?;
    if bytes.len()>65_536 {return Err("作業領域設定が上限超過");}
    let text=std::str::from_utf8(&bytes).map_err(|_| "作業領域設定はUTF-8に限る")?;
    let config:StartupConfig=super::json_input::read_unique(text).map_err(|_| "作業領域設定の構造が不正")?;
    if config.version!=1 || config.workspaces.len()>16 {return Err("作業領域設定の版または件数が不正");}
    Ok(config)
}

fn device(dir: &Dir) -> Result<u64, &'static str> {
    Ok(cap_fs_ext::MetadataExt::dev(&dir.dir_metadata().map_err(|_| "root metadataを確認できない")?))
}
fn safe_directory(dir: &Dir) -> Result<(), &'static str> {
    let meta=dir.dir_metadata().map_err(|_| "root metadataを確認できない")?;
    if !meta.is_dir() {return Err("rootはdirectoryに限る");}
    #[cfg(windows)] {
        use cap_std::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {return Err("rootのreparse pointを拒否");}
    }
    Ok(())
}

fn parts(path: &Path) -> Result<Vec<String>, &'static str> {
    let raw=path.to_str().ok_or("rootの文字列が不正")?;
    if raw.len()>4096 || !path.is_absolute() {return Err("rootは上限内の絶対pathに限る");}
    // componentsが消すdot要素も、入力の段階で拒否する。
    if raw.split(['/', '\\']).any(|p| matches!(p,"."|"..")) {return Err("rootの相対要素を拒否");}
    let mut result=Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(name) => {
                let part=name.to_str().ok_or("rootの文字列が不正")?;
                crate::workspace_reader::validate_root_part(part).map_err(|_| "rootの要素が不正またはsecret対象")?;
                result.push(part.into());
            }
            Component::Prefix(_) | Component::RootDir => {},
            _ => return Err("rootの相対要素を拒否"),
        }
    }
    if result.is_empty() || result.len()>64 {return Err("volume全体または深すぎるrootを拒否");}
    Ok(result)
}

#[cfg(windows)]
fn open_path(path: &Path) -> Result<(Dir, &'static str), &'static str> {
    use std::path::Prefix;
    let components=parts(path)?;
    let letter=match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => letter,
            _ => return Err("rootはローカルdriveの絶対pathに限る"),
        },
        _ => return Err("rootのdriveが不正"),
    };
    let drive=format!("{}:\\",char::from(letter).to_ascii_uppercase());
    let mut root=Dir::open_ambient_dir(&drive,cap_std::ambient_authority()).map_err(|_| "root driveを開けない")?;
    safe_directory(&root)?;
    let serial=device(&root)?;
    let mut observed_serial=0u32;let mut filesystem=String::new();
    if winsafe::GetDriveType(Some(&drive))!=winsafe::co::DRIVE::FIXED {return Err("rootは固定diskに限る");}
    winsafe::GetVolumeInformation(Some(&drive),None,Some(&mut observed_serial),None,None,Some(&mut filesystem)).map_err(|_| "root filesystemを確認できない")?;
    if filesystem!="NTFS" || serial!=u64::from(observed_serial) {return Err("rootは同一volumeのNTFSに限る");}
    for component in components {
        root=root.open_dir_nofollow(component).map_err(|_| "rootのlinkまたは不在directoryを拒否")?;
        safe_directory(&root)?;
        if device(&root)?!=serial {return Err("root途中の別volumeを拒否");}
    }
    Ok((root,"NTFS"))
}

#[cfg(unix)]
fn open_path(path: &Path) -> Result<(Dir, &'static str), &'static str> {
    let components=parts(path)?;
    let mut root=Dir::open_ambient_dir("/",cap_std::ambient_authority()).map_err(|_| "root directoryを開けない")?;
    for component in components {
        root=root.open_dir_nofollow(component).map_err(|_| "rootのlinkまたは不在directoryを拒否")?;
        safe_directory(&root)?;
    }
    let filesystem=unix_filesystem(&root)?;
    Ok((root,filesystem))
}
#[cfg(target_os="linux")]
fn unix_filesystem(root: &Dir) -> Result<&'static str, &'static str> {
    let fs=rustix::fs::fstatfs(root).map_err(|_| "root filesystemを確認できない")?;
    if fs.f_type as i64==libc::EXT4_SUPER_MAGIC as i64 {Ok("ext4")}
    else if fs.f_type as i64==libc::TMPFS_MAGIC as i64 {Ok("tmpfs")}
    else {Err("root filesystemが対応対象外")}
}
#[cfg(target_os="macos")]
fn unix_filesystem(root: &Dir) -> Result<&'static str, &'static str> {
    let fs=rustix::fs::fstatfs(root).map_err(|_| "root filesystemを確認できない")?;
    let name:Vec<u8>=fs.f_fstypename.iter().take_while(|c| **c!=0).map(|c| *c as u8).collect();
    if name==b"apfs" {Ok("APFS")} else {Err("root filesystemが対応対象外")}
}
#[cfg(all(unix,not(any(target_os="linux",target_os="macos"))))]
fn unix_filesystem(_root: &Dir) -> Result<&'static str, &'static str> {Err("このOSのlocal root登録は未対応")}

fn canonical_protected(path: &Path) -> Result<PathBuf, &'static str> {
    if path.exists() {return std::fs::canonicalize(path).map_err(|_| "保護pathを確認できない");}
    let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let parent=std::fs::canonicalize(parent).map_err(|_| "保護pathの親を確認できない")?;
    Ok(parent.join(path.file_name().ok_or("保護pathが不正")?))
}
fn comparison_parts(path: &Path) -> Vec<String> {
    path.components().map(|part| {
        let text=part.as_os_str().to_string_lossy().into_owned();
        #[cfg(windows)] {text.to_lowercase()}
        #[cfg(not(windows))] {text}
    }).collect()
}

pub(crate) fn open_registered_root(config: &WorkspaceStartup, protected: &[PathBuf]) -> Result<(Dir, &'static str), &'static str> {
    open_isolated_root(Path::new(&config.root_path), protected)
}

pub(crate) fn open_isolated_root(path: &Path, protected: &[PathBuf]) -> Result<(Dir, &'static str), &'static str> {
    let (root,filesystem)=open_path(path)?;
    let canonical=std::fs::canonicalize(path).map_err(|_| "rootの正本pathを確認できない")?;
    let root_parts=comparison_parts(&canonical);
    for protected_path in protected {
        let guarded=comparison_parts(&canonical_protected(protected_path)?);
        if guarded.starts_with(&root_parts) || root_parts.starts_with(&guarded) {return Err("rootがBroker内部store・資格・設定と重なる");}
    }
    let (current,current_fs)=open_path(path)?;
    let original=root.dir_metadata().map_err(|_| "root metadataを確認できない")?;
    let current_meta=current.dir_metadata().map_err(|_| "root metadataを再確認できない")?;
    if current_fs!=filesystem || device(&root)?!=device(&current)? || cap_fs_ext::MetadataExt::ino(&original)!=cap_fs_ext::MetadataExt::ino(&current_meta) {return Err("確認中のroot置換を拒否");}
    Ok((root,filesystem))
}
