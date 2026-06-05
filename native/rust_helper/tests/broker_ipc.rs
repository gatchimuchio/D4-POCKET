use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gui_shell_rust_helper::broker::{BrokerEndpoint, BrokerRequestEnvelope};
use serde_json::Value;

struct BrokerProcess {
    child: Child,
    endpoint: BrokerEndpoint,
}

impl Drop for BrokerProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = try_send_raw(
                &self.endpoint,
                &self.endpoint.session_secret,
                &shutdown_request(&self.endpoint.session_id),
            );
            thread::sleep(Duration::from_millis(100));
        }
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[test]
fn broker_process_launch_connect_and_shutdown() {
    let workspace = temp_workspace("launch-connect-shutdown");
    let mut process = spawn_broker(&workspace, 64 * 1024);
    let response = send_request(&process.endpoint, &health_request("request-1", "nonce-1"));
    assert_eq!(response["status"], "accepted");
    assert_eq!(response["health"]["persistence_ready"], true);
    assert_eq!(
        response["health"]["audit_persistence"],
        "durable_file_store"
    );

    let shutdown = send_request(
        &process.endpoint,
        &shutdown_request(&process.endpoint.session_id),
    );
    assert_eq!(shutdown["status"], "accepted");
    assert_eq!(shutdown["shutdown_requested"], true);
    let status = process.child.wait().unwrap();
    assert!(status.success());

    let unavailable = try_send_raw(
        &process.endpoint,
        &process.endpoint.session_secret,
        &health_request("request-after-shutdown", "nonce-after-shutdown"),
    );
    if let Ok(response) = unavailable {
        assert_ne!(response["status"], "accepted");
    }
}

#[test]
fn broker_ipc_rejects_unauthenticated_malformed_oversized_and_stale_requests() {
    let workspace = temp_workspace("negative-ipc");
    let process = spawn_broker(&workspace, 512);

    let unauthenticated = send_raw(
        &process.endpoint,
        "wrong-secret",
        &health_request("request-unauth", "nonce-unauth"),
    );
    assert_eq!(unauthenticated["status"], "rejected");
    assert_eq!(
        unauthenticated["error"]["code"],
        "broker_authentication_failed"
    );

    let malformed = send_request(&process.endpoint, "{not-json");
    assert_eq!(malformed["status"], "rejected");
    assert_eq!(malformed["error"]["code"], "broker_request_malformed");

    let oversized_payload = format!(
        "{{\"request_id\":\"oversized\",\"operation\":\"health\",\"payload_hash\":\"sha256:{}\",\"nonce\":\"{}\",\"issued_at\":\"{}\",\"metadata\":{{\"padding\":\"{}\"}}}}",
        "a".repeat(64),
        "nonce-oversized",
        BrokerRequestEnvelope::current_issued_at(),
        "x".repeat(2048)
    );
    let oversized = send_request(&process.endpoint, &oversized_payload);
    assert_eq!(oversized["status"], "rejected");
    assert_eq!(oversized["error"]["code"], "broker_request_oversized");

    let oversized_without_newline = send_raw_without_request_newline(
        &process.endpoint,
        &process.endpoint.session_secret,
        &oversized_payload,
    );
    assert_eq!(oversized_without_newline["status"], "rejected");
    assert_eq!(
        oversized_without_newline["error"]["code"],
        "broker_request_oversized"
    );

    abandon_connection_before_auth(&process.endpoint);
    let after_abandoned_connection = send_request(
        &process.endpoint,
        &health_request("request-after-abandoned", "nonce-after-abandoned"),
    );
    assert_eq!(after_abandoned_connection["status"], "accepted");

    let stale = send_request(
        &process.endpoint,
        &health_request_at("request-stale", "nonce-stale", "2000-01-01T00:00:00Z"),
    );
    assert_eq!(stale["status"], "rejected");
    assert_eq!(stale["error"]["code"], "broker_issued_at_invalid");
}

#[test]
fn broker_ipc_rejects_replay_after_process_restart() {
    let workspace = temp_workspace("restart-replay");
    {
        let process = spawn_broker(&workspace, 64 * 1024);
        let response = send_request(&process.endpoint, &health_request("request-1", "nonce-1"));
        assert_eq!(response["status"], "accepted");
        let shutdown = send_request(
            &process.endpoint,
            &shutdown_request(&process.endpoint.session_id),
        );
        assert_eq!(shutdown["status"], "accepted");
    }

    let restarted = spawn_broker(&workspace, 64 * 1024);
    let replay = send_request(&restarted.endpoint, &health_request("request-2", "nonce-1"));
    assert_eq!(replay["status"], "rejected");
    assert_eq!(replay["error"]["code"], "broker_replay_detected");
}

fn spawn_broker(workspace: &Workspace, max_request_bytes: usize) -> BrokerProcess {
    let binary = env!("CARGO_BIN_EXE_gui_shell_rust_helper");
    let _ = fs::remove_file(&workspace.session_file);
    let mut child = Command::new(binary)
        .arg("broker-server")
        .arg("--store-dir")
        .arg(&workspace.store_dir)
        .arg("--session-file")
        .arg(&workspace.session_file)
        .arg("--max-request-bytes")
        .arg(max_request_bytes.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let endpoint = wait_for_endpoint(&workspace.session_file).unwrap_or_else(|| {
        let _ = child.kill();
        panic!("broker endpoint file was not created")
    });
    BrokerProcess { child, endpoint }
}

fn wait_for_endpoint(path: &PathBuf) -> Option<BrokerEndpoint> {
    for _ in 0..100 {
        if let Ok(raw) = fs::read_to_string(path) {
            if let Ok(endpoint) = serde_json::from_str::<BrokerEndpoint>(&raw) {
                return Some(endpoint);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    None
}

fn send_request(endpoint: &BrokerEndpoint, request: &str) -> Value {
    send_raw(endpoint, &endpoint.session_secret, request)
}

fn send_raw(endpoint: &BrokerEndpoint, secret: &str, request: &str) -> Value {
    try_send_raw(endpoint, secret, request).unwrap()
}

fn send_raw_without_request_newline(
    endpoint: &BrokerEndpoint,
    secret: &str,
    request: &str,
) -> Value {
    let mut stream = TcpStream::connect((endpoint.host.as_str(), endpoint.port)).unwrap();
    stream.write_all(secret.as_bytes()).unwrap();
    stream.write_all(b"\n").unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).unwrap();
    serde_json::from_str(response.trim()).unwrap()
}

fn abandon_connection_before_auth(endpoint: &BrokerEndpoint) {
    let _stream = TcpStream::connect((endpoint.host.as_str(), endpoint.port)).unwrap();
}

fn try_send_raw(endpoint: &BrokerEndpoint, secret: &str, request: &str) -> std::io::Result<Value> {
    let mut stream = TcpStream::connect((endpoint.host.as_str(), endpoint.port))?;
    stream.write_all(secret.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.write_all(request.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    if response.trim().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "broker IPC response was empty",
        ));
    }
    serde_json::from_str(response.trim())
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

fn health_request(request_id: &str, nonce: &str) -> String {
    health_request_at(
        request_id,
        nonce,
        &BrokerRequestEnvelope::current_issued_at(),
    )
}

fn health_request_at(request_id: &str, nonce: &str, issued_at: &str) -> String {
    format!(
        "{{\"request_id\":\"{}\",\"operation\":\"health\",\"payload_hash\":\"sha256:{}\",\"nonce\":\"{}\",\"issued_at\":\"{}\",\"metadata\":{{\"client\":\"desktop_flutter\"}}}}",
        request_id,
        "a".repeat(64),
        nonce,
        issued_at
    )
}

fn shutdown_request(session_id: &str) -> String {
    format!(
        "{{\"request_id\":\"shutdown-request\",\"session_id\":\"{}\",\"operation\":\"shutdown\",\"payload_hash\":\"sha256:{}\",\"nonce\":\"shutdown-nonce-{}\",\"issued_at\":\"{}\",\"metadata\":{{\"client\":\"desktop_flutter\"}}}}",
        session_id,
        "a".repeat(64),
        session_id,
        BrokerRequestEnvelope::current_issued_at()
    )
}

struct Workspace {
    store_dir: PathBuf,
    session_file: PathBuf,
}

fn temp_workspace(test_name: &str) -> Workspace {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "gui-shell-broker-ipc-{test_name}-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    Workspace {
        store_dir: root.join("store"),
        session_file: root.join("broker_session.json"),
    }
}
