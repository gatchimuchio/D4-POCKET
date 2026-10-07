#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn main() {
    let directory = env::current_dir().expect("試験frontend作業directory");
    if env::args().nth(1).as_deref() == Some("--descendant") {
        fs::write(
            directory.join("descendant.started"),
            std::process::id().to_string(),
        )
        .expect("試験孫process起動marker");
        thread::sleep(Duration::from_secs(15));
        return;
    }

    fs::write(
        directory.join("frontend.started"),
        std::process::id().to_string(),
    )
    .expect("試験frontend起動marker");
    let executable = env::current_exe().expect("試験frontend実行file");
    let descendant = Command::new(executable)
        .arg("--descendant")
        .current_dir(&directory)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Job内の試験孫process");
    fs::write(
        directory.join("descendant.pid"),
        descendant.id().to_string(),
    )
    .expect("試験孫process ID marker");
    drop(descendant);
    thread::sleep(Duration::from_secs(15));
}
