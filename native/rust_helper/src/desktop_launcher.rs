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

use crate::audit_hash::sha256_tagged;
use serde::Deserialize;
use serde_json::{json, Value};
use zeroize::Zeroize;

use crate::broker::export_center::{self, OwnerConfirmationSummary as ExportConfirmationSummary};
#[cfg(test)]
use crate::broker::ipc_server::run_loopback_server_cancellable;
use crate::broker::ipc_server::{BrokerServerError, DesktopOwnerOperationRequest};
use crate::broker::protocol::{
    canonical_payload_hash, owner_delete_confirmation_summary, owner_recovery_confirmation_summary,
    owner_registration_confirmation_summary, request_issued_at_is_current,
    OwnerDeleteConfirmationSummary, OwnerRecoveryConfirmationSummary,
    OwnerRegistrationConfirmationSummary,
};
use crate::broker::{
    BrokerCredentialRole, BrokerEndpoint, BrokerOperation, BrokerRequestEnvelope,
    BrokerServerConfig,
};

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

#[cfg(feature = "r2-e2e")]
const AGENT_CLI_REGISTRATION_NOTICE: &str = "このr2-e2e専用検証buildでは、loopback試験portと隔離CODEX_HOMEが有効な場合にTask実行Capabilityがsupportedになります。Task API接続先は環境指定の127.0.0.1偽Responses API用portです。登録自体はTaskを実行せず、Permission、Approval、Trust、Credentialを生成・保存しません。Task用Workspace PermissionとTaskごとのOwner Approvalは別のOwner確認が必要です。Credential実値を使用せず、合成Workspaceだけで試験してください。Broker終了時に登録は消えます。";

#[cfg(not(feature = "r2-e2e"))]
const AGENT_CLI_REGISTRATION_NOTICE: &str = "Taskは実行せず、Task実行能力はunsupportedのままです。Permission、Approval、Trust、Credentialを生成・保存しません。Broker終了時に登録は消えます。";

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
    AgentTaskResultExposure {
        task_id: String,
        result_hash: String,
        content_visibility: String,
        payload_hash: String,
    },
    WorkspaceContentApproval {
        workspace_id: String,
        registration_hash: String,
        content_visibility: String,
        payload_hash: String,
    },
    WorkspaceContentRevocation {
        workspace_id: String,
        registration_hash: String,
        payload_hash: String,
    },
    WorkspaceBaselineCapture {
        workspace_id: String,
        registration_hash: String,
        payload_hash: String,
    },
    AgentCliRuntimeWorkspaceRegistration {
        adapter_id: String,
        interface_scope: String,
        runtime_id: String,
        cli_path: String,
        workspace_id: String,
        workspace_root: String,
        secret_paths: Vec<String>,
        provider_id: String,
        model_id: String,
        authentication_source: String,
        credential_id: Option<String>,
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
    CredentialRevocation {
        credential_id: String,
        purpose: String,
        target: String,
        ciphertext_hash: String,
        created_audit_id: String,
        payload_hash: String,
    },
    McpConnect {
        server_id: String,
        executable: String,
        workspace: String,
        argument_count: usize,
        arguments_hash: String,
        credential_id: Option<String>,
        credential_environment_variable: Option<String>,
        payload_hash: String,
    },
    A2aConnect {
        agent_id: String,
        target: String,
        payload_hash: String,
    },
    McpDisconnect {
        server_id: String,
        payload_hash: String,
    },
    McpToolCall {
        server_id: String,
        tool_id: String,
        name: String,
        argument_count: usize,
        arguments_hash: String,
        payload_hash: String,
    },
    UpdateDownload {
        confirmation: crate::broker::update_center::UpdateDownloadConfirmation,
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
struct AgentTaskResultExposureRequest {
    task_id: String,
    result_hash: String,
    content_visibility: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceContentApprovalRequest {
    #[serde(rename = "作業領域ID")]
    workspace_id: String,
    #[serde(rename = "登録hash")]
    registration_hash: String,
    #[serde(rename = "表示範囲")]
    content_visibility: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceBindingRequest {
    #[serde(rename = "作業領域ID")]
    workspace_id: String,
    #[serde(rename = "登録hash")]
    registration_hash: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentCliRuntimeWorkspaceRegistrationRequest {
    version: u8,
    adapter_id: String,
    runtime_id: String,
    cli_path: String,
    workspace_id: String,
    workspace_root: String,
    secret_paths: Vec<String>,
    provider_model_selection: crate::adapters::ProviderModelSelection,
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
    #[serde(default)]
    environment_variable: Option<String>,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialRevocationOwnerRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "資格情報ID")]
    credential_id: String,
    #[serde(rename = "用途")]
    purpose: String,
    #[serde(rename = "接続対象")]
    target: String,
    #[serde(rename = "暗号文hash")]
    ciphertext_hash: String,
    #[serde(rename = "作成監査ID")]
    created_audit_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct McpToolCallOwnerRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "操作")]
    operation: String,
    #[serde(rename = "ServerID")]
    server_id: String,
    #[serde(rename = "ToolID")]
    tool_id: String,
    #[serde(rename = "名前")]
    name: String,
    #[serde(rename = "arguments")]
    arguments: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateDownloadOwnerRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "更新ID")]
    update_id: String,
    #[serde(rename = "候補hash")]
    candidate_hash: String,
}

fn valid_update_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn is_tagged_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn current_update_download_confirmation(
    request: &UpdateDownloadOwnerRequest,
    payload_hash: &str,
    endpoint: &BrokerEndpoint,
) -> Option<crate::broker::update_center::UpdateDownloadConfirmation> {
    let listing_payload = json!({"版": 1});
    let mut nonce = [0u8; 16];
    getrandom::getrandom(&mut nonce).ok()?;
    let preflight = json!({
        "request_id": format!("desktop-update-preflight-{}", hex::encode(nonce)),
        "session_id": endpoint.session_id,
        "operation": "更新一覧",
        "payload_hash": canonical_payload_hash(Some(&listing_payload)),
        "nonce": format!("desktop-update-preflight-nonce-{}", hex::encode(nonce)),
        "issued_at": BrokerRequestEnvelope::current_issued_at(),
        "metadata": {"client": "desktop_flutter"},
        "payload": listing_payload,
    });
    let response = relay_normalized_channel_request(preflight.to_string().as_bytes(), endpoint)?;
    let response: Value = serde_json::from_slice(&response).ok()?;
    if response["operation"] != "更新一覧" || response["status"] != "accepted" {
        return None;
    }
    let updates = response["body"]["更新一覧"].as_array()?;
    let matching: Vec<&Value> = updates
        .iter()
        .filter(|value| {
            value["更新ID"] == request.update_id
                && value["候補hash"] == request.candidate_hash
                && value["署名状態"] == "verified"
                && value["版"] == 2
        })
        .collect();
    let [record] = matching.as_slice() else {
        return None;
    };
    let source = &record["取得元"];
    if source["状態"] != "configured" {
        return None;
    }
    let source_url = source["URL"].as_str()?;
    let url = reqwest::Url::parse(source_url).ok()?;
    let host = url.host_str()?;
    if url.scheme() != "https"
        || url.username() != ""
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !source_url.ends_with(&format!("/{}.pkg", request.update_id))
    {
        return None;
    }
    let package_sha256 = record["package_sha256"].as_str()?;
    if package_sha256.len() != 64
        || !package_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    let package_size_bytes = record["package_size_bytes"].as_u64()?;
    if package_size_bytes == 0 || package_size_bytes > 4 * 1024 * 1024 * 1024 {
        return None;
    }
    Some(crate::broker::update_center::UpdateDownloadConfirmation {
        update_id: request.update_id.clone(),
        candidate_hash: request.candidate_hash.clone(),
        source_url: source_url.to_owned(),
        package_sha256: package_sha256.to_owned(),
        package_size_bytes,
        offered_version: record["提供版"].as_str()?.to_owned(),
        channel: record["channel"].as_str()?.to_owned(),
        summary: record["内容概要"].as_str()?.to_owned(),
        display_host: host.to_owned(),
        payload_hash: payload_hash.to_owned(),
    })
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

fn agent_task_result_identifier_is_valid(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn tagged_sha256_is_valid(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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

    pub fn requires_immediate_exit(&self) -> bool {
        matches!(
            self.code,
            "BROKER_STOPPED" | "BROKER_SHUTDOWN_FAILED" | "BROKER_CHANNEL_SHUTDOWN_FAILED"
        )
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
    // 安定したlock fileを残す。handle終了でOS lockは解放され、file再作成による別inodeの競合を避ける。
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
        || !envelope
            .issued_at
            .as_deref()
            .is_some_and(request_issued_at_is_current)
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
    let payload = envelope
        .payload
        .as_ref()
        .unwrap_or(&serde_json::Value::Null);
    let summary = match envelope.operation? {
        BrokerOperation::A2A接続 => {
            let (agent_id, target) =
                crate::broker::a2a_center::owner_confirmation_summary(payload).ok()?;
            DesktopOwnerOperationSummary::A2aConnect {
                agent_id,
                target,
                payload_hash,
            }
        }
        BrokerOperation::MCP接続 => {
            let request: McpConnectOwnerRequest = serde_json::from_value(payload.clone()).ok()?;
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
                || !valid_mcp_credential_reference(reference)
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
                credential_id: (reference.status == "configured")
                    .then(|| reference.credential_id.clone()),
                credential_environment_variable: reference.environment_variable.clone(),
                payload_hash,
            }
        }
        BrokerOperation::GuiShell書出し => DesktopOwnerOperationSummary::GuiShellExport(
            export_center::owner_confirmation_summary(payload, &payload_hash).ok()?,
        ),
        BrokerOperation::AgentCLI実行系作業領域登録 => {
            let request: AgentCliRuntimeWorkspaceRegistrationRequest =
                serde_json::from_value(payload.clone()).ok()?;
            let Some(interface_scope) =
                crate::adapters::cli_adapter_confirmation_scope(&request.adapter_id)
            else {
                return None;
            };
            if request.version != 1
                || !agent_task_permission_identifier_is_valid(&request.runtime_id)
                || !agent_task_permission_identifier_is_valid(&request.workspace_id)
                || request.cli_path.is_empty()
                || request.cli_path.len() > 1024
                || request.cli_path.chars().any(char::is_control)
                || !Path::new(&request.cli_path).is_absolute()
                || request.workspace_root.is_empty()
                || request.workspace_root.len() > 1024
                || request.workspace_root.chars().any(char::is_control)
                || !Path::new(&request.workspace_root).is_absolute()
                || request.secret_paths.len() > 16
                || request.secret_paths.iter().any(|path| {
                    path.is_empty()
                        || path.len() > 256
                        || path.chars().any(char::is_control)
                        || Path::new(path).is_absolute()
                        || path
                            .split(['/', '\\'])
                            .any(|part| part == "." || part == "..")
                })
                || request
                    .secret_paths
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != request.secret_paths.len()
                || !request.provider_model_selection.is_valid()
            {
                return None;
            }
            DesktopOwnerOperationSummary::AgentCliRuntimeWorkspaceRegistration {
                adapter_id: request.adapter_id,
                interface_scope: interface_scope.to_owned(),
                runtime_id: request.runtime_id,
                cli_path: request.cli_path,
                workspace_id: request.workspace_id,
                workspace_root: request.workspace_root,
                secret_paths: request.secret_paths,
                provider_id: request.provider_model_selection.provider_id.clone(),
                model_id: request.provider_model_selection.model_id.clone(),
                authentication_source: request
                    .provider_model_selection
                    .authentication_source()
                    .to_owned(),
                credential_id: request
                    .provider_model_selection
                    .credential_id()
                    .map(str::to_owned),
                payload_hash,
            }
        }
        BrokerOperation::更新download要求 => {
            let request: UpdateDownloadOwnerRequest =
                serde_json::from_value(payload.clone()).ok()?;
            if request.version != 1
                || !valid_update_identifier(&request.update_id)
                || !is_tagged_sha256(&request.candidate_hash)
            {
                return None;
            }
            let confirmation =
                current_update_download_confirmation(&request, &payload_hash, endpoint)?;
            DesktopOwnerOperationSummary::UpdateDownload {
                confirmation,
                payload_hash,
            }
        }
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
        BrokerOperation::AgentTask結果表示承認 => {
            let request: AgentTaskResultExposureRequest =
                serde_json::from_value(payload.clone()).ok()?;
            if !agent_task_result_identifier_is_valid(&request.task_id)
                || !tagged_sha256_is_valid(&request.result_hash)
                || !["none", "hash_only", "summary", "redacted", "full"]
                    .contains(&request.content_visibility.as_str())
            {
                return None;
            }
            DesktopOwnerOperationSummary::AgentTaskResultExposure {
                task_id: request.task_id,
                result_hash: request.result_hash,
                content_visibility: request.content_visibility,
                payload_hash,
            }
        }
        BrokerOperation::作業領域承認 => {
            let request: WorkspaceContentApprovalRequest =
                serde_json::from_value(payload.clone()).ok()?;
            if !agent_task_permission_identifier_is_valid(&request.workspace_id)
                || !tagged_sha256_is_valid(&request.registration_hash)
                || !["none", "hash_only", "summary", "redacted", "full"]
                    .contains(&request.content_visibility.as_str())
            {
                return None;
            }
            DesktopOwnerOperationSummary::WorkspaceContentApproval {
                workspace_id: request.workspace_id,
                registration_hash: request.registration_hash,
                content_visibility: request.content_visibility,
                payload_hash,
            }
        }
        BrokerOperation::作業領域失効 => {
            let request: WorkspaceBindingRequest = serde_json::from_value(payload.clone()).ok()?;
            if !agent_task_permission_identifier_is_valid(&request.workspace_id)
                || !tagged_sha256_is_valid(&request.registration_hash)
            {
                return None;
            }
            DesktopOwnerOperationSummary::WorkspaceContentRevocation {
                workspace_id: request.workspace_id,
                registration_hash: request.registration_hash,
                payload_hash,
            }
        }
        BrokerOperation::作業領域全体基準点保存 => {
            let request: WorkspaceBindingRequest = serde_json::from_value(payload.clone()).ok()?;
            if !agent_task_permission_identifier_is_valid(&request.workspace_id)
                || !tagged_sha256_is_valid(&request.registration_hash)
            {
                return None;
            }
            DesktopOwnerOperationSummary::WorkspaceBaselineCapture {
                workspace_id: request.workspace_id,
                registration_hash: request.registration_hash,
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
        BrokerOperation::MCPTool実行 => {
            let request: McpToolCallOwnerRequest = serde_json::from_value(payload.clone()).ok()?;
            if request.version != 1
                || request.operation != "実行"
                || request.server_id.is_empty()
                || request.server_id.len() > 128
                || request.server_id.chars().any(char::is_control)
                || request.tool_id.len() != 147
                || !request.tool_id.starts_with("tool-")
                || !request.tool_id[5..]
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                || request.name.is_empty()
                || request.name.len() > 256
                || request.name.chars().any(char::is_control)
                || !mcp_arguments_within_limits(&request.arguments)
            {
                return None;
            }
            let arguments = request.arguments.as_object()?;
            let arguments_hash = sha256_tagged(&serde_json::to_vec(&request.arguments).ok()?);
            DesktopOwnerOperationSummary::McpToolCall {
                server_id: request.server_id,
                tool_id: request.tool_id,
                name: request.name,
                argument_count: arguments.len(),
                arguments_hash,
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
        BrokerOperation::資格情報失効 => {
            let request: CredentialRevocationOwnerRequest =
                serde_json::from_value(payload.clone()).ok()?;
            if request.version != 1
                || request.credential_id.len() != 32
                || !request
                    .credential_id
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                || request.purpose.is_empty()
                || request.purpose.len() > 256
                || request.purpose.chars().any(char::is_control)
                || request.target.is_empty()
                || request.target.len() > 256
                || request.target.chars().any(char::is_control)
                || !is_tagged_sha256(&request.ciphertext_hash)
                || request.created_audit_id.is_empty()
                || request.created_audit_id.len() > 256
                || request.created_audit_id.chars().any(char::is_control)
            {
                return None;
            }
            DesktopOwnerOperationSummary::CredentialRevocation {
                credential_id: request.credential_id,
                purpose: request.purpose,
                target: request.target,
                ciphertext_hash: request.ciphertext_hash,
                created_audit_id: request.created_audit_id,
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

fn valid_mcp_credential_reference(reference: &McpConnectCredentialReference) -> bool {
    match reference.status.as_str() {
        "missing" => {
            !reference.required
                && reference.credential_id == "00000000000000000000000000000000"
                && reference.environment_variable.is_none()
        }
        "configured" => {
            reference.required
                && reference
                    .environment_variable
                    .as_deref()
                    .is_some_and(valid_mcp_credential_environment_name)
        }
        _ => false,
    }
}

fn valid_mcp_credential_environment_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == b'_')
        || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        || name.len() > 128
    {
        return false;
    }
    let normalized = name.to_ascii_uppercase();
    !matches!(
        normalized.as_str(),
        "PATH"
            | "SYSTEMROOT"
            | "WINDIR"
            | "TEMP"
            | "TMP"
            | "USERPROFILE"
            | "HOME"
            | "APPDATA"
            | "LOCALAPPDATA"
            | "PROGRAMDATA"
            | "SYSTEMDRIVE"
            | "COMSPEC"
            | "PATHEXT"
            | "PSMODULEPATH"
    ) && !normalized.starts_with("GUI_SHELL_")
}

fn mcp_arguments_within_limits(value: &Value) -> bool {
    if !value.is_object()
        || serde_json::to_vec(value)
            .ok()
            .is_none_or(|encoded| encoded.len() > 32 * 1024)
        || mcp_argument_contains_sensitive_field(value)
    {
        return false;
    }
    let mut pending = vec![(value, 1usize)];
    let mut nodes = 0usize;
    while let Some((current, depth)) = pending.pop() {
        nodes += 1;
        if nodes > 2048 || depth > 32 {
            return false;
        }
        match current {
            Value::Object(object) => {
                pending.extend(object.values().map(|child| (child, depth + 1)))
            }
            Value::Array(values) => pending.extend(values.iter().map(|child| (child, depth + 1))),
            _ => {}
        }
    }
    true
}

fn mcp_argument_contains_sensitive_field(value: &Value) -> bool {
    match value {
        Value::Object(object) => {
            object.keys().any(|key| {
                matches!(
                    key.to_ascii_lowercase().as_str(),
                    "authority"
                        | "authority_id"
                        | "permission"
                        | "permission_id"
                        | "approval"
                        | "approval_id"
                        | "capability"
                        | "capability_grant"
                        | "secret"
                        | "secret_value"
                        | "secretvalue"
                        | "token"
                        | "access_token"
                        | "refresh_token"
                        | "password"
                        | "credential"
                        | "credentials"
                        | "credential_value"
                        | "api_key"
                        | "apikey"
                        | "authorization"
                )
            }) || object.values().any(mcp_argument_contains_sensitive_field)
        }
        Value::Array(values) => values.iter().any(mcp_argument_contains_sensitive_field),
        _ => false,
    }
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
                        let download_confirmation = match &summary {
                            DesktopOwnerOperationSummary::UpdateDownload {
                                confirmation, ..
                            } => Some(confirmation.clone()),
                            _ => None,
                        };
                        owner_operations
                            .send(DesktopOwnerOperationRequest {
                                request_json,
                                download_confirmation,
                                reply,
                            })
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
        DesktopOwnerOperationSummary::AgentCliRuntimeWorkspaceRegistration {
            adapter_id,
            interface_scope,
            runtime_id,
            cli_path,
            workspace_id,
            workspace_root,
            secret_paths,
            provider_id,
            model_id,
            authentication_source,
            credential_id,
            payload_hash,
        } => format!(
            "Agent CLI実行系と作業領域を、このDesktop起動中だけBrokerへ登録しますか？\n\nAdapter ID: {}\nRuntime ID: {}\nCLI実行file: {}\nCLI検査範囲: {}\n提供元・模型: {} / {}\n{}\n提供元接続・模型利用可否: 不明（登録時はCLI接続面のみ確認）\n自動代替実行: 無効\nWorkspace ID: {}\nWorkspace root: {}\n秘密path除外（本文は受け取りません）:\n{}\n\n{}\n\npayload hash:\n{}",
            owner_confirmation_value(adapter_id),
            owner_confirmation_value(runtime_id),
            owner_confirmation_value(cli_path),
            owner_confirmation_value(interface_scope),
            owner_confirmation_value(provider_id),
            owner_confirmation_value(model_id),
            if authentication_source == "broker_credential_vault" {
                format!(
                    "認証: Broker資格情報保管庫のProvider API keyを、Approval済み実行だけに使用\n資格情報ID: {}（metadata参照のみ。秘密値をFlutter／この確認要求へ含めない）",
                    credential_id
                        .as_deref()
                        .map(owner_confirmation_value)
                        .unwrap_or_else(|| "不正・未指定".into())
                )
            } else {
                "認証: Codex CLI管理設定を使用（D4 Pocketは秘密値を受け取らない）".into()
            },
            owner_confirmation_value(workspace_id),
            owner_confirmation_value(workspace_root),
            if secret_paths.is_empty() {
                "（除外pathなし）".to_owned()
            } else {
                secret_paths
                    .iter()
                    .map(|path| format!("• {}", owner_confirmation_value(path)))
                    .collect::<Vec<_>>()
                    .join("\n")
            },
            AGENT_CLI_REGISTRATION_NOTICE,
            payload_hash
        ),
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
        DesktopOwnerOperationSummary::AgentTaskResultExposure {
            task_id,
            result_hash,
            content_visibility,
            payload_hash,
        } => format!(
            "完了したAgent Taskの結果を一回だけ表示するApprovalを発行しますか？\n\nTask ID: {}\n確定結果hash: {}\nContent Exposure: {}\n範囲: このTaskと結果hashに限定、5分以内に一回だけ取得\n\n`full`はAgentが返した本文全体を表示し、秘密情報や危険な指示を含む可能性があります。Agentのtest結果・主張はBroker検証済みではありません。画面の表示範囲を確認してください。これはTask実行ApprovalでもWorkspace読取許可でもなく、Permission・Credential・Authorityを与えません。Auditには本文を保存しません。\n\npayload hash:\n{}",
            owner_confirmation_value(task_id), owner_confirmation_value(result_hash),
            owner_confirmation_value(content_visibility), payload_hash
        ),
        DesktopOwnerOperationSummary::WorkspaceContentApproval {
            workspace_id,
            registration_hash,
            content_visibility,
            payload_hash,
        } => format!(
            "この登録済みWorkspaceの読取内容を一時承認しますか？\n\nWorkspace ID: {}\n登録hash: {}\nContent Exposure: {}\n範囲: このWorkspace登録に限定、5分間\n\n`full`では登録済みsecret除外以外のファイル内容をBrokerへ読み込み、画面に表示できます。secret除外は登録されたpath境界であり、一般的な秘密検出ではありません。これはTask実行Permission／ApprovalでもWorkspace書込許可でもありません。\n\npayload hash:\n{}",
            owner_confirmation_value(workspace_id),
            owner_confirmation_value(registration_hash),
            owner_confirmation_value(content_visibility),
            payload_hash
        ),
        DesktopOwnerOperationSummary::WorkspaceContentRevocation {
            workspace_id,
            registration_hash,
            payload_hash,
        } => format!(
            "このWorkspaceの一時読取許可と比較基準点を失効しますか？\n\nWorkspace ID: {}\n登録hash: {}\n結果: 現在の読取許可とBroker内baselineを破棄します。Workspace上のfileは変更・削除しません。\n\npayload hash:\n{}",
            owner_confirmation_value(workspace_id),
            owner_confirmation_value(registration_hash),
            payload_hash
        ),
        DesktopOwnerOperationSummary::WorkspaceBaselineCapture {
            workspace_id,
            registration_hash,
            payload_hash,
        } => format!(
            "このWorkspaceの現在状態を比較baselineとしてBroker内へ取得しますか？\n\nWorkspace ID: {}\n登録hash: {}\n範囲: 現在のfull読取承認で列挙できる全file。Brokerが保持するbaselineはWorkspaceあたり一つで、既存baselineがあれば置換します。\n\n内容を読む操作です。宣言済みsecret pathを除外し、保持量はBroker上限に従います。Workspace fileの変更・Task実行はしません。Task IDには結合されないため、別Taskの比較結果と混同しないでください。\n\npayload hash:\n{}",
            owner_confirmation_value(workspace_id),
            owner_confirmation_value(registration_hash),
            payload_hash
        ),
        DesktopOwnerOperationSummary::McpDisconnect {
            server_id,
            payload_hash,
        } => format!(
            "指定したMCP Serverの接続を切断しますか？\n\nServer ID: {}\n対象: このBrokerが保持する指定stdio process群だけ\n処理: Windows Job Objectによるprocess群停止確認後、永続Auditを確定して接続記録を解消\n\nCredential、他Server、Permission、Approvalを変更せず、Toolを実行しません。停止またはAudit確定に失敗した場合は成功扱いしません。Windows accountの再認証ではありません。\n\npayload hash:\n{}",
            owner_confirmation_value(server_id),
            payload_hash
        ),
        DesktopOwnerOperationSummary::A2aConnect {
            agent_id,
            target,
            payload_hash,
        } => format!(
            "指定したloopback A2A Agent Cardのmetadataを取得しますか？\n\nAgent ID: {}\n取得先: {}\nTransport: HTTP（loopback IP限定）\n\nCredential実値は受け取らず、Task送信・Message送信・Workspace変更は行いません。Agent Cardは未信頼metadataとして記録し、Trustはpending reviewのままです。外部Agentの宣言からPermission、Approval、Authorityを生成しません。Windows accountの再認証ではありません。\n\npayload hash:\n{}",
            owner_confirmation_value(agent_id),
            owner_confirmation_value(target),
            payload_hash
        ),
        DesktopOwnerOperationSummary::McpConnect {
            server_id,
            executable,
            workspace,
            argument_count,
            arguments_hash,
            credential_id,
            credential_environment_variable,
            payload_hash,
        } => {
            let credential_summary = match (credential_id, credential_environment_variable) {
                (Some(id), Some(name)) => format!(
                    "Credential ID: {}\n子process環境変数名: {}\n秘密値はこの確認画面・Flutter・Auditへ返しません。",
                    owner_confirmation_value(id),
                    owner_confirmation_value(name),
                ),
                _ => "Credential: 未選択（秘密値を渡しません）".to_string(),
            };
            format!(
                "指定したMCP stdio Serverを起動し、検証済みmetadataを取得しますか？\n\nServer ID: {}\n実行file: {}\n作業folder: {}\n起動引数: {}件（引数本文は秘密値を含む可能性があるため表示しません）\n引数hash: {}\n{}\n\n前画面で実行file、作業folder、引数、Credentialの対象を確認してください。Credentialを選んだ場合、対象MCP processとその子孫processは値を読み取り、外部送信できます。Windows Job Objectはprocess寿命を監督しますがsandboxではありません。protocol fallback時は同じ対象processを再起動して同じCredentialを渡す場合があります。ServerのTrust、Tool実行権、追加Permissionは付与しません。Windows accountの再認証ではありません。\n\npayload hash:\n{}",
                owner_confirmation_value(server_id),
                owner_confirmation_value(executable),
                owner_confirmation_value(workspace),
                argument_count,
                arguments_hash,
                credential_summary,
                payload_hash
            )
        }
        DesktopOwnerOperationSummary::McpToolCall {
            server_id,
            tool_id,
            name,
            argument_count,
            arguments_hash,
            payload_hash,
        } => format!(
            "このMCP Toolを指定Serverへ一度だけ実行しますか？\n\nServer ID: {}\nTool: {} ({})\narguments: {}項目\narguments hash: {}\n\n前画面でJSON arguments全文と対象Server／Toolを確認してから判断してください。ここでは引数本文もTool結果本文も表示しません。秘密値をargumentsへ含めないでください。Brokerは現在CatalogとJSON Schemaを再検査し、この呼出しだけの一回限りPermissionを消費してAuditを記録します。MCP Server側の外部副作用は取り消せません。送信後に結果を確定できない場合は接続を隔離し、自動再送しません。ownerが外部状態を確認してから切断・再接続してください。Windows accountの再認証ではありません。\n\npayload hash:\n{}",
            owner_confirmation_value(server_id),
            owner_confirmation_value(name),
            owner_confirmation_value(tool_id),
            argument_count,
            arguments_hash,
            payload_hash
        ),
        DesktopOwnerOperationSummary::UpdateDownload {
            confirmation,
            payload_hash,
        } => format!(
            "署名済みD4 Pocket更新packageをdownloadしますか？\n\n更新ID: {}\n提供版: {}\nchannel: {}\n配布host（Broker現在設定）: {}\n署名済みpackage SHA-256: {}\n署名済みbyte長: {}\n内容概要: {}\n\nこの画面の前にRust起動器が通常認証Brokerから現在の候補を取得しました。確認後もBrokerがtrust・候補hash・取得URLを再検証し、表示時から変化していれば拒否します。公開HTTPSの直接接続のみ、redirect・system proxy・private／local IPを拒否します。取得byte長とSHA-256を検証し、固定Broker storeへ保存します。これはdownloadだけで、install／process起動／Permission付与は行いません。完了Audit後も適用操作は別途suspendedです。\n\npayload hash:\n{}",
            owner_confirmation_value(&confirmation.update_id),
            owner_confirmation_value(&confirmation.offered_version),
            owner_confirmation_value(&confirmation.channel),
            owner_confirmation_value(&confirmation.display_host),
            confirmation.package_sha256,
            confirmation.package_size_bytes,
            owner_confirmation_value(&confirmation.summary),
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
        DesktopOwnerOperationSummary::CredentialRevocation {
            credential_id,
            purpose,
            target,
            ciphertext_hash,
            created_audit_id,
            payload_hash,
        } => format!(
            "指定Credentialを論理失効させますか？\n\nCredential ID: {}\n用途: {}\n接続対象: {}\n登録監査ID: {}\n暗号文hash: {}\n\nこの操作は永続Auditへ失効状態を記録し、以後のMCP注入を拒否します。Brokerは表示した登録監査ID・暗号文hash・用途・接続対象を処理時に再照合します。暗号文fileは削除せず保管を継続します。失効はこの操作では取消できません。再利用が必要なら別IDで登録してください。暗号文の物理削除とRecoveryは別操作で、ここでは実行しません。Windows accountの再認証ではありません。\n\npayload hash:\n{}",
            owner_confirmation_value(credential_id),
            owner_confirmation_value(purpose),
            owner_confirmation_value(target),
            owner_confirmation_value(created_audit_id),
            owner_confirmation_value(ciphertext_hash),
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
        let export_root =
            cap_std::fs::Dir::open_ambient_dir(&export_dir, cap_std::ambient_authority()).map_err(
                |_| {
                    DesktopLaunchError::new(
                        "EXPORT_DIRECTORY_UNAVAILABLE",
                        "Manifestの保存先を安全に開けません。",
                    )
                },
            )?;
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

    #[cfg(windows)]
    let frontend_job = {
        use std::os::windows::process::CommandExt;

        let job = gui_shell_process_supervision::Job::create().map_err(|_| {
            DesktopLaunchError::new(
                "FRONTEND_SUPERVISION_UNAVAILABLE",
                "D4 Pocket画面の終了監督を準備できません。",
            )
        })?;
        command.creation_flags(gui_shell_process_supervision::CREATE_SUSPENDED);
        job
    };

    let mut frontend = command.spawn().map_err(|_| {
        DesktopLaunchError::new(
            "FRONTEND_START_FAILED",
            "D4 Pocket画面を起動できません。製品ファイルを確認してください。",
        )
    })?;

    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;

        if frontend_job
            .assign_and_resume(frontend.id(), frontend.as_raw_handle())
            .is_err()
        {
            let _ = frontend_job.terminate_and_wait();
            let _ = frontend.kill();
            let _ = frontend.wait();
            return Err(DesktopLaunchError::new(
                "FRONTEND_SUPERVISION_FAILED",
                "D4 Pocket画面の終了監督を開始できません。",
            ));
        }
    }

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
    let runtime_dir = runtime_directory_with_identity(&local_app_data, product_identity.as_ref())?;
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

    #[test]
    fn broker_failure_codes_exit_without_dialog() {
        for code in [
            "BROKER_STOPPED",
            "BROKER_SHUTDOWN_FAILED",
            "BROKER_CHANNEL_SHUTDOWN_FAILED",
        ] {
            assert!(DesktopLaunchError::new(code, "test").requires_immediate_exit());
        }
        for code in ["FRONTEND_START_FAILED", "FRONTEND_EXIT_FAILED"] {
            assert!(!DesktopLaunchError::new(code, "test").requires_immediate_exit());
        }
    }

    #[cfg(windows)]
    fn automate_native_owner_confirmation(
        summary: &DesktopOwnerOperationSummary,
        confirm: bool,
        forbidden_text: Option<&str>,
    ) -> bool {
        use std::process::Stdio;
        use winsafe::{co, HwKbMouse, HWND, KEYBDINPUT};

        const IDYES: u16 = 6;
        const IDNO: u16 = 7;
        const ID_STATIC_TEXT: u16 = 0xffff;

        fn is_owner_dialog(window: &HWND, process_id: u32) -> bool {
            if !window.IsWindow() || !window.IsWindowVisible() {
                return false;
            }
            let (_, owner_process_id) = window.GetWindowThreadProcessId();
            owner_process_id == process_id
                && window.GetClassName().is_ok_and(|name| name == "#32770")
                && window
                    .GetWindowText()
                    .is_ok_and(|title| title == "D4 Pocket Owner確認")
        }

        fn find_owner_dialog(process_id: u32) -> Option<HWND> {
            let mut found = None;
            winsafe::EnumWindows(|window| {
                if is_owner_dialog(&window, process_id) {
                    found = Some(window);
                }
                true
            })
            .expect("Windows top-level window列挙");
            found
        }

        fn wait_for_focus(dialog: &HWND, control_id: u16, timeout: Duration) -> bool {
            let (thread_id, process_id) = dialog.GetWindowThreadProcessId();
            let deadline = Instant::now() + timeout;
            while Instant::now() < deadline {
                if let Ok(info) = winsafe::GetGUIThreadInfo(thread_id) {
                    if is_owner_dialog(&info.hwndActive, process_id)
                        && info.hwndFocus.GetDlgCtrlID().ok() == Some(control_id)
                    {
                        return true;
                    }
                }
                thread::sleep(Duration::from_millis(25));
            }
            false
        }

        fn send_key(key: co::VK) {
            let sent = winsafe::SendInput(&[
                HwKbMouse::Kb(KEYBDINPUT {
                    wVk: key,
                    ..Default::default()
                }),
                HwKbMouse::Kb(KEYBDINPUT {
                    wVk: key,
                    dwFlags: co::KEYEVENTF::KEYUP,
                    ..Default::default()
                }),
            ])
            .expect("Owner確認dialogへの合成キー入力");
            assert_eq!(sent, 2, "Owner確認dialogへのキー入力件数");
        }

        let expected_text = owner_confirmation_text(summary);
        if let Some(forbidden_text) = forbidden_text {
            assert!(
                !expected_text.contains(forbidden_text),
                "秘密のTask本文がnative Owner確認へ露出した"
            );
        }
        let request = match summary {
            DesktopOwnerOperationSummary::GuiShellExport(summary) => json!({
                "summary": {
                    "kind": "gui_shell_export",
                    "display_name": summary.display_name,
                    "export_id": summary.export_id,
                    "distribution_channel": summary.distribution_channel,
                    "optional_module_count": summary.optional_module_count,
                    "payload_hash": summary.payload_hash
                },
                "confirm": confirm
            }),
            DesktopOwnerOperationSummary::AgentTaskWorkspacePermission {
                runtime_id,
                session_id,
                workspace_id,
                payload_hash,
            } => json!({
                "summary": {
                    "kind": "agent_task_workspace_permission",
                    "runtime_id": runtime_id,
                    "session_id": session_id,
                    "workspace_id": workspace_id,
                    "payload_hash": payload_hash
                },
                "confirm": confirm
            }),
            DesktopOwnerOperationSummary::AgentTaskOwnerApproval {
                runtime_id,
                session_id,
                workspace_id,
                instruction_characters,
                instruction_hash,
                payload_hash,
            } => json!({
                "summary": {
                    "kind": "agent_task_owner_approval",
                    "runtime_id": runtime_id,
                    "session_id": session_id,
                    "workspace_id": workspace_id,
                    "instruction_characters": instruction_characters,
                    "instruction_hash": instruction_hash,
                    "payload_hash": payload_hash
                },
                "confirm": confirm
            }),
            DesktopOwnerOperationSummary::AgentTaskResultExposure {
                task_id,
                result_hash,
                content_visibility,
                payload_hash,
            } => json!({
                "summary": {
                    "kind": "agent_task_result_exposure",
                    "task_id": task_id,
                    "result_hash": result_hash,
                    "content_visibility": content_visibility,
                    "payload_hash": payload_hash
                },
                "confirm": confirm
            }),
            DesktopOwnerOperationSummary::WorkspaceContentApproval {
                workspace_id,
                registration_hash,
                content_visibility,
                payload_hash,
            } => json!({
                "summary": {
                    "kind": "workspace_content_approval",
                    "workspace_id": workspace_id,
                    "registration_hash": registration_hash,
                    "content_visibility": content_visibility,
                    "payload_hash": payload_hash
                },
                "confirm": confirm
            }),
            DesktopOwnerOperationSummary::WorkspaceContentRevocation {
                workspace_id,
                registration_hash,
                payload_hash,
            } => json!({
                "summary": {
                    "kind": "workspace_content_revocation",
                    "workspace_id": workspace_id,
                    "registration_hash": registration_hash,
                    "payload_hash": payload_hash
                },
                "confirm": confirm
            }),
            DesktopOwnerOperationSummary::WorkspaceBaselineCapture {
                workspace_id,
                registration_hash,
                payload_hash,
            } => json!({
                "summary": {
                    "kind": "workspace_baseline_capture",
                    "workspace_id": workspace_id,
                    "registration_hash": registration_hash,
                    "payload_hash": payload_hash
                },
                "confirm": confirm
            }),
            DesktopOwnerOperationSummary::AgentCliRuntimeWorkspaceRegistration {
                adapter_id,
                interface_scope,
                runtime_id,
                cli_path,
                workspace_id,
                workspace_root,
                secret_paths,
                    provider_id,
                    model_id,
                    authentication_source,
                    credential_id,
                    payload_hash,
                } => json!({
                "summary": {
                    "kind": "agent_cli_runtime_workspace_registration",
                    "adapter_id": adapter_id,
                    "interface_scope": interface_scope,
                    "runtime_id": runtime_id,
                    "cli_path": cli_path,
                    "workspace_id": workspace_id,
                    "workspace_root": workspace_root,
                    "secret_paths": secret_paths,
                    "provider_id": provider_id,
                    "model_id": model_id,
                    "authentication_source": authentication_source,
                    "credential_id": credential_id,
                    "payload_hash": payload_hash
                },
                "confirm": confirm
            }),
            _ => panic!("このUI試験ではAgent Taskの固定summaryだけを使用する"),
        };

        let executable = std::env::current_exe().expect("Rust試験実行fileの所在");
        let mut command = Command::new(executable);
        command
            .env_clear()
            .args([
                "--exact",
                "desktop_launcher::tests::owner_confirmation_dialog_child",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("GUI_SHELL_OWNER_CONFIRMATION_UI_CHILD", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for name in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let mut child = OwnerDialogTestChild(Some(
            command
                .spawn()
                .expect("isolated Owner確認dialog child process"),
        ));
        let child_process_id = child.0.as_ref().unwrap().id();
        let stdin = child
            .0
            .as_mut()
            .unwrap()
            .stdin
            .take()
            .expect("Owner確認dialog child stdin");
        serde_json::to_writer(stdin, &request).expect("Owner確認dialog test request");

        let deadline = Instant::now() + Duration::from_secs(15);
        let dialog = loop {
            if let Some(dialog) = find_owner_dialog(child_process_id) {
                break dialog;
            }
            assert!(
                Instant::now() < deadline,
                "15秒以内にchild process所有のOwner確認dialogを発見できない"
            );
            thread::sleep(Duration::from_millis(25));
        };

        let displayed_text = dialog
            .GetDlgItem(ID_STATIC_TEXT)
            .and_then(|control| control.GetWindowText())
            .expect("Owner確認dialogの本文control");
        assert_eq!(
            displayed_text, expected_text,
            "実表示がproduction確認文と異なる"
        );
        let yes_button = dialog
            .GetDlgItem(IDYES)
            .and_then(|button| button.GetWindowText())
            .expect("Owner確認dialogのYes button");
        let no_button = dialog
            .GetDlgItem(IDNO)
            .and_then(|button| button.GetWindowText())
            .expect("Owner確認dialogのNo button");
        let yes_label = yes_button
            .split(['(', '（'])
            .next()
            .unwrap_or_default()
            .trim();
        let no_label = no_button
            .split(['(', '（'])
            .next()
            .unwrap_or_default()
            .trim();
        assert!(
            yes_label.eq_ignore_ascii_case("Yes") || yes_label == "はい",
            "Win32 MessageBoxのYes button labelが想定外"
        );
        assert!(
            no_label.eq_ignore_ascii_case("No") || no_label == "いいえ",
            "Win32 MessageBoxのNo button labelが想定外"
        );

        assert!(
            dialog.SetForegroundWindow(),
            "Owner確認dialogをforegroundへ設定できない"
        );
        let foreground_deadline = Instant::now() + Duration::from_secs(5);
        while !HWND::GetForegroundWindow()
            .is_some_and(|foreground| is_owner_dialog(&foreground, child_process_id))
        {
            assert!(
                Instant::now() < foreground_deadline,
                "Owner確認dialogがforegroundにならないためキーを送らない"
            );
            thread::sleep(Duration::from_millis(25));
        }

        assert!(
            wait_for_focus(&dialog, IDNO, Duration::from_secs(5)),
            "Owner確認dialogの既定focusがNo buttonではないためキーを送らない"
        );
        if confirm {
            send_key(co::VK::TAB);
            assert!(
                wait_for_focus(&dialog, IDYES, Duration::from_secs(2)),
                "Tab後のfocusがYes buttonではないため確定キーを送らない"
            );
        }
        assert!(
            HWND::GetForegroundWindow()
                .is_some_and(|foreground| is_owner_dialog(&foreground, child_process_id)),
            "キー送信直前にforegroundが変わったため操作を中止する"
        );
        send_key(co::VK::RETURN);

        let child_deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if child
                .0
                .as_mut()
                .unwrap()
                .try_wait()
                .expect("Owner確認dialog child process status")
                .is_some()
            {
                break;
            }
            assert!(
                Instant::now() < child_deadline,
                "Owner確認dialog選択後、child processが10秒以内に終了しない"
            );
            thread::sleep(Duration::from_millis(25));
        }
        let output = child
            .0
            .take()
            .unwrap()
            .wait_with_output()
            .expect("Owner確認dialog child output");
        assert!(
            output.status.success(),
            "実Win32 dialogの選択結果がchild processで期待値と一致しない"
        );
        confirm
    }

    #[cfg(windows)]
    struct OwnerDialogTestChild(Option<Child>);

    #[cfg(windows)]
    impl Drop for OwnerDialogTestChild {
        fn drop(&mut self) {
            if let Some(child) = self.0.as_mut() {
                if child.try_wait().ok().flatten().is_none() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
    }

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

    #[cfg(windows)]
    #[test]
    #[ignore = "対話型Windows desktopで実Win32 Owner dialogを制御自動操作する"]
    #[allow(non_snake_case)]
    fn nativeOwner確認dialogのYesNoを制御UI自動化できTask本文を露出しない() {
        let instruction = "owner-ui-test-private-instruction-marker";
        let summary = DesktopOwnerOperationSummary::AgentTaskOwnerApproval {
            runtime_id: "owner-ui-test-runtime".into(),
            session_id: "owner-ui-test-session".into(),
            workspace_id: "owner-ui-test-workspace".into(),
            instruction_characters: instruction.chars().count(),
            instruction_hash: sha256_tagged(instruction.as_bytes()),
            payload_hash: sha256_tagged(b"owner-ui-test-payload"),
        };

        assert!(!automate_native_owner_confirmation(
            &summary,
            false,
            Some(instruction)
        ));
        assert!(automate_native_owner_confirmation(
            &summary,
            true,
            Some(instruction)
        ));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "対話型Windowsで実Win32 Export Owner dialogとRust Brokerの生成結果を接続する"]
    #[allow(non_snake_case)]
    fn nativeOwner確認dialogのExportNoYesをBrokerReceiptManifestまで接続する() {
        const PRIVATE_COMPOSE_ID_MARKER: &str = "ui-private-compose-marker";

        let root = test_root("desktop-export-owner-dialog");
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
        let server_export_root =
            cap_std::fs::Dir::open_ambient_dir(&server_export_dir, cap_std::ambient_authority())
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

        let mut rejected_request = desktop_export_request(
            "desktop-export-owner-dialog-no",
            "desktop-export-owner-dialog-no-nonce",
        );
        rejected_request["payload"]["compose_manifest"]["compose_id"] =
            serde_json::Value::String(PRIVATE_COMPOSE_ID_MARKER.to_string());
        rejected_request["payload_hash"] =
            serde_json::Value::String(canonical_payload_hash(rejected_request.get("payload")));
        let mut rejected_prompt_count = 0;
        let rejected = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&rejected_request).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                rejected_prompt_count += 1;
                assert!(matches!(
                    summary,
                    DesktopOwnerOperationSummary::GuiShellExport(_)
                ));
                assert!(!owner_confirmation_text(summary).contains(PRIVATE_COMPOSE_ID_MARKER));
                automate_native_owner_confirmation(summary, false, Some(PRIVATE_COMPOSE_ID_MARKER))
            },
        )
        .unwrap();
        let rejected: serde_json::Value = serde_json::from_slice(&rejected).unwrap();
        assert_eq!(rejected_prompt_count, 1);
        assert_eq!(rejected["status"], "rejected");
        assert_eq!(rejected["error"]["code"], "owner_required");
        assert!(fs::read_dir(&export_dir).unwrap().next().is_none());

        let mut accepted_request = desktop_export_request(
            "desktop-export-owner-dialog-yes",
            "desktop-export-owner-dialog-yes-nonce",
        );
        accepted_request["payload"]["compose_manifest"]["compose_id"] =
            serde_json::Value::String(PRIVATE_COMPOSE_ID_MARKER.to_string());
        accepted_request["payload_hash"] =
            serde_json::Value::String(canonical_payload_hash(accepted_request.get("payload")));
        let mut accepted_prompt_count = 0;
        let accepted =
            relay_channel_frame_with_owner_operations(
                gui_shell_windows_broker_channel::PipeFrame::Line(
                    serde_json::to_vec(&accepted_request).unwrap(),
                ),
                &broker_endpoint,
                Some(&owner_operation_tx),
                |summary| {
                    accepted_prompt_count += 1;
                    let DesktopOwnerOperationSummary::GuiShellExport(summary) = summary else {
                        panic!("Export要求はExport固有の確認summaryを使う")
                    };
                    assert_eq!(summary.export_id, "export-desktop-test");
                    assert_eq!(summary.optional_module_count, 1);
                    assert_eq!(
                        summary.payload_hash,
                        accepted_request["payload_hash"].as_str().unwrap()
                    );
                    assert!(!owner_confirmation_text(
                        &DesktopOwnerOperationSummary::GuiShellExport(summary.clone())
                    )
                    .contains(PRIVATE_COMPOSE_ID_MARKER));
                    automate_native_owner_confirmation(
                        &DesktopOwnerOperationSummary::GuiShellExport(summary.clone()),
                        true,
                        Some(PRIVATE_COMPOSE_ID_MARKER),
                    )
                },
            )
            .unwrap();
        let accepted: serde_json::Value = serde_json::from_slice(&accepted).unwrap();
        assert_eq!(accepted_prompt_count, 1);
        assert_eq!(accepted["status"], "accepted");
        assert_eq!(accepted["request_id"], "desktop-export-owner-dialog-yes");
        assert_eq!(accepted["body"]["authority_strip"], true);
        assert_eq!(accepted["body"]["credential_inherited"], false);
        assert_eq!(accepted["body"]["manifest_file_status"], "written");
        assert_eq!(accepted["body"]["build_status"], "not_started");
        assert_eq!(accepted["body"]["artifact_status"], "not_built");

        let manifest_path = accepted["body"]["manifest_file"]["path"].as_str().unwrap();
        let manifest_bytes = fs::read(manifest_path).unwrap();
        assert_eq!(
            crate::audit_hash::sha256_tagged(&manifest_bytes),
            accepted["body"]["manifest_file"]["sha256"]
        );
        let manifest_file: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
        assert_eq!(manifest_file["product"], "D4 Pocket");
        assert_eq!(
            manifest_file["manifest"]["inheritance_policy"]["credential"],
            "none"
        );
        assert_eq!(
            manifest_file["manifest"]["inheritance_policy"]["authority"],
            "none"
        );
        assert_eq!(
            manifest_file["manifest"]["app_identity"]["app_id"],
            accepted["body"]["export_manifest"]["app_identity"]["app_id"]
        );
        assert_eq!(
            manifest_file["manifest"]["audit_store"]["store_id"],
            accepted["body"]["export_manifest"]["audit_store"]["store_id"]
        );

        if let Some(capture_dir) = std::env::var_os("GUI_SHELL_EXPORT_E2E_CAPTURE_DIR") {
            let capture_dir = PathBuf::from(capture_dir);
            let metadata = fs::symlink_metadata(&capture_dir).unwrap();
            assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
            assert!(!is_reparse_point(&metadata));
            let resolved_capture = fs::canonicalize(&capture_dir).unwrap();
            let resolved_temp = fs::canonicalize(std::env::temp_dir()).unwrap();
            assert!(resolved_capture.starts_with(&resolved_temp));
            assert_ne!(resolved_capture, resolved_temp);
            assert!(fs::read_dir(&resolved_capture).unwrap().next().is_none());

            let receipt_bytes = serde_json::to_vec_pretty(&accepted["body"]).unwrap();
            fs::write(
                resolved_capture.join("d4_pocket_build_receipt.json"),
                receipt_bytes,
            )
            .unwrap();
            fs::write(
                resolved_capture.join("broker-generated-manifest.json"),
                &manifest_bytes,
            )
            .unwrap();
        }

        shutdown.store(true, Ordering::Release);
        server.join().unwrap().unwrap();
        let audit = fs::read_to_string(store_dir.join("audit.jsonl")).unwrap();
        assert!(audit.contains("desktop-export-owner-dialog-no"));
        assert!(audit.contains("desktop-export-owner-dialog-yes"));
        assert!(audit.contains("owner_required"));
        assert!(audit.contains("Rust Desktop起動器のネイティブ確認"));
        assert!(!audit.contains(PRIVATE_COMPOSE_ID_MARKER));

        broker_endpoint.session_secret.zeroize();
        session_bytes.zeroize();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "Owner確認dialog自動操作test専用の子process入口"]
    fn owner_confirmation_dialog_child() {
        use std::io::Read;

        assert_eq!(
            std::env::var_os("GUI_SHELL_OWNER_CONFIRMATION_UI_CHILD").as_deref(),
            Some(OsStr::new("1")),
            "child process markerが一致しない"
        );
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "snake_case")]
        enum TestSummary {
            GuiShellExport {
                display_name: String,
                export_id: String,
                distribution_channel: String,
                optional_module_count: usize,
                payload_hash: String,
            },
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
            AgentTaskResultExposure {
                task_id: String,
                result_hash: String,
                content_visibility: String,
                payload_hash: String,
            },
            WorkspaceContentApproval {
                workspace_id: String,
                registration_hash: String,
                content_visibility: String,
                payload_hash: String,
            },
            WorkspaceContentRevocation {
                workspace_id: String,
                registration_hash: String,
                payload_hash: String,
            },
            WorkspaceBaselineCapture {
                workspace_id: String,
                registration_hash: String,
                payload_hash: String,
            },
            AgentCliRuntimeWorkspaceRegistration {
                adapter_id: String,
                interface_scope: String,
                runtime_id: String,
                cli_path: String,
                workspace_id: String,
                workspace_root: String,
                secret_paths: Vec<String>,
                provider_id: String,
                model_id: String,
                authentication_source: String,
                credential_id: Option<String>,
                payload_hash: String,
            },
        }
        #[derive(Deserialize)]
        struct TestRequest {
            summary: TestSummary,
            confirm: bool,
        }

        let mut input = String::new();
        std::io::stdin()
            .take(8 * 1024)
            .read_to_string(&mut input)
            .expect("Owner確認dialog child request");
        let request: TestRequest =
            serde_json::from_str(&input).expect("Owner確認dialog child request JSON");
        let summary = match request.summary {
            TestSummary::GuiShellExport {
                display_name,
                export_id,
                distribution_channel,
                optional_module_count,
                payload_hash,
            } => DesktopOwnerOperationSummary::GuiShellExport(ExportConfirmationSummary {
                display_name,
                export_id,
                distribution_channel,
                optional_module_count,
                payload_hash,
            }),
            TestSummary::AgentTaskWorkspacePermission {
                runtime_id,
                session_id,
                workspace_id,
                payload_hash,
            } => DesktopOwnerOperationSummary::AgentTaskWorkspacePermission {
                runtime_id,
                session_id,
                workspace_id,
                payload_hash,
            },
            TestSummary::AgentTaskOwnerApproval {
                runtime_id,
                session_id,
                workspace_id,
                instruction_characters,
                instruction_hash,
                payload_hash,
            } => DesktopOwnerOperationSummary::AgentTaskOwnerApproval {
                runtime_id,
                session_id,
                workspace_id,
                instruction_characters,
                instruction_hash,
                payload_hash,
            },
            TestSummary::AgentTaskResultExposure {
                task_id,
                result_hash,
                content_visibility,
                payload_hash,
            } => DesktopOwnerOperationSummary::AgentTaskResultExposure {
                task_id,
                result_hash,
                content_visibility,
                payload_hash,
            },
            TestSummary::WorkspaceContentApproval {
                workspace_id,
                registration_hash,
                content_visibility,
                payload_hash,
            } => DesktopOwnerOperationSummary::WorkspaceContentApproval {
                workspace_id,
                registration_hash,
                content_visibility,
                payload_hash,
            },
            TestSummary::WorkspaceContentRevocation {
                workspace_id,
                registration_hash,
                payload_hash,
            } => DesktopOwnerOperationSummary::WorkspaceContentRevocation {
                workspace_id,
                registration_hash,
                payload_hash,
            },
            TestSummary::WorkspaceBaselineCapture {
                workspace_id,
                registration_hash,
                payload_hash,
            } => DesktopOwnerOperationSummary::WorkspaceBaselineCapture {
                workspace_id,
                registration_hash,
                payload_hash,
            },
            TestSummary::AgentCliRuntimeWorkspaceRegistration {
                adapter_id,
                interface_scope,
                runtime_id,
                cli_path,
                workspace_id,
                workspace_root,
                secret_paths,
                provider_id,
                model_id,
                authentication_source,
                credential_id,
                payload_hash,
            } => DesktopOwnerOperationSummary::AgentCliRuntimeWorkspaceRegistration {
                adapter_id,
                interface_scope,
                runtime_id,
                cli_path,
                workspace_id,
                workspace_root,
                secret_paths,
                provider_id,
                model_id,
                authentication_source,
                credential_id,
                payload_hash,
            },
        };
        assert_eq!(
            confirm_owner_operation(&summary, None),
            request.confirm,
            "実Win32 Owner確認dialogが選択を期待値へ反映しない"
        );
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
    fn launcher_recovery_removes_stale_endpoint_and_partial_file_but_keeps_audit() {
        let root = test_root("startup-session-recovery");
        let session_file = root.join(SESSION_FILE);
        let temporary_file = session_file.with_extension("json.tmp");
        let audit_file = root.join("store").join("audit.jsonl");
        fs::create_dir_all(audit_file.parent().unwrap()).unwrap();
        fs::write(&session_file, b"stale-synthetic-endpoint").unwrap();
        fs::write(&temporary_file, b"partial-synthetic-endpoint").unwrap();
        fs::write(&audit_file, b"durable-audit-marker").unwrap();

        prepare_session_paths(&session_file).expect("起動時の古い接続file回収");

        assert!(!session_file.exists(), "前回Brokerのendpointを回収する");
        assert!(
            !temporary_file.exists(),
            "不完全なendpoint一時fileを回収する"
        );
        assert_eq!(
            fs::read(&audit_file).unwrap(),
            b"durable-audit-marker",
            "永続Audit storeは起動時回収の対象外"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn launcher_recovery_rejects_invalid_stale_endpoint_without_deleting_it() {
        let root = test_root("startup-session-invalid");
        let session_file = root.join(SESSION_FILE);
        let temporary_file = session_file.with_extension("json.tmp");
        fs::create_dir(&session_file).unwrap();
        fs::write(&temporary_file, b"partial-synthetic-endpoint").unwrap();

        let error = prepare_session_paths(&session_file).unwrap_err();

        assert_eq!(error.code, "SESSION_FILE_INVALID");
        assert!(session_file.is_dir(), "不正なpathを削除しない");
        assert!(temporary_file.is_file(), "拒否後に別fileを掃除しない");
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
        assert_eq!(
            owner_rejected["error"]["code"],
            "desktop_native_owner_confirmation_required"
        );

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
        let server_export_root =
            cap_std::fs::Dir::open_ambient_dir(&server_export_dir, cap_std::ambient_authority())
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
        )
        .unwrap();
        let normal: serde_json::Value = serde_json::from_slice(&normal).unwrap();
        assert_eq!(normal["status"], "rejected");
        assert_eq!(normal["error"]["code"], "owner_required");

        let confirmed =
            desktop_export_request("desktop-export-confirmed", "desktop-export-confirmed-nonce");
        let mut prompt_count = 0;
        let accepted = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&confirmed).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                prompt_count += 1;
                let DesktopOwnerOperationSummary::GuiShellExport(summary) = summary else {
                    panic!("Export要求はExport固有の確認summaryを使う")
                };
                assert_eq!(summary.export_id, "export-desktop-test");
                assert_eq!(summary.optional_module_count, 1);
                assert_eq!(
                    summary.payload_hash,
                    confirmed["payload_hash"].as_str().unwrap()
                );
                true
            },
        )
        .unwrap();
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
        assert_eq!(
            manifest_file["manifest"]["inheritance_policy"]["credential"],
            "none"
        );

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
                assert_eq!(
                    summary.definition_hash,
                    format!("sha256:{}", "d".repeat(64))
                );
                assert_eq!(
                    summary.ciphertext_hash,
                    format!("sha256:{}", "e".repeat(64))
                );
                assert_eq!(payload_hash, delete["payload_hash"].as_str().unwrap());
                let text =
                    owner_confirmation_text(&DesktopOwnerOperationSummary::RegressionCaseDelete {
                        summary: (*summary).clone(),
                        payload_hash: (*payload_hash).clone(),
                    });
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
                assert_eq!(
                    summary.input_characters,
                    "owner-authored-redacted-input".chars().count()
                );
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
        secret_registration["request_id"] =
            serde_json::Value::String("desktop-registration-secret-marker".into());
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

        let declined =
            desktop_export_request("desktop-export-declined", "desktop-export-declined-nonce");
        let declined = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&declined).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |_| false,
        )
        .unwrap();
        let declined: serde_json::Value = serde_json::from_slice(&declined).unwrap();
        assert_eq!(declined["status"], "rejected");
        assert_eq!(declined["error"]["code"], "owner_required");

        let task_permission_payload = serde_json::json!({
            "agent_runtime_id": "fixture-agent",
            "session_id": "fixture-session",
            "workspace_id": "fixture-workspace"
        });
        let task_permission = desktop_owner_request(
            "AgentTaskWorkspacePermissionGrant",
            "desktop-agent-task-permission-declined",
            "desktop-agent-task-permission-declined-nonce",
            task_permission_payload,
        );
        let mut task_permission_prompt_count = 0;
        let declined_task_permission = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&task_permission).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                task_permission_prompt_count += 1;
                let DesktopOwnerOperationSummary::AgentTaskWorkspacePermission {
                    runtime_id,
                    session_id,
                    workspace_id,
                    payload_hash,
                } = summary
                else {
                    panic!("Agent Task Permissionは固定範囲のnative確認を使う")
                };
                assert_eq!(runtime_id, "fixture-agent");
                assert_eq!(session_id, "fixture-session");
                assert_eq!(workspace_id, "fixture-workspace");
                assert_eq!(
                    payload_hash,
                    task_permission["payload_hash"].as_str().unwrap()
                );
                false
            },
        )
        .unwrap();
        let declined_task_permission: serde_json::Value =
            serde_json::from_slice(&declined_task_permission).unwrap();
        assert_eq!(task_permission_prompt_count, 1);
        assert_eq!(declined_task_permission["status"], "rejected");
        assert_eq!(
            declined_task_permission["error"]["code"],
            "desktop_native_owner_confirmation_required"
        );

        let declined_instruction = "Ownerが拒否したTask本文";
        let task_approval_payload = serde_json::json!({
            "agent_runtime_id": "fixture-agent",
            "session_id": "fixture-session",
            "workspace_id": "fixture-workspace",
            "instruction": declined_instruction
        });
        let task_approval = desktop_owner_request(
            "AgentTaskOwnerApprovalGrant",
            "desktop-agent-task-approval-declined",
            "desktop-agent-task-approval-declined-nonce",
            task_approval_payload,
        );
        let mut task_approval_prompt_count = 0;
        let declined_task_approval = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&task_approval).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                task_approval_prompt_count += 1;
                let DesktopOwnerOperationSummary::AgentTaskOwnerApproval {
                    runtime_id,
                    session_id,
                    workspace_id,
                    instruction_hash,
                    payload_hash,
                    ..
                } = summary
                else {
                    panic!("Agent Task Approvalは本文を表示しないnative確認を使う")
                };
                assert_eq!(runtime_id, "fixture-agent");
                assert_eq!(session_id, "fixture-session");
                assert_eq!(workspace_id, "fixture-workspace");
                assert_eq!(
                    instruction_hash,
                    &sha256_tagged(declined_instruction.as_bytes())
                );
                assert_eq!(
                    payload_hash,
                    task_approval["payload_hash"].as_str().unwrap()
                );
                assert!(!owner_confirmation_text(summary).contains(declined_instruction));
                false
            },
        )
        .unwrap();
        let declined_task_approval: serde_json::Value =
            serde_json::from_slice(&declined_task_approval).unwrap();
        assert_eq!(task_approval_prompt_count, 1);
        assert_eq!(declined_task_approval["status"], "rejected");
        assert_eq!(
            declined_task_approval["error"]["code"],
            "desktop_native_owner_confirmation_required"
        );
        assert!(!declined_task_approval
            .to_string()
            .contains(declined_instruction));

        let confirmed_permission = desktop_owner_request(
            "AgentTaskWorkspacePermissionGrant",
            "desktop-agent-task-permission-confirmed-without-runtime",
            "desktop-agent-task-permission-confirmed-without-runtime-nonce",
            serde_json::json!({
                "agent_runtime_id": "unregistered-fixture-agent",
                "session_id": "fixture-session",
                "workspace_id": "fixture-workspace"
            }),
        );
        let mut confirmed_permission_prompt_count = 0;
        let denied_unregistered_permission = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&confirmed_permission).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                confirmed_permission_prompt_count += 1;
                let DesktopOwnerOperationSummary::AgentTaskWorkspacePermission {
                    runtime_id,
                    session_id,
                    workspace_id,
                    payload_hash,
                } = summary
                else {
                    panic!("Permission確認はAgent Task固有のnative確認を使う")
                };
                assert_eq!(runtime_id, "unregistered-fixture-agent");
                assert_eq!(session_id, "fixture-session");
                assert_eq!(workspace_id, "fixture-workspace");
                assert_eq!(
                    payload_hash,
                    confirmed_permission["payload_hash"].as_str().unwrap()
                );
                true
            },
        )
        .unwrap();
        let denied_unregistered_permission: serde_json::Value =
            serde_json::from_slice(&denied_unregistered_permission).unwrap();
        assert_eq!(confirmed_permission_prompt_count, 1);
        assert_eq!(denied_unregistered_permission["status"], "rejected");
        assert_eq!(
            denied_unregistered_permission["error"]["code"],
            "作業領域不在"
        );

        let unregistered_instruction = "未登録Agentへ発行しないOwner Approval本文";
        let confirmed_approval = desktop_owner_request(
            "AgentTaskOwnerApprovalGrant",
            "desktop-agent-task-approval-confirmed-without-runtime",
            "desktop-agent-task-approval-confirmed-without-runtime-nonce",
            serde_json::json!({
                "agent_runtime_id": "unregistered-fixture-agent",
                "session_id": "fixture-session",
                "workspace_id": "fixture-workspace",
                "instruction": unregistered_instruction
            }),
        );
        let mut confirmed_approval_prompt_count = 0;
        let denied_unregistered_approval = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&confirmed_approval).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                confirmed_approval_prompt_count += 1;
                let DesktopOwnerOperationSummary::AgentTaskOwnerApproval {
                    runtime_id,
                    session_id,
                    workspace_id,
                    instruction_hash,
                    payload_hash,
                    ..
                } = summary
                else {
                    panic!("Approval確認はTask本文を表示しないnative確認を使う")
                };
                assert_eq!(runtime_id, "unregistered-fixture-agent");
                assert_eq!(session_id, "fixture-session");
                assert_eq!(workspace_id, "fixture-workspace");
                assert_eq!(
                    instruction_hash,
                    &sha256_tagged(unregistered_instruction.as_bytes())
                );
                assert_eq!(
                    payload_hash,
                    confirmed_approval["payload_hash"].as_str().unwrap()
                );
                assert!(!owner_confirmation_text(summary).contains(unregistered_instruction));
                true
            },
        )
        .unwrap();
        let denied_unregistered_approval: serde_json::Value =
            serde_json::from_slice(&denied_unregistered_approval).unwrap();
        assert_eq!(confirmed_approval_prompt_count, 1);
        assert_eq!(denied_unregistered_approval["status"], "rejected");
        assert_eq!(
            denied_unregistered_approval["error"]["code"],
            "作業領域不在"
        );
        assert!(!denied_unregistered_approval
            .to_string()
            .contains(unregistered_instruction));

        let mut changed_hash = desktop_export_request(
            "desktop-export-changed-hash",
            "desktop-export-changed-hash-nonce",
        );
        changed_hash["payload_hash"] = serde_json::Value::String("sha256:forged".into());
        let mut invalid_prompt_count = 0;
        let rejected = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&changed_hash).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |_| {
                invalid_prompt_count += 1;
                true
            },
        )
        .unwrap();
        let rejected: serde_json::Value = serde_json::from_slice(&rejected).unwrap();
        assert_eq!(invalid_prompt_count, 0);
        assert_eq!(rejected["error"]["code"], "broker_payload_hash_invalid");

        let mut authority_metadata = desktop_export_request(
            "desktop-export-authority-metadata",
            "desktop-export-authority-metadata-nonce",
        );
        authority_metadata["metadata"]["authority"] = serde_json::Value::Bool(true);
        let rejected = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&authority_metadata).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |_| panic!("権限metadataで確認画面へ到達してはならない"),
        )
        .unwrap();
        let rejected: serde_json::Value = serde_json::from_slice(&rejected).unwrap();
        assert_eq!(
            rejected["error"]["code"],
            "broker_authority_metadata_rejected"
        );

        let mut forged_session = desktop_export_request(
            "desktop-export-forged-session",
            "desktop-export-forged-session-nonce",
        );
        forged_session["session_id"] = serde_json::Value::String("forged-session".into());
        let rejected = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&forged_session).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |_| panic!("偽造sessionで確認画面へ到達してはならない"),
        )
        .unwrap();
        let rejected: serde_json::Value = serde_json::from_slice(&rejected).unwrap();
        assert_eq!(rejected["error"]["code"], "broker_request_malformed");

        let mut stale =
            desktop_export_request("desktop-export-stale", "desktop-export-stale-nonce");
        stale["issued_at"] = serde_json::Value::String("2000-01-01T00:00:00Z".into());
        let rejected = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(serde_json::to_vec(&stale).unwrap()),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |_| panic!("期限切れ要求で確認画面へ到達してはならない"),
        )
        .unwrap();
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
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&non_export).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |_| panic!("allowlist外のOwner操作で確認画面を表示してはならない"),
        )
        .unwrap();
        let rejected: serde_json::Value = serde_json::from_slice(&rejected).unwrap();
        assert_eq!(rejected["status"], "rejected");

        shutdown.store(true, Ordering::Release);
        server.join().unwrap().unwrap();
        let audit = fs::read_to_string(store_dir.join("audit.jsonl")).unwrap();
        assert!(audit.contains("Rust Desktop起動器のネイティブ確認"));
        assert!(audit.contains("Desktop固定ProtectedStore起動"));
        assert!(audit.contains("owner_required"));
        assert!(audit.contains("AgentTaskWorkspacePermissionGrant"));
        assert!(audit.contains("AgentTaskOwnerApprovalGrant"));
        assert!(audit.contains("desktop_native_owner_confirmation_required"));
        assert!(audit.contains("作業領域不在"));
        assert!(!audit.contains("Ownerが拒否したTask本文"));
        assert!(!audit.contains(unregistered_instruction));
        assert!(audit.contains("broker_payload_hash_invalid"));
        broker_endpoint.session_secret.zeroize();
        session_bytes.zeroize();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "GUI_SHELL_CODEX_TASK_BROKER_TEST_EXEで指定したCodex CLIを登録し、実Win32 Owner確認を自動操作する。Taskは実行しない"]
    #[allow(non_snake_case)]
    fn 登録CodexへのnativeOwner確認後もAgentTask非対応gateを維持する() {
        const RUNTIME_ID: &str = "installed-codex-fixture";
        const WORKSPACE_ID: &str = "registered-workspace-fixture";

        let executable = std::env::var_os("GUI_SHELL_CODEX_TASK_BROKER_TEST_EXE")
            .map(PathBuf::from)
            .expect("検証対象Codex CLIを環境変数で明示する");
        assert!(
            executable.is_absolute(),
            "Codex CLI pathは絶対pathで指定する"
        );

        let root = test_root("desktop-owner-registered-codex");
        let workspace_root = root.parent().unwrap().join(format!(
            "{}-workspace",
            root.file_name().unwrap().to_string_lossy()
        ));
        assert!(!workspace_root.exists(), "合成Workspace pathが一意である");
        fs::create_dir(&workspace_root).unwrap();
        fs::write(
            workspace_root.join(".env"),
            b"synthetic-secret-content-never-returned",
        )
        .unwrap();
        let workspace_root = fs::canonicalize(workspace_root).unwrap();

        let session_file = root.join(SESSION_FILE);
        let owner_session_file = root.join("owner-session.json");
        let store_dir = root.join("store");
        let shutdown = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (owner_operation_tx, owner_operation_rx) = mpsc::sync_channel(1);
        let server_shutdown = Arc::clone(&shutdown);
        let server_session_file = session_file.clone();
        let server_store_dir = store_dir.clone();
        let server = thread::spawn(move || {
            let mut config = BrokerServerConfig::new(server_store_dir, server_session_file);
            config.owner_session_file = Some(owner_session_file);
            config.desktop_install_path_verified = true;
            crate::broker::ipc_server::run_loopback_server_cancellable_with_owner_operations(
                config,
                server_shutdown,
                ready_tx,
                owner_operation_rx,
                None,
            )
        });
        if let Err(error) = ready_rx.recv_timeout(Duration::from_secs(20)) {
            shutdown.store(true, Ordering::Release);
            let server_result = server.join();
            panic!("Codex登録Brokerの起動失敗: {error:?}; server={server_result:?}");
        }
        let (mut session_bytes, mut broker_endpoint) = read_endpoint(&session_file).unwrap();

        let registration_payload = serde_json::json!({
            "version": 1,
            "adapter_id": "codex-cli",
            "runtime_id": RUNTIME_ID,
            "cli_path": executable.to_string_lossy(),
            "workspace_id": WORKSPACE_ID,
            "workspace_root": workspace_root.to_string_lossy(),
            "secret_paths": [".env"],
            "provider_model_selection": {
                "version": 1,
                "provider_id": "openai_codex_cli",
                "model_id": "model-test-01",
                "authentication_source": "codex_cli_managed",
                "automatic_fallback": false
            }
        });
        let validate_registration_summary =
            |summary: &DesktopOwnerOperationSummary, expected_payload_hash: &str| {
                let DesktopOwnerOperationSummary::AgentCliRuntimeWorkspaceRegistration {
                    adapter_id,
                    interface_scope,
                    runtime_id,
                    cli_path,
                    workspace_id,
                    workspace_root: confirmed_workspace_root,
                    secret_paths,
                    provider_id,
                    model_id,
                    authentication_source,
                    credential_id,
                    payload_hash,
                } = summary
                else {
                    panic!("Agent CLI登録はscope固定のnative Owner確認を使う")
                };
                assert_eq!(adapter_id, "codex-cli");
                assert_eq!(
                    interface_scope,
                    "--version と exec --help（--model対応）"
                );
                assert_eq!(runtime_id, RUNTIME_ID);
                assert_eq!(cli_path, executable.to_str().unwrap());
                assert_eq!(workspace_id, WORKSPACE_ID);
                assert_eq!(confirmed_workspace_root, workspace_root.to_str().unwrap());
                assert_eq!(secret_paths.len(), 1);
                assert_eq!(secret_paths[0], ".env");
                assert_eq!(provider_id, "openai_codex_cli");
                assert_eq!(model_id, "model-test-01");
                assert_eq!(authentication_source, "codex_cli_managed");
                assert_eq!(credential_id, &None);
                assert_eq!(payload_hash, expected_payload_hash);
                let confirmation_text = owner_confirmation_text(summary);
                assert!(confirmation_text.contains(AGENT_CLI_REGISTRATION_NOTICE));
                assert!(confirmation_text.contains("model-test-01"));
                assert!(confirmation_text.contains("自動代替実行: 無効"));
                assert!(confirmation_text.contains(".env"));
                assert!(!confirmation_text.contains("synthetic-secret-content-never-returned"));
            };

        let declined_registration = desktop_owner_request(
            "AgentCLI実行系作業領域登録",
            "desktop-codex-runtime-registration-declined",
            "desktop-codex-runtime-registration-declined-nonce",
            registration_payload.clone(),
        );
        let mut declined_registration_confirmation_count = 0;
        let declined_registration_response = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&declined_registration).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                declined_registration_confirmation_count += 1;
                validate_registration_summary(
                    summary,
                    declined_registration["payload_hash"].as_str().unwrap(),
                );
                automate_native_owner_confirmation(summary, false, None)
            },
        )
        .expect("native Ownerが拒否したAgent CLI登録応答");
        let declined_registration_response: serde_json::Value =
            serde_json::from_slice(&declined_registration_response).unwrap();
        assert_eq!(declined_registration_confirmation_count, 1);
        assert_eq!(declined_registration_response["status"], "rejected");
        assert_eq!(
            declined_registration_response["error"]["code"],
            "desktop_native_owner_confirmation_required"
        );
        assert!(declined_registration_response["body"].is_null());

        let agents_after_decline = relay_channel_frame(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&desktop_owner_request(
                    "Agent一覧",
                    "desktop-codex-runtime-agent-list-after-decline",
                    "desktop-codex-runtime-agent-list-after-decline-nonce",
                    serde_json::json!({}),
                ))
                .unwrap(),
            ),
            &broker_endpoint,
        )
        .expect("No後の通常Desktop IPC Agent一覧");
        let agents_after_decline: serde_json::Value =
            serde_json::from_slice(&agents_after_decline).unwrap();
        assert_eq!(agents_after_decline["status"], "accepted");
        assert_eq!(agents_after_decline["body"]["Agent"], serde_json::json!([]));

        let registration = desktop_owner_request(
            "AgentCLI実行系作業領域登録",
            "desktop-codex-runtime-registration-confirmed",
            "desktop-codex-runtime-registration-confirmed-nonce",
            registration_payload,
        );
        let mut registration_confirmation_count = 0;
        let registration_response = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&registration).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                registration_confirmation_count += 1;
                validate_registration_summary(
                    summary,
                    registration["payload_hash"].as_str().unwrap(),
                );
                automate_native_owner_confirmation(
                    summary,
                    true,
                    Some("synthetic-secret-content-never-returned"),
                )
            },
        )
        .expect("native Owner確認後のAgent CLI登録応答");
        let registration_response: serde_json::Value =
            serde_json::from_slice(&registration_response).unwrap();
        assert_eq!(registration_confirmation_count, 1);
        assert_eq!(
            registration_response["status"], "accepted",
            "{registration_response}"
        );
        assert_eq!(registration_response["body"]["runtime_id"], RUNTIME_ID);
        assert_eq!(registration_response["body"]["workspace_id"], WORKSPACE_ID);
        assert_eq!(
            registration_response["body"]["task_execution"],
            "unsupported"
        );
        assert_eq!(registration_response["body"]["permission_generated"], false);
        assert_eq!(registration_response["body"]["approval_generated"], false);
        assert_eq!(
            registration_response["body"]["credential_value_accepted"],
            false
        );

        let agent_list = relay_channel_frame(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&desktop_owner_request(
                    "Agent一覧",
                    "desktop-codex-runtime-agent-list",
                    "desktop-codex-runtime-agent-list-nonce",
                    serde_json::json!({}),
                ))
                .unwrap(),
            ),
            &broker_endpoint,
        )
        .expect("通常Desktop IPCのAgent一覧");
        let agent_list: serde_json::Value = serde_json::from_slice(&agent_list).unwrap();
        assert_eq!(agent_list["status"], "accepted", "{agent_list}");
        let agent = agent_list["body"]["Agent"]
            .as_array()
            .unwrap()
            .iter()
            .find(|agent| agent["adapter_id"] == "codex-cli")
            .expect("native確認後の実CLI metadataが通常IPC projectionへ出る");
        assert!(agent["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|capability| {
                capability["capability_id"] == "task_execution"
                    && capability["support"]["status"] == "unsupported"
            }));

        let session_start = desktop_owner_request(
            "対話開始",
            "desktop-codex-task-session-start",
            "desktop-codex-task-session-start-nonce",
            serde_json::json!({
                "実行系ID": RUNTIME_ID,
                "作業領域ID": WORKSPACE_ID
            }),
        );
        let session_start = relay_channel_frame(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&session_start).unwrap(),
            ),
            &broker_endpoint,
        )
        .expect("通常資格IPCからAgent Sessionを作成する");
        let session_start: serde_json::Value = serde_json::from_slice(&session_start).unwrap();
        assert_eq!(session_start["status"], "accepted", "{session_start}");
        let agent_session_id = session_start["body"]["対話セッションID"]
            .as_str()
            .expect("Broker発行Session ID")
            .to_owned();

        let declined_permission = desktop_owner_request(
            "AgentTaskWorkspacePermissionGrant",
            "desktop-codex-task-permission-ui-declined",
            "desktop-codex-task-permission-ui-declined-nonce",
            serde_json::json!({
                "agent_runtime_id": RUNTIME_ID,
                "session_id": agent_session_id,
                "workspace_id": WORKSPACE_ID
            }),
        );
        let declined_permission_response = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&declined_permission).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                let DesktopOwnerOperationSummary::AgentTaskWorkspacePermission {
                    runtime_id,
                    session_id,
                    workspace_id,
                    payload_hash,
                } = summary
                else {
                    panic!("Workspace Permissionは実Win32 Owner確認を使う")
                };
                assert_eq!(runtime_id, RUNTIME_ID);
                assert_eq!(session_id, &agent_session_id);
                assert_eq!(workspace_id, WORKSPACE_ID);
                assert_eq!(
                    payload_hash,
                    declined_permission["payload_hash"].as_str().unwrap()
                );
                automate_native_owner_confirmation(summary, false, None)
            },
        )
        .expect("拒否後のBroker応答");
        let declined_permission_response: serde_json::Value =
            serde_json::from_slice(&declined_permission_response).unwrap();
        assert_eq!(declined_permission_response["status"], "rejected");
        assert_eq!(
            declined_permission_response["error"]["code"],
            "desktop_native_owner_confirmation_required"
        );

        let permission_payload = serde_json::json!({
            "agent_runtime_id": RUNTIME_ID,
            "session_id": agent_session_id,
            "workspace_id": WORKSPACE_ID
        });
        let permission = desktop_owner_request(
            "AgentTaskWorkspacePermissionGrant",
            "desktop-codex-task-permission-confirmed",
            "desktop-codex-task-permission-confirmed-nonce",
            permission_payload,
        );
        let mut permission_confirmation_count = 0;
        let permission_response = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&permission).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                permission_confirmation_count += 1;
                let DesktopOwnerOperationSummary::AgentTaskWorkspacePermission {
                    runtime_id,
                    session_id,
                    workspace_id,
                    payload_hash,
                } = summary
                else {
                    panic!("Workspace Permissionは固定scopeのnative確認を使う")
                };
                assert_eq!(runtime_id, RUNTIME_ID);
                assert_eq!(session_id, &agent_session_id);
                assert_eq!(workspace_id, WORKSPACE_ID);
                assert_eq!(payload_hash, permission["payload_hash"].as_str().unwrap());
                automate_native_owner_confirmation(summary, true, None)
            },
        )
        .expect("native確認後のBroker応答");
        let permission_response: serde_json::Value =
            serde_json::from_slice(&permission_response).unwrap();
        assert_eq!(permission_confirmation_count, 1);
        assert_eq!(permission_response["status"], "rejected");
        assert_eq!(permission_response["error"]["code"], "AgentTask実行非対応");
        assert!(permission_response["body"].is_null());

        let instruction = "fixture-only task; no Agent execution";
        let approval_payload = serde_json::json!({
            "agent_runtime_id": RUNTIME_ID,
            "session_id": agent_session_id,
            "workspace_id": WORKSPACE_ID,
            "instruction": instruction
        });
        let approval = desktop_owner_request(
            "AgentTaskOwnerApprovalGrant",
            "desktop-codex-task-approval-confirmed",
            "desktop-codex-task-approval-confirmed-nonce",
            approval_payload,
        );
        let mut approval_confirmation_count = 0;
        let approval_response = relay_channel_frame_with_owner_operations(
            gui_shell_windows_broker_channel::PipeFrame::Line(
                serde_json::to_vec(&approval).unwrap(),
            ),
            &broker_endpoint,
            Some(&owner_operation_tx),
            |summary| {
                approval_confirmation_count += 1;
                let DesktopOwnerOperationSummary::AgentTaskOwnerApproval {
                    runtime_id,
                    session_id,
                    workspace_id,
                    instruction_hash,
                    payload_hash,
                    ..
                } = summary
                else {
                    panic!("Task本文はnative確認画面へ露出しない")
                };
                assert_eq!(runtime_id, RUNTIME_ID);
                assert_eq!(session_id, &agent_session_id);
                assert_eq!(workspace_id, WORKSPACE_ID);
                assert_eq!(instruction_hash, &sha256_tagged(instruction.as_bytes()));
                assert_eq!(payload_hash, approval["payload_hash"].as_str().unwrap());
                assert!(!owner_confirmation_text(summary).contains(instruction));
                automate_native_owner_confirmation(summary, true, Some(instruction))
            },
        )
        .expect("native確認後のBroker応答");
        let approval_response: serde_json::Value =
            serde_json::from_slice(&approval_response).unwrap();
        assert_eq!(approval_confirmation_count, 1);
        assert_eq!(approval_response["status"], "rejected");
        assert_eq!(approval_response["error"]["code"], "AgentTask実行非対応");
        assert!(approval_response["body"].is_null());
        assert!(!approval_response.to_string().contains(instruction));

        let production_task_marker = "R2_DESKTOP_NATIVE_TASK_UNSUPPORTED_MARKER";
        let task_payload = serde_json::json!({
            "agent_runtime_id": RUNTIME_ID,
            "session_id": agent_session_id,
            "workspace_id": WORKSPACE_ID,
            "instruction": production_task_marker
        });
        let mut task_responses = Vec::new();
        for (operation, request_id, nonce) in [
            (
                "Agent作業要求検査",
                "desktop-codex-task-preflight",
                "desktop-codex-task-preflight-nonce",
            ),
            (
                "AgentTask実行",
                "desktop-codex-task-start",
                "desktop-codex-task-start-nonce",
            ),
        ] {
            let request = desktop_owner_request(operation, request_id, nonce, task_payload.clone());
            let response = relay_channel_frame(
                gui_shell_windows_broker_channel::PipeFrame::Line(
                    serde_json::to_vec(&request).unwrap(),
                ),
                &broker_endpoint,
            )
            .expect("通常Desktop IPCのAgent Task拒否応答");
            let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
            assert_eq!(response["status"], "rejected", "{operation}: {response}");
            assert_eq!(response["error"]["code"], "AgentTask実行非対応");
            assert!(response["body"].is_null());
            assert!(!response.to_string().contains(production_task_marker));
            task_responses.push(response);
        }

        shutdown.store(true, Ordering::Release);
        server.join().unwrap().unwrap();
        let audit = fs::read_to_string(store_dir.join("audit.jsonl")).unwrap();
        assert!(audit.contains("対話Sessionと登録済みWorkspaceの明示結合"));
        assert!(audit.contains("AgentCLI実行系作業領域登録"));
        assert!(!audit.contains("synthetic-secret-content-never-returned"));
        assert!(audit.contains("AgentTaskWorkspacePermissionGrant"));
        assert!(audit.contains("AgentTaskOwnerApprovalGrant"));
        assert!(audit.matches("AgentTask実行非対応").count() >= 2);
        assert!(
            !audit.contains("Agent Task用Workspace Permission発行（native Owner確認・Task未実行）")
        );
        assert!(!audit.contains(
            "Agent Task本文hash・実行条件hashへのOwner Approval発行（native確認・Task未実行）"
        ));
        assert!(!audit.contains(instruction));
        assert!(!audit.contains(production_task_marker));
        for response in &task_responses {
            let audit_id = response["audit_event_id"].as_str().unwrap();
            assert!(audit.contains(audit_id));
        }
        broker_endpoint.session_secret.zeroize();
        session_bytes.zeroize();
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(workspace_root).unwrap();
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
    fn launcher_recovery_keeps_runtime_state_separate_and_reuses_lock_anchor() {
        let root = fs::canonicalize(test_root("runtime")).unwrap();
        let runtime = runtime_directory_with_identity(&root, None).unwrap();
        assert!(runtime.starts_with(&root));
        assert!(runtime.ends_with(Path::new("GUI-Shell").join("broker").join("desktop")));
        let export_dir = ensure_export_directory(&runtime).unwrap();
        assert_eq!(export_dir, runtime.join(EXPORT_DIRECTORY_NAME));
        let lock_path = runtime.join("desktop_launcher.lock");
        let first = acquire_instance_lock(&runtime).unwrap();
        assert_eq!(fs::metadata(&lock_path).unwrap().len(), 0);
        let second = acquire_instance_lock(&runtime).unwrap_err();
        assert_eq!(second.code, "INSTANCE_ALREADY_RUNNING");
        drop(first);
        assert!(
            lock_path.is_file(),
            "lock anchor fileは安定したinodeとして残す"
        );
        assert_eq!(fs::metadata(&lock_path).unwrap().len(), 0);
        let restarted = acquire_instance_lock(&runtime).unwrap();
        drop(restarted);
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
    fn agent_cli_runtime_workspace_confirmation_matches_build_scope() {
        let summary = DesktopOwnerOperationSummary::AgentCliRuntimeWorkspaceRegistration {
            adapter_id: "codex-cli".into(),
            interface_scope: "--version と exec --help（--model対応）".into(),
            runtime_id: "codex-r2-synthetic".into(),
            cli_path: r"C:\Tools\Codex\codex.exe".into(),
            workspace_id: "workspace-r2-synthetic".into(),
            workspace_root: r"C:\d4-r2-synthetic-workspace".into(),
            secret_paths: vec![".env".into(), "secrets".into()],
            provider_id: "openai_codex_cli".into(),
            model_id: "model-test-01".into(),
            authentication_source: "codex_cli_managed".into(),
            credential_id: None,
            payload_hash: format!("sha256:{}", "a".repeat(64)),
        };
        let text = owner_confirmation_text(&summary);
        assert!(text.contains("codex-r2-synthetic"));
        assert!(text.contains(r"C:\Tools\Codex\codex.exe"));
        assert!(text.contains(r"C:\d4-r2-synthetic-workspace"));
        assert!(text.contains(".env"));
        assert!(text.contains("secrets"));
        assert!(text.contains("Adapter ID: codex-cli"));
        assert!(text.contains("--version と exec --help（--model対応）"));
        assert!(text.contains("openai_codex_cli / model-test-01"));
        assert!(text.contains("自動代替実行: 無効"));
        assert!(text.contains(AGENT_CLI_REGISTRATION_NOTICE));
        #[cfg(feature = "r2-e2e")]
        {
            assert!(text.contains("r2-e2e専用検証build"));
            assert!(text.contains("127.0.0.1偽Responses API"));
            assert!(!text.contains("Task実行能力はunsupported"));
        }
        #[cfg(not(feature = "r2-e2e"))]
        assert!(text.contains("Task実行能力はunsupported"));
        assert!(text.contains("Permission、Approval、Trust、Credentialを生成・保存しません"));
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
    fn A2A接続のnative確認はloopback対象を示し秘密とqueryを拒否する() {
        let payload = serde_json::json!({
            "版": 1,
            "操作": "接続",
            "AgentID": "remote-agent-fixture",
            "Agent Card URI": "http://127.0.0.1:9080/.well-known/agent-card.json",
            "protocol_version": "1.0",
            "Transport": "http",
            "Credential ref": {
                "credential_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "purpose": "A2A接続",
                "target": "remote-agent-fixture",
                "required": false,
                "status": "missing"
            }
        });
        let request = desktop_owner_request(
            "A2A接続",
            "desktop-a2a-connect",
            "desktop-a2a-connect-nonce",
            payload.clone(),
        );
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
            owner_operation_candidate(request.to_string().as_bytes(), &endpoint)
                .expect("A2A接続はnative Owner確認候補");
        assert_eq!(
            BrokerRequestEnvelope::from_json_str(&normalized)
                .unwrap()
                .session_id
                .as_deref(),
            Some("desktop-session")
        );
        let DesktopOwnerOperationSummary::A2aConnect {
            agent_id,
            target,
            payload_hash,
        } = summary
        else {
            panic!("A2A接続はA2A専用のnative確認を使う")
        };
        assert_eq!(agent_id, "remote-agent-fixture");
        assert_eq!(target, "http://127.0.0.1:9080/.well-known/agent-card.json");
        assert_eq!(payload_hash, request["payload_hash"]);
        let confirmation = owner_confirmation_text(&DesktopOwnerOperationSummary::A2aConnect {
            agent_id,
            target,
            payload_hash,
        });
        assert!(confirmation.contains("loopback IP限定"));
        assert!(confirmation.contains("Trustはpending review"));
        assert!(confirmation.contains("Task送信"));
        assert!(!confirmation.contains("credential_value"));

        for uri in [
            "http://127.0.0.1:9080/card?token=secret-marker",
            "http://192.0.2.7:9080/card",
            "https://127.0.0.1:9080/card",
            "http://user@127.0.0.1:9080/card",
        ] {
            let mut invalid_payload = payload.clone();
            invalid_payload["Agent Card URI"] = serde_json::json!(uri);
            let invalid_request = desktop_owner_request(
                "A2A接続",
                "desktop-a2a-invalid",
                "desktop-a2a-invalid-nonce",
                invalid_payload,
            );
            assert!(
                owner_operation_candidate(invalid_request.to_string().as_bytes(), &endpoint)
                    .is_none()
            );
        }

        let mut secret_payload = payload;
        secret_payload["Credential ref"]["secret_value"] =
            serde_json::json!("secret-marker");
        let secret_request = desktop_owner_request(
            "A2A接続",
            "desktop-a2a-secret",
            "desktop-a2a-secret-nonce",
            secret_payload,
        );
        assert!(
            owner_operation_candidate(secret_request.to_string().as_bytes(), &endpoint).is_none()
        );
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
        let (normalized, summary) =
            owner_operation_candidate(input.to_string().as_bytes(), &endpoint)
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
    fn MCP_Tool実行のnative確認は対象とhashを示し引数本文や秘密fieldを拒否する() {
        let tool_id = format!("tool-{}", "a".repeat(142));
        let payload = serde_json::json!({
            "版": 1,
            "操作": "実行",
            "ServerID": "mcp-fixture",
            "ToolID": tool_id,
            "名前": "search",
            "arguments": {"query": "SENSITIVE_ARGUMENT_MARKER"}
        });
        let input = serde_json::json!({
            "request_id": "desktop-mcp-tool-call",
            "operation": "MCP Tool実行",
            "payload": payload.clone(),
            "payload_hash": canonical_payload_hash(Some(&payload)),
            "nonce": "desktop-mcp-tool-call-nonce",
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
            .expect("Tool呼出しはnative Owner確認候補");
        let confirmation = owner_confirmation_text(&summary);
        assert!(confirmation.contains("mcp-fixture"));
        assert!(confirmation.contains("search"));
        assert!(confirmation.contains(&tool_id));
        assert!(confirmation.contains(&sha256_tagged(
            &serde_json::to_vec(&payload["arguments"]).expect("引数hash")
        )));
        assert!(confirmation.contains("一回限りPermission"));
        assert!(confirmation.contains("自動再送しません"));
        assert!(!confirmation.contains("SENSITIVE_ARGUMENT_MARKER"));

        let mut escalated = payload.clone();
        escalated["arguments"]["nested"] = serde_json::json!({"Credential_Value":"secret-marker"});
        let request = serde_json::json!({
            "request_id": "desktop-mcp-tool-call-escalated",
            "operation": "MCP Tool実行",
            "payload_hash": canonical_payload_hash(Some(&escalated)),
            "payload": escalated,
            "nonce": "desktop-mcp-tool-call-escalated-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        assert!(owner_operation_candidate(request.to_string().as_bytes(), &endpoint).is_none());

        let mut unknown = payload;
        unknown["extra"] = serde_json::json!(true);
        let request = serde_json::json!({
            "request_id": "desktop-mcp-tool-call-unknown",
            "operation": "MCP Tool実行",
            "payload_hash": canonical_payload_hash(Some(&unknown)),
            "payload": unknown,
            "nonce": "desktop-mcp-tool-call-unknown-nonce",
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
        let (normalized, summary) =
            owner_operation_candidate(input.to_string().as_bytes(), &endpoint)
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
        assert!(confirmation.contains("Credential: 未選択（秘密値を渡しません）"));
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
    fn mcp_credential_confirmation_shows_only_reference_and_warns_about_server_access() {
        let mut payload = serde_json::json!({
            "版": 1,
            "操作": "接続",
            "ServerID": "mcp-fixture",
            "実行file": r"C:\Program Files\D4 Pocket\mcp-fixture.exe",
            "引数": ["--stdio"],
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
        payload["Credential ref"] = serde_json::json!({
            "credential_id": "abcdef0123456789abcdef0123456789",
            "purpose": "mcp_transport",
            "target": "mcp-fixture",
            "required": true,
            "status": "configured",
            "environment_variable": "MCP_API_KEY"
        });
        let payload_hash = canonical_payload_hash(Some(&payload));
        let input = serde_json::json!({
            "request_id": "desktop-mcp-connect-credential",
            "operation": "MCP接続",
            "payload": payload,
            "payload_hash": payload_hash,
            "nonce": "desktop-mcp-connect-credential-nonce",
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
            .expect("configured Credentialはnative Owner確認必須");
        let confirmation = owner_confirmation_text(&summary);
        assert!(confirmation.contains("abcdef0123456789abcdef0123456789"));
        assert!(confirmation.contains("MCP_API_KEY"));
        assert!(confirmation.contains("外部送信できます"));
        assert!(confirmation.contains("sandboxではありません"));
        assert!(!confirmation.contains("secret-marker"));

        let mut unsafe_name = serde_json::json!({
            "版": 1,
            "操作": "接続",
            "ServerID": "mcp-fixture",
            "実行file": r"C:\Program Files\D4 Pocket\mcp-fixture.exe",
            "引数": ["--stdio"],
            "workspace": r"C:\Users\Public\D4PocketWorkspace",
            "Transport": "stdio",
            "Credential ref": {
                "credential_id": "abcdef0123456789abcdef0123456789",
                "purpose": "mcp_transport",
                "target": "mcp-fixture",
                "required": true,
                "status": "configured",
                "environment_variable": "MCP_API_KEY"
            }
        });
        unsafe_name["Credential ref"]["environment_variable"] = serde_json::json!("PATH");
        let unsafe_hash = canonical_payload_hash(Some(&unsafe_name));
        let request = serde_json::json!({
            "request_id": "desktop-mcp-connect-credential-unsafe-env",
            "operation": "MCP接続",
            "payload": unsafe_name,
            "payload_hash": unsafe_hash,
            "nonce": "desktop-mcp-connect-credential-unsafe-env-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        assert!(owner_operation_candidate(request.to_string().as_bytes(), &endpoint).is_none());
    }

    #[test]
    fn credential_revocation_native_confirmation_binds_one_id_and_displays_current_metadata() {
        let payload = serde_json::json!({
            "版": 1,
            "資格情報ID": "0123456789abcdef0123456789abcdef",
            "用途": "mcp_transport",
            "接続対象": "fixture-mcp-server",
            "暗号文hash": format!("sha256:{}", "a".repeat(64)),
            "作成監査ID": "credential-created-1"
        });
        let input = serde_json::json!({
            "request_id": "desktop-credential-revoke",
            "operation": "資格情報失効",
            "payload": payload,
            "payload_hash": canonical_payload_hash(Some(&payload)),
            "nonce": "desktop-credential-revoke-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        let endpoint = BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 43123,
            session_id: "desktop-session".into(),
            session_secret: "not-used-by-candidate".into(),
            credential_role: BrokerCredentialRole::Normal,
            transport: "tcp".into(),
            max_request_bytes: MAX_REQUEST_BYTES,
        };
        let (normalized, summary) =
            owner_operation_candidate(input.to_string().as_bytes(), &endpoint)
                .expect("論理失効をnative確認へ結び付ける");
        let request = BrokerRequestEnvelope::from_json_str(&normalized).expect("正規化要求");
        assert_eq!(request.session_id.as_deref(), Some("desktop-session"));
        assert_eq!(request.operation, Some(BrokerOperation::資格情報失効));
        let DesktopOwnerOperationSummary::CredentialRevocation {
            credential_id,
            purpose,
            target,
            ciphertext_hash,
            created_audit_id,
            payload_hash,
        } = summary
        else {
            panic!("Credential失効専用のnative確認要約を使う")
        };
        assert_eq!(credential_id, "0123456789abcdef0123456789abcdef");
        assert_eq!(purpose, "mcp_transport");
        assert_eq!(target, "fixture-mcp-server");
        assert_eq!(ciphertext_hash, input["payload"]["暗号文hash"]);
        assert_eq!(created_audit_id, "credential-created-1");
        assert_eq!(payload_hash, input["payload_hash"].as_str().unwrap());
        let text = owner_confirmation_text(&DesktopOwnerOperationSummary::CredentialRevocation {
            credential_id,
            purpose,
            target,
            ciphertext_hash,
            created_audit_id,
            payload_hash,
        });
        assert!(text.contains("論理失効"));
        assert!(text.contains("fixture-mcp-server"));
        assert!(text.contains("mcp_transport"));
        assert!(text.contains("登録監査ID: credential-created-1"));
        assert!(text.contains(&format!(
            "暗号文hash: {}",
            input["payload"]["暗号文hash"].as_str().unwrap()
        )));
        assert!(text.contains("以後のMCP注入を拒否"));
        assert!(text.contains("暗号文fileは削除せず"));
        assert!(text.contains("取消できません"));
        assert!(text.contains("payload hash"));
        assert!(!text.contains("secret-marker"));

        let mut escalated = payload.clone();
        escalated["秘密値"] = serde_json::json!("secret-marker");
        let invalid = serde_json::json!({
            "request_id": "desktop-credential-revoke-escalated",
            "operation": "資格情報失効",
            "payload": escalated,
            "payload_hash": canonical_payload_hash(Some(&escalated)),
            "nonce": "desktop-credential-revoke-escalated-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        assert!(owner_operation_candidate(invalid.to_string().as_bytes(), &endpoint).is_none());
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
    #[allow(non_snake_case)]
    fn AgentTask結果表示native確認はTask_hash_visibilityを固定し危険を明示する() {
        let task_id = "a".repeat(32);
        let result_hash = format!("sha256:{}", "b".repeat(64));
        let payload = serde_json::json!({
            "task_id": task_id,
            "result_hash": result_hash,
            "content_visibility": "full"
        });
        let input = serde_json::json!({
            "request_id": "desktop-agent-result-exposure",
            "operation": "AgentTask結果表示承認",
            "payload": payload,
            "payload_hash": canonical_payload_hash(Some(&payload)),
            "nonce": "desktop-agent-result-exposure-nonce",
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
            .expect("Task結果表示はnative確認候補");
        let text = owner_confirmation_text(&summary);
        assert!(text.contains(&task_id));
        assert!(text.contains(&result_hash));
        assert!(text.contains("full"));
        assert!(text.contains("秘密情報や危険な指示を含む可能性"));
        assert!(text.contains("test結果・主張はBroker検証済みではありません"));
        assert!(text.contains("Task実行ApprovalでもWorkspace読取許可でもなく"));

        for invalid_payload in [
            serde_json::json!({"task_id":"short", "result_hash":result_hash, "content_visibility":"full"}),
            serde_json::json!({"task_id":task_id, "result_hash":"raw", "content_visibility":"full"}),
            serde_json::json!({"task_id":task_id, "result_hash":result_hash, "content_visibility":"all"}),
            serde_json::json!({"task_id":task_id, "result_hash":result_hash, "content_visibility":"full", "approval_id":"caller-chosen"}),
        ] {
            let invalid = serde_json::json!({
                "request_id": "desktop-agent-result-invalid",
                "operation": "AgentTask結果表示承認",
                "payload": invalid_payload,
                "payload_hash": canonical_payload_hash(Some(&invalid_payload)),
                "nonce": "desktop-agent-result-invalid-nonce",
                "issued_at": BrokerRequestEnvelope::current_issued_at(),
                "metadata": {"client": "desktop_flutter"}
            });
            assert!(owner_operation_candidate(invalid.to_string().as_bytes(), &endpoint).is_none());
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn Workspace読取とbaselineのnative確認は対象と範囲を固定する() {
        let endpoint = BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 43123,
            session_id: "desktop-session".into(),
            session_secret: "not-used-by-candidate".into(),
            credential_role: BrokerCredentialRole::Owner,
            transport: "tcp".into(),
            max_request_bytes: MAX_REQUEST_BYTES,
        };
        let registration_hash = format!("sha256:{}", "b".repeat(64));
        let request = |request_id: &str, operation: &str, payload: Value| {
            serde_json::json!({
                "request_id": request_id,
                "operation": operation,
                "payload": payload,
                "payload_hash": canonical_payload_hash(Some(&payload)),
                "nonce": format!("{request_id}-nonce"),
                "issued_at": BrokerRequestEnvelope::current_issued_at(),
                "metadata": {"client": "desktop_flutter"}
            })
        };
        let approval_payload = serde_json::json!({
            "作業領域ID": "workspace-a",
            "登録hash": registration_hash,
            "表示範囲": "full"
        });
        let approval = request(
            "desktop-workspace-content-approval",
            "作業領域承認",
            approval_payload.clone(),
        );
        let (_, summary) = owner_operation_candidate(approval.to_string().as_bytes(), &endpoint)
            .expect("Workspace読取Approvalはnative確認候補");
        assert!(matches!(
            &summary,
            DesktopOwnerOperationSummary::WorkspaceContentApproval { .. }
        ));
        let prompt = owner_confirmation_text(&summary);
        assert!(prompt.contains("workspace-a"));
        assert!(prompt.contains(&registration_hash));
        assert!(prompt.contains("full"));
        assert!(prompt.contains("一般的な秘密検出ではありません"));
        assert!(prompt.contains("Task実行Permission／ApprovalでもWorkspace書込許可でもありません"));

        let baseline = request(
            "desktop-workspace-baseline",
            "作業領域全体基準点保存",
            serde_json::json!({
                "作業領域ID": "workspace-a",
                "登録hash": registration_hash
            }),
        );
        let (_, summary) = owner_operation_candidate(baseline.to_string().as_bytes(), &endpoint)
            .expect("Workspace全体baselineはnative確認候補");
        let prompt = owner_confirmation_text(&summary);
        assert!(prompt.contains("既存baselineがあれば置換します"));
        assert!(prompt.contains("Task IDには結合されない"));
        assert!(prompt.contains("Workspace fileの変更・Task実行はしません"));

        let revoked = request(
            "desktop-workspace-revoke",
            "作業領域失効",
            serde_json::json!({
                "作業領域ID": "workspace-a",
                "登録hash": registration_hash
            }),
        );
        let (_, summary) = owner_operation_candidate(revoked.to_string().as_bytes(), &endpoint)
            .expect("Workspace失効はnative確認候補");
        assert!(owner_confirmation_text(&summary)
            .contains("現在の読取許可とBroker内baselineを破棄します"));

        let escalated = request(
            "desktop-workspace-escalated",
            "作業領域承認",
            serde_json::json!({
                "作業領域ID": "workspace-a",
                "登録hash": registration_hash,
                "表示範囲": "full",
                "permission": "workspace.write"
            }),
        );
        assert!(owner_operation_candidate(escalated.to_string().as_bytes(), &endpoint).is_none());
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
        assert!(
            result.status.success(),
            "Export保存先junction試験の作成失敗"
        );

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
