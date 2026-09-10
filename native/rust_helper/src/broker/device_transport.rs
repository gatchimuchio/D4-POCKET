//! 明示起動された端末TLS経路。資格の判断はBrokerが所有する。
use super::Broker;
use rustls::pki_types::PrivatePkcs8KeyDer;
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) struct DeviceListener {
    listener: TcpListener,
    tls: Arc<ServerConfig>,
}
impl DeviceListener {
    pub fn bind(address: &str, broker: &mut Broker) -> Result<Self, &'static str> {
        let addr: SocketAddr = address.parse().map_err(|_| "端末bind先が不正")?;
        if !addr.is_ipv4() || !super::device_link::接続先検査(&addr.ip().to_string()) {
            return Err("端末bindはprivate IPv4またはloopbackに限定する");
        }
        let listener = TcpListener::bind(addr).map_err(|_| "端末portをbindできない")?;
        listener
            .set_nonblocking(true)
            .map_err(|_| "端末listenerを設定できない")?;
        let certified = rcgen::generate_simple_self_signed(vec![
            "gui-shell.local".into(),
            addr.ip().to_string(),
        ])
        .map_err(|_| "端末証明書を生成できない")?;
        let cert = certified.cert.der().clone();
        let hash = crate::audit_hash::sha256_tagged(cert.as_ref())
            .trim_start_matches("sha256:")
            .to_owned();
        let mut tls = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![cert],
                PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der()).into(),
            )
            .map_err(|_| "端末TLSを設定できない")?;
        tls.max_early_data_size = 0;
        tls.send_tls13_tickets = 0;
        tls.session_storage = Arc::new(rustls::server::NoServerSessionStorage {});
        broker.端末経路設定(
            hash,
            listener.local_addr().map_err(|_| "端末portが不明")?.port(),
        )?;
        Ok(Self {
            listener,
            tls: Arc::new(tls),
        })
    }

    /// 一度に一接続、有限期限・有限frame。秘密や本文を診断出力しない。
    pub fn poll(&self, broker: &mut Broker) {
        if let Ok((stream, _)) = self.listener.accept() {
            if self.handle(stream, broker).is_err() {
                broker.reject_ipc("端末通信拒否", "端末TLSまたはframeを拒否", true);
            }
        }
    }
    fn handle(&self, stream: TcpStream, broker: &mut Broker) -> Result<(), &'static str> {
        stream
            .set_nonblocking(false)
            .map_err(|_| "端末socket設定失敗")?;
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .map_err(|_| "端末受信期限設定失敗")?;
        stream
            .set_write_timeout(Some(Duration::from_secs(1)))
            .map_err(|_| "端末送信期限設定失敗")?;
        let connection =
            ServerConnection::new(Arc::clone(&self.tls)).map_err(|_| "端末TLS初期化失敗")?;
        let mut tls = StreamOwned::new(connection, stream);
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut bytes = Vec::new();
        loop {
            if Instant::now() >= deadline {
                return Err("端末受信期限超過");
            }
            let mut chunk = [0u8; 4096];
            let n = tls.read(&mut chunk).map_err(|_| "端末TLS受信失敗")?;
            if n == 0 {
                return Err("端末frame未完了");
            }
            if bytes.len() + n > 65536 {
                return Err("端末frame上限");
            }
            bytes.extend_from_slice(&chunk[..n]);
            if let Some(end) = bytes.iter().position(|b| *b == b'\n') {
                if end + 1 != bytes.len() {
                    return Err("一接続一要求に限定する");
                }
                bytes.pop();
                break;
            }
        }
        let raw = std::str::from_utf8(&bytes).map_err(|_| "端末frameがUTF-8ではない")?;
        let response = broker
            .端末要求処理(raw)
            .to_json_string()
            .map_err(|_| "端末応答生成失敗")?;
        if response.len() > 4 * 1024 * 1024 {
            return Err("端末応答上限");
        }
        tls.write_all(response.as_bytes())
            .map_err(|_| "端末応答送信失敗")?;
        tls.write_all(b"\n").map_err(|_| "端末応答終端失敗")?;
        tls.flush().map_err(|_| "端末応答送信失敗")?;
        tls.conn.send_close_notify();
        let _ = tls.flush();
        Ok(())
    }
}
