use std::io;
use std::process::{Child, Command, ExitStatus};

pub(super) struct SupervisedChild {
    pub(super) child: Child,
    #[cfg(windows)]
    job: gui_shell_process_supervision::Job,
    #[cfg(target_os = "macos")]
    group: Option<rustix::process::Pid>,
}

impl std::fmt::Debug for SupervisedChild {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SupervisedChild")
            .field("process_id", &self.child.id())
            .finish()
    }
}

pub(super) fn spawn(mut command: Command) -> io::Result<SupervisedChild> {
    #[cfg(windows)]
    {
        use std::os::windows::{io::AsRawHandle, process::CommandExt};

        let job = gui_shell_process_supervision::Job::create()?;
        command.creation_flags(gui_shell_process_supervision::CREATE_SUSPENDED);
        let mut child = command.spawn()?;
        if let Err(error) = job.assign_and_resume(child.id(), child.as_raw_handle()) {
            let _ = job.terminate_and_wait();
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(SupervisedChild { child, job })
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
        let mut child = command.spawn()?;
        let group = i32::try_from(child.id())
            .ok()
            .filter(|id| *id > 1)
            .and_then(rustix::process::Pid::from_raw);
        let valid = group.is_some_and(|id| {
            id != rustix::process::getpgrp() && rustix::process::getpgid(Some(id)).ok() == Some(id)
        });
        if !valid {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other(
                "所有childの独立process groupを確認できない",
            ));
        }
        Ok(SupervisedChild { child, group })
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        command.spawn().map(|child| SupervisedChild { child })
    }
}

impl SupervisedChild {
    pub(super) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    /// Root process終了後に残るprocessを停止し、継承されたpipe readerを解放する。
    pub(super) fn stop_descendants(&mut self) -> io::Result<()> {
        #[cfg(windows)]
        self.job.terminate_and_wait()?;
        #[cfg(target_os = "macos")]
        self.terminate_mac_group()?;
        Ok(())
    }

    pub(super) fn terminate_tree(&mut self) -> io::Result<()> {
        #[cfg(windows)]
        {
            self.job.terminate_and_wait()?;
            self.child.wait()?;
            Ok(())
        }
        #[cfg(target_os = "macos")]
        {
            self.terminate_mac_group()
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            if self.child.try_wait()?.is_none() {
                self.child.kill()?;
            }
            self.child.wait()?;
            Ok(())
        }
    }

    #[cfg(target_os = "macos")]
    fn terminate_mac_group(&mut self) -> io::Result<()> {
        use rustix::process::{kill_process_group, test_kill_process_group, Signal};
        let Some(group) = self.group else {
            return Ok(());
        };
        match kill_process_group(group, Signal::KILL) {
            Ok(()) | Err(rustix::io::Errno::SRCH) => {}
            Err(error) => return Err(error.into()),
        }
        // groupから移動したrootも所有Childとしてだけ回収する。
        if self.child.try_wait()?.is_none() {
            self.child.kill()?;
        }
        self.child.wait()?;
        let started = std::time::Instant::now();
        loop {
            match test_kill_process_group(group) {
                Err(rustix::io::Errno::SRCH) => {
                    self.group = None;
                    return Ok(());
                }
                Err(error) => return Err(error.into()),
                Ok(()) if started.elapsed() >= std::time::Duration::from_secs(5) => {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "所有process groupの停止が未確定",
                    ));
                }
                Ok(()) => std::thread::sleep(std::time::Duration::from_millis(10)),
            }
        }
    }
}

impl Drop for SupervisedChild {
    fn drop(&mut self) {
        let _ = self.terminate_tree();
    }
}

#[cfg(all(test, target_os = "macos"))]
mod macos_tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    fn macos_owned_group_stops_root_and_inherited_child() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("d4-mcp-group-{}-{nonce}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let marker = root.join("ready");
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "sleep 60 & echo ready > ready; wait"])
            .current_dir(&root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = spawn(command).expect("独立groupのfixture起動");
        assert_ne!(child.group.unwrap(), rustix::process::getpgrp());
        let started = std::time::Instant::now();
        while !marker.exists() {
            assert!(
                started.elapsed() < std::time::Duration::from_secs(5),
                "fixture起動期限"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let group = child.group.unwrap();
        child.terminate_tree().expect("所有groupとrootの停止");
        assert_eq!(
            rustix::process::test_kill_process_group(group),
            Err(rustix::io::Errno::SRCH)
        );
        assert!(child.child.try_wait().unwrap().is_some());
        assert!(child.group.is_none());
        std::fs::remove_file(marker).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Stdio;
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    const READY_MARKER: &str = "GUI_SHELL_SUPERVISED_CHILD_READY";

    #[test]
    fn cancellation_terminates_the_supervised_child() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("時計")
            .as_nanos();
        let marker = std::env::temp_dir().join(format!(
            "gui-shell-supervised-child-{}-{nonce}",
            std::process::id()
        ));
        let executable = std::env::current_exe().expect("試験実行file");
        let mut command = Command::new(executable);
        command
            .args([
                "--exact",
                "adapters::process_tree::tests::persistent_child_fixture",
                "--nocapture",
            ])
            .env(READY_MARKER, &marker)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = spawn(command).expect("監督付きchild process起動");

        let started = Instant::now();
        while !marker.exists() {
            if started.elapsed() > Duration::from_secs(10) {
                let _ = child.terminate_tree();
                let _ = fs::remove_file(&marker);
                panic!("child fixtureの起動を確認できない");
            }
            thread::sleep(Duration::from_millis(10));
        }

        child.terminate_tree().expect("process群を停止して回収");
        assert!(child.child.try_wait().expect("child状態確認").is_some());
        fs::remove_file(marker).expect("marker削除");
    }

    #[test]
    fn persistent_child_fixture() {
        if let Some(marker) = std::env::var_os(READY_MARKER) {
            fs::write(marker, b"ready").expect("起動marker記録");
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}
