#![forbid(unsafe_code)]

use std::io::{self, BufRead};

use gui_shell_rust_helper::broker::{Broker, BrokerRequestEnvelope, BrokerStatus};

fn main() {
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
