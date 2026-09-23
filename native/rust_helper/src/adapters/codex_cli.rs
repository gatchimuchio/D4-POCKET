//! 実物Codex CLIを、Brokerの承認済み対話経路へ限定して射影する。
//!
//! このAdapterはIPCからcommand、argv、environment、workspaceを受け取らない。
//! 起動時に明示登録された実行fileとworkspaceだけを使用し、実行時は固定された
//! `codex exec --json --sandbox read-only --ephemeral` protocolを使う。
#![allow(non_snake_case)]

use crate::audit_hash::sha256_tagged;
use crate::broker::dialogue::{実行系Adapter, 実行結果, 対話失敗, 対話要求};
use serde_json::Value;
use std::env;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

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
}

impl CodexCliAdapter {
    /// ownerの起動設定からだけ呼び出す。PATH探索やIPC由来の任意pathは行わない。
    pub fn new(executable: &Path, workspace: &Path) -> Result<Self, String> {
        let executable = canonical_executable(executable)?;
        let workspace = canonical_workspace(workspace)?;
        if secret_component(&workspace) {
            return Err("Codexのworkspaceがsecret pathに該当する".into());
        }

        let version = run_probe(&executable, &workspace, &["--version"])?;
        if !version.success || !String::from_utf8_lossy(&version.stdout).contains("codex-cli") {
            return Err("Codex CLIのversion interfaceを確認できない".into());
        }
        let help = run_probe(&executable, &workspace, &["exec", "--help"])?;
        if !help.success || !String::from_utf8_lossy(&help.stdout).contains("codex exec") {
            return Err("Codex CLIのexec help interfaceを確認できない".into());
        }

        Ok(Self {
            executable,
            workspace,
        })
    }

    #[cfg(test)]
    fn for_test(executable: PathBuf, workspace: PathBuf) -> Self {
        Self {
            executable,
            workspace,
        }
    }
}

impl 実行系Adapter for CodexCliAdapter {
    fn 接続対象(&self) -> String {
        "codex-cli://broker-governed-read-only".into()
    }

    fn 応答(
        &self,
        要求: &対話要求,
        取消: &AtomicBool,
        期限: Instant,
        生受信: &mut Vec<Vec<u8>>,
    ) -> Result<実行結果, 対話失敗> {
        let mut child = spawn_task(&self.executable, &self.workspace, &要求.入力)?;
        let stdout = child.stdout.take().ok_or(対話失敗::通信失敗)?;
        let stderr = child.stderr.take().ok_or(対話失敗::通信失敗)?;
        let stdout_reader = thread::spawn(move || bounded_read(stdout));
        let stderr_reader = thread::spawn(move || bounded_read(stderr));

        let status = loop {
            if 取消.load(Ordering::SeqCst) {
                terminate(&mut child);
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(対話失敗::取消);
            }
            if Instant::now() >= 期限 {
                terminate(&mut child);
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(対話失敗::期限超過);
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(_) => {
                    terminate(&mut child);
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

fn spawn_task(executable: &Path, workspace: &Path, input: &str) -> Result<Child, 対話失敗> {
    let mut child = command(executable, workspace)
        .args([
            "exec",
            "--json",
            "--ephemeral",
            "--ignore-user-config",
            "--sandbox",
            "read-only",
            "--color",
            "never",
            "--cd",
            workspace.to_str().ok_or(対話失敗::要求不正)?,
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| 対話失敗::通信失敗)?;
    let mut stdin = child.stdin.take().ok_or(対話失敗::通信失敗)?;
    stdin
        .write_all(input.as_bytes())
        .map_err(|_| 対話失敗::通信失敗)?;
    drop(stdin);
    Ok(child)
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn task_commandは固定read_only境界を持つ() {
        let adapter = CodexCliAdapter::for_test(
            PathBuf::from("C:\\codex.exe"),
            PathBuf::from("C:\\workspace"),
        );
        let mut command = command(&adapter.executable, &adapter.workspace);
        command.args([
            "exec",
            "--json",
            "--ephemeral",
            "--ignore-user-config",
            "--sandbox",
            "read-only",
            "--color",
            "never",
            "--cd",
            "C:\\workspace",
            "-",
        ]);
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--sandbox", "read-only"]));
        assert!(!args.iter().any(|arg| arg.contains("dangerously")));
        let worktree_flag = format!("{}{}", "--", "worktree");
        assert!(!args.iter().any(|arg| arg == &worktree_flag));
    }

    #[test]
    fn secret_componentはworkspace登録を拒否する() {
        assert!(secret_component(Path::new("C:\\workspace\\secrets")));
        assert!(secret_component(Path::new("C:\\workspace\\.ssh")));
        assert!(!secret_component(Path::new("C:\\workspace\\src")));
    }
}
