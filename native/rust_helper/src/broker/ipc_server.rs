use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, SyncSender},
    Arc,
};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::broker::{Broker, BrokerResponse};

const DEFAULT_MAX_REQUEST_BYTES: usize = 64 * 1024;
const AUTH_LINE_MAX_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerServerConfig {
    pub store_dir: PathBuf,
    pub session_file: PathBuf,
    pub port: u16,
    pub max_request_bytes: usize,
    pub owner_session_file: Option<PathBuf>,
    pub minidora_runtimes: Vec<(String, String)>,
    /// ownerが明示した絶対executableとworkspaceだけをCodex Adapterへ登録する。
    pub codex_runtimes: Vec<(String, PathBuf, PathBuf)>,
    /// debug buildでしか受理しない、固定childを使うC4開発実証用flag。
    pub development_lifecycle_fixture_enabled: bool,
    pub mobile_bind: Option<String>,
    pub workspace_config: Option<PathBuf>,
    pub protected_store_dir: Option<PathBuf>,
    /// Rust Desktop起動器が固定runtime隣接pathを渡す製品内ProtectedStore。
    /// owner起動設定の任意ProtectedStoreとは別経路で、資格・権限を生成しない。
    pub desktop_protected_store_dir: Option<PathBuf>,
    /// Rust Desktop起動器が固定sibling構成のpackage layoutを検証済みの場合のみtrue。
    pub(crate) desktop_package_layout_verified: bool,
    /// Rust Desktop起動器が正式installed rootを独立に検証済みの場合のみtrue。
    pub(crate) desktop_install_path_verified: bool,
    /// Product Build時に埋込み、要求payloadから受け取らないApp／Audit Store identity。
    pub(crate) desktop_product_identity: Option<(String, String)>,
}

impl BrokerServerConfig {
    pub fn new(store_dir: PathBuf, session_file: PathBuf) -> Self {
        Self {
            store_dir,
            session_file,
            port: 0,
            max_request_bytes: DEFAULT_MAX_REQUEST_BYTES,
            owner_session_file: None,
            minidora_runtimes: Vec::new(),
            codex_runtimes: Vec::new(),
            development_lifecycle_fixture_enabled: false,
            mobile_bind: None,
            workspace_config: None,
            protected_store_dir: None,
            desktop_protected_store_dir: None,
            desktop_package_layout_verified: false,
            desktop_install_path_verified: false,
            desktop_product_identity: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerCredentialRole {
    Normal,
    Owner,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerEndpoint {
    pub host: String,
    pub port: u16,
    pub session_id: String,
    pub session_secret: String,
    pub credential_role: BrokerCredentialRole,
    pub transport: String,
    pub max_request_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerServerError {
    pub message: String,
}

impl BrokerServerError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Desktop起動器内だけで使う、Owner確認済みallowlist操作の一回限り要求。
pub(crate) struct DesktopOwnerOperationRequest {
    pub request_json: String,
    pub download_confirmation: Option<super::update_center::UpdateDownloadConfirmation>,
    pub apply_confirmation: Option<super::update_center::UpdateApplyConfirmation>,
    pub activation_confirmation: Option<super::update_center::UpdateActivationConfirmation>,
    pub product_repair_confirmation: Option<super::update_center::ProductRepairConfirmation>,
    pub product_uninstall_confirmation:
        Option<super::update_center::ProductUninstallConfirmation>,
    pub reply: SyncSender<BrokerResponse>,
}

pub fn run_loopback_server(config: BrokerServerConfig) -> Result<(), BrokerServerError> {
    run_loopback_server_inner(config, None, None, None, None)
}

/// Rust desktop launcherだけが使う、process内管理停止付きのBroker起動口。
/// 停止通知はIPCへ露出せず、Flutterのnormal資格やUI stateから生成できない。
pub fn run_loopback_server_cancellable(
    config: BrokerServerConfig,
    shutdown: Arc<AtomicBool>,
    ready: SyncSender<()>,
) -> Result<(), BrokerServerError> {
    run_loopback_server_inner(config, Some(shutdown), Some(ready), None, None)
}

/// Windows Desktop起動器専用のprocess内Owner確認経路。
/// receiverはnamed pipe／Flutterへ公開せず、処理はBroker所有threadで直列化する。
pub(crate) fn run_loopback_server_cancellable_with_owner_operations(
    config: BrokerServerConfig,
    shutdown: Arc<AtomicBool>,
    ready: SyncSender<()>,
    owner_operations: Receiver<DesktopOwnerOperationRequest>,
    desktop_export_root: Option<(PathBuf, cap_std::fs::Dir)>,
) -> Result<(), BrokerServerError> {
    run_loopback_server_inner(
        config,
        Some(shutdown),
        Some(ready),
        Some(owner_operations),
        desktop_export_root,
    )
}

fn run_loopback_server_inner(
    config: BrokerServerConfig,
    shutdown: Option<Arc<AtomicBool>>,
    ready: Option<SyncSender<()>>,
    owner_operations: Option<Receiver<DesktopOwnerOperationRequest>>,
    desktop_export_root: Option<(PathBuf, cap_std::fs::Dir)>,
) -> Result<(), BrokerServerError> {
    if shutdown_requested(&shutdown) {
        return Ok(());
    }
    let session_id = format!("broker-session-{}", random_hex(16)?);
    let session_secret = random_hex(32)?;
    let mut broker = Broker::new_persistent(&session_id, &config.store_dir)
        .map_err(|error| BrokerServerError::new(error.message()))?;
    if let Some((app_id, audit_store_id)) = config.desktop_product_identity.clone() {
        broker
            .set_desktop_product_identity(app_id, audit_store_id)
            .map_err(BrokerServerError::new)?;
    }
    let mut desktop_agent_workspace_protected_paths =
        vec![config.store_dir.clone(), config.session_file.clone()];
    desktop_agent_workspace_protected_paths
        .extend(config.store_dir.parent().map(std::path::Path::to_path_buf));
    desktop_agent_workspace_protected_paths.extend(
        config
            .session_file
            .parent()
            .map(std::path::Path::to_path_buf),
    );
    desktop_agent_workspace_protected_paths.extend(config.owner_session_file.iter().cloned());
    desktop_agent_workspace_protected_paths.extend(config.workspace_config.iter().cloned());
    desktop_agent_workspace_protected_paths.extend(config.protected_store_dir.iter().cloned());
    desktop_agent_workspace_protected_paths
        .extend(config.desktop_protected_store_dir.iter().cloned());
    broker.set_desktop_agent_workspace_protected_paths(desktop_agent_workspace_protected_paths);
    if let Some((path, root)) = desktop_export_root {
        broker.set_desktop_export_root(path, root);
    }
    if shutdown_requested(&shutdown) {
        return Ok(());
    }

    if shutdown.is_some() {
        let reason = if config.desktop_install_path_verified {
            "Capability=desktop.launch product.runtime.launch Permission=Rust起動器が有効版recordと導入先launcherを照合した現在版のみ Approval=Start Menuからの通常起動、portableからの初回Install後は別途native Owner確認済みの有効版切替 RecoveryAction=画面起動失敗時は有効版recordを保持しStart Menuから再試行。Brokerは実行状態をLIVE_RUNTIMEで記録"
        } else {
            "Capability=desktop.launch Permission=固定Desktop起動器のlifecycleのみ Approval=通常画面起動のためOwner承認不要・privileged actionは非承認 RecoveryAction=失敗時は起動器管理Brokerを停止し未変更endpointだけを整理してstoreを保持"
        };
        let payload_hash = crate::audit_hash::sha256_tagged(b"gui-shell-desktop-launcher:start:v1");
        broker
            .append_audit(
                &format!("desktop-launcher:{}:start", session_id),
                "D4 Pocket Desktop起動",
                "recorded",
                reason,
                "LIVE_RUNTIME",
                &payload_hash,
            )
            .map_err(|error| BrokerServerError::new(error.message()))?;
    }
    if shutdown.is_some() && config.desktop_package_layout_verified {
        broker.set_desktop_setup_doctor_runtime_evidence(
            config.desktop_package_layout_verified,
            config.desktop_install_path_verified,
            false,
        );
        broker
            .initialize_desktop_first_run_configuration()
            .map_err(|_| {
                BrokerServerError::new("初回UI設定を安全に生成・検証できないため起動を停止しました")
            })?;
    }

    let workspace_startup = if let Some(path) = &config.workspace_config {
        broker
            .作業領域設定監査(path, "received", "owner起動設定の読取を受信")
            .map_err(BrokerServerError::new)?;
        let parsed = (|| {
            let owner_file = config
                .owner_session_file
                .as_ref()
                .ok_or("作業領域登録にはowner資格設定が必要")?;
            Ok((super::workspace_root::read_config(path)?, owner_file))
        })();
        let (settings, owner_file) = match parsed {
            Ok(value) => value,
            Err(reason) => {
                broker
                    .作業領域設定監査(path, "rejected", reason)
                    .map_err(BrokerServerError::new)?;
                return Err(BrokerServerError::new(reason));
            }
        };
        let mut protected = vec![
            config.store_dir.clone(),
            config.session_file.clone(),
            owner_file.clone(),
            path.clone(),
        ];
        protected.extend(config.protected_store_dir.iter().cloned());
        protected.extend(config.desktop_protected_store_dir.iter().cloned());
        if let Err(reason) =
            verify_codex_workspace_roots(&config.codex_runtimes, &settings, &protected)
        {
            broker
                .作業領域設定監査(path, "rejected", reason)
                .map_err(BrokerServerError::new)?;
            return Err(BrokerServerError::new(reason));
        }
        Some((settings, protected))
    } else {
        None
    };

    for (id, address) in &config.minidora_runtimes {
        let adapter = crate::adapters::minidora::MinidoraAdapter::new(address)
            .map_err(|_| BrokerServerError::new("実行系接続先が不正"))?;
        broker
            .実行系登録(id, std::sync::Arc::new(adapter))
            .map_err(|_| BrokerServerError::new("実行系登録が不正または重複"))?;
    }
    for (id, executable, workspace) in &config.codex_runtimes {
        let adapter = crate::adapters::codex_cli::CodexCliAdapter::new(executable, workspace)
            .map_err(BrokerServerError::new)?;
        broker
            .実行系登録(id, std::sync::Arc::new(adapter))
            .map_err(|_| BrokerServerError::new("Codex実行系登録が不正または重複"))?;
    }
    if config.development_lifecycle_fixture_enabled {
        if config.owner_session_file.is_none() {
            return Err(BrokerServerError::new(
                "開発用lifecycle fixtureにはowner資格設定が必要",
            ));
        }
        #[cfg(debug_assertions)]
        broker
            .開発用ライフサイクル実行系登録()
            .map_err(BrokerServerError::new)?;
        #[cfg(not(debug_assertions))]
        return Err(BrokerServerError::new(
            "release buildは開発用lifecycle fixtureを受け付けない",
        ));
    }
    let owner_secret = if config.owner_session_file.is_some() {
        Some(random_hex(32)?)
    } else {
        None
    };

    if config.desktop_protected_store_dir.is_some()
        && (config.protected_store_dir.is_some()
            || config.owner_session_file.is_some()
            || config.workspace_config.is_some())
    {
        return Err(BrokerServerError::new(
            "Desktop固定ProtectedStoreはOwner起動設定と併用できない",
        ));
    }

    if let Some(path) = &config.protected_store_dir {
        let mut protected = vec![config.store_dir.clone(), config.session_file.clone()];
        protected.extend(config.owner_session_file.iter().cloned());
        protected.extend(config.workspace_config.iter().cloned());
        broker
            .保管先起動登録(path, config.owner_session_file.is_some(), &protected)
            .map_err(BrokerServerError::new)?;
    }

    if let Some(path) = &config.desktop_protected_store_dir {
        let expected = config
            .store_dir
            .parent()
            .map(|runtime_dir| runtime_dir.join("protected"));
        if expected.as_deref() != Some(path.as_path()) {
            return Err(BrokerServerError::new(
                "Desktop ProtectedStore pathはruntime隣接の固定領域に限る",
            ));
        }
        let mut protected = vec![config.store_dir.clone(), config.session_file.clone()];
        protected.extend(config.owner_session_file.iter().cloned());
        protected.extend(config.workspace_config.iter().cloned());
        broker
            .desktop_protected_store_startup(path, &protected)
            .map_err(BrokerServerError::new)?;
    }

    if let Some((settings, protected)) = workspace_startup {
        for workspace in &settings.workspaces {
            broker
                .作業領域起動登録(workspace, &protected)
                .map_err(BrokerServerError::new)?;
        }
    }

    let mobile = if let Some(address) = &config.mobile_bind {
        if owner_secret.is_none() {
            return Err(BrokerServerError::new("端末連携はowner資格設定が必要"));
        }
        Some(
            super::device_transport::DeviceListener::bind(address, &mut broker)
                .map_err(BrokerServerError::new)?,
        )
    } else {
        None
    };
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), config.port);
    let listener = TcpListener::bind(bind_addr)
        .map_err(|error| BrokerServerError::new(format!("broker IPCのbindに失敗: {error}")))?;
    let local_addr = listener.local_addr().map_err(|error| {
        BrokerServerError::new(format!("broker IPC addrの読取りに失敗: {error}"))
    })?;
    if local_addr.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) {
        return Err(BrokerServerError::new(
            "broker IPCはloopback以外のbind address露出を拒否した",
        ));
    }
    broker.set_desktop_setup_doctor_runtime_evidence(
        config.desktop_package_layout_verified,
        config.desktop_install_path_verified,
        local_addr.ip() == IpAddr::V4(Ipv4Addr::LOCALHOST),
    );

    let endpoint = BrokerEndpoint {
        host: "127.0.0.1".to_string(),
        port: local_addr.port(),
        session_id,
        session_secret,
        credential_role: BrokerCredentialRole::Normal,
        transport: "authenticated_loopback_tcp".to_string(),
        max_request_bytes: config.max_request_bytes,
    };
    if shutdown_requested(&shutdown) {
        return Ok(());
    }
    if let Some(path) = &config.owner_session_file {
        let absolute = |p: &std::path::Path| -> Result<String, BrokerServerError> {
            let parent = p
                .parent()
                .filter(|v| !v.as_os_str().is_empty())
                .unwrap_or(std::path::Path::new("."));
            let parent = std::fs::canonicalize(parent)
                .map_err(|_| BrokerServerError::new("資格fileの親directoryを確認できない"))?;
            Ok(parent
                .join(
                    p.file_name()
                        .ok_or_else(|| BrokerServerError::new("資格file名が不正"))?,
                )
                .to_string_lossy()
                .to_lowercase())
        };
        if absolute(path)? == absolute(&config.session_file)?
            || path.is_symlink()
            || config.session_file.is_symlink()
        {
            return Err(BrokerServerError::new(
                "owner資格と通常資格は異なる通常fileを指定する",
            ));
        }
        let mut control = endpoint.clone();
        control.session_secret = owner_secret
            .clone()
            .ok_or_else(|| BrokerServerError::new("owner資格がない"))?;
        control.credential_role = BrokerCredentialRole::Owner;
        write_endpoint_file(path, &control)?;
    }
    write_endpoint_file(&config.session_file, &endpoint)?;

    listener
        .set_nonblocking(true)
        .map_err(|_| BrokerServerError::new("listener設定失敗"))?;
    if shutdown_requested(&shutdown) {
        return Ok(());
    }
    if let Some(ready) = ready {
        let _ = ready.send(());
    }
    loop {
        if shutdown_requested(&shutdown) {
            break;
        }
        broker.端末期限処理();
        if let Some(owner_operations) = &owner_operations {
            if let Ok(request) = owner_operations.try_recv() {
                let response = broker.desktop_owner_operation_json_with_all_confirmations(
                    &request.request_json,
                    request.download_confirmation,
                    request.apply_confirmation,
                    request.activation_confirmation,
                    request.product_uninstall_confirmation,
                    request.product_repair_confirmation,
                );
                let _ = request.reply.send(response);
            }
        }
        broker.update_download_tick();
        if let Ok((stream, _)) = listener.accept() {
            if handle_stream(
                stream,
                &endpoint.session_secret,
                owner_secret.as_deref(),
                &mut broker,
                &config,
            )
            .unwrap_or(false)
            {
                break;
            }
        }
        if let Some(mobile) = &mobile {
            mobile.poll(&mut broker);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    if shutdown_requested(&shutdown) {
        let reason = "Capability=desktop.launch Permission=起動器が所有するDesktop process lifecycleのみ Approval=UI終了後の内部停止通知でありprivileged actionは非承認 AuditEvent=Broker終了 RecoveryAction=変更endpointは削除せずdurable storeを保持";
        let payload_hash = crate::audit_hash::sha256_tagged(b"gui-shell-desktop-launcher:stop:v1");
        broker
            .append_audit(
                &format!("desktop-launcher:{}:stop", endpoint.session_id),
                "D4 Pocket Desktop終了",
                "recorded",
                reason,
                "LIVE_RUNTIME",
                &payload_hash,
            )
            .map_err(|error| BrokerServerError::new(error.message()))?;
    }
    Ok(())
}

fn verify_codex_workspace_roots(
    codex_runtimes: &[(String, PathBuf, PathBuf)],
    settings: &super::workspace_root::StartupConfig,
    protected: &[PathBuf],
) -> Result<(), &'static str> {
    for (runtime_id, _, codex_workspace) in codex_runtimes {
        for workspace in settings
            .workspaces
            .iter()
            .filter(|workspace| workspace.runtime_id == *runtime_id)
        {
            super::workspace_root::verify_same_physical_root(
                codex_workspace,
                std::path::Path::new(&workspace.root_path),
                protected,
            )?;
        }
    }
    Ok(())
}

fn shutdown_requested(shutdown: &Option<Arc<AtomicBool>>) -> bool {
    shutdown
        .as_ref()
        .is_some_and(|signal| signal.load(Ordering::Acquire))
}

fn handle_stream(
    stream: TcpStream,
    session_secret: &str,
    owner_secret: Option<&str>,
    broker: &mut Broker,
    config: &BrokerServerConfig,
) -> Result<bool, BrokerServerError> {
    stream
        .set_nonblocking(false)
        .map_err(|_| BrokerServerError::new("IPC blocking設定失敗"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| {
            BrokerServerError::new(format!("IPC read timeoutの設定に失敗: {error}"))
        })?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| {
            BrokerServerError::new(format!("IPC write timeoutの設定に失敗: {error}"))
        })?;

    let mut reader = BufReader::new(stream);

    let auth_line = match read_limited_line(&mut reader, AUTH_LINE_MAX_BYTES) {
        Ok(Some(line)) => line,
        Ok(None) => {
            // 相手が資格情報を送る前に接続を閉じた。拒否応答を届ける
            // 有効な通信路がないため、応答を返さず終了する。
            return Ok(false);
        }
        Err(IpcLineError::Oversized) => {
            let response = broker.reject_ipc(
                "broker_request_oversized",
                "broker IPC auth line exceeds the configured request limit",
                true,
            );
            write_response(reader.get_mut(), &response)?;
            drain_after_response(&mut reader)?;
            return Ok(false);
        }
        Err(IpcLineError::Io(message)) => return Err(BrokerServerError::new(message)),
    };

    let owner = owner_secret.is_some_and(|secret| auth_line == secret);
    if auth_line != session_secret && !owner {
        let response = broker.reject_ipc(
            "broker_authentication_failed",
            "broker IPC authentication failed",
            true,
        );
        write_response(reader.get_mut(), &response)?;
        drain_after_response(&mut reader)?;
        return Ok(false);
    }

    let request_json = match read_limited_line(&mut reader, config.max_request_bytes) {
        Ok(Some(line)) => line,
        Ok(None) => {
            let response = broker.reject_ipc(
                "broker_ipc_malformed",
                "broker IPC request envelope is missing",
                true,
            );
            write_response(reader.get_mut(), &response)?;
            drain_after_response(&mut reader)?;
            return Ok(false);
        }
        Err(IpcLineError::Oversized) => {
            let response = broker.reject_ipc(
                "broker_request_oversized",
                "broker IPC request exceeds the configured request limit",
                true,
            );
            write_response(reader.get_mut(), &response)?;
            drain_after_response(&mut reader)?;
            return Ok(false);
        }
        Err(IpcLineError::Io(message)) => return Err(BrokerServerError::new(message)),
    };

    let response = if owner {
        broker.owner要求処理(&request_json)
    } else {
        broker.handle_json(&request_json)
    };
    let shutdown = response.shutdown_requested;
    write_response(reader.get_mut(), &response)?;
    response_drain_result(shutdown, drain_after_response(&mut reader))
}

fn response_drain_result(
    shutdown: bool,
    drain_result: Result<(), BrokerServerError>,
) -> Result<bool, BrokerServerError> {
    if shutdown {
        // 認証・監査済みshutdownは、応答後のpeer切断状態にかかわらず終了させる。
        // close/RST処理の失敗を通常継続へ変換すると、accepted応答後もlistenし続ける。
        return Ok(true);
    }
    drain_result.map(|()| false)
}

fn drain_after_response(reader: &mut BufReader<TcpStream>) -> Result<(), BrokerServerError> {
    // 応答を読んだpeerのcloseを短時間待ち、Windowsの未読受信data付きcloseによるRSTを避ける。
    reader
        .get_mut()
        .set_read_timeout(Some(Duration::from_millis(100)))
        .map_err(|error| BrokerServerError::new(format!("IPC切断待機の設定に失敗: {error}")))?;
    let mut buffer = [0u8; 1024];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                return Ok(())
            }
            Err(error) => {
                return Err(BrokerServerError::new(format!(
                    "IPC切断待機中の読取りに失敗: {error}"
                )))
            }
        }
    }
}

fn read_limited_line(
    reader: &mut BufReader<TcpStream>,
    max_bytes: usize,
) -> Result<Option<String>, IpcLineError> {
    let mut buffer = Vec::new();
    loop {
        let available = reader
            .fill_buf()
            .map_err(|error| IpcLineError::Io(format!("broker IPC lineの読取りに失敗: {error}")))?;
        if available.is_empty() {
            if buffer.is_empty() {
                return Ok(None);
            }
            break;
        }

        let take = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|position| position + 1)
            .unwrap_or(available.len());
        if buffer.len() + take > max_bytes {
            reader.consume(take);
            return Err(IpcLineError::Oversized);
        }
        buffer.extend_from_slice(&available[..take]);
        reader.consume(take);
        if buffer.last() == Some(&b'\n') {
            break;
        }
    }
    while matches!(buffer.last(), Some(b'\n' | b'\r')) {
        buffer.pop();
    }
    String::from_utf8(buffer)
        .map(Some)
        .map_err(|error| IpcLineError::Io(format!("broker IPC lineがUTF-8ではない: {error}")))
}

fn write_response(
    stream: &mut TcpStream,
    response: &BrokerResponse,
) -> Result<(), BrokerServerError> {
    let encoded = response.to_json_string().map_err(|error| {
        BrokerServerError::new(format!("broker responseのencodeに失敗: {error}"))
    })?;
    let mut frame = encoded.into_bytes();
    frame.push(b'\n');
    stream.write_all(&frame).map_err(|error| {
        BrokerServerError::new(format!("broker responseの書込みに失敗: {error}"))
    })?;
    stream.flush().map_err(|error| {
        BrokerServerError::new(format!("broker responseのflushに失敗: {error}"))
    })?;
    Ok(())
}

fn write_endpoint_file(path: &PathBuf, endpoint: &BrokerEndpoint) -> Result<(), BrokerServerError> {
    let encoded = serde_json::to_string(endpoint).map_err(|error| {
        BrokerServerError::new(format!("broker endpointのencodeに失敗: {error}"))
    })?;
    let temporary_path = path.with_extension("json.tmp");
    {
        let mut options = OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary_path).map_err(|error| {
            BrokerServerError::new(format!("broker endpoint fileの作成に失敗: {error}"))
        })?;
        file.write_all(encoded.as_bytes()).map_err(|error| {
            BrokerServerError::new(format!("broker endpoint fileの書込みに失敗: {error}"))
        })?;
        file.sync_data().map_err(|error| {
            BrokerServerError::new(format!("broker endpoint fileのsyncに失敗: {error}"))
        })?;
    }
    std::fs::rename(&temporary_path, path).map_err(|error| {
        BrokerServerError::new(format!("broker endpoint fileのcommitに失敗: {error}"))
    })
}

fn random_hex(byte_count: usize) -> Result<String, BrokerServerError> {
    let mut bytes = vec![0u8; byte_count];
    getrandom::getrandom(&mut bytes)
        .map_err(|error| BrokerServerError::new(format!("broker secretの生成に失敗: {error}")))?;
    Ok(hex::encode(bytes))
}

enum IpcLineError {
    Io(String),
    Oversized,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 監査済みshutdownは応答後の切断errorにかかわらず終了する() {
        let error = BrokerServerError::new("試験用の切断error");
        assert!(response_drain_result(true, Err(error)).unwrap());
        assert!(!response_drain_result(false, Ok(())).unwrap());
        assert!(
            response_drain_result(false, Err(BrokerServerError::new("試験用の切断error"))).is_err()
        );
    }

    fn temporary_directory() -> PathBuf {
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).expect("乱数識別子");
        let path =
            std::env::temp_dir().join(format!("gui-shell-launcher-broker-{}", hex::encode(random)));
        std::fs::create_dir_all(&path).expect("一時作業ディレクトリ");
        path
    }

    fn read_audit(path: &std::path::Path) -> String {
        let mut file = std::fs::File::open(path).expect("監査記録を開く");
        let mut text = String::new();
        file.read_to_string(&mut text).expect("監査記録を読む");
        text
    }

    fn workspace_settings(
        runtime_id: &str,
        roots: &[&std::path::Path],
    ) -> super::super::workspace_root::StartupConfig {
        super::super::workspace_root::StartupConfig {
            version: 1,
            workspaces: roots
                .iter()
                .enumerate()
                .map(
                    |(index, root)| super::super::workspace_root::WorkspaceStartup {
                        runtime_id: runtime_id.to_string(),
                        workspace_id: format!("workspace-{index}"),
                        root_path: root.to_string_lossy().into_owned(),
                        secret_paths: Vec::new(),
                    },
                )
                .collect(),
        }
    }

    #[test]
    fn codex_workspace_config_requires_matching_physical_root_for_same_runtime() {
        let root = temporary_directory();
        let codex_root = root.join("codex-workspace");
        let different_root = root.join("different-workspace");
        std::fs::create_dir(&codex_root).expect("Codexの作業領域");
        std::fs::create_dir(&different_root).expect("別Workspace");
        let runtime = "codex-agent".to_string();
        let codex_runtimes = vec![(runtime.clone(), root.join("codex.exe"), codex_root.clone())];

        let matching = workspace_settings(&runtime, &[&codex_root]);
        assert!(verify_codex_workspace_roots(&codex_runtimes, &matching, &[]).is_ok());

        let mismatching = workspace_settings(&runtime, &[&different_root]);
        assert_eq!(
            verify_codex_workspace_roots(&codex_runtimes, &mismatching, &[]),
            Err("Codex実行系の作業pathとWorkspace rootの物理directoryが一致しない"),
        );
        let multiple = workspace_settings(&runtime, &[&codex_root, &different_root]);
        assert_eq!(
            verify_codex_workspace_roots(&codex_runtimes, &multiple, &[]),
            Err("Codex実行系の作業pathとWorkspace rootの物理directoryが一致しない"),
        );

        let mut unbound = workspace_settings("another-runtime", &[&different_root]);
        assert!(verify_codex_workspace_roots(&codex_runtimes, &unbound, &[]).is_ok());
        unbound
            .workspaces
            .push(super::super::workspace_root::WorkspaceStartup {
                runtime_id: runtime,
                workspace_id: "workspace-extra".into(),
                root_path: codex_root.to_string_lossy().into_owned(),
                secret_paths: Vec::new(),
            });
        assert!(verify_codex_workspace_roots(&codex_runtimes, &unbound, &[]).is_ok());

        std::fs::remove_dir_all(root).expect("試験専用directory");
    }

    #[test]
    fn mismatched_codex_workspace_is_audited_before_cli_probe_or_endpoint_creation() {
        let root = temporary_directory();
        let codex_root = root.join("codex-workspace");
        let registered_root = root.join("registered-workspace");
        std::fs::create_dir(&codex_root).expect("Codexの作業領域");
        std::fs::create_dir(&registered_root).expect("登録Workspace");
        let workspace_config = root.join("workspace.json");
        let config_text = serde_json::json!({
            "version": 1,
            "workspaces": [{
                "runtime_id": "codex-agent",
                "workspace_id": "codex-workspace",
                "root_path": registered_root,
                "secret_paths": []
            }]
        })
        .to_string();
        std::fs::File::create(&workspace_config)
            .expect("Workspace起動設定file")
            .write_all(config_text.as_bytes())
            .expect("Workspace起動設定を書き込む");

        let store_dir = root.join("store");
        let session_file = root.join("normal.json");
        let mut config = BrokerServerConfig::new(store_dir.clone(), session_file.clone());
        config.owner_session_file = Some(root.join("owner.json"));
        config.workspace_config = Some(workspace_config);
        config.codex_runtimes.push((
            "codex-agent".into(),
            root.join("not-installed-codex.exe"),
            codex_root,
        ));

        let error = run_loopback_server(config).expect_err("root不一致を先に拒否");
        assert_eq!(
            error.message,
            "Codex実行系の作業pathとWorkspace rootの物理directoryが一致しない"
        );
        assert!(!session_file.exists(), "拒否時にIPC endpointを作らない");
        let audit = read_audit(&store_dir.join("audit.jsonl"));
        assert!(
            audit.contains("物理directoryが一致しない"),
            "拒否理由を監査へ記録する"
        );
        assert!(audit.contains("rejected"), "設定拒否の判定を監査へ記録する");
        assert!(
            !audit.contains("not-installed-codex.exe"),
            "executable pathを監査へ複写しない"
        );

        std::fs::remove_dir_all(root).expect("試験専用directory");
    }

    #[test]
    fn process_internal_cancellation_is_audited_and_preserves_durable_state() {
        let root = temporary_directory();
        let session_file = root.join("broker_session.json");
        let store_dir = root.join("store");
        let shutdown = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        let thread_shutdown = Arc::clone(&shutdown);
        let thread_session_file = session_file.clone();
        let thread_store_dir = store_dir.clone();
        let server = std::thread::spawn(move || {
            run_loopback_server_cancellable(
                BrokerServerConfig::new(thread_store_dir, thread_session_file),
                thread_shutdown,
                ready_tx,
            )
        });

        ready_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("Broker待受け準備完了");
        assert!(session_file.is_file());
        assert!(store_dir.is_dir());
        let startup: serde_json::Value = serde_json::from_str(
            read_audit(&store_dir.join("audit.jsonl"))
                .lines()
                .next()
                .expect("起動監査event"),
        )
        .expect("起動監査JSON");
        assert_eq!(startup["operation"], "D4 Pocket Desktop起動");
        assert!(startup["reason"]
            .as_str()
            .unwrap()
            .contains("Capability=desktop.launch"));
        assert_eq!(startup["evidence_source"], "LIVE_RUNTIME");

        shutdown.store(true, Ordering::Release);
        server
            .join()
            .expect("Broker threadの終了待ち")
            .expect("Broker終了");

        assert!(session_file.is_file(), "session fileの後処理は起動器が担う");
        assert!(
            store_dir.is_dir(),
            "durable user state must survive app exit"
        );
        let audit_text = read_audit(&store_dir.join("audit.jsonl"));
        let events: Vec<serde_json::Value> = audit_text
            .lines()
            .map(|line| serde_json::from_str(line).expect("監査event JSONの形式"))
            .collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1]["operation"], "D4 Pocket Desktop終了");
        assert!(events[1]["reason"]
            .as_str()
            .unwrap()
            .contains("RecoveryAction="));
        std::fs::remove_dir_all(root).expect("試験専用の一時ディレクトリだけを削除");
    }

    #[test]
    fn desktop_protected_store_rejects_non_fixed_path_before_endpoint_creation() {
        let root = temporary_directory();
        let store_dir = root.join("store");
        let session_file = root.join("broker_session.json");
        let mut config = BrokerServerConfig::new(store_dir.clone(), session_file.clone());
        config.desktop_protected_store_dir = Some(root.join("other-protected"));

        let error = run_loopback_server(config).unwrap_err();
        assert!(error.message.contains("固定領域に限る"));
        assert!(!session_file.exists(), "検査拒否時に接続endpointを作らない");
        assert!(store_dir.is_dir(), "既存Broker stateを削除しない");
        std::fs::remove_dir_all(root).expect("試験専用の一時ディレクトリだけを削除");
    }
}
