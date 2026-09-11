#![forbid(unsafe_code)]
mod owner_cli;
mod checkpoint_cli;

use std::env;
use std::io::{self, BufRead};
use std::path::PathBuf;

use gui_shell_rust_helper::broker::{
    run_loopback_server, Broker, BrokerRequestEnvelope, BrokerServerConfig, BrokerStatus,
};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().is_some_and(|v| v == "監査チェックポイント") {
        match checkpoint_cli::run(&args[1..]) { Ok(()) => return, Err(error) => { eprintln!("{error}"); std::process::exit(1); } }
    }
    if args.first().is_some_and(|v| v == "対話承認操作") {
        match owner_cli::実行(&args[1..]) { Ok(()) => return, Err(error) => { eprintln!("{error}"); std::process::exit(1); } }
    }
    if let Some(exit_code) = maybe_run_broker_server() {
        std::process::exit(exit_code);
    }
    if let Some(exit_code) = maybe_run_dev_stdin_smoke() {
        std::process::exit(exit_code);
    }
    eprintln!("使用法: gui_shell_rust_helper broker-server --store-dir <path> --session-file <path> [--port <port>] [--max-request-bytes <bytes>]");
    eprintln!("開発専用: gui_shell_rust_helper dev-stdin-smoke");
    eprintln!("対話登録: broker-server ... --owner-session-file <owner資格file> --minidora-runtime <ID=127.0.0.1:port>");
    eprintln!("端末経路: broker-server ... --mobile-bind <private IPv4:port>（owner資格必須）");
    eprintln!("owner操作: 対話承認操作 --session-file <owner資格file> 一覧 | 承認 <要求ID> <要求hash> <表示範囲>");
    std::process::exit(2);
}

fn maybe_run_broker_server() -> Option<i32> {
    let mut args = env::args().skip(1);
    let Some(mode) = args.next() else {
        return None;
    };
    if mode != "broker-server" {
        return None;
    }

    let mut store_dir: Option<PathBuf> = None;
    let mut session_file: Option<PathBuf> = None;
    let mut port: u16 = 0;
    let mut max_request_bytes: usize = 64 * 1024;
    let mut owner_session_file = None;
    let mut minidora_runtimes = Vec::new();
    let mut mobile_bind = None;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--mobile-bind" => {
                let Some(value) = args.next() else {eprintln!("端末bind先の値が必要");return Some(2);};
                mobile_bind = Some(value);
            }
            "--owner-session-file" => {
                let Some(value) = args.next() else { eprintln!("owner資格fileの値が必要"); return Some(2); };
                owner_session_file = Some(PathBuf::from(value));
            }
            "--minidora-runtime" => {
                let Some(value) = args.next() else { eprintln!("実行系ID=127.0.0.1:portの値が必要"); return Some(2); };
                let Some((id, address)) = value.split_once('=') else { eprintln!("実行系登録の形式が不正"); return Some(2); };
                minidora_runtimes.push((id.to_owned(), address.to_owned()));
            }
            "--store-dir" => {
                let Some(value) = args.next() else {
                eprintln!("--store-dirには値が必要");
                    return Some(2);
                };
                store_dir = Some(PathBuf::from(value));
            }
            "--session-file" => {
                let Some(value) = args.next() else {
                eprintln!("--session-fileには値が必要");
                    return Some(2);
                };
                session_file = Some(PathBuf::from(value));
            }
            "--port" => {
                let Some(value) = args.next() else {
                eprintln!("--portには値が必要");
                    return Some(2);
                };
                let Ok(parsed) = value.parse::<u16>() else {
                    eprintln!("--portはTCP port番号でなければならない");
                    return Some(2);
                };
                port = parsed;
            }
            "--max-request-bytes" => {
                let Some(value) = args.next() else {
                eprintln!("--max-request-bytesには値が必要");
                    return Some(2);
                };
                let Ok(parsed) = value.parse::<usize>() else {
                    eprintln!("--max-request-bytesは正のintegerでなければならない");
                    return Some(2);
                };
                if parsed == 0 {
                    eprintln!("--max-request-bytesはゼロ以外でなければならない");
                    return Some(2);
                }
                max_request_bytes = parsed;
            }
            _ => {
            eprintln!("未知のbroker-server引数: {argument}");
                return Some(2);
            }
        }
    }

    let Some(store_dir) = store_dir else {
        eprintln!("broker-serverには--store-dirが必要");
        return Some(2);
    };
    let Some(session_file) = session_file else {
        eprintln!("broker-serverには--session-fileが必要");
        return Some(2);
    };
    let mut config = BrokerServerConfig::new(store_dir, session_file);
    config.port = port;
    config.max_request_bytes = max_request_bytes;
    config.owner_session_file = owner_session_file;
    config.minidora_runtimes = minidora_runtimes;
    config.mobile_bind = mobile_bind;

    match run_loopback_server(config) {
        Ok(()) => Some(0),
        Err(error) => {
            eprintln!("broker-serverが失敗: {}", error.message);
            Some(1)
        }
    }
}

fn maybe_run_dev_stdin_smoke() -> Option<i32> {
    let mut args = env::args().skip(1);
    let Some(mode) = args.next() else {
        return None;
    };
    if mode != "dev-stdin-smoke" {
        return None;
    }
    if args.next().is_some() {
        eprintln!("dev-stdin-smokeは引数を受け付けない");
        return Some(2);
    }
    run_dev_stdin_smoke();
    Some(0)
}

fn run_dev_stdin_smoke() {
    let mut broker = Broker::new("local-dev-session");
    for (index, line) in io::stdin().lock().lines().enumerate() {
        let Ok(line) = line else {
            break;
        };
        let command = line.trim();
        let request_id = format!("stdin-request-{}", index + 1);
        let nonce = format!("stdin-nonce-{}", index + 1);
        let issued_at = BrokerRequestEnvelope::current_issued_at();
        let response = if command.starts_with('{') {
            broker.handle_json(command)
        } else {
            match command {
                "health" => broker.handle(BrokerRequestEnvelope::health_at(
                    &request_id,
                    &nonce,
                    &issued_at,
                )),
                "shutdown" => broker.handle(BrokerRequestEnvelope::shutdown_at(
                    &request_id,
                    "local-dev-session",
                    &nonce,
                    &issued_at,
                )),
                _ => broker.handle(BrokerRequestEnvelope {
                    request_id: Some("stdin-malformed".to_string()),
                    session_id: None,
                    operation: None,
                    payload_hash: None,
                    nonce: None,
                    issued_at: None,
                    metadata: vec![],
                    metadata_present: false,
                    payload: None,
                }),
            }
        };
        let response_json = response
            .to_json_string()
            .unwrap_or_else(|_| "{\"status\":\"rejected\"}".to_string());
        println!("{response_json}");
        if response.status == BrokerStatus::Accepted && response.shutdown_requested {
            break;
        }
    }
}
