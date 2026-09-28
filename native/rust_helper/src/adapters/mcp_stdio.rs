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
    lines: Receiver<Result<Vec<u8>, McpError>>,
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
        let modern_message = connection.request(1, modern_discover_request(1)?)?;
        match parse_discovery(&modern_message, McpProtocolEra::Modern) {
            Ok(discovery) => connection.finish_catalog(discovery, 2, McpProtocolEra::Modern)?,
            Err(error)
                if modern_message.is_method_not_found_error()
                    || error.code == "mcp_protocol_unsupported" =>
            {
                // 旧protocolへの切替は未対応methodまたは版交渉に限定し、その他の失敗は停止する。
                connection.terminate()?;
                connection = Self::spawn(&executable, arguments, &workspace)?;
                connection.era = McpProtocolEra::Legacy;
                let discovery = connection
                    .request(1, legacy_initialize_request(1)?)
                    .and_then(|message| parse_discovery(&message, McpProtocolEra::Legacy))?;
                let initialized = notification_line("notifications/initialized", json_object())?;
                connection.write_line(&initialized)?;
                connection.finish_catalog(discovery, 2, McpProtocolEra::Legacy)?;
            }
            Err(error) => return Err(error),
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
                    match read_bounded_line(&mut reader) {
                        Ok(Some(line)) => {
                            if sender.send(Ok(line)).is_err() {
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(error) => {
                            let _ = sender.send(Err(error));
                            break;
                        }
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
                Ok(Ok(line)) => line,
                Ok(Err(error)) => return Err(error),
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
            if self.era == McpProtocolEra::Modern {
                message.validate_complete_result()?;
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

fn read_bounded_line(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, McpError> {
    let mut line = Vec::with_capacity(8 * 1024);
    loop {
        let available = reader.fill_buf().map_err(|_| {
            McpError::new("mcp_server_unavailable", "MCP stdio responseを読み取れない")
        })?;
        if available.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Ok(Some(line))
            };
        }

        let newline = available.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(available.len(), |index| index + 1);
        if line.len().saturating_add(consumed) > MAX_STDIO_LINE_BYTES {
            return Err(McpError::new(
                "mcp_wire_oversized",
                "MCP stdio response lineが上限を超過した",
            ));
        }
        line.extend_from_slice(&available[..consumed]);
        reader.consume(consumed);
        if newline.is_some() {
            return Ok(Some(line));
        }
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

#[cfg(test)]
mod bounded_line_tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn reads_newline_terminated_and_final_unterminated_lines() {
        let mut terminated = BufReader::new(Cursor::new(b"{}\nnext\n".to_vec()));
        assert_eq!(
            read_bounded_line(&mut terminated).unwrap(),
            Some(b"{}\n".to_vec())
        );
        assert_eq!(
            read_bounded_line(&mut terminated).unwrap(),
            Some(b"next\n".to_vec())
        );

        let mut final_line = BufReader::new(Cursor::new(b"{}".to_vec()));
        assert_eq!(
            read_bounded_line(&mut final_line).unwrap(),
            Some(b"{}".to_vec())
        );
        assert_eq!(read_bounded_line(&mut final_line).unwrap(), None);
    }

    #[test]
    fn rejects_a_line_that_exceeds_the_limit_before_newline() {
        let mut exact_limit = vec![b'x'; MAX_STDIO_LINE_BYTES - 1];
        exact_limit.push(b'\n');
        let mut reader = BufReader::new(Cursor::new(exact_limit));
        assert_eq!(
            read_bounded_line(&mut reader).unwrap().unwrap().len(),
            MAX_STDIO_LINE_BYTES
        );

        let input = vec![b'x'; MAX_STDIO_LINE_BYTES + 1];
        let mut reader = BufReader::new(Cursor::new(input));
        let error = read_bounded_line(&mut reader).expect_err("超過行を拒否");
        assert_eq!(error.code, "mcp_wire_oversized");
    }
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
echo {"jsonrpc":"2.0","id":1,"result":{"resultType":"complete","supportedVersions":["__PROTOCOL_VERSION__"],"capabilities":{}},"_meta":{"io.modelcontextprotocol/serverInfo":{"name":"process-supervision-fixture","version":"1"}}}
"%SystemRoot%\System32\ping.exe" -t 127.0.0.1 > nul
"#;
    const MCP_METHOD_FALLBACK_FIXTURE: &str = r#"@echo off
echo started>>"__STARTS_PATH__"
set /p request=
echo %request% | findstr /c:"server/discover" > nul
if errorlevel 1 (
  echo {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-11-25","capabilities":{}}}
) else (
  echo {"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"method not found"}}
)
"%SystemRoot%\System32\ping.exe" -t 127.0.0.1 > nul
"#;
    const MCP_NON_FALLBACK_FIXTURE: &str = r#"@echo off
echo started>>"__STARTS_PATH__"
set /p request=
echo {"jsonrpc":"2.0","id":1,"error":{"code":-32602,"message":"invalid params"}}
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

    #[test]
    fn legacy_fallback_only_runs_for_method_not_found() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("時計")
            .as_nanos();
        let workspace = std::env::temp_dir().join(format!(
            "gui-shell-mcp-fallback-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&workspace).expect("試験用作業領域を作成");
        let system_root =
            PathBuf::from(env::var_os(SYSTEM_ROOT_ENVIRONMENT_KEY).expect("システム領域を取得"));
        let executable = system_root.join("System32").join("cmd.exe");
        let starts_path = workspace.join("starts.txt");
        let script_path = workspace.join("mcp-server.cmd");
        let script =
            MCP_METHOD_FALLBACK_FIXTURE.replace("__STARTS_PATH__", &starts_path.to_string_lossy());
        fs::write(&script_path, script).expect("legacy fallback fixtureを作成");
        let arguments = vec![
            "/D".to_string(),
            "/C".to_string(),
            script_path.to_string_lossy().into_owned(),
        ];

        let mut connection = McpStdioConnection::connect(&executable, &arguments, &workspace)
            .expect("method-not-foundだけlegacyへfallback");
        assert_eq!(connection.catalog().discovery.era, McpProtocolEra::Legacy);
        assert_eq!(
            fs::read_to_string(&starts_path)
                .expect("spawn記録")
                .lines()
                .count(),
            2
        );
        connection.terminate().expect("fixture process群を停止");
        drop(connection);
        fs::remove_dir_all(workspace).expect("fixture workspace削除");
    }

    #[test]
    fn invalid_params_does_not_restart_as_legacy() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("時計")
            .as_nanos();
        let workspace = std::env::temp_dir().join(format!(
            "gui-shell-mcp-no-fallback-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&workspace).expect("試験用作業領域を作成");
        let system_root =
            PathBuf::from(env::var_os(SYSTEM_ROOT_ENVIRONMENT_KEY).expect("システム領域を取得"));
        let executable = system_root.join("System32").join("cmd.exe");
        let starts_path = workspace.join("starts.txt");
        let script_path = workspace.join("mcp-server.cmd");
        let script =
            MCP_NON_FALLBACK_FIXTURE.replace("__STARTS_PATH__", &starts_path.to_string_lossy());
        fs::write(&script_path, script).expect("fallback拒否fixtureを作成");
        let arguments = vec![
            "/D".to_string(),
            "/C".to_string(),
            script_path.to_string_lossy().into_owned(),
        ];

        let error = McpStdioConnection::connect(&executable, &arguments, &workspace)
            .expect_err("invalid paramsはlegacy fallbackしない");
        assert_eq!(error.code, "mcp_discovery_failed");
        assert_eq!(
            fs::read_to_string(&starts_path)
                .expect("spawn記録")
                .lines()
                .count(),
            1
        );
        fs::remove_dir_all(workspace).expect("fixture workspace削除");
    }
}
