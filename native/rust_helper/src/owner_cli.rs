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
        return Err("使用法: 対話承認操作 --session-file <owner資格file> 一覧 | 承認 <要求ID> <要求hash> <表示範囲> | 端末招待 <端末ID> <接続先Host> <新規出力file> | 端末一覧 | 端末招待取消 <招待ID> | 端末失効 <結合ID>".into());
    }
    let (operation, payload) = match args[2].as_str() {
        "内容閲覧承認" if args.len()==6 => ("対話内容承認",json!({"要求ID":args[3],"保存監査ID":args[4],"保存監査hash":args[5]})),
        "内容閲覧失効" if args.len()==3 => ("対話内容失効",json!({})),

        "内容削除" if args.len()==6 => ("対話内容削除",json!({"要求ID":args[3],"保存監査ID":args[4],"保存監査hash":args[5]})),
        "削除中断確認" if args.len()==6 => ("対話削除中断確認",json!({"要求ID":args[3],"要求hash":args[4],"削除承認監査ID":args[5]})),
        "部分破棄中断確認" if args.len()==6 => ("対話部分破棄中断確認",json!({"要求ID":args[3],"要求hash":args[4],"部分保存破棄承認監査ID":args[5]})),
        "部分保存破棄" if args.len()==7 => ("対話部分保存破棄",json!({"要求ID":args[3],"要求hash":args[4],"保存試行監査ID":args[5],"暗号文hash":args[6]})),
        "保管状態" if args.len()==5 => ("対話保管状態",json!({"要求ID":args[3],"要求hash":args[4]})),
        "内容保存" if args.len() == 5 => ("対話内容保存", json!({"要求ID":args[3],"要求hash":args[4]})),
        "作業領域基準点保存" if (6..=133).contains(&args.len()) => ("作業領域基準点保存", json!({"作業領域ID":args[3],"登録hash":args[4],"相対paths":args[5..]})),
        "作業領域全体基準点保存" if args.len() == 5 => ("作業領域全体基準点保存", json!({"作業領域ID":args[3],"登録hash":args[4]})),
        "作業領域一覧" if args.len() == 3 => ("作業領域一覧", json!({})),
        "作業領域承認" if args.len() == 6 => ("作業領域承認", json!({"作業領域ID":args[3],"登録hash":args[4],"表示範囲":args[5]})),
        "作業領域失効" if args.len() == 5 => ("作業領域失効", json!({"作業領域ID":args[3],"登録hash":args[4]})),
        "端末招待" if args.len() == 6 => ("端末招待", json!({"端末ID":args[3],"接続先Host":args[4]})),
        "端末一覧" if args.len() == 3 => ("端末一覧", json!({})),
        "端末招待取消" if args.len() == 4 => ("端末招待取消", json!({"招待ID":args[3]})),
        "端末失効" if args.len() == 4 => ("端末失効", json!({"結合ID":args[3]})),
        "履歴承認" if args.len()==4 => ("対話履歴承認",json!({"実行系ID":args[3]})),
        "履歴失効" if args.len()==3 => ("対話履歴失効",json!({})),
        "履歴閲覧状態" if args.len()==3 => ("対話履歴閲覧状態",json!({})),
        "履歴" | "履歴集約" if args.len() >= 5 && args.len() <= 13 && (args.len() - 5) % 2 == 0 => {
            let mut filter = serde_json::Map::new();
            for pair in args[5..].chunks_exact(2) {
                if filter.insert(pair[0].clone(), json!(pair[1])).is_some() {
                    return Err("履歴検索条件が重複".into());
                }
            }
            ("対話履歴一覧", json!({"after":args[3].parse::<usize>().map_err(|_| "cursorが不正")?,"limit":args[4].parse::<usize>().map_err(|_| "件数が不正")?,"filter":filter,"latest_per_request":args[2]=="履歴集約"}))
        },
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
    if operation == "端末招待" {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)] {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut output = options.open(&args[5]).map_err(|_| "招待fileを新規作成できない。既存fileは上書きしない")?;
        let data = serde_json::to_vec_pretty(&response["body"]).map_err(|_|"招待の保存形式不正")?;
        output.write_all(&data).map_err(|_|"招待保存失敗")?;
        output.sync_all().map_err(|_|"招待保存の確定失敗")?;
        println!("端末招待を指定fileへ保存した。秘密を含むため対面で渡し、結合後に削除する。");
        return Ok(());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&response["body"]).map_err(|_| "結果の表示に失敗")?
    );
    Ok(())
}
