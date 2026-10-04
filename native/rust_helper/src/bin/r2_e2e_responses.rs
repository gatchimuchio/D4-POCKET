#[allow(dead_code)]
#[path = "../../tests/support/codex_loopback_responses.rs"]
mod fixture;

use serde_json::json;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::ExitCode;

fn workspace_boundary_fixtures_valid(workspace: &Path) -> bool {
    let Some(parent) = workspace.parent() else {
        return false;
    };
    let outside_read = parent.join("outside-read-marker.txt");
    let outside_write = parent.join("outside-write-marker.txt");
    matches!(
        fs::symlink_metadata(&outside_read),
        Ok(metadata)
            if metadata.file_type().is_file()
                && fs::read(&outside_read)
                    .is_ok_and(|contents| contents.as_slice() == b"synthetic outside marker")
    ) && matches!(
        fs::symlink_metadata(outside_write),
        Err(error) if error.kind() == io::ErrorKind::NotFound
    )
}

fn evidence_matches_expected_outcome(evidence: &serde_json::Value, expect_deadline: bool) -> bool {
    let common = evidence["requests"].as_u64().unwrap_or_default() >= 1
        && evidence["tool_offered"] == true
        && evidence["tool_call_sent"] == true
        && evidence["repeated_tool_call_rejections"] == 0
        && evidence["invalid_bodies"] == 0
        && evidence["response_write_failures"] == 0
        && evidence["incomplete_requests"] == 0
        && evidence["blocked_external_requests"] == 0
        && evidence["workspace_boundary_fixtures_valid_after_task"] == true;
    if expect_deadline {
        common
            && evidence["tool_result_received"] == false
            && evidence["workspace_marker_exists"] == false
            && evidence["expected_outcome"] == "deadline"
    } else {
        common
            && evidence["requests"].as_u64() == Some(2)
            && evidence["tool_result_received"] == true
            && evidence["workspace_marker_exists"] == true
            && evidence["expected_outcome"] == "completion"
    }
}

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
    if !workspace_boundary_fixtures_valid(workspace) {
        eprintln!("合成Workspace外境界fixtureを確認できない");
        return ExitCode::from(2);
    }
    let port = match std::env::args_os().nth(2) {
        None => 0,
        Some(value) => match value
            .to_str()
            .and_then(|value| value.parse::<u16>().ok())
            .filter(|port| *port != 0)
        {
            Some(port) => port,
            None => {
                eprintln!("任意指定portは1から65535で指定する");
                return ExitCode::from(2);
            }
        },
    };
    let expected_deadline = match std::env::args_os().nth(3) {
        None => false,
        Some(value) if value == "--expect-deadline" => true,
        Some(_) => {
            eprintln!("任意指定モードは--expect-deadlineだけを受理する");
            return ExitCode::from(2);
        }
    };
    if std::env::args_os().nth(4).is_some() {
        eprintln!("余分な引数を受理しない");
        return ExitCode::from(2);
    }
    let server = match fixture::CodexLoopbackResponses::start_on(workspace, port) {
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
    let mut evidence = json!({
        "expected_outcome": if expected_deadline { "deadline" } else { "completion" },
        "requests": server.post_requests(),
        "models": server.model_list_requests(),
        "tool_offered": server.tool_was_offered(),
        "tool_call_sent": server.tool_call_was_sent(),
        "tool_result_received": server.tool_result_was_received(),
        "tool_output_diagnostics": server.tool_output_diagnostics(),
        "repeated_tool_call_rejections": server.repeated_tool_call_rejections(),
        "invalid_bodies": server.invalid_post_bodies(),
        "response_write_failures": server.response_write_failures(),
        "incomplete_requests": server.incomplete_request_count(),
        "blocked_external_requests": server.blocked_external_requests(),
        "workspace_marker_exists": workspace.join("broker-real-codex-marker.txt").is_file(),
    });
    drop(server);
    evidence["workspace_boundary_fixtures_valid_after_task"] =
        json!(workspace_boundary_fixtures_valid(workspace));
    println!("EVIDENCE {evidence}");
    if !evidence_matches_expected_outcome(&evidence, expected_deadline) {
        eprintln!("R2 loopbackResponses APIの検証条件が成立しない");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{evidence_matches_expected_outcome, workspace_boundary_fixtures_valid};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn deadline_mode_accepts_only_tool_start_without_result_or_workspace_write() {
        let valid = serde_json::json!({
            "expected_outcome": "deadline",
            "requests": 1,
            "tool_offered": true,
            "tool_call_sent": true,
            "tool_result_received": false,
            "repeated_tool_call_rejections": 0,
            "invalid_bodies": 0,
            "response_write_failures": 0,
            "incomplete_requests": 0,
            "blocked_external_requests": 0,
            "workspace_marker_exists": false,
            "workspace_boundary_fixtures_valid_after_task": true
        });
        assert!(evidence_matches_expected_outcome(&valid, true));

        let mut unexpected_completion = valid.clone();
        unexpected_completion["workspace_marker_exists"] = serde_json::json!(true);
        assert!(!evidence_matches_expected_outcome(
            &unexpected_completion,
            true
        ));

        let mut rejected_repeat = valid.clone();
        rejected_repeat["repeated_tool_call_rejections"] = serde_json::json!(1);
        assert!(!evidence_matches_expected_outcome(&rejected_repeat, true));
    }

    #[test]
    fn normal_mode_still_requires_completed_tool_and_workspace_write() {
        let completed = serde_json::json!({
            "expected_outcome": "completion",
            "requests": 2,
            "tool_offered": true,
            "tool_call_sent": true,
            "tool_result_received": true,
            "repeated_tool_call_rejections": 0,
            "invalid_bodies": 0,
            "response_write_failures": 0,
            "incomplete_requests": 0,
            "blocked_external_requests": 0,
            "workspace_marker_exists": true,
            "workspace_boundary_fixtures_valid_after_task": true
        });
        assert!(evidence_matches_expected_outcome(&completed, false));

        let mut no_result = completed.clone();
        no_result["tool_result_received"] = serde_json::json!(false);
        assert!(!evidence_matches_expected_outcome(&no_result, false));
    }

    #[test]
    fn loopback_harness_checks_outside_markers_before_the_sandboxed_probe() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test用時刻")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "d4p-r2-outside-marker-fixture-{}-{suffix}",
            std::process::id()
        ));
        let workspace = root.join("workspace");
        fs::create_dir_all(&workspace).expect("test専用Workspace作成");
        let missing_read_marker_is_rejected = !workspace_boundary_fixtures_valid(&workspace);
        let outside_read = root.join("outside-read-marker.txt");
        fs::write(&outside_read, b"synthetic outside marker")
            .expect("合成Workspace外read marker作成");
        let valid_fixture_is_accepted = workspace_boundary_fixtures_valid(&workspace);
        fs::write(&outside_read, b"unexpected modification").expect("改変read markerの拒否試験");
        let modified_read_marker_is_rejected = !workspace_boundary_fixtures_valid(&workspace);
        fs::write(&outside_read, b"synthetic outside marker").expect("read marker復元");
        fs::write(root.join("outside-write-marker.txt"), b"unexpected")
            .expect("既存write markerの拒否試験");
        let existing_write_marker_is_rejected = !workspace_boundary_fixtures_valid(&workspace);
        fs::remove_dir_all(&root).expect("test専用fixture削除");

        assert!(missing_read_marker_is_rejected);
        assert!(valid_fixture_is_accepted);
        assert!(modified_read_marker_is_rejected);
        assert!(existing_write_marker_is_rejected);
    }
}
