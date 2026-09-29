use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) struct BrokerCodexFixture {
    root: PathBuf,
    workspace: PathBuf,
    executable: PathBuf,
}

impl BrokerCodexFixture {
    pub(super) fn create() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Unix epoch以後の時刻")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "gui-shell-broker-codex-task-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&root).expect("専用fixture directory");
        let workspace = root.join("workspace");
        std::fs::create_dir_all(&workspace).expect("偽Task用作業領域");

        let source = root.join("fake_codex_cli.rs");
        std::fs::write(&source, include_str!("../fixtures/fake_codex_cli.rs"))
            .expect("fake Codex CLIの試験source");
        let executable = root.join("fake-codex-cli.exe");
        let compiled = Command::new("rustc")
            .args(["--edition=2021"])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .expect("fake Codex CLI試験fileをcompileする");
        assert!(
            compiled.status.success(),
            "fake Codex CLIのcompile失敗: {}",
            String::from_utf8_lossy(&compiled.stderr)
        );

        Self {
            root,
            workspace,
            executable,
        }
    }

    pub(super) fn workspace_path(&self) -> &Path {
        &self.workspace
    }

    pub(super) fn executable_path(&self) -> &Path {
        &self.executable
    }
}

impl Drop for BrokerCodexFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
