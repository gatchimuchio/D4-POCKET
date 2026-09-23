//! C6 回帰Caseのowner登録経路。
//!
//! 元の対話本文を自動コピーせず、Broker内の完了結果証跡とownerが明示した
//! サニタイズ済み定義を結合して、C5とは別purposeのProtectedStoreへ保管する。
#![allow(non_snake_case)]

use super::*;
use crate::audit_hash::sha256_tagged;
use crate::broker::dialogue::識別子生成;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const OPERATION: &str = "回帰Case登録";
const MAX_OWNER_REGRESSION_BYTES: usize = 48 * 1024;

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
    value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
