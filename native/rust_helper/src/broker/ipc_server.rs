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
}

impl BrokerServerConfig {
    pub fn new(store_dir: PathBuf, session_file: PathBuf) -> Self {
        Self {
            store_dir,
            session_file,
            port: 0,
            max_request_bytes: DEFAULT_MAX_REQUEST_BYTES,
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

    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), config.port);
    let listener = TcpListener::bind(bind_addr)
        .map_err(|error| BrokerServerError::new(format!("failed to bind broker IPC: {error}")))?;
    let local_addr = listener.local_addr().map_err(|error| {
        BrokerServerError::new(format!("failed to read broker IPC addr: {error}"))
    })?;
    if local_addr.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) {
        return Err(BrokerServerError::new(
            "broker IPC refused to expose a non-loopback bind address",
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
    write_endpoint_file(&config.session_file, &endpoint)?;

    for incoming in listener.incoming() {
        let stream = match incoming {
            Ok(stream) => stream,
            Err(_) => continue,
        };
        let shutdown = match handle_stream(stream, &endpoint.session_secret, &mut broker, &config) {
            Ok(shutdown) => shutdown,
            Err(_) => continue,
        };
        if shutdown {
            break;
        }
    }
    Ok(())
}

fn handle_stream(
    mut stream: TcpStream,
    session_secret: &str,
    broker: &mut Broker,
    config: &BrokerServerConfig,
) -> Result<bool, BrokerServerError> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| {
            BrokerServerError::new(format!("failed to set IPC read timeout: {error}"))
        })?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| {
            BrokerServerError::new(format!("failed to set IPC write timeout: {error}"))
        })?;

    let mut reader =
        BufReader::new(stream.try_clone().map_err(|error| {
            BrokerServerError::new(format!("failed to clone IPC stream: {error}"))
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

    if auth_line != session_secret {
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

    let response = broker.handle_json(&request_json);
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
            IpcLineError::Io(format!("failed to read broker IPC line: {error}"))
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
        .map_err(|error| IpcLineError::Io(format!("broker IPC line is not UTF-8: {error}")))
}

fn write_response(
    stream: &mut TcpStream,
    response: &BrokerResponse,
) -> Result<(), BrokerServerError> {
    let encoded = response.to_json_string().map_err(|error| {
        BrokerServerError::new(format!("failed to encode broker response: {error}"))
    })?;
    stream
        .write_all(encoded.as_bytes())
        .and_then(|_| stream.write_all(b"\n"))
        .map_err(|error| {
            BrokerServerError::new(format!("failed to write broker response: {error}"))
        })
}

fn write_endpoint_file(path: &PathBuf, endpoint: &BrokerEndpoint) -> Result<(), BrokerServerError> {
    let encoded = serde_json::to_string(endpoint).map_err(|error| {
        BrokerServerError::new(format!("failed to encode broker endpoint: {error}"))
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
            BrokerServerError::new(format!("failed to create broker endpoint file: {error}"))
        })?;
        file.write_all(encoded.as_bytes()).map_err(|error| {
            BrokerServerError::new(format!("failed to write broker endpoint file: {error}"))
        })?;
        file.sync_data().map_err(|error| {
            BrokerServerError::new(format!("failed to sync broker endpoint file: {error}"))
        })?;
    }
    std::fs::rename(&temporary_path, path).map_err(|error| {
        BrokerServerError::new(format!("failed to commit broker endpoint file: {error}"))
    })
}

fn random_hex(byte_count: usize) -> Result<String, BrokerServerError> {
    let mut bytes = vec![0u8; byte_count];
    getrandom::getrandom(&mut bytes).map_err(|error| {
        BrokerServerError::new(format!("failed to generate broker secret: {error}"))
    })?;
    Ok(hex::encode(bytes))
}

enum IpcLineError {
    Io(String),
    Oversized,
}
