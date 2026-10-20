//! Linux installed entrypointから検証済みbundleのFlutter画面を起動する。

use crate::linux_product_bundle::{self, VerifiedBundle};
use rustix::fs::{flock, FlockOperation};
use rustix::process::{kill_process_group, test_kill_process_group, Pid, Signal};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};
use zeroize::{Zeroize, Zeroizing};

pub(crate) const LAUNCH_NONCE_ENV: &str = "D4P_LAUNCH_NONCE";
const UNINSTALL_TICKET_PREFIX: &str = "product-uninstall-";

const FRONTEND_ENVIRONMENT_ALLOWLIST: &[&str] = &[
    "HOME",
    "XDG_DATA_HOME",
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
    "XDG_STATE_HOME",
    "XDG_RUNTIME_DIR",
    "XDG_DATA_DIRS",
    "DISPLAY",
    "WAYLAND_DISPLAY",
    "XAUTHORITY",
    "DBUS_SESSION_BUS_ADDRESS",
    "XDG_SESSION_TYPE",
    "XDG_CURRENT_DESKTOP",
    "XDG_SESSION_DESKTOP",
    "GDK_BACKEND",
    "LANG",
    "LANGUAGE",
    "LC_ALL",
    "LC_CTYPE",
    "LC_MESSAGES",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxLaunchError(&'static str);

impl std::fmt::Display for LinuxLaunchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for LinuxLaunchError {}

fn failure(message: &'static str) -> LinuxLaunchError {
    LinuxLaunchError(message)
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn uninstall_ticket_path(runtime: &Path, launch_nonce: &str) -> Option<PathBuf> {
    valid_hex(launch_nonce, 32)
        .then(|| runtime.join(format!("{UNINSTALL_TICKET_PREFIX}{launch_nonce}.ticket")))
}

/// Broker応答に含まれる一回ticketをRust専用handoffへ移し、Flutter応答から除去する。
pub(crate) fn capture_product_uninstall_ticket(
    response: &mut Vec<u8>,
    runtime: &Path,
    launch_nonce: Option<&str>,
) -> std::io::Result<Vec<u8>> {
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(response) else {
        return Ok(response.clone());
    };
    if value.get("operation").and_then(serde_json::Value::as_str)
        != Some("製品アンインストール要求")
        || value.get("status").and_then(serde_json::Value::as_str) != Some("accepted")
    {
        return Ok(response.clone());
    }
    let result = (|| {
        let nonce = launch_nonce
            .filter(|nonce| valid_hex(nonce, 32))
            .ok_or_else(|| std::io::Error::other("製品削除後処理のlaunch bindingがありません"))?;
        let path = uninstall_ticket_path(runtime, nonce)
            .ok_or_else(|| std::io::Error::other("製品削除後処理のlaunch bindingが不正です"))?;
        let body = value
            .get_mut("body")
            .and_then(serde_json::Value::as_object_mut)
            .ok_or_else(|| std::io::Error::other("Brokerの製品削除応答bodyが不正です"))?;
        let authorization_audit_id = body
            .get("認可監査ID")
            .and_then(serde_json::Value::as_str)
            .filter(|id| {
                !id.is_empty()
                    && id.len() <= 128
                    && id
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:".contains(&byte))
            })
            .ok_or_else(|| std::io::Error::other("製品削除の認可Audit IDが不正です"))?;
        let _ = authorization_audit_id;
        let ticket = body
            .get_mut("ticket")
            .and_then(|ticket| ticket.as_str())
            .filter(|ticket| valid_hex(ticket, 64))
            .map(|ticket| Zeroizing::new(ticket.to_owned()))
            .ok_or_else(|| std::io::Error::other("Brokerの製品削除ticketが不正です"))?;

        let runtime_metadata = fs::symlink_metadata(runtime)?;
        if !runtime_metadata.is_dir()
            || runtime_metadata.file_type().is_symlink()
            || runtime_metadata.uid() != rustix::process::geteuid().as_raw()
            || runtime_metadata.permissions().mode() & 0o077 != 0
        {
            return Err(std::io::Error::other(
                "製品削除handoff directoryがprivateではありません",
            ));
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(&path)?;
        let write_result = file
            .write_all(ticket.as_bytes())
            .and_then(|()| file.sync_all());
        if let Err(error) = write_result {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(error);
        }
        let metadata = file.metadata()?;
        if !metadata.is_file()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.nlink() != 1
            || metadata.len() != 64
            || metadata.permissions().mode() & 0o077 != 0
        {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(std::io::Error::other("製品削除ticket handoffが不正です"));
        }
        drop(file);

        if let Some(body) = value
            .get_mut("body")
            .and_then(serde_json::Value::as_object_mut)
        {
            if let Some(serde_json::Value::String(stored_ticket)) = body.get_mut("ticket") {
                stored_ticket.zeroize();
            }
            body.remove("ticket");
        }
        let sanitized = serde_json::to_vec(&value)
            .map_err(|_| std::io::Error::other("製品削除応答を安全に整形できません"))?;
        Ok(sanitized)
    })();
    response.zeroize();
    result
}

fn take_product_uninstall_ticket(
    runtime: &Path,
    launch_nonce: &str,
) -> Result<Option<Zeroizing<String>>, LinuxLaunchError> {
    let path = uninstall_ticket_path(runtime, launch_nonce)
        .ok_or_else(|| failure("製品削除後処理のlaunch bindingが不正です。"))?;
    let mut file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(&path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(failure("製品削除ticketを安全に開けません。")),
    };
    let before = file
        .metadata()
        .map_err(|_| failure("製品削除ticketを検査できません。"))?;
    if !before.is_file()
        || before.uid() != rustix::process::geteuid().as_raw()
        || before.nlink() != 1
        || before.len() != 64
        || before.permissions().mode() & 0o077 != 0
    {
        return Err(failure("製品削除ticket handoffが不正です。"));
    }
    let mut bytes = Zeroizing::new(Vec::with_capacity(64));
    (&mut file)
        .take(65)
        .read_to_end(&mut bytes)
        .map_err(|_| failure("製品削除ticketを読み取れません。"))?;
    if bytes.len() != 64 || !bytes.iter().all(u8::is_ascii_hexdigit) {
        return Err(failure("製品削除ticketの内容が不正です。"));
    }
    let ticket =
        std::str::from_utf8(&bytes).map_err(|_| failure("製品削除ticketを復号できません。"))?;
    if !valid_hex(ticket, 64) {
        return Err(failure("製品削除ticketの形式が不正です。"));
    }
    let current = fs::symlink_metadata(&path)
        .map_err(|_| failure("製品削除ticketのidentityが変化しました。"))?;
    if current.dev() != before.dev() || current.ino() != before.ino() || !current.is_file() {
        return Err(failure("製品削除ticketのidentityが変化しました。"));
    }
    fs::remove_file(&path).map_err(|_| failure("製品削除ticketを消費できません。"))?;
    Ok(Some(Zeroizing::new(ticket.to_owned())))
}

pub(crate) fn cancel_product_uninstall_ticket(
    runtime: &Path,
    launch_nonce: &str,
) -> std::io::Result<()> {
    take_product_uninstall_ticket(runtime, launch_nonce)
        .map(|ticket| drop(ticket))
        .map_err(|_| std::io::Error::other("アンインストール取消時に保有ticketを破棄できません"))
}

pub(crate) fn cancel_product_uninstall_ticket_for_request(
    frame: &[u8],
    runtime: &Path,
    launch_nonce: &str,
) -> std::io::Result<bool> {
    let is_cancel = serde_json::from_slice::<serde_json::Value>(frame)
        .ok()
        .and_then(|value| {
            value
                .get("operation")
                .and_then(serde_json::Value::as_str)
                .map(|operation| operation == "製品アンインストール取消要求")
        })
        .unwrap_or(false);
    if is_cancel {
        cancel_product_uninstall_ticket(runtime, launch_nonce)?;
    }
    Ok(is_cancel)
}

struct ProductLauncherLock {
    _file: File,
}

impl ProductLauncherLock {
    fn acquire(runtime: &Path) -> Result<Self, LinuxLaunchError> {
        let path = runtime.join("product-launcher.lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(path)
            .map_err(|_| failure("D4 Pocketの起動状態を安全に固定できません。"))?;
        let metadata = file
            .metadata()
            .map_err(|_| failure("D4 Pocketの起動状態を確認できません。"))?;
        if !metadata.is_file()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.nlink() != 1
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(failure("D4 Pocketの起動lockが不正です。"));
        }
        flock(&file, FlockOperation::NonBlockingLockExclusive)
            .map_err(|_| failure("D4 Pocketはすでに実行中です。"))?;
        Ok(Self { _file: file })
    }
}

enum LaunchPlan {
    DispatchToActiveVersion(VerifiedBundle),
    RunBundle(VerifiedBundle),
}

fn resolve_launch_plan(
    current_launcher: &Path,
    identity: Option<&(String, String)>,
    product_data_home: Option<&Path>,
) -> Result<LaunchPlan, LinuxLaunchError> {
    let Some((app_id, audit_store_id)) = identity else {
        return linux_product_bundle::from_launcher(current_launcher)
            .map(LaunchPlan::RunBundle)
            .map_err(|_| failure("製品bundleの起動fileを確認できません。再導入してください。"));
    };
    let data_home =
        product_data_home.ok_or_else(|| failure("製品の利用者保存領域を確認できません。"))?;
    if crate::product_bootstrapper::is_fixed_installed_entrypoint(
        current_launcher,
        data_home,
        app_id,
    ) {
        let active_launcher = crate::product_bootstrapper::resolve_active_version_launcher(
            data_home,
            app_id,
            audit_store_id,
        )
        .map_err(|_| failure("D4 Pocketの有効版を確認できません。導入状態を確認してください。"))?;
        let bundle = linux_product_bundle::from_launcher(&active_launcher)
            .map_err(|_| failure("D4 Pocketの有効版bundleが不正です。"))?;
        return Ok(LaunchPlan::DispatchToActiveVersion(bundle));
    }
    if !crate::product_bootstrapper::is_active_version_launcher(
        current_launcher,
        data_home,
        app_id,
        audit_store_id,
    ) {
        return Err(failure(
            "このLinux製品launcherは現在の有効版ではありません。",
        ));
    }
    linux_product_bundle::from_launcher(current_launcher)
        .map(LaunchPlan::RunBundle)
        .map_err(|_| failure("製品bundleの起動fileを確認できません。再導入してください。"))
}

fn filtered_frontend_environment(
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
) -> Vec<(OsString, OsString)> {
    inherited
        .into_iter()
        .filter(|(name, _)| {
            name.to_str().is_some_and(|name| {
                FRONTEND_ENVIRONMENT_ALLOWLIST
                    .iter()
                    .any(|allowed| name == *allowed)
            })
        })
        .collect()
}

fn run_frontend(
    bundle: &VerifiedBundle,
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
    launch_nonce: Option<&str>,
) -> Result<(), LinuxLaunchError> {
    let mut command = Command::new(&bundle.desktop_runner);
    command
        .env_clear()
        .envs(filtered_frontend_environment(inherited))
        .current_dir(&bundle.root)
        .process_group(0);
    if let Some(launch_nonce) = launch_nonce {
        if !valid_hex(launch_nonce, 32) {
            return Err(failure("D4 Pocketの起動相関値が不正です。"));
        }
        command.env(LAUNCH_NONCE_ENV, launch_nonce);
    }
    let mut child = command
        .spawn()
        .map_err(|_| failure("D4 Pocketの画面を起動できません。"))?;
    let process_group = checked_process_group(&mut child)?;
    let status = child.wait();
    let cleanup = stop_owned_process_group(process_group);
    let status = status.map_err(|_| failure("D4 Pocketの終了状態を確認できません。"))?;
    cleanup?;
    if !status.success() {
        return Err(failure("D4 Pocketが正常に終了しませんでした。"));
    }
    Ok(())
}

fn checked_process_group(child: &mut Child) -> Result<Pid, LinuxLaunchError> {
    let group = i32::try_from(child.id())
        .ok()
        .and_then(Pid::from_raw)
        .filter(|group| *group != rustix::process::getpgrp());
    if let Some(group) = group {
        return Ok(group);
    }
    let _ = child.kill();
    let _ = child.wait();
    Err(failure("D4 Pocketの終了監督を開始できません。"))
}

fn stop_owned_process_group(group: Pid) -> Result<(), LinuxLaunchError> {
    if wait_for_group_exit(group, Duration::from_secs(8))? {
        return Ok(());
    }
    match kill_process_group(group, Signal::TERM) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => {}
        Err(_) => return Err(failure("D4 Pocketの子processを停止できません。")),
    }
    if wait_for_group_exit(group, Duration::from_secs(2))? {
        return Ok(());
    }
    match kill_process_group(group, Signal::KILL) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => {}
        Err(_) => return Err(failure("D4 Pocketの子processを停止できません。")),
    }
    if wait_for_group_exit(group, Duration::from_secs(5))? {
        Ok(())
    } else {
        Err(failure("D4 Pocketの子process停止を確認できません。"))
    }
}

fn wait_for_group_exit(group: Pid, timeout: Duration) -> Result<bool, LinuxLaunchError> {
    let started = Instant::now();
    loop {
        match test_kill_process_group(group) {
            Err(rustix::io::Errno::SRCH) => return Ok(true),
            Ok(()) if started.elapsed() >= timeout => return Ok(false),
            Ok(()) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => return Err(failure("D4 Pocketの子process状態を確認できません。")),
        }
    }
}

fn dispatch_to_active_version(
    bundle: &VerifiedBundle,
    launch_nonce: &str,
) -> Result<(), LinuxLaunchError> {
    let launcher = bundle
        .versioned_launcher
        .as_deref()
        .ok_or_else(|| failure("D4 Pocketの有効版launcherがありません。"))?;
    let mut command = Command::new(launcher);
    let status = command
        .env_clear()
        .envs(filtered_frontend_environment(std::env::vars_os()))
        .env(LAUNCH_NONCE_ENV, launch_nonce)
        .current_dir(&bundle.root)
        .status()
        .map_err(|_| failure("D4 Pocketの有効版を起動できません。"))?;
    if !status.success() {
        return Err(failure("D4 Pocketの有効版が正常に終了しませんでした。"));
    }
    Ok(())
}

fn verify_existing_linux_audit_store(store: &Path) -> Result<(), LinuxLaunchError> {
    let metadata = fs::symlink_metadata(store)
        .map_err(|_| failure("既存Audit Storeを確認できません。製品を削除しません。"))?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o022 != 0
    {
        return Err(failure("既存Audit Storeの安全性を確認できません。"));
    }
    for name in ["audit.jsonl", "audit_anchor.json", "audit_anchor.key"] {
        let path = store.join(name);
        let file = fs::symlink_metadata(path)
            .map_err(|_| failure("Audit chainの必須fileがありません。製品を削除しません。"))?;
        if !file.is_file()
            || file.file_type().is_symlink()
            || file.uid() != rustix::process::geteuid().as_raw()
            || file.nlink() != 1
            || file.permissions().mode() & 0o022 != 0
            || (name == "audit_anchor.key" && file.permissions().mode() & 0o077 != 0)
        {
            return Err(failure("既存Audit Storeのfile検証に失敗しました。"));
        }
    }
    Ok(())
}

fn verify_audit_store_outside_product_root(
    data_home: &Path,
    app_id: &str,
    store: &Path,
) -> Result<(), LinuxLaunchError> {
    let product_root = crate::broker::product_install::product_install_root(data_home, app_id)
        .map_err(|_| failure("固定製品rootを導出できません。"))?;
    let product_root =
        fs::canonicalize(product_root).map_err(|_| failure("固定製品rootを確認できません。"))?;
    if store.starts_with(&product_root) {
        return Err(failure(
            "Audit Storeが製品root内にあるため製品を削除しません。",
        ));
    }
    Ok(())
}

fn finalize_linux_product_uninstall(
    data_home: &Path,
    identity: &(String, String),
    runtime: &Path,
    launch_nonce: &str,
) -> Result<(), LinuxLaunchError> {
    let Some(ticket) = take_product_uninstall_ticket(runtime, launch_nonce)? else {
        return Ok(());
    };
    let home = PathBuf::from(
        std::env::var_os("HOME").ok_or_else(|| failure("利用者homeを確認できません。"))?,
    );
    let home_metadata =
        fs::symlink_metadata(&home).map_err(|_| failure("利用者homeを確認できません。"))?;
    if !home_metadata.is_dir()
        || home_metadata.file_type().is_symlink()
        || home_metadata.uid() != rustix::process::geteuid().as_raw()
    {
        return Err(failure("利用者homeの安全性を確認できません。"));
    }
    std::env::set_current_dir(&home)
        .map_err(|_| failure("製品root外へ作業directoryを移動できません。"))?;

    let store = crate::linux_desktop_worker::existing_store_directory()
        .map_err(|_| failure("既存Audit Storeを確認できません。製品を削除しません。"))?;
    crate::broker::product_install::validate_linux_product_uninstall_targets(
        data_home,
        &identity.0,
    )
    .map_err(|_| failure("固定製品rootを安全に確認できません。製品を削除しません。"))?;
    verify_audit_store_outside_product_root(data_home, &identity.0, &store)?;
    verify_existing_linux_audit_store(&store)?;
    let mut broker = crate::broker::Broker::new_persistent(
        &format!("uninstall-finalizer-{}", std::process::id()),
        &store,
    )
    .map_err(|_| failure("Audit chainを検証できません。製品を削除しません。"))?;
    broker
        .set_desktop_product_identity(identity.0.clone(), identity.1.clone())
        .map_err(|_| failure("製品identityを検証できません。製品を削除しません。"))?;
    let ticket_hash = broker
        .begin_product_uninstall_finalization(&ticket)
        .map_err(|_| failure("削除ticketが無効または使用済みです。製品は変更していません。"))?;
    match crate::broker::product_install::remove_installed_linux_product(data_home, &identity.0) {
        Ok(()) => broker
            .finish_product_uninstall_finalization(&ticket_hash, true, None)
            .map_err(|_| {
                failure("製品削除後の完了Auditを確定できません。保持dataを確認してください。")
            }),
        Err(error) => {
            let _ =
                broker.finish_product_uninstall_finalization(&ticket_hash, false, Some(error.0));
            Err(failure(
                "固定製品rootまたはdesktop entryを安全に削除できません。",
            ))
        }
    }
}

pub fn run() -> Result<(), LinuxLaunchError> {
    if std::env::args_os().len() != 1 {
        return Err(failure(
            "この起動方法では追加のcommandやpathを指定できません。",
        ));
    }
    let launcher =
        std::env::current_exe().map_err(|_| failure("起動元の製品launcherを確認できません。"))?;
    let identity = crate::broker::update_center::embedded_product_identity()
        .map_err(|_| failure("製品identityを確認できません。"))?;
    let data_home = if identity.is_some() {
        Some(
            crate::broker::product_install::current_user_local_app_data()
                .map_err(|_| failure("製品の利用者保存領域を確認できません。"))?,
        )
    } else {
        None
    };
    let plan = resolve_launch_plan(&launcher, identity.as_ref(), data_home.as_deref())?;
    match plan {
        LaunchPlan::DispatchToActiveVersion(bundle) => {
            let runtime = crate::linux_desktop_worker::prepare_runtime_directory()
                .map_err(|_| failure("製品runtime directoryを安全に準備できません。"))?;
            let _launcher_lock = ProductLauncherLock::acquire(&runtime)?;
            let mut nonce_bytes = [0u8; 16];
            getrandom::getrandom(&mut nonce_bytes)
                .map_err(|_| failure("製品起動の相関値を生成できません。"))?;
            let launch_nonce = hex::encode(nonce_bytes);
            nonce_bytes.zeroize();
            let launch_result = dispatch_to_active_version(&bundle, &launch_nonce);
            if let Err(error) = launch_result {
                let ticket = take_product_uninstall_ticket(&runtime, &launch_nonce);
                if ticket.is_err() {
                    return Err(failure("終了後削除の未確定ticketを安全に回収できません。"));
                }
                return Err(error);
            }
            if let Some(identity) = identity.as_ref() {
                finalize_linux_product_uninstall(
                    data_home
                        .as_deref()
                        .ok_or_else(|| failure("製品の利用者保存領域を確認できません。"))?,
                    identity,
                    &runtime,
                    &launch_nonce,
                )?;
            }
            Ok(())
        }
        LaunchPlan::RunBundle(bundle) => {
            let launch_nonce = std::env::var(LAUNCH_NONCE_ENV).ok();
            run_frontend(&bundle, std::env::vars_os(), launch_nonce.as_deref())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cap_std::fs::Dir;
    use std::ffi::OsStr;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    const APP_ID: &str = "d4-pocket-app-11111111111111111111111111111111";
    const AUDIT_ID: &str = "audit-store-22222222222222222222222222222222";

    fn unique_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "d4-linux-launcher-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn write_executable(path: &Path, content: &[u8]) {
        fs::write(path, content).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn write_bundle(root: &Path) {
        fs::create_dir_all(root.join("libexec")).unwrap();
        write_executable(&root.join("gui_shell_desktop"), b"#!/bin/sh\nexit 0\n");
        write_executable(
            &root.join("libexec/gui_shell_linux_broker"),
            b"#! /bin/sh\nexit 0\n",
        );
        write_executable(
            &root.join("gui_shell_desktop_launcher"),
            b"#! /bin/sh\nexit 0\n",
        );
        fs::write(root.join("product_manifest.json"), b"product manifest").unwrap();
    }

    #[test]
    fn development_launcher_accepts_only_its_adjacent_bundle() {
        let root = unique_root("development");
        write_bundle(&root);
        let plan =
            resolve_launch_plan(&root.join("gui_shell_desktop_launcher"), None, None).unwrap();
        assert!(matches!(plan, LaunchPlan::RunBundle(_)));
        assert!(resolve_launch_plan(&root.join("elsewhere"), None, None).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn installed_root_launcher_dispatches_only_to_the_recorded_active_bundle() {
        let local_app_data = unique_root("installed");
        let product_root = local_app_data.join("apps").join("d4-pocket").join(APP_ID);
        let package_hash = "a".repeat(64);
        let version_root = product_root
            .join("versions")
            .join(format!("1.2.3-{package_hash}"));
        write_bundle(&version_root);

        let root_dir = Dir::open_ambient_dir(&product_root, cap_std::ambient_authority()).unwrap();
        let versions_dir =
            Dir::open_ambient_dir(product_root.join("versions"), cap_std::ambient_authority())
                .unwrap();
        crate::product_bootstrapper::activate_staged_version(
            &root_dir,
            &versions_dir,
            "1.2.3",
            APP_ID,
            AUDIT_ID,
            &package_hash,
            "update-linux-launcher",
            &format!("sha256:{}", "c".repeat(64)),
            "0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        drop(versions_dir);
        drop(root_dir);

        let identity = (APP_ID.to_owned(), AUDIT_ID.to_owned());
        let plan = resolve_launch_plan(
            &product_root.join("gui_shell_desktop_launcher"),
            Some(&identity),
            Some(&local_app_data),
        )
        .unwrap();
        assert!(matches!(plan, LaunchPlan::DispatchToActiveVersion(_)));
        assert!(resolve_launch_plan(
            &product_root.join("gui_shell_desktop_launcher"),
            Some(&(
                APP_ID.to_owned(),
                "audit-store-33333333333333333333333333333333".into()
            )),
            Some(&local_app_data),
        )
        .is_err());
        fs::remove_dir_all(local_app_data).unwrap();
    }

    #[test]
    fn active_version_must_match_identity_and_record() {
        let local_app_data = unique_root("active");
        let product_root = local_app_data.join("apps").join("d4-pocket").join(APP_ID);
        let package_hash = "b".repeat(64);
        let version_root = product_root
            .join("versions")
            .join(format!("1.2.3-{package_hash}"));
        write_bundle(&version_root);
        let root_dir = Dir::open_ambient_dir(&product_root, cap_std::ambient_authority()).unwrap();
        let versions_dir =
            Dir::open_ambient_dir(product_root.join("versions"), cap_std::ambient_authority())
                .unwrap();
        crate::product_bootstrapper::activate_staged_version(
            &root_dir,
            &versions_dir,
            "1.2.3",
            APP_ID,
            AUDIT_ID,
            &package_hash,
            "update-linux-launcher-active",
            &format!("sha256:{}", "d".repeat(64)),
            "fedcba9876543210fedcba9876543210",
        )
        .unwrap();
        drop(versions_dir);
        drop(root_dir);
        let identity = (APP_ID.to_owned(), AUDIT_ID.to_owned());
        let active_launcher = version_root.join("gui_shell_desktop_launcher");
        assert!(matches!(
            resolve_launch_plan(&active_launcher, Some(&identity), Some(&local_app_data)),
            Ok(LaunchPlan::RunBundle(_))
        ));
        let other_root = unique_root("other-data");
        fs::create_dir_all(&other_root).unwrap();
        assert!(
            resolve_launch_plan(&active_launcher, Some(&identity), Some(&other_root),).is_err()
        );
        fs::remove_dir_all(other_root).unwrap();
        fs::remove_dir_all(local_app_data).unwrap();
    }

    #[test]
    fn frontend_environment_drops_credentials_and_loader_overrides() {
        let selected = filtered_frontend_environment([
            (OsString::from("HOME"), OsString::from("/home/owner")),
            (
                OsString::from("XDG_DATA_HOME"),
                OsString::from("/home/owner/data"),
            ),
            (
                OsString::from("WAYLAND_DISPLAY"),
                OsString::from("wayland-0"),
            ),
            (OsString::from("LANG"), OsString::from("ja_JP.UTF-8")),
            (
                OsString::from("OPENAI_API_KEY"),
                OsString::from("must-not-pass"),
            ),
            (
                OsString::from("LD_PRELOAD"),
                OsString::from("/tmp/inject.so"),
            ),
            (OsString::from("PATH"), OsString::from("/tmp/inject-bin")),
        ]);
        assert!(selected.iter().any(|(name, value)| {
            name == OsStr::new("XDG_DATA_HOME") && value == OsStr::new("/home/owner/data")
        }));
        assert!(selected
            .iter()
            .any(|(name, _)| name == OsStr::new("WAYLAND_DISPLAY")));
        assert!(selected.iter().any(|(name, _)| name == OsStr::new("LANG")));
        for forbidden in ["OPENAI_API_KEY", "LD_PRELOAD", "PATH"] {
            assert!(!selected
                .iter()
                .any(|(name, _)| name == OsStr::new(forbidden)));
        }
        assert!(!selected
            .iter()
            .any(|(name, _)| name == OsStr::new(LAUNCH_NONCE_ENV)));
    }

    #[test]
    fn frontend_exit_stops_a_remaining_owned_process_group() {
        let root = unique_root("descendant-cleanup");
        fs::create_dir_all(&root).unwrap();
        let runner = root.join("gui_shell_desktop");
        write_executable(&runner, b"#!/bin/sh\nsleep 30 &\nexit 0\n");
        let bundle = VerifiedBundle {
            root: root.clone(),
            desktop_runner: runner,
            broker_helper: root.join("libexec/gui_shell_linux_broker"),
            versioned_launcher: None,
        };

        assert!(run_frontend(&bundle, std::iter::empty(), None).is_ok());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn uninstall_ticket_is_removed_from_flutter_response_and_consumed_once() {
        use std::os::unix::fs::PermissionsExt;

        let runtime = unique_root("uninstall-handoff");
        fs::create_dir_all(&runtime).unwrap();
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).unwrap();
        let nonce = "a".repeat(32);
        let ticket = "b".repeat(64);
        let mut response = serde_json::to_vec(&serde_json::json!({
            "request_id": "uninstall-request",
            "operation": "製品アンインストール要求",
            "status": "accepted",
            "body": {"版": 1, "状態": "uninstall_authorized", "認可監査ID": "broker-audit-uninstall", "ticket": ticket}
        }))
        .unwrap();

        let sanitized =
            capture_product_uninstall_ticket(&mut response, &runtime, Some(&nonce)).unwrap();
        assert!(response.iter().all(|byte| *byte == 0));
        assert!(!String::from_utf8_lossy(&sanitized).contains(&ticket));
        let path = uninstall_ticket_path(&runtime, &nonce).unwrap();
        let metadata = fs::symlink_metadata(&path).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        assert_eq!(fs::read(&path).unwrap(), ticket.as_bytes());
        assert_eq!(
            take_product_uninstall_ticket(&runtime, &nonce)
                .unwrap()
                .as_deref()
                .map(String::as_str),
            Some(ticket.as_str())
        );
        assert!(take_product_uninstall_ticket(&runtime, &nonce)
            .unwrap()
            .is_none());
        fs::remove_dir_all(runtime).unwrap();
    }

    #[test]
    fn uninstall_ticket_handoff_fails_closed_without_launch_binding() {
        let runtime = unique_root("uninstall-no-binding");
        fs::create_dir_all(&runtime).unwrap();
        let ticket = "c".repeat(64);
        let mut response = serde_json::to_vec(&serde_json::json!({
            "operation": "製品アンインストール要求",
            "status": "accepted",
            "body": {"ticket": ticket}
        }))
        .unwrap();
        assert!(capture_product_uninstall_ticket(&mut response, &runtime, None).is_err());
        assert!(fs::read_dir(&runtime).unwrap().next().is_none());
        fs::remove_dir_all(runtime).unwrap();
    }

    #[test]
    fn uninstall_cancel_request_discards_only_the_current_launch_ticket() {
        use std::os::unix::fs::PermissionsExt;

        let runtime = unique_root("uninstall-cancel-handoff");
        fs::create_dir_all(&runtime).unwrap();
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).unwrap();
        let nonce = "d".repeat(32);
        let ticket = "e".repeat(64);
        let mut response = serde_json::to_vec(&serde_json::json!({
            "operation": "製品アンインストール要求",
            "status": "accepted",
            "body": {"版": 1, "認可監査ID": "broker-audit-uninstall-cancel", "ticket": ticket}
        }))
        .unwrap();
        let _ = capture_product_uninstall_ticket(&mut response, &runtime, Some(&nonce)).unwrap();

        let unrelated = r#"{"operation":"更新一覧"}"#.as_bytes();
        assert!(!cancel_product_uninstall_ticket_for_request(unrelated, &runtime, &nonce).unwrap());
        assert!(uninstall_ticket_path(&runtime, &nonce).unwrap().exists());
        let cancellation = r#"{"operation":"製品アンインストール取消要求"}"#.as_bytes();
        assert!(
            cancel_product_uninstall_ticket_for_request(cancellation, &runtime, &nonce).unwrap()
        );
        assert!(!uninstall_ticket_path(&runtime, &nonce).unwrap().exists());
        assert!(take_product_uninstall_ticket(&runtime, &nonce)
            .unwrap()
            .is_none());
        fs::remove_dir_all(runtime).unwrap();
    }

    #[test]
    fn uninstall_refuses_to_remove_audit_store_nested_in_product_root() {
        let root = unique_root("uninstall-audit-boundary");
        let data_home = root.join("xdg-data");
        let product_root = data_home.join("apps/d4-pocket").join(APP_ID);
        let store = product_root.join("state/linux-broker/store");
        fs::create_dir_all(&store).unwrap();
        assert!(verify_audit_store_outside_product_root(&data_home, APP_ID, &store).is_err());

        let retained_store = root.join("xdg-state/d4-pocket/linux-broker/store");
        fs::create_dir_all(&retained_store).unwrap();
        assert!(
            verify_audit_store_outside_product_root(&data_home, APP_ID, &retained_store).is_ok()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
