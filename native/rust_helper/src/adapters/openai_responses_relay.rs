//! Broker内でProvider API資格情報を保ち、Codex CLIのResponses通信だけを中継する。
//!
//! CLIへAPI keyを渡さず、loopbackのTask限定channelを通してBrokerがOpenAIへ接続する。
//! Workspace tool processにはAPI keyもchannel tokenも環境変数として継承させない。

use crate::broker::update_download::{
    build_pinned_public_https_client, resolve_public_https_addresses,
};
use reqwest::header::{HeaderValue, ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Method, StatusCode};
use ring::rand::{SecureRandom, SystemRandom};
use std::io::{BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use zeroize::{Zeroize, Zeroizing};

const OPENAI_HOST: &str = "api.openai.com";
const CHANNEL_HEADER: &str = "x-d4p-broker-channel";
const MAX_REQUEST_HEADER_BYTES: usize = 32 * 1024;
const MAX_REQUEST_BODY_BYTES: usize = 4 * 1024 * 1024;
const MAX_RESPONSE_BODY_BYTES: usize = 32 * 1024 * 1024;
const MAX_ACTIVE_CONNECTIONS: usize = 4;
const MAX_API_REQUESTS_PER_TASK: usize = 256;
const HTTP_IO_TIMEOUT: Duration = Duration::from_secs(5);
const CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Clone, Copy)]
enum ApiOperation {
    Models,
    Responses,
}

impl ApiOperation {
    fn method(self) -> Method {
        match self {
            Self::Models => Method::GET,
            Self::Responses => Method::POST,
        }
    }

    fn path(self) -> &'static str {
        match self {
            Self::Models => "/v1/models",
            Self::Responses => "/v1/responses",
        }
    }

    fn accept(self) -> &'static str {
        match self {
            Self::Models => "application/json",
            Self::Responses => "text/event-stream, application/json",
        }
    }
}

struct RelayState {
    api_key: Zeroizing<Vec<u8>>,
    channel: Arc<Zeroizing<String>>,
    client: Client,
    expected_host: String,
    model_id: String,
    shutdown: Arc<AtomicBool>,
    deadline: Instant,
    request_count: AtomicUsize,
    active_connections: Arc<AtomicUsize>,
}

pub(crate) struct OpenAiResponsesRelay {
    base_url: String,
    channel: Arc<Zeroizing<String>>,
    shutdown: Arc<AtomicBool>,
    server: Option<JoinHandle<()>>,
}

impl OpenAiResponsesRelay {
    pub(crate) fn start(
        api_key: &[u8],
        model_id: &str,
        deadline: Instant,
        cancel: &AtomicBool,
    ) -> Result<Self, ()> {
        if api_key.is_empty()
            || api_key.iter().any(u8::is_ascii_control)
            || model_id.is_empty()
            || model_id.len() > 128
            || model_id.bytes().enumerate().any(|(index, byte)| {
                !byte.is_ascii_alphanumeric() && !(index > 0 && b"._:/-".contains(&byte))
            })
            || cancel.load(Ordering::Acquire)
            || Instant::now() >= deadline
        {
            return Err(());
        }

        let addresses =
            resolve_public_https_addresses(OPENAI_HOST, cancel, deadline).map_err(|_| ())?;
        if cancel.load(Ordering::Acquire) || Instant::now() >= deadline {
            return Err(());
        }
        let timeout = deadline.saturating_duration_since(Instant::now());
        let client =
            build_pinned_public_https_client(OPENAI_HOST, &addresses, timeout).map_err(|_| ())?;

        let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .map_err(|_| ())?;
        listener.set_nonblocking(true).map_err(|_| ())?;
        let port = listener.local_addr().map_err(|_| ())?.port();
        if port == 0 {
            return Err(());
        }

        let mut random_bytes = Zeroizing::new([0u8; 32]);
        SystemRandom::new()
            .fill(random_bytes.as_mut())
            .map_err(|_| ())?;
        let mut channel_text = String::with_capacity(64);
        for byte in random_bytes.iter() {
            use std::fmt::Write as _;
            write!(&mut channel_text, "{byte:02x}").map_err(|_| ())?;
        }
        random_bytes.zeroize();
        let channel = Arc::new(Zeroizing::new(channel_text));

        let shutdown = Arc::new(AtomicBool::new(false));
        let state = Arc::new(RelayState {
            api_key: Zeroizing::new(api_key.to_vec()),
            channel: Arc::clone(&channel),
            client,
            expected_host: format!("127.0.0.1:{port}"),
            model_id: model_id.to_owned(),
            shutdown: Arc::clone(&shutdown),
            deadline,
            request_count: AtomicUsize::new(0),
            active_connections: Arc::new(AtomicUsize::new(0)),
        });
        let server_state = Arc::clone(&state);
        let server = thread::Builder::new()
            .name("D4Pocket Provider API relay".to_owned())
            .spawn(move || serve_loopback(listener, server_state))
            .map_err(|_| ())?;

        Ok(Self {
            base_url: format!("http://127.0.0.1:{port}/v1"),
            channel,
            shutdown,
            server: Some(server),
        })
    }

    pub(crate) fn codex_config_overrides(&self) -> Vec<String> {
        let provider = "d4p_broker_openai";
        vec![
            "model_provider=\"d4p_broker_openai\"".to_owned(),
            format!("model_providers.{provider}.name=\"D4 Pocket Broker\""),
            format!("model_providers.{provider}.base_url=\"{}\"", self.base_url),
            format!("model_providers.{provider}.wire_api=\"responses\""),
            format!("model_providers.{provider}.requires_openai_auth=false"),
            format!("model_providers.{provider}.supports_websockets=false"),
            format!("model_providers.{provider}.request_max_retries=0"),
            format!("model_providers.{provider}.stream_max_retries=0"),
            format!(
                "model_providers.{provider}.http_headers={{\"{CHANNEL_HEADER}\"=\"{}\"}}",
                self.channel.as_str()
            ),
            "features.apps=false".to_owned(),
            "features.plugins=false".to_owned(),
            "analytics.enabled=false".to_owned(),
        ]
    }
}

impl Drop for OpenAiResponsesRelay {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        if let Some(server) = self.server.take() {
            let _ = server.join();
        }
    }
}

struct ParsedRequest {
    operation: ApiOperation,
    content_length: usize,
}

fn serve_loopback(listener: TcpListener, state: Arc<RelayState>) {
    let mut connections = Vec::<JoinHandle<()>>::new();
    while !state.shutdown.load(Ordering::Acquire) && Instant::now() < state.deadline {
        let mut index = 0;
        while index < connections.len() {
            if connections[index].is_finished() {
                let completed = connections.swap_remove(index);
                let _ = completed.join();
            } else {
                index += 1;
            }
        }
        match listener.accept() {
            Ok((stream, peer)) => {
                if !peer.ip().is_loopback() {
                    continue;
                }
                let active = state.active_connections.fetch_add(1, Ordering::AcqRel);
                if active >= MAX_ACTIVE_CONNECTIONS {
                    state.active_connections.fetch_sub(1, Ordering::AcqRel);
                    let mut stream = stream;
                    write_local_error(&mut stream, StatusCode::TOO_MANY_REQUESTS);
                    continue;
                }
                let connection_state = Arc::clone(&state);
                let active_connections = Arc::clone(&connection_state.active_connections);
                match thread::Builder::new()
                    .name("D4Pocket Provider API request".to_owned())
                    .spawn(move || {
                        let _active = ActiveConnection(active_connections);
                        let _ = handle_client(stream, connection_state);
                    }) {
                    Ok(connection) => connections.push(connection),
                    Err(_) => {
                        state.active_connections.fetch_sub(1, Ordering::AcqRel);
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => break,
        }
    }
    state.shutdown.store(true, Ordering::Release);
    for connection in connections {
        let _ = connection.join();
    }
}

struct ActiveConnection(Arc<AtomicUsize>);

impl Drop for ActiveConnection {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

fn handle_client(mut stream: TcpStream, state: Arc<RelayState>) -> Result<(), ()> {
    stream
        .set_read_timeout(Some(HTTP_IO_TIMEOUT))
        .map_err(|_| ())?;
    stream
        .set_write_timeout(Some(HTTP_IO_TIMEOUT))
        .map_err(|_| ())?;
    let reader_stream = stream.try_clone().map_err(|_| ())?;
    let mut reader = BufReader::new(reader_stream);
    let header = read_header_block(&mut reader).map_err(|_| ())?;
    let parsed = match parse_request(&header, &state) {
        Ok(request) => request,
        Err(status) => {
            write_local_error(&mut stream, status);
            return Err(());
        }
    };
    if state.shutdown.load(Ordering::Acquire) || Instant::now() >= state.deadline {
        write_local_error(&mut stream, StatusCode::SERVICE_UNAVAILABLE);
        return Err(());
    }
    let previous = state.request_count.fetch_add(1, Ordering::AcqRel);
    if previous >= MAX_API_REQUESTS_PER_TASK {
        write_local_error(&mut stream, StatusCode::TOO_MANY_REQUESTS);
        return Err(());
    }

    let mut body = Zeroizing::new(vec![0u8; parsed.content_length]);
    reader.read_exact(&mut body).map_err(|_| ())?;
    if let ApiOperation::Responses = parsed.operation {
        let value: serde_json::Value = serde_json::from_slice(&body).map_err(|_| {
            write_local_error(&mut stream, StatusCode::BAD_REQUEST);
        })?;
        if value.get("model").and_then(serde_json::Value::as_str) != Some(state.model_id.as_str()) {
            write_local_error(&mut stream, StatusCode::BAD_REQUEST);
            return Err(());
        }
    }

    let request_body = std::mem::take(&mut *body);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| ())?;
    runtime.block_on(forward_to_openai(
        stream,
        parsed.operation,
        request_body,
        state,
    ))
}

fn read_header_block(reader: &mut BufReader<TcpStream>) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(2048);
    while bytes.len() < MAX_REQUEST_HEADER_BYTES {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte)?;
        bytes.push(byte[0]);
        if bytes.ends_with(b"\r\n\r\n") {
            return Ok(bytes);
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "request header bound exceeded",
    ))
}

fn parse_request(header: &[u8], state: &RelayState) -> Result<ParsedRequest, StatusCode> {
    let text = std::str::from_utf8(header).map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next().ok_or(StatusCode::BAD_REQUEST)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or(StatusCode::BAD_REQUEST)?;
    let path = parts.next().ok_or(StatusCode::BAD_REQUEST)?;
    let version = parts.next().ok_or(StatusCode::BAD_REQUEST)?;
    if parts.next().is_some() || version != "HTTP/1.1" {
        return Err(StatusCode::BAD_REQUEST);
    }
    let operation = match (method, path) {
        ("GET", "/v1/models") => ApiOperation::Models,
        ("POST", "/v1/responses") => ApiOperation::Responses,
        _ => return Err(StatusCode::NOT_FOUND),
    };

    let mut content_length = None;
    let mut host = None;
    let mut channel = None;
    let mut content_type_seen = false;
    for line in lines {
        if line.is_empty() {
            break;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            return Err(StatusCode::BAD_REQUEST);
        }
        let (name, value) = line.split_once(':').ok_or(StatusCode::BAD_REQUEST)?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || value
                .bytes()
                .any(|byte| byte.is_ascii_control() && byte != b'\t')
        {
            return Err(StatusCode::BAD_REQUEST);
        }
        let value = value.trim_matches(|character| character == ' ' || character == '\t');
        match name.to_ascii_lowercase().as_str() {
            "authorization"
            | "proxy-authorization"
            | "cookie"
            | "transfer-encoding"
            | "content-encoding" => return Err(StatusCode::BAD_REQUEST),
            "host" => {
                if host.replace(value).is_some() {
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
            "content-length" => {
                if content_length.replace(value).is_some() {
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
            CHANNEL_HEADER => {
                if channel.replace(value).is_some() {
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
            "content-type" => {
                if content_type_seen {
                    return Err(StatusCode::BAD_REQUEST);
                }
                content_type_seen = true;
                if matches!(operation, ApiOperation::Responses)
                    && !value
                        .split(';')
                        .next()
                        .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
                {
                    return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
                }
            }
            _ => {}
        }
    }
    if matches!(operation, ApiOperation::Responses) && !content_type_seen {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    if host != Some(state.expected_host.as_str())
        || channel.map_or(true, |candidate| {
            !channel_matches(candidate, state.channel.as_str())
        })
    {
        return Err(StatusCode::FORBIDDEN);
    }
    let content_length = match content_length {
        Some(raw) => raw.parse::<usize>().map_err(|_| StatusCode::BAD_REQUEST)?,
        None if matches!(operation, ApiOperation::Models) => 0,
        None => return Err(StatusCode::LENGTH_REQUIRED),
    };
    if content_length > MAX_REQUEST_BODY_BYTES
        || (matches!(operation, ApiOperation::Responses) && content_length == 0)
        || (matches!(operation, ApiOperation::Models) && content_length != 0)
    {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    Ok(ParsedRequest {
        operation,
        content_length,
    })
}

fn channel_matches(candidate: &str, expected: &str) -> bool {
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, expected.as_bytes());
    let tag = ring::hmac::sign(&key, candidate.as_bytes());
    ring::hmac::verify(&key, expected.as_bytes(), tag.as_ref()).is_ok()
}

async fn forward_to_openai(
    mut stream: TcpStream,
    operation: ApiOperation,
    body: Vec<u8>,
    state: Arc<RelayState>,
) -> Result<(), ()> {
    let mut authorization = Zeroizing::new(Vec::with_capacity(7 + state.api_key.len()));
    authorization.extend_from_slice(b"Bearer ");
    authorization.extend_from_slice(&state.api_key);
    let authorization = HeaderValue::from_bytes(authorization.as_slice()).map_err(|_| ())?;

    let url = format!("https://{}{path}", OPENAI_HOST, path = operation.path());
    let mut request = state
        .client
        .request(operation.method(), url)
        .header(AUTHORIZATION, authorization)
        .header(ACCEPT, operation.accept())
        .header(ACCEPT_ENCODING, "identity");
    if matches!(operation, ApiOperation::Responses) {
        request = request.header(CONTENT_TYPE, "application/json").body(body);
    }

    let request_future = request.send();
    let mut request_future = Box::pin(request_future);
    let response = loop {
        if state.shutdown.load(Ordering::Acquire) || Instant::now() >= state.deadline {
            write_local_error(&mut stream, StatusCode::SERVICE_UNAVAILABLE);
            return Err(());
        }
        match tokio::time::timeout(CANCEL_POLL_INTERVAL, &mut request_future).await {
            Ok(Ok(response)) => break response,
            Ok(Err(_)) => {
                write_local_error(&mut stream, StatusCode::BAD_GATEWAY);
                return Err(());
            }
            Err(_) => continue,
        }
    };

    let status = response.status();
    let upstream_content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let content_type = if matches!(operation, ApiOperation::Responses)
        && upstream_content_type
            .split(';')
            .next()
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("text/event-stream"))
    {
        "text/event-stream"
    } else {
        "application/json"
    };
    let mut response = response;
    if status == StatusCode::NO_CONTENT || status == StatusCode::NOT_MODIFIED {
        write!(
            stream,
            "HTTP/1.1 {} {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Response")
        )
        .map_err(|_| ())?;
        let _ = stream.flush();
        return Ok(());
    }
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Response"),
        content_type
    )
    .map_err(|_| ())?;
    stream.flush().map_err(|_| ())?;

    let mut response_bytes = 0usize;
    loop {
        if state.shutdown.load(Ordering::Acquire) || Instant::now() >= state.deadline {
            return Err(());
        }
        let mut chunk_future = Box::pin(response.chunk());
        let chunk = loop {
            if state.shutdown.load(Ordering::Acquire) || Instant::now() >= state.deadline {
                return Err(());
            }
            match tokio::time::timeout(CANCEL_POLL_INTERVAL, &mut chunk_future).await {
                Ok(Ok(chunk)) => break chunk,
                Ok(Err(_)) => return Err(()),
                Err(_) => continue,
            }
        };
        let Some(chunk) = chunk else {
            break;
        };
        response_bytes = response_bytes.checked_add(chunk.len()).ok_or(())?;
        if response_bytes > MAX_RESPONSE_BODY_BYTES {
            return Err(());
        }
        write!(stream, "{:x}\r\n", chunk.len()).map_err(|_| ())?;
        stream.write_all(&chunk).map_err(|_| ())?;
        stream.write_all(b"\r\n").map_err(|_| ())?;
        stream.flush().map_err(|_| ())?;
    }
    stream.write_all(b"0\r\n\r\n").map_err(|_| ())?;
    stream.flush().map_err(|_| ())?;
    Ok(())
}

fn write_local_error(stream: &mut TcpStream, status: StatusCode) {
    let body =
        b"{\"error\":{\"type\":\"d4p_broker_relay\",\"message\":\"Provider request unavailable\"}}";
    let reason = status.canonical_reason().unwrap_or("Error");
    let _ = write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status.as_u16(),
        reason,
        body.len()
    );
    let _ = stream.write_all(body);
    let _ = stream.flush();
}
