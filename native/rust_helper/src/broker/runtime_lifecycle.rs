#![allow(non_snake_case)]

//! 実行系lifecycleの閉じたBroker内制御面。
//!
//! このmoduleはIPCからPID、endpoint、command、argv、environment、状態、authority、
//! 統治IDを受け取らない。実行対象と統治対応は、Broker起動時に登録したadapterと
//! registryだけが所有する。generic `command_envelope` の代替ではない。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::audit_hash::sha256_tagged;
use crate::broker::audit::BrokerAuditEvent;
use crate::broker::dialogue::実行系ID妥当;

const APPROVAL_WINDOW_SECONDS: i64 = 300;

/// development buildだけが明示opt-inして登録できる固定runtime ID。
#[cfg(any(test, debug_assertions))]
pub(crate) const DEVELOPMENT_LIFECYCLE_RUNTIME_ID: &str = "development-lifecycle-fixture";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum LifecycleAction {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "restart")]
    Restart,
    #[serde(rename = "pause")]
    Pause,
    #[serde(rename = "resume")]
    Resume,
    #[serde(rename = "quarantine")]
    Quarantine,
}

impl LifecycleAction {
    #[cfg(any(test, debug_assertions))]
    pub(crate) const ALL: [Self; 6] = [
        Self::Start,
        Self::Stop,
        Self::Restart,
        Self::Pause,
        Self::Resume,
        Self::Quarantine,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Quarantine => "quarantine",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LifecycleState {
    Stopped,
    Ready,
    Paused,
    Quarantined,
    Unknown,
}

impl LifecycleState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Stopped => "stopped",
            Self::Ready => "ready",
            Self::Paused => "paused",
            Self::Quarantined => "quarantined",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LifecycleError {
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

impl LifecycleError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LifecycleAdapterFailure {
    pub(crate) message: String,
}

impl LifecycleAdapterFailure {
    #[cfg(any(test, debug_assertions))]
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Brokerのtrusted registrationだけが実装を渡せる、閉じたlifecycle adapter。
/// 外部requestのtarget、PID、command、environmentを受け取るmethodは意図的に持たない。
pub(crate) trait LifecycleAdapter: Send + Sync {
    /// 実装が固定して宣言する操作だけをBrokerが統治対象として登録する。
    /// 呼出側やAdapter metadataから操作集合を与える経路は持たない。
    fn supported_actions(&self) -> &'static [LifecycleAction];

    fn transition(
        &self,
        action: LifecycleAction,
    ) -> Result<LifecycleAdapterResult, LifecycleAdapterFailure>;

    /// audit障害またはadapter不整合時に、外部実行を継続させないためのbest-effort cleanup。
    fn fail_closed(&self);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LifecycleAdapterResult {
    pub(crate) next_state: LifecycleState,
    /// adapterが固定protocolで確認したack。raw child outputはここへ渡さない。
    pub(crate) acknowledgement: &'static str,
}

#[derive(Clone)]
struct LifecycleGovernance {
    capability_id: String,
    permission_id: String,
    permission_granted: bool,
    recovery_id: Option<String>,
}

impl LifecycleGovernance {
    fn fixed(action: LifecycleAction) -> Self {
        Self {
            capability_id: format!("runtime.lifecycle.{}", action.as_str()),
            permission_id: format!("permission.runtime.lifecycle.{}", action.as_str()),
            permission_granted: true,
            recovery_id: Some(format!("recover-runtime-lifecycle-{}", action.as_str())),
        }
    }
}

struct LifecycleRuntime {
    adapter: Arc<dyn LifecycleAdapter>,
    state: LifecycleState,
    governance: BTreeMap<LifecycleAction, LifecycleGovernance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LifecycleApprovalState {
    Pending,
    Approved,
    Consumed,
    Invalidated,
}

impl LifecycleApprovalState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Consumed => "consumed",
            Self::Invalidated => "invalidated",
        }
    }

    fn visible(self) -> bool {
        matches!(self, Self::Pending | Self::Approved)
    }
}

#[derive(Clone)]
struct LifecycleApproval {
    approval_id: String,
    approval_hash: String,
    execution_payload_hash: String,
    runtime_id: String,
    action: LifecycleAction,
    state: LifecycleApprovalState,
    expires_at_epoch_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LifecycleTransition {
    pub(crate) runtime_id: String,
    pub(crate) action: LifecycleAction,
    pub(crate) previous_state: LifecycleState,
    pub(crate) next_state: LifecycleState,
    pub(crate) approval_id: String,
    pub(crate) observed_at_epoch_millis: i64,
}

/// Adapterのcapability、current permission、approval、recovery、状態をBrokerだけが保持する。
#[derive(Default)]
pub(crate) struct RuntimeLifecycleRegistry {
    runtimes: BTreeMap<String, LifecycleRuntime>,
    approvals: BTreeMap<String, LifecycleApproval>,
    terminal_quarantines: BTreeSet<String>,
}

impl fmt::Debug for RuntimeLifecycleRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeLifecycleRegistry")
            .field("runtime_count", &self.runtimes.len())
            .field("approval_count", &self.approvals.len())
            .field(
                "terminal_quarantine_count",
                &self.terminal_quarantines.len(),
            )
            .finish()
    }
}

impl RuntimeLifecycleRegistry {
    /// `BrokerPersistentStore`が検証済みのaudit chainから復元したterminal隔離だけを渡す。
    pub(crate) fn with_terminal_quarantines(terminal_quarantines: BTreeSet<String>) -> Self {
        Self {
            runtimes: BTreeMap::new(),
            approvals: BTreeMap::new(),
            terminal_quarantines,
        }
    }

    /// durable監査から復元したterminal隔離は、通常の実行系登録で置換できない。
    pub(crate) fn is_terminally_quarantined(&self, runtime_id: &str) -> bool {
        self.terminal_quarantines.contains(runtime_id)
    }

    /// durable隔離予約が確定した時点で通常経路の再接続を止める。
    /// adapter実行の成否やBroker processの継続に依存させない。
    pub(crate) fn mark_terminal_quarantine(&mut self, runtime_id: &str) {
        self.terminal_quarantines.insert(runtime_id.to_string());
    }

    /// 隔離操作は外部実行前にdurable auditへ予約する。予約済みrecordは、終了監査が
    /// 失敗またはprocessが停止しても再起動時のterminal隔離を維持する。
    pub(crate) fn quarantine_reservation_reason(runtime_id: &str) -> String {
        format!("実行系ライフサイクル隔離予約 実行系ID={runtime_id}")
    }

    /// `BrokerPersistentStore`でhash chainとanchorを検証した後にだけ使う。
    /// reservation recordが壊れている場合は、過去の隔離を無視して再稼働させず起動を拒否する。
    pub(crate) fn terminal_quarantines_from_verified_audit(
        events: &[BrokerAuditEvent],
    ) -> Result<BTreeSet<String>, LifecycleError> {
        const PREFIX: &str = "実行系ライフサイクル隔離予約 実行系ID=";
        let mut quarantines = BTreeSet::new();
        for event in events {
            if event.operation != "実行系ライフサイクル操作" || !event.reason.starts_with(PREFIX)
            {
                continue;
            }
            let runtime_id = event.reason.strip_prefix(PREFIX).unwrap_or_default();
            if event.decision != "received"
                || event.evidence_source != "INTERNAL_STATE"
                || !実行系ID妥当(runtime_id)
                || event.reason != Self::quarantine_reservation_reason(runtime_id)
            {
                return Err(LifecycleError::new(
                    "lifecycle_quarantine_record_invalid",
                    "検証済み監査chain内のlifecycle隔離予約recordが不正",
                ));
            }
            quarantines.insert(runtime_id.to_string());
        }
        Ok(quarantines)
    }

    /// trusted bootstrap/test pathだけが使う。IPC、Adapter metadata、UI stateからは到達しない。
    #[allow(dead_code)] // 現在のrelease buildにはtrusted lifecycle adapterを同梱しない。
    pub(crate) fn register_trusted(
        &mut self,
        runtime_id: &str,
        adapter: Arc<dyn LifecycleAdapter>,
    ) -> Result<(), LifecycleError> {
        if !実行系ID妥当(runtime_id) {
            return Err(LifecycleError::new(
                "lifecycle_runtime_invalid",
                "実行系IDがlifecycle登録境界に適合しない",
            ));
        }
        if self.runtimes.contains_key(runtime_id) {
            return Err(LifecycleError::new(
                "lifecycle_runtime_duplicate",
                "実行系lifecycleは重複登録できない",
            ));
        }
        let mut governance = BTreeMap::new();
        for action in adapter.supported_actions().iter().copied() {
            if governance
                .insert(action, LifecycleGovernance::fixed(action))
                .is_some()
            {
                return Err(LifecycleError::new(
                    "lifecycle_capability_duplicate",
                    "adapterの宣言操作に重複がある",
                ));
            }
        }
        if governance.is_empty() {
            return Err(LifecycleError::new(
                "lifecycle_capability_empty",
                "adapterは少なくとも一つの閉じたlifecycle操作を宣言しなければならない",
            ));
        }
        let restored_quarantine = self.terminal_quarantines.contains(runtime_id);
        if restored_quarantine {
            adapter.fail_closed();
        }
        self.runtimes.insert(
            runtime_id.to_string(),
            LifecycleRuntime {
                adapter,
                state: if restored_quarantine {
                    LifecycleState::Quarantined
                } else {
                    LifecycleState::Stopped
                },
                governance,
            },
        );
        Ok(())
    }

    #[cfg(any(test, debug_assertions))]
    pub(crate) fn unregister_and_fail_closed(&mut self, runtime_id: &str) {
        if let Some(runtime) = self.runtimes.remove(runtime_id) {
            runtime.adapter.fail_closed();
        }
        for approval in self.approvals.values_mut() {
            if approval.runtime_id == runtime_id && approval.state.visible() {
                approval.state = LifecycleApprovalState::Invalidated;
            }
        }
    }

    /// durable auditを追加できないとき、実行済み・承認済みのどちらも再利用させない。
    pub(crate) fn fail_closed(&mut self) {
        for runtime in self.runtimes.values_mut() {
            runtime.adapter.fail_closed();
            runtime.state = LifecycleState::Unknown;
        }
        for approval in self.approvals.values_mut() {
            if approval.state.visible() {
                approval.state = LifecycleApprovalState::Invalidated;
            }
        }
    }

    pub(crate) fn status_body(
        &mut self,
        runtime_id: &str,
        now_epoch_seconds: i64,
    ) -> Option<Value> {
        self.expire(now_epoch_seconds);
        if self.is_terminally_quarantined(runtime_id) {
            return Some(json!({
                "版": 1,
                "実行系ID": runtime_id,
                "対応": true,
                "状態": LifecycleState::Quarantined.as_str(),
                "証拠種別": "INTERNAL_STATE",
                "操作一覧": [],
                "承認一覧": [],
            }));
        }
        let runtime = self.runtimes.get(runtime_id)?;
        let state = runtime.state;
        let operations = if state == LifecycleState::Quarantined {
            Vec::new()
        } else {
            runtime
            .governance
            .iter()
            .map(|(action, governance)| {
                json!({
                    "操作": action.as_str(),
                    "能力ID": governance.capability_id,
                    "権限ID": governance.permission_id,
                    "承認必要": true,
                    "復旧ID": governance.recovery_id.clone().unwrap_or_default(),
                    "実行可能": governance.permission_granted
                        && governance.recovery_id.as_deref().is_some_and(|value| !value.is_empty())
                        && transition_after(state, *action).is_some(),
                })
            })
            .collect::<Vec<_>>()
        };
        let approvals = if state == LifecycleState::Quarantined {
            Vec::new()
        } else {
            self.approvals
                .values()
                .filter(|approval| {
                    approval.runtime_id == runtime_id
                        && approval.state.visible()
                        && approval.expires_at_epoch_seconds > now_epoch_seconds
                })
                .filter_map(|approval| {
                    runtime
                        .governance
                        .get(&approval.action)
                        .map(|governance| approval_projection(approval, governance))
                })
                .collect::<Vec<_>>()
        };
        Some(json!({
            "版": 1,
            "実行系ID": runtime_id,
            "対応": true,
            "状態": state.as_str(),
            "証拠種別": "INTERNAL_STATE",
            "操作一覧": operations,
            "承認一覧": approvals,
        }))
    }

    pub(crate) fn unsupported_status_body(runtime_id: &str) -> Value {
        json!({
            "版": 1,
            "実行系ID": runtime_id,
            "対応": false,
            "状態": "not_supported",
            "証拠種別": "INTERNAL_STATE",
            "操作一覧": [],
            "承認一覧": [],
        })
    }

    pub(crate) fn request_approval(
        &mut self,
        runtime_id: &str,
        action: LifecycleAction,
        now_epoch_seconds: i64,
    ) -> Result<Value, LifecycleError> {
        self.expire(now_epoch_seconds);
        self.validate_requestable(runtime_id, action)?;
        if let Some(existing) = self.approvals.values().find(|approval| {
            approval.runtime_id == runtime_id
                && approval.action == action
                && approval.state == LifecycleApprovalState::Pending
                && approval.expires_at_epoch_seconds > now_epoch_seconds
        }) {
            let governance = self
                .runtimes
                .get(runtime_id)
                .and_then(|runtime| runtime.governance.get(&action))
                .expect("登録済みactionの統治対応");
            return Ok(approval_projection(existing, governance));
        }
        if self.approvals.values().any(|approval| {
            approval.runtime_id == runtime_id
                && approval.action == action
                && approval.state == LifecycleApprovalState::Approved
                && approval.expires_at_epoch_seconds > now_epoch_seconds
        }) {
            return Err(LifecycleError::new(
                "lifecycle_approval_already_approved",
                "同じ操作には未消費のowner承認がある",
            ));
        }
        let approval_id = random_identifier()?;
        let expires_at_epoch_seconds = now_epoch_seconds.saturating_add(APPROVAL_WINDOW_SECONDS);
        let execution_payload_hash =
            canonical_execution_payload_hash(&approval_id, runtime_id, action);
        let approval_hash = approval_hash(
            &approval_id,
            runtime_id,
            action,
            expires_at_epoch_seconds,
            &execution_payload_hash,
        );
        let approval = LifecycleApproval {
            approval_id: approval_id.clone(),
            approval_hash,
            execution_payload_hash,
            runtime_id: runtime_id.to_string(),
            action,
            state: LifecycleApprovalState::Pending,
            expires_at_epoch_seconds,
        };
        let governance = self
            .runtimes
            .get(runtime_id)
            .and_then(|runtime| runtime.governance.get(&action))
            .expect("登録済みactionの統治対応")
            .clone();
        let body = approval_projection(&approval, &governance);
        self.approvals.insert(approval_id, approval);
        Ok(body)
    }

    pub(crate) fn approve(
        &mut self,
        approval_id: &str,
        supplied_hash: &str,
        now_epoch_seconds: i64,
    ) -> Result<Value, LifecycleError> {
        self.expire(now_epoch_seconds);
        let approval = self.approvals.get(approval_id).cloned().ok_or_else(|| {
            LifecycleError::new("lifecycle_approval_missing", "承認IDが存在しない")
        })?;
        if approval.expires_at_epoch_seconds <= now_epoch_seconds {
            return Err(LifecycleError::new(
                "lifecycle_approval_expired",
                "承認の有効期限が切れている",
            ));
        }
        if approval.state != LifecycleApprovalState::Pending {
            return Err(LifecycleError::new(
                "lifecycle_approval_not_pending",
                "承認はpending状態でなければならない",
            ));
        }
        if approval.approval_hash != supplied_hash
            || approval.approval_hash
                != approval_hash(
                    &approval.approval_id,
                    &approval.runtime_id,
                    approval.action,
                    approval.expires_at_epoch_seconds,
                    &approval.execution_payload_hash,
                )
        {
            return Err(LifecycleError::new(
                "lifecycle_approval_hash_mismatch",
                "承認hashがBroker保存済みcanonical承認と一致しない",
            ));
        }
        self.validate_requestable(&approval.runtime_id, approval.action)?;
        let governance = self
            .runtimes
            .get(&approval.runtime_id)
            .and_then(|runtime| runtime.governance.get(&approval.action))
            .expect("登録済みactionの統治対応")
            .clone();
        let stored = self
            .approvals
            .get_mut(approval_id)
            .expect("直前に確認済みの承認");
        stored.state = LifecycleApprovalState::Approved;
        Ok(approval_projection(stored, &governance))
    }

    /// pre-audit前の確認。実action直前にも`execute`で同じ条件を再確認する。
    pub(crate) fn preflight_execution(
        &mut self,
        runtime_id: &str,
        action: LifecycleAction,
        approval_id: &str,
        execution_payload_hash: &str,
        now_epoch_seconds: i64,
    ) -> Result<(), LifecycleError> {
        self.authorized_execution(
            runtime_id,
            action,
            approval_id,
            execution_payload_hash,
            now_epoch_seconds,
        )
            .map(|_| ())
    }

    /// Broker直列処理中に、adapter呼出しの直前で全統治条件を再照合する。
    pub(crate) fn execute(
        &mut self,
        runtime_id: &str,
        action: LifecycleAction,
        approval_id: &str,
        execution_payload_hash: &str,
        now_epoch_seconds: i64,
        observed_at_epoch_millis: i64,
    ) -> Result<LifecycleTransition, LifecycleError> {
        let (previous_state, adapter) = self.authorized_execution(
            runtime_id,
            action,
            approval_id,
            execution_payload_hash,
            now_epoch_seconds,
        )?;
        let expected_state = transition_after(previous_state, action).ok_or_else(|| {
            LifecycleError::new(
                "lifecycle_invalid_transition",
                "現在状態から要求されたlifecycle操作へ遷移できない",
            )
        })?;
        let result = match adapter.transition(action) {
            Ok(result)
                if result.next_state == expected_state
                    && !result.acknowledgement.is_empty()
                    && result.acknowledgement.len() <= 128 =>
            {
                result
            }
            Ok(_) => {
                adapter.fail_closed();
                self.consume_and_fail_closed(runtime_id, approval_id);
                return Err(LifecycleError::new(
                    "lifecycle_adapter_state_mismatch",
                    "adapter確認結果がBroker定義の状態遷移と一致しない",
                ));
            }
            Err(error) => {
                adapter.fail_closed();
                self.consume_and_fail_closed(runtime_id, approval_id);
                return Err(LifecycleError::new(
                    "lifecycle_adapter_failed",
                    format!("登録済みlifecycle adapterが失敗: {}", error.message),
                ));
            }
        };
        let _acknowledged = result.acknowledgement;
        let runtime = self
            .runtimes
            .get_mut(runtime_id)
            .expect("直前に登録済みruntimeを確認済み");
        runtime.state = expected_state;
        self.approvals
            .get_mut(approval_id)
            .expect("直前に承認を確認済み")
            .state = LifecycleApprovalState::Consumed;
        if action == LifecycleAction::Quarantine {
            self.mark_terminal_quarantine(runtime_id);
        }
        Ok(LifecycleTransition {
            runtime_id: runtime_id.to_string(),
            action,
            previous_state,
            next_state: expected_state,
            approval_id: approval_id.to_string(),
            observed_at_epoch_millis: observed_at_epoch_millis.max(0),
        })
    }

    fn validate_requestable(
        &self,
        runtime_id: &str,
        action: LifecycleAction,
    ) -> Result<(), LifecycleError> {
        let runtime = self.runtimes.get(runtime_id).ok_or_else(|| {
            LifecycleError::new(
                "lifecycle_runtime_unknown",
                "実行系lifecycleが登録されていない",
            )
        })?;
        let governance = runtime.governance.get(&action).ok_or_else(|| {
            LifecycleError::new(
                "lifecycle_capability_missing",
                "要求されたlifecycle capabilityは登録されていない",
            )
        })?;
        if !governance.permission_granted {
            return Err(LifecycleError::new(
                "lifecycle_permission_denied",
                "Broker所有permissionがlifecycle操作を許可していない",
            ));
        }
        if governance
            .recovery_id
            .as_deref()
            .is_none_or(|recovery| recovery.is_empty())
        {
            return Err(LifecycleError::new(
                "lifecycle_recovery_missing",
                "lifecycle操作に対応するRecoveryActionがない",
            ));
        }
        if transition_after(runtime.state, action).is_none() {
            return Err(LifecycleError::new(
                "lifecycle_invalid_transition",
                "現在状態から要求されたlifecycle操作へ遷移できない",
            ));
        }
        Ok(())
    }

    fn authorized_execution(
        &mut self,
        runtime_id: &str,
        action: LifecycleAction,
        approval_id: &str,
        execution_payload_hash: &str,
        now_epoch_seconds: i64,
    ) -> Result<(LifecycleState, Arc<dyn LifecycleAdapter>), LifecycleError> {
        self.expire(now_epoch_seconds);
        self.validate_requestable(runtime_id, action)?;
        let approval = self.approvals.get(approval_id).cloned().ok_or_else(|| {
            LifecycleError::new("lifecycle_approval_missing", "承認IDが存在しない")
        })?;
        if approval.runtime_id != runtime_id || approval.action != action {
            return Err(LifecycleError::new(
                "lifecycle_approval_binding_mismatch",
                "承認は指定された実行系と操作に結合されていない",
            ));
        }
        if approval.expires_at_epoch_seconds <= now_epoch_seconds {
            return Err(LifecycleError::new(
                "lifecycle_approval_expired",
                "承認の有効期限が切れている",
            ));
        }
        if approval.state != LifecycleApprovalState::Approved {
            return Err(LifecycleError::new(
                "lifecycle_approval_not_approved",
                "owner承認済みかつ未消費の承認が必要",
            ));
        }
        if approval.approval_hash
            != approval_hash(
                &approval.approval_id,
                &approval.runtime_id,
                approval.action,
                approval.expires_at_epoch_seconds,
                &approval.execution_payload_hash,
            )
        {
            return Err(LifecycleError::new(
                "lifecycle_approval_hash_mismatch",
                "Broker保存済み承認hashの整合を確認できない",
            ));
        }
        if approval.execution_payload_hash
            != canonical_execution_payload_hash(
                &approval.approval_id,
                &approval.runtime_id,
                approval.action,
            )
        {
            return Err(LifecycleError::new(
                "lifecycle_approval_hash_mismatch",
                "Broker保存済み承認の実行要求hashを確認できない",
            ));
        }
        if approval.execution_payload_hash != execution_payload_hash {
            return Err(LifecycleError::new(
                "lifecycle_approval_payload_hash_mismatch",
                "承認は現在のcanonical実行要求payloadに結合されていない",
            ));
        }
        let runtime = self
            .runtimes
            .get(runtime_id)
            .expect("validate_requestableで登録済み確認済み");
        Ok((runtime.state, Arc::clone(&runtime.adapter)))
    }

    fn consume_and_fail_closed(&mut self, runtime_id: &str, approval_id: &str) {
        if let Some(approval) = self.approvals.get_mut(approval_id) {
            approval.state = LifecycleApprovalState::Consumed;
        }
        if let Some(runtime) = self.runtimes.get_mut(runtime_id) {
            runtime.state = LifecycleState::Unknown;
        }
    }

    fn expire(&mut self, now_epoch_seconds: i64) {
        for approval in self.approvals.values_mut() {
            if approval.state.visible() && approval.expires_at_epoch_seconds <= now_epoch_seconds {
                approval.state = LifecycleApprovalState::Invalidated;
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn set_permission_for_test(
        &mut self,
        runtime_id: &str,
        action: LifecycleAction,
        granted: bool,
    ) {
        self.runtimes
            .get_mut(runtime_id)
            .unwrap()
            .governance
            .get_mut(&action)
            .unwrap()
            .permission_granted = granted;
    }

    #[cfg(test)]
    pub(crate) fn remove_recovery_for_test(&mut self, runtime_id: &str, action: LifecycleAction) {
        self.runtimes
            .get_mut(runtime_id)
            .unwrap()
            .governance
            .get_mut(&action)
            .unwrap()
            .recovery_id = None;
    }

    #[cfg(test)]
    pub(crate) fn remove_capability_for_test(&mut self, runtime_id: &str, action: LifecycleAction) {
        self.runtimes
            .get_mut(runtime_id)
            .unwrap()
            .governance
            .remove(&action);
    }
}

fn transition_after(state: LifecycleState, action: LifecycleAction) -> Option<LifecycleState> {
    match (state, action) {
        (LifecycleState::Stopped, LifecycleAction::Start) => Some(LifecycleState::Ready),
        (LifecycleState::Ready, LifecycleAction::Stop) => Some(LifecycleState::Stopped),
        (LifecycleState::Paused, LifecycleAction::Stop) => Some(LifecycleState::Stopped),
        (LifecycleState::Ready, LifecycleAction::Restart) => Some(LifecycleState::Ready),
        (LifecycleState::Ready, LifecycleAction::Pause) => Some(LifecycleState::Paused),
        (LifecycleState::Paused, LifecycleAction::Resume) => Some(LifecycleState::Ready),
        (
            LifecycleState::Stopped | LifecycleState::Ready | LifecycleState::Paused,
            LifecycleAction::Quarantine,
        ) => Some(LifecycleState::Quarantined),
        _ => None,
    }
}

fn approval_hash(
    approval_id: &str,
    runtime_id: &str,
    action: LifecycleAction,
    expires_at_epoch_seconds: i64,
    execution_payload_hash: &str,
) -> String {
    sha256_tagged(
        json!({
            "版": 1,
            "承認ID": approval_id,
            "実行系ID": runtime_id,
            "操作": action.as_str(),
            "有効期限UnixSeconds": expires_at_epoch_seconds,
            "実行要求hash": execution_payload_hash,
        })
        .to_string()
        .as_bytes(),
    )
}

/// 実行payloadは閉じたfield集合なので、owner承認IDを生成した後に正本hashを固定できる。
/// IPC外側のpayload hashはBroker protocolでこの同じ正本との一致を先に確認する。
fn canonical_execution_payload_hash(
    approval_id: &str,
    runtime_id: &str,
    action: LifecycleAction,
) -> String {
    let payload = json!({
        "版": 1,
        "実行系ID": runtime_id,
        "操作": action.as_str(),
        "承認ID": approval_id,
    });
    sha256_tagged(
        &serde_json::to_vec(&payload).unwrap_or_else(|_| b"null".to_vec()),
    )
}

fn approval_projection(approval: &LifecycleApproval, governance: &LifecycleGovernance) -> Value {
    json!({
        "版": 1,
        "承認ID": approval.approval_id,
        "承認hash": approval.approval_hash,
        "実行系ID": approval.runtime_id,
        "操作": approval.action.as_str(),
        "状態": approval.state.as_str(),
        "有効期限UnixSeconds": approval.expires_at_epoch_seconds,
        "統治": {
            "能力ID": governance.capability_id,
            "権限ID": governance.permission_id,
            "承認ID": approval.approval_id,
            "承認状態": approval.state.as_str(),
            "復旧ID": governance.recovery_id.clone().unwrap_or_default(),
        },
    })
}

fn random_identifier() -> Result<String, LifecycleError> {
    let mut bytes = [0_u8; 16];
    getrandom::getrandom(&mut bytes).map_err(|_| {
        LifecycleError::new(
            "lifecycle_identifier_generation_failed",
            "lifecycle承認IDを生成できない",
        )
    })?;
    Ok(hex::encode(bytes))
}

#[cfg(debug_assertions)]
mod development_fixture {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Child, ChildStdin, Command, Stdio};
    use std::sync::{mpsc, Mutex};
    use std::thread;
    use std::time::Duration;

    use super::{
        LifecycleAction, LifecycleAdapter, LifecycleAdapterFailure, LifecycleAdapterResult,
        LifecycleState,
    };

    const ACK_TIMEOUT: Duration = Duration::from_secs(2);

    pub(crate) struct DevelopmentLifecycleAdapter {
        child: Mutex<Option<FixtureChild>>,
    }

    struct FixtureChild {
        child: Child,
        stdin: ChildStdin,
        acknowledgement: mpsc::Receiver<String>,
    }

    impl DevelopmentLifecycleAdapter {
        pub(crate) fn new() -> Result<Self, LifecycleAdapterFailure> {
            Ok(Self {
                child: Mutex::new(None),
            })
        }

        fn start_locked(slot: &mut Option<FixtureChild>) -> Result<(), LifecycleAdapterFailure> {
            if slot.is_some() {
                return Err(LifecycleAdapterFailure::new("fixture childが既に存在する"));
            }
            let executable = std::env::current_exe().map_err(|_| {
                LifecycleAdapterFailure::new("fixture childの実行fileを確認できない")
            })?;
            let mut child = Command::new(executable)
                .arg("development-lifecycle-fixture-child")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|_| LifecycleAdapterFailure::new("固定fixture childを起動できない"))?;
            let stdin = match child.stdin.take() {
                Some(value) => value,
                None => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(LifecycleAdapterFailure::new(
                        "fixture child stdinを所有できない",
                    ));
                }
            };
            let stdout = match child.stdout.take() {
                Some(value) => value,
                None => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(LifecycleAdapterFailure::new(
                        "fixture child stdoutを所有できない",
                    ));
                }
            };
            let (sender, receiver) = mpsc::sync_channel(8);
            if thread::Builder::new()
                .name("lifecycle-fixture-stdout".to_string())
                .spawn(move || {
                    for line in BufReader::new(stdout).lines() {
                        match line {
                            Ok(line) => {
                                if sender.send(line).is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                })
                .is_err()
            {
                let _ = child.kill();
                let _ = child.wait();
                return Err(LifecycleAdapterFailure::new(
                    "fixture child応答readerを起動できない",
                ));
            }
            let mut fixture = FixtureChild {
                child,
                stdin,
                acknowledgement: receiver,
            };
            if fixture.receive("ready").is_err() {
                fixture.terminate();
                return Err(LifecycleAdapterFailure::new(
                    "fixture child起動ackを確認できない",
                ));
            }
            *slot = Some(fixture);
            Ok(())
        }

        fn send(&self, command: &str) -> Result<(), LifecycleAdapterFailure> {
            let mut guard = self
                .child
                .lock()
                .map_err(|_| LifecycleAdapterFailure::new("fixture child状態lockを取得できない"))?;
            let fixture = guard
                .as_mut()
                .ok_or_else(|| LifecycleAdapterFailure::new("fixture childが存在しない"))?;
            fixture.send(command)
        }

        fn stop_and_clear(&self, command: &str) -> Result<(), LifecycleAdapterFailure> {
            let mut guard = self
                .child
                .lock()
                .map_err(|_| LifecycleAdapterFailure::new("fixture child状態lockを取得できない"))?;
            let Some(mut fixture) = guard.take() else {
                return Err(LifecycleAdapterFailure::new("fixture childが存在しない"));
            };
            let result = fixture.send(command);
            fixture.terminate();
            result
        }

        fn restart(&self) -> Result<(), LifecycleAdapterFailure> {
            self.stop_and_clear("stop")?;
            let mut guard = self
                .child
                .lock()
                .map_err(|_| LifecycleAdapterFailure::new("fixture child状態lockを取得できない"))?;
            Self::start_locked(&mut guard)
        }

        fn cleanup(&self) {
            if let Ok(mut guard) = self.child.lock() {
                if let Some(mut fixture) = guard.take() {
                    fixture.terminate();
                }
            }
        }
    }

    impl LifecycleAdapter for DevelopmentLifecycleAdapter {
        fn supported_actions(&self) -> &'static [LifecycleAction] {
            &LifecycleAction::ALL
        }

        fn transition(
            &self,
            action: LifecycleAction,
        ) -> Result<LifecycleAdapterResult, LifecycleAdapterFailure> {
            match action {
                LifecycleAction::Start => {
                    let mut guard = self.child.lock().map_err(|_| {
                        LifecycleAdapterFailure::new("fixture child状態lockを取得できない")
                    })?;
                    Self::start_locked(&mut guard)?;
                    Ok(LifecycleAdapterResult {
                        next_state: LifecycleState::Ready,
                        acknowledgement: "fixture:start",
                    })
                }
                LifecycleAction::Stop => {
                    self.stop_and_clear("stop")?;
                    Ok(LifecycleAdapterResult {
                        next_state: LifecycleState::Stopped,
                        acknowledgement: "fixture:stop",
                    })
                }
                LifecycleAction::Restart => {
                    self.restart()?;
                    Ok(LifecycleAdapterResult {
                        next_state: LifecycleState::Ready,
                        acknowledgement: "fixture:restart",
                    })
                }
                LifecycleAction::Pause => {
                    self.send("pause")?;
                    Ok(LifecycleAdapterResult {
                        next_state: LifecycleState::Paused,
                        acknowledgement: "fixture:pause",
                    })
                }
                LifecycleAction::Resume => {
                    self.send("resume")?;
                    Ok(LifecycleAdapterResult {
                        next_state: LifecycleState::Ready,
                        acknowledgement: "fixture:resume",
                    })
                }
                LifecycleAction::Quarantine => {
                    self.stop_and_clear("quarantine")?;
                    Ok(LifecycleAdapterResult {
                        next_state: LifecycleState::Quarantined,
                        acknowledgement: "fixture:quarantine",
                    })
                }
            }
        }

        fn fail_closed(&self) {
            self.cleanup();
        }
    }

    impl Drop for DevelopmentLifecycleAdapter {
        fn drop(&mut self) {
            self.cleanup();
        }
    }

    impl FixtureChild {
        fn receive(&mut self, expected: &str) -> Result<(), LifecycleAdapterFailure> {
            match self.acknowledgement.recv_timeout(ACK_TIMEOUT) {
                Ok(value) if value == expected => Ok(()),
                _ => Err(LifecycleAdapterFailure::new("fixture childのackが不正")),
            }
        }

        fn send(&mut self, command: &str) -> Result<(), LifecycleAdapterFailure> {
            self.stdin
                .write_all(format!("{command}\n").as_bytes())
                .and_then(|_| self.stdin.flush())
                .map_err(|_| LifecycleAdapterFailure::new("fixture childへ制御を書き込めない"))?;
            self.receive(&format!("ack:{command}"))
        }

        fn terminate(&mut self) {
            for _ in 0..20 {
                if self.child.try_wait().ok().flatten().is_some() {
                    return;
                }
                thread::sleep(Duration::from_millis(25));
            }
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    pub(crate) fn adapter() -> Result<DevelopmentLifecycleAdapter, LifecycleAdapterFailure> {
        DevelopmentLifecycleAdapter::new()
    }

    /// fixed child protocol。parentが所有するstdin/stdoutだけを使い、任意引数を受け取らない。
    pub(crate) fn run_child() -> i32 {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        if writeln!(stdout, "ready")
            .and_then(|_| stdout.flush())
            .is_err()
        {
            return 1;
        }
        let mut paused = false;
        for line in stdin.lock().lines() {
            let Ok(line) = line else {
                return 1;
            };
            let (ack, exit) = match line.as_str() {
                "pause" if !paused => {
                    paused = true;
                    ("ack:pause", false)
                }
                "resume" if paused => {
                    paused = false;
                    ("ack:resume", false)
                }
                "stop" | "quarantine" => {
                    let ack = if line == "stop" {
                        "ack:stop"
                    } else {
                        "ack:quarantine"
                    };
                    (ack, true)
                }
                _ => ("ack:invalid", true),
            };
            if writeln!(stdout, "{ack}")
                .and_then(|_| stdout.flush())
                .is_err()
            {
                return 1;
            }
            if exit {
                return if ack == "ack:invalid" { 2 } else { 0 };
            }
        }
        0
    }
}

#[cfg(debug_assertions)]
pub(crate) fn development_fixture_adapter(
) -> Result<development_fixture::DevelopmentLifecycleAdapter, LifecycleAdapterFailure> {
    development_fixture::adapter()
}

#[cfg(debug_assertions)]
pub(crate) fn run_development_fixture_child() -> i32 {
    development_fixture::run_child()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use crate::broker::audit::BrokerAuditLog;

    use super::*;

    struct TestAdapter {
        calls: Arc<AtomicUsize>,
        actions: &'static [LifecycleAction],
    }

    impl LifecycleAdapter for TestAdapter {
        fn supported_actions(&self) -> &'static [LifecycleAction] {
            self.actions
        }

        fn transition(
            &self,
            action: LifecycleAction,
        ) -> Result<LifecycleAdapterResult, LifecycleAdapterFailure> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let next_state = match action {
                LifecycleAction::Start | LifecycleAction::Restart | LifecycleAction::Resume => {
                    LifecycleState::Ready
                }
                LifecycleAction::Stop => LifecycleState::Stopped,
                LifecycleAction::Pause => LifecycleState::Paused,
                LifecycleAction::Quarantine => LifecycleState::Quarantined,
            };
            Ok(LifecycleAdapterResult {
                next_state,
                acknowledgement: "test:ack",
            })
        }

        fn fail_closed(&self) {}
    }

    fn new_registry() -> (RuntimeLifecycleRegistry, Arc<AtomicUsize>) {
        registry_with_actions(&LifecycleAction::ALL)
    }

    fn registry_with_actions(
        actions: &'static [LifecycleAction],
    ) -> (RuntimeLifecycleRegistry, Arc<AtomicUsize>) {
        let mut registry = RuntimeLifecycleRegistry::default();
        let calls = Arc::new(AtomicUsize::new(0));
        registry
            .register_trusted(
                "fixture",
                Arc::new(TestAdapter {
                    calls: Arc::clone(&calls),
                    actions,
                }),
            )
            .unwrap();
        (registry, calls)
    }

    fn approval(registry: &mut RuntimeLifecycleRegistry, action: LifecycleAction) -> Value {
        let pending = registry.request_approval("fixture", action, 100).unwrap();
        registry
            .approve(
                pending["承認ID"].as_str().unwrap(),
                pending["承認hash"].as_str().unwrap(),
                101,
            )
            .unwrap()
    }

    fn execution_hash(action: LifecycleAction, approval: &Value) -> String {
        canonical_execution_payload_hash(
            approval["承認ID"].as_str().unwrap(),
            "fixture",
            action,
        )
    }

    #[test]
    fn registry_rejects_unknown_capability_permission_recovery_and_invalid_state_without_adapter_call(
    ) {
        {
            let (mut registry, _) = new_registry();
            assert_eq!(
                registry
                    .request_approval("unknown", LifecycleAction::Start, 100)
                    .unwrap_err()
                    .code,
                "lifecycle_runtime_unknown"
            );
            registry.remove_capability_for_test("fixture", LifecycleAction::Start);
            assert_eq!(
                registry
                    .request_approval("fixture", LifecycleAction::Start, 100)
                    .unwrap_err()
                    .code,
                "lifecycle_capability_missing"
            );
        }
        {
            let (mut registry, calls) = new_registry();
            registry.set_permission_for_test("fixture", LifecycleAction::Start, false);
            assert_eq!(
                registry
                    .request_approval("fixture", LifecycleAction::Start, 100)
                    .unwrap_err()
                    .code,
                "lifecycle_permission_denied"
            );
            registry.set_permission_for_test("fixture", LifecycleAction::Start, true);
            registry.remove_recovery_for_test("fixture", LifecycleAction::Start);
            assert_eq!(
                registry
                    .request_approval("fixture", LifecycleAction::Start, 100)
                    .unwrap_err()
                    .code,
                "lifecycle_recovery_missing"
            );
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        }
        {
            let (mut registry, calls) = new_registry();
            assert_eq!(
                registry
                    .request_approval("fixture", LifecycleAction::Pause, 100)
                    .unwrap_err()
                    .code,
                "lifecycle_invalid_transition"
            );
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        }
    }

    #[test]
    fn registry_projects_only_adapter_declared_actions() {
        const DECLARED: &[LifecycleAction] = &[LifecycleAction::Start, LifecycleAction::Quarantine];
        let (mut registry, calls) = registry_with_actions(DECLARED);
        let status = registry.status_body("fixture", 100).unwrap();
        let actions = status["操作一覧"].as_array().unwrap();
        assert_eq!(actions.len(), 2);
        assert!(actions.iter().any(|entry| entry["操作"] == "start"));
        assert!(actions.iter().any(|entry| entry["操作"] == "quarantine"));
        assert!(!actions.iter().any(|entry| entry["操作"] == "pause"));
        assert_eq!(
            registry
                .request_approval("fixture", LifecycleAction::Pause, 100)
                .unwrap_err()
                .code,
            "lifecycle_capability_missing"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn verified_quarantine_reservation_restores_terminal_state_and_rejects_malformed_record() {
        let mut audit = BrokerAuditLog::default();
        audit.append(
            "quarantine-reservation",
            "実行系ライフサイクル操作",
            "received",
            &RuntimeLifecycleRegistry::quarantine_reservation_reason("fixture"),
            "INTERNAL_STATE",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        let restored =
            RuntimeLifecycleRegistry::terminal_quarantines_from_verified_audit(audit.events())
                .unwrap();
        assert!(restored.contains("fixture"));

        let calls = Arc::new(AtomicUsize::new(0));
        let mut registry = RuntimeLifecycleRegistry::with_terminal_quarantines(restored);
        let unregistered_status = registry.status_body("fixture", 100).unwrap();
        assert_eq!(unregistered_status["状態"], "quarantined");
        assert!(unregistered_status["操作一覧"].as_array().unwrap().is_empty());
        assert!(unregistered_status["承認一覧"].as_array().unwrap().is_empty());
        registry
            .register_trusted(
                "fixture",
                Arc::new(TestAdapter {
                    calls: Arc::clone(&calls),
                    actions: &LifecycleAction::ALL,
                }),
            )
            .unwrap();
        let status = registry.status_body("fixture", 100).unwrap();
        assert_eq!(status["状態"], "quarantined");
        assert!(status["操作一覧"].as_array().unwrap().is_empty());
        assert!(status["承認一覧"].as_array().unwrap().is_empty());
        assert_eq!(
            registry
                .request_approval("fixture", LifecycleAction::Start, 100)
                .unwrap_err()
                .code,
            "lifecycle_invalid_transition"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        let mut malformed = BrokerAuditLog::default();
        malformed.append(
            "malformed-quarantine-reservation",
            "実行系ライフサイクル操作",
            "received",
            "実行系ライフサイクル隔離予約 実行系ID=bad runtime id",
            "INTERNAL_STATE",
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        assert_eq!(
            RuntimeLifecycleRegistry::terminal_quarantines_from_verified_audit(malformed.events())
                .unwrap_err()
                .code,
            "lifecycle_quarantine_record_invalid"
        );
    }

    #[test]
    fn approval_is_hash_bound_expiring_and_consumed_once() {
        let (mut registry, calls) = new_registry();
        let pending = registry
            .request_approval("fixture", LifecycleAction::Start, 100)
            .unwrap();
        let id = pending["承認ID"].as_str().unwrap().to_string();
        assert_eq!(
            registry.approve(&id, "sha256:bad", 101).unwrap_err().code,
            "lifecycle_approval_hash_mismatch"
        );
        let approved = registry
            .approve(&id, pending["承認hash"].as_str().unwrap(), 101)
            .unwrap();
        assert_eq!(approved["状態"], "approved");
        let mismatched_execution_payload = format!("sha256:{}", "0".repeat(64));
        assert_eq!(
            registry
                .preflight_execution(
                    "fixture",
                    LifecycleAction::Start,
                    &id,
                    &mismatched_execution_payload,
                    102,
                )
                .unwrap_err()
                .code,
            "lifecycle_approval_payload_hash_mismatch"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let transition = registry
            .execute(
                "fixture",
                LifecycleAction::Start,
                &id,
                &execution_hash(LifecycleAction::Start, &approved),
                102,
                102_000,
            )
            .unwrap();
        assert_eq!(transition.next_state, LifecycleState::Ready);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            registry
                .execute(
                    "fixture",
                    LifecycleAction::Start,
                    &id,
                    &execution_hash(LifecycleAction::Start, &approved),
                    103,
                    103_000,
                )
                .unwrap_err()
                .code,
            "lifecycle_invalid_transition"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let restart = registry
            .request_approval("fixture", LifecycleAction::Restart, 104)
            .unwrap();
        let restart_id = restart["承認ID"].as_str().unwrap().to_string();
        registry
            .approve(&restart_id, restart["承認hash"].as_str().unwrap(), 105)
            .unwrap();
        registry
            .execute(
                "fixture",
                LifecycleAction::Restart,
                &restart_id,
                &execution_hash(LifecycleAction::Restart, &restart),
                106,
                106_000,
            )
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            registry
                .execute(
                    "fixture",
                    LifecycleAction::Restart,
                    &restart_id,
                    &execution_hash(LifecycleAction::Restart, &restart),
                    107,
                    107_000,
                )
                .unwrap_err()
                .code,
            "lifecycle_approval_not_approved"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        {
            let (mut registry, calls) = new_registry();
            let pending = registry
                .request_approval("fixture", LifecycleAction::Start, 100)
                .unwrap();
            let id = pending["承認ID"].as_str().unwrap();
            assert_eq!(
                registry
                    .approve(id, pending["承認hash"].as_str().unwrap(), 500)
                    .unwrap_err()
                    .code,
                "lifecycle_approval_expired"
            );
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        }
    }

    #[test]
    fn quarantine_blocks_normal_operations_but_keeps_diagnostic_status() {
        let (mut registry, calls) = new_registry();
        let start = approval(&mut registry, LifecycleAction::Start);
        registry
            .execute(
                "fixture",
                LifecycleAction::Start,
                start["承認ID"].as_str().unwrap(),
                &execution_hash(LifecycleAction::Start, &start),
                102,
                102_000,
            )
            .unwrap();
        let quarantine = approval(&mut registry, LifecycleAction::Quarantine);
        registry
            .execute(
                "fixture",
                LifecycleAction::Quarantine,
                quarantine["承認ID"].as_str().unwrap(),
                &execution_hash(LifecycleAction::Quarantine, &quarantine),
                104,
                104_000,
            )
            .unwrap();
        let status = registry.status_body("fixture", 105).unwrap();
        assert_eq!(status["状態"], "quarantined");
        assert!(status["操作一覧"].as_array().unwrap().is_empty());
        assert!(status["承認一覧"].as_array().unwrap().is_empty());
        assert_eq!(
            registry
                .request_approval("fixture", LifecycleAction::Start, 105)
                .unwrap_err()
                .code,
            "lifecycle_invalid_transition"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
