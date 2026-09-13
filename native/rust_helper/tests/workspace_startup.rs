use gui_shell_rust_helper::{audit_hash::sha256_tagged, broker::{BrokerEndpoint, BrokerRequestEnvelope}};
use serde_json::{json,Value};
use std::{fs,io::{BufRead,BufReader,Write},net::TcpStream,path::PathBuf,process::{Child,Command,Stdio},time::{Duration,Instant}};

struct Fixture {root:PathBuf, child:Option<Child>}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(mut child)=self.child.take() {let _=child.kill();let _=child.wait();}
        let _=fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    fn new() -> Self {
        let root=std::env::temp_dir().join(format!("gui-shell-startup-{}",gui_shell_rust_helper::broker::dialogue::識別子生成().unwrap()));
        fs::create_dir_all(root.join("project/private")).unwrap();
        let root=fs::canonicalize(root).unwrap();
        fs::write(root.join("project/本文.txt"),"登録から実読取まで\r\n").unwrap();
        fs::write(root.join("project/.env"),"秘密").unwrap();
        fs::write(root.join("project/private/file.txt"),"ownerの除外").unwrap();
        let result=Self {root,child:None};
        result.configure(result.root.join("project").to_str().unwrap());result
    }
    fn configure(&self, path:&str) {
        fs::write(self.root.join("workspace.json"),json!({"version":1,"workspaces":[{"runtime_id":"gui_shell_rust_broker","workspace_id":"workspace-a","root_path":path,"secret_paths":["private"]}]}).to_string()).unwrap();
    }
    fn command(&self, owner:bool) -> Command {
        let mut cmd=Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper"));
        cmd.arg("broker-server").arg("--store-dir").arg(self.root.join("store"))
            .arg("--session-file").arg(self.root.join("normal.json"))
            .arg("--workspace-config").arg(self.root.join("workspace.json"));
        if owner {cmd.arg("--owner-session-file").arg(self.root.join("owner.json"));}
        cmd
    }
    fn start(&mut self) -> (BrokerEndpoint,BrokerEndpoint) {
        self.child=Some(self.command(true).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap());
        let deadline=Instant::now()+Duration::from_secs(15);
        loop {
            let read=|name:&str| -> Option<BrokerEndpoint> {serde_json::from_slice(&fs::read(self.root.join(name)).ok()?).ok()};
            if let (Some(normal),Some(owner))=(read("normal.json"),read("owner.json")) {return (normal,owner);}
            assert!(self.child.as_mut().unwrap().try_wait().unwrap().is_none(),"Brokerが登録時に終了した");
            assert!(Instant::now()<deadline,"Broker起動期限超過");std::thread::sleep(Duration::from_millis(30));
        }
    }
    fn failed_start(&self,owner:bool) {
        let result=self.command(owner).output().unwrap();
        assert!(!result.status.success());
        assert!(!self.root.join("normal.json").exists());assert!(!self.root.join("owner.json").exists());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("秘密"));
    }
}
fn request(endpoint:&BrokerEndpoint,operation:&str,payload:Value) -> Value {
    let id=gui_shell_rust_helper::broker::dialogue::識別子生成().unwrap();
    let request=json!({"request_id":id,"nonce":id,"session_id":endpoint.session_id,"issued_at":BrokerRequestEnvelope::current_issued_at(),"operation":operation,"metadata":{},"payload_hash":sha256_tagged(payload.to_string().as_bytes()),"payload":payload});
    let mut stream=TcpStream::connect((endpoint.host.as_str(),endpoint.port)).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    writeln!(stream,"{}\n{}",endpoint.session_secret,request).unwrap();
    let mut line=String::new();BufReader::new(stream).read_line(&mut line).unwrap();serde_json::from_str(&line).unwrap()
}
fn cli(f:&Fixture,args:&[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_gui_shell_rust_helper")).args(["作業領域制御","--session-file"]).arg(f.root.join("owner.json")).args(args).output().unwrap()
}

#[test]
fn startup_owner_approval_real_ipc_read_and_revocation_are_connected() {
    let mut f=Fixture::new();let (normal,owner)=f.start();
    let selection=json!({"作業領域ID":"workspace-a","相対path":"本文.txt"});
    assert_eq!(request(&normal,"作業領域読取",selection.clone())["status"],"rejected");
    let listed=request(&normal,"作業領域一覧",json!({}));
    let hash=listed["body"]["作業領域"][0]["登録hash"].as_str().unwrap();
    let approval=json!({"作業領域ID":"workspace-a","登録hash":hash,"表示範囲":"full"});
    assert_eq!(request(&normal,"作業領域承認",approval.clone())["error"]["code"],"権限拒否");
    let approved=cli(&f,&["作業領域承認","workspace-a",hash,"full"]);
    assert!(approved.status.success(),"{}",String::from_utf8_lossy(&approved.stderr));
    assert!(!String::from_utf8_lossy(&approved.stdout).contains(&owner.session_secret));
    let response=request(&normal,"作業領域読取",selection.clone());
    assert_eq!(response["status"],"accepted");assert_eq!(response["evidence_source"],"LIVE_RUNTIME");
    assert_eq!(response["body"]["projection"]["text"],"登録から実読取まで\r\n");
    let current=request(&normal,"作業領域一覧",json!({}));
    let current=&current["body"]["作業領域"][0];
    for field in ["作業領域ID","実行系ID","登録hash","approval_id","有効期限","表示範囲"] {
        assert_eq!(response["body"][field],current[field],"{field}");
    }
    assert_eq!(response["body"]["version"],1);
    assert_eq!(response["body"]["operation"],"作業領域読取");
    assert_eq!(response["body"]["要求hash"],sha256_tagged(selection.to_string().as_bytes()));

    let tree=request(&normal,"作業領域ツリー",json!({"作業領域ID":"workspace-a","相対path":""}));
    assert_eq!(tree["status"],"accepted");assert_eq!(tree["body"]["projection"]["entries"].as_array().unwrap().len(),1);
    for path in [".env","private/file.txt","../owner.json"] {
        assert_eq!(request(&normal,"作業領域読取",json!({"作業領域ID":"workspace-a","相対path":path}))["status"],"rejected");
    }
    assert!(cli(&f,&["作業領域失効","workspace-a",hash]).status.success());
    assert_eq!(request(&normal,"作業領域読取",json!({"作業領域ID":"workspace-a","相対path":"本文.txt"}))["status"],"rejected");
    assert_eq!(request(&normal,"shutdown",Value::Null)["status"],"accepted");
    assert!(f.child.take().unwrap().wait().unwrap().success());
    let (_,state)=gui_shell_rust_helper::broker::BrokerPersistentStore::open_or_create(f.root.join("store"),"verify").unwrap();
    let event=state.audit_log.events().iter().find(|e|e.event_id==response["audit_event_id"].as_str().unwrap()).unwrap();
    assert_eq!(event.payload_hash,sha256_tagged(response["body"].to_string().as_bytes()));
    let log=fs::read_to_string(f.root.join("store/audit.jsonl")).unwrap();
    for secret in ["本文.txt","登録から実読取まで",&owner.session_secret] {assert!(!log.contains(secret));}
}

#[test]
fn startup_rejects_internal_credentials_overlap_relative_secret_and_missing_owner() {
    for root in [".","../project","/", "secret"] {
        let f=Fixture::new();
        let path=if root=="secret" {let p=f.root.join(".git");fs::create_dir(&p).unwrap();p.to_string_lossy().into_owned()}else{root.into()};
        f.configure(&path);f.failed_start(true);
    }
    for child in ["", "store"] {
        let f=Fixture::new();fs::create_dir_all(f.root.join("store")).unwrap();
        f.configure(f.root.join(child).to_str().unwrap());f.failed_start(true);
        let log=fs::read_to_string(f.root.join("store/audit.jsonl")).unwrap();assert!(log.contains("内部store・資格・設定と重なる"));
    }
    Fixture::new().failed_start(false);
    let f=Fixture::new();
    let owner_in_workspace=f.root.join("project/owner.json");
    let result=f.command(false).arg("--owner-session-file").arg(&owner_in_workspace).output().unwrap();
    assert!(!result.status.success());assert!(!owner_in_workspace.exists());
    assert!(String::from_utf8_lossy(&result.stderr).contains("内部store・資格・設定と重なる"));
    #[cfg(windows)] {
        let f=Fixture::new();f.configure(&f.root.to_string_lossy().to_uppercase());f.failed_start(true);
    }
}

#[test]
fn startup_rejects_duplicate_unknown_oversized_and_versioned_configuration() {
    for invalid in [r#"{"version":1,"version":1,"workspaces":[]}"#.to_owned(),
        r#"{"version":2,"workspaces":[]}"#.to_owned(),r#"{"version":1,"workspaces":[],"approval":"full"}"#.to_owned()," ".repeat(65_537)] {
        let f=Fixture::new();fs::write(f.root.join("workspace.json"),invalid).unwrap();f.failed_start(true);
        let log=fs::read_to_string(f.root.join("store/audit.jsonl")).unwrap();
        assert!(log.contains("作業領域設定読取"));assert!(log.contains("rejected"));
    }
}

#[test]
fn startup_rejects_root_directory_link() {
    let f=Fixture::new();let link=f.root.join("alias");let target=f.root.join("project");
    #[cfg(windows)] {assert!(Command::new("cmd").args(["/C","mklink","/J"]).arg(&link).arg(&target).stdout(Stdio::null()).status().unwrap().success());}
    #[cfg(unix)] {std::os::unix::fs::symlink(&target,&link).unwrap();}
    f.configure(link.to_str().unwrap());f.failed_start(true);
    #[cfg(windows)] {fs::remove_dir(link).unwrap();}
    #[cfg(unix)] {fs::remove_file(link).unwrap();}
    assert_eq!(fs::read_to_string(target.join("本文.txt")).unwrap(),"登録から実読取まで\r\n");
}

#[cfg(target_os="linux")]
#[test]
fn startup_rejects_procfs_and_reader_rejects_cross_device_directory() {
    let f=Fixture::new();f.configure("/proc");f.failed_start(true);
    let reader=gui_shell_rust_helper::workspace_reader::WorkspaceReader::from_registered_dir(cap_std::fs::Dir::open_ambient_dir("/",cap_std::ambient_authority()).unwrap(),&[]).unwrap();
    assert_eq!(reader.list("proc"),Err(gui_shell_rust_helper::workspace_reader::ReadError::UnsafeFile));
}
