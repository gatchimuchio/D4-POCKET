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

/// Desktop起動器内だけで使う、Owner確認済み書出しの一回限り要求。
pub(crate) struct DesktopOwnerExportRequest {
    pub request_json: String,
    pub reply: SyncSender<BrokerResponse>,
}

pub fn run_loopback_server(config: BrokerServerConfig) -> Result<(), BrokerServerError> {
    run_loopback_server_inner(config, None, None, None)
}

/// Rust desktop launcherだけが使う、process内管理停止付きのBroker起動口。
/// 停止通知はIPCへ露出せず、Flutterのnormal資格やUI stateから生成できない。
pub fn run_loopback_server_cancellable(
    config: BrokerServerConfig,
    shutdown: Arc<AtomicBool>,
    ready: SyncSender<()>,
) -> Result<(), BrokerServerError> {
    run_loopback_server_inner(config, Some(shutdown), Some(ready), None)
}

/// Windows Desktop起動器専用のprocess内Owner確認経路。
/// receiverはnamed pipe／Flutterへ公開せず、処理はBroker所有threadで直列化する。
pub(crate) fn run_loopback_server_cancellable_with_owner_exports(
    config: BrokerServerConfig,
    shutdown: Arc<AtomicBool>,
    ready: SyncSender<()>,
    owner_exports: Receiver<DesktopOwnerExportRequest>,
) -> Result<(), BrokerServerError> {
    run_loopback_server_inner(config, Some(shutdown), Some(ready), Some(owner_exports))
}

fn run_loopback_server_inner(
    config: BrokerServerConfig,
    shutdown: Option<Arc<AtomicBool>>,
    ready: Option<SyncSender<()>>,
    owner_exports: Option<Receiver<DesktopOwnerExportRequest>>,
) -> Result<(), BrokerServerError> {
    if shutdown_requested(&shutdown) { return Ok(()); }
    let session_id = format!("broker-session-{}", random_hex(16)?);
    let session_secret = random_hex(32)?;
    let mut broker = Broker::new_persistent(&session_id, &config.store_dir)
        .map_err(|error| BrokerServerError::new(error.message()))?;
    if shutdown_requested(&shutdown) { return Ok(()); }

    if shutdown.is_some() {
        let reason = "Capability=desktop.launch Permission=固定Desktop起動器のlifecycleのみ Approval=通常画面起動のためOwner承認不要・privileged actionは非承認 RecoveryAction=失敗時は起動器管理Brokerを停止し未変更endpointだけを整理してstoreを保持";
        let payload_hash = crate::audit_hash::sha256_tagged(b"gui-shell-desktop-launcher:start:v1");
        broker.append_audit(
            &format!("desktop-launcher:{}:start", session_id),
            "D4 Pocket Desktop起動",
            "recorded",
            reason,
            "LIVE_RUNTIME",
            &payload_hash,
        ).map_err(|error| BrokerServerError::new(error.message()))?;
    }

    for (id, address) in &config.minidora_runtimes {
        let adapter = crate::adapters::minidora::MinidoraAdapter::new(address).map_err(|_| BrokerServerError::new("実行系接続先が不正"))?;
        broker.実行系登録(id, std::sync::Arc::new(adapter)).map_err(|_| BrokerServerError::new("実行系登録が不正または重複"))?;
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
    let owner_secret = if config.owner_session_file.is_some() { Some(random_hex(32)?) } else { None };

    if let Some(path) = &config.protected_store_dir {
        let mut protected = vec![config.store_dir.clone(), config.session_file.clone()];
        protected.extend(config.owner_session_file.iter().cloned());
        protected.extend(config.workspace_config.iter().cloned());
        broker.保管先起動登録(path, config.owner_session_file.is_some(), &protected).map_err(BrokerServerError::new)?;
    }

    if let Some(path)=&config.workspace_config {
        broker.作業領域設定監査(path,"received","owner起動設定の読取を受信").map_err(BrokerServerError::new)?;
        let parsed=(|| {
            let owner_file=config.owner_session_file.as_ref().ok_or("作業領域登録にはowner資格設定が必要")?;
            Ok((super::workspace_root::read_config(path)?,owner_file))
        })();
        let (settings,owner_file)=match parsed {
            Ok(value)=>value,
            Err(reason)=>{
                broker.作業領域設定監査(path,"rejected",reason).map_err(BrokerServerError::new)?;
                return Err(BrokerServerError::new(reason));
            }
        };
        let mut protected=vec![config.store_dir.clone(),config.session_file.clone(),owner_file.clone(),path.clone()];
        protected.extend(config.protected_store_dir.iter().cloned());
        for workspace in &settings.workspaces {
            broker.作業領域起動登録(workspace,&protected).map_err(BrokerServerError::new)?;
        }
    }

    let mobile = if let Some(address) = &config.mobile_bind {
        if owner_secret.is_none() {return Err(BrokerServerError::new("端末連携はowner資格設定が必要"));}
        Some(super::device_transport::DeviceListener::bind(address,&mut broker).map_err(BrokerServerError::new)?)
    } else {None};
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

    let endpoint = BrokerEndpoint {
        host: "127.0.0.1".to_string(),
        port: local_addr.port(),
        session_id,
        session_secret,
        credential_role: BrokerCredentialRole::Normal,
        transport: "authenticated_loopback_tcp".to_string(),
        max_request_bytes: config.max_request_bytes,
    };
    if shutdown_requested(&shutdown) { return Ok(()); }
    if let Some(path) = &config.owner_session_file {
        let absolute = |p: &std::path::Path| -> Result<String, BrokerServerError> {
            let parent = p.parent().filter(|v| !v.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));
            let parent = std::fs::canonicalize(parent).map_err(|_| BrokerServerError::new("資格fileの親directoryを確認できない"))?;
            Ok(parent.join(p.file_name().ok_or_else(|| BrokerServerError::new("資格file名が不正"))?).to_string_lossy().to_lowercase())
        };
        if absolute(path)? == absolute(&config.session_file)? || path.is_symlink() || config.session_file.is_symlink() {
            return Err(BrokerServerError::new("owner資格と通常資格は異なる通常fileを指定する"));
        }
        let mut control = endpoint.clone();
        control.session_secret = owner_secret.clone().ok_or_else(|| BrokerServerError::new("owner資格がない"))?;
        control.credential_role = BrokerCredentialRole::Owner;
        write_endpoint_file(path, &control)?;
    }
    write_endpoint_file(&config.session_file, &endpoint)?;

    listener.set_nonblocking(true).map_err(|_|BrokerServerError::new("listener設定失敗"))?;
    if shutdown_requested(&shutdown) { return Ok(()); }
    if let Some(ready) = ready { let _ = ready.send(()); }
    loop {
        if shutdown_requested(&shutdown) { break; }
        broker.端末期限処理();
        if let Some(owner_exports) = &owner_exports {
            if let Ok(request) = owner_exports.try_recv() {
                let response = broker.desktop_owner_export_json(&request.request_json);
                let _ = request.reply.send(response);
            }
        }
        if let Ok((stream,_)) = listener.accept() {
            if handle_stream(stream, &endpoint.session_secret, owner_secret.as_deref(), &mut broker, &config).unwrap_or(false) {break;}
        }
        if let Some(mobile) = &mobile {mobile.poll(&mut broker);}
        std::thread::sleep(Duration::from_millis(10));
    }
    if shutdown_requested(&shutdown) {
        let reason = "Capability=desktop.launch Permission=起動器が所有するDesktop process lifecycleのみ Approval=UI終了後の内部停止通知でありprivileged actionは非承認 AuditEvent=Broker終了 RecoveryAction=変更endpointは削除せずdurable storeを保持";
        let payload_hash = crate::audit_hash::sha256_tagged(b"gui-shell-desktop-launcher:stop:v1");
        broker.append_audit(
            &format!("desktop-launcher:{}:stop", endpoint.session_id),
            "D4 Pocket Desktop終了",
            "recorded",
            reason,
            "LIVE_RUNTIME",
            &payload_hash,
        ).map_err(|error| BrokerServerError::new(error.message()))?;
    }
    Ok(())
}

fn shutdown_requested(shutdown: &Option<Arc<AtomicBool>>) -> bool {
    shutdown.as_ref().is_some_and(|signal| signal.load(Ordering::Acquire))
}

fn handle_stream(
    stream: TcpStream,
    session_secret: &str,
    owner_secret: Option<&str>,
    broker: &mut Broker,
    config: &BrokerServerConfig,
) -> Result<bool, BrokerServerError> {
    stream.set_nonblocking(false).map_err(|_|BrokerServerError::new("IPC blocking設定失敗"))?;
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

    let response = if owner { broker.owner要求処理(&request_json) } else { broker.handle_json(&request_json) };
    let shutdown = response.shutdown_requested;
    write_response(reader.get_mut(), &response)?;
    drain_after_response(&mut reader)?;
    Ok(shutdown)
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
                ) => return Ok(()),
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
        let available = reader.fill_buf().map_err(|error| {
            IpcLineError::Io(format!("broker IPC lineの読取りに失敗: {error}"))
        })?;
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
    stream
        .write_all(&frame)
        .map_err(|error| {
            BrokerServerError::new(format!("broker responseの書込みに失敗: {error}"))
        })?;
    stream
        .flush()
        .map_err(|error| BrokerServerError::new(format!("broker responseのflushに失敗: {error}")))?;
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
    getrandom::getrandom(&mut bytes).map_err(|error| {
        BrokerServerError::new(format!("broker secretの生成に失敗: {error}"))
    })?;
    Ok(hex::encode(bytes))
}

enum IpcLineError {
    Io(String),
    Oversized,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_directory() -> PathBuf {
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).expect("乱数識別子");
        let path = std::env::temp_dir().join(format!(
            "gui-shell-launcher-broker-{}",
            hex::encode(random)
        ));
        std::fs::create_dir_all(&path).expect("一時作業ディレクトリ");
        path
    }

    fn read_audit(path: &std::path::Path) -> String {
        let mut file = std::fs::File::open(path).expect("監査記録を開く");
        let mut text = String::new();
        file.read_to_string(&mut text).expect("監査記録を読む");
        text
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
        assert!(store_dir.is_dir(), "durable user state must survive app exit");
        let audit_text = read_audit(&store_dir.join("audit.jsonl"));
        let events: Vec<serde_json::Value> = audit_text
            .lines()
            .map(|line| serde_json::from_str(line).expect("監査event JSONの形式"))
            .collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1]["operation"], "D4 Pocket Desktop終了");
        assert!(events[1]["reason"].as_str().unwrap().contains("RecoveryAction="));
        std::fs::remove_dir_all(root).expect("試験専用の一時ディレクトリだけを削除");
    }
}
