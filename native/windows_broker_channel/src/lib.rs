#![cfg(windows)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc::SyncSender, Arc};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_BROKEN_PIPE, ERROR_IO_PENDING, ERROR_NO_DATA,
    ERROR_PIPE_CONNECTED, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Storage::FileSystem::{
    ReadFile, WriteFile, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeClientProcessId,
    PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

const PIPE_PREFIX: &str = r"\\.\pipe\D4PocketBroker-";
const PIPE_IO_TIMEOUT: Duration = Duration::from_secs(5);
const SHUTDOWN_POLL: Duration = Duration::from_millis(50);
const CHUNK_BYTES: usize = 8192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipeError {
    code: &'static str,
}

impl PipeError {
    fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipeFrame {
    Line(Vec<u8>),
    Oversized,
}

/// 接続先札を起動ごとの128-bit乱数suffixから構成する。
pub fn pipe_name(suffix: &str) -> Result<String, PipeError> {
    if suffix.len() != 32
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PipeError::new("PIPE_NAME_INVALID"));
    }
    Ok(format!("{PIPE_PREFIX}{suffix}"))
}

/// 指定されたFlutter child process以外の接続を拒否し、各要求を一度だけ
/// callbackへ渡す。処理内容とnormal資格は呼出し側Rust Broker transportが所有する。
pub fn run_server<F>(
    name: String,
    expected_client_pid: Arc<AtomicU32>,
    shutdown: Arc<AtomicBool>,
    ready: SyncSender<()>,
    max_request_bytes: usize,
    max_response_bytes: usize,
    mut handle: F,
) -> Result<(), PipeError>
where
    F: FnMut(PipeFrame) -> Option<Vec<u8>>,
{
    if !name.starts_with(PIPE_PREFIX) || pipe_name(&name[PIPE_PREFIX.len()..])? != name {
        return Err(PipeError::new("PIPE_NAME_INVALID"));
    }
    if max_request_bytes == 0 || max_response_bytes == 0 {
        return Err(PipeError::new("PIPE_LIMIT_INVALID"));
    }

    let wide_name = wide_null(&name);
    let mut readiness_sent = false;
    while !shutdown.load(Ordering::Acquire) {
        let pipe = unsafe {
            CreateNamedPipeW(
                wide_name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                CHUNK_BYTES as u32,
                CHUNK_BYTES as u32,
                0,
                std::ptr::null(),
            )
        };
        if pipe == INVALID_HANDLE_VALUE {
            return Err(PipeError::new("PIPE_CREATE_FAILED"));
        }
        let pipe = OwnedHandle(pipe);
        if !readiness_sent {
            let _ = ready.send(());
            readiness_sent = true;
        }

        match connect_client(pipe.0, &shutdown) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(_error) if shutdown.load(Ordering::Acquire) => return Ok(()),
            Err(error) => return Err(error),
        }

        let mut client_pid = 0u32;
        let pid_read = unsafe { GetNamedPipeClientProcessId(pipe.0, &mut client_pid) };
        if pid_read == 0
            || client_pid == 0
            || client_pid != expected_client_pid.load(Ordering::Acquire)
        {
            unsafe {
                DisconnectNamedPipe(pipe.0);
            }
            continue;
        }

        if let Ok(frame) = read_frame(pipe.0, &shutdown, max_request_bytes) {
            if !shutdown.load(Ordering::Acquire) {
                if let Some(response) = handle(frame) {
                    if response.len() <= max_response_bytes {
                        let mut response = response;
                        response.push(b'\n');
                        let _ = write_all(pipe.0, &response, &shutdown);
                    }
                }
            }
        }
        unsafe {
            DisconnectNamedPipe(pipe.0);
        }
    }
    Ok(())
}

fn connect_client(
    pipe: windows_sys::Win32::Foundation::HANDLE,
    shutdown: &AtomicBool,
) -> Result<bool, PipeError> {
    let event = create_event()?;
    let mut overlapped = empty_overlapped(event.0);
    let connected = unsafe { ConnectNamedPipe(pipe, &mut overlapped) };
    if connected != 0 {
        return Ok(true);
    }
    match unsafe { GetLastError() } {
        ERROR_PIPE_CONNECTED => Ok(true),
        ERROR_IO_PENDING => {
            wait_overlapped(pipe, &overlapped, event.0, None, shutdown)?;
            let mut transferred = 0u32;
            if unsafe { GetOverlappedResult(pipe, &overlapped, &mut transferred, 0) } == 0 {
                Err(PipeError::new("PIPE_CONNECT_FAILED"))
            } else {
                Ok(true)
            }
        }
        _ => Err(PipeError::new("PIPE_CONNECT_FAILED")),
    }
}

fn read_frame(
    pipe: windows_sys::Win32::Foundation::HANDLE,
    shutdown: &AtomicBool,
    max_bytes: usize,
) -> Result<PipeFrame, PipeError> {
    let deadline = Instant::now() + PIPE_IO_TIMEOUT;
    let mut frame = Vec::new();
    loop {
        if shutdown.load(Ordering::Acquire) {
            return Err(PipeError::new("PIPE_SHUTDOWN"));
        }
        let mut chunk = [0u8; CHUNK_BYTES];
        let count = match read_some(pipe, &mut chunk, deadline, shutdown) {
            Ok(count) => count,
            Err(error) if error.code == "PIPE_PEER_CLOSED" => {
                return Ok(PipeFrame::Line(frame));
            }
            Err(error) => return Err(error),
        };
        if count == 0 {
            return Ok(PipeFrame::Line(frame));
        }
        if frame.len().saturating_add(count) > max_bytes {
            return Ok(PipeFrame::Oversized);
        }
        let bytes = &chunk[..count];
        if let Some(newline) = bytes.iter().position(|byte| *byte == b'\n') {
            if newline + 1 != bytes.len() {
                return Err(PipeError::new("PIPE_MULTIPLE_FRAMES"));
            }
            frame.extend_from_slice(&bytes[..newline]);
            if frame.last() == Some(&b'\r') {
                frame.pop();
            }
            return Ok(PipeFrame::Line(frame));
        }
        frame.extend_from_slice(bytes);
    }
}

fn read_some(
    pipe: windows_sys::Win32::Foundation::HANDLE,
    buffer: &mut [u8],
    deadline: Instant,
    shutdown: &AtomicBool,
) -> Result<usize, PipeError> {
    let event = create_event()?;
    let mut overlapped = empty_overlapped(event.0);
    let mut transferred = 0u32;
    let succeeded = unsafe {
        ReadFile(
            pipe,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            &mut transferred,
            &mut overlapped,
        )
    };
    if succeeded != 0 {
        return Ok(transferred as usize);
    }
    match unsafe { GetLastError() } {
        ERROR_BROKEN_PIPE | ERROR_NO_DATA => Err(PipeError::new("PIPE_PEER_CLOSED")),
        ERROR_IO_PENDING => {
            wait_overlapped(pipe, &overlapped, event.0, Some(deadline), shutdown)?;
            if unsafe { GetOverlappedResult(pipe, &overlapped, &mut transferred, 0) } == 0 {
                if matches!(unsafe { GetLastError() }, ERROR_BROKEN_PIPE | ERROR_NO_DATA) {
                    Err(PipeError::new("PIPE_PEER_CLOSED"))
                } else {
                    Err(PipeError::new("PIPE_READ_FAILED"))
                }
            } else {
                Ok(transferred as usize)
            }
        }
        _ => Err(PipeError::new("PIPE_READ_FAILED")),
    }
}

fn write_all(
    pipe: windows_sys::Win32::Foundation::HANDLE,
    bytes: &[u8],
    shutdown: &AtomicBool,
) -> Result<(), PipeError> {
    let deadline = Instant::now() + PIPE_IO_TIMEOUT;
    let mut offset = 0;
    while offset < bytes.len() {
        let event = create_event()?;
        let mut overlapped = empty_overlapped(event.0);
        let mut transferred = 0u32;
        let succeeded = unsafe {
            WriteFile(
                pipe,
                bytes[offset..].as_ptr(),
                (bytes.len() - offset) as u32,
                &mut transferred,
                &mut overlapped,
            )
        };
        if succeeded == 0 {
            match unsafe { GetLastError() } {
                ERROR_IO_PENDING => {
                    wait_overlapped(pipe, &overlapped, event.0, Some(deadline), shutdown)?;
                    if unsafe { GetOverlappedResult(pipe, &overlapped, &mut transferred, 0) } == 0 {
                        return Err(PipeError::new("PIPE_WRITE_FAILED"));
                    }
                }
                _ => return Err(PipeError::new("PIPE_WRITE_FAILED")),
            }
        }
        if transferred == 0 {
            return Err(PipeError::new("PIPE_WRITE_FAILED"));
        }
        offset += transferred as usize;
    }
    Ok(())
}

fn wait_overlapped(
    pipe: windows_sys::Win32::Foundation::HANDLE,
    overlapped: &OVERLAPPED,
    event: windows_sys::Win32::Foundation::HANDLE,
    deadline: Option<Instant>,
    shutdown: &AtomicBool,
) -> Result<(), PipeError> {
    loop {
        if shutdown.load(Ordering::Acquire)
            || deadline.is_some_and(|deadline| Instant::now() >= deadline)
        {
            unsafe {
                CancelIoEx(pipe, overlapped);
                WaitForSingleObject(event, u32::MAX);
            }
            return Err(PipeError::new("PIPE_IO_TIMEOUT"));
        }
        let wait_ms = deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .unwrap_or(SHUTDOWN_POLL)
            .min(SHUTDOWN_POLL)
            .as_millis()
            .max(1) as u32;
        match unsafe { WaitForSingleObject(event, wait_ms) } {
            WAIT_OBJECT_0 => return Ok(()),
            WAIT_TIMEOUT => {}
            _ => return Err(PipeError::new("PIPE_WAIT_FAILED")),
        }
    }
}

fn create_event() -> Result<OwnedHandle, PipeError> {
    let event = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
    if event == 0 {
        Err(PipeError::new("PIPE_EVENT_FAILED"))
    } else {
        Ok(OwnedHandle(event))
    }
}

fn empty_overlapped(event: windows_sys::Win32::Foundation::HANDLE) -> OVERLAPPED {
    let mut value: OVERLAPPED = unsafe { std::mem::zeroed() };
    value.hEvent = event;
    value
}

fn wide_null(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

struct OwnedHandle(windows_sys::Win32::Foundation::HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use std::sync::atomic::AtomicUsize;
    use std::thread;

    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, ReadFile, WriteFile, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Pipes::WaitNamedPipeW;
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;

    const TEST_PIPE_LIMIT: usize = 128;

    fn test_pipe_name() -> String {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        pipe_name(&format!("{nonce:032x}")).unwrap()
    }

    fn client_handle(name: &str) -> OwnedHandle {
        let wide: Vec<u16> = OsStr::new(name).encode_wide().chain(Some(0)).collect();
        assert_ne!(unsafe { WaitNamedPipeW(wide.as_ptr(), 2_000) }, 0);
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                0,
            )
        };
        assert_ne!(handle, INVALID_HANDLE_VALUE);
        OwnedHandle(handle)
    }

    fn client_exchange(name: &str, request: &[u8]) -> Option<Vec<u8>> {
        let client = client_handle(name);
        let mut written = 0u32;
        assert_ne!(
            unsafe {
                WriteFile(
                    client.0,
                    request.as_ptr(),
                    request.len() as u32,
                    &mut written,
                    std::ptr::null_mut(),
                )
            },
            0
        );
        assert_eq!(written as usize, request.len());
        let mut response = [0u8; 64];
        let mut read = 0u32;
        let ok = unsafe {
            ReadFile(
                client.0,
                response.as_mut_ptr(),
                response.len() as u32,
                &mut read,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 || read == 0 {
            None
        } else {
            Some(response[..read as usize].to_vec())
        }
    }

    #[test]
    fn pipe_suffix_is_lowercase_hex_and_fixed_length() {
        assert!(pipe_name("0a9b".repeat(8).as_str()).is_ok());
        assert!(pipe_name("0A9B".repeat(8).as_str()).is_err());
        assert!(pipe_name("a".repeat(31).as_str()).is_err());
        assert!(pipe_name(&format!("{}\\x", "a".repeat(31))).is_err());
    }

    #[test]
    fn only_the_expected_process_pid_can_exchange_a_frame() {
        let name = test_pipe_name();
        let shutdown = Arc::new(AtomicBool::new(false));
        let expected_pid = Arc::new(AtomicU32::new(unsafe { GetCurrentProcessId() }));
        let invoked = Arc::new(AtomicUsize::new(0));
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        let server_shutdown = Arc::clone(&shutdown);
        let server_pid = Arc::clone(&expected_pid);
        let server_invoked = Arc::clone(&invoked);
        let server_name = name.clone();
        let server = thread::spawn(move || {
            run_server(
                server_name,
                server_pid,
                server_shutdown,
                ready_tx,
                TEST_PIPE_LIMIT,
                TEST_PIPE_LIMIT,
                move |frame| {
                    server_invoked.fetch_add(1, Ordering::SeqCst);
                    match frame {
                        PipeFrame::Line(bytes) => Some(bytes),
                        PipeFrame::Oversized => Some(b"oversized".to_vec()),
                    }
                },
            )
        });
        ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(
            client_exchange(&name, b"request\n"),
            Some(b"request\n".to_vec())
        );
        assert_eq!(invoked.load(Ordering::SeqCst), 1);

        let other_pid = unsafe { GetCurrentProcessId() }.wrapping_add(1).max(1);
        expected_pid.store(other_pid, Ordering::Release);
        assert_eq!(client_exchange(&name, b"forbidden\n"), None);
        assert_eq!(invoked.load(Ordering::SeqCst), 1);

        shutdown.store(true, Ordering::Release);
        server.join().unwrap().unwrap();
    }
}
