//! Linux Desktop bundleの固定file配置を検証する。

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedBundle {
    pub(crate) root: PathBuf,
    pub(crate) desktop_runner: PathBuf,
    pub(crate) broker_helper: PathBuf,
    pub(crate) versioned_launcher: Option<PathBuf>,
}

fn failure() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Linux製品bundle配置を確認できません",
    )
}

pub(crate) fn from_helper(helper: &Path) -> io::Result<VerifiedBundle> {
    if !helper.is_absolute()
        || helper.file_name().and_then(|value| value.to_str()) != Some("gui_shell_linux_broker")
    {
        return Err(failure());
    }
    let libexec = helper.parent().ok_or_else(failure)?;
    let root = libexec.parent().ok_or_else(failure)?;
    if libexec.file_name().and_then(|value| value.to_str()) != Some("libexec") {
        return Err(failure());
    }
    let bundle = verify_root(root)?;
    if fs::canonicalize(helper)? != bundle.broker_helper {
        return Err(failure());
    }
    Ok(bundle)
}

pub(crate) fn from_launcher(launcher: &Path) -> io::Result<VerifiedBundle> {
    if !launcher.is_absolute()
        || launcher.file_name().and_then(|value| value.to_str())
            != Some("gui_shell_desktop_launcher")
    {
        return Err(failure());
    }
    let root = launcher.parent().ok_or_else(failure)?;
    let bundle = verify_root(root)?;
    let canonical_launcher = fs::canonicalize(launcher)?;
    if bundle.versioned_launcher.as_deref() != Some(canonical_launcher.as_path()) {
        return Err(failure());
    }
    Ok(bundle)
}

fn verify_root(root: &Path) -> io::Result<VerifiedBundle> {
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(failure());
    }
    let root = fs::canonicalize(root)?;
    let desktop_runner = checked_executable(&root, "gui_shell_desktop")?;
    let broker_helper = checked_executable(&root, "libexec/gui_shell_linux_broker")?;
    let launcher_path = root.join("gui_shell_desktop_launcher");
    let versioned_launcher = match fs::symlink_metadata(&launcher_path) {
        Ok(_) => Some(checked_executable(&root, "gui_shell_desktop_launcher")?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    Ok(VerifiedBundle {
        root,
        desktop_runner,
        broker_helper,
        versioned_launcher,
    })
}

fn checked_executable(root: &Path, relative: &str) -> io::Result<PathBuf> {
    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o111 == 0
    {
        return Err(failure());
    }
    let path = fs::canonicalize(path)?;
    if !path.starts_with(root) {
        return Err(failure());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "d4-linux-bundle-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("libexec")).unwrap();
        for path in [
            root.join("gui_shell_desktop"),
            root.join("libexec/gui_shell_linux_broker"),
            root.join("gui_shell_desktop_launcher"),
        ] {
            fs::write(&path, b"fixture executable").unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        root
    }

    #[test]
    fn bundle_verification_accepts_fixed_executable_layout() {
        let root = fixture("valid");
        let bundle = from_helper(&root.join("libexec/gui_shell_linux_broker")).unwrap();
        assert_eq!(bundle.root, fs::canonicalize(&root).unwrap());
        assert_eq!(
            bundle.desktop_runner,
            fs::canonicalize(root.join("gui_shell_desktop")).unwrap()
        );
        assert_eq!(
            bundle.versioned_launcher,
            Some(fs::canonicalize(root.join("gui_shell_desktop_launcher")).unwrap())
        );
        assert_eq!(
            from_launcher(&root.join("gui_shell_desktop_launcher")).unwrap(),
            bundle
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_verification_rejects_non_executable_and_symlink_entrypoints() {
        let root = fixture("negative");
        let runner = root.join("gui_shell_desktop");
        fs::set_permissions(&runner, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(from_helper(&root.join("libexec/gui_shell_linux_broker")).is_err());

        fs::remove_file(&runner).unwrap();
        let outside = root.parent().unwrap().join(format!(
            "d4-linux-bundle-outside-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&outside, b"outside runner").unwrap();
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o755)).unwrap();
        symlink(&outside, &runner).unwrap();
        assert!(from_helper(&root.join("libexec/gui_shell_linux_broker")).is_err());
        fs::remove_dir_all(root).unwrap();
        fs::remove_file(outside).unwrap();
    }

    #[test]
    fn helper_must_be_the_fixed_bundle_relative_file() {
        let root = fixture("helper-path");
        assert!(from_helper(&root.join("gui_shell_linux_broker")).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
