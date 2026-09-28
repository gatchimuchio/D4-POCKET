use std::fs;
use std::path::{Path, PathBuf};

pub(super) struct FixtureTempDirectory(PathBuf);

impl FixtureTempDirectory {
    pub(super) fn create() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("時計")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "gui-shell-codex-adapter-fixture-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("試験専用temporary directory");
        Self(path)
    }

    pub(super) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for FixtureTempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn compile_fake_codex_cli(directory: &Path) -> PathBuf {
    let source = directory.join("fake_codex_cli.rs");
    fs::write(&source, include_str!("../fixtures/fake_codex_cli.rs")).expect("偽CLI試験source");
    let executable = directory.join("fake-codex-cli.exe");
    let output = std::process::Command::new("rustc")
        .args(["--edition=2021"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("Rust試験toolchain compiler");
    assert!(
        output.status.success(),
        "偽Codex CLI試験用fileのcompileに失敗: {:?}",
        output.status.code()
    );
    executable
}

pub(super) fn assert_no_workspace_task_scratch(workspace: &Path) {
    for entry in fs::read_dir(workspace).expect("作業領域を読む") {
        let entry = entry.expect("作業領域項目");
        assert!(
            !entry.file_name().to_string_lossy().starts_with(".d4p-tmp-"),
            "Adapter完了後にTask一時領域が残っている"
        );
    }
}
