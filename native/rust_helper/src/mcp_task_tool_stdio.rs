//! Codex TaskからMCP要求をBrokerへ返す、外部作用を持たないstdio tool。
//!
//! このserverはTool要求を検証して受理結果だけを返す。Codex CLIが出力する構造化
//! `mcp_tool_call` eventからAdapterが要求を回収し、Task完了後に既存Broker経路へ戻す。
//! 外部MCP Toolの実作用はBrokerのCatalog・Owner確認・Permission・Audit後だけ行う。
#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

pub const TASK_TOOL_SERVER_ARGUMENT: &str = "d4p-mcp-task-tool-stdio";
pub const TASK_TOOL_SERVER_NAME: &str = "d4p_task_request";
pub const TASK_TOOL_NAME: &str = "d4_submit_mcp_tool_request";
const MAX_WIRE_LINE: usize = 64 * 1024;
const MAX_PROPOSAL_BYTES: usize = 32 * 1024;

/// 専用CLI modeから呼ぶ。通常の起動argumentには作用しない。
pub fn maybe_run_stdio_from_process() -> Result<bool, &'static str> {
    let mut arguments = std::env::args_os().skip(1);
    let Some(first) = arguments.next() else {
        return Ok(false);
    };
    if first != std::ffi::OsStr::new(TASK_TOOL_SERVER_ARGUMENT) {
        return Ok(false);
    }
    if arguments.next().is_some() {
        return Err("MCP Task serverの起動引数が不正です。");
    }

    e2e_diagnostic_event("started");
    let stdin = io::stdin();
    let stdout = io::stdout();
    serve_stdio(stdin.lock(), stdout.lock())?;
    e2e_diagnostic_event("stopped");
    Ok(true)
}

#[cfg(feature = "r2-e2e")]
fn e2e_diagnostic_event(event: &'static str) {
    if matches!(
        event,
        "started" | "initialize" | "tools_list" | "tools_call" | "stopped"
    ) {
        // Fixed marker only: never log MCP input, result, path, or credential data.
        eprintln!("D4P_R2_E2E_MCP_STDIO_EVENT={event}");
    }
}

#[cfg(not(feature = "r2-e2e"))]
fn e2e_diagnostic_event(_event: &'static str) {}

fn serve_stdio<R: BufRead, W: Write>(mut input: R, mut output: W) -> Result<(), &'static str> {
    let mut submitted = false;
    loop {
        let Some(line) = read_bounded_line(&mut input, MAX_WIRE_LINE)? else {
            return Ok(());
        };
        let parsed = serde_json::from_slice::<Value>(&line);
        let response = match parsed {
            Ok(request) => handle_request(&request, &mut submitted),
            Err(_) => Some(json_rpc_error(
                Value::Null,
                -32700,
                "JSON-RPC messageが不正です。",
            )),
        };
        if let Some(response) = response {
            write_json_line(&mut output, &response)?;
        }
    }
}

fn handle_request(request: &Value, submitted: &mut bool) -> Option<Value> {
    let id = request.get("id").cloned();
    if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return id.map(|id| json_rpc_error(id, -32600, "JSON-RPC requestが不正です。"));
    }
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return id.map(|id| json_rpc_error(id, -32600, "JSON-RPC requestが不正です。"));
    };
    if id.is_none() {
        return None;
    }
    let id = id.unwrap_or(Value::Null);
    if id.is_null() || !(id.is_string() || id.is_number()) {
        return Some(json_rpc_error(
            Value::Null,
            -32600,
            "JSON-RPC idが不正です。",
        ));
    }

    let result = match method {
        "initialize" => {
            e2e_diagnostic_event("initialize");
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "d4-pocket-task-request", "version": "1"}
            })
        }
        "ping" => json!({}),
        "tools/list" => {
            e2e_diagnostic_event("tools_list");
            json!({"tools": [task_request_tool()]})
        }
        "tools/call" => {
            e2e_diagnostic_event("tools_call");
            match submit_tool_request(request.get("params"), submitted) {
                Ok(()) => json!({
                    "content": [{"type": "text", "text": "D4 Pocketが権限を持たないMCP Tool要求を一件受け付けました。このserverは外部MCP serverへ接続しません。まず現在のTaskが終了します。その後、Brokerが最新Catalogを再検査し、native Owner確認・一回限りPermission・Auditを確認してから実Toolを実行します。実結果は別途認可された後続Taskへ渡されます。この応答は外部Toolの実結果ではありません。"}],
                    "isError": false
                }),
                Err(message) => json!({
                    "content": [{"type": "text", "text": message}],
                    "isError": true
                }),
            }
        }
        _ => return Some(json_rpc_error(id, -32601, "未対応のJSON-RPC methodです。")),
    };
    Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

fn task_request_tool() -> Value {
    json!({
        "name": TASK_TOOL_NAME,
        "title": "D4 Pocket MCP Tool要求",
        "description": "D4 PocketへMCP Tool呼出し案を一件送信します。権限を持たない受理応答だけを返し、外部MCP serverは呼び出しません。現在のTask終了後、D4 Pocketが最新server・Tool Catalogを再検査し、native Owner確認、一回限りPermission、Auditの成立後だけ実Toolを呼び出します。実結果は別途認可された後続Taskへ渡されます。この受理応答を外部Toolの実結果として扱わないでください。",
        "inputSchema": {
            "type": "object",
            "properties": {
                "server_id": {"type": "string", "minLength": 1, "maxLength": 128},
                "tool_id": {"type": "string", "pattern": "^tool-[a-f0-9]{142}$"},
                "name": {"type": "string", "minLength": 1, "maxLength": 256},
                "arguments": {"type": "object", "maxProperties": 32}
            },
            "required": ["server_id", "tool_id", "name", "arguments"],
            "additionalProperties": false
        }
    })
}

fn submit_tool_request(params: Option<&Value>, submitted: &mut bool) -> Result<(), &'static str> {
    if *submitted {
        return Err("このTaskはMCP Tool要求をすでに一件送信しています。");
    }
    let params = params
        .and_then(Value::as_object)
        .ok_or("Tool request argumentsが不正です。")?;
    if params.get("name").and_then(Value::as_str) != Some(TASK_TOOL_NAME) {
        return Err("未対応のD4 Pocket MCP Toolです。");
    }
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or("Tool request argumentsはobjectである必要があります。")?;
    let envelope = json!({"d4_mcp_tool_request": arguments});
    validate_proposal(&envelope).ok_or("Tool requestが不正か、上限を超えています。")?;
    *submitted = true;
    Ok(())
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ToolRequestEnvelope {
    d4_mcp_tool_request: ToolRequest,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ToolRequest {
    server_id: String,
    tool_id: String,
    name: String,
    arguments: Value,
}

pub(crate) fn validate_proposal(value: &Value) -> Option<String> {
    let encoded = serde_json::to_vec(value).ok()?;
    if encoded.len() > MAX_PROPOSAL_BYTES {
        return None;
    }
    let proposal: ToolRequestEnvelope = serde_json::from_slice(&encoded).ok()?;
    let request = &proposal.d4_mcp_tool_request;
    if request.server_id.is_empty()
        || request.server_id.len() > 128
        || request.server_id.chars().any(char::is_control)
        || request.name.is_empty()
        || request.name.len() > 256
        || request.name.chars().any(char::is_control)
        || request.tool_id.len() != 147
        || !request.tool_id.starts_with("tool-")
        || !request.tool_id[5..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || !request.arguments.is_object()
        || request.arguments.as_object()?.len() > 32
        || serde_json::to_vec(&request.arguments).ok()?.len() > MAX_PROPOSAL_BYTES
    {
        return None;
    }
    serde_json::to_string(&proposal).ok()
}

fn json_rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message}
    })
}

fn write_json_line(writer: &mut impl Write, value: &Value) -> Result<(), &'static str> {
    serde_json::to_writer(&mut *writer, value).map_err(|_| "MCP stdio responseに失敗しました。")?;
    writer
        .write_all(b"\n")
        .and_then(|_| writer.flush())
        .map_err(|_| "MCP stdio responseに失敗しました。")
}

fn read_bounded_line(
    reader: &mut impl BufRead,
    limit: usize,
) -> Result<Option<Vec<u8>>, &'static str> {
    let mut line = Vec::with_capacity(1024);
    loop {
        let available = reader
            .fill_buf()
            .map_err(|_| "MCP stdio requestに失敗しました。")?;
        if available.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Ok(Some(line))
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(available.len(), |index| index + 1);
        if line.len().saturating_add(consumed) > limit {
            return Err("MCP stdio requestが上限を超えています。");
        }
        line.extend_from_slice(&available[..consumed]);
        reader.consume(consumed);
        if newline.is_some() {
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            return Ok(Some(line));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn valid_call(id: u64) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": TASK_TOOL_NAME,
                "arguments": {
                    "server_id": "server-fixture",
                    "tool_id": format!("tool-{}", "a".repeat(142)),
                    "name": "read_fixture",
                    "arguments": {"query": "fixture"}
                }
            }
        })
    }

    #[test]
    fn mcp_task_tool_stdio_exposes_one_non_authoritative_tool_and_only_returns_an_acknowledgement()
    {
        let mut input = String::new();
        input.push_str(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
        input.push('\n');
        input.push_str(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
        input.push('\n');
        input.push_str(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#);
        input.push('\n');
        input.push_str(&valid_call(3).to_string());
        input.push('\n');
        input.push_str(&valid_call(4).to_string());
        input.push('\n');

        let mut output = Vec::new();
        serve_stdio(Cursor::new(input), &mut output).expect("Codex stdio protocolを処理する");
        let responses = output
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice::<Value>(line).expect("JSON-RPC responseを読む"))
            .collect::<Vec<_>>();
        assert_eq!(responses.len(), 4);
        assert_eq!(
            responses[0]["result"]["serverInfo"]["name"],
            "d4-pocket-task-request"
        );
        assert_eq!(responses[1]["result"]["tools"].as_array().unwrap().len(), 1);
        assert_eq!(responses[1]["result"]["tools"][0]["name"], TASK_TOOL_NAME);
        assert_eq!(responses[2]["id"], 3);
        let receipt = responses[2]["result"]["content"][0]["text"]
            .as_str()
            .expect("上限付き受理応答");
        assert!(receipt.contains("外部MCP serverへ接続しません"));
        assert!(receipt.contains("この応答は外部Toolの実結果ではありません"));
        assert_eq!(responses[3]["result"]["isError"], true);
    }

    #[test]
    fn proposal_rejects_authority_fields_invalid_tool_id_and_oversized_arguments() {
        let mut valid = valid_call(1)["params"]["arguments"].clone();
        valid["permission"] = json!("owner");
        assert!(validate_proposal(&json!({"d4_mcp_tool_request": valid})).is_none());

        let mut valid = valid_call(1)["params"]["arguments"].clone();
        valid["tool_id"] = json!("tool-short");
        assert!(validate_proposal(&json!({"d4_mcp_tool_request": valid})).is_none());

        let mut valid = valid_call(1)["params"]["arguments"].clone();
        valid["arguments"] = json!({"value": "x".repeat(MAX_PROPOSAL_BYTES)});
        assert!(validate_proposal(&json!({"d4_mcp_tool_request": valid})).is_none());
    }

    #[test]
    fn malformed_mcp_request_fails_without_claiming_external_tool_execution() {
        let mut request = valid_call(1);
        request["params"]["arguments"]["tool_id"] = json!("tool-short");
        let mut output = Vec::new();
        serve_stdio(Cursor::new(format!("{request}\n")), &mut output).unwrap();
        let response: Value =
            serde_json::from_slice(output.split(|byte| *byte == b'\n').next().unwrap()).unwrap();
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("不正"));
    }
}
