//! 現在のBroker trustと署名済み更新recordから再導出した入力だけで、HTTPS packageを取得する。

use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, MetadataExt, OpenOptions};
use reqwest::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, TRANSFER_ENCODING};
use reqwest::{redirect, retry, Certificate, Client, Response, StatusCode, Url};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::{Read, Write};
#[cfg(not(windows))]
use std::net::ToSocketAddrs;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[cfg(windows)]
#[path = "update_download/windows_dns.rs"]
mod windows_dns;

const MAX_PACKAGE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_DOWNLOAD_TIME: Duration = Duration::from_secs(24 * 60 * 60);
const READ_TIMEOUT: Duration = Duration::from_secs(60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const DNS_RESOLUTION_TIMEOUT: Duration = Duration::from_secs(15);
const COPY_BUFFER_BYTES: usize = 64 * 1024;
const PROGRESS_QUANTUM_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DownloadedPackage {
    pub(crate) file_name: String,
    pub(crate) bytes: u64,
    pub(crate) sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DownloadError {
    InvalidRequest,
    NameResolution,
    NonPublicAddress,
    Network,
    RedirectOrUnexpectedStatus,
    InvalidHeaders,
    SizeMismatch,
    DigestMismatch,
    Cancelled,
    TimedOut,
    ResolverBusy,
    DnsCancellationFailed,
    Storage,
}

pub(crate) fn remove_abandoned_partials(directory: &Dir) -> Result<usize, DownloadError> {
    let mut removed = 0usize;
    let mut scanned = 0usize;
    for entry in directory
        .read_dir(".")
        .map_err(|_| DownloadError::Storage)?
    {
        scanned += 1;
        if scanned > 128 {
            return Err(DownloadError::Storage);
        }
        let entry = entry.map_err(|_| DownloadError::Storage)?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(random) = name.strip_suffix(".part") else {
            continue;
        };
        if random.len() != 32
            || !random
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            continue;
        }
        directory
            .remove_file(name.as_ref())
            .map_err(|_| DownloadError::Storage)?;
        removed = removed.saturating_add(1);
    }
    Ok(removed)
}

pub(crate) fn ensure_package_storage_room(
    directory: &Dir,
    expected_sha256: &str,
    expected_bytes: u64,
) -> Result<(), DownloadError> {
    let mut package_count = 0usize;
    let mut package_bytes = 0u64;
    let mut scanned = 0usize;
    for entry in directory
        .read_dir(".")
        .map_err(|_| DownloadError::Storage)?
    {
        scanned += 1;
        if scanned > 128 {
            return Err(DownloadError::Storage);
        }
        let entry = entry.map_err(|_| DownloadError::Storage)?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with(".pkg") {
            continue;
        }
        package_count += 1;
        if package_count > 2 {
            return Err(DownloadError::Storage);
        }
        let digest = name.strip_suffix(".pkg").ok_or(DownloadError::Storage)?;
        if !valid_sha256(digest) || digest != expected_sha256 {
            return Err(DownloadError::Storage);
        }
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let file = directory
            .open_with(name.as_ref(), &options)
            .map_err(|_| DownloadError::Storage)?;
        let metadata = file.metadata().map_err(|_| DownloadError::Storage)?;
        if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
            return Err(DownloadError::Storage);
        }
        package_bytes = package_bytes
            .checked_add(metadata.len())
            .ok_or(DownloadError::Storage)?;
    }
    if (package_count == 0 && expected_bytes <= MAX_PACKAGE_BYTES)
        || (package_count == 1 && package_bytes <= MAX_PACKAGE_BYTES)
    {
        return Ok(());
    }
    Err(DownloadError::Storage)
}

pub(crate) fn download_package<F>(
    directory: &Dir,
    url_text: &str,
    expected_sha256: &str,
    expected_bytes: u64,
    cancel: &AtomicBool,
    mut progress: F,
) -> Result<DownloadedPackage, DownloadError>
where
    F: FnMut(u64),
{
    if expected_bytes == 0 || expected_bytes > MAX_PACKAGE_BYTES || !valid_sha256(expected_sha256) {
        return Err(DownloadError::InvalidRequest);
    }
    let deadline = std::time::Instant::now() + MAX_DOWNLOAD_TIME;
    let url = parse_source_url(url_text)?;
    if verify_existing_package(
        directory,
        expected_sha256,
        expected_bytes,
        cancel,
        deadline,
        &mut progress,
    )? {
        return Ok(DownloadedPackage {
            file_name: format!("{expected_sha256}.pkg"),
            bytes: expected_bytes,
            sha256: expected_sha256.to_owned(),
        });
    }
    let host = url.host_str().ok_or(DownloadError::InvalidRequest)?;
    let addresses = resolve_public_addresses(host, 443, cancel, deadline)?;
    let client = build_download_client(host, &addresses, &[])?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| DownloadError::Network)?;
    runtime.block_on(async {
        let response = client
            .get(url)
            .header(ACCEPT_ENCODING, "identity")
            .send()
            .await
            .map_err(map_reqwest_error)?;
        receive_package(
            directory,
            response,
            expected_sha256,
            expected_bytes,
            cancel,
            deadline,
            &mut progress,
        )
        .await
    })
}

fn parse_source_url(url_text: &str) -> Result<Url, DownloadError> {
    let url = Url::parse(url_text).map_err(|_| DownloadError::InvalidRequest)?;
    let raw_authority = url_text
        .strip_prefix("https://")
        .and_then(|remainder| remainder.split('/').next())
        .ok_or(DownloadError::InvalidRequest)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || url.username() != ""
        || url.password().is_some()
        || raw_authority.contains(':')
        || raw_authority.contains('@')
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(DownloadError::InvalidRequest);
    }
    Ok(url)
}

fn build_download_client(
    host: &str,
    addresses: &[SocketAddr],
    additional_roots: &[Certificate],
) -> Result<Client, DownloadError> {
    let mut builder = Client::builder()
        .https_only(true)
        .no_proxy()
        .redirect(redirect::Policy::none())
        .retry(retry::never())
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(MAX_DOWNLOAD_TIME)
        .read_timeout(READ_TIMEOUT)
        .resolve_to_addrs(host, addresses)
        .http1_only();
    for certificate in additional_roots {
        builder = builder.add_root_certificate(certificate.clone());
    }
    builder.build().map_err(|_| DownloadError::Network)
}

async fn receive_package<F>(
    directory: &Dir,
    mut response: Response,
    expected_sha256: &str,
    expected_bytes: u64,
    cancel: &AtomicBool,
    deadline: std::time::Instant,
    progress: &mut F,
) -> Result<DownloadedPackage, DownloadError>
where
    F: FnMut(u64),
{
    if response.status() != StatusCode::OK {
        return Err(DownloadError::RedirectOrUnexpectedStatus);
    }
    validate_response_headers(response.headers(), expected_bytes)?;

    let mut random = [0u8; 16];
    getrandom::getrandom(&mut random).map_err(|_| DownloadError::Storage)?;
    let temporary_name = format!("{}.part", hex::encode(random));
    let final_name = format!("{expected_sha256}.pkg");
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    let mut output = directory
        .open_with(&temporary_name, &options)
        .map_err(|_| DownloadError::Storage)?;
    let copy_result = async {
        let mut digest = Sha256::new();
        let mut total = 0u64;
        let mut next_progress = PROGRESS_QUANTUM_BYTES;
        loop {
            if cancel.load(Ordering::Acquire) {
                return Err(DownloadError::Cancelled);
            }
            if std::time::Instant::now() >= deadline {
                return Err(DownloadError::TimedOut);
            }
            let chunk = response.chunk().await.map_err(map_reqwest_error)?;
            let Some(chunk) = chunk else {
                break;
            };
            consume_chunk(
                &chunk,
                &mut output,
                &mut digest,
                &mut total,
                &mut next_progress,
                expected_bytes,
                progress,
            )?;
        }
        if total != expected_bytes {
            return Err(DownloadError::SizeMismatch);
        }
        if std::time::Instant::now() >= deadline {
            return Err(DownloadError::TimedOut);
        }
        if hex::encode(digest.finalize()) != expected_sha256 {
            return Err(DownloadError::DigestMismatch);
        }
        progress(total);
        Ok(())
    }
    .await;
    if let Err(error) = copy_result {
        drop(output);
        let _ = directory.remove_file(&temporary_name);
        return Err(error);
    }
    if output.sync_all().is_err() {
        drop(output);
        let _ = directory.remove_file(&temporary_name);
        return Err(DownloadError::Storage);
    }
    drop(output);

    // 既存packageを置換せずlinkも追跡しない、新規作成だけの公開。
    if directory
        .hard_link(&temporary_name, directory, &final_name)
        .is_err()
    {
        let _ = directory.remove_file(&temporary_name);
        return Err(DownloadError::Storage);
    }
    directory
        .remove_file(&temporary_name)
        .map_err(|_| DownloadError::Storage)?;
    Ok(DownloadedPackage {
        file_name: final_name,
        bytes: expected_bytes,
        sha256: expected_sha256.to_owned(),
    })
}

fn verify_existing_package(
    directory: &Dir,
    expected_sha256: &str,
    expected_bytes: u64,
    cancel: &AtomicBool,
    deadline: std::time::Instant,
    progress: &mut impl FnMut(u64),
) -> Result<bool, DownloadError> {
    let name = format!("{expected_sha256}.pkg");
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut file = match directory.open_with(&name, &options) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(DownloadError::Storage),
    };
    let metadata = file.metadata().map_err(|_| DownloadError::Storage)?;
    if !metadata.is_file()
        || metadata.file_attributes() & 0x400 != 0
        || metadata.len() != expected_bytes
    {
        return Err(DownloadError::Storage);
    }
    let mut digest = Sha256::new();
    let mut total = 0u64;
    let mut next_progress = PROGRESS_QUANTUM_BYTES;
    let mut buffer = [0u8; COPY_BUFFER_BYTES];
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err(DownloadError::Cancelled);
        }
        if std::time::Instant::now() >= deadline {
            return Err(DownloadError::TimedOut);
        }
        let read = file.read(&mut buffer).map_err(|_| DownloadError::Storage)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .filter(|total| *total <= expected_bytes)
            .ok_or(DownloadError::SizeMismatch)?;
        digest.update(&buffer[..read]);
        if total >= next_progress {
            progress(total);
            next_progress = total.saturating_add(PROGRESS_QUANTUM_BYTES);
        }
    }
    if total != expected_bytes || hex::encode(digest.finalize()) != expected_sha256 {
        return Err(DownloadError::DigestMismatch);
    }
    progress(total);
    Ok(true)
}

fn validate_response_headers(
    headers: &reqwest::header::HeaderMap,
    expected_bytes: u64,
) -> Result<(), DownloadError> {
    if headers.get_all(CONTENT_LENGTH).iter().count() != 1
        || headers
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            != Some(expected_bytes)
        || headers.get(TRANSFER_ENCODING).is_some()
        || headers.get(CONTENT_ENCODING).is_some()
    {
        return Err(DownloadError::InvalidHeaders);
    }
    Ok(())
}

fn consume_chunk<W, F>(
    chunk: &[u8],
    output: &mut W,
    digest: &mut Sha256,
    total: &mut u64,
    next_progress: &mut u64,
    expected_bytes: u64,
    progress: &mut F,
) -> Result<(), DownloadError>
where
    W: Write,
    F: FnMut(u64),
{
    for buffer in chunk.chunks(COPY_BUFFER_BYTES) {
        *total = total
            .checked_add(buffer.len() as u64)
            .filter(|total| *total <= expected_bytes)
            .ok_or(DownloadError::SizeMismatch)?;
        output
            .write_all(buffer)
            .map_err(|_| DownloadError::Storage)?;
        digest.update(buffer);
        if *total >= *next_progress {
            progress(*total);
            *next_progress = total.saturating_add(PROGRESS_QUANTUM_BYTES);
        }
    }
    Ok(())
}

#[cfg(test)]
fn copy_and_verify<R, W, F>(
    input: &mut R,
    output: &mut W,
    expected_sha256: &str,
    expected_bytes: u64,
    cancel: &AtomicBool,
    deadline: std::time::Instant,
    progress: &mut F,
) -> Result<(), DownloadError>
where
    R: Read,
    W: Write,
    F: FnMut(u64),
{
    let mut digest = Sha256::new();
    let mut buffer = [0u8; COPY_BUFFER_BYTES];
    let mut total = 0u64;
    let mut next_progress = PROGRESS_QUANTUM_BYTES;
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err(DownloadError::Cancelled);
        }
        if std::time::Instant::now() >= deadline {
            return Err(DownloadError::TimedOut);
        }
        let read = input.read(&mut buffer).map_err(|error| {
            if error.kind() == std::io::ErrorKind::TimedOut {
                DownloadError::TimedOut
            } else {
                DownloadError::Network
            }
        })?;
        if read == 0 {
            break;
        }
        consume_chunk(
            &buffer[..read],
            output,
            &mut digest,
            &mut total,
            &mut next_progress,
            expected_bytes,
            progress,
        )?;
    }
    if total != expected_bytes {
        return Err(DownloadError::SizeMismatch);
    }
    if std::time::Instant::now() >= deadline {
        return Err(DownloadError::TimedOut);
    }
    if hex::encode(digest.finalize()) != expected_sha256 {
        return Err(DownloadError::DigestMismatch);
    }
    progress(total);
    Ok(())
}

fn map_reqwest_error(error: reqwest::Error) -> DownloadError {
    if error.is_timeout() {
        DownloadError::TimedOut
    } else {
        DownloadError::Network
    }
}

fn resolve_public_addresses(
    host: &str,
    port: u16,
    cancel: &AtomicBool,
    deadline: std::time::Instant,
) -> Result<Vec<SocketAddr>, DownloadError> {
    if cancel.load(Ordering::Acquire) {
        return Err(DownloadError::Cancelled);
    }
    if std::time::Instant::now() >= deadline {
        return Err(DownloadError::TimedOut);
    }
    let resolved = if let Ok(address) = host.parse::<IpAddr>() {
        vec![SocketAddr::new(address, port)]
    } else {
        #[cfg(windows)]
        {
            windows_dns::resolve(
                host,
                cancel,
                deadline.min(std::time::Instant::now() + DNS_RESOLUTION_TIMEOUT),
            )?
            .into_iter()
            .map(|address| SocketAddr::new(address, port))
            .collect()
        }
        #[cfg(not(windows))]
        {
            (host, port)
                .to_socket_addrs()
                .map_err(|_| DownloadError::NameResolution)?
                .collect()
        }
    };
    let mut addresses = BTreeSet::new();
    for address in resolved {
        if !is_public_address(address.ip()) {
            return Err(DownloadError::NonPublicAddress);
        }
        addresses.insert(address);
    }
    if addresses.is_empty() {
        return Err(DownloadError::NameResolution);
    }
    Ok(addresses.into_iter().collect())
}

fn is_public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(a == 0
                || a == 10
                || a == 127
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192 && b == 0 && c == 0)
                || (a == 192 && b == 0 && c == 2)
                || (a == 192 && b == 168)
                || (a == 192 && b == 88 && c == 99)
                || (a == 198 && (b == 18 || b == 19))
                || (a == 198 && b == 51 && c == 100)
                || (a == 203 && b == 0 && c == 113)
                || a >= 224)
        }
        IpAddr::V6(ip) => {
            let octets = ip.octets();
            // Global unicastだけを許可し、protocol割当て、文書例、
            // 非公開宛先を符号化または中継し得る6to4範囲を除外する。
            (octets[0] & 0xe0) == 0x20
                && !(octets[0] == 0x20
                    && octets[1] == 0x01
                    && ((octets[2] == 0x0d && octets[3] == 0xb8)
                        || (octets[2] == 0x00 && octets[3] <= 0x01)))
                && !(octets[0] == 0x20 && octets[1] == 0x02)
                && !(octets[0] == 0x3f && (octets[1] & 0xf0) == 0xf0)
        }
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Cursor};
    use std::net::{Ipv4Addr, TcpListener};
    use std::sync::Arc;
    use std::thread;

    fn temporary_directory(label: &str) -> std::path::PathBuf {
        let mut random = [0u8; 8];
        getrandom::getrandom(&mut random).unwrap();
        let path = std::env::temp_dir().join(format!(
            "d4-pocket-{label}-{}-{}",
            std::process::id(),
            hex::encode(random)
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn public_address_filter_rejects_non_global_and_accepts_global_unicast() {
        for address in [
            "0.0.0.0",
            "10.1.2.3",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.1.2",
            "172.16.0.1",
            "192.0.2.1",
            "192.168.1.1",
            "198.18.0.1",
            "198.51.100.2",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
            "2002::1",
            "3fff::1",
        ] {
            let address: IpAddr = address.parse().unwrap();
            assert!(
                !is_public_address(address),
                "unexpectedly public: {address}"
            );
        }
        for address in ["8.8.8.8", "1.1.1.1", "2606:4700:4700::1111"] {
            let address: IpAddr = address.parse().unwrap();
            assert!(
                is_public_address(address),
                "unexpectedly blocked: {address}"
            );
        }
    }

    #[test]
    fn cancelled_or_expired_name_resolution_stops_before_resolver_call() {
        let cancelled = AtomicBool::new(true);
        assert_eq!(
            resolve_public_addresses(
                "updates.example",
                443,
                &cancelled,
                std::time::Instant::now() + Duration::from_secs(5),
            ),
            Err(DownloadError::Cancelled)
        );

        let expired = std::time::Instant::now() - Duration::from_secs(1);
        assert_eq!(
            resolve_public_addresses("updates.example", 443, &AtomicBool::new(false), expired,),
            Err(DownloadError::TimedOut)
        );
    }

    #[test]
    fn source_url_rejects_non_https_authority_and_explicit_port() {
        for url in [
            "http://updates.example.invalid/update.pkg",
            "https://user@updates.example.invalid/update.pkg",
            "https://updates.example.invalid:443/update.pkg",
            "https://updates.example.invalid/update.pkg?token=secret",
            "https://updates.example.invalid/update.pkg#fragment",
        ] {
            assert_eq!(
                parse_source_url(url),
                Err(DownloadError::InvalidRequest),
                "{url}"
            );
        }
        assert!(parse_source_url("https://updates.example.invalid/update.pkg").is_ok());
    }

    #[test]
    fn stream_is_bounded_and_digest_bound_before_publication() {
        let bytes = b"signed package bytes";
        let digest = hex::encode(Sha256::digest(bytes));
        let mut input = Cursor::new(bytes);
        let mut output = Vec::new();
        let cancel = AtomicBool::new(false);
        let mut progress_values = Vec::new();
        copy_and_verify(
            &mut input,
            &mut output,
            &digest,
            bytes.len() as u64,
            &cancel,
            std::time::Instant::now() + Duration::from_secs(60),
            &mut |value| progress_values.push(value),
        )
        .unwrap();
        assert_eq!(output, bytes);
        assert_eq!(progress_values, [bytes.len() as u64]);
    }

    #[test]
    fn stream_rejects_short_long_wrong_digest_and_cancelled_payloads() {
        let cancel = AtomicBool::new(false);
        let mut output = Vec::new();
        let mut progress = |_| {};
        let bytes = b"abc";
        let digest = hex::encode(Sha256::digest(bytes));
        assert_eq!(
            copy_and_verify(
                &mut Cursor::new(bytes),
                &mut output,
                &digest,
                4,
                &cancel,
                std::time::Instant::now() + Duration::from_secs(60),
                &mut progress
            ),
            Err(DownloadError::SizeMismatch)
        );
        assert_eq!(
            copy_and_verify(
                &mut Cursor::new(b"abcd"),
                &mut output,
                &digest,
                3,
                &cancel,
                std::time::Instant::now() + Duration::from_secs(60),
                &mut progress
            ),
            Err(DownloadError::SizeMismatch)
        );
        assert_eq!(
            copy_and_verify(
                &mut Cursor::new(bytes),
                &mut output,
                &"0".repeat(64),
                3,
                &cancel,
                std::time::Instant::now() + Duration::from_secs(60),
                &mut progress
            ),
            Err(DownloadError::DigestMismatch)
        );
        cancel.store(true, Ordering::Release);
        assert_eq!(
            copy_and_verify(
                &mut Cursor::new(bytes),
                &mut output,
                &digest,
                3,
                &cancel,
                std::time::Instant::now() + Duration::from_secs(60),
                &mut progress
            ),
            Err(DownloadError::Cancelled)
        );
    }

    #[test]
    fn response_requires_exact_length_without_transfer_or_content_encoding() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(CONTENT_LENGTH, "42".parse().unwrap());
        assert_eq!(validate_response_headers(&headers, 42), Ok(()));

        headers.append(CONTENT_LENGTH, "42".parse().unwrap());
        assert_eq!(
            validate_response_headers(&headers, 42),
            Err(DownloadError::InvalidHeaders)
        );
        headers.remove(CONTENT_LENGTH);
        headers.insert(CONTENT_LENGTH, "43".parse().unwrap());
        assert_eq!(
            validate_response_headers(&headers, 42),
            Err(DownloadError::InvalidHeaders)
        );
        headers.insert(CONTENT_LENGTH, "42".parse().unwrap());
        headers.insert(TRANSFER_ENCODING, "chunked".parse().unwrap());
        assert_eq!(
            validate_response_headers(&headers, 42),
            Err(DownloadError::InvalidHeaders)
        );
        headers.remove(TRANSFER_ENCODING);
        headers.insert(CONTENT_ENCODING, "gzip".parse().unwrap());
        assert_eq!(
            validate_response_headers(&headers, 42),
            Err(DownloadError::InvalidHeaders)
        );
    }

    #[test]
    fn package_store_is_content_addressed_and_rejects_other_packages() {
        let path = temporary_directory("update-package-store");
        let directory = Dir::open_ambient_dir(&path, cap_std::ambient_authority()).unwrap();
        let bytes = b"package";
        let digest = hex::encode(Sha256::digest(bytes));
        ensure_package_storage_room(&directory, &digest, bytes.len() as u64).unwrap();
        std::fs::write(path.join(format!("{digest}.pkg")), bytes).unwrap();
        ensure_package_storage_room(&directory, &digest, bytes.len() as u64).unwrap();
        assert_eq!(
            ensure_package_storage_room(&directory, &"a".repeat(64), bytes.len() as u64),
            Err(DownloadError::Storage)
        );
        drop(directory);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn only_generated_abandoned_partial_names_are_removed() {
        let path = temporary_directory("update-partial-recovery");
        let directory = Dir::open_ambient_dir(&path, cap_std::ambient_authority()).unwrap();
        let generated = format!("{}.part", "a".repeat(32));
        std::fs::write(path.join(&generated), b"partial").unwrap();
        std::fs::write(path.join("owner-notes.part"), b"preserve").unwrap();
        assert_eq!(remove_abandoned_partials(&directory).unwrap(), 1);
        assert!(!path.join(&generated).exists());
        assert!(path.join("owner-notes.part").exists());
        drop(directory);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn local_tls_server_downloads_and_publishes_only_verified_package_bytes() {
        let bytes = b"locally served signed package";
        let digest = hex::encode(Sha256::digest(bytes));
        let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![certified.cert.der().clone()],
                rustls::pki_types::PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der())
                    .into(),
            )
            .unwrap();
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let connection = rustls::ServerConnection::new(Arc::new(server_config)).unwrap();
            let mut tls = rustls::StreamOwned::new(connection, stream);
            let mut request = String::new();
            {
                let mut reader = BufReader::new(&mut tls);
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    request.push_str(&line);
                    assert!(request.len() <= 16 * 1024);
                }
            }
            assert!(request.starts_with("GET /update.pkg HTTP/1.1\r\n"));
            write!(
                tls,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            )
            .unwrap();
            tls.write_all(bytes).unwrap();
            tls.flush().unwrap();
        });

        let root = Certificate::from_der(certified.cert.der().as_ref()).unwrap();
        let client = build_download_client("localhost", &[address], &[root]).unwrap();
        let url = Url::parse(&format!("https://localhost:{}/update.pkg", address.port())).unwrap();
        let path = temporary_directory("update-local-tls");
        let directory = Dir::open_ambient_dir(&path, cap_std::ambient_authority()).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime
            .block_on(async {
                let response = client
                    .get(url)
                    .header(ACCEPT_ENCODING, "identity")
                    .send()
                    .await
                    .unwrap();
                receive_package(
                    &directory,
                    response,
                    &digest,
                    bytes.len() as u64,
                    &AtomicBool::new(false),
                    std::time::Instant::now() + Duration::from_secs(30),
                    &mut |_| {},
                )
                .await
            })
            .unwrap();
        server.join().unwrap();
        assert_eq!(result.file_name, format!("{digest}.pkg"));
        assert_eq!(std::fs::read(path.join(&result.file_name)).unwrap(), bytes);
        drop(directory);
        std::fs::remove_dir_all(path).unwrap();
    }
}
