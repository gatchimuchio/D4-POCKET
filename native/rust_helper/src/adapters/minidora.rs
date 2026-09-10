//! MINIDORA の公開HTTP境界だけを汎用対話へ射影する。
#![allow(non_snake_case)]
use crate::broker::dialogue::{実行系Adapter, 実行結果, 対話失敗, 対話要求};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub struct MinidoraAdapter {
    接続先: SocketAddr,
}
impl MinidoraAdapter {
    pub fn new(接続先: &str) -> Result<Self, 対話失敗> {
        let addr: SocketAddr = 接続先.parse().map_err(|_| 対話失敗::要求不正)?;
        if addr.ip() != std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST) || addr.port() == 0 {
            return Err(対話失敗::要求不正);
        }
        Ok(Self { 接続先: addr })
    }
    fn 取得(
        &self,
        経路: &str,
        本文: Option<Vec<u8>>,
        取消: &AtomicBool,
        期限: Instant,
    ) -> Result<Vec<u8>, 対話失敗> {
        時間確認(取消, 期限)?;
        let mut stream = TcpStream::connect_timeout(
            &self.接続先,
            Duration::from_secs(1).min(期限.saturating_duration_since(Instant::now())),
        )
        .map_err(|_| 対話失敗::通信失敗)?;
        stream
            .set_read_timeout(Some(Duration::from_millis(100)))
            .map_err(|_| 対話失敗::通信失敗)?;
        stream
            .set_write_timeout(Some(Duration::from_millis(100)))
            .map_err(|_| 対話失敗::通信失敗)?;
        let method = if 本文.is_some() { "POST" } else { "GET" };
        let bytes = 本文.unwrap_or_default();
        let mut request = format!("{method} {経路} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", self.接続先, bytes.len()).into_bytes();
        request.extend(bytes);
        let mut sent = 0;
        while sent < request.len() {
            時間確認(取消, 期限)?;
            match stream.write(&request[sent..]) {
                Ok(0) => return Err(対話失敗::通信失敗),
                Ok(n) => sent += n,
                Err(e) if 待機可能(&e) => continue,
                Err(_) => return Err(対話失敗::通信失敗),
            }
        }
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 8192];
        let (開始, 長さ) = loop {
            時間確認(取消, 期限)?;
            match stream.read(&mut chunk) {
                Ok(0) => return Err(対話失敗::応答不正),
                Ok(n) => buffer.extend_from_slice(&chunk[..n]),
                Err(e) if 待機可能(&e) => continue,
                Err(_) => return Err(対話失敗::通信失敗),
            }
            if let Some(pos) = buffer.windows(4).position(|v| v == b"\r\n\r\n") {
                if pos > 16384 {
                    return Err(対話失敗::応答不正);
                }
                let header =
                    std::str::from_utf8(&buffer[..pos]).map_err(|_| 対話失敗::応答不正)?;
                let mut lines = header.split("\r\n");
                let mut status = lines.next().ok_or(対話失敗::応答不正)?.split_whitespace();
                if !matches!(status.next(), Some("HTTP/1.0" | "HTTP/1.1"))
                    || status.next() != Some("200")
                {
                    return Err(対話失敗::通信失敗);
                }
                let mut length = None;
                let mut content_type = None;
                for line in lines {
                    let (name, value) = line.split_once(':').ok_or(対話失敗::応答不正)?;
                    match name.to_ascii_lowercase().as_str() {
                        "transfer-encoding" | "content-encoding" => return Err(対話失敗::応答不正),
                        "content-length" => {
                            if length.is_some() {
                                return Err(対話失敗::応答不正);
                            }
                            length = Some(
                                value
                                    .trim()
                                    .parse::<usize>()
                                    .map_err(|_| 対話失敗::応答不正)?,
                            );
                        }
                        "content-type" => {
                            if content_type.is_some() {
                                return Err(対話失敗::応答不正);
                            }
                            content_type = Some(value.trim().to_ascii_lowercase());
                        }
                        _ => (),
                    }
                }
                if !content_type
                    .is_some_and(|v| v.split(';').next().map(str::trim) == Some("application/json"))
                {
                    return Err(対話失敗::応答不正);
                }
                let length = length.ok_or(対話失敗::応答不正)?;
                if length == 0 || length > 1024 * 1024 {
                    return Err(対話失敗::応答不正);
                }
                break (pos + 4, length);
            }
            if buffer.len() > 16384 {
                return Err(対話失敗::応答不正);
            }
        };
        while buffer.len() < 開始 + 長さ {
            時間確認(取消, 期限)?;
            let limit = chunk.len().min(開始 + 長さ - buffer.len());
            match stream.read(&mut chunk[..limit]) {
                Ok(0) => return Err(対話失敗::応答不正),
                Ok(n) => buffer.extend_from_slice(&chunk[..n]),
                Err(e) if 待機可能(&e) => continue,
                Err(_) => return Err(対話失敗::通信失敗),
            }
        }
        if buffer.len() != 開始 + 長さ {
            return Err(対話失敗::応答不正);
        }
        時間確認(取消, 期限)?;
        Ok(buffer[開始..].to_vec())
    }
}
fn 待機可能(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::Interrupted
    )
}
fn 時間確認(取消: &AtomicBool, 期限: Instant) -> Result<(), 対話失敗> {
    if 取消.load(Ordering::SeqCst) {
        Err(対話失敗::取消)
    } else if Instant::now() >= 期限 {
        Err(対話失敗::期限超過)
    } else {
        Ok(())
    }
}
fn json読取(bytes: &[u8]) -> Result<Value, 対話失敗> {
    serde_json::from_slice(bytes).map_err(|_| 対話失敗::応答不正)
}
fn 文字列(v: &Value, key: &str, 上限: usize) -> Result<String, 対話失敗> {
    let s = v
        .get(key)
        .and_then(Value::as_str)
        .ok_or(対話失敗::応答不正)?;
    if s.chars().count() > 上限 {
        return Err(対話失敗::応答不正);
    }
    Ok(s.into())
}
fn hex固定(s: &str, 長さ: usize) -> bool {
    s.len() == 長さ
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn 文字列列(v: &Value) -> Result<Vec<String>, 対話失敗> {
    let a = v.as_array().ok_or(対話失敗::応答不正)?;
    if a.len() > 64 {
        return Err(対話失敗::応答不正);
    }
    a.iter()
        .map(|x| {
            x.as_str()
                .filter(|s| s.chars().count() <= 256)
                .map(str::to_owned)
                .ok_or(対話失敗::応答不正)
        })
        .collect()
}
impl 実行系Adapter for MinidoraAdapter {
    fn 接続対象(&self) -> String {
        format!("http://{}", self.接続先)
    }
    fn 応答(
        &self,
        要求: &対話要求,
        取消: &AtomicBool,
        期限: Instant,
        生受信: &mut Vec<Vec<u8>>,
    ) -> Result<実行結果, 対話失敗> {
        let mut 取得 = |経路: &str, 本文: Option<Vec<u8>>| {
            let bytes = self.取得(経路, 本文, 取消, 期限)?;
            生受信.push(bytes.clone());
            Ok::<_, 対話失敗>(bytes)
        };
        let health = json読取(&取得("/health", None)?)?;
        if health["ok"] != true || health["api_version"] != "MINIDORA-PRODUCT-API-v1" {
            return Err(対話失敗::応答不正);
        }
        let caps = json読取(&取得("/api/capabilities", None)?)?;
        let _宣言能力 = 文字列列(&caps["capabilities"])?;
        let raw = 取得(
            "/api/chat",
            Some(
                json!({"session_id": 要求.対話セッションID, "message": 要求.入力})
                    .to_string()
                    .into_bytes(),
            ),
        )?;
        let v = json読取(&raw)?;
        if v["session_id"] != 要求.対話セッションID {
            return Err(対話失敗::セッション不一致);
        }
        let 保留 = match v["status"].as_str() {
            Some("合格") => false,
            Some("保留") => true,
            _ => return Err(対話失敗::応答不正),
        };
        let 本文 = 文字列(&v, "response", 65536)?;
        let 経路 = 文字列(&v, "route", 256)?;
        let 能力 = 文字列列(&v["capabilities"])?;
        let 追跡ID = 文字列(&v, "trace_id", 32)?;
        let hash = 文字列(&v, "trace_hash", 64)?;
        if !hex固定(&追跡ID, 32) || !hex固定(&hash, 64) {
            return Err(対話失敗::応答不正);
        }
        let sources = v["sources"]
            .as_array()
            .filter(|a| a.len() <= 64)
            .ok_or(対話失敗::応答不正)?;
        let mut 参照 = Vec::new();
        for s in sources {
            let 題名 = 文字列(s, "題名", 512)?;
            let 出典 = 文字列(s, "出典", 512)?;
            let url = 文字列(s, "URL", 512)?;
            参照.push(format!("{題名} / {出典} / {url}"));
        }
        let trace = json読取(&取得(&format!("/api/trace/{追跡ID}"), None)?)?;
        if trace["valid"] != true
            || trace["trace"]["追跡ID"] != 追跡ID
            || trace["trace"]["セッションID"] != 要求.対話セッションID
            || trace["trace"]["ルートハッシュ"] != hash
        {
            return Err(対話失敗::応答不正);
        }
        Ok(実行結果 {
            対話セッションID: 要求.対話セッションID.clone(),
            本文,
            参照,
            能力,
            経路,
            追跡ID,
            追跡hash: format!("sha256:{hash}"),
            保留,
            生応答: raw,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn 一回応答(raw: Vec<u8>) -> (MinidoraAdapter, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adapter = MinidoraAdapter::new(&listener.local_addr().unwrap().to_string()).unwrap();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut buffer = [0; 8192];
            let _ = stream.read(&mut buffer);
            let _ = stream.write_all(&raw);
        });
        (adapter, worker)
    }
    #[test]
    fn HTTP境界でredirectや重複長さや過大応答を拒否する() {
        for raw in [
            "HTTP/1.1 302 Found\r\nLocation: http://example.invalid/\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{}",
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{}",
            "HTTP/1.1 200 OK\r\nContent-Length: 2000000\r\nContent-Type: application/json\r\n\r\n{}",
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: application/json\r\n\r\n{}",
            "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nContent-Type: application/json\r\n\r\n{}",
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Type: text/html\r\n\r\n{}",
        ] {
            let (a,w)=一回応答(raw.as_bytes().to_vec());
            assert!(a.取得("/health",None,&AtomicBool::new(false),Instant::now()+Duration::from_secs(2)).is_err()); w.join().unwrap();
        }
    }
    #[test]
    fn ContentLength付きJSONだけを期限内に取得する() {
        let (a,w)=一回応答(b"HTTP/1.0 200 OK\r\nContent-Length: 2\r\nContent-Type: application/json; charset=utf-8\r\n\r\n{}".to_vec());
        assert_eq!(
            a.取得(
                "/health",
                None,
                &AtomicBool::new(false),
                Instant::now() + Duration::from_secs(2)
            )
            .unwrap(),
            b"{}"
        );
        w.join().unwrap();
        assert_eq!(
            a.取得(
                "/health",
                None,
                &AtomicBool::new(true),
                Instant::now() + Duration::from_secs(2)
            ),
            Err(対話失敗::取消)
        );
        assert_eq!(
            a.取得("/health", None, &AtomicBool::new(false), Instant::now()),
            Err(対話失敗::期限超過)
        );
    }
    #[test]
    fn 接続先はIPv4loopbackの明示portに限定する() {
        for addr in [
            "localhost:8080",
            "0.0.0.0:8080",
            "127.0.0.1:0",
            "http://127.0.0.1:8080",
            "192.168.1.2:8080",
            "[::1]:8080",
        ] {
            assert!(MinidoraAdapter::new(addr).is_err());
        }
    }
}
