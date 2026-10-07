//! 同梱macOS helper。匿名pipeはtransportだけで、権限判断は既存認証Brokerへ残す。
use crate::broker::ipc_server::run_loopback_server_cancellable_with_owner_operations;
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
    let bundle = verify_bundle(&std::env::current_exe()?)?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or_else(failure)?);
    if !home.is_absolute() || !home.is_dir() {
        return Err(failure());
    }
    let home = home.canonicalize()?;
    let root = private_directory(
        &home,
        &["Library", "Application Support", "D4Pocket", "macos-broker"],
    )?;
    serve(
        &root,
        &bundle,
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
        &mut |summary| gui_shell_macos_owner::confirm(summary, Duration::from_secs(300)),
    )
}

/// 固定bundle配置の検査結果。署名・配布identity・Owner承認を表さない。
struct VerifiedBundle;

fn verify_bundle(helper: &Path) -> io::Result<VerifiedBundle> {
    if !helper.is_absolute()
        || helper.file_name().and_then(|v| v.to_str()) != Some("gui_shell_macos_broker")
    {
        return Err(failure());
    }
    let macos = helper.parent().ok_or_else(failure)?;
    let contents = macos.parent().ok_or_else(failure)?;
    let bundle = contents.parent().ok_or_else(failure)?;
    if macos.file_name().and_then(|v| v.to_str()) != Some("MacOS")
        || contents.file_name().and_then(|v| v.to_str()) != Some("Contents")
        || bundle.extension().and_then(|v| v.to_str()) != Some("app")
    {
        return Err(failure());
    }
    let canonical_bundle = bundle.canonicalize()?;
    for name in [
        "Info.plist",
        "MacOS/gui_shell_desktop",
        "MacOS/gui_shell_macos_broker",
    ] {
        let path = contents.join(name);
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || !path.canonicalize()?.starts_with(&canonical_bundle)
        {
            return Err(failure());
        }
    }
    Ok(VerifiedBundle)
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

fn serve(
    root: &Path,
    _bundle: &VerifiedBundle,
    input: &mut impl BufRead,
    output: &mut impl Write,
    confirm: &mut impl FnMut(&str) -> bool,
) -> io::Result<()> {
    let _lock = RunLock::acquire(root)?;
    let session = root.join("normal-session.json");
    if session.try_exists()? || session.with_extension("json.tmp").try_exists()? {
        return Err(failure());
    }
    let shutdown = Arc::new(AtomicBool::new(false));
    let stop = Arc::clone(&shutdown);
    let mut config = BrokerServerConfig::new(root.join("store"), session.clone());
    config.desktop_package_layout_verified = true;
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (owner_tx, owner_rx) = mpsc::sync_channel(1);
    let server = thread::Builder::new()
        .name("macos-security-broker".into())
        .spawn(move || {
            run_loopback_server_cancellable_with_owner_operations(
                config, stop, ready_tx, owner_rx, None,
            )
        })?;
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
            let response = match super::macos_desktop_owner::dispatch(
                &frame,
                &endpoint.0,
                &owner_tx,
                confirm,
            )? {
                Some(response) => response,
                None => relay(&normalize(&frame, &endpoint.0.session_id), &endpoint.0)?,
            };
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

pub(super) fn normalize(input: &[u8], session_id: &str) -> Vec<u8> {
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
#[path = "../tests/unit/codex_cli_fixture.rs"]
#[allow(dead_code)]
mod registration_fixture;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::protocol::BrokerRequestEnvelope;
    use serde_json::{json, Value};

    #[test]
    fn owner_agent_registration_登録だけを承認しtask権限を生成しない() {
        let temp = registration_fixture::FixtureTempDirectory::create();
        let root = temp.path().to_path_buf();
        #[cfg(unix)]
        let root = root.canonicalize().unwrap();
        let broker = root.join("broker");
        let workspace = root.join("workspace");
        fs::create_dir(&broker).unwrap();
        fs::create_dir(&workspace).unwrap();
        let cli = registration_fixture::compile_fake_codex_cli(&root);
        let payload = json!({
            "version": 1, "adapter_id": "codex-cli", "runtime_id": "macos-agent",
            "workspace_id": "macos-workspace", "cli_path": cli, "workspace_root": workspace,
            "secret_paths": ["private-canary.env"],
            "provider_model_selection": {"version": 1, "provider_id": "openai_codex_cli",
                "model_id": "test-model", "authentication_source": "codex_cli_managed",
                "automatic_fallback": false}
        });
        let request = |id: &str, operation: &str, payload: Value| {
            json!({
                "request_id": id, "nonce": id, "issued_at": BrokerRequestEnvelope::current_issued_at(),
                "operation": operation, "metadata": {"client":"desktop_flutter"},
                "payload_hash": crate::broker::protocol::canonical_payload_hash(Some(&payload)), "payload":payload,
            })
        };
        let registration = |id: &str, payload| request(id, "AgentCLI実行系作業領域登録", payload);
        let mut injected = payload.clone();
        injected["approval"] = true.into();
        let mut unknown = payload.clone();
        unknown["adapter_id"] = "unknown-cli".into();
        let mut missing = payload.clone();
        missing["runtime_id"] = "missing-agent".into();
        missing["workspace_id"] = "missing-workspace".into();
        missing["cli_path"] = root.join("missing-cli").to_str().unwrap().into();
        let mut protected = missing.clone();
        protected["workspace_root"] = broker.to_str().unwrap().into();
        let mut malformed = payload.clone();
        malformed["secret_paths"] = json!(["../outside"]);
        let frames = [
            registration("denied", payload.clone()),
            request("empty", "作業領域一覧", json!({})),
            registration("accepted", payload.clone()),
            request("registered", "作業領域一覧", json!({})),
            registration("duplicate", payload),
            registration("injection", injected),
            registration("unknown", unknown),
            registration("missing", missing),
            registration("protected", protected),
            registration("malformed", malformed),
        ];
        let input = frames.iter().map(|v| format!("{v}\n")).collect::<String>();
        let mut output = Vec::new();
        let mut confirmations = 0;
        serve(
            &broker,
            &VerifiedBundle,
            &mut io::Cursor::new(input),
            &mut output,
            &mut |summary| {
                confirmations += 1;
                assert!(summary.contains("Task実行能力はunsupported"));
                assert!(summary.contains("要求hash: sha256:"));
                assert!(summary.contains("private-canary.env"));
                confirmations != 1
            },
        )
        .unwrap();
        let replies: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(replies.len(), frames.len());
        assert_eq!(
            replies[0]["error"]["code"],
            "desktop_native_owner_confirmation_required"
        );
        assert_eq!(replies[1]["body"]["作業領域"], json!([]));
        assert_eq!(replies[2]["status"], "accepted", "{}", replies[2]);
        assert_eq!(replies[2]["body"]["task_execution"], "unsupported");
        assert_eq!(replies[2]["body"]["permission_generated"], false);
        assert_eq!(replies[2]["body"]["approval_generated"], false);
        assert_eq!(replies[3]["body"]["作業領域"].as_array().unwrap().len(), 1);
        for reply in &replies[4..] {
            assert_ne!(reply["status"], "accepted", "{reply}");
        }
        assert_eq!(confirmations, 5);
        assert_eq!(fs::read_dir(&workspace).unwrap().count(), 0);
        let audit = fs::read_to_string(broker.join("store/audit.jsonl")).unwrap();
        assert!(audit.contains("AgentCLI実行系作業領域登録"));
        assert!(!audit.contains("private-canary.env"));
        assert!(!audit.contains(workspace.to_str().unwrap()));
    }

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
        let mut first_run = request("macos-first-run", "初回設定取得");
        let first_run_payload = json!({"version": 1});
        first_run["payload_hash"] =
            crate::broker::protocol::canonical_payload_hash(Some(&first_run_payload)).into();
        first_run["payload"] = first_run_payload;
        let frames = [
            health.clone(),
            health,
            injected,
            request("macos-owner", "作業領域承認"),
            first_run,
        ];
        let bytes = frames
            .iter()
            .map(|frame| format!("{frame}\n"))
            .collect::<String>();
        let mut output = Vec::new();
        serve(
            &root,
            &VerifiedBundle,
            &mut io::Cursor::new(bytes),
            &mut output,
            &mut |_| false,
        )
        .unwrap();
        let replies: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(replies.len(), 5);
        assert_eq!(replies[0]["status"], "accepted");
        for reply in &replies[1..4] {
            assert_ne!(reply["status"], "accepted");
        }
        assert_eq!(replies[1]["error"]["code"], "broker_replay_detected");
        assert_eq!(
            replies[3]["error"]["code"],
            "desktop_native_owner_confirmation_required"
        );
        assert_eq!(replies[4]["status"], "accepted");
        assert!(root.join("store/first_run_configuration.json").is_file());
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

    #[test]
    fn bundle_layout_requires_the_fixed_app_and_helper() {
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).unwrap();
        let root = std::env::temp_dir().join(format!("d4p-macos-bundle-{}", hex::encode(random)));
        let contents = root.join("D4Pocket.app/Contents");
        fs::create_dir_all(contents.join("MacOS")).unwrap();
        let helper = contents.join("MacOS/gui_shell_macos_broker");
        fs::write(&helper, b"synthetic helper").unwrap();
        fs::write(contents.join("Info.plist"), b"synthetic plist").unwrap();
        assert!(verify_bundle(&helper).is_err());
        fs::write(contents.join("MacOS/gui_shell_desktop"), b"synthetic app").unwrap();
        assert!(verify_bundle(&helper).is_ok());
        assert!(verify_bundle(&contents.join("MacOS/other")).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn owner_adapter_lifecycle_拒否と現在条件を保持して状態管理する() {
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).unwrap();
        let root = std::env::temp_dir().join(format!("d4p-macos-adapter-{}", hex::encode(random)));
        fs::create_dir(&root).unwrap();
        let install: Value = serde_json::from_str(include_str!(
            "../../../examples/contracts/adapter_management_request.valid.json"
        ))
        .unwrap();
        let adapter_hash =
            crate::broker::protocol::canonical_payload_hash(Some(&install["Manifest"]));
        let payload = |operation: &str| {
            json!({
                "版": 1, "操作": operation, "Adapter ID": install["Manifest"]["Adapter ID"],
                "Adapter hash": adapter_hash,
            })
        };
        let request = |id: &str, operation: &str, payload: Value| {
            json!({
                "request_id": id, "nonce": id, "operation": operation,
                "issued_at": BrokerRequestEnvelope::current_issued_at(),
                "payload_hash": crate::broker::protocol::canonical_payload_hash(Some(&payload)),
                "metadata": {"client": "desktop_flutter"}, "payload": payload,
            })
        };
        let mut stale = payload("隔離");
        stale["Adapter hash"] = format!("sha256:{}", "0".repeat(64)).into();
        let mut injected = payload("隔離");
        injected["approval"] = true.into();
        let frames = [
            request("setup", "アダプター導入", install.clone()),
            request("denied", "アダプター無効化", payload("無効化")),
            request("unchanged", "アダプター一覧", json!({"版":1})),
            request("verify", "アダプター検証", payload("検証")),
            request("enable", "アダプター有効化", payload("有効化")),
            request("disable", "アダプター無効化", payload("無効化")),
            request("stale", "アダプター隔離", stale),
            request("injected", "アダプター隔離", injected),
            request("mismatch", "アダプター検証", payload("無効化")),
            request("quarantine", "アダプター隔離", payload("隔離")),
            request("remove", "アダプター削除", payload("削除")),
            request("empty", "アダプター一覧", json!({"版":1})),
        ];
        let bytes = frames.iter().map(|v| format!("{v}\n")).collect::<String>();
        let mut output = Vec::new();
        let mut confirmations = 0;
        serve(
            &root,
            &VerifiedBundle,
            &mut io::Cursor::new(bytes),
            &mut output,
            &mut |summary| {
                confirmations += 1;
                assert!(summary.contains("要求hash: sha256:"));
                if confirmations > 1 {
                    assert!(summary.contains("Adapter: mock_local_llm_adapter"));
                    assert!(summary.contains("現在Adapter hash: sha256:"));
                }
                confirmations != 2
            },
        )
        .unwrap();
        let replies: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(replies.len(), frames.len());
        assert_eq!(replies[0]["status"], "accepted");
        assert_eq!(replies[1]["status"], "suspended");
        assert_eq!(
            replies[2]["body"]["Adapter一覧"][0]["管理状態"],
            "installed"
        );
        assert_eq!(
            replies[3]["error"]["code"],
            "adapter_signed_payload_mismatch"
        );
        assert_eq!(replies[4]["error"]["code"], "adapter_transition_denied");
        assert_eq!(replies[5]["body"]["管理状態"], "disabled");
        assert_eq!(replies[6]["error"]["code"], "adapter_stale");
        for reply in &replies[7..9] {
            assert_ne!(reply["status"], "accepted");
        }
        assert_eq!(replies[9]["body"]["管理状態"], "quarantined");
        assert_eq!(replies[10]["body"]["管理状態"], "removed");
        assert!(replies[11]["body"]["Adapter一覧"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(confirmations, 8);
        let audit = fs::read_to_string(root.join("store/audit.jsonl")).unwrap();
        for operation in ["検証", "有効化", "無効化", "隔離", "削除"] {
            assert!(audit.contains(&format!("アダプター{operation}")));
        }
        assert!(!root.join("normal-session.json").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn owner_adapter_roundtrip_denial_install_update_replay_and_injection() {
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).unwrap();
        let root = std::env::temp_dir().join(format!("d4p-macos-owner-{}", hex::encode(random)));
        fs::create_dir(&root).unwrap();
        let payload: Value = serde_json::from_str(include_str!(
            "../../../examples/contracts/adapter_management_request.valid.json"
        ))
        .unwrap();
        let request = |id: &str, operation: &str, payload: Value| {
            json!({
                "request_id": id, "nonce": id, "operation": operation,
                "issued_at": BrokerRequestEnvelope::current_issued_at(),
                "payload_hash": crate::broker::protocol::canonical_payload_hash(Some(&payload)),
                "metadata": {"client": "desktop_flutter"}, "payload": payload,
            })
        };
        let denied = request("macos-denied", "アダプター導入", payload.clone());
        let installed = request("macos-install", "アダプター導入", payload.clone());
        let mut updated_payload = payload.clone();
        updated_payload["操作"] = "更新".into();
        updated_payload["Adapter hash"] =
            crate::broker::protocol::canonical_payload_hash(Some(&payload["Manifest"])).into();
        updated_payload["Manifest"]["version"] = "1.1.0".into();
        let updated = request("macos-update", "アダプター更新", updated_payload);
        let mut injected = request("macos-owner-injected", "アダプター導入", payload.clone());
        injected["session_id"] = "owner".into();
        let mut bad_hash = request("macos-owner-bad-hash", "アダプター導入", payload);
        bad_hash["payload_hash"] = format!("sha256:{}", "0".repeat(64)).into();
        let bytes = [
            denied,
            request("macos-empty", "アダプター一覧", json!({"版":1})),
            installed.clone(),
            installed,
            updated,
            injected,
            bad_hash,
        ]
        .iter()
        .map(|v| format!("{v}\n"))
        .collect::<String>();
        let mut output = Vec::new();
        let mut confirmations = 0;
        serve(
            &root,
            &VerifiedBundle,
            &mut io::Cursor::new(bytes),
            &mut output,
            &mut |summary| {
                confirmations += 1;
                assert!(summary.contains("要求hash: sha256:"));
                confirmations != 1
            },
        )
        .unwrap();
        let replies: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_ne!(replies[0]["status"], "accepted");
        assert_eq!(
            replies[1]["body"]["Adapter一覧"].as_array().unwrap().len(),
            0
        );
        assert_eq!(replies[2]["status"], "accepted");
        assert_eq!(replies[3]["error"]["code"], "broker_replay_detected");
        assert_eq!(replies[4]["status"], "accepted");
        assert_ne!(replies[5]["status"], "accepted");
        assert_ne!(replies[6]["status"], "accepted");
        assert_eq!(confirmations, 4);
        let catalog = fs::read_to_string(root.join("store/adapters.json")).unwrap();
        assert!(catalog.contains("1.1.0"));
        let audit = fs::read_to_string(root.join("store/audit.jsonl")).unwrap();
        assert!(audit.contains("アダプター更新"));
        assert!(!audit.contains("GUI-Shell開発fixture"));
        fs::remove_dir_all(root).unwrap();
    }
}
