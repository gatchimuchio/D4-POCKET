//! Windows向けDesktop起動管理。権限判断はBrokerに残し、FlutterへBroker資格を渡さない。

use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use zeroize::Zeroize;
use serde::Deserialize;
use crate::audit_hash::sha256_tagged;

use crate::broker::export_center::{self, OwnerConfirmationSummary as ExportConfirmationSummary};
use crate::broker::ipc_server::{BrokerServerError, DesktopOwnerOperationRequest};
use crate::broker::{
    BrokerCredentialRole, BrokerEndpoint, BrokerOperation, BrokerRequestEnvelope,
    BrokerServerConfig,
};
use crate::broker::protocol::{
    canonical_payload_hash, owner_delete_confirmation_summary,
    owner_recovery_confirmation_summary, owner_registration_confirmation_summary,
    request_issued_at_is_current, OwnerDeleteConfirmationSummary,
    OwnerRecoveryConfirmationSummary, OwnerRegistrationConfirmationSummary,
};
#[cfg(test)]
use crate::broker::ipc_server::run_loopback_server_cancellable;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
const UI_POLL: Duration = Duration::from_millis(50);
const MAX_ENDPOINT_BYTES: u64 = 4096;
const MAX_REQUEST_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const RELAY_IO_TIMEOUT: Duration = Duration::from_secs(5);
const SESSION_FILE: &str = "broker_session.json";
const EXPORT_DIRECTORY_NAME: &str = "exports";
const EMBEDDED_PRODUCT_APP_ID: Option<&str> = option_env!("GUI_SHELL_PRODUCT_APP_ID");
const EMBEDDED_PRODUCT_AUDIT_STORE_ID: Option<&str> =
    option_env!("GUI_SHELL_PRODUCT_AUDIT_STORE_ID");
const FRONTEND_ENVIRONMENT_ALLOWLIST: &[&str] = &[
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMDATA",
    "SYSTEMDRIVE",
    "SYSTEMROOT",
    "TEMP",
    "TMP",
    "USERPROFILE",
    "WINDIR",
];

#[derive(Debug, Clone, PartialEq, Eq)]
enum DesktopOwnerOperationSummary {
    GuiShellExport(ExportConfirmationSummary),
    AgentTaskWorkspacePermission {
        runtime_id: String,
        session_id: String,
        workspace_id: String,
        payload_hash: String,
    },
    AgentTaskOwnerApproval {
        runtime_id: String,
        session_id: String,
        workspace_id: String,
        instruction_characters: usize,
        instruction_hash: String,
        payload_hash: String,
    },
    RegressionCaseDelete {
        summary: OwnerDeleteConfirmationSummary,
        payload_hash: String,
    },
    RegressionCaseRecovery {
        summary: OwnerRecoveryConfirmationSummary,
        payload_hash: String,
    },
    RegressionCaseRegistration {
        summary: OwnerRegistrationConfirmationSummary,
        payload_hash: String,
    },
    McpConnect {
        server_id: String,
        executable: String,
        workspace: String,
        argument_count: usize,
        arguments_hash: String,
        payload_hash: String,
    },
    McpDisconnect {
        server_id: String,
        payload_hash: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentTaskWorkspacePermissionRequest {
    agent_runtime_id: String,
    session_id: String,
    workspace_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentTaskOwnerApprovalRequest {
    agent_runtime_id: String,
    session_id: String,
    workspace_id: String,
    instruction: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct McpConnectCredentialReference {
    #[serde(rename = "credential_id")]
    credential_id: String,
    purpose: String,
    target: String,
    required: bool,
    status: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct McpConnectOwnerRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "操作")]
    operation: String,
    #[serde(rename = "ServerID")]
    server_id: String,
    #[serde(rename = "実行file")]
    executable: String,
    #[serde(rename = "引数")]
    arguments: Vec<String>,
    #[serde(rename = "workspace")]
    workspace: String,
    #[serde(rename = "Transport")]
    transport: String,
    #[serde(rename = "Credential ref")]
    credential_ref: McpConnectCredentialReference,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct McpDisconnectOwnerRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "操作")]
    operation: String,
    #[serde(rename = "ServerID")]
    server_id: String,
}

fn owner_confirmation_value(value: &str) -> String {
    let mut visible = String::with_capacity(value.len());
    for character in value.chars() {
        let codepoint = character as u32;
        if character.is_control()
            || matches!(codepoint, 0x061c | 0x200e | 0x200f | 0x202a..=0x202e | 0x2066..=0x206f)
        {
            visible.push_str(&format!("［文字コード{codepoint:04X}］"));
        } else {
            visible.push(character);
        }
    }
    visible
}

fn agent_task_permission_identifier_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopLaunchError {
    code: &'static str,
    message: &'static str,
}

impl DesktopLaunchError {
    fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }

    pub fn dialog_text(&self) -> String {
        format!(
            "{}\n\nエラーコード: {}\n\nD4 Pocketを終了して再起動してください。繰り返す場合は管理者へこのコードを伝えてください。",
            self.message, self.code
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PackageLayout {
    app_dir: PathBuf,
    app_exe: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProductRuntimeIdentity {
    app_id: String,
    audit_store_id: String,
}

fn has_generated_identity(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        suffix.len() == 32
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn product_runtime_identity(
    app_id: Option<&str>,
    audit_store_id: Option<&str>,
) -> Result<Option<ProductRuntimeIdentity>, DesktopLaunchError> {
    match (app_id, audit_store_id) {
        (None, None) => Ok(None),
        (Some(app_id), Some(audit_store_id))
            if has_generated_identity(app_id, "d4-pocket-app-")
                && has_generated_identity(audit_store_id, "audit-store-") =>
        {
            Ok(Some(ProductRuntimeIdentity {
                app_id: app_id.to_string(),
                audit_store_id: audit_store_id.to_string(),
            }))
        }
        _ => Err(DesktopLaunchError::new(
            "PRODUCT_RUNTIME_IDENTITY_INVALID",
            "書出しAppの保存領域identityが不正です。製品を再生成してください。",
        )),
    }
}

fn compiled_product_runtime_identity() -> Result<Option<ProductRuntimeIdentity>, DesktopLaunchError>
{
    product_runtime_identity(EMBEDDED_PRODUCT_APP_ID, EMBEDDED_PRODUCT_AUDIT_STORE_ID)
}

fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

fn checked_directory(path: &Path, boundary: &Path) -> Result<PathBuf, DesktopLaunchError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        DesktopLaunchError::new(
            "PACKAGE_DIRECTORY_MISSING",
            "起動に必要な製品ファイルを確認できません。",
        )
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(DesktopLaunchError::new(
            "PACKAGE_DIRECTORY_INVALID",
            "製品ファイルの配置が不正です。",
        ));
    }
    let canonical = fs::canonicalize(path).map_err(|_| {
        DesktopLaunchError::new(
            "PACKAGE_DIRECTORY_INVALID",
            "製品ファイルの配置が不正です。",
        )
    })?;
    if !canonical.starts_with(boundary) {
        return Err(DesktopLaunchError::new(
            "PACKAGE_PATH_OUTSIDE_ROOT",
            "製品ファイルが規定の配置範囲外にあります。",
        ));
    }
    Ok(canonical)
}

fn checked_file(path: &Path, boundary: &Path) -> Result<PathBuf, DesktopLaunchError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        DesktopLaunchError::new(
            "PACKAGE_FILE_MISSING",
            "起動に必要な製品ファイルがありません。再インストールしてください。",
        )
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(DesktopLaunchError::new(
            "PACKAGE_FILE_INVALID",
            "製品ファイルの配置が不正です。",
        ));
    }
    let canonical = fs::canonicalize(path).map_err(|_| {
        DesktopLaunchError::new("PACKAGE_FILE_INVALID", "製品ファイルの配置が不正です。")
    })?;
    if !canonical.starts_with(boundary) {
        return Err(DesktopLaunchError::new(
            "PACKAGE_PATH_OUTSIDE_ROOT",
            "製品ファイルが規定の配置範囲外にあります。",
        ));
    }
    Ok(canonical)
}

fn resolve_package_layout(launcher_exe: &Path) -> Result<PackageLayout, DesktopLaunchError> {
    let launcher = fs::canonicalize(launcher_exe).map_err(|_| {
        DesktopLaunchError::new("LAUNCHER_PATH_INVALID", "起動元を確認できません。")
    })?;
    let root = launcher.parent().ok_or_else(|| {
        DesktopLaunchError::new("PACKAGE_ROOT_MISSING", "製品の配置先を確認できません。")
    })?;
    let root = fs::canonicalize(root).map_err(|_| {
        DesktopLaunchError::new("PACKAGE_ROOT_MISSING", "製品の配置先を確認できません。")
    })?;
    let app_dir = checked_directory(&root.join("app"), &root)?;
    let broker_dir = checked_directory(&root.join("broker"), &root)?;
    let app_exe = checked_file(&app_dir.join("gui_shell_desktop.exe"), &app_dir)?;
    checked_file(&app_dir.join("flutter_windows.dll"), &app_dir)?;
    let data_dir = checked_directory(&app_dir.join("data"), &app_dir)?;
    checked_file(&data_dir.join("app.so"), &app_dir)?;
    checked_file(&data_dir.join("icudtl.dat"), &app_dir)?;
    checked_directory(&data_dir.join("flutter_assets"), &app_dir)?;
    checked_file(&broker_dir.join("gui_shell_rust_helper.exe"), &broker_dir)?;
    Ok(PackageLayout { app_dir, app_exe })
}

#[cfg(test)]
fn runtime_directory(local_app_data: &Path) -> Result<PathBuf, DesktopLaunchError> {
    let identity = compiled_product_runtime_identity()?;
    runtime_directory_with_identity(local_app_data, identity.as_ref())
}

fn runtime_directory_with_identity(
    local_app_data: &Path,
    identity: Option<&ProductRuntimeIdentity>,
) -> Result<PathBuf, DesktopLaunchError> {
    if identity.is_some_and(|identity| {
        !has_generated_identity(&identity.app_id, "d4-pocket-app-")
            || !has_generated_identity(&identity.audit_store_id, "audit-store-")
    }) {
        return Err(DesktopLaunchError::new(
            "PRODUCT_RUNTIME_IDENTITY_INVALID",
            "書出しAppの保存領域identityが不正です。製品を再生成してください。",
        ));
    }
    if !local_app_data.is_absolute() {
        return Err(DesktopLaunchError::new(
            "USER_DATA_ROOT_INVALID",
            "Windowsのユーザー保存先を確認できません。",
        ));
    }
    let root_metadata = fs::symlink_metadata(local_app_data).map_err(|_| {
        DesktopLaunchError::new(
            "USER_DATA_ROOT_UNAVAILABLE",
            "Windowsのユーザー保存先を確認できません。",
        )
    })?;
    if !root_metadata.is_dir()
        || root_metadata.file_type().is_symlink()
        || is_reparse_point(&root_metadata)
    {
        return Err(DesktopLaunchError::new(
            "USER_DATA_ROOT_INVALID",
            "Windowsのユーザー保存先の配置が不正です。",
        ));
    }
    let root = fs::canonicalize(local_app_data).map_err(|_| {
        DesktopLaunchError::new(
            "USER_DATA_ROOT_UNAVAILABLE",
            "ユーザー保存先を確認できません。",
        )
    })?;
    let mut canonical = root.clone();
    let components = match identity {
        Some(identity) => vec![
            "D4Pocket".to_string(),
            "apps".to_string(),
            identity.app_id.clone(),
            "stores".to_string(),
            identity.audit_store_id.clone(),
        ],
        None => vec![
            "GUI-Shell".to_string(),
            "broker".to_string(),
            "desktop".to_string(),
        ],
    };
    for component in components {
        let candidate = canonical.join(component);
        match fs::symlink_metadata(&candidate) {
            Ok(metadata)
                if metadata.is_dir()
                    && !metadata.file_type().is_symlink()
                    && !is_reparse_point(&metadata) => {}
            Ok(_) => {
                return Err(DesktopLaunchError::new(
                    "USER_DATA_ROOT_INVALID",
                    "ユーザー保存先に再解析pointまたは不正なfolderがあります。",
                ));
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {
                match fs::create_dir(&candidate) {
                    Ok(()) => {}
                    Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
                    Err(_) => {
                        return Err(DesktopLaunchError::new(
                            "USER_DATA_ROOT_UNAVAILABLE",
                            "ユーザー保存先を作成できません。空き容量とfolderのaccess権を確認してください。",
                        ));
                    }
                }
                let metadata = fs::symlink_metadata(&candidate).map_err(|_| {
                    DesktopLaunchError::new(
                        "USER_DATA_ROOT_UNAVAILABLE",
                        "ユーザー保存先を確認できません。",
                    )
                })?;
                if !metadata.is_dir()
                    || metadata.file_type().is_symlink()
                    || is_reparse_point(&metadata)
                {
                    return Err(DesktopLaunchError::new(
                        "USER_DATA_ROOT_INVALID",
                        "ユーザー保存先に再解析pointまたは不正なfolderがあります。",
                    ));
                }
            }
            Err(_) => {
                return Err(DesktopLaunchError::new(
                    "USER_DATA_ROOT_UNAVAILABLE",
                    "ユーザー保存先を確認できません。",
                ));
            }
        }
        canonical = fs::canonicalize(&candidate).map_err(|_| {
            DesktopLaunchError::new(
                "USER_DATA_ROOT_UNAVAILABLE",
                "ユーザー保存先を確認できません。",
            )
        })?;
        if !canonical.starts_with(&root) {
            return Err(DesktopLaunchError::new(
                "USER_DATA_ROOT_INVALID",
                "ユーザー保存先が指定rootの範囲外を指しています。",
            ));
        }
    }
    Ok(canonical)
}

fn ensure_store_directory(runtime_dir: &Path) -> Result<PathBuf, DesktopLaunchError> {
    let store_dir = runtime_dir.join("store");
    match fs::symlink_metadata(&store_dir) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => match fs::create_dir(&store_dir) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(_) => {
                return Err(DesktopLaunchError::new(
                        "BROKER_STORE_UNAVAILABLE",
                        "Brokerの保存領域を準備できません。空き容量とフォルダーのアクセス権を確認してください。",
                    ));
            }
        },
        Err(_) => {
            return Err(DesktopLaunchError::new(
                "BROKER_STORE_UNAVAILABLE",
                "Brokerの保存領域を確認できません。",
            ));
        }
    }

    let metadata = fs::symlink_metadata(&store_dir).map_err(|_| {
        DesktopLaunchError::new(
            "BROKER_STORE_UNAVAILABLE",
            "Brokerの保存領域を確認できません。",
        )
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(DesktopLaunchError::new(
            "BROKER_STORE_INVALID",
            "Brokerの保存領域に再解析pointまたは不正なfolderがあります。",
        ));
    }
    let runtime_root = fs::canonicalize(runtime_dir).map_err(|_| {
        DesktopLaunchError::new(
            "BROKER_STORE_UNAVAILABLE",
            "Brokerの保存領域を確認できません。",
        )
    })?;
    let canonical_store = fs::canonicalize(&store_dir).map_err(|_| {
        DesktopLaunchError::new(
            "BROKER_STORE_UNAVAILABLE",
            "Brokerの保存領域を確認できません。",
        )
    })?;
    if !canonical_store.starts_with(&runtime_root) {
        return Err(DesktopLaunchError::new(
            "BROKER_STORE_INVALID",
            "Brokerの保存領域が指定rootの範囲外を指しています。",
        ));
    }
    Ok(canonical_store)
}

fn ensure_protected_store_directory(runtime_dir: &Path) -> Result<PathBuf, DesktopLaunchError> {
    let protected_dir = runtime_dir.join("protected");
    match fs::symlink_metadata(&protected_dir) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => match fs::create_dir(&protected_dir) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(_) => {
                return Err(DesktopLaunchError::new(
                    "PROTECTED_STORE_UNAVAILABLE",
                    "保護された保存領域を準備できません。空き容量とフォルダーのアクセス権を確認してください。",
                ));
            }
        },
        Err(_) => {
            return Err(DesktopLaunchError::new(
                "PROTECTED_STORE_UNAVAILABLE",
                "保護された保存領域を確認できません。",
            ));
        }
    }

    let metadata = fs::symlink_metadata(&protected_dir).map_err(|_| {
        DesktopLaunchError::new(
            "PROTECTED_STORE_UNAVAILABLE",
            "保護された保存領域を確認できません。",
        )
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(DesktopLaunchError::new(
            "PROTECTED_STORE_INVALID",
            "保護された保存領域に再解析pointまたは不正なfolderがあります。",
        ));
    }
    let runtime_root = fs::canonicalize(runtime_dir).map_err(|_| {
        DesktopLaunchError::new(
            "PROTECTED_STORE_UNAVAILABLE",
            "保護された保存領域を確認できません。",
        )
    })?;
    let canonical_protected = fs::canonicalize(&protected_dir).map_err(|_| {
        DesktopLaunchError::new(
            "PROTECTED_STORE_UNAVAILABLE",
            "保護された保存領域を確認できません。",
        )
    })?;
    if !canonical_protected.starts_with(&runtime_root) {
        return Err(DesktopLaunchError::new(
            "PROTECTED_STORE_INVALID",
            "保護された保存領域が指定rootの範囲外を指しています。",
        ));
    }
    Ok(canonical_protected)
}

fn ensure_export_directory(runtime_dir: &Path) -> Result<PathBuf, DesktopLaunchError> {
    let export_dir = runtime_dir.join(EXPORT_DIRECTORY_NAME);
    match fs::symlink_metadata(&export_dir) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => match fs::create_dir(&export_dir) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(_) => {
                return Err(DesktopLaunchError::new(
                    "EXPORT_DIRECTORY_UNAVAILABLE",
                    "Manifestの保存先を準備できません。",
                ));
            }
        },
        Err(_) => {
            return Err(DesktopLaunchError::new(
                "EXPORT_DIRECTORY_UNAVAILABLE",
                "Manifestの保存先を確認できません。",
            ));
        }
    }
    let metadata = fs::symlink_metadata(&export_dir).map_err(|_| {
        DesktopLaunchError::new(
            "EXPORT_DIRECTORY_UNAVAILABLE",
            "Manifestの保存先を確認できません。",
        )
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(DesktopLaunchError::new(
            "EXPORT_DIRECTORY_INVALID",
            "Manifestの保存先に再解析pointまたは不正なfolderがあります。",
        ));
    }
    let runtime_root = fs::canonicalize(runtime_dir).map_err(|_| {
        DesktopLaunchError::new(
            "EXPORT_DIRECTORY_UNAVAILABLE",
            "Manifestの保存先を確認できません。",
        )
    })?;
    let canonical_export_dir = fs::canonicalize(&export_dir).map_err(|_| {
        DesktopLaunchError::new(
            "EXPORT_DIRECTORY_UNAVAILABLE",
            "Manifestの保存先を確認できません。",
        )
    })?;
    if !canonical_export_dir.starts_with(&runtime_root) {
        return Err(DesktopLaunchError::new(
            "EXPORT_DIRECTORY_INVALID",
            "Manifestの保存先が固定rootの範囲外です。",
        ));
    }
    Ok(canonical_export_dir)
}

fn reject_reparse_file(path: &Path) -> Result<Option<fs::Metadata>, DesktopLaunchError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || is_reparse_point(&metadata)
            {
                return Err(DesktopLaunchError::new(
                    "SESSION_FILE_INVALID",
                    "Broker接続情報の保存状態が不正です。",
                ));
            }
            if metadata.len() > MAX_ENDPOINT_BYTES {
                return Err(DesktopLaunchError::new(
                    "SESSION_FILE_OVERSIZED",
                    "Broker接続情報の保存状態が上限を超えています。",
                ));
            }
            Ok(Some(metadata))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(_) => Err(DesktopLaunchError::new(
            "SESSION_FILE_UNAVAILABLE",
            "Broker接続情報の保存状態を確認できません。",
        )),
    }
}

fn remove_managed_file(path: &Path) -> Result<(), DesktopLaunchError> {
    if reject_reparse_file(path)?.is_some() {
        fs::remove_file(path).map_err(|_| {
            DesktopLaunchError::new(
                "SESSION_FILE_CLEANUP_FAILED",
                "古いBroker接続情報を安全に片付けられません。",
            )
        })?;
    }
    Ok(())
}

fn prepare_session_paths(session_file: &Path) -> Result<(), DesktopLaunchError> {
    remove_managed_file(session_file)?;
    remove_managed_file(&session_file.with_extension("json.tmp"))
}

fn acquire_instance_lock(runtime_dir: &Path) -> Result<File, DesktopLaunchError> {
    let lock_path = runtime_dir.join("desktop_launcher.lock");
    reject_reparse_file(&lock_path)?;
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(lock_path)
        .map_err(|_| {
            DesktopLaunchError::new(
                "INSTANCE_LOCK_UNAVAILABLE",
                "多重起動防止の準備に失敗しました。",
            )
        })?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(std::fs::TryLockError::WouldBlock) => Err(DesktopLaunchError::new(
            "INSTANCE_ALREADY_RUNNING",
            "D4 Pocketはすでに起動しています。既存の画面を確認してください。",
        )),
        Err(std::fs::TryLockError::Error(_)) => Err(DesktopLaunchError::new(
            "INSTANCE_LOCK_UNAVAILABLE",
            "多重起動防止の準備に失敗しました。",
        )),
    }
}

fn validate_normal_endpoint(endpoint: &BrokerEndpoint) -> Result<(), DesktopLaunchError> {
    let session_id_valid = !endpoint.session_id.is_empty()
        && endpoint.session_id.len() <= 96
        && endpoint
            .session_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    let secret_valid = endpoint.session_secret.len() == 64
        && endpoint
            .session_secret
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if endpoint.host != "127.0.0.1"
        || endpoint.port == 0
        || !session_id_valid
        || !secret_valid
        || endpoint.credential_role != BrokerCredentialRole::Normal
        || endpoint.transport != "authenticated_loopback_tcp"
        || endpoint.max_request_bytes == 0
        || endpoint.max_request_bytes > MAX_REQUEST_BYTES
    {
        return Err(DesktopLaunchError::new(
            "BROKER_ENDPOINT_INVALID",
            "Brokerの通常接続情報が不正です。安全のため起動を中止しました。",
        ));
    }
    Ok(())
}

fn read_endpoint(path: &Path) -> Result<(Vec<u8>, BrokerEndpoint), DesktopLaunchError> {
    reject_reparse_file(path)?.ok_or_else(|| {
        DesktopLaunchError::new(
            "BROKER_ENDPOINT_MISSING",
            "Brokerの準備が完了しませんでした。",
        )
    })?;
    let mut file = File::open(path).map_err(|_| {
        DesktopLaunchError::new(
            "BROKER_ENDPOINT_UNAVAILABLE",
            "Broker接続情報を読み取れません。",
        )
    })?;
    let mut bytes = Vec::with_capacity(MAX_ENDPOINT_BYTES as usize);
    Read::by_ref(&mut file)
        .take(MAX_ENDPOINT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| {
            DesktopLaunchError::new(
                "BROKER_ENDPOINT_UNAVAILABLE",
                "Broker接続情報を読み取れません。",
            )
        })?;
    if bytes.len() as u64 > MAX_ENDPOINT_BYTES {
        return Err(DesktopLaunchError::new(
            "SESSION_FILE_OVERSIZED",
            "Broker接続情報の保存状態が上限を超えています。",
        ));
    }
    let endpoint: BrokerEndpoint = serde_json::from_slice(&bytes).map_err(|_| {
        DesktopLaunchError::new(
            "BROKER_ENDPOINT_INVALID",
            "Broker接続情報を検証できません。",
        )
    })?;
    validate_normal_endpoint(&endpoint)?;
    Ok((bytes, endpoint))
}

fn remove_session_if_unchanged(path: &Path, expected: &[u8]) -> Result<(), DesktopLaunchError> {
    if reject_reparse_file(path)?.is_none() {
        return Ok(());
    }
    match fs::read(path) {
        Ok(current) if current == expected => fs::remove_file(path).map_err(|_| {
            DesktopLaunchError::new(
                "SESSION_FILE_CLEANUP_FAILED",
                "Broker接続情報を安全に片付けられません。再起動前に管理者へ連絡してください。",
            )
        }),
        Ok(_) => Err(DesktopLaunchError::new(
            "SESSION_FILE_CHANGED",
            "起動中にBroker接続情報が変更されたため、自動削除を中止しました。",
        )),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(DesktopLaunchError::new(
            "SESSION_FILE_CLEANUP_FAILED",
            "Broker接続情報を安全に片付けられません。再起動前に管理者へ連絡してください。",
        )),
    }
}

fn remove_failed_start_files(session_file: &Path) {
    let _ = remove_managed_file(session_file);
    let _ = remove_managed_file(&session_file.with_extension("json.tmp"));
}

/// Rust起動器内だけで通常資格を保持し、要求を既存Brokerへ転送する。
/// Pipe側はBroker認可・Approval・Auditの代替を行わない。
struct RelayEndpoint(BrokerEndpoint);

impl Drop for RelayEndpoint {
    fn drop(&mut self) {
        self.0.session_secret.zeroize();
    }
}

fn normalize_channel_request(input: &[u8], session_id: &str) -> Vec<u8> {
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(input) else {
        return input.to_vec();
    };
    let Some(object) = value.as_object_mut() else {
        return input.to_vec();
    };
    if object.contains_key("session_id") {
        object.insert(
            "desktop_channel_session_id_forbidden".into(),
            serde_json::Value::Bool(true),
        );
    } else {
        object.insert(
            "session_id".into(),
            serde_json::Value::String(session_id.to_owned()),
        );
    }
    serde_json::to_vec(&value).unwrap_or_else(|_| input.to_vec())
}

#[cfg(test)]
fn relay_channel_frame(
    frame: gui_shell_windows_broker_channel::PipeFrame,
    endpoint: &BrokerEndpoint,
) -> Option<Vec<u8>> {
    relay_channel_frame_with_owner_operations(frame, endpoint, None, |_| false)
}

fn owner_operation_candidate(
    input: &[u8],
    endpoint: &BrokerEndpoint,
) -> Option<(String, DesktopOwnerOperationSummary)> {
    if input.len() > endpoint.max_request_bytes {
        return None;
    }
    let input = std::str::from_utf8(input).ok()?;
    let envelope = BrokerRequestEnvelope::from_json_str(input).ok()?;
    if envelope.session_id.is_some()
        || envelope.request_id.as_deref().is_none_or(str::is_empty)
        || envelope.nonce.as_deref().is_none_or(str::is_empty)
        || !envelope.issued_at.as_deref().is_some_and(request_issued_at_is_current)
        || envelope.metadata.len() != 1
        || envelope.metadata[0].key != "client"
        || envelope.metadata[0].value != "desktop_flutter"
    {
        return None;
    }
    let payload_hash = canonical_payload_hash(envelope.payload.as_ref());
    if envelope.payload_hash.as_deref() != Some(payload_hash.as_str()) {
        return None;
    }
    let payload = envelope.payload.as_ref().unwrap_or(&serde_json::Value::Null);
    let summary = match envelope.operation? {
        BrokerOperation::MCP接続 => {
            let request: McpConnectOwnerRequest =
                serde_json::from_value(payload.clone()).ok()?;
            let reference = &request.credential_ref;
            if request.version != 1
                || request.operation != "接続"
                || request.server_id.is_empty()
                || request.server_id.len() > 128
                || request.server_id.chars().any(char::is_control)
                || request.executable.is_empty()
                || request.executable.len() > 1024
                || request.executable.chars().any(char::is_control)
                || !Path::new(&request.executable).is_absolute()
                || request.workspace.is_empty()
                || request.workspace.len() > 1024
                || request.workspace.chars().any(char::is_control)
                || !Path::new(&request.workspace).is_absolute()
                || request.transport != "stdio"
                || request.arguments.len() > 32
                || request.arguments.iter().any(|argument| {
                    argument.is_empty()
                        || argument.len() > 1024
                        || argument.chars().any(char::is_control)
                })
                || reference.credential_id.len() != 32
                || !reference
                    .credential_id
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                || reference.purpose != "mcp_transport"
                || reference.target != request.server_id
                || reference.required
                || reference.status != "missing"
            {
                return None;
            }
            let arguments_hash = sha256_tagged(&serde_json::to_vec(&request.arguments).ok()?);
            DesktopOwnerOperationSummary::McpConnect {
                server_id: request.server_id,
                executable: request.executable,
                workspace: request.workspace,
                argument_count: request.arguments.len(),
                arguments_hash,
                payload_hash,
            }
        }
        BrokerOperation::GuiShell書出し => DesktopOwnerOperationSummary::GuiShellExport(
            export_center::owner_confirmation_summary(payload, &payload_hash).ok()?,
        ),
        BrokerOperation::AgentTaskWorkspacePermissionGrant => {
            let request: AgentTaskWorkspacePermissionRequest =
                serde_json::from_value(payload.clone()).ok()?;
            if !agent_task_permission_identifier_is_valid(&request.agent_runtime_id)
                || !agent_task_permission_identifier_is_valid(&request.session_id)
                || !agent_task_permission_identifier_is_valid(&request.workspace_id)
            {
                return None;
            }
            DesktopOwnerOperationSummary::AgentTaskWorkspacePermission {
                runtime_id: request.agent_runtime_id,
                session_id: request.session_id,
                workspace_id: request.workspace_id,
                payload_hash,
            }
        }
        BrokerOperation::AgentTaskOwnerApprovalGrant => {
            let request: AgentTaskOwnerApprovalRequest =
                serde_json::from_value(payload.clone()).ok()?;
            if !agent_task_permission_identifier_is_valid(&request.agent_runtime_id)
                || !agent_task_permission_identifier_is_valid(&request.session_id)
                || !agent_task_permission_identifier_is_valid(&request.workspace_id)
                || request.instruction.trim().is_empty()
                || request.instruction.chars().count() > 32_768
            {
                return None;
            }
            DesktopOwnerOperationSummary::AgentTaskOwnerApproval {
                runtime_id: request.agent_runtime_id,
                session_id: request.session_id,
                workspace_id: request.workspace_id,
                instruction_characters: request.instruction.chars().count(),
                instruction_hash: sha256_tagged(request.instruction.as_bytes()),
                payload_hash,
            }
        }
        BrokerOperation::MCP切断 => {
            let request: McpDisconnectOwnerRequest =
                serde_json::from_value(payload.clone()).ok()?;
            if request.version != 1
                || request.operation != "切断"
                || request.server_id.is_empty()
                || request.server_id.len() > 128
                || request.server_id.chars().any(char::is_control)
            {
                return None;
            }
            DesktopOwnerOperationSummary::McpDisconnect {
                server_id: request.server_id,
                payload_hash,
            }
        }
        BrokerOperation::回帰Case削除 => DesktopOwnerOperationSummary::RegressionCaseDelete {
            summary: owner_delete_confirmation_summary(payload).ok()?,
            payload_hash,
        },
        BrokerOperation::回帰Case削除中断確認 => {
            DesktopOwnerOperationSummary::RegressionCaseRecovery {
                summary: owner_recovery_confirmation_summary(payload).ok()?,
                payload_hash,
            }
        }
        BrokerOperation::回帰Case登録 => {
            DesktopOwnerOperationSummary::RegressionCaseRegistration {
                summary: owner_registration_confirmation_summary(payload).ok()?,
                payload_hash,
            }
        }
        _ => return None,
    };
    let normalized = normalize_channel_request(input.as_bytes(), &endpoint.session_id);
    let normalized = String::from_utf8(normalized).ok()?;
    let normalized_envelope = BrokerRequestEnvelope::from_json_str(&normalized).ok()?;
    if normalized_envelope.session_id.as_deref() != Some(endpoint.session_id.as_str()) {
        return None;
    }
    Some((normalized, summary))
}

fn relay_channel_frame_with_owner_operations<F>(
    frame: gui_shell_windows_broker_channel::PipeFrame,
    endpoint: &BrokerEndpoint,
    owner_operations: Option<&mpsc::SyncSender<DesktopOwnerOperationRequest>>,
    mut confirm_owner_operation: F,
) -> Option<Vec<u8>>
where
    F: FnMut(&DesktopOwnerOperationSummary) -> bool,
{
    let request = match frame {
        gui_shell_windows_broker_channel::PipeFrame::Line(bytes) => {
            if let Some(owner_operations) = owner_operations {
                if let Some((request_json, summary)) = owner_operation_candidate(&bytes, endpoint) {
                    if confirm_owner_operation(&summary) {
                        let (reply, response) = mpsc::sync_channel(1);
                        owner_operations
                            .send(DesktopOwnerOperationRequest { request_json, reply })
                            .ok()?;
                        let response = response.recv().ok()?;
                        return response.to_json_string().ok().map(String::into_bytes);
                    }
                }
            }
            normalize_channel_request(&bytes, &endpoint.session_id)
        }
        gui_shell_windows_broker_channel::PipeFrame::Oversized => {
            vec![b' '; endpoint.max_request_bytes.saturating_add(1)]
        }
    };
    relay_normalized_channel_request(&request, endpoint)
}

fn relay_normalized_channel_request(request: &[u8], endpoint: &BrokerEndpoint) -> Option<Vec<u8>> {
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), endpoint.port);
    let deadline = Instant::now() + RELAY_IO_TIMEOUT;
    let mut stream = TcpStream::connect_timeout(&address, RELAY_IO_TIMEOUT).ok()?;
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return None;
    }
    stream.set_write_timeout(Some(remaining)).ok()?;
    stream.write_all(endpoint.session_secret.as_bytes()).ok()?;
    stream.write_all(b"\n").ok()?;
    stream.write_all(&request).ok()?;
    stream.write_all(b"\n").ok()?;
    stream.flush().ok()?;

    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return None;
    }
    stream.set_read_timeout(Some(remaining)).ok()?;
    let reader = BufReader::new(stream);
    let mut response = Vec::new();
    let read = reader
        .take((MAX_RESPONSE_BYTES + 1) as u64)
        .read_until(b'\n', &mut response)
        .ok()?;
    if read == 0 || response.len() > MAX_RESPONSE_BYTES || response.last() != Some(&b'\n') {
        return None;
    }
    response.pop();
    if response.last() == Some(&b'\r') {
        response.pop();
    }
    Some(response)
}

fn confirm_owner_operation(
    summary: &DesktopOwnerOperationSummary,
    product_identity: Option<&ProductRuntimeIdentity>,
) -> bool {
    use winsafe::{co, prelude::*, HWND};

    let text = owner_confirmation_text_for_identity(summary, product_identity);
    matches!(
        HWND::NULL.MessageBox(
            &text,
            "D4 Pocket Owner確認",
            co::MB::YESNO | co::MB::DEFBUTTON2 | co::MB::ICONWARNING | co::MB::TASKMODAL,
        ),
        Ok(co::DLGID::YES)
    )
}

#[cfg(test)]
fn owner_confirmation_text(summary: &DesktopOwnerOperationSummary) -> String {
    owner_confirmation_text_for_identity(summary, None)
}

fn owner_confirmation_text_for_identity(
    summary: &DesktopOwnerOperationSummary,
    product_identity: Option<&ProductRuntimeIdentity>,
) -> String {
    match summary {
        DesktopOwnerOperationSummary::GuiShellExport(summary) => {
            let export_path = match product_identity {
                Some(identity) => format!(
                    r"%LOCALAPPDATA%\D4Pocket\apps\{}\stores\{}\exports\<新規App ID>.json",
                    identity.app_id, identity.audit_store_id
                ),
                None => format!(
                    r"%LOCALAPPDATA%\GUI-Shell\broker\desktop\{}\<新規App ID>.json",
                    EXPORT_DIRECTORY_NAME
                ),
            };
            format!(
                "このGUI Shell構成のWindows向け独立Manifest file作成を許可しますか？\n\nアプリ名: {}\nExport ID: {}\n配布channel: {}\n選択任意Module数: {}\n保存先: {}\n\nこの操作はBroker監査へ記録されます。実行可能App、Module除去、build、Installer、署名は作成しません。Credential、Permission、Approval、Audit chainは継承しません。Windows accountの再認証ではありません。\n\npayload hash:\n{}",
                summary.display_name,
                summary.export_id,
                summary.distribution_channel,
                summary.optional_module_count,
                export_path,
                summary.payload_hash
            )
        }
        DesktopOwnerOperationSummary::AgentTaskWorkspacePermission {
            runtime_id,
            session_id,
            workspace_id,
            payload_hash,
        } => format!(
            "このAgent Sessionに限定したTask用Workspace Permissionを発行しますか？\n\nRuntime ID: {}\nSession ID: {}\nWorkspace ID: {}\n許可operation: agent_task.execute\n範囲: このSession・Workspaceで1回、5分以内\n\nこの確認はTask本文を承認せず、Taskを実行・保存・変更しません。実行時にはTask本文と条件を示す別のOwner Approvalが必要です。Windows accountの再認証ではありません。\n\npayload hash:\n{}",
            runtime_id, session_id, workspace_id, payload_hash
        ),
        DesktopOwnerOperationSummary::AgentTaskOwnerApproval {
            runtime_id,
            session_id,
            workspace_id,
            instruction_characters,
            instruction_hash,
            payload_hash,
        } => format!(
            "このAgent Taskの一回限りOwner Approvalを発行しますか？\n\nRuntime ID: {}\nSession ID: {}\nWorkspace ID: {}\n指示文字数: {}\n指示hash: {}\n要求ポリシー: gui-shell-agent-task-sandbox-v1-max-runtime-900s\n範囲: このSession・Workspace・指示hash・実行条件に限定、Approval発行後5分以内に開始、開始後の最大実行時間15分\n\nCompose画面でTask本文を確認してから判断してください。この確認画面は本文を表示しません。このApprovalはTask開始と最大15分の実行を一回だけ許可し、Workspace差分の確認と判断は別操作です。Credential等の秘密をTask本文へ含めないでください。発行はTaskを保存・変更・実行せず、実行可能なsandboxの存在も証明しません。Windows accountの再認証ではありません。\n\npayload hash:\n{}",
            runtime_id, session_id, workspace_id, instruction_characters, instruction_hash, payload_hash
        ),
        DesktopOwnerOperationSummary::McpDisconnect {
            server_id,
            payload_hash,
        } => format!(
            "指定したMCP Serverの接続を切断しますか？\n\nServer ID: {}\n対象: このBrokerが保持する指定stdio process群だけ\n処理: Windows Job Objectによるprocess群停止確認後、永続Auditを確定して接続記録を解消\n\nCredential、他Server、Permission、Approvalを変更せず、Toolを実行しません。停止またはAudit確定に失敗した場合は成功扱いしません。Windows accountの再認証ではありません。\n\npayload hash:\n{}",
            owner_confirmation_value(server_id),
            payload_hash
        ),
        DesktopOwnerOperationSummary::McpConnect {
            server_id,
            executable,
            workspace,
            argument_count,
            arguments_hash,
            payload_hash,
        } => format!(
            "指定したMCP stdio Serverを起動し、検証済みmetadataを取得しますか？\n\nServer ID: {}\n実行file: {}\n作業folder: {}\n起動引数: {}件（引数本文は秘密値を含む可能性があるため表示しません）\n引数hash: {}\nCredential ref: status=missing（Credential実値は送られません）\n\n前画面で実行file、作業folder、引数を確認してください。秘密値を引数へ含めないでください。ServerのTrustを成立させず、Tool実行、Permission、Approvalを付与しません。Windows Job Object監督下でprocess群を起動し、接続metadataとAuditをBrokerが確定します。Windows accountの再認証ではありません。\n\npayload hash:\n{}",
            owner_confirmation_value(server_id),
            owner_confirmation_value(executable),
            owner_confirmation_value(workspace),
            argument_count,
            arguments_hash,
            payload_hash
        ),
        DesktopOwnerOperationSummary::RegressionCaseDelete {
            summary,
            payload_hash,
        } => format!(
            "指定した回帰Caseの保存暗号文を削除しますか？\n\n回帰Case ID: {}\n定義hash: {}\n暗号文hash: {}\n\n削除前にBrokerが登録AuditとProtectedStoreを再照合します。削除後のfile不在と結果Auditが確認された場合だけ確定します。媒体の物理消去は保証しません。Windows accountの再認証ではありません。\n\npayload hash:\n{}",
            summary.case_id,
            summary.definition_hash,
            summary.ciphertext_hash,
            payload_hash
        ),
        DesktopOwnerOperationSummary::RegressionCaseRecovery {
            summary,
            payload_hash,
        } => format!(
            "指定回帰Caseの未確定削除状態を照合しますか？\n\n回帰Case ID: {}\n\nこの操作は削除を実行しません。Brokerが永続AuditとProtectedStoreの現在状態を照合します。自動再削除は行いません。Windows accountの再認証ではありません。\n\npayload hash:\n{}",
            summary.case_id,
            payload_hash
        ),
        DesktopOwnerOperationSummary::RegressionCaseRegistration {
            summary,
            payload_hash,
        } => format!(
            "完了済み対話から回帰Caseを登録しますか？\n\n公開名: {}\n対象要求ID: {}\n対象要求hash: {}\n期待状態: {}\nowner記入本文: {}文字\n必要条件: {}件 / 禁止条件: {}件 / 必要参照: {}件\n\n入力本文・条件・参照・期待経路はこの確認画面やAuditへ表示しません。元の対話本文を自動コピーしていません。内容を確認し、秘密を除いた定義であることを確認してください。秘密候補の検査は秘密不存在を証明しません。Brokerは処理時に現在の対話証跡とProtectedStore条件を再確認します。Windows accountの再認証ではありません。\n\npayload hash:\n{}",
            summary.display_name.escape_debug(),
            summary.request_id,
            summary.request_hash,
            summary.expected_status,
            summary.input_characters,
            summary.required_condition_count,
            summary.forbidden_condition_count,
            summary.required_reference_count,
            payload_hash
        ),
    }
}

fn run_channel_server(
    pipe_name: String,
    endpoint: RelayEndpoint,
    expected_client_pid: Arc<AtomicU32>,
    shutdown: Arc<AtomicBool>,
    ready: mpsc::SyncSender<()>,
    owner_operations: mpsc::SyncSender<DesktopOwnerOperationRequest>,
    product_identity: Option<ProductRuntimeIdentity>,
) -> Result<(), BrokerServerError> {
    gui_shell_windows_broker_channel::run_server(
        pipe_name,
        expected_client_pid,
        shutdown,
        ready,
        endpoint.0.max_request_bytes,
        MAX_RESPONSE_BYTES,
        |frame| {
            relay_channel_frame_with_owner_operations(
                frame,
                &endpoint.0,
                Some(&owner_operations),
                |summary| confirm_owner_operation(summary, product_identity.as_ref()),
            )
        },
    )
    .map_err(|error| BrokerServerError {
        message: format!("安全Brokerの通信路に問題があります。code={}", error.code()),
    })
}

struct RunningBroker {
    shutdown: Arc<AtomicBool>,
    server: Option<JoinHandle<Result<(), BrokerServerError>>>,
    channel_server: Option<JoinHandle<Result<(), BrokerServerError>>>,
    channel_pipe_name: String,
    frontend_pid: Arc<AtomicU32>,
    session_file: PathBuf,
    session_bytes: Vec<u8>,
}

impl RunningBroker {
    fn start(
        runtime_dir: &Path,
        installed_package_verified: bool,
        product_identity: Option<ProductRuntimeIdentity>,
    ) -> Result<Self, DesktopLaunchError> {
        let store_dir = ensure_store_directory(runtime_dir)?;
        let protected_store_dir = ensure_protected_store_directory(runtime_dir)?;
        let export_dir = ensure_export_directory(runtime_dir)?;
        let export_root = cap_std::fs::Dir::open_ambient_dir(
            &export_dir,
            cap_std::ambient_authority(),
        )
        .map_err(|_| {
            DesktopLaunchError::new(
                "EXPORT_DIRECTORY_UNAVAILABLE",
                "Manifestの保存先を安全に開けません。",
            )
        })?;
        let session_file = runtime_dir.join(SESSION_FILE);
        prepare_session_paths(&session_file)?;

        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = Arc::clone(&shutdown);
        let mut config = BrokerServerConfig::new(store_dir, session_file.clone());
        config.desktop_protected_store_dir = Some(protected_store_dir);
        config.desktop_install_path_verified = installed_package_verified;
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (owner_operation_tx, owner_operation_rx) = mpsc::sync_channel(1);
        let server = thread::Builder::new()
            .name("gui-shell-security-broker".to_string())
            .spawn(move || {
                crate::broker::ipc_server::run_loopback_server_cancellable_with_owner_operations(
                    config,
                    thread_shutdown,
                    ready_tx,
                    owner_operation_rx,
                    Some((export_dir, export_root)),
                )
            })
            .map_err(|_| {
                DesktopLaunchError::new("BROKER_THREAD_FAILED", "安全Brokerを起動できません。")
            })?;

        match ready_rx.recv_timeout(STARTUP_TIMEOUT) {
            Ok(()) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = server.join();
                remove_failed_start_files(&session_file);
                return Err(DesktopLaunchError::new(
                    "BROKER_START_FAILED",
                    "安全Brokerが準備前に終了しました。",
                ));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                shutdown.store(true, Ordering::Release);
                let _ = server.join();
                remove_failed_start_files(&session_file);
                return Err(DesktopLaunchError::new(
                    "BROKER_START_TIMEOUT",
                    "安全Brokerの準備が時間内に完了しませんでした。",
                ));
            }
        }

        let (mut session_bytes, mut endpoint) = match read_endpoint(&session_file) {
            Ok(value) => value,
            Err(error) => {
                shutdown.store(true, Ordering::Release);
                let _ = server.join();
                remove_failed_start_files(&session_file);
                return Err(error);
            }
        };
        let mut random = [0u8; 16];
        if getrandom::getrandom(&mut random).is_err() {
            endpoint.session_secret.zeroize();
            session_bytes.zeroize();
            shutdown.store(true, Ordering::Release);
            let _ = server.join();
            remove_failed_start_files(&session_file);
            return Err(DesktopLaunchError::new(
                "BROKER_CHANNEL_START_FAILED",
                "画面と安全Brokerの通信路を準備できません。",
            ));
        }
        let channel_pipe_name =
            match gui_shell_windows_broker_channel::pipe_name(&hex::encode(random)) {
                Ok(name) => name,
                Err(_) => {
                    endpoint.session_secret.zeroize();
                    session_bytes.zeroize();
                    shutdown.store(true, Ordering::Release);
                    let _ = server.join();
                    remove_failed_start_files(&session_file);
                    return Err(DesktopLaunchError::new(
                        "BROKER_CHANNEL_START_FAILED",
                        "画面と安全Brokerの通信路を準備できません。",
                    ));
                }
            };
        let frontend_pid = Arc::new(AtomicU32::new(0));
        let channel_shutdown = Arc::clone(&shutdown);
        let channel_expected_pid = Arc::clone(&frontend_pid);
        let (channel_ready_tx, channel_ready_rx) = mpsc::sync_channel(1);
        let channel_name_for_thread = channel_pipe_name.clone();
        let relay_endpoint = RelayEndpoint(endpoint);
        let channel_product_identity = product_identity.clone();
        let channel_server = match thread::Builder::new()
            .name("gui-shell-desktop-broker-pipe".to_string())
            .spawn(move || {
                run_channel_server(
                    channel_name_for_thread,
                    relay_endpoint,
                    channel_expected_pid,
                    channel_shutdown,
                    channel_ready_tx,
                    owner_operation_tx,
                    channel_product_identity,
                )
            }) {
            Ok(server) => server,
            Err(_) => {
                session_bytes.zeroize();
                shutdown.store(true, Ordering::Release);
                let _ = server.join();
                remove_failed_start_files(&session_file);
                return Err(DesktopLaunchError::new(
                    "BROKER_CHANNEL_START_FAILED",
                    "画面と安全Brokerの通信路を準備できません。",
                ));
            }
        };
        match channel_ready_rx.recv_timeout(STARTUP_TIMEOUT) {
            Ok(()) => {}
            Err(_) => {
                shutdown.store(true, Ordering::Release);
                let _ = channel_server.join();
                let _ = server.join();
                session_bytes.zeroize();
                remove_failed_start_files(&session_file);
                return Err(DesktopLaunchError::new(
                    "BROKER_CHANNEL_START_FAILED",
                    "画面と安全Brokerの通信路を準備できません。",
                ));
            }
        }
        Ok(Self {
            shutdown,
            server: Some(server),
            channel_server: Some(channel_server),
            channel_pipe_name,
            frontend_pid,
            session_file,
            session_bytes,
        })
    }

    fn has_stopped(&self) -> bool {
        self.server.as_ref().is_none_or(JoinHandle::is_finished)
            || self
                .channel_server
                .as_ref()
                .is_none_or(JoinHandle::is_finished)
    }

    fn finish(&mut self) -> Result<(), DesktopLaunchError> {
        self.shutdown.store(true, Ordering::Release);
        let channel_result = self
            .channel_server
            .take()
            .map(|server| match server.join() {
                Ok(Ok(())) => Ok(()),
                Ok(Err(_)) | Err(_) => Err(DesktopLaunchError::new(
                    "BROKER_CHANNEL_SHUTDOWN_FAILED",
                    "画面と安全Brokerの通信路を正常に終了できませんでした。",
                )),
            });
        let server_result = self.server.take().map(|server| match server.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) | Err(_) => Err(DesktopLaunchError::new(
                "BROKER_SHUTDOWN_FAILED",
                "安全Brokerを正常に終了できませんでした。",
            )),
        });
        let session_result = remove_session_if_unchanged(&self.session_file, &self.session_bytes);
        let temporary_result = remove_managed_file(&self.session_file.with_extension("json.tmp"));
        self.session_bytes.zeroize();
        session_result?;
        temporary_result?;
        channel_result.unwrap_or(Ok(()))?;
        server_result.unwrap_or(Ok(()))
    }
}

impl Drop for RunningBroker {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

fn wait_for_frontend(
    frontend: &mut Child,
    broker: &RunningBroker,
) -> Result<ExitStatus, DesktopLaunchError> {
    loop {
        match frontend.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(_) => {
                let _ = frontend.kill();
                let _ = frontend.wait();
                return Err(DesktopLaunchError::new(
                    "FRONTEND_STATUS_UNAVAILABLE",
                    "D4 Pocketの実行状態を確認できません。",
                ));
            }
        }
        if broker.has_stopped() {
            let _ = frontend.kill();
            let _ = frontend.wait();
            return Err(DesktopLaunchError::new(
                "BROKER_STOPPED",
                "安全Brokerが停止したため、D4 Pocketを安全のため終了しました。",
            ));
        }
        thread::sleep(UI_POLL);
    }
}

fn launch_frontend(
    layout: &PackageLayout,
    broker: &RunningBroker,
) -> Result<ExitStatus, DesktopLaunchError> {
    let mut command = Command::new(&layout.app_exe);
    configure_frontend_environment(
        &mut command,
        std::env::vars_os(),
        OsStr::new(&broker.channel_pipe_name),
    );
    command.current_dir(&layout.app_dir);
    let mut frontend = command.spawn().map_err(|_| {
        DesktopLaunchError::new(
            "FRONTEND_START_FAILED",
            "D4 Pocket画面を起動できません。製品ファイルを確認してください。",
        )
    })?;
    broker.frontend_pid.store(frontend.id(), Ordering::Release);
    wait_for_frontend(&mut frontend, broker)
}

fn filtered_frontend_environment(
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
) -> Vec<(OsString, OsString)> {
    inherited
        .into_iter()
        .filter(|(name, _)| {
            name.to_str().is_some_and(|name| {
                FRONTEND_ENVIRONMENT_ALLOWLIST
                    .iter()
                    .any(|allowed| name.eq_ignore_ascii_case(allowed))
            })
        })
        .collect()
}

fn configure_frontend_environment(
    command: &mut Command,
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
    channel_pipe_name: &OsStr,
) {
    command
        .env_clear()
        .envs(filtered_frontend_environment(inherited))
        .env("GUI_SHELL_BROKER_CHANNEL_PIPE", channel_pipe_name);
}

pub fn run() -> Result<(), DesktopLaunchError> {
    if std::env::args_os().len() != 1 {
        return Err(DesktopLaunchError::new(
            "UNSUPPORTED_ARGUMENTS",
            "この起動方法では追加のcommandやpathを指定できません。",
        ));
    }
    let launcher_exe = std::env::current_exe().map_err(|_| {
        DesktopLaunchError::new("LAUNCHER_PATH_INVALID", "起動元を確認できません。")
    })?;
    let layout = resolve_package_layout(&launcher_exe)?;
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| {
            DesktopLaunchError::new(
                "USER_DATA_ROOT_INVALID",
                "Windowsのユーザー保存先を確認できません。",
            )
        })?;
    let product_identity = compiled_product_runtime_identity()?;
    let runtime_dir =
        runtime_directory_with_identity(&local_app_data, product_identity.as_ref())?;
    let _instance_lock = acquire_instance_lock(&runtime_dir)?;
    let mut broker = RunningBroker::start(&runtime_dir, true, product_identity)?;

    let frontend_result = launch_frontend(&layout, &broker);
    let broker_result = broker.finish();
    broker_result?;
    let status = frontend_result?;
    if !status.success() {
        return Err(DesktopLaunchError::new(
            "FRONTEND_EXIT_FAILED",
            "D4 Pocketが正常に終了しませんでした。",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root(label: &str) -> PathBuf {
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).expect("乱数識別子");
        let path = std::env::temp_dir().join(format!(
            "gui-shell-launcher-{label}-{}",
            hex::encode(random)
        ));
        fs::create_dir_all(&path).expect("一時作業ディレクトリ");
        path
    }

    #[test]
    fn flutter_child_environment_is_allowlisted() {
        let system_root = std::env::var_os("SystemRoot").expect("SystemRoot環境変数がない");
        let system32 = PathBuf::from(&system_root).join("System32");
        let command_exe = system32.join("cmd.exe");
        let inherited = vec![
            (OsString::from("SystemRoot"), system_root),
            (
                OsString::from("OPENAI_API_KEY"),
                OsString::from("secret-marker-openai"),
            ),
            (
                OsString::from("CODEX_HOME"),
                OsString::from("secret-marker-codex"),
            ),
            (OsString::from("PATH"), OsString::from("secret-marker-path")),
            (
                OsString::from("GUI_SHELL_BROKER_RUNTIME_DIR"),
                OsString::from("private-runtime-path"),
            ),
            (
                OsString::from("GUI_SHELL_BROKER_ENDPOINT_JSON"),
                OsString::from("endpoint-marker"),
            ),
            (
                OsString::from("GUI_SHELL_BROKER_SESSION_JSON"),
                OsString::from("session-marker"),
            ),
            (
                OsString::from("GUI_SHELL_PRODUCT_APP_ID"),
                OsString::from("d4-pocket-app-0123456789abcdef0123456789abcdef"),
            ),
            (
                OsString::from("GUI_SHELL_PRODUCT_AUDIT_STORE_ID"),
                OsString::from("audit-store-0123456789abcdef0123456789abcdef"),
            ),
        ];
        let mut command = Command::new(command_exe);
        configure_frontend_environment(
            &mut command,
            inherited,
            OsStr::new(r"\\.\pipe\D4PocketBroker-0123456789abcdef0123456789abcdef"),
        );
        let output = command
            .args(["/d", "/c", "set"])
            .output()
            .expect("隔離環境の確認process");
        assert!(output.status.success(), "環境確認processが失敗した");
        let environment = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        assert!(environment.contains("systemroot="));
        assert!(environment.contains("gui_shell_broker_channel_pipe="));
        for marker in [
            "openai_api_key",
            "secret-marker-openai",
            "codex_home",
            "secret-marker-codex",
            "path=secret-marker-path",
            "gui_shell_broker_runtime_dir",
            "private-runtime-path",
            "gui_shell_broker_endpoint_json",
            "endpoint-marker",
            "gui_shell_broker_session_json",
            "session-marker",
            "gui_shell_product_app_id",
            "d4-pocket-app-0123456789abcdef0123456789abcdef",
            "gui_shell_product_audit_store_id",
            "audit-store-0123456789abcdef0123456789abcdef",
        ] {
            assert!(
                !environment.contains(marker),
                "親process環境が子processへ漏れた: {marker}"
            );
        }
    }

    fn endpoint() -> BrokerEndpoint {
        BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 43123,
            session_id: "broker-session-0123456789abcdef0123456789abcdef".into(),
            session_secret: "a".repeat(64),
            credential_role: BrokerCredentialRole::Normal,
            transport: "authenticated_loopback_tcp".into(),
            max_request_bytes: MAX_REQUEST_BYTES,
        }
    }

    fn desktop_export_request(request_id: &str, nonce: &str) -> serde_json::Value {
        let payload = serde_json::json!({
            "version": 1,
            "export_id": "export-desktop-test",
            "compose_manifest": {
                "version": 1,
                "compose_id": "desktop-test",
                "display_name": "D4 Pocket Desktop test",
                "runtime_ids": ["gui_shell_rust_broker"],
                "agent_ids": ["codex"],
                "tool_ids": [],
                "mcp_connection_ids": [],
                "theme": {"theme_id": "d4-pocket", "mode": "system"},
                "capability_requirements": ["runtime.read"],
                "settings": {
                    "locale": "ja-JP",
                    "density": "comfortable",
                    "content_visibility": "summary"
                },
                "inheritance_policy": {
                    "authority": "none",
                    "permission": "none",
                    "approval": "none",
                    "credential": "none",
                    "audit_chain": "none"
                },
                "output_mode": "manifest_only"
            },
            "target_platform": "windows",
            "export_mode": "manifest_file",
            "distribution_channel": "local",
            "module_selection": {"optional_module_ids": ["shell.history"]}
        });
        serde_json::json!({
            "request_id": request_id,
            "operation": "GUI Shell書出し",
            "payload_hash": canonical_payload_hash(Some(&payload)),
            "nonce": nonce,
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"},
            "payload": payload
        })
    }

    fn desktop_owner_request(
        operation: &str,
        request_id: &str,
        nonce: &str,
        payload: serde_json::Value,
    ) -> serde_json::Value {
        serde_json::json!({
            "request_id": request_id,
            "operation": operation,
            "payload_hash": canonical_payload_hash(Some(&payload)),
            "nonce": nonce,
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"},
            "payload": payload
        })
    }

    #[test]
    fn package_layout_requires_fixed_sibling_artifacts() {
        let root = test_root("layout");
        let app = root.join("app");
        let broker = root.join("broker");
        fs::create_dir_all(app.join("data")).unwrap();
        fs::create_dir_all(&broker).unwrap();
        for path in [
            app.join("gui_shell_desktop.exe"),
            app.join("flutter_windows.dll"),
            app.join("data").join("app.so"),
        ] {
            fs::write(path, b"test artifact").unwrap();
        }
        fs::write(app.join("data").join("icudtl.dat"), b"icu").unwrap();
        fs::create_dir(app.join("data").join("flutter_assets")).unwrap();
        fs::write(broker.join("gui_shell_rust_helper.exe"), b"test broker").unwrap();
        fs::write(
            root.join("gui_shell_desktop_launcher.exe"),
            b"test launcher",
        )
        .unwrap();
        let layout = resolve_package_layout(&root.join("gui_shell_desktop_launcher.exe")).unwrap();
        assert_eq!(
            layout.app_exe,
            fs::canonicalize(app.join("gui_shell_desktop.exe")).unwrap()
        );

        fs::remove_file(broker.join("gui_shell_rust_helper.exe")).unwrap();
        let error =
            resolve_package_layout(&root.join("gui_shell_desktop_launcher.exe")).unwrap_err();
        assert_eq!(error.code, "PACKAGE_FILE_MISSING");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn endpoint_requires_loopback_normal_role_and_bounded_secret() {
        let valid = endpoint();
        validate_normal_endpoint(&valid).unwrap();

        let mut invalid = valid.clone();
        invalid.host = "0.0.0.0".into();
        assert_eq!(
            validate_normal_endpoint(&invalid).unwrap_err().code,
            "BROKER_ENDPOINT_INVALID"
        );
        let mut invalid = valid.clone();
        invalid.credential_role = BrokerCredentialRole::Owner;
        let error = validate_normal_endpoint(&invalid).unwrap_err();
        assert_eq!(error.code, "BROKER_ENDPOINT_INVALID");
        assert!(!error.dialog_text().contains(&valid.session_secret));
        let mut invalid = valid.clone();
        invalid.transport = "unauthenticated_tcp".into();
        assert!(validate_normal_endpoint(&invalid).is_err());
        let mut invalid = valid.clone();
        invalid.session_secret = "A".repeat(64);
        assert!(validate_normal_endpoint(&invalid).is_err());
        let mut invalid = valid;
        invalid.max_request_bytes = MAX_REQUEST_BYTES + 1;
        assert!(validate_normal_endpoint(&invalid).is_err());
    }

    #[test]
    fn endpoint_cleanup_removes_only_the_original_bytes() {
        let root = test_root("endpoint-cleanup");
        let path = root.join(SESSION_FILE);
        let expected = serde_json::to_vec(&endpoint()).unwrap();
        fs::write(&path, &expected).unwrap();
        remove_session_if_unchanged(&path, &expected).unwrap();
        assert!(!path.exists());

        fs::write(&path, &expected).unwrap();
        fs::write(&path, b"replacement endpoint").unwrap();
        assert_eq!(
            remove_session_if_unchanged(&path, &expected)
                .unwrap_err()
                .code,
            "SESSION_FILE_CHANGED"
        );
        assert!(path.exists(), "changed file must not be deleted");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn desktop_channel_relay_reuses_normal_broker_auth_and_audited_rejections() {
        let root = test_root("desktop-channel-relay");
        let session_file = root.join(SESSION_FILE);
        let store_dir = root.join("store");
        let shutdown = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let server_shutdown = Arc::clone(&shutdown);
        let server_session_file = session_file.clone();
        let server_store_dir = store_dir.clone();
        let server = thread::spawn(move || {
            run_loopback_server_cancellable(
                BrokerServerConfig::new(server_store_dir, server_session_file),
                server_shutdown,
                ready_tx,
            )
        });
        ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let (mut session_bytes, mut endpoint) = read_endpoint(&session_file).unwrap();

        let request = serde_json::json!({
            "request_id": "desktop-channel-health-1",
            "operation": "health",
            "payload_hash": "sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b",
            "nonce": "desktop-channel-nonce-1",
            "issued_at": crate::broker::BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        let accepted = relay_channel_frame(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&request).unwrap(),
            ),
            &endpoint,
        )
        .unwrap();
        let accepted: serde_json::Value = serde_json::from_slice(&accepted).unwrap();
        assert_eq!(accepted["status"], "accepted");
        assert_eq!(accepted["operation"], "health");
        assert_eq!(accepted["request_id"], "desktop-channel-health-1");

        let replayed = relay_channel_frame(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&request).unwrap(),
            ),
            &endpoint,
        )
        .unwrap();
        let replayed: serde_json::Value = serde_json::from_slice(&replayed).unwrap();
        assert_eq!(replayed["status"], "rejected");
        assert_eq!(replayed["error"]["code"], "broker_replay_detected");

        let owner_request = serde_json::json!({
            "request_id": "desktop-channel-owner-operation-1",
            "operation": "作業領域承認",
            "payload_hash": "sha256:d3626ac30a87e6f7a6428233b3c68299976865fa5508e4267c5415c76af7a772",
            "nonce": "desktop-channel-owner-nonce-1",
            "issued_at": crate::broker::BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"},
            "payload": {"b": 1, "a": 2}
        });
        let owner_rejected = relay_channel_frame(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&owner_request).unwrap(),
            ),
            &endpoint,
        )
        .unwrap();
        let owner_rejected: serde_json::Value = serde_json::from_slice(&owner_rejected).unwrap();
        assert_eq!(owner_rejected["status"], "rejected");
        assert_eq!(owner_rejected["error"]["code"], "権限拒否");

        let stale = serde_json::json!({
            "request_id": "desktop-channel-stale-1",
            "operation": "health",
            "payload_hash": "sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b",
            "nonce": "desktop-channel-stale-nonce-1",
            "issued_at": "2000-01-01T00:00:00Z",
            "metadata": {"client": "desktop_flutter"}
        });
        let stale = relay_channel_frame(
            gui_shell_windows_broker_channel::PipeFrame::Line(serde_json::to_vec(&stale).unwrap()),
            &endpoint,
        )
        .unwrap();
        let stale: serde_json::Value = serde_json::from_slice(&stale).unwrap();
        assert_eq!(stale["status"], "rejected");
        assert_eq!(stale["error"]["code"], "broker_issued_at_invalid");

        for (field, value) in [
            ("session_secret", serde_json::json!("not-a-credential")),
            ("credential_role", serde_json::json!("owner")),
            ("authority", serde_json::json!({"approved": true})),
        ] {
            let mut injected = request.clone();
            injected[field] = value;
            let rejected = relay_channel_frame(
                gui_shell_windows_broker_channel::PipeFrame::Line(
                    serde_json::to_vec(&injected).unwrap(),
                ),
                &endpoint,
            )
            .unwrap();
            let rejected: serde_json::Value = serde_json::from_slice(&rejected).unwrap();
            assert_eq!(rejected["status"], "rejected");
            assert_eq!(rejected["error"]["code"], "broker_request_malformed");
        }

        let mut injected_session = request.clone();
        injected_session["session_id"] = serde_json::Value::String("forged-session".into());
        let rejected = relay_channel_frame(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&injected_session).unwrap(),
            ),
            &endpoint,
        )
        .unwrap();
        let rejected: serde_json::Value = serde_json::from_slice(&rejected).unwrap();
        assert_eq!(rejected["status"], "rejected");
        assert_eq!(rejected["error"]["code"], "broker_request_malformed");

        let malformed = relay_channel_frame(
            gui_shell_windows_broker_channel::PipeFrame::Line(b"{".to_vec()),
            &endpoint,
        )
        .unwrap();
        let malformed: serde_json::Value = serde_json::from_slice(&malformed).unwrap();
        assert_eq!(malformed["status"], "rejected");
        assert_eq!(malformed["error"]["code"], "broker_request_malformed");

        let oversized = relay_channel_frame(
            gui_shell_windows_broker_channel::PipeFrame::Oversized,
            &endpoint,
        )
        .unwrap();
        let oversized: serde_json::Value = serde_json::from_slice(&oversized).unwrap();
        assert_eq!(oversized["status"], "rejected");
        assert_eq!(oversized["error"]["code"], "broker_request_oversized");

        shutdown.store(true, Ordering::Release);
        server.join().unwrap().unwrap();
        let mut unavailable_endpoint = endpoint.clone();
        unavailable_endpoint.port = 0;
        assert!(relay_channel_frame(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&request).unwrap()
            ),
            &unavailable_endpoint,
        )
        .is_none());
        let audit = fs::read_to_string(store_dir.join("audit.jsonl")).unwrap();
        assert!(audit.contains("broker_request_malformed"));
        assert!(audit.contains("broker_request_oversized"));
        endpoint.session_secret.zeroize();
        session_bytes.zeroize();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn desktop_owner_allowlist_requires_native_confirmation_and_broker_audits_both_outcomes() {
        let root = test_root("desktop-owner-export");
        let session_file = root.join(SESSION_FILE);
        let store_dir = root.join("store");
        let protected_store_dir = root.join("protected");
        let export_dir = root.join(EXPORT_DIRECTORY_NAME);
        fs::create_dir(&protected_store_dir).unwrap();
        fs::create_dir(&export_dir).unwrap();
        let shutdown = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (owner_operation_tx, owner_operation_rx) = mpsc::sync_channel(1);
        let server_shutdown = Arc::clone(&shutdown);
        let server_session_file = session_file.clone();
        let server_store_dir = store_dir.clone();
        let server_protected_store_dir = protected_store_dir.clone();
        let server_export_dir = export_dir.clone();
        let server_export_root = cap_std::fs::Dir::open_ambient_dir(
            &server_export_dir,
            cap_std::ambient_authority(),
        )
        .unwrap();
        let server = thread::spawn(move || {
            let mut config = BrokerServerConfig::new(server_store_dir, server_session_file);
            config.desktop_protected_store_dir = Some(server_protected_store_dir);
            crate::broker::ipc_server::run_loopback_server_cancellable_with_owner_operations(
                config,
                server_shutdown,
                ready_tx,
                owner_operation_rx,
                Some((server_export_dir, server_export_root)),
            )
        });
        ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let (mut session_bytes, mut broker_endpoint) = read_endpoint(&session_file).unwrap();

        let normal = desktop_export_request("desktop-export-normal", "desktop-export-normal-nonce");
        let normal = relay_channel_frame(
            gui_shell_windows_broker_channel::PipeFrame::Line(serde_json::to_vec(&normal).unwrap()),
            &broker_endpoint,
        ).unwrap();
        let normal: serde_json::Value = serde_json::from_slice(&normal).unwrap();
        assert_eq!(normal["status"], "rejected");
        assert_eq!(normal["error"]["code"], "owner_required");

        let confirmed = desktop_export_request("desktop-export-confirmed", "desktop-export-confirmed-nonce");
        let mut prompt_count = 0;
        let accepted = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(serde_json::to_vec(&confirmed).unwrap()),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                prompt_count += 1;
                let DesktopOwnerOperationSummary::GuiShellExport(summary) = summary else {
                    panic!("Export要求はExport固有の確認summaryを使う")
                };
                assert_eq!(summary.export_id, "export-desktop-test");
                assert_eq!(summary.optional_module_count, 1);
                assert_eq!(summary.payload_hash, confirmed["payload_hash"].as_str().unwrap());
                true
            },
        ).unwrap();
        let accepted: serde_json::Value = serde_json::from_slice(&accepted).unwrap();
        assert_eq!(prompt_count, 1);
        assert_eq!(accepted["status"], "accepted");
        assert_eq!(accepted["request_id"], "desktop-export-confirmed");
        assert_eq!(accepted["body"]["authority_strip"], true);
        assert_eq!(accepted["body"]["credential_inherited"], false);
        assert_eq!(accepted["body"]["manifest_file_status"], "written");
        let manifest_path = accepted["body"]["manifest_file"]["path"].as_str().unwrap();
        let manifest_bytes = fs::read(manifest_path).unwrap();
        assert_eq!(
            crate::audit_hash::sha256_tagged(&manifest_bytes),
            accepted["body"]["manifest_file"]["sha256"]
        );
        let manifest_file: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
        assert_eq!(manifest_file["product"], "D4 Pocket");
        assert_eq!(manifest_file["manifest"]["inheritance_policy"]["credential"], "none");

        let delete_payload = serde_json::json!({
            "版": 1,
            "回帰CaseID": "cccccccccccccccccccccccccccccccc",
            "定義hash": format!("sha256:{}", "d".repeat(64)),
            "暗号文hash": format!("sha256:{}", "e".repeat(64))
        });
        let delete = desktop_owner_request(
            "回帰Case削除",
            "desktop-delete-confirmed",
            "desktop-delete-confirmed-nonce",
            delete_payload,
        );
        let mut delete_prompt_count = 0;
        let delete_response = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(serde_json::to_vec(&delete).unwrap()),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                delete_prompt_count += 1;
                let DesktopOwnerOperationSummary::RegressionCaseDelete {
                    summary,
                    payload_hash,
                } = summary
                else {
                    panic!("削除はCase IDと両hashを示す専用確認を使う")
                };
                assert_eq!(summary.case_id, "cccccccccccccccccccccccccccccccc");
                assert_eq!(summary.definition_hash, format!("sha256:{}", "d".repeat(64)));
                assert_eq!(summary.ciphertext_hash, format!("sha256:{}", "e".repeat(64)));
                assert_eq!(payload_hash, delete["payload_hash"].as_str().unwrap());
                let text = owner_confirmation_text(
                    &DesktopOwnerOperationSummary::RegressionCaseDelete {
                        summary: (*summary).clone(),
                        payload_hash: (*payload_hash).clone(),
                    },
                );
                assert!(text.contains("物理消去は保証しません"));
                true
            },
        )
        .unwrap();
        let delete_response: serde_json::Value = serde_json::from_slice(&delete_response).unwrap();
        assert_eq!(delete_prompt_count, 1);
        assert_eq!(delete_response["status"], "rejected");
        assert_eq!(delete_response["error"]["code"], "削除対象不明");

        let recovery_payload = serde_json::json!({
            "版": 1,
            "回帰CaseID": "cccccccccccccccccccccccccccccccc"
        });
        let recovery = desktop_owner_request(
            "回帰Case削除中断確認",
            "desktop-recovery-confirmed",
            "desktop-recovery-confirmed-nonce",
            recovery_payload,
        );
        let mut recovery_prompt_count = 0;
        let recovery_response = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&recovery).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                recovery_prompt_count += 1;
                let DesktopOwnerOperationSummary::RegressionCaseRecovery {
                    summary,
                    payload_hash,
                } = summary
                else {
                    panic!("中断照合はCase IDだけを示す専用確認を使う")
                };
                assert_eq!(summary.case_id, "cccccccccccccccccccccccccccccccc");
                assert_eq!(payload_hash, recovery["payload_hash"].as_str().unwrap());
                assert!(owner_confirmation_text(
                    &DesktopOwnerOperationSummary::RegressionCaseRecovery {
                        summary: (*summary).clone(),
                        payload_hash: (*payload_hash).clone(),
                    },
                )
                .contains("自動再削除は行いません"));
                true
            },
        )
        .unwrap();
        let recovery_response: serde_json::Value =
            serde_json::from_slice(&recovery_response).unwrap();
        assert_eq!(recovery_prompt_count, 1);
        assert_eq!(recovery_response["status"], "rejected");
        assert_eq!(recovery_response["error"]["code"], "復旧対象不明");

        let registration_payload = serde_json::json!({
            "版": 1,
            "要求ID": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "要求hash": format!("sha256:{}", "b".repeat(64)),
            "公開表示名": "回帰Case登録試験",
            "入力方式": "owner_explicit_redacted",
            "入力": {"内容表示範囲": "full", "本文": "owner-authored-redacted-input"},
            "必要条件": ["private-required-condition"],
            "禁止条件": [],
            "期待状態": "成功",
            "必要参照": ["private-reference"],
            "期待経路": "private-route"
        });
        let registration = desktop_owner_request(
            "回帰Case登録",
            "desktop-registration-confirmed",
            "desktop-registration-confirmed-nonce",
            registration_payload,
        );
        let mut registration_prompt_count = 0;
        let registration_response = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&registration).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                registration_prompt_count += 1;
                let DesktopOwnerOperationSummary::RegressionCaseRegistration {
                    summary,
                    payload_hash,
                } = summary
                else {
                    panic!("登録は公開要約だけを表示する専用native確認を使う")
                };
                assert_eq!(summary.request_id, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
                assert_eq!(summary.display_name, "回帰Case登録試験");
                assert_eq!(summary.input_characters, "owner-authored-redacted-input".chars().count());
                assert_eq!(summary.required_condition_count, 1);
                assert_eq!(payload_hash, registration["payload_hash"].as_str().unwrap());
                let text = owner_confirmation_text(
                    &DesktopOwnerOperationSummary::RegressionCaseRegistration {
                        summary: (*summary).clone(),
                        payload_hash: (*payload_hash).clone(),
                    },
                );
                assert!(text.contains("現在の対話証跡"));
                assert!(text.contains(&summary.request_hash));
                assert!(!text.contains("owner-authored-redacted-input"));
                assert!(!text.contains("private-required-condition"));
                assert!(!text.contains("private-reference"));
                assert!(!text.contains("private-route"));
                true
            },
        )
        .unwrap();
        let registration_response: serde_json::Value =
            serde_json::from_slice(&registration_response).unwrap();
        assert_eq!(registration_prompt_count, 1);
        assert_eq!(registration_response["status"], "rejected");
        assert_eq!(registration_response["error"]["code"], "対話証跡不在");

        let mut secret_registration = registration.clone();
        secret_registration["request_id"] = serde_json::Value::String(
            "desktop-registration-secret-marker".into(),
        );
        secret_registration["nonce"] =
            serde_json::Value::String("desktop-registration-secret-marker-nonce".into());
        secret_registration["payload"]["入力"]["本文"] =
            serde_json::Value::String("api_key=must-not-be-shown".into());
        secret_registration["payload_hash"] = serde_json::Value::String(
            canonical_payload_hash(secret_registration.get("payload")).to_string(),
        );
        let mut secret_prompt_count = 0;
        let secret_response = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&secret_registration).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |_| {
                secret_prompt_count += 1;
                true
            },
        )
        .unwrap();
        let secret_response: serde_json::Value = serde_json::from_slice(&secret_response).unwrap();
        assert_eq!(secret_prompt_count, 0);
        assert_eq!(secret_response["status"], "rejected");
        assert_eq!(secret_response["error"]["code"], "権限拒否");

        let stale_approval = desktop_owner_request(
            "回帰Case削除中断確認",
            "desktop-recovery-stale-approval",
            "desktop-recovery-stale-approval-nonce",
            serde_json::json!({
                "版": 1,
                "回帰CaseID": "cccccccccccccccccccccccccccccccc",
                "削除承認監査ID": "audit.history.must-not-be-authority"
            }),
        );
        let stale_approval_response = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&stale_approval).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |_| panic!("過去の承認IDを含むRecovery要求はnative確認へ進めない"),
        )
        .unwrap();
        let stale_approval_response: serde_json::Value =
            serde_json::from_slice(&stale_approval_response).unwrap();
        assert_eq!(stale_approval_response["status"], "rejected");
        assert_eq!(stale_approval_response["error"]["code"], "権限拒否");

        let declined = desktop_export_request("desktop-export-declined", "desktop-export-declined-nonce");
        let declined = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(serde_json::to_vec(&declined).unwrap()),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |_| false,
        ).unwrap();
        let declined: serde_json::Value = serde_json::from_slice(&declined).unwrap();
        assert_eq!(declined["status"], "rejected");
        assert_eq!(declined["error"]["code"], "owner_required");

        let mut changed_hash = desktop_export_request("desktop-export-changed-hash", "desktop-export-changed-hash-nonce");
        changed_hash["payload_hash"] = serde_json::Value::String("sha256:forged".into());
        let mut invalid_prompt_count = 0;
        let rejected = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(serde_json::to_vec(&changed_hash).unwrap()),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |_| { invalid_prompt_count += 1; true },
        ).unwrap();
        let rejected: serde_json::Value = serde_json::from_slice(&rejected).unwrap();
        assert_eq!(invalid_prompt_count, 0);
        assert_eq!(rejected["error"]["code"], "broker_payload_hash_invalid");

        let mut authority_metadata = desktop_export_request("desktop-export-authority-metadata", "desktop-export-authority-metadata-nonce");
        authority_metadata["metadata"]["authority"] = serde_json::Value::Bool(true);
        let rejected = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(serde_json::to_vec(&authority_metadata).unwrap()),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |_| panic!("権限metadataで確認画面へ到達してはならない"),
        ).unwrap();
        let rejected: serde_json::Value = serde_json::from_slice(&rejected).unwrap();
        assert_eq!(rejected["error"]["code"], "broker_authority_metadata_rejected");

        let mut forged_session = desktop_export_request("desktop-export-forged-session", "desktop-export-forged-session-nonce");
        forged_session["session_id"] = serde_json::Value::String("forged-session".into());
        let rejected = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(serde_json::to_vec(&forged_session).unwrap()),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |_| panic!("偽造sessionで確認画面へ到達してはならない"),
        ).unwrap();
        let rejected: serde_json::Value = serde_json::from_slice(&rejected).unwrap();
        assert_eq!(rejected["error"]["code"], "broker_request_malformed");

        let mut stale = desktop_export_request("desktop-export-stale", "desktop-export-stale-nonce");
        stale["issued_at"] = serde_json::Value::String("2000-01-01T00:00:00Z".into());
        let rejected = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(serde_json::to_vec(&stale).unwrap()),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |_| panic!("期限切れ要求で確認画面へ到達してはならない"),
        ).unwrap();
        let rejected: serde_json::Value = serde_json::from_slice(&rejected).unwrap();
        assert_eq!(rejected["error"]["code"], "broker_issued_at_invalid");

        let non_export = serde_json::json!({
            "request_id": "desktop-owner-operation-not-export",
            "operation": "資格情報一覧",
            "payload_hash": canonical_payload_hash(None),
            "nonce": "desktop-owner-operation-not-export-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        let rejected = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(serde_json::to_vec(&non_export).unwrap()),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |_| panic!("allowlist外のOwner操作で確認画面を表示してはならない"),
        ).unwrap();
        let rejected: serde_json::Value = serde_json::from_slice(&rejected).unwrap();
        assert_eq!(rejected["status"], "rejected");

        shutdown.store(true, Ordering::Release);
        server.join().unwrap().unwrap();
        let audit = fs::read_to_string(store_dir.join("audit.jsonl")).unwrap();
        assert!(audit.contains("Rust Desktop起動器のネイティブ確認"));
        assert!(audit.contains("Desktop固定ProtectedStore起動"));
        assert!(audit.contains("owner_required"));
        assert!(audit.contains("broker_payload_hash_invalid"));
        broker_endpoint.session_secret.zeroize();
        session_bytes.zeroize();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn desktop_channel_normalization_never_accepts_ui_session_identity() {
        let valid = br#"{"request_id":"r1","operation":"health"}"#;
        let normalized = normalize_channel_request(valid, "broker-session-owned");
        let normalized: serde_json::Value = serde_json::from_slice(&normalized).unwrap();
        assert_eq!(normalized["session_id"], "broker-session-owned");
        assert!(normalized
            .get("desktop_channel_session_id_forbidden")
            .is_none());

        let forged = br#"{"request_id":"r2","operation":"health","session_id":"fake"}"#;
        let forged = normalize_channel_request(forged, "broker-session-owned");
        let forged: serde_json::Value = serde_json::from_slice(&forged).unwrap();
        assert_eq!(forged["session_id"], "fake");
        assert_eq!(forged["desktop_channel_session_id_forbidden"], true);

        let malformed = b"{";
        assert_eq!(normalize_channel_request(malformed, "session"), malformed);
        let non_object = b"[]";
        assert_eq!(normalize_channel_request(non_object, "session"), non_object);
    }

    #[test]
    fn runtime_state_is_separate_from_installation_and_lock_prevents_duplicate_launch() {
        let root = fs::canonicalize(test_root("runtime")).unwrap();
        let runtime = runtime_directory_with_identity(&root, None).unwrap();
        assert!(runtime.starts_with(&root));
        assert!(runtime.ends_with(Path::new("GUI-Shell").join("broker").join("desktop")));
        let export_dir = ensure_export_directory(&runtime).unwrap();
        assert_eq!(export_dir, runtime.join(EXPORT_DIRECTORY_NAME));
        let first = acquire_instance_lock(&runtime).unwrap();
        let second = acquire_instance_lock(&runtime).unwrap_err();
        assert_eq!(second.code, "INSTANCE_ALREADY_RUNNING");
        drop(first);
        assert!(acquire_instance_lock(&runtime).is_ok());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn product_runtime_identity_requires_both_generated_ids() {
        let app_id = "d4-pocket-app-0123456789abcdef0123456789abcdef";
        let audit_store_id = "audit-store-fedcba9876543210fedcba9876543210";
        assert!(product_runtime_identity(None, None).unwrap().is_none());
        assert!(product_runtime_identity(Some(app_id), Some(audit_store_id))
            .unwrap()
            .is_some());

        for (candidate_app_id, candidate_store_id) in [
            (Some(app_id), None),
            (None, Some(audit_store_id)),
            (Some("d4-pocket-app-../outside"), Some(audit_store_id)),
            (
                Some("d4-pocket-app-0123456789ABCDEF0123456789ABCDEF"),
                Some(audit_store_id),
            ),
            (Some(app_id), Some("audit-store-example")),
        ] {
            let error = product_runtime_identity(candidate_app_id, candidate_store_id).unwrap_err();
            assert_eq!(error.code, "PRODUCT_RUNTIME_IDENTITY_INVALID");
        }
    }

    #[test]
    fn exported_products_get_distinct_app_and_audit_store_directories() {
        let root = fs::canonicalize(test_root("runtime-product-identities")).unwrap();
        let first = product_runtime_identity(
            Some("d4-pocket-app-0123456789abcdef0123456789abcdef"),
            Some("audit-store-0123456789abcdef0123456789abcdef"),
        )
        .unwrap()
        .unwrap();
        let second = product_runtime_identity(
            Some("d4-pocket-app-fedcba9876543210fedcba9876543210"),
            Some("audit-store-fedcba9876543210fedcba9876543210"),
        )
        .unwrap()
        .unwrap();
        let first_runtime = runtime_directory_with_identity(&root, Some(&first)).unwrap();
        let second_runtime = runtime_directory_with_identity(&root, Some(&second)).unwrap();
        assert_ne!(first_runtime, second_runtime);
        assert!(first_runtime.starts_with(&root));
        assert!(second_runtime.starts_with(&root));
        assert!(first_runtime.ends_with(
            Path::new("D4Pocket")
                .join("apps")
                .join(&first.app_id)
                .join("stores")
                .join(&first.audit_store_id)
        ));
        assert!(second_runtime.ends_with(
            Path::new("D4Pocket")
                .join("apps")
                .join(&second.app_id)
                .join("stores")
                .join(&second.audit_store_id)
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_product_identity_is_rejected_before_creating_user_data() {
        let root = fs::canonicalize(test_root("runtime-invalid-product-identity")).unwrap();
        let identity = ProductRuntimeIdentity {
            app_id: "d4-pocket-app-../outside".into(),
            audit_store_id: "audit-store-0123456789abcdef0123456789abcdef".into(),
        };

        let error = runtime_directory_with_identity(&root, Some(&identity)).unwrap_err();
        assert_eq!(error.code, "PRODUCT_RUNTIME_IDENTITY_INVALID");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn runtime_directory_uses_only_embedded_identity_values() {
        let root = fs::canonicalize(test_root("runtime-embedded-identity")).unwrap();
        let identity = compiled_product_runtime_identity().unwrap();
        let runtime = runtime_directory(&root).unwrap();
        let expected = runtime_directory_with_identity(&root, identity.as_ref()).unwrap();
        assert_eq!(runtime, expected);
        if let Some(identity) = identity {
            assert!(runtime.ends_with(
                Path::new("D4Pocket")
                    .join("apps")
                    .join(identity.app_id)
                    .join("stores")
                    .join(identity.audit_store_id)
            ));
        } else {
            assert!(runtime.ends_with(Path::new("GUI-Shell").join("broker").join("desktop")));
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn product_export_confirmation_shows_its_isolated_store_path() {
        let identity = product_runtime_identity(
            Some("d4-pocket-app-0123456789abcdef0123456789abcdef"),
            Some("audit-store-fedcba9876543210fedcba9876543210"),
        )
        .unwrap()
        .unwrap();
        let summary = DesktopOwnerOperationSummary::GuiShellExport(ExportConfirmationSummary {
            display_name: "独立構成".into(),
            export_id: "export-test".into(),
            distribution_channel: "local".into(),
            optional_module_count: 1,
            payload_hash: format!("sha256:{}", "a".repeat(64)),
        });
        let text = owner_confirmation_text_for_identity(&summary, Some(&identity));
        assert!(text.contains(&format!(
            r"%LOCALAPPDATA%\D4Pocket\apps\{}\stores\{}\exports\<新規App ID>.json",
            identity.app_id, identity.audit_store_id
        )));
        assert!(!text.contains(r"%LOCALAPPDATA%\GUI-Shell\broker\desktop\exports"));
        assert!(owner_confirmation_text(&summary)
            .contains(r"%LOCALAPPDATA%\GUI-Shell\broker\desktop\exports"));
    }

    #[test]
    #[allow(non_snake_case)]
    fn MCP切断はServer固定のnativeOwner確認を要求し余分なfieldを拒否する() {
        let payload = serde_json::json!({
            "版": 1,
            "操作": "切断",
            "ServerID": "mcp-fixture"
        });
        let payload_hash = canonical_payload_hash(Some(&payload));
        let input = serde_json::json!({
            "request_id": "desktop-mcp-disconnect",
            "operation": "MCP切断",
            "payload": payload.clone(),
            "payload_hash": payload_hash,
            "nonce": "desktop-mcp-disconnect-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        let endpoint = BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 43123,
            session_id: "desktop-session".into(),
            session_secret: "not-used-by-candidate".into(),
            credential_role: BrokerCredentialRole::Owner,
            transport: "tcp".into(),
            max_request_bytes: MAX_REQUEST_BYTES,
        };
        let (normalized, summary) = owner_operation_candidate(
            input.to_string().as_bytes(),
            &endpoint,
        )
        .expect("MCP切断はnative Owner確認候補");
        assert_eq!(
            BrokerRequestEnvelope::from_json_str(&normalized)
                .unwrap()
                .session_id
                .as_deref(),
            Some("desktop-session")
        );
        let confirmation = owner_confirmation_text(&summary);
        assert!(confirmation.contains("mcp-fixture"));
        assert!(confirmation.contains("Windows Job Object"));
        assert!(confirmation.contains("永続Audit"));
        assert!(confirmation.contains("payload hash"));
        assert!(!confirmation.contains("credential_value"));
        assert!(!confirmation.contains("実行file"));

        let mut escalated = payload.clone();
        escalated["credential_value"] = serde_json::json!("secret-marker");
        let request = serde_json::json!({
            "request_id": "desktop-mcp-disconnect-escalated",
            "operation": "MCP切断",
            "payload_hash": canonical_payload_hash(Some(&escalated)),
            "payload": escalated,
            "nonce": "desktop-mcp-disconnect-escalated-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        assert!(owner_operation_candidate(request.to_string().as_bytes(), &endpoint).is_none());

        let malformed = serde_json::json!({
            "版": 1,
            "操作": "切断",
            "ServerID": "mcp\nfixture"
        });
        let request = serde_json::json!({
            "request_id": "desktop-mcp-disconnect-malformed",
            "operation": "MCP切断",
            "payload_hash": canonical_payload_hash(Some(&malformed)),
            "payload": malformed,
            "nonce": "desktop-mcp-disconnect-malformed-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        assert!(owner_operation_candidate(request.to_string().as_bytes(), &endpoint).is_none());
    }

    #[test]
    #[allow(non_snake_case)]
    fn MCP接続はnativeOwner確認で起動範囲を示し秘密候補の引数本文を表示しない() {
        let arguments = vec!["--stdio", "--token", "secret-marker"];
        let payload = serde_json::json!({
            "版": 1,
            "操作": "接続",
            "ServerID": "mcp-fixture",
            "実行file": r"C:\Program Files\D4 Pocket\mcp-fixture.exe",
            "引数": arguments,
            "workspace": r"C:\Users\Public\D4PocketWorkspace",
            "Transport": "stdio",
            "Credential ref": {
                "credential_id": "00000000000000000000000000000000",
                "purpose": "mcp_transport",
                "target": "mcp-fixture",
                "required": false,
                "status": "missing"
            }
        });
        let payload_hash = canonical_payload_hash(Some(&payload));
        let input = serde_json::json!({
            "request_id": "desktop-mcp-connect",
            "operation": "MCP接続",
            "payload": payload.clone(),
            "payload_hash": payload_hash,
            "nonce": "desktop-mcp-connect-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        let endpoint = BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 43123,
            session_id: "desktop-session".into(),
            session_secret: "not-used-by-candidate".into(),
            credential_role: BrokerCredentialRole::Owner,
            transport: "tcp".into(),
            max_request_bytes: MAX_REQUEST_BYTES,
        };
        let (normalized, summary) = owner_operation_candidate(
            input.to_string().as_bytes(),
            &endpoint,
        )
        .expect("MCP接続はnative Owner確認候補");
        assert_eq!(
            BrokerRequestEnvelope::from_json_str(&normalized)
                .unwrap()
                .session_id
                .as_deref(),
            Some("desktop-session")
        );
        let confirmation = owner_confirmation_text(&summary);
        assert!(confirmation.contains(r"C:\Program Files\D4 Pocket\mcp-fixture.exe"));
        assert!(confirmation.contains(r"C:\Users\Public\D4PocketWorkspace"));
        assert!(confirmation.contains(&sha256_tagged(
            &serde_json::to_vec(&arguments).expect("引数hash対象")
        )));
        assert!(confirmation.contains("Credential実値は送られません"));
        assert!(confirmation.contains("payload hash"));
        assert!(!confirmation.contains("--token"));
        assert!(!confirmation.contains("secret-marker"));

        let mut escalated = payload.clone();
        escalated["Credential ref"]["secret_value"] = serde_json::json!("secret-marker");
        let request = serde_json::json!({
            "request_id": "desktop-mcp-connect-escalated",
            "operation": "MCP接続",
            "payload_hash": canonical_payload_hash(Some(&escalated)),
            "payload": escalated,
            "nonce": "desktop-mcp-connect-escalated-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        assert!(owner_operation_candidate(request.to_string().as_bytes(), &endpoint).is_none());

        let mut required_credential = payload;
        required_credential["Credential ref"]["required"] = serde_json::json!(true);
        let request = serde_json::json!({
            "request_id": "desktop-mcp-connect-required-credential",
            "operation": "MCP接続",
            "payload_hash": canonical_payload_hash(Some(&required_credential)),
            "payload": required_credential,
            "nonce": "desktop-mcp-connect-required-credential-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        assert!(owner_operation_candidate(request.to_string().as_bytes(), &endpoint).is_none());
    }

    #[test]
    #[allow(non_snake_case)]
    fn AgentTaskPermissionのnative確認は固定範囲を示しTask本文と追加権限を拒否する() {
        let payload = serde_json::json!({
            "agent_runtime_id": "fixture-agent",
            "session_id": "fixture-session",
            "workspace_id": "fixture-workspace"
        });
        let payload_hash = canonical_payload_hash(Some(&payload));
        let input = serde_json::json!({
            "request_id": "desktop-agent-permission",
            "operation": "AgentTaskWorkspacePermissionGrant",
            "payload": payload,
            "payload_hash": payload_hash,
            "nonce": "desktop-agent-permission-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        let endpoint = BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 43123,
            session_id: "desktop-session".into(),
            session_secret: "not-used-by-candidate".into(),
            credential_role: BrokerCredentialRole::Owner,
            transport: "tcp".into(),
            max_request_bytes: MAX_REQUEST_BYTES,
        };
        let (normalized, summary) =
            owner_operation_candidate(input.to_string().as_bytes(), &endpoint).unwrap();
        assert_eq!(
            BrokerRequestEnvelope::from_json_str(&normalized)
                .unwrap()
                .session_id
                .as_deref(),
            Some("desktop-session")
        );
        let text = owner_confirmation_text(&summary);
        assert!(text.contains("agent_task.execute"));
        assert!(text.contains("1回、5分以内"));
        assert!(text.contains("別のOwner Approvalが必要"));
        assert!(!text.contains("instruction"));

        let escalated = serde_json::json!({
            "agent_runtime_id": "fixture-agent",
            "session_id": "fixture-session",
            "workspace_id": "fixture-workspace",
            "command": "arbitrary-command"
        });
        let escalated_hash = canonical_payload_hash(Some(&escalated));
        let request = serde_json::json!({
            "request_id": "desktop-agent-permission-escalated",
            "operation": "AgentTaskWorkspacePermissionGrant",
            "payload": escalated,
            "payload_hash": escalated_hash,
            "nonce": "desktop-agent-permission-escalated-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        assert!(owner_operation_candidate(request.to_string().as_bytes(), &endpoint).is_none());
    }

    #[test]
    #[allow(non_snake_case)]
    fn AgentTaskOwnerApprovalのnative確認は本文を表示せずhashと未実行境界を示す() {
        let instruction = "秘密を含まないTask本文のfixture";
        let payload = serde_json::json!({
            "agent_runtime_id": "fixture-agent",
            "session_id": "fixture-session",
            "workspace_id": "fixture-workspace",
            "instruction": instruction
        });
        let input = serde_json::json!({
            "request_id": "desktop-agent-task-approval",
            "operation": "AgentTaskOwnerApprovalGrant",
            "payload": payload,
            "payload_hash": canonical_payload_hash(Some(&payload)),
            "nonce": "desktop-agent-task-approval-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        let endpoint = BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 43123,
            session_id: "desktop-session".into(),
            session_secret: "not-used-by-candidate".into(),
            credential_role: BrokerCredentialRole::Owner,
            transport: "tcp".into(),
            max_request_bytes: MAX_REQUEST_BYTES,
        };
        let (_, summary) = owner_operation_candidate(input.to_string().as_bytes(), &endpoint)
            .expect("Task Owner Approvalはnative確認候補");
        let text = owner_confirmation_text(&summary);
        assert!(text.contains(&sha256_tagged(instruction.as_bytes())));
        assert!(text.contains("gui-shell-agent-task-sandbox-v1-max-runtime-900s"));
        assert!(text.contains("開始後の最大実行時間15分"));
        assert!(text.contains("Taskを保存・変更・実行せず"));
        assert!(!text.contains(instruction));

        let escalated = serde_json::json!({
            "agent_runtime_id": "fixture-agent",
            "session_id": "fixture-session",
            "workspace_id": "fixture-workspace",
            "instruction": instruction,
            "approval_id": "caller-controlled"
        });
        let invalid = serde_json::json!({
            "request_id": "desktop-agent-task-approval-escalated",
            "operation": "AgentTaskOwnerApprovalGrant",
            "payload": escalated,
            "payload_hash": canonical_payload_hash(Some(&escalated)),
            "nonce": "desktop-agent-task-approval-escalated-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        assert!(owner_operation_candidate(invalid.to_string().as_bytes(), &endpoint).is_none());
    }

    #[test]
    fn export_directory_rejects_reparse_point_outside_runtime() {
        let root = fs::canonicalize(test_root("export-junction")).unwrap();
        let outside = fs::canonicalize(test_root("export-junction-outside")).unwrap();
        let runtime = runtime_directory_with_identity(&root, None).unwrap();
        let junction = runtime.join(EXPORT_DIRECTORY_NAME);
        let result = Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&junction)
            .arg(&outside)
            .output()
            .expect("Export保存先junction試験の作成");
        assert!(result.status.success(), "Export保存先junction試験の作成失敗");

        let error = ensure_export_directory(&runtime).unwrap_err();
        assert_eq!(error.code, "EXPORT_DIRECTORY_INVALID");
        assert!(fs::read_dir(&outside).unwrap().next().is_none());

        fs::remove_dir(&junction).unwrap();
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn runtime_directory_rejects_reparse_parent_outside_user_root() {
        let root = fs::canonicalize(test_root("runtime-junction")).unwrap();
        let outside = fs::canonicalize(test_root("runtime-junction-outside")).unwrap();
        let junction = root.join("GUI-Shell");
        let result = Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&junction)
            .arg(&outside)
            .output()
            .expect("Windows junction試験用実体の作成");
        assert!(result.status.success(), "junction試験用実体の作成失敗");

        let error = runtime_directory_with_identity(&root, None).unwrap_err();
        assert_eq!(error.code, "USER_DATA_ROOT_INVALID");
        assert!(!outside.join("broker").exists());

        fs::remove_dir(&junction).unwrap();
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn product_runtime_directory_rejects_reparse_identity_parent() {
        let root = fs::canonicalize(test_root("runtime-product-junction")).unwrap();
        let outside = fs::canonicalize(test_root("runtime-product-junction-outside")).unwrap();
        let identity = product_runtime_identity(
            Some("d4-pocket-app-0123456789abcdef0123456789abcdef"),
            Some("audit-store-fedcba9876543210fedcba9876543210"),
        )
        .unwrap()
        .unwrap();
        let apps = root.join("D4Pocket").join("apps");
        fs::create_dir_all(&apps).unwrap();
        let junction = apps.join(&identity.app_id);
        let result = Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&junction)
            .arg(&outside)
            .output()
            .expect("Export runtime junction試験の作成");
        assert!(
            result.status.success(),
            "Export runtime junction試験の作成失敗"
        );

        let error = runtime_directory_with_identity(&root, Some(&identity)).unwrap_err();
        assert_eq!(error.code, "USER_DATA_ROOT_INVALID");
        assert!(!outside.join("stores").exists());

        fs::remove_dir(&junction).unwrap();
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn broker_store_directory_rejects_junction_outside_runtime_root() {
        let root = fs::canonicalize(test_root("store-junction")).unwrap();
        let outside = fs::canonicalize(test_root("store-junction-outside")).unwrap();
        let junction = root.join("store");
        let result = Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&junction)
            .arg(&outside)
            .output()
            .expect("Windows junction試験用実体の作成");
        assert!(result.status.success(), "junction試験用実体の作成失敗");

        let error = ensure_store_directory(&root).unwrap_err();
        assert_eq!(error.code, "BROKER_STORE_INVALID");
        assert!(fs::read_dir(&outside).unwrap().next().is_none());

        fs::remove_dir(&junction).unwrap();
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn protected_store_directory_is_fixed_under_runtime_and_rejects_junction() {
        let root = fs::canonicalize(test_root("protected-store-directory")).unwrap();
        let protected = ensure_protected_store_directory(&root).unwrap();
        assert_eq!(protected, root.join("protected"));
        assert!(protected.is_dir());
        fs::remove_dir_all(&root).unwrap();

        let root = fs::canonicalize(test_root("protected-store-junction")).unwrap();
        let outside = fs::canonicalize(test_root("protected-store-junction-outside")).unwrap();
        let junction = root.join("protected");
        let result = Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&junction)
            .arg(&outside)
            .output()
            .expect("Windows junction試験用実体の作成");
        assert!(result.status.success(), "junction試験用実体の作成失敗");

        let error = ensure_protected_store_directory(&root).unwrap_err();
        assert_eq!(error.code, "PROTECTED_STORE_INVALID");
        assert!(fs::read_dir(&outside).unwrap().next().is_none());

        fs::remove_dir(&junction).unwrap();
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
}
