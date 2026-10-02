use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
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
    tool_result_received: AtomicBool,
    tool_output_items_seen: AtomicUsize,
    tool_output_missing_call_id: AtomicUsize,
    tool_output_id_mismatches: AtomicUsize,
    tool_output_diagnostics: Mutex<Vec<Value>>,
    request_shapes: Mutex<Vec<String>>,
    repeated_tool_call_rejections: AtomicUsize,
    tool_call_count: AtomicUsize,
    active_tool_call_id: Mutex<Option<String>>,
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
            tool_result_received: AtomicBool::new(false),
            tool_output_items_seen: AtomicUsize::new(0),
            tool_output_missing_call_id: AtomicUsize::new(0),
            tool_output_id_mismatches: AtomicUsize::new(0),
            tool_output_diagnostics: Mutex::new(Vec::new()),
            request_shapes: Mutex::new(Vec::new()),
            repeated_tool_call_rejections: AtomicUsize::new(0),
            tool_call_count: AtomicUsize::new(0),
            active_tool_call_id: Mutex::new(None),
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
        self.state.tool_offered.store(false, Ordering::SeqCst);
        self.state
            .repeated_tool_call_rejections
            .store(0, Ordering::SeqCst);
        self.state
            .tool_result_received
            .store(false, Ordering::SeqCst);
        self.state.tool_output_items_seen.store(0, Ordering::SeqCst);
        self.state
            .tool_output_missing_call_id
            .store(0, Ordering::SeqCst);
        self.state
            .tool_output_id_mismatches
            .store(0, Ordering::SeqCst);
        self.state
            .tool_output_diagnostics
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        self.state
            .request_shapes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        *self
            .state
            .active_tool_call_id
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
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

    pub(super) fn tool_result_was_received(&self) -> bool {
        self.state.tool_result_received.load(Ordering::SeqCst)
    }

    pub(super) fn repeated_tool_call_rejections(&self) -> usize {
        self.state
            .repeated_tool_call_rejections
            .load(Ordering::SeqCst)
    }

    pub(super) fn tool_output_summary(&self) -> String {
        format!(
            "items={}, missing_call_id={}, id_mismatch={}",
            self.state.tool_output_items_seen.load(Ordering::SeqCst),
            self.state
                .tool_output_missing_call_id
                .load(Ordering::SeqCst),
            self.state.tool_output_id_mismatches.load(Ordering::SeqCst),
        )
    }

    pub(super) fn tool_output_diagnostics(&self) -> Vec<Value> {
        self.state
            .tool_output_diagnostics
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub(super) fn request_shape_summary(&self) -> String {
        format!(
            "{:?}",
            *self
                .state
                .request_shapes
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        )
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

// TEMP書込み結果は個別に記録する。許可scopeを確認した後の独立したWorkspace／secret境界試験は、TEMP失敗だけでは省略しない。
fn synthetic_workspace_probe() -> String {
    concat!(
        "$ErrorActionPreference='Stop'; ",
        "Write-Output 'D4P_R2_SYNTHETIC_PROBE_STARTED'; ",
        "$workspace=(Get-Location).Path; ",
        "$tempReport=Join-Path $workspace 'broker-real-codex-temp-report.json'; ",
        "$tempMarkerName='d4p-broker-temp-observer-'+[IO.Path]::GetFileName($workspace)+'.marker'; ",
        "$tempStep='initial'; $tempState=@{version=1;stage='started';temp=$env:TEMP;tmp=$env:TMP;marker_name=$tempMarkerName;temp_configured=([string]::IsNullOrWhiteSpace($env:TEMP) -eq $false);tmp_configured=([string]::IsNullOrWhiteSpace($env:TMP) -eq $false);temp_matches_workspace_scratch=$false;tmp_matches_workspace_scratch=$false;temp_scope='unrecognized';temp_write='pending';temp_step=$tempStep;temp_error_type=$null;temp_error_hresult=$null}|ConvertTo-Json -Compress; ",
        "[IO.File]::WriteAllText($tempReport,$tempState); ",
        "$tempStep='scratch_discovery'; try { $scratch=@(Get-ChildItem -LiteralPath $workspace -Directory -Force | Where-Object { $_.Name -like '.d4p-tmp-*' }); if($scratch.Count -ne 1){throw [InvalidOperationException]::new('registered scratch count mismatch')}; ",
        "$tempStep='scope_validation'; ",
        "$expected=[IO.Path]::GetFullPath($scratch[0].FullName).TrimEnd([char]92); $tempActual=[IO.Path]::GetFullPath($env:TEMP).TrimEnd([char]92); $tmpActual=[IO.Path]::GetFullPath($env:TMP).TrimEnd([char]92); ",
        "$tempMatches=[string]::Equals($expected,$tempActual,[StringComparison]::OrdinalIgnoreCase); $tmpMatches=[string]::Equals($expected,$tmpActual,[StringComparison]::OrdinalIgnoreCase); ",
        "$tempParts=$tempActual.Split([char]92); $sandboxGuid=[Guid]::Empty; $mxcTemp=$false; if($tempParts.Count -ge 9){$mxcTemp=($tempParts[-1] -ieq 'Temp' -and $tempParts[-2] -ieq 'AC' -and $tempParts[-3].StartsWith('sandbox.',[StringComparison]::OrdinalIgnoreCase) -and [Guid]::TryParse($tempParts[-3].Substring(8),[ref]$sandboxGuid))}; ",
        "if($tempMatches -and $tmpMatches){$tempScope='broker_workspace_scratch'; $tempTarget=$expected} elseif([string]::Equals($tempActual,$tmpActual,[StringComparison]::OrdinalIgnoreCase) -and $mxcTemp){$tempScope='mxc_appcontainer'; $tempTarget=$tempActual} else {throw [InvalidOperationException]::new('TEMP/TMP is outside the Broker scratch or MxC AppContainer temp')}; ",
        "$tempStep='temp_directory_check'; if(-not (Test-Path -LiteralPath $tempTarget -PathType Container)){throw [DirectoryNotFoundException]::new('scoped TEMP directory is absent')}; $tempStep='temp_write'; [IO.File]::WriteAllText((Join-Path $tempTarget $tempMarkerName),'d4p synthetic temp observer'); $tempStep='temp_write_completed'; $tempWrite='passed'; $tempErrorType=$null; $tempErrorHResult=$null } catch { $tempError=$_.Exception.InnerException; if($null -eq $tempError){$tempError=$_.Exception}; $tempWrite='failed'; if($null -eq $tempMatches){$tempMatches=$false}; if($null -eq $tmpMatches){$tmpMatches=$false}; if($null -eq $tempScope){$tempScope='unrecognized'}; $tempErrorType=$tempError.GetType().FullName; $tempErrorHResult=$tempError.HResult }; ",
        "$tempState=@{version=1;stage='temp_checked';temp=$env:TEMP;tmp=$env:TMP;marker_name=$tempMarkerName;temp_configured=([string]::IsNullOrWhiteSpace($env:TEMP) -eq $false);tmp_configured=([string]::IsNullOrWhiteSpace($env:TMP) -eq $false);temp_matches_workspace_scratch=$tempMatches;tmp_matches_workspace_scratch=$tmpMatches;temp_scope=$tempScope;temp_write=$tempWrite;temp_step=$tempStep;temp_error_type=$tempErrorType;temp_error_hresult=$tempErrorHResult}|ConvertTo-Json -Compress; ",
        "[IO.File]::WriteAllText($tempReport,$tempState); ",
        "$continue=Join-Path $workspace 'broker-real-codex-task-continue'; ",
        "$deadline=[DateTime]::UtcNow.AddSeconds(30); ",
        "while(-not [IO.File]::Exists($continue)){if([DateTime]::UtcNow -ge $deadline){exit 44}; Start-Sleep -Milliseconds 25}; ",
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

fn collect_output_text(value: &Value, depth: usize, output: &mut String) {
    if depth > 8 || output.len() >= MAX_REQUEST_BYTES {
        return;
    }
    match value {
        Value::String(text) => {
            let remaining = MAX_REQUEST_BYTES.saturating_sub(output.len());
            let end = text
                .char_indices()
                .map(|(index, character)| index + character.len_utf8())
                .take_while(|end| *end <= remaining)
                .last()
                .unwrap_or_default();
            output.push_str(&text[..end]);
            output.push('\n');
        }
        Value::Object(object) => {
            for key in [
                "error",
                "message",
                "stdout",
                "stderr",
                "output",
                "status",
                "exit_code",
                "exitCode",
            ] {
                if let Some(value) = object.get(key) {
                    collect_output_text(value, depth + 1, output);
                }
            }
        }
        Value::Array(items) => {
            for item in items.iter().take(32) {
                collect_output_text(item, depth + 1, output);
            }
        }
        Value::Number(number) => {
            output.push_str(&number.to_string());
            output.push('\n');
        }
        Value::Null | Value::Bool(_) => {}
    }
}

fn safe_tool_output_diagnostic(output: &Value) -> Value {
    let output_type = match output {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    };
    let output_bytes = output
        .as_str()
        .map(str::len)
        .unwrap_or_else(|| output.to_string().len());
    let parsed_string = output
        .as_str()
        .and_then(|text| serde_json::from_str::<Value>(text).ok());
    let mut diagnostic_text = String::new();
    if let Some(parsed) = parsed_string.as_ref() {
        collect_output_text(parsed, 0, &mut diagnostic_text);
    } else {
        collect_output_text(output, 0, &mut diagnostic_text);
    }
    let normalized = diagnostic_text.to_ascii_lowercase();
    let error_category = if normalized.contains("0x80008085") || normalized.contains("-2147450747")
    {
        "hostfxr起動失敗"
    } else if normalized.contains("commandnotfoundexception")
        || (normalized.contains("powershell")
            && (normalized.contains("not recognized")
                || normalized.contains("not found")
                || normalized.contains("could not find")))
    {
        "shell解決失敗"
    } else if normalized.contains("unauthorizedaccessexception")
        || normalized.contains("access is denied")
        || normalized.contains("permission denied")
    {
        "sandboxアクセス拒否"
    } else if normalized.contains("timed out") || normalized.contains("timeout") {
        "実行期限超過"
    } else if normalized.contains("error") || normalized.contains("exception") {
        "その他エラー"
    } else if output.is_null() || output.as_str().is_some_and(str::is_empty) {
        "出力なし"
    } else {
        "error兆候なし"
    };
    json!({
        "output_type": output_type,
        "output_bytes": output_bytes,
        "error_category": error_category,
        "probe_started": diagnostic_text.contains("D4P_R2_SYNTHETIC_PROBE_STARTED"),
    })
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
            "data": [{"id": "o3", "object": "model", "created": 0, "owned_by": "local-test"}]
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
    let input_items = request
        .get("input")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut output_items = 0;
    let mut message_items = 0;
    let mut reasoning_items = 0;
    let mut other_items = 0;
    for item in input_items {
        match item.get("type").and_then(Value::as_str) {
            Some("function_call_output") => output_items += 1,
            Some("message") => message_items += 1,
            Some("reasoning") => reasoning_items += 1,
            _ => other_items += 1,
        }
    }
    let previous_response_id = request
        .get("previous_response_id")
        .is_some_and(Value::is_string);
    let has_exec_command = request
        .get("tools")
        .and_then(Value::as_array)
        .is_some_and(|tools| {
            tools
                .iter()
                .any(|tool| tool.get("name").and_then(Value::as_str) == Some("exec_command"))
        });
    state
        .request_shapes
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(format!(
            "要求{request_number}の構成: 入力項目数={}, 結果項目数={output_items}, 文章項目数={message_items}, 推論項目数={reasoning_items}, その他項目数={other_items}, 前回応答ID有無={previous_response_id}, コマンド実行有無={has_exec_command}",
            input_items.len()
        ));
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
    let active_tool_call_id = state
        .active_tool_call_id
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let output_items: Vec<&Value> = request
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("function_call_output"))
        .collect();
    state
        .tool_output_items_seen
        .fetch_add(output_items.len(), Ordering::SeqCst);
    let tool_result_received = active_tool_call_id
        .as_deref()
        .is_some_and(|expected_call_id| {
            let mut matched = false;
            for item in &output_items {
                match item.get("call_id").and_then(Value::as_str) {
                    Some(call_id) if call_id == expected_call_id => matched = true,
                    Some(_) => {
                        state
                            .tool_output_id_mismatches
                            .fetch_add(1, Ordering::SeqCst);
                    }
                    None => {
                        state
                            .tool_output_missing_call_id
                            .fetch_add(1, Ordering::SeqCst);
                    }
                }
            }
            matched
        });
    if tool_result_received {
        state.tool_result_received.store(true, Ordering::SeqCst);
        if let Some(expected_call_id) = active_tool_call_id.as_deref() {
            if let Some(item) = output_items
                .iter()
                .find(|item| item.get("call_id").and_then(Value::as_str) == Some(expected_call_id))
            {
                let mut diagnostics = state
                    .tool_output_diagnostics
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if diagnostics.is_empty() {
                    diagnostics.push(safe_tool_output_diagnostic(
                        item.get("output").unwrap_or(&Value::Null),
                    ));
                }
            }
        }
    }
    let response_id = format!("resp_d4p_broker_task_{request_number}");
    let mut events = vec![
        response_lifecycle_event("response.created", &response_id, "in_progress", json!([])),
        response_lifecycle_event(
            "response.in_progress",
            &response_id,
            "in_progress",
            json!([]),
        ),
    ];
    let tool_result_exists = Path::new(&state.workspace)
        .join("broker-real-codex-marker.txt")
        .is_file();
    let tool_was_offered = request
        .get("tools")
        .and_then(Value::as_array)
        .is_some_and(|tools| {
            tools
                .iter()
                .any(|tool| tool.get("name").and_then(Value::as_str) == Some("exec_command"))
        });
    if tool_result_exists && state.tool_result_received.load(Ordering::SeqCst) {
        *state
            .active_tool_call_id
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
        let message_id = format!("msg_d4p_broker_task_{request_number}");
        let text = "合成試験Taskが完了しました";
        let started_item = json!({
            "type":"message",
            "role":"assistant",
            "id":message_id,
            "status":"in_progress",
            "content":[]
        });
        let item = json!({
            "type":"message",
            "role":"assistant",
            "id":message_id,
            "status":"completed",
            "content":[{"type":"output_text","text":text,"annotations":[],"logprobs":[]}]
        });
        events.push(event(
            "response.output_item.added",
            json!({
                "type":"response.output_item.added",
                "response_id":response_id,
                "output_index":0,
                "item":started_item
            }),
        ));
        events.push(event(
            "response.content_part.added",
            json!({
                "type":"response.content_part.added",
                "response_id":response_id,
                "item_id":message_id,
                "output_index":0,
                "content_index":0,
                "part":{"type":"output_text","text":"","annotations":[],"logprobs":[]}
            }),
        ));
        events.push(event(
            "response.output_text.delta",
            json!({
                "type":"response.output_text.delta",
                "response_id":response_id,
                "item_id":message_id,
                "output_index":0,
                "content_index":0,
                "delta":text,
                "logprobs":[]
            }),
        ));
        events.push(event(
            "response.output_text.done",
            json!({
                "type":"response.output_text.done",
                "response_id":response_id,
                "item_id":message_id,
                "output_index":0,
                "content_index":0,
                "text":text,
                "logprobs":[]
            }),
        ));
        events.push(event(
            "response.content_part.done",
            json!({
                "type":"response.content_part.done",
                "response_id":response_id,
                "item_id":message_id,
                "output_index":0,
                "content_index":0,
                "part":{"type":"output_text","text":text,"annotations":[],"logprobs":[]}
            }),
        ));
        events.push(event(
            "response.output_item.done",
            json!({
                "type":"response.output_item.done",
                "response_id":response_id,
                "output_index":0,
                "item":item.clone()
            }),
        ));
        events.push(completed_response_event(&response_id, item));
    } else if active_tool_call_id.is_none() && tool_was_offered {
        state.tool_offered.store(true, Ordering::SeqCst);
        let call_number = state.tool_call_count.fetch_add(1, Ordering::SeqCst) + 1;
        let call_id = format!("call_d4p_broker_task_{call_number}");
        let item_id = format!("fc_d4p_broker_task_{call_number}");
        *state
            .active_tool_call_id
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(call_id.clone());
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
        let started_item = json!({
            "type":"function_call",
            "id":item_id,
            "call_id":call_id,
            "name":"exec_command",
            "status":"in_progress",
            "arguments":""
        });
        let item = json!({
            "type":"function_call",
            "id":item_id,
            "call_id":call_id,
            "name":"exec_command",
            "status":"completed",
            "arguments":arguments.to_string()
        });
        events.push(event(
            "response.output_item.added",
            json!({
                "type":"response.output_item.added",
                "response_id":response_id,
                "output_index":0,
                "item":started_item
            }),
        ));
        events.push(event(
            "response.function_call_arguments.delta",
            json!({
                "type":"response.function_call_arguments.delta",
                "response_id":response_id,
                "item_id":item_id,
                "output_index":0,
                "delta":arguments.to_string()
            }),
        ));
        events.push(event(
            "response.function_call_arguments.done",
            json!({
                "type":"response.function_call_arguments.done",
                "response_id":response_id,
                "item_id":item_id,
                "output_index":0,
                "arguments":arguments.to_string()
            }),
        ));
        events.push(event(
            "response.output_item.done",
            json!({
                "type":"response.output_item.done",
                "response_id":response_id,
                "output_index":0,
                "item":item.clone()
            }),
        ));
        state.tool_call_sent.store(true, Ordering::SeqCst);
        events.push(completed_response_event(&response_id, item));
    } else {
        state
            .repeated_tool_call_rejections
            .fetch_add(1, Ordering::SeqCst);
        if respond(
            &mut stream,
            409,
            "text/plain",
            b"synthetic tool result missing expected Workspace marker",
        )
        .is_err()
        {
            state.response_write_failures.fetch_add(1, Ordering::SeqCst);
        }
        return;
    }
    for (sequence_number, event) in events.iter_mut().enumerate() {
        event
            .as_object_mut()
            .expect("Responses eventはobject")
            .insert("sequence_number".to_owned(), json!(sequence_number));
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn post(server: &CodexLoopbackResponses, request: &Value) -> Vec<u8> {
        let port = server.port();
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("loopback偽APIへ接続");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("応答期限を設定");
        let body = request.to_string();
        write!(
            stream,
            "POST /v1/responses HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .expect("合成Responses要求を送信");
        let mut response = Vec::new();
        let mut chunk = [0; 4096];
        loop {
            let size = match stream.read(&mut chunk) {
                Ok(size) => size,
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {
                    let header_end = response
                        .windows(4)
                        .position(|part| part == b"\r\n\r\n")
                        .expect("ConnectionReset前にHTTP応答headerが届く");
                    let header = String::from_utf8_lossy(&response[..header_end]);
                    let content_length = header
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .expect("HTTP応答にContent-Lengthがある");
                    assert!(
                        response.len() >= header_end + 4 + content_length,
                        "ConnectionResetでHTTP応答が途中切断: status={:?}; received={}; expected={}",
                        header.lines().next(),
                        response.len().saturating_sub(header_end + 4),
                        content_length
                    );
                    return response;
                }
                Err(error) => panic!(
                    "合成Responses応答を読む: {error}; accepted={}; incomplete=({}); posts={}; response_writes=({})",
                    server.accepted_connections(),
                    server.incomplete_request_summary(),
                    server.post_requests(),
                    server.response_write_summary(),
                ),
            };
            if size == 0 {
                return response;
            }
            response.extend_from_slice(&chunk[..size]);
            let Some(header_end) = response.windows(4).position(|part| part == b"\r\n\r\n") else {
                continue;
            };
            let header = String::from_utf8_lossy(&response[..header_end]);
            let content_length = header.lines().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            });
            if let Some(content_length) = content_length {
                if response.len() >= header_end + 4 + content_length {
                    return response;
                }
            }
        }
    }

    #[test]
    fn failed_tool_result_is_not_replayed_as_another_exec_command() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("時刻を取得")
            .as_nanos();
        let workspace = std::env::temp_dir().join(format!(
            "gui-shell-r2-fake-api-repeat-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir(&workspace).expect("専用Workspaceを作成");
        let server = CodexLoopbackResponses::start(&workspace).expect("fake APIを開始");

        let first = post(
            &server,
            &json!({
                "tools":[{"type":"function","name":"exec_command"}],
                "input":[]
            }),
        );
        let first = String::from_utf8(first).expect("HTTP応答はUTF-8");
        assert!(
            first.starts_with("HTTP/1.0 200 OK"),
            "{first}; accepted={}; incomplete=({}); posts={}; invalid={}",
            server.accepted_connections(),
            server.incomplete_request_summary(),
            server.post_requests(),
            server.invalid_post_bodies()
        );
        assert!(first.contains("function_call"), "{first}");

        let second = post(
            &server,
            &json!({
                "input":[{
                    "type":"function_call_output",
                    "call_id":"call_d4p_broker_task_1",
                    "output":"synthetic command failed"
                }]
            }),
        );
        let second = String::from_utf8(second).expect("HTTP応答はUTF-8");
        assert!(second.starts_with("HTTP/1.0 409 Conflict"), "{second}");
        assert!(
            !second.contains("function_call"),
            "同じtoolを再送しない: {second}"
        );
        assert!(server.tool_call_was_sent());
        assert!(server.tool_result_was_received());
        assert_eq!(server.repeated_tool_call_rejections(), 1);
        assert_eq!(server.post_requests(), 2);

        drop(server);
        std::fs::remove_dir_all(workspace).expect("test専用Workspaceを削除");
    }

    #[test]
    fn tool_output_diagnostic_records_only_bounded_safe_classification() {
        let private_output = "D4P_R2_SYNTHETIC_PROBE_STARTED UnauthorizedAccessException synthetic-secret-content C:\\Users\\example\\private.txt";
        let diagnostic = safe_tool_output_diagnostic(&json!(private_output));
        assert_eq!(diagnostic["error_category"], "sandboxアクセス拒否");
        assert_eq!(diagnostic["probe_started"], true);
        assert_eq!(diagnostic["output_type"], "string");
        assert!(!diagnostic.to_string().contains("synthetic-secret-content"));
        assert!(!diagnostic.to_string().contains("C:\\\\Users"));
        assert!(!diagnostic.to_string().contains("private.txt"));

        let launch_failure = safe_tool_output_diagnostic(&json!({
            "stderr": "PowerShell host failed: 0x80008085",
            "exit_code": -2147450747,
        }));
        assert_eq!(launch_failure["error_category"], "hostfxr起動失敗");
        assert!(!launch_failure
            .to_string()
            .contains("PowerShell host failed"));
    }

    #[test]
    fn temp_probe_keeps_boundary_checks_independent_from_temp_diagnostic() {
        let probe = synthetic_workspace_probe();
        assert!(probe.contains("temp_step=$tempStep"));
        assert!(probe.contains("$tempStep='temp_write'"));
        assert!(!probe.contains("exit 45"));
        assert!(probe.contains("if (-not $secretDenied) { exit 41 }"));
        assert!(probe.contains("if (-not $outsideReadDenied) { exit 42 }"));
        assert!(probe.contains("if (-not $outsideWriteDenied) { exit 43 }"));
        assert!(probe.contains("broker-real-codex-marker.txt"));
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

fn completed_response_event(response_id: &str, output_item: Value) -> Value {
    response_lifecycle_event(
        "response.completed",
        response_id,
        "completed",
        json!([output_item]),
    )
}

fn response_lifecycle_event(
    event_type: &str,
    response_id: &str,
    status: &str,
    output: Value,
) -> Value {
    let completed = status == "completed";
    let completed_at = if completed { json!(1) } else { Value::Null };
    let usage = if completed {
        json!({
            "input_tokens":1,
            "input_tokens_details":{"cached_tokens":0,"cache_write_tokens":0},
            "output_tokens":1,
            "output_tokens_details":{"reasoning_tokens":0},
            "total_tokens":2
        })
    } else {
        Value::Null
    };
    json!({
        "type":event_type,
        "response":{
            "id":response_id,
            "object":"response",
            "access_programs":null,
            "created_at":0,
            "status":status,
            "completed_at":completed_at,
            "background":false,
            "error":null,
            "incomplete_details":null,
            "instructions":null,
            "max_output_tokens":null,
            "max_tool_calls":null,
            "model":"o3",
            "output":output,
            "parallel_tool_calls":true,
            "previous_response_id":null,
            "reasoning":{"effort":null,"summary":null},
            "service_tier":"default",
            "store":true,
            "temperature":1.0,
            "text":{"format":{"type":"text"}},
            "tool_choice":"auto",
            "tools":[],
            "top_p":1.0,
            "truncation":"disabled",
            "usage":usage,
            "user":null,
            "metadata":{}
        }
    })
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
        409 => "Conflict",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let headers = format!(
        "HTTP/1.0 {status} {reason}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    let _ = stream.shutdown(Shutdown::Write);
    let mut trailing = [0; 1024];
    loop {
        match stream.read(&mut trailing) {
            Ok(0) => break,
            Ok(_) => continue,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                ) =>
            {
                break;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
