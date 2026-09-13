use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Fixture {
    root: PathBuf,
    child: Option<Child>,
}
impl Fixture {
    fn new() -> Self {
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).unwrap();
        let root =
            std::env::temp_dir().join(format!("gui-shell-protected-start-{}", hex::encode(random)));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("vault")).unwrap();
        Self { root, child: None }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        #[cfg(windows)]
        if self.root.join("linked").exists() {
            fs::remove_dir(self.root.join("linked")).unwrap();
        }
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn 保管先起動はownerと独立rootを必要とする() {
    for case in 0..9 {
        let mut fixture = Fixture::new();
        let root = &fixture.root;
        let mut command = Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"));
        command
            .arg("broker-server")
            .arg("--store-dir")
            .arg(root.join("store"))
            .arg("--session-file")
            .arg(root.join("normal.json"))
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if case != 1 {
            command
                .arg("--owner-session-file")
                .arg(root.join("owner.json"));
        }
        if case == 8 {
            fs::create_dir(root.join("store")).unwrap();
            fs::create_dir(root.join("store/audit.jsonl")).unwrap();
        }
        #[cfg(windows)]
        if case == 6 {
            let status = Command::new("cmd")
                .args(["/D", "/C", "mklink", "/J"])
                .arg(root.join("linked"))
                .arg(root.join("vault"))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();
            assert!(status.success());
        }
        let path = match case {
            2 => PathBuf::from("relative-vault"),
            3 => root.join("store"),
            4 => root.join("absent"),
            6 => root.join("linked"),
            _ => root.join("vault"),
        };
        command.arg("--protected-store-dir").arg(&path);
        if case == 7 {
            command.arg("--protected-store-dir").arg(&path);
        }
        if case == 5 {
            let config = root.join("workspace.json");
            fs::write(&config, json!({"version":1,"workspaces":[{"runtime_id":"local","workspace_id":"overlap","root_path":path,"secret_paths":[]}]}).to_string()).unwrap();
            command
                .args([
                    "--minidora-runtime",
                    "local=127.0.0.1:9",
                    "--workspace-config",
                ])
                .arg(config);
        }
        fixture.child = Some(command.spawn().unwrap());
        let deadline = Instant::now() + Duration::from_secs(15);
        let success = cfg!(windows) && case == 0;
        loop {
            if let Some(status) = fixture.child.as_mut().unwrap().try_wait().unwrap() {
                assert!(!success && !status.success(), "case={case}");
                assert!(!root.join("normal.json").exists());
                break;
            }
            if success && root.join("normal.json").exists() {
                let events: Vec<Value> = fs::read_to_string(root.join("store/audit.jsonl"))
                    .unwrap()
                    .lines()
                    .map(|s| serde_json::from_str(s).unwrap())
                    .collect();
                let verified: Vec<_> = events
                    .iter()
                    .filter(|e| e["operation"] == "保管先登録" && e["decision"] == "verified")
                    .collect();
                assert_eq!(verified.len(), 1);
                assert_eq!(verified[0]["evidence_source"], "LIVE_RUNTIME");
                assert_eq!(
                    verified[0]["payload_hash"],
                    gui_shell_rust_helper::audit_hash::sha256_tagged(
                        path.to_string_lossy().as_bytes()
                    )
                );
                assert_eq!(fs::read_dir(root.join("vault")).unwrap().count(), 0);
                break;
            }
            assert!(Instant::now() < deadline, "起動試験期限 case={case}");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
