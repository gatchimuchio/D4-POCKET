//! Windows製品の導入先をBroker所有の固定規則から導出する。
//!
//! 固定path計画、Broker専用のversions directory capability取得、Start Menu shortcut登録を行う。
//! package展開、Permission／Approval／Auditは行わず、実作用はBroker consumerが統治する。

#[cfg(windows)]
use cap_fs_ext::OsMetadataExt as _;
use cap_fs_ext::{DirExt, MetadataExt as CapMetadataExt};
#[cfg(windows)]
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt as _};
use cap_std::fs::Dir;
#[cfg(windows)]
use cap_std::fs::OpenOptions;
use std::path::{Component, Path, PathBuf};

const INSTALL_DIRECTORY: &str = "Programs";
const PRODUCT_DIRECTORY: &str = "D4 Pocket";
const VERSIONS_DIRECTORY: &str = "versions";
#[cfg(windows)]
const START_MENU_PROGRAMS_DIRECTORY: &str = "Programs";
#[cfg(windows)]
const START_MENU_PRODUCT_DIRECTORY: &str = "D4 Pocket";
#[cfg(windows)]
const START_MENU_SHORTCUT_FILE: &str = "D4 Pocket.lnk";
#[cfg(windows)]
const MAX_START_MENU_SHORTCUT_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductInstallPlan {
    pub(crate) root: PathBuf,
    pub(crate) version_directory: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductInstallPlanError(pub(crate) &'static str);

/// LocalAppData配下の固定install `versions` directoryをcapabilityで開く。
pub(crate) fn open_product_versions_directory(
    local_app_data: &Path,
    app_id: &str,
) -> Result<(PathBuf, Dir), ProductInstallPlanError> {
    if !local_app_data.is_absolute()
        || local_app_data
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(ProductInstallPlanError(
            "product_install_local_app_data_invalid",
        ));
    }
    if !valid_identity(app_id) {
        return Err(ProductInstallPlanError(
            "product_install_app_identity_invalid",
        ));
    }
    let metadata = std::fs::symlink_metadata(local_app_data)
        .map_err(|_| ProductInstallPlanError("product_install_local_app_data_unavailable"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(ProductInstallPlanError(
            "product_install_local_app_data_invalid",
        ));
    }
    let local_app_data = std::fs::canonicalize(local_app_data)
        .map_err(|_| ProductInstallPlanError("product_install_local_app_data_unavailable"))?;
    let root = Dir::open_ambient_dir(&local_app_data, cap_std::ambient_authority())
        .map_err(|_| ProductInstallPlanError("product_install_local_app_data_unavailable"))?;
    let root_metadata = root
        .dir_metadata()
        .map_err(|_| ProductInstallPlanError("product_install_local_app_data_unavailable"))?;
    if !root_metadata.is_dir() || is_cap_reparse_point(&root_metadata) {
        return Err(ProductInstallPlanError(
            "product_install_local_app_data_invalid",
        ));
    }
    let root_device = CapMetadataExt::dev(&root_metadata);
    let mut current = root;
    for component in [
        INSTALL_DIRECTORY,
        PRODUCT_DIRECTORY,
        app_id,
        VERSIONS_DIRECTORY,
    ] {
        current = open_or_create_child_directory(&current, component, root_device)?;
    }
    let versions_path = local_app_data
        .join(INSTALL_DIRECTORY)
        .join(PRODUCT_DIRECTORY)
        .join(app_id)
        .join(VERSIONS_DIRECTORY);
    Ok((versions_path, current))
}

/// 既存の固定製品rootだけをno-followで開く。起動時に導入directoryを作らない。
pub(crate) fn open_existing_product_root(
    local_app_data: &Path,
    app_id: &str,
) -> Result<(PathBuf, Dir), ProductInstallPlanError> {
    validate_install_inputs(local_app_data, app_id)?;
    let metadata = std::fs::symlink_metadata(local_app_data)
        .map_err(|_| ProductInstallPlanError("product_install_local_app_data_unavailable"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(ProductInstallPlanError(
            "product_install_local_app_data_invalid",
        ));
    }
    let local_app_data = std::fs::canonicalize(local_app_data)
        .map_err(|_| ProductInstallPlanError("product_install_local_app_data_unavailable"))?;
    let root = Dir::open_ambient_dir(&local_app_data, cap_std::ambient_authority())
        .map_err(|_| ProductInstallPlanError("product_install_local_app_data_unavailable"))?;
    let root_metadata = root
        .dir_metadata()
        .map_err(|_| ProductInstallPlanError("product_install_local_app_data_unavailable"))?;
    if !root_metadata.is_dir() || is_cap_reparse_point(&root_metadata) {
        return Err(ProductInstallPlanError(
            "product_install_local_app_data_invalid",
        ));
    }
    let root_device = CapMetadataExt::dev(&root_metadata);
    let mut current = root;
    for component in [INSTALL_DIRECTORY, PRODUCT_DIRECTORY, app_id] {
        current = open_existing_child_directory(&current, component, root_device)?;
    }
    let product_root = local_app_data
        .join(INSTALL_DIRECTORY)
        .join(PRODUCT_DIRECTORY)
        .join(app_id);
    Ok((product_root, current))
}

/// 既存の固定rootと`versions`だけをno-followで開く。activation時にdirectoryを新規作成しない。
pub(crate) fn open_existing_product_versions_directory(
    local_app_data: &Path,
    app_id: &str,
) -> Result<(PathBuf, Dir), ProductInstallPlanError> {
    let (root_path, root) = open_existing_product_root(local_app_data, app_id)?;
    let metadata = root
        .dir_metadata()
        .map_err(|_| ProductInstallPlanError("product_install_directory_invalid"))?;
    let device = CapMetadataExt::dev(&metadata);
    let versions = open_existing_child_directory(&root, VERSIONS_DIRECTORY, device)?;
    Ok((root_path.join(VERSIONS_DIRECTORY), versions))
}

/// 固定導入root pathを導出する。directoryを作成せず、任意pathを受け取らない。
pub(crate) fn product_install_root(
    local_app_data: &Path,
    app_id: &str,
) -> Result<PathBuf, ProductInstallPlanError> {
    validate_install_inputs(local_app_data, app_id)?;
    Ok(local_app_data
        .join(INSTALL_DIRECTORY)
        .join(PRODUCT_DIRECTORY)
        .join(app_id))
}

fn validate_install_inputs(
    local_app_data: &Path,
    app_id: &str,
) -> Result<(), ProductInstallPlanError> {
    if !local_app_data.is_absolute()
        || local_app_data
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(ProductInstallPlanError(
            "product_install_local_app_data_invalid",
        ));
    }
    if !valid_identity(app_id) {
        return Err(ProductInstallPlanError(
            "product_install_app_identity_invalid",
        ));
    }
    Ok(())
}

/// 製品経路ではOS Known Folder API以外からLocalAppDataを受け取らない。
#[cfg(windows)]
pub(crate) fn open_current_user_product_versions_directory(
    app_id: &str,
) -> Result<(PathBuf, Dir), ProductInstallPlanError> {
    let local_app_data = current_user_local_app_data()?;
    open_product_versions_directory(Path::new(&local_app_data), app_id)
}

#[cfg(windows)]
pub(crate) fn current_user_local_app_data() -> Result<PathBuf, ProductInstallPlanError> {
    winsafe::SHGetKnownFolderPath(
        &winsafe::co::KNOWNFOLDERID::LocalAppData,
        winsafe::co::KF::DEFAULT,
        None,
    )
    .map(PathBuf::from)
    .map_err(|_| ProductInstallPlanError("product_install_known_folder_unavailable"))
}

/// Windows Known Folder APIから現在利用者のStart Menu rootを取得する。
#[cfg(windows)]
pub(crate) fn current_user_start_menu_directory() -> Result<PathBuf, ProductInstallPlanError> {
    winsafe::SHGetKnownFolderPath(
        &winsafe::co::KNOWNFOLDERID::StartMenu,
        winsafe::co::KF::DEFAULT,
        None,
    )
    .map(PathBuf::from)
    .map_err(|_| ProductInstallPlanError("product_start_menu_known_folder_unavailable"))
}

/// 現在利用者向けStart Menu shortcutの固定配置pathを計画する。
#[cfg(windows)]
pub(crate) fn plan_current_user_start_menu_shortcut(
    app_id: &str,
) -> Result<PathBuf, ProductInstallPlanError> {
    let start_menu = current_user_start_menu_directory()?;
    plan_start_menu_shortcut(&start_menu, app_id)
}

#[cfg(windows)]
pub(crate) fn plan_start_menu_shortcut(
    start_menu_directory: &Path,
    app_id: &str,
) -> Result<PathBuf, ProductInstallPlanError> {
    if !start_menu_directory.is_absolute()
        || start_menu_directory
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(ProductInstallPlanError(
            "product_start_menu_directory_invalid",
        ));
    }
    let start_menu_metadata = std::fs::symlink_metadata(start_menu_directory)
        .map_err(|_| ProductInstallPlanError("product_start_menu_directory_unavailable"))?;
    if !start_menu_metadata.is_dir()
        || start_menu_metadata.file_type().is_symlink()
        || is_reparse_point(&start_menu_metadata)
    {
        return Err(ProductInstallPlanError(
            "product_start_menu_directory_invalid",
        ));
    }
    let start_menu_directory = std::fs::canonicalize(start_menu_directory)
        .map_err(|_| ProductInstallPlanError("product_start_menu_directory_unavailable"))?;
    if !valid_identity(app_id) {
        return Err(ProductInstallPlanError(
            "product_install_app_identity_invalid",
        ));
    }
    Ok(start_menu_directory
        .join(START_MENU_PROGRAMS_DIRECTORY)
        .join(START_MENU_PRODUCT_DIRECTORY)
        .join(START_MENU_SHORTCUT_FILE))
}

/// 現在利用者の固定製品rootを指すStart Menu shortcutをcreate-onlyで登録する。
/// 既存entryが同じrootを指す場合だけ再利用し、別targetは上書きしない。
#[cfg(windows)]
pub(crate) fn register_start_menu_shortcut(
    local_app_data: &Path,
    start_menu_directory: &Path,
    app_id: &str,
    product_root: &Path,
) -> Result<PathBuf, ProductInstallPlanError> {
    let expected_root = product_install_root(local_app_data, app_id)?;
    let expected_root = std::fs::canonicalize(expected_root)
        .map_err(|_| ProductInstallPlanError("product_install_root_unavailable"))?;
    let product_root_metadata = std::fs::symlink_metadata(product_root)
        .map_err(|_| ProductInstallPlanError("product_install_root_unavailable"))?;
    if !product_root_metadata.is_dir()
        || product_root_metadata.file_type().is_symlink()
        || is_reparse_point(&product_root_metadata)
    {
        return Err(ProductInstallPlanError("product_install_root_invalid"));
    }
    let product_root = std::fs::canonicalize(product_root)
        .map_err(|_| ProductInstallPlanError("product_install_root_unavailable"))?;
    if !windows_path_equal(&expected_root, &product_root) {
        return Err(ProductInstallPlanError("product_install_root_mismatch"));
    }

    let launcher_path = product_root.join("gui_shell_desktop_launcher.exe");
    let launcher_metadata = std::fs::symlink_metadata(&launcher_path)
        .map_err(|_| ProductInstallPlanError("installed_bootstrapper_unavailable"))?;
    if !launcher_metadata.is_file()
        || launcher_metadata.file_type().is_symlink()
        || is_reparse_point(&launcher_metadata)
        || launcher_metadata.len() == 0
    {
        return Err(ProductInstallPlanError("installed_bootstrapper_invalid"));
    }
    let launcher_path = std::fs::canonicalize(&launcher_path)
        .map_err(|_| ProductInstallPlanError("installed_bootstrapper_invalid"))?;
    if !launcher_path.starts_with(&product_root) {
        return Err(ProductInstallPlanError("installed_bootstrapper_invalid"));
    }

    if !start_menu_directory.is_absolute()
        || start_menu_directory
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(ProductInstallPlanError(
            "product_start_menu_directory_invalid",
        ));
    }
    let supplied_start_menu_metadata = std::fs::symlink_metadata(start_menu_directory)
        .map_err(|_| ProductInstallPlanError("product_start_menu_directory_unavailable"))?;
    if !supplied_start_menu_metadata.is_dir()
        || supplied_start_menu_metadata.file_type().is_symlink()
        || is_reparse_point(&supplied_start_menu_metadata)
    {
        return Err(ProductInstallPlanError(
            "product_start_menu_directory_invalid",
        ));
    }
    let start_menu_path = std::fs::canonicalize(start_menu_directory)
        .map_err(|_| ProductInstallPlanError("product_start_menu_directory_unavailable"))?;
    let start_menu_metadata = std::fs::symlink_metadata(&start_menu_path)
        .map_err(|_| ProductInstallPlanError("product_start_menu_directory_unavailable"))?;
    if !start_menu_metadata.is_dir()
        || start_menu_metadata.file_type().is_symlink()
        || is_reparse_point(&start_menu_metadata)
    {
        return Err(ProductInstallPlanError(
            "product_start_menu_directory_invalid",
        ));
    }
    let start_menu = Dir::open_ambient_dir(&start_menu_path, cap_std::ambient_authority())
        .map_err(|_| ProductInstallPlanError("product_start_menu_directory_unavailable"))?;
    let start_menu_metadata = start_menu
        .dir_metadata()
        .map_err(|_| ProductInstallPlanError("product_start_menu_directory_invalid"))?;
    if !start_menu_metadata.is_dir() || is_cap_reparse_point(&start_menu_metadata) {
        return Err(ProductInstallPlanError(
            "product_start_menu_directory_invalid",
        ));
    }
    let start_menu_device = CapMetadataExt::dev(&start_menu_metadata);
    let programs = open_or_create_child_directory(
        &start_menu,
        START_MENU_PROGRAMS_DIRECTORY,
        start_menu_device,
    )?;
    let product =
        open_or_create_child_directory(&programs, START_MENU_PRODUCT_DIRECTORY, start_menu_device)?;
    let shortcut_path = start_menu_path
        .join(START_MENU_PROGRAMS_DIRECTORY)
        .join(START_MENU_PRODUCT_DIRECTORY)
        .join(START_MENU_SHORTCUT_FILE);

    match product.symlink_metadata(START_MENU_SHORTCUT_FILE) {
        Ok(_) => {
            verify_existing_start_menu_shortcut(&product, &shortcut_path, &launcher_path)?;
            return Ok(shortcut_path);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(ProductInstallPlanError(
                "product_start_menu_shortcut_unavailable",
            ))
        }
    }

    let mut suffix = [0u8; 16];
    getrandom::getrandom(&mut suffix)
        .map_err(|_| ProductInstallPlanError("product_start_menu_nonce_failed"))?;
    let temporary_name = format!("D4Pocket.{}.lnk.tmp", hex::encode(suffix));
    let temporary_path = start_menu_path
        .join(START_MENU_PROGRAMS_DIRECTORY)
        .join(START_MENU_PRODUCT_DIRECTORY)
        .join(&temporary_name);
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    let reservation = product
        .open_with(&temporary_name, &options)
        .map_err(|_| ProductInstallPlanError("product_start_menu_temporary_create_failed"))?;
    drop(reservation);

    if let Err(error) = save_shell_link(&temporary_path, &launcher_path) {
        let _ = product.remove_file(&temporary_name);
        return Err(error);
    }
    let temporary_metadata = product
        .symlink_metadata(&temporary_name)
        .map_err(|_| ProductInstallPlanError("product_start_menu_temporary_invalid"))?;
    if !temporary_metadata.is_file()
        || is_cap_reparse_point(&temporary_metadata)
        || temporary_metadata.len() == 0
        || temporary_metadata.len() > MAX_START_MENU_SHORTCUT_BYTES
    {
        let _ = product.remove_file(&temporary_name);
        return Err(ProductInstallPlanError(
            "product_start_menu_temporary_invalid",
        ));
    }

    match product.hard_link(&temporary_name, &product, START_MENU_SHORTCUT_FILE) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let _ = product.remove_file(&temporary_name);
            verify_existing_start_menu_shortcut(&product, &shortcut_path, &launcher_path)?;
            return Ok(shortcut_path);
        }
        Err(_) => {
            let _ = product.remove_file(&temporary_name);
            return Err(ProductInstallPlanError(
                "product_start_menu_shortcut_publish_failed",
            ));
        }
    }
    product
        .remove_file(&temporary_name)
        .map_err(|_| ProductInstallPlanError("product_start_menu_temporary_cleanup_failed"))?;
    verify_existing_start_menu_shortcut(&product, &shortcut_path, &launcher_path)?;
    Ok(shortcut_path)
}

#[cfg(windows)]
fn verify_existing_start_menu_shortcut(
    product_directory: &Dir,
    shortcut_path: &Path,
    expected_launcher: &Path,
) -> Result<(), ProductInstallPlanError> {
    let metadata = product_directory
        .symlink_metadata(START_MENU_SHORTCUT_FILE)
        .map_err(|_| ProductInstallPlanError("product_start_menu_shortcut_unavailable"))?;
    if !metadata.is_file()
        || is_cap_reparse_point(&metadata)
        || metadata.len() == 0
        || metadata.len() > MAX_START_MENU_SHORTCUT_BYTES
    {
        return Err(ProductInstallPlanError(
            "product_start_menu_shortcut_conflict",
        ));
    }
    let existing_target = load_shell_link_target(shortcut_path)
        .map_err(|_| ProductInstallPlanError("product_start_menu_shortcut_conflict"))?;
    let existing_target = std::fs::canonicalize(existing_target)
        .map_err(|_| ProductInstallPlanError("product_start_menu_shortcut_conflict"))?;
    if !windows_path_equal(&existing_target, expected_launcher) {
        return Err(ProductInstallPlanError(
            "product_start_menu_shortcut_conflict",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn save_shell_link(
    shortcut_path: &Path,
    target_path: &Path,
) -> Result<(), ProductInstallPlanError> {
    let shortcut_path = shell_link_compatible_path(shortcut_path)?;
    let target_path = shell_link_compatible_path(target_path)?;
    run_shell_link_com(move || {
        use winsafe::prelude::*;

        let shell_link = winsafe::CoCreateInstance::<winsafe::IShellLink>(
            &winsafe::co::CLSID::ShellLink,
            None::<&winsafe::IUnknown>,
            winsafe::co::CLSCTX::INPROC_SERVER,
        )
        .map_err(|_| ProductInstallPlanError("product_start_menu_shell_link_create_failed"))?;
        shell_link
            .SetPath(&target_path)
            .map_err(|_| ProductInstallPlanError("product_start_menu_shell_link_target_failed"))?;
        shell_link.SetDescription("D4 Pocket").map_err(|_| {
            ProductInstallPlanError("product_start_menu_shell_link_description_failed")
        })?;
        let persist_file = shell_link
            .QueryInterface::<winsafe::IPersistFile>()
            .map_err(|_| ProductInstallPlanError("product_start_menu_persist_interface_failed"))?;
        persist_file
            .Save(Some(&shortcut_path), false)
            .map_err(|_| ProductInstallPlanError("product_start_menu_shell_link_save_failed"))
    })
}

#[cfg(windows)]
fn load_shell_link_target(shortcut_path: &Path) -> Result<PathBuf, ProductInstallPlanError> {
    let shortcut_path = shell_link_compatible_path(shortcut_path)?;
    run_shell_link_com(move || {
        use winsafe::prelude::*;

        let shell_link = winsafe::CoCreateInstance::<winsafe::IShellLink>(
            &winsafe::co::CLSID::ShellLink,
            None::<&winsafe::IUnknown>,
            winsafe::co::CLSCTX::INPROC_SERVER,
        )
        .map_err(|_| ProductInstallPlanError("product_start_menu_shell_link_load_failed"))?;
        let persist_file = shell_link
            .QueryInterface::<winsafe::IPersistFile>()
            .map_err(|_| ProductInstallPlanError("product_start_menu_shell_link_load_failed"))?;
        persist_file
            .Load(&shortcut_path, winsafe::co::STGM::READ)
            .map_err(|_| ProductInstallPlanError("product_start_menu_shell_link_load_failed"))?;
        shell_link
            .GetPath(None, winsafe::co::SLGP::RAWPATH)
            .map(PathBuf::from)
            .map_err(|_| ProductInstallPlanError("product_start_menu_shell_link_load_failed"))
    })
}

#[cfg(windows)]
fn shell_link_compatible_path(path: &Path) -> Result<String, ProductInstallPlanError> {
    let value = path.to_str().ok_or(ProductInstallPlanError(
        "product_start_menu_path_not_unicode",
    ))?;
    let normalized = if let Some(extended) = value.strip_prefix("\\\\?\\") {
        if let Some(unc) = extended.strip_prefix("UNC\\") {
            format!("\\\\{unc}")
        } else if extended.as_bytes().get(1) == Some(&b':')
            && extended.as_bytes().get(2) == Some(&b'\\')
        {
            extended.to_owned()
        } else {
            return Err(ProductInstallPlanError(
                "product_start_menu_path_not_shell_compatible",
            ));
        }
    } else {
        value.to_owned()
    };
    Ok(normalized)
}

#[cfg(windows)]
fn run_shell_link_com<T, F>(operation: F) -> Result<T, ProductInstallPlanError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, ProductInstallPlanError> + Send + 'static,
{
    std::thread::Builder::new()
        .name("D4 Pocket Start Menu COM".to_owned())
        .spawn(move || {
            let _com = winsafe::CoInitializeEx(
                winsafe::co::COINIT::APARTMENTTHREADED | winsafe::co::COINIT::DISABLE_OLE1DDE,
            )
            .map_err(|_| ProductInstallPlanError("product_start_menu_com_initialize_failed"))?;
            operation()
        })
        .map_err(|_| ProductInstallPlanError("product_start_menu_com_thread_failed"))?
        .join()
        .map_err(|_| ProductInstallPlanError("product_start_menu_com_thread_failed"))?
}

#[cfg(windows)]
fn windows_path_equal(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .replace('/', "\\")
        .eq_ignore_ascii_case(&right.to_string_lossy().replace('/', "\\"))
}

fn open_existing_child_directory(
    parent: &Dir,
    name: &str,
    root_device: u64,
) -> Result<Dir, ProductInstallPlanError> {
    let before_open = parent
        .symlink_metadata(name)
        .map_err(|_| ProductInstallPlanError("product_install_directory_unavailable"))?;
    if !before_open.is_dir() || is_cap_reparse_point(&before_open) {
        return Err(ProductInstallPlanError("product_install_directory_invalid"));
    }
    let opened = parent
        .open_dir_nofollow(name)
        .map_err(|_| ProductInstallPlanError("product_install_directory_invalid"))?;
    let after_open = opened
        .dir_metadata()
        .map_err(|_| ProductInstallPlanError("product_install_directory_invalid"))?;
    if !after_open.is_dir()
        || is_cap_reparse_point(&after_open)
        || !same_cap_file(&before_open, &after_open)
        || CapMetadataExt::dev(&after_open) != root_device
    {
        return Err(ProductInstallPlanError("product_install_directory_invalid"));
    }
    Ok(opened)
}

fn open_or_create_child_directory(
    parent: &Dir,
    name: &str,
    root_device: u64,
) -> Result<Dir, ProductInstallPlanError> {
    match parent.symlink_metadata(name) {
        Ok(metadata) if metadata.is_dir() && !is_cap_reparse_point(&metadata) => {}
        Ok(_) => return Err(ProductInstallPlanError("product_install_directory_invalid")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match parent.create_dir(name) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => {
                    return Err(ProductInstallPlanError(
                        "product_install_directory_unavailable",
                    ))
                }
            }
        }
        Err(_) => {
            return Err(ProductInstallPlanError(
                "product_install_directory_unavailable",
            ))
        }
    }
    let before_open = parent
        .symlink_metadata(name)
        .map_err(|_| ProductInstallPlanError("product_install_directory_invalid"))?;
    if !before_open.is_dir() || is_cap_reparse_point(&before_open) {
        return Err(ProductInstallPlanError("product_install_directory_invalid"));
    }
    let opened = parent
        .open_dir_nofollow(name)
        .map_err(|_| ProductInstallPlanError("product_install_directory_invalid"))?;
    let after_open = opened
        .dir_metadata()
        .map_err(|_| ProductInstallPlanError("product_install_directory_invalid"))?;
    if !after_open.is_dir()
        || is_cap_reparse_point(&after_open)
        || !same_cap_file(&before_open, &after_open)
        || CapMetadataExt::dev(&after_open) != root_device
    {
        return Err(ProductInstallPlanError("product_install_directory_invalid"));
    }
    Ok(opened)
}

fn is_cap_reparse_point(metadata: &cap_std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn same_cap_file(left: &cap_std::fs::Metadata, right: &cap_std::fs::Metadata) -> bool {
    let left_id = CapMetadataExt::ino(left);
    left_id != 0
        && CapMetadataExt::dev(left) == CapMetadataExt::dev(right)
        && left_id == CapMetadataExt::ino(right)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_: &std::fs::Metadata) -> bool {
    false
}

/// Windows Known Folder APIが返す現在利用者LocalAppDataから導入先を計画する。
#[cfg(windows)]
pub(crate) fn plan_current_user_product_install(
    app_id: &str,
    product_version: &str,
    package_sha256: &str,
) -> Result<ProductInstallPlan, ProductInstallPlanError> {
    let local_app_data = current_user_local_app_data()?;
    plan_product_install(&local_app_data, app_id, product_version, package_sha256)
}

/// OSが返した現在利用者のLocalAppDataと製品Manifest identityから、導入先を固定導出する。
/// UI、manifest、更新候補から任意の保存pathを受け取らない。
pub(crate) fn plan_product_install(
    local_app_data: &Path,
    app_id: &str,
    product_version: &str,
    package_sha256: &str,
) -> Result<ProductInstallPlan, ProductInstallPlanError> {
    let root = product_install_root(local_app_data, app_id)?;
    if !valid_product_version(product_version) {
        return Err(ProductInstallPlanError("product_install_version_invalid"));
    }
    if !valid_sha256(package_sha256) {
        return Err(ProductInstallPlanError(
            "product_install_package_hash_invalid",
        ));
    }

    let version_directory = root
        .join(VERSIONS_DIRECTORY)
        .join(format!("{product_version}-{package_sha256}"));
    if !version_directory.starts_with(&root) {
        return Err(ProductInstallPlanError("product_install_path_outside_root"));
    }
    Ok(ProductInstallPlan {
        root,
        version_directory,
    })
}

fn valid_identity(value: &str) -> bool {
    value.strip_prefix("d4-pocket-app-").is_some_and(|suffix| {
        suffix.len() == 32
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_product_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+' | b'_'))
        && value.bytes().any(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{
        open_existing_product_root, open_existing_product_versions_directory,
        open_product_versions_directory, plan_product_install,
    };
    use std::path::Path;

    const APP_ID: &str = "d4-pocket-app-11111111111111111111111111111111";
    const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn install_path_is_per_user_identity_and_content_bound() {
        let plan = plan_product_install(
            Path::new("C:/Users/test/AppData/Local"),
            APP_ID,
            "1.2.3",
            HASH,
        )
        .unwrap();
        assert_eq!(
            plan.root,
            Path::new("C:/Users/test/AppData/Local/Programs/D4 Pocket").join(APP_ID)
        );
        assert_eq!(
            plan.version_directory,
            plan.root.join("versions").join(format!("1.2.3-{HASH}"))
        );
        assert!(plan.version_directory.starts_with(&plan.root));
    }

    #[test]
    fn versions_directory_is_created_and_returned_as_a_fixed_capability() {
        let local_app_data = std::env::temp_dir().join(format!(
            "d4p-product-install-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&local_app_data).unwrap();
        let canonical_local_app_data = std::fs::canonicalize(&local_app_data).unwrap();
        let (path, capability) = open_product_versions_directory(&local_app_data, APP_ID).unwrap();
        assert_eq!(
            path,
            canonical_local_app_data
                .join("Programs")
                .join("D4 Pocket")
                .join(APP_ID)
                .join("versions")
        );
        assert!(capability.dir_metadata().unwrap().is_dir());
        assert!(local_app_data
            .join("Programs")
            .join("D4 Pocket")
            .join(APP_ID)
            .join("versions")
            .is_dir());
        drop(capability);
        std::fs::remove_dir_all(local_app_data).unwrap();
    }

    #[test]
    fn existing_product_root_opens_without_creating_missing_installation() {
        let local_app_data = std::env::temp_dir().join(format!(
            "d4p-product-root-readonly-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&local_app_data).unwrap();
        assert_eq!(
            open_existing_product_root(&local_app_data, APP_ID)
                .unwrap_err()
                .0,
            "product_install_directory_unavailable"
        );
        assert!(!local_app_data.join("Programs").exists());
        let product_root = local_app_data
            .join("Programs")
            .join("D4 Pocket")
            .join(APP_ID);
        std::fs::create_dir_all(&product_root).unwrap();
        let (opened_path, capability) =
            open_existing_product_root(&local_app_data, APP_ID).unwrap();
        assert_eq!(opened_path, std::fs::canonicalize(product_root).unwrap());
        assert!(capability.dir_metadata().unwrap().is_dir());
        drop(capability);
        std::fs::remove_dir_all(local_app_data).unwrap();
    }

    #[test]
    fn existing_versions_capability_never_creates_installation_directories() {
        let local_app_data = std::env::temp_dir().join(format!(
            "d4p-product-versions-readonly-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&local_app_data).unwrap();
        assert_eq!(
            open_existing_product_versions_directory(&local_app_data, APP_ID)
                .unwrap_err()
                .0,
            "product_install_directory_unavailable"
        );
        assert!(!local_app_data.join("Programs").exists());

        let product_root = local_app_data
            .join("Programs")
            .join("D4 Pocket")
            .join(APP_ID);
        std::fs::create_dir_all(product_root.join("versions")).unwrap();
        let (versions_path, capability) =
            open_existing_product_versions_directory(&local_app_data, APP_ID).unwrap();
        assert_eq!(
            versions_path,
            std::fs::canonicalize(&product_root)
                .unwrap()
                .join("versions")
        );
        assert!(capability.dir_metadata().unwrap().is_dir());
        drop(capability);
        std::fs::remove_dir_all(local_app_data).unwrap();
    }

    #[test]
    fn invalid_identity_path_version_and_digest_are_rejected() {
        assert_eq!(
            plan_product_install(Path::new("relative"), APP_ID, "1.2.3", HASH)
                .unwrap_err()
                .0,
            "product_install_local_app_data_invalid"
        );
        assert_eq!(
            plan_product_install(Path::new("C:/Users/test/../other"), APP_ID, "1.2.3", HASH)
                .unwrap_err()
                .0,
            "product_install_local_app_data_invalid"
        );
        assert_eq!(
            plan_product_install(
                Path::new("C:/Users/test/AppData/Local"),
                "../../escape",
                "1.2.3",
                HASH
            )
            .unwrap_err()
            .0,
            "product_install_app_identity_invalid"
        );
        assert_eq!(
            plan_product_install(
                Path::new("C:/Users/test/AppData/Local"),
                APP_ID,
                "../1.2.3",
                HASH
            )
            .unwrap_err()
            .0,
            "product_install_version_invalid"
        );
        assert_eq!(
            plan_product_install(
                Path::new("C:/Users/test/AppData/Local"),
                APP_ID,
                "1.2.3",
                "A".repeat(64).as_str()
            )
            .unwrap_err()
            .0,
            "product_install_package_hash_invalid"
        );
    }

    #[cfg(windows)]
    #[test]
    fn start_menu_shortcut_targets_fixed_bootstrapper_and_is_idempotent() {
        let scratch = std::env::temp_dir().join(format!(
            "d4p-start-menu-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let local_app_data = scratch.join("Local");
        let start_menu = scratch.join("StartMenu");
        let product_root = local_app_data
            .join("Programs")
            .join("D4 Pocket")
            .join(APP_ID);
        std::fs::create_dir_all(&product_root).unwrap();
        std::fs::create_dir_all(&start_menu).unwrap();
        let launcher = product_root.join("gui_shell_desktop_launcher.exe");
        std::fs::write(&launcher, b"fixture bootstrapper").unwrap();

        let registered = super::register_start_menu_shortcut(
            &local_app_data,
            &start_menu,
            APP_ID,
            &product_root,
        )
        .unwrap();
        assert_eq!(
            registered,
            super::plan_start_menu_shortcut(&start_menu, APP_ID).unwrap()
        );
        assert_eq!(
            std::fs::canonicalize(super::load_shell_link_target(&registered).unwrap()).unwrap(),
            std::fs::canonicalize(&launcher).unwrap()
        );
        let first_bytes = std::fs::read(&registered).unwrap();

        assert_eq!(
            super::register_start_menu_shortcut(
                &local_app_data,
                &start_menu,
                APP_ID,
                &product_root,
            )
            .unwrap(),
            registered
        );
        assert_eq!(std::fs::read(&registered).unwrap(), first_bytes);
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn start_menu_conflict_is_rejected_without_overwriting_existing_shortcut() {
        let scratch = std::env::temp_dir().join(format!(
            "d4p-start-menu-conflict-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let local_app_data = scratch.join("Local");
        let start_menu = scratch.join("StartMenu");
        let product_root = local_app_data
            .join("Programs")
            .join("D4 Pocket")
            .join(APP_ID);
        let programs = start_menu.join("Programs").join("D4 Pocket");
        std::fs::create_dir_all(&product_root).unwrap();
        std::fs::create_dir_all(&programs).unwrap();
        let launcher = product_root.join("gui_shell_desktop_launcher.exe");
        let other_target = scratch.join("unrelated.exe");
        std::fs::write(&launcher, b"fixture bootstrapper").unwrap();
        std::fs::write(&other_target, b"unrelated target").unwrap();
        let shortcut = programs.join("D4 Pocket.lnk");
        super::save_shell_link(&shortcut, &other_target).unwrap();
        let original_bytes = std::fs::read(&shortcut).unwrap();

        assert_eq!(
            super::register_start_menu_shortcut(
                &local_app_data,
                &start_menu,
                APP_ID,
                &product_root,
            )
            .unwrap_err()
            .0,
            "product_start_menu_shortcut_conflict"
        );
        assert_eq!(std::fs::read(&shortcut).unwrap(), original_bytes);
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn current_user_known_folder_produces_a_fixed_install_plan() {
        let plan = super::plan_current_user_product_install(APP_ID, "1.2.3", HASH).unwrap();
        assert!(plan.root.is_absolute());
        assert_eq!(plan.root.file_name().unwrap(), APP_ID);
        assert_eq!(
            plan.root.parent().unwrap().file_name().unwrap(),
            super::PRODUCT_DIRECTORY
        );
        assert!(plan.version_directory.starts_with(&plan.root));
    }
}
