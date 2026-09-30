use std::ffi::c_void;
use std::io;
use std::mem::size_of;
use std::os::windows::io::RawHandle;
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, SetLastError, ERROR_NO_MORE_FILES, HANDLE, INVALID_HANDLE_VALUE,
};
#[cfg(test)]
use windows_sys::Win32::Foundation::{ERROR_INVALID_PARAMETER, WAIT_OBJECT_0};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAccountingInformation,
    JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
    TerminateJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
#[cfg(test)]
use windows_sys::Win32::System::Threading::{
    OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
};
use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

const TREE_STOP_TIMEOUT: Duration = Duration::from_secs(5);
const TERMINATION_EXIT_CODE: u32 = 0xD4;

/// Process群をBrokerの生存期間に結びつけるWindows Job Object。
pub struct Job(HANDLE);

impl Job {
    pub fn create() -> io::Result<Self> {
        // SAFETY: 安全属性と名前をnullにし、名前なし・既定保護のJob Objectを作成する。
        let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let job = Self(handle);
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: limitsは正しく初期化され、API呼出し中も有効な領域に保持される。
        let configured = unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const c_void,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if configured == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(job)
    }

    /// 停止中の初期threadが再開する前にprocessをJobへ割り当てる。
    pub fn assign_and_resume(&self, process_id: u32, process_handle: RawHandle) -> io::Result<()> {
        let process_handle = process_handle as HANDLE;
        // SAFETY: 呼出側はstd::process::Childが返した有効なprocess handleを渡す。
        if unsafe { AssignProcessToJobObject(self.0, process_handle) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let thread_id = primary_suspended_thread_id(process_id)?;
        // SAFETY: IDはToolhelp snapshotから取得し、thread再開に必要なaccessだけを要求する。
        let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, thread_id) };
        if thread.is_null() {
            return Err(io::Error::last_os_error());
        }
        let _thread = OwnedHandle(thread);
        // SAFETY: threadはTHREAD_SUSPEND_RESUME権限で開いた有効なhandleである。
        let previous_suspend_count = unsafe { ResumeThread(thread) };
        match previous_suspend_count {
            u32::MAX => Err(io::Error::last_os_error()),
            1 => Ok(()),
            _ => Err(io::Error::other("停止中processの初期suspend状態が想定外")),
        }
    }

    pub fn terminate_and_wait(&self) -> io::Result<()> {
        if self.active_processes()? > 0 {
            // SAFETY: self.0はこの構造体が所有するJob Object handleである。
            if unsafe { TerminateJobObject(self.0, TERMINATION_EXIT_CODE) } == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        let started = Instant::now();
        loop {
            if self.active_processes()? == 0 {
                return Ok(());
            }
            if started.elapsed() >= TREE_STOP_TIMEOUT {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Windows Job Object内のprocess終了確認が期限を超過した",
                ));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn active_processes(&self) -> io::Result<u32> {
        let mut information = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        // SAFETY: informationは要求した情報classに合う大きさの書込可能領域である。
        let queried = unsafe {
            QueryInformationJobObject(
                self.0,
                JobObjectBasicAccountingInformation,
                &mut information as *mut _ as *mut c_void,
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                ptr::null_mut(),
            )
        };
        if queried == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(information.ActiveProcesses)
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: handleは単独所有であり、最後の防護としてKILL_ON_JOB_CLOSEも設定済み。
        unsafe {
            let _ = TerminateJobObject(self.0, TERMINATION_EXIT_CODE);
            CloseHandle(self.0);
        }
    }
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: このwrapperはOpenThreadまたはsnapshot作成が返したhandleを単独所有する。
        unsafe { CloseHandle(self.0) };
    }
}

fn primary_suspended_thread_id(process_id: u32) -> io::Result<u32> {
    // SAFETY: snapshot flagsとprocess IDはToolhelp APIの契約に従っている。
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let _snapshot = OwnedHandle(snapshot);
    let mut entry = THREADENTRY32::default();
    entry.dwSize = size_of::<THREADENTRY32>() as u32;
    // SAFETY: entryは初期化済みで、API呼出し中に書込可能な状態を保つ。
    if unsafe { Thread32First(snapshot, &mut entry) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut primary = None;
    loop {
        if entry.th32OwnerProcessID == process_id {
            if primary.replace(entry.th32ThreadID).is_some() {
                return Err(io::Error::other(
                    "停止中processに複数threadがあり初期threadを一意に識別できない",
                ));
            }
        }
        // SAFETY: entryは有効なTHREADENTRY32の書込先であり、last-errorで終端を識別する。
        unsafe { SetLastError(0) };
        // SAFETY: snapshot handleとentryは有効であり、snapshotの次のthreadを取得する。
        if unsafe { Thread32Next(snapshot, &mut entry) } == 0 {
            // SAFETY: GetLastErrorは直前のAPI呼出しが設定した状態を読み取る。
            let error = unsafe { GetLastError() };
            if error != ERROR_NO_MORE_FILES {
                return Err(io::Error::from_raw_os_error(error as i32));
            }
            break;
        }
    }
    primary.ok_or_else(|| io::Error::other("停止中processの初期threadが見つからない"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::windows::{io::AsRawHandle, process::CommandExt};
    use std::process::{Command, Stdio};

    const OWNER_MARKER: &str = "GUI_SHELL_PROCESS_JOB_OWNER_MARKER";
    const CHILD_FIXTURE: &str = "GUI_SHELL_PROCESS_JOB_CHILD_FIXTURE";
    const DESCENDANT_MARKER: &str = "GUI_SHELL_PROCESS_JOB_DESCENDANT_MARKER";
    const DESCENDANT_FIXTURE: &str = "GUI_SHELL_PROCESS_JOB_DESCENDANT_FIXTURE";

    #[test]
    fn abrupt_broker_exit_closes_job_and_terminates_descendants() {
        if let Some(marker) = std::env::var_os(OWNER_MARKER) {
            let marker = std::path::PathBuf::from(marker);
            let descendant_marker = marker.with_extension("descendant");
            let job = Job::create().expect("Job Object作成");
            start_persistent_tree(&job, &marker, &descendant_marker)
                .expect("Job内の子・孫process起動");
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時計")
            .as_nanos();
        let marker = std::env::temp_dir().join(format!(
            "gui-shell-process-job-{}-{nonce}",
            std::process::id()
        ));
        let descendant_marker = marker.with_extension("descendant");
        let executable = std::env::current_exe().expect("試験実行file");
        let mut owner = Command::new(executable)
            .args([
                "--exact",
                "windows_job::tests::abrupt_broker_exit_closes_job_and_terminates_descendants",
                "--nocapture",
            ])
            .env(OWNER_MARKER, &marker)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Job owner process起動");

        let started = Instant::now();
        let (child_id, descendant_id) = loop {
            let child = fs::read_to_string(&marker);
            let descendant = fs::read_to_string(&descendant_marker);
            if let (Ok(child), Ok(descendant)) = (child, descendant) {
                break (
                    child.parse::<u32>().expect("子process ID"),
                    descendant.parse::<u32>().expect("孫process ID"),
                );
            }
            if started.elapsed() > Duration::from_secs(10)
                || owner.try_wait().expect("owner状態").is_some()
            {
                let _ = owner.kill();
                let _ = owner.wait();
                let _ = fs::remove_file(&marker);
                let _ = fs::remove_file(&descendant_marker);
                panic!("Job ownerが子・孫process登録を完了しない");
            }
            thread::sleep(Duration::from_millis(10));
        };

        owner.kill().expect("Broker相当processを強制終了");
        owner.wait().expect("Broker相当process回収");
        wait_for_process_exit(child_id, Duration::from_secs(10))
            .expect("Job handle close後の子process終了");
        wait_for_process_exit(descendant_id, Duration::from_secs(10))
            .expect("Job handle close後の孫process終了");
        fs::remove_file(marker).expect("marker削除");
        fs::remove_file(descendant_marker).expect("孫process marker削除");
    }

    #[test]
    fn explicit_termination_stops_descendants_and_waits_for_empty_job() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時計")
            .as_nanos();
        let base = std::env::temp_dir().join(format!(
            "gui-shell-process-job-cancel-{}-{nonce}",
            std::process::id()
        ));
        let child_marker = base.with_extension("child");
        let descendant_marker = base.with_extension("descendant");
        let job = Job::create().expect("Job Object作成");
        let child_id = start_persistent_tree(&job, &child_marker, &descendant_marker)
            .expect("Job内の子・孫process起動");

        let started = Instant::now();
        let descendant_id = loop {
            if let Ok(value) = fs::read_to_string(&descendant_marker) {
                break value.parse::<u32>().expect("孫process ID");
            }
            if started.elapsed() > Duration::from_secs(10) {
                let _ = job.terminate_and_wait();
                let _ = fs::remove_file(&child_marker);
                let _ = fs::remove_file(&descendant_marker);
                panic!("Job内の孫process起動を確認できない");
            }
            thread::sleep(Duration::from_millis(10));
        };

        job.terminate_and_wait()
            .expect("取消相当のJob停止とprocess群回収");
        assert_eq!(job.active_processes().expect("Job内process数"), 0);
        wait_for_process_exit(child_id, Duration::from_secs(10)).expect("取消後の子process終了");
        wait_for_process_exit(descendant_id, Duration::from_secs(10))
            .expect("取消後の孫process終了");
        fs::remove_file(child_marker).expect("子process marker削除");
        fs::remove_file(descendant_marker).expect("孫process marker削除");
    }

    #[test]
    fn persistent_child_fixture() {
        if std::env::var_os(CHILD_FIXTURE).is_some() {
            let marker = std::path::PathBuf::from(
                std::env::var_os(DESCENDANT_MARKER).expect("孫process marker path"),
            );
            let executable = std::env::current_exe().expect("試験実行file");
            let descendant = Command::new(executable)
                .args([
                    "--exact",
                    "windows_job::tests::persistent_descendant_fixture",
                    "--nocapture",
                ])
                .env(DESCENDANT_FIXTURE, "1")
                .env_remove(CHILD_FIXTURE)
                .env_remove(OWNER_MARKER)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("Jobを継承する孫process起動");
            fs::write(marker, descendant.id().to_string()).expect("孫process ID記録");
            drop(descendant);
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
    }

    #[test]
    fn persistent_descendant_fixture() {
        if std::env::var_os(DESCENDANT_FIXTURE).is_some() {
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
    }

    fn start_persistent_tree(
        job: &Job,
        child_marker: &std::path::Path,
        descendant_marker: &std::path::Path,
    ) -> io::Result<u32> {
        let executable = std::env::current_exe()?;
        let mut command = Command::new(executable);
        command
            .args([
                "--exact",
                "windows_job::tests::persistent_child_fixture",
                "--nocapture",
            ])
            .env(CHILD_FIXTURE, "1")
            .env(DESCENDANT_MARKER, descendant_marker)
            .env_remove(OWNER_MARKER)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(crate::CREATE_SUSPENDED);
        let child = command.spawn()?;
        if let Err(error) = job.assign_and_resume(child.id(), child.as_raw_handle()) {
            let mut child = child;
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        let child_id = child.id();
        drop(child);
        if let Err(error) = fs::write(child_marker, child_id.to_string()) {
            let _ = job.terminate_and_wait();
            return Err(error);
        }
        Ok(child_id)
    }

    fn wait_for_process_exit(process_id: u32, timeout: Duration) -> io::Result<()> {
        // SAFETY: 観測対象の子process IDへ同期専用handleを開く。
        let process = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, process_id) };
        if process.is_null() {
            // SAFETY: 直前のOpenProcess呼出しが設定したerrorを読み取る。
            let error = unsafe { GetLastError() };
            return if error == ERROR_INVALID_PARAMETER {
                Ok(())
            } else {
                Err(io::Error::from_raw_os_error(error as i32))
            };
        }
        let process = OwnedHandle(process);
        let millis = timeout.as_millis().min(u32::MAX as u128) as u32;
        // SAFETY: processはPROCESS_SYNCHRONIZE権限で開いた有効なhandleである。
        if unsafe { WaitForSingleObject(process.0, millis) } == WAIT_OBJECT_0 {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Job Objectを閉じても子processが終了しない",
            ))
        }
    }
}
