#![cfg(target_os = "linux")]

use std::io::{self, Read};

/// `gui_shell_linux_broker`の内部起動専用引数。
pub const INTERNAL_ARGUMENT: &str = "--internal-process-group-guardian-v1";

/// Brokerが保持する監視pipeのEOF後、guardian自身が所有するprocess groupを停止する。
/// PIDとPGIDが一致しない起動ではsignalを送らず、誤った呼出元groupを保護する。
pub fn run_from_stdin() -> io::Result<()> {
    let process = rustix::process::getpid();
    let group = rustix::process::getpgrp();
    if process != group {
        return Err(io::Error::other(
            "guardianは専用process groupのleaderとして起動する必要がある",
        ));
    }

    let mut input = io::stdin().lock();
    let mut byte = [0_u8; 1];
    loop {
        match input.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "guardian監視pipeに予期しないdataがある",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }

    match rustix::process::kill_process_group(group, rustix::process::Signal::KILL) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn guardian_refuses_to_signal_a_group_it_does_not_lead() {
        let process = rustix::process::getpid();
        let group = rustix::process::getpgrp();
        if process != group {
            assert!(super::run_from_stdin().is_err());
        }
    }
}
