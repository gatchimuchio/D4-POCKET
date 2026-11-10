//! 現在のBroker trustと署名済み更新recordから再導出した入力だけで、HTTPS packageを取得する。

use cap_fs_ext::{FollowSymlinks, MetadataExt as CapMetadataExt, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptions};
use reqwest::header::{
    ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING,
};
use reqwest::{redirect, retry, Certificate, Client, Response, StatusCode, Url};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::{Read, Write};
#[cfg(not(windows))]
use std::net::ToSocketAddrs;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(any(not(windows), test))]
use std::sync::mpsc;
use std::time::Duration;

use super::product_install::is_cap_reparse_point;

#[cfg(windows)]
#[path = "update_download/windows_dns.rs"]
mod windows_dns;

const MAX_PACKAGE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_CATALOG_BYTES: usize = 64 * 1024;
const MAX_A2A_CARD_BYTES: usize = 1024 * 1024;
const MAX_A2A_HEADER_BYTES: usize = 16 * 1024;
const MAX_A2A_HEADER_COUNT: usize = 64;
const MAX_DOWNLOAD_TIME: Duration = Duration::from_secs(24 * 60 * 60);
pub(crate) const CATALOG_TIMEOUT: Duration = Duration::from_secs(30);
const READ_TIMEOUT: Duration = Duration::from_secs(60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const DNS_RESOLUTION_TIMEOUT: Duration = Duration::from_secs(15);
const COPY_BUFFER_BYTES: usize = 64 * 1024;
const PROGRESS_QUANTUM_BYTES: u64 = 4 * 1024 * 1024;

#[cfg(not(windows))]
static SYSTEM_DNS_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "r2-e2e")]
const R2_E2E_UPDATE_HOST: &str = "updates.d4p-test.example";
#[cfg(feature = "r2-e2e")]
const R2_E2E_UPDATE_CA_ENV: &str = "GUI_SHELL_R2_E2E_UPDATE_CA_PEM";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DownloadedPackage {
    pub(crate) file_name: String,
    pub(crate) bytes: u64,
    pub(crate) sha256: String,
    pub(crate) disposition: PackageDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PackageDisposition {
    AlreadyVerified,
    Downloaded,
    RepairedCorrupt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExistingPackageState {
    Missing,
    Verified,
    Corrupt,
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
    DocumentTooLarge,
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
        if !metadata.is_file() || is_cap_reparse_point(&metadata) {
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

pub(crate) fn remove_verified_package(
    directory: &Dir,
    expected_sha256: &str,
    expected_bytes: u64,
) -> Result<bool, DownloadError> {
    if !valid_sha256(expected_sha256) || expected_bytes == 0 || expected_bytes > MAX_PACKAGE_BYTES {
        return Err(DownloadError::InvalidRequest);
    }
    let name = format!("{expected_sha256}.pkg");
    let before = match directory.symlink_metadata(&name) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(DownloadError::Storage),
    };
    if !before.is_file() || is_cap_reparse_point(&before) || before.len() != expected_bytes {
        return Err(DownloadError::Storage);
    }
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut file = directory
        .open_with(&name, &options)
        .map_err(|_| DownloadError::Storage)?;
    let opened = file.metadata().map_err(|_| DownloadError::Storage)?;
    if !opened.is_file()
        || is_cap_reparse_point(&opened)
        || !same_file_identity(&before, &opened)
        || opened.len() != expected_bytes
    {
        return Err(DownloadError::Storage);
    }
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; COPY_BUFFER_BYTES];
    loop {
        let read = file.read(&mut buffer).map_err(|_| DownloadError::Storage)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or(DownloadError::Storage)?;
        if total > expected_bytes {
            return Err(DownloadError::SizeMismatch);
        }
        hasher.update(&buffer[..read]);
    }
    if total != expected_bytes || hex::encode(hasher.finalize()) != expected_sha256 {
        return Err(DownloadError::DigestMismatch);
    }
    let after = directory
        .symlink_metadata(&name)
        .map_err(|_| DownloadError::Storage)?;
    if !after.is_file()
        || is_cap_reparse_point(&after)
        || !same_file_identity(&opened, &after)
        || after.len() != expected_bytes
    {
        return Err(DownloadError::Storage);
    }
    drop(file);
    directory
        .remove_file(&name)
        .map_err(|_| DownloadError::Storage)?;
    Ok(true)
}

fn same_file_identity(left: &cap_std::fs::Metadata, right: &cap_std::fs::Metadata) -> bool {
    let inode = CapMetadataExt::ino(left);
    inode != 0
        && CapMetadataExt::dev(left) == CapMetadataExt::dev(right)
        && inode == CapMetadataExt::ino(right)
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
    let existing_state = verify_existing_package(
        directory,
        expected_sha256,
        expected_bytes,
        cancel,
        deadline,
        &mut progress,
    )?;
    if existing_state == ExistingPackageState::Verified {
        return Ok(DownloadedPackage {
            file_name: format!("{expected_sha256}.pkg"),
            bytes: expected_bytes,
            sha256: expected_sha256.to_owned(),
            disposition: PackageDisposition::AlreadyVerified,
        });
    }
    let host = url.host_str().ok_or(DownloadError::InvalidRequest)?;
    let addresses = resolve_public_addresses(host, 443, cancel, deadline)?;
    let client = build_update_download_client(host, &addresses, MAX_DOWNLOAD_TIME)?;
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
            existing_state == ExistingPackageState::Corrupt,
            cancel,
            deadline,
            &mut progress,
        )
        .await
    })
}

pub(crate) fn fetch_update_catalog(
    url_text: &str,
    max_bytes: usize,
    deadline: std::time::Instant,
) -> Result<Vec<u8>, DownloadError> {
    super::update_center::set_catalog_panic_stage(7);
    if max_bytes == 0 || max_bytes > MAX_CATALOG_BYTES {
        return Err(DownloadError::InvalidRequest);
    }
    if std::time::Instant::now() >= deadline {
        return Err(DownloadError::TimedOut);
    }
    let url = parse_source_url(url_text)?;
    let host = url.host_str().ok_or(DownloadError::InvalidRequest)?;
    super::update_center::set_catalog_panic_stage(8);
    let addresses = resolve_public_addresses(host, 443, &AtomicBool::new(false), deadline)?;
    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
    if remaining.is_zero() {
        return Err(DownloadError::TimedOut);
    }
    super::update_center::set_catalog_panic_stage(9);
    let client = build_update_download_client(host, &addresses, remaining)?;
    super::update_center::set_catalog_panic_stage(10);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| DownloadError::Network)?;
    runtime.block_on(async {
        super::update_center::set_catalog_panic_stage(11);
        let response = client
            .get(url)
            .header(ACCEPT_ENCODING, "identity")
            .send()
            .await
            .map_err(map_reqwest_error)?;
        super::update_center::set_catalog_panic_stage(12);
        receive_bounded_document(response, max_bytes, deadline).await
    })
}

pub(crate) fn fetch_a2a_agent_card_document(
    url_text: &str,
    deadline: std::time::Instant,
) -> Result<Vec<u8>, DownloadError> {
    if url_text.len() > 2048 || std::time::Instant::now() >= deadline {
        return Err(if std::time::Instant::now() >= deadline {
            DownloadError::TimedOut
        } else {
            DownloadError::InvalidRequest
        });
    }
    let url = Url::parse(url_text).map_err(|_| DownloadError::InvalidRequest)?;
    let raw_authority = url_text
        .split_once("://")
        .map(|(_, remainder)| remainder.split('/').next().unwrap_or_default())
        .ok_or(DownloadError::InvalidRequest)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || url.username() != ""
        || url.password().is_some()
        || raw_authority.contains('@')
        || raw_authority.contains('%')
        || !raw_authority.is_ascii()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(DownloadError::InvalidRequest);
    }
    let host = url
        .host_str()
        .ok_or(DownloadError::InvalidRequest)?
        .to_owned();
    let port = url
        .port_or_known_default()
        .ok_or(DownloadError::InvalidRequest)?;
    let addresses = resolve_public_addresses(&host, port, &AtomicBool::new(false), deadline)?;
    fetch_a2a_agent_card_document_with_addresses(
        url,
        &host,
        &addresses,
        &[],
        MAX_A2A_CARD_BYTES,
        deadline,
    )
}

pub(crate) fn send_a2a_json_rpc_document(
    url_text: &str,
    request_body: &[u8],
    deadline: std::time::Instant,
) -> Result<Vec<u8>, DownloadError> {
    if url_text.len() > 2048
        || request_body.is_empty()
        || request_body.len() > 64 * 1024
        || std::time::Instant::now() >= deadline
    {
        return Err(if std::time::Instant::now() >= deadline {
            DownloadError::TimedOut
        } else {
            DownloadError::InvalidRequest
        });
    }
    let url = Url::parse(url_text).map_err(|_| DownloadError::InvalidRequest)?;
    let raw_authority = url_text
        .split_once("://")
        .map(|(_, remainder)| remainder.split('/').next().unwrap_or_default())
        .ok_or(DownloadError::InvalidRequest)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || url.username() != ""
        || url.password().is_some()
        || raw_authority.contains('@')
        || raw_authority.contains('%')
        || !raw_authority.is_ascii()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(DownloadError::InvalidRequest);
    }
    let host = url
        .host_str()
        .ok_or(DownloadError::InvalidRequest)?
        .to_owned();
    let port = url
        .port_or_known_default()
        .ok_or(DownloadError::InvalidRequest)?;
    let addresses = resolve_public_addresses(&host, port, &AtomicBool::new(false), deadline)?;
    send_a2a_json_rpc_document_with_addresses(url, &host, &addresses, request_body, deadline)
}

fn send_a2a_json_rpc_document_with_addresses(
    url: Url,
    host: &str,
    addresses: &[SocketAddr],
    request_body: &[u8],
    deadline: std::time::Instant,
) -> Result<Vec<u8>, DownloadError> {
    if request_body.is_empty() || request_body.len() > 64 * 1024 || addresses.is_empty() {
        return Err(DownloadError::InvalidRequest);
    }
    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
    if remaining.is_zero() {
        return Err(DownloadError::TimedOut);
    }
    let client = build_download_client_with_header_limit(
        host,
        addresses,
        &[],
        remaining,
        MAX_A2A_HEADER_COUNT,
    )?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| DownloadError::Network)?;
    runtime.block_on(async {
        let response = client
            .post(url)
            .header(ACCEPT, "application/json")
            .header(ACCEPT_ENCODING, "identity")
            .header(CONTENT_TYPE, "application/json")
            .header("A2A-Version", "1.0")
            .body(request_body.to_vec())
            .send()
            .await
            .map_err(map_reqwest_error)?;
        if !a2a_response_headers_within_bound(response.headers()) {
            return Err(DownloadError::InvalidHeaders);
        }
        let content_types = response.headers().get_all(CONTENT_TYPE);
        if content_types.iter().count() != 1
            || !content_types
                .iter()
                .next()
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| {
                    value.split(';').next().is_some_and(|media_type| {
                        media_type.trim().eq_ignore_ascii_case("application/json")
                    })
                })
        {
            return Err(DownloadError::InvalidHeaders);
        }
        receive_bounded_document(response, MAX_A2A_CARD_BYTES, deadline).await
    })
}

fn fetch_a2a_agent_card_document_with_addresses(
    url: Url,
    host: &str,
    addresses: &[SocketAddr],
    additional_roots: &[Certificate],
    max_bytes: usize,
    deadline: std::time::Instant,
) -> Result<Vec<u8>, DownloadError> {
    if max_bytes == 0 || max_bytes > MAX_A2A_CARD_BYTES || addresses.is_empty() {
        return Err(DownloadError::InvalidRequest);
    }
    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
    if remaining.is_zero() {
        return Err(DownloadError::TimedOut);
    }
    let client = build_download_client_with_header_limit(
        host,
        addresses,
        additional_roots,
        remaining,
        MAX_A2A_HEADER_COUNT,
    )?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| DownloadError::Network)?;
    runtime.block_on(async {
        let response = client
            .get(url)
            .header(ACCEPT, "application/json")
            .header(ACCEPT_ENCODING, "identity")
            .send()
            .await
            .map_err(map_reqwest_error)?;
        if !a2a_response_headers_within_bound(response.headers()) {
            return Err(DownloadError::InvalidHeaders);
        }
        let content_types = response.headers().get_all(CONTENT_TYPE);
        if content_types.iter().count() != 1
            || !content_types
                .iter()
                .next()
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| {
                    value.split(';').next().is_some_and(|media_type| {
                        media_type.trim().eq_ignore_ascii_case("application/json")
                    })
                })
        {
            return Err(DownloadError::InvalidHeaders);
        }
        receive_bounded_document(response, max_bytes, deadline).await
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
    timeout: Duration,
) -> Result<Client, DownloadError> {
    build_download_client_with_header_limit(host, addresses, additional_roots, timeout, 0)
}

fn build_download_client_with_header_limit(
    host: &str,
    addresses: &[SocketAddr],
    additional_roots: &[Certificate],
    timeout: Duration,
    max_headers: usize,
) -> Result<Client, DownloadError> {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        return Err(DownloadError::Network);
    }
    let mut builder = Client::builder()
        .https_only(true)
        .no_proxy()
        .redirect(redirect::Policy::none())
        .retry(retry::never())
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(timeout)
        .read_timeout(READ_TIMEOUT)
        .resolve_to_addrs(host, addresses)
        .http1_only();
    if max_headers > 0 {
        builder = builder.http1_max_headers(max_headers);
    }
    for certificate in additional_roots {
        builder = builder.add_root_certificate(certificate.clone());
    }
    builder.build().map_err(|_| DownloadError::Network)
}

fn build_update_download_client(
    host: &str,
    addresses: &[SocketAddr],
    timeout: Duration,
) -> Result<Client, DownloadError> {
    #[cfg(feature = "r2-e2e")]
    let additional_roots = r2_e2e_update_roots(host)?;
    #[cfg(not(feature = "r2-e2e"))]
    let additional_roots: Vec<Certificate> = Vec::new();
    build_download_client(host, addresses, &additional_roots, timeout)
}

#[cfg(feature = "r2-e2e")]
fn r2_e2e_update_roots(host: &str) -> Result<Vec<Certificate>, DownloadError> {
    if host != R2_E2E_UPDATE_HOST {
        return Ok(Vec::new());
    }
    let path = std::env::var_os(R2_E2E_UPDATE_CA_ENV)
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or(DownloadError::Network)?;
    let metadata = std::fs::symlink_metadata(&path).map_err(|_| DownloadError::Network)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > 16 * 1024
    {
        return Err(DownloadError::Network);
    }
    let pem = std::fs::read(path).map_err(|_| DownloadError::Network)?;
    let certificate = Certificate::from_pem(&pem).map_err(|_| DownloadError::Network)?;
    Ok(vec![certificate])
}

async fn receive_bounded_document(
    mut response: Response,
    max_bytes: usize,
    deadline: std::time::Instant,
) -> Result<Vec<u8>, DownloadError> {
    if response.status() != StatusCode::OK {
        return Err(DownloadError::RedirectOrUnexpectedStatus);
    }
    let declared_bytes = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok());
    if response.headers().get_all(CONTENT_LENGTH).iter().count() != 1
        || response.headers().get(TRANSFER_ENCODING).is_some()
        || response.headers().get(CONTENT_ENCODING).is_some()
    {
        return Err(DownloadError::InvalidHeaders);
    }
    let declared_bytes = declared_bytes.ok_or(DownloadError::InvalidHeaders)?;
    if declared_bytes == 0 || declared_bytes > max_bytes {
        return Err(DownloadError::DocumentTooLarge);
    }
    let mut bytes = Vec::with_capacity(declared_bytes);
    loop {
        if std::time::Instant::now() >= deadline {
            return Err(DownloadError::TimedOut);
        }
        let chunk = response.chunk().await.map_err(map_reqwest_error)?;
        let Some(chunk) = chunk else {
            break;
        };
        if bytes
            .len()
            .checked_add(chunk.len())
            .is_none_or(|total| total > max_bytes || total > declared_bytes)
        {
            return Err(DownloadError::DocumentTooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.len() != declared_bytes {
        return Err(DownloadError::SizeMismatch);
    }
    if std::time::Instant::now() >= deadline {
        return Err(DownloadError::TimedOut);
    }
    Ok(bytes)
}

fn a2a_response_headers_within_bound(headers: &reqwest::header::HeaderMap) -> bool {
    if headers.iter().count() > MAX_A2A_HEADER_COUNT {
        return false;
    }
    headers
        .iter()
        .try_fold(0usize, |total, (name, value)| {
            total
                .checked_add(name.as_str().len())
                .and_then(|total| total.checked_add(value.as_bytes().len()))
                .and_then(|total| total.checked_add(4))
                .filter(|total| *total <= MAX_A2A_HEADER_BYTES)
        })
        .is_some()
}

async fn receive_package<F>(
    directory: &Dir,
    mut response: Response,
    expected_sha256: &str,
    expected_bytes: u64,
    replace_corrupt: bool,
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

    if cancel.load(Ordering::Acquire) {
        let _ = directory.remove_file(&temporary_name);
        return Err(DownloadError::Cancelled);
    }
    if std::time::Instant::now() >= deadline {
        let _ = directory.remove_file(&temporary_name);
        return Err(DownloadError::TimedOut);
    }

    let disposition =
        publish_verified_package(directory, &temporary_name, &final_name, replace_corrupt)?;
    Ok(DownloadedPackage {
        file_name: final_name,
        bytes: expected_bytes,
        sha256: expected_sha256.to_owned(),
        disposition,
    })
}

fn publish_verified_package(
    directory: &Dir,
    temporary_name: &str,
    final_name: &str,
    replace_corrupt: bool,
) -> Result<PackageDisposition, DownloadError> {
    let result = (|| {
        if replace_corrupt && package_entry_is_replaceable(directory, final_name)? {
            // 同一digest名の通常fileだけを、完全検証・fsync済みの一時fileで原子的に置換する。
            directory
                .rename(temporary_name, directory, final_name)
                .map_err(|_| DownloadError::Storage)?;
            return Ok(PackageDisposition::RepairedCorrupt);
        }
        // 欠損時は既存fileを上書きせず、create-only hard linkで公開する。
        directory
            .hard_link(temporary_name, directory, final_name)
            .map_err(|_| DownloadError::Storage)?;
        directory
            .remove_file(temporary_name)
            .map_err(|_| DownloadError::Storage)?;
        Ok(PackageDisposition::Downloaded)
    })();
    if result.is_err() {
        let _ = directory.remove_file(temporary_name);
    }
    result
}

fn package_entry_is_replaceable(directory: &Dir, file_name: &str) -> Result<bool, DownloadError> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let file = match directory.open_with(file_name, &options) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(DownloadError::Storage),
    };
    let metadata = file.metadata().map_err(|_| DownloadError::Storage)?;
    if !metadata.is_file() || is_cap_reparse_point(&metadata) {
        return Err(DownloadError::Storage);
    }
    Ok(true)
}

fn verify_existing_package(
    directory: &Dir,
    expected_sha256: &str,
    expected_bytes: u64,
    cancel: &AtomicBool,
    deadline: std::time::Instant,
    progress: &mut impl FnMut(u64),
) -> Result<ExistingPackageState, DownloadError> {
    let name = format!("{expected_sha256}.pkg");
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut file = match directory.open_with(&name, &options) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ExistingPackageState::Missing);
        }
        Err(_) => return Err(DownloadError::Storage),
    };
    let metadata = file.metadata().map_err(|_| DownloadError::Storage)?;
    if !metadata.is_file() || is_cap_reparse_point(&metadata) {
        return Err(DownloadError::Storage);
    }
    if metadata.len() != expected_bytes {
        return Ok(ExistingPackageState::Corrupt);
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
            .ok_or(DownloadError::Storage)?;
        if total > expected_bytes {
            return Ok(ExistingPackageState::Corrupt);
        }
        digest.update(&buffer[..read]);
        if total >= next_progress {
            progress(total);
            next_progress = total.saturating_add(PROGRESS_QUANTUM_BYTES);
        }
    }
    if total != expected_bytes || hex::encode(digest.finalize()) != expected_sha256 {
        return Ok(ExistingPackageState::Corrupt);
    }
    progress(total);
    Ok(ExistingPackageState::Verified)
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
    #[cfg(feature = "r2-e2e")]
    if let Some(address) = r2_e2e_update_address(host, port) {
        return Ok(vec![address]);
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
            resolve_system_addresses(
                host,
                port,
                cancel,
                deadline.min(std::time::Instant::now() + DNS_RESOLUTION_TIMEOUT),
            )?
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

#[cfg(any(not(windows), test))]
struct SystemDnsWorkerGuard(&'static AtomicBool);

#[cfg(any(not(windows), test))]
impl Drop for SystemDnsWorkerGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[cfg(not(windows))]
fn resolve_system_addresses(
    host: &str,
    port: u16,
    cancel: &AtomicBool,
    deadline: std::time::Instant,
) -> Result<Vec<SocketAddr>, DownloadError> {
    resolve_system_addresses_with(
        host,
        port,
        cancel,
        deadline,
        &SYSTEM_DNS_IN_FLIGHT,
        |host, port| {
            (host, port)
                .to_socket_addrs()
                .map(|addresses| addresses.collect())
                .map_err(|_| DownloadError::NameResolution)
        },
    )
}

#[cfg(any(not(windows), test))]
fn resolve_system_addresses_with<F>(
    host: &str,
    port: u16,
    cancel: &AtomicBool,
    deadline: std::time::Instant,
    in_flight: &'static AtomicBool,
    resolver: F,
) -> Result<Vec<SocketAddr>, DownloadError>
where
    F: FnOnce(&str, u16) -> Result<Vec<SocketAddr>, DownloadError> + Send + 'static,
{
    if cancel.load(Ordering::Acquire) {
        return Err(DownloadError::Cancelled);
    }
    if std::time::Instant::now() >= deadline {
        return Err(DownloadError::TimedOut);
    }
    in_flight
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| DownloadError::ResolverBusy)?;
    let (reply, result) = mpsc::sync_channel(1);
    let worker_host = host.to_string();
    if std::thread::Builder::new()
        .name("d4p-bounded-system-dns".into())
        .spawn(move || {
            let _guard = SystemDnsWorkerGuard(in_flight);
            let outcome = resolver(&worker_host, port);
            let _ = reply.send(outcome);
        })
        .is_err()
    {
        in_flight.store(false, Ordering::Release);
        return Err(DownloadError::Network);
    }
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err(DownloadError::Cancelled);
        }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(DownloadError::TimedOut);
        }
        match result.recv_timeout(remaining.min(Duration::from_millis(25))) {
            Ok(outcome) => return outcome,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err(DownloadError::Network),
        }
    }
}

#[cfg(feature = "r2-e2e")]
fn r2_e2e_update_address(host: &str, port: u16) -> Option<SocketAddr> {
    (host == R2_E2E_UPDATE_HOST && port == 443).then(|| SocketAddr::from(([127, 0, 0, 1], 443)))
}

fn is_iana_2001_global_exception(ip: std::net::Ipv6Addr) -> bool {
    let segments = ip.segments();
    if segments[0] != 0x2001 || segments[1] > 0x01ff {
        return false;
    }
    (segments[1] == 1
        && segments[2..7].iter().all(|segment| *segment == 0)
        && (1..=3).contains(&segments[7]))
        || segments[1] == 3
        || (segments[1] == 4 && segments[2] == 0x0112)
        || (0x20..=0x2f).contains(&segments[1])
        || (0x30..=0x3f).contains(&segments[1])
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
            // Global unicastだけを許可し、IANAの2001::/23は明示された
            // global reachability例外以外を拒否する。文書例と6to4も除外する。
            (octets[0] & 0xe0) == 0x20
                && !(octets[0] == 0x20
                    && octets[1] == 0x01
                    && octets[2] == 0x0d
                    && octets[3] == 0xb8)
                && !(ip.segments()[0] == 0x2001
                    && ip.segments()[1] <= 0x01ff
                    && !is_iana_2001_global_exception(ip))
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

    // 全件試験の負荷でWindowsの試験用TLS確立が遅れる場合がある。
    // 製品のA2A要求期限は8秒のまま。
    const A2A_TLS_FIXTURE_TIMEOUT: Duration = Duration::from_secs(30);

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
            "2001:1::4",
            "2001:2::1",
            "2001:10::1",
            "2001:4:113::1",
            "2001:1f:ffff:ffff:ffff:ffff:ffff:ffff",
            "2001:1ff::1",
            "2002::1",
            "3fff::1",
        ] {
            let address: IpAddr = address.parse().unwrap();
            assert!(
                !is_public_address(address),
                "unexpectedly public: {address}"
            );
        }
        for address in [
            "8.8.8.8",
            "1.1.1.1",
            "2001:1::1",
            "2001:1::2",
            "2001:1::3",
            "2001:3::1",
            "2001:4:112::1",
            "2001:20::1",
            "2001:2f:ffff:ffff:ffff:ffff:ffff:ffff",
            "2001:30::1",
            "2001:3f:ffff:ffff:ffff:ffff:ffff:ffff",
            "2606:4700:4700::1111",
        ] {
            let address: IpAddr = address.parse().unwrap();
            assert!(
                is_public_address(address),
                "unexpectedly blocked: {address}"
            );
        }
    }

    #[cfg(feature = "r2-e2e")]
    #[test]
    fn acceptance_loopback_override_is_fixed_to_the_test_host_and_https_port() {
        assert_eq!(
            r2_e2e_update_address(R2_E2E_UPDATE_HOST, 443),
            Some(SocketAddr::from(([127, 0, 0, 1], 443)))
        );
        for (host, port) in [
            ("updates.example.com", 443),
            (R2_E2E_UPDATE_HOST, 8443),
            ("127.0.0.1", 443),
        ] {
            assert_eq!(r2_e2e_update_address(host, port), None);
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

    #[cfg(unix)]
    #[test]
    fn unix更新保管先とpackageのsymbolic_linkを拒否する() {
        use std::os::unix::fs::symlink;

        let path = temporary_directory("update-link-boundary");
        let outside = path.join("outside");
        std::fs::create_dir(&outside).unwrap();
        let (store, _) = crate::broker::store::BrokerPersistentStore::open_or_create(
            &path.join("store"),
            "update-link-test",
        )
        .unwrap();
        symlink(&outside, store.root().join("update_packages")).unwrap();
        assert!(store.open_update_package_directory().is_err());

        let bytes = b"package";
        let digest = hex::encode(Sha256::digest(bytes));
        let target = outside.join("original");
        std::fs::write(&target, bytes).unwrap();
        let packages = path.join("packages");
        std::fs::create_dir(&packages).unwrap();
        symlink(&target, packages.join(format!("{digest}.pkg"))).unwrap();
        let directory = Dir::open_ambient_dir(&packages, cap_std::ambient_authority()).unwrap();
        assert_eq!(
            ensure_package_storage_room(&directory, &digest, bytes.len() as u64),
            Err(DownloadError::Storage)
        );
        assert_eq!(
            remove_verified_package(&directory, &digest, bytes.len() as u64),
            Err(DownloadError::Storage)
        );
        assert_eq!(std::fs::read(&target).unwrap(), bytes);
        drop(directory);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn package_cleanup_removes_only_the_exact_verified_content_addressed_file() {
        let path = temporary_directory("update-package-cleanup");
        let directory = Dir::open_ambient_dir(&path, cap_std::ambient_authority()).unwrap();
        let bytes = b"verified staged package";
        let digest = hex::encode(Sha256::digest(bytes));
        let package_path = path.join(format!("{digest}.pkg"));
        std::fs::write(&package_path, bytes).unwrap();
        assert_eq!(
            remove_verified_package(&directory, &digest, (bytes.len() + 1) as u64),
            Err(DownloadError::Storage)
        );
        assert_eq!(
            remove_verified_package(&directory, &"a".repeat(64), bytes.len() as u64),
            Ok(false)
        );
        assert!(package_path.exists());
        assert_eq!(
            remove_verified_package(&directory, &digest, bytes.len() as u64),
            Ok(true)
        );
        assert!(!package_path.exists());
        assert_eq!(
            remove_verified_package(&directory, &digest, bytes.len() as u64),
            Ok(false)
        );
        drop(directory);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn corrupt_same_digest_package_is_atomically_replaced_after_verification() {
        let path = temporary_directory("update-package-repair");
        let directory = Dir::open_ambient_dir(&path, cap_std::ambient_authority()).unwrap();
        let bytes = b"verified-package";
        let digest = hex::encode(Sha256::digest(bytes));
        let final_name = format!("{digest}.pkg");
        let temporary_name = format!("{}.part", "a".repeat(32));
        std::fs::write(path.join(&final_name), b"tampered-package").unwrap();
        std::fs::write(path.join(&temporary_name), bytes).unwrap();

        assert_eq!(
            verify_existing_package(
                &directory,
                &digest,
                bytes.len() as u64,
                &AtomicBool::new(false),
                std::time::Instant::now() + Duration::from_secs(5),
                &mut |_| {},
            )
            .unwrap(),
            ExistingPackageState::Corrupt
        );
        assert_eq!(
            publish_verified_package(&directory, &temporary_name, &final_name, true),
            Ok(PackageDisposition::RepairedCorrupt)
        );
        assert_eq!(std::fs::read(path.join(&final_name)).unwrap(), bytes);
        assert!(!path.join(&temporary_name).exists());
        drop(directory);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn package_repair_refuses_non_file_destination_and_cleans_temporary_bytes() {
        let path = temporary_directory("update-package-repair-unsafe-entry");
        let directory = Dir::open_ambient_dir(&path, cap_std::ambient_authority()).unwrap();
        let final_name = format!("{}.pkg", "b".repeat(64));
        let temporary_name = format!("{}.part", "c".repeat(32));
        std::fs::create_dir(path.join(&final_name)).unwrap();
        std::fs::write(path.join(&temporary_name), b"verified").unwrap();

        assert_eq!(
            publish_verified_package(&directory, &temporary_name, &final_name, true),
            Err(DownloadError::Storage)
        );
        assert!(path.join(&final_name).is_dir());
        assert!(!path.join(&temporary_name).exists());
        drop(directory);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn missing_package_uses_create_only_publication() {
        let path = temporary_directory("update-package-create-only");
        let directory = Dir::open_ambient_dir(&path, cap_std::ambient_authority()).unwrap();
        let final_name = format!("{}.pkg", "d".repeat(64));
        let temporary_name = format!("{}.part", "e".repeat(32));
        std::fs::write(path.join(&temporary_name), b"first").unwrap();
        assert_eq!(
            publish_verified_package(&directory, &temporary_name, &final_name, false),
            Ok(PackageDisposition::Downloaded)
        );
        std::fs::write(path.join(&temporary_name), b"second").unwrap();
        assert_eq!(
            publish_verified_package(&directory, &temporary_name, &final_name, false),
            Err(DownloadError::Storage)
        );
        assert_eq!(std::fs::read(path.join(&final_name)).unwrap(), b"first");
        assert!(!path.join(&temporary_name).exists());
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

    fn local_tls_endpoint(
        path: &'static str,
        bytes: &'static [u8],
        timeout: Duration,
    ) -> (
        Client,
        Url,
        thread::JoinHandle<Result<String, String>>,
        Certificate,
        Vec<SocketAddr>,
    ) {
        local_tls_response_endpoint(path, bytes, timeout, "200 OK", "application/json")
    }

    fn local_tls_response_endpoint(
        path: &'static str,
        bytes: &'static [u8],
        timeout: Duration,
        status: &'static str,
        content_type: &'static str,
    ) -> (
        Client,
        Url,
        thread::JoinHandle<Result<String, String>>,
        Certificate,
        Vec<SocketAddr>,
    ) {
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
        let server = thread::spawn(move || -> Result<String, String> {
            let (stream, peer) = listener
                .accept()
                .map_err(|error| format!("accept失敗: {error}"))?;
            let local = stream
                .local_addr()
                .map_err(|error| format!("local_addr取得失敗: {error}"))?;
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .map_err(|error| format!("read timeout設定失敗: {error}"))?;
            stream
                .set_write_timeout(Some(Duration::from_secs(10)))
                .map_err(|error| format!("write timeout設定失敗: {error}"))?;
            let connection = rustls::ServerConnection::new(Arc::new(server_config)).unwrap();
            let mut tls = rustls::StreamOwned::new(connection, stream);
            let mut request = String::new();
            let mut read_failure = None;
            {
                let mut reader = BufReader::new(&mut tls);
                loop {
                    let mut line = String::new();
                    if let Err(error) = reader.read_line(&mut line) {
                        read_failure = Some(format!("{:?}: {error}", error.kind()));
                        break;
                    }
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    request.push_str(&line);
                    if request.len() > 16 * 1024 {
                        return Err(format!(
                            "peer={peer}; local={local}; TLS要求header上限超過; bytes={}",
                            request.len()
                        ));
                    }
                }
            }
            if let Some(error) = read_failure {
                return Err(format!(
                    "peer={peer}; local={local}; tls_handshaking={}; request_bytes={}; read_error={error}",
                    tls.conn.is_handshaking(),
                    request.len()
                ));
            }
            let request_target = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1));
            if request_target != Some(path) {
                return Err(format!(
                    "peer={peer}; local={local}; TLS要求先不一致; target={request_target:?}"
                ));
            }
            write!(
                tls,
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            )
            .map_err(|error| {
                format!("peer={peer}; local={local}; response header書込失敗: {error}")
            })?;
            tls.write_all(bytes).map_err(|error| {
                format!("peer={peer}; local={local}; response body書込失敗: {error}")
            })?;
            tls.flush().map_err(|error| {
                format!("peer={peer}; local={local}; response flush失敗: {error}")
            })?;
            tls.conn.send_close_notify();
            tls.flush().map_err(|error| {
                format!("peer={peer}; local={local}; close_notify flush失敗: {error}")
            })?;
            tls.sock
                .set_read_timeout(Some(Duration::from_secs(2)))
                .map_err(|error| {
                    format!("peer={peer}; local={local}; drain timeout設定失敗: {error}")
                })?;
            let mut discarded = [0u8; 512];
            let peer_close = loop {
                match tls.read(&mut discarded) {
                    Ok(0) => break "graceful_tls_close",
                    Ok(read) => {
                        return Err(format!(
                            "peer={peer}; local={local}; response後に予期しないTLS data; bytes={read}"
                        ));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {
                        break "reset_after_response";
                    }
                    Err(error) => {
                        return Err(format!(
                            "peer={peer}; local={local}; response後peer close観測失敗: {error}"
                        ));
                    }
                }
            };
            let authorization_header = request.lines().any(|line| {
                line.split_once(':').is_some_and(|(name, _)| {
                    name.eq_ignore_ascii_case("authorization")
                        || name.eq_ignore_ascii_case("proxy-authorization")
                })
            });
            Ok(format!(
                "peer={peer}; local={local}; response=written; peer_close={peer_close}; authorization_header={authorization_header}"
            ))
        });
        let root = Certificate::from_der(certified.cert.der().as_ref()).unwrap();
        let client =
            build_download_client("localhost", &[address], &[root.clone()], timeout).unwrap();
        let url = Url::parse(&format!("https://localhost:{}{path}", address.port())).unwrap();
        (client, url, server, root, vec![address])
    }

    #[test]
    fn a2a_https_agent_card_accepts_verified_local_fixture_and_never_sends_credentials() {
        let bytes = br#"{"name":"fixture-agent"}"#;
        let (_client, url, server, root, addresses) = local_tls_endpoint(
            "/.well-known/agent-card.json",
            bytes,
            A2A_TLS_FIXTURE_TIMEOUT,
        );
        let result = fetch_a2a_agent_card_document_with_addresses(
            url,
            "localhost",
            &addresses,
            &[root],
            MAX_A2A_CARD_BYTES,
            std::time::Instant::now() + A2A_TLS_FIXTURE_TIMEOUT,
        );
        let observation = server.join().unwrap();
        assert!(
            observation.is_ok(),
            "TLS fixture failed before producing the expected response: {observation:?}; client result: {result:?}"
        );
        assert_eq!(result.unwrap(), bytes);
        let observation = observation.unwrap();
        assert!(observation.contains("authorization_header=false"));
    }

    #[test]
    fn a2a_https_agent_card_rejects_untrusted_certificate() {
        let bytes = br#"{"name":"fixture-agent"}"#;
        let (_client, url, server, _, addresses) =
            local_tls_endpoint("/card", bytes, A2A_TLS_FIXTURE_TIMEOUT);
        let unrelated =
            rcgen::generate_simple_self_signed(vec!["other.example".to_string()]).unwrap();
        let unrelated_root = Certificate::from_der(unrelated.cert.der().as_ref()).unwrap();
        let error = fetch_a2a_agent_card_document_with_addresses(
            url,
            "localhost",
            &addresses,
            &[unrelated_root],
            MAX_A2A_CARD_BYTES,
            std::time::Instant::now() + A2A_TLS_FIXTURE_TIMEOUT,
        )
        .unwrap_err();
        assert_eq!(error, DownloadError::Network);
        assert!(server.join().unwrap().is_err());
    }

    #[test]
    fn a2a_https_agent_card_rejects_redirect_wrong_media_type_and_oversize() {
        for (status, content_type, body, max_bytes, expected) in [
            (
                "302 Found",
                "application/json",
                b"{}".as_slice(),
                MAX_A2A_CARD_BYTES,
                DownloadError::RedirectOrUnexpectedStatus,
            ),
            (
                "200 OK",
                "text/html",
                b"{}".as_slice(),
                MAX_A2A_CARD_BYTES,
                DownloadError::InvalidHeaders,
            ),
            (
                "200 OK",
                "application/json",
                b"12345".as_slice(),
                4,
                DownloadError::DocumentTooLarge,
            ),
        ] {
            let (_client, url, server, root, addresses) = local_tls_response_endpoint(
                "/card",
                body,
                A2A_TLS_FIXTURE_TIMEOUT,
                status,
                content_type,
            );
            let result = fetch_a2a_agent_card_document_with_addresses(
                url,
                "localhost",
                &addresses,
                &[root],
                max_bytes,
                std::time::Instant::now() + A2A_TLS_FIXTURE_TIMEOUT,
            );
            let observation = server.join().unwrap();
            assert!(
                observation.is_ok(),
                "TLS fixture failed before producing the expected response: {observation:?}; expected client result: {expected:?}; actual: {result:?}"
            );
            assert_eq!(result.unwrap_err(), expected);
        }
    }

    #[test]
    fn a2a_response_header_budget_enforces_total_bytes_and_field_count() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        assert!(a2a_response_headers_within_bound(&headers));

        let padding = "x".repeat(MAX_A2A_HEADER_BYTES);
        headers.insert(
            reqwest::header::HeaderName::from_static("x-padding"),
            reqwest::header::HeaderValue::from_str(&padding).unwrap(),
        );
        assert!(!a2a_response_headers_within_bound(&headers));

        let mut headers = reqwest::header::HeaderMap::new();
        for index in 0..=MAX_A2A_HEADER_COUNT {
            headers.insert(
                reqwest::header::HeaderName::from_bytes(format!("x-{index}").as_bytes()).unwrap(),
                "v".parse().unwrap(),
            );
        }
        assert!(!a2a_response_headers_within_bound(&headers));
    }

    #[test]
    fn a2a_https_public_address_resolution_rejects_loopback_before_connecting() {
        assert_eq!(
            fetch_a2a_agent_card_document(
                "https://127.0.0.1/card",
                std::time::Instant::now() + Duration::from_secs(1),
            )
            .unwrap_err(),
            DownloadError::NonPublicAddress
        );
    }

    #[test]
    fn system_dns_deadline_limits_wait_and_fails_closed_while_worker_is_stuck() {
        static TEST_DNS_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
        let in_flight = &TEST_DNS_IN_FLIGHT;
        let started = std::time::Instant::now();
        let error = resolve_system_addresses_with(
            "slow.invalid",
            443,
            &AtomicBool::new(false),
            started + Duration::from_millis(10),
            in_flight,
            |_, _| {
                thread::sleep(Duration::from_millis(120));
                Ok(vec![SocketAddr::from(([8, 8, 8, 8], 443))])
            },
        )
        .unwrap_err();
        assert_eq!(error, DownloadError::TimedOut);
        assert!(started.elapsed() < Duration::from_millis(100));
        assert_eq!(
            resolve_system_addresses_with(
                "second.invalid",
                443,
                &AtomicBool::new(false),
                std::time::Instant::now() + Duration::from_secs(1),
                in_flight,
                |_, _| panic!("同時resolver workerを起動しない"),
            )
            .unwrap_err(),
            DownloadError::ResolverBusy
        );
        thread::sleep(Duration::from_millis(150));
        assert!(!in_flight.load(Ordering::Acquire));
    }

    #[test]
    fn bounded_catalog_fetch_accepts_small_https_document() {
        let bytes = r#"{"版":1,"候補":[]}"#.as_bytes();
        let (client, url, server, _, _) =
            local_tls_endpoint("/updates.json", bytes, CATALOG_TIMEOUT);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(async {
            let response = client
                .get(url)
                .header(ACCEPT_ENCODING, "identity")
                .send()
                .await
                .map_err(|error| format!("更新一覧のHTTP要求失敗: {error:?}"))?;
            receive_bounded_document(
                response,
                MAX_CATALOG_BYTES,
                std::time::Instant::now() + CATALOG_TIMEOUT,
            )
            .await
            .map_err(|error| format!("更新一覧応答の読取り失敗: {error:?}"))
        });
        let server_result = server
            .join()
            .unwrap_or_else(|_| Err("TLS試験用server threadがpanicした".to_string()));
        let result = result.unwrap_or_else(|client_error| {
            panic!("更新一覧受信失敗: client={client_error}; server={server_result:?}")
        });
        let server_observation = server_result
            .unwrap_or_else(|server_error| panic!("更新一覧TLS試験用server失敗: {server_error}"));
        assert!(server_observation.contains("response=written"));
        assert_eq!(result, bytes);
    }

    #[test]
    fn bounded_catalog_fetch_rejects_declared_document_over_limit() {
        let bytes = b"oversized";
        let (client, url, server, _, _) =
            local_tls_endpoint("/updates.json", bytes, CATALOG_TIMEOUT);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(async {
            let response = client
                .get(url)
                .header(ACCEPT_ENCODING, "identity")
                .send()
                .await
                .map_err(|error| format!("{error:?}"))?;
            Ok::<_, String>(
                receive_bounded_document(response, 4, std::time::Instant::now() + CATALOG_TIMEOUT)
                    .await,
            )
        });
        let server_result = runtime
            .block_on(async { tokio::task::spawn_blocking(move || server.join()).await })
            .expect("TLS fixture join workerがpanicした")
            .unwrap_or_else(|_| Err("TLS fixture server threadがpanicした".to_string()));
        let result = result.unwrap_or_else(|error| {
            panic!("TLS catalog要求失敗: client={error}; server={server_result:?}")
        });
        let server_observation = server_result.unwrap();
        assert!(server_observation.contains("response=written"));
        assert_eq!(result, Err(DownloadError::DocumentTooLarge));
    }

    #[test]
    fn catalog_fetch_rejects_expired_operation_deadline_before_dns() {
        let deadline = std::time::Instant::now() - Duration::from_secs(1);
        assert_eq!(
            fetch_update_catalog(
                "https://updates.example.invalid/d4/stable/updates.json",
                MAX_CATALOG_BYTES,
                deadline,
            ),
            Err(DownloadError::TimedOut),
        );
    }

    #[test]
    #[ignore = "Linux installed acceptanceでreqwestの明示ring provider初期化を確認する"]
    fn catalog_client_builds_with_the_explicit_ring_crypto_provider() {
        let addresses = [SocketAddr::from((Ipv4Addr::new(8, 8, 8, 8), 443))];
        let result =
            build_download_client("updates.example.invalid", &addresses, &[], CATALOG_TIMEOUT);
        assert!(result.is_ok());
    }

    #[cfg(feature = "r2-e2e")]
    #[test]
    #[ignore = "Linux same-source installed acceptanceでrun-scoped TLS trustを確認する"]
    fn same_source_catalog_client_accepts_the_run_scoped_ca_and_host_certificate() {
        let host = R2_E2E_UPDATE_HOST;
        let address = r2_e2e_update_address(host, 443)
            .expect("same-source acceptance hostはloopback TLSへ固定される");
        let url = Url::parse(&format!("https://{host}/d4/stable/updates.json")).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let body = runtime.block_on(async {
            let client = build_update_download_client(host, &[address], CATALOG_TIMEOUT)
                .expect("run-scoped CAから同一source用TLS clientを構築する");
            let response = client.get(url).send().await.unwrap_or_else(|error| {
                panic!("run-scoped TLS server証明書を検証できない: {error:?}")
            });
            assert_eq!(response.status(), StatusCode::OK);
            response
                .bytes()
                .await
                .expect("TLS受信後のcatalog bodyを読む")
        });
        assert!(!body.is_empty());
        assert!(body.len() <= MAX_CATALOG_BYTES);
    }

    #[test]
    fn local_tls_server_repairs_only_after_verified_package_bytes() {
        let bytes = b"locally served signed package";
        let digest = hex::encode(Sha256::digest(bytes));
        let (client, url, server, _, _) = local_tls_endpoint("/update.pkg", bytes, CATALOG_TIMEOUT);
        let path = temporary_directory("update-local-tls");
        let directory = Dir::open_ambient_dir(&path, cap_std::ambient_authority()).unwrap();
        std::fs::write(path.join(format!("{digest}.pkg")), vec![b'x'; bytes.len()]).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(async {
            let response = client
                .get(url)
                .header(ACCEPT_ENCODING, "identity")
                .send()
                .await
                .map_err(|error| format!("TLS要求失敗: {error:?}"))?;
            receive_package(
                &directory,
                response,
                &digest,
                bytes.len() as u64,
                true,
                &AtomicBool::new(false),
                std::time::Instant::now() + Duration::from_secs(30),
                &mut |_| {},
            )
            .await
            .map_err(|error| format!("package受信失敗: {error:?}"))
        });
        let server_result = server
            .join()
            .unwrap_or_else(|_| Err("TLS fixture server threadがpanicした".to_string()));
        let result = result.unwrap_or_else(|client_error| {
            panic!("TLS package修復失敗: client={client_error}; server={server_result:?}")
        });
        let server_observation = server_result
            .unwrap_or_else(|server_error| panic!("TLS fixture server失敗: {server_error}"));
        assert!(server_observation.contains("response=written"));
        assert_eq!(result.file_name, format!("{digest}.pkg"));
        assert_eq!(result.disposition, PackageDisposition::RepairedCorrupt);
        assert_eq!(std::fs::read(path.join(&result.file_name)).unwrap(), bytes);
        drop(directory);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn failed_replacement_keeps_the_existing_corrupt_package_unchanged() {
        let response_bytes = b"wrong-content";
        let trusted_bytes = b"right-content";
        let trusted_digest = hex::encode(Sha256::digest(trusted_bytes));
        let (client, url, server, _, _) =
            local_tls_endpoint("/update.pkg", response_bytes, CATALOG_TIMEOUT);
        let path = temporary_directory("update-package-repair-failed-transfer");
        let directory = Dir::open_ambient_dir(&path, cap_std::ambient_authority()).unwrap();
        let final_name = format!("{trusted_digest}.pkg");
        let old_corrupt_bytes = b"tampered-data";
        assert_eq!(old_corrupt_bytes.len(), trusted_bytes.len());
        std::fs::write(path.join(&final_name), old_corrupt_bytes).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(async {
            let response = client
                .get(url)
                .header(ACCEPT_ENCODING, "identity")
                .send()
                .await
                .map_err(|error| format!("{error:?}"))?;
            Ok::<_, String>(
                receive_package(
                    &directory,
                    response,
                    &trusted_digest,
                    trusted_bytes.len() as u64,
                    true,
                    &AtomicBool::new(false),
                    std::time::Instant::now() + Duration::from_secs(30),
                    &mut |_| {},
                )
                .await,
            )
        });
        let server_result = server.join().unwrap();
        let result = result.unwrap_or_else(|error| {
            panic!("TLS取得失敗: client={error}; server={server_result:?}")
        });
        let server_observation = server_result.unwrap();
        assert!(server_observation.contains("response=written"));
        assert_eq!(result, Err(DownloadError::DigestMismatch));
        assert_eq!(
            std::fs::read(path.join(&final_name)).unwrap(),
            old_corrupt_bytes
        );
        assert_eq!(std::fs::read_dir(&path).unwrap().count(), 1);
        drop(directory);
        std::fs::remove_dir_all(path).unwrap();
    }
}
