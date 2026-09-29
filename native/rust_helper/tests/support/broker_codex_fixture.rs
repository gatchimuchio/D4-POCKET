use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

    pub(super) fn descendant_heartbeat_path(&self) -> PathBuf {
        self.root.join("descendant-heartbeat")
    }

    pub(super) fn wait_for_descendant_heartbeat(&self, timeout: Duration) -> u64 {
        let path = self.descendant_heartbeat_path();
        let deadline = Instant::now() + timeout;
        loop {
            if let Ok(metadata) = std::fs::metadata(&path) {
                if metadata.len() >= 2 {
                    return metadata.len();
                }
            }
            assert!(Instant::now() < deadline, "子processの稼働marker待ち期限");
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub(super) fn descendant_heartbeat_len(&self) -> u64 {
        std::fs::metadata(self.descendant_heartbeat_path())
            .expect("子processの稼働marker")
            .len()
    }

    pub(super) fn assert_no_workspace_task_scratch(&self) {
        for entry in std::fs::read_dir(&self.workspace).expect("Workspace内Task scratch確認") {
            assert!(
                !entry
                    .expect("作業領域の項目")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".d4p-tmp-"),
                "Task終了後にBroker管理scratchを残さない"
            );
        }
    }
}

impl Drop for BrokerCodexFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
