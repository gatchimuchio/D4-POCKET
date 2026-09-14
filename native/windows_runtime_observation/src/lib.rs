//! WindowsのIPv4 loopback TCP listenerを、OSが返す一意のPIDへ限定して照合する。
//!
//! このcrateはOS観測だけを担い、Permission、Approval、IPC、process制御、
//! runtime登録を所有しない。返すPIDはlistenerのOS上の所有者候補であり、
//! runtime同一性または操作権限を単独で証明しない。

#![cfg(windows)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::BTreeSet;
use std::mem::{offset_of, size_of};
use std::net::SocketAddrV4;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
};
use windows_sys::Win32::Networking::WinSock::{ntohl, ntohs, AF_INET};

const MAX_TABLE_BYTES: usize = 16 * 1024 * 1024;
const MAX_QUERY_ATTEMPTS: usize = 3;
const TABLE_ROWS_OFFSET: usize = offset_of!(MIB_TCPTABLE_OWNER_PID, table);

/// IPv4 loopback listener ownerを安全に一意化できなかった理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// 観測対象は明示的なIPv4 loopback endpointでなければならない。
    NonLoopbackTarget,
    /// Port 0はbind時の自動割当用であり、照合対象にできない。
    InvalidPort,
    /// 同じportにwildcardまたは非loopback listenerがあり、loopback専用と断定できない。
    NonLoopbackScope,
    /// loopback listenerはあったが、OSが有効なPIDを返さなかった。
    PidUnavailable,
    /// 複数のPIDが対象loopback listenerを所有していた。
    Ambiguous,
    /// OSが要求したTCP table bufferがこの観測境界の上限を超えた。
    TableTooLarge { bytes: u32 },
    /// OSが返したtable length、entry count、row layoutが整合しない。
    MalformedTable,
    /// TCP tableが連続して変化し、有限回の再照会でも安定して読めなかった。
    QueryChangedTooOften,
    /// `GetExtendedTcpTable` が返したWindows error code。
    Windows(u32),
}

/// 指定されたIPv4 loopback TCP listener endpointの一意なOS owner PIDを返す。
///
/// wildcard (`0.0.0.0`) と非loopback addressはloopbackとして扱わない。同じportに
/// それらが存在する場合も、外部到達可能性を見落とさないため失敗する。別のloopback
/// addressのlistenerは要求endpointの所有者として混ぜない。複数PID、PID 0、table異常、
/// 再照会中の継続的な変化もfail-closedでerrorにする。`Ok(None)` は、要求endpointに
/// 明示的なIPv4 loopback listenerが存在しないことだけを表す。
pub fn loopback_tcp_listener_owner_pid(target: SocketAddrV4) -> Result<Option<u32>, Error> {
    if !target.ip().is_loopback() {
        return Err(Error::NonLoopbackTarget);
    }
    if target.port() == 0 {
        return Err(Error::InvalidPort);
    }

    let mut required_bytes = 0_u32;
    // SAFETY: 初回照会はWindows APIの仕様どおりnull bufferと有効なsize pointerを渡す。
    // bufferを読ませず必要サイズだけ取得し、reservedは必ず0にする。
    let first_result = unsafe {
        GetExtendedTcpTable(
            null_mut(),
            &mut required_bytes,
            0,
            u32::from(AF_INET),
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        )
    };

    if first_result != 0 && first_result != ERROR_INSUFFICIENT_BUFFER {
        return Err(Error::Windows(first_result));
    }
    if required_bytes == 0 {
        return if first_result == 0 {
            Ok(None)
        } else {
            Err(Error::MalformedTable)
        };
    }

    for attempt in 0..MAX_QUERY_ATTEMPTS {
        let mut table = allocate_table(required_bytes)?;
        let table_bytes = table_byte_len(&table)?;
        let mut reported_bytes = required_bytes;

        // SAFETY: `table` はu32要素なのでWindows tableに必要な4-byte alignmentを持つ。
        // 渡す長さは実際に割り当てたbuffer以下であり、call後は成功時だけ境界検証して読む。
        let result = unsafe {
            GetExtendedTcpTable(
                table.as_mut_ptr().cast(),
                &mut reported_bytes,
                0,
                u32::from(AF_INET),
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            )
        };

        if result == 0 {
            return parse_listener_owner_pid(&table, table_bytes, target);
        }

        if result != ERROR_INSUFFICIENT_BUFFER {
            return Err(Error::Windows(result));
        }

        if attempt + 1 == MAX_QUERY_ATTEMPTS {
            return Err(Error::QueryChangedTooOften);
        }
        if reported_bytes == 0 {
            return Err(Error::MalformedTable);
        }
        required_bytes = reported_bytes;
    }

    Err(Error::QueryChangedTooOften)
}

fn allocate_table(required_bytes: u32) -> Result<Vec<u32>, Error> {
    let required = usize::try_from(required_bytes).map_err(|_| Error::TableTooLarge {
        bytes: required_bytes,
    })?;

    if required > MAX_TABLE_BYTES {
        return Err(Error::TableTooLarge {
            bytes: required_bytes,
        });
    }
    if required < TABLE_ROWS_OFFSET {
        return Err(Error::MalformedTable);
    }

    let word_size = size_of::<u32>();
    let word_count = required
        .checked_add(word_size - 1)
        .ok_or(Error::TableTooLarge {
            bytes: required_bytes,
        })?
        / word_size;
    let allocated_bytes = word_count
        .checked_mul(word_size)
        .ok_or(Error::TableTooLarge {
            bytes: required_bytes,
        })?;

    // 丸め後の最大3 byteだけは許容する。APIが要求したbytesは上限を超えない。
    if allocated_bytes > MAX_TABLE_BYTES + (word_size - 1) {
        return Err(Error::TableTooLarge {
            bytes: required_bytes,
        });
    }

    Ok(vec![0; word_count])
}

fn table_byte_len(table: &[u32]) -> Result<usize, Error> {
    table
        .len()
        .checked_mul(size_of::<u32>())
        .ok_or(Error::MalformedTable)
}

fn parse_listener_owner_pid(
    table: &[u32],
    table_bytes: usize,
    target: SocketAddrV4,
) -> Result<Option<u32>, Error> {
    if table_bytes < size_of::<u32>() || table_bytes < TABLE_ROWS_OFFSET {
        return Err(Error::MalformedTable);
    }

    let table_base = table.as_ptr().cast::<u8>();
    // SAFETY: `table_base` は少なくともheader先頭のu32を含むbufferを指す。
    // alignmentに依存せず、未初期化領域を読まないためread_unalignedを使う。
    let entry_count = unsafe { std::ptr::read_unaligned(table_base.cast::<u32>()) };
    let entry_count = usize::try_from(entry_count).map_err(|_| Error::MalformedTable)?;
    let row_size = size_of::<MIB_TCPROW_OWNER_PID>();
    let rows_bytes = entry_count
        .checked_mul(row_size)
        .ok_or(Error::MalformedTable)?;
    let required_table_bytes = TABLE_ROWS_OFFSET
        .checked_add(rows_bytes)
        .ok_or(Error::MalformedTable)?;

    if required_table_bytes > table_bytes {
        return Err(Error::MalformedTable);
    }

    let mut owner_pids = BTreeSet::new();
    let mut saw_pid_unavailable = false;
    let mut saw_non_loopback_scope = false;

    for entry_index in 0..entry_count {
        let row_offset = TABLE_ROWS_OFFSET
            .checked_add(
                entry_index
                    .checked_mul(row_size)
                    .ok_or(Error::MalformedTable)?,
            )
            .ok_or(Error::MalformedTable)?;
        let row_ptr = unsafe {
            // SAFETY: entry_countとrow_sizeを検証済みなので、row_offsetから1 row分はtable内にある。
            table_base.add(row_offset).cast::<MIB_TCPROW_OWNER_PID>()
        };
        // SAFETY: row_ptrは検証済みのbuffer範囲内を指す。Windows tableのpaddingやalignmentに依存せず読む。
        let row = unsafe { std::ptr::read_unaligned(row_ptr) };

        collect_matching_owner_pid(
            row,
            target,
            &mut owner_pids,
            &mut saw_pid_unavailable,
            &mut saw_non_loopback_scope,
        );
    }

    if saw_non_loopback_scope {
        return Err(Error::NonLoopbackScope);
    }
    if saw_pid_unavailable {
        return Err(Error::PidUnavailable);
    }

    match owner_pids.len() {
        0 => Ok(None),
        1 => owner_pids
            .iter()
            .next()
            .copied()
            .map(Some)
            .ok_or(Error::MalformedTable),
        _ => Err(Error::Ambiguous),
    }
}

fn collect_matching_owner_pid(
    row: MIB_TCPROW_OWNER_PID,
    target: SocketAddrV4,
    owner_pids: &mut BTreeSet<u32>,
    saw_pid_unavailable: &mut bool,
    saw_non_loopback_scope: &mut bool,
) {
    // MIB_TCPROW2仕様ではportに使うのは下位16bitだけであり、上位16bitは未初期化でよい。
    // MIB_TCPROW_OWNER_PIDも同じDWORD network-order port表現なので、上位bitを検証対象にしない。
    let local_port_network_order = (row.dwLocalPort & u32::from(u16::MAX)) as u16;
    // SAFETY: ntohsはu16値をhost byte orderへ変換するだけで、pointerや外部stateを参照しない。
    let local_port = unsafe { ntohs(local_port_network_order) };

    if local_port != target.port() {
        return;
    }

    // SAFETY: ntohlはu32値をhost byte orderへ変換するだけで、pointerや外部stateを参照しない。
    let local_address = unsafe { ntohl(row.dwLocalAddr) };
    if (local_address >> 24) != 127 {
        *saw_non_loopback_scope = true;
        return;
    }
    if local_address != u32::from(*target.ip()) {
        return;
    }

    if row.dwOwningPid == 0 {
        *saw_pid_unavailable = true;
        return;
    }
    owner_pids.insert(row.dwOwningPid);
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
    use std::thread;
    use std::time::Duration;

    use super::{collect_matching_owner_pid, loopback_tcp_listener_owner_pid, Error};
    use windows_sys::Win32::NetworkManagement::IpHelper::MIB_TCPROW_OWNER_PID;

    #[test]
    fn accepts_a_matching_listener_when_windows_leaves_local_port_high_bits_uninitialized() {
        let target = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 12_345);
        let row = MIB_TCPROW_OWNER_PID {
            dwState: 2,
            dwLocalAddr: u32::from(*target.ip()).to_be(),
            dwLocalPort: 0xa5a5_0000 | u32::from(target.port().to_be()),
            dwRemoteAddr: 0,
            dwRemotePort: 0,
            dwOwningPid: 4242,
        };
        let mut owners = BTreeSet::new();
        let mut saw_pid_unavailable = false;
        let mut saw_non_loopback_scope = false;

        collect_matching_owner_pid(
            row,
            target,
            &mut owners,
            &mut saw_pid_unavailable,
            &mut saw_non_loopback_scope,
        );

        assert_eq!(owners.into_iter().collect::<Vec<_>>(), vec![4242]);
        assert!(!saw_pid_unavailable);
        assert!(!saw_non_loopback_scope);
    }

    #[test]
    fn finds_current_process_for_an_explicit_ipv4_loopback_listener() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .expect("一時IPv4 loopback listenerをbindできる");
        let port = listener
            .local_addr()
            .expect("listenerのlocal addressを取得できる")
            .port();

        for _ in 0..50 {
            match loopback_tcp_listener_owner_pid(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)) {
                Ok(Some(pid)) => {
                    assert_eq!(pid, std::process::id());
                    return;
                }
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(error) => panic!("listener所有processの照合に失敗: {error:?}"),
            }
        }

        panic!("Windows TCP tableからlistener所有processを確認できなかった");
    }

    #[test]
    fn rejects_port_zero_before_querying_windows() {
        assert_eq!(
            loopback_tcp_listener_owner_pid(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)),
            Err(Error::InvalidPort)
        );
    }

    #[test]
    fn does_not_substitute_a_different_loopback_address_with_the_same_port() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .expect("一時IPv4 loopback listenerをbindできる");
        let port = listener
            .local_addr()
            .expect("listenerのlocal addressを取得できる")
            .port();

        let mut exact_endpoint_visible = false;
        for _ in 0..50 {
            match loopback_tcp_listener_owner_pid(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)) {
                Ok(Some(pid)) if pid == std::process::id() => {
                    exact_endpoint_visible = true;
                    break;
                }
                Ok(_) => thread::sleep(Duration::from_millis(10)),
                Err(error) => panic!("正しいloopback endpointの照合に失敗: {error:?}"),
            }
        }
        assert!(
            exact_endpoint_visible,
            "正しいloopback endpointを確認できなかった"
        );
        assert_eq!(
            loopback_tcp_listener_owner_pid(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 2), port,)),
            Ok(None)
        );
    }

    #[test]
    fn rejects_nonloopback_target_before_querying_windows() {
        assert_eq!(
            loopback_tcp_listener_owner_pid(
                SocketAddrV4::new(Ipv4Addr::new(192, 0, 2, 1), 12_345,)
            ),
            Err(Error::NonLoopbackTarget)
        );
    }
}
