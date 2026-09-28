//! 実物Codex CLIを、Brokerの承認済み対話経路へ限定して射影する。
//!
//! このAdapterはIPCからcommand、argv、environment、workspaceを受け取らない。
//! 起動時に明示登録された実行fileとworkspaceだけを使用する。Dialogueはread-only、
//! Owner承認済みAgent Taskは固定permission profileを使う。
#![allow(non_snake_case)]

use super::process_tree;
use crate::audit_hash::sha256_tagged;
use crate::broker::dialogue::{実行系Adapter, 実行結果, 対話失敗, 対話要求};
use cap_fs_ext::DirExt;
use cap_std::fs::Dir;
use serde_json::{json, Value};
use std::env;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const TASK_PERMISSION_PROFILE_OVERRIDES: &[&str] = &[
    "default_permissions=\"d4p-agent-task\"",
    // user設定を無視するTaskでもmxcを固定し、backend選択をuser configへ委ねない。
    "windows.sandbox=\"mxc\"",
    "permissions.d4p-agent-task.extends=\":workspace\"",
    // root accessは閉じ、必要なruntime読取とWorkspace内deny globだけを明示する。
    "permissions.d4p-agent-task.filesystem={\":root\"=\"deny\",\":minimal\"=\"read\",\":workspace_roots\"={\"**/*.env\"=\"deny\",\"**/.ssh/**\"=\"deny\",\"**/secrets/**\"=\"deny\"},\"glob_scan_max_depth\"=8}",
    "permissions.d4p-agent-task.network.enabled=false",
];

const SAFE_ENVIRONMENT: &[&str] = &[
    "PATH",
    "SystemRoot",
    "WINDIR",
    "TEMP",
    "TMP",
    "USERPROFILE",
    "HOME",
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMDATA",
    "SystemDrive",
    "CODEX_HOME",
];

pub struct CodexCliAdapter {
    executable: PathBuf,
    workspace: PathBuf,
    workspace_identity: crate::broker::workspace_root::DirectoryIdentity,
    workspace_write_interface: bool,
    version: String,
}

impl CodexCliAdapter {
    /// ownerの起動設定からだけ呼び出す。PATH探索やIPC由来の任意pathは行わない。
    pub fn new(executable: &Path, workspace: &Path) -> Result<Self, String> {
        let executable = canonical_executable(executable)?;
        let workspace = canonical_workspace(workspace)?;
        let workspace_guard = crate::broker::workspace_root::pin_workspace_path(&workspace)
            .map_err(|_| "Codexのworkspaceを安全に固定できない")?;
        let workspace_identity = workspace_guard.identity;
        if secret_component(&workspace) {
            return Err("Codexのworkspaceがsecret pathに該当する".into());
        }

        let version = run_probe(&executable, &workspace, &["--version"])?;
        let version_output = String::from_utf8_lossy(&version.stdout);
        if !version.success || !version_output.contains("codex-cli") {
            return Err("Codex CLIのversion interfaceを確認できない".into());
        }
        let help = run_probe(&executable, &workspace, &["exec", "--help"])?;
        if !help.success || !exec_interface_present(&help.stdout) {
            return Err("Codex CLIのexec help interfaceを確認できない".into());
        }
        let workspace_write_interface = workspace_write_interface_present(&help.stdout);

        Ok(Self {
            executable,
            workspace,
            workspace_identity,
            workspace_write_interface,
            version: version_output
                .split_whitespace()
                .nth(1)
                .unwrap_or("unknown")
                .to_string(),
        })
    }

    #[cfg(test)]
    fn for_test(executable: PathBuf, workspace: PathBuf) -> Self {
        Self {
            executable,
            workspace,
            workspace_identity: crate::broker::workspace_root::DirectoryIdentity {
                device: 1,
                file_id: 1,
            },
            workspace_write_interface: true,
            version: "test".into(),
        }
    }
}

impl 実行系Adapter for CodexCliAdapter {
    fn 接続対象(&self) -> String {
        "codex-cli://broker-governed-read-only".into()
    }

    fn 作業領域実体識別子(
        &self,
    ) -> Option<crate::broker::dialogue::AgentTaskWorkspaceIdentity> {
        Some(
            crate::broker::dialogue::AgentTaskWorkspaceIdentity::from_directory_identity(
                self.workspace_identity,
            ),
        )
    }

    fn agent_metadata(&self) -> Option<Value> {
        Some(json!({
            "adapter_id": "codex-cli",
            "agent_id": "codex",
            "provider": "OpenAI",
            "version": self.version,
            "model": "unknown",
            "status": "degraded",
            "capabilities": [
                {"capability_id": "task_execution", "support": {"status": "unsupported", "reason": "専用permission profileの実Task、隔離、後始末を実Agentで検証していない"}},
                {"capability_id": "session_control", "support": {"status": "unknown", "reason": "help interfaceの表記だけで実動作を確認していない"}}
            ],
            "workspace_requirements": {
                "mode": "required",
                "boundary_policy": "deny_outside_workspace",
                "secret_paths": [".env", ".ssh", "secrets/"]
            },
            "tool_support": {"status": "unknown", "reason": "実taskを実行してtool経路を確認していない"},
            "mcp_support": {"status": "unknown", "reason": "MCP接続を確認していない"},
            "session_support": {"status": "unknown", "reason": "session操作の実動作を確認していない"},
            "cancellation_support": {"status": "unknown", "reason": "取消経路の実動作を確認していない"},
            "usage_metrics_support": {"status": "unknown", "reason": "実taskのmetricsを取得していない"},
            "cost_metrics_support": {"status": "unknown", "reason": "cost情報を取得していない"},
            "authentication": {"method": "unknown", "secret_value_present": false},
            "host_requirements": {
                "platforms": [host_platform()],
                "network_scope": "unknown",
                "process_spawn": {"status": "unsupported", "reason": "汎用command dispatchはBrokerで停止中"}
            },
            "evidence_source": "LIVE_RUNTIME",
            "evidence_reason": "起動時にversion/help interfaceを実物確認したが、write-capable task経路は実証前のためunsupported"
        }))
    }

    fn AgentTask実行対応(&self) -> bool {
        cfg!(windows) && self.workspace_write_interface
    }

    fn AgentTask実行(
        &self,
        instruction: &str,
        cancel: &AtomicBool,
        deadline: Instant,
        context: Option<crate::broker::agent_task_scratch::AgentTaskScratchContext>,
    ) -> Result<String, 対話失敗> {
        if !self.AgentTask実行対応() {
            return Err(対話失敗::AgentTask非対応);
        }
        let context = context.ok_or(対話失敗::AgentTask非対応)?;
        let mut scratch =
            WorkspaceTaskScratch::create(&self.workspace, self.workspace_identity, &context)?;
        let result = run_agent_task(
            &self.executable,
            &self.workspace,
            self.workspace_identity,
            &scratch,
            instruction,
            cancel,
            deadline,
        );
        match scratch.cleanup() {
            Ok(()) => result,
            Err(error) => Err(error),
        }
    }

    fn 応答(
        &self,
        要求: &対話要求,
        取消: &AtomicBool,
        期限: Instant,
        生受信: &mut Vec<Vec<u8>>,
    ) -> Result<実行結果, 対話失敗> {
        let mut child = spawn_codex_task(
            &self.executable,
            &self.workspace,
            self.workspace_identity,
            &要求.入力,
            CodexSandbox::ReadOnly,
            None,
        )?;
        let stdout = child.child.stdout.take().ok_or(対話失敗::通信失敗)?;
        let stderr = child.child.stderr.take().ok_or(対話失敗::通信失敗)?;
        let stdout_reader = thread::spawn(move || bounded_read(stdout));
        let stderr_reader = thread::spawn(move || bounded_read(stderr));

        let status = loop {
            if 取消.load(Ordering::SeqCst) {
                if child.terminate_tree().is_err() {
                    return Err(対話失敗::通信失敗);
                }
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(対話失敗::取消);
            }
            if Instant::now() >= 期限 {
                if child.terminate_tree().is_err() {
                    return Err(対話失敗::通信失敗);
                }
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(対話失敗::期限超過);
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    if child.stop_descendants().is_err() {
                        return Err(対話失敗::通信失敗);
                    }
                    break status;
                }
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(_) => {
                    if child.terminate_tree().is_err() {
                        return Err(対話失敗::通信失敗);
                    }
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err(対話失敗::通信失敗);
                }
            }
        };

        let stdout = stdout_reader
            .join()
            .map_err(|_| 対話失敗::通信失敗)?
            .map_err(|_| 対話失敗::応答不正)?;
        let _stderr = stderr_reader
            .join()
            .map_err(|_| 対話失敗::通信失敗)?
            .map_err(|_| 対話失敗::応答不正)?;
        if !status.success() {
            return Err(対話失敗::通信失敗);
        }

        let (本文, thread_id) = parse_jsonl(&stdout)?;
        let trace_id = trace_id(&要求.要求ID);
        let trace_hash = sha256_tagged(&stdout);
        生受信.push(stdout.clone());
        Ok(実行結果 {
            対話セッションID: 要求.対話セッションID.clone(),
            本文,
            参照: Vec::new(),
            能力: Vec::new(),
            経路: format!("codex-cli:{thread_id}"),
            追跡ID: trace_id,
            追跡hash: trace_hash,
            保留: false,
            生応答: stdout,
        })
    }
}

fn host_platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "android") {
        "android"
    } else if cfg!(target_os = "ios") {
        "ios"
    } else {
        "unknown"
    }
}

struct ProbeOutput {
    success: bool,
    stdout: Vec<u8>,
}

fn canonical_executable(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("Codex executableは絶対pathで指定する".into());
    }
    let resolved = path
        .canonicalize()
        .map_err(|_| "Codex executableを確認できない")?;
    if !resolved.is_file() {
        return Err("Codex executableが通常fileではない".into());
    }
    Ok(resolved)
}

fn canonical_workspace(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("Codex workspaceは絶対pathで指定する".into());
    }
    let resolved = path
        .canonicalize()
        .map_err(|_| "Codex workspaceを確認できない")?;
    if !resolved.is_dir() {
        return Err("Codex workspaceがdirectoryではない".into());
    }
    Ok(resolved)
}

fn secret_component(path: &Path) -> bool {
    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy().to_ascii_lowercase();
        matches!(value.as_str(), ".env" | ".ssh" | ".gnupg" | "secrets")
    })
}

fn command(executable: &Path, workspace: &Path) -> Command {
    let mut command = Command::new(executable);
    command.env_clear();
    for name in SAFE_ENVIRONMENT {
        if let Some(value) = env::var_os(name) {
            command.env(name, value);
        }
    }
    command.current_dir(workspace);
    command
}

fn run_probe(executable: &Path, workspace: &Path, args: &[&str]) -> Result<ProbeOutput, String> {
    let mut child = command(executable, workspace)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Codex CLI probeを起動できない")?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Codex CLI probeのstdoutを取得できない")?;
    let reader = thread::spawn(move || bounded_read(stdout));
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = reader
                    .join()
                    .map_err(|_| "Codex CLI probeのreaderが異常終了した")?
                    .map_err(|_| "Codex CLI probeの出力が上限を超えた")?;
                return Ok(ProbeOutput {
                    success: status.success(),
                    stdout: output,
                });
            }
            Ok(None) if started.elapsed() < PROBE_TIMEOUT => {
                thread::sleep(Duration::from_millis(10))
            }
            Ok(None) => {
                terminate(&mut child);
                let _ = reader.join();
                return Err("Codex CLI probeが期限を超過した".into());
            }
            Err(_) => {
                terminate(&mut child);
                let _ = reader.join();
                return Err("Codex CLI probeの状態を取得できない".into());
            }
        }
    }
}

fn exec_interface_present(output: &[u8]) -> bool {
    let help = String::from_utf8_lossy(output);
    [
        "codex exec",
        "--sandbox",
        "--cd",
        "--json",
        "--ephemeral",
        "--ignore-user-config",
    ]
    .iter()
    .all(|required| help.contains(required))
}

fn workspace_write_interface_present(output: &[u8]) -> bool {
    exec_interface_present(output) && String::from_utf8_lossy(output).contains("workspace-write")
}

#[derive(Clone, Copy)]
enum CodexSandbox {
    ReadOnly,
    WorkspaceWrite,
}

struct WorkspaceTaskScratch {
    _workspace_guard: crate::broker::workspace_root::WorkspacePathGuard,
    _workspace_dir: Dir,
    scratch_dir: Option<Dir>,
    path: PathBuf,
    journal: crate::broker::agent_task_scratch::AgentTaskScratchJournal,
    record_id: String,
}

impl WorkspaceTaskScratch {
    fn create(
        workspace: &Path,
        expected: crate::broker::workspace_root::DirectoryIdentity,
        context: &crate::broker::agent_task_scratch::AgentTaskScratchContext,
    ) -> Result<Self, 対話失敗> {
        if context.root_identity != expected {
            return Err(対話失敗::作業領域不在);
        }
        let workspace_guard = pin_registered_workspace(workspace, expected)?;
        let workspace_dir = Dir::open_ambient_dir(workspace, cap_std::ambient_authority())
            .map_err(|_| 対話失敗::通信失敗)?;
        let metadata = workspace_dir
            .dir_metadata()
            .map_err(|_| 対話失敗::通信失敗)?;
        let identity = crate::broker::workspace_root::DirectoryIdentity {
            device: cap_fs_ext::MetadataExt::dev(&metadata),
            file_id: cap_fs_ext::MetadataExt::ino(&metadata),
        };
        if identity != expected {
            return Err(対話失敗::通信失敗);
        }

        for _ in 0..8 {
            let mut nonce = [0u8; 16];
            getrandom::getrandom(&mut nonce).map_err(|_| 対話失敗::通信失敗)?;
            let name = format!(".d4p-tmp-{}", hex::encode(nonce));
            let record_id = context
                .journal
                .reserve(
                    &context.task_id,
                    &context.runtime_id,
                    &context.workspace_id,
                    &context.recovery_binding_hash,
                    expected,
                    &name,
                )
                .map_err(|_| 対話失敗::通信失敗)?;
            match workspace_dir.create_dir(&name) {
                Ok(()) => {
                    let scratch_dir = workspace_dir
                        .open_dir_nofollow(&name)
                        .map_err(|_| 対話失敗::通信失敗)?;
                    let metadata =
                        scratch_dir.dir_metadata().map_err(|_| 対話失敗::通信失敗)?;
                    let scratch_identity = crate::broker::workspace_root::DirectoryIdentity {
                        device: cap_fs_ext::MetadataExt::dev(&metadata),
                        file_id: cap_fs_ext::MetadataExt::ino(&metadata),
                    };
                    context
                        .journal
                        .activate(&record_id, scratch_identity)
                        .map_err(|_| 対話失敗::通信失敗)?;
                    return Ok(Self {
                        _workspace_guard: workspace_guard,
                        _workspace_dir: workspace_dir,
                        scratch_dir: Some(scratch_dir),
                        path: workspace.join(name),
                        journal: context.journal.clone(),
                        record_id,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    context
                        .journal
                        .complete(&record_id)
                        .map_err(|_| 対話失敗::通信失敗)?;
                    continue;
                }
                Err(_) => return Err(対話失敗::通信失敗),
            }
        }
        Err(対話失敗::通信失敗)
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn cleanup(&mut self) -> Result<(), 対話失敗> {
        self.scratch_dir
            .take()
            .ok_or(対話失敗::通信失敗)?
            .remove_open_dir_all()
            .map_err(|_| 対話失敗::通信失敗)?;
        self.journal
            .complete(&self.record_id)
            .map_err(|_| 対話失敗::通信失敗)
    }
}

impl Drop for WorkspaceTaskScratch {
    fn drop(&mut self) {
        if let Some(dir) = self.scratch_dir.take() {
            if dir.remove_open_dir_all().is_ok() {
                let _ = self.journal.complete(&self.record_id);
            }
        }
    }
}

fn spawn_codex_task(
    executable: &Path,
    workspace: &Path,
    expected_workspace: crate::broker::workspace_root::DirectoryIdentity,
    input: &str,
    sandbox: CodexSandbox,
    scratch: Option<&WorkspaceTaskScratch>,
) -> Result<process_tree::SupervisedChild, 対話失敗> {
    let workspace_guard = pin_registered_workspace(workspace, expected_workspace)?;
    let task_command = build_codex_command(
        executable,
        workspace,
        sandbox,
        scratch.map(WorkspaceTaskScratch::path),
    )?;
    let child_result = process_tree::spawn(task_command);
    // WindowsではCreateProcessがcurrent_dirを解決し終えるまでpath階層を固定する。
    drop(workspace_guard);
    let mut child = child_result.map_err(|_| 対話失敗::通信失敗)?;
    let mut stdin = child.child.stdin.take().ok_or(対話失敗::通信失敗)?;
    stdin
        .write_all(input.as_bytes())
        .map_err(|_| 対話失敗::通信失敗)?;
    drop(stdin);
    Ok(child)
}

fn build_codex_command(
    executable: &Path,
    workspace: &Path,
    sandbox: CodexSandbox,
    scratch: Option<&Path>,
) -> Result<Command, 対話失敗> {
    let workspace_path = workspace.to_str().ok_or(対話失敗::要求不正)?;
    let mut task_command = command(executable, workspace);
    if matches!(sandbox, CodexSandbox::WorkspaceWrite) {
        for setting in TASK_PERMISSION_PROFILE_OVERRIDES {
            task_command.arg("-c").arg(setting);
        }
    }
    task_command.args(["exec", "--json", "--ephemeral", "--ignore-user-config"]);
    if let CodexSandbox::ReadOnly = sandbox {
        task_command.args(["--sandbox", "read-only"]);
    }
    task_command
        .args(["--color", "never", "--cd", workspace_path, "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(scratch) = scratch {
        if !scratch.starts_with(workspace) || scratch == workspace {
            return Err(対話失敗::要求不正);
        }
        let scratch_path = scratch.to_str().ok_or(対話失敗::要求不正)?;
        task_command
            .env("TEMP", scratch_path)
            .env("TMP", scratch_path);
    }
    Ok(task_command)
}

fn run_agent_task(
    executable: &Path,
    workspace: &Path,
    expected_workspace: crate::broker::workspace_root::DirectoryIdentity,
    scratch: &WorkspaceTaskScratch,
    instruction: &str,
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<String, 対話失敗> {
    if cancel.load(Ordering::SeqCst) {
        return Err(対話失敗::取消);
    }
    if Instant::now() >= deadline {
        return Err(対話失敗::期限超過);
    }
    let mut child = spawn_codex_task(
        executable,
        workspace,
        expected_workspace,
        instruction,
        CodexSandbox::WorkspaceWrite,
        Some(scratch),
    )?;
    let stdout = child.child.stdout.take().ok_or(対話失敗::通信失敗)?;
    let stderr = child.child.stderr.take().ok_or(対話失敗::通信失敗)?;
    let stdout_reader = thread::spawn(move || bounded_read(stdout));
    let stderr_reader = thread::spawn(move || bounded_read(stderr));

    let status = loop {
        if cancel.load(Ordering::SeqCst) {
            if child.terminate_tree().is_err() {
                return Err(対話失敗::通信失敗);
            }
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(対話失敗::取消);
        }
        if Instant::now() >= deadline {
            if child.terminate_tree().is_err() {
                return Err(対話失敗::通信失敗);
            }
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(対話失敗::期限超過);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if child.stop_descendants().is_err() {
                    return Err(対話失敗::通信失敗);
                }
                break status;
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(_) => {
                if child.terminate_tree().is_err() {
                    return Err(対話失敗::通信失敗);
                }
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(対話失敗::通信失敗);
            }
        }
    };

    let stdout = stdout_reader
        .join()
        .map_err(|_| 対話失敗::通信失敗)?
        .map_err(|_| 対話失敗::応答不正)?;
    let _stderr = stderr_reader
        .join()
        .map_err(|_| 対話失敗::通信失敗)?
        .map_err(|_| 対話失敗::応答不正)?;
    if !status.success() {
        return Err(対話失敗::通信失敗);
    }
    parse_jsonl(&stdout).map(|(message, _)| message)
}

fn pin_registered_workspace(
    workspace: &Path,
    expected: crate::broker::workspace_root::DirectoryIdentity,
) -> Result<crate::broker::workspace_root::WorkspacePathGuard, 対話失敗> {
    let guard = crate::broker::workspace_root::pin_workspace_path(workspace)
        .map_err(|_| 対話失敗::通信失敗)?;
    if guard.identity != expected {
        return Err(対話失敗::通信失敗);
    }
    Ok(guard)
}

fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn bounded_read<R: Read>(mut reader: R) -> Result<Vec<u8>, ()> {
    let mut output = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = reader.read(&mut buffer).map_err(|_| ())?;
        if count == 0 {
            return Ok(output);
        }
        if output.len().saturating_add(count) > MAX_OUTPUT_BYTES {
            return Err(());
        }
        output.extend_from_slice(&buffer[..count]);
    }
}

fn parse_jsonl(output: &[u8]) -> Result<(String, String), 対話失敗> {
    let text = std::str::from_utf8(output).map_err(|_| 対話失敗::応答不正)?;
    let mut final_text = None;
    let mut thread_id = None;
    let mut turn_completed = false;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let value: Value = serde_json::from_str(line).map_err(|_| 対話失敗::応答不正)?;
        match value.get("type").and_then(Value::as_str) {
            Some("thread.started") => {
                thread_id = value
                    .get("thread_id")
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned);
            }
            Some("item.completed") => {
                let item = value.get("item").ok_or(対話失敗::応答不正)?;
                if item.get("type").and_then(Value::as_str) == Some("agent_message") {
                    let message = item
                        .get("text")
                        .and_then(Value::as_str)
                        .filter(|value| !value.is_empty())
                        .ok_or(対話失敗::応答不正)?;
                    if message.chars().count() > 65_536 {
                        return Err(対話失敗::応答不正);
                    }
                    final_text = Some(message.to_owned());
                }
            }
            Some("turn.completed") => turn_completed = true,
            Some("turn.failed") | Some("error") => return Err(対話失敗::通信失敗),
            Some(_) | None => {}
        }
    }
    if !turn_completed {
        return Err(対話失敗::応答不正);
    }
    Ok((
        final_text.ok_or(対話失敗::応答不正)?,
        thread_id.unwrap_or_else(|| "unknown".into()),
    ))
}

fn trace_id(request_id: &str) -> String {
    sha256_tagged(request_id.as_bytes())
        .trim_start_matches("sha256:")
        .chars()
        .take(32)
        .collect()
}

#[cfg(all(test, windows))]
#[path = "../../tests/unit/codex_cli_fixture.rs"]
mod codex_cli_fixture;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch_context(
        identity: crate::broker::workspace_root::DirectoryIdentity,
    ) -> crate::broker::agent_task_scratch::AgentTaskScratchContext {
        crate::broker::agent_task_scratch::AgentTaskScratchContext {
            task_id: "task-fixture".into(),
            runtime_id: "runtime-fixture".into(),
            workspace_id: "workspace-fixture".into(),
            recovery_binding_hash: format!("sha256:{}", "a".repeat(64)),
            root_identity: identity,
            journal: crate::broker::agent_task_scratch::AgentTaskScratchJournal::in_memory(),
        }
    }

    #[test]
    fn 実物jsonlのfinal_messageだけを本文へ射影する() {
        let output =
            br#"{"type":"thread.started","thread_id":"01a0cd58-c4fc-7221-8d25-dc52d12ba3fd"}
{"type":"turn.started"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"READY"}}
{"type":"turn.completed","usage":{"input_tokens":1,"output_tokens":1}}"#;
        assert_eq!(
            parse_jsonl(output).unwrap(),
            (
                "READY".into(),
                "01a0cd58-c4fc-7221-8d25-dc52d12ba3fd".into()
            )
        );
    }

    #[test]
    fn 不完全または失敗eventを成功へ昇格しない() {
        assert_eq!(
            parse_jsonl(br#"{"type":"turn.completed"}"#),
            Err(対話失敗::応答不正)
        );
        assert_eq!(parse_jsonl(br#"{"type":"error"}"#), Err(対話失敗::通信失敗));
        assert_eq!(parse_jsonl(br#"not-json"#), Err(対話失敗::応答不正));
    }

    #[test]
    fn Dialogueはread_onlyのままTaskだけ専用permission_profileを使う() {
        let executable = Path::new(r"C:\codex.exe");
        let workspace = Path::new(r"C:\workspace");
        let scratch = Path::new(r"C:\workspace\.d4p-tmp-test");
        for sandbox in [CodexSandbox::ReadOnly, CodexSandbox::WorkspaceWrite] {
            let is_task = matches!(sandbox, CodexSandbox::WorkspaceWrite);
            let command =
                build_codex_command(executable, workspace, sandbox, is_task.then_some(scratch))
                    .expect("固定Codex command");
            let args: Vec<_> = command
                .get_args()
                .map(|arg| arg.to_string_lossy().to_string())
                .collect();
            assert!(args
                .windows(2)
                .any(|pair| pair == ["--cd", r"C:\workspace"]));
            if is_task {
                assert!(!args.iter().any(|arg| arg == "--sandbox"));
                assert!(args.iter().any(|arg| arg == "--ignore-user-config"));
                for setting in TASK_PERMISSION_PROFILE_OVERRIDES {
                    assert!(args.windows(2).any(|pair| pair == ["-c", *setting]));
                }
                let filesystem_override = TASK_PERMISSION_PROFILE_OVERRIDES
                    .iter()
                    .find(|setting| setting.starts_with("permissions.d4p-agent-task.filesystem={"))
                    .expect("glob走査深度を含む固定filesystem設定");
                assert!(filesystem_override.starts_with("permissions.d4p-agent-task.filesystem={"));
                assert_eq!(
                    TASK_PERMISSION_PROFILE_OVERRIDES
                        .iter()
                        .filter(
                            |setting| setting.starts_with("permissions.d4p-agent-task.filesystem")
                        )
                        .count(),
                    1,
                    "filesystem policyは単一overrideにまとめて後続tableに潰されない"
                );
                assert!(args
                    .windows(2)
                    .any(|pair| { pair == ["-c", "windows.sandbox=\"mxc\""] }));
                assert!(filesystem_override.contains("\":root\"=\"deny\""));
            } else {
                assert!(args
                    .windows(2)
                    .any(|pair| pair == ["--sandbox", "read-only"]));
                assert!(!args.iter().any(|arg| arg == "-c"));
            }
            let worktree_flag = format!("{}{}", "--", "worktree");
            let add_dir_flag = format!("{}{}", "--", "add-dir");
            assert!(!args.iter().any(|arg| {
                arg.contains("dangerously") || arg == &worktree_flag || arg == &add_dir_flag
            }));
            let has_temp = command
                .get_envs()
                .any(|(key, value)| key == "TEMP" && value == Some(scratch.as_os_str()));
            let has_tmp = command
                .get_envs()
                .any(|(key, value)| key == "TMP" && value == Some(scratch.as_os_str()));
            assert_eq!(has_temp, is_task);
            assert_eq!(has_tmp, is_task);
        }
        assert!(build_codex_command(
            executable,
            workspace,
            CodexSandbox::WorkspaceWrite,
            Some(Path::new(r"C:\outside-temp")),
        )
        .is_err());
    }

    #[test]
    fn read_onlyは維持しworkspace_writeはTaskだけに要求する() {
        let help = b"Usage: codex exec [OPTIONS] [PROMPT]\n--sandbox [read-only]\n--cd DIR\n--json\n--ephemeral\n--ignore-user-config";
        assert!(exec_interface_present(help));
        assert!(!workspace_write_interface_present(help));
        let adapter = CodexCliAdapter {
            executable: PathBuf::from(r"C:\codex.exe"),
            workspace: PathBuf::from(r"C:\workspace"),
            workspace_identity: crate::broker::workspace_root::DirectoryIdentity {
                device: 1,
                file_id: 1,
            },
            workspace_write_interface: false,
            version: "test".into(),
        };
        assert!(!adapter.AgentTask実行対応());
        assert_eq!(
            adapter.AgentTask実行("test", &AtomicBool::new(false), Instant::now(), None),
            Err(対話失敗::AgentTask非対応)
        );
        let help = "codex execの能力検査用fixture\nUsage: codex exec [OPTIONS] [PROMPT]\n--sandbox [read-only, workspace-write]\n--cd DIR\n--json\n--ephemeral\n--ignore-user-config";
        assert!(exec_interface_present(help.as_bytes()));
        assert!(workspace_write_interface_present(help.as_bytes()));
    }

    #[test]
    fn workspace_task_tempは登録Workspace内に作成され明示cleanupされる() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("時計")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "gui-shell-agent-task-scratch-{}-{nonce}",
            std::process::id()
        ));
        let workspace = root.join("workspace");
        std::fs::create_dir_all(&workspace).expect("Workspace試験root");
        let identity = crate::broker::workspace_root::pin_workspace_path(&workspace)
            .expect("Workspaceを固定")
            .identity;

        let context = scratch_context(identity);
        let mut scratch = WorkspaceTaskScratch::create(&workspace, identity, &context)
            .expect("Workspace内scratchを作成");
        let scratch_path = scratch.path().to_path_buf();
        assert_eq!(scratch_path.parent(), Some(workspace.as_path()));
        fs::write(scratch_path.join("fixture.tmp"), b"bounded").expect("scratch内file");
        scratch.cleanup().expect("scratchを明示削除");
        assert!(!scratch_path.exists());
        assert!(!context.journal.has_pending_workspace("workspace-fixture"));
        drop(scratch);
        std::fs::remove_dir_all(root).expect("試験rootを削除");
    }

    #[cfg(windows)]
    #[test]
    fn workspace_task_tempはdrop時にもcleanupする() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("時計")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "gui-shell-agent-task-scratch-drop-{}-{nonce}",
            std::process::id()
        ));
        let workspace = root.join("workspace");
        std::fs::create_dir_all(&workspace).expect("Workspace試験root");
        let identity = crate::broker::workspace_root::pin_workspace_path(&workspace)
            .expect("Workspaceを固定")
            .identity;
        let context = scratch_context(identity);
        let scratch = WorkspaceTaskScratch::create(&workspace, identity, &context)
            .expect("Workspace内scratchを作成");
        let scratch_path = scratch.path().to_path_buf();
        fs::write(scratch_path.join("fixture.tmp"), b"bounded").expect("scratch内file");
        drop(scratch);
        assert!(!scratch_path.exists());
        assert!(!context.journal.has_pending_workspace("workspace-fixture"));
        std::fs::remove_dir_all(root).expect("試験rootを削除");
    }

    #[cfg(windows)]
    #[test]
    fn 偽CodexCLIはAdapterのTask成功・期限超過cleanup・子孫停止を通る() {
        let root = codex_cli_fixture::FixtureTempDirectory::create();
        let fixture_directory = root.path().join("fixture");
        let workspace = root.path().join("workspace");
        fs::create_dir(&fixture_directory).expect("偽CLI用領域");
        fs::create_dir(&workspace).expect("登録作業領域");
        let executable = codex_cli_fixture::compile_fake_codex_cli(&fixture_directory);
        let identity = crate::broker::workspace_root::pin_workspace_path(&workspace)
            .expect("登録Workspace identity")
            .identity;
        let adapter =
            CodexCliAdapter::new(&executable, &workspace).expect("偽CLIのversion/help検査が成功");
        assert!(adapter.AgentTask実行対応());

        let context = scratch_context(identity);
        let completed = adapter.AgentTask実行(
            "fixture success",
            &AtomicBool::new(false),
            Instant::now() + Duration::from_secs(5),
            Some(context.clone()),
        );
        assert_eq!(completed, Ok("fixture-task-completed".into()));
        codex_cli_fixture::assert_no_workspace_task_scratch(&workspace);
        assert!(!context.journal.has_pending_workspace("workspace-fixture"));

        let timed_out = adapter.AgentTask実行(
            "FIXTURE_TIMEOUT",
            &AtomicBool::new(false),
            Instant::now() + Duration::from_secs(1),
            Some(context.clone()),
        );
        assert_eq!(timed_out, Err(対話失敗::期限超過));
        codex_cli_fixture::assert_no_workspace_task_scratch(&workspace);
        assert!(!context.journal.has_pending_workspace("workspace-fixture"));

        let descendant_heartbeat = root.path().join("descendant-heartbeat");
        let instruction = format!(
            "FIXTURE_TIMEOUT_WITH_DESCENDANT {}",
            descendant_heartbeat.display()
        );
        let timed_out_with_descendant = adapter.AgentTask実行(
            &instruction,
            &AtomicBool::new(false),
            Instant::now() + Duration::from_secs(2),
            Some(context.clone()),
        );
        assert_eq!(
            timed_out_with_descendant,
            Err(対話失敗::期限超過),
            "親CLIが生存中の子孫を生成してもTaskは期限超過で終了する"
        );
        let heartbeat_before = fs::metadata(&descendant_heartbeat)
            .expect("期限超過前に子孫が稼働markerを書き込む")
            .len();
        assert!(heartbeat_before > 1, "子孫が複数回heartbeatを記録する");
        thread::sleep(Duration::from_millis(150));
        let heartbeat_after = fs::metadata(&descendant_heartbeat)
            .expect("停止確認中も子孫のmarkerを保持する")
            .len();
        assert_eq!(
            heartbeat_after, heartbeat_before,
            "Adapterの期限超過後にJob Object配下の子孫が追加書込しない"
        );
        codex_cli_fixture::assert_no_workspace_task_scratch(&workspace);
        assert!(!context.journal.has_pending_workspace("workspace-fixture"));
    }

    #[test]
    fn Agent一覧の投影はread_only状態と秘密値非保持を表す() {
        let adapter = CodexCliAdapter::for_test(
            PathBuf::from("C:\\codex.exe"),
            PathBuf::from("C:\\workspace"),
        );
        let metadata = adapter
            .agent_metadata()
            .expect("CodexはAgent Adapterとして投影する");
        assert_eq!(metadata["agent_id"], "codex");
        assert_eq!(metadata["status"], "degraded");
        assert_eq!(metadata["evidence_source"], "LIVE_RUNTIME");
        assert_eq!(metadata["authentication"]["secret_value_present"], false);
        let task_execution = metadata["capabilities"]
            .as_array()
            .expect("capability一覧")
            .iter()
            .find(|capability| capability["capability_id"] == "task_execution")
            .expect("Task実行能力");
        assert_eq!(task_execution["support"]["status"], "unsupported");
        assert_eq!(
            metadata["host_requirements"]["process_spawn"]["status"],
            "unsupported"
        );
        assert!(metadata.get("executable").is_none());
        assert!(metadata.get("workspace").is_none());
    }

    #[test]
    fn secret_componentはworkspace登録を拒否する() {
        assert!(secret_component(Path::new("C:\\workspace\\secrets")));
        assert!(secret_component(Path::new("C:\\workspace\\.ssh")));
        assert!(!secret_component(Path::new("C:\\workspace\\src")));
    }

    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    #[test]
    fn task起動時に登録後のWorkspace差し替えを拒否する() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("時計")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "gui-shell-codex-workspace-{}-{nonce}",
            std::process::id()
        ));
        let workspace = root.join("workspace");
        let moved = root.join("workspace-original");
        std::fs::create_dir_all(&workspace).expect("Workspace試験root");
        let original = crate::broker::workspace_root::pin_workspace_path(&workspace)
            .expect("登録時のWorkspace identity");
        let identity = original.identity;
        drop(original);

        std::fs::rename(&workspace, &moved).expect("元Workspaceを退避");
        std::fs::create_dir(&workspace).expect("同じpathへ別Workspaceを作成");
        assert!(matches!(
            pin_registered_workspace(&workspace, identity),
            Err(対話失敗::通信失敗)
        ));

        std::fs::remove_dir_all(root).expect("試験rootを削除");
    }

    #[cfg(windows)]
    #[test]
    fn spawn中のpath_guardはWorkspaceと親directoryのrenameを阻止する() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("時計")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "gui-shell-codex-workspace-guard-{}-{nonce}",
            std::process::id()
        ));
        let parent = root.join("parent");
        let workspace = parent.join("workspace");
        std::fs::create_dir_all(&workspace).expect("Workspace試験root");
        let guard =
            crate::broker::workspace_root::pin_workspace_path(&workspace).expect("作業領域の固定");

        assert!(std::fs::rename(&workspace, parent.join("renamed-workspace")).is_err());
        assert!(std::fs::rename(&parent, root.join("renamed-parent")).is_err());
        drop(guard);
        std::fs::remove_dir_all(root).expect("試験rootを削除");
    }
}
