#![cfg(all(windows, feature = "r2-e2e"))]

use gui_shell_rust_helper::audit_hash::sha256_tagged;
use gui_shell_rust_helper::broker::{
    BrokerCredentialRole, BrokerEndpoint, BrokerPersistentStore, BrokerRequestEnvelope,
};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use winsafe::{co, HPROCESSLIST};

const RUNTIME_ID: &str = "r2-crash-codex";
const WORKSPACE_ID: &str = "r2-crash-workspace";
const TASK_INSTRUCTION: &str = "D4P_CRASH_TASK_SENTINEL: perform only the bounded synthetic probe supplied by the local test API";
const BROKER_EXE: &str = env!("CARGO_BIN_EXE_gui_shell_rust_helper");
const RESPONSES_EXE: &str = env!("CARGO_BIN_EXE_gui_shell_r2_e2e_responses");
const R2_E2E_TASK_EXECUTION_LIMIT_ENV: &str = "GUI_SHELL_R2_E2E_TASK_EXECUTION_LIMIT_MS";

fn setup_test_codex_home(codex_cli: &Path, codex_home: &Path) {
    let status = Command::new(codex_cli)
        .args([
            "sandbox",
            "setup",
            "--elevated",
            "--current-user",
            "--codex-home",
        ])
        .arg(codex_home)
        .current_dir(codex_home)
        .env("CODEX_HOME", codex_home)
        .env_remove("OPENAI_API_KEY")
        .env_remove("CODEX_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("GEMINI_API_KEY")
        .env_remove("GOOGLE_API_KEY")
        .env_remove("AZURE_OPENAI_API_KEY")
        .env_remove("GH_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .env_remove("ACTIONS_RUNTIME_TOKEN")
        .env_remove("ACTIONS_ID_TOKEN_REQUEST_TOKEN")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("test専用Codex HomeのWindows sandbox初期化processを起動");
    assert!(
        status.success(),
        "test専用Codex Homeの公式Windows sandbox初期化に失敗: exit={:?}; production Owner UIやsandbox緩和で代替しない",
        status.code()
    );
}

struct Fixture {
    root: PathBuf,
    store: PathBuf,
    workspace: PathBuf,
    workspace_config: PathBuf,
    codex: PathBuf,
    broker_exe: PathBuf,
    responses_port: u16,
    broker: Option<Child>,
    normal: Option<BrokerEndpoint>,
    owner: Option<BrokerEndpoint>,
    responses: Option<Child>,
    responses_stdin: Option<ChildStdin>,
    responses_stdout: Option<BufReader<ChildStdout>>,
    generation: u32,
    task_execution_limit_ms: Option<u64>,
    expected_responses_shape: &'static str,
}

impl Fixture {
    fn new(codex: PathBuf) -> Self {
        Self::new_with_settings(codex, None, "--expect-cancellation")
    }

    fn new_success(codex: PathBuf) -> Self {
        Self::new_with_settings(codex, None, "")
    }

    fn with_deadline(codex: PathBuf, milliseconds: u64) -> Self {
        Self::new_with_settings(codex, Some(milliseconds), "--expect-deadline")
    }

    fn with_provider_failure(codex: PathBuf) -> Self {
        Self::new_with_settings(codex, None, "--expect-provider-failure")
    }

    fn new_with_settings(
        codex: PathBuf,
        task_execution_limit_ms: Option<u64>,
        expected_responses_shape: &'static str,
    ) -> Self {
        Self::new_with_settings_and_export_runtime(
            codex,
            task_execution_limit_ms,
            expected_responses_shape,
            None,
        )
    }

    fn new_success_with_export_runtime(
        codex: PathBuf,
        broker_exe: PathBuf,
        app_id: String,
        audit_store_id: String,
    ) -> Self {
        Self::new_with_settings_and_export_runtime(
            codex,
            None,
            "",
            Some((broker_exe, app_id, audit_store_id)),
        )
    }

    fn new_with_settings_and_export_runtime(
        codex: PathBuf,
        task_execution_limit_ms: Option<u64>,
        expected_responses_shape: &'static str,
        export_runtime: Option<(PathBuf, String, String)>,
    ) -> Self {
        assert!(codex.is_absolute() && codex.is_file(), "Codex CLIの実file");
        assert!(
            !codex.to_string_lossy().contains('='),
            "Codex CLI pathにruntime引数区切り文字がない"
        );
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("時計")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "gui-shell-r2-crash-e2e-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("専用E2E root");
        let workspace = root.join("registered-workspace");
        fs::create_dir_all(workspace.join("private")).expect("合成Workspaceとsecret directory");
        fs::write(
            workspace.join("private/fixture-secret.txt"),
            b"synthetic-secret-content-never-returned",
        )
        .expect("合成secret marker");
        fs::write(
            root.join("outside-read-marker.txt"),
            b"synthetic outside marker",
        )
        .expect("Workspace外read marker");
        assert!(!root.join("outside-write-marker.txt").exists());
        let workspace = fs::canonicalize(workspace).expect("Workspace path正準化");
        let workspace_config = root.join("workspace.json");
        fs::write(
            &workspace_config,
            json!({
                "version": 1,
                "workspaces": [{
                    "runtime_id": RUNTIME_ID,
                    "workspace_id": WORKSPACE_ID,
                    "root_path": workspace.to_string_lossy(),
                    "secret_paths": ["private/fixture-secret.txt"]
                }]
            })
            .to_string(),
        )
        .expect("Workspace起動設定");
        let (store, broker_exe) = match export_runtime {
            Some((helper, app_id, audit_store_id)) => {
                assert!(helper.is_absolute(), "Export Broker helperは絶対path");
                assert!(
                    app_id.strip_prefix("d4-pocket-app-").is_some_and(|suffix| {
                        suffix.len() == 32
                            && suffix
                                .bytes()
                                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                    }),
                    "Export App ID形式"
                );
                assert!(
                    audit_store_id
                        .strip_prefix("audit-store-")
                        .is_some_and(|suffix| {
                            suffix.len() == 32
                                && suffix.bytes().all(|byte| {
                                    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
                                })
                        }),
                    "Export Audit Store ID形式"
                );
                let store = root
                    .join("app-data")
                    .join("apps")
                    .join(app_id)
                    .join("stores")
                    .join(audit_store_id);
                (store, helper)
            }
            None => (root.join("store"), PathBuf::from(BROKER_EXE)),
        };
        let broker_exe = broker_exe.canonicalize().expect("Broker helper完全path");
        assert!(broker_exe.is_file(), "Broker helper実file");
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("一時loopback port");
        let responses_port = listener.local_addr().expect("一時port取得").port();
        drop(listener);
        let mut fixture = Self {
            root: root.clone(),
            store,
            workspace,
            workspace_config,
            codex,
            broker_exe,
            responses_port,
            broker: None,
            normal: None,
            owner: None,
            responses: None,
            responses_stdin: None,
            responses_stdout: None,
            generation: 0,
            task_execution_limit_ms,
            expected_responses_shape,
        };
        fixture.start_responses();
        fixture.start_broker();
        fixture
    }

    fn start_responses(&mut self) {
        let mut command = Command::new(RESPONSES_EXE);
        command
            .arg(&self.workspace)
            .arg(self.responses_port.to_string());
        if !self.expected_responses_shape.is_empty() {
            command.arg(self.expected_responses_shape);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("資格情報なしloopback Responses fixture起動");
        let stdin = child.stdin.take().expect("応答器の標準入力を取得");
        let stdout = child.stdout.take().expect("応答器の標準出力を取得");
        let mut stdout = BufReader::new(stdout);
        let mut ready = String::new();
        stdout
            .read_line(&mut ready)
            .expect("応答器の起動通知を読む");
        assert_eq!(
            ready.trim(),
            format!("READY {}", self.responses_port),
            "loopback fixture readiness"
        );
        self.responses = Some(child);
        self.responses_stdin = Some(stdin);
        self.responses_stdout = Some(stdout);
    }

    fn start_broker(&mut self) {
        let (child, session_file, owner_file) = self.spawn_broker_process(Stdio::null());
        self.broker = Some(child);
        self.normal = None;
        self.owner = None;
        let normal = self.wait_for_endpoint(&session_file);
        let owner = self.wait_for_endpoint(&owner_file);
        assert_ne!(normal.session_secret, owner.session_secret);
        self.normal = Some(normal);
        self.owner = Some(owner);
    }

    fn spawn_broker_process(&mut self, stderr: Stdio) -> (Child, PathBuf, PathBuf) {
        self.generation += 1;
        let generation_root = self
            .root
            .join(format!("broker-generation-{}", self.generation))
            .join("D4Pocket-R2-E2E-SYNTHETIC");
        fs::create_dir_all(&generation_root).expect("R2 test build専用root");
        fs::write(
            generation_root.join("OWNER-APPROVED-SYNTHETIC.txt"),
            b"synthetic owner fixture; not an authority source",
        )
        .expect("合成試験用基点目印を書き込む");
        let codex_home = generation_root.join("codex-home");
        fs::create_dir(&codex_home).expect("世代ごとの空CODEX_HOME");
        setup_test_codex_home(&self.codex, &codex_home);

        let runtime = format!(
            "{RUNTIME_ID}={}={}",
            self.codex.display(),
            self.workspace.display()
        );
        let suffix = self.generation;
        let session_file = self.root.join(format!("normal-{suffix}.json"));
        let owner_file = self.root.join(format!("owner-{suffix}.json"));
        let mut command = Command::new(&self.broker_exe);
        command
            .args(["broker-server", "--store-dir"])
            .arg(&self.store)
            .arg("--session-file")
            .arg(&session_file)
            .arg("--owner-session-file")
            .arg(&owner_file)
            .arg("--workspace-config")
            .arg(&self.workspace_config)
            .arg("--codex-runtime")
            .arg(runtime)
            .arg("--r2-e2e-synthetic-owner-confirmation")
            .current_dir(&self.root)
            .env("CODEX_HOME", &codex_home)
            .env("GUI_SHELL_R2_E2E_CODEX_HOME", &codex_home)
            .env(
                "GUI_SHELL_R2_E2E_RESPONSES_PORT",
                self.responses_port.to_string(),
            )
            .env_remove(R2_E2E_TASK_EXECUTION_LIMIT_ENV)
            .env_remove("OPENAI_API_KEY")
            .env_remove("CODEX_API_KEY")
            .env_remove("ANTHROPIC_API_KEY")
            .stdout(Stdio::null())
            .stderr(stderr);
        if let Some(milliseconds) = self.task_execution_limit_ms {
            command.env(R2_E2E_TASK_EXECUTION_LIMIT_ENV, milliseconds.to_string());
        }
        let child = command.spawn().expect("r2-e2e Broker process起動");
        (child, session_file, owner_file)
    }

    fn start_broker_expect_failure(&mut self, diagnostic_fragment: &str) {
        let (mut child, session_file, owner_file) = self.spawn_broker_process(Stdio::piped());
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if child
                .try_wait()
                .expect("破損journal起動Broker状態")
                .is_some()
            {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let output = child
                    .wait_with_output()
                    .expect("期限超過Broker process回収");
                panic!(
                    "破損journalでBrokerが期限内にfail-closedしない: status={:?}; stderr={}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            thread::sleep(Duration::from_millis(25));
        }
        let output = child
            .wait_with_output()
            .expect("fail-closed Broker process回収");
        assert!(!output.status.success(), "破損journalでBroker起動を拒否");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(diagnostic_fragment),
            "journal認証failureを報告: {stderr}"
        );
        assert!(!session_file.exists(), "通常IPC endpointを生成しない");
        assert!(!owner_file.exists(), "Owner IPC endpointを生成しない");
        self.broker = None;
        self.normal = None;
        self.owner = None;
    }

    fn wait_for_endpoint(&mut self, path: &Path) -> BrokerEndpoint {
        let deadline = Instant::now() + Duration::from_secs(45);
        loop {
            if let Ok(bytes) = fs::read(path) {
                if let Ok(endpoint) = serde_json::from_slice::<BrokerEndpoint>(&bytes) {
                    return endpoint;
                }
            }
            if let Some(child) = self.broker.as_mut() {
                assert!(
                    child.try_wait().expect("Broker process状態").is_none(),
                    "Brokerがendpoint生成前に終了"
                );
            }
            assert!(Instant::now() < deadline, "Broker endpoint生成期限超過");
            thread::sleep(Duration::from_millis(25));
        }
    }

    fn normal(&self) -> &BrokerEndpoint {
        self.normal.as_ref().expect("通常IPC endpoint")
    }

    fn owner(&self) -> &BrokerEndpoint {
        self.owner.as_ref().expect("所有者確認用IPC接続先")
    }

    fn kill_broker(&mut self) {
        let mut child = self.broker.take().expect("仲介処理系processを取得");
        child.kill().expect("Brokerを強制終了");
        assert!(!child.wait().expect("Broker process回収").success());
        self.normal = None;
        self.owner = None;
    }

    fn stop_broker(&mut self) {
        let endpoint = self.normal().clone();
        let response = send_request(&endpoint, "shutdown", Value::Null);
        assert_eq!(response["status"], "accepted", "{response}");
        assert!(self
            .broker
            .take()
            .expect("仲介処理系の終了を待つ")
            .wait()
            .expect("Broker正常終了待ち")
            .success());
        self.normal = None;
        self.owner = None;
    }

    fn collect_responses(&mut self) -> (Value, bool, Option<i32>) {
        let mut stdin = self.responses_stdin.take().expect("応答器の標準入力を取得");
        writeln!(stdin, "stop").expect("fixture停止要求");
        stdin.flush().expect("fixture停止要求flush");
        drop(stdin);
        let mut stdout = self
            .responses_stdout
            .take()
            .expect("応答器の標準出力を取得");
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut evidence: Option<Value> = None;
        loop {
            let mut line = String::new();
            assert!(Instant::now() < deadline, "loopback fixture終了期限");
            let read = stdout.read_line(&mut line).expect("fixture evidence読取");
            if read == 0 {
                break;
            }
            if let Some(json) = line.strip_prefix("EVIDENCE ") {
                evidence = Some(serde_json::from_str(json).expect("応答器の検証記録JSONを解析"));
            }
        }
        let evidence = evidence.expect("応答器の検証記録");
        let status = self
            .responses
            .take()
            .expect("応答器processを取得")
            .wait()
            .expect("fixture終了待ち");
        (evidence, status.success(), status.code())
    }

    fn finish_responses(&mut self) -> Value {
        let (evidence, success, exit_code) = self.collect_responses();
        assert!(
            success,
            "loopback fixture終了code={exit_code:?}; expected_shape={}; requests={}; tool_offered={}; tool_call_sent={}; tool_result_received={}; repeated_rejections={}; invalid_bodies={}; write_failures={}; incomplete={}; blocked_external={}; workspace_marker={}; boundary_valid={}",
            evidence["expected_request_shape"],
            evidence["requests"],
            evidence["tool_offered"],
            evidence["tool_call_sent"],
            evidence["tool_result_received"],
            evidence["repeated_tool_call_rejections"],
            evidence["invalid_bodies"],
            evidence["response_write_failures"],
            evidence["incomplete_requests"],
            evidence["blocked_external_requests"],
            evidence["workspace_marker_exists"],
            evidence["workspace_boundary_fixtures_valid_after_task"],
        );
        evidence
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(mut child) = self.broker.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        if let Some(mut child) = self.responses.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn send_request(endpoint: &BrokerEndpoint, operation: &str, payload: Value) -> Value {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let request_id = format!(
        "r2-crash-e2e-{}",
        NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let payload_hash = sha256_tagged(payload.to_string().as_bytes());
    let request = json!({
        "request_id": request_id,
        "session_id": endpoint.session_id,
        "operation": operation,
        "payload_hash": payload_hash,
        "nonce": format!("{request_id}-nonce"),
        "issued_at": BrokerRequestEnvelope::current_issued_at(),
        "metadata": {"client": if endpoint.credential_role == BrokerCredentialRole::Owner {
            "r2_e2e_synthetic_owner"
        } else {
            "r2_active_task_crash_recovery_e2e"
        }},
        "payload": payload,
    });
    let mut stream = TcpStream::connect((endpoint.host.as_str(), endpoint.port))
        .expect("認証済みloopback IPC接続");
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .expect("IPC応答期限");
    write!(stream, "{}\n{}\n", endpoint.session_secret, request)
        .expect("認証資格とIPC request送信");
    let mut response = String::new();
    BufReader::new(stream)
        .read_line(&mut response)
        .expect("IPC応答読取");
    serde_json::from_str(&response).expect("IPC応答JSON")
}

fn send_same_request_twice(
    endpoint: &BrokerEndpoint,
    operation: &str,
    payload: Value,
) -> (Value, Value, String) {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let request_id = format!(
        "r2-crash-e2e-replay-{}",
        NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let payload_hash = sha256_tagged(payload.to_string().as_bytes());
    let request = json!({
        "request_id": request_id,
        "session_id": endpoint.session_id,
        "operation": operation,
        "payload_hash": payload_hash,
        "nonce": format!("{request_id}-nonce"),
        "issued_at": BrokerRequestEnvelope::current_issued_at(),
        "metadata": {"client": "r2_active_task_crash_recovery_e2e"},
        "payload": payload,
    })
    .to_string();
    let send = || {
        let mut stream = TcpStream::connect((endpoint.host.as_str(), endpoint.port))
            .expect("認証済みloopback IPC再送接続");
        stream
            .set_read_timeout(Some(Duration::from_secs(15)))
            .expect("IPC再送応答期限");
        write!(stream, "{}\n{}\n", endpoint.session_secret, request)
            .expect("同一IPC request envelope再送");
        let mut response = String::new();
        BufReader::new(stream)
            .read_line(&mut response)
            .expect("IPC再送応答読取");
        serde_json::from_str(&response).expect("IPC再送応答JSON")
    };
    (send(), send(), request_id)
}

fn only_scratch(workspace: &Path) -> Option<PathBuf> {
    let paths = scratch_directories(workspace).ok()?;
    (paths.len() == 1).then(|| paths[0].clone())
}

fn scratch_directories(workspace: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(workspace)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && entry.file_name().to_string_lossy().starts_with(".d4p-tmp-")
        {
            paths.push(entry.path());
        }
    }
    Ok(paths)
}

fn tamper_scratch_journal_state(path: &Path) -> Vec<u8> {
    let before = fs::read(path).expect("active Task scratch journalを読む");
    let mut envelope: Value = serde_json::from_slice(&before).expect("署名付きjournal JSON");
    let original_hmac = envelope["hmac"]
        .as_str()
        .expect("journal HMAC項目")
        .to_owned();
    let entries = envelope["state"]["entries"]
        .as_array_mut()
        .expect("稼働中scratch journal項目一覧");
    assert_eq!(entries.len(), 1, "active Taskのscratch journal entryは一件");
    entries[0]["task_id"] = json!("tampered-task-scope");
    assert_eq!(
        envelope["hmac"].as_str(),
        Some(original_hmac.as_str()),
        "改変では元のHMACを再生成しない"
    );
    let tampered = serde_json::to_vec(&envelope).expect("改変journalをserialize");
    assert_ne!(before, tampered, "journal stateを実際に改変");
    fs::write(path, &tampered).expect("HMAC不一致journalを保存");
    tampered
}

fn process_ids() -> HashSet<u32> {
    let mut snapshot = HPROCESSLIST::CreateToolhelp32Snapshot(co::TH32CS::SNAPPROCESS, None)
        .expect("process一覧snapshot");
    snapshot
        .iter_processes()
        .map(|entry| entry.expect("process一覧entry").th32ProcessID)
        .collect()
}

fn process_tree_ids(root_pid: u32) -> HashSet<u32> {
    let mut snapshot = HPROCESSLIST::CreateToolhelp32Snapshot(co::TH32CS::SNAPPROCESS, None)
        .expect("子孫プロセス一覧のスナップショットを作成");
    let parent_by_pid = snapshot
        .iter_processes()
        .map(|entry| {
            let entry = entry.expect("子孫プロセス一覧の要素を取得");
            (entry.th32ProcessID, entry.th32ParentProcessID)
        })
        .collect::<Vec<_>>();
    let mut descendants = HashSet::new();
    descendants.insert(root_pid);
    loop {
        let previous_len = descendants.len();
        for (pid, parent_pid) in &parent_by_pid {
            if descendants.contains(parent_pid) {
                descendants.insert(*pid);
            }
        }
        if descendants.len() == previous_len {
            return descendants;
        }
    }
}

fn wait_for_process_tree_exit(processes: &HashSet<u32>, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        let live = process_ids();
        let survivors = processes.intersection(&live).copied().collect::<Vec<_>>();
        if survivors.is_empty() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "AgentTask終端後にprocess treeが残る: {survivors:?}"
        );
        thread::sleep(Duration::from_millis(100));
    }
}

struct ActiveTaskObservation {
    heartbeat: PathBuf,
    scratch: PathBuf,
    process_tree: HashSet<u32>,
}

fn wait_for_active_codex_task(
    fixture: &Fixture,
    normal: &BrokerEndpoint,
    task_id: &str,
    broker_pid: u32,
    codex: &Path,
    processes_before_task: &HashSet<u32>,
    activation_timeout: Duration,
) -> ActiveTaskObservation {
    let deadline = Instant::now() + activation_timeout;
    let heartbeat = fixture.workspace.join("broker-real-codex-heartbeat.txt");
    let report_path = fixture.workspace.join("broker-real-codex-temp-report.json");
    loop {
        if let (Some(scratch), Ok(metadata)) =
            (only_scratch(&fixture.workspace), fs::metadata(&heartbeat))
        {
            let report = fs::read(&report_path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
            if report
                .as_ref()
                .is_some_and(|value| value["stage"] == "temp_checked")
            {
                let roots = newly_spawned_codex_root(broker_pid, codex, processes_before_task);
                if !roots.is_empty() {
                    assert_eq!(roots.len(), 1, "今回のBrokerが起動したCodex rootは一意");
                    let process_tree = process_tree_ids(roots[0]);
                    if process_tree.len() >= 2 {
                        let before = metadata.len();
                        thread::sleep(Duration::from_millis(250));
                        let heartbeat_length = fs::metadata(&heartbeat)
                            .expect("稼働中MxCの継続記録を取得")
                            .len();
                        assert!(
                            heartbeat_length > before,
                            "中断要求前に実MxC child heartbeatが継続"
                        );
                        let state =
                            send_request(normal, "AgentTask状態", json!({"task_id":task_id}));
                        assert_eq!(state["body"]["status"], "running", "{state}");
                        return ActiveTaskObservation {
                            heartbeat,
                            scratch,
                            process_tree,
                        };
                    }
                }
            }
        }
        assert!(
            Instant::now() < deadline,
            "active Codex/MxC Task検出期限超過"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

const E2E_DEADLINE_MS: u64 = 20_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActiveTaskInterruption {
    OwnerCancellation,
    Deadline,
    ProviderFailure,
}

fn run_active_task_interruption_e2e(codex: PathBuf, interruption: ActiveTaskInterruption) {
    let deadline = interruption == ActiveTaskInterruption::Deadline;
    let cancellation = interruption == ActiveTaskInterruption::OwnerCancellation;
    let mut fixture = match interruption {
        ActiveTaskInterruption::OwnerCancellation => Fixture::new(codex.clone()),
        ActiveTaskInterruption::Deadline => Fixture::with_deadline(codex.clone(), E2E_DEADLINE_MS),
        ActiveTaskInterruption::ProviderFailure => Fixture::with_provider_failure(codex.clone()),
    };
    let normal = fixture.normal().clone();
    let session = send_request(
        &normal,
        "対話開始",
        json!({"実行系ID":RUNTIME_ID,"作業領域ID":WORKSPACE_ID}),
    );
    assert_eq!(session["status"], "accepted", "{session}");
    let session_id = session["body"]["対話セッションID"]
        .as_str()
        .expect("Agentセッション識別子")
        .to_owned();
    let task = json!({
        "agent_runtime_id": RUNTIME_ID,
        "session_id": session_id,
        "workspace_id": WORKSPACE_ID,
        "instruction": TASK_INSTRUCTION,
    });
    let permission = json!({
        "agent_runtime_id": RUNTIME_ID,
        "session_id": session_id,
        "workspace_id": WORKSPACE_ID,
    });
    let granted = send_request(
        fixture.owner(),
        "AgentTaskWorkspacePermissionGrant",
        permission.clone(),
    );
    assert_eq!(granted["status"], "accepted", "{granted}");
    let approved = send_request(fixture.owner(), "AgentTaskOwnerApprovalGrant", task.clone());
    assert_eq!(approved["status"], "accepted", "{approved}");

    let processes_before_task = process_ids();
    let broker_pid = fixture.broker.as_ref().expect("Brokerプロセス").id();
    let (started, replayed_start, replay_request_id) =
        send_same_request_twice(&normal, "AgentTask実行", task.clone());
    assert_eq!(started["status"], "accepted", "{started}");
    assert_eq!(started["body"]["status"], "running", "{started}");
    assert_eq!(replayed_start["status"], "rejected", "{replayed_start}");
    assert_eq!(
        replayed_start["error"]["code"], "broker_replay_detected",
        "同じrequest ID／nonceのAgentTask再送を拒否: {replayed_start}"
    );
    let task_id = started["body"]["task_id"]
        .as_str()
        .expect("AgentTask識別子")
        .to_owned();
    let active = wait_for_active_codex_task(
        &fixture,
        &normal,
        &task_id,
        broker_pid,
        &codex,
        &processes_before_task,
        Duration::from_millis(if deadline {
            E2E_DEADLINE_MS - 5_000
        } else {
            50_000
        }),
    );

    if cancellation {
        let cancellation = send_request(&normal, "AgentTask取消", json!({"task_id":task_id}));
        assert_eq!(cancellation["status"], "accepted", "{cancellation}");
        assert_eq!(
            cancellation["body"]["status"], "running",
            "要求受理だけではprocess群の終了前にterminal化しない"
        );
    }

    let (expected_status, expected_failure_class) = match interruption {
        ActiveTaskInterruption::OwnerCancellation => ("cancelled", "取消"),
        ActiveTaskInterruption::Deadline => ("failed", "期限超過"),
        ActiveTaskInterruption::ProviderFailure => ("failed", "通信失敗"),
    };
    let terminal_deadline = Instant::now()
        + Duration::from_millis(if deadline {
            E2E_DEADLINE_MS + 30_000
        } else {
            30_000
        });
    let terminal = loop {
        let state = send_request(&normal, "AgentTask状態", json!({"task_id":task_id}));
        if state["body"]["status"] != "running" {
            break state;
        }
        assert!(
            Instant::now() < terminal_deadline,
            "AgentTask terminal期限超過"
        );
        thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(terminal["body"]["status"], expected_status, "{terminal}");
    assert!(terminal["body"].get("result_hash").is_none());
    wait_for_process_tree_exit(&active.process_tree, Duration::from_secs(20));
    let stopped_heartbeat_length = fs::metadata(&active.heartbeat)
        .expect("process tree終了後のMxC heartbeat")
        .len();
    thread::sleep(Duration::from_millis(350));
    assert_eq!(
        fs::metadata(&active.heartbeat)
            .expect("terminal後のMxC heartbeat")
            .len(),
        stopped_heartbeat_length,
        "AgentTask terminal後に実MxC child heartbeatが停止"
    );
    assert!(
        !active.scratch.exists(),
        "terminal後にWorkspaceTaskScratchを回収"
    );
    assert!(
        scratch_directories(&fixture.workspace)
            .expect("Workspace scratch一覧")
            .is_empty(),
        "terminal後にBroker-owned scratch directoryが残らない"
    );
    assert!(
        !fixture
            .workspace
            .join("broker-real-codex-marker.txt")
            .exists(),
        "中断Taskは完了markerを書かない"
    );

    let renewed_permission = send_request(
        fixture.owner(),
        "AgentTaskWorkspacePermissionGrant",
        permission,
    );
    assert_eq!(
        renewed_permission["status"], "accepted",
        "{renewed_permission}"
    );
    let replay = send_request(&normal, "AgentTask実行", task);
    assert_eq!(
        replay["status"], "rejected",
        "新しいWorkspace Permissionだけでは中断Taskの消費済みApprovalを再利用しない: {replay}"
    );

    fixture.stop_broker();
    let evidence = fixture.finish_responses();
    let expected_request_shape = if interruption == ActiveTaskInterruption::ProviderFailure {
        "tool_result_provider_failure"
    } else {
        "tool_call_without_result"
    };
    assert_eq!(evidence["expected_request_shape"], expected_request_shape);
    assert_eq!(evidence["tool_call_sent"], true);
    assert_eq!(
        evidence["tool_result_received"],
        interruption == ActiveTaskInterruption::ProviderFailure
    );
    assert_eq!(
        evidence["provider_failure_responses"]
            .as_u64()
            .unwrap_or_default()
            >= 1,
        interruption == ActiveTaskInterruption::ProviderFailure
    );
    assert_eq!(evidence["workspace_marker_exists"], false);
    assert_eq!(
        evidence["workspace_boundary_fixtures_valid_after_task"],
        true
    );

    let audit_text =
        fs::read_to_string(fixture.store.join("audit.jsonl")).expect("中断Taskのfile-backed Audit");
    assert!(!audit_text.contains(TASK_INSTRUCTION));
    assert!(!audit_text.contains("synthetic-secret-content-never-returned"));
    let (_, persisted) =
        BrokerPersistentStore::open_or_create(&fixture.store, "active-task-interruption-verify")
            .expect("中断後のAudit chain読戻し");
    let terminal_event = persisted
        .audit_log
        .events()
        .iter()
        .find(|event| {
            event.request_id == task_id
                && event.reason.starts_with("AgentTask履歴:")
                && event.reason.contains("\"stage\":\"terminal\"")
        })
        .expect("中断Task terminal Audit");
    let terminal_record: Value = serde_json::from_str(
        terminal_event
            .reason
            .strip_prefix("AgentTask履歴:")
            .expect("Task履歴Audit prefix"),
    )
    .expect("terminal Task履歴Audit JSON");
    assert_eq!(terminal_record["status"], expected_status);
    assert_eq!(terminal_record["failure_class"], expected_failure_class);
    assert!(persisted.audit_log.events().iter().any(|event| {
        event.request_id == replay_request_id
            && event.operation == "AgentTask実行"
            && event.decision == "rejected"
            && event.reason == "broker_replay_detected"
    }));
    if cancellation {
        assert!(persisted.audit_log.events().iter().any(|event| {
            event.operation.contains("取消要求") || event.reason.contains("取消要求")
        }));
    }
}

fn normalized_windows_path(path: &str) -> String {
    let path = path.replace('/', "\\");
    path.strip_prefix("\\\\?\\")
        .unwrap_or(&path)
        .to_ascii_lowercase()
}

fn newly_spawned_codex_root(
    broker_pid: u32,
    executable: &Path,
    before_task: &HashSet<u32>,
) -> Vec<u32> {
    let expected_path = fs::canonicalize(executable)
        .expect("Codex CLI実行体の完全path")
        .to_string_lossy()
        .into_owned();
    let expected_name = executable
        .file_name()
        .expect("Codex CLI file名")
        .to_string_lossy()
        .into_owned();
    let mut snapshot = HPROCESSLIST::CreateToolhelp32Snapshot(co::TH32CS::SNAPPROCESS, None)
        .expect("Codex起動後process一覧snapshot");
    snapshot
        .iter_processes()
        .filter_map(|entry| {
            let entry = entry.expect("Codex起動後process一覧entry");
            let pid = entry.th32ProcessID;
            if pid == 0
                || entry.th32ParentProcessID != broker_pid
                || before_task.contains(&pid)
                || !entry.szExeFile().eq_ignore_ascii_case(&expected_name)
            {
                return None;
            }
            let process = winsafe::HPROCESS::OpenProcess(
                co::PROCESS::QUERY_LIMITED_INFORMATION
                    | co::PROCESS::SYNCHRONIZE
                    | co::PROCESS::TERMINATE,
                false,
                pid,
            )
            .ok()?;
            let actual_path = process
                .QueryFullProcessImageName(co::PROCESS_NAME::WIN32)
                .ok()?;
            (normalized_windows_path(&actual_path) == normalized_windows_path(&expected_path))
                .then_some(pid)
        })
        .collect()
}

fn terminate_codex_root(pid: u32, executable: &Path) {
    let process = winsafe::HPROCESS::OpenProcess(
        co::PROCESS::QUERY_LIMITED_INFORMATION | co::PROCESS::SYNCHRONIZE | co::PROCESS::TERMINATE,
        false,
        pid,
    )
    .expect("特定済みCodex root processを開く");
    let actual_path = process
        .QueryFullProcessImageName(co::PROCESS_NAME::WIN32)
        .expect("Codex rootの完全pathを再照合");
    let expected_path = fs::canonicalize(executable)
        .expect("Codex CLI実行体の完全path")
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        normalized_windows_path(&actual_path),
        normalized_windows_path(&expected_path),
        "停止対象が今回指定されたCodex CLI実行体と一致"
    );
    process
        .TerminateProcess(0xD4)
        .expect("今回のCodex root processだけを異常終了");
    assert_eq!(
        process
            .WaitForSingleObject(Some(5_000))
            .expect("Codex root process終了待ち"),
        co::WAIT::OBJECT_0,
        "Codex root process終了を確認"
    );
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    hex::encode(Sha256::digest(bytes))
}

#[test]
#[ignore = "WindowsのManifest-only Export runtime copyから隔離test identityで実Codex Tool Taskを確認するときに実行する"]
fn exported_windows_runtime_completes_real_codex_task_with_isolated_identity() {
    let required_path = |name: &str| {
        let path = std::env::var_os(name)
            .map(PathBuf::from)
            .unwrap_or_else(|| panic!("{name}を絶対pathで指定"));
        assert!(path.is_absolute(), "{name}は絶対path");
        path
    };
    let export_root = fs::canonicalize(required_path("GUI_SHELL_W_E03_EXPORT_RUNTIME_ROOT"))
        .expect("Export runtime copyを解決");
    assert!(export_root.is_dir(), "Export runtime copyはdirectory");
    let helper_destination = export_root
        .join(".developer-verification")
        .join("broker")
        .join("gui_shell_rust_helper.exe");
    let helper_parent = helper_destination
        .parent()
        .expect("Export runtime Broker親directory")
        .to_path_buf();
    fs::create_dir_all(&helper_parent).expect("run-owned test Broker directory");
    let helper_parent = fs::canonicalize(helper_parent).expect("test Broker directoryを解決");
    assert!(
        helper_parent.starts_with(&export_root),
        "test Broker overlayはExport runtime copy内"
    );
    assert!(
        !helper_destination.exists(),
        "既存Export runtime fileを上書きしない"
    );
    let mut compiled_helper = fs::File::open(BROKER_EXE).expect("r2-e2e Broker helper");
    let mut helper_output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&helper_destination)
        .expect("test-only Broker overlayを新規作成");
    std::io::copy(&mut compiled_helper, &mut helper_output).expect("test Brokerをoverlayへ複製");
    helper_output
        .sync_all()
        .expect("test Broker overlayをflush");
    drop(helper_output);
    let helper = fs::canonicalize(&helper_destination).expect("Export runtime Brokerを解決");
    assert!(
        helper.is_file() && helper.starts_with(&export_root),
        "Broker helperはExport runtime copy内"
    );

    let manifest_path = fs::canonicalize(required_path("GUI_SHELL_W_E03_MANIFEST_PATH"))
        .expect("Export Manifestを解決");
    assert!(manifest_path.is_file() && manifest_path.starts_with(&export_root));
    let manifest_bytes = fs::read(&manifest_path).expect("未信頼Manifest bytes");
    let manifest_sha256 = sha256_hex(&manifest_bytes);
    assert_eq!(
        manifest_sha256,
        std::env::var("GUI_SHELL_W_E03_MANIFEST_SHA256").expect("Manifest SHA-256")
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes).expect("Manifest JSON");
    let app_id = manifest["manifest"]["app_identity"]["app_id"]
        .as_str()
        .expect("Export App ID")
        .to_owned();
    let audit_store_id = manifest["manifest"]["audit_store"]["store_id"]
        .as_str()
        .expect("Export Audit Store ID")
        .to_owned();
    assert_eq!(
        manifest["manifest"]["app_identity"]["target_platform"],
        "windows"
    );
    assert_eq!(manifest["manifest"]["audit_store"]["inherited"], false);
    assert_eq!(
        manifest["manifest"]["inheritance_policy"]["authority"],
        "none"
    );
    assert_eq!(
        manifest["manifest"]["inheritance_policy"]["permission"],
        "none"
    );
    assert_eq!(
        manifest["manifest"]["inheritance_policy"]["approval"],
        "none"
    );
    assert_eq!(
        manifest["manifest"]["inheritance_policy"]["credential"],
        "none"
    );
    assert_eq!(
        manifest["manifest"]["inheritance_policy"]["audit_chain"],
        "none"
    );
    assert_ne!(app_id, "d4-pocket-app-11111111111111111111111111111111");
    assert_ne!(
        audit_store_id,
        "audit-store-22222222222222222222222222222222"
    );
    assert_eq!(
        app_id,
        std::env::var("GUI_SHELL_W_E03_APP_ID").expect("Export App ID env")
    );
    assert_eq!(
        audit_store_id,
        std::env::var("GUI_SHELL_W_E03_AUDIT_STORE_ID").expect("Export Audit Store ID env")
    );

    let source_commit = std::env::var("GUI_SHELL_W_E03_SOURCE_COMMIT").expect("source commit SHA");
    assert!(
        source_commit.len() == 40
            && source_commit
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "source commit SHA形式"
    );
    let evidence_path = required_path("GUI_SHELL_W_E03_EVIDENCE_PATH");
    assert!(evidence_path.parent().is_some_and(Path::is_dir));
    assert!(!evidence_path.exists(), "既存の証拠を上書きしない");

    let codex = fs::canonicalize(required_path("GUI_SHELL_CODEX_TASK_CRASH_E2E_CLI"))
        .expect("実Codex CLIを解決");
    assert!(codex.is_file(), "Codex CLI実file");
    let helper_bytes = fs::read(&helper).expect("Export runtime Broker bytes");
    let helper_sha256 = sha256_hex(&helper_bytes);
    let mut fixture = Fixture::new_success_with_export_runtime(
        codex.clone(),
        helper.clone(),
        app_id.clone(),
        audit_store_id.clone(),
    );
    let normal = fixture.normal().clone();
    let session = send_request(
        &normal,
        "対話開始",
        json!({"実行系ID":RUNTIME_ID,"作業領域ID":WORKSPACE_ID}),
    );
    assert_eq!(session["status"], "accepted", "{session}");
    let session_id = session["body"]["対話セッションID"]
        .as_str()
        .expect("Agent session ID")
        .to_owned();
    let task = json!({
        "agent_runtime_id": RUNTIME_ID,
        "session_id": session_id,
        "workspace_id": WORKSPACE_ID,
        "instruction": TASK_INSTRUCTION,
    });
    let permission = json!({
        "agent_runtime_id": RUNTIME_ID,
        "session_id": session_id,
        "workspace_id": WORKSPACE_ID,
    });
    let permission_grant = send_request(
        fixture.owner(),
        "AgentTaskWorkspacePermissionGrant",
        permission,
    );
    assert_eq!(permission_grant["status"], "accepted", "{permission_grant}");
    let approval_grant = send_request(fixture.owner(), "AgentTaskOwnerApprovalGrant", task.clone());
    assert_eq!(approval_grant["status"], "accepted", "{approval_grant}");

    let processes_before_task = process_ids();
    let broker_pid = fixture
        .broker
        .as_ref()
        .expect("Export runtime Broker process")
        .id();
    let started = send_request(&normal, "AgentTask実行", task.clone());
    assert_eq!(started["status"], "accepted", "{started}");
    assert_eq!(started["body"]["status"], "running", "{started}");
    let task_id = started["body"]["task_id"]
        .as_str()
        .expect("AgentTask ID")
        .to_owned();
    let mut process_tree = HashSet::new();
    let deadline = Instant::now() + Duration::from_secs(120);
    let terminal = loop {
        let roots = newly_spawned_codex_root(broker_pid, &codex, &processes_before_task);
        assert!(
            roots.len() <= 1,
            "今回の独立Brokerが起動したCodex rootは一意: {roots:?}"
        );
        if let Some(root_pid) = roots.first() {
            process_tree.extend(process_tree_ids(*root_pid));
        }
        let state = send_request(&normal, "AgentTask状態", json!({"task_id":task_id}));
        assert_eq!(state["status"], "accepted", "{state}");
        if state["body"]["status"] != "running" {
            break state;
        }
        assert!(
            Instant::now() < deadline,
            "独立Export Runtime Taskの終端期限超過"
        );
        thread::sleep(Duration::from_millis(50));
    };
    if terminal["body"]["status"] != "completed" {
        let (responses_evidence, fixture_success, fixture_exit_code) = fixture.collect_responses();
        fixture.stop_broker();
        panic!(
            "Export Runtime Task failed without weakening the completion condition: task_status={}; fixture_success={fixture_success}; fixture_exit_code={fixture_exit_code:?}; expected_shape={}; requests={}; models={}; tool_offered={}; tool_call_sent={}; tool_result_received={}; tool_output_diagnostics={}; agent_failure_diagnostics={}; workspace_marker={}; blocked_external={}; invalid_bodies={}",
            terminal["body"]["status"],
            responses_evidence["expected_request_shape"],
            responses_evidence["requests"],
            responses_evidence["models"],
            responses_evidence["tool_offered"],
            responses_evidence["tool_call_sent"],
            responses_evidence["tool_result_received"],
            serde_json::to_string(&responses_evidence["tool_output_diagnostics"])
                .unwrap_or_else(|_| "unavailable".to_owned()),
            serde_json::to_string(&responses_evidence["safe_agent_failure_diagnostics"])
                .unwrap_or_else(|_| "unavailable".to_owned()),
            responses_evidence["workspace_marker_exists"],
            responses_evidence["blocked_external_requests"],
            responses_evidence["invalid_bodies"],
        );
    }
    let result_hash = terminal["body"]["result_hash"]
        .as_str()
        .expect("completed Task positive result hash")
        .to_owned();
    assert!(
        result_hash.strip_prefix("sha256:").is_some_and(|value| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }),
        "result hash形式"
    );
    assert!(
        !process_tree.is_empty(),
        "今回のBrokerが起動したCodex processを観測"
    );
    wait_for_process_tree_exit(&process_tree, Duration::from_secs(20));
    assert_eq!(
        fs::read(fixture.workspace.join("broker-real-codex-marker.txt"))
            .expect("実Codex ToolによるWorkspace書込み"),
        b"synthetic-task-write"
    );
    assert_eq!(
        fs::read(fixture.workspace.join("private/fixture-secret.txt"))
            .expect("secret boundary marker"),
        b"synthetic-secret-content-never-returned"
    );
    assert_eq!(
        fs::read(fixture.root.join("outside-read-marker.txt")).expect("Workspace外marker"),
        b"synthetic outside marker"
    );
    assert!(!fixture.root.join("outside-write-marker.txt").exists());
    assert!(scratch_directories(&fixture.workspace)
        .expect("Workspace scratch一覧")
        .is_empty());

    let approval_replay = send_request(&normal, "AgentTask実行", task);
    assert_eq!(
        approval_replay["status"], "rejected",
        "完了済みTaskの一回Approvalを再利用しない: {approval_replay}"
    );
    let approval_replay_audit_id = approval_replay["audit_event_id"]
        .as_str()
        .expect("Approval再利用拒否のAudit ID")
        .to_owned();
    fixture.stop_broker();
    let responses_evidence = fixture.finish_responses();
    assert_eq!(
        responses_evidence["expected_request_shape"],
        "completed_tool_write"
    );
    assert_eq!(responses_evidence["requests"], 2);
    assert_eq!(responses_evidence["tool_call_sent"], true);
    assert_eq!(responses_evidence["tool_result_received"], true);
    assert_eq!(responses_evidence["workspace_marker_exists"], true);
    assert_eq!(
        responses_evidence["workspace_boundary_fixtures_valid_after_task"],
        true
    );
    assert_eq!(responses_evidence["blocked_external_requests"], 0);

    let audit_path = fixture.store.join("audit.jsonl");
    let audit_bytes = fs::read(&audit_path).expect("独立Export Runtimeのdurable Audit");
    let audit_text = String::from_utf8(audit_bytes.clone()).expect("Audit UTF-8");
    assert!(
        !audit_text.contains(TASK_INSTRUCTION),
        "Task本文をAuditへ保存しない"
    );
    assert!(
        !audit_text.contains("synthetic-secret-content-never-returned"),
        "secretをAuditへ保存しない"
    );
    let (_, persisted) = BrokerPersistentStore::open_or_create(&fixture.store, "w-e03-verify")
        .expect("Broker終了後に永続Audit chainを再検証");
    let terminal_event = persisted
        .audit_log
        .events()
        .iter()
        .find(|event| {
            event.request_id == task_id
                && event.reason.starts_with("AgentTask履歴:")
                && event.reason.contains("\"stage\":\"terminal\"")
        })
        .expect("completed Task terminal Audit");
    let terminal_record: Value = serde_json::from_str(
        terminal_event
            .reason
            .strip_prefix("AgentTask履歴:")
            .expect("Task履歴Audit prefix"),
    )
    .expect("terminal Task履歴 JSON");
    assert_eq!(terminal_record["status"], "completed");
    assert_eq!(terminal_record["result_hash"], result_hash);
    assert!(persisted.audit_log.events().iter().any(|event| {
        event.operation == "AgentTaskWorkspacePermissionGrant"
            && event.evidence_source == "FIXTURE"
            && event.reason.contains("synthetic Owner fixture")
    }));
    assert!(persisted.audit_log.events().iter().any(|event| {
        event.operation == "AgentTaskOwnerApprovalGrant"
            && event.evidence_source == "FIXTURE"
            && event.reason.contains("synthetic Owner fixture")
    }));
    assert!(persisted.audit_log.events().iter().any(|event| {
        event.event_id == approval_replay_audit_id
            && event.operation == "AgentTask実行"
            && event.decision == "rejected"
    }));

    let evidence = json!({
        "evidence_type": "d4_pocket_w_e03_windows_export_runtime_developer",
        "evidence_class": "LIVE_RUNTIME",
        "source_commit": source_commit,
        "manifest_sha256": manifest_sha256,
        "app_id": app_id,
        "audit_store_id": audit_store_id,
        "export_runtime_broker_relative_path": helper.strip_prefix(&export_root)
            .expect("Broker path is bound to Export runtime root")
            .to_string_lossy(),
        "export_runtime_broker_sha256": helper_sha256,
        "real_owner_authority": false,
        "synthetic_owner_fixture": true,
        "windows_sandbox_setup": "passed",
        "external_provider_requests": 0,
        "task_status": "completed",
        "task_result_hash": result_hash,
        "workspace_marker_sha256": sha256_hex(b"synthetic-task-write"),
        "outside_workspace_write": "denied",
        "secret_value_exposed": false,
        "approval_replay": "rejected",
        "scratch_cleanup": "passed",
        "codex_process_tree_cleanup": "passed",
        "durable_audit_chain": "passed",
        "durable_audit_event_count": persisted.audit_log.events().len(),
        "durable_audit_sha256": sha256_hex(&audit_bytes),
        "task_body_in_audit": false,
        "scope_limit": "isolated r2-e2e Broker and synthetic Owner fixture; this does not establish normal Release UI, real Owner authority, or source-product stop/uninstall acceptance"
    });
    let evidence_bytes = serde_json::to_vec_pretty(&evidence).expect("bounded evidence JSON");
    assert!(
        evidence_bytes.len() <= 32 * 1024,
        "W-E03 evidence size bound"
    );
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&evidence_path)
        .expect("新規evidence pathだけ作成")
        .write_all(&evidence_bytes)
        .expect("W-E03開発証拠を書込");
}

#[test]
#[ignore = "Windows上で認証済みBroker IPCから実Codex/MxC Taskを取消し、子孫停止とscratch回収を確認するときに実行する"]
fn owner_cancellation_of_active_codex_task_stops_descendants_and_recovers_scratch() {
    let codex = std::env::var_os("GUI_SHELL_CODEX_TASK_CRASH_E2E_CLI")
        .map(PathBuf::from)
        .expect("GUI_SHELL_CODEX_TASK_CRASH_E2E_CLIにCodex CLI絶対pathを設定");
    run_active_task_interruption_e2e(codex, ActiveTaskInterruption::OwnerCancellation);
}

#[test]
#[ignore = "Windows上で実Codex/MxC Taskをdeadlineで停止し、子孫停止とscratch回収を確認するときに実行する"]
fn deadline_of_active_codex_task_stops_descendants_and_recovers_scratch() {
    let codex = std::env::var_os("GUI_SHELL_CODEX_TASK_CRASH_E2E_CLI")
        .map(PathBuf::from)
        .expect("GUI_SHELL_CODEX_TASK_CRASH_E2E_CLIにCodex CLI絶対pathを設定");
    run_active_task_interruption_e2e(codex, ActiveTaskInterruption::Deadline);
}

#[test]
#[ignore = "Windows上で隔離Responses APIのHTTP 503後に実Codex Taskが失敗・回復することを確認するときに実行する"]
fn provider_http_503_after_active_codex_task_stops_descendants_and_recovers_scratch() {
    let codex = std::env::var_os("GUI_SHELL_CODEX_TASK_CRASH_E2E_CLI")
        .map(PathBuf::from)
        .expect("GUI_SHELL_CODEX_TASK_CRASH_E2E_CLIにCodex CLI絶対pathを設定");
    run_active_task_interruption_e2e(codex, ActiveTaskInterruption::ProviderFailure);
}

#[test]
#[ignore = "Windows上でr2-e2e Broker processを強制終了し、実Codex/MxCの停止と再起動回復を確認するときに実行する"]
fn broker_kill_during_active_codex_task_stops_descendants_recovers_scratch_and_does_not_reuse_approval(
) {
    let codex = std::env::var_os("GUI_SHELL_CODEX_TASK_CRASH_E2E_CLI")
        .map(PathBuf::from)
        .expect("GUI_SHELL_CODEX_TASK_CRASH_E2E_CLIにCodex CLI絶対pathを設定");
    let mut fixture = Fixture::new(codex);
    let old_normal = fixture.normal().clone();

    let started = send_request(
        &old_normal,
        "対話開始",
        json!({"実行系ID":RUNTIME_ID,"作業領域ID":WORKSPACE_ID}),
    );
    assert_eq!(started["status"], "accepted", "{started}");
    let session_id = started["body"]["対話セッションID"]
        .as_str()
        .expect("対話セッション識別子")
        .to_owned();
    for operation in ["MCP Tool実行", "AgentTask結果表示承認"] {
        let outside_scope = send_request(fixture.owner(), operation, json!({}));
        assert_eq!(
            outside_scope["status"], "rejected",
            "{operation}: {outside_scope}"
        );
        assert_eq!(
            outside_scope["error"]["code"], "r2_e2e_synthetic_owner_request_invalid",
            "synthetic Owner fixtureの固定allowlist"
        );
    }
    let task = json!({
        "agent_runtime_id": RUNTIME_ID,
        "session_id": session_id,
        "workspace_id": WORKSPACE_ID,
        "instruction": TASK_INSTRUCTION,
    });
    let permission = json!({
        "agent_runtime_id": RUNTIME_ID,
        "session_id": session_id,
        "workspace_id": WORKSPACE_ID,
    });
    let granted = send_request(
        fixture.owner(),
        "AgentTaskWorkspacePermissionGrant",
        permission,
    );
    assert_eq!(granted["status"], "accepted", "{granted}");
    let approved = send_request(fixture.owner(), "AgentTaskOwnerApprovalGrant", task.clone());
    assert_eq!(approved["status"], "accepted", "{approved}");
    let task_started = send_request(&old_normal, "AgentTask実行", task.clone());
    assert_eq!(task_started["status"], "accepted", "{task_started}");
    assert_eq!(task_started["body"]["status"], "running", "{task_started}");
    let task_id = task_started["body"]["task_id"]
        .as_str()
        .expect("作業要求識別子")
        .to_owned();

    let deadline = Instant::now() + Duration::from_secs(75);
    let heartbeat = fixture.workspace.join("broker-real-codex-heartbeat.txt");
    let mut prior_length = 0;
    let scratch = loop {
        if let (Some(directory), Ok(metadata)) =
            (only_scratch(&fixture.workspace), fs::metadata(&heartbeat))
        {
            let report = fs::read(fixture.workspace.join("broker-real-codex-temp-report.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
            if report
                .as_ref()
                .is_some_and(|value| value["stage"] == "temp_checked")
                && metadata.len() > prior_length
            {
                prior_length = metadata.len();
                break directory;
            }
        }
        assert!(
            Instant::now() < deadline,
            "実Codex/MxC probeとWorkspaceTaskScratch生成期限"
        );
        thread::sleep(Duration::from_millis(50));
    };
    thread::sleep(Duration::from_millis(250));
    let active_length = fs::metadata(&heartbeat).expect("稼働heartbeat").len();
    assert!(active_length > prior_length, "active child heartbeatが継続");
    let active_status = send_request(&old_normal, "AgentTask状態", json!({"task_id": task_id}));
    assert_eq!(
        active_status["body"]["status"], "running",
        "{active_status}"
    );

    fixture.kill_broker();
    let stopped_baseline = fs::metadata(&heartbeat)
        .expect("Broker process終了直後heartbeat")
        .len();
    thread::sleep(Duration::from_millis(350));
    let stopped_length = fs::metadata(&heartbeat)
        .expect("Broker crash後heartbeat")
        .len();
    assert_eq!(
        stopped_length, stopped_baseline,
        "Broker hard kill後にCodex/MxC child processのheartbeatが停止"
    );
    assert!(scratch.is_dir(), "再起動前は中断Task scratchを保持");

    fixture.start_broker();
    assert!(
        !scratch.exists(),
        "現在Workspaceへの登録時に中断scratchを回収"
    );
    let new_normal = fixture.normal().clone();
    let mut stale_endpoint = new_normal.clone();
    stale_endpoint.session_id = old_normal.session_id.clone();
    stale_endpoint.session_secret = old_normal.session_secret.clone();
    let stale = send_request(&stale_endpoint, "AgentTask実行", task.clone());
    assert_eq!(
        stale["status"], "rejected",
        "旧IPC資格を再利用しない: {stale}"
    );

    let new_session = send_request(
        &new_normal,
        "対話開始",
        json!({"実行系ID":RUNTIME_ID,"作業領域ID":WORKSPACE_ID}),
    );
    assert_eq!(new_session["status"], "accepted", "{new_session}");
    let new_session_id = new_session["body"]["対話セッションID"]
        .as_str()
        .expect("新規Broker Session ID")
        .to_owned();
    let mut fresh_task = task.clone();
    fresh_task["session_id"] = json!(new_session_id);
    let fresh_permission = send_request(
        fixture.owner(),
        "AgentTaskWorkspacePermissionGrant",
        json!({
            "agent_runtime_id": RUNTIME_ID,
            "session_id": new_session_id,
            "workspace_id": WORKSPACE_ID,
        }),
    );
    assert_eq!(fresh_permission["status"], "accepted", "{fresh_permission}");
    let without_new_approval = send_request(&new_normal, "AgentTask実行", fresh_task);
    assert_eq!(
        without_new_approval["status"], "rejected",
        "新SessionのWorkspace Permissionだけでは以前のTask Approvalを再利用できない: {without_new_approval}"
    );

    fixture.stop_broker();
    let evidence = fixture.finish_responses();
    assert_eq!(
        evidence["expected_request_shape"],
        "tool_call_without_result"
    );
    assert_eq!(
        evidence["tool_call_sent"], true,
        "偽APIが実Codex tool callを受信"
    );
    assert_eq!(
        evidence["tool_result_received"], false,
        "中断taskのtool resultが返らない"
    );
    assert_eq!(
        evidence["workspace_marker_exists"], false,
        "中断taskは完了writeしない"
    );
    assert_eq!(
        evidence["workspace_boundary_fixtures_valid_after_task"],
        true
    );

    let audit_path = fixture.store.join("audit.jsonl");
    let audit_text = fs::read_to_string(&audit_path).expect("Broker再起動後の永続Audit");
    assert!(
        !audit_text.contains(TASK_INSTRUCTION),
        "Task本文をAuditへ保存しない"
    );
    assert!(!audit_text.contains("synthetic-secret-content-never-returned"));
    let (_, persisted) = BrokerPersistentStore::open_or_create(&fixture.store, "crash-e2e-verify")
        .expect("永続Audit chain再検証");
    assert!(persisted.audit_log.events().iter().any(|event| {
        event.operation.starts_with("Agent Task中断回復")
            && event.request_id == task_id
            && event.decision == "suspended"
    }));
    assert!(persisted.audit_log.events().iter().any(|event| {
        event.operation == "AgentTaskWorkspacePermissionGrant"
            && event.evidence_source == "FIXTURE"
            && event.reason.contains("synthetic Owner fixture")
    }));
    assert!(persisted.audit_log.events().iter().any(|event| {
        event.operation == "Agent Task scratch回復" && event.evidence_source == "LIVE_RUNTIME"
    }));
    assert!(
        only_scratch(&fixture.workspace).is_none(),
        "再起動後にTask scratchが残らない"
    );
}

#[test]
#[ignore = "Windows上で実Agent Task後の改竄scratch journalに対するBroker fail-closedを確認するときに実行する"]
fn tampered_scratch_journal_blocks_broker_restart_and_preserves_orphan_scratch() {
    let codex = std::env::var_os("GUI_SHELL_CODEX_TASK_CRASH_E2E_CLI")
        .map(PathBuf::from)
        .expect("GUI_SHELL_CODEX_TASK_CRASH_E2E_CLIにCodex CLI絶対pathを設定");
    let mut fixture = Fixture::new(codex.clone());
    let normal = fixture.normal().clone();
    let started = send_request(
        &normal,
        "対話開始",
        json!({"実行系ID":RUNTIME_ID,"作業領域ID":WORKSPACE_ID}),
    );
    assert_eq!(started["status"], "accepted", "{started}");
    let session_id = started["body"]["対話セッションID"]
        .as_str()
        .expect("Agent対話Session識別子")
        .to_owned();
    let task = json!({
        "agent_runtime_id": RUNTIME_ID,
        "session_id": session_id,
        "workspace_id": WORKSPACE_ID,
        "instruction": TASK_INSTRUCTION,
    });
    let permission = send_request(
        fixture.owner(),
        "AgentTaskWorkspacePermissionGrant",
        json!({
            "agent_runtime_id": RUNTIME_ID,
            "session_id": session_id,
            "workspace_id": WORKSPACE_ID,
        }),
    );
    assert_eq!(permission["status"], "accepted", "{permission}");
    let approval = send_request(fixture.owner(), "AgentTaskOwnerApprovalGrant", task.clone());
    assert_eq!(approval["status"], "accepted", "{approval}");

    let processes_before_task = process_ids();
    let broker_pid = fixture.broker.as_ref().expect("Broker実体").id();
    let task_started = send_request(&normal, "AgentTask実行", task);
    assert_eq!(task_started["status"], "accepted", "{task_started}");
    assert_eq!(task_started["body"]["status"], "running", "{task_started}");
    let task_id = task_started["body"]["task_id"]
        .as_str()
        .expect("Agent Task識別子")
        .to_owned();
    let active = wait_for_active_codex_task(
        &fixture,
        &normal,
        &task_id,
        broker_pid,
        &codex,
        &processes_before_task,
        Duration::from_secs(75),
    );

    fixture.kill_broker();
    wait_for_process_tree_exit(&active.process_tree, Duration::from_secs(15));
    let stopped_heartbeat = fs::metadata(&active.heartbeat)
        .expect("Broker終了後のheartbeat")
        .len();
    thread::sleep(Duration::from_millis(350));
    assert_eq!(
        fs::metadata(&active.heartbeat)
            .expect("停止確認後heartbeat")
            .len(),
        stopped_heartbeat,
        "Broker kill後にCodex／MxC process群が停止"
    );
    assert!(active.scratch.is_dir(), "Broker停止後の孤立scratchを保持");

    let journal_path = fixture.store.join("agent_task_scratch_recovery.json");
    let tampered_journal = tamper_scratch_journal_state(&journal_path);
    fixture.start_broker_expect_failure("Agent Task scratch回復記録の認証に失敗");

    assert!(
        active.scratch.is_dir(),
        "改竄journal時に孤立scratchを削除しない"
    );
    assert_eq!(
        fs::read(&journal_path).expect("fail-closed後のjournal"),
        tampered_journal,
        "Broker起動失敗時に改竄journalを上書きしない"
    );
    assert!(
        active.process_tree.is_disjoint(&process_ids()),
        "失敗したBroker再起動後もCodex／MxC process群は不在"
    );

    let evidence = fixture.finish_responses();
    assert_eq!(
        evidence["expected_request_shape"],
        "tool_call_without_result"
    );
    assert_eq!(evidence["tool_call_sent"], true);
    assert_eq!(evidence["tool_result_received"], false);
    assert_eq!(evidence["workspace_marker_exists"], false);
}

#[test]
#[ignore = "Windows上でr2-e2e Brokerを維持したまま実Codex rootを異常終了し、子孫停止とTask回復を確認するときに実行する"]
fn codex_root_crash_keeps_broker_alive_stops_descendants_recovers_task_and_does_not_reuse_approval()
{
    let codex = std::env::var_os("GUI_SHELL_CODEX_TASK_CRASH_E2E_CLI")
        .map(PathBuf::from)
        .expect("GUI_SHELL_CODEX_TASK_CRASH_E2E_CLIにCodex CLI絶対pathを設定");
    let mut fixture = Fixture::new(codex.clone());
    let normal = fixture.normal().clone();
    let started = send_request(
        &normal,
        "対話開始",
        json!({"実行系ID":RUNTIME_ID,"作業領域ID":WORKSPACE_ID}),
    );
    assert_eq!(started["status"], "accepted", "{started}");
    let session_id = started["body"]["対話セッションID"]
        .as_str()
        .expect("対話セッション識別子")
        .to_owned();
    let task = json!({
        "agent_runtime_id": RUNTIME_ID,
        "session_id": session_id,
        "workspace_id": WORKSPACE_ID,
        "instruction": TASK_INSTRUCTION,
    });
    let permission = json!({
        "agent_runtime_id": RUNTIME_ID,
        "session_id": session_id,
        "workspace_id": WORKSPACE_ID,
    });
    let granted = send_request(
        fixture.owner(),
        "AgentTaskWorkspacePermissionGrant",
        permission.clone(),
    );
    assert_eq!(granted["status"], "accepted", "{granted}");
    let approved = send_request(fixture.owner(), "AgentTaskOwnerApprovalGrant", task.clone());
    assert_eq!(approved["status"], "accepted", "{approved}");
    let processes_before_task = process_ids();
    let broker_pid = fixture.broker.as_ref().expect("仲介処理系の実行状態").id();
    let task_started = send_request(&normal, "AgentTask実行", task.clone());
    assert_eq!(task_started["status"], "accepted", "{task_started}");
    assert_eq!(task_started["body"]["status"], "running", "{task_started}");
    let task_id = task_started["body"]["task_id"]
        .as_str()
        .expect("作業要求識別子")
        .to_owned();

    let deadline = Instant::now() + Duration::from_secs(75);
    let heartbeat = fixture.workspace.join("broker-real-codex-heartbeat.txt");
    let (root_pid, heartbeat_before_active) = loop {
        if fs::metadata(&heartbeat).is_ok() && only_scratch(&fixture.workspace).is_some() {
            let candidates = newly_spawned_codex_root(broker_pid, &codex, &processes_before_task);
            if !candidates.is_empty() {
                assert_eq!(
                    candidates.len(),
                    1,
                    "今回のBrokerが起動したCodex rootは一意"
                );
                let heartbeat_length = fs::metadata(&heartbeat)
                    .expect("実MxC child heartbeat")
                    .len();
                break (candidates[0], heartbeat_length);
            }
        }
        assert!(Instant::now() < deadline, "実Codex rootとheartbeat検出期限");
        thread::sleep(Duration::from_millis(50));
    };
    thread::sleep(Duration::from_millis(250));
    let active_heartbeat = fs::metadata(&heartbeat)
        .expect("実MxC child heartbeat")
        .len();
    assert!(
        active_heartbeat > heartbeat_before_active,
        "Codex root crash前にMxC child heartbeatが実際に進行"
    );
    let active_status = send_request(&normal, "AgentTask状態", json!({"task_id": task_id}));
    assert_eq!(
        active_status["body"]["status"], "running",
        "{active_status}"
    );
    terminate_codex_root(root_pid, &codex);

    let mut previous_heartbeat = fs::metadata(&heartbeat)
        .expect("Codex root crash直後のMxC child heartbeat")
        .len();
    let heartbeat_deadline = Instant::now() + Duration::from_secs(20);
    let child_heartbeat_stopped = loop {
        assert!(
            Instant::now() < heartbeat_deadline,
            "Codex root crash後のMxC child heartbeat停止期限超過; last_length={previous_heartbeat}"
        );
        thread::sleep(Duration::from_millis(350));
        let current_heartbeat = fs::metadata(&heartbeat)
            .expect("Codex root crash後のMxC child heartbeat")
            .len();
        if current_heartbeat == previous_heartbeat {
            break true;
        }
        previous_heartbeat = current_heartbeat;
    };
    assert!(
        child_heartbeat_stopped,
        "Codex root crash後にMxC childが停止"
    );

    let mut terminal_status = Value::Null;
    let recovery_deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < recovery_deadline {
        assert!(
            fixture
                .broker
                .as_mut()
                .expect("Codex crash後もBrokerが存続")
                .try_wait()
                .expect("Broker状態確認")
                .is_none(),
            "Codex root crashがBroker processへ波及しない"
        );
        terminal_status = send_request(&normal, "AgentTask状態", json!({"task_id": task_id}));
        if terminal_status["body"]["status"] != "running" {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        terminal_status["body"]["status"], "failed",
        "{terminal_status}"
    );
    assert!(
        only_scratch(&fixture.workspace).is_none(),
        "Codex root crash後にD4-owned Task scratchを回収"
    );
    assert!(
        !fixture
            .workspace
            .join("D4P_CRASH_TASK_COMPLETED.txt")
            .exists(),
        "異常終了TaskはWorkspace完了markerを書かない"
    );

    let new_permission = send_request(
        fixture.owner(),
        "AgentTaskWorkspacePermissionGrant",
        permission,
    );
    assert_eq!(new_permission["status"], "accepted", "{new_permission}");
    let approval_reuse = send_request(&normal, "AgentTask実行", task);
    assert_eq!(
        approval_reuse["status"], "rejected",
        "Codex crashで消費済みのApprovalをPermission再発行だけで再利用しない: {approval_reuse}"
    );

    fixture.stop_broker();
    let evidence = fixture.finish_responses();
    assert_eq!(
        evidence["expected_request_shape"],
        "tool_call_without_result"
    );
    assert_eq!(
        evidence["tool_call_sent"], true,
        "偽APIがCodex tool callを受信"
    );
    assert_eq!(
        evidence["tool_result_received"], false,
        "停止後にtool resultが戻らない"
    );
    assert_eq!(evidence["workspace_marker_exists"], false);
    let audit_text =
        fs::read_to_string(fixture.store.join("audit.jsonl")).expect("Codex異常終了後の監査記録");
    assert!(
        !audit_text.contains(TASK_INSTRUCTION),
        "Task本文をAuditへ保存しない"
    );
    let (_, persisted) =
        BrokerPersistentStore::open_or_create(&fixture.store, "codex-crash-verify")
            .expect("永続Audit chain再検証");
    let terminal_event = persisted
        .audit_log
        .events()
        .iter()
        .find(|event| {
            event.request_id == task_id
                && event.reason.starts_with("AgentTask履歴:")
                && event.reason.contains("\"stage\":\"terminal\"")
                && event.evidence_source == "INTERNAL_STATE"
        })
        .expect("Codex crash後のterminal Audit");
    let terminal_record: Value = serde_json::from_str(
        terminal_event
            .reason
            .strip_prefix("AgentTask履歴:")
            .expect("Task履歴Audit prefix"),
    )
    .expect("terminal Task履歴Audit JSON");
    assert_eq!(terminal_record["status"], "failed");
    assert_eq!(terminal_record["failure_class"], "通信失敗");
}
