//! 実物Codex CLIを、Brokerの承認済み対話経路へ限定して射影する。
//!
//! このAdapterはIPCからcommand、argv、environment、workspaceを受け取らない。
//! 起動時に明示登録された実行fileとworkspaceだけを使用する。Dialogueはread-only、
//! Owner承認済みAgent Taskは登録Workspaceから導出した限定permission profileを使う。
#![allow(non_snake_case)]

use super::process_tree;
use crate::audit_hash::sha256_tagged;
use crate::broker::dialogue::{実行系Adapter, 実行結果, 対話失敗, 対話要求};
use cap_fs_ext::DirExt;
use cap_std::fs::Dir;
use serde_json::{json, Value};
use std::env;
#[cfg(all(feature = "r2-e2e", not(test)))]
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const TASK_PERMISSION_PROFILE_PREFIX_OVERRIDES: &[&str] = &[
    "default_permissions=\"d4p-agent-task\"",
    // user設定を無視するTaskでもmxcを固定し、backend選択をuser configへ委ねない。
    "windows.sandbox=\"mxc\"",
    "permissions.d4p-agent-task.extends=\":workspace\"",
];
const TASK_PERMISSION_PROFILE_SUFFIX_OVERRIDES: &[&str] =
    &["permissions.d4p-agent-task.network.enabled=false"];
const TASK_BASE_DENY_GLOBS: &[&str] = &["**/*.env", "**/.env.*", "**/.ssh/**", "**/secrets/**"];
const MAX_TASK_FILESYSTEM_OVERRIDE_BYTES: usize = 12 * 1024;
const MAX_REGISTERED_SECRET_PATHS: usize = 256;
const TASK_FILESYSTEM_OVERRIDE_PREFIX: &str =
    "permissions.d4p-agent-task.filesystem={\":root\"=\"deny\",\":minimal\"=\"read\",\":workspace_roots\"={";

#[cfg(any(test, feature = "r2-e2e"))]
struct CodexCliTestResponses {
    port: u16,
    codex_home: PathBuf,
}

#[cfg(all(feature = "r2-e2e", not(test)))]
fn loopback_responses_from_environment() -> Result<Option<CodexCliTestResponses>, String> {
    const PORT_ENV: &str = "GUI_SHELL_R2_E2E_RESPONSES_PORT";
    const HOME_ENV: &str = "GUI_SHELL_R2_E2E_CODEX_HOME";
    let port = env::var(PORT_ENV).ok();
    let codex_home = env::var_os(HOME_ENV).map(PathBuf::from);
    match (port, codex_home) {
        (None, None) => Ok(None),
        (Some(port), Some(codex_home)) => {
            let port = port
                .parse::<u16>()
                .ok()
                .filter(|port| *port != 0)
                .ok_or_else(|| "R2 loopback偽API portが不正".to_string())?;
            if !codex_home.is_absolute()
                || codex_home.file_name().and_then(|name| name.to_str()) != Some("codex-home")
            {
                return Err("R2 loopback試験CODEX_HOMEが不正".into());
            }
            let metadata = fs::symlink_metadata(&codex_home)
                .map_err(|_| "R2 loopback試験CODEX_HOMEを確認できない")?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err("R2 loopback試験CODEX_HOMEが通常directoryではない".into());
            }
            if !codex_home.parent().is_some_and(|parent| {
                parent.file_name().and_then(|name| name.to_str())
                    == Some("D4Pocket-R2-E2E-SYNTHETIC")
                    && parent.join("OWNER-APPROVED-SYNTHETIC.txt").is_file()
            }) {
                return Err("R2 loopback試験fixture rootを確認できない".into());
            }
            if fs::read_dir(&codex_home)
                .map_err(|_| "R2 loopback試験CODEX_HOMEを検査できない")?
                .next()
                .transpose()
                .map_err(|_| "R2 loopback試験CODEX_HOMEを検査できない")?
                .is_some()
            {
                return Err("R2 loopback試験CODEX_HOMEに既存fileがある".into());
            }
            Ok(Some(CodexCliTestResponses { port, codex_home }))
        }
        _ => Err("R2 loopback試験設定が揃っていない".into()),
    }
}

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
    model_id: Option<String>,
    #[cfg(any(test, feature = "r2-e2e"))]
    test_responses_api: Option<CodexCliTestResponses>,
}

impl CodexCliAdapter {
    /// ownerの起動設定からだけ呼び出す。PATH探索やIPC由来の任意pathは行わない。
    pub fn new(executable: &Path, workspace: &Path) -> Result<Self, String> {
        Self::new_configured(executable, workspace, None)
    }

    pub fn new_with_model(
        executable: &Path,
        workspace: &Path,
        model_id: &str,
    ) -> Result<Self, String> {
        if !valid_model_id(model_id) {
            return Err("Codex Model IDが不正".into());
        }
        Self::new_configured(executable, workspace, Some(model_id.to_owned()))
    }

    fn new_configured(
        executable: &Path,
        workspace: &Path,
        model_id: Option<String>,
    ) -> Result<Self, String> {
        let executable = canonical_executable(executable)?;
        let workspace = canonical_workspace(workspace)?;
        let workspace_guard = crate::broker::workspace_root::pin_workspace_path(&workspace)
            .map_err(|_| "Codexのworkspaceを安全に固定できない")?;
        let workspace_identity = workspace_guard.identity;
        if secret_component(&workspace) {
            return Err("Codexのworkspaceがsecret pathに該当する".into());
        }

        // CLIのversion/help probe自身がCODEX_HOMEへ一時directoryを作る版がある。
        // 試験用homeの空状態はprobeより先に検証し、実利用者homeには触れない。
        #[cfg(all(feature = "r2-e2e", not(test)))]
        let test_responses_api = loopback_responses_from_environment()?;

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

        #[cfg(test)]
        let test_responses_api: Option<CodexCliTestResponses> = None;

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
            model_id,
            #[cfg(any(test, feature = "r2-e2e"))]
            test_responses_api,
        })
    }

    #[cfg(test)]
    pub(crate) fn use_test_loopback_responses_api(&mut self, port: u16, codex_home: PathBuf) {
        assert_ne!(port, 0, "loopback test server port must be assigned");
        assert!(
            codex_home.is_absolute(),
            "isolated CODEX_HOME must be absolute"
        );
        self.test_responses_api = Some(CodexCliTestResponses { port, codex_home });
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
            model_id: None,
            #[cfg(test)]
            test_responses_api: None,
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
        #[cfg(all(feature = "r2-e2e", not(test)))]
        let task_execution_capability = if self.test_responses_api.is_some() {
            json!({
                "capability_id": "task_execution",
                "support": {
                    "status": "supported",
                    "reason": "決定論的なlocalhost偽APIを使うR2統合検証build内だけのTask実行"
                }
            })
        } else {
            json!({
                "capability_id": "task_execution",
                "support": {
                    "status": "unsupported",
                    "reason": "R2統合検証buildのloopback試験条件が設定されていない"
                }
            })
        };
        #[cfg(not(all(feature = "r2-e2e", not(test))))]
        let task_execution_capability = json!({
            "capability_id": "task_execution",
            "support": {
                "status": "unsupported",
                "reason": "専用permission profileの実Task、隔離、後始末を実Agentで検証していない"
            }
        });
        Some(json!({
            "adapter_id": "codex-cli",
            "agent_id": "codex",
            "provider": "OpenAI",
            "provider_id": "openai_codex_cli",
            "version": self.version,
            "model": self.model_id.as_deref().unwrap_or("unknown"),
            "status": "degraded",
            "capabilities": [
                task_execution_capability,
                {"capability_id": "provider_selection", "support": {"status": "supported", "reason": "登録済みOpenAI via Codex CLI経路だけを選択可能"}},
                {"capability_id": "model_selection", "support": {"status": "supported", "reason": "選択ModelをCodex CLI --modelへ渡す。Modelの実在性・利用可否は未確認"}},
                {"capability_id": "session_control", "support": {"status": "unknown", "reason": "help interfaceの表記だけで実動作を確認していない"}}
            ],
            "provider_health": {"status": "unknown", "reason": "登録時はCLI interfaceのみ検査し、Provider接続・Model利用可否を確認していない"},
            "automatic_fallback": false,
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
            "authentication": {"method": "codex_cli_managed", "status": "unknown", "secret_value_present": false},
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
        validate_task_secret_paths(
            &self.workspace,
            self.workspace_identity,
            &context.secret_paths,
        )?;
        let mut scratch =
            WorkspaceTaskScratch::create(&self.workspace, self.workspace_identity, &context)?;
        #[cfg(any(test, feature = "r2-e2e"))]
        let result = run_agent_task(
            &self.executable,
            &self.workspace,
            self.workspace_identity,
            &scratch,
            &context.secret_paths,
            instruction,
            self.model_id.as_deref(),
            cancel,
            deadline,
            self.test_responses_api.as_ref(),
        );
        #[cfg(not(any(test, feature = "r2-e2e")))]
        let result = run_agent_task(
            &self.executable,
            &self.workspace,
            self.workspace_identity,
            &scratch,
            &context.secret_paths,
            instruction,
            self.model_id.as_deref(),
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
        #[cfg(any(test, feature = "r2-e2e"))]
        let mut child = spawn_codex_task(
            &self.executable,
            &self.workspace,
            self.workspace_identity,
            &要求.入力,
            self.model_id.as_deref(),
            CodexSandbox::ReadOnly,
            None,
            &[],
            None,
        )?;
        #[cfg(not(any(test, feature = "r2-e2e")))]
        let mut child = spawn_codex_task(
            &self.executable,
            &self.workspace,
            self.workspace_identity,
            &要求.入力,
            self.model_id.as_deref(),
            CodexSandbox::ReadOnly,
            None,
            &[],
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
        "--skip-git-repo-check",
        "--model",
    ]
    .iter()
    .all(|required| help.contains(required))
}

fn workspace_write_interface_present(output: &[u8]) -> bool {
    exec_interface_present(output) && String::from_utf8_lossy(output).contains("workspace-write")
}

fn valid_model_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphanumeric() || (index > 0 && b"._:/-".contains(&byte))
        })
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

fn discard_unactivated_task_scratch(
    scratch_dir: Dir,
    context: &crate::broker::agent_task_scratch::AgentTaskScratchContext,
    record_id: &str,
) {
    if scratch_dir.remove_open_dir_all().is_ok() {
        let _ = context.journal.complete(record_id);
    }
}

fn activate_task_scratch_directory(
    scratch_dir: Dir,
    scratch_identity: crate::broker::workspace_root::DirectoryIdentity,
    context: &crate::broker::agent_task_scratch::AgentTaskScratchContext,
    record_id: &str,
) -> Result<Dir, 対話失敗> {
    if context
        .journal
        .activate(record_id, scratch_identity)
        .is_err()
    {
        discard_unactivated_task_scratch(scratch_dir, context, record_id);
        return Err(対話失敗::通信失敗);
    }
    Ok(scratch_dir)
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
                    let metadata = match scratch_dir.dir_metadata() {
                        Ok(metadata) => metadata,
                        Err(_) => {
                            discard_unactivated_task_scratch(scratch_dir, context, &record_id);
                            return Err(対話失敗::通信失敗);
                        }
                    };
                    let scratch_identity = crate::broker::workspace_root::DirectoryIdentity {
                        device: cap_fs_ext::MetadataExt::dev(&metadata),
                        file_id: cap_fs_ext::MetadataExt::ino(&metadata),
                    };
                    let scratch_dir = activate_task_scratch_directory(
                        scratch_dir,
                        scratch_identity,
                        context,
                        &record_id,
                    )?;
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
    model_id: Option<&str>,
    sandbox: CodexSandbox,
    scratch: Option<&WorkspaceTaskScratch>,
    secret_paths: &[String],
    #[cfg(any(test, feature = "r2-e2e"))] test_responses_api: Option<&CodexCliTestResponses>,
) -> Result<process_tree::SupervisedChild, 対話失敗> {
    let workspace_guard = pin_registered_workspace(workspace, expected_workspace)?;
    #[cfg(any(test, feature = "r2-e2e"))]
    let task_command = build_codex_command_with_model(
        executable,
        workspace,
        sandbox,
        scratch.map(WorkspaceTaskScratch::path),
        secret_paths,
        model_id,
        test_responses_api,
    )?;
    #[cfg(not(any(test, feature = "r2-e2e")))]
    let task_command = build_codex_command_with_model(
        executable,
        workspace,
        sandbox,
        scratch.map(WorkspaceTaskScratch::path),
        secret_paths,
        model_id,
    )?;
    if matches!(sandbox, CodexSandbox::WorkspaceWrite) {
        // Scratch作成後のWorkspace変化をもう一度確認し、CLI起動直前の秘密別名を拒否する。
        validate_task_secret_paths(workspace, expected_workspace, secret_paths)?;
    }
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

#[cfg(test)]
fn build_codex_command(
    executable: &Path,
    workspace: &Path,
    sandbox: CodexSandbox,
    scratch: Option<&Path>,
    secret_paths: &[String],
    #[cfg(any(test, feature = "r2-e2e"))] test_responses_api: Option<&CodexCliTestResponses>,
) -> Result<Command, 対話失敗> {
    build_codex_command_with_model(
        executable,
        workspace,
        sandbox,
        scratch,
        secret_paths,
        None,
        #[cfg(any(test, feature = "r2-e2e"))]
        test_responses_api,
    )
}

fn build_codex_command_with_model(
    executable: &Path,
    workspace: &Path,
    sandbox: CodexSandbox,
    scratch: Option<&Path>,
    secret_paths: &[String],
    model_id: Option<&str>,
    #[cfg(any(test, feature = "r2-e2e"))] test_responses_api: Option<&CodexCliTestResponses>,
) -> Result<Command, 対話失敗> {
    if model_id.is_some_and(|model| !valid_model_id(model)) {
        return Err(対話失敗::要求不正);
    }
    let workspace_path = workspace.to_str().ok_or(対話失敗::要求不正)?;
    let mut task_command = command(executable, workspace);
    #[cfg(any(test, feature = "r2-e2e"))]
    if let Some(test_api) = test_responses_api {
        let base_url = format!("http://127.0.0.1:{}/v1", test_api.port);
        let overrides = [
            // 偽API fixtureではCLI標準helpにあるmodel例を使い、独自識別子への依存を避ける。
            "model=\"o3\"".to_owned(),
            "model_provider=\"d4p_loopback_probe\"".to_owned(),
            "model_providers.d4p_loopback_probe.name=\"D4 Pocket loopback probe\"".to_owned(),
            format!("model_providers.d4p_loopback_probe.base_url=\"{base_url}\""),
            "model_providers.d4p_loopback_probe.wire_api=\"responses\"".to_owned(),
            "model_providers.d4p_loopback_probe.requires_openai_auth=false".to_owned(),
            "model_providers.d4p_loopback_probe.supports_websockets=false".to_owned(),
            "model_providers.d4p_loopback_probe.request_max_retries=0".to_owned(),
            "model_providers.d4p_loopback_probe.stream_max_retries=2".to_owned(),
            // 任意の外部Apps／Plugins catalog取得を止め、偽API以外を試験対象から除く。
            "features.apps=false".to_owned(),
            "features.plugins=false".to_owned(),
            // 認証なしloopback試験ではmachine analyticsの送信も無効化する。
            "analytics.enabled=false".to_owned(),
            "approval_policy=\"never\"".to_owned(),
        ];
        for setting in overrides {
            task_command.arg("-c").arg(setting);
        }
        let proxy = format!("http://127.0.0.1:{}", test_api.port);
        task_command
            .env("CODEX_HOME", &test_api.codex_home)
            .env("HTTP_PROXY", &proxy)
            .env("HTTPS_PROXY", &proxy)
            .env("ALL_PROXY", &proxy)
            .env("NO_PROXY", "127.0.0.1,localhost")
            .env("http_proxy", &proxy)
            .env("https_proxy", &proxy)
            .env("all_proxy", &proxy)
            .env("no_proxy", "127.0.0.1,localhost")
            .env("RUST_LOG", "warn");
    }
    if matches!(sandbox, CodexSandbox::WorkspaceWrite) {
        let scratch = scratch.ok_or(対話失敗::要求不正)?;
        if !scratch.starts_with(workspace) || scratch == workspace {
            return Err(対話失敗::要求不正);
        }
        for setting in TASK_PERMISSION_PROFILE_PREFIX_OVERRIDES {
            task_command.arg("-c").arg(setting);
        }
        task_command
            .arg("-c")
            .arg(task_filesystem_override(secret_paths)?);
        for setting in TASK_PERMISSION_PROFILE_SUFFIX_OVERRIDES {
            task_command.arg("-c").arg(setting);
        }
        task_command
            .arg("-c")
            .arg(task_scratch_environment_override(scratch)?);
    } else if !secret_paths.is_empty() {
        return Err(対話失敗::要求不正);
    }
    task_command.args(["exec", "--json", "--ephemeral", "--ignore-user-config"]);
    if let Some(model_id) = model_id {
        task_command.args(["--model", model_id]);
    }
    if let CodexSandbox::ReadOnly = sandbox {
        task_command.args(["--sandbox", "read-only"]);
    }
    task_command
        .args([
            "--color",
            "never",
            "--skip-git-repo-check",
            "--cd",
            workspace_path,
            "-",
        ])
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

fn task_scratch_environment_override(scratch: &Path) -> Result<String, 対話失敗> {
    let scratch_path = scratch
        .to_str()
        .ok_or(対話失敗::要求不正)?
        .replace('\\', "/");
    let encoded_path =
        serde_json::to_string(&scratch_path).map_err(|_| 対話失敗::要求不正)?;
    Ok(format!(
        "shell_environment_policy.set={{TEMP={encoded_path},TMP={encoded_path}}}"
    ))
}

fn task_filesystem_override(secret_paths: &[String]) -> Result<String, 対話失敗> {
    if secret_paths.len() > MAX_REGISTERED_SECRET_PATHS {
        return Err(対話失敗::要求不正);
    }
    let mut deny_globs = std::collections::BTreeSet::new();
    deny_globs.extend(
        TASK_BASE_DENY_GLOBS
            .iter()
            .map(|pattern| (*pattern).to_owned()),
    );
    for path in secret_paths {
        crate::workspace_reader::WorkspaceReader::validate_registered_secret_path(path)
            .map_err(|_| 対話失敗::要求不正)?;
        let literal = escape_codex_glob_literal(path);
        deny_globs.insert(literal.clone());
        deny_globs.insert(format!("{literal}/**"));
    }

    let mut setting = String::from(TASK_FILESYSTEM_OVERRIDE_PREFIX);
    for (index, pattern) in deny_globs.iter().enumerate() {
        if index > 0 {
            setting.push(',');
        }
        setting.push('"');
        setting.push_str(pattern);
        setting.push_str("\"=\"deny\"");
    }
    setting.push_str("},\"glob_scan_max_depth\"=32}");
    if setting.len() > MAX_TASK_FILESYSTEM_OVERRIDE_BYTES {
        return Err(対話失敗::要求不正);
    }
    Ok(setting)
}

fn escape_codex_glob_literal(path: &str) -> String {
    let mut escaped = String::with_capacity(path.len());
    for character in path.chars() {
        match character {
            '[' => escaped.push_str("[[]"),
            ']' => escaped.push_str("[]]"),
            '{' => escaped.push_str("[{]"),
            '}' => escaped.push_str("[}]"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn run_agent_task(
    executable: &Path,
    workspace: &Path,
    expected_workspace: crate::broker::workspace_root::DirectoryIdentity,
    scratch: &WorkspaceTaskScratch,
    secret_paths: &[String],
    instruction: &str,
    model_id: Option<&str>,
    cancel: &AtomicBool,
    deadline: Instant,
    #[cfg(any(test, feature = "r2-e2e"))] test_responses_api: Option<&CodexCliTestResponses>,
) -> Result<String, 対話失敗> {
    if Instant::now() >= deadline {
        return Err(対話失敗::期限超過);
    }
    if cancel.load(Ordering::SeqCst) {
        return Err(対話失敗::取消);
    }
    #[cfg(any(test, feature = "r2-e2e"))]
    let mut child = spawn_codex_task(
        executable,
        workspace,
        expected_workspace,
        instruction,
        model_id,
        CodexSandbox::WorkspaceWrite,
        Some(scratch),
        secret_paths,
        test_responses_api,
    )?;
    #[cfg(not(any(test, feature = "r2-e2e")))]
    let mut child = spawn_codex_task(
        executable,
        workspace,
        expected_workspace,
        instruction,
        model_id,
        CodexSandbox::WorkspaceWrite,
        Some(scratch),
        secret_paths,
    )?;
    let stdout = child.child.stdout.take().ok_or(対話失敗::通信失敗)?;
    let stderr = child.child.stderr.take().ok_or(対話失敗::通信失敗)?;
    let stdout_reader = thread::spawn(move || bounded_read(stdout));
    let stderr_reader = thread::spawn(move || bounded_read(stderr));

    let status = loop {
        if Instant::now() >= deadline {
            if child.terminate_tree().is_err() {
                return Err(対話失敗::通信失敗);
            }
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(対話失敗::期限超過);
        }
        if cancel.load(Ordering::SeqCst) {
            if child.terminate_tree().is_err() {
                return Err(対話失敗::通信失敗);
            }
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(対話失敗::取消);
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
        #[cfg(test)]
        if test_responses_api.is_some() {
            eprintln!(
                "Codex偽API試験の秘匿済み失敗概要: {}",
                loopback_test_failure_summary(status.code(), &stdout, &_stderr)
            );
        }
        return Err(対話失敗::通信失敗);
    }
    let parsed = parse_jsonl(&stdout);
    #[cfg(test)]
    if test_responses_api.is_some() && parsed.is_err() {
        eprintln!(
            "Codex偽API試験の秘匿済み応答解析概要: {}",
            loopback_test_failure_summary(status.code(), &stdout, &_stderr)
        );
    }
    parsed.map(|(message, _)| message)
}

#[cfg(test)]
fn loopback_test_failure_summary(exit_code: Option<i32>, stdout: &[u8], stderr: &[u8]) -> String {
    let stderr_summary = redact_loopback_test_message(&String::from_utf8_lossy(stderr));
    let events = stdout
        .split(|byte| *byte == b'\n')
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .take(16)
        .collect::<Vec<_>>();
    let event_types = events
        .iter()
        .filter_map(|event| event.get("type").and_then(Value::as_str).map(str::to_owned))
        .collect::<Vec<_>>();
    let error_message = events
        .iter()
        .find(|event| event.get("type").and_then(Value::as_str) == Some("error"))
        .and_then(|event| event.get("message"))
        .or_else(|| {
            events
                .iter()
                .find_map(|event| event.get("item").and_then(|item| item.get("message")))
        })
        .and_then(Value::as_str)
        .map(redact_loopback_test_message)
        .unwrap_or_else(|| "<absent>".to_owned());
    format!(
        "exit_code={exit_code:?}; stdout_event_types={event_types:?}; error_message={error_message:?}; stderr_summary={stderr_summary:?}; stderr_bytes={}",
        stderr.len(),
    )
}

#[cfg(test)]
fn redact_loopback_test_message(message: &str) -> String {
    let mut sanitized = message.to_owned();
    for marker in [
        "INTEGRATION_TASK_INSTRUCTION_SENTINEL",
        "synthetic-secret-content-never-returned",
        "synthetic-outside-workspace-marker",
    ] {
        sanitized = sanitized.replace(marker, "<SYNTHETIC_MARKER>");
    }
    for name in ["USERPROFILE", "HOME", "TEMP", "TMP"] {
        if let Some(path) = env::var_os(name).and_then(|value| value.into_string().ok()) {
            if !path.is_empty() {
                sanitized = sanitized.replace(&path, "<LOCAL_PATH>");
            }
        }
    }
    for pattern in [
        r"(?i)bearer\s+[A-Za-z0-9._~+/=-]+",
        r"(?i)sk-[A-Za-z0-9_-]{8,}",
        r"(?i)gh[pousr]_[A-Za-z0-9_]{8,}",
        r"(?i)github_pat_[A-Za-z0-9_]{8,}",
    ] {
        let expression = regex::Regex::new(pattern).expect("試験診断文の秘匿化正規表現");
        sanitized = expression
            .replace_all(&sanitized, "<REDACTED>")
            .into_owned();
    }
    sanitized
        .chars()
        .filter(|character| !character.is_control() || *character == '\t')
        .take(700)
        .collect()
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

fn validate_task_secret_paths(
    workspace: &Path,
    expected: crate::broker::workspace_root::DirectoryIdentity,
    secret_paths: &[String],
) -> Result<(), 対話失敗> {
    let _guard = pin_registered_workspace(workspace, expected)?;
    let root = Dir::open_ambient_dir(workspace, cap_std::ambient_authority())
        .map_err(|_| 対話失敗::作業領域不在)?;
    let metadata = root.dir_metadata().map_err(|_| 対話失敗::作業領域不在)?;
    let observed = crate::broker::workspace_root::DirectoryIdentity {
        device: cap_fs_ext::MetadataExt::dev(&metadata),
        file_id: cap_fs_ext::MetadataExt::ino(&metadata),
    };
    if observed != expected {
        return Err(対話失敗::作業領域不在);
    }
    crate::workspace_reader::WorkspaceReader::from_registered_dir(root, secret_paths)
        .map(|_| ())
        .map_err(|_| 対話失敗::作業領域不在)
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
    #[cfg(windows)]
    use std::os::windows::fs::MetadataExt;

    const BRACKET_SECRET_GLOB: &str = "private/vault[[]ab[]].json";
    const BRACKET_SECRET_SUBTREE_GLOB: &str = "private/vault[[]ab[]].json/**";
    const BRACE_SECRET_GLOB: &str = "private/x[{]y[}].json";
    const BRACE_SECRET_SUBTREE_GLOB: &str = "private/x[{]y[}].json/**";

    fn debug_task_context(
        context: &crate::broker::agent_task_scratch::AgentTaskScratchContext,
    ) -> String {
        format!("{context:?}")
    }

    fn scratch_context(
        identity: crate::broker::workspace_root::DirectoryIdentity,
    ) -> crate::broker::agent_task_scratch::AgentTaskScratchContext {
        crate::broker::agent_task_scratch::AgentTaskScratchContext {
            task_id: "task-fixture".into(),
            runtime_id: "runtime-fixture".into(),
            workspace_id: "workspace-fixture".into(),
            recovery_binding_hash: format!("sha256:{}", "a".repeat(64)),
            root_identity: identity,
            secret_paths: vec!["private/credential-backup.txt".into()],
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
            let secret_paths = if is_task {
                vec!["private/credential-backup.txt".to_owned()]
            } else {
                Vec::new()
            };
            let command = build_codex_command(
                executable,
                workspace,
                sandbox,
                is_task.then_some(scratch),
                &secret_paths,
                None,
            )
            .expect("固定Codex command");
            let args: Vec<_> = command
                .get_args()
                .map(|arg| arg.to_string_lossy().to_string())
                .collect();
            assert!(args.iter().any(|arg| arg == "--skip-git-repo-check"));
            assert!(args
                .windows(2)
                .any(|pair| pair == ["--cd", r"C:\workspace"]));
            if is_task {
                assert!(!args.iter().any(|arg| arg == "--sandbox"));
                assert!(args.iter().any(|arg| arg == "--ignore-user-config"));
                for setting in TASK_PERMISSION_PROFILE_PREFIX_OVERRIDES
                    .iter()
                    .chain(TASK_PERMISSION_PROFILE_SUFFIX_OVERRIDES.iter())
                {
                    assert!(args.windows(2).any(|pair| pair == ["-c", *setting]));
                }
                assert!(args.windows(2).any(|pair| {
                    pair == [
                        "-c",
                        "shell_environment_policy.set={TEMP=\"C:/workspace/.d4p-tmp-test\",TMP=\"C:/workspace/.d4p-tmp-test\"}"
                    ]
                }));
                let filesystem_override = args
                    .windows(2)
                    .find(|pair| {
                        pair[0] == "-c"
                            && pair[1].starts_with("permissions.d4p-agent-task.filesystem={")
                    })
                    .map(|pair| pair[1].as_str())
                    .expect("登録secret pathを含む生成filesystem設定");
                assert!(filesystem_override.starts_with("permissions.d4p-agent-task.filesystem={"));
                assert!(filesystem_override.contains("**/.env.*"));
                assert!(filesystem_override.contains("private/credential-backup.txt"));
                assert!(filesystem_override.contains("private/credential-backup.txt/**"));
                assert!(filesystem_override.contains(r#""glob_scan_max_depth"=32"#));
                assert_eq!(
                    args.windows(2)
                        .filter(|pair| {
                            pair[0] == "-c"
                                && pair[1].starts_with("permissions.d4p-agent-task.filesystem")
                        })
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
            &[],
            None,
        )
        .is_err());
        assert!(build_codex_command(
            executable,
            workspace,
            CodexSandbox::WorkspaceWrite,
            None,
            &[],
            None,
        )
        .is_err());
    }

    #[test]
    fn 選択ModelをCodex実物interface引数へ固定しfallbackしない() {
        let command = build_codex_command_with_model(
            Path::new(r"C:\codex.exe"),
            Path::new(r"C:\workspace"),
            CodexSandbox::ReadOnly,
            None,
            &[],
            Some("model-fixture-v1"),
            None,
        )
        .expect("選択Modelを使う固定CLI command");
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--model", "model-fixture-v1"]));
        assert_eq!(args.iter().filter(|arg| arg.as_str() == "--model").count(), 1);
        assert!(build_codex_command_with_model(
            Path::new(r"C:\codex.exe"),
            Path::new(r"C:\workspace"),
            CodexSandbox::ReadOnly,
            None,
            &[],
            Some("--dangerous"),
            None,
        )
        .is_err());

        let mut adapter = CodexCliAdapter::for_test(
            PathBuf::from(r"C:\codex.exe"),
            PathBuf::from(r"C:\workspace"),
        );
        adapter.model_id = Some("model-fixture-v1".into());
        let metadata = adapter.agent_metadata().expect("提供元・模型情報を取得");
        assert_eq!(metadata["provider"], "OpenAI");
        assert_eq!(metadata["model"], "model-fixture-v1");
        assert_eq!(metadata["provider_id"], "openai_codex_cli");
        assert_eq!(metadata["provider_health"]["status"], "unknown");
        assert_eq!(metadata["automatic_fallback"], false);
        assert_eq!(metadata["authentication"]["method"], "codex_cli_managed");
        assert_eq!(metadata["authentication"]["status"], "unknown");
        assert_eq!(metadata["authentication"]["secret_value_present"], false);
    }

    #[test]
    fn 登録secret_globはliteral_escapeされ完全一致と子孫をdenyする() {
        let paths = ["private/vault[ab].json".into(), "private/x{y}.json".into()];
        let result = task_filesystem_override(&paths);
        assert!(result.is_ok(), "登録secret globの生成が成功する");
        let setting = result.unwrap_or_default();
        assert!(setting.contains(BRACKET_SECRET_GLOB));
        assert!(setting.contains(BRACKET_SECRET_SUBTREE_GLOB));
        assert!(setting.contains(BRACE_SECRET_GLOB));
        assert!(setting.contains(BRACE_SECRET_SUBTREE_GLOB));
        assert!(setting.contains("\":root\"=\"deny\""));
        assert!(setting.contains("\":minimal\"=\"read\""));
        assert!(setting.contains("\"glob_scan_max_depth\"=32"));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "明示指定したCodex CLIで合成pathだけを検証するときに実行する"]
    fn Rust生成Task設定で実Windows隔離の登録secretを拒否する() {
        use std::ffi::OsString;

        let executable = std::env::var_os("GUI_SHELL_CODEX_SANDBOX_TEST_EXE")
            .map(PathBuf::from)
            .expect("GUI_SHELL_CODEX_SANDBOX_TEST_EXEに検証対象CLIの絶対pathを指定する");
        assert!(executable.is_absolute(), "Codex CLIは絶対pathで指定する");
        assert!(executable.is_file(), "Codex CLIの実行fileが存在する");

        let root = super::codex_cli_fixture::FixtureTempDirectory::create();
        let workspace = root.path().join("workspace");
        let peer_workspace = root.path().join("workspace-agent-b");
        let private = workspace.join("private");
        let scratch = workspace.join(".d4p-tmp-live-probe");
        let peer_scratch = peer_workspace.join(".d4p-tmp-live-probe");
        let isolated_codex_home = root.path().join("codex-home");
        fs::create_dir(&workspace).expect("合成Workspace");
        fs::create_dir(&peer_workspace).expect("別Agent用の合成Workspace");
        fs::create_dir(&private).expect("合成secret領域");
        fs::create_dir(&scratch).expect("Task scratch相当directory");
        fs::create_dir(&peer_scratch).expect("別Agent用Task scratch相当directory");
        fs::create_dir(&isolated_codex_home).expect("検証専用Codex home");
        let peer_marker = peer_workspace.join("agent-b-marker.txt");
        let peer_write_target = peer_workspace.join("agent-b-write-marker.txt");
        let workspace_marker = workspace.join("agent-a-marker.txt");
        let workspace_write_target = workspace.join("agent-a-write-marker.txt");
        let workspace_ready = workspace.join(".probe-agent-a-ready");
        let workspace_start = workspace.join(".probe-agent-a-start");
        let workspace_checked = workspace.join(".probe-agent-a-checked");
        let workspace_continue = workspace.join(".probe-agent-a-continue");
        let peer_ready = peer_workspace.join(".probe-agent-b-ready");
        let peer_start = peer_workspace.join(".probe-agent-b-start");
        let peer_checked = peer_workspace.join(".probe-agent-b-checked");
        let peer_continue = peer_workspace.join(".probe-agent-b-continue");
        let peer_write_result = peer_workspace.join("agent-b-write-result.txt");
        let temp_probe_nonce = format!(
            "{}-{}",
            std::process::id(),
            root.path()
                .file_name()
                .expect("検証専用rootの識別子")
                .to_string_lossy()
        );
        let agent_a_temp_marker = format!("d4p-agent-a-{temp_probe_nonce}.txt");
        let agent_b_temp_marker = format!("d4p-agent-b-{temp_probe_nonce}.txt");
        fs::write(&peer_marker, b"synthetic-agent-b-marker").expect("別Agent Workspace marker");
        fs::write(&workspace_marker, b"synthetic-agent-a-marker")
            .expect("Agent Aの作業領域確認file");

        let registered_paths = [
            "private/registered-marker.txt",
            "private/registered-directory",
            "private/registered-write-target.txt",
            "private/秘密[ab].txt",
            "private/vault{x}.txt",
            "private/deep-secret-root",
        ];
        let deep_relative = vec!["d"; 40].join("/");
        let deep_secret_relative = format!("private/deep-secret-root/{deep_relative}/secret.txt");
        let denied_paths = [
            "private/registered-marker.txt",
            "private/registered-directory/child.txt",
            "private/秘密[ab].txt",
            "private/vault{x}.txt",
        ];
        fs::write(
            private.join("registered-marker.txt"),
            b"synthetic-secret-marker",
        )
        .expect("合成登録file");
        let registered_directory = private.join("registered-directory");
        fs::create_dir(&registered_directory).expect("合成登録directory");
        fs::write(
            registered_directory.join("child.txt"),
            b"synthetic-child-marker",
        )
        .expect("合成登録directory内marker");
        fs::write(
            private.join("秘密[ab].txt"),
            b"synthetic-literal-bracket-marker",
        )
        .expect("合成literal bracket marker");
        fs::write(private.join("vaulta.txt"), b"synthetic-glob-decoy").expect("合成glob decoy");
        fs::write(
            private.join("vault{x}.txt"),
            b"synthetic-literal-brace-marker",
        )
        .expect("合成literal brace marker");
        fs::write(private.join("vaultx.txt"), b"synthetic-brace-decoy").expect("合成brace decoy");

        let deep_secret = workspace.join(&deep_secret_relative);
        fs::create_dir_all(deep_secret.parent().expect("深いsecretの親directory"))
            .expect("深さ40の登録secret directory");
        fs::write(&deep_secret, b"synthetic-deep-secret-marker").expect("深さ40の登録secret file");

        let secret_paths = registered_paths.map(str::to_owned);
        let generated = build_codex_command(
            &executable,
            &workspace,
            CodexSandbox::WorkspaceWrite,
            Some(&scratch),
            &secret_paths,
            None,
        )
        .expect("本番Task command設定");
        let mut generated_args = generated.get_args();
        let mut profile_args = Vec::<OsString>::new();
        while let Some(argument) = generated_args.next() {
            if argument == "exec" {
                break;
            }
            assert_eq!(argument, "-c", "Task設定はCodex CLIのconfig overrideである");
            profile_args.push(argument.to_os_string());
            profile_args.push(
                generated_args
                    .next()
                    .expect("config override値")
                    .to_os_string(),
            );
        }
        assert!(!profile_args.is_empty(), "Rust生成profile設定がある");
        let permission_profile = profile_args
            .windows(2)
            .find(|pair| {
                pair[0] == "-c"
                    && pair[1]
                        .to_string_lossy()
                        .starts_with("default_permissions=")
            })
            .and_then(|pair| pair[1].to_str())
            .and_then(|setting| setting.strip_prefix("default_permissions="))
            .and_then(|value| serde_json::from_str::<String>(value).ok())
            .expect("Rust生成default_permissionsからsandbox profile名を得る");
        let peer_generated = build_codex_command(
            &executable,
            &peer_workspace,
            CodexSandbox::WorkspaceWrite,
            Some(&peer_scratch),
            &[],
            None,
        )
        .expect("別Agent用Rust生成Task command設定");
        let mut peer_generated_args = peer_generated.get_args();
        let mut peer_profile_args = Vec::<OsString>::new();
        while let Some(argument) = peer_generated_args.next() {
            if argument == "exec" {
                break;
            }
            assert_eq!(
                argument, "-c",
                "別Agent用Task設定はCodex CLIのconfig overrideである"
            );
            peer_profile_args.push(argument.to_os_string());
            peer_profile_args.push(
                peer_generated_args
                    .next()
                    .expect("別Agent用config override値")
                    .to_os_string(),
            );
        }
        let peer_permission_profile = peer_profile_args
            .windows(2)
            .find(|pair| {
                pair[0] == "-c"
                    && pair[1]
                        .to_string_lossy()
                        .starts_with("default_permissions=")
            })
            .and_then(|pair| pair[1].to_str())
            .and_then(|setting| setting.strip_prefix("default_permissions="))
            .and_then(|value| serde_json::from_str::<String>(value).ok())
            .expect("別Agent用Rust生成default_permissionsからsandbox profile名を得る");
        let peer_task_environment = peer_generated
            .get_envs()
            .filter_map(|(name, value)| {
                let value = value?;
                matches!(name.to_str(), Some("TEMP" | "TMP"))
                    .then(|| (name.to_os_string(), value.to_os_string()))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            peer_task_environment.len(),
            2,
            "別Agent用TEMP／TMP設定がある"
        );
        let quote_path = |path: &Path| format!("'{}'", path.to_string_lossy().replace('\'', "''"));
        let cloud_file =
            std::env::var_os("GUI_SHELL_ONEDRIVE_CLOUD_FILE_TEST_PATH").map(PathBuf::from);
        let cloud_read_probe = if let Some(path) = cloud_file.as_ref() {
            assert!(path.is_absolute(), "OneDrive合成Cloud File pathは絶対path");
            assert_eq!(
                path.file_name().and_then(|name| name.to_str()),
                Some("OWNER-APPROVED-SYNTHETIC.txt"),
                "Cloud File probeは所有者承認済み合成fileだけを対象にする"
            );
            assert!(
                path.ancestors().any(|ancestor| {
                    ancestor.file_name().and_then(|name| name.to_str())
                        == Some("D4Pocket-R2-E2E-SYNTHETIC")
                }),
                "Cloud File probeは専用合成fixture root内に限る"
            );
            let attributes = std::fs::symlink_metadata(path)
                .expect("OneDrive合成Cloud File metadata")
                .file_attributes();
            const FILE_ATTRIBUTE_UNPINNED: u32 = 0x0010_0000;
            const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
            assert_ne!(
                attributes & FILE_ATTRIBUTE_UNPINNED,
                0,
                "OneDrive合成fileがonline-onlyである (attributes=0x{attributes:08X})"
            );
            assert_ne!(
                attributes & FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
                0,
                "OneDrive合成fileがremote data accessを要求するplaceholderである (attributes=0x{attributes:08X})"
            );
            String::from(
                r#"$cloudErrorType=''; $cloudErrorHResult=0; try{$null=Get-Content -Raw -LiteralPath __D4P_CLOUD_FILE_PATH__ -ErrorAction Stop}catch{$cloudErrorType=$_.Exception.GetType().Name; $cloudErrorHResult=$_.Exception.HResult}; if($cloudErrorType -ne 'UnauthorizedAccessException' -or $cloudErrorHResult -ne -2147024891){[Console]::Error.WriteLine('OneDrive Cloud Filesの読取拒否が想定外です'); exit 61};"#,
            )
            .replace("__D4P_CLOUD_FILE_PATH__", &quote_path(path))
        } else {
            String::new()
        };
        let mut denied_paths = denied_paths
            .iter()
            .map(|path| quote_path(&workspace.join(path)))
            .collect::<Vec<_>>();
        if let Some(path) = cloud_file.as_ref() {
            denied_paths.push(quote_path(path));
        }
        let denied_array = denied_paths.join(",");
        let denied_aliases = [
            workspace.join(r"PRIVATE\REGISTERED-MARKER.TXT"),
            deep_secret.clone(),
        ]
        .iter()
        .map(|path| quote_path(path))
        .collect::<Vec<_>>()
        .join(",");
        let decoys = [
            quote_path(&private.join("vaulta.txt")),
            quote_path(&private.join("vaultx.txt")),
        ]
        .join(",");
        let write_denied = [
            private.join("registered-write-target.txt"),
            private.join("registered-directory").join("write-child.txt"),
            workspace.join(r"PRIVATE\REGISTERED-MARKER.TXT"),
            workspace.join(&deep_secret_relative),
        ]
        .iter()
        .map(|path| quote_path(path))
        .collect::<Vec<_>>();
        let write_denied = write_denied.join(",");
        let workspace_output = workspace.join("workspace-write-marker.txt");
        let environment_report = workspace.join("sandbox-environment-report.txt");
        let hardlink_alias = workspace.join("synthetic-secret-hardlink-alias.txt");
        let script = format!(
            "$probeLabel='合成path検査'; $ErrorActionPreference='Stop'; [IO.File]::WriteAllText({},'ready'); $deadline=[DateTime]::UtcNow.AddSeconds(90); while(-not [IO.File]::Exists({})){{if([DateTime]::UtcNow -ge $deadline){{exit 46}}; Start-Sleep -Milliseconds 25}}; $denied=@({}); foreach($p in $denied){{try{{$null=Get-Content -Raw -LiteralPath $p -ErrorAction Stop; exit 41}}catch{{}}}}; {}; $peerReadError=0; try{{$null=Get-Content -Raw -LiteralPath {} -ErrorAction Stop}}catch{{$peerReadError=$_.Exception.HResult}}; if($peerReadError -ne -2147024891){{[Console]::Error.WriteLine(\"peer_read_hresult=$peerReadError\"); exit 44}}; $peerWriteError=0; $peerWriteErrorType=''; try{{[IO.File]::WriteAllText({},'synthetic-agent-a-write')}}catch{{$peerWriteError=$_.Exception.InnerException.HResult; $peerWriteErrorType=$_.Exception.InnerException.GetType().Name}}; if($peerWriteErrorType -ne 'UnauthorizedAccessException' -or $peerWriteError -ne -2147024891){{[Console]::Error.WriteLine(\"peer_write_type=$peerWriteErrorType; peer_write_hresult=$peerWriteError; target_exists=$([IO.File]::Exists({}))\"); exit 45}}; [IO.File]::WriteAllText({},'checked'); $deadline=[DateTime]::UtcNow.AddSeconds(90); while(-not [IO.File]::Exists({})){{if([DateTime]::UtcNow -ge $deadline){{exit 47}}; Start-Sleep -Milliseconds 25}}; $deniedAliases=@({}); $aliasIndex=0; foreach($p in $deniedAliases){{$aliasIndex++; try{{$null=Get-Content -Raw -LiteralPath $p -ErrorAction Stop; exit (43+$aliasIndex)}}catch{{}}}}; $decoys=@({}); foreach($p in $decoys){{$null=Get-Content -Raw -LiteralPath $p -ErrorAction Stop}}; $writeDenied=@({}); foreach($p in $writeDenied){{try{{[IO.File]::WriteAllText($p,'synthetic-write'); exit 42}}catch{{}}}}; $hardlinkState='creation-denied'; $hardlinkErrorType=''; $hardlinkErrorHResult=0; try{{New-Item -ItemType HardLink -Path {} -Target {} -ErrorAction Stop | Out-Null; try{{$null=Get-Content -Raw -LiteralPath {} -ErrorAction Stop; $hardlinkState='created-readable'}}catch{{$hardlinkState='created-read-denied'}}}}catch{{$hardlinkErrorType=$_.Exception.GetType().Name; $hardlinkErrorHResult=$_.Exception.HResult}}; $expectedScratch=[IO.Path]::GetFullPath({}); $tempMatchesScratch=[string]::Equals([IO.Path]::GetFullPath($env:TEMP),$expectedScratch,[StringComparison]::OrdinalIgnoreCase); $tmpMatchesScratch=[string]::Equals([IO.Path]::GetFullPath($env:TMP),$expectedScratch,[StringComparison]::OrdinalIgnoreCase); [IO.File]::WriteAllText({},'workspace-write-marker'); [IO.File]::WriteAllText({},\"peerReadHResult=$peerReadError`npeerWriteException=$peerWriteErrorType`npeerWriteHResult=$peerWriteError`nhardlinkState=$hardlinkState`nhardlinkErrorType=$hardlinkErrorType`nhardlinkErrorHResult=$hardlinkErrorHResult`nTEMP作業領域一致=$tempMatchesScratch`nTMP作業領域一致=$tmpMatchesScratch\"); exit 0",
            quote_path(&workspace_ready),
            quote_path(&workspace_start),
            denied_array,
            cloud_read_probe,
            quote_path(&peer_marker),
            quote_path(&peer_write_target),
            quote_path(&peer_write_target),
            quote_path(&workspace_checked),
            quote_path(&workspace_continue),
            denied_aliases,
            decoys,
            write_denied,
            quote_path(&hardlink_alias),
            quote_path(&private.join("registered-marker.txt")),
            quote_path(&hardlink_alias),
            quote_path(&scratch),
            quote_path(&workspace_output),
            quote_path(&environment_report)
        );
        let workspace_checked_command = format!(
            "<# 作業領域検査の同期点 #>[IO.File]::WriteAllText({},'checked');",
            quote_path(&workspace_checked)
        );
        let agent_a_temp_checkpoint = format!(
            "<# child自身の一時領域への書込と読戻し #>$tempMarkerA=Join-Path ([IO.Path]::GetFullPath($env:TEMP)) '{}'; [IO.File]::WriteAllText($tempMarkerA,'synthetic-agent-a-temp'); if([IO.File]::ReadAllText($tempMarkerA) -ne 'synthetic-agent-a-temp'){{exit 50}}; {}",
            agent_a_temp_marker,
            workspace_checked_command
        );
        let script = script.replacen(&workspace_checked_command, &agent_a_temp_checkpoint, 1);
        let agent_a_temp_peer_probe = r#"; $tempPeerReadHResult=0; $tempPeerReadExceptionType=''; try { $tempPeerValue=[IO.File]::ReadAllText((Join-Path ([IO.Path]::GetFullPath($env:TEMP)) '__GUI_SHELL_PEER_TEMP_MARKER__')); if($tempPeerValue -ne 'synthetic-agent-b-temp'){exit 48} } catch { $tempError=$_.Exception.InnerException; if($null -eq $tempError){$tempError=$_.Exception}; $tempPeerReadHResult=$tempError.HResult; $tempPeerReadExceptionType=$tempError.GetType().FullName }; if(($tempPeerReadHResult -ne -2147024894) -and ($tempPeerReadHResult -ne -2147024891)){[Console]::Error.WriteLine("peer-temp-hresult=$tempPeerReadHResult; exception=$tempPeerReadExceptionType"); exit 49}; $deniedAliases=@("#
            .replace("__GUI_SHELL_PEER_TEMP_MARKER__", &agent_b_temp_marker);
        let script = script.replacen("; $deniedAliases=@(", &agent_a_temp_peer_probe, 1);
        let agent_a_temp_report = format!(
            "; <# 相手一時領域の拒否結果を合成Workspaceへ保存 #>[IO.File]::AppendAllText({},\"`npeerTempReadHResult=$tempPeerReadHResult`npeerTempReadException=$tempPeerReadExceptionType\"); exit 0",
            quote_path(&environment_report)
        );
        let script = script.replacen("; exit 0", &agent_a_temp_report, 1);

        let peer_script = r#"
$ErrorActionPreference = 'Stop'
[IO.File]::WriteAllText(__GUI_SHELL_PEER_READY__, 'ready')
$deadline = [DateTime]::UtcNow.AddSeconds(90)
while (-not [IO.File]::Exists(__GUI_SHELL_PEER_START__)) {
    if ([DateTime]::UtcNow -ge $deadline) { exit 56 }
    Start-Sleep -Milliseconds 25
}
$peerReadError = 0
try { $null = Get-Content -Raw -LiteralPath __GUI_SHELL_WORKSPACE_MARKER__ -ErrorAction Stop }
catch { $peerReadError = $_.Exception.HResult }
if ($peerReadError -ne -2147024891) {
    [Console]::Error.WriteLine("相手Workspace読取HRESULT=$peerReadError")
    exit 54
}
$peerWriteError = 0
$peerWriteErrorType = ''
try { [IO.File]::WriteAllText(__GUI_SHELL_WORKSPACE_WRITE_TARGET__, 'synthetic-agent-b-write') }
catch {
    $peerWriteError = $_.Exception.InnerException.HResult
    $peerWriteErrorType = $_.Exception.InnerException.GetType().Name
}
if ($peerWriteErrorType -ne 'UnauthorizedAccessException' -or $peerWriteError -ne -2147024891) {
    [Console]::Error.WriteLine("相手Workspace書込例外型=$peerWriteErrorType; HRESULT=$peerWriteError; 書込先存在=$([IO.File]::Exists(__GUI_SHELL_WORKSPACE_WRITE_TARGET__))")
    exit 55
}
[IO.File]::WriteAllText(__GUI_SHELL_PEER_OWN_WRITE__, 'agent-b-own-write')
$tempMarkerB = Join-Path ([IO.Path]::GetFullPath($env:TEMP)) '__GUI_SHELL_TEMP_OWN_MARKER__'
[IO.File]::WriteAllText($tempMarkerB, 'synthetic-agent-b-temp')
if ([IO.File]::ReadAllText($tempMarkerB) -ne 'synthetic-agent-b-temp') { exit 58 }
[IO.File]::WriteAllText(__GUI_SHELL_PEER_CHECKED__, 'checked')
$deadline = [DateTime]::UtcNow.AddSeconds(90)
while (-not [IO.File]::Exists(__GUI_SHELL_PEER_CONTINUE__)) {
    if ([DateTime]::UtcNow -ge $deadline) { exit 57 }
    Start-Sleep -Milliseconds 25
}
$tempPeerReadHResult = 0
$tempPeerReadExceptionType = ''
try {
    $tempPeerValue = [IO.File]::ReadAllText((Join-Path ([IO.Path]::GetFullPath($env:TEMP)) '__GUI_SHELL_TEMP_PEER_MARKER__'))
    if ($tempPeerValue -ne 'synthetic-agent-a-temp') { exit 59 }
} catch {
    $tempError = $_.Exception.InnerException
    if ($null -eq $tempError) { $tempError = $_.Exception }
    $tempPeerReadHResult = $tempError.HResult
    $tempPeerReadExceptionType = $tempError.GetType().FullName
}
if (($tempPeerReadHResult -ne -2147024894) -and ($tempPeerReadHResult -ne -2147024891)) {
    [Console]::Error.WriteLine("peer-temp-hresult=$tempPeerReadHResult; exception=$tempPeerReadExceptionType")
    exit 60
}
[IO.File]::WriteAllText(__GUI_SHELL_PEER_REPORT__, "相手Workspace読取HRESULT=$peerReadError`n相手Workspace書込例外型=$peerWriteErrorType`n相手Workspace書込HRESULT=$peerWriteError`n相手TEMP読取HRESULT=$tempPeerReadHResult`n相手TEMP読取例外=$tempPeerReadExceptionType")
exit 0
"#
        .replace("__GUI_SHELL_PEER_READY__", &quote_path(&peer_ready))
        .replace("__GUI_SHELL_PEER_START__", &quote_path(&peer_start))
        .replace(
            "__GUI_SHELL_WORKSPACE_MARKER__",
            &quote_path(&workspace_marker),
        )
        .replace(
            "__GUI_SHELL_WORKSPACE_WRITE_TARGET__",
            &quote_path(&workspace_write_target),
        )
        .replace(
            "__GUI_SHELL_PEER_OWN_WRITE__",
            &quote_path(&peer_workspace.join("agent-b-own-write.txt")),
        )
        .replace("__GUI_SHELL_PEER_CHECKED__", &quote_path(&peer_checked))
        .replace("__GUI_SHELL_PEER_CONTINUE__", &quote_path(&peer_continue))
        .replace("__GUI_SHELL_PEER_REPORT__", &quote_path(&peer_write_result))
        .replace("__GUI_SHELL_TEMP_OWN_MARKER__", &agent_b_temp_marker)
        .replace("__GUI_SHELL_TEMP_PEER_MARKER__", &agent_a_temp_marker);

        let task_environment = generated
            .get_envs()
            .filter_map(|(name, value)| {
                let value = value?;
                matches!(name.to_str(), Some("TEMP" | "TMP"))
                    .then(|| (name.to_os_string(), value.to_os_string()))
            })
            .collect::<Vec<_>>();
        assert_eq!(task_environment.len(), 2, "Rust生成TaskのTEMP／TMP値がある");
        let mut sandbox = command(&executable, &workspace);
        sandbox
            .env("CODEX_HOME", &isolated_codex_home)
            .envs(task_environment.iter().map(|(name, value)| (name, value)))
            .args(["sandbox"])
            .args(["--permission-profile", &permission_profile])
            .args(&profile_args)
            .args(["--cd"])
            .arg(&workspace)
            .args([
                "powershell.exe",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
            ])
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut peer_sandbox = command(&executable, &peer_workspace);
        peer_sandbox
            .env("CODEX_HOME", &isolated_codex_home)
            .envs(
                peer_task_environment
                    .iter()
                    .map(|(name, value)| (name, value)),
            )
            .args(["sandbox"])
            .args(["--permission-profile", &peer_permission_profile])
            .args(&peer_profile_args)
            .args(["--cd"])
            .arg(&peer_workspace)
            .args([
                "powershell.exe",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
            ])
            .arg(peer_script)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        fn terminate_pair(
            first: &mut super::process_tree::SupervisedChild,
            second: &mut super::process_tree::SupervisedChild,
        ) {
            let _ = first.terminate_tree();
            let _ = second.terminate_tree();
        }

        fn read_pipe_lossy<R: std::io::Read>(mut pipe: R) -> std::io::Result<String> {
            let mut bytes = Vec::new();
            pipe.read_to_end(&mut bytes)?;
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        }

        fn process_output(child: &mut super::process_tree::SupervisedChild) -> (String, String) {
            let mut stdout = String::new();
            let mut stderr = String::new();
            if let Some(pipe) = child.child.stdout.take() {
                stdout = read_pipe_lossy(pipe).unwrap_or_default();
            }
            if let Some(pipe) = child.child.stderr.take() {
                stderr = read_pipe_lossy(pipe).unwrap_or_default();
            }
            (stdout, stderr)
        }

        fn wait_for_markers(
            first: &mut super::process_tree::SupervisedChild,
            second: &mut super::process_tree::SupervisedChild,
            markers: [&Path; 2],
            phase: &str,
        ) {
            let deadline = Instant::now() + Duration::from_secs(100);
            loop {
                match first.try_wait() {
                    Ok(Some(status)) => {
                        terminate_pair(first, second);
                        let (stdout, stderr) = process_output(first);
                        panic!("同時MxC試験の{phase}前にAgent Aが終了: {status}; stdout={stdout}; stderr={stderr}");
                    }
                    Ok(None) => {}
                    Err(error) => {
                        terminate_pair(first, second);
                        panic!("同時MxC試験の{phase}でAgent A状態を読めない: {error}");
                    }
                }
                match second.try_wait() {
                    Ok(Some(status)) => {
                        terminate_pair(first, second);
                        let (stdout, stderr) = process_output(second);
                        panic!("同時MxC試験の{phase}前にAgent Bが終了: {status}; stdout={stdout}; stderr={stderr}");
                    }
                    Ok(None) => {}
                    Err(error) => {
                        terminate_pair(first, second);
                        panic!("同時MxC試験の{phase}でAgent B状態を読めない: {error}");
                    }
                }
                if markers.iter().all(|marker| marker.exists()) {
                    return;
                }
                if Instant::now() >= deadline {
                    terminate_pair(first, second);
                    panic!("同時MxC試験の{phase}同期が期限超過");
                }
                thread::sleep(Duration::from_millis(25));
            }
        }

        let mut sandbox = process_tree::spawn(sandbox).expect("Agent Aの実Codex sandboxを起動");
        let mut peer_sandbox = match process_tree::spawn(peer_sandbox) {
            Ok(child) => child,
            Err(error) => {
                let _ = sandbox.terminate_tree();
                panic!("Agent Bの実Codex sandboxを起動できない: {error}");
            }
        };
        wait_for_markers(
            &mut sandbox,
            &mut peer_sandbox,
            [&workspace_ready, &peer_ready],
            "開始",
        );
        fs::write(&workspace_start, b"go").expect("Agent Aへ同時試験開始を通知");
        fs::write(&peer_start, b"go").expect("Agent Bへ同時試験開始を通知");
        wait_for_markers(
            &mut sandbox,
            &mut peer_sandbox,
            [&workspace_checked, &peer_checked],
            "相互Workspace検査",
        );
        fs::write(&workspace_continue, b"continue").expect("Agent Aの後続path検査を解放");
        fs::write(&peer_continue, b"continue").expect("Agent Bの後続path検査を解放");

        let deadline = Instant::now() + Duration::from_secs(100);
        let (status, peer_status) = loop {
            let status = sandbox.try_wait().expect("Agent Aのsandbox終了状態");
            let peer_status = peer_sandbox.try_wait().expect("Agent Bのsandbox終了状態");
            if let (Some(status), Some(peer_status)) = (status, peer_status) {
                break (status, peer_status);
            }
            if Instant::now() >= deadline {
                terminate_pair(&mut sandbox, &mut peer_sandbox);
                panic!("同時MxC試験の終了待ちが期限超過");
            }
            thread::sleep(Duration::from_millis(25));
        };
        sandbox
            .stop_descendants()
            .expect("Agent Aの残存process群を停止");
        peer_sandbox
            .stop_descendants()
            .expect("Agent Bの残存process群を停止");
        let mut stdout = String::new();
        let mut stderr = String::new();
        if let Some(pipe) = sandbox.child.stdout.take() {
            stdout = read_pipe_lossy(pipe).expect("Agent Aの標準出力を取得");
        }
        if let Some(pipe) = sandbox.child.stderr.take() {
            stderr = read_pipe_lossy(pipe).expect("Agent Aの標準errorを取得");
        }
        let mut peer_stdout = String::new();
        let mut peer_stderr = String::new();
        if let Some(pipe) = peer_sandbox.child.stdout.take() {
            peer_stdout = read_pipe_lossy(pipe).expect("Agent Bの標準出力を取得");
        }
        if let Some(pipe) = peer_sandbox.child.stderr.take() {
            peer_stderr = read_pipe_lossy(pipe).expect("Agent Bの標準errorを取得");
        }
        assert!(
            status.success(),
            "Agent AのRust生成profile probeが失敗: code={:?}, stdout={stdout}, stderr={stderr}",
            status.code(),
        );
        assert!(
            peer_status.success(),
            "Agent BのRust生成profile probeが失敗: code={:?}, stdout={peer_stdout}, stderr={peer_stderr}",
            peer_status.code(),
        );
        if let Some(path) = cloud_file.as_ref() {
            const FILE_ATTRIBUTE_UNPINNED: u32 = 0x0010_0000;
            const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
            let attributes = std::fs::symlink_metadata(path)
                .expect("Task後もOneDrive合成Cloud File metadata")
                .file_attributes();
            assert_ne!(
                attributes & FILE_ATTRIBUTE_UNPINNED,
                0,
                "Task後も合成Cloud Fileがonline-onlyのままである"
            );
            assert_ne!(
                attributes & FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
                0,
                "Agentの拒否probeでCloud Fileがhydrateされない"
            );
        }
        let sequential_temp_report = workspace.join("sandbox-sequential-temp-report.txt");
        let sequential_observer_script = r#"
<# 先行Taskの一時目印を後続childから再読できるか照合する #>
$ErrorActionPreference = 'Stop'
function Get-PreviousMarkerState {
    param([string]$MarkerPath, [string]$Expected)
    try {
        $actual = [IO.File]::ReadAllText($MarkerPath)
        if ($actual -eq $Expected) { return 'readable' }
        return 'changed'
    } catch {
        $probeError = $_.Exception.InnerException
        if ($null -eq $probeError) { $probeError = $_.Exception }
        return "$($probeError.GetType().FullName)/$($probeError.HResult)"
    }
}
$tempRoot = [IO.Path]::GetFullPath($env:TEMP)
$markerA = Join-Path $tempRoot '__GUI_SHELL_PREVIOUS_TEMP_A__'
$markerB = Join-Path $tempRoot '__GUI_SHELL_PREVIOUS_TEMP_B__'
$stateA = Get-PreviousMarkerState -MarkerPath $markerA -Expected 'synthetic-agent-a-temp'
$stateB = Get-PreviousMarkerState -MarkerPath $markerB -Expected 'synthetic-agent-b-temp'
[IO.File]::WriteAllText(__GUI_SHELL_SEQUENTIAL_REPORT__, "agent_a=$stateA`nagent_b=$stateB")
exit 0
"#
        .replace("__GUI_SHELL_PREVIOUS_TEMP_A__", &agent_a_temp_marker)
        .replace("__GUI_SHELL_PREVIOUS_TEMP_B__", &agent_b_temp_marker)
        .replace(
            "__GUI_SHELL_SEQUENTIAL_REPORT__",
            &quote_path(&sequential_temp_report),
        );
        let mut sequential_observer = command(&executable, &workspace);
        sequential_observer
            .env("CODEX_HOME", &isolated_codex_home)
            .envs(task_environment.iter().map(|(name, value)| (name, value)))
            .args(["sandbox", "--permission-profile", &permission_profile])
            .args(&profile_args)
            .args(["--cd"])
            .arg(&workspace)
            .args([
                "powershell.exe",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
            ])
            .arg(sequential_observer_script)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut sequential_observer = process_tree::spawn(sequential_observer)
            .expect("同一CODEX_HOMEの後続MxC observerを起動");
        let sequential_deadline = Instant::now() + Duration::from_secs(100);
        let sequential_status = loop {
            if let Some(status) = sequential_observer
                .try_wait()
                .expect("後続MxC observerの終了状態")
            {
                break status;
            }
            if Instant::now() >= sequential_deadline {
                let _ = sequential_observer.terminate_tree();
                panic!("後続MxC observerが期限超過");
            }
            thread::sleep(Duration::from_millis(25));
        };
        sequential_observer
            .stop_descendants()
            .expect("後続MxC observerの残存process群を停止");
        let (sequential_stdout, sequential_stderr) = process_output(&mut sequential_observer);
        assert!(
            sequential_status.success(),
            "後続MxC observerが失敗: code={:?}, stdout={sequential_stdout}, stderr={sequential_stderr}",
            sequential_status.code()
        );
        let sequential_temp_result =
            fs::read_to_string(sequential_temp_report).expect("後続MxC observerのTEMP照合結果");
        let sequential_states = ["agent_a=", "agent_b="].map(|prefix| {
            sequential_temp_result
                .lines()
                .find_map(|line| line.strip_prefix(prefix))
                .unwrap_or_else(|| {
                    panic!("後続MxC observer結果が欠落: {prefix}={sequential_temp_result}")
                })
        });
        assert!(
            sequential_states.iter().all(|state| {
                *state == "System.IO.FileNotFoundException/-2147024894"
                    || *state == "System.UnauthorizedAccessException/-2147024891"
            }),
            "同一CODEX_HOMEの後続MxC childが先行Task TEMP目印を読めない: {sequential_temp_result}"
        );
        eprintln!("同一CODEX_HOMEの後続MxC TEMP照合: {sequential_temp_result}");
        assert_eq!(
            fs::read(workspace_output).expect("Workspace内write結果"),
            b"workspace-write-marker"
        );
        assert_eq!(
            fs::read(&peer_marker).expect("別Agent Workspaceのmarker"),
            b"synthetic-agent-b-marker",
            "Task sandboxは隣接する別Agent Workspaceを読み取れない"
        );
        assert!(
            !peer_write_target.exists(),
            "Task sandboxは隣接する別Agent Workspaceへ書き込めない"
        );
        assert_eq!(
            fs::read(&workspace_marker).expect("Agent Aの作業領域確認file"),
            b"synthetic-agent-a-marker",
            "同時実行中のAgent BがAgent A markerを変更しない"
        );
        assert!(
            !workspace_write_target.exists(),
            "同時実行中のAgent B sandboxはAgent A Workspaceへ書き込めない"
        );
        assert_eq!(
            fs::read(peer_workspace.join("agent-b-own-write.txt"))
                .expect("Agent B自身のWorkspace書込み"),
            b"agent-b-own-write",
            "Agent Bは自身のWorkspaceへ書き込める"
        );
        let peer_write_result =
            fs::read_to_string(peer_write_result).expect("Agent B相互隔離の観測結果");
        assert!(
            peer_write_result.contains("相手Workspace読取HRESULT=-2147024891")
                && peer_write_result
                    .contains("相手Workspace書込例外型=UnauthorizedAccessException")
                && peer_write_result.contains("相手Workspace書込HRESULT=-2147024891"),
            "Agent Bも同時実行中にAgent Aの読取・書込を拒否する: {peer_write_result}"
        );
        let environment_report =
            fs::read_to_string(environment_report).expect("sandbox内TEMP／TMP照合結果");
        let agent_a_temp_read_hresult = environment_report
            .lines()
            .find_map(|line| line.strip_prefix("peerTempReadHResult="))
            .expect("Agent Aの相手TEMP読取結果")
            .parse::<i32>()
            .expect("Agent Aの相手TEMP HRESULT");
        let agent_b_temp_read_hresult = peer_write_result
            .lines()
            .find_map(|line| line.strip_prefix("相手TEMP読取HRESULT="))
            .expect("Agent Bの相手TEMP読取結果")
            .parse::<i32>()
            .expect("Agent Bの相手TEMP HRESULT");
        let agent_a_temp_read_exception = environment_report
            .lines()
            .find_map(|line| line.strip_prefix("peerTempReadException="))
            .expect("Agent Aの相手TEMP例外型");
        let agent_b_temp_read_exception = peer_write_result
            .lines()
            .find_map(|line| line.strip_prefix("相手TEMP読取例外="))
            .expect("Agent Bの相手TEMP例外型");
        assert_ne!(
            agent_a_temp_read_hresult, 0,
            "同時実行中のAgent AがAgent BのMxC TEMP目印を読めるため、task間temporary隔離が漏れている"
        );
        assert_ne!(
            agent_b_temp_read_hresult, 0,
            "同時実行中のAgent BがAgent AのMxC TEMP目印を読めるため、task間temporary隔離が漏れている"
        );
        assert!(
            [agent_a_temp_read_hresult, agent_b_temp_read_hresult]
                .iter()
                .all(|code| *code == -2147024894 || *code == -2147024891),
            "相手TEMPはfile不在またはAccess Deniedで拒否される: Agent A={agent_a_temp_read_hresult}, Agent B={agent_b_temp_read_hresult}"
        );
        assert!(
            (agent_a_temp_read_hresult == -2147024894
                && agent_a_temp_read_exception == "System.IO.FileNotFoundException")
                || (agent_a_temp_read_hresult == -2147024891
                    && agent_a_temp_read_exception == "System.UnauthorizedAccessException"),
            "Agent Aの相手TEMP拒否はfile不在またはAccess Deniedである: HRESULT={agent_a_temp_read_hresult}, exception={agent_a_temp_read_exception}"
        );
        assert!(
            (agent_b_temp_read_hresult == -2147024894
                && agent_b_temp_read_exception == "System.IO.FileNotFoundException")
                || (agent_b_temp_read_hresult == -2147024891
                    && agent_b_temp_read_exception == "System.UnauthorizedAccessException"),
            "Agent Bの相手TEMP拒否はfile不在またはAccess Deniedである: HRESULT={agent_b_temp_read_hresult}, exception={agent_b_temp_read_exception}"
        );
        eprintln!(
            "同時MxC TEMP相互読取: Agent A={agent_a_temp_read_exception}/{agent_a_temp_read_hresult}, Agent B={agent_b_temp_read_exception}/{agent_b_temp_read_hresult}"
        );
        assert!(
            !private.join("registered-write-target.txt").exists(),
            "登録secret fileへの書込みを拒否する"
        );
        assert!(
            !registered_directory.join("write-child.txt").exists(),
            "登録secret directory内への新規書込みを拒否する"
        );
        assert_eq!(
            fs::read(private.join("registered-marker.txt")).expect("合成secretの再読"),
            b"synthetic-secret-marker",
            "case alias probeが元の合成secretを変更しない"
        );
        assert_eq!(
            fs::read(&deep_secret).expect("深さ40の登録secret再読"),
            b"synthetic-deep-secret-marker",
            "深い登録secretへのwriteを拒否する"
        );
        eprintln!("MxC直接sandboxの合成観測: {environment_report}");
        assert!(
            environment_report.contains("hardlinkState=created-read-denied")
                || environment_report.contains("hardlinkState=creation-denied"),
            "MxC childが作成したhardlink aliasから登録secretを読めない: {environment_report}"
        );
        if environment_report.contains("hardlinkState=creation-denied") {
            assert!(
                environment_report.contains("hardlinkErrorType=UnauthorizedAccessException")
                    && environment_report.contains("hardlinkErrorHResult=-2147024891"),
                "hardlink作成拒否がWindows access deniedである: {environment_report}"
            );
        }
        assert!(
            environment_report.contains("TEMP作業領域一致=False"),
            "MxC子processのTEMPはRust生成Task scratchと異なる: {environment_report}"
        );
        assert!(
            environment_report.contains("TMP作業領域一致=False"),
            "MxC子processのTMPはRust生成Task scratchと異なる: {environment_report}"
        );
    }

    #[test]
    fn 登録secret_globは不正pathと上限超過を拒否する() {
        for invalid in ["../outside", "private/*.txt", "private/has\\slash"] {
            assert_eq!(
                task_filesystem_override(&[invalid.into()]),
                Err(対話失敗::要求不正)
            );
        }
        let too_many = (0..=MAX_REGISTERED_SECRET_PATHS)
            .map(|index| format!("private/secret-{index}.txt"))
            .collect::<Vec<_>>();
        assert_eq!(task_filesystem_override(&too_many), Err(対話失敗::要求不正));

        let too_large = (0..64)
            .map(|index| format!("private/{index:03}-{}", "x".repeat(240)))
            .collect::<Vec<_>>();
        assert_eq!(
            task_filesystem_override(&too_large),
            Err(対話失敗::要求不正)
        );
    }

    #[cfg(windows)]
    #[test]
    fn 登録後に増えたsecret_hardlink_aliasはTask起動前に拒否する() {
        use std::io::Write;

        let root = codex_cli_fixture::FixtureTempDirectory::create();
        let workspace = root.path().join("workspace");
        let private = workspace.join("private");
        std::fs::create_dir_all(&private).unwrap();
        let mut secret_file = std::fs::File::create(private.join("secret.txt")).unwrap();
        secret_file.write_all(b"synthetic secret").unwrap();
        let identity = crate::broker::workspace_root::pin_workspace_path(&workspace)
            .unwrap()
            .identity;
        let secrets = ["private/secret.txt".to_owned()];

        assert!(validate_task_secret_paths(&workspace, identity, &secrets).is_ok());
        std::fs::hard_link(
            private.join("secret.txt"),
            workspace.join("public-alias.txt"),
        )
        .unwrap();
        assert_eq!(
            validate_task_secret_paths(&workspace, identity, &secrets),
            Err(対話失敗::作業領域不在),
            "Task実行直前に追加されたhardlink aliasを拒否する"
        );

        let mut adapter =
            CodexCliAdapter::for_test(root.path().join("missing-codex.exe"), workspace.clone());
        adapter.workspace_identity = identity;
        let mut context = scratch_context(identity);
        context.secret_paths = secrets.to_vec();
        assert!(adapter.AgentTask実行対応());
        assert_eq!(
            adapter.AgentTask実行(
                "synthetic fixture task",
                &AtomicBool::new(false),
                Instant::now() + Duration::from_secs(30),
                Some(context),
            ),
            Err(対話失敗::作業領域不在),
            "実Task入口でaliasを拒否し、未存在CLIの起動へ進まない"
        );
    }

    #[cfg(windows)]
    #[test]
    fn scratch作成後に増えたsecret_hardlink_aliasはCLI起動直前に拒否する() {
        use std::io::Write;

        let root = codex_cli_fixture::FixtureTempDirectory::create();
        let workspace = root.path().join("workspace");
        let private = workspace.join("private");
        std::fs::create_dir_all(&private).unwrap();
        let mut secret_file = std::fs::File::create(private.join("secret.txt")).unwrap();
        secret_file.write_all(b"synthetic secret").unwrap();
        let identity = crate::broker::workspace_root::pin_workspace_path(&workspace)
            .unwrap()
            .identity;
        let secrets = ["private/secret.txt".to_owned()];
        let mut context = scratch_context(identity);
        context.secret_paths = secrets.to_vec();
        let mut scratch = WorkspaceTaskScratch::create(&workspace, identity, &context).unwrap();

        std::fs::hard_link(
            private.join("secret.txt"),
            workspace.join("public-alias.txt"),
        )
        .unwrap();

        assert!(matches!(
            spawn_codex_task(
                &root.path().join("missing-codex.exe"),
                &workspace,
                identity,
                "synthetic fixture task",
                None,
                CodexSandbox::WorkspaceWrite,
                Some(&scratch),
                &secrets,
                None,
            ),
            Err(対話失敗::作業領域不在)
        ));
        scratch.cleanup().unwrap();
        assert!(!context.journal.has_pending_workspace("workspace-fixture"));
    }

    #[test]
    fn TaskContextのDebugは登録secretのpath名を隠す() {
        let context = scratch_context(crate::broker::workspace_root::DirectoryIdentity {
            device: 1,
            file_id: 1,
        });
        let debug = debug_task_context(&context);
        assert!(debug.contains("secret_path_count: 1"));
        assert!(!debug.contains("credential-backup"));
    }

    #[test]
    fn read_onlyは維持しworkspace_writeはTaskだけに要求する() {
        let help = b"Usage: codex exec [OPTIONS] [PROMPT]\n--sandbox [read-only]\n--model MODEL\n--cd DIR\n--json\n--ephemeral\n--ignore-user-config\n--skip-git-repo-check";
        assert!(exec_interface_present(help));
        assert!(!workspace_write_interface_present(help));
        let without_non_git_flag =
            b"Usage: codex exec [OPTIONS] [PROMPT]\n--json\n--model MODEL\n--ephemeral\n--ignore-user-config";
        assert!(!exec_interface_present(without_non_git_flag));
        let adapter = CodexCliAdapter {
            executable: PathBuf::from(r"C:\codex.exe"),
            workspace: PathBuf::from(r"C:\workspace"),
            workspace_identity: crate::broker::workspace_root::DirectoryIdentity {
                device: 1,
                file_id: 1,
            },
            workspace_write_interface: false,
            version: "test".into(),
            model_id: None,
            test_responses_api: None,
        };
        assert!(!adapter.AgentTask実行対応());
        assert_eq!(
            adapter.AgentTask実行("test", &AtomicBool::new(false), Instant::now(), None),
            Err(対話失敗::AgentTask非対応)
        );
        let help = "codex execの能力検査用fixture\nUsage: codex exec [OPTIONS] [PROMPT]\n--sandbox [read-only, workspace-write]\n--model MODEL\n--cd DIR\n--json\n--ephemeral\n--ignore-user-config\n--skip-git-repo-check";
        assert!(exec_interface_present(help.as_bytes()));
        assert!(workspace_write_interface_present(help.as_bytes()));
    }

    #[test]
    #[allow(non_snake_case)]
    fn CodexAdapterのmetadataはAuthority入力検査に誤拒否されない() {
        let adapter = CodexCliAdapter::for_test(
            PathBuf::from(r"C:\codex.exe"),
            PathBuf::from(r"C:\workspace"),
        );
        let metadata = adapter.agent_metadata().expect("登録Agent情報を取得");
        assert!(
            !crate::broker::protocol::metadata_attempts_authority_value(&metadata),
            "Capabilityの説明文をAuthority field/valueと誤認しない: {metadata}"
        );
        let task_execution = metadata["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .find(|capability| capability["capability_id"] == "task_execution")
            .unwrap();
        assert_eq!(task_execution["support"]["status"], "unsupported");
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

    #[test]
    fn scratchのjournal有効化失敗では未起動directoryを回収する() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("時計")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "gui-shell-agent-task-scratch-activation-failure-{}-{nonce}",
            std::process::id()
        ));
        let workspace = root.join("workspace");
        std::fs::create_dir_all(&workspace).expect("Workspace試験root");
        let identity = crate::broker::workspace_root::pin_workspace_path(&workspace)
            .expect("Workspaceを固定")
            .identity;
        let context = scratch_context(identity);
        let name = format!(".d4p-tmp-{}", "8".repeat(32));
        let record_id = context
            .journal
            .reserve(
                &context.task_id,
                &context.runtime_id,
                &context.workspace_id,
                &context.recovery_binding_hash,
                identity,
                &name,
            )
            .expect("scratch回復予約");
        let workspace_dir = Dir::open_ambient_dir(&workspace, cap_std::ambient_authority())
            .expect("作業領域directoryの操作口を開く");
        workspace_dir.create_dir(&name).expect("未起動scratch");
        fs::write(
            workspace.join(&name).join("fixture.tmp"),
            "試験用一時領域の内容".as_bytes(),
        )
        .expect("未起動scratch内fixture");
        let scratch_dir = workspace_dir
            .open_dir_nofollow(&name)
            .expect("一時領域の操作口を開く");

        assert!(matches!(
            activate_task_scratch_directory(
                scratch_dir,
                crate::broker::workspace_root::DirectoryIdentity {
                    device: identity.device,
                    file_id: 0,
                },
                &context,
                &record_id,
            ),
            Err(対話失敗::通信失敗)
        ));
        assert!(!workspace.join(&name).exists());
        assert!(!context.journal.has_pending_workspace("workspace-fixture"));
        drop(workspace_dir);
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
    fn 選択Modelが偽CodexCLIのTask実行へ届く() {
        let root = codex_cli_fixture::FixtureTempDirectory::create();
        let fixture_directory = root.path().join("fixture");
        let workspace = root.path().join("workspace");
        fs::create_dir(&fixture_directory).expect("偽CLI用領域");
        fs::create_dir(&workspace).expect("登録作業領域");
        let executable = codex_cli_fixture::compile_fake_codex_cli(&fixture_directory);
        let identity = crate::broker::workspace_root::pin_workspace_path(&workspace)
            .expect("登録Workspace identity")
            .identity;
        let adapter = CodexCliAdapter::new_with_model(
            &executable,
            &workspace,
            "model-fixture-v1",
        )
        .expect("偽CLIがModel指定interfaceを公開する");
        let context = scratch_context(identity);
        let completed = adapter.AgentTask実行(
            "Provider / Model選択のfixture Task",
            &AtomicBool::new(false),
            Instant::now() + Duration::from_secs(5),
            Some(context.clone()),
        );
        assert_eq!(
            completed,
            Ok("fixture-task-completed:model-fixture-v1".into())
        );
        codex_cli_fixture::assert_no_workspace_task_scratch(&workspace);
        assert!(!context.journal.has_pending_workspace("workspace-fixture"));
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
        let cancellation_requested = AtomicBool::new(true);
        let expired_before_start = adapter.AgentTask実行(
            "同時期限・取消試験用Task",
            &cancellation_requested,
            Instant::now() - Duration::from_secs(1),
            Some(context.clone()),
        );
        assert_eq!(
            expired_before_start,
            Err(対話失敗::期限超過),
            "deadline到達時のBroker取消flagをOwner取消へ誤分類しない"
        );
        codex_cli_fixture::assert_no_workspace_task_scratch(&workspace);
        assert!(!context.journal.has_pending_workspace("workspace-fixture"));

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
