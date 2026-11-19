//! 同梱macOS helper。匿名pipeはtransportだけで、権限判断は既存認証Brokerへ残す。
use crate::broker::ipc_server::run_loopback_server_cancellable_with_owner_operations;
use crate::broker::{BrokerCredentialRole, BrokerEndpoint, BrokerServerConfig};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
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
const SESSION_FILE_LIMIT: u64 = 4096;
const PRODUCT_MANIFEST_LIMIT: u64 = 64 * 1024;
#[cfg(unix)]
const RUN_LOCK_FORMAT: &[u8] = b"D4 Pocket macOS Broker lock v2\n";
const IO_TIMEOUT: Duration = Duration::from_secs(4);

fn failure() -> io::Error {
    io::Error::other("macOS Broker接続を確認できません")
}

#[cfg(target_os = "macos")]
pub fn run() -> io::Result<()> {
    if std::env::args_os().len() != 1 {
        return Err(failure());
    }
    let product_identity =
        crate::broker::update_center::embedded_product_identity().map_err(|_| failure())?;
    let manifest_sha256 =
        crate::broker::update_center::embedded_product_manifest_sha256().map_err(|_| failure())?;
    if product_identity.is_some() != manifest_sha256.is_some() {
        return Err(failure());
    }
    let expected_identity = product_identity
        .as_ref()
        .map(|(app_id, audit_id)| (app_id.as_str(), audit_id.as_str()));
    let bundle = verify_bundle_with_product(
        &std::env::current_exe()?,
        expected_identity,
        manifest_sha256,
    )?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or_else(failure)?);
    if !home.is_absolute() || !home.is_dir() {
        return Err(failure());
    }
    let home = home.canonicalize()?;
    let storage_components = broker_storage_components(
        bundle
            .product_identity
            .as_ref()
            .map(|(app_id, audit_id)| (app_id.as_str(), audit_id.as_str())),
    )?;
    let root = private_directory(&home, &storage_components)?;
    serve(
        &root,
        &bundle,
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
        &mut |summary| gui_shell_macos_owner::confirm(summary, Duration::from_secs(300)),
    )
}

/// 固定bundle配置とcompile-time identityの検査結果。
/// 署名・配布identity・Owner承認・Permissionを表さない。
#[derive(Default)]
struct VerifiedBundle {
    product_identity: Option<(String, String)>,
}

fn verify_bundle(helper: &Path) -> io::Result<VerifiedBundle> {
    verify_bundle_with_product(helper, None, None)
}

fn verify_bundle_with_product(
    helper: &Path,
    expected_identity: Option<(&str, &str)>,
    expected_manifest_sha256: Option<&str>,
) -> io::Result<VerifiedBundle> {
    if expected_identity.is_some() != expected_manifest_sha256.is_some() {
        return Err(failure());
    }
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
    let manifest_path = contents.join("Resources/product_manifest.json");
    let product_identity = match (expected_identity, expected_manifest_sha256) {
        (None, None) => {
            if manifest_path.exists() || manifest_path.is_symlink() {
                return Err(failure());
            }
            None
        }
        (Some((app_id, audit_store_id)), Some(expected_hash)) => {
            if !valid_product_id(app_id, "d4-pocket-app-")
                || !valid_product_id(audit_store_id, "audit-store-")
                || expected_hash.len() != 64
                || !expected_hash
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(failure());
            }
            let resources = manifest_path.parent().ok_or_else(failure)?;
            let resource_metadata = fs::symlink_metadata(resources)?;
            let manifest_metadata = fs::symlink_metadata(&manifest_path)?;
            if !resource_metadata.is_dir()
                || resource_metadata.file_type().is_symlink()
                || !manifest_metadata.is_file()
                || manifest_metadata.file_type().is_symlink()
                || manifest_metadata.len() > PRODUCT_MANIFEST_LIMIT
                || !manifest_path.canonicalize()?.starts_with(&canonical_bundle)
            {
                return Err(failure());
            }
            let raw = fs::read(&manifest_path)?;
            use sha2::Digest as _;
            let actual_hash = hex::encode(sha2::Sha256::digest(&raw));
            if actual_hash != expected_hash {
                return Err(failure());
            }
            let manifest: serde_json::Value =
                serde_json::from_slice(&raw).map_err(|_| failure())?;
            if manifest.get("product").and_then(serde_json::Value::as_str) != Some("D4 Pocket")
                || manifest
                    .pointer("/manifest/app_identity/app_id")
                    .and_then(serde_json::Value::as_str)
                    != Some(app_id)
                || manifest
                    .pointer("/manifest/audit_store/store_id")
                    .and_then(serde_json::Value::as_str)
                    != Some(audit_store_id)
            {
                return Err(failure());
            }
            Some((app_id.to_owned(), audit_store_id.to_owned()))
        }
        _ => return Err(failure()),
    };
    Ok(VerifiedBundle { product_identity })
}

fn valid_product_id(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        suffix.len() == 32
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn broker_storage_components(identity: Option<(&str, &str)>) -> io::Result<Vec<String>> {
    let mut components = vec![
        "Library".to_owned(),
        "Application Support".to_owned(),
        "D4Pocket".to_owned(),
    ];
    match identity {
        None => components.push("macos-broker".to_owned()),
        Some((app_id, audit_store_id)) => {
            if !valid_product_id(app_id, "d4-pocket-app-")
                || !valid_product_id(audit_store_id, "audit-store-")
            {
                return Err(failure());
            }
            components.extend([
                "apps".to_owned(),
                app_id.to_owned(),
                "stores".to_owned(),
                audit_store_id.to_owned(),
            ]);
        }
    }
    Ok(components)
}

#[cfg(target_os = "macos")]
fn private_directory(base: &Path, names: &[String]) -> io::Result<PathBuf> {
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
    #[cfg(unix)]
    recovering: bool,
}
impl RunLock {
    #[cfg(unix)]
    fn acquire(root: &Path) -> io::Result<Self> {
        use rustix::fs::{flock, FlockOperation};

        let metadata = fs::symlink_metadata(root)?;
        if !metadata.is_dir()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o077 != 0
        {
            return Err(failure());
        }
        let path = root.join("running.lock");
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
        let (file, recovering) = match options.clone().create_new(true).open(&path) {
            Ok(file) => (file, false),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let file = options.open(&path)?;
                // 旧版の空fileや作成中のfileでは排他取得も回復も行わない。
                verify_lock_format(&file)?;
                (file, true)
            }
            Err(error) => return Err(error),
        };
        flock(&file, FlockOperation::NonBlockingLockExclusive).map_err(|_| failure())?;
        verify_private_file_identity(&path, &file)?;
        if recovering {
            verify_lock_format(&file)?;
        }
        Ok(Self {
            path,
            file,
            recovering,
        })
    }

    #[cfg(not(unix))]
    fn acquire(root: &Path) -> io::Result<Self> {
        let path = root.join("running.lock");
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        Ok(Self { path, file })
    }

    fn prepare_session(&mut self, path: &Path) -> io::Result<()> {
        let pending = path.with_extension("json.tmp");
        #[cfg(unix)]
        if self.recovering {
            let endpoints = [path, pending.as_path()];
            // 固定名を両方検査してから回収し、不正な場合は既存物を保持する。
            let inspected = [
                stale_endpoint_metadata(endpoints[0])?,
                stale_endpoint_metadata(endpoints[1])?,
            ];
            for (endpoint, old) in endpoints.into_iter().zip(inspected) {
                if let Some(old) = old {
                    let current = stale_endpoint_metadata(endpoint)?.ok_or_else(failure)?;
                    if !same_inode(&old, &current) {
                        return Err(failure());
                    }
                    fs::remove_file(endpoint)?;
                }
            }
            return Ok(());
        }
        let empty = [path, pending.as_path()]
            .into_iter()
            .try_for_each(|endpoint| match fs::symlink_metadata(endpoint) {
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
                Ok(_) => Err(failure()),
            });
        #[cfg(unix)]
        {
            // 元から存在した接続情報を自分の起動から残ったものとして扱わない。
            let marked = empty.and_then(|_| {
                self.file.write_all(RUN_LOCK_FORMAT)?;
                self.file.sync_all()
            });
            if marked.is_err() {
                self.remove_current_lock()?;
            }
            marked
        }
        #[cfg(not(unix))]
        empty
    }

    #[cfg(unix)]
    fn finish(self) -> io::Result<()> {
        self.remove_current_lock()
    }

    #[cfg(unix)]
    fn remove_current_lock(&self) -> io::Result<()> {
        verify_private_file_identity(&self.path, &self.file)?;
        fs::remove_file(&self.path)
    }

    #[cfg(not(unix))]
    fn finish(self) -> io::Result<()> {
        Ok(())
    }
}
#[cfg(not(unix))]
impl Drop for RunLock {
    fn drop(&mut self) {
        let _ = self.file.sync_all();
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(unix)]
fn private_file(metadata: &fs::Metadata) -> bool {
    metadata.is_file()
        && metadata.nlink() == 1
        && metadata.uid() == rustix::process::geteuid().as_raw()
        && metadata.mode() & 0o077 == 0
}

#[cfg(unix)]
fn same_inode(first: &fs::Metadata, second: &fs::Metadata) -> bool {
    first.ino() != 0 && first.ino() == second.ino() && first.dev() == second.dev()
}

#[cfg(unix)]
fn verify_private_file_identity(path: &Path, file: &File) -> io::Result<()> {
    let opened = file.metadata()?;
    let observed = fs::symlink_metadata(path)?;
    if !private_file(&opened) || !private_file(&observed) || !same_inode(&opened, &observed) {
        return Err(failure());
    }
    Ok(())
}

#[cfg(unix)]
fn verify_lock_format(file: &File) -> io::Result<()> {
    let metadata = file.metadata()?;
    if !private_file(&metadata) || metadata.len() != RUN_LOCK_FORMAT.len() as u64 {
        return Err(failure());
    }
    let mut marker = [0; RUN_LOCK_FORMAT.len()];
    file.read_exact_at(&mut marker, 0)?;
    if marker.as_slice() != RUN_LOCK_FORMAT {
        return Err(failure());
    }
    Ok(())
}

#[cfg(unix)]
fn stale_endpoint_metadata(path: &Path) -> io::Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if private_file(&metadata) && metadata.len() <= SESSION_FILE_LIMIT => {
            Ok(Some(metadata))
        }
        Ok(_) => Err(failure()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
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
    let mut lock = RunLock::acquire(root)?;
    let session = root.join("normal-session.json");
    lock.prepare_session(&session)?;
    // 保存先を検査できない場合、Brokerは書出しだけを拒否する。
    #[cfg(unix)]
    let export_root = crate::broker::export_center::open_private_export_directory(root).ok();
    #[cfg(not(unix))]
    let export_root = None;
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
                config,
                stop,
                ready_tx,
                owner_rx,
                export_root,
            )
        })?;
    let mut owned_session = Zeroizing::new(Vec::new());
    #[cfg(target_os = "macos")]
    let mut workspace_scopes = Vec::new();
    #[cfg(target_os = "macos")]
    let mut workspace_selection_nonces = std::collections::BTreeSet::new();
    #[cfg(target_os = "macos")]
    let mut credential_input_nonces = std::collections::BTreeSet::new();
    let result = (|| {
        ready_rx
            .recv_timeout(Duration::from_secs(15))
            .map_err(|_| failure())?;
        let metadata = fs::symlink_metadata(&session)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > SESSION_FILE_LIMIT
        {
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
            // private OS bookmarkを上限内のmemoryだけで処理し、処理後に消去する。
            let frame = Zeroizing::new(frame);
            #[cfg(target_os = "macos")]
            if let Some(response) = super::macos_credential_input::dispatch_native(
                &frame,
                &endpoint.0,
                &owner_tx,
                &mut credential_input_nonces,
                confirm,
            )? {
                output.write_all(&response)?;
                output.write_all(b"\n")?;
                output.flush()?;
                continue;
            }
            #[cfg(target_os = "macos")]
            let selection = super::macos_workspace_selection::dispatch_native(
                &frame,
                &endpoint.0,
                &owner_tx,
                &mut workspace_scopes,
                &mut workspace_selection_nonces,
            )?;
            #[cfg(not(target_os = "macos"))]
            let selection: Option<Vec<u8>> = None;
            let response = if let Some(response) = selection {
                response
            } else {
                match super::macos_desktop_owner::dispatch(&frame, &endpoint.0, &owner_tx, confirm)?
                {
                    Some(response) => response,
                    None => relay(&normalize(&frame, &endpoint.0.session_id), &endpoint.0)?,
                }
            };
            output.write_all(&response)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
        Ok(())
    })();
    shutdown.store(true, Ordering::Release);
    let stopped = server.join().map_err(|_| failure())?.map_err(|_| failure());
    #[cfg(target_os = "macos")]
    drop(workspace_scopes);
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
    result.and(stopped).and(cleanup)?;
    lock.finish()
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

pub(super) fn relay(request: &[u8], endpoint: &BrokerEndpoint) -> io::Result<Vec<u8>> {
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

    #[cfg(unix)]
    #[test]
    fn native_ai_edit_proposal_records_only_the_confirmed_request() {
        let (_temp, root) = recovery_fixture();
        let payload: Value = serde_json::from_str(include_str!(
            "../../../examples/contracts/gui_shell_edit_proposal.valid.json"
        ))
        .unwrap();
        let request = |id: &str, payload: &Value| {
            json!({
                "request_id": id, "nonce": id, "operation": "GUI Shell編集提案",
                "issued_at": BrokerRequestEnvelope::current_issued_at(),
                "metadata": {"client": "desktop_flutter"},
                "payload_hash": crate::broker::protocol::canonical_payload_hash(Some(payload)),
                "payload": payload,
            })
        };
        let denied = request("edit-proposal-denied", &payload);
        let mut invalid_payload = payload.clone();
        invalid_payload["self_approval"] = json!(true);
        let invalid = request("edit-proposal-invalid", &invalid_payload);
        let approved = request("edit-proposal-approved", &payload);
        let hash = approved["payload_hash"].as_str().unwrap();
        let instruction = payload["instruction"].as_str().unwrap();
        let mut confirmations = 0;
        let mut output = Vec::new();
        serve(
            &root,
            &VerifiedBundle::default(),
            &mut io::Cursor::new(format!("{denied}\n{invalid}\n{approved}\n{approved}\n")),
            &mut output,
            &mut |summary| {
                assert!(summary.contains(hash));
                assert!(summary.contains("対象ファイル 2 件"));
                assert!(!summary.contains(instruction));
                confirmations += 1;
                confirmations != 1
            },
        )
        .unwrap();
        assert_eq!(confirmations, 3);
        let replies: Vec<Value> = std::str::from_utf8(&output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(replies.len(), 4);
        assert_eq!(replies[0]["error"]["code"], "owner_required");
        assert_eq!(replies[1]["error"]["code"], "owner_required");
        assert_eq!(replies[2]["status"], "accepted");
        let receipt = &replies[2]["body"];
        assert_eq!(receipt["instruction_hash"], hash);
        assert_eq!(receipt["apply_status"], "not_started");
        assert_eq!(receipt["files_written"], false);
        assert_eq!(receipt["permission_generated"], false);
        assert_eq!(replies[3]["error"]["code"], "broker_replay_detected");
        let audit = fs::read_to_string(root.join("store/audit.jsonl")).unwrap();
        let accepted: Vec<Value> = audit
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .filter(|event| {
                event["operation"] == "GUI Shell編集提案" && event["decision"] == "accepted"
            })
            .collect();
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0]["event_id"], receipt["audit_id"]);
        assert_eq!(accepted[0]["payload_hash"], hash);
        assert!(!audit.contains(instruction));
        assert!(!std::str::from_utf8(&output).unwrap().contains(instruction));
    }

    #[cfg(unix)]
    fn export_request(id: &str) -> Value {
        let mut payload: Value = serde_json::from_str(include_str!(
            "../../../examples/contracts/gui_shell_export.valid.json"
        ))
        .unwrap();
        payload["target_platform"] = json!("macos");
        for field in ["runtime_ids", "agent_ids", "capability_requirements"] {
            payload["compose_manifest"][field] = json!([]);
        }
        json!({
            "request_id": id, "nonce": id, "operation": "GUI Shell書出し",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"},
            "payload_hash": crate::broker::protocol::canonical_payload_hash(Some(&payload)),
            "payload": payload,
        })
    }

    #[cfg(unix)]
    #[test]
    fn owner_export_uses_fixed_private_directory_and_preserves_denial() {
        use std::os::unix::fs::DirBuilderExt;

        let temp = registration_fixture::FixtureTempDirectory::create();
        let root = temp.path().canonicalize().unwrap().join("broker");
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let denied = export_request("macos-export-denied");
        let approved = export_request("macos-export-approved");
        let mut output = Vec::new();
        let mut confirmations = 0;
        serve(
            &root,
            &VerifiedBundle::default(),
            &mut io::Cursor::new(format!("{denied}\n{approved}\n")),
            &mut output,
            &mut |summary| {
                assert!(summary.contains("対象platform: macOS"));
                assert!(summary.contains("固定Export directory"));
                confirmations += 1;
                confirmations == 2
            },
        )
        .unwrap();
        assert_eq!(confirmations, 2);
        let replies: Vec<Value> = std::str::from_utf8(&output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(replies.len(), 2);
        assert_eq!(replies[0]["error"]["code"], "owner_required");
        assert_eq!(
            replies[1]["status"], "accepted",
            "{}",
            replies[1]["error"]["code"]
        );
        let receipt = &replies[1]["body"];
        assert_eq!(receipt["manifest_file_status"], "written");
        assert_eq!(receipt["artifact_status"], "not_built");
        assert_eq!(receipt["authority_strip"], true);
        assert_eq!(receipt["credential_inherited"], false);
        let path = Path::new(receipt["manifest_file"]["path"].as_str().unwrap());
        assert_eq!(path.parent(), Some(root.join("exports").as_path()));
        assert_eq!(fs::read_dir(root.join("exports")).unwrap().count(), 1);
        let bytes = fs::read(path).unwrap();
        assert_eq!(
            receipt["manifest_file"]["sha256"],
            crate::audit_hash::sha256_tagged(&bytes)
        );
        let manifest: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            manifest["manifest"]["app_identity"]["target_platform"],
            "macos"
        );
        assert_eq!(
            manifest["manifest"]["inheritance_policy"]["authority"],
            "none"
        );
        let audit = fs::read_to_string(root.join("store/audit.jsonl")).unwrap();
        assert!(audit.contains(replies[1]["audit_event_id"].as_str().unwrap()));
        assert!(audit.contains(receipt["manifest_file"]["sha256"].as_str().unwrap()));
    }

    #[cfg(unix)]
    #[test]
    fn owner_export_rejects_linked_directory_and_leaves_broker_available() {
        use std::os::unix::fs::{symlink, DirBuilderExt};

        let temp = registration_fixture::FixtureTempDirectory::create();
        let root = temp.path().canonicalize().unwrap().join("broker");
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let target = temp.path().join("outside");
        fs::DirBuilder::new().mode(0o700).create(&target).unwrap();
        symlink(target.canonicalize().unwrap(), root.join("exports")).unwrap();
        let request = export_request("macos-export-linked");
        let health = json!({
            "request_id": "macos-export-health", "nonce": "macos-export-health",
            "operation": "health",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"},
            "payload_hash": crate::broker::protocol::canonical_payload_hash(None),
        });
        let mut output = Vec::new();
        serve(
            &root,
            &VerifiedBundle::default(),
            &mut io::Cursor::new(format!("{request}\n{health}\n")),
            &mut output,
            &mut |_| true,
        )
        .unwrap();
        let replies: Vec<Value> = std::str::from_utf8(&output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            replies[0]["error"]["code"],
            "gui_shell_export_target_unavailable"
        );
        assert_eq!(replies[1]["status"], "accepted");
        assert_eq!(fs::read_dir(target).unwrap().count(), 0);
        assert!(root.join("exports").is_symlink());
    }

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
            &VerifiedBundle::default(),
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
            &VerifiedBundle::default(),
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
        lock.finish().unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[cfg(unix)]
    fn recovery_fixture() -> (registration_fixture::FixtureTempDirectory, PathBuf) {
        use std::os::unix::fs::DirBuilderExt;

        let temp = registration_fixture::FixtureTempDirectory::create();
        let root = temp.path().canonicalize().unwrap().join("broker");
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        (temp, root)
    }

    #[cfg(unix)]
    fn write_private_fixture(path: &Path, bytes: &[u8]) {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .unwrap()
            .write_all(bytes)
            .unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn restart_reclaims_only_released_run_and_preserves_broker_audit() {
        let (_temp, root) = recovery_fixture();
        let session = root.join("normal-session.json");
        let pending = session.with_extension("json.tmp");
        let audit = root.join("store/audit.jsonl");
        let health = |id: &str| {
            json!({
                "request_id": id, "nonce": id, "operation": "health",
                "issued_at": BrokerRequestEnvelope::current_issued_at(),
                "payload_hash": crate::broker::protocol::canonical_payload_hash(None),
                "metadata": {"client": "desktop_flutter"}
            })
        };
        let first = health("macos-before-restart");
        serve(
            &root,
            &VerifiedBundle::default(),
            &mut io::Cursor::new(format!("{first}\n")),
            &mut Vec::new(),
            &mut |_| false,
        )
        .unwrap();
        let previous_audit = fs::read(&audit).unwrap();

        let mut old_run = RunLock::acquire(&root).unwrap();
        old_run.prepare_session(&session).unwrap();
        write_private_fixture(&session, b"old synthetic endpoint");
        write_private_fixture(&pending, b"interrupted synthetic endpoint");
        assert!(RunLock::acquire(&root).is_err());
        assert!(session.is_file() && pending.is_file());
        drop(old_run);

        let mut output = Vec::new();
        let second = health("macos-after-restart");
        serve(
            &root,
            &VerifiedBundle::default(),
            &mut io::Cursor::new(format!("{second}\n")),
            &mut output,
            &mut |_| false,
        )
        .unwrap();
        let reply: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(reply["status"], "accepted");
        assert!(fs::read(&audit).unwrap().starts_with(&previous_audit));
        assert!(!session.exists());
        assert!(!pending.exists());
        assert!(!root.join("running.lock").exists());
    }

    #[cfg(unix)]
    #[test]
    fn restart_does_not_adopt_legacy_lock_or_unattributed_endpoint() {
        let (_temp, root) = recovery_fixture();
        let session = root.join("normal-session.json");
        let legacy_lock = root.join("running.lock");
        write_private_fixture(&legacy_lock, b"");
        write_private_fixture(&session, b"unattributed endpoint");
        assert!(RunLock::acquire(&root).is_err());
        assert_eq!(fs::read(&legacy_lock).unwrap(), b"");
        assert_eq!(fs::read(&session).unwrap(), b"unattributed endpoint");

        fs::remove_file(&legacy_lock).unwrap();
        for _ in 0..2 {
            let mut fresh = RunLock::acquire(&root).unwrap();
            assert!(fresh.prepare_session(&session).is_err());
            assert!(!legacy_lock.exists());
            assert_eq!(fs::read(&session).unwrap(), b"unattributed endpoint");
        }
    }

    #[cfg(unix)]
    #[test]
    fn restart_refuses_linked_pending_endpoint_before_removing_other_files() {
        use std::os::unix::fs::symlink;

        let (_temp, root) = recovery_fixture();
        let session = root.join("normal-session.json");
        let pending = session.with_extension("json.tmp");
        let target = root.join("protected");
        write_private_fixture(&target, b"protected data");
        let mut old_run = RunLock::acquire(&root).unwrap();
        old_run.prepare_session(&session).unwrap();
        write_private_fixture(&session, b"stale endpoint");
        symlink(&target, &pending).unwrap();
        drop(old_run);

        let mut recovered = RunLock::acquire(&root).unwrap();
        assert!(recovered.prepare_session(&session).is_err());
        assert_eq!(fs::read(&session).unwrap(), b"stale endpoint");
        assert_eq!(fs::read(&target).unwrap(), b"protected data");
        assert!(pending.is_symlink());
        assert!(root.join("running.lock").is_file());
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
    fn exported_bundle_manifest_is_compile_time_bound_and_storage_isolated() {
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).unwrap();
        let root = std::env::temp_dir().join(format!("d4p-macos-identity-{}", hex::encode(random)));
        let contents = root.join("D4Pocket.app/Contents");
        fs::create_dir_all(contents.join("MacOS")).unwrap();
        fs::create_dir_all(contents.join("Resources")).unwrap();
        fs::write(
            contents.join("MacOS/gui_shell_macos_broker"),
            b"synthetic helper",
        )
        .unwrap();
        fs::write(contents.join("MacOS/gui_shell_desktop"), b"synthetic app").unwrap();
        fs::write(contents.join("Info.plist"), b"synthetic plist").unwrap();

        let app_id = format!("d4-pocket-app-{}", "1".repeat(32));
        let audit_id = format!("audit-store-{}", "2".repeat(32));
        let manifest = serde_json::json!({
            "version": 1,
            "product": "D4 Pocket",
            "export_id": "fixture-export",
            "manifest": {
                "app_identity": {"app_id": app_id},
                "audit_store": {"store_id": audit_id}
            }
        });
        let manifest_raw = serde_json::to_vec(&manifest).unwrap();
        use sha2::Digest as _;
        let manifest_hash = hex::encode(sha2::Sha256::digest(&manifest_raw));
        fs::write(
            contents.join("Resources/product_manifest.json"),
            &manifest_raw,
        )
        .unwrap();

        let helper = contents.join("MacOS/gui_shell_macos_broker");
        let bundle =
            verify_bundle_with_product(&helper, Some((&app_id, &audit_id)), Some(&manifest_hash))
                .unwrap();
        assert_eq!(
            bundle.product_identity,
            Some((app_id.clone(), audit_id.clone()))
        );
        assert!(verify_bundle_with_product(
            &helper,
            Some((&app_id, &audit_id)),
            Some(&"0".repeat(64)),
        )
        .is_err());
        let other_app_id = format!("d4-pocket-app-{}", "3".repeat(32));
        assert!(verify_bundle_with_product(
            &helper,
            Some((&other_app_id, &audit_id)),
            Some(&manifest_hash),
        )
        .is_err());

        let first = broker_storage_components(Some((&app_id, &audit_id))).unwrap();
        let second_audit = format!("audit-store-{}", "4".repeat(32));
        let second = broker_storage_components(Some((&app_id, &second_audit))).unwrap();
        let second_app = broker_storage_components(Some((&other_app_id, &audit_id))).unwrap();
        assert_ne!(first, second);
        assert_ne!(first, second_app);
        assert!(broker_storage_components(Some(("../outside", &audit_id))).is_err());
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
            &VerifiedBundle::default(),
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
            &VerifiedBundle::default(),
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
