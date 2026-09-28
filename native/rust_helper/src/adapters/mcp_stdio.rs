//! MCP stdio clientのprocess境界。
//!
//! このclientはowner起動設定から固定されたexecutable/workspaceだけを使う。受信した
//! Server metadataは`crate::mcp`で検証し、Tool実行やCredential実値の注入は行わない。
#![allow(non_snake_case)]

use crate::mcp::{
    legacy_initialize_request, modern_discover_request, notification_line, parse_discovery,
    parse_prompts_response, parse_resources_response, parse_tools_response, McpCatalog, McpError,
    McpJsonRpcMessage, McpProtocolEra,
};
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_STDIO_LINE_BYTES: usize = 256 * 1024;
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
];

#[derive(Debug)]
pub struct McpStdioConnection {
    child: super::process_tree::SupervisedChild,
    stdin: ChildStdin,
    lines: Receiver<Vec<u8>>,
    era: McpProtocolEra,
    catalog: McpCatalog,
}

impl McpStdioConnection {
    /// owner起動設定からだけ呼び出す。IPC payloadから任意processを直接起動する用途ではない。
    pub fn connect(
        executable: &Path,
        arguments: &[String],
        workspace: &Path,
    ) -> Result<Self, McpError> {
        let executable = canonical_executable(executable)?;
        let workspace = canonical_workspace(workspace)?;
        if secret_component(&workspace) {
            return Err(McpError::new(
                "mcp_workspace_secret_path",
                "MCP workspaceがsecret pathに該当する",
            ));
        }
        let mut connection = Self::spawn(&executable, arguments, &workspace)?;
        let modern = connection
            .request(1, modern_discover_request(1)?)
            .and_then(|message| parse_discovery(&message, McpProtocolEra::Modern));
        if modern.is_err() {
            connection.terminate()?;
            connection = Self::spawn(&executable, arguments, &workspace)?;
            let discovery = connection
                .request(1, legacy_initialize_request(1)?)
                .and_then(|message| parse_discovery(&message, McpProtocolEra::Legacy))?;
            let initialized = notification_line("notifications/initialized", json_object())?;
            connection.write_line(&initialized)?;
            connection.finish_catalog(discovery, 2, McpProtocolEra::Legacy)?;
        } else {
            let discovery = modern.expect("modern discoveryのErr確認済み");
            connection.finish_catalog(discovery, 2, McpProtocolEra::Modern)?;
        }
        if !connection.is_alive() {
            return Err(McpError::new(
                "mcp_server_unavailable",
                "MCP stdio Serverがcatalog取得後に終了した",
            ));
        }
        Ok(connection)
    }

    pub fn catalog(&self) -> &McpCatalog {
        &self.catalog
    }

    fn spawn(executable: &Path, arguments: &[String], workspace: &Path) -> Result<Self, McpError> {
        let mut command = Command::new(executable);
        command.args(arguments).current_dir(workspace).env_clear();
        for name in SAFE_ENVIRONMENT {
            if let Some(value) = env::var_os(name) {
                command.env(name, value);
            }
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = super::process_tree::spawn(command).map_err(|_| {
            McpError::new(
                "mcp_server_unavailable",
                "監督下でMCP stdio Serverを起動できない",
            )
        })?;
        let Some(stdin) = child.child.stdin.take() else {
            let _ = child.terminate_tree();
            return Err(McpError::new(
                "mcp_stdio_unavailable",
                "MCP stdio stdinを取得できない",
            ));
        };
        let Some(stdout) = child.child.stdout.take() else {
            let _ = child.terminate_tree();
            return Err(McpError::new(
                "mcp_stdio_unavailable",
                "MCP stdio stdoutを取得できない",
            ));
        };
        let (sender, receiver) = mpsc::sync_channel(32);
        let reader = thread::Builder::new()
            .name("mcp-stdio-reader".to_string())
            .spawn(move || {
                let mut reader = BufReader::new(stdout);
                loop {
                    let mut line = Vec::new();
                    match reader.read_until(b'\n', &mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            if line.len() > MAX_STDIO_LINE_BYTES || sender.send(line).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        if reader.is_err() {
            let _ = child.terminate_tree();
            return Err(McpError::new(
                "mcp_stdio_reader_failed",
                "MCP stdio readerを起動できない",
            ));
        }
        Ok(Self {
            child,
            stdin,
            lines: receiver,
            era: McpProtocolEra::Modern,
            catalog: McpCatalog {
                discovery: crate::mcp::McpDiscovery {
                    era: McpProtocolEra::Modern,
                    protocol_version: String::new(),
                    server_name: None,
                    server_version: None,
                    capabilities: serde_json::json!({}),
                    metadata_hash: String::new(),
                },
                tools: Vec::new(),
                resources: Vec::new(),
                prompts: Vec::new(),
            },
        })
    }

    fn finish_catalog(
        &mut self,
        discovery: crate::mcp::McpDiscovery,
        first_id: u64,
        era: McpProtocolEra,
    ) -> Result<(), McpError> {
        self.era = era;
        let mut id = first_id;
        let tools = if discovery.capabilities.get("tools").is_some() {
            let response = self.request(
                id,
                crate::mcp::request_line(id, era, "tools/list", json_object())?,
            )?;
            id = id.saturating_add(1);
            parse_tools_response(&response)?
        } else {
            Vec::new()
        };
        let resources = if discovery.capabilities.get("resources").is_some() {
            let response = self.request(
                id,
                crate::mcp::request_line(id, era, "resources/list", json_object())?,
            )?;
            id = id.saturating_add(1);
            parse_resources_response(&response)?
        } else {
            Vec::new()
        };
        let prompts = if discovery.capabilities.get("prompts").is_some() {
            let response = self.request(
                id,
                crate::mcp::request_line(id, era, "prompts/list", json_object())?,
            )?;
            parse_prompts_response(&response)?
        } else {
            Vec::new()
        };
        self.catalog = McpCatalog {
            discovery,
            tools,
            resources,
            prompts,
        };
        Ok(())
    }

    fn request(&mut self, expected_id: u64, line: Vec<u8>) -> Result<McpJsonRpcMessage, McpError> {
        self.write_line(&line)?;
        let started = Instant::now();
        loop {
            let remaining = RESPONSE_TIMEOUT.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Err(McpError::new(
                    "mcp_timeout",
                    "MCP responseが期限内に届かない",
                ));
            }
            let line = match self.lines.recv_timeout(remaining) {
                Ok(line) => line,
                Err(RecvTimeoutError::Timeout) => {
                    return Err(McpError::new(
                        "mcp_timeout",
                        "MCP responseが期限内に届かない",
                    ))
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(McpError::new(
                        "mcp_server_unavailable",
                        "MCP stdio Serverが終了した",
                    ))
                }
            };
            let message = McpJsonRpcMessage::parse_line(&line)?;
            if message.is_notification() {
                continue;
            }
            if !message.response_for(expected_id) {
                return Err(McpError::new(
                    "mcp_response_id_mismatch",
                    "MCP response idが要求と一致しない",
                ));
            }
            return Ok(message);
        }
    }

    fn write_line(&mut self, line: &[u8]) -> Result<(), McpError> {
        self.stdin
            .write_all(line)
            .and_then(|_| self.stdin.flush())
            .map_err(|_| McpError::new("mcp_server_unavailable", "MCP stdio requestを送信できない"))
    }

    pub(crate) fn terminate(&mut self) -> Result<(), McpError> {
        self.child.terminate_tree().map_err(|_| {
            McpError::new(
                "mcp_process_termination_failed",
                "MCP stdio Server process群の終了を確認できない",
            )
        })
    }

    pub(crate) fn is_alive(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(None) => true,
            Ok(Some(_)) => {
                let _ = self.child.stop_descendants();
                false
            }
            Err(_) => {
                let _ = self.child.terminate_tree();
                false
            }
        }
    }
}

impl Drop for McpStdioConnection {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

fn json_object() -> serde_json::Value {
    serde_json::json!({})
}

fn canonical_executable(path: &Path) -> Result<PathBuf, McpError> {
    if !path.is_absolute() {
        return Err(McpError::new(
            "mcp_executable_invalid",
            "MCP executableは絶対pathで指定する",
        ));
    }
    let resolved = path
        .canonicalize()
        .map_err(|_| McpError::new("mcp_server_unavailable", "MCP executableを確認できない"))?;
    if !resolved.is_file() {
        return Err(McpError::new(
            "mcp_executable_invalid",
            "MCP executableが通常fileではない",
        ));
    }
    Ok(resolved)
}

fn canonical_workspace(path: &Path) -> Result<PathBuf, McpError> {
    if !path.is_absolute() {
        return Err(McpError::new(
            "mcp_workspace_invalid",
            "MCP workspaceは絶対pathで指定する",
        ));
    }
    let resolved = path
        .canonicalize()
        .map_err(|_| McpError::new("mcp_workspace_invalid", "MCP workspaceを確認できない"))?;
    if !resolved.is_dir() {
        return Err(McpError::new(
            "mcp_workspace_invalid",
            "MCP workspaceがdirectoryではない",
        ));
    }
    Ok(resolved)
}

fn secret_component(path: &Path) -> bool {
    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy().to_ascii_lowercase();
        matches!(value.as_str(), ".env" | ".ssh" | ".gnupg" | "secrets")
    })
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    const DESCENDANT_READY: &str = ".gui-shell-mcp-descendant-ready";
    const SYSTEM_ROOT_ENVIRONMENT_KEY: &str = "SystemRoot";
    const MCP_FIXTURE: &str = r#"@echo off
start "" /b "%SystemRoot%\System32\ping.exe" -t 127.0.0.1 > nul
echo ready>"__READY_PATH__"
echo {"jsonrpc":"2.0","id":1,"result":{"supportedVersions":["__PROTOCOL_VERSION__"],"capabilities":{}},"_meta":{"io.modelcontextprotocol/serverInfo":{"name":"process-supervision-fixture","version":"1"}}}
"%SystemRoot%\System32\ping.exe" -t 127.0.0.1 > nul
"#;

    #[test]
    fn MCP_stdio_Serverのprocess群をBroker監督下で起動し終了する() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("時計")
            .as_nanos();
        let workspace = std::env::temp_dir().join(format!(
            "gui-shell-mcp-process-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&workspace).expect("試験用作業領域を作成");
        let system_root =
            PathBuf::from(env::var_os(SYSTEM_ROOT_ENVIRONMENT_KEY).expect("システム領域を取得"));
        let executable = system_root.join("System32").join("cmd.exe");
        let ready_path = workspace.join(DESCENDANT_READY);
        let script = MCP_FIXTURE
            .replace("__READY_PATH__", &ready_path.to_string_lossy())
            .replace("__PROTOCOL_VERSION__", crate::mcp::MODERN_PROTOCOL_VERSION);
        let script_path = workspace.join("mcp-server.cmd");
        fs::write(&script_path, script).expect("試験用MCP Server scriptを作成");
        let arguments = vec![
            "/D".to_string(),
            "/C".to_string(),
            script_path.to_string_lossy().into_owned(),
        ];
        let mut connection = McpStdioConnection::connect(&executable, &arguments, &workspace)
            .expect("試験用MCP Serverのdiscoveryを実行");

        let started = Instant::now();
        while !workspace.join(DESCENDANT_READY).exists() {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "MCP fixtureの子孫processが起動しない"
            );
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            connection.is_alive(),
            "MCP Server processは接続中に存続する"
        );

        connection
            .terminate()
            .expect("Job Object内のrootと全子孫を停止・確認");
        assert!(
            connection.child.try_wait().expect("root状態確認").is_some(),
            "MCP root processが終了している"
        );
        drop(connection);
        fs::remove_dir_all(workspace).expect("fixture workspace削除");
    }
}
