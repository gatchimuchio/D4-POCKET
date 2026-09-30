//! Windows DNS helperの結果をBroker download failureへ射影する。

use super::DownloadError;
use std::net::IpAddr;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

pub(super) fn resolve(
    host: &str,
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<Vec<IpAddr>, DownloadError> {
    gui_shell_windows_dns::resolve(host, cancel, deadline).map_err(|error| match error {
        gui_shell_windows_dns::ResolveError::NameResolution => DownloadError::NameResolution,
        gui_shell_windows_dns::ResolveError::Cancelled => DownloadError::Cancelled,
        gui_shell_windows_dns::ResolveError::TimedOut => DownloadError::TimedOut,
        gui_shell_windows_dns::ResolveError::Busy => DownloadError::ResolverBusy,
        gui_shell_windows_dns::ResolveError::CancellationFailed => {
            DownloadError::DnsCancellationFailed
        }
        gui_shell_windows_dns::ResolveError::InvalidResponse => DownloadError::Network,
    })
}
