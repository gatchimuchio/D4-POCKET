//! C7 資格情報保管庫のowner登録、metadata一覧、統治済みMCP利用。
//!
//! このmoduleは秘密値をRuntime、Flutter、Audit reason、応答へ返さない。秘密値の
//! MCP以外への注入、更新、失効、削除、接続先変更は未接続に保つ。
#![cfg(windows)]

use super::*;
use crate::broker::protocol::{canonical_payload_hash, EVIDENCE_SOURCE_INTERNAL_STATE};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const VERSION: u64 = 1;
const RECORD_PREFIX: &str = "資格情報記録:";
const MCP_USE_PREFIX: &str = "MCP資格情報使用:";
const MAX_TEXT_BYTES: usize = 256;
const MAX_SECRET_BYTES: usize = 65_536;

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
    #[serde(rename = "秘密値")]
    secret: String,
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

impl Broker {
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
        let entries = match stored_credentials(&self.audit_log) {
            Ok(value) => value,
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

    pub(super) fn 資格情報MCP使用確定処理(
        &mut self,
        request_id: &str,
        credential_id: &str,
        target: &str,
        environment_variable: &str,
        payload_hash: &str,
    ) -> Result<(), ()> {
        let entries = stored_credentials(&self.audit_log)?;
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
        let existing = match stored_credentials(&self.audit_log) {
            Ok(value) => value,
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

        if self.protected_store.is_none() {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_storage_unregistered",
                "起動制御でWindows ProtectedStoreを登録してから再試行してください",
                true,
                payload_hash,
            );
        }
        let store = self
            .protected_store
            .as_ref()
            .expect("ProtectedStore登録確認済み");
        let ciphertext_hash = match store.create(
            crate::protected_store::Purpose::Credential,
            &registration.credential_id,
            registration.secret.as_bytes(),
        ) {
            Ok(value) => value,
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
                self.新規資格情報暗号文を破棄(&stored.storage_id, &stored.public.ciphertext_hash);
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
        if self.protected_store.is_none() {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "credential_storage_unregistered",
                "資格情報一覧には登録済みProtectedStoreが必要です",
                true,
                payload_hash,
            );
        }
        let mut entries = match stored_credentials(&self.audit_log) {
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
        let last_use = match credential_last_use(&self.audit_log, &entries) {
            Ok(value) => value,
            Err(_) => {
                return self.reject_with_payload_hash(
                    request_id,
                    OPERATION,
                    "credential_audit_invalid",
                    "資格情報使用Auditを検証できないため一覧を停止しました",
                    true,
                    payload_hash,
                );
            }
        };
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
            let store = self
                .protected_store
                .as_ref()
                .expect("ProtectedStore登録確認済み");
            entries.iter().find_map(|entry| {
                match store.inspect(crate::protected_store::Purpose::Credential, &entry.storage_id) {
                    Ok(Some((hash, _))) if hash == entry.public.ciphertext_hash => None,
                    Ok(Some(_)) => Some((
                        "credential_storage_changed",
                        "資格情報暗号文の整合性を確認できないため一覧を返しません",
                    )),
                    Ok(None) | Err(_) => Some((
                        "credential_storage_missing",
                        "資格情報暗号文の欠落または読取失敗を一覧へ変換しません",
                    )),
                }
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
            .map(|entry| serde_json::to_value(&entry.public).expect("公開資格情報metadataはJSON化可能"))
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

    fn 新規資格情報暗号文を破棄(&self, storage_id: &str, ciphertext_hash: &str) -> bool {
        self.protected_store
            .as_ref()
            .and_then(|store| {
                store
                    .prepare_delete(crate::protected_store::Purpose::Credential, storage_id, ciphertext_hash)
                    .ok()
            })
            .and_then(|prepared| prepared.commit().ok())
            .is_some()
    }
}

fn parse_registration(payload: &Value) -> Result<Registration, &'static str> {
    let registration: Registration = serde_json::from_value(payload.clone())
        .map_err(|_| "資格情報登録payloadの構造が不正です")?;
    if registration.version != VERSION
        || registration.operation != "追加"
        || registration.storage != "windows_dpapi"
        || registration.registrant != "owner"
        || registration.route != "owner_control"
        || !hex_identifier(&registration.credential_id)
        || !safe_text(&registration.purpose, MAX_TEXT_BYTES)
        || !safe_text(&registration.target, MAX_TEXT_BYTES)
        || !matches!(registration.kind.as_str(), "api_key" | "oauth" | "basic" | "ssh" | "custom")
        || registration.secret.is_empty()
        || registration.secret.as_bytes().len() > MAX_SECRET_BYTES
    {
        return Err("資格情報登録payloadの値またはowner経路が不正です");
    }
    Ok(registration)
}

fn stored_credentials(log: &BrokerAuditLog) -> Result<Vec<StoredCredential>, ()> {
    let mut by_id = BTreeMap::new();
    for event in log.events() {
        if event.operation != "資格情報登録" || event.decision != "accepted" {
            continue;
        }
        let encoded = event.reason.strip_prefix(RECORD_PREFIX).ok_or(())?;
        let record: StoredCredential = serde_json::from_str(encoded).map_err(|_| ())?;
        if record.public.created_audit_id != event.event_id
            || record.public.version != VERSION
            || record.public.exposure != "metadata_only"
            || record.public.evidence_source != EVIDENCE_SOURCE_INTERNAL_STATE
            || !hex_identifier(&record.storage_id)
            || record.storage_id != record.public.credential_id
        {
            return Err(());
        }
        if by_id.insert(record.public.credential_id.clone(), record).is_some() {
            return Err(());
        }
    }
    Ok(by_id.into_values().collect())
}

fn credential_last_use(
    log: &BrokerAuditLog,
    entries: &[StoredCredential],
) -> Result<BTreeMap<String, i64>, ()> {
    let mut last_use = BTreeMap::new();
    for event in log.events() {
        if event.operation != "MCP Credential使用" || event.decision != "accepted" {
            continue;
        }
        let encoded = event.reason.strip_prefix(MCP_USE_PREFIX).ok_or(())?;
        let record: McpCredentialUseRecord = serde_json::from_str(encoded).map_err(|_| ())?;
        if record.version != VERSION
            || !hex_identifier(&record.credential_id)
            || record.purpose != "mcp_transport"
            || !safe_text(&record.target, MAX_TEXT_BYTES)
            || !super::mcp_center::safe_credential_environment_name(&record.environment_variable)
            || record.used_at <= 0
            || !entries.iter().any(|entry| {
                entry.public.credential_id == record.credential_id
                    && entry.public.purpose == record.purpose
                    && entry.public.target == record.target
            })
        {
            return Err(());
        }
        last_use
            .entry(record.credential_id)
            .and_modify(|used_at: &mut i64| *used_at = (*used_at).max(record.used_at))
            .or_insert(record.used_at);
    }
    Ok(last_use)
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
    value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
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

    fn call(broker: &mut Broker, operation: BrokerOperation, payload: Value, owner: bool) -> BrokerResponse {
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
        assert_eq!(
            wrong_target.expect_err("別Serverへの再利用を拒否").error.unwrap().code,
            "credential_not_available"
        );

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
        let audit = serde_json::to_string(&broker.audit_events()).expect("Audit");
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
        assert_eq!(normal_add.error.expect("error").code, "credential_owner_required");
        let owner_list = call(
            &mut broker,
            BrokerOperation::資格情報一覧,
            json!({"版": 1}),
            true,
        );
        assert_eq!(owner_list.status, BrokerStatus::Rejected);
        assert_eq!(owner_list.error.expect("error").code, "credential_normal_channel_required");
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn missing_or_changed_credential_storage_does_not_return_partial_list() {
        let (mut broker, root) = broker_with_vault("missing");
        let id = "e".repeat(32);
        assert_eq!(
            call(&mut broker, BrokerOperation::資格情報登録, fixture_payload(&id), true).status,
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
        assert_eq!(listed.error.expect("error").code, "credential_storage_missing");
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
