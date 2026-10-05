//! Windows製品の導入先をBroker所有の固定規則から導出する。
//!
//! このmoduleはpath計画だけを行い、directory作成、package展開、shortcut登録、
//! Permission／Approval／Auditを実行しない。実作用はBroker consumerからのみ接続する。

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

/// Windows Known Folder APIが返す現在利用者LocalAppDataから導入先を計画する。
#[cfg(windows)]
pub(crate) fn plan_current_user_product_install(
    app_id: &str,
    product_version: &str,
    package_sha256: &str,
) -> Result<ProductInstallPlan, ProductInstallPlanError> {
    let local_app_data = winsafe::SHGetKnownFolderPath(
        &winsafe::co::KNOWNFOLDERID::LocalAppData,
        winsafe::co::KF::DEFAULT,
        None,
    )
    .map_err(|_| ProductInstallPlanError("product_install_known_folder_unavailable"))?;
    plan_product_install(
        Path::new(&local_app_data),
        app_id,
        product_version,
        package_sha256,
    )
}

/// OSが返した現在利用者のLocalAppDataと製品Manifest identityから、導入先を固定導出する。
/// UI、manifest、更新候補から任意の保存pathを受け取らない。
pub(crate) fn plan_product_install(
    local_app_data: &Path,
    app_id: &str,
    product_version: &str,
    package_sha256: &str,
) -> Result<ProductInstallPlan, ProductInstallPlanError> {
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
    if !valid_product_version(product_version) {
        return Err(ProductInstallPlanError("product_install_version_invalid"));
    }
    if !valid_sha256(package_sha256) {
        return Err(ProductInstallPlanError(
            "product_install_package_hash_invalid",
        ));
    }

    let root = local_app_data
        .join(INSTALL_DIRECTORY)
        .join(PRODUCT_DIRECTORY)
        .join(app_id);
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
    use super::plan_product_install;
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
