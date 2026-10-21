//! Linux Desktop同梱helper。秘密とBroker資格をRust内に保持し、既存Brokerだけへ中継する。
use crate::broker::ipc_server::run_loopback_server_cancellable_with_owner_operations;
use crate::broker::{BrokerCredentialRole, BrokerEndpoint, BrokerServerConfig};
use crate::linux_credential_input::LinuxCredentialUi;
use crate::linux_product_bundle::{self, VerifiedBundle};
use cap_std::fs::Dir;
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
    error::Error,
    fmt,
    io::{self, BufRead, BufReader, Read, Write},
};
use zeroize::{Zeroize, Zeroizing};

const REQUEST_LIMIT: usize = 64 * 1024;
const RESPONSE_LIMIT: usize = 4 * 1024 * 1024;
const OWNER_SUMMARY_LIMIT: usize = 16 * 1024;
const OWNER_REPLY_LIMIT: usize = 128;
const OWNER_PROMPT_LIMIT: usize = OWNER_SUMMARY_LIMIT * 2 + 64;
const WORKSPACE_PICKER_REPLY_LIMIT: usize = 2 * 1024 + 128;
const OWNER_PROTOCOL_MAGIC: [u8; 8] = [68, 52, 79, 87, 78, 69, 82, 49];
const OWNER_REPLY_YES: [u8; 3] = [89, 69, 83];
// Catalog HTTPS has a 30-second Broker deadline; allow bounded relay headroom.
const CATALOG_RELAY_RESPONSE_TIMEOUT: Duration = Duration::from_secs(35);
#[cfg(test)]
const OWNER_REPLY_NO: [u8; 2] = [78, 79];
const IO_TIMEOUT: Duration = Duration::from_secs(4);

fn failure() -> io::Error {
    io::Error::other("Linux安全Broker接続を確認できません")
}

#[derive(Debug)]
struct ClassifiedHelperError {
    exit_code: i32,
}

impl fmt::Display for ClassifiedHelperError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Linux helper failure category")
    }
}

impl Error for ClassifiedHelperError {}

fn helper_error_kind_offset(kind: io::ErrorKind) -> i32 {
    match kind {
        io::ErrorKind::Other => 0,
        io::ErrorKind::TimedOut => 1,
        io::ErrorKind::ConnectionRefused => 2,
        io::ErrorKind::ConnectionReset => 3,
        io::ErrorKind::BrokenPipe => 4,
        io::ErrorKind::UnexpectedEof => 5,
        io::ErrorKind::InvalidData => 6,
        io::ErrorKind::PermissionDenied => 7,
        io::ErrorKind::NotFound => 8,
        io::ErrorKind::WouldBlock => 9,
        _ => 10,
    }
}

pub fn helper_exit_code(error: &io::Error) -> i32 {
    error
        .get_ref()
        .and_then(|source| source.downcast_ref::<ClassifiedHelperError>())
        .map(|classified| classified.exit_code)
        .unwrap_or_else(|| 20 + helper_error_kind_offset(error.kind()))
}

fn classify_stage_error(error: io::Error, exit_code: i32) -> io::Error {
    io::Error::new(error.kind(), ClassifiedHelperError { exit_code })
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
    let runtime = prepare_runtime_directory()?;
    let launch_nonce = std::env::var(crate::linux_desktop_launcher::LAUNCH_NONCE_ENV).ok();
    serve(
        &state,
        &runtime,
        &bundle,
        launch_nonce.as_deref(),
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
    )
}

pub(crate) fn prepare_runtime_directory() -> io::Result<PathBuf> {
    let runtime_base = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").ok_or_else(failure)?);
    let runtime_base = verify_owned_directory(&runtime_base, true)?;
    private_directory(&runtime_base, &["d4-pocket", "linux-broker"], 0)
}

pub(crate) fn existing_store_directory() -> io::Result<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or_else(failure)?);
    let home = verify_owned_directory(&home, false)?;
    let state_base = match std::env::var_os("XDG_STATE_HOME") {
        Some(path) => verify_owned_directory(Path::new(&path), false)?,
        None => verify_owned_directory(&home.join(".local").join("state"), false)?,
    };
    let mut state = state_base.join("d4-pocket");
    state = verify_owned_directory(&state, true)?;
    state.push("linux-broker");
    state = verify_owned_directory(&state, true)?;
    let store = verify_owned_directory(&state.join("store"), false)?;
    Ok(store)
}

fn verify_bundle(helper: &Path) -> io::Result<VerifiedBundle> {
    linux_product_bundle::from_helper(helper)
}

fn installed_product_path_verified(
    bundle: &VerifiedBundle,
    identity: Option<&(String, String)>,
    local_app_data: Option<&Path>,
) -> bool {
    let (Some(launcher), Some((app_id, audit_store_id)), Some(local_app_data)) = (
        bundle.versioned_launcher.as_deref(),
        identity,
        local_app_data,
    ) else {
        return false;
    };
    crate::product_bootstrapper::is_active_version_launcher(
        launcher,
        local_app_data,
        app_id,
        audit_store_id,
    )
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

/// Broker state配下の固定Export rootを検査し、Broker専用directory capabilityを返す。
fn open_export_directory(state: &Path) -> io::Result<(PathBuf, Dir)> {
    let export_dir = private_directory(state, &["exports"], 0)?;
    let before = fs::symlink_metadata(&export_dir)?;
    if !before.is_dir()
        || before.file_type().is_symlink()
        || before.uid() != rustix::process::geteuid().as_raw()
        || before.permissions().mode() & 0o077 != 0
    {
        return Err(failure());
    }

    let root =
        Dir::open_ambient_dir(&export_dir, cap_std::ambient_authority()).map_err(|_| failure())?;
    let opened = root.dir_metadata().map_err(|_| failure())?;
    if !opened.is_dir()
        || opened.file_type().is_symlink()
        || cap_fs_ext::MetadataExt::dev(&before) != cap_fs_ext::MetadataExt::dev(&opened)
        || cap_fs_ext::MetadataExt::ino(&before) == 0
        || cap_fs_ext::MetadataExt::ino(&before) != cap_fs_ext::MetadataExt::ino(&opened)
    {
        return Err(failure());
    }
    Ok((export_dir, root))
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
    bundle: &VerifiedBundle,
    launch_nonce: Option<&str>,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> io::Result<()> {
    let _lock = RunLock::acquire(runtime)?;
    let export_root = open_export_directory(state)?;
    let store = private_directory(state, &["store"], 0)?;
    let session = runtime.join("normal-session.json");
    remove_stale_private_file(&session)?;
    remove_stale_private_file(&session.with_extension("json.tmp"))?;
    let shutdown = Arc::new(AtomicBool::new(false));
    let stop = Arc::clone(&shutdown);
    let mut config = BrokerServerConfig::new(store, session.clone());
    config.max_request_bytes = REQUEST_LIMIT;
    config.desktop_package_layout_verified = true;
    let product_identity =
        crate::broker::update_center::embedded_product_identity().map_err(|_| failure())?;
    let product_data_home = crate::broker::product_install::current_user_local_app_data().ok();
    let desktop_install_path_verified = installed_product_path_verified(
        bundle,
        product_identity.as_ref(),
        product_data_home.as_deref(),
    );
    let allow_initial_install_handoff =
        product_identity.is_some() && !desktop_install_path_verified;
    config.desktop_install_path_verified = desktop_install_path_verified;
    config.desktop_product_identity = product_identity;
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (owner_tx, owner_rx) = mpsc::sync_channel(4);
    let server = thread::Builder::new()
        .name("linux-security-broker".into())
        .spawn(move || {
            run_loopback_server_cancellable_with_owner_operations(
                config,
                stop,
                ready_tx,
                owner_rx,
                Some(export_root),
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
        let mut seen_workspace_selection_nonces = BTreeSet::new();
        loop {
            let Some(frame) = read_frame(input, REQUEST_LIMIT)
                .map_err(|error| classify_stage_error(error, 50))?
            else {
                break;
            };
            let frame = Zeroizing::new(frame);
            if let Some(nonce) = launch_nonce {
                crate::linux_desktop_launcher::cancel_product_uninstall_ticket_for_request(
                    &frame, runtime, nonce,
                )
                .map_err(|error| classify_stage_error(error, 51))?;
            }
            let mut confirmation_error = None;
            let selection_response = crate::linux_workspace_selection::dispatch_native(
                &frame,
                &endpoint.0,
                &owner_tx,
                &mut seen_workspace_selection_nonces,
                &mut |challenge| request_workspace_selection(challenge, input, output),
            )
            .map_err(|error| classify_stage_error(error, 52))?;
            let owner_response = if selection_response.is_some() {
                selection_response
            } else {
                let mut ui = LinuxNativeUi { input, output };
                match crate::linux_credential_input::dispatch_native(
                    &frame,
                    &endpoint.0,
                    &owner_tx,
                    &mut seen_credential_nonces,
                    &mut ui,
                )
                .map_err(|error| classify_stage_error(error, 53))?
                {
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
                    )
                    .map_err(|error| classify_stage_error(error, 54))?,
                }
            };
            if let Some(error) = confirmation_error {
                return Err(classify_stage_error(error, 55));
            }
            let mut response = match owner_response {
                Some(response) => response,
                None => {
                    let request = Zeroizing::new(normalize(&frame, &endpoint.0.session_id));
                    relay(&request, &endpoint.0)?
                }
            };
            mark_initial_install_handoff(&frame, &mut response, allow_initial_install_handoff)
                .map_err(|error| classify_stage_error(error, 56))?;
            let sanitized = crate::linux_desktop_launcher::capture_product_uninstall_ticket(
                &mut response,
                runtime,
                launch_nonce,
            )
            .map_err(|error| classify_stage_error(error, 57))?;
            output
                .write_all(&sanitized)
                .and_then(|()| output.write_all(b"\n"))
                .and_then(|()| output.flush())
                .map_err(|error| classify_stage_error(error, 58))?;
        }
        Ok(())
    })();
    shutdown.store(true, Ordering::Release);
    let stopped = match server.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(_)) => Err(classify_stage_error(failure(), 59)),
        Err(_) => Err(classify_stage_error(failure(), 61)),
    };
    let cleanup = if !owned_session.is_empty() {
        match fs::read(&session) {
            Ok(current) => {
                let current = Zeroizing::new(current);
                if *current == *owned_session {
                    fs::remove_file(&session).map_err(|_| classify_stage_error(failure(), 60))
                } else {
                    Err(classify_stage_error(failure(), 60))
                }
            }
            Err(_) => Err(classify_stage_error(failure(), 60)),
        }
    } else {
        Ok(())
    };
    result.and(stopped).and(cleanup)
}

fn request_workspace_selection(
    challenge: &str,
    input: &mut (impl BufRead + ?Sized),
    output: &mut (impl Write + ?Sized),
) -> io::Result<crate::linux_workspace_selection::NativeSelection> {
    let prompt = format!("D4PICK1 {challenge}\n");
    output.write_all(prompt.as_bytes())?;
    output.flush()?;
    let Some(reply) = read_frame(input, WORKSPACE_PICKER_REPLY_LIMIT)? else {
        return Err(failure());
    };
    crate::linux_workspace_selection::parse_picker_reply(challenge, &reply)
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

fn mark_initial_install_handoff(
    request_frame: &[u8],
    response_frame: &mut Vec<u8>,
    allow_initial_install_handoff: bool,
) -> io::Result<()> {
    if !allow_initial_install_handoff {
        return Ok(());
    }
    let Ok(request) = serde_json::from_slice::<serde_json::Value>(request_frame) else {
        return Ok(());
    };
    let Ok(mut response) = serde_json::from_slice::<serde_json::Value>(response_frame.as_slice())
    else {
        return Ok(());
    };
    let Some(request_id) = request
        .get("request_id")
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(());
    };
    if request.get("operation").and_then(serde_json::Value::as_str) != Some("更新有効版切替要求")
        || response
            .get("request_id")
            .and_then(serde_json::Value::as_str)
            != Some(request_id)
        || response
            .get("operation")
            .and_then(serde_json::Value::as_str)
            != Some("更新有効版切替要求")
        || response.get("status").and_then(serde_json::Value::as_str) != Some("accepted")
    {
        return Ok(());
    }
    let Some(body) = response
        .get_mut("body")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return Ok(());
    };
    if body.get("有効化").and_then(serde_json::Value::as_str) != Some("active_version_recorded")
    {
        return Ok(());
    }
    body.insert(
        "起動".to_owned(),
        serde_json::Value::String("after_current_exit".to_owned()),
    );
    *response_frame = serde_json::to_vec(&response)
        .map_err(|_| io::Error::other("Linux初回導入応答を安全に整形できません"))?;
    Ok(())
}

fn relay(request: &[u8], endpoint: &BrokerEndpoint) -> io::Result<Vec<u8>> {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, endpoint.port));
    let mut socket = TcpStream::connect_timeout(&address, IO_TIMEOUT)
        .map_err(|error| classify_stage_error(error, 70))?;
    socket
        .set_read_timeout(Some(relay_response_timeout(request)))
        .map_err(|error| classify_stage_error(error, 71))?;
    socket
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|error| classify_stage_error(error, 72))?;
    socket
        .write_all(endpoint.session_secret.as_bytes())
        .and_then(|()| socket.write_all(b"\n"))
        .map_err(|error| classify_stage_error(error, 73))?;
    socket
        .write_all(request)
        .and_then(|()| socket.write_all(b"\n"))
        .and_then(|()| socket.flush())
        .map_err(|error| classify_stage_error(error, 74))?;
    let response = read_frame(&mut BufReader::new(socket), RESPONSE_LIMIT)
        .map_err(|error| classify_stage_error(error, 75))?;
    response.ok_or_else(|| classify_stage_error(failure(), 76))
}

fn relay_response_timeout(request: &[u8]) -> Duration {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(request) else {
        return IO_TIMEOUT;
    };
    if value.get("operation").and_then(serde_json::Value::as_str) == Some("更新候補取得") {
        CATALOG_RELAY_RESPONSE_TIMEOUT
    } else {
        IO_TIMEOUT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRODUCT_APP_ID: &str = "d4-pocket-app-11111111111111111111111111111111";
    const PRODUCT_AUDIT_ID: &str = "audit-store-22222222222222222222222222222222";

    #[test]
    fn initial_install_handoff_requires_matching_accepted_activation() {
        let request = r#"{"request_id":"request-install","operation":"更新有効版切替要求"}"#;
        let mut response = r#"{"request_id":"request-install","operation":"更新有効版切替要求","status":"accepted","body":{"有効化":"active_version_recorded","起動":"not_started"}}"#.as_bytes().to_vec();
        mark_initial_install_handoff(request.as_bytes(), &mut response, true).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(value["body"]["起動"], "after_current_exit");

        let mut denied = r#"{"request_id":"request-install","operation":"更新有効版切替要求","status":"rejected","body":{"有効化":"active_version_recorded","起動":"not_started"}}"#.as_bytes().to_vec();
        let original = denied.clone();
        mark_initial_install_handoff(request.as_bytes(), &mut denied, true).unwrap();
        assert_eq!(denied, original);

        let mut mismatched = r#"{"request_id":"different-request","operation":"更新有効版切替要求","status":"accepted","body":{"有効化":"active_version_recorded","起動":"not_started"}}"#.as_bytes().to_vec();
        let original = mismatched.clone();
        mark_initial_install_handoff(request.as_bytes(), &mut mismatched, true).unwrap();
        assert_eq!(mismatched, original);

        let mut installed = r#"{"request_id":"request-install","operation":"更新有効版切替要求","status":"accepted","body":{"有効化":"active_version_recorded","起動":"not_started"}}"#.as_bytes().to_vec();
        let original = installed.clone();
        mark_initial_install_handoff(request.as_bytes(), &mut installed, false).unwrap();
        assert_eq!(installed, original);
    }

    fn unique_test_root(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "d4-linux-bundle-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn broker_export_root_is_private_owned_and_rejects_symlinks() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let state = unique_test_root("export-root");
        fs::create_dir(&state).unwrap();
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();

        let (export_path, capability) = open_export_directory(&state).unwrap();
        assert_eq!(export_path, state.join("exports"));
        assert!(capability.dir_metadata().unwrap().is_dir());
        let metadata = fs::symlink_metadata(&export_path).unwrap();
        assert_eq!(metadata.uid(), rustix::process::geteuid().as_raw());
        assert_eq!(metadata.permissions().mode() & 0o077, 0);
        drop(capability);

        fs::set_permissions(&export_path, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(open_export_directory(&state).is_err());
        fs::set_permissions(&export_path, fs::Permissions::from_mode(0o700)).unwrap();
        fs::remove_dir(&export_path).unwrap();

        let outside = unique_test_root("export-outside");
        fs::create_dir(&outside).unwrap();
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o700)).unwrap();
        symlink(&outside, &export_path).unwrap();
        assert!(open_export_directory(&state).is_err());

        fs::remove_file(&export_path).unwrap();
        fs::remove_dir(&outside).unwrap();
        fs::remove_dir(&state).unwrap();
    }

    #[test]
    fn verified_bundle_accepts_only_regular_executable_versioned_launcher() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let root = unique_test_root("launcher");
        let libexec = root.join("libexec");
        fs::create_dir_all(&libexec).unwrap();
        fs::write(root.join("gui_shell_desktop"), b"desktop runner").unwrap();
        let helper = libexec.join("gui_shell_linux_broker");
        fs::write(&helper, b"broker helper").unwrap();
        let launcher = root.join("gui_shell_desktop_launcher");
        fs::write(&launcher, b"versioned launcher").unwrap();
        fs::set_permissions(
            root.join("gui_shell_desktop"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).unwrap();

        assert!(verify_bundle(&helper).unwrap().versioned_launcher.is_some());

        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(verify_bundle(&helper).is_err());
        fs::remove_file(&launcher).unwrap();
        let outside = unique_test_root("outside-launcher");
        fs::write(&outside, b"outside launcher").unwrap();
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o755)).unwrap();
        symlink(&outside, &launcher).unwrap();
        assert!(verify_bundle(&helper).is_err());

        fs::remove_dir_all(&root).unwrap();
        fs::remove_file(outside).unwrap();
    }

    #[test]
    fn installed_path_verification_requires_the_active_version_and_identity() {
        use cap_std::fs::Dir;
        use std::os::unix::fs::PermissionsExt;

        let local_app_data = unique_test_root("installed");
        let product_root = local_app_data
            .join("apps")
            .join("d4-pocket")
            .join(PRODUCT_APP_ID);
        let package_hash = "a".repeat(64);
        let version_root = product_root
            .join("versions")
            .join(format!("1.2.3-{package_hash}"));
        let libexec = version_root.join("libexec");
        fs::create_dir_all(&libexec).unwrap();
        fs::write(version_root.join("gui_shell_desktop"), b"desktop runner").unwrap();
        fs::write(libexec.join("gui_shell_linux_broker"), b"broker helper").unwrap();
        fs::set_permissions(
            version_root.join("gui_shell_desktop"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        fs::set_permissions(
            libexec.join("gui_shell_linux_broker"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        fs::write(
            version_root.join("product_manifest.json"),
            b"product manifest",
        )
        .unwrap();
        let launcher = version_root.join("gui_shell_desktop_launcher");
        fs::write(&launcher, b"versioned launcher").unwrap();
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).unwrap();

        let root_dir = Dir::open_ambient_dir(&product_root, cap_std::ambient_authority()).unwrap();
        let versions_dir =
            Dir::open_ambient_dir(product_root.join("versions"), cap_std::ambient_authority())
                .unwrap();
        crate::product_bootstrapper::activate_staged_version(
            &root_dir,
            &versions_dir,
            "1.2.3",
            PRODUCT_APP_ID,
            PRODUCT_AUDIT_ID,
            &package_hash,
            "update-linux-installed-path",
            &format!("sha256:{}", "c".repeat(64)),
            "0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        drop(versions_dir);
        drop(root_dir);

        let bundle = verify_bundle(&libexec.join("gui_shell_linux_broker")).unwrap();
        let identity = (PRODUCT_APP_ID.to_owned(), PRODUCT_AUDIT_ID.to_owned());
        assert!(installed_product_path_verified(
            &bundle,
            Some(&identity),
            Some(&local_app_data)
        ));
        assert!(!installed_product_path_verified(
            &bundle,
            Some(&identity),
            Some(&local_app_data.join("other"))
        ));
        assert!(!installed_product_path_verified(
            &bundle,
            Some(&(
                PRODUCT_APP_ID.to_owned(),
                "audit-store-33333333333333333333333333333333".into()
            )),
            Some(&local_app_data)
        ));
        assert!(!installed_product_path_verified(
            &bundle,
            None,
            Some(&local_app_data)
        ));

        fs::remove_dir_all(local_app_data).unwrap();
    }

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
    #[ignore = "Linux helperのcatalog relay期限をLinux手動Acceptanceで検証する"]
    fn catalog_relay_wait_matches_broker_fetch_deadline_without_extending_other_operations() {
        assert_eq!(
            CATALOG_RELAY_RESPONSE_TIMEOUT,
            crate::broker::update_download::CATALOG_TIMEOUT + Duration::from_secs(5)
        );
        assert_eq!(
            relay_response_timeout(r#"{"operation":"更新候補取得"}"#.as_bytes()),
            CATALOG_RELAY_RESPONSE_TIMEOUT
        );
        assert_eq!(
            relay_response_timeout(r#"{"operation":"更新候補一覧"}"#.as_bytes()),
            IO_TIMEOUT
        );
        assert_eq!(relay_response_timeout(b"malformed"), IO_TIMEOUT);
    }

    #[test]
    #[ignore = "Linux helperの安全な終了分類をLinux手動Acceptanceで検証する"]
    fn helper_exit_code_exposes_only_a_bounded_io_error_class() {
        assert_eq!(
            helper_exit_code(&io::Error::from(io::ErrorKind::TimedOut)),
            21
        );
        assert_eq!(
            helper_exit_code(&io::Error::from(io::ErrorKind::ConnectionRefused)),
            22
        );
        assert_eq!(
            helper_exit_code(&io::Error::from(io::ErrorKind::UnexpectedEof)),
            25
        );
        assert_eq!(helper_exit_code(&io::Error::other("omitted")), 20);
        assert_eq!(
            helper_exit_code(&io::Error::from(io::ErrorKind::AddrNotAvailable)),
            30
        );
        assert_eq!(
            helper_exit_code(&classify_stage_error(io::Error::other("omitted"), 52)),
            52
        );
        assert_eq!(
            helper_exit_code(&classify_stage_error(io::Error::other("omitted"), 59)),
            59
        );
        assert_eq!(
            helper_exit_code(&classify_stage_error(io::Error::other("omitted"), 76)),
            76
        );
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
