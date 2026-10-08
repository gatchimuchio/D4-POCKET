//! 開発専用。合成公開Agent CardをIPv4 loopbackへ一回だけ返す。
use std::io::{self, Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:34101")?;
    listener.set_nonblocking(true)?;
    println!("D4_A2A_FIXTURE_READY");
    io::stdout().flush()?;
    let deadline = Instant::now() + Duration::from_secs(600);
    while Instant::now() < deadline {
        match listener.accept() {
            Ok((mut stream, peer)) => {
                if !peer.ip().is_loopback() {
                    return Err(io::Error::other("fixture対象外"));
                }
                stream.set_read_timeout(Some(Duration::from_secs(2)))?;
                stream.set_write_timeout(Some(Duration::from_secs(2)))?;
                let mut bytes = Vec::new();
                let mut chunk = [0; 1024];
                while !bytes.ends_with(b"\r\n\r\n") && bytes.len() < 4096 {
                    let count = stream.read(&mut chunk)?;
                    if count == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&chunk[..count]);
                }
                if bytes.len() > 4096
                    || !bytes.starts_with(b"GET /card HTTP/1.1\r\n")
                    || !bytes.ends_with(b"\r\n\r\n")
                {
                    return Err(io::Error::other("fixture要求不正"));
                }
                let body = serde_json::to_vec(&serde_json::json!({
                    "name":"Mac A2A fixture","description":"Public test declaration only",
                    "version":"1.0.0","supportedInterfaces":[{"protocolBinding":"JSONRPC","protocolVersion":"1.0"}],
                    "capabilities":{"streaming":false,"pushNotifications":false,"stateTransitionHistory":false,"extendedAgentCard":false},
                    "securitySchemes":{},"skills":[]
                }))?;
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len())?;
                stream.write_all(&body)?;
                stream.flush()?;
                println!("D4_A2A_FIXTURE_SERVED_ONCE");
                return Ok(());
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20))
            }
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::other("fixture期限超過"))
}
