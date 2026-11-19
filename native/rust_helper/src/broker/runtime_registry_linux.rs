//! Linux native runtime observation through read-only kernel interfaces.
//!
//! The Broker never accepts a PID from IPC. It queries the registered loopback
//! listener with SOCK_DIAG, resolves that socket inode only among the current
//! user's visible process descriptors, then rechecks the same endpoint around
//! each `/proc/<pid>` sample. Any ambiguity or collection limit is unknown.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::os::unix::fs::MetadataExt;
use std::time::{Duration, Instant};

use rustix::net::{
    self, netlink, sockopt, AddressFamily, RecvFlags, SendFlags, SocketFlags, SocketType,
};

use super::ProcessSample;

const SOCK_DIAG_BY_FAMILY: u16 = 20;
const TCP_LISTEN: u8 = 10;
const NETLINK_REQUEST_DUMP: u16 = 0x301;
const NETLINK_DUMP_INTERRUPTED: u16 = 0x10;
const MAX_DIAG_PACKETS: usize = 32;
const OBSERVATION_BUDGET: Duration = Duration::from_millis(250);
const MAX_PROC_ENTRIES: usize = 4096;
const MAX_FD_ENTRIES: usize = 65_536;

pub(super) fn listener_owner_pid(target: SocketAddrV4) -> Result<Option<u32>, ()> {
    if *target.ip() != Ipv4Addr::LOCALHOST || target.port() == 0 {
        return Ok(None);
    }

    let Some((inode, socket_uid)) = listener_inode(target)? else {
        return Ok(None);
    };
    // Runtime observation is scoped to the Broker user's Runtime. A listener
    // owned by another UID is unavailable rather than a reason to escalate.
    let current_uid = rustix::process::geteuid().as_raw();
    if socket_uid != current_uid {
        return Ok(None);
    }
    find_socket_owner(inode, socket_uid)
}

fn listener_inode(target: SocketAddrV4) -> Result<Option<(u32, u32)>, ()> {
    let socket = net::socket_with(
        AddressFamily::NETLINK,
        SocketType::DGRAM,
        SocketFlags::CLOEXEC,
        Some(netlink::SOCK_DIAG),
    )
    .map_err(|_| ())?;
    sockopt::set_socket_timeout(&socket, sockopt::Timeout::Recv, Some(OBSERVATION_BUDGET))
        .map_err(|_| ())?;

    let kernel = netlink::SocketAddrNetlink::new(0, 0);
    let request = diag_request(target);
    if net::sendto(&socket, &request, SendFlags::empty(), &kernel).map_err(|_| ())? != request.len()
    {
        return Err(());
    }

    // 二つ以上の listener が SO_REUSEPORT で同じ接続先を共有し得る。
    // 一件だけ返す exact 照会は使わず、列挙が完了してから一意性を確定する。
    let deadline = Instant::now() + OBSERVATION_BUDGET;
    let mut listener = None;
    for _ in 0..MAX_DIAG_PACKETS {
        if Instant::now() >= deadline {
            return Err(());
        }
        let mut response = [0_u8; 16 * 1024];
        let (_, received, peer) =
            net::recvfrom(&socket, &mut response[..], RecvFlags::TRUNC).map_err(|_| ())?;
        let peer = netlink::SocketAddrNetlink::try_from(peer.ok_or(())?).map_err(|_| ())?;
        if peer != kernel || received < 16 || received > response.len() {
            return Err(());
        }
        let mut packet = &response[..received];
        while !packet.is_empty() {
            let length = read_native_u32(packet, 0).ok_or(())? as usize;
            let message_type = read_native_u16(packet, 4).ok_or(())?;
            let flags = read_native_u16(packet, 6).ok_or(())?;
            let sequence = read_native_u32(packet, 8).ok_or(())?;
            if sequence != 1
                || flags & NETLINK_DUMP_INTERRUPTED != 0
                || length < 16
                || length > packet.len()
            {
                return Err(());
            }
            let body = &packet[16..length];
            match i32::from(message_type) {
                libc::NLMSG_DONE if read_native_i32(body, 0) == Some(0) => {
                    return Ok(listener);
                }
                value if value == i32::from(SOCK_DIAG_BY_FAMILY) => {
                    if let Some(candidate) = matching_listener(body, target)? {
                        if listener.replace(candidate).is_some() {
                            return Ok(None);
                        }
                    }
                }
                _ => return Err(()),
            }
            let aligned_length = (length + 3) & !3;
            packet = packet.get(aligned_length..).ok_or(())?;
        }
    }
    Err(())
}

fn diag_request(target: SocketAddrV4) -> [u8; 72] {
    // Linux UAPI: nlmsghdr (16 bytes) + inet_diag_req_v2 (56 bytes).
    let mut request = [0_u8; 72];
    request[0..4].copy_from_slice(&72_u32.to_ne_bytes());
    request[4..6].copy_from_slice(&SOCK_DIAG_BY_FAMILY.to_ne_bytes());
    request[6..8].copy_from_slice(&NETLINK_REQUEST_DUMP.to_ne_bytes());
    request[8..12].copy_from_slice(&1_u32.to_ne_bytes());

    request[16] = libc::AF_INET as u8;
    request[17] = libc::IPPROTO_TCP as u8;
    request[20..24].copy_from_slice(&(1_u32 << TCP_LISTEN).to_ne_bytes());
    request[24..26].copy_from_slice(&target.port().to_be_bytes());
    request[28..32].copy_from_slice(&target.ip().octets());
    request[64..72].fill(0xff); // INET_DIAG_NOCOOKIE
    request
}

fn matching_listener(body: &[u8], target: SocketAddrV4) -> Result<Option<(u32, u32)>, ()> {
    if body.len() < 72 {
        return Err(());
    }
    if body[0] != libc::AF_INET as u8
        || body[1] != TCP_LISTEN
        || u16::from_be_bytes([body[4], body[5]]) != target.port()
        || body[8..12] != target.ip().octets()
    {
        return Ok(None);
    }
    let socket_uid = read_native_u32(body, 64).ok_or(())?;
    let inode = read_native_u32(body, 68).ok_or(())?;
    if inode == 0 {
        return Err(());
    }
    Ok(Some((inode, socket_uid)))
}

fn find_socket_owner(inode: u32, expected_uid: u32) -> Result<Option<u32>, ()> {
    let processes = fs::read_dir("/proc").map_err(|_| ())?;
    let deadline = Instant::now() + OBSERVATION_BUDGET;
    let mut process_count = 0_usize;
    let mut descriptor_count = 0_usize;
    let mut owners = BTreeSet::new();
    let expected_link = format!("socket:[{inode}]");

    for process in processes {
        let process = process.map_err(|_| ())?;
        let Some(pid) = process
            .file_name()
            .to_str()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|pid| *pid > 0)
        else {
            continue;
        };
        process_count += 1;
        if process_count > MAX_PROC_ENTRIES || Instant::now() >= deadline {
            return Err(());
        }

        let process_path = process.path();
        let metadata = match fs::metadata(&process_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(_) => return Err(()),
        };
        if metadata.uid() != expected_uid {
            continue;
        }
        let descriptors = match fs::read_dir(process_path.join("fd")) {
            Ok(descriptors) => descriptors,
            Err(error) if unavailable_descriptor(&error) => continue,
            Err(_) => return Err(()),
        };
        for descriptor in descriptors {
            let descriptor = descriptor.map_err(|_| ())?;
            descriptor_count += 1;
            if descriptor_count > MAX_FD_ENTRIES || Instant::now() >= deadline {
                return Err(());
            }
            let link = match fs::read_link(descriptor.path()) {
                Ok(link) => link,
                Err(error) if unavailable_descriptor(&error) => continue,
                Err(_) => return Err(()),
            };
            if link.to_string_lossy() == expected_link {
                owners.insert(pid);
                if owners.len() > 1 {
                    return Ok(None);
                }
            }
        }
    }
    Ok(owners.into_iter().next())
}

fn unavailable_descriptor(error: &io::Error) -> bool {
    // 同じUIDでも保護されたprocessのfdは読めない。見えないfdからPIDを推定しない。
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
    ) || error.raw_os_error() == Some(libc::ESRCH)
}

pub(super) fn capture_process(pid: u32) -> Result<ProcessSample, ()> {
    if pid == 0 {
        return Err(());
    }
    let process = read_process_stat(pid)?;
    let ticks_per_second = rustix::param::clock_ticks_per_second();
    let page_size = rustix::param::page_size();
    if ticks_per_second == 0 || page_size == 0 {
        return Err(());
    }
    let cpu_ticks = process
        .user_ticks
        .checked_add(process.system_ticks)
        .ok_or(())?;
    let rss_bytes = process
        .resident_pages
        .checked_mul(page_size as u64)
        .ok_or(())?;
    Ok(ProcessSample {
        creation_time_100ns: ticks_to_100ns(process.start_ticks, ticks_per_second).ok_or(())?,
        creation_time_unix_millis: process_creation_unix_millis(
            process.start_ticks,
            ticks_per_second,
        ),
        total_cpu_100ns: ticks_to_100ns(cpu_ticks, ticks_per_second).ok_or(())?,
        working_set_bytes: rss_bytes,
        private_bytes: read_private_bytes(pid),
        disk_io_bytes: read_disk_io_bytes(pid),
        sampled_at: Instant::now(),
    })
}

struct ProcStat {
    user_ticks: u64,
    system_ticks: u64,
    start_ticks: u64,
    resident_pages: u64,
}

fn read_process_stat(pid: u32) -> Result<ProcStat, ()> {
    let raw = fs::read_to_string(format!("/proc/{pid}/stat")).map_err(|_| ())?;
    parse_process_stat(&raw).ok_or(())
}

fn parse_process_stat(raw: &str) -> Option<ProcStat> {
    // comm may contain spaces and parentheses; the closing delimiter before
    // state and numeric fields is the final ')' in the kernel record.
    let close = raw.rfind(')')?;
    let fields = raw.get(close + 1..)?.split_whitespace().collect::<Vec<_>>();
    if fields.first()?.len() != 1 {
        return None;
    }
    let resident_pages = fields.get(21)?.parse::<i64>().ok()?;
    Some(ProcStat {
        user_ticks: fields.get(11)?.parse().ok()?,
        system_ticks: fields.get(12)?.parse().ok()?,
        start_ticks: fields.get(19)?.parse().ok()?,
        resident_pages: u64::try_from(resident_pages).ok()?,
    })
}

fn ticks_to_100ns(ticks: u64, ticks_per_second: u64) -> Option<u64> {
    if ticks_per_second == 0 {
        return None;
    }
    u64::try_from(u128::from(ticks).checked_mul(10_000_000)? / u128::from(ticks_per_second)).ok()
}

fn process_creation_unix_millis(start_ticks: u64, ticks_per_second: u64) -> Option<i64> {
    let boot_seconds = fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("btime "))?
        .parse::<u64>()
        .ok()?;
    let boot_millis = u128::from(boot_seconds).checked_mul(1000)?;
    let process_millis = u128::from(start_ticks)
        .checked_mul(1000)?
        .checked_div(u128::from(ticks_per_second))?;
    i64::try_from(boot_millis.checked_add(process_millis)?).ok()
}

fn read_private_bytes(pid: u32) -> Option<u64> {
    let content = fs::read_to_string(format!("/proc/{pid}/smaps_rollup")).ok()?;
    let mut clean: Option<u64> = None;
    let mut dirty: Option<u64> = None;
    let mut hugetlb: Option<u64> = None;
    for line in content.lines() {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let target = match name {
            "Private_Clean" => &mut clean,
            "Private_Dirty" => &mut dirty,
            "Private_Hugetlb" => &mut hugetlb,
            _ => continue,
        };
        let mut fields = rest.split_whitespace();
        let kilobytes = fields.next()?.parse::<u64>().ok()?;
        if fields.next()? != "kB" || target.replace(kilobytes).is_some() {
            return None;
        }
    }
    clean?
        .checked_add(dirty?)?
        .checked_add(hugetlb?)?
        .checked_mul(1024)
}

fn read_disk_io_bytes(pid: u32) -> Option<u64> {
    let content = fs::read_to_string(format!("/proc/{pid}/io")).ok()?;
    let mut read_bytes = None;
    let mut write_bytes = None;
    for line in content.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let target = match name {
            "read_bytes" => &mut read_bytes,
            "write_bytes" => &mut write_bytes,
            _ => continue,
        };
        if target.replace(value.trim().parse::<u64>().ok()?).is_some() {
            return None;
        }
    }
    read_bytes?.checked_add(write_bytes?)
}

fn read_native_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_ne_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_native_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_ne_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_native_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_ne_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::minidora::MinidoraAdapter;
    use crate::broker::protocol::{Broker, BrokerOperation, BrokerRequestEnvelope, BrokerStatus};
    use serde_json::json;
    use std::net::TcpListener;
    use std::sync::Arc;

    fn reusable_loopback_listener(port: u16) -> TcpListener {
        let socket = net::socket_with(
            AddressFamily::INET,
            SocketType::STREAM,
            SocketFlags::CLOEXEC,
            Some(net::ipproto::TCP),
        )
        .unwrap();
        sockopt::set_socket_reuseport(&socket, true).unwrap();
        net::bind(&socket, &SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)).unwrap();
        net::listen(&socket, 1).unwrap();
        TcpListener::from(socket)
    }

    #[test]
    fn linux_runtime_resource_rejects_ambiguous_loopback_listener() {
        let first = reusable_loopback_listener(0);
        let std::net::SocketAddr::V4(target) = first.local_addr().unwrap() else {
            panic!("IPv4 loopback listener expected");
        };
        assert_eq!(listener_owner_pid(target), Ok(Some(std::process::id())));
        let _second = reusable_loopback_listener(target.port());
        assert_eq!(listener_owner_pid(target), Ok(None));
    }

    #[test]
    fn linux_runtime_resource_sock_diag結合は実listenerのinodeからBroker所有PIDを特定する() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("loopback listener");
        let target = match listener.local_addr().expect("listener address") {
            std::net::SocketAddr::V4(target) => target,
            _ => panic!("IPv4 loopback listener expected"),
        };
        assert_eq!(
            listener_owner_pid(target).expect("SOCK_DIAG query"),
            Some(std::process::id())
        );
    }

    #[test]
    fn linux_runtime_resource_proc観測は現在processの実CPU_RAM及びDiskIOを読む() {
        let sample = capture_process(std::process::id()).expect("current process sample");
        assert!(sample.creation_time_100ns > 0);
        assert!(sample
            .creation_time_unix_millis
            .is_some_and(|value| value > 0));
        assert!(sample.working_set_bytes > 0);
        assert!(sample.private_bytes.is_some());
        assert!(sample.disk_io_bytes.is_some());
    }

    #[test]
    fn linux_runtime_resource_brokerは実listenerの観測と失効を返す() {
        let query = |request_id: &str| {
            let mut request = BrokerRequestEnvelope::command_envelope_at(
                request_id,
                "resource-session",
                request_id,
                &BrokerRequestEnvelope::current_issued_at(),
            );
            request.operation = Some(BrokerOperation::実行系資源観測);
            request.payload = Some(json!({"版": 1, "実行系ID": "local"}));
            request.refresh_payload_hash();
            request
        };
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("loopback listener");
        let target = listener.local_addr().expect("listener address");
        let adapter = MinidoraAdapter::new(&target.to_string()).unwrap();
        let mut broker = Broker::new("resource-session");
        broker.実行系登録("local", Arc::new(adapter)).unwrap();

        let observation = broker.handle(query("resource-before"));
        assert_eq!(observation.status, BrokerStatus::Accepted);
        assert_eq!(observation.evidence_source, "LIVE_RUNTIME");
        let body = observation.body.unwrap();
        assert_eq!(body["観測監査ID"], observation.audit_event_id);
        assert_eq!(body["結合"]["状態"], "bound");
        assert_eq!(body["結合"]["PID"], std::process::id());
        assert_eq!(body["計測"]["RAMWorkingSetBytes"]["状態"], "measured");
        assert_eq!(body["計測"]["DiskIOBytes"]["証拠種別"], "LIVE_RUNTIME");

        drop(listener);
        let invalidated = broker.handle(query("resource-after"));
        assert_eq!(invalidated.status, BrokerStatus::Accepted);
        assert_eq!(invalidated.evidence_source, "INTERNAL_STATE");
        let body = invalidated.body.unwrap();
        assert_eq!(body["結合"]["状態"], "binding_mismatch");
        assert!(body["結合"]["PID"].is_null());
        assert_eq!(body["短期履歴"], json!([]));
        assert!(body["計測"]
            .as_object()
            .unwrap()
            .values()
            .all(|metric| { metric["状態"] == "unknown" && metric["値"].is_null() }));
    }
}
