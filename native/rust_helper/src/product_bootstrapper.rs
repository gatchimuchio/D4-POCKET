//! 固定installed rootの起動入口からBroker選択済み版を安全に解決する。

use cap_fs_ext::{
    DirExt, FollowSymlinks, MetadataExt as CapMetadataExt, OpenOptionsFollowExt, OsMetadataExt as _,
};
use cap_std::fs::{Dir, OpenOptions};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const ACTIVE_VERSION_FILE: &str = "active_version.json";
const VERSIONED_LAUNCHER: &str = "gui_shell_desktop_launcher.exe";
const PRODUCT_MANIFEST: &str = "product_manifest.json";
const MAX_ACTIVE_VERSION_BYTES: u64 = 4096;
const MAX_PRODUCT_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_LAUNCHER_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveVersionRecord {
    version: u32,
    product: String,
    app_id: String,
    audit_store_id: String,
    product_version: String,
    package_sha256: String,
    launcher_sha256: String,
    product_manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BootstrapperError(pub(crate) &'static str);

/// OS Known Folder由来のrootだけを認識し、任意pathを有効版として採用しない。
pub(crate) fn is_fixed_installed_entrypoint(
    launcher: &Path,
    local_app_data: &Path,
    app_id: &str,
) -> bool {
    let Ok(root) = crate::broker::product_install::product_install_root(local_app_data, app_id)
    else {
        return false;
    };
    let expected = root.join(VERSIONED_LAUNCHER);
    let (Ok(actual), Ok(expected)) = (fs::canonicalize(launcher), fs::canonicalize(expected))
    else {
        return false;
    };
    actual
        .to_string_lossy()
        .eq_ignore_ascii_case(&expected.to_string_lossy())
}

/// Brokerが作成した固定有効版記録から、内容hash一致した版別起動器だけを返す。
pub(crate) fn resolve_active_version_launcher(
    local_app_data: &Path,
    expected_app_id: &str,
    expected_audit_store_id: &str,
) -> Result<PathBuf, BootstrapperError> {
    let (product_root, root) =
        crate::broker::product_install::open_existing_product_root(local_app_data, expected_app_id)
            .map_err(|_| BootstrapperError("installed_product_root_unavailable"))?;
    let record_bytes = read_regular_file(&root, ACTIVE_VERSION_FILE, MAX_ACTIVE_VERSION_BYTES)?;
    let record: ActiveVersionRecord = serde_json::from_slice(&record_bytes)
        .map_err(|_| BootstrapperError("active_version_record_invalid"))?;
    if record.version != 1
        || record.product != "D4 Pocket"
        || record.app_id != expected_app_id
        || record.audit_store_id != expected_audit_store_id
        || !valid_product_version(&record.product_version)
        || !valid_sha256(&record.package_sha256)
        || !valid_sha256(&record.launcher_sha256)
        || !valid_sha256(&record.product_manifest_sha256)
    {
        return Err(BootstrapperError("active_version_record_invalid"));
    }

    let root_metadata = root
        .dir_metadata()
        .map_err(|_| BootstrapperError("installed_product_root_unavailable"))?;
    let root_device = CapMetadataExt::dev(&root_metadata);
    let versions = open_existing_directory(&root, "versions", root_device)?;
    let stage_name = format!("{}-{}", record.product_version, record.package_sha256);
    let stage = open_existing_directory(&versions, &stage_name, root_device)?;
    if hash_regular_file(&stage, VERSIONED_LAUNCHER, MAX_LAUNCHER_BYTES)? != record.launcher_sha256
        || hash_regular_file(&stage, PRODUCT_MANIFEST, MAX_PRODUCT_MANIFEST_BYTES)?
            != record.product_manifest_sha256
    {
        return Err(BootstrapperError("active_version_payload_mismatch"));
    }

    let stage_path = product_root.join("versions").join(stage_name);
    let launcher_path = stage_path.join(VERSIONED_LAUNCHER);
    let stage_path = fs::canonicalize(&stage_path)
        .map_err(|_| BootstrapperError("active_version_directory_invalid"))?;
    let launcher_metadata = fs::symlink_metadata(&launcher_path)
        .map_err(|_| BootstrapperError("active_version_launcher_unavailable"))?;
    if !launcher_metadata.is_file()
        || launcher_metadata.file_type().is_symlink()
        || is_reparse_point(&launcher_metadata)
    {
        return Err(BootstrapperError("active_version_launcher_invalid"));
    }
    let launcher_path = fs::canonicalize(&launcher_path)
        .map_err(|_| BootstrapperError("active_version_launcher_invalid"))?;
    if !launcher_path.starts_with(&stage_path) {
        return Err(BootstrapperError("active_version_launcher_outside_stage"));
    }
    Ok(launcher_path)
}

fn read_regular_file(
    directory: &Dir,
    name: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, BootstrapperError> {
    let before = directory
        .symlink_metadata(name)
        .map_err(|_| BootstrapperError("active_version_file_unavailable"))?;
    if !before.is_file() || is_cap_reparse_point(&before) || before.len() > max_bytes {
        return Err(BootstrapperError("active_version_file_invalid"));
    }
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut file = directory
        .open_with(name, &options)
        .map_err(|_| BootstrapperError("active_version_file_invalid"))?;
    let opened = file
        .metadata()
        .map_err(|_| BootstrapperError("active_version_file_invalid"))?;
    if !opened.is_file() || is_cap_reparse_point(&opened) || !same_cap_file(&before, &opened) {
        return Err(BootstrapperError("active_version_file_changed"));
    }
    let mut bytes = Vec::with_capacity(opened.len() as usize);
    let mut limited = file.by_ref().take(max_bytes.saturating_add(1));
    limited
        .read_to_end(&mut bytes)
        .map_err(|_| BootstrapperError("active_version_file_read_failed"))?;
    if bytes.len() as u64 > max_bytes {
        return Err(BootstrapperError("active_version_file_invalid"));
    }
    let after = directory
        .symlink_metadata(name)
        .map_err(|_| BootstrapperError("active_version_file_changed"))?;
    if !same_cap_file(&opened, &after) || after.len() != bytes.len() as u64 {
        return Err(BootstrapperError("active_version_file_changed"));
    }
    Ok(bytes)
}

fn hash_regular_file(
    directory: &Dir,
    name: &str,
    max_bytes: u64,
) -> Result<String, BootstrapperError> {
    let before = directory
        .symlink_metadata(name)
        .map_err(|_| BootstrapperError("active_version_payload_unavailable"))?;
    if !before.is_file() || is_cap_reparse_point(&before) || before.len() > max_bytes {
        return Err(BootstrapperError("active_version_payload_invalid"));
    }
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut file = directory
        .open_with(name, &options)
        .map_err(|_| BootstrapperError("active_version_payload_invalid"))?;
    let opened = file
        .metadata()
        .map_err(|_| BootstrapperError("active_version_payload_invalid"))?;
    if !opened.is_file() || is_cap_reparse_point(&opened) || !same_cap_file(&before, &opened) {
        return Err(BootstrapperError("active_version_payload_changed"));
    }
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| BootstrapperError("active_version_payload_read_failed"))?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        if total > max_bytes {
            return Err(BootstrapperError("active_version_payload_invalid"));
        }
        hasher.update(&buffer[..read]);
    }
    let after = directory
        .symlink_metadata(name)
        .map_err(|_| BootstrapperError("active_version_payload_changed"))?;
    if !same_cap_file(&opened, &after) || after.len() != total || opened.len() != total {
        return Err(BootstrapperError("active_version_payload_changed"));
    }
    Ok(hex::encode(hasher.finalize()))
}

fn open_existing_directory(
    parent: &Dir,
    name: &str,
    root_device: u64,
) -> Result<Dir, BootstrapperError> {
    let before = parent
        .symlink_metadata(name)
        .map_err(|_| BootstrapperError("active_version_directory_unavailable"))?;
    if !before.is_dir() || is_cap_reparse_point(&before) {
        return Err(BootstrapperError("active_version_directory_invalid"));
    }
    let opened = parent
        .open_dir_nofollow(name)
        .map_err(|_| BootstrapperError("active_version_directory_invalid"))?;
    let after = opened
        .dir_metadata()
        .map_err(|_| BootstrapperError("active_version_directory_invalid"))?;
    if !after.is_dir()
        || is_cap_reparse_point(&after)
        || !same_cap_file(&before, &after)
        || CapMetadataExt::dev(&after) != root_device
    {
        return Err(BootstrapperError("active_version_directory_changed"));
    }
    Ok(opened)
}

fn same_cap_file(left: &cap_std::fs::Metadata, right: &cap_std::fs::Metadata) -> bool {
    let inode = CapMetadataExt::ino(left);
    inode != 0
        && CapMetadataExt::dev(left) == CapMetadataExt::dev(right)
        && inode == CapMetadataExt::ino(right)
}

fn is_cap_reparse_point(metadata: &cap_std::fs::Metadata) -> bool {
    metadata.file_attributes() & 0x400 != 0
}

fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

fn valid_product_version(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+' | b'_'))
        && value.bytes().any(|byte| byte.is_ascii_digit())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::resolve_active_version_launcher;
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::path::Path;

    const APP_ID: &str = "d4-pocket-app-11111111111111111111111111111111";
    const AUDIT_ID: &str = "audit-store-22222222222222222222222222222222";

    fn fixture(label: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let local = std::env::temp_dir().join(format!(
            "d4p-bootstrapper-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = local.join("Programs").join("D4 Pocket").join(APP_ID);
        let package_hash = "a".repeat(64);
        let version_dir = root.join("versions").join(format!("1.2.3-{package_hash}"));
        fs::create_dir_all(&version_dir).unwrap();
        fs::write(
            root.join("gui_shell_desktop_launcher.exe"),
            b"bootstrapper launcher",
        )
        .unwrap();
        let launcher_bytes = b"fixed version launcher";
        let manifest_bytes = b"fixed product manifest";
        fs::write(
            version_dir.join("gui_shell_desktop_launcher.exe"),
            launcher_bytes,
        )
        .unwrap();
        fs::write(version_dir.join("product_manifest.json"), manifest_bytes).unwrap();
        let record = json!({
            "version": 1,
            "product": "D4 Pocket",
            "app_id": APP_ID,
            "audit_store_id": AUDIT_ID,
            "product_version": "1.2.3",
            "package_sha256": package_hash,
            "launcher_sha256": hex::encode(Sha256::digest(launcher_bytes)),
            "product_manifest_sha256": hex::encode(Sha256::digest(manifest_bytes)),
        });
        fs::write(
            root.join("active_version.json"),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
        (local, root)
    }

    #[test]
    fn active_version_resolves_only_fixed_root_hashed_launcher() {
        let (local, root) = fixture("valid");
        let result = resolve_active_version_launcher(&local, APP_ID, AUDIT_ID).unwrap();
        assert_eq!(
            result,
            fs::canonicalize(
                root.join("versions")
                    .join(format!("1.2.3-{}", "a".repeat(64)))
                    .join("gui_shell_desktop_launcher.exe")
            )
            .unwrap()
        );
        fs::remove_dir_all(local).unwrap();
    }

    #[test]
    fn active_version_rejects_identity_hash_and_duplicate_field_mismatch() {
        let (local, root) = fixture("reject");
        assert_eq!(
            resolve_active_version_launcher(
                &local,
                APP_ID,
                "audit-store-33333333333333333333333333333333"
            )
            .unwrap_err()
            .0,
            "active_version_record_invalid"
        );
        let active = root.join("active_version.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&active).unwrap()).unwrap();
        value["launcher_sha256"] = json!("b".repeat(64));
        fs::write(&active, serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(
            resolve_active_version_launcher(&local, APP_ID, AUDIT_ID)
                .unwrap_err()
                .0,
            "active_version_payload_mismatch"
        );
        fs::write(
            &active,
            br#"{"version":1,"version":1,"product":"D4 Pocket"}"#,
        )
        .unwrap();
        assert_eq!(
            resolve_active_version_launcher(&local, APP_ID, AUDIT_ID)
                .unwrap_err()
                .0,
            "active_version_record_invalid"
        );
        fs::remove_dir_all(local).unwrap();
    }

    #[test]
    fn active_version_rejects_unavailable_and_non_regular_stage_payload() {
        let (local, root) = fixture("missing");
        fs::remove_file(
            root.join("versions")
                .join(format!("1.2.3-{}", "a".repeat(64)))
                .join("product_manifest.json"),
        )
        .unwrap();
        assert_eq!(
            resolve_active_version_launcher(&local, APP_ID, AUDIT_ID)
                .unwrap_err()
                .0,
            "active_version_payload_unavailable"
        );
        fs::remove_dir_all(local).unwrap();
    }

    #[test]
    fn active_version_rejects_stage_junction() {
        let (local, root) = fixture("junction");
        let versions = root.join("versions");
        let stage = versions.join(format!("1.2.3-{}", "a".repeat(64)));
        let outside = local.join("outside-stage");
        fs::remove_dir_all(&stage).unwrap();
        fs::create_dir_all(&outside).unwrap();

        let result = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&stage)
            .arg(&outside)
            .output()
            .expect("Bootstrapper junction試験の作成");
        assert!(
            result.status.success(),
            "Bootstrapper junction試験の作成失敗"
        );

        assert_eq!(
            resolve_active_version_launcher(&local, APP_ID, AUDIT_ID)
                .unwrap_err()
                .0,
            "active_version_directory_invalid"
        );
        assert!(fs::read_dir(&outside).unwrap().next().is_none());

        fs::remove_dir(&stage).unwrap();
        fs::remove_dir_all(local).unwrap();
    }

    #[test]
    fn installed_entrypoint_recognition_is_fixed_to_identity_root() {
        let (local, root) = fixture("entrypoint");
        let local = fs::canonicalize(local).unwrap();
        assert!(super::is_fixed_installed_entrypoint(
            &root.join("gui_shell_desktop_launcher.exe"),
            &local,
            APP_ID,
        ));
        assert!(!super::is_fixed_installed_entrypoint(
            Path::new("C:/portable/gui_shell_desktop_launcher.exe"),
            &local,
            APP_ID,
        ));
        fs::remove_dir_all(local).unwrap();
    }
}
