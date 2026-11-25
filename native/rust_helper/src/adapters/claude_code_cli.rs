//! Claude Code CLIを、Brokerが固定したWorkspace・Task・Process境界へ接続する。
//!
//! User/project設定、MCP server、shell/code tool、session保存を無効化し、
//! Taskごとのsecret path拒否を明示した非対話実行だけを使う。Provider資格情報は
//! CLIの管理領域に委譲し、環境変数・D4の資格情報保管庫からCLIへ渡さない。
#![allow(non_snake_case)]

use super::process_tree;
use crate::audit_hash::sha256_tagged;
use crate::broker::dialogue::{
    実行系Adapter, 実行結果, 対話失敗, 対話要求, AgentTaskAdapterExecution,
    AgentTaskScratchObservation,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const MINIMUM_CLAUDE_CODE_VERSION: (u32, u32, u32) = (2, 1, 314);
const MAX_SETTINGS_BYTES: usize = 24 * 1024;
const MAX_RESULT_CHARS: usize = 65_536;
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
    "CLAUDE_CONFIG_DIR",
];

pub struct ClaudeCodeCliAdapter {
    executable: PathBuf,
    workspace: PathBuf,
    workspace_identity: crate::broker::workspace_root::DirectoryIdentity,
    version: String,
    model_id: String,
}

impl ClaudeCodeCliAdapter {
    /// BrokerのOwner確認後にだけ呼ばれる。CLI実物のversion/helpだけを読み、
    /// ProviderへのTask要求やWorkspace変更は行わない。
    pub fn new(executable: &Path, workspace: &Path, model_id: &str) -> Result<Self, String> {
        if !super::codex_cli::valid_model_id(model_id) {
            return Err("Claude Code Model IDが不正".into());
        }
        let executable = canonical_executable(executable)?;
        let workspace = canonical_workspace(workspace)?;
        let workspace_guard = crate::broker::workspace_root::pin_workspace_path(&workspace)
            .map_err(|_| "Claude Code Workspaceを安全に固定できない")?;
        let workspace_identity = workspace_guard.identity;
        if secret_component(&workspace) {
            return Err("Claude Code Workspaceがsecret pathに該当する".into());
        }

        let version_output = run_probe(&executable, &workspace, &["--version"])?;
        let version_text = String::from_utf8_lossy(&version_output.stdout);
        let version = version_text.split_whitespace().next().unwrap_or_default();
        if !version_output.success
            || !version_text.contains("Claude Code")
            || !version_at_least(version, MINIMUM_CLAUDE_CODE_VERSION)
        {
            return Err("Claude Code CLIのversion interfaceを確認できない".into());
        }

        let help = run_probe(&executable, &workspace, &["--help"])?;
        if !help.success || !task_interface_present(&help.stdout) {
            return Err("Claude Code CLIのTask interfaceを確認できない".into());
        }

        Ok(Self {
            executable,
            workspace,
            workspace_identity,
            version: version.to_owned(),
            model_id: model_id.to_owned(),
        })
    }

    fn execute_agent_task_observed(
        &self,
        instruction: &str,
        cancel: &AtomicBool,
        deadline: Instant,
        context: Option<crate::broker::agent_task_scratch::AgentTaskScratchContext>,
        credential: Option<zeroize::Zeroizing<Vec<u8>>>,
    ) -> AgentTaskAdapterExecution {
        let failed = |error| AgentTaskAdapterExecution {
            result: Err(error),
            scratch: None,
            tool_exit_code: None,
        };
        if credential.is_some() {
            return failed(対話失敗::権限拒否);
        }
        if !self.AgentTask実行対応() {
            return failed(対話失敗::AgentTask非対応);
        }
        let Some(context) = context else {
            return failed(対話失敗::AgentTask非対応);
        };
        if let Err(error) = super::codex_cli::validate_task_secret_paths(
            &self.workspace,
            self.workspace_identity,
            &context.secret_paths,
        ) {
            return failed(error);
        }
        let mut scratch = match super::codex_cli::WorkspaceTaskScratch::create(
            &self.workspace,
            self.workspace_identity,
            &context,
            deadline,
        ) {
            Ok(scratch) => scratch,
            Err(error) => return failed(error),
        };
        let before_state_hash = temporary_state_hash_before();
        let after_state_hash = temporary_state_hash_after(scratch.path());
        let settings = match task_settings(&context.secret_paths, scratch.path()) {
            Some(settings) => settings,
            None => {
                let cleanup = scratch.cleanup_after_task_quiesced();
                return failed(cleanup.err().unwrap_or(対話失敗::要求不正));
            }
        };
        let run = run_claude(
            &self.executable,
            &self.workspace,
            self.workspace_identity,
            &self.model_id,
            Some(&settings),
            Some(scratch.path()),
            "Read,Edit,Write,Glob,Grep",
            "acceptEdits",
            instruction,
            cancel,
            deadline,
        );
        let tool_exit_code = if run.result.is_ok() && run.exit_code == Some(0) {
            Some(0)
        } else {
            None
        };
        let cleanup = if run.process_stopped {
            scratch.cleanup_after_task_quiesced()
        } else {
            Err(対話失敗::通信失敗)
        };
        let cleanup_completed = cleanup.is_ok();
        let result = match cleanup {
            Ok(()) => run.result.map(|response| response.text),
            Err(error) => Err(error),
        };
        AgentTaskAdapterExecution {
            result,
            scratch: Some(AgentTaskScratchObservation {
                task_id: context.task_id,
                runtime_id: context.runtime_id,
                workspace_id: context.workspace_id,
                recovery_binding_hash: context.recovery_binding_hash,
                root_identity:
                    crate::broker::dialogue::AgentTaskWorkspaceIdentity::from_directory_identity(
                        context.root_identity,
                    ),
                before_state_hash,
                after_state_hash,
                cleanup_completed,
            }),
            tool_exit_code,
        }
    }

    fn respond_with_no_tools(
        &self,
        request: &対話要求,
        cancel: &AtomicBool,
        deadline: Instant,
        raw_received: &mut Vec<Vec<u8>>,
    ) -> Result<実行結果, 対話失敗> {
        let run = run_claude(
            &self.executable,
            &self.workspace,
            self.workspace_identity,
            &self.model_id,
            None,
            None,
            "",
            "plan",
            &request.入力,
            cancel,
            deadline,
        );
        if !run.process_stopped {
            return Err(対話失敗::通信失敗);
        }
        let response = run.result?;
        let trace_id = super::codex_cli::trace_id(&request.要求ID);
        let trace_hash = sha256_tagged(&response.raw);
        raw_received.push(response.raw.clone());
        Ok(実行結果 {
            対話セッションID: request.対話セッションID.clone(),
            本文: response.text,
            参照: Vec::new(),
            能力: Vec::new(),
            経路: format!("claude-code-cli:{}", response.session_id),
            追跡ID: trace_id,
            追跡hash: trace_hash,
            保留: false,
            生応答: response.raw,
        })
    }
}

impl 実行系Adapter for ClaudeCodeCliAdapter {
    fn 接続対象(&self) -> String {
        "claude-code-cli://broker-governed-task".into()
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
        let supported_os = cfg!(any(windows, target_os = "linux", target_os = "macos"));
        let task_status = if !supported_os {
            ("unsupported", "このOSのBroker process群監督は未対応")
        } else if cfg!(debug_assertions) || cfg!(test) {
            (
                "unsupported",
                "debug/test buildでは製品Agent Task capabilityを公開しない",
            )
        } else {
            (
                "supported",
                "登録CLIの制限Task interfaceを確認。Provider接続状態は別評価",
            )
        };
        Some(json!({
            "adapter_id": "claude-code-cli",
            "agent_id": "claude-code",
            "provider": "Anthropic Claude Code CLI",
            "provider_id": "anthropic_claude_code_cli",
            "version": self.version,
            "model": self.model_id,
            "status": "degraded",
            "capabilities": [
                {"capability_id": "task_execution", "support": {"status": task_status.0, "reason": task_status.1}},
                {"capability_id": "provider_selection", "support": {"status": "supported", "reason": "Claude Code CLI managed認証経路だけを選択可能"}},
                {"capability_id": "model_selection", "support": {"status": "supported", "reason": "選択Model aliasをClaude Code CLI --modelへ渡す。実在性・利用可否は未確認"}},
                {"capability_id": "session_control", "support": {"status": "unsupported", "reason": "各要求は非永続の独立CLI実行で、Claude sessionの再開を使わない"}}
            ],
            "provider_health": {"status": "unknown", "reason": "登録時にCLI interfaceだけを確認しProvider接続を行っていない"},
            "automatic_fallback": false,
            "workspace_requirements": {
                "mode": "required",
            "boundary_policy": "deny_outside_workspace",
                "secret_paths": [".env", ".ssh", ".gnupg", "secrets/"]
            },
            "tool_support": {"status": "supported", "reason": "Task processはRead/Edit/Write/Glob/Grepだけを許可しshell/code/MCPを無効化"},
            "mcp_support": {"status": "unsupported", "reason": "Claude Code sessionのMCP serverは無効化"},
            "session_support": {"status": "unsupported", "reason": "--no-session-persistenceを使用"},
            "cancellation_support": {"status": if supported_os {"supported"} else {"unsupported"}, "reason": "Broker所有process treeの取消と期限停止へ接続"},
            "usage_metrics_support": {"status": "unknown", "reason": "CLIのusage metricsを取得・保存しない"},
            "cost_metrics_support": {"status": "unknown", "reason": "Provider費用を取得・推定しない"},
            "authentication": {
                "method": "claude_code_cli_managed",
                "status": "unknown",
                "secret_value_present": false
            },
            "host_requirements": {
                "platforms": [super::codex_cli::host_platform()],
                "network_scope": "unknown",
                "process_spawn": {"status": if supported_os {"supported"} else {"unsupported"}, "reason": "固定Claude Code CLI実行fileだけをBroker監督下で起動"}
            },
            "evidence_source": "LIVE_RUNTIME",
            "evidence_reason": "登録時に実Claude Code CLIのversion/helpとrestricted・safe-mode・非永続Task用interfaceを確認。Provider接続は未実施"
        }))
    }

    fn AgentTask実行対応(&self) -> bool {
        cfg!(all(
            not(debug_assertions),
            any(windows, target_os = "linux", target_os = "macos")
        ))
    }

    fn AgentTask実行(
        &self,
        instruction: &str,
        cancel: &AtomicBool,
        deadline: Instant,
        context: Option<crate::broker::agent_task_scratch::AgentTaskScratchContext>,
    ) -> Result<String, 対話失敗> {
        self.execute_agent_task_observed(instruction, cancel, deadline, context, None)
            .result
    }

    fn AgentTask実行資格情報付き(
        &self,
        instruction: &str,
        cancel: &AtomicBool,
        deadline: Instant,
        context: Option<crate::broker::agent_task_scratch::AgentTaskScratchContext>,
        credential: Option<zeroize::Zeroizing<Vec<u8>>>,
    ) -> Result<String, 対話失敗> {
        self.execute_agent_task_observed(instruction, cancel, deadline, context, credential)
            .result
    }

    fn AgentTask実行観測付き資格情報付き(
        &self,
        instruction: &str,
        cancel: &AtomicBool,
        deadline: Instant,
        context: Option<crate::broker::agent_task_scratch::AgentTaskScratchContext>,
        credential: Option<zeroize::Zeroizing<Vec<u8>>>,
    ) -> AgentTaskAdapterExecution {
        self.execute_agent_task_observed(instruction, cancel, deadline, context, credential)
    }

    fn 応答(
        &self,
        request: &対話要求,
        cancel: &AtomicBool,
        deadline: Instant,
        raw_received: &mut Vec<Vec<u8>>,
    ) -> Result<実行結果, 対話失敗> {
        self.respond_with_no_tools(request, cancel, deadline, raw_received)
    }

    fn 応答資格情報付き(
        &self,
        request: &対話要求,
        cancel: &AtomicBool,
        deadline: Instant,
        raw_received: &mut Vec<Vec<u8>>,
        credential: Option<zeroize::Zeroizing<Vec<u8>>>,
    ) -> Result<実行結果, 対話失敗> {
        if credential.is_some() {
            return Err(対話失敗::権限拒否);
        }
        self.respond_with_no_tools(request, cancel, deadline, raw_received)
    }
}

struct ProbeOutput {
    success: bool,
    stdout: Vec<u8>,
}

struct ClaudeResponse {
    text: String,
    session_id: String,
    raw: Vec<u8>,
}

struct ClaudeRunOutcome {
    result: Result<ClaudeResponse, 対話失敗>,
    process_stopped: bool,
    exit_code: Option<i32>,
}

fn canonical_executable(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("Claude Code executableは絶対pathで指定する".into());
    }
    let resolved = path
        .canonicalize()
        .map_err(|_| "Claude Code executableを確認できない")?;
    let metadata = std::fs::metadata(&resolved)
        .map_err(|_| "Claude Code executable metadataを確認できない")?;
    if !metadata.is_file() {
        return Err("Claude Code executableが通常fileではない".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err("Claude Code executableに実行bitがない".into());
        }
    }
    Ok(resolved)
}

fn canonical_workspace(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("Claude Code Workspaceは絶対pathで指定する".into());
    }
    let resolved = path
        .canonicalize()
        .map_err(|_| "Claude Code Workspaceを確認できない")?;
    if !resolved.is_dir() {
        return Err("Claude Code Workspaceがdirectoryではない".into());
    }
    Ok(resolved)
}

fn secret_component(path: &Path) -> bool {
    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy().to_ascii_lowercase();
        matches!(value.as_str(), ".env" | ".ssh" | ".gnupg" | "secrets")
    })
}

fn base_command(executable: &Path, workspace: &Path) -> Command {
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
    let mut command = base_command(executable, workspace);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = process_tree::spawn_non_interactive(command)
        .map_err(|_| "Claude Code CLI probeを起動できない")?;
    let Some(stdout) = child.child.stdout.take() else {
        let stopped = child.terminate_tree().is_ok();
        return Err(if stopped {
            "Claude Code CLI probeのstdoutを取得できない".into()
        } else {
            "Claude Code CLI probe process群の停止を確認できない".into()
        });
    };
    let reader = thread::spawn(move || super::codex_cli::bounded_read(stdout));
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if child.stop_descendants().is_err() {
                    return Err("Claude Code CLI probe process群を停止できない".into());
                }
                let stdout = reader
                    .join()
                    .map_err(|_| "Claude Code CLI probe readerが異常終了した")?
                    .map_err(|_| "Claude Code CLI probeの出力が上限を超えた")?;
                return Ok(ProbeOutput {
                    success: status.success(),
                    stdout,
                });
            }
            Ok(None) if started.elapsed() < PROBE_TIMEOUT => {
                thread::sleep(Duration::from_millis(10))
            }
            Ok(None) => {
                let stopped = child.terminate_tree().is_ok();
                let _ = reader.join();
                return Err(if stopped {
                    "Claude Code CLI probeが期限を超過した".into()
                } else {
                    "Claude Code CLI probe process群の停止を確認できない".into()
                });
            }
            Err(_) => {
                let _ = child.terminate_tree();
                let _ = reader.join();
                return Err("Claude Code CLI probeの状態を取得できない".into());
            }
        }
    }
}

fn task_interface_present(output: &[u8]) -> bool {
    let help = String::from_utf8_lossy(output);
    [
        "--print",
        "--input-format",
        "--output-format",
        "--model",
        "--settings",
        "--permission-mode",
        "--permission-prompts",
        "--no-session-persistence",
        "--restricted",
        "--safe-mode",
        "--strict-mcp-config",
        "--disable-slash-commands",
        "--tools",
    ]
    .iter()
    .all(|required| help.contains(required))
}

fn version_at_least(version: &str, minimum: (u32, u32, u32)) -> bool {
    let mut parts = version.split('.').map(str::parse::<u32>);
    let Some(Ok(major)) = parts.next() else {
        return false;
    };
    let Some(Ok(minor)) = parts.next() else {
        return false;
    };
    let Some(Ok(patch)) = parts.next() else {
        return false;
    };
    (major, minor, patch) >= minimum
}

fn task_settings(secret_paths: &[String], scratch: &Path) -> Option<String> {
    if secret_paths.len() > 16 {
        return None;
    }
    let scratch_name = scratch.file_name()?.to_str()?;
    if !scratch_name.starts_with(".d4p-tmp-")
        || !scratch_name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-' || byte == b'_'
        })
    {
        return None;
    }

    let mut deny = BTreeSet::new();
    for path in [
        ".env",
        "**/*.env",
        "**/.env.*",
        ".ssh/**",
        "**/.ssh/**",
        ".gnupg/**",
        "**/.gnupg/**",
        "secrets/**",
        "**/secrets/**",
    ] {
        insert_path_deny(&mut deny, path);
    }
    insert_path_deny(&mut deny, &format!("./{scratch_name}"));
    insert_path_deny(&mut deny, &format!("./{scratch_name}/**"));

    for secret in secret_paths {
        let path = normalize_secret_path(secret)?;
        insert_path_deny(&mut deny, &format!("./{path}"));
        insert_path_deny(&mut deny, &format!("./{path}/**"));
    }

    let settings = json!({
        "permissions": {
            "blockReadsOutsideWorkingDirectories": true,
            "blockWritesOutsideWorkingDirectories": true,
            "deny": deny.into_iter().collect::<Vec<_>>()
        }
    });
    let encoded = serde_json::to_string(&settings).ok()?;
    (encoded.len() <= MAX_SETTINGS_BYTES).then_some(encoded)
}

fn insert_path_deny(rules: &mut BTreeSet<String>, path: &str) {
    rules.insert(format!("Read({path})"));
    rules.insert(format!("Edit({path})"));
}

fn normalize_secret_path(path: &str) -> Option<String> {
    if path.is_empty() || path.chars().any(char::is_control) {
        return None;
    }
    let normalized = if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.to_owned()
    };
    if normalized.starts_with('/') || normalized.contains(":/") {
        return None;
    }
    let parts = normalized.split('/').collect::<Vec<_>>();
    if parts
        .iter()
        .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return None;
    }
    Some(
        parts
            .iter()
            .map(|part| escape_permission_segment(part))
            .collect::<Vec<_>>()
            .join("/"),
    )
}

fn escape_permission_segment(segment: &str) -> String {
    let mut escaped = String::with_capacity(segment.len());
    for character in segment.chars() {
        if matches!(character, '\\' | '*' | '?' | '[' | ']' | '!') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

fn temporary_state_hash_before() -> String {
    #[cfg(unix)]
    let state = format!(
        "TEMP={:?}\0TMP={:?}\0TMPDIR={:?}",
        env::var_os("TEMP").map(|value| value.to_string_lossy().into_owned()),
        env::var_os("TMP").map(|value| value.to_string_lossy().into_owned()),
        env::var_os("TMPDIR").map(|value| value.to_string_lossy().into_owned())
    );
    #[cfg(not(unix))]
    let state = format!(
        "TEMP={:?}\0TMP={:?}",
        env::var_os("TEMP").map(|value| value.to_string_lossy().into_owned()),
        env::var_os("TMP").map(|value| value.to_string_lossy().into_owned())
    );
    sha256_tagged(state.as_bytes())
}

fn temporary_state_hash_after(scratch: &Path) -> String {
    let path = scratch.to_string_lossy();
    #[cfg(unix)]
    let state = format!("TEMP={path}\0TMP={path}\0TMPDIR={path}");
    #[cfg(not(unix))]
    let state = format!("TEMP={path}\0TMP={path}");
    sha256_tagged(state.as_bytes())
}

fn run_claude(
    executable: &Path,
    workspace: &Path,
    expected_workspace: crate::broker::workspace_root::DirectoryIdentity,
    model_id: &str,
    settings: Option<&str>,
    temporary_root: Option<&Path>,
    tools: &str,
    permission_mode: &str,
    input: &str,
    cancel: &AtomicBool,
    deadline: Instant,
) -> ClaudeRunOutcome {
    let stopped = |result, exit_code| ClaudeRunOutcome {
        result,
        process_stopped: true,
        exit_code,
    };
    if input.len() > 64 * 1024 {
        return stopped(Err(対話失敗::要求不正), None);
    }
    if Instant::now() >= deadline {
        return stopped(Err(対話失敗::期限超過), None);
    }
    if cancel.load(Ordering::SeqCst) {
        return stopped(Err(対話失敗::取消), None);
    }

    let workspace_guard =
        match super::codex_cli::pin_registered_workspace(workspace, expected_workspace) {
            Ok(guard) => guard,
            Err(_) => return stopped(Err(対話失敗::作業領域不在), None),
        };

    let mut command = base_command(executable, workspace);
    command
        .args([
            "--print",
            "--input-format",
            "text",
            "--output-format",
            "json",
            "--permission-mode",
            permission_mode,
            "--permission-prompts",
            "none",
            "--no-session-persistence",
            "--restricted",
            "--safe-mode",
            "--strict-mcp-config",
            "--disable-slash-commands",
            "--tools",
            tools,
            "--model",
            model_id,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(settings) = settings {
        command.arg("--settings").arg(settings);
    }
    command.env("CLAUDE_CODE_SKIP_PROMPT_HISTORY", "1");
    if let Some(temporary_root) = temporary_root {
        command
            .env("TEMP", temporary_root)
            .env("TMP", temporary_root);
        #[cfg(unix)]
        command.env("TMPDIR", temporary_root);
    }
    let spawn_result = process_tree::spawn_non_interactive(command);
    drop(workspace_guard);
    let mut child = match spawn_result {
        Ok(child) => child,
        Err(_) => return stopped(Err(対話失敗::通信失敗), None),
    };
    let Some(stdout) = child.child.stdout.take() else {
        let process_stopped = child.terminate_tree().is_ok();
        return ClaudeRunOutcome {
            result: Err(対話失敗::通信失敗),
            process_stopped,
            exit_code: None,
        };
    };
    let Some(stderr) = child.child.stderr.take() else {
        let process_stopped = child.terminate_tree().is_ok();
        return ClaudeRunOutcome {
            result: Err(対話失敗::通信失敗),
            process_stopped,
            exit_code: None,
        };
    };
    let stdout_reader = thread::spawn(move || super::codex_cli::bounded_read(stdout));
    let stderr_reader = thread::spawn(move || super::codex_cli::bounded_read(stderr));
    let Some(mut stdin) = child.child.stdin.take() else {
        let process_stopped = child.terminate_tree().is_ok();
        let _ = stdout_reader.join();
        let _ = stderr_reader.join();
        return ClaudeRunOutcome {
            result: Err(対話失敗::通信失敗),
            process_stopped,
            exit_code: None,
        };
    };
    if stdin.write_all(input.as_bytes()).is_err() || stdin.flush().is_err() {
        drop(stdin);
        let process_stopped = child.terminate_tree().is_ok();
        let _ = stdout_reader.join();
        let _ = stderr_reader.join();
        return ClaudeRunOutcome {
            result: Err(対話失敗::通信失敗),
            process_stopped,
            exit_code: None,
        };
    }
    drop(stdin);

    let status = loop {
        if Instant::now() >= deadline {
            let process_stopped = child.terminate_tree().is_ok();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return ClaudeRunOutcome {
                result: Err(対話失敗::期限超過),
                process_stopped,
                exit_code: None,
            };
        }
        if cancel.load(Ordering::SeqCst) {
            let process_stopped = child.terminate_tree().is_ok();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return ClaudeRunOutcome {
                result: Err(対話失敗::取消),
                process_stopped,
                exit_code: None,
            };
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if child.stop_descendants().is_err() {
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return ClaudeRunOutcome {
                        result: Err(対話失敗::通信失敗),
                        process_stopped: false,
                        exit_code: status.code(),
                    };
                }
                break status;
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(_) => {
                let process_stopped = child.terminate_tree().is_ok();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return ClaudeRunOutcome {
                    result: Err(対話失敗::通信失敗),
                    process_stopped,
                    exit_code: None,
                };
            }
        }
    };

    let stdout = match stdout_reader.join() {
        Ok(Ok(stdout)) => stdout,
        _ => return stopped(Err(対話失敗::応答不正), status.code()),
    };
    let _stderr = match stderr_reader.join() {
        Ok(Ok(stderr)) => stderr,
        _ => return stopped(Err(対話失敗::応答不正), status.code()),
    };
    if !status.success() {
        return stopped(Err(対話失敗::通信失敗), status.code());
    }
    let result = parse_claude_result(&stdout);
    stopped(result, status.code())
}

fn parse_claude_result(output: &[u8]) -> Result<ClaudeResponse, 対話失敗> {
    let value: Value = serde_json::from_slice(output).map_err(|_| 対話失敗::応答不正)?;
    if value.get("type").and_then(Value::as_str) != Some("result") {
        return Err(対話失敗::応答不正);
    }
    let stop_reason = value.get("stop_reason").and_then(Value::as_str);
    let permission_denied = value
        .get("permission_denials")
        .and_then(Value::as_array)
        .is_some_and(|denials| !denials.is_empty());
    if stop_reason == Some("refusal") || permission_denied {
        return Err(対話失敗::権限拒否);
    }
    if value.get("is_error").and_then(Value::as_bool) == Some(true)
        || value
            .get("subtype")
            .and_then(Value::as_str)
            .is_some_and(|subtype| subtype.starts_with("error"))
    {
        return Err(対話失敗::通信失敗);
    }
    let text = value
        .get("result")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .ok_or(対話失敗::応答不正)?;
    if text.chars().count() > MAX_RESULT_CHARS {
        return Err(対話失敗::応答不正);
    }
    let session_id = value
        .get("session_id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= 128)
        .unwrap_or("nonpersistent")
        .to_owned();
    Ok(ClaudeResponse {
        text: text.to_owned(),
        session_id,
        raw: output.to_vec(),
    })
}
