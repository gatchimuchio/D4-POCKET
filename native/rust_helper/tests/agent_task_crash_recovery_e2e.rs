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
const TASK_INSTRUCTION: &str =
    "D4P_CRASH_TASK_SENTINEL: perform only the bounded synthetic probe supplied by the local test API";
const BROKER_EXE: &str = env!("CARGO_BIN_EXE_gui_shell_rust_helper");
const RESPONSES_EXE: &str = env!("CARGO_BIN_EXE_gui_shell_r2_e2e_responses");

struct Fixture {
    root: PathBuf,
    store: PathBuf,
    workspace: PathBuf,
    workspace_config: PathBuf,
    codex: PathBuf,
    responses_port: u16,
    broker: Option<Child>,
    normal: Option<BrokerEndpoint>,
    owner: Option<BrokerEndpoint>,
    responses: Option<Child>,
    responses_stdin: Option<ChildStdin>,
    responses_stdout: Option<BufReader<ChildStdout>>,
    generation: u32,
}

impl Fixture {
    fn new(codex: PathBuf) -> Self {
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
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("一時loopback port");
        let responses_port = listener.local_addr().expect("一時port取得").port();
        drop(listener);
        let mut fixture = Self {
            root: root.clone(),
            store: root.join("store"),
            workspace,
            workspace_config,
            codex,
            responses_port,
            broker: None,
            normal: None,
            owner: None,
            responses: None,
            responses_stdin: None,
            responses_stdout: None,
            generation: 0,
        };
        fixture.start_responses();
        fixture.start_broker();
        fixture
    }

    fn start_responses(&mut self) {
        let mut child = Command::new(RESPONSES_EXE)
            .arg(&self.workspace)
            .arg(self.responses_port.to_string())
            .arg("--expect-cancellation")
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

        let runtime = format!(
            "{RUNTIME_ID}={}={}",
            self.codex.display(),
            self.workspace.display()
        );
        let suffix = self.generation;
        let session_file = self.root.join(format!("normal-{suffix}.json"));
        let owner_file = self.root.join(format!("owner-{suffix}.json"));
        let child = Command::new(BROKER_EXE)
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
            .env_remove("OPENAI_API_KEY")
            .env_remove("CODEX_API_KEY")
            .env_remove("ANTHROPIC_API_KEY")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("r2-e2e Broker process起動");
        self.broker = Some(child);
        self.normal = None;
        self.owner = None;
        let normal = self.wait_for_endpoint(&session_file);
        let owner = self.wait_for_endpoint(&owner_file);
        assert_ne!(normal.session_secret, owner.session_secret);
        self.normal = Some(normal);
        self.owner = Some(owner);
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

    fn finish_responses(&mut self) -> Value {
        let mut stdin = self.responses_stdin.take().expect("応答器の標準入力を取得");
        writeln!(stdin, "stop").expect("fixture停止要求");
        stdin.flush().expect("fixture停止要求flush");
        drop(stdin);
        let mut stdout = self
            .responses_stdout
            .take()
            .expect("応答器の標準出力を取得");
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut evidence = None;
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
        assert!(self
            .responses
            .take()
            .expect("応答器processを取得")
            .wait()
            .expect("fixture終了待ち")
            .success());
        evidence.expect("応答器の検証記録")
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

fn only_scratch(workspace: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(workspace).ok()?;
    let paths = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(".d4p-tmp-"))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    (paths.len() == 1).then(|| paths[0].clone())
}

fn process_ids() -> HashSet<u32> {
    let mut snapshot = HPROCESSLIST::CreateToolhelp32Snapshot(co::TH32CS::SNAPPROCESS, None)
        .expect("process一覧snapshot");
    snapshot
        .iter_processes()
        .map(|entry| entry.expect("process一覧entry").th32ProcessID)
        .collect()
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
