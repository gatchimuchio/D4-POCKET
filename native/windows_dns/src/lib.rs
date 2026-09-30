//! Windows DNS Client APIの期限付き・取消可能な名前解決。
#![cfg(windows)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::cell::UnsafeCell;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{DNS_INFO_NO_RECORDS, DNS_REQUEST_PENDING};
use windows_sys::Win32::NetworkManagement::Dns::{
    DnsCancelQuery, DnsFree, DnsFreeRecordList, DnsQueryEx, DNS_AAAA_DATA, DNS_ADDR_ARRAY,
    DNS_QUERY_BYPASS_CACHE, DNS_QUERY_CANCEL, DNS_QUERY_REQUEST, DNS_QUERY_REQUEST_VERSION1,
    DNS_QUERY_RESULT, DNS_QUERY_RESULTS_VERSION1, DNS_QUERY_STANDARD, DNS_QUERY_TREAT_AS_FQDN,
    DNS_RECORDA, DNS_TYPE_A, DNS_TYPE_AAAA,
};

const QUERY_POLL_INTERVAL: Duration = Duration::from_millis(20);
const CANCEL_CALLBACK_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_DNS_RECORDS: usize = 128;
const DNS_STATUS_INVALID_RESPONSE: i32 = 9502;

static PENDING_QUERY: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveError {
    NameResolution,
    Cancelled,
    TimedOut,
    Busy,
    CancellationFailed,
    InvalidResponse,
}

struct QueryContext {
    result: UnsafeCell<DNS_QUERY_RESULT>,
    cancel_handle: UnsafeCell<DNS_QUERY_CANCEL>,
    completion_tx: mpsc::SyncSender<Result<Vec<IpAddr>, i32>>,
    completed: AtomicBool,
    query_name: Vec<u16>,
    server_list: Option<DNS_ADDR_ARRAY>,
}

// SAFETY: DNS_QUERY_RESULTとDNS_QUERY_CANCELはDnsQueryEx実行中だけDNS Clientが書き、
// 完了callback後に本crateが読む。各QueryContextはArcでcallback完了まで保持する。
unsafe impl Send for QueryContext {}
unsafe impl Sync for QueryContext {}

struct RecordList(*mut DNS_RECORDA);

impl Drop for RecordList {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                DnsFree(self.0.cast(), DnsFreeRecordList);
            }
        }
    }
}

/// Windows DNS Clientのsystem configurationを使い、指定期限内にA／AAAAを解決する。
///
/// 取消または期限到達時はDnsCancelQueryを呼び、callback未完了queryが複数残らないよう
/// process内のpending slotを保持する。取消callbackが規定時間に到着しなければ次の照会を拒否する。
pub fn resolve(
    host: &str,
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<Vec<IpAddr>, ResolveError> {
    resolve_with_server_list(host, cancel, deadline, None)
}

fn resolve_with_server_list(
    host: &str,
    cancel: &AtomicBool,
    deadline: Instant,
    server_list: Option<DNS_ADDR_ARRAY>,
) -> Result<Vec<IpAddr>, ResolveError> {
    if cancel.load(Ordering::Acquire) {
        return Err(ResolveError::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(ResolveError::TimedOut);
    }

    let mut addresses = Vec::new();
    for query_type in [DNS_TYPE_A, DNS_TYPE_AAAA] {
        if cancel.load(Ordering::Acquire) {
            return Err(ResolveError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(ResolveError::TimedOut);
        }
        addresses.extend(query(host, query_type, cancel, deadline, server_list)?);
    }
    addresses.sort_unstable();
    addresses.dedup();
    if addresses.is_empty() {
        return Err(ResolveError::NameResolution);
    }
    Ok(addresses)
}

fn query(
    host: &str,
    query_type: u16,
    cancel: &AtomicBool,
    deadline: Instant,
    server_list: Option<DNS_ADDR_ARRAY>,
) -> Result<Vec<IpAddr>, ResolveError> {
    if PENDING_QUERY
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(ResolveError::Busy);
    }

    let (completion_tx, completion_rx) = mpsc::sync_channel(1);
    let mut query_name = host
        .trim_end_matches('.')
        .encode_utf16()
        .collect::<Vec<_>>();
    query_name.push(b'.' as u16);
    query_name.push(0);
    let context = Arc::new(QueryContext {
        result: UnsafeCell::new(DNS_QUERY_RESULT {
            Version: DNS_QUERY_RESULTS_VERSION1,
            ..Default::default()
        }),
        cancel_handle: UnsafeCell::new(DNS_QUERY_CANCEL::default()),
        completion_tx,
        completed: AtomicBool::new(false),
        query_name,
        server_list,
    });
    let callback_context = Arc::into_raw(Arc::clone(&context));
    let request = DNS_QUERY_REQUEST {
        Version: DNS_QUERY_REQUEST_VERSION1,
        QueryName: context.query_name.as_ptr(),
        QueryType: query_type,
        QueryOptions: (DNS_QUERY_STANDARD
            | DNS_QUERY_TREAT_AS_FQDN
            | if context.server_list.is_some() {
                DNS_QUERY_BYPASS_CACHE
            } else {
                0
            }) as u64,
        pDnsServerList: context
            .server_list
            .as_ref()
            .map_or(std::ptr::null_mut(), |servers| {
                std::ptr::from_ref(servers).cast_mut()
            }),
        InterfaceIndex: 0,
        pQueryCompletionCallback: Some(query_complete),
        pQueryContext: callback_context.cast_mut().cast(),
    };

    let status = unsafe { DnsQueryEx(&request, context.result.get(), context.cancel_handle.get()) };
    if status != DNS_REQUEST_PENDING {
        unsafe {
            drop(Arc::from_raw(callback_context));
        }
        PENDING_QUERY.store(false, Ordering::Release);
        return unsafe { read_records(context.result.get()) }.map_err(map_query_status);
    }

    wait_for_query(context, completion_rx, cancel, deadline)
}

fn wait_for_query(
    context: Arc<QueryContext>,
    completion_rx: mpsc::Receiver<Result<Vec<IpAddr>, i32>>,
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<Vec<IpAddr>, ResolveError> {
    loop {
        if cancel.load(Ordering::Acquire) {
            return cancel_pending_query(&context, &completion_rx, ResolveError::Cancelled);
        }
        let now = Instant::now();
        if now >= deadline {
            return cancel_pending_query(&context, &completion_rx, ResolveError::TimedOut);
        }
        let wait = QUERY_POLL_INTERVAL.min(deadline.saturating_duration_since(now));
        match completion_rx.recv_timeout(wait) {
            Ok(result) => return result.map_err(map_query_status),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err(ResolveError::InvalidResponse),
        }
    }
}

fn cancel_pending_query(
    context: &QueryContext,
    completion_rx: &mpsc::Receiver<Result<Vec<IpAddr>, i32>>,
    outcome: ResolveError,
) -> Result<Vec<IpAddr>, ResolveError> {
    if !context.completed.load(Ordering::Acquire) {
        let status = unsafe { DnsCancelQuery(context.cancel_handle.get()) };
        if status != 0 && !context.completed.load(Ordering::Acquire) {
            return Err(ResolveError::CancellationFailed);
        }
    }
    match completion_rx.recv_timeout(CANCEL_CALLBACK_TIMEOUT) {
        Ok(_) => Err(outcome),
        Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {
            Err(ResolveError::CancellationFailed)
        }
    }
}

unsafe extern "system" fn query_complete(
    query_context: *const std::ffi::c_void,
    query_result: *mut DNS_QUERY_RESULT,
) {
    if query_context.is_null() {
        return;
    }
    let context = unsafe { Arc::from_raw(query_context.cast::<QueryContext>()) };
    context.completed.store(true, Ordering::Release);
    let result = catch_unwind(AssertUnwindSafe(|| unsafe { read_records(query_result) }))
        .unwrap_or(Err(DNS_STATUS_INVALID_RESPONSE));
    PENDING_QUERY.store(false, Ordering::Release);
    let _ = context.completion_tx.send(result);
}

unsafe fn read_records(query_result: *mut DNS_QUERY_RESULT) -> Result<Vec<IpAddr>, i32> {
    if query_result.is_null() {
        return Err(DNS_STATUS_INVALID_RESPONSE);
    }
    let status = unsafe { (*query_result).QueryStatus };
    let records = unsafe { (*query_result).pQueryRecords };
    let _records = RecordList(records);
    if status == DNS_INFO_NO_RECORDS {
        return Ok(Vec::new());
    }
    if status != 0 {
        return Err(status);
    }

    let mut addresses = Vec::new();
    let mut current = records;
    let mut count = 0usize;
    while !current.is_null() {
        count += 1;
        if count > MAX_DNS_RECORDS {
            return Err(DNS_STATUS_INVALID_RESPONSE);
        }
        match unsafe { (*current).wType } {
            DNS_TYPE_A => {
                let raw = unsafe { (*current).Data.A.IpAddress };
                addresses.push(IpAddr::V4(Ipv4Addr::from(raw.to_ne_bytes())));
            }
            DNS_TYPE_AAAA => {
                let data: DNS_AAAA_DATA = unsafe { (*current).Data.AAAA };
                let octets = unsafe { data.Ip6Address.IP6Byte };
                addresses.push(IpAddr::V6(Ipv6Addr::from(octets)));
            }
            _ => {}
        }
        current = unsafe { (*current).pNext };
    }
    Ok(addresses)
}

fn map_query_status(status: i32) -> ResolveError {
    if status == DNS_STATUS_INVALID_RESPONSE {
        ResolveError::InvalidResponse
    } else {
        ResolveError::NameResolution
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
    use std::sync::atomic::AtomicBool;
    use std::thread;
    use windows_sys::Win32::NetworkManagement::Dns::DNS_ADDR;

    fn local_dns_server_list(address: SocketAddr) -> DNS_ADDR_ARRAY {
        let SocketAddr::V4(address) = address else {
            panic!("DNS試験用serverはIPv4でbindする")
        };
        let mut server = DNS_ADDR::default();
        let mut socket_address = [0i8; 32];
        socket_address[0..2].copy_from_slice(&2u16.to_ne_bytes().map(|byte| byte as i8));
        socket_address[2..4].copy_from_slice(&0u16.to_ne_bytes().map(|byte| byte as i8));
        socket_address[4..8].copy_from_slice(&address.ip().octets().map(|byte| byte as i8));
        server.MaxSa = socket_address;
        let mut servers = DNS_ADDR_ARRAY {
            MaxCount: 1,
            AddrCount: 1,
            ..Default::default()
        };
        unsafe {
            std::ptr::addr_of_mut!(servers.AddrArray)
                .cast::<DNS_ADDR>()
                .write_unaligned(server);
        }
        servers
    }

    fn run_blackhole_dns_probe(deadline: Instant, cancel_after_packet: bool) -> ResolveError {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 53)).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let server_address = socket.local_addr().unwrap();
        let servers = local_dns_server_list(server_address);
        let (seen_tx, seen_rx) = mpsc::sync_channel(1);
        let shutdown = Arc::new(AtomicBool::new(false));
        let server_shutdown = Arc::clone(&shutdown);
        let server = thread::spawn(move || {
            let mut packet = [0u8; 1500];
            let mut notified = false;
            while !server_shutdown.load(Ordering::Acquire) {
                if socket.recv_from(&mut packet).is_ok() && !notified {
                    notified = true;
                    let _ = seen_tx.send(());
                }
            }
        });

        let cancel = Arc::new(AtomicBool::new(false));
        let query_cancel = Arc::clone(&cancel);
        let query = thread::spawn(move || {
            resolve_with_server_list(
                "cancel-probe.invalid",
                &query_cancel,
                deadline,
                Some(servers),
            )
        });
        let packet_seen = seen_rx.recv_timeout(Duration::from_secs(3)).is_ok();
        if cancel_after_packet || !packet_seen {
            cancel.store(true, Ordering::Release);
        }
        let result = query.join().unwrap().unwrap_err();
        shutdown.store(true, Ordering::Release);
        server.join().unwrap();
        assert!(
            packet_seen,
            "Windows DNS Clientがloopback fixtureへ照会しなかった: {result:?}"
        );
        result
    }

    fn answer_address_query(packet: &[u8]) -> Vec<u8> {
        assert!(packet.len() >= 17, "DNS query has a complete question");
        let mut question_end = 12usize;
        loop {
            let label_len = packet[question_end] as usize;
            question_end += 1;
            if label_len == 0 {
                break;
            }
            question_end += label_len;
            assert!(question_end < packet.len(), "DNS question label is bounded");
        }
        let query_type = u16::from_be_bytes([packet[question_end], packet[question_end + 1]]);
        let mut response = packet[..question_end + 4].to_vec();
        response[2] = 0x81;
        response[3] = 0x80;
        response[6] = 0;
        response[7] = 1;
        response[10] = 0;
        response[11] = 0;
        response.extend_from_slice(&[0xc0, 0x0c]);
        response.extend_from_slice(&query_type.to_be_bytes());
        response.extend_from_slice(&1u16.to_be_bytes());
        response.extend_from_slice(&60u32.to_be_bytes());
        match query_type {
            DNS_TYPE_A => {
                response.extend_from_slice(&4u16.to_be_bytes());
                response.extend_from_slice(&[1, 2, 3, 4]);
            }
            DNS_TYPE_AAAA => {
                response.extend_from_slice(&16u16.to_be_bytes());
                response.extend_from_slice(
                    &"2606:4700:4700::1111".parse::<Ipv6Addr>().unwrap().octets(),
                );
            }
            other => panic!("未対応のDNS query type: {other}"),
        }
        response
    }

    #[test]
    fn pre_cancelled_dns_request_fails_before_calling_windows_resolver() {
        let cancel = AtomicBool::new(true);
        assert_eq!(
            resolve_with_server_list(
                "cancel-probe.invalid",
                &cancel,
                Instant::now() + Duration::from_secs(5),
                None,
            ),
            Err(ResolveError::Cancelled)
        );
    }

    #[test]
    fn pending_dns_query_is_cancelled_through_windows_dns_client() {
        assert_eq!(
            run_blackhole_dns_probe(Instant::now() + Duration::from_secs(5), true),
            ResolveError::Cancelled
        );
    }

    #[test]
    fn pending_dns_query_has_a_finite_deadline_and_is_cancelled() {
        assert_eq!(
            run_blackhole_dns_probe(Instant::now() + Duration::from_secs(5), false),
            ResolveError::TimedOut
        );
    }

    #[test]
    fn windows_dns_client_returns_and_parses_bounded_dual_stack_answers() {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 53)).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let servers = local_dns_server_list(socket.local_addr().unwrap());
        let shutdown = Arc::new(AtomicBool::new(false));
        let server_shutdown = Arc::clone(&shutdown);
        let server = thread::spawn(move || {
            let mut packet = [0u8; 1500];
            while !server_shutdown.load(Ordering::Acquire) {
                if let Ok((length, peer)) = socket.recv_from(&mut packet) {
                    let response = answer_address_query(&packet[..length]);
                    let _ = socket.send_to(&response, peer);
                }
            }
        });

        let result = resolve_with_server_list(
            "answer-probe.invalid",
            &AtomicBool::new(false),
            Instant::now() + Duration::from_secs(5),
            Some(servers),
        );
        shutdown.store(true, Ordering::Release);
        server.join().unwrap();
        assert_eq!(
            result,
            Ok(vec![
                IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)),
                IpAddr::V6("2606:4700:4700::1111".parse().unwrap()),
            ])
        );
    }
}
