//! D4 Pocket の無圧縮・固定順 package reader。
//!
//! package の外側digest／署名はBrokerが検証する。本moduleはpackage構造、製品identity、
//! file一覧、各file hashを再検査して新しい一時directoryへ展開する。Authorityを生成しない。

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

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
    if !(12..=MAX_PACKAGE_BYTES).contains(&package_bytes) {
        return Err(err("package_size_invalid"));
    }
    if expected_package_sha256.is_some_and(|digest| !valid_sha256(digest)) {
        return Err(err("package_outer_hash_invalid"));
    }
    if destination.exists() || fs::symlink_metadata(destination).is_ok() {
        return Err(err("package_destination_exists"));
    }
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
        .ok_or_else(|| err("package_destination_invalid"))?;
    let stage_root = parent.join(destination_name);

    let mut input = PackageHashReader::new(input);
    let result =
        extract_inner(&mut input, &stage_root, &expected, package_bytes).and_then(|info| {
            if expected_package_sha256.is_some_and(|expected| input.finish() != expected) {
                return Err(err("package_outer_hash_mismatch"));
            }
            Ok(info)
        });
    if result.is_err() {
        if let Ok(root_metadata) = fs::symlink_metadata(&stage_root) {
            if root_metadata.is_dir()
                && !root_metadata.file_type().is_symlink()
                && !is_reparse_point(&root_metadata)
            {
                let _ = fs::remove_dir_all(&stage_root);
            }
        }
    }
    result
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
    destination: &Path,
    expected: &ProductPackageExpectation<'_>,
    package_bytes: u64,
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

    fs::create_dir(destination).map_err(|_| err("package_stage_create_failed"))?;
    let canonical_root = fs::canonicalize(destination).map_err(|_| err("package_stage_invalid"))?;
    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
    let mut product_manifest_validated = false;
    for entry in &manifest.files {
        if entry.path == "product_manifest.json" && entry.byte_length > MAX_PRODUCT_MANIFEST_BYTES {
            return Err(err("package_product_manifest_size_invalid"));
        }
        let target = safe_target(&canonical_root, &entry.path)?;
        let parent = target.parent().ok_or_else(|| err("package_path_invalid"))?;
        create_safe_directories(&canonical_root, parent)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|_| err("package_file_create_failed"))?;
        let mut remaining = entry.byte_length;
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
            output
                .write_all(&buffer[..count])
                .map_err(|_| err("package_file_write_failed"))?;
            digest.update(&buffer[..count]);
            if let Some(captured) = captured_manifest.as_mut() {
                captured.extend_from_slice(&buffer[..count]);
            }
            remaining -= count as u64;
        }
        output
            .sync_all()
            .map_err(|_| err("package_file_sync_failed"))?;
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

fn safe_target(root: &Path, relative: &str) -> Result<PathBuf, ProductPackageError> {
    validate_path(relative)?;
    let target = relative
        .split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part));
    if !target.starts_with(root) {
        return Err(err("package_path_outside_payload"));
    }
    Ok(target)
}

fn create_safe_directories(root: &Path, parent: &Path) -> Result<(), ProductPackageError> {
    if !parent.starts_with(root) {
        return Err(err("package_path_outside_payload"));
    }
    let relative = parent
        .strip_prefix(root)
        .map_err(|_| err("package_path_outside_payload"))?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(err("package_path_invalid"));
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata)
                if metadata.is_dir()
                    && !metadata.file_type().is_symlink()
                    && !is_reparse_point(&metadata) => {}
            Ok(_) => return Err(err("package_path_reparse_or_conflict")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|_| err("package_directory_create_failed"))?;
            }
            Err(_) => return Err(err("package_directory_invalid")),
        }
    }
    Ok(())
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
        let destination = path.parent().unwrap().join("stage-valid");
        let info = extract_verified_package_from_reader(
            file,
            length,
            Some(&package_sha256),
            &destination,
            expectation(Some("1.2.3")),
        )
        .unwrap();
        assert_eq!(info.product_version, "1.2.3");
        assert!(destination.join("product_manifest.json").is_file());

        let file = directory.open_with("package.pkg", &options).unwrap();
        let rejected_destination = path.parent().unwrap().join("stage-bad-hash");
        assert_eq!(
            extract_verified_package_from_reader(
                file,
                length,
                Some(&"b".repeat(64)),
                &rejected_destination,
                expectation(Some("1.2.3")),
            )
            .unwrap_err()
            .0,
            "package_outer_hash_mismatch"
        );
        assert!(!rejected_destination.exists());
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
