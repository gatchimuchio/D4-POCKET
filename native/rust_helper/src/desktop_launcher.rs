//! Windows向けDesktop起動管理。権限判断はBrokerに残し、Flutterへ通常IPC資格だけを渡す。

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::broker::ipc_server::BrokerServerError;
use crate::broker::{
    run_loopback_server_cancellable, BrokerCredentialRole, BrokerEndpoint, BrokerServerConfig,
};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
const UI_POLL: Duration = Duration::from_millis(50);
const MAX_ENDPOINT_BYTES: u64 = 4096;
const MAX_REQUEST_BYTES: usize = 64 * 1024;
const SESSION_FILE: &str = "broker_session.json";

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

fn runtime_directory(local_app_data: &Path) -> Result<PathBuf, DesktopLaunchError> {
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
    for component in ["GUI-Shell", "broker", "desktop"] {
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
    file.by_ref()
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

struct RunningBroker {
    shutdown: Arc<AtomicBool>,
    server: Option<JoinHandle<Result<(), BrokerServerError>>>,
    session_file: PathBuf,
    session_bytes: Vec<u8>,
}

impl RunningBroker {
    fn start(runtime_dir: &Path) -> Result<Self, DesktopLaunchError> {
        let store_dir = ensure_store_directory(runtime_dir)?;
        let session_file = runtime_dir.join(SESSION_FILE);
        prepare_session_paths(&session_file)?;

        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = Arc::clone(&shutdown);
        let config = BrokerServerConfig::new(store_dir, session_file.clone());
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let server = thread::Builder::new()
            .name("gui-shell-security-broker".to_string())
            .spawn(move || run_loopback_server_cancellable(config, thread_shutdown, ready_tx))
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

        let (session_bytes, _endpoint) = match read_endpoint(&session_file) {
            Ok(value) => value,
            Err(error) => {
                shutdown.store(true, Ordering::Release);
                let _ = server.join();
                remove_failed_start_files(&session_file);
                return Err(error);
            }
        };
        Ok(Self {
            shutdown,
            server: Some(server),
            session_file,
            session_bytes,
        })
    }

    fn has_stopped(&self) -> bool {
        self.server.as_ref().is_none_or(JoinHandle::is_finished)
    }

    fn finish(&mut self) -> Result<(), DesktopLaunchError> {
        self.shutdown.store(true, Ordering::Release);
        let server_result = self.server.take().map(|server| match server.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) | Err(_) => Err(DesktopLaunchError::new(
                "BROKER_SHUTDOWN_FAILED",
                "安全Brokerを正常に終了できませんでした。",
            )),
        });
        let session_result = remove_session_if_unchanged(&self.session_file, &self.session_bytes);
        let temporary_result = remove_managed_file(&self.session_file.with_extension("json.tmp"));
        self.session_bytes.fill(0);
        session_result?;
        temporary_result?;
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
    runtime_dir: &Path,
    broker: &RunningBroker,
) -> Result<ExitStatus, DesktopLaunchError> {
    let session_file = runtime_dir.join(SESSION_FILE);
    let mut command = Command::new(&layout.app_exe);
    command
        .current_dir(&layout.app_dir)
        .env("GUI_SHELL_BROKER_ENDPOINT_JSON", &session_file)
        .env("GUI_SHELL_BROKER_RUNTIME_DIR", runtime_dir)
        .env_remove("GUI_SHELL_BROKER_SESSION_JSON")
        .env_remove("GUI_SHELL_SNAPSHOT_JSON")
        .env_remove("GUI_SHELL_SETUP_DOCTOR_EXPORT_JSON")
        .env_remove("GUI_SHELL_SETUP_DOCTOR_CONTEXT_JSON")
        .env_remove("GUI_SHELL_SURFACE_SEMANTICS_EXPORT_JSON");
    let mut frontend = command.spawn().map_err(|_| {
        DesktopLaunchError::new(
            "FRONTEND_START_FAILED",
            "D4 Pocket画面を起動できません。製品ファイルを確認してください。",
        )
    })?;
    wait_for_frontend(&mut frontend, broker)
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
    let runtime_dir = runtime_directory(&local_app_data)?;
    let _instance_lock = acquire_instance_lock(&runtime_dir)?;
    let mut broker = RunningBroker::start(&runtime_dir)?;

    let frontend_result = launch_frontend(&layout, &runtime_dir, &broker);
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
        getrandom::getrandom(&mut random).expect("random id");
        let path = std::env::temp_dir().join(format!(
            "gui-shell-launcher-{label}-{}",
            hex::encode(random)
        ));
        fs::create_dir_all(&path).expect("temporary directory");
        path
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
    fn runtime_state_is_separate_from_installation_and_lock_prevents_duplicate_launch() {
        let root = fs::canonicalize(test_root("runtime")).unwrap();
        let runtime = runtime_directory(&root).unwrap();
        assert!(runtime.starts_with(&root));
        assert!(runtime.ends_with(Path::new("GUI-Shell").join("broker").join("desktop")));
        let first = acquire_instance_lock(&runtime).unwrap();
        let second = acquire_instance_lock(&runtime).unwrap_err();
        assert_eq!(second.code, "INSTANCE_ALREADY_RUNNING");
        drop(first);
        assert!(acquire_instance_lock(&runtime).is_ok());
        fs::remove_dir_all(root).unwrap();
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
            .expect("create Windows junction fixture");
        assert!(result.status.success(), "junction fixture creation failed");

        let error = runtime_directory(&root).unwrap_err();
        assert_eq!(error.code, "USER_DATA_ROOT_INVALID");
        assert!(!outside.join("broker").exists());

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
            .expect("create Windows junction fixture");
        assert!(result.status.success(), "junction fixture creation failed");

        let error = ensure_store_directory(&root).unwrap_err();
        assert_eq!(error.code, "BROKER_STORE_INVALID");
        assert!(fs::read_dir(&outside).unwrap().next().is_none());

        fs::remove_dir(&junction).unwrap();
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
}
