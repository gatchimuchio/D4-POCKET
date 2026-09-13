use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
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
    pub mobile_bind: Option<String>,
    pub workspace_config: Option<PathBuf>,
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
            mobile_bind: None,
            workspace_config: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokerEndpoint {
    pub host: String,
    pub port: u16,
    pub session_id: String,
    pub session_secret: String,
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

pub fn run_loopback_server(config: BrokerServerConfig) -> Result<(), BrokerServerError> {
    let session_id = format!("broker-session-{}", random_hex(16)?);
    let session_secret = random_hex(32)?;
    let mut broker = Broker::new_persistent(&session_id, &config.store_dir)
        .map_err(|error| BrokerServerError::new(error.message()))?;

    for (id, address) in &config.minidora_runtimes {
        let adapter = crate::adapters::minidora::MinidoraAdapter::new(address).map_err(|_| BrokerServerError::new("実行系接続先が不正"))?;
        broker.実行系登録(id, std::sync::Arc::new(adapter)).map_err(|_| BrokerServerError::new("実行系登録が不正または重複"))?;
    }
    let owner_secret = if config.owner_session_file.is_some() { Some(random_hex(32)?) } else { None };

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
        let protected=[config.store_dir.clone(),config.session_file.clone(),owner_file.clone(),path.clone()];
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
        transport: "authenticated_loopback_tcp".to_string(),
        max_request_bytes: config.max_request_bytes,
    };
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
        write_endpoint_file(path, &control)?;
    }
    write_endpoint_file(&config.session_file, &endpoint)?;

    listener.set_nonblocking(true).map_err(|_|BrokerServerError::new("listener設定失敗"))?;
    loop {
        broker.端末期限処理();
        if let Ok((stream,_)) = listener.accept() {
            if handle_stream(stream, &endpoint.session_secret, owner_secret.as_deref(), &mut broker, &config).unwrap_or(false) {break;}
        }
        if let Some(mobile) = &mobile {mobile.poll(&mut broker);}
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

fn handle_stream(
    mut stream: TcpStream,
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

    let mut reader =
        BufReader::new(stream.try_clone().map_err(|error| {
            BrokerServerError::new(format!("IPC streamのcloneに失敗: {error}"))
        })?);

    let auth_line = match read_limited_line(&mut reader, AUTH_LINE_MAX_BYTES) {
        Ok(Some(line)) => line,
        Ok(None) => {
            let response = broker.reject_ipc(
                "broker_ipc_malformed",
                "broker IPC auth line is missing",
                true,
            );
            write_response(&mut stream, &response)?;
            return Ok(false);
        }
        Err(IpcLineError::Oversized) => {
            let response = broker.reject_ipc(
                "broker_request_oversized",
                "broker IPC auth line exceeds the configured request limit",
                true,
            );
            write_response(&mut stream, &response)?;
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
        write_response(&mut stream, &response)?;
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
            write_response(&mut stream, &response)?;
            return Ok(false);
        }
        Err(IpcLineError::Oversized) => {
            let response = broker.reject_ipc(
                "broker_request_oversized",
                "broker IPC request exceeds the configured request limit",
                true,
            );
            write_response(&mut stream, &response)?;
            return Ok(false);
        }
        Err(IpcLineError::Io(message)) => return Err(BrokerServerError::new(message)),
    };

    let response = if owner { broker.owner要求処理(&request_json) } else { broker.handle_json(&request_json) };
    let shutdown = response.shutdown_requested;
    write_response(&mut stream, &response)?;
    Ok(shutdown)
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
    stream
        .write_all(encoded.as_bytes())
        .and_then(|_| stream.write_all(b"\n"))
        .map_err(|error| {
        BrokerServerError::new(format!("broker responseの書込みに失敗: {error}"))
        })
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
