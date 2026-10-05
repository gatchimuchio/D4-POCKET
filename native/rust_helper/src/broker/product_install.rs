//! Windows製品の導入先をBroker所有の固定規則から導出する。
//!
//! 固定path計画とBroker専用のversions directory capability取得を行う。
//! package展開、shortcut登録、Permission／Approval／Auditは行わず、実作用はBroker consumerが統治する。

#[cfg(windows)]
use cap_fs_ext::OsMetadataExt as _;
use cap_fs_ext::{DirExt, MetadataExt as CapMetadataExt};
use cap_std::fs::Dir;
use std::path::{Component, Path, PathBuf};

const INSTALL_DIRECTORY: &str = "Programs";
const PRODUCT_DIRECTORY: &str = "D4 Pocket";
const VERSIONS_DIRECTORY: &str = "versions";

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
        open_existing_product_root, open_product_versions_directory, plan_product_install,
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
