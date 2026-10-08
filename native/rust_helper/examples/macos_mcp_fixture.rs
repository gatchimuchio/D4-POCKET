//! P13開発専用の合成stdio Server。通常製品へ同梱しない。
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

fn main() {
    let input = io::stdin();
    let mut output = io::stdout().lock();
    for line in input.lock().lines() {
        let line = line.expect("fixture入力");
        assert!(line.len() <= 32768, "fixture入力上限");
        let request: Value = serde_json::from_str(&line).expect("合成試験のJSON");
        let id = request["id"].clone();
        if id.is_null() {
            continue;
        }
        let result = match request["method"].as_str() {
            Some("server/discover") => json!({"resultType":"complete",
                "supportedVersions":["2026-07-28"],"capabilities":{"tools":{}}}),
            Some("tools/list") => json!({"resultType":"complete","tools":[{
                "name":"echo","inputSchema":{"type":"object",
                "properties":{"text":{"type":"string"}},"required":["text"],"additionalProperties":false}}]}),
            Some("tools/call") => {
                assert_eq!(request["params"]["name"], "echo", "fixture Tool束縛");
                json!({"resultType":"complete","content":[{"type":"text","text":"macos-mcp-public-result"}],"isError":false})
            }
            _ => panic!("fixture未対応method"),
        };
        let response = json!({"jsonrpc":"2.0","id":id,"result":result,
            "_meta":{"io.modelcontextprotocol/serverInfo":{"name":"Mac MCP試験Server","version":"1"}}});
        writeln!(output, "{response}").expect("fixture応答");
        output.flush().expect("fixture応答flush");
    }
}
