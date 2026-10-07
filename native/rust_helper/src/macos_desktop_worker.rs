//! 同梱macOS helper。匿名pipeはtransportだけで、権限判断は既存認証Brokerへ残す。
use crate::broker::ipc_server::run_loopback_server_cancellable;
use crate::broker::{BrokerCredentialRole, BrokerEndpoint, BrokerServerConfig};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::thread;
use std::time::Duration;
use zeroize::{Zeroize, Zeroizing};

const REQUEST_LIMIT: usize = 64 * 1024;
const RESPONSE_LIMIT: usize = 4 * 1024 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(4);

fn failure() -> io::Error {
    io::Error::other("macOS Broker接続を確認できません")
}

#[cfg(target_os = "macos")]
pub fn run() -> io::Result<()> {
    if std::env::args_os().len() != 1 {
        return Err(failure());
    }
    let home = PathBuf::from(std::env::var_os("HOME").ok_or_else(failure)?);
    if !home.is_absolute() || !home.is_dir() {
        return Err(failure());
    }
    let home = home.canonicalize()?;
    let root = private_directory(
        &home,
        &["Library", "Application Support", "D4Pocket", "macos-broker"],
    )?;
    serve(&root, &mut io::stdin().lock(), &mut io::stdout().lock())
}

#[cfg(target_os = "macos")]
fn private_directory(base: &Path, names: &[&str]) -> io::Result<PathBuf> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let mut path = base.to_path_buf();
    for (index, name) in names.iter().enumerate() {
        path.push(name);
        match fs::DirBuilder::new().mode(0o700).create(&path) {
            Ok(()) => (),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
            Err(error) => return Err(error),
        }
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || (index >= 2 && metadata.permissions().mode() & 0o077 != 0)
        {
            return Err(failure());
        }
    }
    Ok(path)
}

struct RunLock {
    path: PathBuf,
    file: File,
}
impl RunLock {
    fn acquire(root: &Path) -> io::Result<Self> {
        let path = root.join("running.lock");
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        Ok(Self { path, file })
    }
}
impl Drop for RunLock {
    fn drop(&mut self) {
        let _ = self.file.sync_all();
        let _ = fs::remove_file(&self.path);
    }
}

struct NormalEndpoint(BrokerEndpoint);
impl Drop for NormalEndpoint {
    fn drop(&mut self) {
        self.0.session_secret.zeroize();
    }
}

fn serve(root: &Path, input: &mut impl BufRead, output: &mut impl Write) -> io::Result<()> {
    let _lock = RunLock::acquire(root)?;
    let session = root.join("normal-session.json");
    if session.try_exists()? || session.with_extension("json.tmp").try_exists()? {
        return Err(failure());
    }
    let shutdown = Arc::new(AtomicBool::new(false));
    let stop = Arc::clone(&shutdown);
    let config = BrokerServerConfig::new(root.join("store"), session.clone());
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let server = thread::Builder::new()
        .name("macos-security-broker".into())
        .spawn(move || run_loopback_server_cancellable(config, stop, ready_tx))?;
    let mut owned_session = Zeroizing::new(Vec::new());
    let result = (|| {
        ready_rx
            .recv_timeout(Duration::from_secs(15))
            .map_err(|_| failure())?;
        let metadata = fs::symlink_metadata(&session)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 4096 {
            return Err(failure());
        }
        *owned_session = fs::read(&session)?;
        let endpoint: BrokerEndpoint =
            serde_json::from_slice(&owned_session).map_err(|_| failure())?;
        let endpoint = NormalEndpoint(endpoint);
        if endpoint.0.host != "127.0.0.1"
            || endpoint.0.port == 0
            || endpoint.0.credential_role != BrokerCredentialRole::Normal
            || endpoint.0.transport != "authenticated_loopback_tcp"
            || endpoint.0.max_request_bytes != REQUEST_LIMIT
        {
            return Err(failure());
        }
        loop {
            let Some(frame) = read_frame(input, REQUEST_LIMIT)? else {
                break;
            };
            let request = normalize(&frame, &endpoint.0.session_id);
            let response = relay(&request, &endpoint.0)?;
            output.write_all(&response)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
        Ok(())
    })();
    shutdown.store(true, Ordering::Release);
    let stopped = server.join().map_err(|_| failure())?.map_err(|_| failure());
    let cleanup = if !owned_session.is_empty() {
        let current = Zeroizing::new(fs::read(&session)?);
        if *current == *owned_session {
            fs::remove_file(&session)
        } else {
            Err(failure())
        }
    } else {
        Ok(())
    };
    result.and(stopped).and(cleanup)
}

fn read_frame(input: &mut impl BufRead, limit: usize) -> io::Result<Option<Vec<u8>>> {
    let mut frame = Vec::new();
    let size = input
        .take((limit + 1) as u64)
        .read_until(b'\n', &mut frame)?;
    if size == 0 {
        return Ok(None);
    }
    if size > limit || frame.pop() != Some(b'\n') || frame.contains(&b'\r') {
        return Err(failure());
    }
    Ok(Some(frame))
}

fn normalize(input: &[u8], session_id: &str) -> Vec<u8> {
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(input) else {
        return input.to_vec();
    };
    let Some(object) = value.as_object_mut() else {
        return input.to_vec();
    };
    if object.contains_key("session_id") {
        object.insert("desktop_channel_session_id_forbidden".into(), true.into());
    } else {
        object.insert("session_id".into(), session_id.into());
    }
    serde_json::to_vec(&value).unwrap_or_else(|_| input.to_vec())
}

fn relay(request: &[u8], endpoint: &BrokerEndpoint) -> io::Result<Vec<u8>> {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, endpoint.port));
    let mut socket = TcpStream::connect_timeout(&address, IO_TIMEOUT)?;
    socket.set_read_timeout(Some(IO_TIMEOUT))?;
    socket.set_write_timeout(Some(IO_TIMEOUT))?;
    socket.write_all(endpoint.session_secret.as_bytes())?;
    socket.write_all(b"\n")?;
    socket.write_all(request)?;
    socket.write_all(b"\n")?;
    socket.flush()?;
    read_frame(&mut BufReader::new(socket), RESPONSE_LIMIT)?.ok_or_else(failure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::protocol::BrokerRequestEnvelope;
    use serde_json::{json, Value};

    #[test]
    fn normal_broker_roundtrip_rejects_injection_replay_and_owner_and_closes() {
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).unwrap();
        let root = std::env::temp_dir().join(format!("d4p-macos-worker-{}", hex::encode(random)));
        fs::create_dir(&root).unwrap();
        let request = |id: &str, operation: &str| {
            json!({
                "request_id": id, "nonce": id, "operation": operation,
                "issued_at": BrokerRequestEnvelope::current_issued_at(),
                "payload_hash": crate::broker::protocol::canonical_payload_hash(None),
                "metadata": {"client": "desktop_flutter"}
            })
        };
        let health = request("macos-health", "health");
        let mut injected = request("macos-injected", "health");
        injected["session_id"] = "owner".into();
        let frames = [
            health.clone(),
            health,
            injected,
            request("macos-owner", "作業領域承認"),
        ];
        let bytes = frames
            .iter()
            .map(|frame| format!("{frame}\n"))
            .collect::<String>();
        let mut output = Vec::new();
        serve(&root, &mut io::Cursor::new(bytes), &mut output).unwrap();
        let replies: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(replies.len(), 4);
        assert_eq!(replies[0]["status"], "accepted");
        for reply in &replies[1..] {
            assert_ne!(reply["status"], "accepted");
        }
        assert_eq!(replies[1]["error"]["code"], "broker_replay_detected");
        assert_eq!(
            replies[3]["error"]["code"],
            "desktop_native_owner_confirmation_required"
        );
        assert!(!root.join("normal-session.json").exists());
        assert!(!root.join("running.lock").exists());
        let audit = fs::read_to_string(root.join("store/audit.jsonl")).unwrap();
        assert!(audit.contains("D4 Pocket Desktop起動"));
        assert!(audit.contains("D4 Pocket Desktop終了"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn frame_bounds_and_second_instance_fail_closed() {
        for data in [b"unterminated".to_vec(), vec![b'a'; REQUEST_LIMIT + 1]] {
            assert!(read_frame(&mut io::Cursor::new(data), REQUEST_LIMIT).is_err());
        }
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).unwrap();
        let root = std::env::temp_dir().join(format!("d4p-macos-lock-{}", hex::encode(random)));
        fs::create_dir(&root).unwrap();
        let lock = RunLock::acquire(&root).unwrap();
        assert!(RunLock::acquire(&root).is_err());
        drop(lock);
        fs::remove_dir(root).unwrap();
    }
}
