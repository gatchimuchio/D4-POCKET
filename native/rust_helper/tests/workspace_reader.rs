use cap_std::fs::Dir;
use gui_shell_rust_helper::workspace_reader::{ReadError, WorkspaceReader};
use std::{fs, path::PathBuf};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).unwrap();
        let path =
            std::env::temp_dir().join(format!("gui-shell-reader-ipc-test-{}", hex::encode(random)));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn reader(&self) -> WorkspaceReader {
        WorkspaceReader::from_registered_dir(
            Dir::open_ambient_dir(&self.0, cap_std::ambient_authority()).unwrap(),
            &[],
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn directory_link_to_outside_is_rejected() {
    let f = Fixture::new();
    let outside = Fixture::new();
    fs::write(outside.0.join("file"), "secret").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside.0, f.0.join("alias")).unwrap();
    #[cfg(windows)]
    {
        let output = std::process::Command::new("cmd")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(f.0.join("alias"))
            .arg(&outside.0)
            .output()
            .unwrap();
        assert!(output.status.success(), "junctionの実試験に失敗");
    }
    let reader = f.reader();
    assert_eq!(reader.read("alias/file"), Err(ReadError::UnsafeFile));
    assert_eq!(reader.read_version("alias/missing"), Err(ReadError::UnsafeFile));
    assert!(reader.list("").unwrap().is_empty());
    // junction先を削除対象にしない。link自身だけを取り除く。
    #[cfg(windows)]
    fs::remove_dir(f.0.join("alias")).unwrap();
    #[cfg(unix)]
    fs::remove_file(f.0.join("alias")).unwrap();
    assert!(outside.0.join("file").exists());
}

#[cfg(unix)]
#[test]
fn final_symlink_to_secret_and_fifo_are_rejected_without_blocking() {
    use std::{sync::mpsc, thread, time::Duration};
    let f = Fixture::new();
    fs::write(f.0.join(".env"), "secret").unwrap();
    std::os::unix::fs::symlink(".env", f.0.join("alias")).unwrap();
    let reader = f.reader();
    assert_eq!(reader.read("alias"), Err(ReadError::UnsafeFile));
    assert_eq!(reader.read_version("alias"), Err(ReadError::UnsafeFile));
    std::os::unix::fs::symlink("absent", f.0.join("dangling")).unwrap();
    assert_eq!(reader.read_version("dangling"), Err(ReadError::UnsafeFile));
    assert!(std::process::Command::new("mkfifo")
        .arg(f.0.join("pipe"))
        .status()
        .unwrap()
        .success());
    let (tx, rx) = mpsc::channel();
    let task = thread::spawn(move || {
        tx.send(reader.read("pipe")).unwrap();
    });
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(2))
            .expect("FIFO読取が停止した"),
        Err(ReadError::UnsafeFile)
    );
    task.join().unwrap();
}

#[test]
fn diff_versions_distinguish_absent_empty_and_rejected_files() {
    use gui_shell_rust_helper::workspace_diff::{generate,DiffKind};
    let f=Fixture::new();
    let reader=f.reader();
    assert_eq!(reader.read_version("missing/file.txt"),Ok(None));
    assert_eq!(reader.read_version("file.txt"),Ok(None));
    fs::write(f.0.join("file.txt"),b"").unwrap();
    assert_eq!(reader.read_version("file.txt"),Ok(Some(vec![])));
    let before=reader.read_version("file.txt").unwrap();
    fs::write(f.0.join("file.txt"),"変更後\n").unwrap();
    let after=reader.read_version("file.txt").unwrap();
    assert_eq!(generate(before.as_deref(),after.as_deref()).kind,DiffKind::Text);
    fs::remove_file(f.0.join("file.txt")).unwrap();
    let absent=reader.read_version("file.txt").unwrap();
    let deleted=generate(after.as_deref(),absent.as_deref());
    assert!(deleted.before.is_some());assert!(deleted.after.is_none());
    assert_eq!(deleted.kind,DiffKind::Text);
    fs::write(f.0.join("large.txt"),vec![b'x';65_537]).unwrap();
    assert_eq!(reader.read_version("large.txt"),Err(ReadError::TooLarge(65_537)));
    fs::write(f.0.join("ordinary"),b"file").unwrap();
    assert_eq!(reader.read_version("ordinary/child"),Err(ReadError::UnsafeFile));
    fs::create_dir(f.0.join("directory")).unwrap();
    assert_eq!(reader.read_version("directory"),Err(ReadError::UnsafeFile));
    fs::hard_link(f.0.join("ordinary"),f.0.join("hardlink")).unwrap();
    assert_eq!(reader.read_version("hardlink"),Err(ReadError::UnsafeFile));
    for path in [".env", "missing/.env", "missing/key.pem"] {
        assert_eq!(reader.read_version(path),Err(ReadError::SecretPath));
    }
    for path in ["", "../outside", "missing/../outside"] {
        assert_eq!(reader.read_version(path),Err(ReadError::InvalidPath));
    }
}
