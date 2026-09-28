use super::*;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct TestDirectory(PathBuf);

impl Drop for TestDirectory {
    fn drop(&mut self) {
        assert_eq!(
            self.0.parent(),
            Some(std::env::temp_dir().as_path()),
            "試験directoryはTemp直下に限定する"
        );
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn owner_request(broker: &mut Broker, id: &str, operation: &str, payload: Value) -> BrokerResponse {
    let request = json!({
        "request_id": id,
        "session_id": "mcp-disconnect-session",
        "operation": operation,
        "payload_hash": canonical_payload_hash(Some(&payload)),
        "nonce": format!("nonce-{id}"),
        "issued_at": BrokerRequestEnvelope::current_issued_at(),
        "metadata": {},
        "payload": payload
    });
    broker.owner要求処理(&request.to_string())
}

#[derive(serde::Serialize)]
struct DisconnectRequest<'a> {
    #[serde(rename = "版")]
    version: u8,
    #[serde(rename = "操作")]
    operation: &'static str,
    #[serde(rename = "ServerID")]
    server_id: &'a str,
}

fn disconnect_request(server_id: &str) -> Value {
    serde_json::to_value(DisconnectRequest {
        version: 1,
        operation: "切断",
        server_id,
    })
    .expect("切断要求を構成")
}

fn normal_request(
    broker: &mut Broker,
    id: &str,
    operation: BrokerOperation,
    payload: Value,
) -> BrokerResponse {
    let mut request = BrokerRequestEnvelope::health(id, &format!("nonce-{id}"));
    request.session_id = Some("mcp-disconnect-session".into());
    request.operation = Some(operation);
    request.payload = Some(payload);
    request.issued_at = Some(BrokerRequestEnvelope::current_issued_at());
    request.refresh_payload_hash();
    broker.handle(request)
}

#[test]
#[allow(non_snake_case)]
fn MCP切断はowner専用でprocess群停止後に永続Auditと記録解消を確定する() {
    const FIXTURE: &str = r#"@echo off
start "" /b "%SystemRoot%\System32\ping.exe" -t 127.0.0.1 > nul
echo ready>"__READY_PATH__"
echo {"jsonrpc":"2.0","id":1,"result":{"supportedVersions":["__PROTOCOL_VERSION__"],"capabilities":{}},"_meta":{"io.modelcontextprotocol/serverInfo":{"name":"disconnect-fixture","version":"1"}}}
"%SystemRoot%\System32\ping.exe" -t 127.0.0.1 > nul
"#;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("時計")
        .as_nanos();
    let temp_root = std::env::temp_dir().join(format!(
        "gui-shell-mcp-disconnect-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir(&temp_root).expect("試験rootを作成");
    let root = TestDirectory(temp_root);
    let workspace = root.0.join("workspace");
    fs::create_dir(&workspace).expect("fixture Workspaceを作成");
    let store = root.0.join("store");
    let mut broker = Broker::new_persistent("mcp-disconnect-session", &store).unwrap();

    const SYSTEM_ROOT_ENVIRONMENT_KEY: &str = "SystemRoot";
    let system_root = PathBuf::from(
        std::env::var_os(SYSTEM_ROOT_ENVIRONMENT_KEY)
            .expect("Windows system root環境変数を取得できない"),
    );
    let executable = system_root.join("System32").join("cmd.exe");
    let script_path = workspace.join("mcp-disconnect-fixture.cmd");
    let ready_path = workspace.join("descendant-ready");
    let script = FIXTURE
        .replace("__READY_PATH__", &ready_path.to_string_lossy())
        .replace("__PROTOCOL_VERSION__", crate::mcp::MODERN_PROTOCOL_VERSION);
    fs::write(&script_path, script).expect("fixture scriptを作成");

    let mut connect_payload: Value = serde_json::from_str(include_str!(
        "../../../../examples/contracts/mcp_connection.valid.json"
    ))
    .expect("既存のMCP接続fixture");
    connect_payload["実行file"] = json!(executable.to_string_lossy());
    connect_payload["引数"] = json!(["/D", "/C", script_path.to_string_lossy()]);
    connect_payload["workspace"] = json!(workspace.to_string_lossy());
    let connected = owner_request(&mut broker, "connect-request", "MCP接続", connect_payload);
    assert_eq!(connected.status, BrokerStatus::Accepted, "MCP stdio接続");
    assert!(broker.mcp_connections.contains_key("mcp-fixture"));

    let started = Instant::now();
    while !ready_path.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "fixture descendant起動marker"
        );
        thread::sleep(Duration::from_millis(10));
    }
    assert!(broker
        .mcp_connections
        .get_mut("mcp-fixture")
        .unwrap()
        .connection
        .is_alive());

    let payload = disconnect_request("mcp-fixture");
    let ordinary = normal_request(
        &mut broker,
        "ordinary-disconnect-request",
        BrokerOperation::MCP切断,
        payload.clone(),
    );
    assert_eq!(
        ordinary.status,
        BrokerStatus::Rejected,
        "通常channelからは拒否"
    );
    assert_eq!(
        ordinary.error.as_ref().map(|error| error.code.as_str()),
        Some("mcp_owner_required")
    );
    assert!(broker.mcp_connections.contains_key("mcp-fixture"));

    let malformed = owner_request(&mut broker, "malformed-disconnect-request", "MCP切断", {
        let mut payload = disconnect_request("mcp-fixture");
        payload["authority"] = json!("owner");
        payload
    });
    assert_eq!(
        malformed.status,
        BrokerStatus::Rejected,
        "未知authority fieldを拒否"
    );
    assert!(broker.mcp_connections.contains_key("mcp-fixture"));

    let unknown = owner_request(
        &mut broker,
        "unknown-disconnect-request",
        "MCP切断",
        disconnect_request("unknown-server"),
    );
    assert_eq!(
        unknown.status,
        BrokerStatus::Rejected,
        "未知Server IDを拒否"
    );
    assert!(broker.mcp_connections.contains_key("mcp-fixture"));

    let listed_before_disconnect = normal_request(
        &mut broker,
        "list-before-disconnect",
        BrokerOperation::MCP接続一覧,
        json!({"版":1}),
    );
    assert_eq!(listed_before_disconnect.status, BrokerStatus::Accepted);
    assert_eq!(
        listed_before_disconnect.evidence_source,
        EVIDENCE_SOURCE_INTERNAL_STATE
    );
    let list_body = listed_before_disconnect.body.as_ref().expect("MCP一覧");
    assert_eq!(list_body["証拠種別"], "INTERNAL_STATE");
    assert_eq!(list_body["件数"], 1);
    assert_eq!(list_body["MCP接続一覧"][0]["証拠種別"], "INTERNAL_STATE");
    assert_eq!(
        list_body["MCP接続一覧"][0]["Server"]["server_id"],
        "mcp-fixture"
    );

    let disconnected = owner_request(&mut broker, "owner-disconnect-request", "MCP切断", payload);
    assert_eq!(disconnected.status, BrokerStatus::Accepted, "owner切断");
    assert_eq!(disconnected.evidence_source, EVIDENCE_SOURCE_LIVE_RUNTIME);
    let receipt = disconnected.body.as_ref().expect("MCP切断receipt");
    assert_eq!(receipt["接続状態"], "disconnected");
    assert_eq!(receipt["権限生成"], "なし");
    assert_eq!(receipt["公開範囲"], "metadata_only");
    assert_eq!(receipt["証拠種別"], "LIVE_RUNTIME");
    assert!(!receipt.to_string().contains("credential_value"));
    assert!(!receipt.to_string().contains("mcp-disconnect-fixture.cmd"));
    assert!(!broker.mcp_connections.contains_key("mcp-fixture"));

    let listed = normal_request(
        &mut broker,
        "list-after-disconnect",
        BrokerOperation::MCP接続一覧,
        json!({"版":1}),
    );
    assert_eq!(listed.status, BrokerStatus::Accepted);
    assert_eq!(listed.body.as_ref().unwrap()["件数"], 0);

    let disconnect_audits: Vec<_> = broker
        .audit_events()
        .iter()
        .filter(|event| event.operation == "MCP切断")
        .collect();
    assert!(disconnect_audits
        .iter()
        .any(|event| event.decision == "received"));
    let accepted_audit = disconnect_audits
        .iter()
        .find(|event| event.decision == "accepted")
        .expect("process群停止後のaccepted Audit");
    assert_eq!(accepted_audit.evidence_source, EVIDENCE_SOURCE_LIVE_RUNTIME);
    assert_eq!(receipt["切断監査ID"], accepted_audit.event_id);
    assert!(disconnect_audits.iter().all(|event| {
        !event
            .reason
            .contains(&workspace.to_string_lossy().to_string())
            && !event.reason.contains("11111111111111111111111111111111")
    }));

    drop(broker);
}
