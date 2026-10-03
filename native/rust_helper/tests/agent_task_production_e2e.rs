#[cfg(windows)]
mod windows {
    use gui_shell_rust_helper::audit_hash::sha256_tagged;
    use gui_shell_rust_helper::broker::{BrokerEndpoint, BrokerRequestEnvelope};
    use serde_json::{json, Value};
    use std::fs;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    struct Fixture {
        root: PathBuf,
        store: PathBuf,
        session_file: PathBuf,
        owner_file: PathBuf,
        workspace: PathBuf,
        child: Option<Child>,
        endpoint: Option<BrokerEndpoint>,
        helper_path: PathBuf,
        codex_path: PathBuf,
        process_generation: u32,
    }

    impl Fixture {
        fn new(helper: &std::path::Path, codex: &std::path::Path) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("システム時刻を取得")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "gui-shell-r2-task-e2e-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir(&root).expect("test専用一時rootを作成");
            let store = root.join("store");
            let session_file = root.join("normal.json");
            let owner_file = root.join("owner.json");
            let workspace = root.join("registered-workspace");
            fs::create_dir_all(workspace.join("private")).expect("合成Workspaceを作成");
            fs::write(
                workspace.join("private/fixture-secret.txt"),
                b"synthetic-secret-content-never-returned",
            )
            .expect("合成secret markerを書込");
            fs::create_dir(root.join("codex-home")).expect("分離CODEX_HOMEを作成");
            let workspace = fs::canonicalize(workspace).expect("Workspace pathを正準化");
            fs::write(
                root.join("workspace.json"),
                json!({
                    "version": 1,
                    "workspaces": [{
                        "runtime_id": "r2-codex",
                        "workspace_id": "r2-workspace",
                        "root_path": workspace.to_string_lossy(),
                        "secret_paths": ["private/fixture-secret.txt"]
                    }]
                })
                .to_string(),
            )
            .expect("Workspace起動設定を書込");
            assert!(
                !codex.to_string_lossy().contains('=')
                    && !workspace.to_string_lossy().contains('='),
                "Codex runtime引数のpathが区切り文字と衝突しない"
            );
            let mut fixture = Self {
                root,
                store,
                session_file,
                owner_file,
                workspace,
                child: None,
                endpoint: None,
                helper_path: helper.to_path_buf(),
                codex_path: codex.to_path_buf(),
                process_generation: 0,
            };
            fixture.start_process();
            fixture
        }

        fn start_process(&mut self) {
            self.process_generation += 1;
            self.session_file = self
                .root
                .join(format!("normal-{}.json", self.process_generation));
            self.owner_file = self
                .root
                .join(format!("owner-{}.json", self.process_generation));
            let runtime = format!(
                "r2-codex={}={}",
                self.codex_path.display(),
                self.workspace.display()
            );
            let child = Command::new(&self.helper_path)
                .args(["broker-server", "--store-dir"])
                .arg(&self.store)
                .arg("--session-file")
                .arg(&self.session_file)
                .arg("--owner-session-file")
                .arg(&self.owner_file)
                .arg("--workspace-config")
                .arg(self.root.join("workspace.json"))
                .arg("--codex-runtime")
                .arg(runtime)
                .current_dir(&self.root)
                .env("CODEX_HOME", self.root.join("codex-home"))
                .env_remove("OPENAI_API_KEY")
                .env_remove("CODEX_API_KEY")
                .env_remove("ANTHROPIC_API_KEY")
                .env_remove("GEMINI_API_KEY")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("Release Broker processを起動");
            self.child = Some(child);
            self.endpoint = None;
            let endpoint = self.wait_for_endpoint(false);
            let owner = self.wait_for_endpoint(true);
            assert_ne!(
                endpoint.session_secret, owner.session_secret,
                "通常IPC資格とOwner資格が分離する"
            );
            self.endpoint = Some(endpoint);
        }

        fn wait_for_endpoint(&mut self, owner: bool) -> BrokerEndpoint {
            let path = if owner {
                &self.owner_file
            } else {
                &self.session_file
            };
            let deadline = Instant::now() + Duration::from_secs(30);
            loop {
                if let Ok(bytes) = fs::read(path) {
                    if let Ok(endpoint) = serde_json::from_slice::<BrokerEndpoint>(&bytes) {
                        return endpoint;
                    }
                }
                if let Some(child) = self.child.as_mut() {
                    assert!(
                        child.try_wait().expect("Broker process状態").is_none(),
                        "Release Brokerがendpoint生成前に終了"
                    );
                }
                assert!(Instant::now() < deadline, "Release Broker起動期限超過");
                thread::sleep(Duration::from_millis(30));
            }
        }

        fn endpoint(&self) -> &BrokerEndpoint {
            self.endpoint.as_ref().expect("通常IPC endpoint")
        }

        fn stop(&mut self) {
            let endpoint = self.endpoint().clone();
            let response = send_request(&endpoint, &shutdown_request(&endpoint.session_id));
            assert_eq!(response["status"], "accepted", "{response}");
            assert!(self
                .child
                .take()
                .expect("Broker子プロセスを取得")
                .wait()
                .expect("Broker process終了待ち")
                .success());
            self.endpoint = None;
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            if let Some(mut child) = self.child.take() {
                if child.try_wait().ok().flatten().is_none() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn send_request(endpoint: &BrokerEndpoint, request: &str) -> Value {
        let mut stream = TcpStream::connect((endpoint.host.as_str(), endpoint.port))
            .expect("認証済みloopback Brokerへ接続");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("IPC応答期限を設定");
        write!(stream, "{}\n{}\n", endpoint.session_secret, request)
            .expect("通常資格とrequestを送信");
        let mut response = String::new();
        BufReader::new(stream)
            .read_line(&mut response)
            .expect("Broker応答を読取");
        serde_json::from_str(&response).expect("Broker応答JSON")
    }

    fn request(endpoint: &BrokerEndpoint, operation: &str, payload: Value) -> String {
        let request_id = gui_shell_rust_helper::broker::dialogue::識別子生成().expect("要求識別子");
        json!({
            "request_id": request_id,
            "session_id": endpoint.session_id,
            "operation": operation,
            "payload_hash": sha256_tagged(payload.to_string().as_bytes()),
            "nonce": format!("{request_id}-nonce"),
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "r2_production_agent_task_e2e"},
            "payload": payload,
        })
        .to_string()
    }

    fn shutdown_request(session_id: &str) -> String {
        let request_id = format!("r2-production-e2e-shutdown-{session_id}");
        json!({
            "request_id": request_id,
            "session_id": session_id,
            "operation": "shutdown",
            "payload_hash": sha256_tagged(b"null"),
            "nonce": format!("{request_id}-nonce"),
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "r2_production_agent_task_e2e"},
        })
        .to_string()
    }

    #[test]
    #[ignore = "明示指定したRelease Brokerと実Codex CLIでR2のproduction unsupported gateを検証するときに実行する"]
    fn production_agent_task_gate_fails_closed_over_authenticated_ipc() {
        let helper = std::env::var_os("GUI_SHELL_AGENT_TASK_E2E_HELPER_EXE")
            .map(PathBuf::from)
            .expect("検査対象のRelease Broker helper path");
        let codex = std::env::var_os("GUI_SHELL_CODEX_TASK_E2E_CLI")
            .map(PathBuf::from)
            .expect("実Codex CLI path");
        assert!(helper.is_absolute() && helper.is_file());
        assert!(codex.is_absolute() && codex.is_file());

        let mut fixture = Fixture::new(&helper, &codex);
        let endpoint = fixture.endpoint().clone();
        let agent_list_request = request(&endpoint, "Agent一覧", json!({}));
        let agent_list = send_request(&endpoint, &agent_list_request);
        assert_eq!(agent_list["status"], "accepted", "{agent_list}");
        let agents = agent_list["body"]["Agent"].as_array().expect("Agent一覧");
        let codex_agent = agents
            .iter()
            .find(|agent| agent["adapter_id"] == "codex-cli")
            .expect("実Codex CLI metadataがBroker Agent一覧へ投影される");
        let task_capability = codex_agent["capabilities"]
            .as_array()
            .and_then(|capabilities| {
                capabilities
                    .iter()
                    .find(|capability| capability["capability_id"] == "task_execution")
            })
            .expect("Task実行Capability");
        assert_eq!(task_capability["support"]["status"], "unsupported");

        let started = send_request(
            &endpoint,
            &request(
                &endpoint,
                "対話開始",
                json!({"実行系ID":"r2-codex","作業領域ID":"r2-workspace"}),
            ),
        );
        assert_eq!(started["status"], "accepted", "{started}");
        let session_id = started["body"]["対話セッションID"]
            .as_str()
            .expect("Brokerが発行したSession ID");
        let instruction = "R2_PRODUCTION_NEGATIVE_E2E_INSTRUCTION_SENTINEL";
        let task = json!({
            "agent_runtime_id":"r2-codex",
            "session_id":session_id,
            "workspace_id":"r2-workspace",
            "instruction":instruction,
        });

        for operation in ["Agent作業要求検査", "AgentTask実行"] {
            let response = send_request(&endpoint, &request(&endpoint, operation, task.clone()));
            assert_eq!(response["status"], "rejected", "{operation}: {response}");
            assert_eq!(response["error"]["code"], "AgentTask実行非対応");
            assert!(response["body"].is_null());
            assert!(!response.to_string().contains(instruction));
        }

        let permission = json!({
            "agent_runtime_id":"r2-codex",
            "session_id":session_id,
            "workspace_id":"r2-workspace",
        });
        let workspace_permission = send_request(
            &endpoint,
            &request(&endpoint, "AgentTaskWorkspacePermissionGrant", permission),
        );
        let owner_approval = send_request(
            &endpoint,
            &request(&endpoint, "AgentTaskOwnerApprovalGrant", task),
        );
        for response in [&workspace_permission, &owner_approval] {
            assert_eq!(response["status"], "rejected", "{response}");
            assert_eq!(
                response["error"]["code"],
                "desktop_native_owner_confirmation_required"
            );
            assert!(response["body"].is_null());
            assert!(!response.to_string().contains(instruction));
        }

        fixture.stop();
        fixture.start_process();
        let restarted_endpoint = fixture.endpoint().clone();
        assert_ne!(
            endpoint.session_id, restarted_endpoint.session_id,
            "Broker再起動で通常IPC sessionを新規発行する"
        );
        assert_ne!(
            endpoint.session_secret, restarted_endpoint.session_secret,
            "Broker再起動で通常IPC資格を新規発行する"
        );
        let stale_session = send_request(&restarted_endpoint, &agent_list_request);
        assert_eq!(stale_session["status"], "rejected", "{stale_session}");
        assert!(stale_session["body"].is_null());
        let stale_session_audit_id = stale_session["audit_event_id"]
            .as_str()
            .expect("再起動前session要求の拒否Audit")
            .to_string();
        let restarted_agent_list = send_request(
            &restarted_endpoint,
            &request(&restarted_endpoint, "Agent一覧", json!({})),
        );
        assert_eq!(
            restarted_agent_list["status"], "accepted",
            "{restarted_agent_list}"
        );
        fixture.stop();
        let audit_path = fixture.store.join("audit.jsonl");
        let audit_text = fs::read_to_string(&audit_path).expect("永続Audit file");
        assert!(!audit_text.contains(instruction));
        assert!(!audit_text.contains("synthetic-secret-content-never-returned"));
        let (_, persisted) = gui_shell_rust_helper::broker::BrokerPersistentStore::open_or_create(
            &fixture.store,
            "r2-production-e2e-verify",
        )
        .expect("終了後に永続Audit chainを再検証");
        for response in [
            &agent_list,
            &started,
            &workspace_permission,
            &owner_approval,
            &stale_session,
            &restarted_agent_list,
        ] {
            let event_id = response["audit_event_id"].as_str().unwrap();
            assert!(persisted
                .audit_log
                .events()
                .iter()
                .any(|event| event.event_id == event_id));
        }
        for operation in ["Agent作業要求検査", "AgentTask実行"] {
            assert!(persisted.audit_log.events().iter().any(|event| {
                event.operation == operation
                    && event.decision == "rejected"
                    && event.reason.contains("AgentTask実行非対応")
            }));
        }
        assert!(persisted
            .audit_log
            .events()
            .iter()
            .any(|event| event.event_id == stale_session_audit_id && event.decision == "rejected"));
        assert!(fs::read_dir(&fixture.workspace).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".d4p-tmp-")
        }));
    }
}
