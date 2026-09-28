use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;
use std::thread;
use std::time::Duration;

const VERSION_OUTPUT: &str = "codex-cli";
const EXEC_HELP_OUTPUT: &str =
    "codex exec --sandbox workspace-write --cd DIR --json --ephemeral --ignore-user-config";

const TASK_PERMISSION_PROFILE: [&str; 5] = [
    "default_permissions=\"d4p-agent-task\"",
    "windows.sandbox=\"mxc\"",
    "permissions.d4p-agent-task.extends=\":workspace\"",
    "permissions.d4p-agent-task.filesystem={\":root\"=\"deny\",\":minimal\"=\"read\",\":workspace_roots\"={\"**/*.env\"=\"deny\",\"**/.ssh/**\"=\"deny\",\"**/secrets/**\"=\"deny\"},\"glob_scan_max_depth\"=8}",
    "permissions.d4p-agent-task.network.enabled=false",
];

fn main() {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() == 1 && arguments[0] == "--version" {
        println!("{VERSION_OUTPUT}");
        return;
    }
    if arguments.len() == 2 && arguments[0] == "exec" && arguments[1] == "--help" {
        println!("{EXEC_HELP_OUTPUT}");
        return;
    }

    if !valid_task_arguments(&arguments) {
        process::exit(41);
    }
    let Some(workspace) = argument_after(&arguments, "--cd").map(PathBuf::from) else {
        process::exit(42);
    };
    let (Some(temp), Some(tmp)) = (env::var_os("TEMP"), env::var_os("TMP")) else {
        process::exit(43);
    };
    if temp != tmp {
        process::exit(44);
    }
    let scratch = PathBuf::from(temp);
    if !scratch_is_bound_to_workspace(&scratch, &workspace) {
        process::exit(45);
    }

    let mut instruction = String::new();
    if io::stdin().read_to_string(&mut instruction).is_err() || instruction.trim().is_empty() {
        process::exit(46);
    }
    if instruction.contains("FIXTURE_TIMEOUT") {
        thread::sleep(Duration::from_secs(30));
        return;
    }
    if fs::write(scratch.join("fixture-canary"), b"fixture-only").is_err() {
        process::exit(47);
    }

    println!(r#"{{"type":"thread.started","thread_id":"01a0cd58-c4fc-7221-8d25-dc52d12ba3fd"}}"#);
    println!(
        r#"{{"type":"item.completed","item":{{"id":"item_0","type":"agent_message","text":"fixture-task-completed"}}}}"#
    );
    println!(r#"{{"type":"turn.completed"}}"#);
}

fn valid_task_arguments(arguments: &[String]) -> bool {
    if !arguments.iter().any(|argument| argument == "exec")
        || !arguments.last().is_some_and(|argument| argument == "-")
        || !contains_pair(arguments, "--json", "--ephemeral")
        || !contains_pair(arguments, "--ignore-user-config", "--color")
        || !contains_pair(arguments, "--color", "never")
        || arguments.iter().any(|argument| {
            matches!(
                argument.as_str(),
                "--sandbox" | "--add-dir" | "--worktree" | "--yolo"
            )
        })
    {
        return false;
    }

    let configs = arguments
        .windows(2)
        .filter(|pair| pair[0] == "-c")
        .map(|pair| pair[1].as_str())
        .collect::<Vec<_>>();
    configs == TASK_PERMISSION_PROFILE.to_vec()
}

fn contains_pair(arguments: &[String], first: &str, second: &str) -> bool {
    arguments
        .windows(2)
        .any(|pair| pair[0] == first && pair[1] == second)
}

fn argument_after<'a>(arguments: &'a [String], flag: &str) -> Option<&'a str> {
    arguments
        .windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].as_str())
}

fn scratch_is_bound_to_workspace(scratch: &Path, workspace: &Path) -> bool {
    scratch.parent() == Some(workspace)
        && scratch
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(".d4p-tmp-"))
}
