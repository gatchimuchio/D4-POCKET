//! C5 評価ラボのBroker統治接続。
//!
//! private Datasetはowner制御だけで登録し、Windowsの`Purpose::Evaluation`へ暗号化して
//! 保管する。通常IPC、監査reason、公開結果はhash_onlyのmanifestまたは安全な判定だけを
//! 扱う。実行は既存対話のSession作成・送信・owner承認・取得を呼ぶだけであり、Adapter、
//! endpoint、commandを直接呼ばない。

use super::*;
use crate::audit_hash::sha256_tagged;
use crate::broker::dialogue::{対話開始監査hash, 識別子生成, 評価対話進捗};
use crate::broker::evaluation_lab::evaluate_case_public_projection;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};

const EVALUATION_VERSION: u64 = 1;
const MAX_DATASETS_CASES: usize = 128;
const MAX_TARGET_RUNTIMES: usize = 8;
const MAX_EVALUATORS_PER_CASE: usize = 16;
const MAX_OWNER_DATASET_BYTES: usize = 48 * 1024;

/// runtimeのprivateな入力や評価条件を`Debug`で露出しないため、手動実装にする。
#[derive(Default)]
pub(super) struct EvaluationControl {
    active: BTreeMap<String, ActiveExperiment>,
}

impl std::fmt::Debug for EvaluationControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvaluationControl")
            .field("active_experiment_count", &self.active.len())
            .finish()
    }
}

struct ActiveExperiment {
    public: Value,
    cases: Vec<PrivateCase>,
    plans: Vec<EvaluationPlan>,
}

struct PrivateRegistration {
    dataset_id: String,
    revision: u64,
    public_name: String,
    cases: Vec<PrivateCase>,
}

struct PrivateCase {
    case_id: String,
    order: u64,
    input: String,
    evaluators: Value,
    definition_hash: String,
}

struct EvaluationPlan {
    case_id: String,
    runtime_id: String,
    request_id: String,
    request_hash: String,
    session_id: String,
    session_start_request_audit_id: String,
    session_start_audit_id: String,
    send_request_audit_id: String,
    /// private入力を復元せずに、既存対話送信へ渡した正規payloadだけを結び付けるhash。
    send_payload_hash: String,
    created_audit_id: String,
    isolation_request_audit_id: String,
    isolation_audit_id: String,
    state: String,
    result: Option<Value>,
    observation: Option<ObservationFingerprints>,
    /// 結果監査後に既存対話の終了が完了していない間だけtrueにする。
    /// resultを先に確定しても、worker受信側が残る場合は次の状態照会で終了を再試行する。
    cleanup_pending: bool,
    /// 通常の対話終了監査を確認済みの場合だけtrueにする。C4 terminal隔離による
    /// in-memory回収は比較可能性の証拠に昇格させない。
    cleanup_verified: bool,
}

/// 再起動後にもCaseと既存対話の結合を検証するため、監査chainだけへ保存する安全な計画。
/// private入力、Evaluator設定、応答、Adapter接続先は含めない。
#[derive(Clone)]
struct PersistedEvaluationPlan {
    case_id: String,
    runtime_id: String,
    request_id: String,
    request_hash: String,
    session_id: String,
    session_start_request_audit_id: String,
    session_start_audit_id: String,
    send_request_audit_id: String,
    send_payload_hash: String,
    created_audit_id: String,
    isolation_request_audit_id: String,
    isolation_audit_id: String,
}

#[derive(Clone)]
struct PersistedEvaluationPlanRecord {
    experiment_id: String,
    dataset_id: String,
    definition_hash: String,
    plans: Vec<PersistedEvaluationPlan>,
}

#[derive(Clone)]
struct RecoveredExperiment {
    public: Value,
    plan_record: PersistedEvaluationPlanRecord,
    plan_event_index: usize,
    experiment_event_index: usize,
    outer_request_id: String,
}

#[derive(Clone)]
struct ObservationFingerprints {
    route: Option<String>,
    reference: Option<String>,
    capability: Option<String>,
}

#[derive(Clone)]
struct DatasetManifest {
    value: Value,
    dataset_id: String,
    revision: u64,
    definition_hash: String,
    storage_id: String,
    ciphertext_hash: String,
}

#[allow(non_snake_case)]
impl Broker {
    /// owner制御経路だけがprivate Datasetを登録する。本文は決して監査reasonや応答へ戻さない。
    pub(super) fn 評価Dataset登録処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        owner: bool,
        payload_hash: &str,
    ) -> BrokerResponse {
        const OPERATION: &str = "評価Dataset登録";
        if !owner {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "権限拒否",
                "評価Dataset登録にはowner制御資格が必要",
                true,
                payload_hash,
            );
        }
        if !self.state_store.persistence_ready() {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "broker_persistence_unavailable",
                "評価Dataset登録には永続監査が必要",
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
                    "評価Dataset不正",
                    reason,
                    true,
                    payload_hash,
                )
            }
        };
        let encoded_private = match serde_json::to_vec(payload) {
            Ok(value) if value.len() <= MAX_OWNER_DATASET_BYTES => value,
            _ => {
                return self.reject_with_payload_hash(
                    request_id,
                    OPERATION,
                    "評価Dataset上限超過",
                    "private Datasetを上限内に分割してowner制御から再登録してください",
                    true,
                    payload_hash,
                )
            }
        };
        let previous = match self.評価検証済み監査() {
            Ok(value) => value,
            Err(()) => {
                return self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "監査失敗",
                    "監査修復後に評価Datasetを再登録してください",
                )
            }
        };
        let manifests = match evaluation_manifests(&previous) {
            Ok(value) => value,
            Err(_) => {
                return self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "監査失敗",
                    "既存評価Dataset記録を検証できません",
                )
            }
        };
        if manifests.iter().any(|manifest| {
            manifest.dataset_id == registration.dataset_id && manifest.revision == registration.revision
        }) {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "評価Dataset重複",
                "同じDataset revisionは再使用せずrevisionを進めてください",
                true,
                payload_hash,
            );
        }
        if manifests
            .iter()
            .filter(|manifest| manifest.dataset_id == registration.dataset_id)
            .map(|manifest| manifest.revision)
            .max()
            .is_some_and(|latest| registration.revision <= latest)
        {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "評価Dataset revision後退",
                "同じDataset IDのrevisionは既存の最大revisionより大きくしてください",
                true,
                payload_hash,
            );
        }
        if self
            .append_audit(
                request_id,
                OPERATION,
                "received",
                "owner制御から評価Dataset登録要求を受信。private本文は監査へ保存しない",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                payload_hash,
            )
            .is_err()
        {
            return self.audit_store_failed_response(
                request_id,
                OPERATION,
                "監査失敗",
                "監査修復後に評価Datasetを再登録してください",
            );
        }

        #[cfg(not(windows))]
        {
            let _ = (registration, encoded_private);
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "保管未対応",
                "評価Datasetのprivate保管はWindows ProtectedStoreがある環境だけで登録できます",
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
            let storage_id = storage_id(&registration.dataset_id, registration.revision);
            let ciphertext_hash = match store.create(
                crate::protected_store::Purpose::Evaluation,
                &storage_id,
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
            let manifest = match public_manifest(
                payload,
                &registration,
                &storage_id,
                &ciphertext_hash,
                self.current_epoch_millis(),
                &audit_id,
            ) {
                Ok(value) => value,
                Err(_) => {
                    return self.audit_store_failed_response(
                        request_id,
                        OPERATION,
                        "監査失敗",
                        "評価Datasetの公開manifestを確定できません",
                    )
                }
            };
            let encoded = manifest.to_string();
            match self.append_audit(
                request_id,
                OPERATION,
                "accepted",
                &format!("評価Dataset登録記録:{encoded}"),
                EVIDENCE_SOURCE_INTERNAL_STATE,
                &canonical_payload_hash(Some(&manifest)),
            ) {
                Ok(event) if event.event_id == audit_id => success_response(
                    request_id,
                    OPERATION,
                    event.event_id,
                    manifest,
                ),
                _ => self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "監査失敗",
                    "暗号文を再使用せず監査修復後に新revisionで再登録してください",
                ),
            }
        }
    }

    /// 評価ラボの通常経路。owner資格では読取・開始・状態・比較を実行できない。
    pub(super) fn 評価通常要求処理(
        &mut self,
        request_id: &str,
        operation: &str,
        payload: &Value,
        owner: bool,
        payload_hash: &str,
    ) -> BrokerResponse {
        if owner {
            return self.reject_with_payload_hash(
                request_id,
                operation,
                "evaluation_normal_channel_required",
                "評価ラボの通常操作は通常資格経路だけが実行できます",
                true,
                payload_hash,
            );
        }
        if !self.state_store.persistence_ready() {
            return self.reject_with_payload_hash(
                request_id,
                operation,
                "broker_persistence_unavailable",
                "評価ラボには永続監査が必要",
                true,
                payload_hash,
            );
        }
        match operation {
            "評価Dataset一覧" => self.評価Dataset一覧処理(request_id, payload, payload_hash),
            "評価実験開始" => self.評価実験開始処理(request_id, payload, payload_hash),
            "評価実験状態" => self.評価実験状態処理(request_id, payload, payload_hash),
            "評価比較" => self.評価比較処理(request_id, payload, payload_hash),
            _ => self.reject_with_payload_hash(
                request_id,
                operation,
                "要求不正",
                "評価ラボ操作が不正です",
                true,
                payload_hash,
            ),
        }
    }

    fn 評価Dataset一覧処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        payload_hash: &str,
    ) -> BrokerResponse {
        const OPERATION: &str = "評価Dataset一覧";
        if !empty_version_payload(payload) {
            return self.reject_with_payload_hash(
                request_id,
                OPERATION,
                "要求不正",
                "評価Dataset一覧は版だけを受け付けます",
                true,
                payload_hash,
            );
        }
        let log = match self.評価検証済み監査() {
            Ok(value) => value,
            Err(()) => {
                return self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "監査失敗",
                    "監査修復後に評価Dataset一覧を再確認してください",
                )
            }
        };
        let manifests = match evaluation_manifests(&log) {
            Ok(value) => value,
            Err(_) => {
                return self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "監査失敗",
                    "評価Dataset記録を検証できません",
                )
            }
        };
        if self
            .append_audit(
                request_id,
                OPERATION,
                "received",
                "評価Dataset公開manifest一覧を要求",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                payload_hash,
            )
            .is_err()
        {
            return self.audit_store_failed_response(request_id, OPERATION, "監査失敗", "監査修復後に再確認してください");
        }
        // 通常UIへはDatasetごとの現行revisionだけを返す。旧revisionは監査chain内の
        // immutable recordとしてのみ保持し、UIのDataset ID重複を発生させない。
        let mut latest = BTreeMap::new();
        for manifest in manifests {
            latest.insert(manifest.dataset_id.clone(), manifest.value);
        }
        let body = json!({
            "版": EVALUATION_VERSION,
            "評価Dataset一覧": latest.into_values().collect::<Vec<_>>(),
        });
        accept_safe_body(self, request_id, OPERATION, body)
    }

    fn 評価実験開始処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        payload_hash: &str,
    ) -> BrokerResponse {
        const OPERATION: &str = "評価実験開始";
        let (dataset_id, runtimes) = match start_request(payload) {
            Ok(value) => value,
            Err(reason) => {
                return self.reject_with_payload_hash(request_id, OPERATION, "要求不正", reason, true, payload_hash)
            }
        };
        let log = match self.評価検証済み監査() {
            Ok(value) => value,
            Err(()) => return self.audit_store_failed_response(request_id, OPERATION, "監査失敗", "監査修復後に評価実験を開始してください"),
        };
        let manifests = match evaluation_manifests(&log) {
            Ok(value) => value,
            Err(_) => return self.audit_store_failed_response(request_id, OPERATION, "監査失敗", "評価Dataset記録を検証できません"),
        };
        let Some(manifest) = manifests
            .iter()
            .filter(|value| value.dataset_id == dataset_id)
            .max_by_key(|value| value.revision)
            .cloned()
        else {
            return self.reject_with_payload_hash(request_id, OPERATION, "評価Dataset不在", "登録済み評価Datasetを指定してください", true, payload_hash);
        };
        let registration = match self.評価privateDataset読取(&manifest) {
            Ok(value) => value,
            Err(reason) => return self.reject_with_payload_hash(request_id, OPERATION, "評価Dataset読取失敗", reason, true, payload_hash),
        };
        if registration.cases.len().saturating_mul(runtimes.len()) > self.対話.評価要求可能数() {
            return self.reject_with_payload_hash(request_id, OPERATION, "対話枠不足", "既存の対話を完了または中止してから評価実験を開始してください", true, payload_hash);
        }
        for runtime_id in &runtimes {
            if self.ライフサイクル.is_terminally_quarantined(runtime_id) {
                return self.reject_with_payload_hash(request_id, OPERATION, "実行系隔離", "terminal隔離中の実行系は評価できません", true, payload_hash);
            }
            if !self.対話.登録済み(runtime_id) {
                return self.reject_with_payload_hash(request_id, OPERATION, "実行系不在", "登録済み実行系だけを評価対象に指定してください", true, payload_hash);
            }
        }
        if self
            .append_audit(
                request_id,
                OPERATION,
                "received",
                "評価実験開始を要求。Caseごとの対話はowner承認待ちだけを作成する",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                payload_hash,
            )
            .is_err()
        {
            return self.audit_store_failed_response(request_id, OPERATION, "監査失敗", "監査修復後に評価実験を開始してください");
        }
        let experiment_id = match 識別子生成() {
            Ok(value) => value,
            Err(_) => return self.reject_with_payload_hash(request_id, OPERATION, "乱数失敗", "評価実験IDを生成できません", true, payload_hash),
        };
        let mut plans = Vec::with_capacity(registration.cases.len() * runtimes.len());
        for case in &registration.cases {
            for runtime_id in &runtimes {
                let (session, session_audit_id, session_start_request_audit_id) = match self.評価対話操作(
                    request_id,
                    "対話開始",
                    &json!({"実行系ID": runtime_id}),
                ) {
                    Ok(value) => value,
                    Err(reason) => {
                        self.評価対話取消(&plans, request_id);
                        return self.reject_with_payload_hash(request_id, OPERATION, "対話作成失敗", reason, true, payload_hash);
                    }
                };
                let Some(session_id) = session.get("対話セッションID").and_then(Value::as_str).filter(|value| valid_identifier(value)).map(str::to_owned) else {
                    self.評価対話取消(&plans, request_id);
                    if let Some(recovered_session_id) = self.評価対話監査対象(&session_audit_id, "対話開始") {
                        self.評価対話を取消して終了(request_id, None, &recovered_session_id);
                    }
                    return self.audit_store_failed_response(request_id, OPERATION, "監査失敗", "対話Sessionを確定できません");
                };
                if self
                    .評価対話監査対象(&session_audit_id, "対話開始")
                    .as_deref()
                    != Some(session_id.as_str())
                {
                    self.評価対話取消(&plans, request_id);
                    self.評価対話を取消して終了(request_id, None, &session_id);
                    return self.audit_store_failed_response(
                        request_id,
                        OPERATION,
                        "監査失敗",
                        "対話Session開始監査を確定できません",
                    );
                }
                let send_payload = json!({
                    "対話セッションID": session_id,
                    "入力": case.input,
                });
                let send_payload_hash = canonical_payload_hash(Some(&send_payload));
                let (pending, pending_audit_id, send_request_audit_id) = match self.評価対話操作(
                    request_id,
                    "対話送信",
                    &send_payload,
                ) {
                    Ok(value) => value,
                    Err(reason) => {
                        self.評価対話取消(&plans, request_id);
                        self.評価対話を取消して終了(request_id, None, &session_id);
                        return self.reject_with_payload_hash(request_id, OPERATION, "対話作成失敗", reason, true, payload_hash);
                    }
                };
                if !self.audit_log.events().iter().any(|event| {
                    event.event_id == send_request_audit_id
                        && send_request_event_matches(event, request_id, &send_payload_hash)
                }) {
                    self.評価対話取消(&plans, request_id);
                    self.評価対話を取消して終了(request_id, None, &session_id);
                    return self.audit_store_failed_response(
                        request_id,
                        OPERATION,
                        "監査失敗",
                        "対話送信要求監査を確定できません",
                    );
                }
                let Some(request_id_value) = pending.get("要求ID").and_then(Value::as_str).filter(|value| valid_identifier(value)).map(str::to_owned) else {
                    self.評価対話取消(&plans, request_id);
                    let recovered_request_id = self.評価対話監査対象(&pending_audit_id, "対話送信");
                    self.評価対話を取消して終了(
                        request_id,
                        recovered_request_id.as_deref(),
                        &session_id,
                    );
                    return self.audit_store_failed_response(request_id, OPERATION, "監査失敗", "対話要求を確定できません");
                };
                let Some(request_hash) = pending
                    .get("要求hash")
                    .and_then(Value::as_str)
                    .filter(|value| valid_hash(value))
                    .map(str::to_owned)
                else {
                    self.評価対話取消(&plans, request_id);
                    self.評価対話を取消して終了(request_id, Some(&request_id_value), &session_id);
                    return self.audit_store_failed_response(
                        request_id,
                        OPERATION,
                        "監査失敗",
                        "対話要求hashを確定できません",
                    );
                };
                let (isolation_audit_id, isolation_request_audit_id) =
                    match self.評価対話隔離(request_id, &request_id_value) {
                    Ok((recorded, received))
                        if valid_audit_id(&recorded) && valid_audit_id(&received) =>
                    {
                        (recorded, received)
                    }
                    _ => {
                        self.評価対話取消(&plans, request_id);
                        self.評価対話を取消して終了(request_id, Some(&request_id_value), &session_id);
                        return self.audit_store_failed_response(
                            request_id,
                            OPERATION,
                            "監査失敗",
                            "評価対話の通常取得隔離を確定できません",
                        );
                    }
                };
                let creation = match self.評価対話進捗(request_id, &request_id_value) {
                    Ok(value) => value,
                    Err(_) => {
                        self.評価対話取消(&plans, request_id);
                        self.評価対話を取消して終了(request_id, Some(&request_id_value), &session_id);
                        return self.audit_store_failed_response(
                            request_id,
                            OPERATION,
                            "監査失敗",
                            "対話作成監査を確定できません",
                        );
                    }
                };
                let Some(created_audit_id) = creation
                    .実行記録
                    .get("作成監査ID")
                    .and_then(Value::as_str)
                    .filter(|value| valid_audit_id(value))
                    .map(str::to_owned)
                else {
                    self.評価対話取消(&plans, request_id);
                    self.評価対話を取消して終了(request_id, Some(&request_id_value), &session_id);
                    return self.audit_store_failed_response(
                        request_id,
                        OPERATION,
                        "監査失敗",
                        "対話作成監査を確定できません",
                    );
                };
                plans.push(EvaluationPlan {
                    case_id: case.case_id.clone(),
                    runtime_id: runtime_id.clone(),
                    request_id: request_id_value,
                    request_hash,
                    session_id,
                    session_start_request_audit_id,
                    session_start_audit_id: session_audit_id,
                    send_request_audit_id,
                    send_payload_hash,
                    created_audit_id,
                    isolation_request_audit_id,
                    isolation_audit_id,
                    state: "承認待ち".to_owned(),
                    result: None,
                    observation: None,
                    cleanup_pending: false,
                    cleanup_verified: false,
                });
            }
        }
        let plan_record = match persisted_plan_record_value(&experiment_id, &manifest, &plans) {
            Ok(value) => value,
            Err(_) => {
                self.評価対話取消(&plans, request_id);
                return self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "監査失敗",
                    "評価Experiment計画recordを確定できません",
                );
            }
        };
        let expected_plan_audit_id = self.audit_log.next_event_id();
        let encoded_plan = plan_record.to_string();
        let plan_audit = self.append_audit(
            request_id,
            OPERATION,
            "recorded",
            &format!("評価Experiment計画記録:{encoded_plan}"),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &canonical_payload_hash(Some(&plan_record)),
        );
        let plan_audit_id = match plan_audit {
            Ok(event) if event.event_id == expected_plan_audit_id => event.event_id,
            _ => {
                self.評価対話取消(&plans, request_id);
                return self.audit_store_failed_response(
                    request_id,
                    OPERATION,
                    "監査失敗",
                    "評価Experiment計画監査を確定できません",
                );
            }
        };
        let audit_id = self.audit_log.next_event_id();
        let public = json!({
            "版": EVALUATION_VERSION,
            "評価ExperimentID": experiment_id,
            "評価DatasetID": manifest.dataset_id,
            "Dataset定義hash": manifest.definition_hash,
            "対象Runtime一覧": runtimes,
            "状態": "承認待ち",
            "計画Case数": registration.cases.len(),
            "結果数": 0,
            "作成時刻UnixMillis": self.current_epoch_millis(),
            "開始時刻UnixMillis": Value::Null,
            "終了時刻UnixMillis": Value::Null,
            "計画監査ID": plan_audit_id,
            "実験監査ID": audit_id,
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        });
        if !valid_experiment(&public) {
            self.評価対話取消(&plans, request_id);
            return self.audit_store_failed_response(request_id, OPERATION, "監査失敗", "評価実験公開recordを確定できません");
        }
        let encoded = public.to_string();
        let audit = self.append_audit(
            request_id,
            OPERATION,
            "accepted",
            &format!("評価Experiment開始記録:{encoded}"),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &canonical_payload_hash(Some(&public)),
        );
        match audit {
            Ok(event)
                if event.event_id
                    == public["実験監査ID"].as_str().unwrap_or_default() => {
                let id = public["評価ExperimentID"].as_str().expect("検証済み").to_owned();
                self.評価.active.insert(id, ActiveExperiment { public: public.clone(), cases: registration.cases, plans });
                success_response(request_id, OPERATION, event.event_id, public)
            }
            _ => {
                self.評価対話取消(&plans, request_id);
                self.audit_store_failed_response(request_id, OPERATION, "監査失敗", "作成済み対話を中止し監査修復後に再試行してください")
            }
        }
    }

    fn 評価実験状態処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        payload_hash: &str,
    ) -> BrokerResponse {
        const OPERATION: &str = "評価実験状態";
        let experiment_id = match experiment_request(payload) {
            Ok(value) => value,
            Err(reason) => return self.reject_with_payload_hash(request_id, OPERATION, "要求不正", reason, true, payload_hash),
        };
        if self
            .append_audit(request_id, OPERATION, "received", "評価実験の安全な状態projectionを要求", EVIDENCE_SOURCE_INTERNAL_STATE, payload_hash)
            .is_err()
        {
            return self.audit_store_failed_response(request_id, OPERATION, "監査失敗", "監査修復後に評価状態を再確認してください");
        }
        let mut control = std::mem::take(&mut self.評価);
        let result = control.status(self, request_id, &experiment_id);
        self.評価 = control;
        match result {
            Ok(body) => accept_safe_body(self, request_id, OPERATION, body),
            Err("監査失敗") => self.audit_store_failed_response(request_id, OPERATION, "監査失敗", "監査修復後に評価状態を再確認してください"),
            Err(reason) => self.reject_with_payload_hash(request_id, OPERATION, "評価実験不在", reason, true, payload_hash),
        }
    }

    fn 評価比較処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        payload_hash: &str,
    ) -> BrokerResponse {
        const OPERATION: &str = "評価比較";
        let experiment_id = match experiment_request(payload) {
            Ok(value) => value,
            Err(reason) => return self.reject_with_payload_hash(request_id, OPERATION, "要求不正", reason, true, payload_hash),
        };
        if self
            .append_audit(request_id, OPERATION, "received", "評価結果のRuntime集計比較を要求。権限やrelease判断は生成しない", EVIDENCE_SOURCE_INTERNAL_STATE, payload_hash)
            .is_err()
        {
            return self.audit_store_failed_response(request_id, OPERATION, "監査失敗", "監査修復後に評価比較を再実行してください");
        }
        let mut control = std::mem::take(&mut self.評価);
        let result = control.comparison(self, request_id, &experiment_id);
        self.評価 = control;
        match result {
            Ok(body) => {
                let encoded = body.to_string();
                let expected_id = body["比較監査ID"].as_str().unwrap_or_default();
                match self.append_audit(request_id, OPERATION, "accepted", &format!("評価比較記録:{encoded}"), EVIDENCE_SOURCE_INTERNAL_STATE, &canonical_payload_hash(Some(&body))) {
                    Ok(event) if event.event_id == expected_id => success_response(request_id, OPERATION, event.event_id, body),
                    _ => self.audit_store_failed_response(request_id, OPERATION, "監査失敗", "評価比較監査を確定できません"),
                }
            }
            Err("監査失敗") => self.audit_store_failed_response(request_id, OPERATION, "監査失敗", "監査修復後に評価比較を再実行してください"),
            Err(reason) => self.reject_with_payload_hash(request_id, OPERATION, "評価比較不能", reason, true, payload_hash),
        }
    }

    fn 評価検証済み監査(&self) -> Result<BrokerAuditLog, ()> {
        let log = self
            .state_store
            .persistent_store
            .as_ref()
            .and_then(|store| store.verified_audit_log().ok())
            .ok_or(())?;
        (log == self.audit_log).then_some(log).ok_or(())
    }

    fn 評価privateDataset読取(&self, manifest: &DatasetManifest) -> Result<PrivateRegistration, &'static str> {
        #[cfg(not(windows))]
        {
            let _ = manifest;
            Err("評価Datasetのprivate保管はこのOSで未対応です")
        }
        #[cfg(windows)]
        {
            let store = self.protected_store.as_ref().ok_or("Windows ProtectedStoreが未登録です")?;
            let secret = store
                .read(
                    crate::protected_store::Purpose::Evaluation,
                    &manifest.storage_id,
                    &manifest.ciphertext_hash,
                )
                .map_err(|_| "評価Datasetの暗号文を検証・復号できません")?;
            let raw: Value = serde_json::from_slice(secret.as_bytes())
                .map_err(|_| "評価Datasetのprivate形式が不正です")?;
            let registration = parse_registration(&raw)?;
            if registration.dataset_id != manifest.dataset_id || registration.revision != manifest.revision {
                return Err("評価DatasetのIDまたはrevisionが監査manifestと不一致です");
            }
            if canonical_payload_hash(Some(&raw)) != manifest.definition_hash {
                return Err("評価Dataset定義hashが監査manifestと不一致です");
            }
            Ok(registration)
        }
    }

    /// C5内部から既存対話統治へ入る。inputはpayload hashだけを監査へ残す。
    fn 評価対話操作(
        &mut self,
        outer_request_id: &str,
        operation: &str,
        payload: &Value,
    ) -> Result<(Value, String, String), &'static str> {
        let payload_hash = canonical_payload_hash(Some(payload));
        let received = self
            .append_audit(
                outer_request_id,
                operation,
                "received",
                "評価実験が既存対話統治経路を要求",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                &payload_hash,
        )
        .map_err(|_| "監査失敗")?;
        let received_audit_id = received.event_id.clone();
        let mut last_audit_id = received.event_id;
        // `記録保存`は既存workのrecorded eventも追加し得る。開始・送信の異常応答を
        // 回収するときに別workのeventを誤認しないよう、当該操作固有のeventだけを捕捉する。
        let mut primary_audit_id = None;
        let now = self.current_epoch_seconds();
        let mut dialogue = std::mem::take(&mut self.対話);
        let result = dialogue.操作(operation, payload, false, now, &mut |reason, id, hash| {
            let event = self
                .append_audit(id, operation, "recorded", reason, EVIDENCE_SOURCE_INTERNAL_STATE, hash)
                .map_err(|_| 対話失敗::監査失敗)?;
            if (operation == "対話開始" && reason == "対話開始")
                || (operation == "対話送信" && reason == "対話承認待ち作成")
            {
                primary_audit_id = Some(event.event_id.clone());
            }
            last_audit_id = event.event_id.clone();
            Ok(event.event_id)
        });
        self.対話 = dialogue;
        result
            .map(|body| {
                (
                    body,
                    primary_audit_id.unwrap_or(last_audit_id),
                    received_audit_id,
                )
            })
            .map_err(|error| match error {
            対話失敗::監査失敗 => "監査失敗",
            対話失敗::隔離済み => "実行系隔離",
            _ => "既存対話統治経路が要求を拒否しました",
        })
    }

    /// C5が作成した対話だけを、通常の対話取得・UI投影から隔離する。
    /// ownerの個別承認を置換せず、隔離失敗時はcallerが作成済み対話を中止してfail-closedにする。
    fn 評価対話隔離(
        &mut self,
        outer_request_id: &str,
        request_id: &str,
    ) -> Result<(String, String), &'static str> {
        let payload = json!({"要求ID": request_id});
        let received = self.append_audit(
            outer_request_id,
            "評価対話隔離",
            "received",
            "評価実験が既存対話を通常取得から隔離するよう要求",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &canonical_payload_hash(Some(&payload)),
        )
        .map_err(|_| "監査失敗")?;
        let mut dialogue = std::mem::take(&mut self.対話);
        let result = dialogue.評価隔離(request_id);
        self.対話 = dialogue;
        result.map_err(|_| "評価対話隔離に失敗")?;
        let event = self.append_audit(
            request_id,
            "評価対話隔離",
            "recorded",
            "評価専用対話を通常の対話取得から隔離",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &canonical_payload_hash(Some(&payload)),
        )
        .map_err(|_| "監査失敗")?;
        Ok((event.event_id, received.event_id))
    }

    fn 評価対話進捗(&mut self, outer_request_id: &str, request_id: &str) -> Result<評価対話進捗, &'static str> {
        let payload = json!({"要求ID": request_id});
        let received = self
            .append_audit(
                outer_request_id,
                "対話取得",
                "received",
                "評価実験が既存対話統治経路の進捗を取得",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                &canonical_payload_hash(Some(&payload)),
            )
            .map_err(|_| "監査失敗")?;
        let mut last_audit_id = received.event_id;
        let now = self.current_epoch_seconds();
        let mut dialogue = std::mem::take(&mut self.対話);
        let result = dialogue.評価進捗(request_id, now, &mut |reason, id, hash| {
            let event = self
                .append_audit(id, "対話取得", "recorded", reason, EVIDENCE_SOURCE_INTERNAL_STATE, hash)
                .map_err(|_| 対話失敗::監査失敗)?;
            last_audit_id = event.event_id.clone();
            Ok(event.event_id)
        });
        self.対話 = dialogue;
        let _ = last_audit_id;
        result.map_err(|_| "監査失敗")
    }

    fn 評価対話取消(&mut self, plans: &[EvaluationPlan], outer_request_id: &str) {
        for plan in plans {
            self.評価対話を取消して終了(outer_request_id, Some(&plan.request_id), &plan.session_id);
        }
    }

    /// 対話応答が内部不変条件を満たさないときも、同じ操作が残したrecorded監査から
    /// 回収対象だけを導く。監査相関を推測せず、一意のrecorded eventだけを受理する。
    fn 評価対話監査対象(&self, audit_id: &str, operation: &str) -> Option<String> {
        let expected_reason = match operation {
            "対話開始" => "対話開始",
            "対話送信" => "対話承認待ち作成",
            _ => return None,
        };
        let mut candidates = self
            .audit_log
            .events()
            .iter()
            .filter(|event| {
                event.event_id == audit_id
                    && event.operation == operation
                    && event.decision == "recorded"
                    && event.reason == expected_reason
                    && event.evidence_source == EVIDENCE_SOURCE_INTERNAL_STATE
                    && valid_identifier(&event.request_id)
            })
            .map(|event| event.request_id.clone());
        let value = candidates.next()?;
        candidates.next().is_none().then_some(value)
    }

    /// 評価Experiment開始中に既知の対話を作り損ねたとき、既存対話統治経路で
    /// 中止してから終了を試行する。回収不能を成功へ置き換えず、callerはfail-closedで返す。
    fn 評価対話を取消して終了(
        &mut self,
        outer_request_id: &str,
        request_id: Option<&str>,
        session_id: &str,
    ) {
        if let Some(request_id) = request_id.filter(|value| valid_identifier(value)) {
            let _ = self.評価対話操作(
                outer_request_id,
                "対話中止",
                &json!({"要求ID": request_id}),
            );
        }
        if valid_identifier(session_id) {
            let _ = self.評価対話操作(
                outer_request_id,
                "対話終了",
                &json!({"対話セッションID": session_id}),
            );
        }
    }
}

impl EvaluationControl {
    fn status(&mut self, broker: &mut Broker, request_id: &str, experiment_id: &str) -> Result<Value, &'static str> {
        if let Some(experiment) = self.active.get_mut(experiment_id) {
            drive_experiment(broker, request_id, experiment)?;
            return public_status_body(experiment);
        }
        recovered_status(broker, experiment_id)
    }

    fn comparison(&mut self, broker: &mut Broker, _request_id: &str, experiment_id: &str) -> Result<Value, &'static str> {
        if let Some(experiment) = self.active.get(experiment_id) {
            return comparison_from_active(broker, experiment);
        }
        comparison_from_recovered(broker, experiment_id)
    }
}

fn drive_experiment(
    broker: &mut Broker,
    outer_request_id: &str,
    experiment: &mut ActiveExperiment,
) -> Result<(), &'static str> {
    let experiment_audit_id = experiment.public["実験監査ID"]
        .as_str()
        .filter(|value| valid_audit_id(value))
        .ok_or("評価Experiment監査IDが不正です")?
        .to_owned();
    for plan in &mut experiment.plans {
        if plan.result.is_some() {
            if plan.cleanup_pending {
                if broker
                    .ライフサイクル
                    .is_terminally_quarantined(&plan.runtime_id)
                {
                    // C4 terminal隔離は外部実行の停止を保証せず、worker受信側も直ちに
                    // 解放しない。既存対話の進捗反映後にsessionと作業が消えた場合だけ
                    // pendingを落とす。C4経路は通常の終了監査を生成しないため、比較用の
                    // cleanup_verifiedには昇格させない。
                    if terminal_dialogue_recovered(broker, outer_request_id, plan)? {
                        plan.cleanup_pending = false;
                    }
                } else if evaluation_dialogue_cleanup(broker, outer_request_id, plan)? {
                    plan.cleanup_pending = false;
                    plan.cleanup_verified = true;
                }
            }
            continue;
        }
        if broker.ライフサイクル.is_terminally_quarantined(&plan.runtime_id) {
            // C4隔離は外部実行の停止を保証しない。解放済み対話を取得し直したり
            // 終了監査IDを推測したりせず、既存対話のworker受信側が実際に回収されるまで
            // pendingを残し、計画と既存監査から確認できる値だけで中断を記録する。
            let terminal_recovered = terminal_dialogue_recovered(broker, outer_request_id, plan)?;
            let case = experiment
                .cases
                .iter()
                .find(|case| case.case_id == plan.case_id)
                .ok_or("監査失敗")?;
            let quarantine_reservation_audit_id = terminal_quarantine_reservation_audit_id(
                &broker.audit_log,
                &experiment_audit_id,
                plan,
            )?;
            let (start_audit, end_audit) = terminal_dialogue_audit_ids(&broker.audit_log, plan)?;
            let evaluator_results = terminal_evaluator_results(&case.evaluators, "中断", "中止")?;
            let audit_id = broker.audit_log.next_event_id();
            let result = json!({
                "版": EVALUATION_VERSION,
                "評価ExperimentID": experiment.public["評価ExperimentID"],
                "評価DatasetID": experiment.public["評価DatasetID"],
                "Dataset定義hash": experiment.public["Dataset定義hash"],
                "評価CaseID": plan.case_id,
                "実行系ID": plan.runtime_id,
                "対話要求ID": plan.request_id,
                "対話セッションID": plan.session_id,
                "作成監査ID": plan.created_audit_id,
                "開始監査ID": start_audit,
                "終了監査ID": end_audit,
                "実行系隔離予約監査ID": quarantine_reservation_audit_id,
                "評価監査ID": audit_id,
                "結果状態": "中止",
                "判定": "中断",
                "評価器判定一覧": evaluator_results,
                "実行時刻UnixMillis": broker.current_epoch_millis(),
                "LatencyMillis": Value::Null,
                "応答hash": Value::Null,
                "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
            });
            if !valid_result(&result) {
                return Err("監査失敗");
            }
            let encoded = result.to_string();
            let event = broker
                .append_audit(
                    &plan.request_id,
                    "評価実験状態",
                    "recorded",
                    &format!("評価結果記録:{encoded}"),
                    EVIDENCE_SOURCE_INTERNAL_STATE,
                    &canonical_payload_hash(Some(&result)),
                )
                .map_err(|_| "監査失敗")?;
            if event.event_id != result["評価監査ID"] {
                return Err("監査失敗");
            }
            plan.state = "中止".to_owned();
            plan.observation = None;
            plan.result = Some(result);
            plan.cleanup_pending = !terminal_recovered;
            plan.cleanup_verified = false;
            continue;
        }
        let progress = broker.評価対話進捗(outer_request_id, &plan.request_id)?;
        if progress.要求ID != plan.request_id || progress.実行系ID != plan.runtime_id || progress.対話セッションID != plan.session_id {
            return Err("監査失敗");
        }
        plan.state = progress.状態.clone();
        let Some(dialogue_result) = progress.結果 else {
            continue;
        };
        let case = experiment
            .cases
            .iter()
            .find(|case| case.case_id == plan.case_id)
            .ok_or("監査失敗")?;
        let execution = progress.実行記録;
        let created_audit = execution
            .get("作成監査ID")
            .and_then(Value::as_str)
            .filter(|value| *value == plan.created_audit_id)
            .ok_or("監査失敗")?;
        let start_audit = optional_audit(execution.get("開始監査ID"))?;
        let end_audit = optional_audit(execution.get("終了監査ID"))?;
        let output_status = dialogue_result.get("状態").and_then(Value::as_str).unwrap_or("失敗");
        let response_hash_available = dialogue_result
            .get("応答hash")
            .and_then(Value::as_str)
            .is_some_and(valid_hash);
        let result_status = match output_status {
            // C5の成功Resultは既存対話の応答hashと結果証跡に相関できなければならない。
            // `none`承認などでhashを取得できない場合は、部分Evaluatorだけを成立へ
            // 昇格させず、明示的に評価不能として終了・回収を続ける。
            "成功" if response_hash_available => "成功",
            "成功" => "評価不能",
            "中止" => "中止",
            "失敗" => "失敗",
            "保留" => "評価不能",
            _ => "評価不能",
        };
        let (verdict, evaluator_results) = match result_status {
            // 取消済みの応答を通常のEvaluatorへ渡すと、取消そのものを期待した
            // status Evaluatorが「成立」になり得る。C5の結果状態と判定を混同
            // しないよう、中断は全Evaluatorを明示的に中断として記録する。
            "中止" => (
                "中断".to_owned(),
                terminal_evaluator_results(&case.evaluators, "中断", "中止")?,
            ),
            // 対話projection自体を取得できない状態は、期待値との照合結果へ
            // 置き換えず、根拠不足として評価不能を保つ。
            "評価不能" => (
                "評価不能".to_owned(),
                terminal_evaluator_results(&case.evaluators, "評価不能", "根拠不足")?,
            ),
            _ => {
                let public_evaluation = evaluate_case_public_projection(
                    &dialogue_result,
                    progress.単調応答Millis,
                    &case.evaluators,
                );
                let evaluator_results = public_evaluation
                    .get("評価結果")
                    .and_then(Value::as_array)
                    .cloned()
                    .ok_or("監査失敗")?;
                let verdict = public_evaluation
                    .get("case判定")
                    .and_then(Value::as_str)
                    .filter(|value| matches!(*value, "成立" | "不成立" | "評価不能"))
                    .ok_or("監査失敗")?
                    .to_owned();
                (verdict, evaluator_results)
            }
        };
        if evaluator_results.is_empty()
            || evaluator_results.len() > MAX_EVALUATORS_PER_CASE
            || evaluator_results.iter().any(|value| !valid_evaluator_result(value))
        {
            return Err("監査失敗");
        }
        let response_hash = dialogue_result
            .get("応答hash")
            .and_then(Value::as_str)
            .filter(|value| valid_hash(value))
            .map(Value::from)
            .unwrap_or(Value::Null);
        let audit_id = broker.audit_log.next_event_id();
        let result = json!({
            "版": EVALUATION_VERSION,
            "評価ExperimentID": experiment.public["評価ExperimentID"],
            "評価DatasetID": experiment.public["評価DatasetID"],
            "Dataset定義hash": experiment.public["Dataset定義hash"],
            "評価CaseID": plan.case_id,
            "実行系ID": plan.runtime_id,
            "対話要求ID": plan.request_id,
            "対話セッションID": plan.session_id,
            "作成監査ID": created_audit,
            "開始監査ID": start_audit,
            "終了監査ID": end_audit,
            "実行系隔離予約監査ID": Value::Null,
            "評価監査ID": audit_id,
            "結果状態": result_status,
            "判定": verdict,
            "評価器判定一覧": evaluator_results,
            "実行時刻UnixMillis": broker.current_epoch_millis(),
            "LatencyMillis": progress.単調応答Millis,
            "応答hash": response_hash,
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        });
        if !valid_result(&result) {
            return Err("監査失敗");
        }
        let encoded = result.to_string();
        let event = broker
            .append_audit(
                &plan.request_id,
                "評価実験状態",
                "recorded",
                &format!("評価結果記録:{encoded}"),
                EVIDENCE_SOURCE_INTERNAL_STATE,
                &canonical_payload_hash(Some(&result)),
            )
            .map_err(|_| "監査失敗")?;
        if event.event_id != result["評価監査ID"] {
            return Err("監査失敗");
        }
        plan.observation = Some(observation_fingerprints(&dialogue_result));
        plan.result = Some(result);
        // 結果監査を確定した対話は既存の終了統治経路で回収する。worker受信側が
        // まだ残ると終了を拒否するため、次の状態照会で再試行できる状態を残す。
        plan.cleanup_pending = true;
        if evaluation_dialogue_cleanup(broker, outer_request_id, plan)? {
            plan.cleanup_pending = false;
            plan.cleanup_verified = true;
        }
    }
    refresh_experiment_public(experiment, broker.current_epoch_millis());
    Ok(())
}

/// 対話終了はworker受信側が残る間に拒否される。これは監査失敗と区別して、
/// 終端結果を改変せず次回の状態照会でだけ再試行する。
fn evaluation_dialogue_cleanup(
    broker: &mut Broker,
    outer_request_id: &str,
    plan: &EvaluationPlan,
) -> Result<bool, &'static str> {
    match broker.評価対話操作(
        outer_request_id,
        "対話終了",
        &json!({"対話セッションID": plan.session_id}),
    ) {
        Ok(_) => Ok(true),
        Err("既存対話統治経路が要求を拒否しました") => Ok(false),
        Err(reason) => Err(reason),
    }
}

/// C4 terminal隔離では通常の`対話終了`を発行してはいけない。隔離済みworkerの
/// receiverを既存対話制御に反映し、作業とsessionの双方が消えたことだけを確認する。
/// これは外部実行の停止、C4の復旧完了、比較可能性の証拠を意味しない。
fn terminal_dialogue_recovered(
    broker: &mut Broker,
    outer_request_id: &str,
    plan: &EvaluationPlan,
) -> Result<bool, &'static str> {
    if broker
        .対話
        .評価対話回収済み(&plan.request_id, &plan.session_id)
    {
        return Ok(true);
    }
    let progress = broker.評価対話進捗(outer_request_id, &plan.request_id);
    if let Err(reason) = progress {
        // 進捗反映中にworkerが完了して資格隔離の回収まで進むと、当該要求は取得前に
        // 消える。その場合だけ回収済みとして扱い、それ以外の失敗は隠さない。
        if broker
            .対話
            .評価対話回収済み(&plan.request_id, &plan.session_id)
        {
            return Ok(true);
        }
        return Err(reason);
    }
    Ok(broker
        .対話
        .評価対話回収済み(&plan.request_id, &plan.session_id))
}

/// 中止または根拠不足のときにも、privateな設定を露出せずにEvaluator相関を保つ。
/// Dataset登録時に検証済みでも、復号後の構造を再検証してfail-closedにする。
fn terminal_evaluator_results(
    evaluators: &Value,
    determination: &'static str,
    reason_code: &'static str,
) -> Result<Vec<Value>, &'static str> {
    let evaluators = evaluators
        .as_array()
        .filter(|values| !values.is_empty() && values.len() <= MAX_EVALUATORS_PER_CASE)
        .ok_or("監査失敗")?;
    let mut output = Vec::with_capacity(evaluators.len());
    for evaluator in evaluators {
        let evaluator_id = validate_private_evaluator(evaluator).map_err(|_| "監査失敗")?;
        let kind = evaluator
            .get("種類")
            .and_then(Value::as_str)
            .filter(|value| {
                matches!(
                    *value,
                    "exact"
                        | "contains"
                        | "regex"
                        | "json_schema"
                        | "reference_count"
                        | "route"
                        | "status"
                        | "capability"
                        | "latency_threshold"
                )
            })
            .ok_or("監査失敗")?;
        output.push(json!({
            "評価器ID": evaluator_id,
            "種類": kind,
            "判定": determination,
            "理由code": reason_code,
        }));
    }
    Ok(output)
}

fn public_status_body(experiment: &ActiveExperiment) -> Result<Value, &'static str> {
    let mut results = experiment
        .plans
        .iter()
        .filter_map(|plan| plan.result.as_ref())
        .map(public_result_projection)
        .collect::<Result<Vec<_>, _>>()?;
    results.sort_by(|left, right| {
        left["実行系ID"]
            .as_str()
            .cmp(&right["実行系ID"].as_str())
            .then_with(|| left["評価CaseID"].as_str().cmp(&right["評価CaseID"].as_str()))
    });
    Ok(json!({"版": EVALUATION_VERSION, "評価実験": experiment.public, "公開結果一覧": results}))
}

fn recovered_status(broker: &Broker, experiment_id: &str) -> Result<Value, &'static str> {
    let log = broker.評価検証済み監査().map_err(|_| "監査失敗")?;
    let Some(recovered) = recovered_experiment(&log, experiment_id)? else {
        return Err("指定評価Experimentがありません");
    };
    let mut experiment = recovered.public.clone();
    let expected = experiment["計画Case数"].as_u64().unwrap_or(0) as usize
        * experiment["対象Runtime一覧"].as_array().map_or(0, Vec::len);
    let results = recovered_results(&log, &recovered)?;
    // 監査から再構成したResultが指定ExperimentとDatasetに対応することを、
    // 公開一覧を返す前に再検証する。
    let _ = aggregate_results(&experiment, results.iter())?;
    let cleanup_verified = recovered_cleanup_verified(&log, &recovered, &results)?;
    if results.len() < expected || !cleanup_verified {
        experiment["状態"] = json!("中断");
        experiment["終了時刻UnixMillis"] = Value::Null;
    } else {
        let any_interrupted = results.iter().any(|result| result["判定"] == "中断");
        let any_unevaluable = results
            .iter()
            .any(|result| result["判定"] == "評価不能");
        experiment["状態"] = json!(if any_interrupted {
            "中断"
        } else if any_unevaluable {
            "評価不能"
        } else {
            "完了"
        });
        let started_at = results
            .iter()
            .filter_map(|result| result["実行時刻UnixMillis"].as_i64())
            .min()
            .ok_or("復元Resultの実行時刻がありません")?;
        let finished_at = results
            .iter()
            .filter_map(|result| result["実行時刻UnixMillis"].as_i64())
            .max()
            .ok_or("復元Resultの実行時刻がありません")?;
        experiment["開始時刻UnixMillis"] = Value::from(started_at);
        experiment["終了時刻UnixMillis"] = Value::from(finished_at);
    }
    experiment["結果数"] = json!(results.len());
    if !valid_experiment(&experiment) {
        return Err("復元Experimentの公開状態が不正です");
    }
    let public_results = results
        .iter()
        .map(public_result_projection)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "版": EVALUATION_VERSION,
        "評価実験": experiment,
        "公開結果一覧": public_results,
    }))
}

fn comparison_from_active(broker: &Broker, experiment: &ActiveExperiment) -> Result<Value, &'static str> {
    if !experiment_is_finalized(experiment) {
        return Err("全CaseとRuntimeの終端結果および対話終了が確定するまで比較できません");
    }
    let aggregates = aggregate_results(&experiment.public, experiment.plans.iter().filter_map(|plan| plan.result.as_ref()))?;
    let differences = active_differences(experiment)?;
    comparison_body(broker, &experiment.public, aggregates, differences)
}

fn comparison_from_recovered(broker: &Broker, experiment_id: &str) -> Result<Value, &'static str> {
    let log = broker.評価検証済み監査().map_err(|_| "監査失敗")?;
    let experiment = recovered_experiment(&log, experiment_id)?.ok_or("指定評価Experimentがありません")?;
    let results = recovered_results(&log, &experiment)?;
    if results.len() != expected_result_count(&experiment.public)? {
        return Err("全CaseとRuntimeの終端結果が監査から復元できるまで比較できません");
    }
    if !recovered_cleanup_verified(&log, &experiment, &results)? {
        return Err("各CaseとRuntimeの通常対話終了監査が復元できるまで比較できません");
    }
    let aggregates = aggregate_results(&experiment.public, results.iter())?;
    comparison_body(
        broker,
        &experiment.public,
        aggregates,
        ("unknown", "unknown", "unknown"),
    )
}

fn expected_result_count(experiment: &Value) -> Result<usize, &'static str> {
    let case_count = experiment["計画Case数"]
        .as_u64()
        .filter(|value| *value > 0 && *value <= MAX_DATASETS_CASES as u64)
        .ok_or("評価Experimentの計画Case数が不正です")?;
    let runtime_count = experiment["対象Runtime一覧"]
        .as_array()
        .filter(|values| !values.is_empty() && values.len() <= MAX_TARGET_RUNTIMES)
        .map(Vec::len)
        .ok_or("評価ExperimentのRuntime一覧が不正です")?;
    usize::try_from(case_count)
        .ok()
        .and_then(|value| value.checked_mul(runtime_count))
        .filter(|value| *value <= MAX_DATASETS_CASES.saturating_mul(MAX_TARGET_RUNTIMES))
        .ok_or("評価Experimentの計画件数が不正です")
}

fn experiment_is_finalized(experiment: &ActiveExperiment) -> bool {
    !experiment.plans.is_empty()
        && experiment.plans.len() == expected_result_count(&experiment.public).unwrap_or(0)
        && experiment
            .plans
            .iter()
            .all(|plan| {
                plan.result.is_some() && !plan.cleanup_pending && plan.cleanup_verified
            })
}

fn comparison_body(
    broker: &Broker,
    experiment: &Value,
    aggregates: Vec<Value>,
    differences: (&str, &str, &str),
) -> Result<Value, &'static str> {
    if aggregates.len() < 2 {
        return Err("二つ以上のRuntimeがある評価Experimentだけを比較できます");
    }
    let counts = |name: &str| -> u64 {
        aggregates
            .iter()
            .map(|entry| entry[name].as_u64().unwrap_or(0))
            .sum()
    };
    let audit_id = broker.audit_log.next_event_id();
    let body = json!({
        "版": EVALUATION_VERSION,
        "比較ID": 識別子生成().map_err(|_| "比較IDを生成できません")?,
        "評価DatasetID": experiment["評価DatasetID"],
        "Dataset定義hash": experiment["Dataset定義hash"],
        "実験一覧": aggregates,
        "計画Case数": experiment["計画Case数"],
        "成立数": counts("成立数"),
        "不成立数": counts("不成立数"),
        "評価不能数": counts("評価不能数"),
        "中断数": counts("中断数"),
        "比較時刻UnixMillis": broker.current_epoch_millis(),
        "経路差": differences.0,
        "参照差": differences.1,
        "能力差": differences.2,
        "比較監査ID": audit_id,
        "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
    });
    valid_comparison(&body).then_some(body).ok_or("比較recordを確定できません")
}

fn aggregate_results<'a>(experiment: &Value, results: impl Iterator<Item = &'a Value>) -> Result<Vec<Value>, &'static str> {
    let runtimes = experiment["対象Runtime一覧"].as_array().ok_or("評価Experimentが不正です")?;
    let case_count = experiment["計画Case数"].as_u64().ok_or("評価Experimentが不正です")?;
    let experiment_id = experiment["評価ExperimentID"]
        .as_str()
        .ok_or("評価Experimentが不正です")?;
    let dataset_id = experiment["評価DatasetID"]
        .as_str()
        .ok_or("評価Experimentが不正です")?;
    let definition_hash = experiment["Dataset定義hash"]
        .as_str()
        .ok_or("評価Experimentが不正です")?;
    let mut grouped: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    let mut pairs = BTreeSet::new();
    for result in results {
        if !valid_result(result) {
            return Err("評価結果記録を検証できません");
        }
        if result["評価ExperimentID"].as_str() != Some(experiment_id)
            || result["評価DatasetID"].as_str() != Some(dataset_id)
            || result["Dataset定義hash"].as_str() != Some(definition_hash)
        {
            return Err("評価結果記録がExperimentと不一致です");
        }
        let runtime = result["実行系ID"].as_str().ok_or("評価結果記録を検証できません")?;
        if !runtimes.iter().any(|value| value.as_str() == Some(runtime)) {
            return Err("評価結果記録の実行系がExperimentにありません");
        }
        let case_id = result["評価CaseID"]
            .as_str()
            .ok_or("評価結果記録を検証できません")?;
        if !pairs.insert((case_id, runtime)) {
            return Err("評価結果記録が重複しています");
        }
        grouped.entry(runtime).or_default().push(result);
    }
    let mut output = Vec::new();
    for runtime in runtimes {
        let runtime = runtime.as_str().ok_or("評価Experimentが不正です")?;
        let records = grouped.get(runtime).cloned().unwrap_or_default();
        if records.len() > case_count as usize {
            return Err("評価結果記録数が計画Case数を超えています");
        }
        let mut established = 0u64;
        let mut not_established = 0u64;
        let mut unevaluable = 0u64;
        let mut interrupted = 0u64;
        let mut latency_total = 0u128;
        let mut latency_count = 0u64;
        for result in &records {
            match result["判定"].as_str() {
                Some("成立") => established += 1,
                Some("不成立") => not_established += 1,
                Some("中断") => interrupted += 1,
                _ => unevaluable += 1,
            }
            if let Some(latency) = result["LatencyMillis"].as_u64() {
                latency_total = latency_total.saturating_add(u128::from(latency));
                latency_count += 1;
            }
        }
        interrupted += case_count.saturating_sub(records.len() as u64);
        output.push(json!({
            "評価ExperimentID": experiment["評価ExperimentID"],
            "実行系ID": runtime,
            "Dataset定義hash": experiment["Dataset定義hash"],
            "成立数": established,
            "不成立数": not_established,
            "評価不能数": unevaluable,
            "中断数": interrupted,
            "平均LatencyMillis": (latency_count > 0).then(|| latency_total as f64 / latency_count as f64),
        }));
    }
    Ok(output)
}

fn active_differences(experiment: &ActiveExperiment) -> Result<(&'static str, &'static str, &'static str), &'static str> {
    let runtimes = experiment.public["対象Runtime一覧"].as_array().ok_or("評価Experimentが不正です")?;
    let mut route = "same";
    let mut reference = "same";
    let mut capability = "same";
    let mut cases = BTreeSet::new();
    for plan in &experiment.plans {
        cases.insert(plan.case_id.as_str());
    }
    for case_id in cases {
        let mut observations = Vec::new();
        for runtime in runtimes {
            let runtime = runtime.as_str().ok_or("評価Experimentが不正です")?;
            let observation = experiment.plans.iter().find(|plan| {
                plan.case_id == case_id && plan.runtime_id == runtime
            }).and_then(|plan| plan.observation.as_ref());
            let Some(observation) = observation else {
                return Ok(("unknown", "unknown", "unknown"));
            };
            observations.push(observation);
        }
        route = merge_difference(route, observations.iter().map(|value| value.route.as_deref()))?;
        reference = merge_difference(reference, observations.iter().map(|value| value.reference.as_deref()))?;
        capability = merge_difference(capability, observations.iter().map(|value| value.capability.as_deref()))?;
    }
    Ok((route, reference, capability))
}

fn merge_difference<'a>(current: &'static str, values: impl Iterator<Item = Option<&'a str>>) -> Result<&'static str, &'static str> {
    if current == "unknown" {
        return Ok(current);
    }
    let values = values.collect::<Vec<_>>();
    if values.iter().any(|value| value.is_none()) {
        return Ok("unknown");
    }
    let first = values.first().and_then(|value| *value).ok_or("比較値がありません")?;
    Ok(if values.iter().all(|value| *value == Some(first)) { current } else { "different" })
}

fn observation_fingerprints(result: &Value) -> ObservationFingerprints {
    if result.get("表示範囲").and_then(Value::as_str) != Some("full") {
        return ObservationFingerprints { route: None, reference: None, capability: None };
    }
    ObservationFingerprints {
        route: result.get("経路").map(|value| canonical_payload_hash(Some(value))),
        reference: result.get("参照").map(|value| canonical_payload_hash(Some(value))),
        capability: result.get("能力").map(|value| canonical_payload_hash(Some(value))),
    }
}

fn refresh_experiment_public(experiment: &mut ActiveExperiment, now_millis: i64) {
    let result_count = experiment.plans.iter().filter(|plan| plan.result.is_some()).count();
    experiment.public["結果数"] = json!(result_count);
    let started = experiment.plans.iter().filter_map(|plan| plan.result.as_ref()).filter_map(|result| result["開始監査ID"].as_str()).next();
    if started.is_some() && experiment.public["開始時刻UnixMillis"].is_null() {
        experiment.public["開始時刻UnixMillis"] = json!(now_millis);
    }
    if result_count == experiment.plans.len()
        && experiment.plans.iter().all(|plan| !plan.cleanup_pending)
    {
        let any_interrupted = experiment
            .plans
            .iter()
            .filter_map(|plan| plan.result.as_ref())
            .any(|result| result["判定"] == "中断");
        let any_unevaluable = experiment
            .plans
            .iter()
            .filter_map(|plan| plan.result.as_ref())
            .any(|result| result["判定"] == "評価不能");
        experiment.public["状態"] = json!(if any_interrupted {
            "中断"
        } else if any_unevaluable {
            "評価不能"
        } else {
            "完了"
        });
        experiment.public["終了時刻UnixMillis"] = json!(now_millis);
        // 完了後の比較は公開Resultと観測hashだけで成立する。private入力とEvaluator設定は
        // ActiveExperimentに残さず、ProtectedStoreと監査相関へ責任を戻す。
        experiment.cases.clear();
    } else if result_count == experiment.plans.len() {
        // 終端結果は監査済みでも、worker受信側が残る間は既存の対話終了を再試行する。
        // 比較や完了状態へ昇格させず、private Caseはこの時点で保持しない。
        experiment.public["状態"] = json!("実行中");
        experiment.public["終了時刻UnixMillis"] = Value::Null;
        experiment.cases.clear();
    } else if experiment.plans.iter().any(|plan| plan.state == "実行中") {
        experiment.public["状態"] = json!("実行中");
    } else {
        experiment.public["状態"] = json!("承認待ち");
    }
}

fn public_result_projection(result: &Value) -> Result<Value, &'static str> {
    let projection = json!({
        "評価ExperimentID": result["評価ExperimentID"],
        "評価DatasetID": result["評価DatasetID"],
        "評価CaseID": result["評価CaseID"],
        "実行系ID": result["実行系ID"],
        "判定": result["判定"],
        "評価器判定一覧": result["評価器判定一覧"],
        "LatencyMillis": result["LatencyMillis"],
        "総合hash": canonical_payload_hash(Some(result)),
        "評価監査ID": result["評価監査ID"],
    });
    valid_public_result_projection(&projection)
        .then_some(projection)
        .ok_or("公開評価Result projectionが不正です")
}

fn persisted_plan_record_value(
    experiment_id: &str,
    manifest: &DatasetManifest,
    plans: &[EvaluationPlan],
) -> Result<Value, &'static str> {
    let value = json!({
        "版": EVALUATION_VERSION,
        "評価ExperimentID": experiment_id,
        "評価DatasetID": manifest.dataset_id,
        "Dataset定義hash": manifest.definition_hash,
        "計画": plans.iter().map(|plan| json!({
            "評価CaseID": plan.case_id,
            "実行系ID": plan.runtime_id,
            "対話要求ID": plan.request_id,
            "要求hash": plan.request_hash,
            "対話セッションID": plan.session_id,
            "Session開始要求監査ID": plan.session_start_request_audit_id,
            "Session開始監査ID": plan.session_start_audit_id,
            "送信要求監査ID": plan.send_request_audit_id,
            "送信内容hash": plan.send_payload_hash,
            "作成監査ID": plan.created_audit_id,
            "隔離要求監査ID": plan.isolation_request_audit_id,
            "隔離監査ID": plan.isolation_audit_id,
        })).collect::<Vec<_>>(),
        "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
    });
    parse_persisted_plan_record(&value)?;
    Ok(value)
}

fn parse_persisted_plan_record(value: &Value) -> Result<PersistedEvaluationPlanRecord, &'static str> {
    let object = exact_object(
        value,
        &[
            "版",
            "評価ExperimentID",
            "評価DatasetID",
            "Dataset定義hash",
            "計画",
            "証拠種別",
        ],
    )?;
    if object.get("版").and_then(Value::as_u64) != Some(EVALUATION_VERSION)
        || !object
            .get("Dataset定義hash")
            .and_then(Value::as_str)
            .is_some_and(valid_hash)
        || object.get("証拠種別").and_then(Value::as_str) != Some(EVIDENCE_SOURCE_INTERNAL_STATE)
    {
        return Err("評価Experiment計画recordが不正です");
    }
    let values = object
        .get("計画")
        .and_then(Value::as_array)
        .filter(|values| {
            !values.is_empty()
                && values.len() <= MAX_DATASETS_CASES.saturating_mul(MAX_TARGET_RUNTIMES)
        })
        .ok_or("評価Experiment計画が不正です")?;
    let mut pairs = BTreeSet::new();
    let mut request_ids = BTreeSet::new();
    let mut session_ids = BTreeSet::new();
    let mut plans = Vec::with_capacity(values.len());
    for value in values {
        let plan = exact_object(
            value,
            &[
                "評価CaseID",
                "実行系ID",
                "対話要求ID",
                "要求hash",
                "対話セッションID",
                "Session開始要求監査ID",
                "Session開始監査ID",
                "送信要求監査ID",
                "送信内容hash",
                "作成監査ID",
                "隔離要求監査ID",
                "隔離監査ID",
            ],
        )?;
        let case_id = required_identifier(plan, "評価CaseID")?;
        let runtime_id = plan
            .get("実行系ID")
            .and_then(Value::as_str)
            .filter(|value| 実行系ID妥当(value))
            .map(str::to_owned)
            .ok_or("評価Experiment計画のRuntime IDが不正です")?;
        let request_id = required_identifier(plan, "対話要求ID")?;
        let request_hash = plan
            .get("要求hash")
            .and_then(Value::as_str)
            .filter(|value| valid_hash(value))
            .map(str::to_owned)
            .ok_or("評価Experiment計画の要求hashが不正です")?;
        let session_id = required_identifier(plan, "対話セッションID")?;
        let session_start_request_audit_id = plan
            .get("Session開始要求監査ID")
            .and_then(Value::as_str)
            .filter(|value| valid_audit_id(value))
            .map(str::to_owned)
            .ok_or("評価Experiment計画のSession開始要求監査IDが不正です")?;
        let session_start_audit_id = plan
            .get("Session開始監査ID")
            .and_then(Value::as_str)
            .filter(|value| valid_audit_id(value))
            .map(str::to_owned)
            .ok_or("評価Experiment計画のSession開始監査IDが不正です")?;
        let send_request_audit_id = plan
            .get("送信要求監査ID")
            .and_then(Value::as_str)
            .filter(|value| valid_audit_id(value))
            .map(str::to_owned)
            .ok_or("評価Experiment計画の送信要求監査IDが不正です")?;
        let send_payload_hash = plan
            .get("送信内容hash")
            .and_then(Value::as_str)
            .filter(|value| valid_hash(value))
            .map(str::to_owned)
            .ok_or("評価Experiment計画の送信内容hashが不正です")?;
        let created_audit_id = plan
            .get("作成監査ID")
            .and_then(Value::as_str)
            .filter(|value| valid_audit_id(value))
            .map(str::to_owned)
            .ok_or("評価Experiment計画の作成監査IDが不正です")?;
        let isolation_request_audit_id = plan
            .get("隔離要求監査ID")
            .and_then(Value::as_str)
            .filter(|value| valid_audit_id(value))
            .map(str::to_owned)
            .ok_or("評価Experiment計画の隔離要求監査IDが不正です")?;
        let isolation_audit_id = plan
            .get("隔離監査ID")
            .and_then(Value::as_str)
            .filter(|value| valid_audit_id(value))
            .map(str::to_owned)
            .ok_or("評価Experiment計画の隔離監査IDが不正です")?;
        if !pairs.insert((case_id.clone(), runtime_id.clone()))
            || !request_ids.insert(request_id.clone())
            || !session_ids.insert(session_id.clone())
        {
            return Err("評価Experiment計画の対話相関が重複しています");
        }
        plans.push(PersistedEvaluationPlan {
            case_id,
            runtime_id,
            request_id,
            request_hash,
            session_id,
            session_start_request_audit_id,
            session_start_audit_id,
            send_request_audit_id,
            send_payload_hash,
            created_audit_id,
            isolation_request_audit_id,
            isolation_audit_id,
        });
    }
    Ok(PersistedEvaluationPlanRecord {
        experiment_id: required_identifier(object, "評価ExperimentID")?,
        dataset_id: required_identifier(object, "評価DatasetID")?,
        definition_hash: object["Dataset定義hash"]
            .as_str()
            .expect("検証済みhash")
            .to_owned(),
        plans,
    })
}

fn audit_event_with_index<'a>(
    log: &'a BrokerAuditLog,
    event_id: &str,
) -> Result<(usize, &'a BrokerAuditEvent), &'static str> {
    let mut events = log
        .events()
        .iter()
        .enumerate()
        .filter(|(_, event)| event.event_id == event_id);
    let event = events.next().ok_or("評価監査eventがありません")?;
    if events.next().is_some() {
        return Err("評価監査eventが重複しています");
    }
    Ok(event)
}

fn audit_event_by_id<'a>(
    log: &'a BrokerAuditLog,
    event_id: &str,
) -> Result<&'a BrokerAuditEvent, &'static str> {
    Ok(audit_event_with_index(log, event_id)?.1)
}

fn validate_plan_against_experiment(
    plan_record: &PersistedEvaluationPlanRecord,
    public: &Value,
    manifest: &DatasetManifest,
) -> Result<(), &'static str> {
    if plan_record.experiment_id != public["評価ExperimentID"]
        || plan_record.dataset_id != public["評価DatasetID"]
        || plan_record.definition_hash != public["Dataset定義hash"]
    {
        return Err("評価Experiment計画が公開recordと不一致です");
    }
    let runtimes = public["対象Runtime一覧"]
        .as_array()
        .ok_or("評価ExperimentのRuntime一覧が不正です")?;
    let mut runtime_ids = BTreeSet::new();
    for runtime in runtimes {
        let runtime = runtime
            .as_str()
            .filter(|value| 実行系ID妥当(value))
            .ok_or("評価ExperimentのRuntime IDが不正です")?;
        if !runtime_ids.insert(runtime) {
            return Err("評価ExperimentのRuntime IDが重複しています");
        }
    }
    let cases = manifest.value["Case一覧"]
        .as_array()
        .ok_or("評価Dataset manifestのCase一覧が不正です")?;
    let mut case_ids = BTreeSet::new();
    for case in cases {
        let case = exact_object(case, &["評価CaseID", "定義hash"])?;
        let case_id = required_identifier(case, "評価CaseID")?;
        if !case_ids.insert(case_id) {
            return Err("評価Dataset manifestのCase IDが重複しています");
        }
    }
    let expected = case_ids.len().saturating_mul(runtime_ids.len());
    if public["計画Case数"].as_u64() != Some(case_ids.len() as u64)
        || plan_record.plans.len() != expected
    {
        return Err("評価Experiment計画件数が不一致です");
    }
    let mut pairs = BTreeSet::new();
    for plan in &plan_record.plans {
        if !case_ids.contains(&plan.case_id)
            || !runtime_ids.contains(plan.runtime_id.as_str())
            || !pairs.insert((&plan.case_id, &plan.runtime_id))
        {
            return Err("評価Experiment計画のCaseまたはRuntimeが不正です");
        }
    }
    (pairs.len() == expected)
        .then_some(())
        .ok_or("評価Experiment計画のCase×Runtimeが不足しています")
}

fn result_matches_plan(
    log: &BrokerAuditLog,
    result: &Value,
    plan: &PersistedEvaluationPlan,
) -> Result<(), &'static str> {
    if result["評価CaseID"] != plan.case_id
        || result["実行系ID"] != plan.runtime_id
        || result["対話要求ID"] != plan.request_id
        || result["対話セッションID"] != plan.session_id
        || result["作成監査ID"] != plan.created_audit_id
    {
        return Err("評価結果が評価Experiment計画と不一致です");
    }
    let created = audit_event_by_id(log, &plan.created_audit_id)?;
    if !created_dialogue_event_matches(created, plan) {
        return Err("評価結果の対話作成監査相関が不正です");
    }
    let isolation = audit_event_by_id(log, &plan.isolation_audit_id)?;
    if !isolation_event_matches(isolation, plan) {
        return Err("評価結果の対話隔離監査相関が不正です");
    }
    if let Some(start_id) = result["開始監査ID"].as_str() {
        let start = audit_event_by_id(log, start_id)?;
        if !start_event_matches(start, plan) {
            return Err("評価結果の対話開始監査相関が不正です");
        }
    }
    if let Some(end_id) = result["終了監査ID"].as_str() {
        let end = audit_event_by_id(log, end_id)?;
        if !terminal_event_matches(end, plan) {
            return Err("評価結果の対話終了監査相関が不正です");
        }
    }
    let start_id = result["開始監査ID"].as_str();
    let end_id = result["終了監査ID"].as_str();
    let c4_terminal_interruption = result["結果状態"] == "中止"
        && result["実行系隔離予約監査ID"].as_str().is_some_and(valid_audit_id);
    // C4の予約付き中止だけは、隔離が既存対話の終端監査を代替しないことを明示した
    // 例外として終了監査なしを保持する。他のResultを対話終端なしで復元・比較へ
    // 進めることは許可しない。
    if !c4_terminal_interruption && end_id.is_none() {
        return Err("C4隔離以外の評価結果に対話終了監査がありません");
    }
    if start_id == Some(plan.created_audit_id.as_str())
        || end_id == Some(plan.created_audit_id.as_str())
        || start_id == Some(plan.isolation_audit_id.as_str())
        || end_id == Some(plan.isolation_audit_id.as_str())
        || (start_id.is_some() && start_id == end_id)
    {
        return Err("評価結果の対話監査IDが重複しています");
    }
    if result["結果状態"] == "成功"
        && (start_id.is_none()
            || end_id.is_none()
            || !result["応答hash"].as_str().is_some_and(valid_hash))
    {
        return Err("成功した評価結果の対話終端または応答hashが不正です");
    }
    Ok(())
}

fn created_dialogue_event_matches(event: &BrokerAuditEvent, plan: &PersistedEvaluationPlan) -> bool {
    event.operation == "対話送信"
        && event.decision == "recorded"
        && event.request_id == plan.request_id
        && event.payload_hash == plan.request_hash
        && event.evidence_source == EVIDENCE_SOURCE_INTERNAL_STATE
        && event.reason == "対話承認待ち作成"
}

fn isolation_event_matches(event: &BrokerAuditEvent, plan: &PersistedEvaluationPlan) -> bool {
    event.operation == "評価対話隔離"
        && event.decision == "recorded"
        && event.request_id == plan.request_id
        && event.evidence_source == EVIDENCE_SOURCE_INTERNAL_STATE
        && event.reason == "評価専用対話を通常の対話取得から隔離"
        && event.payload_hash == canonical_payload_hash(Some(&json!({"要求ID": plan.request_id})))
}

fn session_start_event_matches(event: &BrokerAuditEvent, plan: &PersistedEvaluationPlan) -> bool {
    event.operation == "対話開始"
        && event.decision == "recorded"
        && event.request_id == plan.session_id
        && event.reason == "対話開始"
        && event.evidence_source == EVIDENCE_SOURCE_INTERNAL_STATE
        && event.payload_hash == 対話開始監査hash(&plan.session_id, &plan.runtime_id)
}

fn session_start_request_event_matches(
    event: &BrokerAuditEvent,
    outer_request_id: &str,
    plan: &PersistedEvaluationPlan,
) -> bool {
    event.operation == "対話開始"
        && event.decision == "received"
        && event.request_id == outer_request_id
        && event.reason == "評価実験が既存対話統治経路を要求"
        && event.evidence_source == EVIDENCE_SOURCE_INTERNAL_STATE
        && event.payload_hash
            == canonical_payload_hash(Some(&json!({"実行系ID": plan.runtime_id})))
}

fn send_request_event_matches(
    event: &BrokerAuditEvent,
    outer_request_id: &str,
    expected_payload_hash: &str,
) -> bool {
    event.operation == "対話送信"
        && event.decision == "received"
        && event.request_id == outer_request_id
        && event.reason == "評価実験が既存対話統治経路を要求"
        && event.evidence_source == EVIDENCE_SOURCE_INTERNAL_STATE
        && event.payload_hash == expected_payload_hash
}

fn isolation_request_event_matches(
    event: &BrokerAuditEvent,
    outer_request_id: &str,
    plan: &PersistedEvaluationPlan,
) -> bool {
    event.operation == "評価対話隔離"
        && event.decision == "received"
        && event.request_id == outer_request_id
        && event.reason == "評価実験が既存対話を通常取得から隔離するよう要求"
        && event.evidence_source == EVIDENCE_SOURCE_INTERNAL_STATE
        && event.payload_hash
            == canonical_payload_hash(Some(&json!({"要求ID": plan.request_id})))
}

fn start_event_visibility(
    event: &BrokerAuditEvent,
    plan: &PersistedEvaluationPlan,
) -> Option<&'static str> {
    if event.operation != "対話承認"
        || event.decision != "recorded"
        || event.request_id != plan.request_id
        || event.payload_hash != plan.request_hash
        || event.evidence_source != EVIDENCE_SOURCE_INTERNAL_STATE
    {
        return None;
    }
    ["none", "hash_only", "summary", "redacted", "full"]
        .into_iter()
        .find(|visibility| {
            event.reason
                == format!(
                    "対話送信承認 Capability=対話送信 Permission={} Approval={} 表示範囲={} 評価隔離=true RecoveryAction=接続再確認",
                    plan.runtime_id, plan.request_hash, visibility,
                )
        })
}

fn start_event_matches(event: &BrokerAuditEvent, plan: &PersistedEvaluationPlan) -> bool {
    start_event_visibility(event, plan).is_some()
}

/// `対話取得`は進捗・結果証跡にも使われるため、operationだけでは終端を表さない。
/// 既存対話制御は各操作の開始時に進捗を反映するため、`対話完了`はそれを観測した
/// 正規の対話操作（例えば別Caseのowner承認）で記録される。終端reasonとrequest IDを
/// 検証し、caller操作名を終端の意味へすり替えない。
fn dialogue_progress_operation(operation: &str) -> bool {
    matches!(
        operation,
        "実行系列挙"
            | "対話開始"
            | "対話送信"
            | "対話承認待ち"
            | "対話承認"
            | "対話取得"
            | "対話中止"
            | "対話終了"
    )
}

/// 既存対話制御が出す終端reasonだけを受理し、種別ごとのpayload hashの意味を混同しない。
fn terminal_event_matches(event: &BrokerAuditEvent, plan: &PersistedEvaluationPlan) -> bool {
    if event.decision != "recorded"
        || event.request_id != plan.request_id
        || event.evidence_source != EVIDENCE_SOURCE_INTERNAL_STATE
    {
        return false;
    }
    match (event.operation.as_str(), event.reason.as_str()) {
        ("対話中止", "対話中止 実行系の停止は保証しない")
        | ("対話承認", "対話worker起動失敗") => event.payload_hash == plan.request_hash,
        (_, "対話期限超過") if dialogue_progress_operation(&event.operation) => {
            event.payload_hash == plan.request_hash
        }
        (_, "対話完了") => dialogue_progress_operation(&event.operation),
        _ => false,
    }
}

/// C4のterminal隔離は、既存対話を停止したという推測ではなく、通常資格経路が先に
/// 記録したdurable reservationにだけ基づく。C5の中止ResultはこのIDを固定するため、
/// 再起動後に別Runtimeや過去Experimentの隔離recordを流用できない。
fn terminal_quarantine_reservation_event_matches(
    event: &BrokerAuditEvent,
    runtime_id: &str,
) -> bool {
    event.operation == "実行系ライフサイクル操作"
        && event.decision == "received"
        && event.evidence_source == EVIDENCE_SOURCE_INTERNAL_STATE
        && event.reason == RuntimeLifecycleRegistry::quarantine_reservation_reason(runtime_id)
}

fn terminal_quarantine_reservation_audit_id(
    log: &BrokerAuditLog,
    experiment_audit_id: &str,
    plan: &EvaluationPlan,
) -> Result<String, &'static str> {
    let (experiment_index, experiment_event) = audit_event_with_index(log, experiment_audit_id)?;
    if experiment_event.operation != "評価実験開始" {
        return Err("C4隔離対象の評価実験開始監査operationが不正です");
    }
    if experiment_event.decision != "accepted" {
        return Err("C4隔離対象の評価実験開始監査decisionが不正です");
    }
    if experiment_event.evidence_source != EVIDENCE_SOURCE_INTERNAL_STATE {
        return Err("C4隔離対象の評価実験開始監査証拠種別が不正です");
    }
    let reservations = log
        .events()
        .iter()
        .enumerate()
        .filter(|(index, event)| {
            *index > experiment_index
                && terminal_quarantine_reservation_event_matches(event, &plan.runtime_id)
        })
        .collect::<Vec<_>>();
    if reservations.len() != 1 {
        return Err("C4実行系隔離予約監査を一意に特定できません");
    }
    Ok(reservations[0].1.event_id.clone())
}

fn validate_plan_audit_correlations(
    log: &BrokerAuditLog,
    plan: &PersistedEvaluationPlan,
    plan_event_index: usize,
    experiment_event_index: usize,
    outer_request_id: &str,
) -> Result<(), &'static str> {
    let (session_start_request_index, session_start_request) =
        audit_event_with_index(log, &plan.session_start_request_audit_id)?;
    let (session_start_index, session_start) =
        audit_event_with_index(log, &plan.session_start_audit_id)?;
    let (send_request_index, send_request) =
        audit_event_with_index(log, &plan.send_request_audit_id)?;
    let (created_index, created) = audit_event_with_index(log, &plan.created_audit_id)?;
    let (isolation_request_index, isolation_request) =
        audit_event_with_index(log, &plan.isolation_request_audit_id)?;
    let (isolation_index, isolation) = audit_event_with_index(log, &plan.isolation_audit_id)?;
    if !session_start_request_event_matches(session_start_request, outer_request_id, plan)
        || !session_start_event_matches(session_start, plan)
        || !send_request_event_matches(send_request, outer_request_id, &plan.send_payload_hash)
        || !created_dialogue_event_matches(created, plan)
        || !isolation_event_matches(isolation, plan)
        || !isolation_request_event_matches(isolation_request, outer_request_id, plan)
        || !(session_start_request_index < session_start_index
            && session_start_index < send_request_index
            && send_request_index < created_index
            && created_index < isolation_request_index
            && isolation_request_index < isolation_index
            && isolation_index < plan_event_index
            && plan_event_index < experiment_event_index)
    {
        return Err("評価Experiment計画のSession・対話作成・隔離監査順序が不正です");
    }
    Ok(())
}

fn validate_recovered_result_causality(
    log: &BrokerAuditLog,
    result: &Value,
    plan: &PersistedEvaluationPlan,
    plan_event_index: usize,
    experiment_event_index: usize,
    outer_request_id: &str,
    result_event_index: usize,
) -> Result<(), &'static str> {
    validate_plan_audit_correlations(
        log,
        plan,
        plan_event_index,
        experiment_event_index,
        outer_request_id,
    )?;
    if result_event_index <= experiment_event_index {
        return Err("評価結果監査が評価Experiment開始より先行しています");
    }
    let (start_index, start_visibility) = match result["開始監査ID"].as_str() {
        Some(id) => {
            let (index, event) = audit_event_with_index(log, id)?;
            let visibility = start_event_visibility(event, plan);
            if visibility.is_none()
                || !(experiment_event_index < index && index < result_event_index)
            {
                return Err("評価結果の対話開始監査順序が不正です");
            }
            (Some(index), visibility)
        }
        None => (None, None),
    };
    let end_index = match result["終了監査ID"].as_str() {
        Some(id) => {
            let (index, event) = audit_event_with_index(log, id)?;
            if !terminal_event_matches(event, plan)
                || !(experiment_event_index < index && index < result_event_index)
                || start_index.is_some_and(|start| index <= start)
            {
                return Err("評価結果の対話終了監査順序が不正です");
            }
            Some(index)
        }
        None => None,
    };
    let c4_terminal_interruption = result["結果状態"] == "中止"
        && result["実行系隔離予約監査ID"].as_str().is_some_and(valid_audit_id);
    if !c4_terminal_interruption && end_index.is_none() {
        return Err("C4隔離以外の評価結果に対話終了監査がありません");
    }
    if result["結果状態"] == "中止" {
        let reservations = log
            .events()
            .iter()
            .enumerate()
            .filter(|(index, event)| {
                experiment_event_index < *index
                    && *index < result_event_index
                    && terminal_quarantine_reservation_event_matches(event, &plan.runtime_id)
            })
            .collect::<Vec<_>>();
        match (result["実行系隔離予約監査ID"].as_str(), reservations.as_slice()) {
            (None, []) if result["実行系隔離予約監査ID"].is_null() => {}
            (Some(reservation_id), [(_, reservation)])
                if reservation.event_id == reservation_id => {}
            _ => return Err("中止した評価結果のC4実行系隔離予約監査相関が不正です"),
        }
    }
    if result["結果状態"] == "成功" {
        let end_index = end_index.ok_or("成功した評価結果に終端監査がありません")?;
        let start_visibility = start_visibility
            .ok_or("成功した評価結果にowner表示範囲承認がありません")?;
        let response_hash = result["応答hash"]
            .as_str()
            .filter(|value| valid_hash(value))
            .ok_or("成功した評価結果の応答hashが不正です")?;
        validate_result_proof(
            log,
            plan,
            result,
            response_hash,
            end_index,
            result_event_index,
            start_visibility,
        )?;
    }
    Ok(())
}

/// 通常経路でC5対話を回収したrecord。C4 terminal隔離はこのeventを生成しないため、
/// その中断結果を比較可能な証拠へ昇格させない。
fn normal_dialogue_cleanup_event_matches(
    event: &BrokerAuditEvent,
    plan: &PersistedEvaluationPlan,
) -> bool {
    event.operation == "対話終了"
        && event.decision == "recorded"
        && event.request_id == plan.session_id
        && event.reason == "対話終了"
        && event.evidence_source == EVIDENCE_SOURCE_INTERNAL_STATE
        && event.payload_hash == sha256_tagged(plan.session_id.as_bytes())
}

fn normal_dialogue_cleanup_verified(
    log: &BrokerAuditLog,
    plan: &PersistedEvaluationPlan,
    result_event_index: usize,
) -> Result<bool, &'static str> {
    let events = log
        .events()
        .iter()
        .enumerate()
        .filter(|(index, event)| {
            *index > result_event_index && normal_dialogue_cleanup_event_matches(event, plan)
        })
        .collect::<Vec<_>>();
    if events.len() > 1 {
        return Err("評価対話終了監査が重複しています");
    }
    Ok(events.len() == 1)
}

fn recovered_cleanup_verified(
    log: &BrokerAuditLog,
    experiment: &RecoveredExperiment,
    results: &[Value],
) -> Result<bool, &'static str> {
    if results.len() != experiment.plan_record.plans.len() {
        return Ok(false);
    }
    let mut plans = BTreeMap::new();
    for plan in &experiment.plan_record.plans {
        if plans
            .insert((plan.case_id.as_str(), plan.runtime_id.as_str()), plan)
            .is_some()
        {
            return Err("評価Experiment計画が重複しています");
        }
    }
    for result in results {
        let case_id = result["評価CaseID"]
            .as_str()
            .ok_or("評価結果recordのCase IDが不正です")?;
        let runtime_id = result["実行系ID"]
            .as_str()
            .ok_or("評価結果recordのRuntime IDが不正です")?;
        let plan = plans
            .get(&(case_id, runtime_id))
            .ok_or("評価結果が評価Experiment計画にありません")?;
        let audit_id = result["評価監査ID"]
            .as_str()
            .ok_or("評価結果監査IDが不正です")?;
        let (result_index, event) = audit_event_with_index(log, audit_id)?;
        if event.operation != "評価実験状態" || event.decision != "recorded" {
            return Err("評価結果監査相関が不正です");
        }
        if !normal_dialogue_cleanup_verified(log, plan, result_index)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn validate_result_proof(
    log: &BrokerAuditLog,
    plan: &PersistedEvaluationPlan,
    result: &Value,
    response_hash: &str,
    end_index: usize,
    result_event_index: usize,
    start_visibility: &str,
) -> Result<(), &'static str> {
    let proofs = log
        .events()
        .iter()
        .enumerate()
        .filter(|(index, event)| {
            *index > end_index
                && *index < result_event_index
                // `記録保存`は各既存対話操作の進捗反映から呼ばれる。結果証跡も
                // 完了を観測した正規対話操作の文脈で記録されるため、取得操作だけに
                // 限定しない。
                && dialogue_progress_operation(&event.operation)
                && event.decision == "recorded"
                && event.request_id == plan.request_id
                && event.evidence_source == EVIDENCE_SOURCE_INTERNAL_STATE
                && event.reason.starts_with("対話結果証跡:")
        })
        .collect::<Vec<_>>();
    if proofs.len() != 1 {
        return Err("成功した評価結果の対話結果証跡が不正です");
    }
    let (_, event) = proofs[0];
    let encoded = event
        .reason
        .strip_prefix("対話結果証跡:")
        .filter(|value| value.len() <= 16 * 1024)
        .ok_or("成功した評価結果の対話結果証跡が不正です")?;
    if event.payload_hash != crate::audit_hash::sha256_tagged(encoded.as_bytes()) {
        return Err("成功した評価結果の対話結果証跡hashが不正です");
    }
    let proof: Value = serde_json::from_str(encoded)
        .map_err(|_| "成功した評価結果の対話結果証跡形式が不正です")?;
    let object = exact_object(
        &proof,
        &[
            "版",
            "要求ID",
            "対話セッションID",
            "実行系ID",
            "要求hash",
            "終了監査ID",
            "表示範囲",
            "応答hash",
            "能力申告hash",
            "経路申告hash",
            "追跡参照hash",
            "証拠種別",
        ],
    )
    .map_err(|_| "成功した評価結果の対話結果証跡fieldが不正です")?;
    let visibility = object
        .get("表示範囲")
        .and_then(Value::as_str)
        .filter(|value| matches!(*value, "hash_only" | "summary" | "redacted" | "full"))
        .ok_or("成功した評価結果の対話結果証跡表示範囲が不正です")?;
    let optional_hashes = ["能力申告hash", "経路申告hash", "追跡参照hash"];
    let optional_hashes_valid = if visibility == "full" {
        optional_hashes.iter().all(|field| {
            object
                .get(*field)
                .and_then(Value::as_str)
                .is_some_and(valid_hash)
        })
    } else {
        optional_hashes
            .iter()
            .all(|field| object.get(*field).is_some_and(Value::is_null))
    };
    if object.get("版").and_then(Value::as_u64) != Some(1)
        || !optional_hashes_valid
        || visibility != start_visibility
        || proof["要求ID"] != plan.request_id
        || proof["対話セッションID"] != plan.session_id
        || proof["実行系ID"] != plan.runtime_id
        || proof["要求hash"] != plan.request_hash
        || proof["終了監査ID"] != result["終了監査ID"]
        || proof["応答hash"].as_str().filter(|value| valid_hash(value)) != Some(response_hash)
        || proof["証拠種別"] != EVIDENCE_SOURCE_INTERNAL_STATE
    {
        return Err("成功した評価結果の対話結果証跡相関が不正です");
    }
    Ok(())
}

/// terminal隔離で対話制御がsessionを解放した後も、既存監査に実在する開始・終了だけを
/// C5結果へ相関させる。存在しない終了を推測で生成しない。
fn terminal_dialogue_audit_ids(
    log: &BrokerAuditLog,
    plan: &EvaluationPlan,
) -> Result<(Value, Value), &'static str> {
    let (created_index, created) = audit_event_with_index(log, &plan.created_audit_id)?;
    if !created_dialogue_event_matches(created, &persisted_plan(plan)) {
        return Err("terminal隔離前の対話作成監査相関が不正です");
    }
    let (isolation_index, isolation) = audit_event_with_index(log, &plan.isolation_audit_id)?;
    if created_index >= isolation_index || !isolation_event_matches(isolation, &persisted_plan(plan)) {
        return Err("terminal隔離前の対話隔離監査相関が不正です");
    }
    let matching = |event: &&BrokerAuditEvent| {
        event.decision == "recorded"
            && event.request_id == plan.request_id
            && event.evidence_source == EVIDENCE_SOURCE_INTERNAL_STATE
    };
    let starts = log
        .events()
        .iter()
        .enumerate()
        .filter(|(index, event)| *index > isolation_index && matching(event))
        .filter(|event| {
            start_event_matches(event.1, &persisted_plan(plan))
        })
        .collect::<Vec<_>>();
    if starts.len() > 1 {
        return Err("terminal隔離前の対話開始監査が重複しています");
    }
    let ends = log
        .events()
        .iter()
        .enumerate()
        .filter(|(index, event)| *index > isolation_index && matching(event))
        .filter(|event| {
            terminal_event_matches(event.1, &persisted_plan(plan))
        })
        .collect::<Vec<_>>();
    if ends.len() > 1 {
        return Err("terminal隔離前の対話終了監査が重複しています");
    }
    Ok((
        starts
            .first()
            .map(|(_, event)| json!(event.event_id))
            .unwrap_or(Value::Null),
        ends.first()
            .map(|(_, event)| json!(event.event_id))
            .unwrap_or(Value::Null),
    ))
}

fn persisted_plan(plan: &EvaluationPlan) -> PersistedEvaluationPlan {
    PersistedEvaluationPlan {
        case_id: plan.case_id.clone(),
        runtime_id: plan.runtime_id.clone(),
        request_id: plan.request_id.clone(),
        request_hash: plan.request_hash.clone(),
        session_id: plan.session_id.clone(),
        session_start_request_audit_id: plan.session_start_request_audit_id.clone(),
        session_start_audit_id: plan.session_start_audit_id.clone(),
        send_request_audit_id: plan.send_request_audit_id.clone(),
        send_payload_hash: plan.send_payload_hash.clone(),
        created_audit_id: plan.created_audit_id.clone(),
        isolation_request_audit_id: plan.isolation_request_audit_id.clone(),
        isolation_audit_id: plan.isolation_audit_id.clone(),
    }
}

fn recovered_experiment(
    log: &BrokerAuditLog,
    experiment_id: &str,
) -> Result<Option<RecoveredExperiment>, &'static str> {
    let manifests = evaluation_manifests(log)?;
    let mut plan_records = BTreeMap::new();
    let mut found = None;
    for (index, event) in log.events().iter().enumerate() {
        if event.operation != "評価実験開始" {
            continue;
        }
        match event.decision.as_str() {
            "recorded" => {
                let raw = event
                    .reason
                    .strip_prefix("評価Experiment計画記録:")
                    .ok_or("評価Experiment計画監査reasonが不正です")?;
                let value: Value = serde_json::from_str(raw)
                    .map_err(|_| "評価Experiment計画recordを検証できません")?;
                let plan_record = parse_persisted_plan_record(&value)?;
                if event.evidence_source != EVIDENCE_SOURCE_INTERNAL_STATE
                    || event.payload_hash != canonical_payload_hash(Some(&value))
                    || plan_records
                        .insert(
                            event.event_id.clone(),
                            (index, event.request_id.clone(), plan_record),
                        )
                        .is_some()
                {
                    return Err("評価Experiment計画監査相関が不正です");
                }
            }
            "accepted" => {
                let raw = event
                    .reason
                    .strip_prefix("評価Experiment開始記録:")
                    .ok_or("評価Experiment監査reasonが不正です")?;
                let value: Value = serde_json::from_str(raw)
                    .map_err(|_| "評価Experiment記録を検証できません")?;
                if !valid_experiment(&value)
                    || event.event_id != value["実験監査ID"]
                    || event.evidence_source != EVIDENCE_SOURCE_INTERNAL_STATE
                    || event.payload_hash != canonical_payload_hash(Some(&value))
                {
                    return Err("評価Experiment監査相関が不正です");
                }
                if value["評価ExperimentID"] == experiment_id {
                    if found.is_some() {
                        return Err("評価Experiment記録が重複しています");
                    }
                    found = Some((value, index, event.request_id.clone()));
                }
            }
            "received" => {}
            _ => return Err("評価Experiment監査decisionが不正です"),
        }
    }
    let Some((public, experiment_index, experiment_request_id)) = found else {
        return Ok(None);
    };
    let plan_audit_id = public["計画監査ID"]
        .as_str()
        .ok_or("評価Experiment計画監査IDがありません")?;
    let Some((plan_index, plan_request_id, plan_record)) = plan_records.remove(plan_audit_id) else {
        return Err("評価Experiment計画監査recordがありません");
    };
    if plan_index >= experiment_index || plan_request_id != experiment_request_id {
        return Err("評価Experiment計画と開始監査の順序またはrequestが不正です");
    }
    let dataset_id = public["評価DatasetID"]
        .as_str()
        .ok_or("評価Experiment記録を検証できません")?;
    let definition_hash = public["Dataset定義hash"]
        .as_str()
        .ok_or("評価Experiment記録を検証できません")?;
    let mut matching_manifests = manifests.into_iter().filter(|manifest| {
        manifest.dataset_id == dataset_id && manifest.definition_hash == definition_hash
    });
    let manifest = matching_manifests
        .next()
        .ok_or("評価ExperimentのDataset manifestがありません")?;
    if matching_manifests.next().is_some() {
        return Err("評価ExperimentのDataset manifestが曖昧です");
    }
    validate_plan_against_experiment(&plan_record, &public, &manifest)?;
    for plan in &plan_record.plans {
        validate_plan_audit_correlations(
            log,
            plan,
            plan_index,
            experiment_index,
            &experiment_request_id,
        )?;
    }
    Ok(Some(RecoveredExperiment {
        public,
        plan_record,
        plan_event_index: plan_index,
        experiment_event_index: experiment_index,
        outer_request_id: experiment_request_id,
    }))
}

fn recovered_results(
    log: &BrokerAuditLog,
    experiment: &RecoveredExperiment,
) -> Result<Vec<Value>, &'static str> {
    let experiment_id = experiment.public["評価ExperimentID"]
        .as_str()
        .ok_or("評価Experiment記録を検証できません")?;
    let mut plans = BTreeMap::new();
    for plan in &experiment.plan_record.plans {
        if plans
            .insert((plan.case_id.clone(), plan.runtime_id.clone()), plan)
            .is_some()
        {
            return Err("評価Experiment計画が重複しています");
        }
    }
    let mut results = Vec::new();
    let mut pairs = BTreeSet::new();
    for (result_event_index, event) in log.events().iter().enumerate() {
        if event.operation != "評価実験状態" || event.decision != "recorded" {
            continue;
        }
        let raw = event
            .reason
            .strip_prefix("評価結果記録:")
            .ok_or("評価結果監査reasonが不正です")?;
        let value: Value = serde_json::from_str(raw)
            .map_err(|_| "評価結果記録を検証できません")?;
        if !valid_result(&value) {
            return Err("評価結果記録を検証できません");
        }
        if value["評価ExperimentID"] != experiment_id {
            continue;
        }
        if event.event_id != value["評価監査ID"]
            || event.request_id != value["対話要求ID"]
            || event.evidence_source != EVIDENCE_SOURCE_INTERNAL_STATE
            || event.payload_hash != canonical_payload_hash(Some(&value))
        {
            return Err("評価結果監査相関が不正です");
        }
        let case_id = value["評価CaseID"]
            .as_str()
            .ok_or("評価結果recordのCase IDが不正です")?;
        let runtime_id = value["実行系ID"]
            .as_str()
            .ok_or("評価結果recordのRuntime IDが不正です")?;
        let plan = plans
            .get(&(case_id.to_owned(), runtime_id.to_owned()))
            .ok_or("評価結果が評価Experiment計画にありません")?;
        result_matches_plan(log, &value, plan)?;
        validate_recovered_result_causality(
            log,
            &value,
            plan,
            experiment.plan_event_index,
            experiment.experiment_event_index,
            &experiment.outer_request_id,
            result_event_index,
        )?;
        if !pairs.insert((case_id.to_owned(), runtime_id.to_owned())) {
            return Err("評価結果記録が重複しています");
        }
        results.push(value);
    }
    Ok(results)
}

fn evaluation_manifests(log: &BrokerAuditLog) -> Result<Vec<DatasetManifest>, &'static str> {
    let mut manifests = Vec::new();
    let mut ids = BTreeSet::new();
    let mut latest_revision = BTreeMap::new();
    for event in log.events() {
        if event.operation != "評価Dataset登録" || event.decision != "accepted" {
            continue;
        }
        let raw = event
            .reason
            .strip_prefix("評価Dataset登録記録:")
            .ok_or("評価Dataset監査reasonが不正です")?;
        let value: Value = serde_json::from_str(raw).map_err(|_| "評価Dataset記録を検証できません")?;
        let manifest = parse_manifest(value)?;
        if event.event_id != manifest.value["作成監査ID"]
            || event.evidence_source != EVIDENCE_SOURCE_INTERNAL_STATE
            || event.payload_hash != canonical_payload_hash(Some(&manifest.value))
        {
            return Err("評価Dataset監査相関が不正です");
        }
        if !ids.insert((manifest.dataset_id.clone(), manifest.revision))
            || latest_revision
                .get(&manifest.dataset_id)
                .is_some_and(|revision| manifest.revision <= *revision)
        {
            return Err("評価Dataset revisionが重複しています");
        }
        latest_revision.insert(manifest.dataset_id.clone(), manifest.revision);
        manifests.push(manifest);
    }
    manifests.sort_by(|left, right| left.dataset_id.cmp(&right.dataset_id).then(left.revision.cmp(&right.revision)));
    Ok(manifests)
}

fn parse_registration(value: &Value) -> Result<PrivateRegistration, &'static str> {
    let object = exact_object(value, &["版", "評価DatasetID", "revision", "公開表示名", "登録者種別", "登録経路", "評価用途", "評価方式", "非公開CasePayload一覧"])?;
    if object.get("版").and_then(Value::as_u64) != Some(EVALUATION_VERSION)
        || object.get("登録者種別").and_then(Value::as_str) != Some("owner")
        || object.get("登録経路").and_then(Value::as_str) != Some("owner_control")
        || object.get("評価用途").and_then(Value::as_str) != Some("運用観測")
        || object.get("評価方式").and_then(Value::as_str) != Some("決定論")
    {
        return Err("評価Datasetの固定fieldが不正です");
    }
    let dataset_id = required_identifier(object, "評価DatasetID")?;
    let revision = object.get("revision").and_then(Value::as_u64).filter(|value| *value >= 1).ok_or("評価Dataset revisionが不正です")?;
    let public_name = bounded_string(object.get("公開表示名"), 128, "公開表示名が不正です")?;
    let cases = object.get("非公開CasePayload一覧").and_then(Value::as_array).filter(|value| !value.is_empty() && value.len() <= MAX_DATASETS_CASES).ok_or("評価Case一覧が不正です")?;
    let mut output = Vec::with_capacity(cases.len());
    let mut case_ids = BTreeSet::new();
    let mut orders = BTreeSet::new();
    for raw in cases {
        let case = parse_private_case(raw)?;
        if !case_ids.insert(case.case_id.clone()) || !orders.insert(case.order) {
            return Err("評価CaseIDまたは順序が重複しています");
        }
        output.push(case);
    }
    output.sort_by_key(|case| case.order);
    Ok(PrivateRegistration { dataset_id, revision, public_name, cases: output })
}

fn parse_private_case(value: &Value) -> Result<PrivateCase, &'static str> {
    let object = exact_object(value, &["評価CaseID", "順序", "公開表示名", "入力", "評価器設定"])?;
    let case_id = required_identifier(object, "評価CaseID")?;
    let order = object.get("順序").and_then(Value::as_u64).filter(|value| (1..=MAX_DATASETS_CASES as u64).contains(value)).ok_or("評価Case順序が不正です")?;
    let _ = bounded_string(
        object.get("公開表示名"),
        128,
        "評価Case公開表示名が不正です",
    )?;
    let input = exact_object(object.get("入力").ok_or("評価Case入力が不正です")?, &["内容表示範囲", "本文"])?;
    if input.get("内容表示範囲").and_then(Value::as_str) != Some("full") {
        return Err("評価Case入力の表示範囲が不正です");
    }
    let body = bounded_string(input.get("本文"), 4096, "評価Case入力本文が不正です")?;
    let evaluators = object.get("評価器設定").and_then(Value::as_array).filter(|value| !value.is_empty() && value.len() <= MAX_EVALUATORS_PER_CASE).ok_or("評価器設定が不正です")?;
    let mut evaluator_ids = BTreeSet::new();
    for evaluator in evaluators {
        let id = validate_private_evaluator(evaluator)?;
        if !evaluator_ids.insert(id) {
            return Err("評価器IDが重複しています");
        }
    }
    Ok(PrivateCase {
        case_id,
        order,
        input: body,
        evaluators: Value::Array(evaluators.clone()),
        definition_hash: canonical_payload_hash(Some(value)),
    })
}

fn validate_private_evaluator(value: &Value) -> Result<String, &'static str> {
    let object = exact_object(value, &["評価器ID", "種類", "設定"])?;
    let id = required_identifier(object, "評価器ID")?;
    let kind = bounded_string(object.get("種類"), 64, "評価器種類が不正です")?;
    if !matches!(kind.as_str(), "exact" | "contains" | "regex" | "json_schema" | "reference_count" | "route" | "status" | "capability" | "latency_threshold") {
        return Err("評価器種類が未対応です");
    }
    let setting = object.get("設定").ok_or("評価器設定が不正です")?;
    let fields: &[&str] = match kind.as_str() {
        "exact" => &["期待本文"],
        "contains" => &["期待断片"],
        "regex" => &["パターン"],
        "json_schema" => &["期待Schema"],
        "reference_count" => &["最小数", "最大数"],
        "route" => &["期待経路"],
        "status" => &["期待状態"],
        "capability" => &["必要能力一覧"],
        "latency_threshold" => &["最大Millis"],
        _ => return Err("評価器種類が未対応です"),
    };
    let setting = exact_object(setting, fields)?;
    match kind.as_str() {
        "exact" => { bounded_string(setting.get("期待本文"), 65_536, "exact設定が不正です")?; }
        "contains" | "regex" => { bounded_string(setting.get(if kind == "contains" { "期待断片" } else { "パターン" }), 8192, "文字列評価器設定が不正です")?; }
        "json_schema" => { if !setting.get("期待Schema").is_some_and(Value::is_object) { return Err("JSON Schema設定が不正です"); } }
        "reference_count" => {
            let min = setting.get("最小数").and_then(Value::as_u64).filter(|value| *value <= 64).ok_or("参照数設定が不正です")?;
            let max = setting.get("最大数").and_then(Value::as_u64).filter(|value| *value <= 64 && *value >= min).ok_or("参照数設定が不正です")?;
            let _ = max;
        }
        "route" => { bounded_string(setting.get("期待経路"), 256, "経路設定が不正です")?; }
        "status" => {
            if !matches!(setting.get("期待状態").and_then(Value::as_str), Some("成功" | "保留" | "失敗" | "中止")) { return Err("状態設定が不正です"); }
        }
        "capability" => {
            let values = setting.get("必要能力一覧").and_then(Value::as_array).filter(|value| !value.is_empty() && value.len() <= 64).ok_or("能力設定が不正です")?;
            if values.iter().any(|value| bounded_string(Some(value), 256, "能力設定が不正です").is_err()) { return Err("能力設定が不正です"); }
        }
        "latency_threshold" => { setting.get("最大Millis").and_then(Value::as_u64).filter(|value| *value <= 86_400_000).ok_or("遅延設定が不正です")?; }
        _ => return Err("評価器種類が未対応です"),
    }
    Ok(id)
}

fn public_manifest(
    raw: &Value,
    registration: &PrivateRegistration,
    storage_id: &str,
    ciphertext_hash: &str,
    created_at: i64,
    audit_id: &str,
) -> Result<Value, &'static str> {
    let definition_hash = canonical_payload_hash(Some(raw));
    let cases = registration.cases.iter().map(|case| json!({"評価CaseID": case.case_id, "定義hash": case.definition_hash})).collect::<Vec<_>>();
    let value = json!({
        "版": EVALUATION_VERSION,
        "評価DatasetID": registration.dataset_id,
        "revision": registration.revision,
        "定義hash": definition_hash,
        "非公開保管ID": storage_id,
        "暗号文hash": ciphertext_hash,
        "公開表示名": registration.public_name,
        "Case数": registration.cases.len(),
        "Case一覧": cases,
        "公開範囲": "hash_only",
        "作成時刻UnixMillis": created_at,
        "作成監査ID": audit_id,
        "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
    });
    valid_manifest(&value).then_some(value).ok_or("公開manifestが不正です")
}

fn parse_manifest(value: Value) -> Result<DatasetManifest, &'static str> {
    if !valid_manifest(&value) { return Err("評価Dataset公開manifestが不正です"); }
    Ok(DatasetManifest {
        dataset_id: value["評価DatasetID"].as_str().unwrap_or_default().to_owned(),
        revision: value["revision"].as_u64().unwrap_or_default(),
        definition_hash: value["定義hash"].as_str().unwrap_or_default().to_owned(),
        storage_id: value["非公開保管ID"].as_str().unwrap_or_default().to_owned(),
        ciphertext_hash: value["暗号文hash"].as_str().unwrap_or_default().to_owned(),
        value,
    })
}

fn start_request(value: &Value) -> Result<(String, Vec<String>), &'static str> {
    let object = exact_object(value, &["版", "評価DatasetID", "対象Runtime一覧"])?;
    if object.get("版").and_then(Value::as_u64) != Some(EVALUATION_VERSION) { return Err("評価実験開始の版が不正です"); }
    let id = required_identifier(object, "評価DatasetID")?;
    let runtimes = object.get("対象Runtime一覧").and_then(Value::as_array).filter(|value| !value.is_empty() && value.len() <= MAX_TARGET_RUNTIMES).ok_or("評価対象Runtime一覧が不正です")?;
    let mut unique = BTreeSet::new();
    let mut output = Vec::with_capacity(runtimes.len());
    for runtime in runtimes {
        let runtime = runtime.as_str().filter(|value| 実行系ID妥当(value)).ok_or("評価対象RuntimeIDが不正です")?;
        if !unique.insert(runtime) { return Err("評価対象RuntimeIDが重複しています"); }
        output.push(runtime.to_owned());
    }
    Ok((id, output))
}

fn experiment_request(value: &Value) -> Result<String, &'static str> {
    let object = exact_object(value, &["版", "評価ExperimentID"])?;
    if object.get("版").and_then(Value::as_u64) != Some(EVALUATION_VERSION) { return Err("評価Experiment要求の版が不正です"); }
    required_identifier(object, "評価ExperimentID")
}

fn empty_version_payload(value: &Value) -> bool {
    exact_object(value, &["版"]).ok().is_some_and(|object| object.get("版").and_then(Value::as_u64) == Some(EVALUATION_VERSION))
}

fn success_response(request_id: &str, operation: &str, audit_event_id: String, body: Value) -> BrokerResponse {
    BrokerResponse {
        request_id: request_id.to_owned(),
        operation: operation.to_owned(),
        status: BrokerStatus::Accepted,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_owned(),
        audit_event_id,
        error: None,
        health: None,
        body: Some(body),
        shutdown_requested: false,
    }
}

fn accept_safe_body(broker: &mut Broker, request_id: &str, operation: &str, body: Value) -> BrokerResponse {
    match broker.append_audit(request_id, operation, "accepted", "評価ラボの安全な運用観測projectionを返却。Authority、release、security判断を生成しない", EVIDENCE_SOURCE_INTERNAL_STATE, &canonical_payload_hash(Some(&body))) {
        Ok(event) => success_response(request_id, operation, event.event_id, body),
        Err(_) => broker.audit_store_failed_response(request_id, operation, "監査失敗", "監査修復後に再確認してください"),
    }
}

fn exact_object<'a>(value: &'a Value, fields: &[&str]) -> Result<&'a Map<String, Value>, &'static str> {
    let object = value.as_object().ok_or("object形式が必要です")?;
    if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) || object.keys().any(|field| !fields.contains(&field.as_str())) {
        return Err("許可されないfieldがあります");
    }
    Ok(object)
}

fn required_identifier(object: &Map<String, Value>, field: &str) -> Result<String, &'static str> {
    object.get(field).and_then(Value::as_str).filter(|value| valid_identifier(value)).map(str::to_owned).ok_or("IDが不正です")
}

fn bounded_string(value: Option<&Value>, max: usize, error: &'static str) -> Result<String, &'static str> {
    value.and_then(Value::as_str).filter(|value| !value.is_empty() && value.chars().count() <= max).map(str::to_owned).ok_or(error)
}

fn valid_identifier(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
}

fn valid_hash(value: &str) -> bool {
    value.len() == 71 && value.starts_with("sha256:") && value.as_bytes()[7..].iter().all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(value))
}

fn valid_audit_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && value.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric) && value.bytes().all(|value| value.is_ascii_alphanumeric() || matches!(value, b'_' | b'.' | b':' | b'-'))
}

fn optional_audit(value: Option<&Value>) -> Result<Value, &'static str> {
    match value {
        Some(Value::Null) | None => Ok(Value::Null),
        Some(Value::String(value)) if valid_audit_id(value) => Ok(Value::String(value.clone())),
        _ => Err("監査IDが不正です"),
    }
}

fn storage_id(dataset_id: &str, revision: u64) -> String {
    sha256_tagged(format!("GUI-Shell:evaluation-private:v1:{dataset_id}:{revision}").as_bytes())[7..39].to_owned()
}

fn valid_manifest(value: &Value) -> bool {
    let Ok(object) = exact_object(value, &["版", "評価DatasetID", "revision", "定義hash", "非公開保管ID", "暗号文hash", "公開表示名", "Case数", "Case一覧", "公開範囲", "作成時刻UnixMillis", "作成監査ID", "証拠種別"]) else { return false; };
    object.get("版").and_then(Value::as_u64) == Some(EVALUATION_VERSION)
        && object.get("評価DatasetID").and_then(Value::as_str).is_some_and(valid_identifier)
        && object.get("revision").and_then(Value::as_u64).is_some_and(|value| value >= 1)
        && object.get("定義hash").and_then(Value::as_str).is_some_and(valid_hash)
        && object.get("非公開保管ID").and_then(Value::as_str).is_some_and(valid_identifier)
        && object.get("暗号文hash").and_then(Value::as_str).is_some_and(valid_hash)
        && object.get("公開表示名").and_then(Value::as_str).is_some_and(|value| !value.is_empty() && value.chars().count() <= 128)
        && object.get("Case数").and_then(Value::as_u64).is_some_and(|value| (1..=MAX_DATASETS_CASES as u64).contains(&value))
        && object.get("Case一覧").and_then(Value::as_array).is_some_and(|values| values.len() == object["Case数"].as_u64().unwrap_or_default() as usize && values.iter().all(|case| exact_object(case, &["評価CaseID", "定義hash"]).ok().is_some_and(|case| case.get("評価CaseID").and_then(Value::as_str).is_some_and(valid_identifier) && case.get("定義hash").and_then(Value::as_str).is_some_and(valid_hash))))
        && object.get("公開範囲").and_then(Value::as_str) == Some("hash_only")
        && object.get("作成時刻UnixMillis").and_then(Value::as_i64).is_some_and(|value| value >= 0)
        && object.get("作成監査ID").and_then(Value::as_str).is_some_and(valid_audit_id)
        && object.get("証拠種別").and_then(Value::as_str) == Some(EVIDENCE_SOURCE_INTERNAL_STATE)
}

fn valid_experiment(value: &Value) -> bool {
    let Ok(object) = exact_object(value, &["版", "評価ExperimentID", "評価DatasetID", "Dataset定義hash", "対象Runtime一覧", "状態", "計画Case数", "結果数", "作成時刻UnixMillis", "開始時刻UnixMillis", "終了時刻UnixMillis", "計画監査ID", "実験監査ID", "証拠種別"]) else { return false; };
    let Some(case_count) = object
        .get("計画Case数")
        .and_then(Value::as_u64)
        .filter(|value| (1..=MAX_DATASETS_CASES as u64).contains(value))
    else {
        return false;
    };
    let Some(runtimes) = object.get("対象Runtime一覧").and_then(Value::as_array) else {
        return false;
    };
    let Some(expected_result_count) = case_count.checked_mul(runtimes.len() as u64) else {
        return false;
    };
    let Some(result_count) = object.get("結果数").and_then(Value::as_u64) else {
        return false;
    };
    let state = object.get("状態").and_then(Value::as_str).unwrap_or_default();
    let started_at = object.get("開始時刻UnixMillis").and_then(Value::as_i64);
    let finished_at = object.get("終了時刻UnixMillis").and_then(Value::as_i64);
    object.get("版").and_then(Value::as_u64) == Some(EVALUATION_VERSION)
        && object.get("評価ExperimentID").and_then(Value::as_str).is_some_and(valid_identifier)
        && object.get("評価DatasetID").and_then(Value::as_str).is_some_and(valid_identifier)
        && object.get("Dataset定義hash").and_then(Value::as_str).is_some_and(valid_hash)
        && valid_runtime_list(object.get("対象Runtime一覧"))
        && matches!(state, "準備済み" | "承認待ち" | "実行中" | "完了" | "中断" | "評価不能")
        && result_count <= 1024
        && result_count <= expected_result_count
        && object.get("作成時刻UnixMillis").and_then(Value::as_i64).is_some_and(|value| value >= 0)
        && optional_nonnegative_int(object.get("開始時刻UnixMillis"))
        && optional_nonnegative_int(object.get("終了時刻UnixMillis"))
        && match (started_at, finished_at) {
            (Some(started), Some(finished)) => finished >= started,
            _ => true,
        }
        && valid_experiment_state(
            state,
            result_count,
            expected_result_count,
            started_at,
            finished_at,
        )
        && object.get("計画監査ID").and_then(Value::as_str).is_some_and(valid_audit_id)
        && object.get("実験監査ID").and_then(Value::as_str).is_some_and(valid_audit_id)
        && object.get("計画監査ID") != object.get("実験監査ID")
        && object.get("証拠種別").and_then(Value::as_str) == Some(EVIDENCE_SOURCE_INTERNAL_STATE)
}

fn valid_experiment_state(
    state: &str,
    result_count: u64,
    expected_result_count: u64,
    started_at: Option<i64>,
    finished_at: Option<i64>,
) -> bool {
    match state {
        "準備済み" | "承認待ち" => {
            result_count == 0 && started_at.is_none() && finished_at.is_none()
        }
        // 全Resultが監査済みでも既存対話の終了回収中であれば、完了へ昇格しない。
        "実行中" => finished_at.is_none(),
        "完了" => {
            result_count == expected_result_count
                && started_at.is_some()
                && finished_at.is_some()
        }
        // owner承認前の期限超過など、開始監査なしで終端する評価不能もある。
        "評価不能" => result_count == expected_result_count && finished_at.is_some(),
        // C4 terminal隔離や再起動復元では、部分Resultのまま中断を示し得る。
        "中断" => true,
        _ => false,
    }
}

fn valid_runtime_list(value: Option<&Value>) -> bool {
    let Some(values) = value.and_then(Value::as_array) else {
        return false;
    };
    if values.is_empty() || values.len() > MAX_TARGET_RUNTIMES {
        return false;
    }
    let mut ids = BTreeSet::new();
    values.iter().all(|value| {
        value
            .as_str()
            .filter(|value| 実行系ID妥当(value))
            .is_some_and(|value| ids.insert(value))
    })
}

fn valid_result(value: &Value) -> bool {
    let Ok(object) = exact_object(value, &["版", "評価ExperimentID", "評価DatasetID", "Dataset定義hash", "評価CaseID", "実行系ID", "対話要求ID", "対話セッションID", "作成監査ID", "開始監査ID", "終了監査ID", "実行系隔離予約監査ID", "評価監査ID", "結果状態", "判定", "評価器判定一覧", "実行時刻UnixMillis", "LatencyMillis", "応答hash", "証拠種別"]) else { return false; };
    object.get("版").and_then(Value::as_u64) == Some(EVALUATION_VERSION)
        && ["評価ExperimentID", "評価DatasetID", "評価CaseID", "対話要求ID", "対話セッションID"].iter().all(|field| object.get(*field).and_then(Value::as_str).is_some_and(valid_identifier))
        && object.get("Dataset定義hash").and_then(Value::as_str).is_some_and(valid_hash)
        && object.get("実行系ID").and_then(Value::as_str).is_some_and(実行系ID妥当)
        && object.get("作成監査ID").and_then(Value::as_str).is_some_and(valid_audit_id)
        && optional_audit(object.get("開始監査ID")).is_ok()
        && optional_audit(object.get("終了監査ID")).is_ok()
        && optional_audit(object.get("実行系隔離予約監査ID")).is_ok()
        && object.get("評価監査ID").and_then(Value::as_str).is_some_and(valid_audit_id)
        && object.get("結果状態").and_then(Value::as_str).is_some_and(|value| matches!(value, "成功" | "失敗" | "中止" | "評価不能"))
        && object.get("判定").and_then(Value::as_str).is_some_and(|value| matches!(value, "成立" | "不成立" | "評価不能" | "中断"))
        && match (
            object.get("結果状態").and_then(Value::as_str),
            object.get("判定").and_then(Value::as_str),
        ) {
            (Some("中止"), Some("中断")) | (Some("評価不能"), Some("評価不能")) => true,
            (Some("中止" | "評価不能"), _) => false,
            _ => true,
        }
        && match object.get("結果状態").and_then(Value::as_str) {
            Some("成功") => {
                object.get("開始監査ID").and_then(Value::as_str).is_some_and(valid_audit_id)
                    && object.get("終了監査ID").and_then(Value::as_str).is_some_and(valid_audit_id)
                    && object.get("実行系隔離予約監査ID").is_some_and(Value::is_null)
            }
            Some("中止") => true,
            Some("失敗" | "評価不能") => object
                .get("実行系隔離予約監査ID")
                .is_some_and(Value::is_null),
            _ => false,
        }
        && object.get("評価器判定一覧").and_then(Value::as_array).is_some_and(|values| !values.is_empty() && values.len() <= MAX_EVALUATORS_PER_CASE && values.iter().all(valid_evaluator_result))
        && object.get("実行時刻UnixMillis").and_then(Value::as_i64).is_some_and(|value| value >= 0)
        && optional_nonnegative_int(object.get("LatencyMillis"))
        && match object.get("応答hash") {
            Some(Value::Null) => object.get("結果状態").and_then(Value::as_str) != Some("成功"),
            Some(Value::String(value)) => valid_hash(value),
            _ => false,
        }
        && object.get("証拠種別").and_then(Value::as_str) == Some(EVIDENCE_SOURCE_INTERNAL_STATE)
}

/// 通常IPCとFlutterが扱うのは、内部Resultから明示的に縮約したこの形だけである。
/// C4予約、対話request/session、応答hash、開始終了監査を混入させない。
fn valid_public_result_projection(value: &Value) -> bool {
    let Ok(object) = exact_object(value, &["評価ExperimentID", "評価DatasetID", "評価CaseID", "実行系ID", "判定", "評価器判定一覧", "LatencyMillis", "総合hash", "評価監査ID"]) else { return false; };
    let Some(evaluators) = object.get("評価器判定一覧").and_then(Value::as_array) else {
        return false;
    };
    let mut evaluator_ids = BTreeSet::new();
    object.get("評価ExperimentID").and_then(Value::as_str).is_some_and(valid_identifier)
        && object.get("評価DatasetID").and_then(Value::as_str).is_some_and(valid_identifier)
        && object.get("評価CaseID").and_then(Value::as_str).is_some_and(valid_identifier)
        && object.get("実行系ID").and_then(Value::as_str).is_some_and(実行系ID妥当)
        && object.get("判定").and_then(Value::as_str).is_some_and(|value| matches!(value, "成立" | "不成立" | "評価不能" | "中断"))
        && !evaluators.is_empty()
        && evaluators.len() <= MAX_EVALUATORS_PER_CASE
        && evaluators.iter().all(|item| {
            valid_evaluator_result(item)
                && item
                    .get("評価器ID")
                    .and_then(Value::as_str)
                    .is_some_and(|id| evaluator_ids.insert(id))
        })
        && optional_nonnegative_int(object.get("LatencyMillis"))
        && object.get("総合hash").and_then(Value::as_str).is_some_and(valid_hash)
        && object.get("評価監査ID").and_then(Value::as_str).is_some_and(valid_audit_id)
}

fn valid_evaluator_result(value: &Value) -> bool {
    let Ok(object) = exact_object(value, &["評価器ID", "種類", "判定", "理由code"]) else { return false; };
    object.get("評価器ID").and_then(Value::as_str).is_some_and(valid_identifier)
        && object.get("種類").and_then(Value::as_str).is_some_and(|value| matches!(value, "exact" | "contains" | "regex" | "json_schema" | "reference_count" | "route" | "status" | "capability" | "latency_threshold"))
        && object.get("判定").and_then(Value::as_str).is_some_and(|value| matches!(value, "成立" | "不成立" | "評価不能" | "中断"))
        && object.get("理由code").and_then(Value::as_str).is_some_and(|value| matches!(value, "一致" | "不一致" | "入力不足" | "応答不正" | "根拠不足" | "閾値超過" | "中止" | "監査失敗" | "評価器不正"))
}

fn valid_comparison(value: &Value) -> bool {
    let Ok(object) = exact_object(value, &["版", "比較ID", "評価DatasetID", "Dataset定義hash", "実験一覧", "計画Case数", "成立数", "不成立数", "評価不能数", "中断数", "比較時刻UnixMillis", "経路差", "参照差", "能力差", "比較監査ID", "証拠種別"]) else { return false; };
    let Some(case_count) = object
        .get("計画Case数")
        .and_then(Value::as_u64)
        .filter(|value| (1..=MAX_DATASETS_CASES as u64).contains(value))
    else {
        return false;
    };
    let Some(entries) = object.get("実験一覧").and_then(Value::as_array) else {
        return false;
    };
    let Some(dataset_definition_hash) = object
        .get("Dataset定義hash")
        .and_then(Value::as_str)
        .filter(|value| valid_hash(value))
    else {
        return false;
    };
    object.get("版").and_then(Value::as_u64) == Some(EVALUATION_VERSION)
        && object.get("比較ID").and_then(Value::as_str).is_some_and(valid_identifier)
        && object.get("評価DatasetID").and_then(Value::as_str).is_some_and(valid_identifier)
        && valid_comparison_entries(entries, case_count, dataset_definition_hash)
        && ["成立数", "不成立数", "評価不能数", "中断数"].iter().all(|field| object.get(*field).and_then(Value::as_u64).is_some_and(|value| value <= 1024))
        && comparison_totals_match(object, entries)
        && object.get("比較時刻UnixMillis").and_then(Value::as_i64).is_some_and(|value| value >= 0)
        && ["経路差", "参照差", "能力差"].iter().all(|field| object.get(*field).and_then(Value::as_str).is_some_and(|value| matches!(value, "same" | "different" | "unknown")))
        && object.get("比較監査ID").and_then(Value::as_str).is_some_and(valid_audit_id)
        && object.get("証拠種別").and_then(Value::as_str) == Some(EVIDENCE_SOURCE_INTERNAL_STATE)
}

const EVALUATION_COUNT_FIELDS: [&str; 4] = ["成立数", "不成立数", "評価不能数", "中断数"];

fn evaluation_count_total(value: &Value) -> Option<u64> {
    EVALUATION_COUNT_FIELDS.iter().try_fold(0u64, |total, field| {
        value.get(*field)?.as_u64()?.checked_add(total)
    })
}

fn comparison_totals_match(object: &Map<String, Value>, entries: &[Value]) -> bool {
    EVALUATION_COUNT_FIELDS.iter().all(|field| {
        let Some(root_count) = object.get(*field).and_then(Value::as_u64) else {
            return false;
        };
        let Some(entry_count) = entries.iter().try_fold(0u64, |total, entry| {
            entry.get(*field)?.as_u64()?.checked_add(total)
        }) else {
            return false;
        };
        root_count == entry_count
    })
}

fn valid_comparison_entries(
    values: &[Value],
    case_count: u64,
    dataset_definition_hash: &str,
) -> bool {
    if values.len() < 2 || values.len() > MAX_TARGET_RUNTIMES || !values.iter().all(valid_aggregate) {
        return false;
    }
    let Some(experiment_id) = values
        .first()
        .and_then(|value| value["評価ExperimentID"].as_str())
    else {
        return false;
    };
    let mut runtimes = BTreeSet::new();
    values.iter().all(|value| {
        value["評価ExperimentID"].as_str() == Some(experiment_id)
            && value["Dataset定義hash"].as_str() == Some(dataset_definition_hash)
            && value["実行系ID"]
                .as_str()
                .is_some_and(|runtime| runtimes.insert(runtime))
            && evaluation_count_total(value) == Some(case_count)
    })
}

fn valid_aggregate(value: &Value) -> bool {
    let Ok(object) = exact_object(value, &["評価ExperimentID", "実行系ID", "Dataset定義hash", "成立数", "不成立数", "評価不能数", "中断数", "平均LatencyMillis"]) else { return false; };
    object.get("評価ExperimentID").and_then(Value::as_str).is_some_and(valid_identifier)
        && object.get("実行系ID").and_then(Value::as_str).is_some_and(実行系ID妥当)
        && object.get("Dataset定義hash").and_then(Value::as_str).is_some_and(valid_hash)
        && ["成立数", "不成立数", "評価不能数", "中断数"].iter().all(|field| object.get(*field).and_then(Value::as_u64).is_some_and(|value| value <= 1024))
        && match object.get("平均LatencyMillis") {
            Some(Value::Null) => true,
            Some(Value::Number(number)) => number
                .as_f64()
                .is_some_and(|value| value >= 0.0 && value.is_finite()),
            _ => false,
        }
}

fn optional_nonnegative_int(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::Null)) || value.and_then(Value::as_i64).is_some_and(|value| value >= 0)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::broker::dialogue::{対話要求, 実行結果};
    use crate::broker::runtime_lifecycle::{
        LifecycleAdapter, LifecycleAdapterFailure, LifecycleAdapterResult, LifecycleState,
    };
    use std::io::{Read, Write};
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    struct EvaluationAdapter {
        calls: Arc<AtomicUsize>,
    }

    /// C4通常資格経路をC5の実行系と同じfixtureへ接続するためだけの試験Adapter。
    /// 対話Adapterの権限をlifecycle capabilityへ昇格させない。
    struct EvaluationLifecycleAdapter;

    impl LifecycleAdapter for EvaluationLifecycleAdapter {
        fn supported_actions(&self) -> &'static [LifecycleAction] {
            &[LifecycleAction::Quarantine]
        }

        fn transition(
            &self,
            action: LifecycleAction,
        ) -> Result<LifecycleAdapterResult, LifecycleAdapterFailure> {
            if action != LifecycleAction::Quarantine {
                return Err(LifecycleAdapterFailure::new("C4試験fixtureの操作が不正"));
            }
            Ok(LifecycleAdapterResult {
                next_state: LifecycleState::Quarantined,
                acknowledgement: "evaluation-lifecycle-quarantine",
            })
        }

        fn fail_closed(&self) {}
    }

    impl 実行系Adapter for EvaluationAdapter {
        fn 接続対象(&self) -> String {
            "evaluation-fixture".to_owned()
        }

        fn 応答(
            &self,
            request: &対話要求,
            _cancelled: &AtomicBool,
            _deadline: Instant,
            raw: &mut Vec<Vec<u8>>,
        ) -> Result<実行結果, 対話失敗> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            raw.push(b"evaluation-private-wire-receipt".to_vec());
            Ok(実行結果 {
                対話セッションID: request.対話セッションID.clone(),
                本文: "approved evaluation response".to_owned(),
                参照: vec!["fixture-reference".to_owned()],
                能力: vec!["fixture-capability".to_owned()],
                経路: "fixture.evaluation".to_owned(),
                追跡ID: "a".repeat(32),
                追跡hash: format!("sha256:{}", "b".repeat(64)),
                保留: false,
                生応答: b"evaluation-private-wire-receipt".to_vec(),
            })
        }
    }

    /// 取消flagを受けても明示releaseまで返らないAdapter。C5は外部実行の停止を
    /// 仮定せず、worker receiverが残る間は対話回収と比較を保留することを試験する。
    struct BlockingEvaluationAdapter {
        entered: Arc<AtomicBool>,
        release: Arc<AtomicBool>,
    }

    impl 実行系Adapter for BlockingEvaluationAdapter {
        fn 接続対象(&self) -> String {
            "evaluation-blocking-fixture".to_owned()
        }

        fn 応答(
            &self,
            request: &対話要求,
            _cancelled: &AtomicBool,
            _deadline: Instant,
            raw: &mut Vec<Vec<u8>>,
        ) -> Result<実行結果, 対話失敗> {
            self.entered.store(true, Ordering::SeqCst);
            while !self.release.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(2));
            }
            raw.push(b"evaluation-blocking-private-wire-receipt".to_vec());
            Ok(実行結果 {
                対話セッションID: request.対話セッションID.clone(),
                本文: "approved evaluation response".to_owned(),
                参照: vec!["fixture-reference".to_owned()],
                能力: vec!["fixture-capability".to_owned()],
                経路: "fixture.evaluation".to_owned(),
                追跡ID: "a".repeat(32),
                追跡hash: format!("sha256:{}", "b".repeat(64)),
                保留: false,
                生応答: b"evaluation-blocking-private-wire-receipt".to_vec(),
            })
        }
    }

    fn test_root(name: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "gui-shell-evaluation-{name}-{}-{suffix}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("試験用ルートを作成できる");
        root
    }

    fn read_test_file(path: &std::path::Path) -> Vec<u8> {
        let mut file = std::fs::File::open(path).expect("試験用ファイルを開ける");
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).expect("試験用ファイルを読める");
        bytes
    }

    fn overwrite_test_file(path: &std::path::Path, bytes: &[u8]) {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)
            .expect("上書きする試験用ファイルを開ける");
        file.write_all(bytes).expect("試験用ファイルを上書きできる");
        file.sync_all().expect("上書き後の試験用ファイルを同期できる");
    }

    fn test_broker(root: &std::path::Path) -> (Broker, Arc<AtomicUsize>) {
        let audit = root.join("audit");
        let vault = root.join("vault");
        std::fs::create_dir(&vault).expect("vault");
        let mut broker = Broker::new_persistent("evaluation-session", &audit).expect("永続化Brokerを作成できる");
        broker.current_epoch_seconds_override = Some(
            super::super::parse_issued_at_epoch_seconds("2026-06-01T00:00:30Z")
                .expect("epoch"),
        );
        broker
            .保管先起動登録(&vault, true, &[audit.clone()])
            .expect("保護保管先を登録できる");
        let calls = Arc::new(AtomicUsize::new(0));
        for runtime in ["fixture-a", "fixture-b"] {
            broker
                .実行系登録(
                    runtime,
                    Arc::new(EvaluationAdapter {
                        calls: Arc::clone(&calls),
                    }),
                )
                .expect("runtime");
        }
        (broker, calls)
    }

    fn blocking_test_broker(
        root: &std::path::Path,
        entered: Arc<AtomicBool>,
        release: Arc<AtomicBool>,
    ) -> Broker {
        let audit = root.join("audit");
        let vault = root.join("vault");
        std::fs::create_dir(&vault).expect("vault");
        let mut broker = Broker::new_persistent("evaluation-session", &audit)
            .expect("永続化Brokerを作成できる");
        broker.current_epoch_seconds_override = Some(
            super::super::parse_issued_at_epoch_seconds("2026-06-01T00:00:30Z")
                .expect("epoch"),
        );
        broker
            .保管先起動登録(&vault, true, &[audit.clone()])
            .expect("保護保管先を登録できる");
        broker
            .実行系登録(
                "fixture-a",
                Arc::new(BlockingEvaluationAdapter { entered, release }),
            )
            .expect("runtime");
        broker
    }

    fn call(
        broker: &mut Broker,
        counter: usize,
        operation: BrokerOperation,
        payload: Value,
        owner: bool,
    ) -> BrokerResponse {
        let mut request = BrokerRequestEnvelope::command_envelope(
            &format!("evaluation-request-{counter}"),
            "evaluation-session",
            &format!("evaluation-nonce-{counter}"),
        );
        request.operation = Some(operation);
        request.payload = Some(payload);
        request.refresh_payload_hash();
        broker.処理(request, owner)
    }

    fn c4_terminal_quarantine(
        broker: &mut Broker,
        counter: usize,
    ) -> Value {
        broker
            .ライフサイクル試験登録("fixture-a", Arc::new(EvaluationLifecycleAdapter))
            .expect("C4 lifecycle fixtureを登録できる");
        let pending = accepted(call(
            broker,
            counter,
            BrokerOperation::実行系ライフサイクル承認要求,
            json!({"版": 1, "実行系ID": "fixture-a", "操作": "quarantine"}),
            false,
        ));
        let approved = accepted(call(
            broker,
            counter + 1,
            BrokerOperation::実行系ライフサイクル承認,
            json!({
                "版": 1,
                "承認ID": pending["承認ID"],
                "承認hash": pending["承認hash"],
            }),
            true,
        ));
        accepted(call(
            broker,
            counter + 2,
            BrokerOperation::実行系ライフサイクル操作,
            json!({
                "版": 1,
                "実行系ID": "fixture-a",
                "操作": "quarantine",
                "承認ID": approved["承認ID"],
            }),
            false,
        ))
    }

    fn accepted(response: BrokerResponse) -> Value {
        assert_eq!(response.status, BrokerStatus::Accepted, "{:?}", response.error);
        response.body.expect("受理応答の本文がある")
    }

    fn private_dataset(input: &str, expected: &str) -> Value {
        json!({
            "版": 1,
            "評価DatasetID": "d".repeat(32),
            "revision": 1,
            "公開表示名": "C5 fixture",
            "登録者種別": "owner",
            "登録経路": "owner_control",
            "評価用途": "運用観測",
            "評価方式": "決定論",
            "非公開CasePayload一覧": [{
                "評価CaseID": "c".repeat(32),
                "順序": 1,
                "公開表示名": "fixture case",
                "入力": {"内容表示範囲": "full", "本文": input},
                "評価器設定": [
                    {"評価器ID": "e".repeat(32), "種類": "exact", "設定": {"期待本文": expected}},
                    {"評価器ID": "f".repeat(32), "種類": "status", "設定": {"期待状態": "成功"}}
                ]
            }]
        })
    }

    fn wait_for_entered(entered: &AtomicBool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !entered.load(Ordering::SeqCst) {
            assert!(Instant::now() < deadline, "評価workerが開始しない");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn dataset_stays_private_and_each_runtime_waits_for_individual_owner_dialogue_approval() {
        let root = test_root("approval");
        let vault = root.join("vault");
        let private_input = "private-evaluation-input-must-not-appear";
        let private_expected = "approved evaluation response";
        let (mut broker, calls) = test_broker(&root);
        let dataset = private_dataset(private_input, private_expected);

        assert_eq!(
            call(
                &mut broker,
                1,
                BrokerOperation::評価Dataset登録,
                dataset.clone(),
                false,
            )
            .error
            .expect("通常経路の登録が拒否される")
            .code,
            "権限拒否"
        );
        let manifest = accepted(call(
            &mut broker,
            2,
            BrokerOperation::評価Dataset登録,
            dataset,
            true,
        ));
        assert_eq!(manifest["公開範囲"], "hash_only");
        assert!(!manifest.to_string().contains(private_input));
        assert!(!manifest.to_string().contains(private_expected));
        let storage_id = manifest["非公開保管ID"].as_str().expect("非公開保管IDがある");
        let ciphertext = read_test_file(&vault.join(format!("evaluation-{storage_id}.dpapi")));
        assert!(!String::from_utf8_lossy(&ciphertext).contains(private_input));
        assert!(!String::from_utf8_lossy(&ciphertext).contains(private_expected));

        assert_eq!(
            call(
                &mut broker,
                3,
                BrokerOperation::評価Dataset一覧,
                json!({"版": 1}),
                true,
            )
            .error
            .expect("ownerの通常操作が拒否される")
            .code,
            "evaluation_normal_channel_required"
        );
        let started = accepted(call(
            &mut broker,
            4,
            BrokerOperation::評価実験開始,
            json!({
                "版": 1,
                "評価DatasetID": manifest["評価DatasetID"],
                "対象Runtime一覧": ["fixture-a", "fixture-b"],
            }),
            false,
        ));
        let experiment_id = started["評価ExperimentID"].as_str().expect("評価実験IDがある").to_owned();
        assert_eq!(started["状態"], "承認待ち");
        assert_eq!(calls.load(Ordering::SeqCst), 0, "C5開始はowner承認前にAdapterを呼ばない");
        let incomplete_comparison = call(
            &mut broker,
            6,
            BrokerOperation::評価比較,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        );
        assert_eq!(incomplete_comparison.status, BrokerStatus::Rejected);
        assert_eq!(
            incomplete_comparison
                .error
                .expect("未完了Experimentの比較が拒否される")
                .code,
            "評価比較不能"
        );
        assert!(
            !broker
                .audit_events()
                .iter()
                .any(|event| event.operation == "評価比較" && event.decision == "accepted"),
            "未完了Experimentを比較記録として確定してはならない"
        );

        let waiting = accepted(call(
            &mut broker,
            5,
            BrokerOperation::対話承認待ち,
            json!({}),
            true,
        ));
        let pending = waiting["要求"].as_array().expect("承認待ち要求一覧がある");
        assert_eq!(pending.len(), 2, "CaseごとRuntimeごとの独立要求");
        let pending_text = waiting.to_string();
        for hidden in [private_input, private_expected, "evaluation-private-wire-receipt"] {
            assert!(
                !pending_text.contains(hidden),
                "C5隔離対話のowner承認待ち一覧へprivate本文を再投影しない: {hidden}"
            );
        }
        for item in pending {
            let request = item["要求"].as_object().expect("隔離要求projectionがobject");
            assert_eq!(request.len(), 2);
            assert!(request.contains_key("要求ID"));
            assert!(request.contains_key("実行系ID"));
            assert!(item["評価隔離"] == true);
            assert!(item.get("接続先").is_none());
        }
        // C5評価のfull承認はownerの個別Approvalを維持するが、通常の対話取得へ
        // private応答を戻してはならない。実行前の隔離要求について通常actorとownerの
        // 両方で拒否されることを確認する。
        for (index, item) in pending.iter().enumerate() {
            let request_id = item["要求"]["要求ID"]
                .as_str()
                .expect("承認待ち要求IDがある");
            for (owner, offset) in [(false, 0usize), (true, 1usize)] {
                let denied = call(
                    &mut broker,
                    30 + index * 2 + offset,
                    BrokerOperation::対話取得,
                    json!({"要求ID": request_id}),
                    owner,
                );
                assert_eq!(denied.status, BrokerStatus::Rejected);
                assert_eq!(denied.error.expect("通常対話取得が拒否される").code, "権限拒否");
            }
        }
        let first = pending.first().expect("最初の評価対話がある");
        let first_approval = accepted(call(
            &mut broker,
            10,
            BrokerOperation::対話承認,
            json!({
                "要求ID": first["要求"]["要求ID"],
                "要求hash": first["要求hash"],
                "表示範囲": "full",
            }),
            true,
        ));
        assert_eq!(first_approval["状態"], "実行中");
        let mut first_result_status = Value::Null;
        for index in 0..200 {
            first_result_status = accepted(call(
                &mut broker,
                1_000 + index,
                BrokerOperation::評価実験状態,
                json!({"版": 1, "評価ExperimentID": experiment_id}),
                false,
            ));
            if first_result_status["評価実験"]["結果数"] == 1 {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(first_result_status["評価実験"]["結果数"], 1);
        broker.current_epoch_seconds_override = Some(
            super::super::parse_issued_at_epoch_seconds("2026-06-01T00:00:45Z")
                .expect("2件目Result用epoch"),
        );
        for (index, item) in pending.iter().enumerate().skip(1) {
            let request = &item["要求"];
            let approval = accepted(call(
                &mut broker,
                10 + index,
                BrokerOperation::対話承認,
                json!({
                    "要求ID": request["要求ID"],
                    "要求hash": item["要求hash"],
                    "表示範囲": "full",
                }),
                true,
            ));
            assert_eq!(approval["状態"], "実行中");
        }
        let mut status = Value::Null;
        for index in 0..200 {
            status = accepted(call(
                &mut broker,
                100 + index,
                BrokerOperation::評価実験状態,
                json!({"版": 1, "評価ExperimentID": experiment_id}),
                false,
            ));
            if status["評価実験"]["結果数"] == 2 {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(status["評価実験"]["状態"], "完了");
        assert_eq!(status["公開結果一覧"].as_array().map_or(0, Vec::len), 2);
        let public = status.to_string();
        for hidden in [private_input, private_expected, "evaluation-private-wire-receipt", "対話要求ID", "対話セッションID", "応答hash"] {
            assert!(!public.contains(hidden), "公開状態に非公開値を含めない: {hidden}");
        }
        let comparison = accepted(call(
            &mut broker,
            400,
            BrokerOperation::評価比較,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        ));
        assert_eq!(comparison["経路差"], "same");
        assert_eq!(comparison["参照差"], "same");
        assert_eq!(comparison["能力差"], "same");
        let audit = serde_json::to_string(broker.audit_events()).expect("監査JSONを直列化できる");
        for hidden in [private_input, private_expected, "evaluation-private-wire-receipt"] {
            assert!(!audit.contains(hidden), "監査にprivate本文を含めない: {hidden}");
        }
        let active = broker
            .評価
            .active
            .get(&experiment_id)
            .expect("稼働中の評価実験がある");
        let expected_started_at = active
            .plans
            .iter()
            .filter_map(|plan| plan.result.as_ref())
            .filter_map(|result| result["実行時刻UnixMillis"].as_i64())
            .min()
            .expect("復元前にResult実行時刻がある");
        let expected_finished_at = active
            .plans
            .iter()
            .filter_map(|plan| plan.result.as_ref())
            .filter_map(|result| result["実行時刻UnixMillis"].as_i64())
            .max()
            .expect("復元前にResult実行時刻がある");
        assert!(
            expected_started_at < expected_finished_at,
            "復元時刻の最小値と最大値を別々に検証できる"
        );
        let plan = &active.plans[0];
        let persisted_plan = PersistedEvaluationPlan {
            case_id: plan.case_id.clone(),
            runtime_id: plan.runtime_id.clone(),
            request_id: plan.request_id.clone(),
            request_hash: plan.request_hash.clone(),
            session_id: plan.session_id.clone(),
            session_start_request_audit_id: plan.session_start_request_audit_id.clone(),
            session_start_audit_id: plan.session_start_audit_id.clone(),
            send_request_audit_id: plan.send_request_audit_id.clone(),
            send_payload_hash: plan.send_payload_hash.clone(),
            created_audit_id: plan.created_audit_id.clone(),
            isolation_request_audit_id: plan.isolation_request_audit_id.clone(),
            isolation_audit_id: plan.isolation_audit_id.clone(),
        };
        let (plan_event_index, _) = audit_event_with_index(
            &broker.audit_log,
            active.public["計画監査ID"]
                .as_str()
                .expect("計画監査IDがある"),
        )
        .expect("計画監査eventがある");
        let (experiment_event_index, experiment_event) = audit_event_with_index(
            &broker.audit_log,
            active.public["実験監査ID"]
                .as_str()
                .expect("実験監査IDがある"),
        )
        .expect("実験監査eventがある");
        assert!(validate_plan_audit_correlations(
            &broker.audit_log,
            &persisted_plan,
            plan_event_index,
            experiment_event_index,
            &experiment_event.request_id,
        )
        .is_ok());
        let mut mismatched_send_payload = persisted_plan.clone();
        mismatched_send_payload.send_payload_hash = format!("sha256:{}", "0".repeat(64));
        assert!(validate_plan_audit_correlations(
            &broker.audit_log,
            &mismatched_send_payload,
            plan_event_index,
            experiment_event_index,
            &experiment_event.request_id,
        )
        .is_err());
        let recorded = plan.result.as_ref().expect("記録済み結果がある");
        for (field, replacement) in [
            ("対話要求ID", "1".repeat(32)),
            ("対話セッションID", "2".repeat(32)),
            ("評価CaseID", "3".repeat(32)),
            ("作成監査ID", "broker-audit-999".to_owned()),
        ] {
            let mut forged = recorded.clone();
            forged[field] = json!(replacement);
            assert!(
                result_matches_plan(&broker.audit_log, &forged, &persisted_plan).is_err(),
                "計画と異なる{field}を復元結果として採用してはならない"
            );
        }
        drop(broker);
        let mut recovered = Broker::new_persistent("evaluation-session", &root.join("audit"))
            .expect("永続化Brokerを再起動できる");
        recovered.current_epoch_seconds_override = Some(
            super::super::parse_issued_at_epoch_seconds("2026-06-01T00:02:00Z")
                .expect("epoch"),
        );
        let restored = accepted(call(
            &mut recovered,
            500,
            BrokerOperation::評価実験状態,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        ));
        assert_eq!(restored["評価実験"]["状態"], "完了");
        assert_eq!(restored["公開結果一覧"].as_array().map_or(0, Vec::len), 2);
        assert_eq!(
            restored["評価実験"]["開始時刻UnixMillis"].as_i64(),
            Some(expected_started_at),
            "復元時刻ではなく監査済みResultの最初の実行時刻を使う"
        );
        assert_eq!(
            restored["評価実験"]["終了時刻UnixMillis"].as_i64(),
            Some(expected_finished_at),
            "復元時刻ではなく監査済みResultの最後の実行時刻を使う"
        );
        assert!(valid_experiment(&restored["評価実験"]));
        let restored_comparison = accepted(call(
            &mut recovered,
            501,
            BrokerOperation::評価比較,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        ));
        assert_eq!(restored_comparison["成立数"], 2);
        drop(recovered);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn owner承認待ちで観測した評価対話期限超過を復元する() {
        let root = test_root("timeout-observed-by-owner-pending");
        let (mut broker, _) = test_broker(&root);
        let manifest = accepted(call(
            &mut broker,
            1,
            BrokerOperation::評価Dataset登録,
            private_dataset("private-timeout-input", "approved evaluation response"),
            true,
        ));
        let started = accepted(call(
            &mut broker,
            2,
            BrokerOperation::評価実験開始,
            json!({
                "版": 1,
                "評価DatasetID": manifest["評価DatasetID"],
                "対象Runtime一覧": ["fixture-a"],
            }),
            false,
        ));
        let experiment_id = started["評価ExperimentID"]
            .as_str()
            .expect("評価ExperimentIDがある")
            .to_owned();
        let timeout_epoch = super::super::parse_issued_at_epoch_seconds("2026-06-01T00:06:00Z")
            .expect("期限後epoch");
        broker.current_epoch_seconds_override = Some(timeout_epoch);

        let mut pending_request = BrokerRequestEnvelope::command_envelope_at(
            "evaluation-request-3",
            "evaluation-session",
            "evaluation-nonce-3",
            "2026-06-01T00:06:00Z",
        );
        pending_request.operation = Some(BrokerOperation::対話承認待ち);
        pending_request.payload = Some(json!({}));
        pending_request.refresh_payload_hash();
        let pending = accepted(broker.処理(pending_request, true));
        assert_eq!(pending["要求"].as_array().map_or(usize::MAX, Vec::len), 0);
        assert!(broker.audit_events().iter().any(|event| {
            event.operation == "対話承認待ち"
                && event.decision == "recorded"
                // 承認待ち期限の既存対話制御は、失敗分類を実行記録へ保持したうえで
                // 終端reasonを`対話完了`として記録する。
                && event.reason == "対話完了"
        }));
        broker.current_epoch_seconds_override = Some(
            super::super::parse_issued_at_epoch_seconds("2026-06-01T00:02:00Z")
                .expect("復元用epoch"),
        );

        let local_status = accepted(call(
            &mut broker,
            4,
            BrokerOperation::評価実験状態,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        ));
        assert_eq!(local_status["評価実験"]["結果数"], 1);
        drop(broker);

        let mut recovered = Broker::new_persistent("evaluation-session", &root.join("audit"))
            .expect("期限超過後のBrokerを再起動できる");
        recovered.current_epoch_seconds_override = Some(
            super::super::parse_issued_at_epoch_seconds("2026-06-01T00:02:00Z")
                .expect("復元要求用epoch"),
        );
        let recovered_status = accepted(call(
            &mut recovered,
            5,
            BrokerOperation::評価実験状態,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        ));
        assert_eq!(recovered_status["評価実験"]["結果数"], 1);
        assert_eq!(
            recovered_status["公開結果一覧"].as_array().map_or(0, Vec::len),
            1
        );
        drop(recovered);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn 既存対話完了が評価session開始監査の途中にあっても復元する() {
        let root = test_root("interleaved-normal-dialogue-completion");
        let (mut broker, _) = test_broker(&root);
        let manifest = accepted(call(
            &mut broker,
            1,
            BrokerOperation::評価Dataset登録,
            private_dataset("interleaved-private-input", "approved evaluation response"),
            true,
        ));

        let normal_session = accepted(call(
            &mut broker,
            2,
            BrokerOperation::対話開始,
            json!({"実行系ID": "fixture-b"}),
            false,
        ))["対話セッションID"]
            .as_str()
            .expect("通常対話Session IDがある")
            .to_owned();
        let normal_pending = accepted(call(
            &mut broker,
            3,
            BrokerOperation::対話送信,
            json!({
                "対話セッションID": normal_session,
                "入力": "normal-dialogue-completion-before-evaluation-session",
            }),
            false,
        ));
        let normal_request_id = normal_pending["要求ID"]
            .as_str()
            .expect("通常対話要求IDがある")
            .to_owned();
        accepted(call(
            &mut broker,
            4,
            BrokerOperation::対話承認,
            json!({
                "要求ID": normal_request_id.clone(),
                "要求hash": normal_pending["要求hash"],
                "表示範囲": "full",
            }),
            true,
        ));
        // workerが送信済みの同じ受信結果を、通常の`対話取得`を呼ばずに進捗前へ
        // 戻す。これによりC5の`対話開始`が既存対話の終端を確実に観測する。
        broker
            .対話
            .試験用受信済みを進捗前に再投入(
                &normal_request_id,
                Instant::now() + Duration::from_secs(2),
            )
            .expect("通常対話の受信結果が到着する");

        let started = accepted(call(
            &mut broker,
            5,
            BrokerOperation::評価実験開始,
            json!({
                "版": 1,
                "評価DatasetID": manifest["評価DatasetID"],
                "対象Runtime一覧": ["fixture-a"],
            }),
            false,
        ));
        let experiment_id = started["評価ExperimentID"]
            .as_str()
            .expect("評価ExperimentIDがある")
            .to_owned();
        let plan = broker
            .評価
            .active
            .get(&experiment_id)
            .expect("評価Experimentが稼働中")
            .plans
            .first()
            .expect("評価計画がある");
        let (session_request_index, _) = audit_event_with_index(
            &broker.audit_log,
            &plan.session_start_request_audit_id,
        )
        .expect("C5 Session開始要求監査がある");
        let (session_start_index, _) =
            audit_event_with_index(&broker.audit_log, &plan.session_start_audit_id)
                .expect("C5 Session開始監査がある");
        let normal_completion_indices = broker
            .audit_events()
            .iter()
            .enumerate()
            .filter(|(_, event)| {
                event.operation == "対話開始"
                    && event.decision == "recorded"
                    && event.request_id == normal_request_id
                    && event.reason == "対話完了"
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        assert_eq!(normal_completion_indices.len(), 1, "通常対話終端監査が一意にある");
        assert!(
            session_request_index < normal_completion_indices[0]
                && normal_completion_indices[0] < session_start_index,
            "既存対話終端監査はC5のSession開始要求とSession開始の間に挿入される"
        );

        drop(broker);
        let mut recovered = Broker::new_persistent("evaluation-session", &root.join("audit"))
            .expect("監査からBrokerを再起動できる");
        recovered.current_epoch_seconds_override = Some(
            super::super::parse_issued_at_epoch_seconds("2026-06-01T00:02:00Z")
                .expect("epoch"),
        );
        let restored = accepted(call(
            &mut recovered,
            6,
            BrokerOperation::評価実験状態,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        ));
        assert_eq!(restored["評価実験"]["状態"], "中断");
        assert_eq!(restored["公開結果一覧"].as_array().map_or(usize::MAX, Vec::len), 0);
        drop(recovered);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn cancelled_dialogue_is_recorded_as_interruption_not_evaluator_success() {
        let root = test_root("cancelled-dialogue");
        let (mut broker, calls) = test_broker(&root);
        let manifest = accepted(call(
            &mut broker,
            1,
            BrokerOperation::評価Dataset登録,
            private_dataset("private-cancel-input", "approved evaluation response"),
            true,
        ));
        let started = accepted(call(
            &mut broker,
            2,
            BrokerOperation::評価実験開始,
            json!({
                "版": 1,
                "評価DatasetID": manifest["評価DatasetID"],
                "対象Runtime一覧": ["fixture-a"],
            }),
            false,
        ));
        let experiment_id = started["評価ExperimentID"]
            .as_str()
            .expect("評価実験IDがある")
            .to_owned();
        let pending = accepted(call(
            &mut broker,
            3,
            BrokerOperation::対話承認待ち,
            json!({}),
            true,
        ));
        let request_id = pending["要求"][0]["要求"]["要求ID"]
            .as_str()
            .expect("承認待ち要求IDがある")
            .to_owned();
        let cancelled = accepted(call(
            &mut broker,
            4,
            BrokerOperation::対話中止,
            json!({"要求ID": request_id}),
            false,
        ));
        assert_eq!(cancelled["状態"], "中止");
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        let status = accepted(call(
            &mut broker,
            5,
            BrokerOperation::評価実験状態,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        ));
        assert_eq!(status["評価実験"]["状態"], "中断");
        let experiment = broker
            .評価
            .active
            .get(&experiment_id)
            .expect("稼働中の評価実験がある");
        let result = experiment.plans[0].result.as_ref().expect("記録済み結果がある");
        assert_eq!(result["結果状態"], "中止");
        assert_eq!(result["判定"], "中断");
        assert!(result["評価器判定一覧"]
            .as_array()
            .expect("評価器判定一覧がある")
            .iter()
            .all(|value| value["判定"] == "中断" && value["理由code"] == "中止"));
        let aggregates = aggregate_results(&experiment.public, experiment.plans.iter().filter_map(|plan| plan.result.as_ref()))
            .expect("中断結果を集計できる");
        assert_eq!(aggregates[0]["中断数"], 1);
        assert_eq!(aggregates[0]["評価不能数"], 0);
        let plan = persisted_plan(&experiment.plans[0]);
        let (plan_event_index, _) = audit_event_with_index(
            &broker.audit_log,
            experiment.public["計画監査ID"]
                .as_str()
                .expect("計画監査IDがある"),
        )
        .expect("計画監査eventがある");
        let (experiment_event_index, experiment_event) = audit_event_with_index(
            &broker.audit_log,
            experiment.public["実験監査ID"]
                .as_str()
                .expect("実験監査IDがある"),
        )
        .expect("実験監査eventがある");
        let (result_event_index, _) = audit_event_with_index(
            &broker.audit_log,
            result["評価監査ID"].as_str().expect("評価監査IDがある"),
        )
        .expect("評価結果監査eventがある");
        let mut missing_terminal = result.clone();
        missing_terminal["終了監査ID"] = Value::Null;
        assert!(validate_recovered_result_causality(
            &broker.audit_log,
            &missing_terminal,
            &plan,
            plan_event_index,
            experiment_event_index,
            &experiment_event.request_id,
            result_event_index,
        )
        .is_err());
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn response_hashがないowner承認は評価不能として回収される() {
        let root = test_root("no-response-hash");
        let (mut broker, _) = test_broker(&root);
        let manifest = accepted(call(
            &mut broker,
            1,
            BrokerOperation::評価Dataset登録,
            private_dataset("private-none-scope-input", "approved evaluation response"),
            true,
        ));
        let started = accepted(call(
            &mut broker,
            2,
            BrokerOperation::評価実験開始,
            json!({
                "版": 1,
                "評価DatasetID": manifest["評価DatasetID"],
                "対象Runtime一覧": ["fixture-a"],
            }),
            false,
        ));
        let experiment_id = started["評価ExperimentID"]
            .as_str()
            .expect("評価ExperimentIDがある")
            .to_owned();
        let pending = accepted(call(
            &mut broker,
            3,
            BrokerOperation::対話承認待ち,
            json!({}),
            true,
        ));
        let request = &pending["要求"][0];
        accepted(call(
            &mut broker,
            4,
            BrokerOperation::対話承認,
            json!({
                "要求ID": request["要求"]["要求ID"],
                "要求hash": request["要求hash"],
                "表示範囲": "none",
            }),
            true,
        ));
        let mut final_status = None;
        for counter in 5..100 {
            let status = accepted(call(
                &mut broker,
                counter,
                BrokerOperation::評価実験状態,
                json!({"版": 1, "評価ExperimentID": experiment_id}),
                false,
            ));
            if status["評価実験"]["状態"] != "実行中" {
                final_status = Some(status);
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let status = final_status.expect("応答hashなし承認が評価不能として終端する");
        assert_eq!(status["評価実験"]["状態"], "評価不能");
        assert_eq!(status["公開結果一覧"][0]["判定"], "評価不能");
        assert!(status["公開結果一覧"][0].get("結果状態").is_none());
        assert!(status["公開結果一覧"][0].get("応答hash").is_none());
        assert_eq!(broker.対話.評価要求可能数(), 64);
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn 別owner対話操作で観測した実行中期限超過を終端として検証する() {
        let plan = PersistedEvaluationPlan {
            case_id: "c".repeat(32),
            runtime_id: "fixture-a".to_owned(),
            request_id: "r".repeat(32),
            request_hash: format!("sha256:{}", "a".repeat(64)),
            session_id: "s".repeat(32),
            session_start_request_audit_id: "audit.session-start-request".to_owned(),
            session_start_audit_id: "audit.session-start".to_owned(),
            send_request_audit_id: "audit.send-request".to_owned(),
            send_payload_hash: format!("sha256:{}", "b".repeat(64)),
            created_audit_id: "audit.create".to_owned(),
            isolation_request_audit_id: "audit.isolation-request".to_owned(),
            isolation_audit_id: "audit.isolation".to_owned(),
        };
        let mut log = BrokerAuditLog::default();
        let observed_by_owner_approval = log.append(
            &plan.request_id,
            "対話承認",
            "recorded",
            "対話期限超過",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &plan.request_hash,
        );
        assert!(terminal_event_matches(&observed_by_owner_approval, &plan));

        let unknown_operation = log.append(
            &plan.request_id,
            "任意操作",
            "recorded",
            "対話期限超過",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &plan.request_hash,
        );
        assert!(!terminal_event_matches(&unknown_operation, &plan));
    }

    #[test]
    fn recovery_proof_rejects_extra_private_field() {
        let plan = PersistedEvaluationPlan {
            case_id: "c".repeat(32),
            runtime_id: "fixture-a".to_owned(),
            request_id: "r".repeat(32),
            request_hash: format!("sha256:{}", "a".repeat(64)),
            session_id: "s".repeat(32),
            session_start_request_audit_id: "audit.session-start-request".to_owned(),
            session_start_audit_id: "audit.session-start".to_owned(),
            send_request_audit_id: "audit.send-request".to_owned(),
            send_payload_hash: format!("sha256:{}", "f".repeat(64)),
            created_audit_id: "audit.create".to_owned(),
            isolation_request_audit_id: "audit.isolation-request".to_owned(),
            isolation_audit_id: "audit.isolation".to_owned(),
        };
        let response_hash = format!("sha256:{}", "b".repeat(64));
        let mut log = BrokerAuditLog::default();
        let end = log.append(
            &plan.request_id,
            "対話取得",
            "recorded",
            "対話完了",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &plan.request_hash,
        );
        let proof = json!({
            "版": 1,
            "要求ID": plan.request_id,
            "対話セッションID": plan.session_id,
            "実行系ID": plan.runtime_id,
            "要求hash": plan.request_hash,
            "終了監査ID": end.event_id,
            "表示範囲": "full",
            "応答hash": response_hash,
            "能力申告hash": format!("sha256:{}", "c".repeat(64)),
            "経路申告hash": format!("sha256:{}", "d".repeat(64)),
            "追跡参照hash": format!("sha256:{}", "e".repeat(64)),
            "本文": "private-proof-field-must-be-rejected",
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        });
        let encoded = proof.to_string();
        log.append(
            &plan.request_id,
            "対話取得",
            "recorded",
            &format!("対話結果証跡:{encoded}"),
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &sha256_tagged(encoded.as_bytes()),
        );
        let result = json!({
            "終了監査ID": end.event_id,
            "応答hash": response_hash,
        });
        assert!(validate_result_proof(
            &log,
            &plan,
            &result,
            result["応答hash"].as_str().expect("hash"),
            0,
            2,
            "full",
        )
        .is_err());
    }

    #[test]
    fn cancelled_inflight_worker_delays_cleanup_and_comparison_until_receiver_is_reclaimed() {
        let root = test_root("cancelled-worker-cleanup");
        let entered = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let mut broker = blocking_test_broker(&root, Arc::clone(&entered), Arc::clone(&release));
        let manifest = accepted(call(
            &mut broker,
            1,
            BrokerOperation::評価Dataset登録,
            private_dataset("private-cancel-input", "approved evaluation response"),
            true,
        ));
        let started = accepted(call(
            &mut broker,
            2,
            BrokerOperation::評価実験開始,
            json!({
                "版": 1,
                "評価DatasetID": manifest["評価DatasetID"],
                "対象Runtime一覧": ["fixture-a"],
            }),
            false,
        ));
        let experiment_id = started["評価ExperimentID"]
            .as_str()
            .expect("評価ExperimentIDがある")
            .to_owned();
        let pending = accepted(call(
            &mut broker,
            3,
            BrokerOperation::対話承認待ち,
            json!({}),
            true,
        ));
        let request = &pending["要求"][0];
        let request_id = request["要求"]["要求ID"]
            .as_str()
            .expect("対話要求IDがある")
            .to_owned();
        accepted(call(
            &mut broker,
            4,
            BrokerOperation::対話承認,
            json!({
                "要求ID": request_id,
                "要求hash": request["要求hash"],
                "表示範囲": "full",
            }),
            true,
        ));
        wait_for_entered(&entered);
        accepted(call(
            &mut broker,
            5,
            BrokerOperation::対話中止,
            json!({"要求ID": request_id}),
            false,
        ));
        let waiting_cleanup = accepted(call(
            &mut broker,
            6,
            BrokerOperation::評価実験状態,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        ));
        let comparison = call(
            &mut broker,
            7,
            BrokerOperation::評価比較,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        );
        let capacity_while_worker_lives = broker.対話.評価要求可能数();
        release.store(true, Ordering::SeqCst);
        assert_eq!(waiting_cleanup["評価実験"]["状態"], "実行中");
        assert_eq!(capacity_while_worker_lives, 63);
        assert_eq!(comparison.status, BrokerStatus::Rejected);
        assert_eq!(comparison.error.expect("比較拒否").code, "評価比較不能");

        let mut final_status = None;
        for counter in 8..120 {
            let status = accepted(call(
                &mut broker,
                counter,
                BrokerOperation::評価実験状態,
                json!({"版": 1, "評価ExperimentID": experiment_id}),
                false,
            ));
            if status["評価実験"]["状態"] == "中断"
                && broker.対話.評価要求可能数() == 64
            {
                final_status = Some(status);
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(final_status.is_some(), "worker回収後に対話容量が戻る");
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn recovered_result_without_dialogue_cleanup_audit_never_becomes_comparable() {
        let root = test_root("recovered-missing-cleanup");
        let entered = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let mut broker = blocking_test_broker(&root, Arc::clone(&entered), Arc::clone(&release));
        let manifest = accepted(call(
            &mut broker,
            1,
            BrokerOperation::評価Dataset登録,
            private_dataset("private-recover-input", "approved evaluation response"),
            true,
        ));
        let started = accepted(call(
            &mut broker,
            2,
            BrokerOperation::評価実験開始,
            json!({
                "版": 1,
                "評価DatasetID": manifest["評価DatasetID"],
                "対象Runtime一覧": ["fixture-a"],
            }),
            false,
        ));
        let experiment_id = started["評価ExperimentID"]
            .as_str()
            .expect("評価ExperimentIDがある")
            .to_owned();
        let pending = accepted(call(
            &mut broker,
            3,
            BrokerOperation::対話承認待ち,
            json!({}),
            true,
        ));
        let request = &pending["要求"][0];
        let request_id = request["要求"]["要求ID"]
            .as_str()
            .expect("対話要求IDがある")
            .to_owned();
        accepted(call(
            &mut broker,
            4,
            BrokerOperation::対話承認,
            json!({
                "要求ID": request_id,
                "要求hash": request["要求hash"],
                "表示範囲": "full",
            }),
            true,
        ));
        wait_for_entered(&entered);
        accepted(call(
            &mut broker,
            5,
            BrokerOperation::対話中止,
            json!({"要求ID": request_id}),
            false,
        ));
        let pending_cleanup = accepted(call(
            &mut broker,
            6,
            BrokerOperation::評価実験状態,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        ));
        assert_eq!(pending_cleanup["評価実験"]["状態"], "実行中");
        drop(broker);

        let mut recovered = Broker::new_persistent("evaluation-session", &root.join("audit"))
            .expect("永続化Brokerを再起動できる");
        recovered.current_epoch_seconds_override = Some(
            super::super::parse_issued_at_epoch_seconds("2026-06-01T00:02:00Z")
                .expect("epoch"),
        );
        let restored = accepted(call(
            &mut recovered,
            7,
            BrokerOperation::評価実験状態,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        ));
        assert_eq!(restored["評価実験"]["状態"], "中断");
        let comparison = call(
            &mut recovered,
            8,
            BrokerOperation::評価比較,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        );
        release.store(true, Ordering::SeqCst);
        assert_eq!(comparison.status, BrokerStatus::Rejected);
        assert!(comparison
            .error
            .expect("比較拒否")
            .message
            .contains("通常対話終了監査"));
        std::thread::sleep(Duration::from_millis(10));
        drop(recovered);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn terminal_quarantine_waits_for_blocked_worker_and_never_confirms_comparison_cleanup() {
        let root = test_root("terminal-blocked-worker");
        let entered = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let mut broker = blocking_test_broker(&root, Arc::clone(&entered), Arc::clone(&release));
        let manifest = accepted(call(
            &mut broker,
            1,
            BrokerOperation::評価Dataset登録,
            private_dataset("private-terminal-input", "approved evaluation response"),
            true,
        ));
        let started = accepted(call(
            &mut broker,
            2,
            BrokerOperation::評価実験開始,
            json!({
                "版": 1,
                "評価DatasetID": manifest["評価DatasetID"],
                "対象Runtime一覧": ["fixture-a"],
            }),
            false,
        ));
        let experiment_id = started["評価ExperimentID"]
            .as_str()
            .expect("評価ExperimentIDがある")
            .to_owned();
        let pending = accepted(call(
            &mut broker,
            3,
            BrokerOperation::対話承認待ち,
            json!({}),
            true,
        ));
        let request = &pending["要求"][0];
        accepted(call(
            &mut broker,
            4,
            BrokerOperation::対話承認,
            json!({
                "要求ID": request["要求"]["要求ID"],
                "要求hash": request["要求hash"],
                "表示範囲": "full",
            }),
            true,
        ));
        wait_for_entered(&entered);
        let c4 = c4_terminal_quarantine(&mut broker, 1_000);
        assert_eq!(c4["遷移後状態"], "quarantined");
        let waiting_cleanup = accepted(call(
            &mut broker,
            5,
            BrokerOperation::評価実験状態,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        ));
        let comparison_while_worker_lives = call(
            &mut broker,
            6,
            BrokerOperation::評価比較,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        );
        let capacity_while_worker_lives = broker.対話.評価要求可能数();
        release.store(true, Ordering::SeqCst);
        assert_eq!(waiting_cleanup["評価実験"]["状態"], "実行中");
        assert_eq!(capacity_while_worker_lives, 63);
        assert_eq!(comparison_while_worker_lives.status, BrokerStatus::Rejected);

        let mut final_status = None;
        for counter in 7..120 {
            let status = accepted(call(
                &mut broker,
                counter,
                BrokerOperation::評価実験状態,
                json!({"版": 1, "評価ExperimentID": experiment_id}),
                false,
            ));
            if status["評価実験"]["状態"] == "中断"
                && broker.対話.評価要求可能数() == 64
            {
                final_status = Some(status);
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(final_status.is_some(), "terminal隔離のworker受信側が回収される");
        let comparison_after_recovery = call(
            &mut broker,
            121,
            BrokerOperation::評価比較,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        );
        assert_eq!(comparison_after_recovery.status, BrokerStatus::Rejected);
        assert_eq!(
            comparison_after_recovery.error.expect("比較拒否").code,
            "評価比較不能"
        );
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn tampered_private_ciphertext_blocks_experiment_before_pending_dialogue_creation() {
        let root = test_root("tamper");
        let vault = root.join("vault");
        let (mut broker, calls) = test_broker(&root);
        let manifest = accepted(call(
            &mut broker,
            1,
            BrokerOperation::評価Dataset登録,
            private_dataset("private-tamper-input", "approved evaluation response"),
            true,
        ));
        let storage_id = manifest["非公開保管ID"].as_str().expect("非公開保管IDがある");
        let path = vault.join(format!("evaluation-{storage_id}.dpapi"));
        let mut ciphertext = read_test_file(&path);
        ciphertext[0] ^= 1;
        overwrite_test_file(&path, &ciphertext);
        let denied = call(
            &mut broker,
            2,
            BrokerOperation::評価実験開始,
            json!({
                "版": 1,
                "評価DatasetID": manifest["評価DatasetID"],
                "対象Runtime一覧": ["fixture-a"],
            }),
            false,
        );
        assert_eq!(denied.status, BrokerStatus::Rejected);
        assert_eq!(denied.error.expect("denial").code, "評価Dataset読取失敗");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            accepted(call(
                &mut broker,
                3,
                BrokerOperation::対話承認待ち,
                json!({}),
                true,
            ))["要求"]
            .as_array()
            .map_or(0, Vec::len),
            0
        );
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn terminal_lifecycle_quarantine_blocks_evaluation_without_reusing_c4_approval() {
        let root = test_root("terminal-quarantine");
        let (mut broker, calls) = test_broker(&root);
        let c4 = c4_terminal_quarantine(&mut broker, 900);
        assert_eq!(c4["遷移後状態"], "quarantined");
        let manifest = accepted(call(
            &mut broker,
            1,
            BrokerOperation::評価Dataset登録,
            private_dataset("private-quarantine-input", "approved evaluation response"),
            true,
        ));
        let denied = call(
            &mut broker,
            2,
            BrokerOperation::評価実験開始,
            json!({
                "版": 1,
                "評価DatasetID": manifest["評価DatasetID"],
                "対象Runtime一覧": ["fixture-a"],
            }),
            false,
        );
        assert_eq!(denied.status, BrokerStatus::Rejected);
        assert_eq!(denied.error.expect("denial").code, "実行系隔離");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            accepted(call(
                &mut broker,
                3,
                BrokerOperation::対話承認待ち,
                json!({}),
                true,
            ))["要求"]
            .as_array()
            .map_or(0, Vec::len),
            0
        );
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn dataset_revision_must_increase_and_normal_list_returns_only_current_revision() {
        let root = test_root("revision");
        let (mut broker, _) = test_broker(&root);
        let revision_one = accepted(call(
            &mut broker,
            1,
            BrokerOperation::評価Dataset登録,
            private_dataset("revision-one-input", "approved evaluation response"),
            true,
        ));
        assert_eq!(revision_one["revision"], 1);
        let mut revision_three = private_dataset("revision-three-input", "approved evaluation response");
        revision_three["revision"] = json!(3);
        let revision_three = accepted(call(
            &mut broker,
            2,
            BrokerOperation::評価Dataset登録,
            revision_three,
            true,
        ));
        assert_eq!(revision_three["revision"], 3);

        let mut rollback = private_dataset("rollback-input", "approved evaluation response");
        rollback["revision"] = json!(2);
        let rejected = call(
            &mut broker,
            3,
            BrokerOperation::評価Dataset登録,
            rollback,
            true,
        );
        assert_eq!(rejected.status, BrokerStatus::Rejected);
        assert_eq!(
            rejected.error.expect("評価用データ集合の版後退が拒否される").code,
            "評価Dataset revision後退"
        );
        let listed = accepted(call(
            &mut broker,
            4,
            BrokerOperation::評価Dataset一覧,
            json!({"版": 1}),
            false,
        ));
        let datasets = listed["評価Dataset一覧"].as_array().expect("評価用データ集合一覧がある");
        assert_eq!(datasets.len(), 1);
        assert_eq!(datasets[0]["revision"], 3);
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn terminal_quarantine_after_start_becomes_interruption_via_existing_dialogue_record() {
        let root = test_root("quarantine-after-start");
        let (mut broker, calls) = test_broker(&root);
        let manifest = accepted(call(
            &mut broker,
            1,
            BrokerOperation::評価Dataset登録,
            private_dataset("private-after-start-input", "approved evaluation response"),
            true,
        ));
        let started = accepted(call(
            &mut broker,
            2,
            BrokerOperation::評価実験開始,
            json!({
                "版": 1,
                "評価DatasetID": manifest["評価DatasetID"],
                "対象Runtime一覧": ["fixture-a"],
            }),
            false,
        ));
        let experiment_id = started["評価ExperimentID"]
            .as_str()
            .expect("評価実験IDがある")
            .to_owned();
        let pending = accepted(call(
            &mut broker,
            3,
            BrokerOperation::対話承認待ち,
            json!({}),
            true,
        ));
        let request = &pending["要求"][0];
        let before_terminal = accepted(call(
            &mut broker,
            5,
            BrokerOperation::評価実験状態,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        ));
        assert_eq!(before_terminal["評価実験"]["結果数"], 0);
        accepted(call(
            &mut broker,
            4,
            BrokerOperation::対話承認,
            json!({
                "要求ID": request["要求"]["要求ID"],
                "要求hash": request["要求hash"],
                "表示範囲": "full",
            }),
            true,
        ));
        let c4 = c4_terminal_quarantine(&mut broker, 1_100);
        assert_eq!(c4["遷移後状態"], "quarantined");
        let mut final_status = None;
        for counter in 6..100 {
            let status = accepted(call(
                &mut broker,
                counter,
                BrokerOperation::評価実験状態,
                json!({"版": 1, "評価ExperimentID": experiment_id}),
                false,
            ));
            if status["評価実験"]["状態"] == "中断"
                && broker.対話.評価要求可能数() == 64
            {
                final_status = Some(status);
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let status = final_status.expect("C4隔離後にworker受信側を回収できる");
        assert_eq!(status["評価実験"]["状態"], "中断");
        assert_eq!(status["公開結果一覧"][0]["判定"], "中断");
        let terminal_result = broker
            .評価
            .active
            .get(&experiment_id)
            .and_then(|experiment| experiment.plans[0].result.as_ref())
            .expect("terminal中断結果が記録される");
        assert!(
            terminal_result["終了監査ID"].is_null(),
            "終端でない進捗記録を終了監査IDへ流用してはならない"
        );
        let reservation_audit_id = terminal_result["実行系隔離予約監査ID"]
            .as_str()
            .expect("C4中断Resultは実行系隔離予約監査IDを固定する")
            .to_owned();
        assert!(broker.audit_events().iter().any(|event| {
            event.event_id == reservation_audit_id
                && terminal_quarantine_reservation_event_matches(event, "fixture-a")
        }));
        let comparison = call(
            &mut broker,
            101,
            BrokerOperation::評価比較,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        );
        assert_eq!(comparison.status, BrokerStatus::Rejected);
        assert_eq!(comparison.error.expect("比較拒否").code, "評価比較不能");
        assert!(calls.load(Ordering::SeqCst) <= 1);
        drop(broker);
        let mut recovered = Broker::new_persistent("evaluation-session", &root.join("audit"))
            .expect("C4隔離後の監査からBrokerを再起動できる");
        recovered.current_epoch_seconds_override = Some(
            super::super::parse_issued_at_epoch_seconds("2026-06-01T00:02:00Z")
                .expect("epoch"),
        );
        let restored = accepted(call(
            &mut recovered,
            102,
            BrokerOperation::評価実験状態,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        ));
        assert_eq!(restored["評価実験"]["状態"], "中断");
        let restored_comparison = call(
            &mut recovered,
            103,
            BrokerOperation::評価比較,
            json!({"版": 1, "評価ExperimentID": experiment_id}),
            false,
        );
        assert_eq!(restored_comparison.status, BrokerStatus::Rejected);
        assert_eq!(
            restored_comparison.error.expect("復元後比較拒否").code,
            "評価比較不能"
        );
        drop(recovered);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn 公開experimentと比較集計の完結性を検証する() {
        let experiment = json!({
            "版": 1,
            "評価ExperimentID": "1".repeat(32),
            "評価DatasetID": "2".repeat(32),
            "Dataset定義hash": format!("sha256:{}", "a".repeat(64)),
            "対象Runtime一覧": ["fixture-a", "fixture-b"],
            "状態": "完了",
            "計画Case数": 1,
            "結果数": 2,
            "作成時刻UnixMillis": 100,
            "開始時刻UnixMillis": 110,
            "終了時刻UnixMillis": 120,
            "計画監査ID": "audit.evaluation.plan.1",
            "実験監査ID": "audit.evaluation.experiment.1",
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        });
        assert!(valid_experiment(&experiment));
        let mut incomplete_complete = experiment.clone();
        incomplete_complete["結果数"] = json!(1);
        assert!(!valid_experiment(&incomplete_complete));
        let mut overflow_complete = experiment.clone();
        overflow_complete["結果数"] = json!(3);
        assert!(!valid_experiment(&overflow_complete));
        let mut complete_without_finished_at = experiment.clone();
        complete_without_finished_at["終了時刻UnixMillis"] = Value::Null;
        assert!(!valid_experiment(&complete_without_finished_at));
        let mut complete_without_started_at = experiment.clone();
        complete_without_started_at["開始時刻UnixMillis"] = Value::Null;
        assert!(!valid_experiment(&complete_without_started_at));
        let mut reversed_timestamps = experiment.clone();
        reversed_timestamps["開始時刻UnixMillis"] = json!(121);
        reversed_timestamps["終了時刻UnixMillis"] = json!(120);
        assert!(!valid_experiment(&reversed_timestamps));
        let mut two_case_complete = experiment.clone();
        two_case_complete["計画Case数"] = json!(2);
        two_case_complete["結果数"] = json!(4);
        assert!(valid_experiment(&two_case_complete));
        let mut two_case_incomplete = two_case_complete.clone();
        two_case_incomplete["結果数"] = json!(2);
        assert!(!valid_experiment(&two_case_incomplete));

        let aggregate = |runtime: &str, passed: u64, failed: u64| {
            json!({
                "評価ExperimentID": "1".repeat(32),
                "実行系ID": runtime,
                "Dataset定義hash": format!("sha256:{}", "a".repeat(64)),
                "成立数": passed,
                "不成立数": failed,
                "評価不能数": 0,
                "中断数": 0,
                "平均LatencyMillis": 1.0,
            })
        };
        let comparison = json!({
            "版": 1,
            "比較ID": "3".repeat(32),
            "評価DatasetID": "2".repeat(32),
            "Dataset定義hash": format!("sha256:{}", "a".repeat(64)),
            "実験一覧": [aggregate("fixture-a", 1, 0), aggregate("fixture-b", 0, 1)],
            "計画Case数": 1,
            "成立数": 1,
            "不成立数": 1,
            "評価不能数": 0,
            "中断数": 0,
            "比較時刻UnixMillis": 130,
            "経路差": "same",
            "参照差": "same",
            "能力差": "unknown",
            "比較監査ID": "audit.evaluation.comparison.1",
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
        });
        assert!(valid_comparison(&comparison));
        let mut entry_overflow = comparison.clone();
        entry_overflow["実験一覧"][0]["成立数"] = json!(2);
        entry_overflow["実験一覧"][1]["不成立数"] = json!(0);
        entry_overflow["成立数"] = json!(2);
        entry_overflow["不成立数"] = json!(0);
        assert!(!valid_comparison(&entry_overflow));
        let mut entry_underflow = comparison.clone();
        entry_underflow["実験一覧"][0]["成立数"] = json!(0);
        entry_underflow["成立数"] = json!(0);
        assert!(!valid_comparison(&entry_underflow));
        let mut two_case_comparison = comparison.clone();
        two_case_comparison["計画Case数"] = json!(2);
        two_case_comparison["実験一覧"][0]["成立数"] = json!(2);
        two_case_comparison["実験一覧"][1]["不成立数"] = json!(2);
        two_case_comparison["成立数"] = json!(2);
        two_case_comparison["不成立数"] = json!(2);
        assert!(valid_comparison(&two_case_comparison));
        let mut two_case_underflow = two_case_comparison.clone();
        two_case_underflow["実験一覧"][0]["成立数"] = json!(1);
        two_case_underflow["成立数"] = json!(1);
        assert!(!valid_comparison(&two_case_underflow));
        let mut root_mismatch = comparison.clone();
        root_mismatch["成立数"] = json!(0);
        assert!(!valid_comparison(&root_mismatch));
        let mut mismatched_dataset_hash = comparison.clone();
        mismatched_dataset_hash["実験一覧"][1]["Dataset定義hash"] =
            json!(format!("sha256:{}", "b".repeat(64)));
        assert!(!valid_comparison(&mismatched_dataset_hash));
    }

    #[test]
    fn interrupted_evaluation_dialogues_release_session_and_request_capacity() {
        let root = test_root("capacity-release");
        let (mut broker, _) = test_broker(&root);
        let manifest = accepted(call(
            &mut broker,
            1,
            BrokerOperation::評価Dataset登録,
            private_dataset("capacity-private-input", "approved evaluation response"),
            true,
        ));
        // Session上限そのものは対話制御の単体試験で64件まで直接確認する。ここでは
        // C5が中止した各対話を既存の終了経路で回収し、連続したExperimentでも
        // 完全な容量へ戻すことを確認する。
        for index in 0..2usize {
            let base = 10_000 + index * 4;
            let started = accepted(call(
                &mut broker,
                base,
                BrokerOperation::評価実験開始,
                json!({
                    "版": 1,
                    "評価DatasetID": manifest["評価DatasetID"],
                    "対象Runtime一覧": ["fixture-a"],
                }),
                false,
            ));
            let experiment_id = started["評価ExperimentID"]
                .as_str()
                .expect("評価実験IDがある")
                .to_owned();
            let waiting = accepted(call(
                &mut broker,
                base + 1,
                BrokerOperation::対話承認待ち,
                json!({}),
                true,
            ));
            let pending = waiting["要求"].as_array().expect("承認待ち要求一覧がある");
            assert_eq!(pending.len(), 1, "round {index} の評価対話だけが承認待ち");
            let request_id = pending[0]["要求"]["要求ID"]
                .as_str()
                .expect("要求IDがある")
                .to_owned();
            accepted(call(
                &mut broker,
                base + 2,
                BrokerOperation::対話中止,
                json!({"要求ID": request_id}),
                false,
            ));
            let status = accepted(call(
                &mut broker,
                base + 3,
                BrokerOperation::評価実験状態,
                json!({"版": 1, "評価ExperimentID": experiment_id}),
                false,
            ));
            assert_eq!(status["評価実験"]["状態"], "中断");
            assert_eq!(broker.対話.評価要求可能数(), 64, "round {index} の回収後容量");
        }
        assert_eq!(broker.対話.評価要求可能数(), 64);
        drop(broker);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
