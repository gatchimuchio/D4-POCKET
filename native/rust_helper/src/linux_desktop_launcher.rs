//! Linux installed entrypointから検証済みbundleのFlutter画面を起動する。

use crate::linux_product_bundle::{self, VerifiedBundle};
use rustix::process::{kill_process_group, test_kill_process_group, Pid, Signal};
use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

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
) -> Result<(), LinuxLaunchError> {
    let mut command = Command::new(&bundle.desktop_runner);
    command
        .env_clear()
        .envs(filtered_frontend_environment(inherited))
        .current_dir(&bundle.root)
        .process_group(0);
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

fn dispatch_to_active_version(bundle: &VerifiedBundle) -> Result<(), LinuxLaunchError> {
    let launcher = bundle
        .versioned_launcher
        .as_deref()
        .ok_or_else(|| failure("D4 Pocketの有効版launcherがありません。"))?;
    let mut command = Command::new(launcher);
    let status = command
        .env_clear()
        .envs(filtered_frontend_environment(std::env::vars_os()))
        .current_dir(&bundle.root)
        .status()
        .map_err(|_| failure("D4 Pocketの有効版を起動できません。"))?;
    if !status.success() {
        return Err(failure("D4 Pocketの有効版が正常に終了しませんでした。"));
    }
    Ok(())
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
        LaunchPlan::DispatchToActiveVersion(bundle) => dispatch_to_active_version(&bundle),
        LaunchPlan::RunBundle(bundle) => run_frontend(&bundle, std::env::vars_os()),
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

        assert!(run_frontend(&bundle, std::iter::empty()).is_ok());
        fs::remove_dir_all(root).unwrap();
    }
}
