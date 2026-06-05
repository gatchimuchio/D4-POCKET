#![forbid(unsafe_code)]

use std::env;
use std::io::{self, BufRead};
use std::path::PathBuf;

use gui_shell_rust_helper::broker::{
    run_loopback_server, Broker, BrokerRequestEnvelope, BrokerServerConfig, BrokerStatus,
};

fn main() {
    if let Some(exit_code) = maybe_run_broker_server() {
        std::process::exit(exit_code);
    }
    if let Some(exit_code) = maybe_run_dev_stdin_smoke() {
        std::process::exit(exit_code);
    }
    eprintln!("usage: gui_shell_rust_helper broker-server --store-dir <path> --session-file <path> [--port <port>] [--max-request-bytes <bytes>]");
    eprintln!("dev-only: gui_shell_rust_helper dev-stdin-smoke");
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

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--store-dir" => {
                let Some(value) = args.next() else {
                    eprintln!("--store-dir requires a value");
                    return Some(2);
                };
                store_dir = Some(PathBuf::from(value));
            }
            "--session-file" => {
                let Some(value) = args.next() else {
                    eprintln!("--session-file requires a value");
                    return Some(2);
                };
                session_file = Some(PathBuf::from(value));
            }
            "--port" => {
                let Some(value) = args.next() else {
                    eprintln!("--port requires a value");
                    return Some(2);
                };
                let Ok(parsed) = value.parse::<u16>() else {
                    eprintln!("--port must be a TCP port number");
                    return Some(2);
                };
                port = parsed;
            }
            "--max-request-bytes" => {
                let Some(value) = args.next() else {
                    eprintln!("--max-request-bytes requires a value");
                    return Some(2);
                };
                let Ok(parsed) = value.parse::<usize>() else {
                    eprintln!("--max-request-bytes must be a positive integer");
                    return Some(2);
                };
                if parsed == 0 {
                    eprintln!("--max-request-bytes must be non-zero");
                    return Some(2);
                }
                max_request_bytes = parsed;
            }
            _ => {
                eprintln!("unknown broker-server argument: {argument}");
                return Some(2);
            }
        }
    }

    let Some(store_dir) = store_dir else {
        eprintln!("broker-server requires --store-dir");
        return Some(2);
    };
    let Some(session_file) = session_file else {
        eprintln!("broker-server requires --session-file");
        return Some(2);
    };
    let mut config = BrokerServerConfig::new(store_dir, session_file);
    config.port = port;
    config.max_request_bytes = max_request_bytes;

    match run_loopback_server(config) {
        Ok(()) => Some(0),
        Err(error) => {
            eprintln!("broker-server failed: {}", error.message);
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
        eprintln!("dev-stdin-smoke accepts no arguments");
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
