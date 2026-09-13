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

#[test]
fn workspace_control_is_rejected_over_normal_authenticated_ipc() {
    let workspace = temp_workspace("workspace-owner-boundary");
    let process = spawn_broker(&workspace, 64 * 1024);
    for op in ["作業領域承認", "作業領域失効"] {
        let payload = serde_json::json!({"作業領域ID":"workspace-a","登録hash":"sha256:".to_string()+&"a".repeat(64),"表示範囲":"full"});
        let request = serde_json::json!({"request_id":op,"nonce":op,"session_id":process.endpoint.session_id,
            "operation":op,"issued_at":BrokerRequestEnvelope::current_issued_at(),"metadata":{},
            "payload_hash":gui_shell_rust_helper::audit_hash::sha256_tagged(payload.to_string().as_bytes()),"payload":payload});
        let result = send_request(&process.endpoint,&request.to_string());
        assert_eq!(result["status"],"rejected");
        assert_eq!(result["error"]["code"],"権限拒否");
        assert!(result["body"].is_null());
    }
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
        null_payload_hash_hex(),
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

#[test]
fn broker_ipc_rejects_payload_hash_mismatch() {
    let workspace = temp_workspace("payload-hash-mismatch");
    let process = spawn_broker(&workspace, 64 * 1024);

    let accepted = send_request(
        &process.endpoint,
        &normalize_payload_request(
            &process.endpoint.session_id,
            "request-normalize-accepted",
            "nonce-normalize-accepted",
            "sha256:787a213a62a6dd88756a81d1b68234f88759d36308adc933625aa48a4507a93b",
        ),
    );
    assert_eq!(accepted["status"], "accepted");

    let mismatched = send_request(
        &process.endpoint,
        &normalize_payload_request(
            &process.endpoint.session_id,
            "request-normalize-mismatch",
            "nonce-normalize-mismatch",
            "sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b",
        ),
    );
    assert_eq!(mismatched["status"], "rejected");
    assert_eq!(mismatched["error"]["code"], "broker_payload_hash_mismatch");
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
        panic!("broker endpoint fileが作成されなかった")
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
        "broker IPC responseが空だった",
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
        null_payload_hash_hex(),
        nonce,
        issued_at
    )
}

fn shutdown_request(session_id: &str) -> String {
    format!(
        "{{\"request_id\":\"shutdown-request\",\"session_id\":\"{}\",\"operation\":\"shutdown\",\"payload_hash\":\"sha256:{}\",\"nonce\":\"shutdown-nonce-{}\",\"issued_at\":\"{}\",\"metadata\":{{\"client\":\"desktop_flutter\"}}}}",
        session_id,
        null_payload_hash_hex(),
        session_id,
        BrokerRequestEnvelope::current_issued_at()
    )
}

fn normalize_payload_request(
    session_id: &str,
    request_id: &str,
    nonce: &str,
    payload_hash: &str,
) -> String {
    format!(
        "{{\"request_id\":\"{}\",\"session_id\":\"{}\",\"operation\":\"normalize_payload\",\"payload_hash\":\"{}\",\"nonce\":\"{}\",\"issued_at\":\"{}\",\"metadata\":{{\"client\":\"desktop_flutter\"}},\"payload\":{{\"client_payload\":\"desktop_flutter_authority_probe\"}}}}",
        request_id,
        session_id,
        payload_hash,
        nonce,
        BrokerRequestEnvelope::current_issued_at()
    )
}

fn null_payload_hash_hex() -> &'static str {
    "74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b"
}

#[test]
fn owner制御資格を通常資格や要求metadataで置換できない() {
    use gui_shell_rust_helper::audit_hash::sha256_tagged;
    use serde_json::json;
    let workspace = temp_workspace("owner-control");
    let owner_file = workspace.session_file.with_file_name("owner.json");
    let mut child = Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"))
        .args(["broker-server", "--store-dir"]).arg(&workspace.store_dir)
        .arg("--session-file").arg(&workspace.session_file)
        .arg("--owner-session-file").arg(&owner_file)
        .args(["--minidora-runtime", "local=127.0.0.1:9"])
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let endpoint = wait_for_endpoint(&workspace.session_file).unwrap_or_else(|| { let _ = child.kill(); panic!("通常資格file不在") });
    let process = BrokerProcess { child, endpoint };
    let owner = wait_for_endpoint(&owner_file).unwrap();
    assert_ne!(process.endpoint.session_secret, owner.session_secret);
    let workspace_cli=Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"))
        .args(["作業領域制御","--session-file"]).arg(&owner_file).arg("作業領域一覧").output().unwrap();
    assert!(workspace_cli.status.success(),"{}",String::from_utf8_lossy(&workspace_cli.stderr));
    let workspace_list:Value=serde_json::from_slice(&workspace_cli.stdout).unwrap();
    assert_eq!(workspace_list["作業領域"],json!([]));
    let unauthorized_cli=Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"))
        .args(["作業領域制御","--session-file"]).arg(&workspace.session_file)
        .args(["作業領域承認","workspace-a",&("sha256:".to_string()+&"a".repeat(64)),"full"]).output().unwrap();
    assert!(!unauthorized_cli.status.success());
    assert!(String::from_utf8_lossy(&unauthorized_cli.stderr).contains("owner制御資格が必要"));
    let request = |op: &str, payload: Value| {
        let nonce = gui_shell_rust_helper::broker::dialogue::識別子生成().unwrap();
        json!({"request_id": nonce, "nonce":nonce, "session_id":process.endpoint.session_id,
            "issued_at":BrokerRequestEnvelope::current_issued_at(),"operation":op,"metadata":{},
            "payload_hash":sha256_tagged(payload.to_string().as_bytes()),"payload":payload}).to_string()
    };
    let normal = send_request(&process.endpoint,&request("対話承認待ち",json!({})));
    assert_eq!(normal["error"]["code"],"権限拒否");
    let listed = send_request(&owner,&request("対話承認待ち",json!({})));
    assert_eq!(listed["status"],"accepted");
    let s=send_request(&process.endpoint,&request("対話開始",json!({"実行系ID":"local"})));
    assert_eq!(s["status"],"accepted");
    let p=send_request(&process.endpoint,&request("対話送信",json!({"対話セッションID":s["body"]["対話セッションID"],"入力":"こんにちは"})));
    assert_eq!(p["body"]["状態"],"承認待ち");
    let payload=json!({"要求ID":p["body"]["要求ID"],"要求hash":p["body"]["要求hash"],"表示範囲":"full"});
    assert_eq!(send_request(&process.endpoint,&request("対話承認",payload.clone()))["error"]["code"],"権限拒否");
    let mut forged: Value=serde_json::from_str(&request("対話承認",payload.clone())).unwrap();
    forged["metadata"]=json!({"authority":"owner"});
    assert_ne!(send_request(&process.endpoint,&forged.to_string())["status"],"accepted");
    let cli=Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"))
        .args(["対話承認操作","--session-file"]).arg(&owner_file).arg("一覧").output().unwrap();
    assert!(cli.status.success(),"{}",String::from_utf8_lossy(&cli.stderr));
    let out=String::from_utf8(cli.stdout).unwrap();
    assert!(out.contains("こんにちは")); assert!(!out.contains(&owner.session_secret));
    assert_eq!(send_request(&owner,&request("対話承認",payload.clone()))["status"],"accepted");
    assert_ne!(send_request(&owner,&request("対話承認",payload))["status"],"accepted");
    let mut record = Value::Null;
    for _ in 0..250 {
        let v = send_request(&process.endpoint,&request("対話取得",json!({"要求ID":p["body"]["要求ID"]})));
        assert_eq!(v["status"], "accepted");
        if v["body"]["状態"] == "完了" {
            assert_eq!(v["body"]["結果"]["状態"], "失敗");
            record = v["body"]["実行記録"].clone();
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(record.is_object(), "実通信失敗の記録待機期限");
    let record_file = workspace.store_dir.parent().unwrap().join("execution-record.json");
    fs::write(&record_file, serde_json::to_vec(&record).unwrap()).unwrap();
    let schema_check = "import json,sys; from pathlib import Path; sys.path.insert(0,sys.argv[1]); from tooling.schema_check.check_schemas import validate_instance; schema=json.loads((Path(sys.argv[1])/'specs/runtime_execution_record.schema.json').read_text(encoding='utf-8')); record=json.loads(Path(sys.argv[2]).read_text(encoding='utf-8')); errors=validate_instance(record,schema); assert not errors, errors";
    let checked = Command::new(if cfg!(windows) {"python"} else {"python3"})
        .args(["-c", schema_check])
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .arg(&record_file).output().unwrap();
    assert!(checked.status.success(), "{}", String::from_utf8_lossy(&checked.stderr));
    for key in ["作成時刻", "開始時刻", "終了時刻"] {
        assert!(record[key].as_i64().is_some());
    }
    assert_eq!(send_request(&process.endpoint, &request("対話終了", json!({"対話セッションID":s["body"]["対話セッションID"]})))["status"], "accepted");
    assert_eq!(send_request(&process.endpoint, &request("対話履歴一覧", json!({"after":0,"limit":1})))["status"], "rejected");
    for invalid in [json!({"after":0,"limit":0}), json!({"after":0,"limit":101}), json!({"after":999999,"limit":1}), json!({"after":0,"limit":1,"owner":true})] {
        assert_ne!(send_request(&owner, &request("対話履歴一覧", invalid))["status"], "accepted");
    }
    let mut cursor = 0;
    let mut states = Vec::new();
    for _ in 0..4 {
        let page = send_request(&owner, &request("対話履歴一覧", json!({"after":cursor,"limit":1})));
        assert_eq!(page["status"], "accepted");
        assert!(page["body"]["next_cursor"].as_u64().unwrap() > cursor);
        cursor = page["body"]["next_cursor"].as_u64().unwrap();
        states.extend(page["body"]["entries"].as_array().unwrap().iter().map(|v|v["record"]["状態"].as_str().unwrap().to_owned()));
        if page["body"]["has_more"] == false { break; }
    }
    assert_eq!(states, ["承認待ち", "実行中", "失敗"]);
    for (filter, count) in [(json!({"状態":"失敗"}),1), (json!({"状態":"成功"}),0), (json!({"要求ID":record["要求ID"],"実行系ID":record["実行系ID"],"対話セッションID":record["対話セッションID"]}),3)] {
        let response = send_request(&owner, &request("対話履歴一覧", json!({"after":0,"limit":100,"filter":filter})));
        assert_eq!(response["status"], "accepted");
        assert_eq!(response["body"]["entries"].as_array().unwrap().len(),count);
        assert_eq!(response["body"]["has_more"],false);
    }
    for filter in [json!(null),json!({"状態":"不明"}),json!({"要求ID":""}),json!({"実行系ID":"../outside"}),json!({"owner":true})] {
        assert_ne!(send_request(&owner,&request("対話履歴一覧",json!({"after":0,"limit":1,"filter":filter})))["status"],"accepted");
    }
    for (filter,count) in [(json!({}),1),(json!({"状態":"承認待ち"}),0),(json!({"状態":"失敗"}),1)] {
        let grouped=send_request(&owner,&request("対話履歴一覧",json!({"after":0,"limit":100,"filter":filter,"latest_per_request":true})));
        assert_eq!(grouped["status"],"accepted");
        let entries=grouped["body"]["entries"].as_array().unwrap();
        assert_eq!(entries.len(),count);
        if count>0 {assert_eq!(entries[0]["record"]["状態"],"失敗");}
    }
    let read_payload = |approval: &Value, runtime: &str| json!({"approval_id":approval,"query":{"after":0,"limit":100,"filter":{"実行系ID":runtime}}});
    assert_eq!(send_request(&process.endpoint,&request("対話履歴閲覧状態",json!({})))["body"]["grant"],Value::Null);
    for op in ["対話履歴承認","対話履歴失効"] {
        assert_ne!(send_request(&process.endpoint,&request(op,json!({"実行系ID":"local"})))["status"],"accepted");
    }
    assert_ne!(send_request(&process.endpoint,&request("対話履歴閲覧",read_payload(&json!("a".repeat(32)),"local")))["status"],"accepted");
    let approval_cli=Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"))
        .args(["対話承認操作","--session-file"]).arg(&owner_file).args(["履歴承認","local"]).output().unwrap();
    assert!(approval_cli.status.success());
    let access:Value=serde_json::from_slice(&approval_cli.stdout).unwrap();
    let approval=&access["grant"]["approval_id"];
    let viewed=send_request(&process.endpoint,&request("対話履歴閲覧",read_payload(approval,"local")));
    assert_eq!(viewed["status"],"accepted");
    assert_eq!(viewed["body"]["page"]["entries"].as_array().unwrap().len(),3);
    fs::write(&record_file,serde_json::to_vec(&viewed["body"]).unwrap()).unwrap();
    let checked=Command::new(if cfg!(windows){"python"}else{"python3"})
        .args(["-c",&schema_check.replace("runtime_execution_record.schema.json","runtime_history_access.schema.json")])
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")).arg(&record_file).output().unwrap();
    assert!(checked.status.success(),"{}",String::from_utf8_lossy(&checked.stderr));
    for invalid in [read_payload(approval,"other"),read_payload(&json!("b".repeat(32)),"local"),json!({"approval_id":approval,"query":{"after":0,"limit":1}})] {
        assert_ne!(send_request(&process.endpoint,&request("対話履歴閲覧",invalid))["status"],"accepted");
    }
    assert_eq!(send_request(&owner,&request("対話履歴失効",json!({})))["status"],"accepted");
    assert_ne!(send_request(&process.endpoint,&request("対話履歴閲覧",read_payload(approval,"local")))["status"],"accepted");
    let renewed=send_request(&owner,&request("対話履歴承認",json!({"実行系ID":"local"})));
    assert_eq!(renewed["status"],"accepted");
    assert_ne!(renewed["body"]["grant"]["approval_id"],*approval);
    assert_ne!(send_request(&process.endpoint,&request("対話履歴閲覧",read_payload(approval,"local")))["status"],"accepted");
    drop(process);
    fs::remove_file(&workspace.session_file).unwrap();
    fs::remove_file(&owner_file).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"))
        .args(["broker-server", "--store-dir"]).arg(&workspace.store_dir)
        .arg("--session-file").arg(&workspace.session_file)
        .arg("--owner-session-file").arg(&owner_file)
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let endpoint = wait_for_endpoint(&workspace.session_file).unwrap();
    let restarted = BrokerProcess {child, endpoint};
    wait_for_endpoint(&owner_file).unwrap();
    let state=Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"))
        .args(["対話承認操作","--session-file"]).arg(&workspace.session_file).arg("履歴閲覧状態").output().unwrap();
    assert!(state.status.success());
    let state:Value=serde_json::from_slice(&state.stdout).unwrap();
    assert_eq!(state["grant"],Value::Null);
    let cli = Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"))
        .args(["対話承認操作", "--session-file"]).arg(&owner_file)
        .args(["履歴", "0", "100"]).output().unwrap();
    assert!(cli.status.success(), "{}", String::from_utf8_lossy(&cli.stderr));
    let page: Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert_eq!(page["entries"].as_array().unwrap().len(), 3);
    fs::write(&record_file, &cli.stdout).unwrap();
    let checked = Command::new(if cfg!(windows) {"python"} else {"python3"})
        .args(["-c", &schema_check.replace("runtime_execution_record.schema.json", "runtime_execution_history_page.schema.json")])
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")).arg(&record_file).output().unwrap();
    assert!(checked.status.success(), "{}", String::from_utf8_lossy(&checked.stderr));
    let grouped=Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"))
        .args(["対話承認操作","--session-file"]).arg(&owner_file).args(["履歴集約","0","100"]).output().unwrap();
    assert!(grouped.status.success());
    let grouped:Value=serde_json::from_slice(&grouped.stdout).unwrap();
    assert_eq!(grouped["entries"].as_array().unwrap().len(),1);
    assert_eq!(grouped["entries"][0]["record"]["状態"],"失敗");
    let filtered = Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"))
        .args(["対話承認操作", "--session-file"]).arg(&owner_file)
        .args(["履歴", "0", "1", "状態", "失敗"]).output().unwrap();
    assert!(filtered.status.success());
    let filtered: Value = serde_json::from_slice(&filtered.stdout).unwrap();
    assert_eq!(filtered["entries"].as_array().unwrap().len(),1);
    assert_eq!(filtered["entries"][0]["record"]["状態"],"失敗");
    assert_eq!(filtered["has_more"],false);
    let duplicate = Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"))
        .args(["対話承認操作", "--session-file"]).arg(&owner_file)
        .args(["履歴", "0", "1", "状態", "失敗", "状態", "成功"]).output().unwrap();
    assert!(!duplicate.status.success());
    assert!(duplicate.stdout.is_empty());
    let live_audit = workspace.store_dir.join("audit.jsonl");
    let saved_audit = fs::read(&live_audit).unwrap();
    let mut broken_audit = saved_audit.clone();
    broken_audit.extend_from_slice(b"broken\n");
    fs::write(&live_audit, broken_audit).unwrap();
    let denied = Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"))
        .args(["対話承認操作", "--session-file"]).arg(&owner_file)
        .args(["履歴", "0", "100"]).output().unwrap();
    assert!(!denied.status.success());
    assert!(denied.stdout.is_empty());
    fs::write(&live_audit, saved_audit).unwrap();
    drop(restarted);
    let (_, reopened) = gui_shell_rust_helper::broker::BrokerPersistentStore::open_or_create(&workspace.store_dir, "record-check").unwrap();
    let history: Vec<Value> = reopened.audit_log.events().iter().filter_map(|e| {
        e.reason.strip_prefix("対話実行記録:").map(|body| {
            assert_eq!(e.payload_hash, sha256_tagged(body.as_bytes()));
            let value: Value = serde_json::from_str(body).unwrap();
            assert_eq!(value["実行記録"]["要求ID"], e.request_id);
            value
        })
    }).collect();
    assert_eq!(history.iter().map(|v| v["状態"].as_str().unwrap()).collect::<Vec<_>>(), ["承認待ち", "実行中", "失敗"]);
    assert_eq!(history.last().unwrap()["実行記録"], record);
    fs::write(&record_file, serde_json::to_vec(&history).unwrap()).unwrap();
    let checked = Command::new(if cfg!(windows) {"python"} else {"python3"})
        .args(["-c", "import json,sys; from pathlib import Path; sys.path.insert(0,sys.argv[1]); from tooling.schema_check.check_schemas import validate_instance; schema=json.loads((Path(sys.argv[1])/'specs/runtime_execution_history.schema.json').read_text(encoding='utf-8')); records=json.loads(Path(sys.argv[2]).read_text(encoding='utf-8')); errors=[e for r in records for e in validate_instance(r,schema)]; assert not errors,errors"])
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")).arg(&record_file).output().unwrap();
    assert!(checked.status.success(), "{}", String::from_utf8_lossy(&checked.stderr));
    for (key, reason) in [("作成監査ID", "対話承認待ち作成"), ("開始監査ID", "対話送信承認"), ("終了監査ID", "対話完了")] {
        let event = reopened.audit_log.events().iter().find(|e| Some(e.event_id.as_str()) == record[key].as_str()).unwrap();
        assert_eq!(event.request_id, p["body"]["要求ID"].as_str().unwrap());
        assert!(event.reason.starts_with(reason));
    }
    let audit_path = workspace.store_dir.join("audit.jsonl");
    let original = fs::read_to_string(&audit_path).unwrap();
    assert!(original.contains("対話実行記録:"));
    let mut changed = false;
    let tampered: Vec<String> = original.lines().map(|line| {
        let mut event: Value = serde_json::from_str(line).unwrap();
        if !changed {
            if let Some(body) = event["reason"].as_str().unwrap().strip_prefix("対話実行記録:") {
                let mut value: Value = serde_json::from_str(body).unwrap();
                value["実行記録"]["作成時刻"] = json!(0);
                event["reason"] = json!(format!("対話実行記録:{value}"));
                changed = true;
            }
        }
        event.to_string()
    }).collect();
    assert!(changed);
    fs::write(&audit_path, tampered.join("\n") + "\n").unwrap();
    assert!(gui_shell_rust_helper::broker::BrokerPersistentStore::open_or_create(&workspace.store_dir, "tamper-check").is_err());
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

#[test]
fn 履歴再要求は新セッションと新承認を必要とする() {
    use gui_shell_rust_helper::audit_hash::sha256_tagged;
    use serde_json::json;
    let workspace=temp_workspace("history-replay");
    let owner_file=workspace.session_file.with_file_name("owner.json");
    let child=Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"))
        .args(["broker-server","--store-dir"]).arg(&workspace.store_dir).arg("--session-file").arg(&workspace.session_file)
        .arg("--owner-session-file").arg(&owner_file).args(["--minidora-runtime","local=127.0.0.1:9"])
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let endpoint=wait_for_endpoint(&workspace.session_file).unwrap();
    let process=BrokerProcess{child,endpoint};let owner=wait_for_endpoint(&owner_file).unwrap();
    let request=|op:&str,payload:Value|{let nonce=gui_shell_rust_helper::broker::dialogue::識別子生成().unwrap();json!({"request_id":nonce,"nonce":nonce,"session_id":process.endpoint.session_id,"operation":op,"payload_hash":sha256_tagged(payload.to_string().as_bytes()),"payload":payload,"metadata":{},"issued_at":BrokerRequestEnvelope::current_issued_at()}).to_string()};
    let session=send_request(&process.endpoint,&request("対話開始",json!({"実行系ID":"local"})))["body"]["対話セッションID"].clone();
    let original=send_request(&process.endpoint,&request("対話送信",json!({"対話セッションID":session,"入力":"原入力"})))["body"].clone();
    assert_eq!(send_request(&process.endpoint,&request("対話終了",json!({"対話セッションID":session})))["status"],"accepted");
    let history=send_request(&owner,&request("対話履歴一覧",json!({"after":0,"limit":100})));
    let parent=&history["body"]["entries"][0];
    let grant=send_request(&owner,&request("対話履歴承認",json!({"実行系ID":"local"})))["body"]["grant"]["approval_id"].clone();
    let selection=json!({"approval_id":grant,"実行系ID":"local","参照監査ID":parent["audit_event_id"],"参照event_hash":parent["event_hash"],"入力":"原入力"});
    for key in ["approval_id","参照event_hash","入力","実行系ID"] {
        let mut bad=selection.clone();bad[key]=json!("invalid");
        assert_ne!(send_request(&process.endpoint,&request("対話再実行",bad))["status"],"accepted");
    }
    let mut stale=selection.clone();stale["対話セッションID"]=session.clone();
    assert_ne!(send_request(&process.endpoint,&request("対話再実行",stale))["status"],"accepted");
    for (op,input) in [("対話再実行","原入力"),("対話分岐","変更入力")] {
        let mut payload=selection.clone();payload["入力"]=json!(input);
        let response=send_request(&process.endpoint,&request(op,payload));assert_eq!(response["status"],"accepted", "{response}");
        let body=&response["body"];
        assert_ne!(body["要求ID"],original["要求ID"]);assert_ne!(body["対話セッションID"],session);assert_eq!(body["状態"],"承認待ち");
        assert_ne!(send_request(&owner,&request("対話承認",json!({"要求ID":body["要求ID"],"要求hash":original["要求hash"],"表示範囲":"full"})))["status"],"accepted");
        let progress=send_request(&process.endpoint,&request("対話取得",json!({"要求ID":body["要求ID"]})));
        assert_eq!(progress["body"]["状態"],"承認待ち");assert_eq!(progress["body"]["実行記録"]["開始時刻"],Value::Null);
        assert_eq!(send_request(&process.endpoint,&request("対話終了",json!({"対話セッションID":body["対話セッションID"]})))["status"],"accepted");
    }
    assert_eq!(send_request(&owner,&request("対話履歴失効",json!({})))["status"],"accepted");
    assert_ne!(send_request(&process.endpoint,&request("対話分岐",selection))["status"],"accepted");
}
