//! Desktop製品の導入先をBroker所有の固定規則から導出する。
//!
//! 固定path計画とBroker専用のversions directory capability取得を行う。
//! WindowsではStart Menu shortcut登録も扱う。
//! package展開、Permission／Approval／Auditは行わず、実作用はBroker consumerが統治する。

#[cfg(windows)]
use cap_fs_ext::OsMetadataExt as _;
use cap_fs_ext::{DirExt, MetadataExt as CapMetadataExt};
use cap_fs_ext::{FollowSymlinks, OpenOptionsExt as _, OpenOptionsFollowExt as _};
use cap_std::fs::Dir;
#[cfg(any(windows, target_os = "linux"))]
use cap_std::fs::OpenOptions;
#[cfg(target_os = "linux")]
use std::io::{Read, Write};
#[cfg(target_os = "linux")]
use std::os::unix::fs::{MetadataExt as StdMetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

#[cfg(not(target_os = "linux"))]
const INSTALL_DIRECTORY: &str = "Programs";
#[cfg(target_os = "linux")]
const INSTALL_DIRECTORY: &str = "apps";
#[cfg(not(target_os = "linux"))]
const PRODUCT_DIRECTORY: &str = "D4 Pocket";
#[cfg(target_os = "linux")]
const PRODUCT_DIRECTORY: &str = "d4-pocket";
const VERSIONS_DIRECTORY: &str = "versions";
#[cfg(target_os = "linux")]
const LINUX_DESKTOP_ENTRY_PREFIX: &str = "com.d4pocket.";
#[cfg(target_os = "linux")]
const MAX_LINUX_DESKTOP_ENTRY_BYTES: u64 = 16 * 1024;
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

/// 現在利用者の固定install `versions` directoryをcapabilityで開く。
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
    #[cfg(target_os = "linux")]
    validate_linux_data_home_metadata(&metadata)?;
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
    #[cfg(target_os = "linux")]
    if StdMetadataExt::dev(&metadata) != CapMetadataExt::dev(&root_metadata)
        || StdMetadataExt::ino(&metadata) != CapMetadataExt::ino(&root_metadata)
    {
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

/// 製品経路ではOSの利用者領域規則以外から導入rootを受け取らない。
#[cfg(windows)]
pub(crate) fn open_current_user_product_versions_directory(
    app_id: &str,
) -> Result<(PathBuf, Dir), ProductInstallPlanError> {
    let local_app_data = current_user_local_app_data()?;
    open_product_versions_directory(Path::new(&local_app_data), app_id)
}

/// LinuxではXDG_DATA_HOME、未指定ならHOME/.local/shareを導入rootに使う。
/// UIや更新manifestから任意pathを受け取らず、ディレクトリ作成はOwner確認後に行う。
#[cfg(target_os = "linux")]
pub(crate) fn current_user_local_app_data() -> Result<PathBuf, ProductInstallPlanError> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or(ProductInstallPlanError("product_install_home_unavailable"))?;
    let home_metadata = std::fs::symlink_metadata(&home)
        .map_err(|_| ProductInstallPlanError("product_install_home_unavailable"))?;
    if !home_metadata.is_dir() || home_metadata.file_type().is_symlink() {
        return Err(ProductInstallPlanError("product_install_home_invalid"));
    }
    validate_linux_data_home_metadata(&home_metadata)
        .map_err(|_| ProductInstallPlanError("product_install_home_invalid"))?;
    let home = std::fs::canonicalize(home)
        .map_err(|_| ProductInstallPlanError("product_install_home_unavailable"))?;
    let xdg_data_home = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    resolve_linux_data_home(&home, xdg_data_home.as_deref())
}

#[cfg(target_os = "linux")]
pub(crate) fn open_current_user_product_versions_directory(
    app_id: &str,
) -> Result<(PathBuf, Dir), ProductInstallPlanError> {
    let local_app_data = current_user_local_app_data()?;
    ensure_linux_data_home(&local_app_data)?;
    open_product_versions_directory(&local_app_data, app_id)
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
            ));
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

/// 固定導入rootと、そこを指すことを再確認したStart Menu shortcutだけを除去する。
/// AppDataのruntime／Audit Storeはこの関数の対象に含めない。
#[cfg(windows)]
pub(crate) fn remove_installed_product(
    local_app_data: &Path,
    start_menu_directory: &Path,
    app_id: &str,
) -> Result<(), ProductInstallPlanError> {
    let (product_root, _) = open_existing_product_root(local_app_data, app_id)?;
    let expected_launcher = product_root.join("gui_shell_desktop_launcher.exe");

    let start_menu_shortcut = plan_start_menu_shortcut(start_menu_directory, app_id)?;
    let start_menu_root = std::fs::canonicalize(start_menu_directory)
        .map_err(|_| ProductInstallPlanError("product_start_menu_directory_unavailable"))?;
    let start_menu = Dir::open_ambient_dir(&start_menu_root, cap_std::ambient_authority())
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
    if let Some(programs) = open_existing_optional_child_directory(
        &start_menu,
        START_MENU_PROGRAMS_DIRECTORY,
        start_menu_device,
    )? {
        if let Some(product_shortcuts) = open_existing_optional_child_directory(
            &programs,
            START_MENU_PRODUCT_DIRECTORY,
            start_menu_device,
        )? {
            match product_shortcuts.symlink_metadata(START_MENU_SHORTCUT_FILE) {
                Ok(metadata) => {
                    if !metadata.is_file()
                        || is_cap_reparse_point(&metadata)
                        || metadata.len() == 0
                        || metadata.len() > MAX_START_MENU_SHORTCUT_BYTES
                    {
                        return Err(ProductInstallPlanError(
                            "product_start_menu_shortcut_conflict",
                        ));
                    }
                    verify_existing_start_menu_shortcut(
                        &product_shortcuts,
                        &start_menu_shortcut,
                        &expected_launcher,
                    )?;
                    product_shortcuts
                        .remove_file(START_MENU_SHORTCUT_FILE)
                        .map_err(|_| {
                            ProductInstallPlanError("product_start_menu_shortcut_remove_failed")
                        })?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => {
                    return Err(ProductInstallPlanError(
                        "product_start_menu_shortcut_unavailable",
                    ));
                }
            }
            let _ = programs.remove_dir(START_MENU_PRODUCT_DIRECTORY);
        }
    }

    let local_app_data = std::fs::canonicalize(local_app_data)
        .map_err(|_| ProductInstallPlanError("product_install_local_app_data_unavailable"))?;
    let local_root = Dir::open_ambient_dir(&local_app_data, cap_std::ambient_authority())
        .map_err(|_| ProductInstallPlanError("product_install_local_app_data_unavailable"))?;
    let local_metadata = local_root
        .dir_metadata()
        .map_err(|_| ProductInstallPlanError("product_install_local_app_data_invalid"))?;
    if !local_metadata.is_dir() || is_cap_reparse_point(&local_metadata) {
        return Err(ProductInstallPlanError(
            "product_install_local_app_data_invalid",
        ));
    }
    let local_device = CapMetadataExt::dev(&local_metadata);
    let programs = open_existing_child_directory(&local_root, INSTALL_DIRECTORY, local_device)?;
    let products = open_existing_child_directory(&programs, PRODUCT_DIRECTORY, local_device)?;
    products
        .remove_dir_all(app_id)
        .map_err(|_| ProductInstallPlanError("product_install_remove_failed"))
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

fn open_existing_optional_child_directory(
    parent: &Dir,
    name: &str,
    root_device: u64,
) -> Result<Option<Dir>, ProductInstallPlanError> {
    match parent.symlink_metadata(name) {
        Ok(_) => open_existing_child_directory(parent, name, root_device).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(ProductInstallPlanError(
            "product_install_directory_unavailable",
        )),
    }
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
                    ));
                }
            }
        }
        Err(_) => {
            return Err(ProductInstallPlanError(
                "product_install_directory_unavailable",
            ));
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

pub(super) fn is_cap_reparse_point(metadata: &cap_std::fs::Metadata) -> bool {
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

#[cfg(target_os = "linux")]
fn resolve_linux_data_home(
    home: &Path,
    configured: Option<&Path>,
) -> Result<PathBuf, ProductInstallPlanError> {
    if !home.is_absolute()
        || home
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(ProductInstallPlanError("product_install_home_invalid"));
    }
    let path = configured
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join(".local").join("share"));
    if !path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::Prefix(_)
            )
        })
    {
        return Err(ProductInstallPlanError("product_install_data_home_invalid"));
    }
    Ok(path)
}

#[cfg(target_os = "linux")]
fn validate_linux_data_home_metadata(
    metadata: &std::fs::Metadata,
) -> Result<(), ProductInstallPlanError> {
    if metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o022 != 0
    {
        return Err(ProductInstallPlanError("product_install_data_home_invalid"));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn ensure_linux_data_home(path: &Path) -> Result<(), ProductInstallPlanError> {
    if !path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::Prefix(_)
            )
        })
    {
        return Err(ProductInstallPlanError("product_install_data_home_invalid"));
    }
    let mut current = Dir::open_ambient_dir("/", cap_std::ambient_authority())
        .map_err(|_| ProductInstallPlanError("product_install_data_home_unavailable"))?;
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                let name = name
                    .to_str()
                    .ok_or(ProductInstallPlanError("product_install_data_home_invalid"))?;
                current = open_or_create_absolute_child_directory(&current, name)?;
            }
            _ => {
                return Err(ProductInstallPlanError("product_install_data_home_invalid"));
            }
        }
    }
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| ProductInstallPlanError("product_install_data_home_unavailable"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ProductInstallPlanError("product_install_data_home_invalid"));
    }
    validate_linux_data_home_metadata(&metadata)?;
    let canonical = std::fs::canonicalize(path)
        .map_err(|_| ProductInstallPlanError("product_install_data_home_unavailable"))?;
    if canonical != path {
        return Err(ProductInstallPlanError("product_install_data_home_invalid"));
    }
    let opened_metadata = current
        .dir_metadata()
        .map_err(|_| ProductInstallPlanError("product_install_data_home_unavailable"))?;
    if StdMetadataExt::dev(&metadata) != CapMetadataExt::dev(&opened_metadata)
        || StdMetadataExt::ino(&metadata) != CapMetadataExt::ino(&opened_metadata)
    {
        return Err(ProductInstallPlanError("product_install_data_home_invalid"));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn open_or_create_absolute_child_directory(
    parent: &Dir,
    name: &str,
) -> Result<Dir, ProductInstallPlanError> {
    match parent.symlink_metadata(name) {
        Ok(metadata) if metadata.is_dir() && !is_cap_reparse_point(&metadata) => {}
        Ok(_) => return Err(ProductInstallPlanError("product_install_data_home_invalid")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match parent.create_dir(name) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => {
                    return Err(ProductInstallPlanError(
                        "product_install_data_home_unavailable",
                    ));
                }
            }
        }
        Err(_) => {
            return Err(ProductInstallPlanError(
                "product_install_data_home_unavailable",
            ));
        }
    }
    let before_open = parent
        .symlink_metadata(name)
        .map_err(|_| ProductInstallPlanError("product_install_data_home_invalid"))?;
    if !before_open.is_dir() || is_cap_reparse_point(&before_open) {
        return Err(ProductInstallPlanError("product_install_data_home_invalid"));
    }
    let opened = parent
        .open_dir_nofollow(name)
        .map_err(|_| ProductInstallPlanError("product_install_data_home_invalid"))?;
    let after_open = opened
        .dir_metadata()
        .map_err(|_| ProductInstallPlanError("product_install_data_home_invalid"))?;
    if !after_open.is_dir()
        || is_cap_reparse_point(&after_open)
        || !same_cap_file(&before_open, &after_open)
    {
        return Err(ProductInstallPlanError("product_install_data_home_invalid"));
    }
    Ok(opened)
}

/// OSの現在利用者領域規則から固定導入先を計画する。
#[cfg(any(windows, target_os = "linux"))]
pub(crate) fn plan_current_user_product_install(
    app_id: &str,
    product_version: &str,
    package_sha256: &str,
) -> Result<ProductInstallPlan, ProductInstallPlanError> {
    let local_app_data = current_user_local_app_data()?;
    plan_product_install(&local_app_data, app_id, product_version, package_sha256)
}

/// 現在利用者のXDG applications directory内に置く固定desktop entryを計画する。
/// pathはnative Owner summaryへ表示するだけで、この関数はdirectoryやfileを作成しない。
#[cfg(target_os = "linux")]
pub(crate) fn plan_current_user_linux_desktop_entry(
    app_id: &str,
) -> Result<PathBuf, ProductInstallPlanError> {
    let data_home = current_user_local_app_data()?;
    plan_linux_desktop_entry(&data_home, app_id)
}

#[cfg(target_os = "linux")]
pub(crate) fn plan_linux_desktop_entry(
    data_home: &Path,
    app_id: &str,
) -> Result<PathBuf, ProductInstallPlanError> {
    validate_install_inputs(data_home, app_id)?;
    let metadata = std::fs::symlink_metadata(data_home)
        .map_err(|_| ProductInstallPlanError("product_install_data_home_unavailable"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ProductInstallPlanError("product_install_data_home_invalid"));
    }
    validate_linux_data_home_metadata(&metadata)?;
    let canonical = std::fs::canonicalize(data_home)
        .map_err(|_| ProductInstallPlanError("product_install_data_home_unavailable"))?;
    if canonical != data_home {
        return Err(ProductInstallPlanError("product_install_data_home_invalid"));
    }
    let name = linux_desktop_entry_name(app_id)?;
    Ok(canonical.join("applications").join(name))
}

/// 確認済みの固定product rootに対するper-user desktop entryをcreate-onlyで登録する。
/// 既存entryは期待launcherを指し、内容・owner・mode・link数が一致する場合だけ再利用する。
#[cfg(target_os = "linux")]
pub(crate) fn register_linux_desktop_entry(
    data_home: &Path,
    app_id: &str,
    audit_store_id: &str,
    product_root: &Path,
) -> Result<PathBuf, ProductInstallPlanError> {
    let expected_root = product_install_root(data_home, app_id)?;
    let expected_root = std::fs::canonicalize(expected_root)
        .map_err(|_| ProductInstallPlanError("product_install_root_unavailable"))?;
    let root_metadata = std::fs::symlink_metadata(product_root)
        .map_err(|_| ProductInstallPlanError("product_install_root_unavailable"))?;
    if !root_metadata.is_dir() || root_metadata.file_type().is_symlink() {
        return Err(ProductInstallPlanError("product_install_root_invalid"));
    }
    let product_root = std::fs::canonicalize(product_root)
        .map_err(|_| ProductInstallPlanError("product_install_root_unavailable"))?;
    if product_root != expected_root {
        return Err(ProductInstallPlanError("product_install_root_mismatch"));
    }

    let (opened_root_path, root) = open_existing_product_root(data_home, app_id)?;
    if opened_root_path != product_root {
        return Err(ProductInstallPlanError("product_install_root_mismatch"));
    }
    let root_metadata = root
        .dir_metadata()
        .map_err(|_| ProductInstallPlanError("product_install_root_invalid"))?;
    let root_device = CapMetadataExt::dev(&root_metadata);
    let launcher_name = "gui_shell_desktop_launcher";
    let launcher_metadata = root
        .symlink_metadata(launcher_name)
        .map_err(|_| ProductInstallPlanError("installed_bootstrapper_unavailable"))?;
    if !launcher_metadata.is_file()
        || is_cap_reparse_point(&launcher_metadata)
        || CapMetadataExt::dev(&launcher_metadata) != root_device
        || launcher_metadata.len() == 0
    {
        return Err(ProductInstallPlanError("installed_bootstrapper_invalid"));
    }
    let launcher_path = product_root.join(launcher_name);
    let launcher_os_metadata = std::fs::symlink_metadata(&launcher_path)
        .map_err(|_| ProductInstallPlanError("installed_bootstrapper_invalid"))?;
    crate::product_bootstrapper::resolve_active_version_launcher(data_home, app_id, audit_store_id)
        .map_err(|_| ProductInstallPlanError("installed_bootstrapper_invalid"))?;
    if !launcher_os_metadata.is_file()
        || launcher_os_metadata.file_type().is_symlink()
        || StdMetadataExt::uid(&launcher_os_metadata) != rustix::process::geteuid().as_raw()
        || !(1..=2).contains(&StdMetadataExt::nlink(&launcher_os_metadata))
        || StdMetadataExt::mode(&launcher_os_metadata) & 0o111 == 0
        || StdMetadataExt::mode(&launcher_os_metadata) & 0o022 != 0
        || StdMetadataExt::dev(&launcher_os_metadata) != CapMetadataExt::dev(&launcher_metadata)
        || StdMetadataExt::ino(&launcher_os_metadata) != CapMetadataExt::ino(&launcher_metadata)
    {
        return Err(ProductInstallPlanError("installed_bootstrapper_invalid"));
    }

    let entry_path = plan_linux_desktop_entry(data_home, app_id)?;
    let data_metadata = std::fs::symlink_metadata(data_home)
        .map_err(|_| ProductInstallPlanError("product_install_data_home_unavailable"))?;
    validate_linux_data_home_metadata(&data_metadata)?;
    let data_home_dir = Dir::open_ambient_dir(data_home, cap_std::ambient_authority())
        .map_err(|_| ProductInstallPlanError("product_install_data_home_unavailable"))?;
    let opened_data_metadata = data_home_dir
        .dir_metadata()
        .map_err(|_| ProductInstallPlanError("product_install_data_home_invalid"))?;
    if !opened_data_metadata.is_dir()
        || is_cap_reparse_point(&opened_data_metadata)
        || StdMetadataExt::dev(&data_metadata) != CapMetadataExt::dev(&opened_data_metadata)
        || StdMetadataExt::ino(&data_metadata) != CapMetadataExt::ino(&opened_data_metadata)
    {
        return Err(ProductInstallPlanError("product_install_data_home_invalid"));
    }
    let applications = open_or_create_child_directory(
        &data_home_dir,
        "applications",
        CapMetadataExt::dev(&opened_data_metadata),
    )?;
    let applications_metadata = applications
        .dir_metadata()
        .map_err(|_| ProductInstallPlanError("product_desktop_entry_directory_invalid"))?;
    let applications_path = data_home.join("applications");
    let applications_os_metadata = std::fs::symlink_metadata(&applications_path)
        .map_err(|_| ProductInstallPlanError("product_desktop_entry_directory_invalid"))?;
    if !applications_metadata.is_dir()
        || is_cap_reparse_point(&applications_metadata)
        || StdMetadataExt::uid(&applications_os_metadata) != rustix::process::geteuid().as_raw()
        || StdMetadataExt::mode(&applications_os_metadata) & 0o022 != 0
        || StdMetadataExt::dev(&applications_os_metadata)
            != CapMetadataExt::dev(&applications_metadata)
        || StdMetadataExt::ino(&applications_os_metadata)
            != CapMetadataExt::ino(&applications_metadata)
    {
        return Err(ProductInstallPlanError(
            "product_desktop_entry_directory_invalid",
        ));
    }

    let expected = linux_desktop_entry_content(&launcher_path)?;
    let entry_name = linux_desktop_entry_name(app_id)?;
    match applications.symlink_metadata(&entry_name) {
        Ok(_) => {
            verify_linux_desktop_entry(&applications, &entry_path, &expected)?;
            return Ok(entry_path);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(ProductInstallPlanError("product_desktop_entry_unavailable"));
        }
    }

    let mut suffix = [0_u8; 16];
    getrandom::getrandom(&mut suffix)
        .map_err(|_| ProductInstallPlanError("product_desktop_entry_nonce_failed"))?;
    let temporary_name = format!(".D4Pocket.{}.desktop.tmp", hex::encode(suffix));
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No)
        .mode(0o600)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let mut file = applications
        .open_with(&temporary_name, &options)
        .map_err(|_| ProductInstallPlanError("product_desktop_entry_temporary_create_failed"))?;
    if let Err(_) = file.write_all(&expected).and_then(|_| file.sync_all()) {
        let _ = applications.remove_file(&temporary_name);
        return Err(ProductInstallPlanError(
            "product_desktop_entry_write_failed",
        ));
    }
    let temporary_metadata = match file.metadata() {
        Ok(metadata) => metadata,
        Err(_) => {
            let _ = applications.remove_file(&temporary_name);
            return Err(ProductInstallPlanError(
                "product_desktop_entry_temporary_invalid",
            ));
        }
    };
    let temporary_os_metadata =
        match std::fs::symlink_metadata(applications_path.join(&temporary_name)) {
            Ok(metadata) => metadata,
            Err(_) => {
                let _ = applications.remove_file(&temporary_name);
                return Err(ProductInstallPlanError(
                    "product_desktop_entry_temporary_invalid",
                ));
            }
        };
    if !temporary_metadata.is_file()
        || is_cap_reparse_point(&temporary_metadata)
        || temporary_metadata.len() == 0
        || temporary_metadata.len() > MAX_LINUX_DESKTOP_ENTRY_BYTES
        || StdMetadataExt::uid(&temporary_os_metadata) != rustix::process::geteuid().as_raw()
        || StdMetadataExt::nlink(&temporary_os_metadata) != 1
        || StdMetadataExt::mode(&temporary_os_metadata) & 0o022 != 0
        || StdMetadataExt::mode(&temporary_os_metadata) & 0o111 != 0
        || StdMetadataExt::dev(&temporary_os_metadata) != CapMetadataExt::dev(&temporary_metadata)
        || StdMetadataExt::ino(&temporary_os_metadata) != CapMetadataExt::ino(&temporary_metadata)
    {
        let _ = applications.remove_file(&temporary_name);
        return Err(ProductInstallPlanError(
            "product_desktop_entry_temporary_invalid",
        ));
    }
    drop(file);

    match applications.hard_link(&temporary_name, &applications, &entry_name) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let _ = applications.remove_file(&temporary_name);
            verify_linux_desktop_entry(&applications, &entry_path, &expected)?;
            return Ok(entry_path);
        }
        Err(_) => {
            let _ = applications.remove_file(&temporary_name);
            return Err(ProductInstallPlanError(
                "product_desktop_entry_publish_failed",
            ));
        }
    }
    applications
        .remove_file(&temporary_name)
        .map_err(|_| ProductInstallPlanError("product_desktop_entry_temporary_cleanup_failed"))?;
    verify_linux_desktop_entry(&applications, &entry_path, &expected)?;
    Ok(entry_path)
}

#[cfg(target_os = "linux")]
fn linux_desktop_entry_name(app_id: &str) -> Result<String, ProductInstallPlanError> {
    if !valid_identity(app_id) {
        return Err(ProductInstallPlanError(
            "product_install_app_identity_invalid",
        ));
    }
    Ok(format!("{LINUX_DESKTOP_ENTRY_PREFIX}{app_id}.desktop"))
}

#[cfg(target_os = "linux")]
fn linux_desktop_entry_content(launcher: &Path) -> Result<Vec<u8>, ProductInstallPlanError> {
    if !launcher.is_absolute()
        || launcher
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(ProductInstallPlanError(
            "product_desktop_entry_launcher_invalid",
        ));
    }
    let launcher = launcher.to_str().ok_or(ProductInstallPlanError(
        "product_desktop_entry_launcher_invalid",
    ))?;
    let escaped = launcher
        .chars()
        .map(|character| match character {
            '\\' => "\\\\".to_owned(),
            '"' => "\\\"".to_owned(),
            '`' => "\\`".to_owned(),
            '$' => "\\$".to_owned(),
            '%' => "%%".to_owned(),
            _ => character.to_string(),
        })
        .collect::<String>();
    Ok(format!(
        "[Desktop Entry]\nType=Application\nName=D4 Pocket\nExec=\"{escaped}\"\nTryExec=\"{escaped}\"\nTerminal=false\nCategories=Utility;Development;\nStartupWMClass=d4-pocket\n"
    )
    .into_bytes())
}

#[cfg(target_os = "linux")]
fn verify_linux_desktop_entry(
    applications: &Dir,
    entry_path: &Path,
    expected: &[u8],
) -> Result<(), ProductInstallPlanError> {
    let entry_name = entry_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(ProductInstallPlanError("product_desktop_entry_invalid"))?;
    let before = applications
        .symlink_metadata(entry_name)
        .map_err(|_| ProductInstallPlanError("product_desktop_entry_unavailable"))?;
    if !before.is_file()
        || is_cap_reparse_point(&before)
        || before.len() == 0
        || before.len() > MAX_LINUX_DESKTOP_ENTRY_BYTES
    {
        return Err(ProductInstallPlanError("product_desktop_entry_invalid"));
    }
    let mut options = OpenOptions::new();
    options
        .read(true)
        .follow(FollowSymlinks::No)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let mut file = applications
        .open_with(entry_name, &options)
        .map_err(|_| ProductInstallPlanError("product_desktop_entry_invalid"))?;
    let opened = file
        .metadata()
        .map_err(|_| ProductInstallPlanError("product_desktop_entry_invalid"))?;
    let os_metadata = std::fs::symlink_metadata(entry_path)
        .map_err(|_| ProductInstallPlanError("product_desktop_entry_invalid"))?;
    if !opened.is_file()
        || is_cap_reparse_point(&opened)
        || !same_cap_file(&before, &opened)
        || StdMetadataExt::uid(&os_metadata) != rustix::process::geteuid().as_raw()
        || StdMetadataExt::nlink(&os_metadata) != 1
        || StdMetadataExt::mode(&os_metadata) & 0o022 != 0
        || StdMetadataExt::mode(&os_metadata) & 0o111 != 0
        || StdMetadataExt::dev(&os_metadata) != CapMetadataExt::dev(&opened)
        || StdMetadataExt::ino(&os_metadata) != CapMetadataExt::ino(&opened)
    {
        return Err(ProductInstallPlanError("product_desktop_entry_invalid"));
    }
    let mut actual = Vec::with_capacity(opened.len() as usize);
    file.read_to_end(&mut actual)
        .map_err(|_| ProductInstallPlanError("product_desktop_entry_invalid"))?;
    if actual != expected {
        return Err(ProductInstallPlanError("product_desktop_entry_conflict"));
    }
    Ok(())
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
        open_product_versions_directory, plan_product_install, INSTALL_DIRECTORY,
        PRODUCT_DIRECTORY, VERSIONS_DIRECTORY,
    };
    use std::path::Path;

    const APP_ID: &str = "d4-pocket-app-11111111111111111111111111111111";
    const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[cfg(windows)]
    fn test_local_app_data() -> &'static Path {
        Path::new("C:/Users/test/AppData/Local")
    }

    #[cfg(not(windows))]
    fn test_local_app_data() -> &'static Path {
        Path::new("/home/test/.local/share")
    }

    #[test]
    fn install_path_is_per_user_identity_and_content_bound() {
        let plan = plan_product_install(test_local_app_data(), APP_ID, "1.2.3", HASH).unwrap();
        assert_eq!(
            plan.root,
            test_local_app_data()
                .join(INSTALL_DIRECTORY)
                .join(PRODUCT_DIRECTORY)
                .join(APP_ID)
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
                .join(INSTALL_DIRECTORY)
                .join(PRODUCT_DIRECTORY)
                .join(APP_ID)
                .join("versions")
        );
        assert!(capability.dir_metadata().unwrap().is_dir());
        assert!(local_app_data
            .join(INSTALL_DIRECTORY)
            .join(PRODUCT_DIRECTORY)
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
        assert!(!local_app_data.join(INSTALL_DIRECTORY).exists());
        let product_root = local_app_data
            .join(INSTALL_DIRECTORY)
            .join(PRODUCT_DIRECTORY)
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
        assert!(!local_app_data.join(INSTALL_DIRECTORY).exists());

        let product_root = local_app_data
            .join(INSTALL_DIRECTORY)
            .join(PRODUCT_DIRECTORY)
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

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_data_home_follows_xdg_default_and_rejects_ambiguous_paths() {
        use super::resolve_linux_data_home;

        assert_eq!(
            resolve_linux_data_home(Path::new("/home/test"), None).unwrap(),
            Path::new("/home/test/.local/share")
        );
        assert_eq!(
            resolve_linux_data_home(
                Path::new("/home/test"),
                Some(Path::new("/mnt/data/user-share"))
            )
            .unwrap(),
            Path::new("/mnt/data/user-share")
        );
        assert_eq!(
            resolve_linux_data_home(Path::new("relative"), None)
                .unwrap_err()
                .0,
            "product_install_home_invalid"
        );
        for invalid in [Path::new("relative"), Path::new("/home/test/../shared")] {
            assert_eq!(
                resolve_linux_data_home(Path::new("/home/test"), Some(invalid))
                    .unwrap_err()
                    .0,
                "product_install_data_home_invalid"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_data_home_creation_is_nofollow_and_owner_private_from_group_writes() {
        use super::ensure_linux_data_home;
        use std::os::unix::fs::{symlink, PermissionsExt};

        let scratch = std::env::temp_dir().join(format!(
            "d4p-linux-product-data-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let data_home = scratch.join("nested").join("share");
        ensure_linux_data_home(&data_home).unwrap();
        assert!(data_home.is_dir());

        let target = scratch.join("target");
        std::fs::create_dir(&target).unwrap();
        let link = scratch.join("link");
        symlink(&target, &link).unwrap();
        assert_eq!(
            ensure_linux_data_home(&link).unwrap_err().0,
            "product_install_data_home_invalid"
        );

        std::fs::set_permissions(&data_home, std::fs::Permissions::from_mode(0o777)).unwrap();
        assert_eq!(
            ensure_linux_data_home(&data_home).unwrap_err().0,
            "product_install_data_home_invalid"
        );
        std::fs::set_permissions(&data_home, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_desktop_entry_is_fixed_atomic_idempotent_and_conflict_safe() {
        use super::{plan_linux_desktop_entry, register_linux_desktop_entry};
        use cap_std::fs::Dir;
        use std::os::unix::fs::{symlink, PermissionsExt};

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "d4 pocket%desktop-entry-{}-{unique}",
            std::process::id()
        ));
        let data_home = root.join("xdg-data");
        let product_root = data_home
            .join(INSTALL_DIRECTORY)
            .join(PRODUCT_DIRECTORY)
            .join(APP_ID);
        let product_version = "1.2.3";
        let package_sha256 = "a".repeat(64);
        let version_directory = product_root
            .join(VERSIONS_DIRECTORY)
            .join(format!("{product_version}-{package_sha256}"));
        std::fs::create_dir_all(&version_directory).unwrap();
        std::fs::write(
            version_directory.join("gui_shell_desktop_launcher"),
            b"#!/bin/sh\nexit 0\n",
        )
        .unwrap();
        std::fs::set_permissions(
            version_directory.join("gui_shell_desktop_launcher"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        std::fs::write(version_directory.join("product_manifest.json"), b"{}").unwrap();
        let data_home = std::fs::canonicalize(data_home).unwrap();
        let product_root = std::fs::canonicalize(product_root).unwrap();
        let root_directory =
            Dir::open_ambient_dir(&product_root, cap_std::ambient_authority()).unwrap();
        let versions_directory = Dir::open_ambient_dir(
            product_root.join(VERSIONS_DIRECTORY),
            cap_std::ambient_authority(),
        )
        .unwrap();
        crate::product_bootstrapper::activate_staged_version(
            &root_directory,
            &versions_directory,
            product_version,
            APP_ID,
            "audit-store-22222222222222222222222222222222",
            &package_sha256,
            "linux-entry-test-update",
            &format!("sha256:{}", "b".repeat(64)),
            &"c".repeat(32),
        )
        .unwrap();
        let next_version = "1.3.0";
        let next_package_sha256 = "d".repeat(64);
        let next_version_directory = product_root
            .join(VERSIONS_DIRECTORY)
            .join(format!("{next_version}-{next_package_sha256}"));
        std::fs::create_dir_all(&next_version_directory).unwrap();
        std::fs::write(
            next_version_directory.join("gui_shell_desktop_launcher"),
            b"#!/bin/sh\nexit 1\n",
        )
        .unwrap();
        std::fs::set_permissions(
            next_version_directory.join("gui_shell_desktop_launcher"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        std::fs::write(
            next_version_directory.join("product_manifest.json"),
            b"{\"v\":2}",
        )
        .unwrap();
        crate::product_bootstrapper::activate_staged_version(
            &root_directory,
            &versions_directory,
            next_version,
            APP_ID,
            "audit-store-22222222222222222222222222222222",
            &next_package_sha256,
            "linux-entry-test-update-2",
            &format!("sha256:{}", "e".repeat(64)),
            &"f".repeat(32),
        )
        .unwrap();
        assert_eq!(
            crate::product_bootstrapper::active_version_snapshot(
                &data_home,
                APP_ID,
                "audit-store-22222222222222222222222222222222",
            )
            .unwrap()
            .current
            .product_version,
            next_version
        );
        let planned = plan_linux_desktop_entry(&data_home, APP_ID).unwrap();
        assert_eq!(
            planned,
            data_home
                .join("applications")
                .join(format!("com.d4pocket.{APP_ID}.desktop"))
        );
        assert!(!planned.parent().unwrap().exists());

        let registered = register_linux_desktop_entry(
            &data_home,
            APP_ID,
            "audit-store-22222222222222222222222222222222",
            &product_root,
        )
        .unwrap();
        assert_eq!(registered, planned);
        let content = std::fs::read(&registered).unwrap();
        let content = String::from_utf8(content).unwrap();
        assert!(content.contains("Name=D4 Pocket\n"));
        assert!(content.contains("Exec=\""));
        assert!(content.contains("d4 pocket%%desktop-entry-"));
        assert_eq!(
            std::fs::metadata(&registered).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            register_linux_desktop_entry(
                &data_home,
                APP_ID,
                "audit-store-22222222222222222222222222222222",
                &product_root,
            )
            .unwrap(),
            registered
        );

        std::fs::write(&registered, b"attacker-controlled desktop entry\n").unwrap();
        assert_eq!(
            register_linux_desktop_entry(
                &data_home,
                APP_ID,
                "audit-store-22222222222222222222222222222222",
                &product_root,
            )
            .unwrap_err()
            .0,
            "product_desktop_entry_conflict"
        );
        std::fs::remove_file(&registered).unwrap();
        let target = root.join("attacker.desktop");
        std::fs::write(&target, b"attacker target\n").unwrap();
        symlink(&target, &registered).unwrap();
        assert!(register_linux_desktop_entry(
            &data_home,
            APP_ID,
            "audit-store-22222222222222222222222222222222",
            &product_root,
        )
        .is_err());
        assert_eq!(std::fs::read(target).unwrap(), b"attacker target\n");
        std::fs::remove_dir_all(root).unwrap();
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
            plan_product_install(
                &test_local_app_data().join("..").join("other"),
                APP_ID,
                "1.2.3",
                HASH
            )
            .unwrap_err()
            .0,
            "product_install_local_app_data_invalid"
        );
        assert_eq!(
            plan_product_install(test_local_app_data(), "../../escape", "1.2.3", HASH)
                .unwrap_err()
                .0,
            "product_install_app_identity_invalid"
        );
        assert_eq!(
            plan_product_install(test_local_app_data(), APP_ID, "../1.2.3", HASH)
                .unwrap_err()
                .0,
            "product_install_version_invalid"
        );
        assert_eq!(
            plan_product_install(
                test_local_app_data(),
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
    fn uninstall_removes_only_fixed_product_and_matching_shortcut() {
        let scratch = std::env::temp_dir().join(format!(
            "d4p-uninstall-{}-{}",
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
        let retained_store = local_app_data
            .join("D4Pocket")
            .join("apps")
            .join(APP_ID)
            .join("stores")
            .join("audit-store-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        std::fs::create_dir_all(product_root.join("versions")).unwrap();
        std::fs::create_dir_all(&retained_store).unwrap();
        std::fs::create_dir_all(&start_menu).unwrap();
        std::fs::write(
            product_root.join("gui_shell_desktop_launcher.exe"),
            b"fixture bootstrapper",
        )
        .unwrap();
        std::fs::write(retained_store.join("audit.jsonl"), b"retained audit").unwrap();
        let shortcut = super::register_start_menu_shortcut(
            &local_app_data,
            &start_menu,
            APP_ID,
            &product_root,
        )
        .unwrap();

        super::remove_installed_product(&local_app_data, &start_menu, APP_ID).unwrap();

        assert!(!product_root.exists());
        assert!(!shortcut.exists());
        assert_eq!(
            std::fs::read(retained_store.join("audit.jsonl")).unwrap(),
            b"retained audit"
        );
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn uninstall_conflicting_shortcut_preserves_product_root() {
        let scratch = std::env::temp_dir().join(format!(
            "d4p-uninstall-conflict-{}-{}",
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
        let shortcut_directory = start_menu.join("Programs").join("D4 Pocket");
        std::fs::create_dir_all(&product_root).unwrap();
        std::fs::create_dir_all(&shortcut_directory).unwrap();
        std::fs::create_dir_all(&start_menu).unwrap();
        std::fs::write(
            product_root.join("gui_shell_desktop_launcher.exe"),
            b"fixture bootstrapper",
        )
        .unwrap();
        let unrelated = scratch.join("unrelated.exe");
        std::fs::write(&unrelated, b"unrelated target").unwrap();
        let shortcut = shortcut_directory.join("D4 Pocket.lnk");
        super::save_shell_link(&shortcut, &unrelated).unwrap();
        let original = std::fs::read(&shortcut).unwrap();

        assert_eq!(
            super::remove_installed_product(&local_app_data, &start_menu, APP_ID)
                .unwrap_err()
                .0,
            "product_start_menu_shortcut_conflict"
        );
        assert!(product_root.exists());
        assert_eq!(std::fs::read(shortcut).unwrap(), original);
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
