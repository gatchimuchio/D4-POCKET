use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const MAX_REQUEST_BYTES: usize = 4 * 1024 * 1024;

struct State {
    stop: AtomicBool,
    accepted_connections: AtomicUsize,
    blocking_mode_failures: AtomicUsize,
    reader_clone_failures: AtomicUsize,
    request_line_failures: AtomicUsize,
    request_line_error_code: AtomicUsize,
    empty_request_lines: AtomicUsize,
    header_read_failures: AtomicUsize,
    post_requests: AtomicUsize,
    response_post_attempts: AtomicUsize,
    invalid_post_bodies: AtomicUsize,
    response_write_failures: AtomicUsize,
    response_bytes_written: AtomicUsize,
    model_list_requests: AtomicUsize,
    tool_offered: AtomicBool,
    tool_call_sent: AtomicBool,
    blocked_connect_requests: AtomicUsize,
    blocked_connect_openai: AtomicUsize,
    blocked_connect_chatgpt: AtomicUsize,
    blocked_connect_other: AtomicUsize,
    blocked_non_local_requests: AtomicUsize,
    blocked_unhandled_local_requests: AtomicUsize,
    command: Mutex<String>,
    workspace: String,
}

pub(super) struct CodexLoopbackResponses {
    port: u16,
    state: Arc<State>,
    worker: Option<JoinHandle<()>>,
    request_workers: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl CodexLoopbackResponses {
    pub(super) fn start(workspace: &Path) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let workspace_text = workspace.to_string_lossy().into_owned();
        let state = Arc::new(State {
            stop: AtomicBool::new(false),
            accepted_connections: AtomicUsize::new(0),
            blocking_mode_failures: AtomicUsize::new(0),
            reader_clone_failures: AtomicUsize::new(0),
            request_line_failures: AtomicUsize::new(0),
            request_line_error_code: AtomicUsize::new(0),
            empty_request_lines: AtomicUsize::new(0),
            header_read_failures: AtomicUsize::new(0),
            post_requests: AtomicUsize::new(0),
            response_post_attempts: AtomicUsize::new(0),
            invalid_post_bodies: AtomicUsize::new(0),
            response_write_failures: AtomicUsize::new(0),
            response_bytes_written: AtomicUsize::new(0),
            model_list_requests: AtomicUsize::new(0),
            tool_offered: AtomicBool::new(false),
            tool_call_sent: AtomicBool::new(false),
            blocked_connect_requests: AtomicUsize::new(0),
            blocked_connect_openai: AtomicUsize::new(0),
            blocked_connect_chatgpt: AtomicUsize::new(0),
            blocked_connect_other: AtomicUsize::new(0),
            blocked_non_local_requests: AtomicUsize::new(0),
            blocked_unhandled_local_requests: AtomicUsize::new(0),
            command: Mutex::new(synthetic_workspace_probe()),
            workspace: workspace_text,
        });
        let worker_state = Arc::clone(&state);
        let request_workers = Arc::new(Mutex::new(Vec::<JoinHandle<()>>::new()));
        let worker_request_workers = Arc::clone(&request_workers);
        let worker = thread::spawn(move || {
            while !worker_state.stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let request_state = Arc::clone(&worker_state);
                        let request_worker = thread::spawn(move || {
                            // Windowsでは受入socketがlistenerの非blocking状態を継承し得る。
                            if stream.set_nonblocking(false).is_err() {
                                request_state
                                    .blocking_mode_failures
                                    .fetch_add(1, Ordering::SeqCst);
                                return;
                            }
                            handle_request(stream, port, &request_state);
                        });
                        worker_request_workers
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .push(request_worker);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            port,
            state,
            worker: Some(worker),
            request_workers,
        })
    }

    pub(super) fn port(&self) -> u16 {
        self.port
    }

    pub(super) fn set_command(&self, command: String) {
        *self
            .state
            .command
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = command;
        self.state.tool_call_sent.store(false, Ordering::SeqCst);
    }

    pub(super) fn accepted_connections(&self) -> usize {
        self.state.accepted_connections.load(Ordering::SeqCst)
    }

    pub(super) fn incomplete_request_summary(&self) -> String {
        format!(
            "blocking_mode_failures={}, reader_clone_failures={}, request_line_failures={}, request_line_error_code={}, empty_request_lines={}, header_read_failures={}",
            self.state.blocking_mode_failures.load(Ordering::SeqCst),
            self.state.reader_clone_failures.load(Ordering::SeqCst),
            self.state.request_line_failures.load(Ordering::SeqCst),
            self.state.request_line_error_code.load(Ordering::SeqCst),
            self.state.empty_request_lines.load(Ordering::SeqCst),
            self.state.header_read_failures.load(Ordering::SeqCst),
        )
    }

    pub(super) fn incomplete_request_count(&self) -> usize {
        self.state.blocking_mode_failures.load(Ordering::SeqCst)
            + self.state.reader_clone_failures.load(Ordering::SeqCst)
            + self.state.request_line_failures.load(Ordering::SeqCst)
            + self.state.empty_request_lines.load(Ordering::SeqCst)
            + self.state.header_read_failures.load(Ordering::SeqCst)
    }

    pub(super) fn post_requests(&self) -> usize {
        self.state.post_requests.load(Ordering::SeqCst)
    }

    pub(super) fn response_post_attempts(&self) -> usize {
        self.state.response_post_attempts.load(Ordering::SeqCst)
    }

    pub(super) fn invalid_post_bodies(&self) -> usize {
        self.state.invalid_post_bodies.load(Ordering::SeqCst)
    }

    pub(super) fn response_write_summary(&self) -> String {
        format!(
            "failures={}, body_bytes={}",
            self.state.response_write_failures.load(Ordering::SeqCst),
            self.state.response_bytes_written.load(Ordering::SeqCst),
        )
    }

    pub(super) fn response_write_failures(&self) -> usize {
        self.state.response_write_failures.load(Ordering::SeqCst)
    }

    pub(super) fn model_list_requests(&self) -> usize {
        self.state.model_list_requests.load(Ordering::SeqCst)
    }

    pub(super) fn tool_was_offered(&self) -> bool {
        self.state.tool_offered.load(Ordering::SeqCst)
    }

    pub(super) fn tool_call_was_sent(&self) -> bool {
        self.state.tool_call_sent.load(Ordering::SeqCst)
    }

    pub(super) fn blocked_external_requests(&self) -> usize {
        self.state.blocked_connect_requests.load(Ordering::SeqCst)
            + self.state.blocked_non_local_requests.load(Ordering::SeqCst)
            + self
                .state
                .blocked_unhandled_local_requests
                .load(Ordering::SeqCst)
    }

    pub(super) fn blocked_external_summary(&self) -> String {
        format!(
            "CONNECT={}, openai={}, chatgpt={}, other={}, non_local={}, unhandled_local={}",
            self.state.blocked_connect_requests.load(Ordering::SeqCst),
            self.state.blocked_connect_openai.load(Ordering::SeqCst),
            self.state.blocked_connect_chatgpt.load(Ordering::SeqCst),
            self.state.blocked_connect_other.load(Ordering::SeqCst),
            self.state.blocked_non_local_requests.load(Ordering::SeqCst),
            self.state
                .blocked_unhandled_local_requests
                .load(Ordering::SeqCst),
        )
    }
}

impl Drop for CodexLoopbackResponses {
    fn drop(&mut self) {
        self.state.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let request_workers = std::mem::take(
            &mut *self
                .request_workers
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        for worker in request_workers {
            let _ = worker.join();
        }
    }
}

fn synthetic_workspace_probe() -> String {
    concat!(
        "$ErrorActionPreference='Stop'; ",
        "$workspace=(Get-Location).Path; ",
        "$secretDenied=$false; try { $null=[IO.File]::ReadAllText((Join-Path $workspace 'private\\credential-backup.txt')) } catch { $secretDenied=$true }; ",
        "if (-not $secretDenied) { exit 41 }; ",
        "$outside=Split-Path -Parent $workspace; ",
        "$outsideReadDenied=$false; try { $null=[IO.File]::ReadAllText((Join-Path $outside 'outside-read-marker.txt')) } catch { $outsideReadDenied=$true }; ",
        "if (-not $outsideReadDenied) { exit 42 }; ",
        "$outsideWriteDenied=$false; try { [IO.File]::WriteAllText((Join-Path $outside 'outside-write-marker.txt'),'unexpected'); } catch { $outsideWriteDenied=$true }; ",
        "if (-not $outsideWriteDenied) { exit 43 }; ",
        "[IO.File]::WriteAllText((Join-Path $workspace 'broker-real-codex-marker.txt'),'synthetic-task-write'); exit 0"
    )
    .to_owned()
}

fn handle_request(mut stream: TcpStream, port: u16, state: &State) {
    state.accepted_connections.fetch_add(1, Ordering::SeqCst);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let reader_stream = match stream.try_clone() {
        Ok(stream) => stream,
        Err(_) => {
            state.reader_clone_failures.fetch_add(1, Ordering::SeqCst);
            return;
        }
    };
    let mut reader = BufReader::new(reader_stream);
    let mut request_line = String::new();
    if let Err(error) = reader.read_line(&mut request_line) {
        state.request_line_error_code.store(
            error.raw_os_error().unwrap_or(0).unsigned_abs() as usize,
            Ordering::SeqCst,
        );
        state.request_line_failures.fetch_add(1, Ordering::SeqCst);
        return;
    }
    if request_line.is_empty() {
        state.empty_request_lines.fetch_add(1, Ordering::SeqCst);
        return;
    }
    let mut content_length = 0usize;
    let mut chunked = false;
    let mut host = String::new();
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => {
                state.header_read_failures.fetch_add(1, Ordering::SeqCst);
                return;
            }
            Ok(_) if line == "\r\n" || line == "\n" => break,
            Ok(_) => {
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        let Ok(parsed) = value.trim().parse::<usize>() else {
                            let _ = respond(&mut stream, 400, "text/plain", b"");
                            return;
                        };
                        content_length = parsed;
                    } else if name.eq_ignore_ascii_case("transfer-encoding") {
                        chunked = value.to_ascii_lowercase().contains("chunked");
                    } else if name.eq_ignore_ascii_case("host") {
                        host = value.trim().to_ascii_lowercase();
                    }
                }
            }
        }
    }
    if content_length > MAX_REQUEST_BYTES {
        let _ = respond(&mut stream, 413, "text/plain", b"");
        return;
    }
    let request_line = request_line.trim_end();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();
    if method.eq_ignore_ascii_case("CONNECT") {
        block_connect(&mut stream, state, target);
        return;
    }
    let expected_host = format!("127.0.0.1:{port}");
    let local_host = host == expected_host || host == format!("localhost:{port}");
    let local_target = target.starts_with('/') && !target.starts_with("//");
    if !local_host || !local_target {
        block_external(&mut stream, state, BlockedRequest::NonLocal);
        return;
    }
    let path = target.split('?').next().unwrap_or(target);
    if method.eq_ignore_ascii_case("GET")
        && (path.ends_with("/v1/models") || path.ends_with("/models"))
    {
        state.model_list_requests.fetch_add(1, Ordering::SeqCst);
        let response = json!({
            "object": "list",
            "data": [{"id": "d4p-local-probe", "object": "model", "created": 0, "owned_by": "local-test"}]
        });
        let payload = response.to_string();
        let _ = respond(&mut stream, 200, "application/json", payload.as_bytes());
        return;
    }
    if !method.eq_ignore_ascii_case("POST")
        || !(path.ends_with("/v1/responses") || path.ends_with("/responses"))
    {
        block_external(&mut stream, state, BlockedRequest::UnhandledLocal);
        return;
    }

    state.response_post_attempts.fetch_add(1, Ordering::SeqCst);
    let body = match read_request_body(&mut reader, content_length, chunked) {
        Ok(body) => body,
        Err(()) => {
            state.invalid_post_bodies.fetch_add(1, Ordering::SeqCst);
            let _ = respond(&mut stream, 400, "text/plain", b"");
            return;
        }
    };

    let request: Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => {
            state.invalid_post_bodies.fetch_add(1, Ordering::SeqCst);
            let _ = respond(&mut stream, 400, "text/plain", b"");
            return;
        }
    };
    let request_number = state.post_requests.fetch_add(1, Ordering::SeqCst) + 1;
    if request_number == 1 {
        let offered = request
            .get("tools")
            .and_then(Value::as_array)
            .is_some_and(|tools| {
                tools
                    .iter()
                    .any(|tool| tool.get("name").and_then(Value::as_str) == Some("exec_command"))
            });
        state.tool_offered.store(offered, Ordering::SeqCst);
    }
    let response_id = format!("d4p-broker-task-{request_number}");
    let mut events = vec![event(
        "response.created",
        json!({"type":"response.created","response":{"id":response_id}}),
    )];
    let tool_result_exists = Path::new(&state.workspace)
        .join("broker-real-codex-marker.txt")
        .is_file();
    if !tool_result_exists && state.tool_offered.load(Ordering::SeqCst) {
        let arguments = json!({
            "cmd": state
                .command
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
            "workdir": state.workspace,
            "shell": "powershell.exe",
            "yield_time_ms": 30000
        });
        let item = json!({
            "type":"function_call",
            "call_id":format!("d4p-broker-task-call-{request_number}"),
            "name":"exec_command",
            "arguments":arguments.to_string()
        });
        events.push(event(
            "response.output_item.done",
            json!({"type":"response.output_item.done","item":item}),
        ));
        state.tool_call_sent.store(true, Ordering::SeqCst);
    } else {
        let item = json!({
            "type":"message",
            "role":"assistant",
            "id":format!("d4p-broker-task-message-{request_number}"),
            "content":[{"type":"output_text","text":"合成試験Taskが完了しました"}]
        });
        events.push(event(
            "response.output_item.done",
            json!({"type":"response.output_item.done","item":item}),
        ));
    }
    events.push(event(
        "response.completed",
        json!({
            "type":"response.completed",
            "response":{"id":response_id,"usage":{"input_tokens":1,"input_tokens_details":null,"output_tokens":1,"output_tokens_details":null,"total_tokens":2}}
        }),
    ));
    let payload = events
        .into_iter()
        .map(|event| {
            format!(
                "event: {}\ndata: {}\n\n",
                event["type"].as_str().unwrap_or_default(),
                event
            )
        })
        .collect::<String>();
    match respond(&mut stream, 200, "text/event-stream", payload.as_bytes()) {
        Ok(()) => {
            state
                .response_bytes_written
                .fetch_add(payload.len(), Ordering::SeqCst);
        }
        Err(_) => {
            state.response_write_failures.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn read_request_body(
    reader: &mut BufReader<TcpStream>,
    content_length: usize,
    chunked: bool,
) -> Result<Vec<u8>, ()> {
    if !chunked {
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).map_err(|_| ())?;
        return Ok(body);
    }
    let mut body = Vec::new();
    loop {
        let mut size_line = String::new();
        reader.read_line(&mut size_line).map_err(|_| ())?;
        let size_text = size_line.split(';').next().ok_or(())?.trim();
        let size = usize::from_str_radix(size_text, 16).map_err(|_| ())?;
        if size == 0 {
            loop {
                let mut trailer = String::new();
                reader.read_line(&mut trailer).map_err(|_| ())?;
                if trailer == "\r\n" || trailer == "\n" {
                    break;
                }
            }
            return Ok(body);
        }
        if body.len().saturating_add(size) > MAX_REQUEST_BYTES {
            return Err(());
        }
        let mut chunk = vec![0; size];
        reader.read_exact(&mut chunk).map_err(|_| ())?;
        let mut terminator = [0; 2];
        reader.read_exact(&mut terminator).map_err(|_| ())?;
        if terminator != *b"\r\n" {
            return Err(());
        }
        body.extend_from_slice(&chunk);
    }
}

fn event(_event_type: &str, payload: Value) -> Value {
    payload
}

enum BlockedRequest {
    NonLocal,
    UnhandledLocal,
}

fn block_external(stream: &mut TcpStream, state: &State, blocked: BlockedRequest) {
    let counter = match blocked {
        BlockedRequest::NonLocal => &state.blocked_non_local_requests,
        BlockedRequest::UnhandledLocal => &state.blocked_unhandled_local_requests,
    };
    counter.fetch_add(1, Ordering::SeqCst);
    let _ = respond(stream, 403, "text/plain", b"");
}

fn block_connect(stream: &mut TcpStream, state: &State, target: &str) {
    state
        .blocked_connect_requests
        .fetch_add(1, Ordering::SeqCst);
    let host = target
        .split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let category = if host.ends_with("openai.com") {
        &state.blocked_connect_openai
    } else if host.ends_with("chatgpt.com") {
        &state.blocked_connect_chatgpt
    } else {
        &state.blocked_connect_other
    };
    category.fetch_add(1, Ordering::SeqCst);
    let _ = respond(stream, 403, "text/plain", b"");
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let headers = format!(
        "HTTP/1.0 {status} {reason}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}
