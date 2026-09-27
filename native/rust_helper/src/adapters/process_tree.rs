use std::io;
use std::process::{Child, Command, ExitStatus};

pub(super) struct SupervisedChild {
    pub(super) child: Child,
    #[cfg(windows)]
    job: gui_shell_process_supervision::Job,
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
    #[cfg(not(windows))]
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
        Ok(())
    }

    pub(super) fn terminate_tree(&mut self) -> io::Result<()> {
        #[cfg(windows)]
        {
            self.job.terminate_and_wait()?;
            self.child.wait()?;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            if self.child.try_wait()?.is_none() {
                self.child.kill()?;
            }
            self.child.wait()?;
            Ok(())
        }
    }
}

impl Drop for SupervisedChild {
    fn drop(&mut self) {
        let _ = self.terminate_tree();
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
