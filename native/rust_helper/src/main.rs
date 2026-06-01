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
        let response = match command {
            "health" => broker.handle(BrokerRequestEnvelope::health(&request_id, &nonce)),
            "shutdown" => broker.handle(BrokerRequestEnvelope::shutdown(
                &request_id,
                "local-dev-session",
                &nonce,
            )),
            _ => broker.handle(BrokerRequestEnvelope {
                request_id: Some("stdin-malformed".to_string()),
                session_id: None,
                operation: None,
                payload_hash: None,
                nonce: None,
                issued_at: None,
                metadata: vec![],
            }),
        };
        println!(
            "request_id={} operation={} status={} audit_event_id={}",
            response.request_id,
            response.operation,
            response.status.as_str(),
            response.audit_event_id
        );
        if response.status == BrokerStatus::Accepted && response.shutdown_requested {
            break;
        }
    }
}
