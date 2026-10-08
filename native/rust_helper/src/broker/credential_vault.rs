//! C7 資格情報保管庫のowner登録・失効、metadata一覧、統治済みMCP利用。
//!
//! このmoduleは秘密値をRuntime、Flutter、Audit reason、応答へ返さない。秘密値の
//! MCP以外への注入、更新、物理削除、接続先変更は未接続に保つ。
#![cfg(any(windows, target_os = "macos"))]
#![allow(non_snake_case)]

use super::*;
use crate::broker::protocol::{canonical_payload_hash, EVIDENCE_SOURCE_INTERNAL_STATE};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const VERSION: u64 = 1;
const RECORD_PREFIX: &str = "資格情報記録:";
const REVOCATION_PREFIX: &str = "資格情報失効記録:";
const MCP_USE_PREFIX: &str = "MCP資格情報使用:";
const PROVIDER_USE_PREFIX: &str = "提供元資格情報使用:";
const MAX_TEXT_BYTES: usize = 256;
const MAX_SECRET_BYTES: usize = 65_536;
#[cfg(windows)]
const STORAGE_NAME: &str = "windows_dpapi";
#[cfg(target_os = "macos")]
const STORAGE_NAME: &str = "macos_keychain";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "操作")]
    operation: String,
    #[serde(rename = "資格情報ID")]
    credential_id: String,
    #[serde(rename = "用途")]
    purpose: String,
    #[serde(rename = "接続対象")]
    target: String,
    #[serde(rename = "種類")]
    kind: String,
    #[serde(rename = "保管方式")]
    storage: String,
    #[serde(rename = "登録者種別")]
    registrant: String,
    #[serde(rename = "登録経路")]
    route: String,
    #[serde(rename = "秘密値", deserialize_with = "secret_string")]
    secret: zeroize::Zeroizing<String>,
}

fn secret_string<'de, D: serde::Deserializer<'de>>(
    value: D,
) -> Result<zeroize::Zeroizing<String>, D::Error> {
    String::deserialize(value).map(zeroize::Zeroizing::new)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevocationRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "資格情報ID")]
    credential_id: String,
    #[serde(rename = "用途")]
    purpose: String,
    #[serde(rename = "接続対象")]
    target: String,
    #[serde(rename = "暗号文hash")]
    ciphertext_hash: String,
    #[serde(rename = "作成監査ID")]
    created_audit_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicCredential {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "資格情報ID")]
    credential_id: String,
    #[serde(rename = "用途")]
    purpose: String,
    #[serde(rename = "接続対象")]
    target: String,
    #[serde(rename = "種類")]
    kind: String,
    #[serde(rename = "保管方式")]
    storage: String,
    #[serde(rename = "状態")]
    status: String,
    #[serde(rename = "作成時刻UnixMillis")]
    created_at: i64,
    #[serde(rename = "最終使用時刻UnixMillis")]
    last_used_at: Option<i64>,
    #[serde(rename = "失効時刻UnixMillis")]
    revoked_at: Option<i64>,
    #[serde(rename = "暗号文hash")]
    ciphertext_hash: String,
    #[serde(rename = "作成監査ID")]
    created_audit_id: String,
    #[serde(rename = "公開範囲")]
    exposure: String,
    #[serde(rename = "証拠種別")]
    evidence_source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredCredential {
    #[serde(rename = "保管ID")]
    storage_id: String,
    #[serde(rename = "公開")]
    public: PublicCredential,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct McpCredentialUseRecord {
    version: u64,
    credential_id: String,
    purpose: String,
    target: String,
    environment_variable: String,
    used_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderCredentialUseRecord {
    version: u64,
    credential_id: String,
    provider_id: String,
    runtime_id: String,
    used_at: i64,
}

#[cfg(windows)]
pub(super) enum ProviderCredentialUseFailure {
    Denied,
    Audit,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialRevocationRecord {
    version: u64,
    credential_id: String,
    created_audit_id: String,
    ciphertext_hash: String,
    revoked_at: i64,
}

struct CredentialLedger {
    entries: Vec<StoredCredential>,
    last_use: BTreeMap<String, i64>,
}

impl Broker {
    #[cfg(windows)]
    pub(super) fn 提供元資格情報参照確認(&self, credential_id: &str) -> bool {
        credential_ledger(&self.audit_log).is_ok_and(|ledger| {
            ledger.entries.iter().any(|entry| {
                entry.public.credential_id == credential_id
                    && entry.public.purpose == "provider_api_key"
                    && entry.public.target == "openai_codex_cli"
                    && entry.public.kind == "api_key"
                    && entry.public.status == "有効"
                    && entry.public.revoked_at.is_none()
            })
        })
    }

    #[cfg(windows)]
    pub(super) fn 提供元資格情報使用処理(
        &mut self,
        request_id: &str,
        runtime_id: &str,
        provider_id: &str,
        credential_id: &str,
        payload_hash: &str,
    ) -> Result<zeroize::Zeroizing<Vec<u8>>, ProviderCredentialUseFailure> {
        const OPERATION: &str = "提供元資格情報使用";
        if !self.state_store.persistence_ready()
            || !hex_identifier(credential_id)
            || provider_id != "openai_codex_cli"
            || !safe_text(runtime_id, MAX_TEXT_BYTES)
        {
            return Err(ProviderCredentialUseFailure::Denied);
        }
        let ledger =
            credential_ledger(&self.audit_log).map_err(|_| ProviderCredentialUseFailure::Audit)?;
        let entry = ledger
            .entries
            .into_iter()
            .find(|entry| {
                entry.public.credential_id == credential_id
                    && entry.public.purpose == "provider_api_key"
                    && entry.public.target == provider_id
                    && entry.public.kind == "api_key"
                    && entry.public.status == "有効"
                    && entry.public.revoked_at.is_none()
            })
            .ok_or(ProviderCredentialUseFailure::Denied)?;
        if self.protected_store.is_none() {
            return Err(ProviderCredentialUseFailure::Denied);
        }
        self.append_audit(
            request_id,
            OPERATION,
            "received",
            "Broker承認済みProvider実行から資格情報使用を要求。ID・提供元・Runtime参照のみを記録し、秘密値は記録しない",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .map_err(|_| ProviderCredentialUseFailure::Audit)?;

        let secret = self
            .資格情報保存先()
            .expect("資格情報保存先の登録確認済み")
            .読取(&entry.storage_id, &entry.public.ciphertext_hash)
            .map_err(|_| ProviderCredentialUseFailure::Denied)?;
        if secret.is_empty()
            || std::str::from_utf8(&secret).is_err()
            || secret.iter().any(u8::is_ascii_control)
        {
            return Err(ProviderCredentialUseFailure::Denied);
        }

        let record = ProviderCredentialUseRecord {
            version: VERSION,
            credential_id: credential_id.to_owned(),
            provider_id: provider_id.to_owned(),
            runtime_id: runtime_id.to_owned(),
            used_at: self.current_epoch_millis(),
        };
        let encoded =
            serde_json::to_string(&record).map_err(|_| ProviderCredentialUseFailure::Audit)?;
        self.append_audit(
            request_id,
            OPERATION,
            "accepted",
            &format!("{PROVIDER_USE_PREFIX}{encoded}"),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .map_err(|_| ProviderCredentialUseFailure::Audit)?;
        Ok(secret)
    }

    #[cfg(windows)]
    pub(super) fn 資格情報MCP使用処理(
        &mut self,
        request_id: &str,
        credential_id: &str,
        target: &str,
        payload_hash: &str,
    ) -> Result<gui_shell_windows_protection::Secret, BrokerResponse> {
        const OPERATION: &str = "MCP Credential使用";
        if !self.state_store.persistence_ready() {
            return Err(self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "broker_persistence_unavailable",
                "MCP Credential使用には永続Auditが必要",
                true,
                payload_hash,
            ));
        }
        if !hex_identifier(credential_id) || !safe_text(target, MAX_TEXT_BYTES) {
            return Err(self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_reference_invalid",
                "MCP Credential参照の識別または対象が不正",
                true,
                payload_hash,
            ));
        }
        let entries = match credential_ledger(&self.audit_log) {
            Ok(value) => value.entries,
            Err(_) => {
                return Err(self.reject_with_payload_hash(
                    request_id,
                    OPERATION,
                    "credential_audit_invalid",
                    "資格情報Auditを検証できないため使用を停止しました",
                    true,
                    payload_hash,
                ));
            }
        };
        let Some(entry) = entries.into_iter().find(|entry| {
            entry.public.credential_id == credential_id
                && entry.public.purpose == "mcp_transport"
                && entry.public.target == target
                && entry.public.status == "有効"
                && entry.public.revoked_at.is_none()
        }) else {
            return Err(self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_not_available",
                "対象Serverに結び付く有効なMCP Credentialを確認できません",
                true,
                payload_hash,
            ));
        };
        if self.protected_store.is_none() {
            return Err(self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_storage_unregistered",
                "MCP Credentialの保管先を確認できません",
                true,
                payload_hash,
            ));
        }
        if self
            .append_audit(
                request_id,
                OPERATION,
                "received",
                &format!(
                    "MCP stdio childへのCredential使用を要求。資格情報ID={} 対象={}。秘密値は記録しない",
                    credential_id, target
                ),
                EVIDENCE_SOURCE_INTERNAL_STATE,
                payload_hash,
            )
            .is_err()
        {
            return Err(self.audit_store_failed_response(
                request_id,
                OPERATION,
                "credential_audit_append_failed",
                "MCP Credential使用の受信Auditを確定できません",
            ));
        }
        let secret = self
            .protected_store
            .as_ref()
            .expect("ProtectedStore登録確認済み")
            .read(
                crate::protected_store::Purpose::Credential,
                &entry.storage_id,
                &entry.public.ciphertext_hash,
            );
        match secret {
            Ok(secret) => Ok(secret),
            Err(_) => Err(self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_storage_unavailable",
                "MCP Credentialを安全に読み出せません",
                true,
                payload_hash,
            )),
        }
    }

    #[cfg(windows)]
    pub(super) fn 資格情報MCP使用確定処理(
        &mut self,
        request_id: &str,
        credential_id: &str,
        target: &str,
        environment_variable: &str,
        payload_hash: &str,
    ) -> Result<(), ()> {
        let entries = credential_ledger(&self.audit_log)?.entries;
        if !entries.iter().any(|entry| {
            entry.public.credential_id == credential_id
                && entry.public.purpose == "mcp_transport"
                && entry.public.target == target
                && entry.public.status == "有効"
                && entry.public.revoked_at.is_none()
        }) || !super::mcp_center::safe_credential_environment_name(environment_variable)
        {
            return Err(());
        }
        let record = McpCredentialUseRecord {
            version: VERSION,
            credential_id: credential_id.to_string(),
            purpose: "mcp_transport".to_string(),
            target: target.to_string(),
            environment_variable: environment_variable.to_string(),
            used_at: self.current_epoch_millis(),
        };
        let encoded = serde_json::to_string(&record).map_err(|_| ())?;
        self.append_audit(
            request_id,
            "MCP Credential使用",
            "accepted",
            &format!("{MCP_USE_PREFIX}{encoded}"),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
        .map(|_| ())
        .map_err(|_| ())
    }

    pub(super) fn 資格情報登録処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        owner: bool,
        payload_hash: &str,
    ) -> BrokerResponse {
        const OPERATION: &str = "資格情報登録";
        if !owner {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_owner_required",
                "資格情報の追加にはowner制御資格が必要",
                true,
                payload_hash,
            );
        }
        if !self.state_store.persistence_ready() {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "broker_persistence_unavailable",
                "資格情報登録には永続Auditが必要",
                true,
                payload_hash,
            );
        }
        let registration = match parse_registration(payload) {
            Ok(value) => value,
            Err(reason) => {
                return self.reject_with_payload_hash(
                    request_id,
                    OPERATION,
                    "credential_registration_invalid",
                    reason,
                    true,
                    payload_hash,
                )
            }
        };
        let existing = match credential_ledger(&self.audit_log) {
            Ok(value) => value.entries,
            Err(_) => {
                return self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "credential_audit_invalid",
                    "資格情報Auditを検証できないため登録を停止しました",
                )
            }
        };
        if existing
            .iter()
            .any(|entry| entry.public.credential_id == registration.credential_id)
        {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_duplicate",
                "同じ資格情報IDを再登録できません",
                true,
                payload_hash,
            );
        }
        if self
            .append_audit(
                request_id,
                OPERATION,
                "received",
                "owner制御から資格情報登録要求を受信。秘密値はAuditへ保存しない",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                payload_hash,
            )
            .is_err()
        {
            return self.audit_store_failed_response(
                request_id,
                OPERATION,
                "credential_audit_append_failed",
                "資格情報登録の受信Auditを確定できません",
            );
        }

        if self.資格情報保存先().is_none() {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_storage_unregistered",
                "起動制御でplatformの資格情報保存先を登録してから再試行してください",
                true,
                payload_hash,
            );
        }
        let store = self.資格情報保存先().expect("資格情報保存先の登録確認済み");
        let ciphertext_hash =
            match store.登録(&registration.credential_id, registration.secret.as_bytes()) {
                Ok(value) => value,
                Err(super::credential_storage::保存失敗::回収未成立) => return self
                    .reject_with_payload_hash(
                    request_id,
                    OPERATION,
                    "credential_recovery_required",
                    "新規Keychain itemの回収が未成立。再使用・再送せず保管状態を確認してください",
                    true,
                    payload_hash,
                ),
                Err(_) => {
                    return self.reject_with_payload_hash(
                        request_id,
                        OPERATION,
                        "credential_storage_failed",
                        "既存または部分暗号文を再使用せず保管状態を確認してください",
                        true,
                        payload_hash,
                    )
                }
            };
        let audit_id = self.audit_log.next_event_id();
        let public = PublicCredential {
            version: VERSION,
            credential_id: registration.credential_id.clone(),
            purpose: registration.purpose,
            target: registration.target,
            kind: registration.kind,
            storage: registration.storage,
            status: "有効".to_string(),
            created_at: self.current_epoch_millis(),
            last_used_at: None,
            revoked_at: None,
            ciphertext_hash,
            created_audit_id: audit_id.clone(),
            exposure: "metadata_only".to_string(),
            evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
        };
        let stored = StoredCredential {
            storage_id: registration.credential_id,
            public,
        };
        let body = serde_json::to_value(&stored.public).expect("公開資格情報metadataはJSON化可能");
        let reason = match serde_json::to_string(&stored) {
            Ok(value) => format!("{RECORD_PREFIX}{value}"),
            Err(_) => {
                self.新規資格情報暗号文を破棄(
                    &stored.storage_id,
                    &stored.public.ciphertext_hash,
                );
                return self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "credential_record_invalid",
                    "資格情報の公開recordを確定できません",
                );
            }
        };
        match self.append_audit(
            request_id,
            OPERATION,
            "accepted",
            &reason,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &canonical_payload_hash(Some(&body)),
        ) {
            Ok(event) if event.event_id == audit_id => BrokerResponse {
                request_id: request_id.to_string(),
                operation: OPERATION.to_string(),
                status: BrokerStatus::Accepted,
                evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
                audit_event_id: event.event_id,
                error: None,
                health: None,
                body: Some(body),
                shutdown_requested: self.shutdown_requested,
            },
            Ok(_) | Err(_) => {
                let cleaned = self.新規資格情報暗号文を破棄(
                    &stored.storage_id,
                    &stored.public.ciphertext_hash,
                );
                if cleaned {
                    self.audit_store_failed_response(
                        request_id,
                        OPERATION,
                        "credential_audit_append_failed",
                        "資格情報Auditを確定できず、新規暗号文を破棄しました。",
                    )
                } else {
                    self.audit_store_failed_response(
                        request_id,
                        OPERATION,
                        "credential_recovery_required",
                        "資格情報Auditと新規暗号文の状態を復旧照合してください",
                    )
                }
            }
        }
    }

    pub(super) fn 資格情報一覧処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        owner: bool,
        payload_hash: &str,
    ) -> BrokerResponse {
        const OPERATION: &str = "資格情報一覧";
        if owner {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_normal_channel_required",
                "資格情報一覧は通常資格経路だけが実行できます",
                true,
                payload_hash,
            );
        }
        if !version_only_payload(payload) {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_request_invalid",
                "資格情報一覧は版だけを受け付けます",
                true,
                payload_hash,
            );
        }
        if self.資格情報保存先().is_none() {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_storage_unregistered",
                "資格情報一覧には登録済みProtectedStoreが必要です",
                true,
                payload_hash,
            );
        }
        let ledger = match credential_ledger(&self.audit_log) {
            Ok(value) => value,
            Err(_) => {
                return self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "credential_audit_invalid",
                    "資格情報Auditを検証できないため一覧を停止しました",
                )
            }
        };
        let mut entries = ledger.entries;
        let last_use = ledger.last_use;
        for entry in &mut entries {
            entry.public.last_used_at = last_use.get(&entry.public.credential_id).copied();
        }
        if self
            .append_audit(
                request_id,
                OPERATION,
                "received",
                "資格情報の公開metadata一覧を要求",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                payload_hash,
            )
            .is_err()
        {
            return self.audit_store_failed_response(
                request_id,
                OPERATION,
                "credential_audit_append_failed",
                "資格情報一覧の受信Auditを確定できません",
            );
        }
        let storage_error = {
            let store = self.資格情報保存先().expect("資格情報保存先の登録確認済み");
            entries
                .iter()
                .find_map(|entry| match store.点検(&entry.storage_id) {
                    Ok(Some((hash, _))) if hash == entry.public.ciphertext_hash => None,
                    Ok(Some(_)) => Some((
                        "credential_storage_changed",
                        "資格情報暗号文の整合性を確認できないため一覧を返しません",
                    )),
                    Ok(None) | Err(_) => Some((
                        "credential_storage_missing",
                        "資格情報暗号文の欠落または読取失敗を一覧へ変換しません",
                    )),
                })
        };
        if let Some((code, message)) = storage_error {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                code,
                message,
                true,
                payload_hash,
            );
        }
        let public: Vec<Value> = entries
            .iter()
            .map(|entry| {
                serde_json::to_value(&entry.public).expect("公開資格情報metadataはJSON化可能")
            })
            .collect();
        let body = json!({
            "版": VERSION,
            "資格情報一覧": public,
            "件数": entries.len(),
            "公開範囲": "metadata_only",
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        });
        let event = match self.append_audit(
            request_id,
            OPERATION,
            "accepted",
            "資格情報公開metadata一覧を返却。秘密値は投影しない",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &canonical_payload_hash(Some(&body)),
        ) {
            Ok(event) => event,
            Err(_) => {
                return self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "credential_audit_append_failed",
                    "資格情報一覧の結果Auditを確定できません",
                )
            }
        };
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: OPERATION.to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
            audit_event_id: event.event_id,
            error: None,
            health: None,
            body: Some(body),
            shutdown_requested: self.shutdown_requested,
        }
    }

    pub(super) fn 資格情報失効処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        owner: bool,
        payload_hash: &str,
    ) -> BrokerResponse {
        const OPERATION: &str = "資格情報失効";
        if !owner {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_owner_required",
                "資格情報失効にはnative Owner確認が必要です",
                true,
                payload_hash,
            );
        }
        if !self.state_store.persistence_ready() {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "broker_persistence_unavailable",
                "資格情報失効には永続Auditが必要です",
                true,
                payload_hash,
            );
        }
        let request: RevocationRequest =
            match serde_json::from_value::<RevocationRequest>(payload.clone()) {
                Ok(value)
                    if value.version == VERSION
                        && hex_identifier(&value.credential_id)
                        && safe_text(&value.purpose, MAX_TEXT_BYTES)
                        && safe_text(&value.target, MAX_TEXT_BYTES)
                        && super::protocol::is_tagged_sha256(&value.ciphertext_hash)
                        && !value.created_audit_id.is_empty()
                        && value.created_audit_id.len() <= 256 =>
                {
                    value
                }
                _ => {
                    return self.reject_with_payload_hash(
                        request_id,
                        OPERATION,
                        "credential_revocation_invalid",
                        "資格情報失効要求の版または識別子が不正です",
                        true,
                        payload_hash,
                    )
                }
            };
        let log = match self.state_store.verified_audit_log() {
            Ok(Some(log)) if log == self.audit_log => log,
            _ => {
                return self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "credential_audit_invalid",
                    "失効前に永続資格情報Auditを再確認してください",
                )
            }
        };
        let ledger = match credential_ledger(&log) {
            Ok(value) => value,
            Err(_) => {
                return self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "credential_audit_invalid",
                    "資格情報Auditを検証できないため失効を停止しました",
                )
            }
        };
        let last_use = ledger.last_use;
        let Some(mut entry) = ledger
            .entries
            .into_iter()
            .find(|entry| entry.public.credential_id == request.credential_id)
        else {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_not_found",
                "登録Auditに一致する資格情報がありません",
                true,
                payload_hash,
            );
        };
        if entry.public.status != "有効" || entry.public.revoked_at.is_some() {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_already_revoked",
                "資格情報はすでに失効しており状態を再利用できません",
                true,
                payload_hash,
            );
        }
        if entry.public.purpose != request.purpose
            || entry.public.target != request.target
            || entry.public.ciphertext_hash != request.ciphertext_hash
            || entry.public.created_audit_id != request.created_audit_id
        {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_metadata_stale",
                "native確認時の資格情報metadataと現在の登録Auditが一致しません",
                true,
                payload_hash,
            );
        }
        entry.public.last_used_at = last_use.get(&entry.public.credential_id).copied();
        if self
            .append_audit(
                request_id,
                OPERATION,
                "received",
                "Capability=credential.revoke Permission=credential.revoke.owner_control Approval=Rust Desktop native Owner確認で対象metadataを照合してOwnerがYesを選択 RecoveryAction=失効は取消不可。再利用が必要なら別IDで再登録し、暗号文物理削除は独立操作で行う。秘密値は記録しない",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                payload_hash,
            )
            .is_err()
        {
            return self.audit_store_failed_response(
                request_id,
                OPERATION,
                "credential_audit_append_failed",
                "資格情報失効の受信Auditを確定できません",
            );
        }
        let record = CredentialRevocationRecord {
            version: VERSION,
            credential_id: entry.public.credential_id.clone(),
            created_audit_id: entry.public.created_audit_id.clone(),
            ciphertext_hash: entry.public.ciphertext_hash.clone(),
            revoked_at: self.current_epoch_millis().max(entry.public.created_at),
        };
        let reason = match serde_json::to_string(&record) {
            Ok(value) => format!("{REVOCATION_PREFIX}{value}"),
            Err(_) => {
                return self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "credential_revocation_record_invalid",
                    "資格情報失効記録を確定できません",
                )
            }
        };
        let mut public = entry.public;
        public.status = "失効".to_string();
        public.revoked_at = Some(record.revoked_at);
        let body = serde_json::to_value(&public).expect("資格情報失効metadataはJSON化可能");
        let event = match self.append_audit(
            request_id,
            OPERATION,
            "accepted",
            &reason,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &canonical_payload_hash(Some(&body)),
        ) {
            Ok(event) => event,
            Err(_) => {
                return self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "credential_audit_append_failed",
                    "失効状態を確定するAuditを保存できません",
                )
            }
        };
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: OPERATION.to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
            audit_event_id: event.event_id,
            error: None,
            health: None,
            body: Some(body),
            shutdown_requested: self.shutdown_requested,
        }
    }

    fn 新規資格情報暗号文を破棄(
        &self,
        storage_id: &str,
        ciphertext_hash: &str,
    ) -> bool {
        self.資格情報保存先()
            .is_some_and(|store| store.新規破棄(storage_id, ciphertext_hash))
    }
}

fn parse_registration(payload: &Value) -> Result<Registration, &'static str> {
    let registration: Registration =
        Registration::deserialize(payload).map_err(|_| "資格情報登録payloadの構造が不正です")?;
    if registration.version != VERSION
        || registration.operation != "追加"
        || registration.storage != STORAGE_NAME
        || registration.registrant != "owner"
        || registration.route != "owner_control"
        || !hex_identifier(&registration.credential_id)
        || !safe_text(&registration.purpose, MAX_TEXT_BYTES)
        || !safe_text(&registration.target, MAX_TEXT_BYTES)
        || !matches!(
            registration.kind.as_str(),
            "api_key" | "oauth" | "basic" | "ssh" | "custom"
        )
        || registration.secret.is_empty()
        || registration.secret.as_bytes().len() > MAX_SECRET_BYTES
    {
        return Err("資格情報登録payloadの値またはowner経路が不正です");
    }
    Ok(registration)
}

fn credential_ledger(log: &BrokerAuditLog) -> Result<CredentialLedger, ()> {
    let mut by_id: BTreeMap<String, StoredCredential> = BTreeMap::new();
    let mut last_use = BTreeMap::new();
    for event in log.events() {
        match (event.operation.as_str(), event.decision.as_str()) {
            ("資格情報登録", "accepted") => {
                let encoded = event.reason.strip_prefix(RECORD_PREFIX).ok_or(())?;
                let record: StoredCredential = serde_json::from_str(encoded).map_err(|_| ())?;
                if record.public.created_audit_id != event.event_id
                    || record.public.version != VERSION
                    || record.public.exposure != "metadata_only"
                    || record.public.evidence_source != EVIDENCE_SOURCE_INTERNAL_STATE
                    || !hex_identifier(&record.storage_id)
                    || record.storage_id != record.public.credential_id
                    || record.public.status != "有効"
                    || record.public.revoked_at.is_some()
                    || record.public.created_at <= 0
                    || record.public.last_used_at.is_some()
                    || !safe_text(&record.public.purpose, MAX_TEXT_BYTES)
                    || !safe_text(&record.public.target, MAX_TEXT_BYTES)
                    || !matches!(
                        record.public.kind.as_str(),
                        "api_key" | "oauth" | "basic" | "ssh" | "custom"
                    )
                    || record.public.storage != STORAGE_NAME
                    || !super::protocol::is_tagged_sha256(&record.public.ciphertext_hash)
                {
                    return Err(());
                }
                if by_id
                    .insert(record.public.credential_id.clone(), record)
                    .is_some()
                {
                    return Err(());
                }
            }
            ("資格情報失効", "accepted") => {
                let encoded = event.reason.strip_prefix(REVOCATION_PREFIX).ok_or(())?;
                let record: CredentialRevocationRecord =
                    serde_json::from_str(encoded).map_err(|_| ())?;
                let entry = by_id.get_mut(&record.credential_id).ok_or(())?;
                if record.version != VERSION
                    || !hex_identifier(&record.credential_id)
                    || record.created_audit_id != entry.public.created_audit_id
                    || record.ciphertext_hash != entry.public.ciphertext_hash
                    || record.revoked_at < entry.public.created_at
                    || entry.public.status != "有効"
                    || entry.public.revoked_at.is_some()
                {
                    return Err(());
                }
                entry.public.status = "失効".to_string();
                entry.public.revoked_at = Some(record.revoked_at);
            }
            ("MCP Credential使用", "accepted") => {
                let encoded = event.reason.strip_prefix(MCP_USE_PREFIX).ok_or(())?;
                let record: McpCredentialUseRecord =
                    serde_json::from_str(encoded).map_err(|_| ())?;
                let entry = by_id.get(&record.credential_id).ok_or(())?;
                if record.version != VERSION
                    || !hex_identifier(&record.credential_id)
                    || record.purpose != "mcp_transport"
                    || !safe_text(&record.target, MAX_TEXT_BYTES)
                    || !super::mcp_center::safe_credential_environment_name(
                        &record.environment_variable,
                    )
                    || record.used_at <= 0
                    || entry.public.purpose != record.purpose
                    || entry.public.target != record.target
                    || entry.public.status != "有効"
                    || entry.public.revoked_at.is_some()
                {
                    return Err(());
                }
                last_use
                    .entry(record.credential_id)
                    .and_modify(|used_at: &mut i64| *used_at = (*used_at).max(record.used_at))
                    .or_insert(record.used_at);
            }
            ("提供元資格情報使用", "accepted") => {
                let encoded = event.reason.strip_prefix(PROVIDER_USE_PREFIX).ok_or(())?;
                let record: ProviderCredentialUseRecord =
                    serde_json::from_str(encoded).map_err(|_| ())?;
                let entry = by_id.get(&record.credential_id).ok_or(())?;
                if record.version != VERSION
                    || !hex_identifier(&record.credential_id)
                    || record.provider_id != "openai_codex_cli"
                    || !safe_text(&record.runtime_id, MAX_TEXT_BYTES)
                    || record.used_at <= 0
                    || entry.public.purpose != "provider_api_key"
                    || entry.public.target != record.provider_id
                    || entry.public.kind != "api_key"
                    || entry.public.status != "有効"
                    || entry.public.revoked_at.is_some()
                {
                    return Err(());
                }
                last_use
                    .entry(record.credential_id)
                    .and_modify(|used_at: &mut i64| *used_at = (*used_at).max(record.used_at))
                    .or_insert(record.used_at);
            }
            _ => {}
        }
    }
    Ok(CredentialLedger {
        entries: by_id.into_values().collect(),
        last_use,
    })
}

fn safe_text(value: &str, max_bytes: usize) -> bool {
    !value.is_empty() && value.as_bytes().len() <= max_bytes && !value.chars().any(char::is_control)
}

fn version_only_payload(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|object| object.len() == 1 && object.get("版") == Some(&Value::from(VERSION)))
}

fn hex_identifier(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::io::Read;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_payload(id: &str) -> Value {
        json!({
            "版": 1,
            "操作": "追加",
            "資格情報ID": id,
            "用途": "synthetic test only",
            "接続対象": "fixture-runtime",
            "種類": "custom",
            "保管方式": "windows_dpapi",
            "登録者種別": "owner",
            "登録経路": "owner_control",
            "秘密値": "synthetic-secret-not-operational",
        })
    }

    fn mcp_fixture_payload(id: &str, target: &str, secret: &str) -> Value {
        json!({
            "版": 1,
            "操作": "追加",
            "資格情報ID": id,
            "用途": "mcp_transport",
            "接続対象": target,
            "種類": "api_key",
            "保管方式": "windows_dpapi",
            "登録者種別": "owner",
            "登録経路": "owner_control",
            "秘密値": secret,
        })
    }

    fn provider_fixture_payload(id: &str, secret: &str) -> Value {
        json!({
            "版": 1,
            "操作": "追加",
            "資格情報ID": id,
            "用途": "provider_api_key",
            "接続対象": "openai_codex_cli",
            "種類": "api_key",
            "保管方式": "windows_dpapi",
            "登録者種別": "owner",
            "登録経路": "owner_control",
            "秘密値": secret,
        })
    }

    fn call(
        broker: &mut Broker,
        operation: BrokerOperation,
        payload: Value,
        owner: bool,
    ) -> BrokerResponse {
        let id = format!("credential-test-{}", broker.audit_events().len());
        let mut envelope = BrokerRequestEnvelope::command_envelope_at(
            &id,
            "session-1",
            &format!("nonce-{id}"),
            &BrokerRequestEnvelope::current_issued_at(),
        );
        envelope.operation = Some(operation);
        envelope.payload = Some(payload);
        envelope.refresh_payload_hash();
        broker.処理(envelope, owner)
    }

    fn native_owner_call(broker: &mut Broker, payload: Value) -> BrokerResponse {
        let id = format!("credential-native-test-{}", broker.audit_events().len());
        let request = json!({
            "request_id": id,
            "session_id": "session-1",
            "operation": "資格情報失効",
            "payload_hash": canonical_payload_hash(Some(&payload)),
            "nonce": format!("native-nonce-{id}"),
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"},
            "payload": payload,
        });
        broker.desktop_owner_operation_json(&request.to_string())
    }

    fn broker_with_vault(name: &str) -> (Broker, std::path::PathBuf) {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("gui-shell-credential-{name}-{stamp}"));
        let audit = root.join("audit");
        let vault = root.join("vault");
        std::fs::create_dir_all(&vault).expect("vault");
        let mut broker = Broker::new_persistent("session-1", &audit).expect("永続Broker");
        broker.current_epoch_seconds_override = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_secs() as i64,
        );
        broker
            .保管先起動登録(&vault, true, std::slice::from_ref(&audit))
            .expect("保護保管登録");
        (broker, root)
    }

    #[test]
    fn owner_adds_credential_and_normal_list_never_projects_secret() {
        let (mut broker, root) = broker_with_vault("public-projection");
        let id = "c".repeat(32);
        let secret = "synthetic-secret-not-operational";
        let added = call(
            &mut broker,
            BrokerOperation::資格情報登録,
            fixture_payload(&id),
            true,
        );
        assert_eq!(added.status, BrokerStatus::Accepted, "{added:?}");
        let body = added.body.expect("receipt");
        let encoded = serde_json::to_string(&body).expect("公開記録の文字列化");
        assert!(!encoded.contains(secret));
        assert_eq!(body["公開範囲"], "metadata_only");
        let ciphertext = root.join("vault").join(format!("credential-{id}.dpapi"));
        let mut bytes = Vec::new();
        std::fs::File::open(&ciphertext)
            .expect("暗号文")
            .read_to_end(&mut bytes)
            .expect("暗号文読取");
        assert!(!String::from_utf8_lossy(&bytes).contains(secret));

        let listed = call(
            &mut broker,
            BrokerOperation::資格情報一覧,
            json!({"版": 1}),
            false,
        );
        assert_eq!(listed.status, BrokerStatus::Accepted, "{listed:?}");
        let listed_json = serde_json::to_string(&listed.body).expect("一覧JSON");
        assert!(!listed_json.contains(secret));
        assert_eq!(listed.body.expect("list")["件数"], 1);
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn mcp_credential_use_is_bound_to_registered_target_and_only_audit_metadata_is_projected() {
        let (mut broker, root) = broker_with_vault("mcp-use");
        let id = "f".repeat(32);
        let target = "fixture-mcp-server";
        let secret_marker = "synthetic-mcp-secret-never-project";
        let added = call(
            &mut broker,
            BrokerOperation::資格情報登録,
            mcp_fixture_payload(&id, target, secret_marker),
            true,
        );
        assert_eq!(added.status, BrokerStatus::Accepted, "{added:?}");

        let wrong_target = broker.資格情報MCP使用処理(
            "mcp-use-wrong-target",
            &id,
            "another-mcp-server",
            &canonical_payload_hash(None),
        );
        let wrong_target_code = match wrong_target {
            Err(response) => response.error.expect("拒否応答のerror").code,
            Ok(_) => panic!("別ServerへのCredential再利用を拒否する"),
        };
        assert_eq!(wrong_target_code, "credential_not_available");

        let secret = broker
            .資格情報MCP使用処理(
                "mcp-use-correct-target",
                &id,
                target,
                &canonical_payload_hash(None),
            )
            .expect("対象と用途が一致するMCP接続だけが復号");
        assert_eq!(secret.as_bytes(), secret_marker.as_bytes());
        broker
            .資格情報MCP使用確定処理(
                "mcp-use-correct-target",
                &id,
                target,
                "MCP_API_KEY",
                &canonical_payload_hash(None),
            )
            .expect("成功した注入を監査へ記録");
        drop(secret);

        let listed = call(
            &mut broker,
            BrokerOperation::資格情報一覧,
            json!({"版": 1}),
            false,
        );
        assert_eq!(listed.status, BrokerStatus::Accepted, "{listed:?}");
        let output = serde_json::to_string(&listed.body).expect("公開metadata");
        let audit = serde_json::to_string(&broker.audit_events()).expect("監査記録の取得");
        let body = listed.body.expect("資格情報一覧receipt");
        assert!(!output.contains(secret_marker));
        assert!(!audit.contains(secret_marker));
        assert_eq!(body["資格情報一覧"][0]["接続対象"], target);
        assert!(body["資格情報一覧"][0]["最終使用時刻UnixMillis"]
            .as_i64()
            .is_some());
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn provider_credential_use_is_bound_to_codex_and_audited_without_secret() {
        let (mut broker, root) = broker_with_vault("provider-use");
        let id = "9".repeat(32);
        let secret_marker = "synthetic-provider-key-never-project";
        let added = call(
            &mut broker,
            BrokerOperation::資格情報登録,
            provider_fixture_payload(&id, secret_marker),
            true,
        );
        assert_eq!(added.status, BrokerStatus::Accepted, "{added:?}");
        assert!(broker.提供元資格情報参照確認(&id));
        assert!(!broker.提供元資格情報参照確認(&"8".repeat(32)));

        assert!(matches!(
            broker.提供元資格情報使用処理(
                "provider-use-wrong-target",
                "codex-runtime-fixture",
                "another-provider",
                &id,
                &canonical_payload_hash(None),
            ),
            Err(ProviderCredentialUseFailure::Denied)
        ));
        let secret = match broker.提供元資格情報使用処理(
            "provider-use-approved-fixture",
            "codex-runtime-fixture",
            "openai_codex_cli",
            &id,
            &canonical_payload_hash(None),
        ) {
            Ok(secret) => secret,
            Err(_) => panic!("一致する提供元だけがBroker内で短命復号"),
        };
        assert_eq!(secret.as_slice(), secret_marker.as_bytes());
        drop(secret);

        let listed = call(
            &mut broker,
            BrokerOperation::資格情報一覧,
            json!({"版": 1}),
            false,
        );
        assert_eq!(listed.status, BrokerStatus::Accepted, "{listed:?}");
        let public = listed.body.expect("Provider資格情報metadata");
        assert_eq!(
            public["資格情報一覧"][0]["最終使用時刻UnixMillis"]
                .as_i64()
                .is_some(),
            true
        );
        let audit = serde_json::to_string(&broker.audit_events()).expect("使用Audit");
        assert!(audit.contains("提供元資格情報使用"));
        assert!(audit.contains(&id));
        assert!(!audit.contains(secret_marker));
        assert!(!public.to_string().contains(secret_marker));

        let revoked = native_owner_call(
            &mut broker,
            json!({
                "版": 1,
                "資格情報ID": id,
                "用途": "provider_api_key",
                "接続対象": "openai_codex_cli",
                "暗号文hash": added.body.as_ref().unwrap()["暗号文hash"],
                "作成監査ID": added.body.as_ref().unwrap()["作成監査ID"],
            }),
        );
        assert_eq!(revoked.status, BrokerStatus::Accepted, "{revoked:?}");
        assert!(!broker.提供元資格情報参照確認(&id));
        assert!(matches!(
            broker.提供元資格情報使用処理(
                "provider-use-after-revoke",
                "codex-runtime-fixture",
                "openai_codex_cli",
                &id,
                &canonical_payload_hash(None),
            ),
            Err(ProviderCredentialUseFailure::Denied)
        ));
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn credential_add_requires_owner_and_list_requires_normal_channel() {
        let (mut broker, root) = broker_with_vault("channels");
        let id = "d".repeat(32);
        let normal_add = call(
            &mut broker,
            BrokerOperation::資格情報登録,
            fixture_payload(&id),
            false,
        );
        assert_eq!(normal_add.status, BrokerStatus::Rejected);
        assert_eq!(
            normal_add.error.expect("error").code,
            "credential_owner_required"
        );
        let owner_list = call(
            &mut broker,
            BrokerOperation::資格情報一覧,
            json!({"版": 1}),
            true,
        );
        assert_eq!(owner_list.status, BrokerStatus::Rejected);
        assert_eq!(
            owner_list.error.expect("error").code,
            "credential_normal_channel_required"
        );
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn credential_revocation_requires_durable_audit_even_after_native_confirmation() {
        let mut broker = Broker::new("session-1");
        let response = native_owner_call(&mut broker, json!({"版": 1}));
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.expect("永続Audit必須").code,
            "broker_persistence_unavailable"
        );
        let audit = broker.audit_events();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].decision, "rejected");
    }

    #[test]
    fn missing_or_changed_credential_storage_does_not_return_partial_list() {
        let (mut broker, root) = broker_with_vault("missing");
        let id = "e".repeat(32);
        assert_eq!(
            call(
                &mut broker,
                BrokerOperation::資格情報登録,
                fixture_payload(&id),
                true
            )
            .status,
            BrokerStatus::Accepted
        );
        let ciphertext = root.join("vault").join(format!("credential-{id}.dpapi"));
        std::fs::remove_file(&ciphertext).expect("資格情報削除");
        let listed = call(
            &mut broker,
            BrokerOperation::資格情報一覧,
            json!({"版": 1}),
            false,
        );
        assert_eq!(listed.status, BrokerStatus::Rejected);
        assert!(listed.body.is_none());
        assert_eq!(
            listed.error.expect("error").code,
            "credential_storage_missing"
        );
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn native_owner_revocation_is_audited_persistent_and_blocks_later_mcp_use() {
        let (mut broker, root) = broker_with_vault("revocation");
        let id = "1".repeat(32);
        let target = "fixture-mcp-server";
        let secret_marker = "synthetic-revocation-secret-never-project";
        let added = call(
            &mut broker,
            BrokerOperation::資格情報登録,
            mcp_fixture_payload(&id, target, secret_marker),
            true,
        );
        assert_eq!(added.status, BrokerStatus::Accepted, "{added:?}");

        let owner_credential_request = call(
            &mut broker,
            BrokerOperation::資格情報失効,
            json!({"版": 1, "資格情報ID": id}),
            true,
        );
        assert_eq!(owner_credential_request.status, BrokerStatus::Rejected);
        assert_eq!(
            owner_credential_request.error.expect("native確認拒否").code,
            "desktop_native_owner_confirmation_required"
        );

        let stale = native_owner_call(
            &mut broker,
            json!({
                "版": 1,
                "資格情報ID": id,
                "用途": "mcp_transport",
                "接続対象": "another-server",
                "暗号文hash": added.body.as_ref().unwrap()["暗号文hash"],
                "作成監査ID": added.body.as_ref().unwrap()["作成監査ID"],
            }),
        );
        assert_eq!(stale.status, BrokerStatus::Rejected);
        assert_eq!(
            stale.error.expect("古いmetadata拒否").code,
            "credential_metadata_stale"
        );

        let revoked = native_owner_call(
            &mut broker,
            json!({
                "版": 1,
                "資格情報ID": id,
                "用途": "mcp_transport",
                "接続対象": target,
                "暗号文hash": added.body.as_ref().unwrap()["暗号文hash"],
                "作成監査ID": added.body.as_ref().unwrap()["作成監査ID"],
            }),
        );
        assert_eq!(revoked.status, BrokerStatus::Accepted, "{revoked:?}");
        let receipt = revoked.body.expect("失効metadata");
        assert_eq!(receipt["資格情報ID"], id);
        assert_eq!(receipt["状態"], "失効");
        assert!(receipt["失効時刻UnixMillis"].as_i64().is_some());
        assert_eq!(receipt["公開範囲"], "metadata_only");
        assert!(!receipt.to_string().contains(secret_marker));

        drop(broker);
        let audit_dir = root.join("audit");
        let vault_dir = root.join("vault");
        let mut broker =
            Broker::new_persistent("session-1", &audit_dir).expect("失効AuditからBrokerを再起動");
        broker.current_epoch_seconds_override = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_secs() as i64,
        );
        broker
            .保管先起動登録(&vault_dir, true, std::slice::from_ref(&audit_dir))
            .expect("再起動後も同じProtectedStoreを登録");

        let unavailable = broker.資格情報MCP使用処理(
            "mcp-use-after-revoke",
            &id,
            target,
            &canonical_payload_hash(None),
        );
        let unavailable_code = match unavailable {
            Err(response) => response.error.expect("失効後の利用拒否").code,
            Ok(_) => panic!("失効済みCredentialの再利用を拒否する"),
        };
        assert_eq!(unavailable_code, "credential_not_available");
        let listed = call(
            &mut broker,
            BrokerOperation::資格情報一覧,
            json!({"版": 1}),
            false,
        );
        assert_eq!(listed.status, BrokerStatus::Accepted, "{listed:?}");
        let public = listed.body.expect("metadata一覧");
        assert_eq!(public["資格情報一覧"][0]["状態"], "失効");
        assert_eq!(
            public["資格情報一覧"][0]["失効時刻UnixMillis"],
            receipt["失効時刻UnixMillis"]
        );
        assert!(!public.to_string().contains(secret_marker));
        assert!(root
            .join("vault")
            .join(format!("credential-{id}.dpapi"))
            .exists());
        let audit = serde_json::to_string(&broker.audit_events()).expect("失効監査");
        assert!(audit.contains("Capability=credential.revoke"));
        assert!(audit.contains("Permission=credential.revoke.owner_control"));
        assert!(audit.contains("Approval=Rust Desktop native Owner確認"));
        assert!(audit.contains("RecoveryAction=失効は取消不可"));
        assert!(!audit.contains(secret_marker));

        let duplicate = native_owner_call(
            &mut broker,
            json!({
                "版": 1,
                "資格情報ID": id,
                "用途": "mcp_transport",
                "接続対象": target,
                "暗号文hash": added.body.as_ref().unwrap()["暗号文hash"],
                "作成監査ID": added.body.as_ref().unwrap()["作成監査ID"],
            }),
        );
        assert_eq!(duplicate.status, BrokerStatus::Rejected);
        assert_eq!(
            duplicate.error.expect("二重失効拒否").code,
            "credential_already_revoked"
        );
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
