//! owner自身が端末から明示実行する制御面。通常UIはこの資格fileを読まない。
#![allow(non_snake_case)]

use gui_shell_rust_helper::audit_hash::sha256_tagged;
use gui_shell_rust_helper::broker::dialogue::識別子生成;
use gui_shell_rust_helper::broker::{
    BrokerCredentialRole, BrokerEndpoint, BrokerRequestEnvelope,
};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

const MAX_EVALUATION_DATASET_BYTES: u64 = 48 * 1024;

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
    let body = owner操作送信(&args[1], operation, payload)?;
    if operation == "端末招待" {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)] {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut output = options.open(&args[5]).map_err(|_| "招待fileを新規作成できない。既存fileは上書きしない")?;
        let data = serde_json::to_vec_pretty(&body).map_err(|_|"招待の保存形式不正")?;
        output.write_all(&data).map_err(|_|"招待保存失敗")?;
        output.sync_all().map_err(|_|"招待保存の確定失敗")?;
        println!("端末招待を指定fileへ保存した。秘密を含むため対面で渡し、結合後に削除する。");
        return Ok(());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&body).map_err(|_| "結果の表示に失敗")?
    );
    Ok(())
}

/// ownerだけがCLIから明示実行するC4承認。通常Flutterはこの資格fileを読まない。
pub fn 実行系ライフサイクル承認(args: &[String]) -> Result<(), String> {
    if args.len() != 5 || args[0] != "--session-file" || args[2] != "承認" {
        return Err("使用法: 実行系ライフサイクル承認 --session-file <owner資格file> 承認 <承認ID> <承認hash>".into());
    }
    let body = owner操作送信(
        &args[1],
        "実行系ライフサイクル承認",
        json!({"版": 1, "承認ID": args[3], "承認hash": args[4]}),
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&body).map_err(|_| "結果の表示に失敗")?
    );
    Ok(())
}

/// privateな評価Datasetをowner制御としてだけBrokerへ渡す。
///
/// 通常UIはこの経路、資格file、private Dataset本文を扱わない。CLI側でも本文や
/// pathを診断へ出さず、Brokerが返す公開manifestだけを表示する。
pub fn 評価データセット登録(args: &[String]) -> Result<(), String> {
    if args.len() != 4 || args[0] != "--session-file" || args[2] != "登録" {
        return Err("使用法: 評価Dataset登録 --session-file <owner資格file> 登録 <非公開評価データセットJSONファイル>".into());
    }
    let payload = 評価データセット読取(&args[3])?;
    let body = owner操作送信(&args[1], "評価Dataset登録", payload)?;
    let manifest = 評価データセット公開投影(&body)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&manifest).map_err(|_| "評価Dataset公開manifestの表示に失敗")?
    );
    Ok(())
}

/// ownerが明示したredacted定義だけをC6の独立Broker経路へ渡す。
/// Brokerの公開receipt以外（入力本文・条件・参照）は表示しない。
pub fn 回帰Case登録(args: &[String]) -> Result<(), String> {
    if args.len() != 4 || args[0] != "--session-file" || args[2] != "登録" {
        return Err("使用法: 回帰Case登録 --session-file <owner資格file> 登録 <回帰Case登録JSONファイル>".into());
    }
    let payload = 評価データセット読取(&args[3])?;
    let body = owner操作送信(&args[1], "回帰Case登録", payload)?;
    let receipt = 回帰Case公開投影(&body)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).map_err(|_| "回帰Case公開receiptの表示に失敗")?
    );
    Ok(())
}

fn 回帰Case公開投影(body: &Value) -> Result<Value, String> {
    let object = body
        .as_object()
        .ok_or_else(|| "回帰Case公開receiptの形式が不正".to_string())?;
    let forbidden = ["入力", "必要条件", "禁止条件", "必要参照", "期待経路", "本文"];
    if forbidden.iter().any(|key| object.contains_key(*key)) {
        return Err("回帰Case公開receiptにprivate本文が含まれている".into());
    }
    let required = [
        "版", "回帰CaseID", "定義hash", "非公開保管ID", "暗号文hash", "公開表示名",
        "要求ID", "要求hash", "実行系ID", "結果状態", "応答hash", "終了監査ID",
        "公開範囲", "必要条件数", "禁止条件数", "必要参照数", "作成時刻UnixMillis",
        "作成監査ID", "証拠種別",
    ];
    if object.len() != required.len() || required.iter().any(|key| !object.contains_key(*key)) {
        return Err("回帰Case公開receiptの形式が不正".into());
    }
    Ok(body.clone())
}

fn 評価データセット読取(path: &str) -> Result<Value, String> {
    let (file, bytes) = 評価データセット通常fileを開く(path)?;
    if bytes > MAX_EVALUATION_DATASET_BYTES {
        return Err("評価Dataset定義fileが48KiB上限を超過".into());
    }

    let mut raw = Vec::with_capacity(bytes as usize);
    let mut bounded = file.take(MAX_EVALUATION_DATASET_BYTES + 1);
    bounded
        .read_to_end(&mut raw)
        .map_err(|_| "評価Dataset定義fileを読めない")?;
    if raw.len() as u64 > MAX_EVALUATION_DATASET_BYTES {
        return Err("評価Dataset定義fileが48KiB上限を超過".into());
    }
    let payload: Value = serde_json::from_slice(&raw)
        .map_err(|_| "評価Dataset定義fileはJSON objectでなければならない")?;
    if !payload.is_object() {
        return Err("評価Dataset定義fileはJSON objectでなければならない".into());
    }
    Ok(payload)
}

/// private Datasetの最終要素を、検査対象と同じhandleとして一度だけ開く。
#[cfg(windows)]
fn 評価データセット通常fileを開く(path: &str) -> Result<(std::fs::File, u64), String> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};

    const 再解析点を開くフラグ: u32 = 0x0020_0000;
    const 再解析点属性: u32 = 0x0000_0400;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(再解析点を開くフラグ)
        .open(path)
        .map_err(|_| "評価Dataset定義fileを開けない")?;
    let metadata = file
        .metadata()
        .map_err(|_| "評価Dataset定義fileの種別を確認できない")?;
    if !metadata.file_type().is_file() || metadata.file_attributes() & 再解析点属性 != 0 {
        return Err("評価Dataset定義fileは通常fileでなければならない".into());
    }
    Ok((file, metadata.len()))
}

/// C5のprivate Dataset保管はWindowsのProtectedStoreに限定する。
#[cfg(not(windows))]
fn 評価データセット通常fileを開く(
    _path: &str,
) -> Result<(std::fs::File, u64), String> {
    Err("評価Dataset登録はWindowsの保護保管が利用できる環境に限定される".into())
}

/// Broker応答からC5の公開manifestだけを選び、private本文が誤って出力されるのを防ぐ。
fn 評価データセット公開投影(body: &Value) -> Result<Value, String> {
    let object = body
        .as_object()
        .ok_or_else(|| "評価Dataset公開manifestの形式が不正".to_string())?;
    let required = [
        "版",
        "評価DatasetID",
        "revision",
        "定義hash",
        "非公開保管ID",
        "暗号文hash",
        "公開表示名",
        "Case数",
        "Case一覧",
        "公開範囲",
        "作成時刻UnixMillis",
        "作成監査ID",
        "証拠種別",
    ];
    if object.len() != required.len() || required.iter().any(|key| !object.contains_key(*key)) {
        return Err("評価Dataset公開manifestの形式が不正".into());
    }
    let cases = object
        .get("Case一覧")
        .and_then(Value::as_array)
        .ok_or_else(|| "評価Dataset公開manifestの形式が不正".to_string())?
        .iter()
        .map(|case| {
            let case = case
                .as_object()
                .ok_or_else(|| "評価Dataset公開manifestの形式が不正".to_string())?;
            if case.len() != 2
                || !case.contains_key("評価CaseID")
                || !case.contains_key("定義hash")
            {
                return Err("評価Dataset公開manifestの形式が不正".to_string());
            }
            Ok(json!({
                "評価CaseID": case["評価CaseID"],
                "定義hash": case["定義hash"],
            }))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "版": object["版"],
        "評価DatasetID": object["評価DatasetID"],
        "revision": object["revision"],
        "定義hash": object["定義hash"],
        "非公開保管ID": object["非公開保管ID"],
        "暗号文hash": object["暗号文hash"],
        "公開表示名": object["公開表示名"],
        "Case数": object["Case数"],
        "Case一覧": cases,
        "公開範囲": object["公開範囲"],
        "作成時刻UnixMillis": object["作成時刻UnixMillis"],
        "作成監査ID": object["作成監査ID"],
        "証拠種別": object["証拠種別"],
    }))
}

fn owner操作送信(session_file: &str, operation: &str, payload: Value) -> Result<Value, String> {
    let file = std::fs::File::open(session_file).map_err(|_| "owner資格fileを開けない")?;
    let mut raw = Vec::new();
    file.take(8193)
        .read_to_end(&mut raw)
        .map_err(|_| "owner資格fileを読めない")?;
    if raw.len() > 8192 {
        return Err("owner資格fileが上限超過".into());
    }
    let endpoint: BrokerEndpoint =
        serde_json::from_slice(&raw).map_err(|_| "owner資格fileの形式が不正")?;
    if endpoint.credential_role != BrokerCredentialRole::Owner
        || endpoint.host != "127.0.0.1"
        || endpoint.port == 0
        || endpoint.transport != "authenticated_loopback_tcp"
        || endpoint.session_secret.len() != 64
        || !endpoint
            .session_secret
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
    {
        return Err("owner制御資格が必要: owner接続先が不正".into());
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
    let frame = format!("{}\n{}\n", endpoint.session_secret, request);
    stream
        .write_all(frame.as_bytes())
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
    Ok(response["body"].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn test_file(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "gui-shell-owner-cli-{name}-{}",
            識別子生成().expect("テスト識別子を生成できる")
        ))
    }

    fn write_test_file(path: &std::path::Path, bytes: impl AsRef<[u8]>) {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .expect("テスト用評価データセットを作成できる");
        file.write_all(bytes.as_ref())
            .expect("テスト用評価データセットへ書き込める");
        file.sync_all()
            .expect("テスト用評価データセットを同期できる");
    }

    #[cfg(windows)]
    #[test]
    fn 評価データセット読取は上限内のオブジェクトだけを受理する() {
        let valid = test_file("valid");
        write_test_file(&valid, r#"{"版":1,"private":"not-output"}"#);
        assert!(評価データセット読取(&valid.to_string_lossy())
            .expect("有効なJSONオブジェクトを読める")
            .is_object());
        std::fs::remove_file(&valid).expect("有効な評価データセットを削除できる");

        let scalar = test_file("scalar");
        write_test_file(&scalar, br#"["private"]"#);
        let error = 評価データセット読取(&scalar.to_string_lossy()).expect_err("配列形式は拒否される");
        assert!(!error.contains("private"));
        let scalar_path = scalar.to_string_lossy();
        assert!(!error.contains(scalar_path.as_ref()));
        std::fs::remove_file(&scalar).expect("配列形式の評価データセットを削除できる");

        let oversized = test_file("oversized");
        write_test_file(
            &oversized,
            vec![b'x'; MAX_EVALUATION_DATASET_BYTES as usize + 1],
        );
        let error = 評価データセット読取(&oversized.to_string_lossy())
            .expect_err("上限超過は拒否される");
        assert!(error.contains("48KiB上限"));
        let oversized_path = oversized.to_string_lossy();
        assert!(!error.contains(oversized_path.as_ref()));
        std::fs::remove_file(&oversized).expect("上限超過の評価データセットを削除できる");
    }

    #[cfg(windows)]
    #[test]
    fn 評価データセット読取は最終要素の再解析点を拒否する() {
        use std::os::windows::fs::symlink_file;

        const リンク作成特権未保持: i32 = 1314;
        let target = test_file("再解析点の対象");
        let link = test_file("再解析点");
        write_test_file(&target, r#"{"版":1,"private":"not-output"}"#);
        match symlink_file(&target, &link) {
            Ok(()) => {}
            // 1314はWindowsがリンク作成特権の未保持を返す値である。
            // それ以外の作成失敗は環境不整合として試験を失敗させる。
            Err(error) if error.raw_os_error() == Some(リンク作成特権未保持) => {
                std::fs::remove_file(&target).expect("再解析点の対象を削除できる");
                return;
            }
            Err(error) => {
                let _ = std::fs::remove_file(&target);
                panic!("テスト用の再解析点を作成できない: {error}");
            }
        }

        let error = 評価データセット読取(&link.to_string_lossy())
            .expect_err("最終要素の再解析点は拒否される");
        assert!(!error.contains("private"));
        let target_path = target.to_string_lossy();
        let link_path = link.to_string_lossy();
        assert!(!error.contains(target_path.as_ref()));
        assert!(!error.contains(link_path.as_ref()));
        std::fs::remove_file(&link).expect("再解析点を削除できる");
        std::fs::remove_file(&target).expect("再解析点の対象を削除できる");
    }

    #[cfg(not(windows))]
    #[test]
    fn 評価データセット読取は非Windows環境で失敗終了する() {
        let path = test_file("非Windows");
        write_test_file(&path, r#"{"版":1}"#);
        let error = 評価データセット読取(&path.to_string_lossy())
            .expect_err("非Windows環境では保護保管がないため拒否される");
        assert!(error.contains("Windowsの保護保管"));
        std::fs::remove_file(&path).expect("非Windows用の評価データセットを削除できる");
    }

    #[test]
    fn 評価公開manifest投影は非公開fieldを除外する() {
        let body = json!({
            "版": 1,
            "評価DatasetID": "a".repeat(32),
            "revision": 1,
            "定義hash": format!("sha256:{}", "b".repeat(64)),
            "非公開保管ID": "c".repeat(32),
            "暗号文hash": format!("sha256:{}", "d".repeat(64)),
            "公開表示名": "表示名",
            "Case数": 1,
            "Case一覧": [{"評価CaseID": "e".repeat(32), "定義hash": format!("sha256:{}", "f".repeat(64))}],
            "公開範囲": "hash_only",
            "作成時刻UnixMillis": 1,
            "作成監査ID": "audit-1",
            "証拠種別": "INTERNAL_STATE"
        });
        assert_eq!(
            評価データセット公開投影(&body).expect("公開マニフェストを取得できる"),
            body
        );
        let mut unsafe_body = body;
        unsafe_body["入力"] = json!("private-dataset-sentinel");
        assert!(評価データセット公開投影(&unsafe_body).is_err());
    }

    #[test]
    fn 回帰Case公開receiptはprivate本文を除外する() {
        let body = json!({
            "版": 1,
            "回帰CaseID": "a".repeat(32),
            "定義hash": format!("sha256:{}", "b".repeat(64)),
            "非公開保管ID": "a".repeat(32),
            "暗号文hash": format!("sha256:{}", "c".repeat(64)),
            "公開表示名": "表示名",
            "要求ID": "d".repeat(32),
            "要求hash": format!("sha256:{}", "e".repeat(64)),
            "実行系ID": "codex",
            "結果状態": "成功",
            "応答hash": format!("sha256:{}", "f".repeat(64)),
            "終了監査ID": "audit-end",
            "公開範囲": "hash_only",
            "必要条件数": 0,
            "禁止条件数": 0,
            "必要参照数": 0,
            "作成時刻UnixMillis": 1,
            "作成監査ID": "audit-create",
            "証拠種別": "INTERNAL_STATE"
        });
        assert_eq!(回帰Case公開投影(&body).expect("公開receiptを取得できる"), body);
        let mut unsafe_body = body;
        unsafe_body["入力"] = json!("private-regression-sentinel");
        assert!(回帰Case公開投影(&unsafe_body).is_err());
    }
}
