use std::io;
use std::process::{Child, Command, ExitStatus};

#[cfg(target_os = "linux")]
use std::process::ChildStdin;

pub(super) struct SupervisedChild {
    pub(super) child: Child,
    #[cfg(windows)]
    job: gui_shell_process_supervision::Job,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    group: Option<rustix::process::Pid>,
    #[cfg(target_os = "linux")]
    guardian: Option<Child>,
    #[cfg(target_os = "linux")]
    guardian_control: Option<ChildStdin>,
}

impl std::fmt::Debug for SupervisedChild {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SupervisedChild")
            .field("process_id", &self.child.id())
            .finish()
    }
}

pub(super) fn spawn(command: Command) -> io::Result<SupervisedChild> {
    #[cfg(windows)]
    {
        use std::os::windows::{io::AsRawHandle, process::CommandExt};

        let mut command = command;
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
        let mut command = command;
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
    #[cfg(target_os = "linux")]
    {
        spawn_linux(command)
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        command.spawn().map(|child| SupervisedChild { child })
    }
}

impl SupervisedChild {
    pub(super) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        #[cfg(target_os = "linux")]
        {
            let guardian_exited = self.group.is_some()
                && match self.guardian.as_mut() {
                    Some(guardian) => guardian.try_wait()?.is_some(),
                    None => true,
                };
            if guardian_exited {
                // Guardian消失後は数値PGIDの再利用を避け、追跡中のrootだけを停止する。
                if self.child.try_wait()?.is_none() {
                    self.child.kill()?;
                }
                self.child.wait()?;
                self.guardian.take();
                self.guardian_control.take();
                self.group = None;
                return Err(io::Error::other(
                    "Linux所有process group guardianがTaskより先に終了した",
                ));
            }
        }
        self.child.try_wait()
    }

    /// Root process終了後に残るprocessを停止し、継承されたpipe readerを解放する。
    pub(super) fn stop_descendants(&mut self) -> io::Result<()> {
        #[cfg(windows)]
        self.job.terminate_and_wait()?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        self.terminate_unix_group()?;
        Ok(())
    }

    pub(super) fn terminate_tree(&mut self) -> io::Result<()> {
        #[cfg(windows)]
        {
            self.job.terminate_and_wait()?;
            self.child.wait()?;
            Ok(())
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.terminate_unix_group()
        }
        #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
        {
            if self.child.try_wait()?.is_none() {
                self.child.kill()?;
            }
            self.child.wait()?;
            Ok(())
        }
    }

    #[cfg(target_os = "linux")]
    fn terminate_unix_group(&mut self) -> io::Result<()> {
        use rustix::process::{kill_process_group, test_kill_process_group, Signal};
        let Some(group) = self.group else {
            return Ok(());
        };
        let guardian_alive = match self.guardian.as_mut() {
            Some(guardian) => guardian.try_wait()?.is_none(),
            None => false,
        };
        if !guardian_alive {
            // guardianがPGIDをpinしていない状態では、再利用されたgroupへsignalしない。
            if self.child.try_wait()?.is_none() {
                self.child.kill()?;
            }
            self.child.wait()?;
            self.guardian.take();
            self.guardian_control.take();
            self.group = None;
            return Err(io::Error::other(
                "Linux guardian消失後のprocess群停止を安全に確認できない",
            ));
        }
        match kill_process_group(group, Signal::KILL) {
            Ok(()) | Err(rustix::io::Errno::SRCH) => {}
            Err(error) => return Err(error.into()),
        }
        self.guardian_control.take();
        if self.child.try_wait()?.is_none() {
            self.child.kill()?;
        }
        self.child.wait()?;
        if let Some(guardian) = self.guardian.as_mut() {
            guardian.wait()?;
        }
        self.guardian.take();
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
                        "Linux所有process groupの停止が未確定",
                    ));
                }
                Ok(()) => std::thread::sleep(std::time::Duration::from_millis(10)),
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn terminate_unix_group(&mut self) -> io::Result<()> {
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

#[cfg(target_os = "linux")]
fn spawn_linux(mut command: Command) -> io::Result<SupervisedChild> {
    use std::os::unix::process::CommandExt;

    let (mut guardian, guardian_control, group, group_raw) = spawn_linux_guardian()?;
    command.process_group(group_raw);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
            drop(guardian_control);
            let _ = guardian.wait();
            return Err(error);
        }
    };
    let valid = group != rustix::process::getpgrp()
        && rustix::process::getpgid(Some(group)).ok() == Some(group);
    if !valid {
        let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
        let _ = child.kill();
        let _ = child.wait();
        drop(guardian_control);
        let _ = guardian.wait();
        return Err(io::Error::other(
            "Taskとguardianの独立process groupを確認できない",
        ));
    }
    Ok(SupervisedChild {
        child,
        group: Some(group),
        guardian: Some(guardian),
        guardian_control: Some(guardian_control),
    })
}

#[cfg(target_os = "linux")]
fn spawn_linux_guardian() -> io::Result<(Child, ChildStdin, rustix::process::Pid, i32)> {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    let mut command = Command::new(std::env::current_exe()?);
    #[cfg(test)]
    command.args([
        "--ignored",
        "--exact",
        "adapters::process_tree::unix_tests::guardian_process_entry",
        "--nocapture",
    ]);
    #[cfg(not(test))]
    command.arg(crate::linux_process_group_guardian::INTERNAL_ARGUMENT);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    let mut guardian = command.spawn()?;
    let group_raw = i32::try_from(guardian.id())
        .map_err(|_| io::Error::other("Linux guardian PIDがprocess group範囲外"))?;
    let group = rustix::process::Pid::from_raw(group_raw)
        .ok_or_else(|| io::Error::other("Linux guardian PIDが不正"))?;
    let valid = group != rustix::process::getpgrp()
        && rustix::process::getpgid(Some(group)).ok() == Some(group);
    let Some(control) = guardian.stdin.take() else {
        let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
        let _ = guardian.wait();
        return Err(io::Error::other("Linux guardian監視pipeを作成できない"));
    };
    if !valid {
        let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
        drop(control);
        let _ = guardian.wait();
        return Err(io::Error::other(
            "Linux guardian専用process groupを確認できない",
        ));
    }
    Ok((guardian, control, group, group_raw))
}

impl Drop for SupervisedChild {
    fn drop(&mut self) {
        let _ = self.terminate_tree();
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod unix_tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    fn unix_owned_group_stops_root_and_inherited_child() {
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

    #[cfg(target_os = "linux")]
    #[test]
    fn broker_pipe_eof_stops_the_owned_group() {
        use std::os::unix::process::CommandExt;
        use std::process::Stdio;
        use std::thread;
        use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("d4-linux-guardian-{}-{nonce}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let marker = root.join("ready");
        let (mut guardian, control, group, group_raw) =
            spawn_linux_guardian().expect("独立Linux guardian起動");
        let mut task = Command::new("/bin/sh")
            .args(["-c", "sleep 60 & echo ready > ready; wait"])
            .current_dir(&root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(group_raw)
            .spawn()
            .expect("guardian所有group内のTask起動");
        let started = Instant::now();
        while !marker.exists() {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "Task fixture起動期限"
            );
            thread::sleep(Duration::from_millis(10));
        }

        // Broker crash相当: 回収処理を呼ばず、監視pipeだけを閉じる。
        drop(control);
        let guardian_status = guardian.wait().expect("guardian終了回収");
        assert!(
            !guardian_status.success(),
            "guardianはgroup signalで停止する"
        );
        let _ = task.wait().expect("Task root終了回収");
        let stopped_at = Instant::now();
        loop {
            match rustix::process::test_kill_process_group(group) {
                Err(rustix::io::Errno::SRCH) => break,
                Err(error) => panic!("process group状態確認: {error}"),
                Ok(()) if stopped_at.elapsed() >= Duration::from_secs(5) => {
                    panic!("Broker pipe EOF後も所有process groupが残存")
                }
                Ok(()) => thread::sleep(Duration::from_millis(10)),
            }
        }
        std::fs::remove_file(marker).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn guardian_loss_fails_closed_without_signaling_a_reusable_group_id() {
        use std::os::unix::process::CommandExt;
        use std::process::Stdio;

        let (guardian, control, group, group_raw) =
            spawn_linux_guardian().expect("独立Linux guardian起動");
        let child = Command::new("/bin/sleep")
            .arg("60")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(group_raw)
            .spawn()
            .expect("guardian所有group内のTask起動");
        let mut supervised = SupervisedChild {
            child,
            group: Some(group),
            guardian: Some(guardian),
            guardian_control: Some(control),
        };
        supervised
            .guardian
            .as_mut()
            .expect("監督対象の子process")
            .kill()
            .expect("failure fixtureでguardianを終了");

        let started = std::time::Instant::now();
        let error = loop {
            match supervised.try_wait() {
                Err(error) => break error,
                Ok(Some(_)) => panic!("guardian消失をTask成功へ昇格しない"),
                Ok(None) if started.elapsed() < std::time::Duration::from_secs(5) => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Ok(None) => panic!("終了したguardianを期限内に観測できない"),
            }
        };
        assert!(error.to_string().contains("guardian"));
        assert!(supervised.child.try_wait().unwrap().is_some());
        assert!(supervised.group.is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "spawn_linux_guardianからだけ起動するfixture"]
    fn guardian_process_entry() {
        crate::linux_process_group_guardian::run_from_stdin().expect("guardian監視pipe");
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
