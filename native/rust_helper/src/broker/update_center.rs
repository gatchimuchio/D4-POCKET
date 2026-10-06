//! C11/C12 更新センターと、P11 Windows製品化用のBroker consumer経路。
//!
//! 更新候補の表示・署名検査・延期・download・未起動版stage・有効版record切替・
//! Broker記録に基づくrollbackを扱う。候補に含まれる公開鍵、metadata、Profile、
//! 履歴は信頼源にならない。署名鍵はBroker所有の永続設定だけから読み、実製品への
//! install／通常起動／installed rollback証拠は別のWindows product gateで扱う。
#![allow(non_snake_case)]

use super::protocol::{
    Broker, BrokerError, BrokerOperation, BrokerResponse, BrokerStatus, OwnerConfirmationSource,
    EVIDENCE_SOURCE_INTERNAL_STATE,
};
use super::store::{BrokerPersistentStore, BrokerStoreError};
use crate::audit_hash::sha256_tagged;
use crate::update_verification::{verify_signed_update_signature, SignedUpdateCandidate};
#[cfg(windows)]
use cap_fs_ext::OsMetadataExt as _;
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::OpenOptions;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;

const VERSION: u64 = 1;
const TRUST_VERSION: u64 = 2;
const CANDIDATE_VERSION: u64 = 2;
const MAX_UPDATES: usize = 64;
const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_CATALOG_CANDIDATES: usize = 64;
const MAX_PACKAGE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const OP_LIST: &str = "更新一覧";
const OP_FETCH_CATALOG: &str = "更新候補取得";
const OP_CHECK: &str = "更新確認";
const OP_VERIFY: &str = "更新署名検査";
const OP_DOWNLOAD: &str = "更新download要求";
const OP_APPLY: &str = "更新適用要求";
const OP_ACTIVATE: &str = "更新有効版切替要求";
const OP_DEFER: &str = "更新延期";
const OP_ROLLBACK: &str = "更新rollback要求";
const OP_PRODUCT_REPAIR: &str = "製品起動項目修復要求";
const OP_UNINSTALL: &str = "製品アンインストール要求";
const OP_UNINSTALL_STARTED: &str = "製品アンインストール開始";
const OP_UNINSTALL_COMPLETED: &str = "製品アンインストール完了";
const OP_UNINSTALL_FAILED: &str = "製品アンインストール失敗";
const EMBEDDED_PRODUCT_APP_ID: Option<&str> = option_env!("GUI_SHELL_PRODUCT_APP_ID");
const EMBEDDED_PRODUCT_AUDIT_STORE_ID: Option<&str> =
    option_env!("GUI_SHELL_PRODUCT_AUDIT_STORE_ID");
const EMBEDDED_PRODUCT_UPDATE_TRUST_JSON: Option<&str> =
    option_env!("GUI_SHELL_UPDATE_TRUST_JSON");

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UpdateDownloadConfirmation {
    pub(crate) update_id: String,
    pub(crate) candidate_hash: String,
    pub(crate) source_url: String,
    pub(crate) package_sha256: String,
    pub(crate) package_size_bytes: u64,
    pub(crate) offered_version: String,
    pub(crate) channel: String,
    pub(crate) summary: String,
    pub(crate) display_host: String,
    pub(crate) payload_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UpdateApplyConfirmation {
    pub(crate) download: UpdateDownloadConfirmation,
    pub(crate) app_id: String,
    pub(crate) audit_store_id: String,
    pub(crate) version_directory: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UpdateActivationConfirmation {
    pub(crate) update_id: String,
    pub(crate) candidate_hash: String,
    pub(crate) package_sha256: String,
    pub(crate) package_size_bytes: u64,
    pub(crate) offered_version: String,
    pub(crate) channel: String,
    pub(crate) summary: String,
    pub(crate) payload_hash: String,
    pub(crate) app_id: String,
    pub(crate) audit_store_id: String,
    pub(crate) version_directory: PathBuf,
    pub(crate) start_menu_shortcut_path: PathBuf,
    pub(crate) rollback_target: Option<UpdateRollbackTargetConfirmation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductUninstallConfirmation {
    pub(crate) app_id: String,
    pub(crate) audit_store_id: String,
    pub(crate) product_root: PathBuf,
    pub(crate) start_menu_shortcut_path: PathBuf,
    pub(crate) payload_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductRepairConfirmation {
    pub(crate) app_id: String,
    pub(crate) audit_store_id: String,
    pub(crate) update_id: String,
    pub(crate) candidate_hash: String,
    pub(crate) product_version: String,
    pub(crate) package_sha256: String,
    pub(crate) product_root: PathBuf,
    pub(crate) start_menu_shortcut_path: PathBuf,
    pub(crate) payload_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UpdateRollbackTargetConfirmation {
    pub(crate) update_id: String,
    pub(crate) candidate_hash: String,
    pub(crate) package_sha256: String,
    pub(crate) offered_version: String,
    pub(crate) version_directory: PathBuf,
}

#[derive(Debug, Clone)]
struct UpdateDownloadJob {
    job_id: String,
    update_id: String,
    candidate_hash: String,
    package_sha256: String,
    expected_bytes: u64,
    state: String,
    error_code: Option<String>,
    progress: Arc<AtomicU64>,
}

#[derive(Debug)]
struct UpdateDownloadCompletion {
    result:
        Result<super::update_download::DownloadedPackage, super::update_download::DownloadError>,
}

#[derive(Debug, Default)]
pub(crate) struct UpdateDownloadRuntime {
    job: Option<UpdateDownloadJob>,
    completion_rx: Option<mpsc::Receiver<UpdateDownloadCompletion>>,
    worker: Option<JoinHandle<()>>,
    cancel: Option<Arc<AtomicBool>>,
}

impl UpdateDownloadRuntime {
    fn is_running(&self) -> bool {
        self.job
            .as_ref()
            .is_some_and(|job| job.state == "downloading")
    }

    fn projection(&self) -> Value {
        let Some(job) = &self.job else {
            return Value::Null;
        };
        json!({
            "job_id": job.job_id,
            "更新ID": job.update_id,
            "候補hash": job.candidate_hash,
            "package_sha256": job.package_sha256,
            "状態": job.state,
            "受信byte数": job.progress.load(Ordering::Acquire).min(job.expected_bytes),
            "全byte数": job.expected_bytes,
            "失敗code": job.error_code,
        })
    }

    fn start(
        &mut self,
        directory: cap_std::fs::Dir,
        request: &UpdateIDRequest,
        record: &UpdateRecord,
        source_url: String,
    ) -> Result<Value, &'static str> {
        if self.is_running() {
            return Err("update_download_busy");
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let package_sha256 = record
            .candidate
            .package_sha256
            .as_deref()
            .ok_or("update_package_binding_required")?;
        let expected_bytes = record
            .candidate
            .package_size_bytes
            .ok_or("update_package_binding_required")?;
        super::update_download::ensure_package_storage_room(
            &directory,
            package_sha256,
            expected_bytes,
        )
        .map_err(|error| download_error_code(&error))?;

        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).map_err(|_| "update_download_job_id_failed")?;
        let job_id = format!("update-download-{}", hex::encode(random));
        let progress = Arc::new(AtomicU64::new(0));
        let worker_progress = Arc::clone(&progress);
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let (completion_tx, completion_rx) = mpsc::sync_channel(1);
        let expected_sha256 = package_sha256.to_owned();
        let worker = std::thread::Builder::new()
            .name("d4p-update-download".to_string())
            .spawn(move || {
                let result = super::update_download::download_package(
                    &directory,
                    &source_url,
                    &expected_sha256,
                    expected_bytes,
                    &worker_cancel,
                    |bytes| worker_progress.store(bytes, Ordering::Release),
                );
                let _ = completion_tx.send(UpdateDownloadCompletion { result });
            })
            .map_err(|_| "update_download_worker_spawn_failed")?;
        self.job = Some(UpdateDownloadJob {
            job_id: job_id.clone(),
            update_id: request.update_id.clone(),
            candidate_hash: request.candidate_hash.clone(),
            package_sha256: package_sha256.to_owned(),
            expected_bytes,
            state: "downloading".to_string(),
            error_code: None,
            progress,
        });
        self.completion_rx = Some(completion_rx);
        self.worker = Some(worker);
        self.cancel = Some(cancel);
        Ok(self.projection())
    }

    fn take_completion(
        &mut self,
    ) -> Option<(
        String,
        String,
        String,
        Result<super::update_download::DownloadedPackage, super::update_download::DownloadError>,
    )> {
        let completion = match self.completion_rx.as_ref()?.try_recv() {
            Ok(completion) => completion,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => UpdateDownloadCompletion {
                result: Err(super::update_download::DownloadError::Network),
            },
        };
        self.completion_rx = None;
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.cancel = None;
        let job = self.job.as_mut()?;
        let (decision, state, error_code) = match &completion.result {
            Ok(package)
                if package.sha256 == job.package_sha256 && package.bytes == job.expected_bytes =>
            {
                job.progress.store(package.bytes, Ordering::Release);
                ("completed", "downloaded", None)
            }
            Ok(_) => ("failed", "failed", Some("update_download_result_mismatch")),
            Err(error) => ("failed", "failed", Some(download_error_code(error))),
        };
        job.state = state.to_string();
        job.error_code = error_code.map(str::to_string);
        Some((
            job.job_id.clone(),
            decision.to_string(),
            job.candidate_hash.clone(),
            completion.result,
        ))
    }

    fn audit_failed(&mut self) {
        if let Some(job) = &mut self.job {
            job.state = "audit_failed".to_string();
            job.error_code = Some("update_download_audit_failed".to_string());
        }
    }
}

impl Drop for UpdateDownloadRuntime {
    fn drop(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.store(true, Ordering::Release);
        }
    }
}

fn download_error_code(error: &super::update_download::DownloadError) -> &'static str {
    use super::update_download::DownloadError as E;
    match error {
        E::InvalidRequest => "update_download_request_invalid",
        E::NameResolution => "update_download_dns_failed",
        E::NonPublicAddress => "update_download_address_blocked",
        E::Network => "update_download_network_failed",
        E::RedirectOrUnexpectedStatus => "update_download_response_rejected",
        E::InvalidHeaders => "update_download_headers_invalid",
        E::SizeMismatch => "update_download_size_mismatch",
        E::DocumentTooLarge => "update_download_size_mismatch",
        E::DigestMismatch => "update_download_digest_mismatch",
        E::Cancelled => "update_download_cancelled",
        E::TimedOut => "update_download_timeout",
        E::ResolverBusy => "update_download_dns_unavailable",
        E::DnsCancellationFailed => "update_download_dns_cancel_failed",
        E::Storage => "update_download_storage_failed",
    }
}

pub(super) fn poll_download_completion(broker: &mut Broker) {
    let Some((job_id, decision, candidate_hash, result)) = broker.update_download.take_completion()
    else {
        return;
    };
    let (decision, reason) = if decision == "completed" {
        let Ok(package) = result else {
            broker.update_download.audit_failed();
            return;
        };
        match package.disposition {
            super::update_download::PackageDisposition::AlreadyVerified => (
                "completed",
                format!("Capability=署名済みupdate package取得 Permission=Broker固定store内の既存packageを全byte再検証 Approval=対象候補とHTTPS hostのRust Desktop native Owner確認済み AuditEvent=既存packageの一致を記録 RecoveryAction=適用前にもfile全体を再hash検証。ここではinstall／process起動しない。{} byte", package.bytes),
            ),
            super::update_download::PackageDisposition::Downloaded => (
                "completed",
                format!("Capability=署名済みupdate package取得 Permission=Broker固定package directoryへ検証済み{} byteを新規保存 Approval=対象候補とHTTPS hostのRust Desktop native Owner確認済み AuditEvent=download完了を記録 RecoveryAction=適用時にfileを再度hash検証。ここではinstall／process起動しない", package.bytes),
            ),
            super::update_download::PackageDisposition::RepairedCorrupt => (
                "recovered",
                format!("Capability=署名済みupdate package取得 Permission=同一digest名の通常fileだけを原子的に置換 Approval=対象候補とHTTPS hostのRust Desktop native Owner確認済み AuditEvent=既存fileの長さ／hash不一致と検証済みpackageへの置換を記録 RecoveryAction=適用前にfile全体を再hash検証。ここではinstall／process起動しない。{} byte", package.bytes),
            ),
        }
    } else {
        (
            "failed",
            "Capability=署名済みupdate package取得 Permission=Broker固定HTTPS接続と一時file書込みを試行 Approval=対象候補とHTTPS hostのRust Desktop native Owner確認済み AuditEvent=失敗を記録 RecoveryAction=final packageは実行せず、次回明示要求時に固定storeを全byte再検証して状態を再判定".to_string(),
        )
    };
    if broker
        .append_audit(
            &format!("{job_id}:result"),
            OP_DOWNLOAD,
            decision,
            &reason,
            "LIVE_RUNTIME",
            &candidate_hash,
        )
        .is_err()
    {
        broker.update_download.audit_failed();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateCandidateDocument {
    #[serde(rename = "版")]
    pub(super) version: u64,
    #[serde(rename = "更新ID")]
    pub(super) update_id: String,
    #[serde(rename = "現行版")]
    pub(super) current_version: String,
    #[serde(rename = "提供版")]
    pub(super) offered_version: String,
    #[serde(rename = "channel")]
    pub(super) channel: String,
    #[serde(rename = "内容概要")]
    pub(super) summary: String,
    #[serde(
        rename = "package_sha256",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub(super) package_sha256: Option<String>,
    #[serde(
        rename = "package_size_bytes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub(super) package_size_bytes: Option<u64>,
    #[serde(rename = "署名対象")]
    pub(super) signed_bytes_hex: String,
    #[serde(rename = "署名")]
    pub(super) signature_hex: String,
    #[serde(rename = "署名者fingerprint")]
    pub(super) signer_fingerprint: String,
    #[serde(rename = "rollback可能")]
    pub(super) rollback_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateRecord {
    #[serde(flatten)]
    pub(super) candidate: UpdateCandidateDocument,
    #[serde(rename = "署名状態")]
    pub(super) signature_status: String,
    #[serde(rename = "候補hash")]
    pub(super) candidate_hash: String,
    #[serde(rename = "延期期限")]
    pub(super) deferred_until: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdatePackageSource {
    pub(super) channel: String,
    pub(super) base_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateTrust {
    #[serde(rename = "版")]
    pub(super) version: u64,
    pub(super) algorithm: String,
    pub(super) public_key_der_hex: String,
    pub(super) public_key_fingerprint: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) package_sources: Vec<UpdatePackageSource>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "候補")]
    candidate: UpdateCandidateDocument,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "候補")]
    candidates: Vec<UpdateCandidateDocument>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateCatalogDocument {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "候補")]
    candidates: Vec<UpdateCandidateDocument>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateIDRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "更新ID")]
    update_id: String,
    #[serde(rename = "候補hash")]
    candidate_hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeferRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "更新ID")]
    update_id: String,
    #[serde(rename = "候補hash")]
    candidate_hash: String,
    #[serde(rename = "延期期限")]
    deferred_until: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SignedManifest<'a> {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "更新ID")]
    update_id: &'a str,
    #[serde(rename = "現行版")]
    current_version: &'a str,
    #[serde(rename = "提供版")]
    offered_version: &'a str,
    #[serde(rename = "channel")]
    channel: &'a str,
    #[serde(rename = "内容概要")]
    summary: &'a str,
    #[serde(rename = "package_sha256")]
    package_sha256: &'a str,
    #[serde(rename = "package_size_bytes")]
    package_size_bytes: u64,
    #[serde(rename = "rollback可能")]
    rollback_available: bool,
}

pub(super) fn load_persistent_updates(
    store: &BrokerPersistentStore,
) -> Result<BTreeMap<String, Value>, BrokerStoreError> {
    let state = store.load_update_state()?;
    decode_state(&state).map_err(BrokerStoreError::MalformedUpdateState)
}

pub(super) fn load_persistent_trust(
    store: &BrokerPersistentStore,
) -> Result<Option<UpdateTrust>, BrokerStoreError> {
    let Some(value) = store.load_update_trust()? else {
        return Ok(None);
    };
    let trust: UpdateTrust = serde_json::from_value(value).map_err(|_| {
        BrokerStoreError::MalformedUpdateTrust(
            "broker update trustのfieldまたは版が不正である".to_string(),
        )
    })?;
    validate_trust(&trust).map_err(BrokerStoreError::MalformedUpdateTrust)?;
    Ok(Some(trust))
}

pub(super) fn load_effective_persistent_trust(
    store: &BrokerPersistentStore,
) -> Result<Option<UpdateTrust>, BrokerStoreError> {
    resolve_build_update_trust(
        EMBEDDED_PRODUCT_APP_ID,
        EMBEDDED_PRODUCT_AUDIT_STORE_ID,
        EMBEDDED_PRODUCT_UPDATE_TRUST_JSON,
        || load_persistent_trust(store),
    )
}

fn resolve_build_update_trust(
    app_id: Option<&str>,
    audit_store_id: Option<&str>,
    embedded_json: Option<&str>,
    load_persistent: impl FnOnce() -> Result<Option<UpdateTrust>, BrokerStoreError>,
) -> Result<Option<UpdateTrust>, BrokerStoreError> {
    match (app_id, audit_store_id, embedded_json) {
        (None, None, None) => load_persistent(),
        (Some(app_id), Some(audit_store_id), embedded_json) => {
            if !valid_product_identity(app_id, "d4-pocket-app-")
                || !valid_product_identity(audit_store_id, "audit-store-")
            {
                return Err(BrokerStoreError::MalformedUpdateTrust(
                    "製品build identityが不正である".to_string(),
                ));
            }
            let Some(raw) = embedded_json else {
                // 製品構成では、変更可能な永続保存値を更新の信頼元にしない。
                return Ok(None);
            };
            if raw.len() > 8 * 1024 {
                return Err(BrokerStoreError::MalformedUpdateTrust(
                    "製品buildの更新trustがsize上限を超過した".to_string(),
                ));
            }
            let trust: UpdateTrust = serde_json::from_str(raw).map_err(|_| {
                BrokerStoreError::MalformedUpdateTrust(
                    "製品buildの更新trust構造が不正である".to_string(),
                )
            })?;
            validate_trust(&trust).map_err(BrokerStoreError::MalformedUpdateTrust)?;
            if trust.version != TRUST_VERSION || trust.package_sources.is_empty() {
                return Err(BrokerStoreError::MalformedUpdateTrust(
                    "製品buildの更新trustは版2公開鍵と配布元を必要とする".to_string(),
                ));
            }
            Ok(Some(trust))
        }
        _ => Err(BrokerStoreError::MalformedUpdateTrust(
            "製品build identityまたは更新trustのcompile-time設定が不完全である".to_string(),
        )),
    }
}

fn valid_product_identity(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        suffix.len() == 32
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn decode_state(state: &Value) -> Result<BTreeMap<String, Value>, String> {
    let object = state
        .as_object()
        .ok_or_else(|| "update stateはobjectでなければならない".to_string())?;
    if object.keys().any(|key| key != "版" && key != "updates") {
        return Err("update stateに未知fieldがある".to_string());
    }
    if object.get("版") != Some(&Value::from(VERSION)) {
        return Err("update stateの版が不正".to_string());
    }
    let values = object
        .get("updates")
        .and_then(Value::as_array)
        .ok_or_else(|| "update stateのupdatesがarrayではない".to_string())?;
    if values.len() > MAX_UPDATES {
        return Err("update stateの件数上限を超過".to_string());
    }
    let mut result = BTreeMap::new();
    for value in values {
        let record: UpdateRecord = serde_json::from_value(value.clone())
            .map_err(|_| "update stateのrecord構造が不正".to_string())?;
        validate_record(&record)?;
        if result
            .insert(record.candidate.update_id.clone(), value.clone())
            .is_some()
        {
            return Err("update stateに重複更新IDがある".to_string());
        }
    }
    Ok(result)
}

pub(super) fn dispatch(
    broker: &mut Broker,
    operation: BrokerOperation,
    payload: &Value,
    request_id: &str,
    payload_hash: &str,
    owner_confirmation: OwnerConfirmationSource,
    download_confirmation: Option<&UpdateDownloadConfirmation>,
    apply_confirmation: Option<&UpdateApplyConfirmation>,
    activation_confirmation: Option<&UpdateActivationConfirmation>,
    product_uninstall_confirmation: Option<&ProductUninstallConfirmation>,
    product_repair_confirmation: Option<&ProductRepairConfirmation>,
) -> BrokerResponse {
    poll_download_completion(broker);
    match operation.as_str() {
        OP_LIST => list(broker, payload, request_id, payload_hash),
        OP_FETCH_CATALOG => fetch_catalog(broker, payload, request_id, payload_hash),
        OP_CHECK => check(broker, payload, request_id, payload_hash),
        OP_VERIFY => verify(broker, payload, request_id, payload_hash),
        OP_DOWNLOAD => execution_request(
            broker,
            OP_DOWNLOAD,
            payload,
            request_id,
            payload_hash,
            owner_confirmation,
            download_confirmation,
            None,
            None,
        ),
        OP_APPLY => execution_request(
            broker,
            OP_APPLY,
            payload,
            request_id,
            payload_hash,
            owner_confirmation,
            None,
            apply_confirmation,
            None,
        ),
        OP_ACTIVATE => execution_request(
            broker,
            OP_ACTIVATE,
            payload,
            request_id,
            payload_hash,
            owner_confirmation,
            None,
            None,
            activation_confirmation,
        ),
        OP_DEFER => defer(broker, payload, request_id, payload_hash),
        OP_ROLLBACK => execution_request(
            broker,
            OP_ROLLBACK,
            payload,
            request_id,
            payload_hash,
            owner_confirmation,
            None,
            None,
            activation_confirmation,
        ),
        OP_PRODUCT_REPAIR => repair_product_launch_entries(
            broker,
            payload,
            request_id,
            payload_hash,
            owner_confirmation,
            product_repair_confirmation,
        ),
        OP_UNINSTALL => request_product_uninstall(
            broker,
            payload,
            request_id,
            payload_hash,
            owner_confirmation,
            product_uninstall_confirmation,
        ),
        _ => broker.reject_with_payload_hash(
            request_id,
            operation.as_str(),
            "update_operation_unknown",
            "更新操作が未定義",
            true,
            payload_hash,
        ),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductUninstallRequest {
    #[serde(rename = "版")]
    version: u64,
}

fn request_product_uninstall(
    broker: &mut Broker,
    payload: &Value,
    request_id: &str,
    payload_hash: &str,
    owner_confirmation: OwnerConfirmationSource,
    confirmation: Option<&ProductUninstallConfirmation>,
) -> BrokerResponse {
    let request: ProductUninstallRequest = match serde_json::from_value::<ProductUninstallRequest>(payload.clone()) {
        Ok(request) if request.version == VERSION => request,
        _ => {
            return reject(
                broker,
                request_id,
                OP_UNINSTALL,
                "product_uninstall_request_invalid",
                "製品アンインストール要求の構造が不正",
                payload_hash,
            )
        }
    };
    let _ = request;
    if owner_confirmation != OwnerConfirmationSource::DesktopNativeConfirmation
        || !broker.desktop_install_path_verified
    {
        return reject(
            broker,
            request_id,
            OP_UNINSTALL,
            "desktop_native_owner_confirmation_required",
            "導入済み製品の削除にはRust Desktop起動器のnative Owner確認が必要",
            payload_hash,
        );
    }
    let Some((app_id, audit_store_id)) = broker.desktop_product_identity.as_ref() else {
        return reject(
            broker,
            request_id,
            OP_UNINSTALL,
            "product_identity_unavailable",
            "導入済み製品identityを確認できない",
            payload_hash,
        );
    };
    let Some(confirmation) = confirmation else {
        return reject(
            broker,
            request_id,
            OP_UNINSTALL,
            "product_uninstall_confirmation_missing",
            "固定導入先と保持する利用者データを示したnative確認記録がない",
            payload_hash,
        );
    };

    #[cfg(windows)]
    let expected_paths = (|| {
        #[cfg(test)]
        let local_app_data = broker
            .desktop_product_install_local_app_data
            .clone()
            .or_else(|| super::product_install::current_user_local_app_data().ok())?;
        #[cfg(not(test))]
        let local_app_data = super::product_install::current_user_local_app_data().ok()?;
        #[cfg(test)]
        let start_menu = broker
            .desktop_product_start_menu_directory
            .clone()
            .or_else(|| super::product_install::current_user_start_menu_directory().ok())?;
        #[cfg(not(test))]
        let start_menu = super::product_install::current_user_start_menu_directory().ok()?;
        let root = super::product_install::product_install_root(&local_app_data, app_id).ok()?;
        let shortcut = super::product_install::plan_start_menu_shortcut(&start_menu, app_id).ok()?;
        Some((local_app_data, root, shortcut))
    })();
    #[cfg(not(windows))]
    let expected_paths: Option<(PathBuf, PathBuf, PathBuf)> = None;

    let Some((local_app_data, expected_root, expected_shortcut)) = expected_paths else {
        return reject(
            broker,
            request_id,
            OP_UNINSTALL,
            "product_uninstall_platform_unavailable",
            "Windows Known Folderから固定導入先を確認できない",
            payload_hash,
        );
    };
    let root_is_present = super::product_install::open_existing_product_root(&local_app_data, app_id)
        .is_ok();
    if !root_is_present
        || confirmation.app_id != *app_id
        || confirmation.audit_store_id != *audit_store_id
        || confirmation.product_root != expected_root
        || confirmation.start_menu_shortcut_path != expected_shortcut
        || confirmation.payload_hash != payload_hash
    {
        return reject(
            broker,
            request_id,
            OP_UNINSTALL,
            "product_uninstall_confirmation_stale",
            "native Owner確認後に固定導入先または製品identityが変化した",
            payload_hash,
        );
    }

    let mut ticket_bytes = [0u8; 32];
    if getrandom::getrandom(&mut ticket_bytes).is_err() {
        return reject(
            broker,
            request_id,
            OP_UNINSTALL,
            "product_uninstall_ticket_failed",
            "一回限りの削除ticketを生成できない",
            payload_hash,
        );
    }
    let ticket = hex::encode(ticket_bytes);
    ticket_bytes.fill(0);
    let ticket_hash = sha256_tagged(ticket.as_bytes());
    let reason = format!(
        "Capability=product.install.uninstall Permission=Known Folderから再導出した固定導入rootと同じ起動先のStart Menu shortcutだけ Approval=導入済み製品identity・固定root・保持データを示した独立Rust Desktop native Owner確認 AuditEvent=削除前意図を永続化 RecoveryAction=終了後helperがBroker ticketを再照合し、AppDataのruntime・Credential・Audit Storeを保持する。失敗後は新しいnative Owner確認から再要求する 製品ID={app_id} 監査保存先ID={audit_store_id} ticket_sha256={ticket_hash}"
    );
    let event = match broker.append_audit(
        request_id,
        OP_UNINSTALL,
        "accepted",
        &reason,
        "LIVE_RUNTIME",
        payload_hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_UNINSTALL,
                "product_uninstall_audit_failed",
                "削除前の監査を確定できないため、製品を削除しない",
            )
        }
    };
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OP_UNINSTALL.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: "LIVE_RUNTIME".to_string(),
        audit_event_id: event.event_id,
        error: None,
        health: None,
        body: Some(json!({"版": VERSION, "状態": "uninstall_authorized", "ticket": ticket})),
        shutdown_requested: broker.shutdown_requested,
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductRepairRequest {
    #[serde(rename = "版")]
    version: u64,
}

#[cfg(windows)]
fn repair_product_launch_entries(
    broker: &mut Broker,
    payload: &Value,
    request_id: &str,
    payload_hash: &str,
    owner_confirmation: OwnerConfirmationSource,
    confirmation: Option<&ProductRepairConfirmation>,
) -> BrokerResponse {
    let request: ProductRepairRequest =
        match serde_json::from_value::<ProductRepairRequest>(payload.clone()) {
            Ok(request) if request.version == VERSION => request,
            _ => {
                return reject(
                    broker,
                    request_id,
                    OP_PRODUCT_REPAIR,
                    "product_repair_request_invalid",
                    "製品起動項目修復要求の構造が不正",
                    payload_hash,
                )
            }
        };
    let _ = request;
    if owner_confirmation != OwnerConfirmationSource::DesktopNativeConfirmation {
        return reject(
            broker,
            request_id,
            OP_PRODUCT_REPAIR,
            "desktop_native_owner_confirmation_required",
            "製品起動項目の修復にはRust Desktop native Owner確認が必要",
            payload_hash,
        );
    }
    let Some((app_id, audit_store_id)) = broker.desktop_product_identity.clone() else {
        return reject(
            broker,
            request_id,
            OP_PRODUCT_REPAIR,
            "product_identity_unavailable",
            "製品identityを確認できない",
            payload_hash,
        );
    };
    let Some(confirmation) = confirmation else {
        return reject(
            broker,
            request_id,
            OP_PRODUCT_REPAIR,
            "product_repair_confirmation_missing",
            "現行版と固定起動先を示したnative確認記録がない",
            payload_hash,
        );
    };

    #[cfg(windows)]
    let paths = (|| {
        #[cfg(test)]
        let local_app_data = broker
            .desktop_product_install_local_app_data
            .clone()
            .or_else(|| super::product_install::current_user_local_app_data().ok())?;
        #[cfg(not(test))]
        let local_app_data = super::product_install::current_user_local_app_data().ok()?;
        #[cfg(test)]
        let start_menu = broker
            .desktop_product_start_menu_directory
            .clone()
            .or_else(|| super::product_install::current_user_start_menu_directory().ok())?;
        #[cfg(not(test))]
        let start_menu = super::product_install::current_user_start_menu_directory().ok()?;
        let root = super::product_install::product_install_root(&local_app_data, &app_id).ok()?;
        let shortcut =
            super::product_install::plan_start_menu_shortcut(&start_menu, &app_id).ok()?;
        Some((local_app_data, start_menu, root, shortcut))
    })();
    #[cfg(not(windows))]
    let paths: Option<(PathBuf, PathBuf, PathBuf, PathBuf)> = None;

    let Some((local_app_data, start_menu, expected_root, expected_shortcut)) = paths else {
        return reject(
            broker,
            request_id,
            OP_PRODUCT_REPAIR,
            "product_repair_platform_unavailable",
            "Windows Known Folderから固定導入先を確認できない",
            payload_hash,
        );
    };
    let snapshot =
        match crate::product_bootstrapper::active_version_snapshot(
            &local_app_data,
            &app_id,
            &audit_store_id,
        ) {
            Ok(snapshot) => snapshot,
            Err(error) => return reject(
                broker,
                request_id,
                OP_PRODUCT_REPAIR,
                error.0,
                "現行active recordまたはversion-local launcherを安全に検証できないため修復しない",
                payload_hash,
            ),
        };
    let current = &snapshot.current;
    if !rollback_descriptor_matches_candidate(broker, current)
        || confirmation.app_id != app_id
        || confirmation.audit_store_id != audit_store_id
        || confirmation.update_id != current.update_id
        || confirmation.candidate_hash != current.candidate_hash
        || confirmation.product_version != current.product_version
        || confirmation.package_sha256 != current.package_sha256
        || confirmation.product_root != expected_root
        || confirmation.start_menu_shortcut_path != expected_shortcut
        || confirmation.payload_hash != payload_hash
    {
        return reject(
            broker,
            request_id,
            OP_PRODUCT_REPAIR,
            "product_repair_confirmation_stale",
            "native Owner確認後にtrust、現行版、製品identityまたは固定pathが変化した",
            payload_hash,
        );
    }
    if !broker.state_store.persistence_ready() {
        return suspended(
            broker,
            OP_PRODUCT_REPAIR,
            request_id,
            json!({"版": VERSION, "状態": "suspended", "復旧ID": "recover-product-launch-repair-audit", "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE}),
            payload_hash,
            "永続Auditを確定できないため起動項目を修復しない",
        );
    }
    let existing_shortcut = match std::fs::symlink_metadata(&expected_shortcut) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => {
            return product_repair_failed(
                broker,
                request_id,
                payload_hash,
                "product_start_menu_shortcut_unavailable",
                "Start Menu shortcutの状態を安全に確認できないため修復しない",
            )
        }
    };
    let intent = match broker.append_audit(
        request_id,
        OP_PRODUCT_REPAIR,
        "queued",
        &format!("Capability=product.install.repair_launch_entries Permission=現在trustで再検証したactive versionのmissing root Bootstrapperと固定Start Menu shortcutだけを復元し、既存file・version payload・active recordを上書きしない Approval=現行版と固定pathを表示した独立Rust Desktop native Owner確認 AuditEvent=修復前意図を永続化 RecoveryAction=不一致または失敗時は既存状態を保持し、原因確認後に新しいOwner確認から再要求 version={} package_sha256={}", current.product_version, current.package_sha256),
        "LIVE_RUNTIME",
        payload_hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_PRODUCT_REPAIR,
                "product_repair_audit_failed",
                "修復前Auditを確定できないため変更しない",
            )
        }
    };

    let bootstrapper_restored = match crate::product_bootstrapper::restore_missing_root_bootstrapper(
        &local_app_data,
        &app_id,
        &audit_store_id,
        current,
    ) {
        Ok(restored) => restored,
        Err(error) => {
            return product_repair_failed(
                broker,
                request_id,
                payload_hash,
                error.0,
                "現行stageから固定root起動器を安全に復元できないため、状態を上書きせず停止した",
            )
        }
    };
    let root_path =
        match super::product_install::open_existing_product_root(&local_app_data, &app_id) {
            Ok((path, _)) if path == expected_root => path,
            _ => {
                return product_repair_failed(
                    broker,
                    request_id,
                    payload_hash,
                    "product_install_root_changed",
                    "修復中に固定製品rootが変化したため停止した",
                )
            }
        };
    let _registered_shortcut = match super::product_install::register_start_menu_shortcut(
        &local_app_data,
        &start_menu,
        &app_id,
        &root_path,
    ) {
        Ok(path) if path == expected_shortcut => path,
        Ok(_) => {
            return product_repair_failed(
                broker,
                request_id,
                payload_hash,
                "product_start_menu_published_path_mismatch",
                "Start Menu shortcutの公開先が確認済み固定pathと一致しない",
            )
        }
        Err(error) => {
            return product_repair_failed(
                broker,
                request_id,
                payload_hash,
                error.0,
                "固定Start Menu shortcutを安全に復元できないため停止した",
            )
        }
    };
    let repaired = bootstrapper_restored || !existing_shortcut;
    let completion = match broker.append_audit(
        &format!("{request_id}:complete"),
        OP_PRODUCT_REPAIR,
        if repaired { "completed" } else { "unchanged" },
        &format!("Capability=product.install.repair_launch_entries Permission=現行signed active versionと固定pathに一致するlaunch entryだけ Approval=独立Rust Desktop native Owner確認 AuditEvent=起動器復元={} shortcut復元={} intent_audit_id={} RecoveryAction=修復済みentryは維持し、版本体・設定・Credential・Workspace・Audit Storeは変更しない", bootstrapper_restored, !existing_shortcut, intent.event_id),
        "LIVE_RUNTIME",
        payload_hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_PRODUCT_REPAIR,
                "product_repair_audit_failed",
                "修復後の完了Auditを確定できない。起動状態を再確認してください",
            )
        }
    };
    BrokerResponse {
        request_id: request_id.to_owned(),
        operation: OP_PRODUCT_REPAIR.to_owned(),
        status: BrokerStatus::Accepted,
        evidence_source: "LIVE_RUNTIME".to_owned(),
        audit_event_id: completion.event_id,
        error: None,
        health: None,
        body: Some(json!({
            "版": VERSION,
            "状態": if repaired { "product_launch_entries_repaired" } else { "unchanged" },
            "起動器復元": bootstrapper_restored,
            "Start Menu復元": !existing_shortcut,
            "開始Audit ID": intent.event_id,
            "証拠種別": "LIVE_RUNTIME",
        })),
        shutdown_requested: broker.shutdown_requested,
    }
}

#[cfg(windows)]
fn product_repair_failed(
    broker: &mut Broker,
    request_id: &str,
    payload_hash: &str,
    failure_code: &str,
    message: &str,
) -> BrokerResponse {
    if broker
        .append_audit(
            &format!("{request_id}:failed"),
            OP_PRODUCT_REPAIR,
            "failed",
            &format!("Capability=product.install.repair_launch_entries Permission=現行signed active versionの欠損launch entryだけ Approval=独立Rust Desktop native Owner確認 AuditEvent=起動項目修復失敗を記録 RecoveryAction=既存file・version payload・active recordを上書きせず、原因確認後に新しいOwner確認から再要求 failure_code={failure_code}"),
            "LIVE_RUNTIME",
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_PRODUCT_REPAIR,
            "product_repair_audit_failed",
            "起動項目修復失敗のAuditを確定できない",
        );
    }
    reject(
        broker,
        request_id,
        OP_PRODUCT_REPAIR,
        failure_code,
        message,
        payload_hash,
    )
}

#[cfg(not(windows))]
fn repair_product_launch_entries(
    broker: &mut Broker,
    payload: &Value,
    request_id: &str,
    payload_hash: &str,
    _owner_confirmation: OwnerConfirmationSource,
    _confirmation: Option<&ProductRepairConfirmation>,
) -> BrokerResponse {
    if !matches!(
        serde_json::from_value::<ProductRepairRequest>(payload.clone()),
        Ok(request) if request.version == VERSION
    ) {
        return reject(
            broker,
            request_id,
            OP_PRODUCT_REPAIR,
            "product_repair_request_invalid",
            "製品起動項目修復要求の構造が不正",
            payload_hash,
        );
    }
    reject(
        broker,
        request_id,
        OP_PRODUCT_REPAIR,
        "product_repair_platform_unavailable",
        "固定導入先の起動項目修復はWindowsでのみ利用できる",
        payload_hash,
    )
}

impl Broker {
    /// Durable Auditに結び付いた一回限りのuninstall ticketを検証し、開始記録を確定する。
    pub(crate) fn begin_product_uninstall_finalization(
        &mut self,
        ticket: &str,
    ) -> Result<String, &'static str> {
        if ticket.len() != 64
            || !ticket
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("product_uninstall_ticket_invalid");
        }
        let ticket_hash = sha256_tagged(ticket.as_bytes());
        let Some((app_id, audit_store_id)) = self.desktop_product_identity.as_ref() else {
            return Err("product_identity_unavailable");
        };
        let identity_marker = format!("製品ID={app_id} 監査保存先ID={audit_store_id}");
        let ticket_marker = format!("ticket_sha256={ticket_hash}");
        let authorized = self.audit_events().iter().any(|event| {
            event.operation == OP_UNINSTALL
                && event.decision == "accepted"
                && event.reason.contains(&identity_marker)
                && event.reason.contains(&ticket_marker)
        });
        if !authorized {
            return Err("product_uninstall_ticket_unrecognized");
        }
        if self.audit_events().iter().any(|event| {
            matches!(
                event.operation.as_str(),
                OP_UNINSTALL_STARTED | OP_UNINSTALL_FAILED | OP_UNINSTALL_COMPLETED
            ) && event.reason.contains(&ticket_marker)
        }) {
            return Err("product_uninstall_ticket_replayed");
        }
        let request_id = format!("uninstall-finalizer:{}:started", &ticket_hash[7..23]);
        self.append_audit(
            &request_id,
            OP_UNINSTALL_STARTED,
            "started",
            &format!(
                "Capability=product.install.uninstall Permission=Brokerが発行した未使用ticketと固定製品identity Approval=先行するnative Owner確認 AuditEvent=終了後削除処理の開始を永続記録 RecoveryAction=このticketの再利用は禁止し、再試行は新しいnative Owner確認を要求する ticket_sha256={ticket_hash} {identity_marker}"
            ),
            "LIVE_RUNTIME",
            &ticket_hash,
        )
        .map_err(|_| "product_uninstall_audit_failed")?;
        Ok(ticket_hash)
    }

    /// 成功・失敗ともticketを消費し、再試行には新しいOwner確認を要求する。
    pub(crate) fn finish_product_uninstall_finalization(
        &mut self,
        ticket_hash: &str,
        success: bool,
        failure_code: Option<&str>,
    ) -> Result<(), &'static str> {
        if ticket_hash.len() != 71 || !ticket_hash.starts_with("sha256:") {
            return Err("product_uninstall_ticket_hash_invalid");
        }
        let Some((app_id, audit_store_id)) = self.desktop_product_identity.as_ref() else {
            return Err("product_identity_unavailable");
        };
        let ticket_marker = format!("ticket_sha256={ticket_hash}");
        let identity_marker = format!("製品ID={app_id} 監査保存先ID={audit_store_id}");
        let has_authorization = self.audit_events().iter().any(|event| {
            event.operation == OP_UNINSTALL
                && event.decision == "accepted"
                && event.reason.contains(&identity_marker)
                && event.reason.contains(&ticket_marker)
        });
        let has_started = self.audit_events().iter().any(|event| {
            event.operation == OP_UNINSTALL_STARTED && event.reason.contains(&ticket_marker)
        });
        let has_terminal = self.audit_events().iter().any(|event| {
            matches!(
                event.operation.as_str(),
                OP_UNINSTALL_FAILED | OP_UNINSTALL_COMPLETED
            )
                && event.reason.contains(&ticket_marker)
        });
        if !has_authorization || !has_started || has_terminal {
            return Err("product_uninstall_ticket_not_pending");
        }
        let (operation, decision, recovery) = if success {
            (
                OP_UNINSTALL_COMPLETED,
                "completed",
                "固定導入rootと同一targetのshortcutだけを除去し、利用者dataとAudit Storeを保持",
            )
        } else {
            (
                OP_UNINSTALL_FAILED,
                "failed",
                "残存fileを保持し、新しいnative Owner確認を経た再要求を許可",
            )
        };
        let detail = failure_code.unwrap_or("none");
        let request_id = format!("uninstall-finalizer:{}:{decision}", &ticket_hash[7..23]);
        self.append_audit(
            &request_id,
            operation,
            decision,
            &format!(
                "Capability=product.install.uninstall Permission=固定導入rootと同一targetのStart Menu entryだけ Approval=先行する独立native Owner確認 AuditEvent=終了後処理 {decision} RecoveryAction={recovery} failure_code={detail} {identity_marker} {ticket_marker}"
            ),
            "LIVE_RUNTIME",
            ticket_hash,
        )
        .map_err(|_| "product_uninstall_audit_failed")?;
        Ok(())
    }
}

fn fetch_catalog(
    broker: &mut Broker,
    payload: &Value,
    request_id: &str,
    hash: &str,
) -> BrokerResponse {
    fetch_catalog_with(broker, payload, request_id, hash, |url, deadline| {
        super::update_download::fetch_update_catalog(url, MAX_MANIFEST_BYTES, deadline)
    })
}

fn fetch_catalog_with<F>(
    broker: &mut Broker,
    payload: &Value,
    request_id: &str,
    hash: &str,
    mut fetch: F,
) -> BrokerResponse
where
    F: FnMut(&str, std::time::Instant) -> Result<Vec<u8>, super::update_download::DownloadError>,
{
    if payload != &json!({"版": VERSION}) {
        return reject_catalog(
            broker,
            request_id,
            "update_catalog_request_invalid",
            "候補取得要求は版だけを受け付ける",
            hash,
        );
    }
    let Some(trust) = broker.update_trust.clone() else {
        return reject_catalog(
            broker,
            request_id,
            "update_trust_unconfigured",
            "Broker所有の更新署名trustが未構成",
            hash,
        );
    };
    if let Err(reason) = validate_trust(&trust) {
        return reject_catalog(broker, request_id, "update_trust_invalid", &reason, hash);
    }
    if trust.package_sources.is_empty() {
        return accepted(
            broker,
            OP_FETCH_CATALOG,
            request_id,
            json!({"版": VERSION, "状態": "unconfigured", "配布元件数": 0, "候補件数": 0, "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE}),
            hash,
            &catalog_audit_reason("Broker trustに候補配布元がないため外部通信を実行しない"),
        );
    }

    let deadline = std::time::Instant::now() + super::update_download::CATALOG_TIMEOUT;
    let mut records = Vec::new();
    let mut seen_ids = BTreeSet::new();
    let mut catalog_hashes = Vec::new();
    let mut total_bytes = 0usize;
    for source in &trust.package_sources {
        let url = format!("{}/updates.json", source.base_url);
        let bytes = match fetch(&url, deadline) {
            Ok(bytes) if !bytes.is_empty() && bytes.len() <= MAX_MANIFEST_BYTES => bytes,
            Ok(_) => {
                return reject_catalog(
                    broker,
                    request_id,
                    "update_catalog_size_invalid",
                    "候補一覧のbyte長が上限外",
                    hash,
                )
            }
            Err(error) => {
                let (code, reason) = catalog_download_error(&error);
                return reject_catalog(broker, request_id, code, reason, hash);
            }
        };
        total_bytes = match total_bytes.checked_add(bytes.len()) {
            Some(total) if total <= MAX_MANIFEST_BYTES * 3 => total,
            _ => {
                return reject_catalog(
                    broker,
                    request_id,
                    "update_catalog_total_size_invalid",
                    "候補一覧の合計byte長が上限外",
                    hash,
                )
            }
        };
        let catalog_hash = sha256_tagged(&bytes);
        catalog_hashes.push(catalog_hash);
        let raw = match std::str::from_utf8(&bytes) {
            Ok(raw) => raw,
            Err(_) => {
                return reject_catalog(
                    broker,
                    request_id,
                    "update_catalog_encoding_invalid",
                    "候補一覧がUTF-8ではない",
                    hash,
                )
            }
        };
        let document: UpdateCatalogDocument =
            match super::json_input::read_unique::<UpdateCatalogDocument>(raw) {
                Ok(document)
                    if document.version == VERSION
                        && document.candidates.len() <= MAX_CATALOG_CANDIDATES =>
                {
                    document
                }
                _ => {
                    return reject_catalog(
                        broker,
                        request_id,
                        "update_catalog_invalid",
                        "候補一覧の構造または版が不正",
                        hash,
                    )
                }
            };
        for candidate in document.candidates {
            if candidate.channel != source.channel || !seen_ids.insert(candidate.update_id.clone())
            {
                return reject_catalog(
                    broker,
                    request_id,
                    "update_catalog_candidate_conflict",
                    "候補channelまたは更新IDが配布元と一致しない",
                    hash,
                );
            }
            match verify_candidate(broker, candidate) {
                Ok(record) => records.push(record),
                Err((code, reason)) => {
                    return reject_catalog(broker, request_id, code, reason, hash)
                }
            }
        }
    }
    if records.len() > MAX_UPDATES {
        return reject_catalog(
            broker,
            request_id,
            "update_catalog_limit_exceeded",
            "候補一覧の総件数上限を超過",
            hash,
        );
    }
    let previous = broker.updates.clone();
    for record in &records {
        broker.updates.insert(
            record.candidate.update_id.clone(),
            serde_json::to_value(record).unwrap_or(Value::Null),
        );
    }
    if broker.updates.len() > MAX_UPDATES {
        broker.updates = previous;
        return reject_catalog(
            broker,
            request_id,
            "update_catalog_limit_exceeded",
            "保存済み候補を含む件数上限を超過",
            hash,
        );
    }
    if persist(broker).is_err() {
        broker.updates = previous;
        return broker.audit_store_failed_response(
            request_id,
            OP_FETCH_CATALOG,
            "update_catalog_persistence_failed",
            &catalog_audit_reason(
                "候補永続化に失敗。既存候補は保持した。Audit／永続storeを確認してから明示再試行",
            ),
        );
    }
    let catalog_hashes = catalog_hashes.join(",");
    accepted_external(
        broker,
        OP_FETCH_CATALOG,
        request_id,
        json!({"版": VERSION, "状態": "updated", "配布元件数": trust.package_sources.len(), "候補件数": records.len(), "証拠種別": "EXTERNAL_EVIDENCE"}),
        hash,
        &catalog_audit_reason(&format!(
            "Broker固定HTTPS配布元から署名検証済み候補を取得し永続化 catalog_hashes={} candidates={}",
            catalog_hashes, records.len()
        )),
    )
}

fn catalog_audit_reason(detail: &str) -> String {
    format!(
        "Capability=更新候補取得 Permission=Broker所有trust.package_sourcesに固定されたchannel別HTTPS配布元 Approval=明示操作によるread-only公開metadata取得はnot_required。package download／Installは別native Owner確認 AuditEvent=配布元件数・catalog hash・候補件数だけ RecoveryAction=失敗時は既存候補を保持し、trust／配布元を修正後に明示再試行 detail={detail}"
    )
}

fn reject_catalog(
    broker: &mut Broker,
    request_id: &str,
    code: &str,
    detail: &str,
    hash: &str,
) -> BrokerResponse {
    reject(
        broker,
        request_id,
        OP_FETCH_CATALOG,
        code,
        &catalog_audit_reason(detail),
        hash,
    )
}

fn catalog_download_error(
    error: &super::update_download::DownloadError,
) -> (&'static str, &'static str) {
    use super::update_download::DownloadError;
    match error {
        DownloadError::InvalidRequest => ("update_catalog_source_invalid", "固定候補URLが不正"),
        DownloadError::NameResolution => {
            ("update_catalog_dns_failed", "候補配布元の名前解決に失敗")
        }
        DownloadError::NonPublicAddress => (
            "update_catalog_address_blocked",
            "許可されない配布元addressを拒否",
        ),
        DownloadError::Network => ("update_catalog_network_failed", "候補配布元HTTPS通信に失敗"),
        DownloadError::RedirectOrUnexpectedStatus => (
            "update_catalog_response_rejected",
            "候補配布元のstatusまたはredirectを拒否",
        ),
        DownloadError::InvalidHeaders => (
            "update_catalog_headers_invalid",
            "候補一覧のHTTP headerが不正",
        ),
        DownloadError::SizeMismatch | DownloadError::DocumentTooLarge => {
            ("update_catalog_size_invalid", "候補一覧のbyte長が不正")
        }
        DownloadError::TimedOut => ("update_catalog_timeout", "候補取得期限を超過"),
        DownloadError::Cancelled => ("update_catalog_cancelled", "候補取得を中断"),
        DownloadError::ResolverBusy | DownloadError::DnsCancellationFailed => (
            "update_catalog_dns_failed",
            "候補配布元の名前解決を完了できない",
        ),
        DownloadError::DigestMismatch | DownloadError::Storage => {
            ("update_catalog_failed", "候補取得処理を完了できない")
        }
    }
}

fn list(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    if payload != &json!({"版": VERSION}) {
        return reject(
            broker,
            request_id,
            OP_LIST,
            "update_list_invalid",
            "更新一覧は版だけを受け付ける",
            hash,
        );
    }
    let trust = broker.update_trust.as_ref();
    let updates: Vec<Value> = broker
        .updates
        .values()
        .map(|value| project_update_record(value, trust))
        .collect();
    let rollback = rollback_state_projection(broker);
    let rollback_available = rollback["状態"] == "available";
    accepted(
        broker,
        OP_LIST,
        request_id,
        json!({
            "版": VERSION,
            "更新一覧": updates,
            "件数": broker.updates.len(),
            "署名信頼設定": if broker.update_trust.is_some() { "configured" } else { "unconfigured" },
            "download実行": if broker.state_store.persistence_ready() { "available" } else { "suspended" },
            "download_job": broker.update_download.projection(),
            "適用実行": "suspended",
            "rollback実行": if rollback_available { "available" } else { "suspended" },
            "rollback状態": rollback,
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        }),
        hash,
        "Brokerの現在状態から更新候補とdownload／適用／rollback可否を返却",
    )
}

fn rollback_state_projection(broker: &Broker) -> Value {
    #[cfg(not(windows))]
    {
        let _ = broker;
        return json!({"状態": "unknown", "現在版": null, "対象版": null});
    }
    #[cfg(windows)]
    {
        let Some((app_id, audit_store_id)) = broker.desktop_product_identity.as_ref() else {
            return json!({"状態": "unknown", "現在版": null, "対象版": null});
        };
        #[cfg(test)]
        let local_app_data = broker
            .desktop_product_install_local_app_data
            .clone()
            .or_else(|| super::product_install::current_user_local_app_data().ok());
        #[cfg(not(test))]
        let local_app_data = super::product_install::current_user_local_app_data().ok();
        let Some(local_app_data) = local_app_data else {
            return json!({"状態": "unknown", "現在版": null, "対象版": null});
        };
        let snapshot = match crate::product_bootstrapper::active_version_snapshot(
            &local_app_data,
            app_id,
            audit_store_id,
        ) {
            Ok(snapshot) => snapshot,
            Err(error) if error.0 == "active_version_file_unavailable" => {
                return json!({"状態": "unavailable", "現在版": null, "対象版": null});
            }
            Err(_) => return json!({"状態": "unknown", "現在版": null, "対象版": null}),
        };
        if !rollback_descriptor_matches_candidate(broker, &snapshot.current) {
            return json!({
                "状態": "unknown",
                "現在版": null,
                "対象版": null,
            });
        }
        let Some(previous) = snapshot.previous else {
            return json!({
                "状態": "unavailable",
                "現在版": rollback_descriptor_projection(&snapshot.current),
                "対象版": null,
            });
        };
        if !rollback_descriptor_matches_candidate(broker, &previous) {
            return json!({
                "状態": "unknown",
                "現在版": null,
                "対象版": null,
            });
        }
        json!({
            "状態": "available",
            "現在版": rollback_descriptor_projection(&snapshot.current),
            "対象版": rollback_descriptor_projection(&previous),
        })
    }
}

#[cfg(windows)]
fn rollback_descriptor_matches_candidate(
    broker: &Broker,
    descriptor: &crate::product_bootstrapper::ActiveVersionDescriptor,
) -> bool {
    let Some(value) = broker.updates.get(&descriptor.update_id) else {
        return false;
    };
    let Ok(record) = serde_json::from_value::<UpdateRecord>(value.clone()) else {
        return false;
    };
    let Ok(verified) = verify_candidate(broker, record.candidate.clone()) else {
        return false;
    };
    verified.candidate_hash == descriptor.candidate_hash
        && verified.signature_status == "verified"
        && verified.candidate.offered_version == descriptor.product_version
        && verified.candidate.package_sha256.as_deref() == Some(descriptor.package_sha256.as_str())
}

fn rollback_descriptor_projection(
    descriptor: &crate::product_bootstrapper::ActiveVersionDescriptor,
) -> Value {
    json!({
        "更新ID": descriptor.update_id,
        "候補hash": descriptor.candidate_hash,
        "提供版": descriptor.product_version,
        "package_sha256": descriptor.package_sha256,
    })
}

fn project_update_record(value: &Value, trust: Option<&UpdateTrust>) -> Value {
    let mut projected = value.clone();
    let (status, record) = match serde_json::from_value::<UpdateRecord>(value.clone()) {
        Ok(record) if record.candidate.version == 1 => ("legacy_unbound", Some(record)),
        Ok(record)
            if record.signature_status == "verified"
                && current_candidate_verification(trust, &record) =>
        {
            ("verified", Some(record))
        }
        Ok(record) => ("verification_stale", Some(record)),
        Err(_) => ("verification_stale", None),
    };
    if let Some(object) = projected.as_object_mut() {
        object.insert("署名状態".into(), Value::String(status.into()));
        object.insert(
            "取得元".into(),
            project_package_source(trust, record.as_ref(), status == "verified"),
        );
    }
    projected
}

fn project_package_source(
    trust: Option<&UpdateTrust>,
    record: Option<&UpdateRecord>,
    candidate_is_currently_verified: bool,
) -> Value {
    let Some(record) = record.filter(|record| {
        candidate_is_currently_verified
            && record.candidate.version == CANDIDATE_VERSION
            && record.candidate.package_sha256.is_some()
            && record.candidate.package_size_bytes.is_some()
    }) else {
        return json!({"状態": "ineligible", "URL": null});
    };
    let Some(trust) = trust.filter(|trust| validate_trust(trust).is_ok()) else {
        return json!({"状態": "ineligible", "URL": null});
    };
    let Some(source) = trust
        .package_sources
        .iter()
        .find(|source| source.channel == record.candidate.channel)
    else {
        return json!({"状態": "unconfigured", "URL": null});
    };
    json!({
        "状態": "configured",
        "URL": format!("{}/{}.pkg", source.base_url, record.candidate.update_id),
    })
}

fn current_candidate_verification(trust: Option<&UpdateTrust>, record: &UpdateRecord) -> bool {
    verify_candidate_with_trust(trust, record.candidate.clone())
        .is_ok_and(|current| current.candidate_hash == record.candidate_hash)
}

fn verify(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    let request: CandidateRequest =
        match serde_json::from_value::<CandidateRequest>(payload.clone()) {
            Ok(request) if request.version == VERSION => request,
            _ => {
                return reject(
                    broker,
                    request_id,
                    OP_VERIFY,
                    "update_candidate_invalid",
                    "更新候補の構造が不正",
                    hash,
                )
            }
        };
    let record = match verify_candidate(broker, request.candidate) {
        Ok(record) => record,
        Err((code, message)) => return reject(broker, request_id, OP_VERIFY, code, message, hash),
    };
    accepted(
        broker,
        OP_VERIFY,
        request_id,
        serde_json::to_value(record).unwrap_or(Value::Null),
        hash,
        "更新候補のEd25519署名を検査",
    )
}

fn check(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    let request: CheckRequest = match serde_json::from_value::<CheckRequest>(payload.clone()) {
        Ok(request) if request.version == VERSION && !request.candidates.is_empty() => request,
        _ => {
            return reject(
                broker,
                request_id,
                OP_CHECK,
                "update_check_invalid",
                "更新確認の構造が不正",
                hash,
            )
        }
    };
    if request.candidates.len() > MAX_UPDATES {
        return reject(
            broker,
            request_id,
            OP_CHECK,
            "update_limit_exceeded",
            "更新候補件数上限を超過",
            hash,
        );
    }
    let mut records = Vec::with_capacity(request.candidates.len());
    for candidate in request.candidates {
        match verify_candidate(broker, candidate) {
            Ok(record) => records.push(record),
            Err((code, message)) => {
                return reject(broker, request_id, OP_CHECK, code, message, hash)
            }
        }
    }
    let previous = broker.updates.clone();
    for record in &records {
        let value = match serde_json::to_value(record) {
            Ok(value) => value,
            Err(_) => {
                return reject(
                    broker,
                    request_id,
                    OP_CHECK,
                    "update_serialize_failed",
                    "更新候補を保存形式へ変換できない",
                    hash,
                )
            }
        };
        broker
            .updates
            .insert(record.candidate.update_id.clone(), value);
    }
    if broker.updates.len() > MAX_UPDATES {
        broker.updates = previous;
        return reject(
            broker,
            request_id,
            OP_CHECK,
            "update_limit_exceeded",
            "更新候補件数上限を超過",
            hash,
        );
    }
    if let Err(reason) = persist(broker) {
        broker.updates = previous;
        return broker.audit_store_failed_response(
            request_id,
            OP_CHECK,
            "update_persistence_failed",
            &reason,
        );
    }
    accepted(
        broker,
        OP_CHECK,
        request_id,
        json!({"版": VERSION, "更新一覧": records, "件数": records.len(), "署名状態": "verified", "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE}),
        hash,
        "更新候補を検査し永続化。未署名・未信頼候補は保存しない",
    )
}

fn defer(broker: &mut Broker, payload: &Value, request_id: &str, hash: &str) -> BrokerResponse {
    let request: DeferRequest = match serde_json::from_value::<DeferRequest>(payload.clone()) {
        Ok(request) if request.version == VERSION && !request.deferred_until.trim().is_empty() => {
            request
        }
        _ => {
            return reject(
                broker,
                request_id,
                OP_DEFER,
                "update_defer_invalid",
                "更新延期の構造が不正",
                hash,
            )
        }
    };
    let Some(current) = broker.updates.get(&request.update_id).cloned() else {
        return reject(
            broker,
            request_id,
            OP_DEFER,
            "update_not_found",
            "延期対象の更新がない",
            hash,
        );
    };
    let mut record: UpdateRecord = match serde_json::from_value(current.clone()) {
        Ok(record) => record,
        Err(_) => {
            return reject(
                broker,
                request_id,
                OP_DEFER,
                "update_state_invalid",
                "更新状態が不正",
                hash,
            )
        }
    };
    if record.candidate_hash != request.candidate_hash {
        return reject(
            broker,
            request_id,
            OP_DEFER,
            "update_stale",
            "更新候補hashが一致しない",
            hash,
        );
    }
    record.deferred_until = Some(request.deferred_until);
    let previous = broker.updates.insert(
        request.update_id.clone(),
        serde_json::to_value(&record).unwrap_or(Value::Null),
    );
    if let Err(reason) = persist(broker) {
        if let Some(previous) = previous {
            broker.updates.insert(request.update_id, previous);
        }
        return broker.audit_store_failed_response(
            request_id,
            OP_DEFER,
            "update_persistence_failed",
            &reason,
        );
    }
    let projected = project_update_record(
        &serde_json::to_value(record).unwrap_or(Value::Null),
        broker.update_trust.as_ref(),
    );
    accepted(
        broker,
        OP_DEFER,
        request_id,
        projected,
        hash,
        "更新延期を永続化。インストールは実行しない",
    )
}

fn execution_request(
    broker: &mut Broker,
    operation: &str,
    payload: &Value,
    request_id: &str,
    hash: &str,
    owner_confirmation: OwnerConfirmationSource,
    download_confirmation: Option<&UpdateDownloadConfirmation>,
    apply_confirmation: Option<&UpdateApplyConfirmation>,
    activation_confirmation: Option<&UpdateActivationConfirmation>,
) -> BrokerResponse {
    let request: UpdateIDRequest = match serde_json::from_value::<UpdateIDRequest>(payload.clone())
    {
        Ok(request) if request.version == VERSION && valid_hash(&request.candidate_hash) => request,
        _ => {
            return reject(
                broker,
                request_id,
                operation,
                "update_request_invalid",
                "更新実行要求の構造が不正",
                hash,
            )
        }
    };
    let Some(value) = broker.updates.get(&request.update_id) else {
        return reject(
            broker,
            request_id,
            operation,
            "update_not_found",
            "対象の更新がない",
            hash,
        );
    };
    let record: UpdateRecord = match serde_json::from_value(value.clone()) {
        Ok(record) => record,
        Err(_) => {
            return reject(
                broker,
                request_id,
                operation,
                "update_state_invalid",
                "更新状態が不正",
                hash,
            )
        }
    };
    if record.candidate_hash != request.candidate_hash {
        return reject(
            broker,
            request_id,
            operation,
            "update_stale",
            "更新候補hashが一致しない",
            hash,
        );
    }
    if record.signature_status != "verified" {
        return reject(
            broker,
            request_id,
            operation,
            "update_signature_required",
            "署名検査済みの更新だけを要求できる",
            hash,
        );
    }
    if record.candidate.version != CANDIDATE_VERSION {
        return reject(
            broker,
            request_id,
            operation,
            "update_package_binding_required",
            "配布packageのhashとbyte長へ署名が結合していない旧候補は実行できない",
            hash,
        );
    }
    let current_record = match verify_candidate(broker, record.candidate.clone()) {
        Ok(current) => current,
        Err((code, message)) => return reject(broker, request_id, operation, code, message, hash),
    };
    if current_record.candidate_hash != record.candidate_hash {
        return reject(
            broker,
            request_id,
            operation,
            "update_state_tampered",
            "保存候補の署名・内容・候補hashが現在状態と一致しない",
            hash,
        );
    }
    if operation == OP_DOWNLOAD {
        if owner_confirmation != OwnerConfirmationSource::DesktopNativeConfirmation {
            return reject(
                broker,
                request_id,
                operation,
                "desktop_native_owner_confirmation_required",
                "更新packageの外部取得はRust Desktop起動器のnative Owner確認が必要",
                hash,
            );
        }
        let Some(confirmation) = download_confirmation else {
            return reject(
                broker,
                request_id,
                operation,
                "update_download_confirmation_missing",
                "更新取得先をBroker状態から表示したnative確認記録がない",
                hash,
            );
        };
        let current_source =
            project_package_source(broker.update_trust.as_ref(), Some(&current_record), true);
        let current_host = current_source["URL"]
            .as_str()
            .and_then(|url| reqwest::Url::parse(url).ok())
            .and_then(|url| url.host_str().map(str::to_owned));
        let package_sha256 = current_record.candidate.package_sha256.as_deref();
        let package_size_bytes = current_record.candidate.package_size_bytes;
        if confirmation.update_id != current_record.candidate.update_id
            || confirmation.candidate_hash != current_record.candidate_hash
            || confirmation.source_url != current_source["URL"].as_str().unwrap_or_default()
            || Some(confirmation.display_host.as_str()) != current_host.as_deref()
            || confirmation.package_sha256 != package_sha256.unwrap_or_default()
            || Some(confirmation.package_size_bytes) != package_size_bytes
            || confirmation.offered_version != current_record.candidate.offered_version
            || confirmation.channel != current_record.candidate.channel
            || confirmation.summary != current_record.candidate.summary
            || confirmation.payload_hash != hash
        {
            return reject(
                broker,
                request_id,
                operation,
                "update_download_confirmation_stale",
                "native Owner確認後にBroker trust、候補、配布先のいずれかが変化した",
                hash,
            );
        }
        if !broker.state_store.persistence_ready() {
            return suspended(
                broker,
                operation,
                request_id,
                json!({"版": VERSION, "更新ID": request.update_id, "候補hash": request.candidate_hash, "状態": "suspended", "復旧ID": "recover-update-download-store", "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE}),
                hash,
                "永続Broker storeがないため更新packageを安全に保持できずsuspended",
            );
        }
        if broker.update_download.is_running() {
            return reject(
                broker,
                request_id,
                operation,
                "update_download_busy",
                "別の更新downloadが実行中のため一時fileを変更しない",
                hash,
            );
        }
        let directory = match broker.state_store.open_update_package_directory() {
            Ok(Some(directory)) => directory,
            Ok(None) => {
                return reject(
                    broker,
                    request_id,
                    operation,
                    "update_download_store_unavailable",
                    "Broker所有の永続package directoryがない",
                    hash,
                )
            }
            Err(_) => {
                return reject(
                    broker,
                    request_id,
                    operation,
                    "update_download_store_invalid",
                    "Broker所有のpackage directoryを安全に開けない",
                    hash,
                )
            }
        };
        let cleanup_request_id = format!("{}:recovery", request_id);
        if broker
            .append_audit(
                &cleanup_request_id,
                "更新download中断file復旧",
                "received",
                "Capability=更新package一時file復旧 Permission=固定Broker update_packages内の生成済み *.partだけ Approval=現在のdownload要求に対するnative Owner確認 AuditEvent=除去前意図を確定 RecoveryAction=確認済みdownload package final fileを保持",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                hash,
            )
            .is_err()
        {
            return broker.audit_store_failed_response(
                request_id,
                operation,
                "update_download_audit_failed",
                "中断download復旧のAuditを確定できない",
            );
        }
        let removed_partials = match super::update_download::remove_abandoned_partials(&directory) {
            Ok(removed) => removed,
            Err(_) => {
                if broker
                    .append_audit(
                        &cleanup_request_id,
                        "更新download中断file復旧",
                        "recovery_failed",
                        "Capability=更新package一時file復旧 Permission=固定Broker update_packages内の生成済み *.partを検査 Approval=現在のdownload要求に対するnative Owner確認 AuditEvent=復旧失敗を確定 RecoveryAction=一時fileを保持してdownloadを開始せず、Broker store状態を確認",
                        EVIDENCE_SOURCE_INTERNAL_STATE,
                        hash,
                    )
                    .is_err()
                {
                    return broker.audit_store_failed_response(
                        request_id,
                        operation,
                        "update_download_audit_failed",
                        "中断download復旧失敗のAuditを確定できない",
                    );
                }
                return reject(
                    broker,
                    request_id,
                    operation,
                    "update_download_recovery_failed",
                    "Broker所有download一時fileの復旧に失敗",
                    hash,
                );
            }
        };
        if broker
            .append_audit(
                &cleanup_request_id,
                "更新download中断file復旧",
                "recovered",
                &format!("Capability=更新package一時file復旧 Permission=固定Broker update_packages内の生成済み *.partだけを{}件除去 Approval=現在のdownload要求に対するnative Owner確認 AuditEvent=除去結果を確定 RecoveryAction=final packageは変更せず署名済みhashで再取得", removed_partials),
                EVIDENCE_SOURCE_INTERNAL_STATE,
                hash,
            )
            .is_err()
        {
            return broker.audit_store_failed_response(
                request_id,
                operation,
                "update_download_audit_failed",
                "中断download復旧結果のAuditを確定できない",
            );
        }
        let queued = match broker.append_audit(
            request_id,
            operation,
            "queued",
            "Capability=署名済みupdate package取得 Permission=Broker固定HTTPS配布元から対象package一つを取得 Approval=Broker現在状態と配布先を示したRust Desktop native Owner確認 AuditEvent=queuedと修復範囲を永続記録 RecoveryAction=同一digest名が長さ／hash不一致の通常fileなら完全検証済み新fileで原子的に置換し、それ以外の不正entryは保持してfail-closed。失敗時はstoreを再検証しinstall／process起動はしない",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            hash,
        ) {
            Ok(event) => event,
            Err(_) => {
                return broker.audit_store_failed_response(
                    request_id,
                    operation,
                    "update_download_audit_failed",
                    "download開始前Auditを確定できない",
                )
            }
        };
        let source_url = confirmation.source_url.clone();
        let started =
            broker
                .update_download
                .start(directory, &request, &current_record, source_url);
        let body = match started {
            Ok(body) => body,
            Err(code) => {
                let event = broker.append_audit(
                    &format!("{}:failed", request_id),
                    operation,
                    "failed",
                    "Capability=署名済みupdate package取得 Permission=download開始前の境界・容量検査だけ Approval=Rust Desktop native Owner確認済み AuditEvent=開始失敗 RecoveryAction=packageを実行せずBroker状態を再確認",
                    EVIDENCE_SOURCE_INTERNAL_STATE,
                    hash,
                );
                if event.is_err() {
                    return broker.audit_store_failed_response(
                        request_id,
                        operation,
                        "update_download_audit_failed",
                        "download開始失敗をAuditできない",
                    );
                }
                return reject(
                    broker,
                    request_id,
                    operation,
                    code,
                    "Brokerが更新download jobを開始できない",
                    hash,
                );
            }
        };
        return BrokerResponse {
            request_id: request_id.to_string(),
            operation: operation.to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
            audit_event_id: queued.event_id,
            error: None,
            health: None,
            body: Some(json!({
                "版": VERSION,
                "download_job": body,
                "install実行": "suspended",
                "復旧ID": "recover-update-download",
                "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
            })),
            shutdown_requested: broker.shutdown_requested,
        };
    }
    if operation == OP_APPLY {
        return apply_verified_package(
            broker,
            &request,
            &current_record,
            request_id,
            hash,
            owner_confirmation,
            apply_confirmation,
        );
    }
    if operation == OP_ACTIVATE {
        return activate_verified_staged_package(
            broker,
            &request,
            &current_record,
            request_id,
            hash,
            owner_confirmation,
            activation_confirmation,
        );
    }
    if operation == OP_ROLLBACK {
        return rollback_active_version(
            broker,
            &request,
            &current_record,
            request_id,
            hash,
            owner_confirmation,
            activation_confirmation,
        );
    }
    suspended(
        broker,
        operation,
        request_id,
        json!({"版": VERSION, "更新ID": request.update_id, "候補hash": request.candidate_hash, "実行状態": "suspended", "復旧ID": "recover-update-execution", "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE}),
        hash,
        "外部download、install、process、rollback実行経路は未接続のためsuspended",
    )
}

#[cfg(windows)]
fn rollback_active_version(
    broker: &mut Broker,
    request: &UpdateIDRequest,
    current_record: &UpdateRecord,
    request_id: &str,
    payload_hash: &str,
    owner_confirmation: OwnerConfirmationSource,
    confirmation: Option<&UpdateActivationConfirmation>,
) -> BrokerResponse {
    if owner_confirmation != OwnerConfirmationSource::DesktopNativeConfirmation {
        return reject(
            broker,
            request_id,
            OP_ROLLBACK,
            "desktop_native_owner_confirmation_required",
            "有効版rollbackには独立したRust Desktop native Owner確認が必要",
            payload_hash,
        );
    }
    let Some(confirmation) = confirmation else {
        return reject(
            broker,
            request_id,
            OP_ROLLBACK,
            "update_rollback_confirmation_missing",
            "現在版とrollback先をBroker状態から表示したnative確認記録がない",
            payload_hash,
        );
    };
    let Some(target_confirmation) = confirmation.rollback_target.as_ref() else {
        return reject(
            broker,
            request_id,
            OP_ROLLBACK,
            "update_rollback_target_missing",
            "rollback先のBroker確認記録がない",
            payload_hash,
        );
    };
    if !current_record.candidate.rollback_available
        || confirmation.update_id != request.update_id
        || confirmation.candidate_hash != request.candidate_hash
        || confirmation.update_id != current_record.candidate.update_id
        || confirmation.candidate_hash != current_record.candidate_hash
        || confirmation.package_sha256
            != current_record
                .candidate
                .package_sha256
                .as_deref()
                .unwrap_or_default()
        || confirmation.offered_version != current_record.candidate.offered_version
        || confirmation.payload_hash != payload_hash
    {
        return reject(
            broker,
            request_id,
            OP_ROLLBACK,
            "update_rollback_confirmation_stale",
            "Owner確認後に現在候補またはpayloadが変化した",
            payload_hash,
        );
    }
    let Some((app_id, audit_store_id)) = broker.desktop_product_identity.clone() else {
        return reject(
            broker,
            request_id,
            OP_ROLLBACK,
            "update_install_product_identity_missing",
            "製品App／Audit Store identityを検証できないためrollbackしない",
            payload_hash,
        );
    };
    if confirmation.app_id != app_id || confirmation.audit_store_id != audit_store_id {
        return reject(
            broker,
            request_id,
            OP_ROLLBACK,
            "update_rollback_identity_stale",
            "native Owner確認の製品identityが現在のBroker identityと異なる",
            payload_hash,
        );
    }
    #[cfg(test)]
    let local_app_data = broker
        .desktop_product_install_local_app_data
        .clone()
        .or_else(|| super::product_install::current_user_local_app_data().ok());
    #[cfg(not(test))]
    let local_app_data = super::product_install::current_user_local_app_data().ok();
    let Some(local_app_data) = local_app_data else {
        return reject(
            broker,
            request_id,
            OP_ROLLBACK,
            "product_install_root_unavailable",
            "現在利用者のKnown Folderを導出できないためrollbackしない",
            payload_hash,
        );
    };
    let snapshot = match crate::product_bootstrapper::active_version_snapshot(
        &local_app_data,
        &app_id,
        &audit_store_id,
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return reject(
                broker,
                request_id,
                OP_ROLLBACK,
                error.0,
                "有効版recordまたはstageを安全に検証できないためrollbackしない",
                payload_hash,
            )
        }
    };
    let Some(previous) = snapshot.previous.as_ref() else {
        return reject(
            broker,
            request_id,
            OP_ROLLBACK,
            "active_version_rollback_unavailable",
            "Brokerが記録した直前の有効版がない",
            payload_hash,
        );
    };
    if !rollback_descriptor_matches_candidate(broker, &snapshot.current)
        || !rollback_descriptor_matches_candidate(broker, previous)
        || snapshot.current.update_id != current_record.candidate.update_id
        || snapshot.current.candidate_hash != current_record.candidate_hash
        || target_confirmation.update_id != previous.update_id
        || target_confirmation.candidate_hash != previous.candidate_hash
        || target_confirmation.package_sha256 != previous.package_sha256
        || target_confirmation.offered_version != previous.product_version
    {
        return reject(
            broker,
            request_id,
            OP_ROLLBACK,
            "update_rollback_target_stale",
            "現在の有効版・直前版・署名候補が確認画面から変化した",
            payload_hash,
        );
    }
    #[cfg(test)]
    let current_plan = super::product_install::plan_product_install(
        &local_app_data,
        &app_id,
        &snapshot.current.product_version,
        &snapshot.current.package_sha256,
    );
    #[cfg(not(test))]
    let current_plan = super::product_install::plan_current_user_product_install(
        &app_id,
        &snapshot.current.product_version,
        &snapshot.current.package_sha256,
    );
    let current_plan = match current_plan {
        Ok(plan) => plan,
        Err(_) => {
            return reject(
                broker,
                request_id,
                OP_ROLLBACK,
                "product_install_plan_invalid",
                "Brokerが現行版の固定導入先を導出できない",
                payload_hash,
            )
        }
    };
    if current_plan.version_directory != confirmation.version_directory {
        return reject(
            broker,
            request_id,
            OP_ROLLBACK,
            "update_rollback_current_destination_stale",
            "Owner確認に表示した現行版stageがKnown Folder由来のstageと異なる",
            payload_hash,
        );
    }
    #[cfg(test)]
    let plan = super::product_install::plan_product_install(
        &local_app_data,
        &app_id,
        &previous.product_version,
        &previous.package_sha256,
    );
    #[cfg(not(test))]
    let plan = super::product_install::plan_current_user_product_install(
        &app_id,
        &previous.product_version,
        &previous.package_sha256,
    );
    let plan = match plan {
        Ok(plan) => plan,
        Err(_) => {
            return reject(
                broker,
                request_id,
                OP_ROLLBACK,
                "product_install_plan_invalid",
                "Brokerがrollback先の固定導入先を導出できない",
                payload_hash,
            )
        }
    };
    if plan.version_directory != target_confirmation.version_directory {
        return reject(
            broker,
            request_id,
            OP_ROLLBACK,
            "update_rollback_destination_stale",
            "Owner確認に表示したrollback先がKnown Folder由来のstageと異なる",
            payload_hash,
        );
    }
    if !broker.state_store.persistence_ready() {
        return suspended(
            broker,
            OP_ROLLBACK,
            request_id,
            json!({"版": VERSION, "更新ID": request.update_id, "候補hash": request.candidate_hash, "状態": "suspended", "復旧ID": "recover-update-rollback-audit", "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE}),
            payload_hash,
            "永続Auditが利用できないため有効版をrollbackしない",
        );
    }
    let (root_path, product_root) =
        match super::product_install::open_existing_product_root(&local_app_data, &app_id) {
            Ok(root) => root,
            Err(_) => {
                return reject(
                    broker,
                    request_id,
                    OP_ROLLBACK,
                    "product_install_root_unavailable",
                    "既存の固定製品rootがないためrollbackしない",
                    payload_hash,
                )
            }
        };
    if root_path != plan.root {
        return reject(
            broker,
            request_id,
            OP_ROLLBACK,
            "product_install_root_mismatch",
            "Known Folder由来の製品rootがrollback計画と一致しない",
            payload_hash,
        );
    }
    let (versions_path, versions) =
        match super::product_install::open_existing_product_versions_directory(
            &local_app_data,
            &app_id,
        ) {
            Ok(versions) => versions,
            Err(_) => {
                return reject(
                    broker,
                    request_id,
                    OP_ROLLBACK,
                    "product_install_versions_unavailable",
                    "固定versions directoryがないためrollbackしない",
                    payload_hash,
                )
            }
        };
    if plan.version_directory.parent() != Some(versions_path.as_path()) {
        return reject(
            broker,
            request_id,
            OP_ROLLBACK,
            "product_install_versions_mismatch",
            "rollback先の親がBroker versions capabilityと一致しない",
            payload_hash,
        );
    }
    let intent = match broker.append_audit(
        request_id,
        OP_ROLLBACK,
        "queued",
        &format!("Capability=product.install.rollback_version Permission=Brokerが過去に有効化した署名済み版recordの切替だけ Approval=現行版と直前版を表示した独立Rust Desktop native Owner確認 AuditEvent=有効版record切替前の意図を永続化 RecoveryAction=両候補を現在trustで再検証し、stage hashを照合。process起動・file削除は行わない current_version={} current_package_sha256={} target_version={} target_package_sha256={}", snapshot.current.product_version, snapshot.current.package_sha256, previous.product_version, previous.package_sha256),
        EVIDENCE_SOURCE_INTERNAL_STATE,
        payload_hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_ROLLBACK,
                "update_rollback_audit_failed",
                "rollback前Auditを確定できない",
            )
        }
    };
    let mut suffix_bytes = [0u8; 16];
    if getrandom::getrandom(&mut suffix_bytes).is_err() {
        return rollback_failed(
            broker,
            request_id,
            payload_hash,
            "active_version_nonce_failed",
            "rollback recordの一時名を生成できない",
        );
    }
    let restored = crate::product_bootstrapper::toggle_previous_active_version(
        &product_root,
        &versions,
        &app_id,
        &audit_store_id,
        &snapshot.current.update_id,
        &snapshot.current.candidate_hash,
        &previous.update_id,
        &previous.candidate_hash,
        &hex::encode(suffix_bytes),
    );
    let restored = match restored {
        Ok(restored) => restored,
        Err(error) => {
            return rollback_failed(
                broker,
                request_id,
                payload_hash,
                error.0,
                "有効版recordを変更できずrollbackを完了しない",
            )
        }
    };
    let completion = match broker.append_audit(
        &format!("{request_id}:complete"),
        OP_ROLLBACK,
        "completed",
        &format!("Capability=product.install.rollback_version Permission=有効版recordだけを直前の署名済みstageへatomic切替 Approval=独立Rust Desktop native Owner確認 AuditEvent=rollback完了を記録 RecoveryAction=Bootstrapperは次回起動時に有効版recordとstage hashを再検証する。process起動・file削除・Start Menu変更なし current_version={} target_version={} intent_audit_id={}", snapshot.current.product_version, restored.product_version, intent.event_id),
        EVIDENCE_SOURCE_INTERNAL_STATE,
        payload_hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_ROLLBACK,
                "update_rollback_audit_failed",
                "rollback後の完了Auditを確定できない。record状態を再確認する",
            )
        }
    };
    BrokerResponse {
        request_id: request_id.to_owned(),
        operation: OP_ROLLBACK.to_owned(),
        status: BrokerStatus::Accepted,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_owned(),
        audit_event_id: completion.event_id,
        error: None,
        health: None,
        body: Some(json!({
            "版": VERSION,
            "rollback": "active_version_restored",
            "現行版": snapshot.current.product_version,
            "復元版": restored.product_version,
            "起動": "not_started",
            "削除": "none",
            "開始Audit ID": intent.event_id,
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        })),
        shutdown_requested: broker.shutdown_requested,
    }
}

#[cfg(not(windows))]
fn rollback_active_version(
    broker: &mut Broker,
    request: &UpdateIDRequest,
    _current_record: &UpdateRecord,
    request_id: &str,
    payload_hash: &str,
    _owner_confirmation: OwnerConfirmationSource,
    _confirmation: Option<&UpdateActivationConfirmation>,
) -> BrokerResponse {
    suspended(
        broker,
        OP_ROLLBACK,
        request_id,
        json!({"版": VERSION, "更新ID": request.update_id, "候補hash": request.candidate_hash, "状態": "suspended", "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE}),
        payload_hash,
        "Windows製品root以外では有効版rollbackを実行しない",
    )
}

fn rollback_failed(
    broker: &mut Broker,
    request_id: &str,
    payload_hash: &str,
    failure_code: &str,
    message: &str,
) -> BrokerResponse {
    if broker
        .append_audit(
            &format!("{request_id}:failed"),
            OP_ROLLBACK,
            "failed",
            &format!("Capability=product.install.rollback_version Permission=固定製品root内の有効版recordだけ Approval=独立Rust Desktop native Owner確認 AuditEvent=rollback失敗を記録 RecoveryAction=stageと既存recordを削除せず、現在の署名候補・root・有効版を再照合 failure_code={failure_code}"),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_ROLLBACK,
            "update_rollback_audit_failed",
            "rollback失敗のAuditを確定できない",
        );
    }
    reject(
        broker,
        request_id,
        OP_ROLLBACK,
        failure_code,
        message,
        payload_hash,
    )
}

fn apply_verified_package(
    broker: &mut Broker,
    request: &UpdateIDRequest,
    record: &UpdateRecord,
    request_id: &str,
    payload_hash: &str,
    owner_confirmation: OwnerConfirmationSource,
    confirmation: Option<&UpdateApplyConfirmation>,
) -> BrokerResponse {
    if owner_confirmation != OwnerConfirmationSource::DesktopNativeConfirmation {
        return reject(
            broker,
            request_id,
            OP_APPLY,
            "desktop_native_owner_confirmation_required",
            "更新package導入にはRust Desktop起動器のnative Owner確認が必要",
            payload_hash,
        );
    }
    let Some(confirmation) = confirmation else {
        return reject(
            broker,
            request_id,
            OP_APPLY,
            "update_apply_confirmation_missing",
            "固定導入先とpackage identityを示したnative確認記録がない",
            payload_hash,
        );
    };
    let Some((app_id, audit_store_id)) = broker.desktop_product_identity.clone() else {
        return reject(
            broker,
            request_id,
            OP_APPLY,
            "update_install_product_identity_missing",
            "製品App／Audit Store identityを検証できないため導入しない",
            payload_hash,
        );
    };
    let Some(package_sha256) = record.candidate.package_sha256.as_deref() else {
        return reject(
            broker,
            request_id,
            OP_APPLY,
            "update_package_binding_required",
            "署名済みpackage digestがない",
            payload_hash,
        );
    };
    let Some(package_bytes) = record.candidate.package_size_bytes else {
        return reject(
            broker,
            request_id,
            OP_APPLY,
            "update_package_binding_required",
            "署名済みpackage byte長がない",
            payload_hash,
        );
    };
    let download = &confirmation.download;
    if download.update_id != record.candidate.update_id
        || download.candidate_hash != record.candidate_hash
        || download.package_sha256 != package_sha256
        || download.package_size_bytes != package_bytes
        || download.offered_version != record.candidate.offered_version
        || download.channel != record.candidate.channel
        || download.summary != record.candidate.summary
        || download.payload_hash != payload_hash
        || confirmation.app_id != app_id
        || confirmation.audit_store_id != audit_store_id
    {
        return reject(
            broker,
            request_id,
            OP_APPLY,
            "update_apply_confirmation_stale",
            "native Owner確認後に候補、製品identity、または操作payloadが変化した",
            payload_hash,
        );
    }

    #[cfg(not(windows))]
    {
        let _ = (request, record);
        return suspended(
            broker,
            OP_APPLY,
            request_id,
            json!({"版": VERSION, "更新ID": request.update_id, "候補hash": request.candidate_hash, "状態": "suspended", "復旧ID": "recover-update-install-platform", "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE}),
            payload_hash,
            "固定install rootのWindows Known Folder経路がないためsuspended",
        );
    }

    #[cfg(windows)]
    {
        #[cfg(test)]
        let install_plan = broker
            .desktop_product_install_local_app_data
            .as_deref()
            .map(|local_app_data| {
                super::product_install::plan_product_install(
                    local_app_data,
                    &app_id,
                    &record.candidate.offered_version,
                    package_sha256,
                )
            });
        #[cfg(not(test))]
        let install_plan: Option<
            Result<
                super::product_install::ProductInstallPlan,
                super::product_install::ProductInstallPlanError,
            >,
        > = None;
        let plan_result = match install_plan {
            Some(plan) => plan,
            None => super::product_install::plan_current_user_product_install(
                &app_id,
                &record.candidate.offered_version,
                package_sha256,
            ),
        };
        let plan = match plan_result {
            Ok(plan) => plan,
            Err(_) => {
                return reject(
                    broker,
                    request_id,
                    OP_APPLY,
                    "product_install_plan_invalid",
                    "Brokerが固定導入先を導出できない",
                    payload_hash,
                )
            }
        };
        if plan.version_directory != confirmation.version_directory {
            return reject(
                broker,
                request_id,
                OP_APPLY,
                "update_apply_destination_stale",
                "native Owner確認に表示した固定導入先が現在のKnown Folder計画と異なる",
                payload_hash,
            );
        }
        let Some(stage_name) = plan
            .version_directory
            .file_name()
            .and_then(|name| name.to_str())
        else {
            return reject(
                broker,
                request_id,
                OP_APPLY,
                "product_install_stage_invalid",
                "content-addressed version directory名が不正",
                payload_hash,
            );
        };
        if !broker.state_store.persistence_ready() {
            return suspended(
                broker,
                OP_APPLY,
                request_id,
                json!({"版": VERSION, "更新ID": request.update_id, "候補hash": request.candidate_hash, "状態": "suspended", "復旧ID": "recover-update-install-audit", "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE}),
                payload_hash,
                "永続Auditが利用できないためpackageを導入しない",
            );
        }
        let package_directory = match broker.state_store.open_update_package_directory() {
            Ok(Some(directory)) => directory,
            _ => {
                return reject(
                    broker,
                    request_id,
                    OP_APPLY,
                    "update_package_store_unavailable",
                    "Broker固定download package directoryを開けない",
                    payload_hash,
                )
            }
        };
        let package_name = format!("{package_sha256}.pkg");
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let mut package = match package_directory.open_with(&package_name, &options) {
            Ok(package) => package,
            Err(_) => {
                return reject(
                    broker,
                    request_id,
                    OP_APPLY,
                    "update_package_missing",
                    "署名済みdigest名のdownload packageがない",
                    payload_hash,
                )
            }
        };
        let package_metadata = match package.metadata() {
            Ok(metadata)
                if metadata.is_file()
                    && metadata.file_attributes() & 0x400 == 0
                    && metadata.len() == package_bytes =>
            {
                metadata
            }
            _ => {
                return reject(
                    broker,
                    request_id,
                    OP_APPLY,
                    "update_package_file_invalid",
                    "download packageの通常file属性または署名済みbyte長が一致しない",
                    payload_hash,
                )
            }
        };
        let intent_reason = format!(
            "Capability=product.install.version_stage Permission=現在の署名済み候補・App ID・version・package digestの固定導入先一回限定 Approval=内容を表示したRust Desktop native Owner確認 AuditEvent=展開前意図を永続化 RecoveryAction=未起動version directoryは自動実行せず、再起動後にpackage digestとstage全体を再検証してから復旧判断。candidate={} version={} package_sha256={} bytes={}",
            record.candidate.update_id,
            record.candidate.offered_version,
            package_sha256,
            package_bytes
        );
        let intent = match broker.append_audit(
            request_id,
            OP_APPLY,
            "queued",
            &intent_reason,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        ) {
            Ok(event) => event,
            Err(_) => {
                return broker.audit_store_failed_response(
                    request_id,
                    OP_APPLY,
                    "update_apply_audit_failed",
                    "導入開始前Auditを確定できない",
                )
            }
        };
        #[cfg(test)]
        let injected_versions_directory = broker
            .desktop_product_install_local_app_data
            .as_deref()
            .map(|local_app_data| {
                super::product_install::open_product_versions_directory(local_app_data, &app_id)
            });
        #[cfg(not(test))]
        let injected_versions_directory: Option<
            Result<(PathBuf, cap_std::fs::Dir), super::product_install::ProductInstallPlanError>,
        > = None;
        let versions_result = match injected_versions_directory {
            Some(result) => result,
            None => super::product_install::open_current_user_product_versions_directory(&app_id),
        };
        let (versions_path, versions_directory) = match versions_result {
            Ok(result) => result,
            Err(_) => {
                if broker
                    .append_audit(
                    &format!("{request_id}:failed"),
                    OP_APPLY,
                    "failed",
                    "Capability=product.install.version_stage Permission=現在候補の導入先のみ Approval=Rust Desktop native Owner確認 AuditEvent=導入rootを開けない結果を記録 RecoveryAction=package内容は変更せずKnown FolderとAuditを再確認",
                    EVIDENCE_SOURCE_INTERNAL_STATE,
                    payload_hash,
                    )
                    .is_err()
                {
                    return broker.audit_store_failed_response(
                        request_id,
                        OP_APPLY,
                        "update_apply_audit_failed",
                        "導入rootを開けない結果のAuditを確定できない",
                    );
                }
                return reject(
                    broker,
                    request_id,
                    OP_APPLY,
                    "product_install_root_unavailable",
                    "Known Folder配下の固定導入先をcapabilityで開けない",
                    payload_hash,
                );
            }
        };
        if plan.version_directory.parent() != Some(versions_path.as_path()) {
            if broker
                .append_audit(
                    &format!("{request_id}:failed"),
                    OP_APPLY,
                    "failed",
                    "Capability=product.install.version_stage Permission=現在候補の導入先のみ Approval=Rust Desktop native Owner確認 AuditEvent=directory capability不一致を記録 RecoveryAction=package内容は変更せず導入先を再確認",
                    EVIDENCE_SOURCE_INTERNAL_STATE,
                    payload_hash,
                )
                .is_err()
            {
                return broker.audit_store_failed_response(
                    request_id,
                    OP_APPLY,
                    "update_apply_audit_failed",
                    "導入先不一致のAuditを確定できない",
                );
            }
            return reject(
                broker,
                request_id,
                OP_APPLY,
                "product_install_root_mismatch",
                "導入先pathとBroker directory capabilityが一致しない",
                payload_hash,
            );
        }
        let extraction = crate::product_package::resume_verified_package_into_directory(
            &mut package,
            package_metadata.len(),
            Some(package_sha256),
            &versions_directory,
            stage_name,
            crate::product_package::ProductPackageExpectation {
                product_version: Some(&record.candidate.offered_version),
                app_id: &app_id,
                audit_store_id: &audit_store_id,
            },
        );
        let (info, resumed) = match extraction {
            Ok(result) => result,
            Err(error) => {
                if broker
                    .append_audit(
                        &format!("{request_id}:failed"),
                        OP_APPLY,
                        "failed",
                        &format!("Capability=product.install.version_stage Permission=確認済み候補に一回限定 Approval=Rust Desktop native Owner確認 AuditEvent=展開失敗を記録 RecoveryAction=新規stageだけをreaderがcleanupし、再開stageは内容不一致時も保持する。再試行前に同じ署名packageと既存file prefixを再検証。failure_code={}", error.0),
                        EVIDENCE_SOURCE_INTERNAL_STATE,
                        payload_hash,
                    )
                    .is_err()
                {
                    return broker.audit_store_failed_response(
                        request_id,
                        OP_APPLY,
                        "update_apply_audit_failed",
                        "導入失敗をAuditへ確定できない",
                    );
                }
                return reject(
                    broker,
                    request_id,
                    OP_APPLY,
                    error.0,
                    "署名済みpackageの構造・identity・file hash検証に失敗",
                    payload_hash,
                );
            }
        };
        let completion = broker.append_audit(
            &format!("{request_id}:complete"),
            OP_APPLY,
            "completed",
            &format!("Capability=product.install.version_stage Permission=署名済みApp identityとcontent-addressed version directory一回限定 Approval=Rust Desktop native Owner確認 AuditEvent=完全展開・package／file digest一致を記録 RecoveryAction=既存stageはpackage prefix hashと全entryを再照合してのみ再開し、不一致・余分なentryを変更しない。versionは起動・有効化しない。resumed={resumed} file_count={} total_bytes={} package_sha256={}", info.file_count, info.total_file_bytes, package_sha256),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        );
        let completion = match completion {
            Ok(event) => event,
            Err(_) => {
                return broker.audit_store_failed_response(
                    request_id,
                    OP_APPLY,
                    "update_apply_audit_failed",
                    "展開後のAuditを確定できない。version directoryは自動起動しない",
                )
            }
        };
        return BrokerResponse {
            request_id: request_id.to_string(),
            operation: OP_APPLY.to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
            audit_event_id: completion.event_id,
            error: None,
            health: None,
            body: Some(json!({
                "版": VERSION,
                "更新ID": request.update_id,
                "候補hash": request.candidate_hash,
                "導入状態": "version_staged",
                "有効化": "suspended",
                "再開": resumed,
                "file数": info.file_count,
                "total_bytes": info.total_file_bytes,
                "復旧ID": "recover-update-install-activation",
                "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
                "開始Audit ID": intent.event_id,
            })),
            shutdown_requested: broker.shutdown_requested,
        };
    }
}

fn activate_verified_staged_package(
    broker: &mut Broker,
    request: &UpdateIDRequest,
    record: &UpdateRecord,
    request_id: &str,
    payload_hash: &str,
    owner_confirmation: OwnerConfirmationSource,
    confirmation: Option<&UpdateActivationConfirmation>,
) -> BrokerResponse {
    if owner_confirmation != OwnerConfirmationSource::DesktopNativeConfirmation {
        return reject(
            broker,
            request_id,
            OP_ACTIVATE,
            "desktop_native_owner_confirmation_required",
            "有効版の切替には独立したRust Desktop native Owner確認が必要",
            payload_hash,
        );
    }
    let Some(confirmation) = confirmation else {
        return reject(
            broker,
            request_id,
            OP_ACTIVATE,
            "update_activation_confirmation_missing",
            "切替対象と固定導入先を表示したnative確認記録がない",
            payload_hash,
        );
    };
    let Some((app_id, audit_store_id)) = broker.desktop_product_identity.clone() else {
        return reject(
            broker,
            request_id,
            OP_ACTIVATE,
            "update_install_product_identity_missing",
            "製品App／Audit Store identityを検証できないため有効化しない",
            payload_hash,
        );
    };
    let Some(package_sha256) = record.candidate.package_sha256.as_deref() else {
        return reject(
            broker,
            request_id,
            OP_ACTIVATE,
            "update_package_binding_required",
            "署名済みpackage digestがない",
            payload_hash,
        );
    };
    let Some(package_bytes) = record.candidate.package_size_bytes else {
        return reject(
            broker,
            request_id,
            OP_ACTIVATE,
            "update_package_binding_required",
            "署名済みpackage byte長がない",
            payload_hash,
        );
    };
    if confirmation.update_id != record.candidate.update_id
        || confirmation.candidate_hash != record.candidate_hash
        || confirmation.package_sha256 != package_sha256
        || confirmation.package_size_bytes != package_bytes
        || confirmation.offered_version != record.candidate.offered_version
        || confirmation.channel != record.candidate.channel
        || confirmation.summary != record.candidate.summary
        || confirmation.payload_hash != payload_hash
        || confirmation.app_id != app_id
        || confirmation.audit_store_id != audit_store_id
    {
        return reject(
            broker,
            request_id,
            OP_ACTIVATE,
            "update_activation_confirmation_stale",
            "native Owner確認後に候補、製品identity、または要求が変化した",
            payload_hash,
        );
    }

    #[cfg(not(windows))]
    {
        let _ = (request, record, confirmation);
        return suspended(
            broker,
            OP_ACTIVATE,
            request_id,
            json!({"版": VERSION, "更新ID": request.update_id, "候補hash": request.candidate_hash, "有効化": "suspended", "復旧ID": "recover-update-activation-platform", "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE}),
            payload_hash,
            "Windows Known Folderと固定root capabilityがないため有効版を切り替えない",
        );
    }

    #[cfg(windows)]
    {
        #[cfg(test)]
        let local_app_data = broker
            .desktop_product_install_local_app_data
            .clone()
            .or_else(|| super::product_install::current_user_local_app_data().ok());
        #[cfg(not(test))]
        let local_app_data = super::product_install::current_user_local_app_data().ok();
        let Some(local_app_data) = local_app_data else {
            return reject(
                broker,
                request_id,
                OP_ACTIVATE,
                "product_install_known_folder_unavailable",
                "Windows Known Folderから固定導入rootを導出できない",
                payload_hash,
            );
        };
        let plan = match super::product_install::plan_product_install(
            &local_app_data,
            &app_id,
            &record.candidate.offered_version,
            package_sha256,
        ) {
            Ok(plan) => plan,
            Err(_) => {
                return reject(
                    broker,
                    request_id,
                    OP_ACTIVATE,
                    "product_install_plan_invalid",
                    "Brokerが切替対象の固定導入先を導出できない",
                    payload_hash,
                )
            }
        };
        #[cfg(test)]
        let start_menu_directory = broker
            .desktop_product_start_menu_directory
            .clone()
            .or_else(|| super::product_install::current_user_start_menu_directory().ok());
        #[cfg(not(test))]
        let start_menu_directory = super::product_install::current_user_start_menu_directory().ok();
        let Some(start_menu_directory) = start_menu_directory else {
            return reject(
                broker,
                request_id,
                OP_ACTIVATE,
                "product_start_menu_known_folder_unavailable",
                "Windows Known Folderから現在利用者のStart Menuを導出できない",
                payload_hash,
            );
        };
        let planned_start_menu_shortcut = match super::product_install::plan_start_menu_shortcut(
            &start_menu_directory,
            &app_id,
        ) {
            Ok(path) => path,
            Err(_) => {
                return reject(
                    broker,
                    request_id,
                    OP_ACTIVATE,
                    "product_start_menu_plan_invalid",
                    "BrokerがStart Menu shortcutの固定配置を導出できない",
                    payload_hash,
                )
            }
        };
        if planned_start_menu_shortcut != confirmation.start_menu_shortcut_path {
            return reject(
                broker,
                request_id,
                OP_ACTIVATE,
                "update_activation_start_menu_stale",
                "native Owner確認に表示したStart Menu shortcut先が現在のKnown Folder計画と異なる",
                payload_hash,
            );
        }
        if plan.version_directory != confirmation.version_directory {
            return reject(
                broker,
                request_id,
                OP_ACTIVATE,
                "update_activation_destination_stale",
                "native Owner確認に表示したversion directoryが現在のKnown Folder計画と異なる",
                payload_hash,
            );
        }
        let Some(stage_name) = plan
            .version_directory
            .file_name()
            .and_then(|name| name.to_str())
        else {
            return reject(
                broker,
                request_id,
                OP_ACTIVATE,
                "product_install_stage_invalid",
                "content-addressed version directory名が不正",
                payload_hash,
            );
        };
        if !broker.state_store.persistence_ready() {
            return suspended(
                broker,
                OP_ACTIVATE,
                request_id,
                json!({"版": VERSION, "更新ID": request.update_id, "候補hash": request.candidate_hash, "有効化": "suspended", "復旧ID": "recover-update-activation-audit", "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE}),
                payload_hash,
                "永続Auditが利用できないため有効版を切り替えない",
            );
        }
        let package_directory = match broker.state_store.open_update_package_directory() {
            Ok(Some(directory)) => directory,
            _ => {
                return reject(
                    broker,
                    request_id,
                    OP_ACTIVATE,
                    "update_package_store_unavailable",
                    "Broker固定download package directoryを開けない",
                    payload_hash,
                )
            }
        };
        let package_name = format!("{package_sha256}.pkg");
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let mut package = match package_directory.open_with(&package_name, &options) {
            Ok(package) => package,
            Err(_) => {
                return reject(
                    broker,
                    request_id,
                    OP_ACTIVATE,
                    "update_package_missing",
                    "署名済みdigest名のdownload packageがない",
                    payload_hash,
                )
            }
        };
        let package_metadata = match package.metadata() {
            Ok(metadata)
                if metadata.is_file()
                    && metadata.file_attributes() & 0x400 == 0
                    && metadata.len() == package_bytes =>
            {
                metadata
            }
            _ => {
                return reject(
                    broker,
                    request_id,
                    OP_ACTIVATE,
                    "update_package_file_invalid",
                    "download packageの通常file属性または署名済みbyte長が一致しない",
                    payload_hash,
                )
            }
        };
        let (root_path, product_root) =
            match super::product_install::open_existing_product_root(&local_app_data, &app_id) {
                Ok(root) => root,
                Err(_) => {
                    return reject(
                        broker,
                        request_id,
                        OP_ACTIVATE,
                        "product_install_root_unavailable",
                        "固定製品rootがないため有効版を切り替えない",
                        payload_hash,
                    )
                }
            };
        if root_path != plan.root {
            return reject(
                broker,
                request_id,
                OP_ACTIVATE,
                "product_install_root_mismatch",
                "Known Folder由来の製品rootが導入計画と一致しない",
                payload_hash,
            );
        }
        let (versions_path, versions) =
            match super::product_install::open_existing_product_versions_directory(
                &local_app_data,
                &app_id,
            ) {
                Ok(versions) => versions,
                Err(_) => {
                    return reject(
                        broker,
                        request_id,
                        OP_ACTIVATE,
                        "product_install_versions_unavailable",
                        "既存の固定versions directoryがないため有効版を切り替えない",
                        payload_hash,
                    )
                }
            };
        if plan.version_directory.parent() != Some(versions_path.as_path()) {
            return reject(
                broker,
                request_id,
                OP_ACTIVATE,
                "product_install_versions_mismatch",
                "導入計画の親とBroker versions capabilityが一致しない",
                payload_hash,
            );
        }

        let intent_reason = format!(
            "Capability=product.install.activate_version Permission=現在の署名候補・App ID・version・package digestとStart Menu Known Folderから固定導出した有効版record／shortcut一回限定 Approval=切替対象とshortcut先を表示した独立Rust Desktop native Owner確認 AuditEvent=切替前意図を永続化 RecoveryAction=stageとpackageをread-only検証し、失敗時は既存stage／active record／shortcutを自動削除しない。process起動・rollbackは行わない。candidate={} version={} package_sha256={} bytes={}",
            record.candidate.update_id,
            record.candidate.offered_version,
            package_sha256,
            package_bytes
        );
        let intent = match broker.append_audit(
            request_id,
            OP_ACTIVATE,
            "queued",
            &intent_reason,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        ) {
            Ok(event) => event,
            Err(_) => {
                return broker.audit_store_failed_response(
                    request_id,
                    OP_ACTIVATE,
                    "update_activation_audit_failed",
                    "有効版切替前Auditを確定できない",
                )
            }
        };
        let info = match crate::product_package::verify_staged_package_into_directory(
            &mut package,
            package_metadata.len(),
            Some(package_sha256),
            &versions,
            stage_name,
            crate::product_package::ProductPackageExpectation {
                product_version: Some(&record.candidate.offered_version),
                app_id: &app_id,
                audit_store_id: &audit_store_id,
            },
        ) {
            Ok(info) => info,
            Err(error) => {
                return activation_failed(
                    broker,
                    request_id,
                    payload_hash,
                    error.0,
                    "署名済みpackageと既存version stageが完全一致しないため切替を拒否",
                )
            }
        };
        let mut suffix_bytes = [0u8; 16];
        if getrandom::getrandom(&mut suffix_bytes).is_err() {
            return activation_failed(
                broker,
                request_id,
                payload_hash,
                "active_version_nonce_failed",
                "有効版recordの一時名を安全に生成できず切替を拒否",
            );
        }
        if let Err(error) = crate::product_bootstrapper::activate_staged_version(
            &product_root,
            &versions,
            &info.product_version,
            &app_id,
            &audit_store_id,
            package_sha256,
            &record.candidate.update_id,
            &record.candidate_hash,
            &hex::encode(suffix_bytes),
        ) {
            return activation_failed(
                broker,
                request_id,
                payload_hash,
                error.0,
                "固定root Bootstrapperまたは有効版recordを安全に公開できず切替を拒否",
            );
        }
        let registered_shortcut =
            match super::product_install::register_start_menu_shortcut(
                &local_app_data,
                &start_menu_directory,
                &app_id,
                &root_path,
            ) {
                Ok(path) => path,
                Err(error) => {
                    return activation_failed(
                        broker,
                        request_id,
                        payload_hash,
                        error.0,
                        "有効版record公開後のStart Menu shortcut登録が未完了。既存状態を維持し、Owner確認付きで再試行する",
                    )
                }
            };
        if registered_shortcut != confirmation.start_menu_shortcut_path {
            return activation_failed(
                broker,
                request_id,
                payload_hash,
                "product_start_menu_published_path_mismatch",
                "Start Menu shortcut公開先が確認済み固定pathと一致しない",
            );
        }
        let completion = broker.append_audit(
            &format!("{request_id}:complete"),
            OP_ACTIVATE,
            "completed",
            &format!("Capability=product.install.activate_version; portable起動元ではproduct.runtime.launchを続行 Permission=現在の署名候補と完全検証済み固定stageだけを有効版recordへ設定し固定Start Menu shortcutを登録 Approval=独立Rust Desktop native Owner確認。portableからのInstall後に導入済み版を起動することを確認画面へ明示 AuditEvent=固定root Bootstrapper配置、有効版recordのatomic公開、Start Menu shortcut登録を記録 RecoveryAction=起動が成立しなければ有効版recordとstageを保持しStart Menuから再試行。installed Broker startupが起動を記録 version={} package_sha256={} file_count={} total_bytes={} Start_Menu=registered intent_audit_id={}", info.product_version, package_sha256, info.file_count, info.total_file_bytes, intent.event_id),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        );
        let completion = match completion {
            Ok(event) => event,
            Err(_) => {
                return broker.audit_store_failed_response(
                    request_id,
                    OP_ACTIVATE,
                    "update_activation_audit_failed",
                    "有効版record公開後の完了Auditを確定できない。再起動・起動成立は主張しない",
                )
            }
        };
        let package_cleanup_intent = broker.append_audit(
            &format!("{request_id}:package-cleanup"),
            "更新package保管領域清掃",
            "queued",
            &format!("Capability=product.update.package_cleanup Permission=完全検証stageが有効版recordへ登録済みの同一content-addressed package fileだけ Approval=有効版切替の独立Rust Desktop native Owner確認 AuditEvent=削除前意図を確定 RecoveryAction=Broker固定store内の期待byte長・SHA-256一致を再検査し、不一致なら保持 package_sha256={} package_bytes={} activation_audit_id={}", package_sha256, package_bytes, completion.event_id),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        );
        drop(package);
        let package_cleanup_result = match package_cleanup_intent {
            Ok(intent) => match crate::broker::update_download::remove_verified_package(
                &package_directory,
                package_sha256,
                package_bytes,
            ) {
                Ok(removed) => broker
                    .append_audit(
                        &format!("{request_id}:package-cleanup:complete"),
                        "更新package保管領域清掃",
                        "completed",
                        &format!("Capability=product.update.package_cleanup Permission=検証済みstaged packageをcontent-addressed filenameとbyte長・SHA-256一致後だけ除去 Approval=有効版切替Owner確認済み AuditEvent=stage／active record完了後のdownload重複file cleanupを記録 RecoveryAction=version stageは保持し、rollback先の実行可能stageとして再検証可能 removed={removed} package_sha256={} intent_audit_id={}", package_sha256, intent.event_id),
                        EVIDENCE_SOURCE_INTERNAL_STATE,
                        payload_hash,
                    )
                    .map(|event| Some(event.event_id))
                    .unwrap_or(None),
                Err(error) => {
                    let _ = broker.append_audit(
                        &format!("{request_id}:package-cleanup:failed"),
                        "更新package保管領域清掃",
                        "failed",
                        &format!("Capability=product.update.package_cleanup Permission=期待hash／byte長が一致するdownload packageだけ Approval=有効版切替Owner確認済み AuditEvent=cleanup失敗を記録 RecoveryAction=有効版recordとstageを維持し、packageを削除せず更新package保管状態を再確認。error_class={error:?}"),
                        EVIDENCE_SOURCE_INTERNAL_STATE,
                        payload_hash,
                    );
                    None
                }
            },
            Err(_) => None,
        };
        return BrokerResponse {
            request_id: request_id.to_owned(),
            operation: OP_ACTIVATE.to_owned(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_owned(),
            audit_event_id: completion.event_id,
            error: None,
            health: None,
            body: Some(json!({
                "版": VERSION,
                "更新ID": request.update_id,
                "候補hash": request.candidate_hash,
                "有効化": "active_version_recorded",
                "起動": "not_started",
                "Start Menu": "registered",
                "download package cleanup": if package_cleanup_result.is_some() { "completed" } else { "pending" },
                "file数": info.file_count,
                "total_bytes": info.total_file_bytes,
                "開始Audit ID": intent.event_id,
                "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
            })),
            shutdown_requested: broker.shutdown_requested,
        };
    }
}

fn activation_failed(
    broker: &mut Broker,
    request_id: &str,
    payload_hash: &str,
    failure_code: &str,
    message: &str,
) -> BrokerResponse {
    if broker
        .append_audit(
            &format!("{request_id}:failed"),
            OP_ACTIVATE,
            "failed",
            &format!("Capability=product.install.activate_version Permission=固定製品root内の有効版recordだけ Approval=独立Rust Desktop native Owner確認 AuditEvent=有効版切替失敗を記録 RecoveryAction=既存active recordとstageを自動削除・修復せず、署名packageと固定rootを再検証。failure_code={failure_code}"),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .is_err()
    {
        return broker.audit_store_failed_response(
            request_id,
            OP_ACTIVATE,
            "update_activation_audit_failed",
            "有効版切替失敗のAuditを確定できない",
        );
    }
    reject(
        broker,
        request_id,
        OP_ACTIVATE,
        failure_code,
        message,
        payload_hash,
    )
}

fn verify_candidate(
    broker: &Broker,
    candidate: UpdateCandidateDocument,
) -> Result<UpdateRecord, (&'static str, &'static str)> {
    verify_candidate_with_trust(broker.update_trust.as_ref(), candidate)
}

fn verify_candidate_with_trust(
    trust: Option<&UpdateTrust>,
    candidate: UpdateCandidateDocument,
) -> Result<UpdateRecord, (&'static str, &'static str)> {
    match candidate.version {
        1 => {
            return Err((
                "update_package_binding_required",
                "新しい更新候補には配布packageのhashとbyte長を署名へ含める必要がある",
            ));
        }
        CANDIDATE_VERSION => {}
        _ => return Err(("update_candidate_invalid", "更新候補の版が未対応")),
    }
    validate_candidate(&candidate)
        .map_err(|_| ("update_candidate_invalid", "更新候補の値が不正"))?;
    let Some(trust) = trust else {
        return Err((
            "update_trust_unconfigured",
            "Broker所有の更新署名信頼設定が未構成",
        ));
    };
    let public_key_der = hex::decode(&trust.public_key_der_hex)
        .map_err(|_| ("update_trusted_key_invalid", "更新公開鍵のhexが不正"))?;
    let signed_bytes = hex::decode(&candidate.signed_bytes_hex)
        .map_err(|_| ("update_candidate_invalid", "署名対象hexが不正"))?;
    let signature = hex::decode(&candidate.signature_hex)
        .map_err(|_| ("update_candidate_invalid", "署名hexが不正"))?;
    let signed_manifest = SignedManifest {
        version: candidate.version,
        update_id: &candidate.update_id,
        current_version: &candidate.current_version,
        offered_version: &candidate.offered_version,
        channel: &candidate.channel,
        summary: &candidate.summary,
        package_sha256: candidate
            .package_sha256
            .as_deref()
            .ok_or(("update_candidate_invalid", "package SHA-256が必要"))?,
        package_size_bytes: candidate
            .package_size_bytes
            .ok_or(("update_candidate_invalid", "package byte長が必要"))?,
        rollback_available: candidate.rollback_available,
    };
    let canonical_bytes = serde_json::to_vec(&signed_manifest)
        .map_err(|_| ("update_candidate_invalid", "署名対象の正本化に失敗"))?;
    if signed_bytes != canonical_bytes || signed_bytes.len() > MAX_MANIFEST_BYTES {
        return Err((
            "update_signed_manifest_mismatch",
            "署名対象が候補metadataの正本byteと一致しない",
        ));
    }
    let response = verify_signed_update_signature(
        &SignedUpdateCandidate {
            update_id: candidate.update_id.clone(),
            signed_bytes,
            signature: Some(signature),
            signer_fingerprint: candidate.signer_fingerprint.clone(),
        },
        &public_key_der,
        &trust.public_key_fingerprint,
    );
    if !response.ok {
        let error = response
            .error
            .ok_or(("update_signature_invalid", "更新署名検査に失敗"))?;
        return Err(match error.code.as_str() {
            "update_trusted_key_invalid" => ("update_trusted_key_invalid", "更新公開鍵が不正"),
            "update_signer_untrusted" => (
                "update_signer_untrusted",
                "更新署名者が信頼設定と一致しない",
            ),
            "update_signature_required" => ("update_signature_required", "更新署名が必要"),
            _ => ("update_signature_invalid", "更新署名検査に失敗"),
        });
    }
    let candidate_value = serde_json::to_value(&candidate)
        .map_err(|_| ("update_candidate_invalid", "候補hashの計算に失敗"))?;
    Ok(UpdateRecord {
        candidate,
        signature_status: "verified".to_string(),
        candidate_hash: crate::broker::protocol::canonical_payload_hash(Some(&candidate_value)),
        deferred_until: None,
    })
}

fn persist(broker: &Broker) -> Result<(), String> {
    broker
        .state_store
        .write_update_state(&json!({"版": VERSION, "updates": broker.updates.values().cloned().collect::<Vec<_>>() }))
        .map_err(|error| error.message())
}

fn validate_candidate(candidate: &UpdateCandidateDocument) -> Result<(), ()> {
    let package_binding_valid = match candidate.version {
        1 => candidate.package_sha256.is_none() && candidate.package_size_bytes.is_none(),
        CANDIDATE_VERSION => {
            candidate
                .package_sha256
                .as_deref()
                .is_some_and(valid_package_sha256)
                && candidate
                    .package_size_bytes
                    .is_some_and(|size| (1..=MAX_PACKAGE_BYTES).contains(&size))
        }
        _ => false,
    };
    if !package_binding_valid
        || !valid_identifier(&candidate.update_id)
        || candidate.current_version.trim().is_empty()
        || candidate.offered_version.trim().is_empty()
        || candidate.current_version.len() > 128
        || candidate.offered_version.len() > 128
        || !["stable", "beta", "nightly"].contains(&candidate.channel.as_str())
        || candidate.summary.trim().is_empty()
        || candidate.summary.chars().count() > 1024
        || !valid_hex(&candidate.signed_bytes_hex)
        || candidate.signed_bytes_hex.len() > MAX_MANIFEST_BYTES * 2
        || !valid_hex(&candidate.signature_hex)
        || candidate.signature_hex.len() != 128
        || !valid_hash(&candidate.signer_fingerprint)
    {
        return Err(());
    }
    Ok(())
}

fn valid_package_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn validate_record(record: &UpdateRecord) -> Result<(), String> {
    validate_candidate(&record.candidate).map_err(|_| "update recordの候補が不正".to_string())?;
    if record.signature_status != "verified" || !valid_hash(&record.candidate_hash) {
        return Err("update recordの署名状態または候補hashが不正".to_string());
    }
    if record.deferred_until.as_deref().is_some_and(str::is_empty) {
        return Err("update recordの延期期限が空である".to_string());
    }
    Ok(())
}

pub(super) fn validate_trust(trust: &UpdateTrust) -> Result<(), String> {
    let mut channels = BTreeSet::new();
    let sources_valid = match trust.version {
        1 => trust.package_sources.is_empty(),
        TRUST_VERSION => {
            trust.package_sources.len() <= 3
                && trust.package_sources.iter().all(|source| {
                    ["stable", "beta", "nightly"].contains(&source.channel.as_str())
                        && channels.insert(source.channel.as_str())
                        && valid_update_base_url(&source.base_url)
                })
        }
        _ => false,
    };
    let key =
        hex::decode(&trust.public_key_der_hex).map_err(|_| "更新公開鍵hexが不正".to_string())?;
    let public_key =
        crate::checkpoint::public_key(&key).map_err(|_| "更新公開鍵DERが不正".to_string())?;
    if !sources_valid
        || trust.algorithm != "Ed25519"
        || trust.public_key_fingerprint != sha256_tagged(public_key)
        || !valid_hash(&trust.public_key_fingerprint)
    {
        return Err("更新署名trustまたはpackage source設定が不正".to_string());
    }
    Ok(())
}

fn valid_update_base_url(value: &str) -> bool {
    if value.len() > 2048
        || !value.starts_with("https://")
        || value
            .bytes()
            .any(|byte| matches!(byte, b'?' | b'#' | b'@' | b'\\' | b'%') || !byte.is_ascii())
    {
        return false;
    }
    let Some((host, path)) = value[8..].split_once('/') else {
        return false;
    };
    if host.is_empty()
        || host.bytes().any(|byte| byte.is_ascii_uppercase())
        || host.contains(':')
        || host.ends_with('.')
        || host.parse::<std::net::IpAddr>().is_ok()
        || host == "localhost"
        || host.ends_with(".localhost")
    {
        return false;
    }
    let labels = host.split('.').collect::<Vec<_>>();
    if labels.len() < 2
        || host.len() > 253
        || labels.iter().any(|label| {
            label.is_empty()
                || label.len() > 63
                || (!label.as_bytes()[0].is_ascii_lowercase()
                    && !label.as_bytes()[0].is_ascii_digit())
                || (!label.as_bytes()[label.len() - 1].is_ascii_lowercase()
                    && !label.as_bytes()[label.len() - 1].is_ascii_digit())
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
    {
        return false;
    }
    if path.is_empty() || path.ends_with('/') {
        return false;
    }
    path.split('/').all(|segment| {
        !segment.is_empty()
            && segment != "."
            && segment != ".."
            && segment.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
            })
    })
}

pub(super) fn verify_bytes_with_trust(
    trust: Option<&UpdateTrust>,
    update_id: &str,
    signed_bytes_hex: &str,
    signature_hex: &str,
    signer_fingerprint: &str,
) -> Result<(), &'static str> {
    let Some(trust) = trust else {
        return Err("broker所有の署名信頼設定が未構成");
    };
    let public_key_der =
        hex::decode(&trust.public_key_der_hex).map_err(|_| "broker所有の署名公開鍵が不正")?;
    let signed_bytes = hex::decode(signed_bytes_hex).map_err(|_| "署名対象hexが不正")?;
    let signature = hex::decode(signature_hex).map_err(|_| "署名hexが不正")?;
    let result = verify_signed_update_signature(
        &SignedUpdateCandidate {
            update_id: update_id.to_string(),
            signed_bytes,
            signature: Some(signature),
            signer_fingerprint: signer_fingerprint.to_string(),
        },
        &public_key_der,
        &trust.public_key_fingerprint,
    );
    if result.ok {
        Ok(())
    } else {
        Err("Adapter署名の検証に失敗")
    }
}

fn accepted(
    broker: &mut Broker,
    operation: &str,
    request_id: &str,
    body: Value,
    hash: &str,
    reason: &str,
) -> BrokerResponse {
    let event = match broker.append_audit(
        request_id,
        operation,
        "accepted",
        reason,
        EVIDENCE_SOURCE_INTERNAL_STATE,
        hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                operation,
                "update_audit_append_failed",
                "更新操作の監査を確定できない",
            )
        }
    };
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: operation.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
        audit_event_id: event.event_id,
        error: None,
        health: None,
        body: Some(body),
        shutdown_requested: broker.shutdown_requested,
    }
}

fn accepted_external(
    broker: &mut Broker,
    operation: &str,
    request_id: &str,
    body: Value,
    hash: &str,
    reason: &str,
) -> BrokerResponse {
    let event = match broker.append_audit(
        request_id,
        operation,
        "accepted",
        reason,
        "EXTERNAL_EVIDENCE",
        hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                operation,
                "update_audit_append_failed",
                "更新操作の監査を確定できない",
            )
        }
    };
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: operation.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: "EXTERNAL_EVIDENCE".to_string(),
        audit_event_id: event.event_id,
        error: None,
        health: None,
        body: Some(body),
        shutdown_requested: broker.shutdown_requested,
    }
}

fn suspended(
    broker: &mut Broker,
    operation: &str,
    request_id: &str,
    body: Value,
    hash: &str,
    reason: &str,
) -> BrokerResponse {
    let event = match broker.append_audit(
        request_id,
        operation,
        "suspended",
        reason,
        EVIDENCE_SOURCE_INTERNAL_STATE,
        hash,
    ) {
        Ok(event) => event,
        Err(_) => {
            return broker.audit_store_failed_response(
                request_id,
                operation,
                "update_audit_append_failed",
                "更新操作の監査を確定できない",
            )
        }
    };
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: operation.to_string(),
        status: BrokerStatus::Suspended,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
        audit_event_id: event.event_id,
        error: Some(BrokerError {
            code: "update_execution_suspended".to_string(),
            message: reason.to_string(),
            recoverable: true,
            audit_event_required: true,
            fail_closed: true,
        }),
        health: None,
        body: Some(body),
        shutdown_requested: broker.shutdown_requested,
    }
}

fn reject(
    broker: &mut Broker,
    request_id: &str,
    operation: &str,
    code: &str,
    reason: &str,
    hash: &str,
) -> BrokerResponse {
    broker.reject_with_payload_hash(request_id, operation, code, reason, true, hash)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn valid_hex(value: &str) -> bool {
    !value.is_empty()
        && value.len().is_multiple_of(2)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_hash(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::protocol::{
        Broker, BrokerOperation, BrokerRequestEnvelope, BrokerStatus, OwnerConfirmationSource,
    };
    use ring::{
        rand::SystemRandom,
        signature::{Ed25519KeyPair, KeyPair},
    };
    use sha2::{Digest, Sha256};
    use std::io::Write;

    fn trust_and_candidate() -> (UpdateTrust, UpdateCandidateDocument) {
        let key = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        let pair = Ed25519KeyPair::from_pkcs8(key.as_ref()).unwrap();
        let der = [
            0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
        ]
        .into_iter()
        .chain(pair.public_key().as_ref().iter().copied())
        .collect::<Vec<_>>();
        let signed = serde_json::to_vec(&SignedManifest {
            version: CANDIDATE_VERSION,
            update_id: "update-1",
            current_version: "1.0.0",
            offered_version: "1.1.0",
            channel: "stable",
            summary: "安全更新",
            package_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            package_size_bytes: 1024,
            rollback_available: true,
        })
        .unwrap();
        let fingerprint = sha256_tagged(pair.public_key().as_ref());
        let signature = pair.sign(&signed);
        (
            UpdateTrust {
                version: TRUST_VERSION,
                algorithm: "Ed25519".into(),
                public_key_der_hex: hex::encode(der),
                public_key_fingerprint: fingerprint.clone(),
                package_sources: Vec::new(),
            },
            UpdateCandidateDocument {
                version: CANDIDATE_VERSION,
                update_id: "update-1".into(),
                current_version: "1.0.0".into(),
                offered_version: "1.1.0".into(),
                channel: "stable".into(),
                summary: "安全更新".into(),
                package_sha256: Some(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                ),
                package_size_bytes: Some(1024),
                signed_bytes_hex: hex::encode(signed),
                signature_hex: hex::encode(signature.as_ref()),
                signer_fingerprint: fingerprint,
                rollback_available: true,
            },
        )
    }

    const INSTALL_APP_ID: &str = "d4-pocket-app-11111111111111111111111111111111";
    const INSTALL_AUDIT_ID: &str = "audit-store-22222222222222222222222222222222";

    fn install_test_package() -> Vec<u8> {
        install_test_package_for_version("1.1.0")
    }

    fn install_test_package_for_version(product_version: &str) -> Vec<u8> {
        let product_manifest = serde_json::to_vec(&json!({
            "version": 1,
            "product": "D4 Pocket",
            "export_id": "export-install-test",
            "manifest": {
                "app_identity": {"app_id": INSTALL_APP_ID},
                "audit_store": {
                    "store_id": INSTALL_AUDIT_ID,
                    "chain_status": "new",
                    "inherited": false
                },
                "inheritance_policy": {
                    "authority": "none",
                    "permission": "none",
                    "approval": "none",
                    "credential": "none",
                    "audit_chain": "none"
                }
            }
        }))
        .unwrap();
        let files: Vec<(&str, &[u8])> = vec![
            ("app/data/app.so", b"a"),
            ("app/data/icudtl.dat", b"b"),
            ("app/flutter_windows.dll", b"c"),
            ("app/gui_shell_desktop.exe", b"d"),
            ("broker/gui_shell_rust_helper.exe", b"e"),
            ("gui_shell_desktop_launcher.exe", b"f"),
            ("product_manifest.json", &product_manifest),
        ];
        let entries: Vec<_> = files
            .iter()
            .map(|(path, bytes)| {
                format!(
                    r#"{{"path":"{path}","byte_length":{},"sha256":"{}"}}"#,
                    bytes.len(),
                    hex::encode(Sha256::digest(bytes)),
                )
            })
            .collect();
        let manifest = format!(
            r#"{{"version":1,"product":"D4 Pocket","product_version":"{product_version}","app_id":"{INSTALL_APP_ID}","audit_store_id":"{INSTALL_AUDIT_ID}","files":[{}]}}"#,
            entries.join(","),
        )
        .into_bytes();
        let mut package = b"D4PKG01\n".to_vec();
        package.extend_from_slice(&(manifest.len() as u32).to_le_bytes());
        package.extend_from_slice(&manifest);
        for (_, bytes) in files {
            package.extend_from_slice(bytes);
        }
        package
    }

    fn trust_and_candidate_pair_for_packages(
        current_package: &[u8],
        previous_package: &[u8],
    ) -> (
        UpdateTrust,
        UpdateCandidateDocument,
        UpdateCandidateDocument,
    ) {
        let key = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        let pair = Ed25519KeyPair::from_pkcs8(key.as_ref()).unwrap();
        let der = [
            0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
        ]
        .into_iter()
        .chain(pair.public_key().as_ref().iter().copied())
        .collect::<Vec<_>>();
        let fingerprint = sha256_tagged(pair.public_key().as_ref());
        let sign_candidate = |package: &[u8],
                              update_id: &'static str,
                              current_version: &'static str,
                              offered_version: &'static str,
                              summary: &'static str| {
            let digest = hex::encode(Sha256::digest(package));
            let signed = serde_json::to_vec(&SignedManifest {
                version: CANDIDATE_VERSION,
                update_id,
                current_version,
                offered_version,
                channel: "stable",
                summary,
                package_sha256: &digest,
                package_size_bytes: package.len() as u64,
                rollback_available: true,
            })
            .unwrap();
            let signature = pair.sign(&signed);
            UpdateCandidateDocument {
                version: CANDIDATE_VERSION,
                update_id: update_id.into(),
                current_version: current_version.into(),
                offered_version: offered_version.into(),
                channel: "stable".into(),
                summary: summary.into(),
                package_sha256: Some(digest),
                package_size_bytes: Some(package.len() as u64),
                signed_bytes_hex: hex::encode(signed),
                signature_hex: hex::encode(signature.as_ref()),
                signer_fingerprint: fingerprint.clone(),
                rollback_available: true,
            }
        };
        (
            UpdateTrust {
                version: TRUST_VERSION,
                algorithm: "Ed25519".into(),
                public_key_der_hex: hex::encode(der),
                public_key_fingerprint: fingerprint.clone(),
                package_sources: Vec::new(),
            },
            sign_candidate(
                current_package,
                "update-apply",
                "1.0.0",
                "1.1.0",
                "安全更新",
            ),
            sign_candidate(
                previous_package,
                "update-previous",
                "0.9.0",
                "1.0.0",
                "直前版",
            ),
        )
    }

    fn configured_product_trust() -> UpdateTrust {
        let (mut trust, _) = trust_and_candidate();
        trust.package_sources = vec![UpdatePackageSource {
            channel: "stable".into(),
            base_url: "https://updates.example.invalid/d4/stable".into(),
        }];
        trust
    }

    #[test]
    fn product_build_uses_embedded_update_trust_and_never_loads_editable_store_trust() {
        let embedded = configured_product_trust();
        let persisted = configured_product_trust();
        assert_ne!(embedded, persisted);
        let embedded_json = serde_json::to_string(&embedded).unwrap();
        let mut persistent_store_read = false;
        let resolved = resolve_build_update_trust(
            Some(INSTALL_APP_ID),
            Some(INSTALL_AUDIT_ID),
            Some(&embedded_json),
            || {
                persistent_store_read = true;
                Ok(Some(persisted))
            },
        )
        .unwrap();
        assert_eq!(resolved, Some(embedded));
        assert!(!persistent_store_read);
    }

    #[test]
    fn product_build_without_embedded_trust_ignores_persistent_trust() {
        let mut persistent_store_read = false;
        let resolved = resolve_build_update_trust(
            Some(INSTALL_APP_ID),
            Some(INSTALL_AUDIT_ID),
            None,
            || {
                persistent_store_read = true;
                Ok(Some(configured_product_trust()))
            },
        )
        .unwrap();
        assert_eq!(resolved, None);
        assert!(!persistent_store_read);
    }

    #[test]
    fn generic_broker_keeps_persistent_update_trust_compatibility() {
        let persisted = configured_product_trust();
        let resolved =
            resolve_build_update_trust(None, None, None, || Ok(Some(persisted.clone()))).unwrap();
        assert_eq!(resolved, Some(persisted));
    }

    #[test]
    fn malformed_or_incomplete_product_update_trust_fails_closed() {
        let valid_json = serde_json::to_string(&configured_product_trust()).unwrap();
        for (app_id, audit_id, embedded_json) in [
            (Some(INSTALL_APP_ID), None, None),
            (Some("invalid-app-id"), Some(INSTALL_AUDIT_ID), None),
            (Some(INSTALL_APP_ID), Some(INSTALL_AUDIT_ID), Some("{")),
            (Some(INSTALL_APP_ID), Some(INSTALL_AUDIT_ID), Some("{}")),
        ] {
            assert!(
                resolve_build_update_trust(app_id, audit_id, embedded_json, || Ok(None)).is_err()
            );
        }

        let mut no_source_trust = configured_product_trust();
        no_source_trust.package_sources.clear();
        let no_source_json = serde_json::to_string(&no_source_trust).unwrap();
        assert!(resolve_build_update_trust(
            Some(INSTALL_APP_ID),
            Some(INSTALL_AUDIT_ID),
            Some(&no_source_json),
            || Ok(None)
        )
        .is_err());
        assert!(resolve_build_update_trust(None, None, Some(&valid_json), || Ok(None)).is_err());
    }

    #[test]
    fn package_source_requires_canonical_https_base_and_unique_known_channels() {
        let (mut trust, _) = trust_and_candidate();
        trust.package_sources = vec![UpdatePackageSource {
            channel: "stable".into(),
            base_url: "https://updates.example.invalid/d4/stable".into(),
        }];
        assert!(validate_trust(&trust).is_ok());

        let (mut legacy_trust, _) = trust_and_candidate();
        legacy_trust.version = 1;
        assert!(validate_trust(&legacy_trust).is_ok());

        for base_url in [
            "http://updates.example.invalid/d4/stable",
            "https://user@updates.example.invalid/d4/stable",
            "https://updates.example.invalid:443/d4/stable",
            "https://updates.example.invalid/d4/stable?next=elsewhere",
            "https://updates.example.invalid/d4/stable#fragment",
            "https://updates.example.invalid/d4/../stable",
            "https://updates.example.invalid/d4/%2e%2e/stable",
            "https://127.0.0.1/d4/stable",
            "https://[::1]/d4/stable",
            "https://localhost/d4/stable",
            "https://updates.localhost/d4/stable",
            "https://updates.example.invalid/d4//stable",
            "https://updates.example.invalid/d4/stable/",
            "https://Updates.example.invalid/d4/stable",
        ] {
            trust.package_sources[0].base_url = base_url.into();
            assert!(validate_trust(&trust).is_err(), "{base_url}");
        }

        trust.package_sources = vec![
            UpdatePackageSource {
                channel: "stable".into(),
                base_url: "https://updates.example.invalid/d4/stable".into(),
            },
            UpdatePackageSource {
                channel: "stable".into(),
                base_url: "https://mirror.example.invalid/d4/stable".into(),
            },
        ];
        assert!(validate_trust(&trust).is_err());
        trust.version = 1;
        assert!(validate_trust(&trust).is_err());
    }

    #[test]
    fn broker_loads_v2_package_sources_from_its_bounded_persistent_trust() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "gui-shell-update-trust-v2-{}-{unique}",
            std::process::id()
        ));
        let (store, _) = BrokerPersistentStore::open_or_create(&root, "update-trust-v2-test")
            .expect("永続storeを初期化する");
        let (mut trust, _) = trust_and_candidate();
        trust.version = 1;
        let trust_path = root.join("update_trust.json");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&trust_path)
            .unwrap();
        file.write_all(&serde_json::to_vec(&trust).unwrap())
            .unwrap();
        drop(file);
        assert_eq!(load_persistent_trust(&store).unwrap(), Some(trust.clone()));

        trust.version = TRUST_VERSION;
        trust.package_sources = vec![UpdatePackageSource {
            channel: "stable".into(),
            base_url: "https://updates.example.invalid/d4/stable".into(),
        }];
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&trust_path)
            .unwrap();
        file.write_all(&serde_json::to_vec(&trust).unwrap())
            .unwrap();
        drop(file);

        assert_eq!(load_persistent_trust(&store).unwrap(), Some(trust));
        let _ = std::fs::remove_dir_all(root);
    }

    fn call(broker: &mut Broker, operation: BrokerOperation, payload: Value) -> BrokerResponse {
        let mut request = BrokerRequestEnvelope::command_envelope_at(
            "update-test",
            "session-1",
            &format!("nonce-{}", broker.audit_events().len()),
            &BrokerRequestEnvelope::current_issued_at(),
        );
        request.operation = Some(operation);
        request.payload = Some(payload);
        request.refresh_payload_hash();
        broker.handle(request)
    }

    #[cfg(windows)]
    #[test]
    fn product_launch_repair_restores_only_missing_entries_after_current_trust_and_owner_confirmation(
    ) {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "d4p-product-repair-{}-{unique}",
            std::process::id()
        ));
        let store_root = root.join("broker-store");
        let install_root = root.join("local-app-data");
        let start_menu_root = root.join("start-menu");
        std::fs::create_dir_all(&install_root).unwrap();
        std::fs::create_dir_all(&start_menu_root).unwrap();
        let install_root = std::fs::canonicalize(&install_root).unwrap();
        let start_menu_root = std::fs::canonicalize(&start_menu_root).unwrap();
        let package = install_test_package();
        let digest = hex::encode(Sha256::digest(&package));
        let (trust, candidate, _) = trust_and_candidate_pair_for_packages(&package, &package);
        let candidate_value = serde_json::to_value(&candidate).unwrap();
        let candidate_hash =
            crate::broker::protocol::canonical_payload_hash(Some(&candidate_value));
        let mut broker = Broker::new_persistent("session-1", &store_root).unwrap();
        broker.update_trust = Some(trust);
        broker
            .set_desktop_product_identity(INSTALL_APP_ID.into(), INSTALL_AUDIT_ID.into())
            .unwrap();
        broker.set_desktop_product_install_local_app_data(install_root.clone());
        broker.set_desktop_product_start_menu_directory(start_menu_root.clone());
        assert_eq!(
            call(
                &mut broker,
                BrokerOperation::更新確認,
                json!({"版": 1, "候補": [candidate]})
            )
            .status,
            BrokerStatus::Accepted
        );

        let (_, versions) = super::super::product_install::open_product_versions_directory(
            &install_root,
            INSTALL_APP_ID,
        )
        .unwrap();
        let stage_name = format!("1.1.0-{digest}");
        crate::product_package::extract_verified_package_into_directory(
            std::io::Cursor::new(package.as_slice()),
            package.len() as u64,
            Some(&digest),
            &versions,
            &stage_name,
            crate::product_package::ProductPackageExpectation {
                product_version: Some("1.1.0"),
                app_id: INSTALL_APP_ID,
                audit_store_id: INSTALL_AUDIT_ID,
            },
        )
        .unwrap();
        let (product_root_path, product_root) =
            super::super::product_install::open_existing_product_root(
                &install_root,
                INSTALL_APP_ID,
            )
            .unwrap();
        crate::product_bootstrapper::activate_staged_version(
            &product_root,
            &versions,
            "1.1.0",
            INSTALL_APP_ID,
            INSTALL_AUDIT_ID,
            &digest,
            &candidate.update_id,
            &candidate_hash,
            "0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        drop(versions);
        drop(product_root);

        let payload = json!({"版": VERSION});
        let payload_hash = crate::broker::protocol::canonical_payload_hash(Some(&payload));
        let active = crate::product_bootstrapper::active_version_snapshot(
            &install_root,
            INSTALL_APP_ID,
            INSTALL_AUDIT_ID,
        )
        .unwrap()
        .current;
        let active_record_bytes =
            std::fs::read(product_root_path.join("active_version.json")).unwrap();
        let shortcut = super::super::product_install::plan_start_menu_shortcut(
            &start_menu_root,
            INSTALL_APP_ID,
        )
        .unwrap();
        let mut confirmation = ProductRepairConfirmation {
            app_id: INSTALL_APP_ID.into(),
            audit_store_id: INSTALL_AUDIT_ID.into(),
            update_id: active.update_id.clone(),
            candidate_hash: active.candidate_hash.clone(),
            product_version: active.product_version.clone(),
            package_sha256: active.package_sha256.clone(),
            product_root: product_root_path.clone(),
            start_menu_shortcut_path: shortcut.clone(),
            payload_hash: payload_hash.clone(),
        };
        let bootstrapper = product_root_path.join("gui_shell_desktop_launcher.exe");
        std::fs::remove_file(&bootstrapper).unwrap();

        let normal_ipc = call(
            &mut broker,
            BrokerOperation::製品起動項目修復要求,
            payload.clone(),
        );
        assert_eq!(normal_ipc.status, BrokerStatus::Rejected);
        assert_eq!(
            normal_ipc.error.unwrap().code,
            "desktop_native_owner_confirmation_required"
        );
        assert!(!bootstrapper.exists());

        confirmation.product_root = root.join("attacker-controlled");
        let stale = repair_product_launch_entries(
            &mut broker,
            &payload,
            "product-repair-stale",
            &payload_hash,
            OwnerConfirmationSource::DesktopNativeConfirmation,
            Some(&confirmation),
        );
        assert_eq!(stale.status, BrokerStatus::Rejected);
        assert_eq!(
            stale.error.unwrap().code,
            "product_repair_confirmation_stale"
        );
        assert!(!bootstrapper.exists());

        confirmation.product_root = product_root_path.clone();
        let staged_payload = install_root
            .join("Programs")
            .join("D4 Pocket")
            .join(INSTALL_APP_ID)
            .join("versions")
            .join(&stage_name)
            .join("app/data/app.so");
        let staged_payload_bytes = std::fs::read(&staged_payload).unwrap();
        std::fs::write(&staged_payload, b"tampered installed payload").unwrap();
        let tampered_stage = repair_product_launch_entries(
            &mut broker,
            &payload,
            "product-repair-tampered-stage",
            &payload_hash,
            OwnerConfirmationSource::DesktopNativeConfirmation,
            Some(&confirmation),
        );
        assert_eq!(tampered_stage.status, BrokerStatus::Rejected);
        assert_eq!(
            tampered_stage.error.unwrap().code,
            "active_version_package_mismatch"
        );
        assert!(!bootstrapper.exists());
        assert!(!shortcut.exists());
        std::fs::write(&staged_payload, staged_payload_bytes).unwrap();

        let repaired = repair_product_launch_entries(
            &mut broker,
            &payload,
            "product-repair-current",
            &payload_hash,
            OwnerConfirmationSource::DesktopNativeConfirmation,
            Some(&confirmation),
        );
        assert_eq!(repaired.status, BrokerStatus::Accepted);
        assert_eq!(repaired.evidence_source, "LIVE_RUNTIME");
        assert_eq!(repaired.body.as_ref().unwrap()["起動器復元"], true);
        assert_eq!(repaired.body.as_ref().unwrap()["Start Menu復元"], true);
        assert!(bootstrapper.is_file());
        assert!(shortcut.is_file());
        assert_eq!(
            std::fs::read(product_root_path.join("active_version.json")).unwrap(),
            active_record_bytes
        );
        assert!(broker.audit_events().iter().any(|event| {
            event.request_id == "product-repair-current:complete"
                && event.operation == OP_PRODUCT_REPAIR
                && event.decision == "completed"
                && event.evidence_source == "LIVE_RUNTIME"
        }));

        let unchanged = repair_product_launch_entries(
            &mut broker,
            &payload,
            "product-repair-idempotent",
            &payload_hash,
            OwnerConfirmationSource::DesktopNativeConfirmation,
            Some(&confirmation),
        );
        assert_eq!(unchanged.status, BrokerStatus::Accepted);
        assert_eq!(unchanged.body.as_ref().unwrap()["状態"], "unchanged");
        assert_eq!(unchanged.body.as_ref().unwrap()["起動器復元"], false);
        assert_eq!(unchanged.body.as_ref().unwrap()["Start Menu復元"], false);

        std::fs::remove_file(&bootstrapper).unwrap();
        std::fs::write(&bootstrapper, b"conflicting launcher").unwrap();
        let conflict = repair_product_launch_entries(
            &mut broker,
            &payload,
            "product-repair-conflict",
            &payload_hash,
            OwnerConfirmationSource::DesktopNativeConfirmation,
            Some(&confirmation),
        );
        assert_eq!(conflict.status, BrokerStatus::Rejected);
        assert_eq!(
            conflict.error.unwrap().code,
            "installed_bootstrapper_mismatch"
        );
        assert_eq!(
            std::fs::read(&bootstrapper).unwrap(),
            b"conflicting launcher"
        );
        drop(broker);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn product_uninstall_requires_owner_and_consumes_durable_ticket_once() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "d4p-uninstall-broker-{}-{unique}",
            std::process::id()
        ));
        let store_root = root.join("broker-store");
        let local_app_data = root.join("local-app-data");
        let start_menu = root.join("start-menu");
        let product_root = super::super::product_install::product_install_root(
            &local_app_data,
            INSTALL_APP_ID,
        )
        .unwrap();
        std::fs::create_dir_all(&product_root).unwrap();
        std::fs::write(
            product_root.join("gui_shell_desktop_launcher.exe"),
            b"synthetic bootstrapper",
        )
        .unwrap();
        std::fs::create_dir_all(&start_menu).unwrap();

        let mut broker = Broker::new_persistent("uninstall-test", &store_root).unwrap();
        broker
            .set_desktop_product_identity(INSTALL_APP_ID.into(), INSTALL_AUDIT_ID.into())
            .unwrap();
        broker.set_desktop_product_install_local_app_data(local_app_data.clone());
        broker.set_desktop_product_start_menu_directory(start_menu.clone());
        broker.set_desktop_setup_doctor_runtime_evidence(true, true, true);
        let payload = json!({"版": VERSION});
        let payload_hash = crate::broker::protocol::canonical_payload_hash(Some(&payload));
        let confirmation = ProductUninstallConfirmation {
            app_id: INSTALL_APP_ID.into(),
            audit_store_id: INSTALL_AUDIT_ID.into(),
            product_root,
            start_menu_shortcut_path: super::super::product_install::plan_start_menu_shortcut(
                &start_menu,
                INSTALL_APP_ID,
            )
            .unwrap(),
            payload_hash: payload_hash.clone(),
        };

        let denied = super::request_product_uninstall(
            &mut broker,
            &payload,
            "uninstall-owner-denied",
            &payload_hash,
            OwnerConfirmationSource::OwnerCredential,
            Some(&confirmation),
        );
        assert_eq!(denied.status, BrokerStatus::Rejected);
        assert_eq!(
            denied.error.as_ref().map(|error| error.code.as_str()),
            Some("desktop_native_owner_confirmation_required")
        );

        let first = super::request_product_uninstall(
            &mut broker,
            &payload,
            "uninstall-owner-accepted-1",
            &payload_hash,
            OwnerConfirmationSource::DesktopNativeConfirmation,
            Some(&confirmation),
        );
        assert_eq!(first.status, BrokerStatus::Accepted);
        let first_ticket = first.body.as_ref().unwrap()["ticket"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(first_ticket.len(), 64);
        let audit_json = serde_json::to_string(broker.audit_events()).unwrap();
        assert!(!audit_json.contains(&first_ticket));
        assert!(audit_json.contains(&sha256_tagged(first_ticket.as_bytes())));
        assert_eq!(
            broker.begin_product_uninstall_finalization("not-a-valid-ticket"),
            Err("product_uninstall_ticket_invalid")
        );
        assert_eq!(
            broker.begin_product_uninstall_finalization(&"b".repeat(64)),
            Err("product_uninstall_ticket_unrecognized")
        );
        let first_hash = sha256_tagged(first_ticket.as_bytes());
        assert_eq!(
            broker.finish_product_uninstall_finalization(&first_hash, true, None),
            Err("product_uninstall_ticket_not_pending")
        );
        assert_eq!(
            broker.begin_product_uninstall_finalization(&first_ticket),
            Ok(first_hash.clone())
        );
        assert_eq!(
            broker.finish_product_uninstall_finalization(
                &first_hash,
                false,
                Some("synthetic_remove_failure"),
            ),
            Ok(())
        );
        assert_eq!(
            broker.begin_product_uninstall_finalization(&first_ticket),
            Err("product_uninstall_ticket_replayed")
        );

        let second = super::request_product_uninstall(
            &mut broker,
            &payload,
            "uninstall-owner-accepted-2",
            &payload_hash,
            OwnerConfirmationSource::DesktopNativeConfirmation,
            Some(&confirmation),
        );
        assert_eq!(second.status, BrokerStatus::Accepted);
        let second_ticket = second.body.as_ref().unwrap()["ticket"]
            .as_str()
            .unwrap()
            .to_owned();
        broker
            .begin_product_uninstall_finalization(&second_ticket)
            .unwrap();

        drop(broker);
        let mut reopened = Broker::new_persistent("uninstall-test-reopen", &store_root).unwrap();
        reopened
            .set_desktop_product_identity(INSTALL_APP_ID.into(), INSTALL_AUDIT_ID.into())
            .unwrap();
        assert_eq!(
            reopened.begin_product_uninstall_finalization(&second_ticket),
            Err("product_uninstall_ticket_replayed")
        );
        reopened.set_desktop_product_install_local_app_data(local_app_data.clone());
        reopened.set_desktop_product_start_menu_directory(start_menu.clone());
        reopened.set_desktop_setup_doctor_runtime_evidence(true, true, true);
        let retry = super::request_product_uninstall(
            &mut reopened,
            &payload,
            "uninstall-owner-accepted-after-crash",
            &payload_hash,
            OwnerConfirmationSource::DesktopNativeConfirmation,
            Some(&confirmation),
        );
        assert_eq!(retry.status, BrokerStatus::Accepted);
        let retry_ticket = retry.body.as_ref().unwrap()["ticket"]
            .as_str()
            .unwrap()
            .to_owned();
        let retry_hash = reopened
            .begin_product_uninstall_finalization(&retry_ticket)
            .unwrap();
        assert_eq!(
            reopened.finish_product_uninstall_finalization(&retry_hash, true, None),
            Ok(())
        );

        drop(reopened);
        let mut verified = Broker::new_persistent("uninstall-test-final-reopen", &store_root).unwrap();
        verified
            .set_desktop_product_identity(INSTALL_APP_ID.into(), INSTALL_AUDIT_ID.into())
            .unwrap();
        assert_eq!(
            verified.begin_product_uninstall_finalization(&retry_ticket),
            Err("product_uninstall_ticket_replayed")
        );
        drop(verified);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn update_candidate_requires_broker_owned_trust() {
        let (_, candidate) = trust_and_candidate();
        let mut broker = Broker::new("session-1");
        let response = call(
            &mut broker,
            BrokerOperation::更新確認,
            json!({"版": 1, "候補": [candidate]}),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "update_trust_unconfigured");
    }

    #[test]
    fn trusted_catalog_fetch_persists_only_signature_verified_channel_candidates() {
        let (mut trust, candidate) = trust_and_candidate();
        trust.package_sources = vec![UpdatePackageSource {
            channel: "stable".into(),
            base_url: "https://updates.example.invalid/d4/stable".into(),
        }];
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        let bytes = serde_json::to_vec(&json!({"版": VERSION, "候補": [candidate]})).unwrap();
        let request_hash = sha256_tagged(b"update-catalog-fetch");
        let response = fetch_catalog_with(
            &mut broker,
            &json!({"版": VERSION}),
            "catalog-fetch-test",
            &request_hash,
            |url, _deadline| {
                assert_eq!(
                    url,
                    "https://updates.example.invalid/d4/stable/updates.json"
                );
                Ok(bytes.clone())
            },
        );

        assert_eq!(response.status, BrokerStatus::Accepted);
        assert_eq!(response.evidence_source, "EXTERNAL_EVIDENCE");
        assert_eq!(response.body.as_ref().unwrap()["候補件数"], 1);
        assert_eq!(broker.updates.len(), 1);
        assert_eq!(broker.updates["update-1"]["署名状態"], "verified");
        let event = broker.audit_events().last().unwrap();
        assert_eq!(event.evidence_source, "EXTERNAL_EVIDENCE");
        assert!(event.reason.contains("catalog_hashes=sha256:"));
        assert!(event.reason.contains("Capability=更新候補取得"));
        assert!(event
            .reason
            .contains("Permission=Broker所有trust.package_sources"));
        assert!(event.reason.contains("Approval=明示操作"));
        assert!(event
            .reason
            .contains("RecoveryAction=失敗時は既存候補を保持"));
    }

    #[test]
    fn invalid_catalog_candidate_does_not_replace_existing_verified_candidate() {
        let (mut trust, candidate) = trust_and_candidate();
        trust.package_sources = vec![UpdatePackageSource {
            channel: "stable".into(),
            base_url: "https://updates.example.invalid/d4/stable".into(),
        }];
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        let existing = call(
            &mut broker,
            BrokerOperation::更新確認,
            json!({"版": VERSION, "候補": [candidate.clone()]}),
        );
        assert_eq!(existing.status, BrokerStatus::Accepted);
        let previous = broker.updates.clone();

        let mut forged = candidate.clone();
        forged.signature_hex = "00".repeat(64);
        let bytes = serde_json::to_vec(&json!({"版": VERSION, "候補": [forged]})).unwrap();
        let response = fetch_catalog_with(
            &mut broker,
            &json!({"版": VERSION}),
            "catalog-forged-test",
            &sha256_tagged(b"update-catalog-forged"),
            |_, _deadline| Ok(bytes.clone()),
        );

        assert_eq!(response.status, BrokerStatus::Rejected);
        let error = response.error.unwrap();
        assert_eq!(error.code, "update_signature_invalid");
        assert!(error
            .message
            .contains("RecoveryAction=失敗時は既存候補を保持"));
        assert_eq!(broker.updates, previous);
        assert_eq!(
            broker.audit_events().last().unwrap().reason,
            "update_signature_invalid"
        );

        let mut mismatched_channel = candidate;
        mismatched_channel.channel = "beta".into();
        let bytes =
            serde_json::to_vec(&json!({"版": VERSION, "候補": [mismatched_channel]})).unwrap();
        let response = fetch_catalog_with(
            &mut broker,
            &json!({"版": VERSION}),
            "catalog-channel-mismatch-test",
            &sha256_tagged(b"update-catalog-channel-mismatch"),
            |_, _deadline| Ok(bytes.clone()),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "update_catalog_candidate_conflict"
        );
        assert_eq!(broker.updates, previous);
    }

    #[test]
    fn all_catalog_sources_share_one_operation_deadline() {
        let (mut trust, candidate) = trust_and_candidate();
        trust.package_sources = vec![
            UpdatePackageSource {
                channel: "stable".into(),
                base_url: "https://updates.example.invalid/d4/stable".into(),
            },
            UpdatePackageSource {
                channel: "beta".into(),
                base_url: "https://updates.example.invalid/d4/beta".into(),
            },
        ];
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        let bytes = serde_json::to_vec(&json!({"版": VERSION, "候補": [candidate]})).unwrap();
        let mut seen_deadline = None;
        let response = fetch_catalog_with(
            &mut broker,
            &json!({"版": VERSION}),
            "catalog-shared-deadline-test",
            &sha256_tagged(b"update-catalog-shared-deadline"),
            |url, deadline| {
                if let Some(first) = seen_deadline {
                    assert_eq!(deadline, first);
                } else {
                    seen_deadline = Some(deadline);
                }
                if url.ends_with("/stable/updates.json") {
                    Ok(bytes.clone())
                } else {
                    Err(super::super::update_download::DownloadError::TimedOut)
                }
            },
        );

        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "update_catalog_timeout");
        assert!(broker.updates.is_empty());
    }

    #[test]
    fn catalog_fetch_without_configured_source_does_not_call_network() {
        let (trust, _) = trust_and_candidate();
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        let response = fetch_catalog_with(
            &mut broker,
            &json!({"版": VERSION}),
            "catalog-unconfigured-test",
            &sha256_tagged(b"update-catalog-unconfigured"),
            |_, _deadline| panic!("未構成配布元では通信しない"),
        );
        assert_eq!(response.status, BrokerStatus::Accepted);
        assert_eq!(response.body.as_ref().unwrap()["状態"], "unconfigured");
    }

    #[test]
    fn update_list_derives_package_url_only_from_current_candidate_and_broker_trust() {
        let (mut trust, candidate) = trust_and_candidate();
        trust.package_sources = vec![UpdatePackageSource {
            channel: "stable".into(),
            base_url: "https://updates.example.invalid/d4/stable".into(),
        }];
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust.clone());
        let checked = call(
            &mut broker,
            BrokerOperation::更新確認,
            json!({"版": 1, "候補": [candidate]}),
        );
        assert_eq!(checked.status, BrokerStatus::Accepted);

        let stored_hash = broker.updates["update-1"]["候補hash"].clone();
        let forged_source = call(
            &mut broker,
            BrokerOperation::更新download要求,
            json!({
                "版": 1,
                "更新ID": "update-1",
                "候補hash": stored_hash,
                "URL": "https://attacker.example.invalid/forged.pkg"
            }),
        );
        assert_eq!(forged_source.status, BrokerStatus::Rejected);
        assert_eq!(forged_source.error.unwrap().code, "update_request_invalid");

        let listed = call(&mut broker, BrokerOperation::更新一覧, json!({"版": 1}));
        let body = listed.body.expect("更新一覧");
        assert_eq!(body["download実行"], "suspended");
        assert_eq!(
            body["更新一覧"][0]["取得元"],
            json!({
                "状態": "configured",
                "URL": "https://updates.example.invalid/d4/stable/update-1.pkg"
            })
        );

        let (mut no_source_trust, second_candidate) = trust_and_candidate();
        no_source_trust.package_sources.clear();
        let mut no_source_broker = Broker::new("session-1");
        no_source_broker.update_trust = Some(no_source_trust);
        let checked = call(
            &mut no_source_broker,
            BrokerOperation::更新確認,
            json!({"版": 1, "候補": [second_candidate]}),
        );
        assert_eq!(checked.status, BrokerStatus::Accepted);
        let listed = call(
            &mut no_source_broker,
            BrokerOperation::更新一覧,
            json!({"版": 1}),
        );
        assert_eq!(
            listed.body.unwrap()["更新一覧"][0]["取得元"],
            json!({"状態": "unconfigured", "URL": null})
        );

        no_source_broker.update_trust = None;
        let listed = call(
            &mut no_source_broker,
            BrokerOperation::更新一覧,
            json!({"版": 1}),
        );
        assert_eq!(
            listed.body.unwrap()["更新一覧"][0]["取得元"],
            json!({"状態": "ineligible", "URL": null})
        );
    }

    #[test]
    fn verified_candidate_is_persisted_only_after_signature_check() {
        let (trust, candidate) = trust_and_candidate();
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        let response = call(
            &mut broker,
            BrokerOperation::更新確認,
            json!({"版": 1, "候補": [candidate]}),
        );
        assert_eq!(response.status, BrokerStatus::Accepted);
        assert_eq!(broker.updates.len(), 1);
        let update = broker.updates.values().next().unwrap();
        assert_eq!(update["署名状態"], "verified");
        assert_eq!(
            update["package_sha256"],
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(update["package_size_bytes"], 1024);
    }

    #[test]
    fn update_signature_binds_package_digest_and_exact_size() {
        let (trust, mut candidate) = trust_and_candidate();
        candidate.package_sha256 =
            Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into());
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        let response = call(
            &mut broker,
            BrokerOperation::更新確認,
            json!({"版": 1, "候補": [candidate]}),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "update_signed_manifest_mismatch"
        );
        assert!(broker.updates.is_empty());

        let (_, mut candidate) = trust_and_candidate();
        candidate.package_size_bytes = Some(MAX_PACKAGE_BYTES + 1);
        assert!(validate_candidate(&candidate).is_err());

        let (_, mut candidate) = trust_and_candidate();
        candidate.package_sha256 = Some("A".repeat(64));
        assert!(validate_candidate(&candidate).is_err());

        let (trust, mut candidate) = trust_and_candidate();
        candidate.package_size_bytes = Some(1025);
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        let response = call(
            &mut broker,
            BrokerOperation::更新確認,
            json!({"版": 1, "候補": [candidate]}),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "update_signed_manifest_mismatch"
        );
        assert!(broker.updates.is_empty());
    }

    #[test]
    fn legacy_candidate_cannot_be_accepted_as_a_new_update() {
        let (trust, mut candidate) = trust_and_candidate();
        candidate.version = 1;
        candidate.package_sha256 = None;
        candidate.package_size_bytes = None;
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        let response = call(
            &mut broker,
            BrokerOperation::更新確認,
            json!({"版": 1, "候補": [candidate]}),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "update_package_binding_required"
        );
        assert!(broker.updates.is_empty());
    }

    #[test]
    fn legacy_unbound_candidate_is_preserved_but_not_actionable() {
        let (_, mut candidate) = trust_and_candidate();
        candidate.version = 1;
        candidate.package_sha256 = None;
        candidate.package_size_bytes = None;
        let record = UpdateRecord {
            candidate,
            signature_status: "verified".into(),
            candidate_hash:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            deferred_until: None,
        };
        let record_value = serde_json::to_value(record).unwrap();
        let decoded = decode_state(&json!({"版": 1, "updates": [record_value.clone()]}));
        assert!(decoded.is_ok(), "旧状態はBroker起動を妨げず保持できる");

        let mut broker = Broker::new("session-1");
        broker.updates.insert("update-1".into(), record_value);
        let listing = call(&mut broker, BrokerOperation::更新一覧, json!({"版": 1}));
        assert_eq!(
            listing.body.unwrap()["更新一覧"][0]["署名状態"],
            "legacy_unbound"
        );
        let deferred = call(
            &mut broker,
            BrokerOperation::更新延期,
            json!({
                "版": 1,
                "更新ID": "update-1",
                "候補hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "延期期限": "2030-01-01T00:00:00Z"
            }),
        );
        assert_eq!(deferred.status, BrokerStatus::Accepted);
        assert_eq!(deferred.body.unwrap()["署名状態"], "legacy_unbound");
        let response = call(
            &mut broker,
            BrokerOperation::更新適用要求,
            json!({
                "版": 1,
                "更新ID": "update-1",
                "候補hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "update_package_binding_required"
        );
    }

    #[test]
    fn rollback_and_apply_reject_without_native_owner_confirmation() {
        let (trust, candidate) = trust_and_candidate();
        let candidate_value = serde_json::to_value(&candidate).unwrap();
        let candidate_hash =
            crate::broker::protocol::canonical_payload_hash(Some(&candidate_value));
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        assert_eq!(
            call(
                &mut broker,
                BrokerOperation::更新確認,
                json!({"版": 1, "候補": [candidate]})
            )
            .status,
            BrokerStatus::Accepted
        );
        let apply = call(
            &mut broker,
            BrokerOperation::更新適用要求,
            json!({"版": 1, "更新ID": "update-1", "候補hash": candidate_hash}),
        );
        assert_eq!(apply.status, BrokerStatus::Rejected);
        assert_eq!(
            apply.error.as_ref().map(|error| error.code.as_str()),
            Some("desktop_native_owner_confirmation_required")
        );
        let rollback = call(
            &mut broker,
            BrokerOperation::更新rollback要求,
            json!({"版": 1, "更新ID": "update-1", "候補hash": candidate_hash}),
        );
        assert_eq!(rollback.status, BrokerStatus::Rejected);
        assert_eq!(
            rollback.error.as_ref().map(|error| error.code.as_str()),
            Some("desktop_native_owner_confirmation_required")
        );
        let event = broker.audit_events().last().unwrap();
        assert_eq!(event.operation, OP_ROLLBACK);
        assert_eq!(event.decision, "rejected");
        assert_eq!(rollback.audit_event_id, event.event_id);
    }

    #[cfg(windows)]
    #[test]
    fn native_owner_apply_stages_exact_signed_package_without_activation_or_overwrite() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("d4p-update-apply-{}-{unique}", std::process::id()));
        let store_root = root.join("broker-store");
        let install_root = root.join("local-app-data");
        let start_menu_root = root.join("start-menu");
        std::fs::create_dir_all(&install_root).unwrap();
        std::fs::create_dir_all(&start_menu_root).unwrap();
        let install_root = std::fs::canonicalize(&install_root).unwrap();
        let start_menu_root = std::fs::canonicalize(&start_menu_root).unwrap();
        let package = install_test_package();
        let previous_package = install_test_package_for_version("1.0.0");
        let digest = hex::encode(Sha256::digest(&package));
        let previous_digest = hex::encode(Sha256::digest(&previous_package));
        let (trust, candidate, previous_candidate) =
            trust_and_candidate_pair_for_packages(&package, &previous_package);
        let candidate_value = serde_json::to_value(&candidate).unwrap();
        let candidate_hash =
            crate::broker::protocol::canonical_payload_hash(Some(&candidate_value));
        let previous_candidate_value = serde_json::to_value(&previous_candidate).unwrap();
        let previous_candidate_hash =
            crate::broker::protocol::canonical_payload_hash(Some(&previous_candidate_value));
        let mut broker = Broker::new_persistent("session-1", &store_root).unwrap();
        broker.update_trust = Some(trust);
        broker
            .set_desktop_product_identity(INSTALL_APP_ID.into(), INSTALL_AUDIT_ID.into())
            .unwrap();
        broker.set_desktop_product_install_local_app_data(install_root.clone());
        broker.set_desktop_product_start_menu_directory(start_menu_root.clone());
        let (previous_versions_path, previous_versions) =
            super::super::product_install::open_product_versions_directory(
                &install_root,
                INSTALL_APP_ID,
            )
            .unwrap();
        let previous_stage_name = format!("1.0.0-{previous_digest}");
        crate::product_package::extract_verified_package_into_directory(
            std::io::Cursor::new(previous_package.as_slice()),
            previous_package.len() as u64,
            Some(&previous_digest),
            &previous_versions,
            &previous_stage_name,
            crate::product_package::ProductPackageExpectation {
                product_version: Some("1.0.0"),
                app_id: INSTALL_APP_ID,
                audit_store_id: INSTALL_AUDIT_ID,
            },
        )
        .unwrap();
        let (previous_root_path, previous_product_root) =
            super::super::product_install::open_existing_product_root(
                &install_root,
                INSTALL_APP_ID,
            )
            .unwrap();
        assert_eq!(previous_root_path, previous_versions_path.parent().unwrap());
        crate::product_bootstrapper::activate_staged_version(
            &previous_product_root,
            &previous_versions,
            "1.0.0",
            INSTALL_APP_ID,
            INSTALL_AUDIT_ID,
            &previous_digest,
            "update-previous",
            &previous_candidate_hash,
            "11111111111111111111111111111111",
        )
        .unwrap();
        assert_eq!(
            call(
                &mut broker,
                BrokerOperation::更新確認,
                json!({"版": 1, "候補": [candidate, previous_candidate]})
            )
            .status,
            BrokerStatus::Accepted
        );
        let first_install_state = rollback_state_projection(&broker);
        assert_eq!(first_install_state["状態"], "unavailable");
        assert_eq!(first_install_state["現在版"]["提供版"], "1.0.0");
        assert!(first_install_state["対象版"].is_null());
        let package_directory = broker
            .state_store
            .open_update_package_directory()
            .unwrap()
            .unwrap();
        drop(package_directory);
        std::fs::write(
            store_root
                .join("update_packages")
                .join(format!("{digest}.pkg")),
            &package,
        )
        .unwrap();
        let payload = json!({
            "版": 1,
            "更新ID": "update-apply",
            "候補hash": candidate_hash,
        });
        let payload_hash = crate::broker::protocol::canonical_payload_hash(Some(&payload));
        let target = super::super::product_install::plan_product_install(
            &install_root,
            INSTALL_APP_ID,
            "1.1.0",
            &digest,
        )
        .unwrap()
        .version_directory;
        let confirmation = UpdateApplyConfirmation {
            download: UpdateDownloadConfirmation {
                update_id: "update-apply".into(),
                candidate_hash: payload["候補hash"].as_str().unwrap().into(),
                source_url: String::new(),
                package_sha256: digest.clone(),
                package_size_bytes: package.len() as u64,
                offered_version: "1.1.0".into(),
                channel: "stable".into(),
                summary: "安全更新".into(),
                display_host: String::new(),
                payload_hash: payload_hash.clone(),
            },
            app_id: INSTALL_APP_ID.into(),
            audit_store_id: INSTALL_AUDIT_ID.into(),
            version_directory: target.clone(),
        };
        let envelope = |request_id: &str| {
            json!({
                "request_id": request_id,
                "session_id": "session-1",
                "operation": OP_APPLY,
                "payload_hash": payload_hash,
                "nonce": format!("{request_id}-nonce"),
                "issued_at": BrokerRequestEnvelope::current_issued_at(),
                "metadata": {"client": "desktop_flutter"},
                "payload": payload,
            })
            .to_string()
        };
        let activation_confirmation = UpdateActivationConfirmation {
            update_id: "update-apply".into(),
            candidate_hash: payload["候補hash"].as_str().unwrap().into(),
            package_sha256: digest.clone(),
            package_size_bytes: package.len() as u64,
            offered_version: "1.1.0".into(),
            channel: "stable".into(),
            summary: "安全更新".into(),
            payload_hash: payload_hash.clone(),
            app_id: INSTALL_APP_ID.into(),
            audit_store_id: INSTALL_AUDIT_ID.into(),
            version_directory: target.clone(),
            start_menu_shortcut_path: super::super::product_install::plan_start_menu_shortcut(
                &start_menu_root,
                INSTALL_APP_ID,
            )
            .unwrap(),
            rollback_target: None,
        };
        let activation_envelope = |request_id: &str| {
            json!({
                "request_id": request_id,
                "session_id": "session-1",
                "operation": OP_ACTIVATE,
                "payload_hash": payload_hash,
                "nonce": format!("{request_id}-nonce"),
                "issued_at": BrokerRequestEnvelope::current_issued_at(),
                "metadata": {"client": "desktop_flutter"},
                "payload": payload,
            })
            .to_string()
        };
        let normal_ipc = call(&mut broker, BrokerOperation::更新適用要求, payload.clone());
        assert_eq!(normal_ipc.status, BrokerStatus::Rejected);
        assert_eq!(
            normal_ipc.error.unwrap().code,
            "desktop_native_owner_confirmation_required"
        );
        let mut stale_confirmation = confirmation.clone();
        stale_confirmation.version_directory = install_root.join("attacker-controlled");
        let stale = broker.desktop_owner_operation_json_with_update_confirmation(
            &envelope("desktop-update-apply-stale"),
            None,
            Some(stale_confirmation),
        );
        assert_eq!(stale.status, BrokerStatus::Rejected);
        assert_eq!(stale.error.unwrap().code, "update_apply_destination_stale");
        assert!(!target.exists());

        let mut stale_identity = confirmation.clone();
        stale_identity.app_id = "d4-pocket-app-33333333333333333333333333333333".into();
        let stale = broker.desktop_owner_operation_json_with_update_confirmation(
            &envelope("desktop-update-apply-identity-stale"),
            None,
            Some(stale_identity),
        );
        assert_eq!(stale.status, BrokerStatus::Rejected);
        assert_eq!(stale.error.unwrap().code, "update_apply_confirmation_stale");
        assert!(!target.exists());

        let package_path = store_root
            .join("update_packages")
            .join(format!("{digest}.pkg"));
        let mut tampered_package = package.clone();
        *tampered_package.last_mut().unwrap() ^= 1;
        std::fs::write(&package_path, tampered_package).unwrap();
        let tampered = broker.desktop_owner_operation_json_with_update_confirmation(
            &envelope("desktop-update-apply-tampered"),
            None,
            Some(confirmation.clone()),
        );
        assert_eq!(tampered.status, BrokerStatus::Rejected);
        assert!(!target.exists());
        assert!(broker.audit_events().iter().any(|event| {
            event.request_id == "desktop-update-apply-tampered:failed" && event.decision == "failed"
        }));
        std::fs::write(&package_path, &package).unwrap();
        std::fs::create_dir_all(target.join("app/data")).unwrap();
        std::fs::write(target.join("app/data/app.so"), b"").unwrap();

        let applied = broker.desktop_owner_operation_json_with_update_confirmation(
            &envelope("desktop-update-apply"),
            None,
            Some(confirmation.clone()),
        );
        assert_eq!(applied.status, BrokerStatus::Accepted);
        assert_eq!(applied.body.as_ref().unwrap()["導入状態"], "version_staged");
        assert_eq!(applied.body.as_ref().unwrap()["有効化"], "suspended");
        assert_eq!(applied.body.as_ref().unwrap()["再開"], true);
        assert_eq!(
            std::fs::read(target.join("app").join("gui_shell_desktop.exe")).unwrap(),
            b"d"
        );
        assert!(broker
            .audit_events()
            .iter()
            .any(|event| { event.operation == OP_APPLY && event.decision == "queued" }));
        assert!(broker.audit_events().iter().any(|event| {
            event.request_id == "desktop-update-apply:complete"
                && event.operation == OP_APPLY
                && event.decision == "completed"
                && event.reason.contains("resumed=true")
        }));

        let repeated = broker.desktop_owner_operation_json_with_update_confirmation(
            &envelope("desktop-update-apply-replay"),
            None,
            Some(confirmation.clone()),
        );
        assert_eq!(repeated.status, BrokerStatus::Accepted);
        assert_eq!(
            repeated.body.as_ref().unwrap()["導入状態"],
            "version_staged"
        );
        assert_eq!(repeated.body.as_ref().unwrap()["再開"], true);
        assert_eq!(
            std::fs::read(target.join("app").join("gui_shell_desktop.exe")).unwrap(),
            b"d"
        );

        let normal_activation = call(
            &mut broker,
            BrokerOperation::更新有効版切替要求,
            payload.clone(),
        );
        assert_eq!(normal_activation.status, BrokerStatus::Rejected);
        assert_eq!(
            normal_activation.error.unwrap().code,
            "desktop_native_owner_confirmation_required"
        );
        let apply_confirmation_does_not_authorize_activation = broker
            .desktop_owner_operation_json_with_update_confirmations(
                &activation_envelope("desktop-update-activation-no-reuse"),
                None,
                Some(confirmation.clone()),
                None,
            );
        assert_eq!(
            apply_confirmation_does_not_authorize_activation
                .error
                .unwrap()
                .code,
            "update_activation_confirmation_missing"
        );

        let mut stale_activation = activation_confirmation.clone();
        stale_activation.version_directory = install_root.join("attacker-controlled");
        let stale = broker.desktop_owner_operation_json_with_update_confirmations(
            &activation_envelope("desktop-update-activation-stale"),
            None,
            None,
            Some(stale_activation),
        );
        assert_eq!(stale.status, BrokerStatus::Rejected);
        assert_eq!(
            stale.error.unwrap().code,
            "update_activation_destination_stale"
        );
        assert_eq!(
            crate::product_bootstrapper::active_version_snapshot(
                &install_root,
                INSTALL_APP_ID,
                INSTALL_AUDIT_ID,
            )
            .unwrap()
            .current
            .product_version,
            "1.0.0"
        );

        let mut stale_start_menu_confirmation = activation_confirmation.clone();
        stale_start_menu_confirmation.start_menu_shortcut_path =
            start_menu_root.join("attacker-controlled.lnk");
        let stale_start_menu = broker.desktop_owner_operation_json_with_update_confirmations(
            &activation_envelope("desktop-update-activation-start-menu-stale"),
            None,
            None,
            Some(stale_start_menu_confirmation),
        );
        assert_eq!(stale_start_menu.status, BrokerStatus::Rejected);
        assert_eq!(
            stale_start_menu.error.unwrap().code,
            "update_activation_start_menu_stale"
        );
        assert_eq!(
            crate::product_bootstrapper::active_version_snapshot(
                &install_root,
                INSTALL_APP_ID,
                INSTALL_AUDIT_ID,
            )
            .unwrap()
            .current
            .product_version,
            "1.0.0"
        );

        let staged_executable = target.join("app/gui_shell_desktop.exe");
        std::fs::write(&staged_executable, b"x").unwrap();
        let changed_stage = broker.desktop_owner_operation_json_with_update_confirmations(
            &activation_envelope("desktop-update-activation-stage-tampered"),
            None,
            None,
            Some(activation_confirmation.clone()),
        );
        assert_eq!(changed_stage.status, BrokerStatus::Rejected);
        assert_eq!(
            changed_stage.error.unwrap().code,
            "package_stage_existing_file_mismatch"
        );
        assert_eq!(std::fs::read(&staged_executable).unwrap(), b"x");
        let product_root = install_root
            .join("Programs")
            .join("D4 Pocket")
            .join(INSTALL_APP_ID);
        assert_eq!(
            crate::product_bootstrapper::active_version_snapshot(
                &install_root,
                INSTALL_APP_ID,
                INSTALL_AUDIT_ID,
            )
            .unwrap()
            .current
            .product_version,
            "1.0.0"
        );
        std::fs::write(&staged_executable, b"d").unwrap();

        let activated = broker.desktop_owner_operation_json_with_update_confirmations(
            &activation_envelope("desktop-update-activation"),
            None,
            None,
            Some(activation_confirmation.clone()),
        );
        assert_eq!(activated.status, BrokerStatus::Accepted);
        assert_eq!(
            activated.body.as_ref().unwrap()["有効化"],
            "active_version_recorded"
        );
        assert_eq!(activated.body.as_ref().unwrap()["起動"], "not_started");
        assert_eq!(activated.body.as_ref().unwrap()["Start Menu"], "registered");
        assert_eq!(
            activated.body.as_ref().unwrap()["download package cleanup"],
            "completed"
        );
        assert!(!package_path.exists());
        assert!(activation_confirmation.start_menu_shortcut_path.is_file());
        assert!(product_root
            .join("gui_shell_desktop_launcher.exe")
            .is_file());
        let selected_launcher = crate::product_bootstrapper::resolve_active_version_launcher(
            &install_root,
            INSTALL_APP_ID,
            INSTALL_AUDIT_ID,
        )
        .unwrap();
        assert_eq!(
            selected_launcher,
            std::fs::canonicalize(target.join("gui_shell_desktop_launcher.exe")).unwrap()
        );
        let active_record: Value = serde_json::from_slice(
            &std::fs::read(product_root.join("active_version.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(active_record["product_version"], "1.1.0");
        assert_eq!(active_record["previous"]["product_version"], "1.0.0");
        assert!(target.is_dir());
        assert!(install_root
            .join("Programs")
            .join("D4 Pocket")
            .join(INSTALL_APP_ID)
            .join("versions")
            .join(&previous_stage_name)
            .is_dir());
        let shortcut_bytes =
            std::fs::read(&activation_confirmation.start_menu_shortcut_path).unwrap();
        assert!(broker.audit_events().iter().any(|event| {
            event.request_id == "desktop-update-activation"
                && event.operation == OP_ACTIVATE
                && event.decision == "queued"
        }));
        assert!(broker.audit_events().iter().any(|event| {
            event.request_id == "desktop-update-activation:complete"
                && event.operation == OP_ACTIVATE
                && event.decision == "completed"
        }));

        // 有効版のRepairは同じ現在trust packageを再検証し、欠損fileだけを復元する。
        // 既存fileの変更は上書きせず拒否し、active recordは切り替えない。
        std::fs::write(&package_path, &package).unwrap();
        let repair_file = target.join("app/data/app.so");
        std::fs::remove_file(&repair_file).unwrap();
        let repaired_active_stage = broker.desktop_owner_operation_json_with_update_confirmation(
            &envelope("desktop-active-version-repair"),
            None,
            Some(confirmation.clone()),
        );
        assert_eq!(repaired_active_stage.status, BrokerStatus::Accepted);
        assert_eq!(
            repaired_active_stage.body.as_ref().unwrap()["導入状態"],
            "version_staged"
        );
        assert_eq!(repaired_active_stage.body.as_ref().unwrap()["再開"], true);
        assert_eq!(std::fs::read(&repair_file).unwrap(), b"a");
        assert_eq!(
            crate::product_bootstrapper::active_version_snapshot(
                &install_root,
                INSTALL_APP_ID,
                INSTALL_AUDIT_ID,
            )
            .unwrap()
            .current
            .product_version,
            "1.1.0"
        );
        assert!(broker.audit_events().iter().any(|event| {
            event.request_id == "desktop-active-version-repair:complete"
                && event.operation == OP_APPLY
                && event.decision == "completed"
        }));

        std::fs::write(&repair_file, b"x").unwrap();
        let mismatched_active_stage =
            broker.desktop_owner_operation_json_with_update_confirmation(
                &envelope("desktop-active-version-repair-mismatch"),
                None,
                Some(confirmation.clone()),
            );
        assert_eq!(mismatched_active_stage.status, BrokerStatus::Rejected);
        assert_eq!(
            mismatched_active_stage.error.as_ref().unwrap().code,
            "package_stage_existing_file_mismatch"
        );
        assert_eq!(std::fs::read(&repair_file).unwrap(), b"x");
        assert!(broker.audit_events().iter().any(|event| {
            event.request_id == "desktop-active-version-repair-mismatch:failed"
                && event.operation == OP_APPLY
                && event.decision == "failed"
        }));
        std::fs::write(&repair_file, b"a").unwrap();

        let rollback_payload = payload.clone();
        let rollback_payload_hash =
            crate::broker::protocol::canonical_payload_hash(Some(&rollback_payload));
        let rollback_envelope = |request_id: &str| {
            json!({
                "request_id": request_id,
                "session_id": "session-1",
                "operation": OP_ROLLBACK,
                "payload_hash": rollback_payload_hash,
                "nonce": format!("{request_id}-nonce"),
                "issued_at": BrokerRequestEnvelope::current_issued_at(),
                "metadata": {"client": "desktop_flutter"},
                "payload": rollback_payload,
            })
            .to_string()
        };
        let normal_rollback = call(
            &mut broker,
            BrokerOperation::更新rollback要求,
            rollback_payload.clone(),
        );
        assert_eq!(normal_rollback.status, BrokerStatus::Rejected);
        assert_eq!(
            normal_rollback.error.unwrap().code,
            "desktop_native_owner_confirmation_required"
        );
        let previous_plan = super::super::product_install::plan_product_install(
            &install_root,
            INSTALL_APP_ID,
            "1.0.0",
            &previous_digest,
        )
        .unwrap();
        let mut rollback_confirmation = activation_confirmation.clone();
        rollback_confirmation.rollback_target = Some(UpdateRollbackTargetConfirmation {
            update_id: "update-previous".into(),
            candidate_hash: previous_candidate_hash.clone(),
            package_sha256: previous_digest.clone(),
            offered_version: "1.0.0".into(),
            version_directory: previous_plan.version_directory,
        });
        let mut stale_rollback = rollback_confirmation.clone();
        stale_rollback
            .rollback_target
            .as_mut()
            .unwrap()
            .version_directory = install_root.join("attacker-controlled");
        let stale = broker.desktop_owner_operation_json_with_update_confirmations(
            &rollback_envelope("desktop-update-rollback-stale"),
            None,
            None,
            Some(stale_rollback),
        );
        assert_eq!(stale.status, BrokerStatus::Rejected);
        assert_eq!(
            stale.error.unwrap().code,
            "update_rollback_destination_stale"
        );
        let rolled_back = broker.desktop_owner_operation_json_with_update_confirmations(
            &rollback_envelope("desktop-update-rollback"),
            None,
            None,
            Some(rollback_confirmation),
        );
        assert_eq!(rolled_back.status, BrokerStatus::Accepted);
        assert_eq!(
            rolled_back.body.as_ref().unwrap()["rollback"],
            "active_version_restored"
        );
        assert_eq!(rolled_back.body.as_ref().unwrap()["現行版"], "1.1.0");
        assert_eq!(rolled_back.body.as_ref().unwrap()["復元版"], "1.0.0");
        assert_eq!(
            crate::product_bootstrapper::active_version_snapshot(
                &install_root,
                INSTALL_APP_ID,
                INSTALL_AUDIT_ID,
            )
            .unwrap()
            .current
            .update_id,
            "update-previous"
        );
        let rollback_launcher = crate::product_bootstrapper::resolve_active_version_launcher(
            &install_root,
            INSTALL_APP_ID,
            INSTALL_AUDIT_ID,
        )
        .unwrap();
        assert_eq!(
            rollback_launcher,
            std::fs::canonicalize(
                install_root
                    .join("Programs")
                    .join("D4 Pocket")
                    .join(INSTALL_APP_ID)
                    .join("versions")
                    .join(previous_stage_name)
                    .join("gui_shell_desktop_launcher.exe")
            )
            .unwrap()
        );
        assert!(target.is_dir());
        assert!(broker.audit_events().iter().any(|event| {
            event.request_id == "desktop-update-rollback"
                && event.operation == OP_ROLLBACK
                && event.decision == "queued"
        }));
        assert!(broker.audit_events().iter().any(|event| {
            event.request_id == "desktop-update-rollback:complete"
                && event.operation == OP_ROLLBACK
                && event.decision == "completed"
        }));
        assert_eq!(
            std::fs::read(&activation_confirmation.start_menu_shortcut_path).unwrap(),
            shortcut_bytes
        );
        assert_ne!(selected_launcher, rollback_launcher);
        drop(previous_product_root);
        drop(previous_versions);
        drop(broker);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn repaired_package_completion_records_a_recovery_audit() {
        let mut broker = Broker::new("session-1");
        let package_sha256 = "a".repeat(64);
        let mut runtime = UpdateDownloadRuntime::default();
        runtime.job = Some(UpdateDownloadJob {
            job_id: "update-download-repair-test".into(),
            update_id: "update-1".into(),
            candidate_hash: "candidate-hash".into(),
            package_sha256: package_sha256.clone(),
            expected_bytes: 123,
            state: "downloading".into(),
            error_code: None,
            progress: Arc::new(AtomicU64::new(0)),
        });
        let (completion_tx, completion_rx) = mpsc::sync_channel(1);
        completion_tx
            .send(UpdateDownloadCompletion {
                result: Ok(super::super::update_download::DownloadedPackage {
                    file_name: format!("{package_sha256}.pkg"),
                    bytes: 123,
                    sha256: package_sha256,
                    disposition: super::super::update_download::PackageDisposition::RepairedCorrupt,
                }),
            })
            .unwrap();
        runtime.completion_rx = Some(completion_rx);
        broker.update_download = runtime;

        poll_download_completion(&mut broker);

        let event = broker.audit_events().last().unwrap();
        assert_eq!(event.operation, OP_DOWNLOAD);
        assert_eq!(event.decision, "recovered");
        assert!(event.reason.contains("置換"));
        assert_eq!(broker.update_download.projection()["状態"], "downloaded");
    }

    #[test]
    fn update_download_requires_current_native_confirmation_and_runs_as_bounded_job() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "d4p-update-download-owner-{}-{unique}",
            std::process::id()
        ));
        let (mut trust, candidate) = trust_and_candidate();
        trust.package_sources = vec![UpdatePackageSource {
            channel: "stable".into(),
            base_url: "https://updates.example.invalid/d4/stable".into(),
        }];
        let mut broker = Broker::new_persistent("session-1", &root).unwrap();
        broker.update_trust = Some(trust);
        assert_eq!(
            call(
                &mut broker,
                BrokerOperation::更新確認,
                json!({"版": 1, "候補": [candidate]})
            )
            .status,
            BrokerStatus::Accepted
        );
        let record: UpdateRecord =
            serde_json::from_value(broker.updates.get("update-1").unwrap().clone()).unwrap();
        let payload = json!({
            "版": 1,
            "更新ID": "update-1",
            "候補hash": record.candidate_hash,
        });
        let payload_hash = crate::broker::protocol::canonical_payload_hash(Some(&payload));
        let envelope = |request_id: &str| {
            json!({
                "request_id": request_id,
                "session_id": "session-1",
                "operation": OP_DOWNLOAD,
                "payload_hash": payload_hash,
                "nonce": format!("{request_id}-nonce"),
                "issued_at": BrokerRequestEnvelope::current_issued_at(),
                "metadata": {"client": "desktop_flutter"},
                "payload": payload,
            })
            .to_string()
        };
        let denied = call(
            &mut broker,
            BrokerOperation::更新download要求,
            payload.clone(),
        );
        assert_eq!(denied.status, BrokerStatus::Rejected);
        assert_eq!(
            denied.error.unwrap().code,
            "desktop_native_owner_confirmation_required"
        );
        let confirmation = UpdateDownloadConfirmation {
            update_id: "update-1".into(),
            candidate_hash: record.candidate_hash.clone(),
            source_url: "https://updates.example.invalid/d4/stable/update-1.pkg".into(),
            package_sha256: record.candidate.package_sha256.clone().unwrap(),
            package_size_bytes: record.candidate.package_size_bytes.unwrap(),
            offered_version: record.candidate.offered_version.clone(),
            channel: record.candidate.channel.clone(),
            summary: record.candidate.summary.clone(),
            display_host: "updates.example.invalid".into(),
            payload_hash: payload_hash.clone(),
        };
        let partial_name = format!("{}.part", "a".repeat(32));
        let package_directory = broker
            .state_store
            .open_update_package_directory()
            .unwrap()
            .unwrap();
        std::fs::write(root.join("update_packages").join(&partial_name), b"active").unwrap();
        broker.update_download.job = Some(UpdateDownloadJob {
            job_id: "update-download-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            update_id: "update-1".into(),
            candidate_hash: record.candidate_hash.clone(),
            package_sha256: record.candidate.package_sha256.clone().unwrap(),
            expected_bytes: record.candidate.package_size_bytes.unwrap(),
            state: "downloading".into(),
            error_code: None,
            progress: Arc::new(AtomicU64::new(0)),
        });
        let busy = broker.desktop_owner_operation_json_with_update_confirmation(
            &envelope("desktop-update-busy"),
            Some(confirmation.clone()),
            None,
        );
        assert_eq!(busy.status, BrokerStatus::Rejected);
        assert_eq!(busy.error.unwrap().code, "update_download_busy");
        assert!(root.join("update_packages").join(&partial_name).exists());
        broker.update_download.job = None;

        let mut stale_confirmation = confirmation.clone();
        stale_confirmation.source_url =
            "https://attacker.example.invalid/d4/stable/update-1.pkg".into();
        let stale = broker.desktop_owner_operation_json_with_update_confirmation(
            &envelope("desktop-update-stale"),
            Some(stale_confirmation),
            None,
        );
        assert_eq!(stale.status, BrokerStatus::Rejected);
        assert_eq!(
            stale.error.unwrap().code,
            "update_download_confirmation_stale"
        );
        let queued = broker.desktop_owner_operation_json_with_update_confirmation(
            &envelope("desktop-update-accepted"),
            Some(confirmation),
            None,
        );
        assert_eq!(queued.status, BrokerStatus::Accepted);
        assert_eq!(
            queued.body.as_ref().unwrap()["download_job"]["状態"],
            "downloading"
        );
        assert!(!root.join("update_packages").join(&partial_name).exists());
        assert!(broker.audit_events().iter().any(|event| {
            event.operation == "更新download中断file復旧" && event.decision == "recovered"
        }));

        for _ in 0..400 {
            broker.update_download_tick();
            if broker.update_download.projection()["状態"] != "downloading" {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let job = broker.update_download.projection();
        assert_eq!(job["状態"], "failed");
        assert!(matches!(
            job["失敗code"].as_str(),
            Some("update_download_dns_failed" | "update_download_network_failed")
        ));
        assert!(broker
            .audit_events()
            .iter()
            .any(|event| { event.operation == OP_DOWNLOAD && event.decision == "failed" }));
        drop(broker);
        drop(package_directory);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn persisted_candidate_is_downgraded_and_rejected_after_trust_rotation() {
        let (trust, candidate) = trust_and_candidate();
        let candidate_hash = crate::broker::protocol::canonical_payload_hash(Some(
            &serde_json::to_value(&candidate).unwrap(),
        ));
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        assert_eq!(
            call(
                &mut broker,
                BrokerOperation::更新確認,
                json!({"版": 1, "候補": [candidate]})
            )
            .status,
            BrokerStatus::Accepted
        );

        let (rotated_trust, _) = trust_and_candidate();
        broker.update_trust = Some(rotated_trust);
        let listing = call(&mut broker, BrokerOperation::更新一覧, json!({"版": 1}));
        assert_eq!(
            listing.body.unwrap()["更新一覧"][0]["署名状態"],
            "verification_stale"
        );
        let deferred = call(
            &mut broker,
            BrokerOperation::更新延期,
            json!({
                "版": 1,
                "更新ID": "update-1",
                "候補hash": candidate_hash,
                "延期期限": "2030-01-01T00:00:00Z"
            }),
        );
        assert_eq!(deferred.status, BrokerStatus::Accepted);
        assert_eq!(deferred.body.unwrap()["署名状態"], "verification_stale");
        let response = call(
            &mut broker,
            BrokerOperation::更新download要求,
            json!({"版": 1, "更新ID": "update-1", "候補hash": candidate_hash}),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "update_signer_untrusted");

        broker.update_trust = None;
        let listing = call(&mut broker, BrokerOperation::更新一覧, json!({"版": 1}));
        assert_eq!(
            listing.body.unwrap()["更新一覧"][0]["署名状態"],
            "verification_stale"
        );
        let response = call(
            &mut broker,
            BrokerOperation::更新適用要求,
            json!({"版": 1, "更新ID": "update-1", "候補hash": candidate_hash}),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "update_trust_unconfigured");
    }

    #[test]
    fn persisted_candidate_content_and_hash_are_rechecked_before_execution() {
        let (trust, candidate) = trust_and_candidate();
        let candidate_hash = crate::broker::protocol::canonical_payload_hash(Some(
            &serde_json::to_value(&candidate).unwrap(),
        ));
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        assert_eq!(
            call(
                &mut broker,
                BrokerOperation::更新確認,
                json!({"版": 1, "候補": [candidate]})
            )
            .status,
            BrokerStatus::Accepted
        );
        broker.updates.get_mut("update-1").unwrap()["内容概要"] =
            Value::String("署名後に改変された概要".into());
        let listing = call(&mut broker, BrokerOperation::更新一覧, json!({"版": 1}));
        assert_eq!(
            listing.body.unwrap()["更新一覧"][0]["署名状態"],
            "verification_stale"
        );
        let response = call(
            &mut broker,
            BrokerOperation::更新適用要求,
            json!({"版": 1, "更新ID": "update-1", "候補hash": candidate_hash}),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "update_signed_manifest_mismatch"
        );

        let (trust, candidate) = trust_and_candidate();
        let candidate_hash = crate::broker::protocol::canonical_payload_hash(Some(
            &serde_json::to_value(&candidate).unwrap(),
        ));
        let mut broker = Broker::new("session-1");
        broker.update_trust = Some(trust);
        assert_eq!(
            call(
                &mut broker,
                BrokerOperation::更新確認,
                json!({"版": 1, "候補": [candidate]})
            )
            .status,
            BrokerStatus::Accepted
        );
        broker.updates.get_mut("update-1").unwrap()["候補hash"] = Value::String(
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
        );
        let listing = call(&mut broker, BrokerOperation::更新一覧, json!({"版": 1}));
        assert_eq!(
            listing.body.unwrap()["更新一覧"][0]["署名状態"],
            "verification_stale"
        );
        let tampered_hash = broker.updates["update-1"]["候補hash"].clone();
        let response = call(
            &mut broker,
            BrokerOperation::更新適用要求,
            json!({"版": 1, "更新ID": "update-1", "候補hash": tampered_hash}),
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "update_state_tampered");
        assert_ne!(candidate_hash, broker.updates["update-1"]["候補hash"]);
    }

    #[test]
    fn verified_update_state_reloads_without_promoting_trust() {
        let (trust, candidate) = trust_and_candidate();
        let candidate_hash = crate::broker::protocol::canonical_payload_hash(Some(
            &serde_json::to_value(&candidate).unwrap(),
        ));
        let root =
            std::env::temp_dir().join(format!("gui-shell-update-center-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        {
            let mut broker = Broker::new_persistent("session-1", &root).unwrap();
            broker.update_trust = Some(trust);
            assert_eq!(
                call(
                    &mut broker,
                    BrokerOperation::更新確認,
                    json!({"版": 1, "候補": [candidate]})
                )
                .status,
                BrokerStatus::Accepted
            );
        }
        let restarted = Broker::new_persistent("session-1", &root).unwrap();
        assert_eq!(restarted.updates.len(), 1);
        assert!(restarted.update_trust.is_none());
        assert!(root.join("updates.json").exists());
        let mut restarted = restarted;
        let listing = call(&mut restarted, BrokerOperation::更新一覧, json!({"版": 1}));
        assert_eq!(
            listing.body.unwrap()["更新一覧"][0]["署名状態"],
            "verification_stale"
        );
        let execution = call(
            &mut restarted,
            BrokerOperation::更新download要求,
            json!({"版": 1, "更新ID": "update-1", "候補hash": candidate_hash}),
        );
        assert_eq!(execution.status, BrokerStatus::Rejected);
        assert_eq!(execution.error.unwrap().code, "update_trust_unconfigured");
        let _ = std::fs::remove_dir_all(root);
    }
}
