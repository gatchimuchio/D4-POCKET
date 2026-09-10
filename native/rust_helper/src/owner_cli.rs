//! owner自身が端末から明示実行する制御面。通常UIはこの資格fileを読まない。
use gui_shell_rust_helper::audit_hash::sha256_tagged;
use gui_shell_rust_helper::broker::dialogue::識別子生成;
use gui_shell_rust_helper::broker::{BrokerEndpoint, BrokerRequestEnvelope};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

pub fn 実行(args: &[String]) -> Result<(), String> {
    if args.len() < 3 || args[0] != "--session-file" {
        return Err("使用法: 対話承認操作 --session-file <owner資格file> 一覧 | 承認 <要求ID> <要求hash> <表示範囲>".into());
    }
    let (operation, payload) = match args[2].as_str() {
        "一覧" if args.len() == 3 => ("対話承認待ち", json!({})),
        "承認" if args.len() == 6 => (
            "対話承認",
            json!({"要求ID": args[3], "要求hash": args[4], "表示範囲": args[5]}),
        ),
        _ => return Err("承認操作または引数が不正".into()),
    };
    let file = std::fs::File::open(&args[1]).map_err(|_| "owner資格fileを開けない")?;
    let mut raw = Vec::new();
    file.take(8193)
        .read_to_end(&mut raw)
        .map_err(|_| "owner資格fileを読めない")?;
    if raw.len() > 8192 {
        return Err("owner資格fileが上限超過".into());
    }
    let endpoint: BrokerEndpoint =
        serde_json::from_slice(&raw).map_err(|_| "owner資格fileの形式が不正")?;
    if endpoint.host != "127.0.0.1"
        || endpoint.port == 0
        || endpoint.transport != "authenticated_loopback_tcp"
        || endpoint.session_secret.len() != 64
        || !endpoint
            .session_secret
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
    {
        return Err("owner接続先が不正".into());
    }
    let request = json!({"request_id": 識別子生成().map_err(|_| "乱数生成失敗")?, "session_id": endpoint.session_id,
        "operation": operation, "payload": payload, "payload_hash": sha256_tagged(payload.to_string().as_bytes()),
        "nonce": 識別子生成().map_err(|_| "乱数生成失敗")?, "issued_at": BrokerRequestEnvelope::current_issued_at(), "metadata": {}});
    let mut stream = TcpStream::connect_timeout(
        &SocketAddr::from((Ipv4Addr::LOCALHOST, endpoint.port)),
        Duration::from_secs(3),
    )
    .map_err(|_| "brokerへ接続できない")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| "受信期限を設定できない")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| "送信期限を設定できない")?;
    writeln!(stream, "{}\n{}", endpoint.session_secret, request)
        .map_err(|_| "承認操作を送信できない")?;
    let mut line = String::new();
    BufReader::new(stream)
        .take(4 * 1024 * 1024 + 1)
        .read_line(&mut line)
        .map_err(|_| "承認結果を受信できない")?;
    if line.len() > 4 * 1024 * 1024 {
        return Err("承認結果が上限超過".into());
    }
    let response: Value = serde_json::from_str(&line).map_err(|_| "承認結果の形式が不正")?;
    // 資格や完全な監査recordは出力しない。要求本文はownerが確認する操作だけに表示する。
    if response["status"] != "accepted" {
        return Err(format!("承認操作を拒否: {}", response["error"]));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&response["body"]).map_err(|_| "結果の表示に失敗")?
    );
    Ok(())
}
