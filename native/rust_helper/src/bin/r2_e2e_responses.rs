#[allow(dead_code)]
#[path = "../../tests/support/codex_loopback_responses.rs"]
mod fixture;

use serde_json::json;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(workspace) = std::env::args_os().nth(1) else {
        eprintln!("合成Workspaceの絶対pathが必要");
        return ExitCode::from(2);
    };
    let workspace = Path::new(&workspace);
    if !workspace.is_absolute() || !workspace.is_dir() {
        eprintln!("合成Workspaceを確認できない");
        return ExitCode::from(2);
    }
    let server = match fixture::CodexLoopbackResponses::start(workspace) {
        Ok(server) => server,
        Err(_) => {
            eprintln!("localhost偽Responses APIを起動できない");
            return ExitCode::FAILURE;
        }
    };
    println!("READY {}", server.port());
    if io::stdout().flush().is_err() {
        return ExitCode::FAILURE;
    }
    let mut command = String::new();
    if io::stdin().lock().read_line(&mut command).is_err() || command.trim() != "stop" {
        eprintln!("R2 loopback偽APIの停止指示を確認できない");
        return ExitCode::FAILURE;
    }
    let evidence = json!({
        "requests": server.post_requests(),
        "models": server.model_list_requests(),
        "tool_offered": server.tool_was_offered(),
        "tool_call_sent": server.tool_call_was_sent(),
        "invalid_bodies": server.invalid_post_bodies(),
        "response_write_failures": server.response_write_failures(),
        "incomplete_requests": server.incomplete_request_count(),
        "blocked_external_requests": server.blocked_external_requests(),
        "workspace_marker_exists": workspace.join("broker-real-codex-marker.txt").is_file(),
    });
    drop(server);
    println!("EVIDENCE {evidence}");
    if evidence["requests"].as_u64().unwrap_or_default() < 2
        || evidence["tool_offered"] != true
        || evidence["tool_call_sent"] != true
        || evidence["invalid_bodies"] != 0
        || evidence["response_write_failures"] != 0
        || evidence["incomplete_requests"] != 0
        || evidence["blocked_external_requests"] != 0
        || evidence["workspace_marker_exists"] != true
    {
        eprintln!("R2 loopbackResponses APIの検証条件が成立しない");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
