//! D4 Pocket の無圧縮・固定順 package reader。
//!
//! package の外側digest／署名はBrokerが検証する。本moduleはpackage構造、製品identity、
//! file一覧、各file hashを再検査して新しい一時directoryへ展開する。Authorityを生成しない。

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
#[cfg(windows)]
use cap_std::fs::MetadataExt as CapMetadataExt;
use cap_std::fs::{Dir, OpenOptions};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path};

const MAGIC: &[u8; 8] = b"D4PKG01\n";
const MAX_MANIFEST_BYTES: u32 = 1024 * 1024;
const MAX_PRODUCT_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_PACKAGE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_FILES: usize = 20_000;
const COPY_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductPackageInfo {
    pub product_version: String,
    pub app_id: String,
    pub audit_store_id: String,
    pub file_count: usize,
    pub total_file_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductPackageExpectation<'a> {
    pub product_version: Option<&'a str>,
    pub app_id: &'a str,
    pub audit_store_id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductPackageError(pub &'static str);

impl std::fmt::Display for ProductPackageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for ProductPackageError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StageMode {
    Create,
    Resume,
    Verify,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageManifest {
    version: u32,
    product: String,
    product_version: String,
    app_id: String,
    audit_store_id: String,
    files: Vec<PackageFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageFile {
    path: String,
    byte_length: u64,
    sha256: String,
}

/// 外側packageの全fileを検証し、未存在のstage directoryだけへ展開する。
/// 失敗時は本関数が作成したstage directoryだけを片付ける。
pub fn extract_verified_package(
    package_path: &Path,
    destination: &Path,
    expected: ProductPackageExpectation<'_>,
) -> Result<ProductPackageInfo, ProductPackageError> {
    let metadata = fs::symlink_metadata(package_path).map_err(|_| err("package_file_missing"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(err("package_file_invalid"));
    }
    if metadata.len() < 12 || metadata.len() > MAX_PACKAGE_BYTES {
        return Err(err("package_size_invalid"));
    }
    let input = File::open(package_path).map_err(|_| err("package_file_open_failed"))?;
    extract_verified_package_from_reader(input, metadata.len(), None, destination, expected)
}

/// Brokerがcapability directoryから開いた同一file handleを検証・展開する。
/// 期待digestを指定した場合、全package byteの読取とstage作成が同一handleに束縛される。
pub(crate) fn extract_verified_package_from_reader<R: Read>(
    input: R,
    package_bytes: u64,
    expected_package_sha256: Option<&str>,
    destination: &Path,
    expected: ProductPackageExpectation<'_>,
) -> Result<ProductPackageInfo, ProductPackageError> {
    let parent = destination
        .parent()
        .ok_or_else(|| err("package_destination_invalid"))?;
    let parent_metadata =
        fs::symlink_metadata(parent).map_err(|_| err("package_parent_missing"))?;
    if !parent_metadata.is_dir()
        || parent_metadata.file_type().is_symlink()
        || is_reparse_point(&parent_metadata)
    {
        return Err(err("package_parent_invalid"));
    }
    let parent = fs::canonicalize(parent).map_err(|_| err("package_parent_invalid"))?;
    let destination_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| err("package_destination_invalid"))?;
    let parent = Dir::open_ambient_dir(&parent, cap_std::ambient_authority())
        .map_err(|_| err("package_parent_invalid"))?;
    extract_verified_package_into_directory(
        input,
        package_bytes,
        expected_package_sha256,
        &parent,
        destination_name,
        expected,
    )
}

/// Brokerが所有するdirectory capabilityの直下にだけpackageを展開する。
/// 途中失敗時は開いたstage handle自身を除去し、ambient pathへ戻らない。
pub(crate) fn extract_verified_package_into_directory<R: Read>(
    input: R,
    package_bytes: u64,
    expected_package_sha256: Option<&str>,
    parent: &Dir,
    stage_name: &str,
    expected: ProductPackageExpectation<'_>,
) -> Result<ProductPackageInfo, ProductPackageError> {
    extract_verified_package_into_directory_mode(
        input,
        package_bytes,
        expected_package_sha256,
        parent,
        stage_name,
        expected,
        StageMode::Create,
    )
    .map(|(info, _)| info)
}

/// 既存stageがある場合は署名packageと一致するprefix fileだけを再利用して展開を続ける。
/// 不一致fileやpackage外entryは上書き・削除せず、fail-closedで保持する。
pub(crate) fn resume_verified_package_into_directory<R: Read>(
    input: R,
    package_bytes: u64,
    expected_package_sha256: Option<&str>,
    parent: &Dir,
    stage_name: &str,
    expected: ProductPackageExpectation<'_>,
) -> Result<(ProductPackageInfo, bool), ProductPackageError> {
    extract_verified_package_into_directory_mode(
        input,
        package_bytes,
        expected_package_sha256,
        parent,
        stage_name,
        expected,
        StageMode::Resume,
    )
}

/// 既存stageを変更せず、署名packageと全file・directory inventoryが一致することだけを検証する。
pub(crate) fn verify_staged_package_into_directory<R: Read>(
    input: R,
    package_bytes: u64,
    expected_package_sha256: Option<&str>,
    parent: &Dir,
    stage_name: &str,
    expected: ProductPackageExpectation<'_>,
) -> Result<ProductPackageInfo, ProductPackageError> {
    extract_verified_package_into_directory_mode(
        input,
        package_bytes,
        expected_package_sha256,
        parent,
        stage_name,
        expected,
        StageMode::Verify,
    )
    .map(|(info, _)| info)
}

fn extract_verified_package_into_directory_mode<R: Read>(
    input: R,
    package_bytes: u64,
    expected_package_sha256: Option<&str>,
    parent: &Dir,
    stage_name: &str,
    expected: ProductPackageExpectation<'_>,
    mode: StageMode,
) -> Result<(ProductPackageInfo, bool), ProductPackageError> {
    if !(12..=MAX_PACKAGE_BYTES).contains(&package_bytes) {
        return Err(err("package_size_invalid"));
    }
    if expected_package_sha256.is_some_and(|digest| !valid_sha256(digest)) {
        return Err(err("package_outer_hash_invalid"));
    }
    validate_stage_name(stage_name)?;
    let parent_metadata = parent
        .dir_metadata()
        .map_err(|_| err("package_parent_invalid"))?;
    if !parent_metadata.is_dir() || is_cap_reparse_point(&parent_metadata) {
        return Err(err("package_parent_invalid"));
    }
    let (stage, created_stage, stage_entry_metadata) = match parent.symlink_metadata(stage_name) {
        Ok(existing_metadata) if mode != StageMode::Create => {
            if !existing_metadata.is_dir() || is_cap_reparse_point(&existing_metadata) {
                return Err(err("package_stage_invalid"));
            }
            let stage = parent
                .open_dir_nofollow(stage_name)
                .map_err(|_| err("package_stage_invalid"))?;
            let opened_metadata = stage
                .dir_metadata()
                .map_err(|_| err("package_stage_invalid"))?;
            if !opened_metadata.is_dir()
                || is_cap_reparse_point(&opened_metadata)
                || !same_cap_file(&existing_metadata, &opened_metadata)
            {
                return Err(err("package_stage_changed"));
            }
            (stage, false, existing_metadata)
        }
        Ok(_) => return Err(err("package_destination_exists")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if mode == StageMode::Verify {
                return Err(err("package_stage_unavailable"));
            }
            parent
                .create_dir(stage_name)
                .map_err(|_| err("package_stage_create_failed"))?;
            let created_metadata = parent
                .symlink_metadata(stage_name)
                .map_err(|_| err("package_stage_invalid"))?;
            if !created_metadata.is_dir() || is_cap_reparse_point(&created_metadata) {
                return Err(err("package_stage_invalid"));
            }
            let stage = parent
                .open_dir_nofollow(stage_name)
                .map_err(|_| err("package_stage_invalid"))?;
            let opened_metadata = stage
                .dir_metadata()
                .map_err(|_| err("package_stage_invalid"))?;
            if !same_cap_file(&created_metadata, &opened_metadata) {
                return Err(err("package_stage_changed"));
            }
            (stage, true, created_metadata)
        }
        Err(_) => return Err(err("package_destination_invalid")),
    };
    let stage_metadata = match stage.dir_metadata() {
        Ok(metadata) => metadata,
        Err(_) => {
            if created_stage {
                let _ = stage.remove_open_dir_all();
            }
            return Err(err("package_stage_invalid"));
        }
    };
    if !same_cap_file(&stage_entry_metadata, &stage_metadata) {
        if created_stage {
            let _ = stage.remove_open_dir_all();
        }
        return Err(err("package_stage_changed"));
    }
    if !stage_metadata.is_dir() || is_cap_reparse_point(&stage_metadata) {
        if created_stage {
            let _ = stage.remove_open_dir_all();
        }
        return Err(err("package_stage_invalid"));
    }

    let mut input = PackageHashReader::new(input);
    let result =
        extract_inner(&mut input, &stage, &expected, package_bytes, mode).and_then(|info| {
            if expected_package_sha256.is_some_and(|expected| input.finish() != expected) {
                return Err(err("package_outer_hash_mismatch"));
            }
            Ok(info)
        });
    if result.is_err() {
        if created_stage {
            let _ = stage.remove_open_dir_all();
        }
    }
    result.map(|info| (info, mode == StageMode::Resume && !created_stage))
}

struct PackageHashReader<R> {
    inner: R,
    digest: Sha256,
}

impl<R> PackageHashReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            digest: Sha256::new(),
        }
    }

    fn finish(self) -> String {
        hex::encode(self.digest.finalize())
    }
}

impl<R: Read> Read for PackageHashReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let count = self.inner.read(buffer)?;
        self.digest.update(&buffer[..count]);
        Ok(count)
    }
}

fn extract_inner<R: Read>(
    input: &mut R,
    destination: &Dir,
    expected: &ProductPackageExpectation<'_>,
    package_bytes: u64,
    mode: StageMode,
) -> Result<ProductPackageInfo, ProductPackageError> {
    let mut magic = [0u8; 8];
    input
        .read_exact(&mut magic)
        .map_err(|_| err("package_header_invalid"))?;
    if &magic != MAGIC {
        return Err(err("package_magic_invalid"));
    }
    let mut length = [0u8; 4];
    input
        .read_exact(&mut length)
        .map_err(|_| err("package_header_invalid"))?;
    let manifest_bytes = u32::from_le_bytes(length);
    if manifest_bytes == 0 || manifest_bytes > MAX_MANIFEST_BYTES {
        return Err(err("package_manifest_size_invalid"));
    }
    let mut raw_manifest = vec![0u8; manifest_bytes as usize];
    input
        .read_exact(&mut raw_manifest)
        .map_err(|_| err("package_manifest_truncated"))?;
    let manifest: PackageManifest =
        serde_json::from_slice(&raw_manifest).map_err(|_| err("package_manifest_invalid"))?;
    validate_manifest(&manifest, expected)?;
    let total_file_bytes = manifest.files.iter().try_fold(0u64, |sum, file| {
        sum.checked_add(file.byte_length)
            .ok_or_else(|| err("package_total_size_overflow"))
    })?;
    if 12u64
        .checked_add(manifest_bytes as u64)
        .and_then(|value| value.checked_add(total_file_bytes))
        != Some(package_bytes)
    {
        return Err(err("package_length_mismatch"));
    }

    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
    let mut existing_buffer = vec![0u8; COPY_BUFFER_BYTES];
    let mut product_manifest_validated = false;
    for entry in &manifest.files {
        if entry.path == "product_manifest.json" && entry.byte_length > MAX_PRODUCT_MANIFEST_BYTES {
            return Err(err("package_product_manifest_size_invalid"));
        }
        let (parent, file_name) = open_package_parent(destination, &entry.path)?;
        let (mut output, existing_prefix_bytes) = match mode {
            StageMode::Resume => {
                let mut options = OpenOptions::new();
                options.read(true).write(true).follow(FollowSymlinks::No);
                match parent.open_with(file_name, &options) {
                    Ok(file) => {
                        let metadata = file
                            .metadata()
                            .map_err(|_| err("package_stage_existing_file_invalid"))?;
                        if !metadata.is_file()
                            || is_cap_reparse_point(&metadata)
                            || metadata.len() > entry.byte_length
                        {
                            return Err(err("package_stage_existing_file_invalid"));
                        }
                        (file, metadata.len())
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        let mut options = OpenOptions::new();
                        options
                            .write(true)
                            .create_new(true)
                            .follow(FollowSymlinks::No);
                        (
                            parent
                                .open_with(file_name, &options)
                                .map_err(|_| err("package_file_create_failed"))?,
                            0,
                        )
                    }
                    Err(_) => return Err(err("package_stage_existing_file_invalid")),
                }
            }
            StageMode::Verify => {
                let mut options = OpenOptions::new();
                options.read(true).follow(FollowSymlinks::No);
                let file = parent
                    .open_with(file_name, &options)
                    .map_err(|_| err("package_stage_existing_file_invalid"))?;
                let metadata = file
                    .metadata()
                    .map_err(|_| err("package_stage_existing_file_invalid"))?;
                if !metadata.is_file()
                    || is_cap_reparse_point(&metadata)
                    || metadata.len() != entry.byte_length
                {
                    return Err(err("package_stage_existing_file_invalid"));
                }
                (file, metadata.len())
            }
            StageMode::Create => {
                let mut options = OpenOptions::new();
                options
                    .write(true)
                    .create_new(true)
                    .follow(FollowSymlinks::No);
                (
                    parent
                        .open_with(file_name, &options)
                        .map_err(|_| err("package_file_create_failed"))?,
                    0,
                )
            }
        };
        let mut remaining = entry.byte_length;
        let mut consumed = 0u64;
        let mut digest = Sha256::new();
        let mut captured_manifest = (entry.path == "product_manifest.json")
            .then(|| Vec::with_capacity(entry.byte_length as usize));
        while remaining > 0 {
            let capacity = usize::try_from(remaining.min(buffer.len() as u64))
                .map_err(|_| err("package_length_invalid"))?;
            let count = input
                .read(&mut buffer[..capacity])
                .map_err(|_| err("package_file_read_failed"))?;
            if count == 0 {
                return Err(err("package_file_truncated"));
            }
            let prefix_count = usize::try_from(
                existing_prefix_bytes
                    .saturating_sub(consumed)
                    .min(count as u64),
            )
            .map_err(|_| err("package_length_invalid"))?;
            if prefix_count > 0 {
                output
                    .read_exact(&mut existing_buffer[..prefix_count])
                    .map_err(|_| err("package_stage_existing_file_invalid"))?;
                if existing_buffer[..prefix_count] != buffer[..prefix_count] {
                    return Err(err("package_stage_existing_file_mismatch"));
                }
            }
            if prefix_count < count {
                if mode == StageMode::Verify {
                    return Err(err("package_stage_existing_file_invalid"));
                }
                output
                    .write_all(&buffer[prefix_count..count])
                    .map_err(|_| err("package_file_write_failed"))?;
            }
            digest.update(&buffer[..count]);
            if let Some(captured) = captured_manifest.as_mut() {
                captured.extend_from_slice(&buffer[..count]);
            }
            remaining -= count as u64;
            consumed += count as u64;
        }
        if mode != StageMode::Verify {
            output
                .sync_all()
                .map_err(|_| err("package_file_sync_failed"))?;
        }
        if hex::encode(digest.finalize()) != entry.sha256 {
            return Err(err("package_file_hash_mismatch"));
        }
        if let Some(captured) = captured_manifest {
            validate_product_manifest(&captured, expected)?;
            product_manifest_validated = true;
        }
    }
    if !product_manifest_validated {
        return Err(err("package_required_file_missing"));
    }
    if mode != StageMode::Create {
        validate_stage_tree(destination, &manifest.files)?;
    }
    let mut trailing = [0u8; 1];
    if input
        .read(&mut trailing)
        .map_err(|_| err("package_read_failed"))?
        != 0
    {
        return Err(err("package_trailing_bytes"));
    }
    Ok(ProductPackageInfo {
        product_version: manifest.product_version,
        app_id: manifest.app_id,
        audit_store_id: manifest.audit_store_id,
        file_count: manifest.files.len(),
        total_file_bytes,
    })
}

fn validate_stage_tree(
    root: &Dir,
    package_files: &[PackageFile],
) -> Result<(), ProductPackageError> {
    let mut expected_files = BTreeSet::new();
    let mut expected_directories = BTreeSet::new();
    for file in package_files {
        expected_files.insert(file.path.clone());
        let mut prefix = String::new();
        let parents: Vec<&str> = file.path.split('/').collect();
        for component in parents.iter().take(parents.len().saturating_sub(1)) {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(component);
            expected_directories.insert(prefix.clone());
        }
    }
    let mut found_files = BTreeSet::new();
    let mut found_directories = BTreeSet::new();
    collect_stage_tree(root, "", &mut found_files, &mut found_directories)?;
    if found_files != expected_files || found_directories != expected_directories {
        return Err(err("package_stage_unexpected_entry"));
    }
    Ok(())
}

fn collect_stage_tree(
    directory: &Dir,
    prefix: &str,
    files: &mut BTreeSet<String>,
    directories: &mut BTreeSet<String>,
) -> Result<(), ProductPackageError> {
    let mut pending = vec![(
        directory
            .try_clone()
            .map_err(|_| err("package_stage_unreadable"))?,
        prefix.to_owned(),
    )];
    while let Some((current, current_prefix)) = pending.pop() {
        for entry in current
            .read_dir(".")
            .map_err(|_| err("package_stage_unreadable"))?
        {
            let entry = entry.map_err(|_| err("package_stage_unreadable"))?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| err("package_stage_entry_invalid"))?;
            let relative = if current_prefix.is_empty() {
                name.clone()
            } else {
                format!("{current_prefix}/{name}")
            };
            let metadata = current
                .symlink_metadata(&name)
                .map_err(|_| err("package_stage_entry_invalid"))?;
            if is_cap_reparse_point(&metadata) {
                return Err(err("package_stage_reparse_point"));
            }
            if metadata.is_dir() {
                directories.insert(relative.clone());
                let child = current
                    .open_dir_nofollow(&name)
                    .map_err(|_| err("package_stage_entry_invalid"))?;
                pending.push((child, relative));
            } else if metadata.is_file() {
                files.insert(relative);
            } else {
                return Err(err("package_stage_entry_invalid"));
            }
        }
    }
    Ok(())
}

fn validate_product_manifest(
    bytes: &[u8],
    expected: &ProductPackageExpectation<'_>,
) -> Result<(), ProductPackageError> {
    let raw = std::str::from_utf8(bytes).map_err(|_| err("package_product_manifest_invalid"))?;
    let value: serde_json::Value = crate::broker::json_input::read_unique(raw)
        .map_err(|_| err("package_product_manifest_invalid"))?;
    if value.get("version").and_then(serde_json::Value::as_u64) != Some(1)
        || value.get("product").and_then(serde_json::Value::as_str) != Some("D4 Pocket")
        || !value
            .get("export_id")
            .and_then(serde_json::Value::as_str)
            .is_some_and(valid_export_id)
    {
        return Err(err("package_product_manifest_identity_invalid"));
    }
    let product = value
        .get("manifest")
        .ok_or_else(|| err("package_product_manifest_invalid"))?;
    let app_id = product
        .get("app_identity")
        .and_then(|identity| identity.get("app_id"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| err("package_product_manifest_invalid"))?;
    let audit = product
        .get("audit_store")
        .ok_or_else(|| err("package_product_manifest_invalid"))?;
    let audit_store_id = audit
        .get("store_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| err("package_product_manifest_invalid"))?;
    let inheritance = product
        .get("inheritance_policy")
        .ok_or_else(|| err("package_product_manifest_invalid"))?;
    if app_id != expected.app_id
        || audit_store_id != expected.audit_store_id
        || audit
            .get("chain_status")
            .and_then(serde_json::Value::as_str)
            != Some("new")
        || audit.get("inherited").and_then(serde_json::Value::as_bool) != Some(false)
        || [
            "authority",
            "permission",
            "approval",
            "credential",
            "audit_chain",
        ]
        .iter()
        .any(|key| inheritance.get(key).and_then(serde_json::Value::as_str) != Some("none"))
    {
        return Err(err("package_product_manifest_identity_mismatch"));
    }
    Ok(())
}

fn validate_manifest(
    manifest: &PackageManifest,
    expected: &ProductPackageExpectation<'_>,
) -> Result<(), ProductPackageError> {
    if manifest.version != 1 || manifest.product != "D4 Pocket" {
        return Err(err("package_identity_invalid"));
    }
    if !valid_id(&manifest.app_id, "d4-pocket-app-")
        || !valid_id(&manifest.audit_store_id, "audit-store-")
        || manifest.app_id != expected.app_id
        || manifest.audit_store_id != expected.audit_store_id
        || expected
            .product_version
            .is_some_and(|version| version != manifest.product_version)
    {
        return Err(err("package_identity_mismatch"));
    }
    if !valid_product_version(&manifest.product_version)
        || manifest.files.is_empty()
        || manifest.files.len() > MAX_FILES
    {
        return Err(err("package_manifest_invalid"));
    }
    let mut seen = BTreeSet::new();
    let mut required = BTreeSet::from([
        "app/gui_shell_desktop.exe",
        "app/flutter_windows.dll",
        "app/data/app.so",
        "app/data/icudtl.dat",
        "broker/gui_shell_rust_helper.exe",
        "gui_shell_desktop_launcher.exe",
        "product_manifest.json",
    ]);
    let mut previous = String::new();
    for file in &manifest.files {
        validate_path(&file.path)?;
        let key = file.path.to_ascii_lowercase();
        if !seen.insert(key.clone()) || (!previous.is_empty() && key <= previous) {
            return Err(err("package_file_order_or_duplicate"));
        }
        previous = key.clone();
        if file.byte_length > MAX_PACKAGE_BYTES || !valid_sha256(&file.sha256) {
            return Err(err("package_file_metadata_invalid"));
        }
        required.remove(key.as_str());
    }
    if !required.is_empty() {
        return Err(err("package_required_file_missing"));
    }
    Ok(())
}

fn valid_id(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
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

fn valid_export_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn validate_path(value: &str) -> Result<(), ProductPackageError> {
    if value.is_empty() || !value.is_ascii() || value.contains('\\') || value.contains(':') {
        return Err(err("package_path_invalid"));
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(err("package_path_invalid"));
    }
    let components: Vec<&str> = value.split('/').collect();
    if components.is_empty()
        || components.iter().any(|part| {
            part.is_empty()
                || *part == "."
                || *part == ".."
                || part.ends_with('.')
                || part.ends_with(' ')
                || part
                    .bytes()
                    .any(|byte| byte < 0x20 || b"<>\"|?*".contains(&byte))
                || is_reserved_windows_component(part)
        })
    {
        return Err(err("package_path_invalid"));
    }
    let allowed_root = matches!(components[0], "app" | "broker")
        || (components.len() == 1
            && matches!(
                components[0],
                "gui_shell_desktop_launcher.exe" | "product_manifest.json"
            ));
    if !allowed_root {
        return Err(err("package_path_outside_payload"));
    }
    Ok(())
}

fn is_reserved_windows_component(value: &str) -> bool {
    let stem = value
        .split('.')
        .next()
        .unwrap_or(value)
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

fn validate_stage_name(value: &str) -> Result<(), ProductPackageError> {
    if value.is_empty()
        || value.len() > 160
        || value == "."
        || value == ".."
        || value.ends_with('.')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        || is_reserved_windows_component(value)
    {
        return Err(err("package_stage_name_invalid"));
    }
    Ok(())
}

fn open_package_parent<'a>(
    root: &Dir,
    relative: &'a str,
) -> Result<(Dir, &'a str), ProductPackageError> {
    validate_path(relative)?;
    let mut components = relative.split('/').peekable();
    let mut current = root
        .try_clone()
        .map_err(|_| err("package_directory_invalid"))?;
    while let Some(component) = components.next() {
        if components.peek().is_none() {
            return Ok((current, component));
        }
        match current.open_dir_nofollow(component) {
            Ok(next) => current = next,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                current
                    .create_dir(component)
                    .map_err(|_| err("package_directory_create_failed"))?;
                current = current
                    .open_dir_nofollow(component)
                    .map_err(|_| err("package_directory_invalid"))?;
            }
            Err(_) => return Err(err("package_path_reparse_or_conflict")),
        }
        let metadata = current
            .dir_metadata()
            .map_err(|_| err("package_directory_invalid"))?;
        if !metadata.is_dir() || is_cap_reparse_point(&metadata) {
            return Err(err("package_path_reparse_or_conflict"));
        }
    }
    Err(err("package_path_invalid"))
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
    let left_id = cap_fs_ext::MetadataExt::ino(left);
    left_id != 0
        && cap_fs_ext::MetadataExt::dev(left) == cap_fs_ext::MetadataExt::dev(right)
        && left_id == cap_fs_ext::MetadataExt::ino(right)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_: &fs::Metadata) -> bool {
    false
}

fn err(code: &'static str) -> ProductPackageError {
    ProductPackageError(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
    use serde_json::json;
    use std::path::PathBuf;

    const APP: &str = "d4-pocket-app-11111111111111111111111111111111";
    const AUDIT: &str = "audit-store-22222222222222222222222222222222";

    const PRODUCT_MANIFEST: &[u8] = br#"{"version":1,"product":"D4 Pocket","export_id":"export-test","manifest":{"app_identity":{"app_id":"d4-pocket-app-11111111111111111111111111111111"},"audit_store":{"store_id":"audit-store-22222222222222222222222222222222","chain_status":"new","inherited":false},"inheritance_policy":{"authority":"none","permission":"none","approval":"none","credential":"none","audit_chain":"none"}}}"#;

    fn package_with(files: Vec<(&str, &[u8])>) -> Vec<u8> {
        let entries: Vec<_> = files
            .iter()
            .map(|(path, bytes)| json!({"path": path, "byte_length": bytes.len(), "sha256": hex::encode(Sha256::digest(bytes))}))
            .collect();
        let manifest = json!({"version": 1, "product": "D4 Pocket", "product_version": "1.2.3", "app_id": APP, "audit_store_id": AUDIT, "files": entries});
        let raw = serde_json::to_vec(&manifest).unwrap();
        let mut output = MAGIC.to_vec();
        output.extend_from_slice(&(raw.len() as u32).to_le_bytes());
        output.extend_from_slice(&raw);
        for (_, bytes) in files {
            output.extend_from_slice(bytes);
        }
        output
    }

    fn files_for_test() -> Vec<(&'static str, &'static [u8])> {
        vec![
            ("app/data/app.so", b"a"),
            ("app/data/icudtl.dat", b"b"),
            ("app/flutter_windows.dll", b"c"),
            ("app/gui_shell_desktop.exe", b"d"),
            ("broker/gui_shell_rust_helper.exe", b"e"),
            ("gui_shell_desktop_launcher.exe", b"f"),
            ("product_manifest.json", PRODUCT_MANIFEST),
        ]
    }

    fn write_temp(bytes: &[u8], name: &str) -> PathBuf {
        let mut random = [0u8; 8];
        let unique = if getrandom::getrandom(&mut random).is_ok() {
            hex::encode(random)
        } else {
            std::process::id().to_string()
        };
        let directory = std::env::temp_dir().join(format!("d4p-product-package-{name}-{unique}"));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("package.pkg");
        fs::write(&path, bytes).unwrap();
        path
    }

    fn expectation<'a>(version: Option<&'a str>) -> ProductPackageExpectation<'a> {
        ProductPackageExpectation {
            product_version: version,
            app_id: APP,
            audit_store_id: AUDIT,
        }
    }

    #[test]
    fn verified_package_extracts_only_manifested_files() {
        let files = files_for_test();
        let bytes = package_with(files.clone());
        let path = write_temp(&bytes, "valid");
        let destination = path.parent().unwrap().join("stage");
        let info =
            extract_verified_package(&path, &destination, expectation(Some("1.2.3"))).unwrap();
        assert_eq!(info.file_count, 7);
        assert_eq!(info.total_file_bytes, PRODUCT_MANIFEST.len() as u64 + 6);
        assert_eq!(
            fs::read(destination.join("app/gui_shell_desktop.exe")).unwrap(),
            b"d"
        );
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 4);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn broker_reader_binds_outer_digest_and_stage_to_one_open_file() {
        let path = write_temp(&package_with(files_for_test()), "outer-digest");
        let package_bytes = fs::read(&path).unwrap();
        let package_sha256 = hex::encode(Sha256::digest(&package_bytes));
        let directory = cap_std::fs::Dir::open_ambient_dir(
            path.parent().unwrap(),
            cap_std::ambient_authority(),
        )
        .unwrap();
        let mut options = cap_std::fs::OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let file = directory.open_with("package.pkg", &options).unwrap();
        let length = file.metadata().unwrap().len();
        let info = extract_verified_package_into_directory(
            file,
            length,
            Some(&package_sha256),
            &directory,
            "stage-valid",
            expectation(Some("1.2.3")),
        )
        .unwrap();
        assert_eq!(info.product_version, "1.2.3");
        let destination = path.parent().unwrap().join("stage-valid");
        assert!(destination.join("product_manifest.json").is_file());

        let file = directory.open_with("package.pkg", &options).unwrap();
        assert_eq!(
            extract_verified_package_into_directory(
                file,
                length,
                Some(&"b".repeat(64)),
                &directory,
                "stage-bad-hash",
                expectation(Some("1.2.3")),
            )
            .unwrap_err()
            .0,
            "package_outer_hash_mismatch"
        );
        let rejected_destination = path.parent().unwrap().join("stage-bad-hash");
        assert!(!rejected_destination.exists());
        drop(directory);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn capability_extraction_rejects_non_component_stage_name_without_mutation() {
        let path = write_temp(&package_with(files_for_test()), "stage-name");
        let directory = cap_std::fs::Dir::open_ambient_dir(
            path.parent().unwrap(),
            cap_std::ambient_authority(),
        )
        .unwrap();
        let package = directory.open("package.pkg").unwrap();
        let length = package.metadata().unwrap().len();
        assert_eq!(
            extract_verified_package_into_directory(
                package,
                length,
                None,
                &directory,
                "../escaped",
                expectation(Some("1.2.3")),
            )
            .unwrap_err()
            .0,
            "package_stage_name_invalid"
        );
        assert!(!path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("escaped")
            .exists());
        drop(directory);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn capability_extraction_preserves_existing_stage_and_cleans_its_own_failed_stage() {
        let path = write_temp(&package_with(files_for_test()), "stage-collision");
        let directory = cap_std::fs::Dir::open_ambient_dir(
            path.parent().unwrap(),
            cap_std::ambient_authority(),
        )
        .unwrap();
        directory.create_dir("stage-existing").unwrap();
        fs::write(
            path.parent().unwrap().join("stage-existing/sentinel"),
            b"keep",
        )
        .unwrap();
        let package = directory.open("package.pkg").unwrap();
        let length = package.metadata().unwrap().len();
        assert_eq!(
            extract_verified_package_into_directory(
                package,
                length,
                None,
                &directory,
                "stage-existing",
                expectation(Some("1.2.3")),
            )
            .unwrap_err()
            .0,
            "package_destination_exists"
        );
        assert_eq!(
            fs::read(path.parent().unwrap().join("stage-existing/sentinel")).unwrap(),
            b"keep"
        );

        let mut conflicting_files = files_for_test();
        conflicting_files.insert(0, ("app", b"not a directory"));
        let bytes = package_with(conflicting_files);
        let conflicting_package = path.parent().unwrap().join("conflict.pkg");
        fs::write(&conflicting_package, &bytes).unwrap();
        let package = directory.open("conflict.pkg").unwrap();
        assert_eq!(
            extract_verified_package_into_directory(
                package,
                bytes.len() as u64,
                None,
                &directory,
                "stage-conflict",
                expectation(Some("1.2.3")),
            )
            .unwrap_err()
            .0,
            "package_path_reparse_or_conflict"
        );
        assert!(!path.parent().unwrap().join("stage-conflict").exists());
        drop(directory);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn capability_resume_completes_only_matching_partial_files_and_rejects_extras() {
        let path = write_temp(&package_with(files_for_test()), "stage-resume");
        let directory = cap_std::fs::Dir::open_ambient_dir(
            path.parent().unwrap(),
            cap_std::ambient_authority(),
        )
        .unwrap();
        let mut files = files_for_test();
        files[0] = ("app/data/app.so", b"signed-prefix-and-suffix");
        let bytes = package_with(files);
        let digest = hex::encode(Sha256::digest(&bytes));
        fs::create_dir_all(path.parent().unwrap().join("stage/app/data")).unwrap();
        fs::write(
            path.parent().unwrap().join("stage/app/data/app.so"),
            b"signed-prefix",
        )
        .unwrap();

        let (info, resumed) = resume_verified_package_into_directory(
            std::io::Cursor::new(bytes.clone()),
            bytes.len() as u64,
            Some(&digest),
            &directory,
            "stage",
            expectation(Some("1.2.3")),
        )
        .unwrap();
        assert!(resumed);
        assert_eq!(info.file_count, files_for_test().len());
        assert_eq!(
            fs::read(path.parent().unwrap().join("stage/app/data/app.so")).unwrap(),
            b"signed-prefix-and-suffix"
        );

        let (recovered, resumed) = resume_verified_package_into_directory(
            std::io::Cursor::new(bytes.clone()),
            bytes.len() as u64,
            Some(&digest),
            &directory,
            "stage",
            expectation(Some("1.2.3")),
        )
        .unwrap();
        assert!(resumed);
        assert_eq!(recovered, info);

        drop(directory);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();

        let path = write_temp(&package_with(files_for_test()), "stage-resume-mismatch");
        let directory = cap_std::fs::Dir::open_ambient_dir(
            path.parent().unwrap(),
            cap_std::ambient_authority(),
        )
        .unwrap();
        fs::create_dir_all(path.parent().unwrap().join("stage/app/data")).unwrap();
        fs::write(path.parent().unwrap().join("stage/app/data/app.so"), b"x").unwrap();
        let bytes = package_with(files_for_test());
        let digest = hex::encode(Sha256::digest(&bytes));
        assert_eq!(
            resume_verified_package_into_directory(
                std::io::Cursor::new(bytes.clone()),
                bytes.len() as u64,
                Some(&digest),
                &directory,
                "stage",
                expectation(Some("1.2.3")),
            )
            .unwrap_err()
            .0,
            "package_stage_existing_file_mismatch"
        );
        assert_eq!(
            fs::read(path.parent().unwrap().join("stage/app/data/app.so")).unwrap(),
            b"x"
        );

        fs::write(path.parent().unwrap().join("stage/app/data/app.so"), b"a").unwrap();
        fs::write(path.parent().unwrap().join("stage/unexpected.bin"), b"keep").unwrap();
        assert_eq!(
            resume_verified_package_into_directory(
                std::io::Cursor::new(bytes.clone()),
                bytes.len() as u64,
                Some(&digest),
                &directory,
                "stage",
                expectation(Some("1.2.3")),
            )
            .unwrap_err()
            .0,
            "package_stage_unexpected_entry"
        );
        assert_eq!(
            fs::read(path.parent().unwrap().join("stage/unexpected.bin")).unwrap(),
            b"keep"
        );
        drop(directory);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn staged_package_verification_is_read_only_and_rejects_content_or_inventory_changes() {
        let bytes = package_with(files_for_test());
        let digest = hex::encode(Sha256::digest(&bytes));
        let path = write_temp(&bytes, "stage-verify");
        let directory = cap_std::fs::Dir::open_ambient_dir(
            path.parent().unwrap(),
            cap_std::ambient_authority(),
        )
        .unwrap();
        extract_verified_package_into_directory(
            std::io::Cursor::new(bytes.clone()),
            bytes.len() as u64,
            Some(&digest),
            &directory,
            "stage",
            expectation(Some("1.2.3")),
        )
        .unwrap();
        let launcher = path
            .parent()
            .unwrap()
            .join("stage/gui_shell_desktop_launcher.exe");
        let original = fs::read(&launcher).unwrap();
        let modified = fs::metadata(&launcher).unwrap().modified().unwrap();

        let verified = verify_staged_package_into_directory(
            std::io::Cursor::new(bytes.clone()),
            bytes.len() as u64,
            Some(&digest),
            &directory,
            "stage",
            expectation(Some("1.2.3")),
        )
        .unwrap();
        assert_eq!(verified.file_count, files_for_test().len());
        assert_eq!(fs::read(&launcher).unwrap(), original);
        assert_eq!(
            fs::metadata(&launcher).unwrap().modified().unwrap(),
            modified
        );

        fs::write(&launcher, b"x").unwrap();
        assert_eq!(
            verify_staged_package_into_directory(
                std::io::Cursor::new(bytes.clone()),
                bytes.len() as u64,
                Some(&digest),
                &directory,
                "stage",
                expectation(Some("1.2.3")),
            )
            .unwrap_err()
            .0,
            "package_stage_existing_file_mismatch"
        );
        assert_eq!(fs::read(&launcher).unwrap(), b"x");

        fs::write(&launcher, &original).unwrap();
        let extra = path.parent().unwrap().join("stage/unexpected.bin");
        fs::write(&extra, b"preserve").unwrap();
        assert_eq!(
            verify_staged_package_into_directory(
                std::io::Cursor::new(bytes.clone()),
                bytes.len() as u64,
                Some(&digest),
                &directory,
                "stage",
                expectation(Some("1.2.3")),
            )
            .unwrap_err()
            .0,
            "package_stage_unexpected_entry"
        );
        assert_eq!(fs::read(&extra).unwrap(), b"preserve");
        assert_eq!(
            verify_staged_package_into_directory(
                std::io::Cursor::new(bytes.clone()),
                bytes.len() as u64,
                Some(&digest),
                &directory,
                "missing-stage",
                expectation(Some("1.2.3")),
            )
            .unwrap_err()
            .0,
            "package_stage_unavailable"
        );
        assert!(!path.parent().unwrap().join("missing-stage").exists());

        drop(directory);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn package_rejects_path_escape_identity_mismatch_and_content_tampering() {
        let path = write_temp(&package_with(files_for_test()), "identity");
        let result = extract_verified_package(
            &path,
            &path.parent().unwrap().join("stage"),
            expectation(Some("9.9.9")),
        );
        assert_eq!(result.unwrap_err().0, "package_identity_mismatch");
        fs::remove_dir_all(path.parent().unwrap()).unwrap();

        let mut bytes = package_with(files_for_test());
        let manifest_length = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let manifest_start = 12;
        let manifest_end = manifest_start + manifest_length;
        let manifest: serde_json::Value =
            serde_json::from_slice(&bytes[manifest_start..manifest_end]).unwrap();
        let mut value = manifest;
        value["files"][0]["path"] = json!("app/../../outside.txt");
        let raw = serde_json::to_vec(&value).unwrap();
        bytes.truncate(8);
        bytes.extend_from_slice(&(raw.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&raw);
        bytes.extend_from_slice(&[b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h']);
        let path = write_temp(&bytes, "escape");
        let result = extract_verified_package(
            &path,
            &path.parent().unwrap().join("stage"),
            expectation(None),
        );
        let path_error = result.unwrap_err().0;
        assert!(
            matches!(
                path_error,
                "package_path_invalid" | "package_file_order_or_duplicate"
            ),
            "unexpected path error: {path_error}"
        );
        fs::remove_dir_all(path.parent().unwrap()).unwrap();

        let mut bytes = package_with(files_for_test());
        *bytes.last_mut().unwrap() ^= 1;
        let path = write_temp(&bytes, "tampered");
        let destination = path.parent().unwrap().join("stage");
        let result = extract_verified_package(&path, &destination, expectation(None));
        assert_eq!(result.unwrap_err().0, "package_file_hash_mismatch");
        assert!(!destination.exists());
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn package_rejects_product_manifest_identity_or_authority_inheritance_mismatch() {
        for (name, manifest) in [
            (
                "manifest-id",
                String::from_utf8(PRODUCT_MANIFEST.to_vec())
                    .unwrap()
                    .replace(APP, "d4-pocket-app-33333333333333333333333333333333"),
            ),
            (
                "manifest-authority",
                String::from_utf8(PRODUCT_MANIFEST.to_vec())
                    .unwrap()
                    .replace("\"permission\":\"none\"", "\"permission\":\"inherited\""),
            ),
        ] {
            let mut files = files_for_test();
            files
                .iter_mut()
                .find(|(path, _)| *path == "product_manifest.json")
                .unwrap()
                .1 = manifest.as_bytes();
            let path = write_temp(&package_with(files), name);
            let destination = path.parent().unwrap().join("stage");
            let result = extract_verified_package(&path, &destination, expectation(None));
            assert_eq!(
                result.unwrap_err().0,
                "package_product_manifest_identity_mismatch"
            );
            assert!(!destination.exists());
            fs::remove_dir_all(path.parent().unwrap()).unwrap();
        }
    }

    #[test]
    fn manifest_requires_order_unique_paths_and_fixed_product_roots() {
        let mut bytes = package_with(files_for_test());
        bytes.extend_from_slice(b"trailing");
        let path = write_temp(&bytes, "trailing");
        let result = extract_verified_package(
            &path,
            &path.parent().unwrap().join("stage"),
            expectation(None),
        );
        assert_eq!(result.unwrap_err().0, "package_length_mismatch");
        fs::remove_dir_all(path.parent().unwrap()).unwrap();

        assert!(validate_path("C:/outside").is_err());
        assert!(validate_path("app/NUL.txt").is_err());
        assert!(validate_path("app/../x").is_err());
        assert!(validate_path("runtime/authority.json").is_err());
    }
}
