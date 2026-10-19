//! Linux Desktop同梱helper。秘密とBroker資格をRust内に保持し、既存Brokerだけへ中継する。
use crate::broker::ipc_server::run_loopback_server_cancellable_with_owner_operations;
use crate::broker::{BrokerCredentialRole, BrokerEndpoint, BrokerServerConfig};
use crate::linux_credential_input::LinuxCredentialUi;
use rustix::fs::{flock, FlockOperation};
use std::fs::{self, File, OpenOptions};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::thread;
use std::time::Duration;
use std::{
    collections::BTreeSet,
    io::{self, BufRead, BufReader, Read, Write},
};
use zeroize::{Zeroize, Zeroizing};

const REQUEST_LIMIT: usize = 64 * 1024;
const RESPONSE_LIMIT: usize = 4 * 1024 * 1024;
const OWNER_SUMMARY_LIMIT: usize = 16 * 1024;
const OWNER_REPLY_LIMIT: usize = 128;
const OWNER_PROMPT_LIMIT: usize = OWNER_SUMMARY_LIMIT * 2 + 64;
const OWNER_PROTOCOL_MAGIC: [u8; 8] = [68, 52, 79, 87, 78, 69, 82, 49];
const OWNER_REPLY_YES: [u8; 3] = [89, 69, 83];
#[cfg(test)]
const OWNER_REPLY_NO: [u8; 2] = [78, 79];
const IO_TIMEOUT: Duration = Duration::from_secs(4);

fn failure() -> io::Error {
    io::Error::other("Linux安全Broker接続を確認できません")
}

pub fn run() -> io::Result<()> {
    if std::env::args_os().len() != 1 {
        return Err(failure());
    }
    let bundle = verify_bundle(&std::env::current_exe()?)?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or_else(failure)?);
    let home = verify_owned_directory(&home, false)?;
    let state = match std::env::var_os("XDG_STATE_HOME") {
        Some(path) => {
            let base = verify_owned_directory(Path::new(&path), false)?;
            private_directory(&base, &["d4-pocket", "linux-broker"], 0)?
        }
        None => private_directory(&home, &[".local", "state", "d4-pocket", "linux-broker"], 2)?,
    };
    let runtime_base = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").ok_or_else(failure)?);
    let runtime_base = verify_owned_directory(&runtime_base, true)?;
    let runtime = private_directory(&runtime_base, &["d4-pocket", "linux-broker"], 0)?;
    serve(
        &state,
        &runtime,
        &bundle,
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
    )
}

/// Runnerとhelperが同じbundleに固定されることを確かめる。署名や配布identityの証明ではない。
struct VerifiedBundle;

fn verify_bundle(helper: &Path) -> io::Result<VerifiedBundle> {
    if !helper.is_absolute()
        || helper.file_name().and_then(|v| v.to_str()) != Some("gui_shell_linux_broker")
    {
        return Err(failure());
    }
    let libexec = helper.parent().ok_or_else(failure)?;
    let bundle = libexec.parent().ok_or_else(failure)?;
    if libexec.file_name().and_then(|v| v.to_str()) != Some("libexec") {
        return Err(failure());
    }
    let canonical_bundle = bundle.canonicalize()?;
    for path in [bundle.join("gui_shell_desktop"), helper.to_path_buf()] {
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

fn verify_owned_directory(path: &Path, private: bool) -> io::Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || (private && metadata.permissions().mode() & 0o077 != 0)
    {
        return Err(failure());
    }
    path.canonicalize()
}

fn private_directory(base: &Path, names: &[&str], private_from: usize) -> io::Result<PathBuf> {
    let uid = rustix::process::geteuid().as_raw();
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
            || metadata.uid() != uid
            || (index >= private_from && metadata.permissions().mode() & 0o077 != 0)
        {
            return Err(failure());
        }
    }
    Ok(path)
}

struct RunLock {
    _file: File,
}

impl RunLock {
    fn acquire(root: &Path) -> io::Result<Self> {
        let path = root.join("running.lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(&path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file()
            || metadata.nlink() != 1
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(failure());
        }
        flock(&file, FlockOperation::NonBlockingLockExclusive).map_err(|_| failure())?;
        Ok(Self { _file: file })
    }
}

struct NormalEndpoint(BrokerEndpoint);

impl Drop for NormalEndpoint {
    fn drop(&mut self) {
        self.0.session_secret.zeroize();
    }
}

fn serve(
    state: &Path,
    runtime: &Path,
    _bundle: &VerifiedBundle,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> io::Result<()> {
    let _lock = RunLock::acquire(runtime)?;
    let session = runtime.join("normal-session.json");
    remove_stale_private_file(&session)?;
    remove_stale_private_file(&session.with_extension("json.tmp"))?;
    let shutdown = Arc::new(AtomicBool::new(false));
    let stop = Arc::clone(&shutdown);
    let mut config = BrokerServerConfig::new(state.join("store"), session.clone());
    config.max_request_bytes = REQUEST_LIMIT;
    config.desktop_package_layout_verified = true;
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (owner_tx, owner_rx) = mpsc::sync_channel(4);
    let server = thread::Builder::new()
        .name("linux-security-broker".into())
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
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.permissions().mode() & 0o077 != 0
            || metadata.len() > 4096
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
            || !valid_session_id(&endpoint.0.session_id)
            || !valid_secret(&endpoint.0.session_secret)
        {
            return Err(failure());
        }
        let mut seen_credential_nonces = BTreeSet::new();
        loop {
            let Some(frame) = read_frame(input, REQUEST_LIMIT)? else {
                break;
            };
            let frame = Zeroizing::new(frame);
            let mut confirmation_error = None;
            let mut ui = LinuxNativeUi { input, output };
            let owner_response = match crate::linux_credential_input::dispatch_native(
                &frame,
                &endpoint.0,
                &owner_tx,
                &mut seen_credential_nonces,
                &mut ui,
            )? {
                Some(response) => Some(response),
                None => crate::macos_desktop_owner::dispatch(
                    &frame,
                    &endpoint.0,
                    &owner_tx,
                    &mut |summary| match ui.confirm_owner(summary) {
                        Ok(confirmed) => confirmed,
                        Err(error) => {
                            confirmation_error = Some(error);
                            false
                        }
                    },
                )?,
            };
            if let Some(error) = confirmation_error {
                return Err(error);
            }
            let response = match owner_response {
                Some(response) => response,
                None => {
                    let request = Zeroizing::new(normalize(&frame, &endpoint.0.session_id));
                    relay(&request, &endpoint.0)?
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

/// GTK Runnerだけが返せる、一回限りの確認応答を現在のsummaryへ束縛する。
fn request_owner_confirmation(
    summary: &str,
    input: &mut (impl BufRead + ?Sized),
    output: &mut (impl Write + ?Sized),
) -> io::Result<bool> {
    let mut challenge = [0_u8; 16];
    getrandom::getrandom(&mut challenge).map_err(|_| failure())?;
    let challenge = hex::encode(challenge);
    let Some(prompt) = owner_confirmation_prompt(&challenge, summary) else {
        return Ok(false);
    };
    output.write_all(prompt.as_bytes())?;
    output.flush()?;
    let Some(reply) = read_frame(input, OWNER_REPLY_LIMIT)? else {
        return Err(failure());
    };
    Ok(owner_confirmation_reply_is_yes(&reply, &challenge))
}

struct LinuxNativeUi<'a> {
    input: &'a mut dyn BufRead,
    output: &'a mut dyn Write,
}

impl crate::linux_credential_input::LinuxCredentialUi for LinuxNativeUi<'_> {
    fn enter_secret(
        &mut self,
        challenge: &str,
        summary: &str,
    ) -> io::Result<Option<Zeroizing<String>>> {
        let Some(prompt) = crate::linux_credential_input::input_prompt(challenge, summary) else {
            return Ok(None);
        };
        self.output.write_all(prompt.as_bytes())?;
        self.output.flush()?;
        let Some(mut reply) =
            read_frame(self.input, crate::linux_credential_input::INPUT_REPLY_LIMIT)?
        else {
            return Err(failure());
        };
        let secret = crate::linux_credential_input::input_reply_secret(&reply, challenge);
        reply.zeroize();
        Ok(secret)
    }

    fn confirm_owner(&mut self, summary: &str) -> io::Result<bool> {
        request_owner_confirmation(summary, self.input, self.output)
    }
}

fn owner_protocol_magic() -> &'static str {
    std::str::from_utf8(&OWNER_PROTOCOL_MAGIC).expect("固定Owner protocolはASCIIである")
}

fn owner_reply_yes() -> &'static str {
    std::str::from_utf8(&OWNER_REPLY_YES).expect("固定Owner応答はASCIIである")
}

#[cfg(test)]
fn owner_reply_no() -> &'static str {
    std::str::from_utf8(&OWNER_REPLY_NO).expect("固定Owner応答はASCIIである")
}

fn owner_confirmation_prompt(challenge: &str, summary: &str) -> Option<String> {
    if challenge.len() != 32
        || !challenge
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || summary.is_empty()
        || summary.len() > OWNER_SUMMARY_LIMIT
        || summary.chars().any(|character| {
            character == '\0' || (character.is_control() && character != '\n' && character != '\t')
        })
    {
        None
    } else {
        let frame = format!(
            "{} {challenge} {}\n",
            owner_protocol_magic(),
            hex::encode(summary)
        );
        (frame.len() <= OWNER_PROMPT_LIMIT).then_some(frame)
    }
}

fn owner_confirmation_reply_is_yes(reply: &[u8], challenge: &str) -> bool {
    let Ok(reply) = std::str::from_utf8(reply) else {
        return false;
    };
    let mut fields = reply.split(' ');
    matches!(
        (fields.next(), fields.next(), fields.next(), fields.next()),
        (Some(magic), Some(received), Some(decision), None)
            if magic == owner_protocol_magic()
                && decision == owner_reply_yes()
                && received == challenge
    )
}

/// exclusive process lock取得後だけ、前回crashが残した固定名の秘密fileを回収する。
fn remove_stale_private_file(path: &Path) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
        || metadata.len() > 4096
    {
        return Err(failure());
    }
    fs::remove_file(path)
}

fn valid_session_id(value: &str) -> bool {
    value.strip_prefix("broker-session-").is_some_and(|hex| {
        hex.len() == 32
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

fn valid_secret(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn read_frame(input: &mut (impl BufRead + ?Sized), limit: usize) -> io::Result<Option<Vec<u8>>> {
    let mut frame = Vec::new();
    let size = input
        .take(limit.saturating_add(2) as u64)
        .read_until(b'\n', &mut frame)?;
    if size == 0 {
        return Ok(None);
    }
    if size > limit.saturating_add(1)
        || frame.pop() != Some(b'\n')
        || frame.len() > limit
        || frame.contains(&b'\r')
    {
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

    #[test]
    fn channel_binds_only_the_rust_owned_session_and_rejects_caller_session() {
        let normalized = normalize(
            br#"{"request_id":"r1"}"#,
            "broker-session-0123456789abcdef0123456789abcdef",
        );
        let value: serde_json::Value = serde_json::from_slice(&normalized).unwrap();
        assert_eq!(
            value["session_id"],
            "broker-session-0123456789abcdef0123456789abcdef"
        );

        let normalized = normalize(
            br#"{"request_id":"r1","session_id":"attacker"}"#,
            "broker-session-0123456789abcdef0123456789abcdef",
        );
        let value: serde_json::Value = serde_json::from_slice(&normalized).unwrap();
        assert_eq!(value["session_id"], "attacker");
        assert_eq!(value["desktop_channel_session_id_forbidden"], true);
    }

    #[test]
    fn channel_frame_reader_bounds_size_and_rejects_crlf() {
        assert_eq!(
            read_frame(&mut &b"{}\n"[..], 2).unwrap(),
            Some(b"{}".to_vec())
        );
        assert_eq!(
            read_frame(&mut &b"123\n"[..], 3).unwrap(),
            Some(b"123".to_vec())
        );
        assert!(read_frame(&mut &b"{}\r\n"[..], 8).is_err());
        assert!(read_frame(&mut &b"1234\n"[..], 3).is_err());
        assert_eq!(read_frame(&mut &b""[..], 4).unwrap(), None);
    }

    #[test]
    fn owner_confirmation_frame_is_bounded_encoded_and_challenge_bound() {
        let challenge = "0123456789abcdef0123456789abcdef";
        let summary = "Owner確認\nWorkspace ID: fixture";
        let frame = owner_confirmation_prompt(challenge, summary).unwrap();
        assert!(frame.starts_with(&format!("{} {challenge} ", owner_protocol_magic())));
        assert!(!frame.contains(summary));
        let encoded = frame
            .trim_end_matches('\n')
            .split_once(' ')
            .unwrap()
            .1
            .split_once(' ')
            .unwrap()
            .1;
        assert_eq!(hex::decode(encoded).unwrap(), summary.as_bytes());
        assert!(frame.len() <= OWNER_PROMPT_LIMIT);
        assert!(owner_confirmation_prompt("bad", summary).is_none());
        assert!(
            owner_confirmation_prompt(challenge, &"x".repeat(OWNER_SUMMARY_LIMIT + 1)).is_none()
        );
        assert!(owner_confirmation_prompt(challenge, "bad\rsummary").is_none());
        assert!(owner_confirmation_prompt(challenge, "bad\0summary").is_none());
    }

    #[test]
    fn owner_confirmation_accepts_only_matching_single_yes_reply() {
        let challenge = "0123456789abcdef0123456789abcdef";
        assert!(owner_confirmation_reply_is_yes(
            format!(
                "{} {challenge} {}",
                owner_protocol_magic(),
                owner_reply_yes()
            )
            .as_bytes(),
            challenge
        ));
        assert!(!owner_confirmation_reply_is_yes(
            format!(
                "{} {challenge} {}",
                owner_protocol_magic(),
                owner_reply_no()
            )
            .as_bytes(),
            challenge
        ));
        assert!(!owner_confirmation_reply_is_yes(
            format!(
                "{} ffffffffffffffffffffffffffffffff {}",
                owner_protocol_magic(),
                owner_reply_yes()
            )
            .as_bytes(),
            challenge
        ));
        assert!(!owner_confirmation_reply_is_yes(
            format!(
                "{} {challenge} {} extra",
                owner_protocol_magic(),
                owner_reply_yes()
            )
            .as_bytes(),
            challenge
        ));
        assert!(!owner_confirmation_reply_is_yes(
            format!(
                "{} {challenge} {}",
                owner_protocol_magic(),
                owner_reply_yes().to_ascii_lowercase()
            )
            .as_bytes(),
            challenge
        ));
    }

    #[test]
    fn linux_session_material_requires_fixed_lowercase_hex_shapes() {
        assert!(valid_session_id(
            "broker-session-0123456789abcdef0123456789abcdef"
        ));
        assert!(!valid_session_id(
            "broker-session-0123456789ABCDEF0123456789ABCDEF"
        ));
        assert!(valid_secret(&"a".repeat(64)));
        assert!(!valid_secret(&"g".repeat(64)));
    }

    #[test]
    fn process_lock_is_exclusive_and_recoverable_after_owner_exit() {
        use std::os::unix::fs::PermissionsExt;
        use std::time::{SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("d4-linux-lock-{}-{nonce}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();

        let first = RunLock::acquire(&root).unwrap();
        assert!(RunLock::acquire(&root).is_err());
        drop(first);
        let recovered = RunLock::acquire(&root).unwrap();
        drop(recovered);
        assert!(root.join("running.lock").is_file());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stale_private_file_cleanup_rejects_links_and_shared_files() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        use std::time::{SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("d4-linux-stale-{}-{nonce}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();

        let stale = root.join("normal-session.json");
        fs::write(&stale, b"stale endpoint").unwrap();
        fs::set_permissions(&stale, fs::Permissions::from_mode(0o600)).unwrap();
        remove_stale_private_file(&stale).unwrap();
        assert!(!stale.exists());

        let target = root.join("preserve-me");
        fs::write(&target, b"keep").unwrap();
        let shared = root.join("shared-session.json");
        fs::write(&shared, b"must not remove").unwrap();
        fs::set_permissions(&shared, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(remove_stale_private_file(&shared).is_err());
        assert_eq!(fs::read(&shared).unwrap(), b"must not remove");

        let link = root.join("normal-session.json.tmp");
        symlink(&target, &link).unwrap();
        assert!(remove_stale_private_file(&link).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"keep");

        fs::remove_dir_all(root).unwrap();
    }
}
