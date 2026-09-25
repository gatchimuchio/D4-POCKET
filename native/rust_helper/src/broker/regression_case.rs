//! C6 回帰Caseのowner登録と通常経路の公開一覧。
//!
//! 元の対話本文を自動コピーせず、Broker内の完了結果証跡とownerが明示した
//! サニタイズ済み定義を結合して、C5とは別purposeのProtectedStoreへ保管する。
#![allow(non_snake_case)]

use super::*;
use crate::audit_hash::sha256_tagged;
use crate::broker::dialogue::識別子生成;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;

const OPERATION: &str = "回帰Case登録";
const LIST_OPERATION: &str = "回帰Case一覧";
const MAX_OWNER_REGRESSION_BYTES: usize = 48 * 1024;
const MAX_LIST_PAGE: usize = 100;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct 一覧要求 {
    #[serde(rename = "版")]
    版: u8,
    #[serde(rename = "after")]
    after: u64,
    #[serde(rename = "limit")]
    limit: u16,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct 公開記録 {
    #[serde(rename = "版")]
    版: u8,
    #[serde(rename = "回帰CaseID")]
    case_id: String,
    #[serde(rename = "定義hash")]
    definition_hash: String,
    #[serde(rename = "非公開保管ID")]
    storage_id: String,
    #[serde(rename = "暗号文hash")]
    ciphertext_hash: String,
    #[serde(rename = "公開表示名")]
    display_name: String,
    #[serde(rename = "要求ID")]
    request_id: String,
    #[serde(rename = "要求hash")]
    request_hash: String,
    #[serde(rename = "実行系ID")]
    runtime_id: String,
    #[serde(rename = "結果状態")]
    result_status: String,
    #[serde(rename = "応答hash")]
    response_hash: String,
    #[serde(rename = "終了監査ID")]
    end_audit_id: String,
    #[serde(rename = "公開範囲")]
    exposure: String,
    #[serde(rename = "必要条件数")]
    required_condition_count: u16,
    #[serde(rename = "禁止条件数")]
    forbidden_condition_count: u16,
    #[serde(rename = "必要参照数")]
    required_reference_count: u16,
    #[serde(rename = "作成時刻UnixMillis")]
    created_at: i64,
    #[serde(rename = "作成監査ID")]
    created_audit_id: String,
    #[serde(rename = "証拠種別")]
    evidence_source: String,
}

impl 公開記録 {
    fn valid_for(&self, audit_event_id: &str) -> bool {
        self.版 == 1
            && hex_identifier(&self.case_id)
            && self.storage_id == self.case_id
            && super::is_tagged_sha256(&self.definition_hash)
            && super::is_tagged_sha256(&self.ciphertext_hash)
            && !self.display_name.trim().is_empty()
            && self.display_name.chars().count() <= 128
            && hex_identifier(&self.request_id)
            && super::is_tagged_sha256(&self.request_hash)
            && !self.runtime_id.is_empty()
            && self.runtime_id.len() <= 128
            && self
                .runtime_id
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
            && self
                .runtime_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
            && matches!(self.result_status.as_str(), "成功" | "保留")
            && super::is_tagged_sha256(&self.response_hash)
            && !self.end_audit_id.is_empty()
            && self.end_audit_id.len() <= 256
            && self.exposure == "hash_only"
            && self.required_condition_count <= 16
            && self.forbidden_condition_count <= 16
            && self.required_reference_count <= 64
            && self.created_at >= 0
            && self.created_audit_id == audit_event_id
            && self.evidence_source == EVIDENCE_SOURCE_INTERNAL_STATE
    }

    fn metadata_projection(&self) -> Value {
        json!({
            "回帰CaseID": self.case_id,
            "定義hash": self.definition_hash,
            "暗号文hash": self.ciphertext_hash,
            "公開表示名": self.display_name,
            "要求ID": self.request_id,
            "要求hash": self.request_hash,
            "実行系ID": self.runtime_id,
            "結果状態": self.result_status,
            "応答hash": self.response_hash,
            "終了監査ID": self.end_audit_id,
            "必要条件数": self.required_condition_count,
            "禁止条件数": self.forbidden_condition_count,
            "必要参照数": self.required_reference_count,
            "作成時刻UnixMillis": self.created_at,
            "作成監査ID": self.created_audit_id,
            "公開範囲": "metadata_only",
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        })
    }
}

fn stored_regression_cases(
    log: &super::BrokerAuditLog,
    after: u64,
    limit: usize,
) -> Result<(Vec<公開記録>, u64), ()> {
    let mut page = Vec::with_capacity(limit);
    let mut total = 0_u64;
    for event in log
        .events()
        .iter()
        .filter(|event| event.operation == OPERATION && event.decision == "accepted")
    {
        let encoded = event.reason.strip_prefix("回帰Case登録記録:").ok_or(())?;
        let record: 公開記録 = serde_json::from_str(encoded).map_err(|_| ())?;
        if event.evidence_source != EVIDENCE_SOURCE_INTERNAL_STATE
            || event.payload_hash != sha256_tagged(encoded.as_bytes())
            || !record.valid_for(&event.event_id)
        {
            return Err(());
        }
        if total >= after && page.len() < limit {
            page.push(record);
        }
        total = total.checked_add(1).ok_or(())?;
    }
    let mut page_ids = HashSet::with_capacity(page.len());
    if page
        .iter()
        .any(|record| !page_ids.insert(record.case_id.as_str()))
    {
        return Err(());
    }
    Ok((page, total))
}

fn parse_list_request(payload: &Value) -> Result<一覧要求, ()> {
    let request: 一覧要求 = serde_json::from_value(payload.clone()).map_err(|_| ())?;
    if request.版 != 1
        || request.limit == 0
        || request.limit as usize > MAX_LIST_PAGE
        || request.after > 9_007_199_254_740_991
    {
        return Err(());
    }
    Ok(request)
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct 入力指定 {
    #[serde(rename = "内容表示範囲")]
    内容表示範囲: String,
    #[serde(rename = "本文")]
    本文: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct 登録指定 {
    #[serde(rename = "版")]
    版: u8,
    #[serde(rename = "要求ID")]
    要求ID: String,
    #[serde(rename = "要求hash")]
    要求hash: String,
    #[serde(rename = "公開表示名")]
    公開表示名: String,
    #[serde(rename = "入力方式")]
    入力方式: String,
    #[serde(rename = "入力")]
    入力: 入力指定,
    #[serde(rename = "必要条件")]
    必要条件: Vec<String>,
    #[serde(rename = "禁止条件")]
    禁止条件: Vec<String>,
    #[serde(rename = "期待状態")]
    期待状態: String,
    #[serde(rename = "必要参照")]
    必要参照: Vec<String>,
    #[serde(rename = "期待経路")]
    期待経路: String,
}

fn hex_identifier(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

/// 既知のcredential表現だけを拒否する補助境界。任意の秘密値の不存在は証明しない。
fn contains_secret_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "api_key=",
        "api-key=",
        "token=",
        "password=",
        "bearer ",
        "openai_api_key",
        "codex_api_key",
        "github_pat_",
        "ghp_",
        "xoxb-",
        "xoxp-",
        "-----begin",
        "AIza",
        "sk-",
    ]
    .iter()
    .any(|marker| lower.contains(&marker.to_ascii_lowercase()))
}

fn contains_secret_marker_in_registration(value: &登録指定) -> bool {
    contains_secret_marker(&value.公開表示名)
        || contains_secret_marker(&value.入力.本文)
        || value.必要条件.iter().any(|item| contains_secret_marker(item))
        || value.禁止条件.iter().any(|item| contains_secret_marker(item))
        || value.必要参照.iter().any(|item| contains_secret_marker(item))
        || contains_secret_marker(&value.期待経路)
}

fn valid_registration(value: &登録指定) -> bool {
    value.版 == 1
        && hex_identifier(&value.要求ID)
        && super::is_tagged_sha256(&value.要求hash)
        && !value.公開表示名.trim().is_empty()
        && value.公開表示名.chars().count() <= 128
        && value.入力方式 == "owner_explicit_redacted"
        && value.入力.内容表示範囲 == "full"
        && !value.入力.本文.trim().is_empty()
        && value.入力.本文.chars().count() <= 4096
        && value.必要条件.len() <= 16
        && value.禁止条件.len() <= 16
        && value.必要条件.iter().all(|item| {
            !item.trim().is_empty() && item.chars().count() <= 512
        })
        && value.禁止条件.iter().all(|item| {
            !item.trim().is_empty() && item.chars().count() <= 512
        })
        && matches!(value.期待状態.as_str(), "成功" | "保留")
        && value.必要参照.len() <= 64
        && value
            .必要参照
            .iter()
            .all(|item| !item.trim().is_empty() && item.chars().count() <= 2048)
        && !value.期待経路.trim().is_empty()
        && value.期待経路.chars().count() <= 256
}

fn source_error_code(error: crate::broker::dialogue::対話失敗) -> &'static str {
    match error {
        crate::broker::dialogue::対話失敗::権限拒否 => "対話証跡不一致",
        crate::broker::dialogue::対話失敗::監査失敗 => "監査失敗",
        _ => "対話証跡不在",
    }
}

#[allow(non_snake_case)]
impl Broker {
    /// owner制御だけが、現在の完了済み対話から回帰Caseを登録する。
    pub(super) fn 回帰Case登録処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        owner: bool,
        payload_hash: &str,
    ) -> BrokerResponse {
        if !owner {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "権限拒否",
                "回帰Case登録にはowner制御資格が必要",
                true,
                payload_hash,
            );
        }
        if !self.state_store.persistence_ready() {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "broker_persistence_unavailable",
                "回帰Case登録には永続監査が必要",
                true,
                payload_hash,
            );
        }
        let registration = match serde_json::from_value::<登録指定>(payload.clone()) {
            Ok(value) if valid_registration(&value) => value,
            _ => {
                return self.reject_with_payload_hash(
                    request_id,
                    OPERATION,
                    "回帰Case不正",
                    "回帰Case登録は版、要求hash、owner明示のredacted定義だけを受け付ける",
                    true,
                    payload_hash,
                )
            }
        };
        if contains_secret_marker_in_registration(&registration) {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "秘密候補入力",
                "秘密候補を含む入力は保存せず、ownerがredactionして再登録してください",
                true,
                payload_hash,
            );
        }
        let _encoded_registration = match serde_json::to_vec(&registration) {
            Ok(value) if value.len() <= MAX_OWNER_REGRESSION_BYTES => value,
            _ => {
                return self.reject_with_payload_hash(
                    request_id,
                    OPERATION,
                    "回帰Case上限超過",
                    "回帰Case定義を上限内に縮小してowner制御から再登録してください",
                    true,
                    payload_hash,
                )
            }
        };

        let dialogue = std::mem::take(&mut self.対話);
        let source = dialogue.回帰Case情報(&registration.要求ID, &registration.要求hash);
        self.対話 = dialogue;
        let source = match source {
            Ok(value) => value,
            Err(error) => {
                return self.reject_with_payload_hash(
                    request_id,
                    OPERATION,
                    source_error_code(error),
                    "現在の完了済み対話の要求hash、結果証跡、終了監査を再確認してください",
                    true,
                    payload_hash,
                )
            }
        };
        if registration.期待状態 != source.結果状態 {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "対話結果不一致",
                "ownerが指定した期待状態と現在の対話結果状態が一致しません",
                true,
                payload_hash,
            );
        }
        if self
            .append_audit(
                request_id,
                OPERATION,
                "received",
                "owner制御から回帰Case登録要求を受信。private定義は監査へ保存しない",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                payload_hash,
            )
            .is_err()
        {
            return self.audit_store_failed_response(
                request_id,
                OPERATION,
                "監査失敗",
                "監査修復後に回帰Caseを再登録してください",
            );
        }

        #[cfg(not(windows))]
        {
            let _ = (_encoded_registration, source);
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "保管未対応",
                "回帰Caseのprivate保管はWindows ProtectedStoreが利用できる環境に限定されます",
                true,
                payload_hash,
            );
        }

        #[cfg(windows)]
        {
            let Some(store) = self.protected_store.as_ref() else {
                return self.reject_with_payload_hash(
                    request_id,
                    OPERATION,
                    "保管未登録",
                    "起動制御でWindows ProtectedStoreを登録してから再試行してください",
                    true,
                    payload_hash,
                );
            };
            let case_id = match 識別子生成() {
                Ok(value) => value,
                Err(_) => {
                    return self.reject_with_payload_hash(
                        request_id,
                        OPERATION,
                        "識別子生成失敗",
                        "回帰Case識別子を生成できません",
                        true,
                        payload_hash,
                    )
                }
            };
            let private = json!({
                "版": 1,
                "回帰CaseID": case_id.clone(),
                "要求ID": source.要求ID.clone(),
                "要求hash": source.要求hash.clone(),
                "実行系ID": source.実行系ID.clone(),
                "対話セッションID": source.対話セッションID.clone(),
                "結果状態": source.結果状態.clone(),
                "応答hash": source.応答hash.clone(),
                "終了監査ID": source.終了監査ID.clone(),
                "公開表示名": registration.公開表示名.clone(),
                "入力方式": registration.入力方式.clone(),
                "入力": registration.入力,
                "必要条件": registration.必要条件.clone(),
                "禁止条件": registration.禁止条件.clone(),
                "期待状態": registration.期待状態.clone(),
                "必要参照": registration.必要参照.clone(),
                "期待経路": registration.期待経路.clone(),
            });
            let encoded_private = match serde_json::to_vec(&private) {
                Ok(value) => value,
                Err(_) => {
                    return self.reject_with_payload_hash(
                        request_id,
                        OPERATION,
                        "回帰Case不正",
                        "private回帰Caseを正本化できません",
                        true,
                        payload_hash,
                    )
                }
            };
            let definition_hash = sha256_tagged(&encoded_private);
            let ciphertext_hash = match store.create(
                crate::protected_store::Purpose::Regression,
                &case_id,
                &encoded_private,
            ) {
                Ok(value) => value,
                Err(_) => {
                    return self.reject_with_payload_hash(
                        request_id,
                        OPERATION,
                        "保管失敗",
                        "既存または部分暗号文を再使用せず保管状態を確認してください",
                        true,
                        payload_hash,
                    )
                }
            };
            let audit_id = self.audit_log.next_event_id();
            let receipt = json!({
                "版": 1,
                "回帰CaseID": case_id.clone(),
                "定義hash": definition_hash,
                "非公開保管ID": case_id.clone(),
                "暗号文hash": ciphertext_hash,
                "公開表示名": registration.公開表示名.clone(),
                "要求ID": source.要求ID.clone(),
                "要求hash": source.要求hash.clone(),
                "実行系ID": source.実行系ID.clone(),
                "結果状態": source.結果状態.clone(),
                "応答hash": source.応答hash.clone(),
                "終了監査ID": source.終了監査ID.clone(),
                "公開範囲": "hash_only",
                "必要条件数": registration.必要条件.len(),
                "禁止条件数": registration.禁止条件.len(),
                "必要参照数": registration.必要参照.len(),
                "作成時刻UnixMillis": self.current_epoch_millis(),
                "作成監査ID": audit_id,
                "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
            });
            let encoded_receipt = receipt.to_string();
            match self.append_audit(
                request_id,
                OPERATION,
                "accepted",
                &format!("回帰Case登録記録:{encoded_receipt}"),
                EVIDENCE_SOURCE_INTERNAL_STATE,
                &sha256_tagged(encoded_receipt.as_bytes()),
            ) {
                Ok(event) if event.event_id == receipt["作成監査ID"] => BrokerResponse {
                    request_id: request_id.to_string(),
                    operation: OPERATION.to_string(),
                    status: BrokerStatus::Accepted,
                    evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
                    audit_event_id: event.event_id,
                    error: None,
                    health: None,
                    body: Some(receipt),
                    shutdown_requested: self.shutdown_requested,
                },
                _ => self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "監査失敗",
                    "暗号文を再使用せず保管監査を修復してください",
                ),
            }
        }
    }

    /// 公開receiptからmetadataだけを通常IPCへ返す。private定義は復号・投影しない。
    pub(super) fn 回帰Case一覧処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        owner: bool,
        payload_hash: &str,
    ) -> BrokerResponse {
        if owner {
            return self.reject_with_payload_hash(
                request_id,
                LIST_OPERATION,
                "通常経路限定",
                "回帰Case metadata一覧は通常IPCの読取専用操作です",
                true,
                payload_hash,
            );
        }
        if !self.state_store.persistence_ready() {
            return self.reject_with_payload_hash(
                request_id,
                LIST_OPERATION,
                "broker_persistence_unavailable",
                "回帰Case一覧には永続監査が必要です",
                true,
                payload_hash,
            );
        }
        let request = match parse_list_request(payload) {
            Ok(value) => value,
            Err(()) => {
                return self.reject_with_payload_hash(
                    request_id,
                    LIST_OPERATION,
                    "回帰Case一覧要求不正",
                    "版、cursor、limitだけを上限内で指定してください",
                    true,
                    payload_hash,
                )
            }
        };
        let (records, total) =
            match stored_regression_cases(&self.audit_log, request.after, request.limit as usize) {
                Ok(value) => value,
                Err(()) => {
                    return self.audit_store_failed_response(
                        request_id,
                        LIST_OPERATION,
                        "回帰Case監査不正",
                        "回帰Case登録Auditを検証できないため一覧を停止しました",
                    )
                }
            };
        if request.after > total {
            return self.reject_with_payload_hash(
                request_id,
                LIST_OPERATION,
                "回帰Case cursor不正",
                "一覧cursorが現在のCase件数を超えています。先頭から再読込してください",
                true,
                payload_hash,
            );
        }
        if self
            .append_audit(
                request_id,
                LIST_OPERATION,
                "received",
                "公開metadataのみの回帰Case一覧を要求",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                payload_hash,
            )
            .is_err()
        {
            return self.audit_store_failed_response(
                request_id,
                LIST_OPERATION,
                "回帰Case監査記録失敗",
                "回帰Case一覧の受信Auditを確定できません",
            );
        }

        #[cfg(not(windows))]
        {
            return self.reject_with_payload_hash(
                request_id,
                LIST_OPERATION,
                "保管未対応",
                "回帰Case保管の照合はWindows ProtectedStore環境に限定されています",
                true,
                payload_hash,
            );
        }

        #[cfg(windows)]
        {
            let Some(store) = self.protected_store.as_ref() else {
                return self.reject_with_payload_hash(
                    request_id,
                    LIST_OPERATION,
                    "保管未登録",
                    "起動制御でWindows ProtectedStoreを登録してから再試行してください",
                    true,
                    payload_hash,
                );
            };
            let end = request.after.saturating_add(records.len() as u64);
            for record in &records {
                match store.inspect(
                    crate::protected_store::Purpose::Regression,
                    &record.storage_id,
                ) {
                    Ok(Some((hash, _))) if hash == record.ciphertext_hash => {}
                    Ok(Some(_)) => {
                        return self.reject_with_payload_hash(
                            request_id,
                            LIST_OPERATION,
                            "回帰Case保管改変",
                            "暗号文のhashが登録Auditと一致しないため部分一覧を返しません",
                            true,
                            payload_hash,
                        )
                    }
                    Ok(None) | Err(_) => {
                        return self.reject_with_payload_hash(
                            request_id,
                            LIST_OPERATION,
                            "回帰Case保管欠落",
                            "暗号文の欠落または安全な読取失敗を部分一覧へ変換しません",
                            true,
                            payload_hash,
                        )
                    }
                }
            }
            let items: Vec<Value> = records.iter().map(公開記録::metadata_projection).collect();
            let body = json!({
                "版": 1,
                "回帰Case一覧": items,
                "件数": records.len(),
                "合計件数": total,
                "次cursor": if end < total { Some(end) } else { None },
                "公開範囲": "metadata_only",
                "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
            });
            let event = match self.append_audit(
                request_id,
                LIST_OPERATION,
                "accepted",
                "回帰Caseのmetadataだけを返却。private定義は復号・投影しない",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                &super::canonical_payload_hash(Some(&body)),
            ) {
                Ok(event) => event,
                Err(_) => {
                    return self.audit_store_failed_response(
                        request_id,
                        LIST_OPERATION,
                        "回帰Case監査記録失敗",
                        "回帰Case一覧の結果Auditを確定できません",
                    )
                }
            };
            BrokerResponse {
                request_id: request_id.to_string(),
                operation: LIST_OPERATION.to_string(),
                status: BrokerStatus::Accepted,
                evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
                audit_event_id: event.event_id,
                error: None,
                health: None,
                body: Some(body),
                shutdown_requested: self.shutdown_requested,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    use std::io::Write;

    #[cfg(windows)]
    fn broker_with_store(name: &str) -> (Broker, std::path::PathBuf) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("gui-shell-regression-{name}-{stamp}"));
        let audit = root.join("audit");
        let vault = root.join("vault");
        std::fs::create_dir_all(&vault).expect("vault");
        let mut broker =
            Broker::new_persistent("session-regression", &audit).expect("永続Brokerを作成");
        broker.current_epoch_seconds_override = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_secs() as i64,
        );
        broker
            .保管先起動登録(&vault, true, std::slice::from_ref(&audit))
            .expect("保護保管先を登録");
        (broker, root)
    }

    #[cfg(windows)]
    fn seed_case(broker: &mut Broker, sequence: usize) -> String {
        let case_id = format!("{sequence:032x}");
        let private = format!("synthetic-private-input-{sequence}");
        let ciphertext_hash = broker
            .protected_store
            .as_ref()
            .expect("保護保管先を取得")
            .create(
                crate::protected_store::Purpose::Regression,
                &case_id,
                private.as_bytes(),
            )
            .expect("暗号化保存");
        let created_audit_id = broker.audit_log.next_event_id();
        let receipt = json!({
            "版": 1,
            "回帰CaseID": case_id,
            "定義hash": sha256_tagged(private.as_bytes()),
            "非公開保管ID": case_id,
            "暗号文hash": ciphertext_hash,
            "公開表示名": format!("回帰試験{sequence}"),
            "要求ID": "a".repeat(32),
            "要求hash": format!("sha256:{}", "b".repeat(64)),
            "実行系ID": "codex",
            "結果状態": "成功",
            "応答hash": format!("sha256:{}", "c".repeat(64)),
            "終了監査ID": "audit-dialogue-end",
            "公開範囲": "hash_only",
            "必要条件数": 1,
            "禁止条件数": 0,
            "必要参照数": 1,
            "作成時刻UnixMillis": 1780000000000_i64,
            "作成監査ID": created_audit_id,
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        });
        let encoded = receipt.to_string();
        broker
            .append_audit(
                "regression-case-fixture",
                OPERATION,
                "accepted",
                &format!("回帰Case登録記録:{encoded}"),
                EVIDENCE_SOURCE_INTERNAL_STATE,
                &sha256_tagged(encoded.as_bytes()),
            )
            .expect("登録監査fixture");
        case_id
    }

    #[cfg(windows)]
    fn list(broker: &mut Broker, payload: Value, owner: bool) -> BrokerResponse {
        let request_id = format!("regression-list-{}", broker.audit_events().len());
        let mut envelope = BrokerRequestEnvelope::command_envelope_at(
            &request_id,
            "session-regression",
            &format!("nonce-{request_id}"),
            &BrokerRequestEnvelope::current_issued_at(),
        );
        envelope.operation = Some(BrokerOperation::回帰Case一覧);
        envelope.payload = Some(payload);
        envelope.refresh_payload_hash();
        broker.処理(envelope, owner)
    }

    fn append_receipt_fixture(log: &mut BrokerAuditLog, sequence: usize) {
        let case_id = format!("{sequence:032x}");
        let created_audit_id = log.next_event_id();
        let receipt = json!({
            "版": 1,
            "回帰CaseID": case_id,
            "定義hash": format!("sha256:{}", "d".repeat(64)),
            "非公開保管ID": case_id,
            "暗号文hash": format!("sha256:{}", "e".repeat(64)),
            "公開表示名": format!("回帰試験{sequence}"),
            "要求ID": "a".repeat(32),
            "要求hash": format!("sha256:{}", "b".repeat(64)),
            "実行系ID": "codex",
            "結果状態": "成功",
            "応答hash": format!("sha256:{}", "c".repeat(64)),
            "終了監査ID": "audit-dialogue-end",
            "公開範囲": "hash_only",
            "必要条件数": 1,
            "禁止条件数": 0,
            "必要参照数": 1,
            "作成時刻UnixMillis": 1780000000000_i64,
            "作成監査ID": created_audit_id,
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        });
        let encoded = receipt.to_string();
        log.append(
            "regression-list-fixture",
            OPERATION,
            "accepted",
            &format!("回帰Case登録記録:{encoded}"),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &sha256_tagged(encoded.as_bytes()),
        );
    }

    #[test]
    fn 一覧要求は厳格にbounded_pageへ制限する() {
        assert!(parse_list_request(&json!({"版":1,"after":0,"limit":100})).is_ok());
        assert!(parse_list_request(&json!({"版":1,"after":0,"limit":0})).is_err());
        assert!(parse_list_request(&json!({"版":1,"after":0,"limit":101})).is_err());
        assert!(parse_list_request(&json!({"版":1,"after":-1,"limit":10})).is_err());
        assert!(
            parse_list_request(&json!({"版":1,"after":0,"limit":10,"approval_id":"x"})).is_err()
        );
    }

    #[test]
    fn 登録監査からbounded_metadata_pageだけを射影する() {
        let mut log = BrokerAuditLog::default();
        append_receipt_fixture(&mut log, 1);
        append_receipt_fixture(&mut log, 2);
        let (page, total) = stored_regression_cases(&log, 1, 1).expect("公開receiptページ");
        assert_eq!(total, 2);
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].display_name, "回帰試験2");
        let projection = page[0].metadata_projection();
        assert!(projection.get("非公開保管ID").is_none());
        assert!(projection.get("期待経路").is_none());
        assert_eq!(projection["公開範囲"], "metadata_only");
    }

    #[test]
    fn 既知の秘密markerだけを拒否し入力本文を返さない() {
        assert!(contains_secret_marker("OPENAI_API_KEY=redacted"));
        assert!(contains_secret_marker("Bearer abc"));
        assert!(contains_secret_marker("sk-test"));
        assert!(!contains_secret_marker("作業状態を要約してください"));
    }

    #[test]
    fn 登録定義はauthorityや自動コピー方式を受け付けない() {
        let valid = json!({
            "版": 1,
            "要求ID": "a".repeat(32),
            "要求hash": format!("sha256:{}", "b".repeat(64)),
            "公開表示名": "case",
            "入力方式": "owner_explicit_redacted",
            "入力": {"内容表示範囲": "full", "本文": "入力"},
            "必要条件": [],
            "禁止条件": [],
            "期待状態": "成功",
            "必要参照": [],
            "期待経路": "codex"
        });
        let parsed: 登録指定 = serde_json::from_value(valid).expect("登録定義を読める");
        assert!(valid_registration(&parsed));
        let mut authority = serde_json::to_value(&parsed).unwrap();
        authority["permission_id"] = json!("permission.injected");
        assert!(serde_json::from_value::<登録指定>(authority).is_err());
        let mut copied = serde_json::to_value(&parsed).unwrap();
        copied["入力方式"] = json!("auto_from_dialogue");
        let copied: 登録指定 = serde_json::from_value(copied).expect("構造は同じでも方式は別");
        assert!(!valid_registration(&copied));
    }

    #[cfg(windows)]
    #[test]
    fn 通常metadata一覧はページ化しprivate定義を返さない() {
        let (mut broker, root) = broker_with_store("paged-list");
        seed_case(&mut broker, 1);
        seed_case(&mut broker, 2);

        let first = list(&mut broker, json!({"版":1,"after":0,"limit":1}), false);
        assert_eq!(first.status, BrokerStatus::Accepted, "{first:?}");
        let first_body = first.body.expect("先頭ページ");
        assert_eq!(first_body["件数"], 1);
        assert_eq!(first_body["合計件数"], 2);
        assert_eq!(first_body["次cursor"], 1);
        assert_eq!(first_body["公開範囲"], "metadata_only");
        let first_json = serde_json::to_string(&first_body).expect("JSON");
        assert!(!first_json.contains("synthetic-private-input"));
        assert!(!first_json.contains("非公開保管ID"));

        let second = list(&mut broker, json!({"版":1,"after":1,"limit":1}), false);
        assert_eq!(second.status, BrokerStatus::Accepted, "{second:?}");
        let second_body = second.body.expect("次ページ");
        assert_eq!(second_body["件数"], 1);
        assert_eq!(second_body["次cursor"], Value::Null);

        let owner = list(&mut broker, json!({"版":1,"after":0,"limit":10}), true);
        assert_eq!(owner.status, BrokerStatus::Rejected);
        assert_eq!(owner.error.expect("error").code, "通常経路限定");
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn cursor外れ権限fieldと破損保管を拒否する() {
        let (mut broker, root) = broker_with_store("negative-list");
        let case_id = seed_case(&mut broker, 3);
        let invalid = list(&mut broker, json!({"版":1,"after":0,"limit":101}), false);
        assert_eq!(invalid.status, BrokerStatus::Rejected);
        let authority = list(
            &mut broker,
            json!({"版":1,"after":0,"limit":10,"approval_id":"injected"}),
            false,
        );
        assert_eq!(authority.status, BrokerStatus::Rejected);
        let stale = list(&mut broker, json!({"版":1,"after":2,"limit":10}), false);
        assert_eq!(stale.status, BrokerStatus::Rejected);
        assert_eq!(stale.error.expect("error").code, "回帰Case cursor不正");

        let ciphertext = root
            .join("vault")
            .join(format!("regression-{case_id}.dpapi"));
        let mut changed_file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&ciphertext)
            .expect("暗号文");
        changed_file
            .write_all(b"synthetic-tampered-ciphertext")
            .expect("改変");
        let changed = list(&mut broker, json!({"版":1,"after":0,"limit":10}), false);
        assert_eq!(changed.status, BrokerStatus::Rejected);
        assert!(changed.body.is_none());
        assert_eq!(changed.error.expect("error").code, "回帰Case保管改変");
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn 欠落暗号文と不正登録監査を一覧へ昇格しない() {
        let (mut broker, root) = broker_with_store("missing-list");
        let case_id = seed_case(&mut broker, 4);
        let ciphertext = root
            .join("vault")
            .join(format!("regression-{case_id}.dpapi"));
        std::fs::remove_file(&ciphertext).expect("暗号文削除");
        let missing = list(&mut broker, json!({"版":1,"after":0,"limit":10}), false);
        assert_eq!(missing.status, BrokerStatus::Rejected);
        assert!(missing.body.is_none());
        assert_eq!(missing.error.expect("error").code, "回帰Case保管欠落");

        broker
            .append_audit(
                "malformed-regression-fixture",
                OPERATION,
                "accepted",
                "回帰Case登録記録:{malformed}",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                &sha256_tagged(b"{malformed}"),
            )
            .expect("不正監査fixture");
        let malformed = list(&mut broker, json!({"版":1,"after":0,"limit":10}), false);
        assert_eq!(malformed.status, BrokerStatus::Rejected);
        assert!(malformed.body.is_none());
        assert_eq!(malformed.error.expect("error").code, "回帰Case監査不正");
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
