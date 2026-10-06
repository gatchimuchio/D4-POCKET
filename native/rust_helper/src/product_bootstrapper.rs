//! 固定installed rootの起動入口からBroker選択済み版を安全に解決する。

use cap_fs_ext::{
    DirExt, FollowSymlinks, MetadataExt as CapMetadataExt, OpenOptionsFollowExt, OsMetadataExt as _,
};
use cap_std::fs::{Dir, OpenOptions};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Deserialize, Serialize)]
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
    #[serde(default)]
    update_id: String,
    #[serde(default)]
    candidate_hash: String,
    #[serde(default)]
    previous: Option<ActiveVersionDescriptor>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActiveVersionDescriptor {
    pub(crate) product_version: String,
    pub(crate) package_sha256: String,
    pub(crate) launcher_sha256: String,
    pub(crate) product_manifest_sha256: String,
    pub(crate) update_id: String,
    pub(crate) candidate_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveVersionSnapshot {
    pub(crate) current: ActiveVersionDescriptor,
    pub(crate) previous: Option<ActiveVersionDescriptor>,
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
    if !valid_record(&record, expected_app_id, expected_audit_store_id) {
        return Err(BootstrapperError("active_version_record_invalid"));
    }

    let root_metadata = root
        .dir_metadata()
        .map_err(|_| BootstrapperError("installed_product_root_unavailable"))?;
    let root_device = CapMetadataExt::dev(&root_metadata);
    let versions = open_existing_directory(&root, "versions", root_device)?;
    let stage_name = format!("{}-{}", record.product_version, record.package_sha256);
    open_existing_directory(&versions, &stage_name, root_device)?;
    validate_descriptor_stage(
        &versions,
        &descriptor_from_record(&record),
        root_device,
    )?;

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

/// 固定rootの有効版recordが現在の実行fileを選択している場合だけ導入済み起動と認める。
pub(crate) fn is_active_version_launcher(
    launcher: &Path,
    local_app_data: &Path,
    expected_app_id: &str,
    expected_audit_store_id: &str,
) -> bool {
    let Ok(active_launcher) =
        resolve_active_version_launcher(local_app_data, expected_app_id, expected_audit_store_id)
    else {
        return false;
    };
    let Ok(launcher) = fs::canonicalize(launcher) else {
        return false;
    };
    canonical_paths_equal(&launcher, &active_launcher)
}

fn canonical_paths_equal(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

/// 完全検証済みstageから固定root Bootstrapperと有効版recordをBroker所有capabilityで公開する。
/// active recordは一時fileを同期した後、同じroot内renameで置換する。
pub(crate) fn activate_staged_version(
    product_root: &Dir,
    versions: &Dir,
    product_version: &str,
    app_id: &str,
    audit_store_id: &str,
    package_sha256: &str,
    update_id: &str,
    candidate_hash: &str,
    temporary_suffix: &str,
) -> Result<(), BootstrapperError> {
    if !valid_product_version(product_version)
        || !valid_sha256(package_sha256)
        || !valid_update_id(update_id)
        || !valid_candidate_hash(candidate_hash)
        || temporary_suffix.len() != 32
        || !temporary_suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(BootstrapperError("active_version_record_invalid"));
    }
    let root_metadata = product_root
        .dir_metadata()
        .map_err(|_| BootstrapperError("installed_product_root_unavailable"))?;
    if !root_metadata.is_dir() || is_cap_reparse_point(&root_metadata) {
        return Err(BootstrapperError("installed_product_root_invalid"));
    }
    let root_device = CapMetadataExt::dev(&root_metadata);
    let versions_entry = product_root
        .symlink_metadata("versions")
        .map_err(|_| BootstrapperError("active_version_directory_unavailable"))?;
    let versions_metadata = versions
        .dir_metadata()
        .map_err(|_| BootstrapperError("active_version_directory_invalid"))?;
    if !versions_entry.is_dir()
        || is_cap_reparse_point(&versions_entry)
        || !versions_metadata.is_dir()
        || is_cap_reparse_point(&versions_metadata)
        || !same_cap_file(&versions_entry, &versions_metadata)
        || CapMetadataExt::dev(&versions_metadata) != root_device
    {
        return Err(BootstrapperError("active_version_directory_invalid"));
    }

    let stage_name = format!("{product_version}-{package_sha256}");
    let stage = open_existing_directory(versions, &stage_name, root_device)?;
    let launcher_sha256 = hash_regular_file(&stage, VERSIONED_LAUNCHER, MAX_LAUNCHER_BYTES)?;
    let product_manifest_sha256 =
        hash_regular_file(&stage, PRODUCT_MANIFEST, MAX_PRODUCT_MANIFEST_BYTES)?;

    let existing = read_existing_active_record(product_root)?;
    let previous = match existing {
        Some(existing) => {
            if !valid_record(&existing, app_id, audit_store_id) {
                return Err(BootstrapperError("active_version_record_invalid"));
            }
            validate_descriptor_stage(versions, &descriptor_from_record(&existing), root_device)?;
            if existing.product_version == product_version
                && existing.package_sha256 == package_sha256
            {
                existing.previous
            } else {
                let descriptor = descriptor_from_record(&existing);
                valid_bound_descriptor(&descriptor).then_some(descriptor)
            }
        }
        None => None,
    };

    match product_root.symlink_metadata(VERSIONED_LAUNCHER) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.len() == 0 || is_cap_reparse_point(&metadata) {
                return Err(BootstrapperError("installed_bootstrapper_invalid"));
            }
            hash_regular_file(product_root, VERSIONED_LAUNCHER, MAX_LAUNCHER_BYTES)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            stage
                .hard_link(VERSIONED_LAUNCHER, product_root, VERSIONED_LAUNCHER)
                .map_err(|_| BootstrapperError("installed_bootstrapper_create_failed"))?;
            let source = stage
                .symlink_metadata(VERSIONED_LAUNCHER)
                .map_err(|_| BootstrapperError("installed_bootstrapper_invalid"))?;
            let installed = product_root
                .symlink_metadata(VERSIONED_LAUNCHER)
                .map_err(|_| BootstrapperError("installed_bootstrapper_invalid"))?;
            if !source.is_file()
                || !installed.is_file()
                || is_cap_reparse_point(&source)
                || is_cap_reparse_point(&installed)
                || !same_cap_file(&source, &installed)
            {
                return Err(BootstrapperError("installed_bootstrapper_invalid"));
            }
        }
        Err(_) => return Err(BootstrapperError("installed_bootstrapper_invalid")),
    }

    let record = ActiveVersionRecord {
        version: 1,
        product: "D4 Pocket".to_owned(),
        app_id: app_id.to_owned(),
        audit_store_id: audit_store_id.to_owned(),
        product_version: product_version.to_owned(),
        package_sha256: package_sha256.to_owned(),
        launcher_sha256,
        product_manifest_sha256,
        update_id: update_id.to_owned(),
        candidate_hash: candidate_hash.to_owned(),
        previous,
    };
    publish_active_version_record(product_root, &record, temporary_suffix)
}

pub(crate) fn active_version_snapshot(
    local_app_data: &Path,
    expected_app_id: &str,
    expected_audit_store_id: &str,
) -> Result<ActiveVersionSnapshot, BootstrapperError> {
    let (product_root, root) =
        crate::broker::product_install::open_existing_product_root(local_app_data, expected_app_id)
            .map_err(|_| BootstrapperError("installed_product_root_unavailable"))?;
    let record = read_existing_active_record(&root)?
        .ok_or(BootstrapperError("active_version_file_unavailable"))?;
    if !valid_record(&record, expected_app_id, expected_audit_store_id) {
        return Err(BootstrapperError("active_version_record_invalid"));
    }
    let root_metadata = root
        .dir_metadata()
        .map_err(|_| BootstrapperError("installed_product_root_unavailable"))?;
    let root_device = CapMetadataExt::dev(&root_metadata);
    let versions = open_existing_directory(&root, "versions", root_device)?;
    let current = descriptor_from_record(&record);
    validate_descriptor_stage(&versions, &current, root_device)?;
    if let Some(previous) = &record.previous {
        if !valid_descriptor(previous) {
            return Err(BootstrapperError("active_version_previous_invalid"));
        }
        validate_descriptor_stage(&versions, previous, root_device)?;
    }
    let _ = product_root;
    Ok(ActiveVersionSnapshot {
        current,
        previous: record.previous,
    })
}

/// Brokerが再照合した現行stageから、欠損している固定root Bootstrapperだけを復元する。
/// 既存file、active record、version-local payloadは上書きしない。
pub(crate) fn restore_missing_root_bootstrapper(
    local_app_data: &Path,
    expected_app_id: &str,
    expected_audit_store_id: &str,
    expected_current: &ActiveVersionDescriptor,
) -> Result<bool, BootstrapperError> {
    let snapshot = active_version_snapshot(
        local_app_data,
        expected_app_id,
        expected_audit_store_id,
    )?;
    if &snapshot.current != expected_current {
        return Err(BootstrapperError("active_version_record_changed"));
    }
    let (_, product_root) =
        crate::broker::product_install::open_existing_product_root(local_app_data, expected_app_id)
            .map_err(|_| BootstrapperError("installed_product_root_unavailable"))?;
    let root_metadata = product_root
        .dir_metadata()
        .map_err(|_| BootstrapperError("installed_product_root_unavailable"))?;
    if !root_metadata.is_dir() || is_cap_reparse_point(&root_metadata) {
        return Err(BootstrapperError("installed_product_root_invalid"));
    }
    let root_device = CapMetadataExt::dev(&root_metadata);
    let versions = open_existing_directory(&product_root, "versions", root_device)?;
    let stage_name = format!(
        "{}-{}",
        expected_current.product_version, expected_current.package_sha256
    );
    let stage = open_existing_directory(&versions, &stage_name, root_device)?;
    validate_descriptor_stage(&versions, expected_current, root_device)?;
    crate::product_package::verify_staged_product_package_hash(
        &stage,
        crate::product_package::ProductPackageExpectation {
            product_version: Some(&expected_current.product_version),
            app_id: expected_app_id,
            audit_store_id: expected_audit_store_id,
        },
        &expected_current.package_sha256,
    )
    .map_err(|_| BootstrapperError("active_version_package_mismatch"))?;

    match product_root.symlink_metadata(VERSIONED_LAUNCHER) {
        Ok(metadata) => {
            if !metadata.is_file() || is_cap_reparse_point(&metadata) {
                return Err(BootstrapperError("installed_bootstrapper_invalid"));
            }
            let digest = hash_regular_file(&product_root, VERSIONED_LAUNCHER, MAX_LAUNCHER_BYTES)?;
            if digest != expected_current.launcher_sha256 {
                return Err(BootstrapperError("installed_bootstrapper_mismatch"));
            }
            Ok(false)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            stage
                .hard_link(VERSIONED_LAUNCHER, &product_root, VERSIONED_LAUNCHER)
                .map_err(|_| BootstrapperError("installed_bootstrapper_create_failed"))?;
            let source = stage
                .symlink_metadata(VERSIONED_LAUNCHER)
                .map_err(|_| BootstrapperError("active_version_launcher_invalid"))?;
            let installed = product_root
                .symlink_metadata(VERSIONED_LAUNCHER)
                .map_err(|_| BootstrapperError("installed_bootstrapper_invalid"))?;
            if !source.is_file()
                || !installed.is_file()
                || is_cap_reparse_point(&source)
                || is_cap_reparse_point(&installed)
                || !same_cap_file(&source, &installed)
                || hash_regular_file(&product_root, VERSIONED_LAUNCHER, MAX_LAUNCHER_BYTES)?
                    != expected_current.launcher_sha256
            {
                return Err(BootstrapperError("installed_bootstrapper_invalid"));
            }
            Ok(true)
        }
        Err(_) => Err(BootstrapperError("installed_bootstrapper_invalid")),
    }
}

pub(crate) fn toggle_previous_active_version(
    product_root: &Dir,
    versions: &Dir,
    app_id: &str,
    audit_store_id: &str,
    current_update_id: &str,
    current_candidate_hash: &str,
    target_update_id: &str,
    target_candidate_hash: &str,
    temporary_suffix: &str,
) -> Result<ActiveVersionDescriptor, BootstrapperError> {
    if temporary_suffix.len() != 32
        || !temporary_suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(BootstrapperError("active_version_record_invalid"));
    }
    let mut current = read_existing_active_record(product_root)?
        .ok_or(BootstrapperError("active_version_file_unavailable"))?;
    if !valid_record(&current, app_id, audit_store_id)
        || current.update_id != current_update_id
        || current.candidate_hash != current_candidate_hash
    {
        return Err(BootstrapperError("active_version_rollback_current_stale"));
    }
    let root_metadata = product_root
        .dir_metadata()
        .map_err(|_| BootstrapperError("installed_product_root_unavailable"))?;
    let root_device = CapMetadataExt::dev(&root_metadata);
    let versions_metadata = versions
        .dir_metadata()
        .map_err(|_| BootstrapperError("active_version_directory_invalid"))?;
    if !versions_metadata.is_dir() || is_cap_reparse_point(&versions_metadata) {
        return Err(BootstrapperError("active_version_directory_invalid"));
    }
    let target = current
        .previous
        .take()
        .ok_or(BootstrapperError("active_version_rollback_unavailable"))?;
    if target.update_id != target_update_id || target.candidate_hash != target_candidate_hash {
        return Err(BootstrapperError("active_version_rollback_target_stale"));
    }
    validate_descriptor_stage(versions, &target, root_device)?;
    let previous = descriptor_from_record(&current);
    current.product_version = target.product_version.clone();
    current.package_sha256 = target.package_sha256.clone();
    current.launcher_sha256 = target.launcher_sha256.clone();
    current.product_manifest_sha256 = target.product_manifest_sha256.clone();
    current.update_id = target.update_id.clone();
    current.candidate_hash = target.candidate_hash.clone();
    current.previous = Some(previous);
    publish_active_version_record(product_root, &current, temporary_suffix)?;
    Ok(target)
}

fn publish_active_version_record(
    product_root: &Dir,
    record: &ActiveVersionRecord,
    temporary_suffix: &str,
) -> Result<(), BootstrapperError> {
    let bytes = serde_json::to_vec(&record)
        .map_err(|_| BootstrapperError("active_version_record_serialize_failed"))?;
    let temporary_name = format!("active_version.{temporary_suffix}.tmp");
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    let mut temporary = product_root
        .open_with(&temporary_name, &options)
        .map_err(|_| BootstrapperError("active_version_temporary_create_failed"))?;
    if std::io::Write::write_all(&mut temporary, &bytes).is_err() || temporary.sync_all().is_err() {
        drop(temporary);
        let _ = product_root.remove_file(&temporary_name);
        return Err(BootstrapperError("active_version_temporary_write_failed"));
    }
    drop(temporary);

    match product_root.symlink_metadata(ACTIVE_VERSION_FILE) {
        Ok(metadata) if metadata.is_file() && !is_cap_reparse_point(&metadata) => {}
        Ok(_) => {
            let _ = product_root.remove_file(&temporary_name);
            return Err(BootstrapperError("active_version_file_invalid"));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            let _ = product_root.remove_file(&temporary_name);
            return Err(BootstrapperError("active_version_file_invalid"));
        }
    }
    if product_root
        .rename(&temporary_name, product_root, ACTIVE_VERSION_FILE)
        .is_err()
    {
        let _ = product_root.remove_file(&temporary_name);
        return Err(BootstrapperError("active_version_record_publish_failed"));
    }
    Ok(())
}

fn read_existing_active_record(
    product_root: &Dir,
) -> Result<Option<ActiveVersionRecord>, BootstrapperError> {
    match product_root.symlink_metadata(ACTIVE_VERSION_FILE) {
        Ok(metadata) if metadata.is_file() && !is_cap_reparse_point(&metadata) => {
            let bytes = read_regular_file(product_root, ACTIVE_VERSION_FILE, MAX_ACTIVE_VERSION_BYTES)?;
            serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|_| BootstrapperError("active_version_record_invalid"))
        }
        Ok(_) => Err(BootstrapperError("active_version_file_invalid")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(BootstrapperError("active_version_file_invalid")),
    }
}

fn descriptor_from_record(record: &ActiveVersionRecord) -> ActiveVersionDescriptor {
    ActiveVersionDescriptor {
        product_version: record.product_version.clone(),
        package_sha256: record.package_sha256.clone(),
        launcher_sha256: record.launcher_sha256.clone(),
        product_manifest_sha256: record.product_manifest_sha256.clone(),
        update_id: record.update_id.clone(),
        candidate_hash: record.candidate_hash.clone(),
    }
}

fn valid_record(record: &ActiveVersionRecord, app_id: &str, audit_store_id: &str) -> bool {
    record.version == 1
        && record.product == "D4 Pocket"
        && record.app_id == app_id
        && record.audit_store_id == audit_store_id
        && valid_descriptor(&descriptor_from_record(record))
        && record.previous.as_ref().is_none_or(valid_bound_descriptor)
}

fn valid_descriptor(descriptor: &ActiveVersionDescriptor) -> bool {
    valid_product_version(&descriptor.product_version)
        && valid_sha256(&descriptor.package_sha256)
        && valid_sha256(&descriptor.launcher_sha256)
        && valid_sha256(&descriptor.product_manifest_sha256)
        && ((descriptor.update_id.is_empty() && descriptor.candidate_hash.is_empty())
            || valid_bound_descriptor(descriptor))
}

fn valid_bound_descriptor(descriptor: &ActiveVersionDescriptor) -> bool {
    valid_update_id(&descriptor.update_id) && valid_candidate_hash(&descriptor.candidate_hash)
}

fn validate_descriptor_stage(
    versions: &Dir,
    descriptor: &ActiveVersionDescriptor,
    root_device: u64,
) -> Result<(), BootstrapperError> {
    if !valid_descriptor(descriptor) {
        return Err(BootstrapperError("active_version_record_invalid"));
    }
    let stage_name = format!(
        "{}-{}",
        descriptor.product_version, descriptor.package_sha256
    );
    let stage = open_existing_directory(versions, &stage_name, root_device)?;
    if hash_regular_file(&stage, VERSIONED_LAUNCHER, MAX_LAUNCHER_BYTES)?
        != descriptor.launcher_sha256
        || hash_regular_file(&stage, PRODUCT_MANIFEST, MAX_PRODUCT_MANIFEST_BYTES)?
            != descriptor.product_manifest_sha256
    {
        return Err(BootstrapperError("active_version_payload_mismatch"));
    }
    Ok(())
}

fn valid_update_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn valid_candidate_hash(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
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
    let mut limited = Read::by_ref(&mut file).take(max_bytes.saturating_add(1));
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
    use super::{
        activate_staged_version, active_version_snapshot, is_active_version_launcher,
        resolve_active_version_launcher, toggle_previous_active_version, ACTIVE_VERSION_FILE,
        VERSIONED_LAUNCHER,
    };
    use cap_std::fs::Dir;
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
        assert!(is_active_version_launcher(
            &result, &local, APP_ID, AUDIT_ID
        ));
        assert!(!is_active_version_launcher(
            &root.join(VERSIONED_LAUNCHER),
            &local,
            APP_ID,
            AUDIT_ID
        ));
        assert!(!is_active_version_launcher(
            &result,
            &local,
            APP_ID,
            "other-audit-store"
        ));
        let unrelated_launcher = local.join("unrelated-launcher.exe");
        fs::write(&unrelated_launcher, b"not the selected product version").unwrap();
        assert!(!is_active_version_launcher(
            &unrelated_launcher,
            &local,
            APP_ID,
            AUDIT_ID
        ));
        fs::remove_dir_all(local).unwrap();
    }

    #[test]
    fn broker_activation_publishes_record_atomically_and_places_root_bootstrapper_once() {
        let (local, root) = fixture("activate-first");
        fs::remove_file(root.join(ACTIVE_VERSION_FILE)).unwrap();
        fs::remove_file(root.join(VERSIONED_LAUNCHER)).unwrap();
        let root_dir = Dir::open_ambient_dir(&root, cap_std::ambient_authority()).unwrap();
        let versions_dir =
            Dir::open_ambient_dir(root.join("versions"), cap_std::ambient_authority()).unwrap();
        let package_hash = "a".repeat(64);

        activate_staged_version(
            &root_dir,
            &versions_dir,
            "1.2.3",
            APP_ID,
            AUDIT_ID,
            &package_hash,
            "update-current",
            &format!("sha256:{}", "c".repeat(64)),
            "0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        assert_eq!(
            resolve_active_version_launcher(&local, APP_ID, AUDIT_ID).unwrap(),
            fs::canonicalize(
                root.join("versions")
                    .join(format!("1.2.3-{package_hash}"))
                    .join(VERSIONED_LAUNCHER)
            )
            .unwrap()
        );
        assert_eq!(
            fs::read(root.join(VERSIONED_LAUNCHER)).unwrap(),
            b"fixed version launcher"
        );
        assert!(!root
            .join("active_version.0123456789abcdef0123456789abcdef.tmp")
            .exists());

        drop(versions_dir);
        drop(root_dir);
        fs::remove_dir_all(local).unwrap();
    }

    #[test]
    fn broker_activation_preserves_existing_bootstrapper_and_replaces_active_record() {
        let (local, root) = fixture("activate-replace");
        let root_dir = Dir::open_ambient_dir(&root, cap_std::ambient_authority()).unwrap();
        let versions_dir =
            Dir::open_ambient_dir(root.join("versions"), cap_std::ambient_authority()).unwrap();
        activate_staged_version(
            &root_dir,
            &versions_dir,
            "1.2.3",
            APP_ID,
            AUDIT_ID,
            &"a".repeat(64),
            "update-current",
            &format!("sha256:{}", "c".repeat(64)),
            "fedcba9876543210fedcba9876543210",
        )
        .unwrap();
        assert_eq!(
            fs::read(root.join(VERSIONED_LAUNCHER)).unwrap(),
            b"bootstrapper launcher"
        );
        resolve_active_version_launcher(&local, APP_ID, AUDIT_ID).unwrap();
        assert!(!root
            .join("active_version.fedcba9876543210fedcba9876543210.tmp")
            .exists());

        drop(versions_dir);
        drop(root_dir);
        fs::remove_dir_all(local).unwrap();
    }

    #[test]
    fn broker_can_atomically_toggle_between_current_and_previous_verified_stages() {
        let (local, root) = fixture("rollback-toggle");
        let root_dir = Dir::open_ambient_dir(&root, cap_std::ambient_authority()).unwrap();
        let versions_path = root.join("versions");
        let versions_dir = Dir::open_ambient_dir(&versions_path, cap_std::ambient_authority())
            .unwrap();
        let old_package_hash = "a".repeat(64);
        let old_candidate_hash = format!("sha256:{}", "b".repeat(64));
        activate_staged_version(
            &root_dir,
            &versions_dir,
            "1.2.3",
            APP_ID,
            AUDIT_ID,
            &old_package_hash,
            "update-old",
            &old_candidate_hash,
            "11111111111111111111111111111111",
        )
        .unwrap();

        let new_package_hash = "d".repeat(64);
        let new_stage_path = versions_path.join(format!("2.0.0-{new_package_hash}"));
        fs::create_dir(&new_stage_path).unwrap();
        fs::write(
            new_stage_path.join(VERSIONED_LAUNCHER),
            b"new version launcher",
        )
        .unwrap();
        fs::write(
            new_stage_path.join("product_manifest.json"),
            b"new version manifest",
        )
        .unwrap();
        let new_candidate_hash = format!("sha256:{}", "e".repeat(64));
        activate_staged_version(
            &root_dir,
            &versions_dir,
            "2.0.0",
            APP_ID,
            AUDIT_ID,
            &new_package_hash,
            "update-new",
            &new_candidate_hash,
            "22222222222222222222222222222222",
        )
        .unwrap();
        let active = active_version_snapshot(&local, APP_ID, AUDIT_ID).unwrap();
        assert_eq!(active.current.product_version, "2.0.0");
        assert_eq!(active.previous.as_ref().unwrap().product_version, "1.2.3");

        let stale_target = toggle_previous_active_version(
            &root_dir,
            &versions_dir,
            APP_ID,
            AUDIT_ID,
            "update-new",
            &new_candidate_hash,
            "attacker-selected-update",
            &old_candidate_hash,
            "55555555555555555555555555555555",
        )
        .unwrap_err();
        assert_eq!(stale_target.0, "active_version_rollback_target_stale");
        assert_eq!(
            active_version_snapshot(&local, APP_ID, AUDIT_ID)
                .unwrap()
                .current
                .update_id,
            "update-new"
        );

        let old_stage = versions_path.join(format!("1.2.3-{old_package_hash}"));
        let old_manifest = old_stage.join("product_manifest.json");
        fs::write(&old_manifest, b"tampered manifest").unwrap();
        let tampered_target = toggle_previous_active_version(
            &root_dir,
            &versions_dir,
            APP_ID,
            AUDIT_ID,
            "update-new",
            &new_candidate_hash,
            "update-old",
            &old_candidate_hash,
            "66666666666666666666666666666666",
        )
        .unwrap_err();
        assert_eq!(tampered_target.0, "active_version_payload_mismatch");
        assert_eq!(
            active_version_snapshot(&local, APP_ID, AUDIT_ID)
                .unwrap_err()
                .0,
            "active_version_payload_mismatch"
        );
        fs::write(&old_manifest, b"fixed product manifest").unwrap();

        let restored = toggle_previous_active_version(
            &root_dir,
            &versions_dir,
            APP_ID,
            AUDIT_ID,
            "update-new",
            &new_candidate_hash,
            "update-old",
            &old_candidate_hash,
            "33333333333333333333333333333333",
        )
        .unwrap();
        assert_eq!(restored.product_version, "1.2.3");
        assert_eq!(
            active_version_snapshot(&local, APP_ID, AUDIT_ID)
                .unwrap()
                .current
                .update_id,
            "update-old"
        );

        toggle_previous_active_version(
            &root_dir,
            &versions_dir,
            APP_ID,
            AUDIT_ID,
            "update-old",
            &old_candidate_hash,
            "update-new",
            &new_candidate_hash,
            "44444444444444444444444444444444",
        )
        .unwrap();
        assert_eq!(
            active_version_snapshot(&local, APP_ID, AUDIT_ID)
                .unwrap()
                .current
                .update_id,
            "update-new"
        );
        drop(versions_dir);
        drop(root_dir);
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
