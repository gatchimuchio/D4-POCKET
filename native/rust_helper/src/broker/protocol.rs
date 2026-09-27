use std::collections::{BTreeMap, HashMap};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use unicode_normalization::UnicodeNormalization;

use std::path::Path;

use crate::audit_hash::sha256_tagged;
use crate::broker::audit::{BrokerAuditEvent, BrokerAuditLog};
use crate::broker::authority::{
    edit_approval, evaluate_authority, evaluate_broker_authority, normalize_inbound_payload,
    project_approval_content, verify_audit_chain, BrokerAuthorityRegistry,
};
use crate::broker::store::{BrokerPersistentStore, BrokerStoreError};
use crate::broker::dialogue::{実行系ID妥当, 対話制御, 対話失敗, 実行系Adapter};
use crate::broker::runtime_lifecycle::{
    LifecycleAction, LifecycleError, RuntimeLifecycleRegistry,
};
#[cfg(test)]
use crate::broker::runtime_lifecycle::LifecycleAdapter;
#[cfg(debug_assertions)]
use crate::broker::runtime_lifecycle::{
    development_fixture_adapter, DEVELOPMENT_LIFECYCLE_RUNTIME_ID,
};
use crate::broker::runtime_registry::{
    ResourceObservationError, RuntimeResourceRegistry,
};
use std::sync::Arc;

#[path = "evaluation_control.rs"]
mod evaluation_control;
#[path = "regression_case.rs"]
mod regression_case;
pub(crate) use regression_case::{
    owner_delete_confirmation_summary, owner_recovery_confirmation_summary,
    owner_registration_confirmation_summary, OwnerDeleteConfirmationSummary,
    OwnerRecoveryConfirmationSummary, OwnerRegistrationConfirmationSummary,
};

const EVIDENCE_SOURCE_LIVE_RUNTIME: &str = "LIVE_RUNTIME";
pub(super) const EVIDENCE_SOURCE_INTERNAL_STATE: &str = "INTERNAL_STATE";
const BROKER_ID: &str = "gui-shell-rust-broker";
const REQUEST_FRESHNESS_WINDOW_SECONDS: u64 = 300;
const ZERO_PAYLOAD_HASH: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetupDoctorReportRequest {
    version: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FirstRunConfigurationRequest {
    version: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 実行系資源観測指定 {
    #[serde(rename = "版")]
    version: u8,
    #[serde(rename = "実行系ID")]
    runtime_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 実行系ライフサイクル状態指定 {
    #[serde(rename = "版")]
    version: u8,
    #[serde(rename = "実行系ID")]
    runtime_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 実行系ライフサイクル承認要求指定 {
    #[serde(rename = "版")]
    version: u8,
    #[serde(rename = "実行系ID")]
    runtime_id: String,
    #[serde(rename = "操作")]
    action: LifecycleAction,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 実行系ライフサイクル承認指定 {
    #[serde(rename = "版")]
    version: u8,
    #[serde(rename = "承認ID")]
    approval_id: String,
    #[serde(rename = "承認hash")]
    approval_hash: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 実行系ライフサイクル操作指定 {
    #[serde(rename = "版")]
    version: u8,
    #[serde(rename = "実行系ID")]
    runtime_id: String,
    #[serde(rename = "操作")]
    action: LifecycleAction,
    #[serde(rename = "承認ID")]
    approval_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 全Runtime停止要求指定 {
    #[serde(rename = "版")]
    version: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokerPersistenceMode {
    InMemorySkeleton,
    PersistentRequiredUnavailable,
    DurableFileStore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerStateStore {
    mode: BrokerPersistenceMode,
    persistent_store: Option<BrokerPersistentStore>,
}

impl BrokerStateStore {
    pub fn in_memory_skeleton() -> Self {
        Self {
            mode: BrokerPersistenceMode::InMemorySkeleton,
            persistent_store: None,
        }
    }

    pub fn persistent_required_unavailable() -> Self {
        Self {
            mode: BrokerPersistenceMode::PersistentRequiredUnavailable,
            persistent_store: None,
        }
    }

    pub fn durable_file_store(store: BrokerPersistentStore) -> Self {
        Self {
            mode: BrokerPersistenceMode::DurableFileStore,
            persistent_store: Some(store),
        }
    }

    pub fn persistence_required(&self) -> bool {
        matches!(
            self.mode,
            BrokerPersistenceMode::PersistentRequiredUnavailable
                | BrokerPersistenceMode::DurableFileStore
        )
    }

    pub fn persistence_ready(&self) -> bool {
        matches!(self.mode, BrokerPersistenceMode::DurableFileStore)
    }

    pub fn health_status(&self) -> &'static str {
        if self.persistence_required() && !self.persistence_ready() {
            "suspend"
        } else {
            "ready"
        }
    }

    pub fn audit_persistence(&self) -> &'static str {
        if self.persistence_ready() {
            "durable_file_store"
        } else {
            "in_memory_skeleton"
        }
    }

    pub fn replay_persistence(&self) -> &'static str {
        if self.persistence_ready() {
            "durable_file_store"
        } else {
            "in_memory_session_only"
        }
    }

    pub fn session_persistence(&self) -> &'static str {
        if self.persistence_ready() {
            "durable_file_store"
        } else {
            "in_memory_session_only"
        }
    }

    pub fn unavailable_message(&self) -> &'static str {
        "persistent audit, replay, and session state are required but unavailable"
    }

    pub fn append_audit_event(&self, event: &BrokerAuditEvent) -> Result<(), BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            store.append_audit_event(event)?;
        }
        Ok(())
    }

    pub fn append_replay_nonce(
        &self,
        nonce: &str,
        recorded_at_epoch_seconds: i64,
    ) -> Result<Option<HashMap<String, i64>>, BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            return store
                .append_replay_nonce(nonce, recorded_at_epoch_seconds)
                .map(Some);
        }
        Ok(None)
    }

    pub fn load_profile_state(&self) -> Result<Option<serde_json::Value>, BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            return store.load_profile_state().map(Some);
        }
        Ok(None)
    }

    pub fn write_profile_state(&self, state: &serde_json::Value) -> Result<(), BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            store.write_profile_state(state)?;
        }
        Ok(())
    }

    pub fn load_update_state(&self) -> Result<Option<serde_json::Value>, BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            return store.load_update_state().map(Some);
        }
        Ok(None)
    }

    pub fn write_update_state(&self, state: &serde_json::Value) -> Result<(), BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            store.write_update_state(state)?;
        }
        Ok(())
    }

    pub fn load_notification_state(&self) -> Result<Option<serde_json::Value>, BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            return store.load_notification_state().map(Some);
        }
        Ok(None)
    }

    pub fn write_notification_state(
        &self,
        state: &serde_json::Value,
    ) -> Result<(), BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            store.write_notification_state(state)?;
        }
        Ok(())
    }

    pub fn load_a2a_state(&self) -> Result<Option<serde_json::Value>, BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            return store.load_a2a_state().map(Some);
        }
        Ok(None)
    }

    pub fn write_a2a_state(&self, state: &serde_json::Value) -> Result<(), BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            store.write_a2a_state(state)?;
        }
        Ok(())
    }

    pub fn load_host_state(&self) -> Result<Option<serde_json::Value>, BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            return store.load_host_state().map(Some);
        }
        Ok(None)
    }

    pub fn write_host_state(&self, state: &serde_json::Value) -> Result<(), BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            store.write_host_state(state)?;
        }
        Ok(())
    }

    pub fn load_adapter_state(&self) -> Result<Option<serde_json::Value>, BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            return store.load_adapter_state().map(Some);
        }
        Ok(None)
    }

    pub fn write_adapter_state(&self, state: &serde_json::Value) -> Result<(), BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            store.write_adapter_state(state)?;
        }
        Ok(())
    }

    pub fn write_setup_doctor_report(&self, bytes: &[u8]) -> Result<(), BrokerStoreError> {
        let Some(store) = &self.persistent_store else {
            return Err(BrokerStoreError::Io(
                "Setup Doctor reportには永続Broker storeが必要".to_string(),
            ));
        };
        store.write_setup_doctor_report(bytes)
    }

    fn initialize_first_run_configuration(
        &self,
    ) -> Result<(Vec<u8>, bool), BrokerStoreError> {
        let Some(store) = &self.persistent_store else {
            return Err(BrokerStoreError::Io(
                "初回設定の生成には永続Broker storeが必要".to_string(),
            ));
        };
        store.initialize_first_run_configuration()
    }

    pub fn load_update_trust(&self) -> Result<Option<serde_json::Value>, BrokerStoreError> {
        if let Some(store) = &self.persistent_store {
            return store.load_update_trust();
        }
        Ok(None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerOperation {
    #[serde(rename = "Setup Doctor報告取得")]
    SetupDoctor報告取得,
    #[serde(rename = "初回設定取得")]
    初回設定取得,
    #[serde(rename = "対話履歴承認")]
    対話履歴承認,
    #[serde(rename = "対話履歴失効")]
    対話履歴失効,
    #[serde(rename = "対話履歴閲覧状態")]
    対話履歴閲覧状態,
    #[serde(rename = "対話履歴閲覧")]
    対話履歴閲覧,
    #[serde(rename = "対話再実行")]
    対話再実行,
    #[serde(rename = "対話分岐")]
    対話分岐,
    #[serde(rename = "対話履歴一覧")]
    対話履歴一覧,
    #[serde(rename = "作業領域一覧")]
    作業領域一覧,
    #[serde(rename = "作業領域承認")]
    作業領域承認,
    #[serde(rename = "作業領域失効")]
    作業領域失効,
    #[serde(rename = "作業領域基準点保存")]
    作業領域基準点保存,
    #[serde(rename = "作業領域差分")]
    作業領域差分,
    #[serde(rename = "作業領域比較範囲")]
    作業領域比較範囲,
    #[serde(rename = "作業領域全体基準点保存")]
    作業領域全体基準点保存,
    #[serde(rename = "作業領域変更一覧")]
    作業領域変更一覧,
    #[serde(rename = "作業領域復旧プレビュー")]
    作業領域復旧プレビュー,
    #[serde(rename = "作業領域ツリー")]
    作業領域ツリー,
    #[serde(rename = "作業領域読取")]
    作業領域読取,

    #[serde(rename = "端末招待")]
    端末招待,
    #[serde(rename = "端末一覧")]
    端末一覧,
    #[serde(rename = "端末招待取消")]
    端末招待取消,
    #[serde(rename = "端末失効")]
    端末失効,
    #[serde(rename = "ホスト能力")]
    ホスト能力,
    Health,
    Shutdown,
    CommandEnvelope,
    AuthorityEvaluate,
    AuthorityFixtureEvaluate,
    ApprovalEdit,
    ContentProjection,
    AuditVerify,
    NormalizePayload,
    #[serde(rename = "実行系列挙")]
    実行系列挙,
    #[serde(rename = "対話セッション一覧")]
    対話セッション一覧,
    #[serde(rename = "Agent一覧")]
    Agent一覧,
    #[serde(rename = "Agent作業要求検査")]
    Agent作業要求検査,
    #[serde(rename = "AgentTaskWorkspacePermissionGrant")]
    AgentTaskWorkspacePermissionGrant,
    #[serde(rename = "AgentTaskOwnerApprovalGrant")]
    AgentTaskOwnerApprovalGrant,
    #[serde(rename = "AgentTask実行")]
    AgentTask実行,
    #[serde(rename = "AgentTask状態")]
    AgentTask状態,
    #[serde(rename = "AgentTask取消")]
    AgentTask取消,
    #[serde(rename = "評価Dataset登録")]
    評価Dataset登録,
    #[serde(rename = "回帰Case登録")]
    回帰Case登録,
    #[serde(rename = "回帰Case一覧")]
    回帰Case一覧,
    #[serde(rename = "回帰Case削除")]
    回帰Case削除,
    #[serde(rename = "回帰Case削除中断確認")]
    回帰Case削除中断確認,
    #[serde(rename = "資格情報登録")]
    資格情報登録,
    #[serde(rename = "資格情報一覧")]
    資格情報一覧,
    #[serde(rename = "MCP接続")]
    MCP接続,
    #[serde(rename = "MCP接続一覧")]
    MCP接続一覧,
    #[serde(rename = "A2A接続")]
    A2A接続,
    #[serde(rename = "A2A接続一覧")]
    A2A接続一覧,
    #[serde(rename = "Host登録")]
    Host登録,
    #[serde(rename = "Host一覧")]
    Host一覧,
    #[serde(rename = "Host切替")]
    Host切替,
    #[serde(rename = "アダプター一覧")]
    アダプター一覧,
    #[serde(rename = "アダプター導入")]
    アダプター導入,
    #[serde(rename = "アダプター検証")]
    アダプター検証,
    #[serde(rename = "アダプター有効化")]
    アダプター有効化,
    #[serde(rename = "アダプター無効化")]
    アダプター無効化,
    #[serde(rename = "アダプター隔離")]
    アダプター隔離,
    #[serde(rename = "アダプター更新")]
    アダプター更新,
    #[serde(rename = "アダプター削除")]
    アダプター削除,
    #[serde(rename = "プロファイル作成")]
    プロファイル作成,
    #[serde(rename = "プロファイル複製")]
    プロファイル複製,
    #[serde(rename = "プロファイル適用要求")]
    プロファイル適用要求,
    #[serde(rename = "プロファイル削除")]
    プロファイル削除,
    #[serde(rename = "プロファイルexport")]
    プロファイルexport,
    #[serde(rename = "プロファイルimport")]
    プロファイルimport,
    #[serde(rename = "プロファイル一覧")]
    プロファイル一覧,
    #[serde(rename = "GUI Shell構成")]
    GuiShell構成,
    #[serde(rename = "GUI Shell構成Preview")]
    GuiShell構成Preview,
    #[serde(rename = "GUI Shell編集提案")]
    GuiShell編集提案,
    #[serde(rename = "GUI Shell書出し")]
    GuiShell書出し,
    #[serde(rename = "更新一覧")]
    更新一覧,
    #[serde(rename = "更新確認")]
    更新確認,
    #[serde(rename = "更新署名検査")]
    更新署名検査,
    #[serde(rename = "更新download要求")]
    更新download要求,
    #[serde(rename = "更新適用要求")]
    更新適用要求,
    #[serde(rename = "更新延期")]
    更新延期,
    #[serde(rename = "更新rollback要求")]
    更新rollback要求,
    #[serde(rename = "通知一覧")]
    通知一覧,
    #[serde(rename = "通知既読")]
    通知既読,
    #[serde(rename = "通知破棄")]
    通知破棄,
    #[serde(rename = "通知全既読")]
    通知全既読,
    #[serde(rename = "観測一覧")]
    観測一覧,
    #[serde(rename = "評価Dataset一覧")]
    評価Dataset一覧,
    #[serde(rename = "評価実験開始")]
    評価実験開始,
    #[serde(rename = "評価実験状態")]
    評価実験状態,
    #[serde(rename = "評価比較")]
    評価比較,
    #[serde(rename = "実行系資源観測")]
    実行系資源観測,
    #[serde(rename = "実行系ライフサイクル状態")]
    実行系ライフサイクル状態,
    #[serde(rename = "実行系ライフサイクル承認要求")]
    実行系ライフサイクル承認要求,
    #[serde(rename = "実行系ライフサイクル承認")]
    実行系ライフサイクル承認,
    #[serde(rename = "実行系ライフサイクル操作")]
    実行系ライフサイクル操作,
    #[serde(rename = "全Runtime停止要求")]
    全Runtime停止要求,
    #[serde(rename = "対話開始")]
    対話開始,
    #[serde(rename = "対話送信")]
    対話送信,
    #[serde(rename = "対話取得")]
    対話取得,
    #[serde(rename = "対話中止")]
    対話中止,
    #[serde(rename = "対話終了")]
    対話終了,
    #[serde(rename = "対話承認")]
    対話承認,
    #[serde(rename = "対話承認待ち")]
    対話承認待ち,
    #[serde(rename = "対話部分保存破棄")]
    対話部分保存破棄,
    #[serde(rename = "対話部分破棄中断確認")]
    対話部分破棄中断確認,
    #[serde(rename = "対話削除中断確認")]
    対話削除中断確認,
    #[serde(rename = "対話保管状態")]
    対話保管状態,
    #[serde(rename = "対話内容削除")]
    対話内容削除,
    #[serde(rename = "対話内容保存")]
    対話内容保存,
    #[serde(rename = "対話内容承認")]
    対話内容承認,
    #[serde(rename = "対話内容失効")]
    対話内容失効,
    #[serde(rename = "対話内容閲覧状態")]
    対話内容閲覧状態,
    #[serde(rename = "対話内容閲覧")]
    対話内容閲覧,


}

impl BrokerOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            BrokerOperation::SetupDoctor報告取得 => "Setup Doctor報告取得",
            BrokerOperation::初回設定取得 => "初回設定取得",
            BrokerOperation::作業領域一覧 => "作業領域一覧",
            BrokerOperation::作業領域承認 => "作業領域承認",
            BrokerOperation::作業領域失効 => "作業領域失効",
            BrokerOperation::作業領域基準点保存 => "作業領域基準点保存",
            BrokerOperation::作業領域差分 => "作業領域差分",
            BrokerOperation::作業領域比較範囲 => "作業領域比較範囲",
            BrokerOperation::作業領域全体基準点保存 => "作業領域全体基準点保存",
            BrokerOperation::作業領域変更一覧 => "作業領域変更一覧",
            BrokerOperation::作業領域復旧プレビュー => "作業領域復旧プレビュー",
            BrokerOperation::作業領域ツリー => "作業領域ツリー",
            BrokerOperation::作業領域読取 => "作業領域読取",
            BrokerOperation::端末招待 => "端末招待",
            BrokerOperation::端末一覧 => "端末一覧",
            BrokerOperation::端末招待取消 => "端末招待取消",
            BrokerOperation::端末失効 => "端末失効",
            BrokerOperation::ホスト能力 => "ホスト能力",
            BrokerOperation::Health => "health",
            BrokerOperation::Shutdown => "shutdown",
            BrokerOperation::CommandEnvelope => "command_envelope",
            BrokerOperation::AuthorityEvaluate => "authority_evaluate",
            BrokerOperation::AuthorityFixtureEvaluate => "authority_fixture_evaluate",
            BrokerOperation::ApprovalEdit => "approval_edit",
            BrokerOperation::ContentProjection => "content_projection",
            BrokerOperation::AuditVerify => "audit_verify",
            BrokerOperation::対話履歴承認 => "対話履歴承認",
            BrokerOperation::対話履歴失効 => "対話履歴失効",
            BrokerOperation::対話履歴閲覧状態 => "対話履歴閲覧状態",
            BrokerOperation::対話履歴閲覧 => "対話履歴閲覧",
            BrokerOperation::対話再実行 => "対話再実行",
            BrokerOperation::対話分岐 => "対話分岐",
            BrokerOperation::対話履歴一覧 => "対話履歴一覧",
            BrokerOperation::NormalizePayload => "normalize_payload",
            BrokerOperation::実行系列挙 => "実行系列挙",
            BrokerOperation::対話セッション一覧 => "対話セッション一覧",
            BrokerOperation::Agent一覧 => "Agent一覧",
            BrokerOperation::Agent作業要求検査 => "Agent作業要求検査",
            BrokerOperation::AgentTaskWorkspacePermissionGrant => "AgentTaskWorkspacePermissionGrant",
            BrokerOperation::AgentTaskOwnerApprovalGrant => "AgentTaskOwnerApprovalGrant",
            BrokerOperation::AgentTask実行 => "AgentTask実行",
            BrokerOperation::AgentTask状態 => "AgentTask状態",
            BrokerOperation::AgentTask取消 => "AgentTask取消",
            BrokerOperation::評価Dataset登録 => "評価Dataset登録",
            BrokerOperation::回帰Case登録 => "回帰Case登録",
            BrokerOperation::回帰Case一覧 => "回帰Case一覧",
            BrokerOperation::回帰Case削除 => "回帰Case削除",
            BrokerOperation::回帰Case削除中断確認 => "回帰Case削除中断確認",
            BrokerOperation::資格情報登録 => "資格情報登録",
            BrokerOperation::資格情報一覧 => "資格情報一覧",
            BrokerOperation::MCP接続 => "MCP接続",
            BrokerOperation::MCP接続一覧 => "MCP接続一覧",
            BrokerOperation::A2A接続 => "A2A接続",
            BrokerOperation::A2A接続一覧 => "A2A接続一覧",
            BrokerOperation::Host登録 => "Host登録",
            BrokerOperation::Host一覧 => "Host一覧",
            BrokerOperation::Host切替 => "Host切替",
            BrokerOperation::アダプター一覧 => "アダプター一覧",
            BrokerOperation::アダプター導入 => "アダプター導入",
            BrokerOperation::アダプター検証 => "アダプター検証",
            BrokerOperation::アダプター有効化 => "アダプター有効化",
            BrokerOperation::アダプター無効化 => "アダプター無効化",
            BrokerOperation::アダプター隔離 => "アダプター隔離",
            BrokerOperation::アダプター更新 => "アダプター更新",
            BrokerOperation::アダプター削除 => "アダプター削除",
            BrokerOperation::プロファイル作成 => "プロファイル作成",
            BrokerOperation::プロファイル複製 => "プロファイル複製",
            BrokerOperation::プロファイル適用要求 => "プロファイル適用要求",
            BrokerOperation::プロファイル削除 => "プロファイル削除",
            BrokerOperation::プロファイルexport => "プロファイルexport",
            BrokerOperation::プロファイルimport => "プロファイルimport",
            BrokerOperation::プロファイル一覧 => "プロファイル一覧",
            BrokerOperation::GuiShell構成 => "GUI Shell構成",
            BrokerOperation::GuiShell構成Preview => "GUI Shell構成Preview",
            BrokerOperation::GuiShell編集提案 => "GUI Shell編集提案",
            BrokerOperation::GuiShell書出し => "GUI Shell書出し",
            BrokerOperation::更新一覧 => "更新一覧",
            BrokerOperation::更新確認 => "更新確認",
            BrokerOperation::更新署名検査 => "更新署名検査",
            BrokerOperation::更新download要求 => "更新download要求",
            BrokerOperation::更新適用要求 => "更新適用要求",
            BrokerOperation::更新延期 => "更新延期",
            BrokerOperation::更新rollback要求 => "更新rollback要求",
            BrokerOperation::通知一覧 => "通知一覧",
            BrokerOperation::通知既読 => "通知既読",
            BrokerOperation::通知破棄 => "通知破棄",
            BrokerOperation::通知全既読 => "通知全既読",
            BrokerOperation::観測一覧 => "観測一覧",
            BrokerOperation::評価Dataset一覧 => "評価Dataset一覧",
            BrokerOperation::評価実験開始 => "評価実験開始",
            BrokerOperation::評価実験状態 => "評価実験状態",
            BrokerOperation::評価比較 => "評価比較",
            BrokerOperation::実行系資源観測 => "実行系資源観測",
            BrokerOperation::実行系ライフサイクル状態 => "実行系ライフサイクル状態",
            BrokerOperation::実行系ライフサイクル承認要求 => "実行系ライフサイクル承認要求",
            BrokerOperation::実行系ライフサイクル承認 => "実行系ライフサイクル承認",
            BrokerOperation::実行系ライフサイクル操作 => "実行系ライフサイクル操作",
            BrokerOperation::全Runtime停止要求 => "全Runtime停止要求",
            BrokerOperation::対話開始 => "対話開始",
            BrokerOperation::対話送信 => "対話送信",
            BrokerOperation::対話取得 => "対話取得",
            BrokerOperation::対話中止 => "対話中止",
            BrokerOperation::対話終了 => "対話終了",
            BrokerOperation::対話承認 => "対話承認",
            BrokerOperation::対話承認待ち => "対話承認待ち",
            BrokerOperation::対話部分保存破棄 => "対話部分保存破棄",
            BrokerOperation::対話削除中断確認 => "対話削除中断確認",
            BrokerOperation::対話部分破棄中断確認 => "対話部分破棄中断確認",
            BrokerOperation::対話保管状態 => "対話保管状態",
            BrokerOperation::対話内容削除 => "対話内容削除",
            BrokerOperation::対話内容保存 => "対話内容保存",
            BrokerOperation::対話内容承認 => "対話内容承認",
            BrokerOperation::対話内容失効 => "対話内容失効",
            BrokerOperation::対話内容閲覧状態 => "対話内容閲覧状態",
            BrokerOperation::対話内容閲覧 => "対話内容閲覧",


        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerMetadata {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerRequestEnvelope {
    pub request_id: Option<String>,
    pub session_id: Option<String>,
    pub operation: Option<BrokerOperation>,
    pub payload_hash: Option<String>,
    pub nonce: Option<String>,
    pub issued_at: Option<String>,
    pub metadata: Vec<BrokerMetadata>,
    pub metadata_present: bool,
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnerConfirmationSource {
    NotOwner,
    OwnerCredential,
    DesktopNativeConfirmation,
}

impl BrokerRequestEnvelope {
    pub fn from_json_str(input: &str) -> Result<Self, serde_json::Error> {
        let raw: JsonRequestEnvelope = super::json_input::read_unique(input)?;
        let metadata_present = raw.metadata.is_some();
        let metadata = raw
            .metadata
            .unwrap_or_default()
            .into_iter()
            .map(|(key, value)| BrokerMetadata {
                key,
                value: json_metadata_value(&value),
            })
            .collect();
        Ok(Self {
            request_id: raw.request_id,
            session_id: raw.session_id,
            operation: raw.operation,
            payload_hash: raw.payload_hash,
            nonce: raw.nonce,
            issued_at: raw.issued_at,
            metadata,
            metadata_present,
            payload: raw.payload,
        })
    }

    pub fn health(request_id: &str, nonce: &str) -> Self {
        Self::health_at(request_id, nonce, "2026-06-01T00:00:00Z")
    }

    pub fn health_at(request_id: &str, nonce: &str, issued_at: &str) -> Self {
        Self {
            request_id: Some(request_id.to_string()),
            session_id: None,
            operation: Some(BrokerOperation::Health),
            payload_hash: Some(canonical_payload_hash(None)),
            nonce: Some(nonce.to_string()),
            issued_at: Some(issued_at.to_string()),
            metadata: vec![],
            metadata_present: true,
            payload: None,
        }
    }

    pub fn shutdown(request_id: &str, session_id: &str, nonce: &str) -> Self {
        Self::shutdown_at(request_id, session_id, nonce, "2026-06-01T00:00:00Z")
    }

    pub fn shutdown_at(request_id: &str, session_id: &str, nonce: &str, issued_at: &str) -> Self {
        Self {
            request_id: Some(request_id.to_string()),
            session_id: Some(session_id.to_string()),
            operation: Some(BrokerOperation::Shutdown),
            payload_hash: Some(canonical_payload_hash(None)),
            nonce: Some(nonce.to_string()),
            issued_at: Some(issued_at.to_string()),
            metadata: vec![],
            metadata_present: true,
            payload: None,
        }
    }

    pub fn command_envelope(request_id: &str, session_id: &str, nonce: &str) -> Self {
        Self::command_envelope_at(request_id, session_id, nonce, "2026-06-01T00:00:00Z")
    }

    pub fn command_envelope_at(
        request_id: &str,
        session_id: &str,
        nonce: &str,
        issued_at: &str,
    ) -> Self {
        Self {
            request_id: Some(request_id.to_string()),
            session_id: Some(session_id.to_string()),
            operation: Some(BrokerOperation::CommandEnvelope),
            payload_hash: Some(canonical_payload_hash(None)),
            nonce: Some(nonce.to_string()),
            issued_at: Some(issued_at.to_string()),
            metadata: vec![],
            metadata_present: true,
            payload: None,
        }
    }

    pub fn current_issued_at() -> String {
        epoch_seconds_to_rfc3339(current_epoch_seconds())
    }

    pub fn refresh_payload_hash(&mut self) {
        self.payload_hash = Some(canonical_payload_hash(self.payload.as_ref()));
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonRequestEnvelope {
    request_id: Option<String>,
    session_id: Option<String>,
    operation: Option<BrokerOperation>,
    payload_hash: Option<String>,
    nonce: Option<String>,
    issued_at: Option<String>,
    metadata: Option<BTreeMap<String, Value>>,
    payload: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BrokerHealth {
    pub broker_id: String,
    pub status: String,
    pub boundary_role: String,
    pub authority_cutover_status: String,
    pub command_dispatch_enabled: bool,
    pub audit_append_enabled: bool,
    pub audit_persistence: String,
    pub replay_persistence: String,
    pub session_persistence: String,
    pub persistence_required: bool,
    pub persistence_ready: bool,
    pub evidence_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BrokerError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
    pub audit_event_required: bool,
    pub fail_closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerStatus {
    Accepted,
    Rejected,
    Suspended,
}

impl BrokerStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BrokerStatus::Accepted => "accepted",
            BrokerStatus::Rejected => "rejected",
            BrokerStatus::Suspended => "suspended",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BrokerResponse {
    pub request_id: String,
    pub operation: String,
    pub status: BrokerStatus,
    pub evidence_source: String,
    pub audit_event_id: String,
    pub error: Option<BrokerError>,
    pub health: Option<BrokerHealth>,
    pub body: Option<Value>,
    pub shutdown_requested: bool,
}

impl BrokerResponse {
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[derive(Debug)]
pub struct Broker {
    session_id: String,
    seen_nonces: HashMap<String, i64>,
    pub(super) audit_log: BrokerAuditLog,
    authority_registry: BrokerAuthorityRegistry,
    対話: 対話制御,
    資源観測: RuntimeResourceRegistry,
    ライフサイクル: RuntimeLifecycleRegistry,
    評価: evaluation_control::EvaluationControl,
    履歴閲覧: super::history_access::HistoryAccess,
    作業領域: super::workspace::WorkspaceRegistry,
    端末: Option<super::device_link::端末制御>,
    pub(super) mcp_connections: BTreeMap<String, super::mcp_center::McpConnectionEntry>,
    pub(super) a2a_connections: BTreeMap<String, Value>,
    pub(super) hosts: BTreeMap<String, Value>,
    pub(super) adapters: BTreeMap<String, Value>,
    pub(super) profiles: BTreeMap<String, Value>,
    pub(super) updates: BTreeMap<String, Value>,
    pub(super) update_trust: Option<super::update_center::UpdateTrust>,
    pub(super) notification_states:
        BTreeMap<String, super::notification_center::NotificationState>,
    pub(super) observations: super::observation_center::ObservationCenter,
    #[cfg(windows)]
    pub(super) protected_store: Option<crate::protected_store::ProtectedStore>,
    #[cfg(windows)]
    内容閲覧: super::content_access::ContentAccess,
    pub(super) shutdown_requested: bool,
    pub(super) current_epoch_seconds_override: Option<i64>,
    pub(super) state_store: BrokerStateStore,
    agent_task_scratch: Option<super::agent_task_scratch::AgentTaskScratchJournal>,
    pub(super) desktop_export_root: Option<(std::path::PathBuf, cap_std::fs::Dir)>,
    desktop_install_path_verified: bool,
    desktop_loopback_bind_verified: bool,
    desktop_first_run_configuration: Option<(Value, Vec<u8>)>,
}

impl Broker {
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            seen_nonces: HashMap::new(),
            audit_log: BrokerAuditLog::default(),
            authority_registry: BrokerAuthorityRegistry::production_default(),
            対話: 対話制御::default(),
            資源観測: RuntimeResourceRegistry::default(),
            ライフサイクル: RuntimeLifecycleRegistry::default(),
            評価: Default::default(),
            履歴閲覧: Default::default(),
            作業領域: super::workspace::WorkspaceRegistry::default(),
            端末: None,
            mcp_connections: BTreeMap::new(),
            a2a_connections: BTreeMap::new(),
            hosts: BTreeMap::new(),
            adapters: BTreeMap::new(),
            profiles: BTreeMap::new(),
            updates: BTreeMap::new(),
            update_trust: None,
            notification_states: BTreeMap::new(),
            observations: super::observation_center::ObservationCenter::default(),
            #[cfg(windows)]
            protected_store: None,
            #[cfg(windows)]
            内容閲覧: Default::default(),
            shutdown_requested: false,
            current_epoch_seconds_override: None,
            state_store: BrokerStateStore::in_memory_skeleton(),
            agent_task_scratch: None,
            desktop_export_root: None,
            desktop_install_path_verified: false,
            desktop_loopback_bind_verified: false,
            desktop_first_run_configuration: None,
        }
    }

    pub fn new_with_current_epoch_seconds(session_id: &str, current_epoch_seconds: i64) -> Self {
        let mut broker = Self::new(session_id);
        broker.current_epoch_seconds_override = Some(current_epoch_seconds);
        broker
    }

    pub fn new_requiring_persistence(session_id: &str) -> Self {
        let mut broker = Self::new(session_id);
        broker.state_store = BrokerStateStore::persistent_required_unavailable();
        broker
    }

    pub fn new_persistent(
        session_id: &str,
        store_root: impl AsRef<Path>,
    ) -> Result<Self, BrokerStoreError> {
        let (persistent_store, persistent_state) =
            BrokerPersistentStore::open_or_create(store_root, session_id)?;
        let agent_task_scratch =
            super::agent_task_scratch::AgentTaskScratchJournal::open(persistent_store.clone())?;
        let profiles = super::profile_center::load_persistent_profiles(&persistent_store)?;
        let updates = super::update_center::load_persistent_updates(&persistent_store)?;
        let update_trust = super::update_center::load_persistent_trust(&persistent_store)?;
        let notification_states =
            super::notification_center::load_persistent_states(&persistent_store)?;
        let a2a_connections =
            super::a2a_center::load_persistent_connections(&persistent_store)?;
        let hosts = super::host_center::load_persistent_hosts(&persistent_store)?;
        let adapters = super::adapter_center::load_persistent_adapters(&persistent_store)?;
        let terminal_quarantines = RuntimeLifecycleRegistry::terminal_quarantines_from_verified_audit(
            persistent_state.audit_log.events(),
        )
        .map_err(|error| BrokerStoreError::TamperedAuditState(error.message))?;
        Ok(Self {
            session_id: session_id.to_string(),
            seen_nonces: persistent_state.seen_nonces,
            audit_log: persistent_state.audit_log,
            authority_registry: BrokerAuthorityRegistry::production_default(),
            対話: 対話制御::default(),
            資源観測: RuntimeResourceRegistry::default(),
            ライフサイクル: RuntimeLifecycleRegistry::with_terminal_quarantines(
                terminal_quarantines,
            ),
            評価: Default::default(),
            履歴閲覧: Default::default(),
            作業領域: super::workspace::WorkspaceRegistry::default(),
            端末: None,
            mcp_connections: BTreeMap::new(),
            a2a_connections,
            hosts,
            adapters,
            profiles,
            updates,
            update_trust,
            notification_states,
            observations: super::observation_center::ObservationCenter::default(),
            #[cfg(windows)]
            protected_store: None,
            #[cfg(windows)]
            内容閲覧: Default::default(),
            shutdown_requested: false,
            current_epoch_seconds_override: None,
            state_store: BrokerStateStore::durable_file_store(persistent_store),
            agent_task_scratch: Some(agent_task_scratch),
            desktop_export_root: None,
            desktop_install_path_verified: false,
            desktop_loopback_bind_verified: false,
            desktop_first_run_configuration: None,
        })
    }

    pub(crate) fn set_desktop_setup_doctor_runtime_evidence(
        &mut self,
        installed_path_verified: bool,
        loopback_bind_verified: bool,
    ) {
        self.desktop_install_path_verified = installed_path_verified;
        self.desktop_loopback_bind_verified = loopback_bind_verified;
    }

    pub(crate) fn set_desktop_export_root(
        &mut self,
        path: std::path::PathBuf,
        root: cap_std::fs::Dir,
    ) {
        self.desktop_export_root = Some((path, root));
    }

    pub(crate) fn initialize_desktop_first_run_configuration(
        &mut self,
    ) -> Result<(), BrokerStoreError> {
        if !self.desktop_install_path_verified {
            return Err(BrokerStoreError::MalformedFirstRunConfiguration(
                "installed package配置の検証がない".to_string(),
            ));
        }
        let operation = "D4 Pocket初回設定生成";
        let request_hash = sha256_tagged(b"d4-pocket-first-run-configuration:v1");
        let receipt_reason = "Capability=製品初回設定 Permission=固定store内の初期設定一fileをcreate-onlyで生成・読取 Approval=非権限の固定既定値初期化のため不要 RecoveryAction=既存fileを保持し固定storeの破損原因を確認";
        self.append_audit(
            &format!("desktop-first-run:{}:received", self.session_id),
            operation,
            "received",
            receipt_reason,
            EVIDENCE_SOURCE_LIVE_RUNTIME,
            &request_hash,
        )?;

        let (bytes, created) = match self.state_store.initialize_first_run_configuration() {
            Ok(result) => result,
            Err(error) => {
                self.append_audit(
                    &format!("desktop-first-run:{}:rejected", self.session_id),
                    operation,
                    "rejected",
                    "first_run_configuration_unavailable RecoveryAction=既存fileを保全して固定storeを確認",
                    EVIDENCE_SOURCE_LIVE_RUNTIME,
                    &request_hash,
                )?;
                return Err(error);
            }
        };
        let configuration: Value = serde_json::from_slice(&bytes).map_err(|_| {
            BrokerStoreError::MalformedFirstRunConfiguration(
                "初回設定のJSON検証に失敗".to_string(),
            )
        })?;
        let content_hash = sha256_tagged(&bytes);
        let accepted_reason = if created {
            "first_run_configuration_created Capability=製品初回設定 Permission=固定store内の初期設定一fileをcreate-onlyで作成 Approval=不要 RecoveryAction=固定storeを確認"
        } else {
            "first_run_configuration_existing_validated Capability=製品初回設定 Permission=固定store内の初期設定一fileを読取 Approval=不要 RecoveryAction=固定storeを確認"
        };
        self.append_audit(
            &format!("desktop-first-run:{}:accepted", self.session_id),
            operation,
            "accepted",
            accepted_reason,
            EVIDENCE_SOURCE_LIVE_RUNTIME,
            &content_hash,
        )?;
        self.desktop_first_run_configuration = Some((configuration, bytes));
        Ok(())
    }

    pub fn handle(&mut self, envelope: BrokerRequestEnvelope) -> BrokerResponse {
        self.処理(envelope, false)
    }

    pub fn 実行系登録(&mut self, 名前: &str, adapter: Arc<dyn 実行系Adapter>) -> Result<(), 対話失敗> {
        if self.ライフサイクル.is_terminally_quarantined(名前)
            || super::adapter_center::runtime_is_quarantined(self, 名前)
        {
            return Err(対話失敗::隔離済み);
        }
        let 観測対象 = adapter.観測対象();
        self.対話.登録(名前, adapter)?;
        let 登録監査 = match self.append_audit(
            名前,
            "実行系資源登録",
            "recorded",
            "実行系資源観測はloopback listener所有processへ限定して登録",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &sha256_tagged(名前.as_bytes()),
        ) {
            Ok(event) => event,
            Err(_) => {
                self.対話.直後登録取消(名前);
                return Err(対話失敗::監査失敗);
            }
        };
        self.資源観測.register(
            名前,
            観測対象,
            self.current_epoch_millis(),
            登録監査.event_id,
        );
        Ok(())
    }

    /// debug buildで明示opt-inされた固定fixtureだけを登録する。
    /// MINIDORAや通常の対話Adapterへlifecycle capabilityを付与しない。
    #[cfg(debug_assertions)]
    pub(crate) fn 開発用ライフサイクル実行系登録(&mut self) -> Result<(), String> {
        let adapter = development_fixture_adapter().map_err(|error| error.message)?;
        self.ライフサイクル
            .register_trusted(DEVELOPMENT_LIFECYCLE_RUNTIME_ID, Arc::new(adapter))
            .map_err(|error| error.message)?;
        let registration_hash = sha256_tagged(DEVELOPMENT_LIFECYCLE_RUNTIME_ID.as_bytes());
        if let Err(error) = self.append_audit(
            DEVELOPMENT_LIFECYCLE_RUNTIME_ID,
            "実行系ライフサイクル登録",
            "recorded",
            "development固定fixtureだけをBroker所有lifecycle adapterとして登録",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            &registration_hash,
        ) {
            self.ライフサイクル
                .unregister_and_fail_closed(DEVELOPMENT_LIFECYCLE_RUNTIME_ID);
            return Err(error.message());
        }
        Ok(())
    }

    #[cfg(test)]
    fn ライフサイクル試験登録(
        &mut self,
        runtime_id: &str,
        adapter: Arc<dyn LifecycleAdapter>,
    ) -> Result<(), LifecycleError> {
        self.ライフサイクル.register_trusted(runtime_id, adapter)
    }

    #[cfg(test)]
    fn ライフサイクル試験permission設定(
        &mut self,
        runtime_id: &str,
        action: LifecycleAction,
        granted: bool,
    ) {
        self.ライフサイクル
            .set_permission_for_test(runtime_id, action, granted);
    }

    #[cfg(test)]
    fn ライフサイクル試験復旧削除(&mut self, runtime_id: &str, action: LifecycleAction) {
        self.ライフサイクル
            .remove_recovery_for_test(runtime_id, action);
    }

    #[cfg(test)]
    fn ライフサイクル試験能力削除(&mut self, runtime_id: &str, action: LifecycleAction) {
        self.ライフサイクル
            .remove_capability_for_test(runtime_id, action);
    }

    pub(crate) fn owner要求処理(&mut self, input: &str) -> BrokerResponse {
        match BrokerRequestEnvelope::from_json_str(input) {
            Ok(envelope) => self.処理_with_export_confirmation(
                envelope,
                true,
                OwnerConfirmationSource::OwnerCredential,
            ),
            Err(_) => self.reject_with_payload_hash("malformed-owner-request", "unknown", "broker_request_malformed", "owner要求が不正", true, &sha256_tagged(input.as_bytes())),
        }
    }

    /// Rust起動器のnative確認を通過した固定allowlistのOwner操作だけを受け付ける。
    /// Owner権限は要求本文やmetadataから作らず、このprocess内呼出しだけが供給する。
    pub(crate) fn desktop_owner_operation_json(&mut self, input: &str) -> BrokerResponse {
        let envelope = match BrokerRequestEnvelope::from_json_str(input) {
            Ok(envelope) => envelope,
            Err(_) => {
                return self.reject_with_payload_hash(
                    "desktop-owner-operation-malformed",
                    "unknown",
                    "broker_request_malformed",
                    "Owner確認後の操作要求が不正",
                    true,
                    &sha256_tagged(input.as_bytes()),
                )
            }
        };
        let request_id = envelope.request_id.as_deref().unwrap_or("malformed-request");
        let operation = envelope
            .operation
            .as_ref()
            .map(BrokerOperation::as_str)
            .unwrap_or("unknown");
        let payload_hash = envelope.payload_hash.as_deref().unwrap_or("unknown");
        let metadata_is_desktop = envelope.metadata.len() == 1
            && envelope.metadata[0].key == "client"
            && envelope.metadata[0].value == "desktop_flutter";
        let operation_is_allowlisted = matches!(
            envelope.operation,
            Some(
                BrokerOperation::GuiShell書出し
                    | BrokerOperation::AgentTaskWorkspacePermissionGrant
                    | BrokerOperation::AgentTaskOwnerApprovalGrant
                    | BrokerOperation::回帰Case削除
                    | BrokerOperation::回帰Case削除中断確認
                    | BrokerOperation::回帰Case登録
            )
        );
        if !operation_is_allowlisted
            || envelope.session_id.as_deref() != Some(self.session_id.as_str())
            || !metadata_is_desktop
        {
            return self.reject_with_payload_hash(
                request_id,
                operation,
                "desktop_owner_operation_invalid",
                "Rust Desktop起動器が確認した許可対象操作ではない",
                true,
                payload_hash,
            );
        }
        self.処理_with_export_confirmation(
            envelope,
            true,
            OwnerConfirmationSource::DesktopNativeConfirmation,
        )
    }

    pub(super) fn 処理(&mut self, envelope: BrokerRequestEnvelope, owner: bool) -> BrokerResponse {
        self.処理_with_export_confirmation(
            envelope,
            owner,
            if owner {
                OwnerConfirmationSource::OwnerCredential
            } else {
                OwnerConfirmationSource::NotOwner
            },
        )
    }

    fn 処理_with_export_confirmation(
        &mut self,
        envelope: BrokerRequestEnvelope,
        owner: bool,
        export_confirmation: OwnerConfirmationSource,
    ) -> BrokerResponse {
        self.端末期限処理();
        let request_id = envelope
            .request_id
            .clone()
            .unwrap_or_else(|| "malformed-request".to_string());
        let operation = envelope
            .operation
            .clone()
            .map(|operation| operation.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        if envelope.request_id.as_deref().unwrap_or("").is_empty()
            || envelope.operation.is_none()
            || envelope.payload_hash.as_deref().unwrap_or("").is_empty()
            || envelope.issued_at.as_deref().unwrap_or("").is_empty()
            || envelope.nonce.as_deref().unwrap_or("").is_empty()
            || !envelope.metadata_present
        {
            return self.reject(
                &request_id,
                &operation,
                "broker_request_malformed",
                "broker request envelope is missing required fields",
                true,
            );
        }

        if envelope.operation == Some(BrokerOperation::AgentTaskWorkspacePermissionGrant)
            || envelope.operation == Some(BrokerOperation::AgentTaskOwnerApprovalGrant)
        {
            if export_confirmation != OwnerConfirmationSource::DesktopNativeConfirmation {
                return self.reject_with_payload_hash(
                    &request_id,
                    &operation,
                    "desktop_native_owner_confirmation_required",
                    "Agent Taskの権限・ApprovalはRust Desktopのnative Owner確認経路だけで発行できます",
                    true,
                    envelope.payload_hash.as_deref().unwrap_or("unknown"),
                );
            }
        }

        let payload_hash = envelope.payload_hash.clone().unwrap_or_default();
        if !is_tagged_sha256(&payload_hash) {
            return self.reject(
                &request_id,
                &operation,
                "broker_payload_hash_invalid",
                "payload_hash must be tagged sha256",
                true,
            );
        }

        let expected_payload_hash = canonical_payload_hash(envelope.payload.as_ref());
        if payload_hash != expected_payload_hash {
            return self.reject_with_payload_hash(
                &request_id,
                &operation,
                "broker_payload_hash_mismatch",
                "payload_hash must match the canonical request payload",
                true,
                &payload_hash,
            );
        }

        if !issued_at_is_fresh(
            envelope.issued_at.as_deref().unwrap_or(""),
            self.current_epoch_seconds(),
        ) {
            return self.reject_with_payload_hash(
                &request_id,
                &operation,
                "broker_issued_at_invalid",
                "issued_at must be RFC3339 and within the broker freshness window",
                true,
                &payload_hash,
            );
        }

        if self.state_store.persistence_required() && !self.state_store.persistence_ready() {
            if envelope.operation == Some(BrokerOperation::Health) {
                return self.suspend_health(&request_id, &payload_hash);
            }
            return self.reject_with_payload_hash(
                &request_id,
                &operation,
                "broker_persistence_unavailable",
                self.state_store.unavailable_message(),
                true,
                &payload_hash,
            );
        }

        if envelope.operation != Some(BrokerOperation::Health)
            && envelope.session_id.as_deref() != Some(self.session_id.as_str())
        {
            return self.reject_with_payload_hash(
                &request_id,
                &operation,
                "broker_stale_session",
                "broker session is missing or stale",
                true,
                &payload_hash,
            );
        }

        let nonce = envelope.nonce.clone().unwrap_or_default();
        if self.seen_nonces.contains_key(&nonce) {
            return self.reject_with_payload_hash(
                &request_id,
                &operation,
                "broker_replay_detected",
                "broker request nonce was replayed",
                true,
                &payload_hash,
            );
        }

        if metadata_attempts_authority(&envelope.metadata) {
            if let Err(error) = self.record_nonce(&nonce) {
                return self.audit_store_failed_response(
                    &request_id,
                    &operation,
                    "broker_persistence_unavailable",
                    &error.message(),
                );
            }
            return self.reject_with_payload_hash(
                &request_id,
                &operation,
                "broker_authority_metadata_rejected",
                "broker metadata attempted to carry authority",
                true,
                &payload_hash,
            );
        }

        if let Err(error) = self.record_nonce(&nonce) {
            return self.reject_with_payload_hash(
                &request_id,
                &operation,
                "broker_persistence_unavailable",
                &error.message(),
                true,
                &payload_hash,
            );
        }

        match envelope.operation.unwrap() {
            operation @ (BrokerOperation::作業領域一覧 | BrokerOperation::作業領域承認 | BrokerOperation::作業領域失効 | BrokerOperation::作業領域ツリー | BrokerOperation::作業領域読取 | BrokerOperation::作業領域基準点保存 | BrokerOperation::作業領域差分 | BrokerOperation::作業領域比較範囲 | BrokerOperation::作業領域全体基準点保存 | BrokerOperation::作業領域変更一覧 | BrokerOperation::作業領域復旧プレビュー) => self.作業領域要求処理(&request_id, operation.as_str(), envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            operation @ (BrokerOperation::端末招待 | BrokerOperation::端末一覧 | BrokerOperation::端末招待取消 | BrokerOperation::端末失効) => self.端末制御処理(&request_id, operation.as_str(), envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::ホスト能力 => self.ホスト能力処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), &payload_hash),
            BrokerOperation::実行系資源観測 => self.実行系資源観測要求処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), &payload_hash),
            BrokerOperation::実行系ライフサイクル状態 => self.実行系ライフサイクル状態処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::実行系ライフサイクル承認要求 => self.実行系ライフサイクル承認要求処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::実行系ライフサイクル承認 => self.実行系ライフサイクル承認処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::実行系ライフサイクル操作 => self.実行系ライフサイクル操作処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::全Runtime停止要求 => self.全Runtime停止要求処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), &payload_hash),
            BrokerOperation::評価Dataset登録 => self.評価Dataset登録処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::回帰Case登録 => self.回帰Case登録処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::回帰Case一覧 => self.回帰Case一覧処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::回帰Case削除 => self.回帰Case削除処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::回帰Case削除中断確認 => self.回帰Case削除中断確認処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            #[cfg(windows)]
            BrokerOperation::資格情報登録 => self.資格情報登録処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            #[cfg(not(windows))]
            BrokerOperation::資格情報登録 => self.reject_with_payload_hash(&request_id, "資格情報登録", "credential_platform_unsupported", "資格情報保管はWindows DPAPI環境だけに対応しています", true, &payload_hash),
            #[cfg(windows)]
            BrokerOperation::資格情報一覧 => self.資格情報一覧処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            #[cfg(not(windows))]
            BrokerOperation::資格情報一覧 => self.reject_with_payload_hash(&request_id, "資格情報一覧", "credential_platform_unsupported", "資格情報保管はWindows DPAPI環境だけに対応しています", true, &payload_hash),
            BrokerOperation::MCP接続 => super::mcp_center::connect(self, &request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::MCP接続一覧 => super::mcp_center::list(self, &request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::A2A接続 => super::a2a_center::connect(self, &request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::A2A接続一覧 => super::a2a_center::list(self, &request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::Host登録 => super::host_center::register(self, &request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::Host一覧 => super::host_center::list(self, &request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::Host切替 => super::host_center::switch(self, &request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            operation @ (BrokerOperation::アダプター一覧 | BrokerOperation::アダプター導入 | BrokerOperation::アダプター検証 | BrokerOperation::アダプター有効化 | BrokerOperation::アダプター無効化 | BrokerOperation::アダプター隔離 | BrokerOperation::アダプター更新 | BrokerOperation::アダプター削除) => super::adapter_center::dispatch(self, operation, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &request_id, &payload_hash),
            operation @ (BrokerOperation::プロファイル作成 | BrokerOperation::プロファイル複製 | BrokerOperation::プロファイル適用要求 | BrokerOperation::プロファイル削除 | BrokerOperation::プロファイルexport | BrokerOperation::プロファイルimport | BrokerOperation::プロファイル一覧) => super::profile_center::dispatch(self, operation, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &request_id, &payload_hash),
            BrokerOperation::GuiShell構成 => super::compose_center::compose(self, &request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::GuiShell構成Preview => super::compose_center::preview(self, &request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::GuiShell編集提案 => super::ai_edit_center::propose(self, &request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::GuiShell書出し => super::export_center::export(self, &request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, export_confirmation, &payload_hash),
            operation @ (BrokerOperation::更新一覧 | BrokerOperation::更新確認 | BrokerOperation::更新署名検査 | BrokerOperation::更新download要求 | BrokerOperation::更新適用要求 | BrokerOperation::更新延期 | BrokerOperation::更新rollback要求) => super::update_center::dispatch(self, operation, envelope.payload.as_ref().unwrap_or(&Value::Null), &request_id, &payload_hash),
            operation @ (BrokerOperation::通知一覧 | BrokerOperation::通知既読 | BrokerOperation::通知破棄 | BrokerOperation::通知全既読) => super::notification_center::dispatch(self, operation, envelope.payload.as_ref().unwrap_or(&Value::Null), &request_id, &payload_hash),
            BrokerOperation::観測一覧 => super::observation_center::dispatch(self, BrokerOperation::観測一覧, envelope.payload.as_ref().unwrap_or(&Value::Null), &request_id, &payload_hash),
            operation @ (BrokerOperation::評価Dataset一覧 | BrokerOperation::評価実験開始 | BrokerOperation::評価実験状態 | BrokerOperation::評価比較) => self.評価通常要求処理(&request_id, operation.as_str(), envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            operation @ (
                BrokerOperation::実行系列挙
                | BrokerOperation::対話セッション一覧
                | BrokerOperation::Agent一覧
                | BrokerOperation::Agent作業要求検査
                | BrokerOperation::AgentTaskWorkspacePermissionGrant
                | BrokerOperation::AgentTaskOwnerApprovalGrant
                | BrokerOperation::AgentTask実行
                | BrokerOperation::AgentTask状態
                | BrokerOperation::AgentTask取消
                | BrokerOperation::対話開始
                | BrokerOperation::対話送信
                | BrokerOperation::対話取得
                | BrokerOperation::対話中止
                | BrokerOperation::対話終了
                | BrokerOperation::対話承認
                | BrokerOperation::対話承認待ち
            ) => self.対話要求処理(
                &request_id,
                operation,
                envelope.payload.as_ref().unwrap_or(&Value::Null),
                owner,
                &payload_hash,
            ),
            operation @ (BrokerOperation::対話内容承認 | BrokerOperation::対話内容失効 | BrokerOperation::対話内容閲覧状態 | BrokerOperation::対話内容閲覧) => self.内容閲覧処理(&request_id, operation.as_str(), envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::対話部分保存破棄 => self.部分保存破棄処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::対話削除中断確認 => self.削除中断確認処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::対話部分破棄中断確認 => self.部分破棄中断確認処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::対話保管状態 => self.保管状態処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::対話内容削除 => self.内容削除処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::対話内容保存 => self.対話内容保存処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::SetupDoctor報告取得 => self.setup_doctor_report(
                &request_id,
                envelope.payload.as_ref().unwrap_or(&Value::Null),
                &payload_hash,
            ),
            BrokerOperation::初回設定取得 => self.first_run_configuration(
                &request_id,
                envelope.payload.as_ref().unwrap_or(&Value::Null),
                &payload_hash,
            ),
            BrokerOperation::Health => self.accept_health(&request_id, &payload_hash),
            BrokerOperation::Shutdown => self.accept_shutdown(&request_id, &payload_hash),
            BrokerOperation::CommandEnvelope => self.suspend_command(
                &request_id,
                envelope.payload.as_ref().unwrap_or(&Value::Null),
                &payload_hash,
            ),
            BrokerOperation::AuthorityEvaluate => {
                let payload = envelope.payload.as_ref().unwrap_or(&Value::Null);
                if payload.get("state").is_some() {
                    self.reject_with_payload_hash(
                        &request_id,
                        BrokerOperation::AuthorityEvaluate.as_str(),
                        "broker_authority_state_rejected",
                        "production authority evaluation does not accept caller-supplied state",
                        true,
                        &payload_hash,
                    )
                } else {
                    let body = evaluate_broker_authority(&self.authority_registry, payload);
                    self.accept_authority_decision(&request_id, body, &payload_hash)
                }
            }
            BrokerOperation::AuthorityFixtureEvaluate => self.accept_body_with_evidence(
                &request_id,
                BrokerOperation::AuthorityFixtureEvaluate,
                evaluate_authority(envelope.payload.as_ref().unwrap_or(&Value::Null)),
                EVIDENCE_SOURCE_INTERNAL_STATE,
                &payload_hash,
            ),
            BrokerOperation::ApprovalEdit => self.accept_body(
                &request_id,
                BrokerOperation::ApprovalEdit,
                edit_approval(envelope.payload.as_ref().unwrap_or(&Value::Null)),
                &payload_hash,
            ),
            BrokerOperation::ContentProjection => self.accept_body(
                &request_id,
                BrokerOperation::ContentProjection,
                project_approval_content(envelope.payload.as_ref().unwrap_or(&Value::Null)),
                &payload_hash,
            ),
            operation @ (BrokerOperation::対話履歴承認 | BrokerOperation::対話履歴失効 | BrokerOperation::対話履歴閲覧状態 | BrokerOperation::対話履歴閲覧) => self.履歴閲覧処理(&request_id, operation.as_str(), envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            operation @ (BrokerOperation::対話再実行 | BrokerOperation::対話分岐) => self.履歴再要求(&request_id, operation.as_str(), envelope.payload.as_ref().unwrap_or(&Value::Null), &payload_hash),
            BrokerOperation::対話履歴一覧 => self.履歴要求処理(&request_id, envelope.payload.as_ref().unwrap_or(&Value::Null), owner, &payload_hash),
            BrokerOperation::AuditVerify => self.accept_body(
                &request_id,
                BrokerOperation::AuditVerify,
                verify_audit_chain(envelope.payload.as_ref().unwrap_or(&Value::Null)),
                &payload_hash,
            ),
            BrokerOperation::NormalizePayload => self.accept_body(
                &request_id,
                BrokerOperation::NormalizePayload,
                normalize_inbound_payload(envelope.payload.as_ref().unwrap_or(&Value::Null)),
                &payload_hash,
            ),
        }
    }

    pub(crate) fn 保管先起動登録(&mut self, path: &Path, owner: bool, protected: &[std::path::PathBuf]) -> Result<(), &'static str> {
        let hash = sha256_tagged(path.to_string_lossy().as_bytes());
        self.append_audit("保管先起動", "保管先登録", "received", "owner起動設定の保管先検査を受信", "CONFIG", &hash).map_err(|_| "保管先監査失敗")?;
        let result = (|| {
            if !owner { return Err("保管先登録にはowner資格設定が必要"); }
            #[cfg(not(windows))] {
                let _ = protected;
                Err("Windows以外の保管先登録は未対応")
            }
            #[cfg(windows)] {
                if self.protected_store.is_some() { return Err("保管先登録の重複"); }
                let (root, _) = super::workspace_root::open_isolated_root(path, protected)?;
                self.append_audit("保管先起動", "保管先登録", "verified", "固定NTFS・link拒否・内部資格分離を確認", EVIDENCE_SOURCE_LIVE_RUNTIME, &hash).map_err(|_| "保管先監査失敗")?;
                self.protected_store = Some(crate::protected_store::ProtectedStore::new(root));
                Ok(())
            }
        })();
        if let Err(reason) = result {
            self.append_audit("保管先起動", "保管先登録", "rejected", reason, EVIDENCE_SOURCE_INTERNAL_STATE, &hash).map_err(|_| "保管先拒否監査失敗")?;
        }
        result
    }

    /// Desktop製品がcodeで固定したruntime隣接ProtectedStoreだけを起動時に開く。
    /// Owner資格・Permission・Approvalを作らず、起動失敗はBroker全体を停止する。
    pub(crate) fn desktop_protected_store_startup(
        &mut self,
        path: &Path,
        protected: &[std::path::PathBuf],
    ) -> Result<(), &'static str> {
        let hash = sha256_tagged(path.to_string_lossy().as_bytes());
        self.append_audit(
            "desktop-protected-store:start",
            "Desktop固定ProtectedStore起動",
            "received",
            "Capability=固定Desktop保存領域をBrokerへ接続 Permission=Broker内部の用途分離保管だけ Approval=privileged operationを承認しない RecoveryAction=検証失敗時はBroker起動を停止し既存fileを保持",
            "CONFIG",
            &hash,
        )
        .map_err(|_| "Desktop ProtectedStore起動監査に失敗")?;
        let result = (|| {
            if self.protected_store.is_some() {
                return Err("ProtectedStoreは既に起動登録されている");
            }
            #[cfg(not(windows))]
            {
                let _ = protected;
                Err("Desktop ProtectedStoreはWindows固定NTFS環境に限定")
            }
            #[cfg(windows)]
            {
                let (root, _) = super::workspace_root::open_isolated_root(path, protected)?;
                self.append_audit(
                    "desktop-protected-store:verified",
                    "Desktop固定ProtectedStore起動",
                    "verified",
                    "固定runtime隣接path・NTFS・link拒否・Broker内部storeとの分離を確認",
                    EVIDENCE_SOURCE_LIVE_RUNTIME,
                    &hash,
                )
                .map_err(|_| "Desktop ProtectedStore検証監査に失敗")?;
                self.protected_store = Some(crate::protected_store::ProtectedStore::new(root));
                Ok(())
            }
        })();
        if let Err(reason) = result {
            self.append_audit(
                "desktop-protected-store:rejected",
                "Desktop固定ProtectedStore起動",
                "rejected",
                reason,
                EVIDENCE_SOURCE_INTERNAL_STATE,
                &hash,
            )
            .map_err(|_| "Desktop ProtectedStore拒否監査に失敗")?;
        }
        result
    }

    pub(crate) fn 作業領域設定監査(&mut self, path: &Path, decision: &str, reason: &str) -> Result<(), &'static str> {
        self.append_audit("作業領域設定", "作業領域設定読取", decision, reason, "CONFIG", &sha256_tagged(path.to_string_lossy().as_bytes()))
            .map(|_|()).map_err(|_| "作業領域設定の監査失敗")
    }

    pub(crate) fn 作業領域起動登録(&mut self, config: &super::workspace_root::WorkspaceStartup, protected: &[std::path::PathBuf]) -> Result<(), &'static str> {
        let payload=serde_json::to_value(config).map_err(|_| "作業領域設定を正本化できない")?;
        let hash=sha256_tagged(payload.to_string().as_bytes());
        self.append_audit("作業領域起動", "作業領域登録", "received", "owner起動設定のroot検査を受信", EVIDENCE_SOURCE_INTERNAL_STATE, &hash).map_err(|_| "作業領域設定の監査失敗")?;
        let result=(|| {
            if self.ライフサイクル.is_terminally_quarantined(&config.runtime_id) {return Err("実行系はterminal隔離中");}
            if !self.authority_registry.runtime_registered(&config.runtime_id) && !self.対話.登録済み(&config.runtime_id) {return Err("実行系が未登録");}
            let (root,filesystem,ancestry)=super::workspace_root::open_registered_root_with_ancestry(config,protected)?;
            self.append_audit("作業領域起動", "作業領域登録", "verified", "rootの対応filesystemと内部資格分離を確認", EVIDENCE_SOURCE_LIVE_RUNTIME, &sha256_tagged(format!("{hash}:{filesystem}").as_bytes())).map_err(|_| "root検査の監査失敗")?;
            self.作業領域登録範囲付き(&config.runtime_id,&config.workspace_id,root,&config.secret_paths,Some(ancestry))?;
            self.recover_agent_task_scratch(config, protected)?;
            Ok(())
        })();
        if let Err(reason)=result {
            self.作業領域=Default::default();
            self.append_audit("作業領域起動", "作業領域登録", "rejected", reason, EVIDENCE_SOURCE_INTERNAL_STATE, &hash).map_err(|_| "登録拒否の監査失敗")?;
        }
        result
    }

    fn recover_agent_task_scratch(
        &mut self,
        config: &super::workspace_root::WorkspaceStartup,
        protected: &[std::path::PathBuf],
    ) -> Result<(), &'static str> {
        let Some(journal) = self.agent_task_scratch.clone() else {
            return Err("Agent Task scratch回復記録が利用不能");
        };
        if !journal.has_pending_workspace(&config.workspace_id) {
            return Ok(());
        }
        let binding = self
            .作業領域
            .dialogue_binding(&config.runtime_id, &config.workspace_id)
            .ok_or("scratch回復対象のWorkspace登録が不在")?;
        let (root, _, _) = super::workspace_root::open_registered_root_with_ancestry(config, protected)?;
        let metadata = root
            .dir_metadata()
            .map_err(|_| "scratch回復rootのidentityを確認できない")?;
        let actual_root = super::workspace_root::DirectoryIdentity {
            device: cap_fs_ext::MetadataExt::dev(&metadata),
            file_id: cap_fs_ext::MetadataExt::ino(&metadata),
        };
        let bound_root = binding.root_directory_identity();
        if actual_root.file_id == 0 || actual_root != bound_root {
            return Err("scratch回復rootが登録時の実体と一致しない");
        }
        let request_hash = sha256_tagged(
            format!(
                "agent-task-scratch-recovery|{}|{}|{}",
                config.runtime_id,
                config.workspace_id,
                binding.recovery_binding_hash()
            )
            .as_bytes(),
        );
        self.append_audit(
            "agent-task-scratch-recovery:start",
            "Agent Task scratch回復",
            "received",
            "Capability=Broker管理scratch回復 Permission=現在登録Workspaceの直接子で実体identityが一致するBroker記録対象だけを削除 Approval=新しい権限を付与せず過去Taskの実行許可を再利用しない RecoveryAction=不一致・予約のみ・unsafe pathは保持してWorkspace Taskを拒否",
            EVIDENCE_SOURCE_LIVE_RUNTIME,
            &request_hash,
        )
        .map_err(|_| "Agent Task scratch回復の開始監査を確定できない")?;
        let outcomes = journal.recover_workspace(
            &config.runtime_id,
            &config.workspace_id,
            binding.recovery_binding_hash(),
            &root,
            actual_root,
        );
        for (index, outcome) in outcomes.iter().enumerate() {
            let outcome_hash = sha256_tagged(
                format!("{}|{}|{:?}", request_hash, index, outcome).as_bytes(),
            );
            self.append_audit(
                &format!("agent-task-scratch-recovery:{index}"),
                "Agent Task scratch回復",
                if matches!(outcome, super::agent_task_scratch::RecoveryOutcome::Removed | super::agent_task_scratch::RecoveryOutcome::MissingReconciled | super::agent_task_scratch::RecoveryOutcome::ReservedMissingReconciled) { "recorded" } else { "suspended" },
                "Broker回復journalと現在Workspace bindingに対する限定回復結果。path・Task本文・資格は監査へ含めない",
                EVIDENCE_SOURCE_LIVE_RUNTIME,
                &outcome_hash,
            )
            .map_err(|_| "Agent Task scratch回復結果の監査を確定できない")?;
        }
        Ok(())
    }

    /// 起動制御面の登録。通常IPCとowner IPCはrootを提供できない。
    pub fn 作業領域登録(&mut self, runtime: &str, id: &str, root: cap_std::fs::Dir, secrets: &[String]) -> Result<(), &'static str> {
        self.作業領域登録範囲付き(runtime,id,root,secrets,None)
    }

    fn 作業領域登録範囲付き(
        &mut self,
        runtime: &str,
        id: &str,
        root: cap_std::fs::Dir,
        secrets: &[String],
        ancestry: Option<Vec<super::workspace_root::DirectoryIdentity>>,
    ) -> Result<(), &'static str> {
        if !self.state_store.persistence_ready() {return Err("作業領域登録には永続監査が必要");}
        if self.ライフサイクル.is_terminally_quarantined(runtime) {return Err("実行系はterminal隔離中");}
        if !self.authority_registry.runtime_registered(runtime) && !self.対話.登録済み(runtime) {return Err("実行系が未登録");}
        let mut registry = std::mem::take(&mut self.作業領域);
        let mut audit_failed = false;
        let result = registry.register(runtime, id, root, secrets, ancestry, &mut |reason, hash| {
            self.append_audit("作業領域登録", "作業領域登録", "recorded", reason, EVIDENCE_SOURCE_INTERNAL_STATE, hash)
                .map(|_|()).map_err(|_| {audit_failed = true; "監査失敗"})
        });
        if !audit_failed {self.作業領域 = registry;}
        result
    }

    fn 作業領域要求処理(&mut self, id: &str, operation: &str, payload: &Value, owner: bool, hash: &str) -> BrokerResponse {
        if !self.state_store.persistence_ready() {
            self.作業領域 = Default::default();
            return self.reject_with_payload_hash(id, operation, "broker_persistence_unavailable", "作業領域には永続監査が必要", true, hash);
        }
        if matches!(operation, "作業領域承認" | "作業領域失効" | "作業領域基準点保存" | "作業領域全体基準点保存") && !owner {
            return self.reject_with_payload_hash(id, operation, "権限拒否", "owner制御資格が必要", true, hash);
        }
        if self.append_audit(id, operation, "received", "作業領域操作を受信", EVIDENCE_SOURCE_INTERNAL_STATE, hash).is_err() {
            self.作業領域 = Default::default();
            return self.audit_store_failed_response(id, operation, "監査失敗", "監査修復後に作業領域を再登録");
        }
        let mut registry = std::mem::take(&mut self.作業領域);
        let mut audit_failed = false;
        let now = self.current_epoch_seconds();
        let result = registry.operate(operation, payload, owner, now, &mut |reason, digest| {
            self.append_audit(id, operation, "recorded", reason, EVIDENCE_SOURCE_INTERNAL_STATE, digest)
                .map(|_|()).map_err(|_| {audit_failed = true; "監査失敗"})
        });
        if audit_failed {
            return self.audit_store_failed_response(id, operation, "監査失敗", "監査修復後に作業領域を再登録");
        }
        self.作業領域 = registry;
        match result {
            Ok(body) => {
                let evidence = if matches!(operation, "作業領域読取" | "作業領域ツリー" | "作業領域基準点保存" | "作業領域全体基準点保存" | "作業領域復旧プレビュー" | "作業領域差分" | "作業領域変更一覧") && matches!(body["表示範囲"].as_str(), Some("full" | "hash_only")) {EVIDENCE_SOURCE_LIVE_RUNTIME} else {EVIDENCE_SOURCE_INTERNAL_STATE};
                match self.append_audit(id, operation, "accepted", "作業領域操作の結果を確定", evidence, &sha256_tagged(body.to_string().as_bytes())) {
                    Ok(event) => {
                        if matches!(operation, "作業領域読取" | "作業領域ツリー" | "作業領域基準点保存" | "作業領域全体基準点保存" | "作業領域復旧プレビュー" | "作業領域差分" | "作業領域変更一覧" | "作業領域比較範囲") && !self.作業領域.response_current(&body, self.current_epoch_seconds()) {
                            self.作業領域 = Default::default();
                            return self.reject_with_payload_hash(id, operation, "作業領域期限超過", "取得後の承認期限を満たさないため再登録が必要", true, hash);
                        }
                        BrokerResponse {request_id:id.into(),operation:operation.into(),status:BrokerStatus::Accepted,evidence_source:evidence.into(),audit_event_id:event.event_id,error:None,health:None,body:Some(body),shutdown_requested:false}
                    },
                    Err(_) => {self.作業領域 = Default::default();self.audit_store_failed_response(id, operation, "監査失敗", "監査修復後に作業領域を再登録")}
                }
            }
            Err(reason) => self.reject_with_payload_hash(id, operation, "作業領域拒否", reason, true, hash),
        }
    }

    pub(crate) fn 端末経路設定(&mut self, certificate_hash: String, port: u16) -> Result<(), &'static str> {
        if !self.state_store.persistence_ready() || self.端末.is_some() {return Err("端末経路を設定できない");}
        self.端末 = Some(super::device_link::端末制御::新規(certificate_hash, port)?);
        Ok(())
    }

    fn 端末全停止(&mut self) {
        if let Some(state) = &mut self.端末 {
            let sessions = state.全停止();
            self.対話.資格隔離(&sessions);
        }
    }

    pub(crate) fn 端末期限処理(&mut self) {
        let now = self.current_epoch_seconds();
        if let Some(state) = &mut self.端末 {
            let (expired, sessions) = state.期限処理(now);
            self.対話.資格隔離(&sessions);
            if !expired.is_empty() && self.append_audit("端末期限", "端末失効", "revoked", "期限超過で対話を隔離", EVIDENCE_SOURCE_INTERNAL_STATE, &sha256_tagged(expired.join(",").as_bytes())).is_err() {
                self.端末全停止();
            }
        }
    }

    fn 端末制御処理(&mut self, request_id: &str, operation: &str, payload: &Value, owner: bool, hash: &str) -> BrokerResponse {
        if !owner {return self.reject_with_payload_hash(request_id, operation, "権限拒否", "owner制御資格が必要", true, hash);}
        let _event = match self.append_audit(request_id, operation, "received", "端末制御操作を受信", EVIDENCE_SOURCE_INTERNAL_STATE, hash) {
            Ok(v) => v,
            Err(_) => {self.端末全停止(); return self.audit_store_failed_response(request_id, operation, "監査失敗", "端末経路を停止");}
        };
        let now = self.current_epoch_seconds();
        let result = self.端末.as_mut().ok_or("端末経路が未設定").and_then(|s| s.制御(operation, payload, now));
        match result {
            Ok((body, sessions)) => {
                self.対話.資格隔離(&sessions);
                match self.append_audit(request_id, operation, "accepted", "端末制御結果を確定", EVIDENCE_SOURCE_INTERNAL_STATE, &sha256_tagged(body.to_string().as_bytes())) {
                    Ok(done) => self.端末成功応答(request_id, operation, body, done.event_id, EVIDENCE_SOURCE_INTERNAL_STATE),
                    Err(_) => {self.端末全停止();self.audit_store_failed_response(request_id,operation,"監査失敗","端末経路を停止")}
                }
            }
            Err(reason) => self.reject_with_payload_hash(request_id, operation, "端末制御拒否", reason, true, hash),
        }
    }

    fn 端末成功応答(&self, id: &str, operation: &str, body: Value, audit_id: String, evidence_source: &str) -> BrokerResponse {
        BrokerResponse {request_id:id.into(),operation:operation.into(),status:BrokerStatus::Accepted,evidence_source:evidence_source.into(),audit_event_id:audit_id,error:None,health:None,body:Some(body),shutdown_requested:false}
    }

    fn 端末作業領域選択投影(body: &Value) -> Result<Value, &'static str> {
        let object = body.as_object().filter(|value| {
            value.len() == 1 && value.contains_key("作業領域")
        }).ok_or("Workspace一覧の応答不正")?;
        let entries = object.get("作業領域").and_then(Value::as_array)
            .filter(|entries| entries.len() <= 16).ok_or("Workspace一覧の上限または形式不正")?;
        let required = ["作業領域ID", "実行系ID", "登録hash", "承認状態", "有効期限", "表示範囲", "approval_id"];
        let valid_id = |value: &str| {
            let bytes = value.as_bytes();
            !bytes.is_empty()
                && bytes.len() <= 128
                && bytes[0].is_ascii_alphanumeric()
                && bytes.iter().all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'.' | b'-'))
        };
        let mut seen = Vec::with_capacity(entries.len());
        let mut projected = Vec::with_capacity(entries.len());
        for entry in entries {
            let fields = entry.as_object().filter(|fields| {
                fields.len() == required.len() && required.iter().all(|key| fields.contains_key(*key))
            }).ok_or("Workspace一覧項目の形式不正")?;
            let id = fields.get("作業領域ID").and_then(Value::as_str).filter(|id| valid_id(id))
                .ok_or("Workspace ID不正")?;
            let runtime = fields.get("実行系ID").and_then(Value::as_str).filter(|id| valid_id(id))
                .ok_or("Runtime ID不正")?;
            if seen.iter().any(|known| known == id) {
                return Err("Workspace ID重複");
            }
            seen.push(id.to_owned());
            projected.push(serde_json::json!({"作業領域ID": id, "実行系ID": runtime}));
        }
        Ok(serde_json::json!({"作業領域": projected}))
    }

    pub(crate) fn 端末要求処理(&mut self, raw: &str) -> BrokerResponse {
        self.端末期限処理();
        let hash = sha256_tagged(raw.as_bytes());
        let parsed = super::device_link::要求読取(raw);
        let r = match parsed {
            Ok(v) => v,
            Err(_) => return self.reject_with_payload_hash("端末不正要求", "端末要求", "要求不正", "端末要求の構造を拒否", true, &hash),
        };
        let op = r.操作.as_str();
        if op != "端末結合" && !super::device_link::許可操作.contains(&op) {
            return self.reject_with_payload_hash("端末不正要求", "端末要求", "権限拒否", "端末操作を拒否", true, &hash);
        }
        let id = if super::device_link::hex形状(&r.nonce,32) {r.nonce.as_str()} else {"端末不正要求"};
        let _event = match self.append_audit(id, op, "received", "暗号化端末要求を受信", EVIDENCE_SOURCE_LIVE_RUNTIME, &hash) {
            Ok(v) => v,
            Err(_) => {self.端末全停止();return self.audit_store_failed_response(id,op,"監査失敗","端末経路を停止");}
        };
        let now = self.current_epoch_seconds();
        let result = if op == "端末結合" {
            self.端末.as_mut().ok_or("端末経路が未設定").and_then(|s|s.結合する(&r,now)).map(|body| (body, EVIDENCE_SOURCE_LIVE_RUNTIME.to_string()))
        } else {
            self.端末.as_mut().ok_or("端末経路が未設定").and_then(|s|s.認証する(&r,now)).and_then(|()|{
                if op == "端末確認" {return Ok((serde_json::json!({"状態":"接続中"}), EVIDENCE_SOURCE_INTERNAL_STATE.to_string()));}
                if op == "端末離脱" {
                    let sessions = self.端末.as_mut().ok_or("端末経路が未設定")?.失効(&r.資格ID)?;
                    self.対話.資格隔離(&sessions);
                    return Ok((serde_json::json!({"状態":"失効"}), EVIDENCE_SOURCE_INTERNAL_STATE.to_string()));
                }
                let response = match op {
                    "実行系資源観測" => self.実行系資源観測要求処理(id, &r.内容, &hash),
                    "実行系ライフサイクル状態" => self.実行系ライフサイクル状態処理(id, &r.内容, false, &hash),
                    "全Runtime停止要求" => self.全Runtime停止要求処理(id, &r.内容, &hash),
                    "通知一覧" => super::notification_center::dispatch(self, BrokerOperation::通知一覧, &r.内容, id, &hash),
                    "対話履歴閲覧状態" | "対話履歴閲覧" => self.履歴閲覧処理(id, op, &r.内容, false, &hash),
                    "作業領域一覧" => self.作業領域要求処理(id, op, &r.内容, false, &hash),
                    _ => {
                        let operation: BrokerOperation = serde_json::from_value(Value::String(op.into())).map_err(|_|"操作不正")?;
                        self.対話要求処理(id, operation, &r.内容, false, &hash)
                    }
                };
                if response.status != BrokerStatus::Accepted {return Err("対話操作拒否");}
                let body = response.body.ok_or("対話応答不正")?;
                let body = if op == "作業領域一覧" {
                    Self::端末作業領域選択投影(&body)?
                } else {
                    body
                };
                let evidence_source = response.evidence_source;
                self.端末.as_mut().ok_or("端末経路が未設定")?.所有記録(&r.資格ID,op,&body)?;
                Ok((body, evidence_source))
            })
        };
        match result {
            Ok((body, evidence_source)) => match self.append_audit(id,op,"accepted","端末要求結果を確定",&evidence_source,&sha256_tagged(body.to_string().as_bytes())) {
                Ok(done) => self.端末成功応答(id,op,body,done.event_id,&evidence_source),
                Err(_) => {self.端末全停止();self.audit_store_failed_response(id,op,"監査失敗","端末経路を停止")}
            },
            Err(reason) => self.reject_with_payload_hash(id,op,"端末要求拒否",reason,true,&hash),
        }
    }

    fn 履歴閲覧処理(&mut self, id: &str, op: &str, payload: &Value, owner: bool, hash: &str) -> BrokerResponse {
        let log = match self.state_store.persistent_store.as_ref().ok_or("永続監査が必要")
            .and_then(|s|s.verified_audit_log().map_err(|_|"監査検証失敗")) {
            Ok(log) if log==self.audit_log => log,
            _ => {self.履歴閲覧=Default::default(); return self.audit_store_failed_response(id,op,"監査失敗","監査修復後に再承認");}
        };
        if self.append_audit(id,op,"received","Capability=dialogue.history.inspect Permission=現在承認照合 Approval=owner発行資格 Recovery=再承認",EVIDENCE_SOURCE_INTERNAL_STATE,hash).is_err() {
            self.履歴閲覧=Default::default();return self.audit_store_failed_response(id,op,"監査失敗","監査修復後に再承認");
        }
        let mut access=std::mem::take(&mut self.履歴閲覧);
        let now=self.current_epoch_seconds();
        let mut audit_failed=false;
        let result=access.operate(op,payload,owner,now,&log,&mut |reason,digest|self.append_audit(id,op,"recorded",reason,EVIDENCE_SOURCE_INTERNAL_STATE,digest).map(|_|()).map_err(|_|{audit_failed=true;"監査失敗"}));
        if audit_failed {return self.audit_store_failed_response(id,op,"監査失敗","監査修復後に再承認");}
        self.履歴閲覧=access;
        match result {
            Err(reason)=>self.reject_with_payload_hash(id,op,"権限拒否",reason,true,hash),
            Ok(body)=>match self.append_audit(id,op,"accepted","履歴metadata閲覧の結果を確定。現在実行権限を生成しない",EVIDENCE_SOURCE_INTERNAL_STATE,&sha256_tagged(body.to_string().as_bytes())) {
                Err(_)=>{self.履歴閲覧=Default::default();self.audit_store_failed_response(id,op,"監査失敗","監査修復後に再承認")},
                Ok(event)=>{
                    let now=self.current_epoch_seconds();
                    if !body["grant"].is_null() && !self.履歴閲覧.current(&body,now) {return self.reject_with_payload_hash(id,op,"期限超過","履歴閲覧の承認期限超過",true,hash);}
                    BrokerResponse{request_id:id.into(),operation:op.into(),status:BrokerStatus::Accepted,evidence_source:EVIDENCE_SOURCE_INTERNAL_STATE.into(),audit_event_id:event.event_id,error:None,health:None,body:Some(body),shutdown_requested:false}
                }
            }
        }
    }

    fn 履歴要求処理(&mut self, id: &str, payload: &Value, owner: bool, hash: &str) -> BrokerResponse {
        let op = "対話履歴一覧";
        if !owner { return self.reject_with_payload_hash(id, op, "権限拒否", "owner制御資格が必要", true, hash); }
        let query = match serde_json::from_value::<super::execution_history::Query>(payload.clone()) {
            Ok(v) => v,
            Err(_) => return self.reject_with_payload_hash(id, op, "要求不正", "履歴範囲が不正", true, hash),
        };
        let log = match self.state_store.persistent_store.as_ref().ok_or("永続監査が必要")
            .and_then(|s| s.verified_audit_log().map_err(|_| "監査検証に失敗")) {
            Ok(log) if log == self.audit_log => log,
            _ => return self.audit_store_failed_response(id, op, "監査失敗", "監査修復後に再確認"),
        };
        if self.append_audit(id, op, "received", "Capability=dialogue.history.inspect Permission=owner-control Approval=owner要求 Recovery=監査修復後の再確認", EVIDENCE_SOURCE_INTERNAL_STATE, hash).is_err() {
            return self.audit_store_failed_response(id, op, "監査失敗", "監査修復後に再確認");
        }
        let body = match super::execution_history::page(&log, query) {
            Ok(body) => body,
            Err(reason) => return self.reject_with_payload_hash(id, op, "履歴拒否", reason, true, hash),
        };
        match self.append_audit(id, op, "accepted", "過去の観測記録を返却。現在権限を生成しない", EVIDENCE_SOURCE_INTERNAL_STATE, &sha256_tagged(body.to_string().as_bytes())) {
            Ok(event) => BrokerResponse {request_id:id.into(),operation:op.into(),status:BrokerStatus::Accepted,evidence_source:EVIDENCE_SOURCE_INTERNAL_STATE.into(),audit_event_id:event.event_id,error:None,health:None,body:Some(body),shutdown_requested:false},
            Err(_) => self.audit_store_failed_response(id, op, "監査失敗", "監査修復後に再確認"),
        }
    }

    fn 対話内容保存処理(&mut self, id: &str, payload: &Value, owner: bool, hash: &str) -> BrokerResponse {
        let op = "対話内容保存";
        if !owner || !self.state_store.persistence_ready() {
            return self.reject_with_payload_hash(id, op, "権限拒否", "owner制御資格と永続監査が必要", true, hash);
        }
        match self.state_store.persistent_store.as_ref().and_then(|s| s.verified_audit_log().ok()) {
            Some(log) if log == self.audit_log => (),
            _ => return self.audit_store_failed_response(id, op, "監査失敗", "保管監査再確認"),
        }
        if self.append_audit(id, op, "received", "対話内容保存要求を受信", EVIDENCE_SOURCE_INTERNAL_STATE, hash).is_err() {
            return self.audit_store_failed_response(id, op, "監査失敗", "保管監査再確認");
        }
        #[cfg(not(windows))]
        {
            let _ = payload;
            self.reject_with_payload_hash(id, op, "保管未対応", "このOSの安全保管は未対応", true, hash)
        }
        #[cfg(windows)]
        {
            if self.protected_store.is_none() {
                return self.reject_with_payload_hash(id, op, "保管未登録", "起動制御で保管先を登録してください", true, hash);
            }
            let content = match self.対話.保存対象(payload) {
                Ok(v) => v,
                Err(e) => return self.reject_with_payload_hash(id, op, e.分類(), e.復旧(), true, hash),
            };
            let bytes = content.to_string().into_bytes();
            if bytes.len() > gui_shell_windows_protection::MAX_PLAINTEXT {
                return self.reject_with_payload_hash(id, op, "保管上限超過", "保管監査再確認", true, hash);
            }
            let target = content["要求"]["要求ID"].as_str().expect("検証済み要求ID");
            if content_discard::blocked(&self.audit_log,target).unwrap_or(true) {
                return self.reject_with_payload_hash(id,op,"破棄承認済み","同じ要求を再保存せず新要求で確認",true,hash);
            }
            if self.audit_log.events().iter().any(|e| e.operation == op && e.decision == "accepted"
                && e.reason.strip_prefix("対話内容保存記録:").and_then(|s| serde_json::from_str::<Value>(s).ok())
                    .is_some_and(|v| v["要求ID"] == target)) {
                return self.reject_with_payload_hash(id, op, "保存済み", "暗号文の有無によらず再保存せず保管監査再確認", true, hash);
            }
            let reason = format!("対話内容保存承認 Capability=対話内容保存 Permission=独立保管先:{target} Approval={hash} RecoveryAction=保管監査再確認");
            let approval = match self.append_audit(id, op, "recorded", &reason, EVIDENCE_SOURCE_INTERNAL_STATE, hash) {
                Ok(v) => v.event_id,
                Err(_) => return self.audit_store_failed_response(id, op, "監査失敗", "保管監査再確認"),
            };
            let cipher_hash = match self.protected_store.as_ref().expect("登録検証済み").create(crate::protected_store::Purpose::History, target, &bytes) {
                Ok(v) => v,
                Err(_) => return self.reject_with_payload_hash(id, op, "保管失敗", "既存/部分fileを変更せず保管監査再確認", true, hash),
            };
            let body = serde_json::json!({"版":1,"要求ID":target,
                "対話セッションID":content["要求"]["対話セッションID"],"実行系ID":content["要求"]["実行系ID"],
                "要求hash":content["要求hash"],"終了監査ID":content["実行記録"]["終了監査ID"],
                "保存承認監査ID":approval,"暗号文hash":cipher_hash,"証拠種別":"INTERNAL_STATE"});
            self.対話内容保存確定(id, body)
        }
    }

    #[cfg(windows)]
    fn 対話内容保存確定(&mut self, id: &str, body: Value) -> BrokerResponse {
            let op = "対話内容保存";
            let encoded = body.to_string();
            match self.append_audit(id, op, "accepted", &format!("対話内容保存記録:{encoded}"), EVIDENCE_SOURCE_INTERNAL_STATE, &sha256_tagged(encoded.as_bytes())) {
                Ok(v) => BrokerResponse {request_id:id.into(),operation:op.into(),status:BrokerStatus::Accepted,evidence_source:EVIDENCE_SOURCE_INTERNAL_STATE.into(),audit_event_id:v.event_id,error:None,health:None,body:Some(body),shutdown_requested:false},
                Err(_) => self.audit_store_failed_response(id, op, "監査失敗", "暗号文を再使用せず保管監査再確認"),
            }
    }

    fn 実行系ライフサイクル状態処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        owner: bool,
        payload_hash: &str,
    ) -> BrokerResponse {
        let operation = BrokerOperation::実行系ライフサイクル状態;
        if owner {
            return self.reject_with_payload_hash(
                request_id,
                operation.as_str(),
                "lifecycle_normal_channel_required",
                "lifecycle状態照会は通常資格経路だけが実行できる",
                true,
                payload_hash,
            );
        }
        let query: 実行系ライフサイクル状態指定 = match serde_json::from_value::<実行系ライフサイクル状態指定>(payload.clone()) {
            Ok(query) if query.version == 1 && 実行系ID妥当(&query.runtime_id) => query,
            _ => {
                return self.reject_with_payload_hash(
                    request_id,
                    operation.as_str(),
                    "lifecycle_request_invalid",
                    "実行系ライフサイクル状態は版と実行系IDだけを受け付ける",
                    true,
                    payload_hash,
                )
            }
        };
        if super::adapter_center::runtime_is_quarantined(self, &query.runtime_id) {
            return self.reject_with_payload_hash(request_id, operation.as_str(), "adapter_quarantined", "隔離済みAdapterのlifecycle状態照会を拒否しました", true, payload_hash);
        }
        let body = match self
            .ライフサイクル
            .status_body(&query.runtime_id, self.current_epoch_seconds())
        {
            Some(body) => body,
            None if self.対話.登録済み(&query.runtime_id) => {
                RuntimeLifecycleRegistry::unsupported_status_body(&query.runtime_id)
            }
            None => {
                return self.reject_with_payload_hash(
                    request_id,
                    operation.as_str(),
                    "lifecycle_runtime_unknown",
                    "登録済み実行系だけをlifecycle状態照会できる",
                    true,
                    payload_hash,
                )
            }
        };
        let response = self.accept_body_with_evidence(
            request_id,
            operation,
            body,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        );
        if response.status == BrokerStatus::Suspended {
            self.ライフサイクル.fail_closed();
        }
        response
    }

    fn 実行系ライフサイクル承認要求処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        owner: bool,
        payload_hash: &str,
    ) -> BrokerResponse {
        let operation = BrokerOperation::実行系ライフサイクル承認要求;
        if owner {
            return self.reject_with_payload_hash(
                request_id,
                operation.as_str(),
                "lifecycle_normal_channel_required",
                "lifecycle承認要求は通常資格経路だけが作成できる",
                true,
                payload_hash,
            );
        }
        let request: 実行系ライフサイクル承認要求指定 = match serde_json::from_value::<実行系ライフサイクル承認要求指定>(payload.clone()) {
            Ok(request) if request.version == 1 && 実行系ID妥当(&request.runtime_id) => request,
            _ => {
                return self.reject_with_payload_hash(
                    request_id,
                    operation.as_str(),
                    "lifecycle_request_invalid",
                    "実行系ライフサイクル承認要求は版、実行系ID、閉じた操作だけを受け付ける",
                    true,
                    payload_hash,
                )
            }
        };
        if super::adapter_center::runtime_is_quarantined(self, &request.runtime_id) {
            return self.reject_with_payload_hash(request_id, operation.as_str(), "adapter_quarantined", "隔離済みAdapterのlifecycle承認要求を拒否しました", true, payload_hash);
        }
        let body = match self.ライフサイクル.request_approval(
            &request.runtime_id,
            request.action,
            self.current_epoch_seconds(),
        ) {
            Ok(body) => body,
            Err(error) => return self.lifecycle_reject(request_id, operation, error, payload_hash),
        };
        let response = self.accept_body_with_evidence(
            request_id,
            operation,
            body,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        );
        if response.status == BrokerStatus::Suspended {
            self.ライフサイクル.fail_closed();
        }
        response
    }

    #[allow(non_snake_case)]
    fn 全Runtime停止要求処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        payload_hash: &str,
    ) -> BrokerResponse {
        let operation = BrokerOperation::全Runtime停止要求;
        let request: 全Runtime停止要求指定 =
            match serde_json::from_value::<全Runtime停止要求指定>(payload.clone()) {
                Ok(request) if request.version == 1 => request,
                _ => {
                    return self.reject_with_payload_hash(
                        request_id,
                        operation.as_str(),
                        "tray_stop_request_invalid",
                        "全Runtime停止要求は版だけを受け付ける",
                        true,
                        payload_hash,
                    )
                }
            };
        let _ = request;
        let body = self
            .ライフサイクル
            .all_stop_request_body(self.current_epoch_seconds());
        self.accept_body_with_evidence(
            request_id,
            operation,
            body,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        )
    }

    fn 実行系ライフサイクル承認処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        owner: bool,
        payload_hash: &str,
    ) -> BrokerResponse {
        let operation = BrokerOperation::実行系ライフサイクル承認;
        if !owner {
            return self.reject_with_payload_hash(
                request_id,
                operation.as_str(),
                "lifecycle_owner_approval_required",
                "lifecycle承認はowner資格経路だけが実行できる",
                true,
                payload_hash,
            );
        }
        let request: 実行系ライフサイクル承認指定 = match serde_json::from_value::<実行系ライフサイクル承認指定>(payload.clone()) {
            Ok(request)
                if request.version == 1
                    && lifecycle_identifier_valid(&request.approval_id)
                    && is_tagged_sha256(&request.approval_hash) => request,
            _ => {
                return self.reject_with_payload_hash(
                    request_id,
                    operation.as_str(),
                    "lifecycle_request_invalid",
                    "実行系ライフサイクル承認は版、承認ID、canonical承認hashだけを受け付ける",
                    true,
                    payload_hash,
                )
            }
        };
        if self.ライフサイクル.approval_runtime_id(&request.approval_id).is_some_and(|runtime_id| super::adapter_center::runtime_is_quarantined(self, runtime_id)) {
            return self.reject_with_payload_hash(request_id, operation.as_str(), "adapter_quarantined", "隔離済みAdapterのlifecycle承認を拒否しました", true, payload_hash);
        }
        let body = match self.ライフサイクル.approve(
            &request.approval_id,
            &request.approval_hash,
            self.current_epoch_seconds(),
        ) {
            Ok(body) => body,
            Err(error) => return self.lifecycle_reject(request_id, operation, error, payload_hash),
        };
        let response = self.accept_body_with_evidence(
            request_id,
            operation,
            body,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        );
        if response.status == BrokerStatus::Suspended {
            self.ライフサイクル.fail_closed();
        }
        response
    }

    fn 実行系ライフサイクル操作処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        owner: bool,
        payload_hash: &str,
    ) -> BrokerResponse {
        let operation = BrokerOperation::実行系ライフサイクル操作;
        if owner {
            return self.reject_with_payload_hash(
                request_id,
                operation.as_str(),
                "lifecycle_normal_channel_required",
                "lifecycle操作はowner承認後の通常資格経路だけが要求できる",
                true,
                payload_hash,
            );
        }
        let request: 実行系ライフサイクル操作指定 = match serde_json::from_value::<実行系ライフサイクル操作指定>(payload.clone()) {
            Ok(request)
                if request.version == 1
                    && 実行系ID妥当(&request.runtime_id)
                    && lifecycle_identifier_valid(&request.approval_id) => request,
            _ => {
                return self.reject_with_payload_hash(
                    request_id,
                    operation.as_str(),
                    "lifecycle_request_invalid",
                    "実行系ライフサイクル操作は版、実行系ID、閉じた操作、承認IDだけを受け付ける",
                    true,
                    payload_hash,
                )
            }
        };
        if super::adapter_center::runtime_is_quarantined(self, &request.runtime_id) {
            return self.reject_with_payload_hash(request_id, operation.as_str(), "adapter_quarantined", "隔離済みAdapterのlifecycle操作を拒否しました", true, payload_hash);
        }
        if let Err(error) = self.ライフサイクル.preflight_execution(
            &request.runtime_id,
            request.action,
            &request.approval_id,
            payload_hash,
            self.current_epoch_seconds(),
        ) {
            return self.lifecycle_reject(request_id, operation, error, payload_hash);
        }
        let pre_audit_reason = if request.action == LifecycleAction::Quarantine {
            RuntimeLifecycleRegistry::quarantine_reservation_reason(&request.runtime_id)
        } else {
            format!(
                "lifecycle実行直前再照合 Capability=runtime.lifecycle.{} Permission=permission.runtime.lifecycle.{} Approval={} Recovery=recover-runtime-lifecycle-{}",
                request.action.as_str(),
                request.action.as_str(),
                request.approval_id,
                request.action.as_str(),
            )
        };
        if let Err(error) = self.append_audit(
            request_id,
            operation.as_str(),
            "received",
            &pre_audit_reason,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        ) {
            self.ライフサイクル.fail_closed();
            return self.audit_store_failed_response(
                request_id,
                operation.as_str(),
                "broker_audit_append_failed",
                &error.message(),
            );
        }
        if request.action == LifecycleAction::Quarantine {
            self.ライフサイクル
                .mark_terminal_quarantine(&request.runtime_id);
            self.対話.実行系隔離(&request.runtime_id);
            self.資源観測.unregister(&request.runtime_id);
            self.作業領域.remove_runtime(&request.runtime_id);
        }
        let transition = match self.ライフサイクル.execute(
            &request.runtime_id,
            request.action,
            &request.approval_id,
            payload_hash,
            self.current_epoch_seconds(),
            self.current_epoch_millis(),
        ) {
            Ok(transition) => transition,
            Err(error) => return self.lifecycle_reject(request_id, operation, error, payload_hash),
        };
        let lifecycle_audit_id = self.audit_log.next_event_id();
        let body = serde_json::json!({
            "版": 1,
            "実行系ID": transition.runtime_id,
            "操作": transition.action.as_str(),
            "遷移前状態": transition.previous_state.as_str(),
            "遷移後状態": transition.next_state.as_str(),
            "観測時刻UnixMillis": transition.observed_at_epoch_millis,
            "証拠種別": EVIDENCE_SOURCE_LIVE_RUNTIME,
            "統治": {
                "能力ID": format!("runtime.lifecycle.{}", transition.action.as_str()),
                "権限ID": format!("permission.runtime.lifecycle.{}", transition.action.as_str()),
                "承認ID": transition.approval_id,
                "承認状態": "consumed",
                "復旧ID": format!("recover-runtime-lifecycle-{}", transition.action.as_str()),
            },
            "ライフサイクル監査ID": lifecycle_audit_id,
        });
        let final_hash = canonical_payload_hash(Some(&body));
        let final_reason = format!(
            "lifecycle遷移確定 Capability=runtime.lifecycle.{} Permission=permission.runtime.lifecycle.{} Approval={} Recovery=recover-runtime-lifecycle-{}",
            transition.action.as_str(),
            transition.action.as_str(),
            transition.approval_id,
            transition.action.as_str(),
        );
        let audit_event = match self.append_audit(
            request_id,
            operation.as_str(),
            "accepted",
            &final_reason,
            EVIDENCE_SOURCE_LIVE_RUNTIME,
            &final_hash,
        ) {
            Ok(event) if event.event_id == lifecycle_audit_id => event,
            Ok(_) => {
                self.ライフサイクル.fail_closed();
                return self.audit_store_failed_response(
                    request_id,
                    operation.as_str(),
                    "lifecycle_audit_id_mismatch",
                    "lifecycle最終監査IDを確定できない",
                );
            }
            Err(error) => {
                self.ライフサイクル.fail_closed();
                return self.audit_store_failed_response(
                    request_id,
                    operation.as_str(),
                    "broker_audit_append_failed",
                    &error.message(),
                )
            }
        };
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: operation.as_str().to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_LIVE_RUNTIME.to_string(),
            audit_event_id: audit_event.event_id,
            error: None,
            health: None,
            body: Some(body),
            shutdown_requested: self.shutdown_requested,
        }
    }

    fn lifecycle_reject(
        &mut self,
        request_id: &str,
        operation: BrokerOperation,
        error: LifecycleError,
        payload_hash: &str,
    ) -> BrokerResponse {
        self.reject_with_payload_hash(
            request_id,
            operation.as_str(),
            error.code,
            &error.message,
            true,
            payload_hash,
        )
    }

    fn 実行系資源観測要求処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        payload_hash: &str,
    ) -> BrokerResponse {
        let operation = BrokerOperation::実行系資源観測.as_str();
        let query: 実行系資源観測指定 = match serde_json::from_value::<実行系資源観測指定>(payload.clone()) {
            Ok(query) if query.version == 1 && 実行系ID妥当(&query.runtime_id) => query,
            _ => {
                return self.reject_with_payload_hash(
                    request_id,
                    operation,
                    "要求不正",
                    "実行系資源観測は版と実行系IDだけを受け付けます",
                    true,
                    payload_hash,
                )
            }
        };
        if super::adapter_center::runtime_is_quarantined(self, &query.runtime_id) {
            return self.reject_with_payload_hash(request_id, operation, "adapter_quarantined", "隔離済みAdapterの資源観測を拒否しました", true, payload_hash);
        }
        let _initial = match self.append_audit(
            request_id,
            operation,
            "received",
            "Capability=runtime.resource.observe Permission=permission.runtime.resource.observe Approval=not_required Recovery=recover-runtime-resource-binding",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        ) {
            Ok(event) => event,
            Err(error) => {
                return self.audit_store_failed_response(
                    request_id,
                    operation,
                    "broker_audit_append_failed",
                    &error.message(),
                )
            }
        };
        let stats = self.対話.資源統計(&query.runtime_id);
        let mut observation = match self.資源観測.observe(
            &query.runtime_id,
            stats,
            self.current_epoch_millis(),
        ) {
            Ok(observation) => observation,
            Err(ResourceObservationError::UnknownRuntime) => {
                return self.reject_with_payload_hash(
                    request_id,
                    operation,
                    "実行系不在",
                    "登録済み実行系だけを資源観測できます",
                    true,
                    payload_hash,
                )
            }
        };
        let observation_audit_id = self.audit_log.next_event_id();
        let Some(body) = observation.body.as_object_mut() else {
            self.資源観測.clear();
            return self.audit_store_failed_response(
                request_id,
                operation,
                "broker_resource_observation_invalid",
                "実行系資源観測の内部構造を確定できません",
            );
        };
        body.insert(
            "観測監査ID".to_string(),
            Value::String(observation_audit_id.clone()),
        );
        let observation_hash = canonical_payload_hash(Some(&observation.body));
        let accepted = match self.append_audit(
            request_id,
            operation,
            "accepted",
            "実行系資源観測結果を確定。観測値は権限や実行可能性を生成しない",
            observation.evidence_source,
            &observation_hash,
        ) {
            Ok(event) => event,
            Err(error) => {
                return self.audit_store_failed_response(
                    request_id,
                    operation,
                    "broker_audit_append_failed",
                    &error.message(),
                )
            }
        };
        if accepted.event_id != observation_audit_id {
            self.資源観測.clear();
            return self.audit_store_failed_response(
                request_id,
                operation,
                "broker_resource_observation_audit_mismatch",
                "実行系資源観測の監査IDを確定できません",
            );
        }
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: operation.to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: observation.evidence_source.to_string(),
            audit_event_id: accepted.event_id,
            error: None,
            health: None,
            body: Some(observation.body),
            shutdown_requested: self.shutdown_requested,
        }
    }

    fn 対話要求処理(&mut self, request_id: &str, operation: BrokerOperation, payload: &Value, owner: bool, payload_hash: &str) -> BrokerResponse {
        if !self.state_store.persistence_ready() {
            return self.reject_with_payload_hash(request_id, operation.as_str(), "broker_persistence_unavailable", "対話には永続監査が必要", true, payload_hash);
        }
        if matches!(operation, BrokerOperation::対話承認 | BrokerOperation::対話承認待ち) && !owner {
            return self.reject_with_payload_hash(request_id, operation.as_str(), "権限拒否", "owner制御資格が必要", true, payload_hash);
        }
        if payload.get("実行系ID").or_else(|| payload.get("agent_runtime_id")).and_then(Value::as_str).is_some_and(|runtime_id| super::adapter_center::runtime_is_quarantined(self, runtime_id)) {
            return self.reject_with_payload_hash(request_id, operation.as_str(), "adapter_quarantined", "隔離済みAdapterの対話操作を拒否しました", true, payload_hash);
        }
        let initial = self.append_audit(request_id, operation.as_str(), "received", "対話操作を受信", EVIDENCE_SOURCE_INTERNAL_STATE, payload_hash);
        let mut last_event = match initial {
            Ok(event) => event.event_id,
            Err(e) => return self.audit_store_failed_response(request_id, operation.as_str(), "broker_audit_append_failed", &e.message()),
        };
        let workspace_binding = if matches!(operation, BrokerOperation::対話開始 | BrokerOperation::Agent作業要求検査 | BrokerOperation::AgentTaskWorkspacePermissionGrant | BrokerOperation::AgentTaskOwnerApprovalGrant | BrokerOperation::AgentTask実行) {
            let (runtime, workspace) = if operation == BrokerOperation::対話開始 {
                (payload.get("実行系ID"), payload.get("作業領域ID"))
            } else {
                (payload.get("agent_runtime_id"), payload.get("workspace_id"))
            };
            match (runtime.and_then(Value::as_str), workspace.and_then(Value::as_str)) {
                (Some(runtime_id), Some(workspace_id)) => {
                    let Some(binding) = self.作業領域.dialogue_binding(runtime_id, workspace_id) else {
                        return self.reject_with_payload_hash(
                            request_id,
                            operation.as_str(),
                            "作業領域不在",
                            "指定Runtimeに結合できる登録済みWorkspaceがありません",
                            true,
                            payload_hash,
                        );
                    };
                    Some(binding)
                }
                _ => None,
            }
        } else {
            None
        };
        let mut 対話 = std::mem::take(&mut self.対話);
        let now = self.current_epoch_seconds();
        let result = 対話.操作_作業領域結合済み_scratch(operation.as_str(), payload, owner, now, workspace_binding.as_ref(), self.agent_task_scratch.clone(), &mut |reason, id, hash| {
            let event = self.append_audit(id, operation.as_str(), "recorded", reason, EVIDENCE_SOURCE_INTERNAL_STATE, hash).map_err(|_| 対話失敗::監査失敗)?;
            last_event = event.event_id.clone(); Ok(event.event_id)
        });
        self.対話 = 対話;
        match result {
            Ok(body) => BrokerResponse { request_id: request_id.into(), operation: operation.as_str().into(), status: BrokerStatus::Accepted, evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.into(), audit_event_id: last_event, error: None, health: None, body: Some(body), shutdown_requested: false },
            Err(対話失敗::監査失敗) => self.audit_store_failed_response(request_id, operation.as_str(), "broker_audit_append_failed", "対話の監査を確定できない"),
            Err(e) => self.reject_with_payload_hash(request_id, operation.as_str(), e.分類(), e.復旧(), true, payload_hash),
        }
    }

    pub fn handle_json(&mut self, input: &str) -> BrokerResponse {
        match BrokerRequestEnvelope::from_json_str(input) {
            Ok(envelope) => self.handle(envelope),
            Err(_) => self.reject_with_payload_hash(
                "malformed-request",
                "unknown",
                "broker_request_malformed",
                "broker request JSON failed to parse or included unknown fields",
                true,
                &sha256_tagged(input.as_bytes()),
            ),
        }
    }

    pub fn audit_events(&self) -> &[BrokerAuditEvent] {
        self.audit_log.events()
    }

    pub fn shutdown_requested(&self) -> bool {
        self.shutdown_requested
    }

    pub fn reject_ipc(&mut self, code: &str, message: &str, recoverable: bool) -> BrokerResponse {
        self.reject("ipc-request", "unknown", code, message, recoverable)
    }

    fn current_epoch_seconds(&self) -> i64 {
        self.current_epoch_seconds_override
            .unwrap_or_else(current_epoch_seconds)
    }

    pub(super) fn current_epoch_millis(&self) -> i64 {
        self.current_epoch_seconds().max(0).saturating_mul(1_000)
    }

    fn accept_health(&mut self, request_id: &str, payload_hash: &str) -> BrokerResponse {
        let audit_event = match self.append_audit(
            request_id,
            BrokerOperation::Health.as_str(),
            "accepted",
            "health status returned",
            EVIDENCE_SOURCE_LIVE_RUNTIME,
            payload_hash,
        ) {
            Ok(event) => event,
            Err(error) => {
                return self.audit_store_failed_response(
                    request_id,
                    BrokerOperation::Health.as_str(),
                    "broker_audit_append_failed",
                    &error.message(),
                )
            }
        };
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: BrokerOperation::Health.as_str().to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_LIVE_RUNTIME.to_string(),
            audit_event_id: audit_event.event_id,
            error: None,
            health: Some(self.health(EVIDENCE_SOURCE_LIVE_RUNTIME)),
            body: None,
            shutdown_requested: false,
        }
    }

    fn first_run_configuration(
        &mut self,
        request_id: &str,
        payload: &Value,
        payload_hash: &str,
    ) -> BrokerResponse {
        match serde_json::from_value::<FirstRunConfigurationRequest>(payload.clone()) {
            Ok(request) if request.version == 1 => {}
            _ => {
                return self.reject_with_payload_hash(
                    request_id,
                    BrokerOperation::初回設定取得.as_str(),
                    "first_run_configuration_request_invalid",
                    "初回設定取得要求はversion 1だけを受け付けます",
                    true,
                    payload_hash,
                );
            }
        }
        let Some((configuration, bytes)) = self.desktop_first_run_configuration.clone() else {
            return self.reject_with_payload_hash(
                request_id,
                BrokerOperation::初回設定取得.as_str(),
                "first_run_configuration_unavailable",
                "Brokerが検証済み初回設定を保持していません。Desktop起動器から再起動してください。",
                true,
                payload_hash,
            );
        };
        let operation = BrokerOperation::初回設定取得.as_str();
        let receipt_reason = "Capability=製品初回設定 Permission=固定store内の初期設定一fileをread-only取得 Approval=不要。権限を生成しない RecoveryAction=取得失敗時はBroker固定storeを確認";
        if let Err(error) = self.append_audit(
            request_id,
            operation,
            "received",
            receipt_reason,
            EVIDENCE_SOURCE_LIVE_RUNTIME,
            payload_hash,
        ) {
            return self.audit_store_failed_response(
                request_id,
                operation,
                "broker_audit_append_failed",
                &error.message(),
            );
        }
        let content_hash = sha256_tagged(&bytes);
        let audit_event = match self.append_audit(
            request_id,
            operation,
            "accepted",
            "初回UI設定を固定Broker storeからprojection。authorityは生成しない",
            EVIDENCE_SOURCE_LIVE_RUNTIME,
            &content_hash,
        ) {
            Ok(event) => event,
            Err(error) => {
                return self.audit_store_failed_response(
                    request_id,
                    operation,
                    "broker_audit_append_failed",
                    &error.message(),
                );
            }
        };
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: operation.to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_LIVE_RUNTIME.to_string(),
            audit_event_id: audit_event.event_id,
            error: None,
            health: None,
            body: Some(configuration),
            shutdown_requested: self.shutdown_requested,
        }
    }

    fn setup_doctor_report(
        &mut self,
        request_id: &str,
        payload: &Value,
        payload_hash: &str,
    ) -> BrokerResponse {
        match serde_json::from_value::<SetupDoctorReportRequest>(payload.clone()) {
            Ok(request) if request.version == 1 => {}
            _ => {
                return self.reject_with_payload_hash(
                    request_id,
                    BrokerOperation::SetupDoctor報告取得.as_str(),
                    "setup_doctor_request_invalid",
                    "Setup Doctor要求はversion 1だけを受け付けます",
                    true,
                    payload_hash,
                );
            }
        }

        let health = self.health(EVIDENCE_SOURCE_LIVE_RUNTIME);
        let checks = vec![
            setup_doctor_check(
                "setup_doctor.ran_from_installed_app_path",
                if self.desktop_install_path_verified { "pass" } else { "unknown" },
                if self.desktop_install_path_verified {
                    "Rust起動器が固定installed package配置を検証しました。"
                } else {
                    "固定installed package配置を実行時に確認できません。"
                },
                "正式installed配置からRust Desktop起動器を開始し、製品file検査を再実行してください。",
                if self.desktop_install_path_verified { EVIDENCE_SOURCE_LIVE_RUNTIME } else { "CONFIG" },
            ),
            setup_doctor_check(
                "setup_doctor.runtime_connection",
                "pass",
                "通常資格で認証されたBroker要求をRust Brokerが処理しています。",
                "Broker接続が切れた場合はD4 Pocketを終了し、Desktop起動器から再起動してください。",
                EVIDENCE_SOURCE_LIVE_RUNTIME,
            ),
            setup_doctor_check(
                "setup_doctor.authority_boundary",
                "pass",
                "Setup Doctor報告は権限を生成せず、installerも権限を付与・承認しません。",
                "権限境界の検査に失敗した場合は権限依存操作を停止し、監査可能なBroker buildへ更新してください。",
                "CONFIG",
            ),
            setup_doctor_check(
                "setup_doctor.network_public_bind",
                if self.desktop_loopback_bind_verified { "pass" } else { "unknown" },
                if self.desktop_loopback_bind_verified {
                    "実際にbindしたBroker IPCはIPv4 loopbackに限定されています。"
                } else {
                    "Brokerの実bind addressをloopback限定として確認できません。"
                },
                "bind範囲を確認できない場合はBrokerを停止し、loopback固定のDesktop起動経路から再起動してください。",
                if self.desktop_loopback_bind_verified { EVIDENCE_SOURCE_LIVE_RUNTIME } else { "INTERNAL_STATE" },
            ),
            setup_doctor_check(
                "setup_doctor.recovery_instruction",
                "pass",
                "各診断checkに個別の復旧案内があります。",
                "復旧案内が欠落するreportを使用せず、製品診断を再実行してください。",
                "CONFIG",
            ),
            setup_doctor_check(
                "setup_doctor.audit_storage",
                if health.persistence_ready { "pass" } else { "warning" },
                if health.persistence_ready {
                    "Brokerは永続Audit storeを使用しています。"
                } else {
                    "Broker永続Audit storeの準備完了を確認できません。"
                },
                "Audit storeを利用できない場合は権限依存操作を停止し、Brokerの固定storeを復旧してください。",
                EVIDENCE_SOURCE_LIVE_RUNTIME,
            ),
            setup_doctor_check(
                "setup_doctor.config_created",
                if self.desktop_first_run_configuration.is_some() { "pass" } else { "unknown" },
                if self.desktop_first_run_configuration.is_some() {
                    "Rust Brokerが固定storeの初回UI設定をSchema検証し、Audit hashへ結合しました。"
                } else {
                    "installed Desktop起動器が検証した初回UI設定を確認できません。"
                },
                "設定状態を確認できない場合はRust Desktop起動器から再起動し、固定Broker storeとAuditを確認してください。",
                if self.desktop_first_run_configuration.is_some() { EVIDENCE_SOURCE_LIVE_RUNTIME } else { "CONFIG" },
            ),
        ];
        let status = if checks.iter().any(|check| check["status"] == "fail") {
            "fail"
        } else if checks.iter().any(|check| {
            matches!(check["status"].as_str(), Some("warning" | "unknown"))
        }) {
            "warning"
        } else {
            "pass"
        };
        let request_hash = sha256_tagged(request_id.as_bytes());
        let report = serde_json::json!({
            "version": 1,
            "report_id": format!("setup-doctor-{}", &request_hash[7..]),
            "generated_at": epoch_seconds_to_rfc3339(self.current_epoch_seconds()),
            "status": status,
            "evidence_source": EVIDENCE_SOURCE_LIVE_RUNTIME,
            "checks": checks,
            "installer_grants_authority": false,
            "installer_silently_approves_permissions": false
        });
        let report_bytes = match serde_json::to_vec(&report) {
            Ok(bytes) if !bytes.is_empty() && bytes.len() <= 64 * 1024 => bytes,
            _ => {
                return self.reject_with_payload_hash(
                    request_id,
                    BrokerOperation::SetupDoctor報告取得.as_str(),
                    "setup_doctor_report_invalid",
                    "Setup Doctor reportを安全な上限内で生成できません",
                    true,
                    payload_hash,
                );
            }
        };
        let operation = BrokerOperation::SetupDoctor報告取得.as_str();
        let receipt_reason = "Capability=diagnostics.report Permission=Broker固定storeの最新診断report一件だけをatomic replace Approval=権限・利用者content・外部共有を含まない診断projectionのためprivileged Approval不要 RecoveryAction=失敗時はreportをrelease evidenceとして使用せずBroker storeを確認";
        if let Err(error) = self.append_audit(
            request_id,
            operation,
            "received",
            receipt_reason,
            EVIDENCE_SOURCE_LIVE_RUNTIME,
            payload_hash,
        ) {
            return self.audit_store_failed_response(
                request_id,
                operation,
                "broker_audit_append_failed",
                &error.message(),
            );
        }
        if self.state_store.write_setup_doctor_report(&report_bytes).is_err() {
            return self.reject_with_payload_hash(
                request_id,
                operation,
                "setup_doctor_report_storage_failed",
                "Setup Doctor reportをBroker固定storeへ安全に保存できません。復旧後に診断を再実行してください。",
                true,
                payload_hash,
            );
        }
        let report_hash = sha256_tagged(&report_bytes);
        let audit_event = match self.append_audit(
            request_id,
            operation,
            "accepted",
            "setup_doctor_report_exported",
            EVIDENCE_SOURCE_LIVE_RUNTIME,
            &report_hash,
        ) {
            Ok(event) => event,
            Err(error) => {
                return self.audit_store_failed_response(
                    request_id,
                    operation,
                    "broker_audit_append_failed",
                    &error.message(),
                );
            }
        };
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: operation.to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_LIVE_RUNTIME.to_string(),
            audit_event_id: audit_event.event_id,
            error: None,
            health: None,
            body: Some(report),
            shutdown_requested: self.shutdown_requested,
        }
    }

    fn ホスト能力処理(
        &mut self,
        request_id: &str,
        payload: &Value,
        payload_hash: &str,
    ) -> BrokerResponse {
        if !payload.is_null() {
            return self.reject_with_payload_hash(
                request_id,
                BrokerOperation::ホスト能力.as_str(),
                "host_capability_request_invalid",
                "ホスト能力照会はpayloadを受け付けない",
                true,
                payload_hash,
            );
        }
        self.accept_body_with_evidence(
            request_id,
            BrokerOperation::ホスト能力,
            super::host_capability::current(),
            EVIDENCE_SOURCE_LIVE_RUNTIME,
            payload_hash,
        )
    }

    fn suspend_health(&mut self, request_id: &str, payload_hash: &str) -> BrokerResponse {
        let audit_event = match self.append_audit(
            request_id,
            BrokerOperation::Health.as_str(),
            "suspended",
            "broker_persistence_unavailable",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        ) {
            Ok(event) => event,
            Err(error) => {
                return self.audit_store_failed_response(
                    request_id,
                    BrokerOperation::Health.as_str(),
                    "broker_audit_append_failed",
                    &error.message(),
                )
            }
        };
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: BrokerOperation::Health.as_str().to_string(),
            status: BrokerStatus::Suspended,
            evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
            audit_event_id: audit_event.event_id,
            error: Some(error(
                "broker_persistence_unavailable",
                self.state_store.unavailable_message(),
                true,
            )),
            health: Some(self.health(EVIDENCE_SOURCE_INTERNAL_STATE)),
            body: None,
            shutdown_requested: false,
        }
    }

    fn health(&self, evidence_source: &str) -> BrokerHealth {
        BrokerHealth {
            broker_id: BROKER_ID.to_string(),
            status: self.state_store.health_status().to_string(),
            boundary_role: "rust_security_broker_candidate".to_string(),
            authority_cutover_status: "not_active".to_string(),
            command_dispatch_enabled: false,
            audit_append_enabled: true,
            audit_persistence: self.state_store.audit_persistence().to_string(),
            replay_persistence: self.state_store.replay_persistence().to_string(),
            session_persistence: self.state_store.session_persistence().to_string(),
            persistence_required: self.state_store.persistence_required(),
            persistence_ready: self.state_store.persistence_ready(),
            evidence_source: evidence_source.to_string(),
        }
    }

    fn accept_shutdown(&mut self, request_id: &str, payload_hash: &str) -> BrokerResponse {
        self.shutdown_requested = true;
        let audit_event = match self.append_audit(
            request_id,
            BrokerOperation::Shutdown.as_str(),
            "accepted",
            "shutdown requested",
            EVIDENCE_SOURCE_LIVE_RUNTIME,
            payload_hash,
        ) {
            Ok(event) => event,
            Err(error) => {
                return self.audit_store_failed_response(
                    request_id,
                    BrokerOperation::Shutdown.as_str(),
                    "broker_audit_append_failed",
                    &error.message(),
                )
            }
        };
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: BrokerOperation::Shutdown.as_str().to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_LIVE_RUNTIME.to_string(),
            audit_event_id: audit_event.event_id,
            error: None,
            health: None,
            body: None,
            shutdown_requested: true,
        }
    }

    fn suspend_command(
        &mut self,
        request_id: &str,
        payload: &Value,
        payload_hash: &str,
    ) -> BrokerResponse {
        let audit_event = match self.append_audit(
            request_id,
            BrokerOperation::CommandEnvelope.as_str(),
            "suspended",
            "external command dispatch disabled in broker skeleton",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        ) {
            Ok(event) => event,
            Err(error) => {
                return self.audit_store_failed_response(
                    request_id,
                    BrokerOperation::CommandEnvelope.as_str(),
                    "broker_audit_append_failed",
                    &error.message(),
                )
            }
        };
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: BrokerOperation::CommandEnvelope.as_str().to_string(),
            status: BrokerStatus::Suspended,
            evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
            audit_event_id: audit_event.event_id,
            error: Some(error(
                "broker_command_dispatch_disabled",
                "external command dispatch is disabled until authority migration tests pass",
                true,
            )),
            health: None,
            body: Some(json_command_eligibility(payload, &self.authority_registry)),
            shutdown_requested: self.shutdown_requested,
        }
    }

    fn accept_authority_decision(
        &mut self,
        request_id: &str,
        body: Value,
        payload_hash: &str,
    ) -> BrokerResponse {
        let decision = body
            .get("decision")
            .and_then(Value::as_str)
            .unwrap_or("denied");
        let audit_event = match self.append_audit(
            request_id,
            BrokerOperation::AuthorityEvaluate.as_str(),
            decision,
            "broker-owned authority decision evaluated",
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        ) {
            Ok(event) => event,
            Err(error) => {
                return self.audit_store_failed_response(
                    request_id,
                    BrokerOperation::AuthorityEvaluate.as_str(),
                    "broker_audit_append_failed",
                    &error.message(),
                )
            }
        };
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: BrokerOperation::AuthorityEvaluate.as_str().to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
            audit_event_id: audit_event.event_id,
            error: None,
            health: None,
            body: Some(body),
            shutdown_requested: self.shutdown_requested,
        }
    }

    fn accept_body(
        &mut self,
        request_id: &str,
        operation: BrokerOperation,
        body: Value,
        payload_hash: &str,
    ) -> BrokerResponse {
        self.accept_body_with_evidence(
            request_id,
            operation,
            body,
            EVIDENCE_SOURCE_LIVE_RUNTIME,
            payload_hash,
        )
    }

    fn accept_body_with_evidence(
        &mut self,
        request_id: &str,
        operation: BrokerOperation,
        body: Value,
        evidence_source: &str,
        payload_hash: &str,
    ) -> BrokerResponse {
        let operation_name = operation.as_str();
        let audit_event = match self.append_audit(
            request_id,
            operation_name,
            "accepted",
            "broker authority operation evaluated",
            evidence_source,
            payload_hash,
        ) {
            Ok(event) => event,
            Err(error) => {
                return self.audit_store_failed_response(
                    request_id,
                    operation_name,
                    "broker_audit_append_failed",
                    &error.message(),
                )
            }
        };
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: operation_name.to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: evidence_source.to_string(),
            audit_event_id: audit_event.event_id,
            error: None,
            health: None,
            body: Some(body),
            shutdown_requested: self.shutdown_requested,
        }
    }

    fn reject(
        &mut self,
        request_id: &str,
        operation: &str,
        code: &str,
        message: &str,
        recoverable: bool,
    ) -> BrokerResponse {
        self.reject_with_payload_hash(
            request_id,
            operation,
            code,
            message,
            recoverable,
            ZERO_PAYLOAD_HASH,
        )
    }

    pub(super) fn reject_with_payload_hash(
        &mut self,
        request_id: &str,
        operation: &str,
        code: &str,
        message: &str,
        recoverable: bool,
        payload_hash: &str,
    ) -> BrokerResponse {
        let audit_event = match self.append_audit(
            request_id,
            operation,
            "rejected",
            code,
            EVIDENCE_SOURCE_INTERNAL_STATE,
            payload_hash,
        ) {
            Ok(event) => event,
            Err(error) => {
                return self.audit_store_failed_response(
                    request_id,
                    operation,
                    "broker_audit_append_failed",
                    &error.message(),
                )
            }
        };
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: operation.to_string(),
            status: BrokerStatus::Rejected,
            evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
            audit_event_id: audit_event.event_id,
            error: Some(error(code, message, recoverable)),
            health: None,
            body: None,
            shutdown_requested: self.shutdown_requested,
        }
    }

    pub(super) fn append_audit(
        &mut self,
        request_id: &str,
        operation: &str,
        decision: &str,
        reason: &str,
        evidence_source: &str,
        payload_hash: &str,
    ) -> Result<BrokerAuditEvent, BrokerStoreError> {
        let observation_started = Instant::now();
        let observation_started_at = self.current_epoch_millis();
        let event = self.audit_log.build_next(
            request_id,
            operation,
            decision,
            reason,
            evidence_source,
            payload_hash,
        );
        if let Err(error) = self.state_store.append_audit_event(&event) {
            self.作業領域 = Default::default();
            self.資源観測.clear();
            self.ライフサイクル.fail_closed();
            #[cfg(windows)]
            self.内容閲覧.revoke();
            return Err(error);
        }
        if let Err(error) = self.audit_log.push_verified(event.clone()) {
            self.作業領域 = Default::default();
            self.資源観測.clear();
            self.ライフサイクル.fail_closed();
            #[cfg(windows)]
            self.内容閲覧.revoke();
            return Err(BrokerStoreError::TamperedAuditState(error));
        }
        self.observations.record_completed_span(
            &event,
            observation_started_at,
            observation_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        );
        Ok(event)
    }

    fn record_nonce(&mut self, nonce: &str) -> Result<(), BrokerStoreError> {
        let recorded_at = current_epoch_seconds();
        let persisted = match self.state_store.append_replay_nonce(nonce, recorded_at) {
            Ok(value) => value,
            Err(error) => {
                self.作業領域 = Default::default();
                self.ライフサイクル.fail_closed();
                return Err(error);
            }
        };
        if let Some(nonces) = persisted {
            self.seen_nonces = nonces;
        } else {
            self.seen_nonces.insert(nonce.to_string(), recorded_at);
        }
        Ok(())
    }

    pub(super) fn audit_store_failed_response(
        &self,
        request_id: &str,
        operation: &str,
        code: &str,
        message: &str,
    ) -> BrokerResponse {
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: operation.to_string(),
            status: BrokerStatus::Suspended,
            evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
            audit_event_id: "broker-audit-unavailable".to_string(),
            error: Some(error(code, message, true)),
            health: None,
            body: None,
            shutdown_requested: self.shutdown_requested,
        }
    }
}

fn error(code: &str, message: &str, recoverable: bool) -> BrokerError {
    BrokerError {
        code: code.to_string(),
        message: message.to_string(),
        recoverable,
        audit_event_required: true,
        fail_closed: true,
    }
}

fn json_command_eligibility(payload: &Value, registry: &BrokerAuthorityRegistry) -> Value {
    let target_kind = command_target_kind(payload);
    serde_json::json!({
        "dispatch_enabled": false,
        "dispatch_decision": "suspended",
        "dispatch_reason": "broker_command_dispatch_disabled",
        "execution_gate": {
            "status": "suspended",
            "target_kind": target_kind,
            "dispatch": "suspended",
            "process": execution_gate_state(target_kind, "process"),
            "credential": execution_gate_state(target_kind, "credential"),
            "update": execution_gate_state(target_kind, "update"),
            "required_before_dispatch": [
                "capability_permission_approval_audit_recovery_eligibility",
                "process_execution_gate",
                "credential_access_gate",
                "update_signature_gate",
                "installed_product_evidence"
            ]
        },
        "eligibility": evaluate_broker_authority(registry, payload)
    })
}

fn execution_gate_state(target_kind: &str, gate: &str) -> &'static str {
    if target_kind == gate {
        "suspended"
    } else {
        "not_requested"
    }
}

fn command_target_kind(payload: &Value) -> &'static str {
    let action = payload.get("action").unwrap_or(payload);
    let mut text = String::new();
    for key in [
        "operation",
        "capability_id",
        "permission_id",
        "runtime_id",
        "command",
        "target",
    ] {
        if let Some(value) = action.get(key).and_then(Value::as_str) {
            text.push(' ');
            text.push_str(value);
        }
    }
    if let Some(action_payload) = action.get("payload") {
        collect_target_text(action_payload, &mut text);
    }
    let lowered = text.to_ascii_lowercase();
    if lowered.contains("credential") || lowered.contains("keychain") || lowered.contains("secret")
    {
        "credential"
    } else if lowered.contains("update") || lowered.contains("installer") {
        "update"
    } else if lowered.contains("process")
        || lowered.contains("spawn")
        || lowered.contains("execute")
        || lowered.contains("command")
    {
        "process"
    } else if lowered.contains("filesystem") {
        "filesystem"
    } else if lowered.contains("network") {
        "network"
    } else if lowered.contains("runtime") {
        "runtime"
    } else {
        "unknown"
    }
}

fn collect_target_text(value: &Value, text: &mut String) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                text.push(' ');
                text.push_str(key);
                collect_target_text(value, text);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_target_text(item, text);
            }
        }
        Value::String(value) => {
            text.push(' ');
            text.push_str(value);
        }
        _ => {}
    }
}

fn is_tagged_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value
            .as_bytes()
            .iter()
            .skip(7)
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

/// approval IDは外部targetや統治情報を埋め込めないASCII識別子に限る。
fn lifecycle_identifier_valid(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 255
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(byte))
}

pub(crate) fn canonical_payload_hash(payload: Option<&Value>) -> String {
    let encoded =
        serde_json::to_vec(payload.unwrap_or(&Value::Null)).unwrap_or_else(|_| b"null".to_vec());
    sha256_tagged(&encoded)
}

fn issued_at_is_fresh(value: &str, current_epoch_seconds: i64) -> bool {
    let Some(issued_epoch_seconds) = parse_issued_at_epoch_seconds(value) else {
        return false;
    };
    issued_epoch_seconds.abs_diff(current_epoch_seconds) <= REQUEST_FRESHNESS_WINDOW_SECONDS
}

pub(crate) fn request_issued_at_is_current(value: &str) -> bool {
    issued_at_is_fresh(value, current_epoch_seconds())
}

fn parse_issued_at_epoch_seconds(value: &str) -> Option<i64> {
    let date_time = value.as_bytes();
    if date_time.len() != 20 && date_time.len() != 25 {
        return None;
    }
    if date_time.get(4) != Some(&b'-')
        || date_time.get(7) != Some(&b'-')
        || !matches!(date_time.get(10), Some(b'T') | Some(b't'))
        || date_time.get(13) != Some(&b':')
        || date_time.get(16) != Some(&b':')
    {
        return None;
    }

    let year = parse_digits(value, 0, 4)? as i32;
    let month = parse_digits(value, 5, 7)? as u32;
    let day = parse_digits(value, 8, 10)? as u32;
    let hour = parse_digits(value, 11, 13)? as u32;
    let minute = parse_digits(value, 14, 16)? as u32;
    let second = parse_digits(value, 17, 19)? as u32;
    if !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }

    let offset_seconds = if date_time.len() == 20 {
        if !matches!(date_time.get(19), Some(b'Z') | Some(b'z')) {
            return None;
        }
        0
    } else {
        let sign = match date_time.get(19) {
            Some(b'+') => 1,
            Some(b'-') => -1,
            _ => return None,
        };
        if date_time.get(22) != Some(&b':') {
            return None;
        }
        let offset_hour = parse_digits(value, 20, 22)? as i64;
        let offset_minute = parse_digits(value, 23, 25)? as i64;
        if offset_hour > 23 || offset_minute > 59 {
            return None;
        }
        sign * ((offset_hour * 3600) + (offset_minute * 60))
    };

    let local_epoch = days_from_civil(year, month, day)
        .checked_mul(86_400)?
        .checked_add((hour as i64) * 3600)?
        .checked_add((minute as i64) * 60)?
        .checked_add(second as i64)?;
    local_epoch.checked_sub(offset_seconds)
}

fn current_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn epoch_seconds_to_rfc3339(epoch_seconds: i64) -> String {
    let days = epoch_seconds.div_euclid(86_400);
    let seconds_of_day = epoch_seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn setup_doctor_check(
    check_id: &str,
    status: &str,
    message: &str,
    recovery_instruction: &str,
    evidence_class: &str,
) -> Value {
    serde_json::json!({
        "check_id": check_id,
        "status": status,
        "message": message,
        "recovery_instruction": recovery_instruction,
        "evidence_class": evidence_class,
        "grants_authority": false
    })
}

fn parse_digits(value: &str, start: usize, end: usize) -> Option<u32> {
    value.get(start..end)?.parse().ok()
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - (era * 400);
    let month_prime = month as i32 + if month > 2 { -3 } else { 9 };
    let day_of_year = ((153 * month_prime + 2) / 5) + day as i32 - 1;
    let day_of_era = (year_of_era * 365) + (year_of_era / 4) - (year_of_era / 100) + day_of_year;
    (era as i64 * 146_097) + day_of_era as i64 - 719_468
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - (era * 146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i32 + era as i32 * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + i32::from(month <= 2);
    (year, month as u32, day as u32)
}

fn metadata_attempts_authority(metadata: &[BrokerMetadata]) -> bool {
    metadata.iter().any(|item| {
        let key = normalize_key(&item.key);
        let value = normalize_key(&item.value);
        canonical_authority_key(&key).is_some()
            || authority_value_present(&value)
            || authority_token_present(&value)
    })
}

pub(crate) fn metadata_attempts_authority_value(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            let key = normalize_authority_token(key);
            canonical_authority_key(&key).is_some() || metadata_attempts_authority_value(value)
        }),
        Value::Array(items) => items.iter().any(metadata_attempts_authority_value),
        Value::String(value) => {
            let value = normalize_authority_token(value);
            authority_value_present(&value) || authority_token_present(&value)
        }
        _ => false,
    }
}

fn canonical_authority_key(normalized: &str) -> Option<&'static str> {
    match normalized {
        "authority" => Some("authority"),
        "authority_context" | "admin_context" => Some("authority_context"),
        "authority_trace" => Some("authority_trace"),
        "approval_state" => Some("approval_state"),
        "approved_by" => Some("approved_by"),
        "permission"
        | "permissions"
        | "permissions_granted"
        | "permissiongrant"
        | "permission_grant"
        | "grant"
        | "grants"
        | "privilege"
        | "privileges" => Some("permission_grant"),
        "permission_override" => Some("permission_override"),
        "role" => Some("role"),
        "scope_escalation" | "elevated" => Some("scope_escalation"),
        "trust_level" => Some("trust_level"),
        _ => None,
    }
}

fn authority_value_present(normalized: &str) -> bool {
    matches!(
        normalized,
        "admin" | "all" | "approved" | "elevated" | "root"
    )
}

fn authority_token_present(normalized: &str) -> bool {
    normalized
        .split('_')
        .any(|token| canonical_authority_key(token).is_some() || authority_value_present(token))
}

fn json_metadata_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value)
            .unwrap_or_else(|_| "unserializable_metadata_value".to_string()),
    }
}

pub(crate) fn normalize_authority_token(value: &str) -> String {
    let mut result = String::new();
    let mut previous_was_underscore = false;
    let mut previous_was_lower_or_digit = false;
    let canonicalized: String = value.nfkc().collect();
    for character in canonicalized
        .replace(['\u{200b}', '\u{200c}', '\u{200d}', '\u{feff}'], "")
        .trim()
        .chars()
    {
        if character.is_ascii_uppercase() && previous_was_lower_or_digit && !previous_was_underscore
        {
            result.push('_');
        }
        if character.is_ascii_alphanumeric() {
            result.push(character.to_ascii_lowercase());
            previous_was_underscore = false;
            previous_was_lower_or_digit =
                character.is_ascii_lowercase() || character.is_ascii_digit();
        } else if !previous_was_underscore {
            result.push('_');
            previous_was_underscore = true;
            previous_was_lower_or_digit = false;
        }
    }
    result.trim_matches('_').to_string()
}

fn normalize_key(value: &str) -> String {
    normalize_authority_token(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cap_fs_ext::DirExt;
    use serde_json::json;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    struct TestChildProcess(Child);

    impl Drop for TestChildProcess {
        fn drop(&mut self) {
            if self.0.try_wait().ok().flatten().is_none() {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }

    fn test_broker() -> Broker {
        Broker::new_with_current_epoch_seconds(
            "session-1",
            parse_issued_at_epoch_seconds("2026-06-01T00:00:30Z").unwrap(),
        )
    }

    fn resource_request(
        request_id: &str,
        session_id: &str,
        nonce: &str,
        payload: Value,
    ) -> BrokerRequestEnvelope {
        let mut request = BrokerRequestEnvelope::command_envelope_at(
            request_id,
            session_id,
            nonce,
            &BrokerRequestEnvelope::current_issued_at(),
        );
        request.operation = Some(BrokerOperation::実行系資源観測);
        request.payload = Some(payload);
        request.refresh_payload_hash();
        request
    }

    #[cfg(windows)]
    fn resource_broker_with_live_loopback_listener() -> (Broker, std::net::TcpListener) {
        use std::net::{Ipv4Addr, TcpListener};
        use std::thread;
        use std::time::Duration;

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = match listener.local_addr().unwrap() {
            std::net::SocketAddr::V4(address) => address,
            std::net::SocketAddr::V6(_) => panic!("IPv4 listenerからIPv6 addressが返った"),
        };
        let mut visible = false;
        for _ in 0..50 {
            match gui_shell_windows_runtime_observation::loopback_tcp_listener_owner_pid(address) {
                Ok(Some(pid)) if pid == std::process::id() => {
                    visible = true;
                    break;
                }
                Ok(_) => thread::sleep(Duration::from_millis(10)),
                Err(error) => panic!("loopback listener所有processの照合に失敗: {error:?}"),
            }
        }
        assert!(visible, "登録前にloopback listener所有processを確認できなかった");
        let mut broker = Broker::new("resource-session");
        let adapter = crate::adapters::minidora::MinidoraAdapter::new(&address.to_string()).unwrap();
        broker.実行系登録("local", Arc::new(adapter)).unwrap();
        (broker, listener)
    }

    fn persistence_required_broker() -> Broker {
        let mut broker = Broker::new_requiring_persistence("session-1");
        broker.current_epoch_seconds_override =
            Some(parse_issued_at_epoch_seconds("2026-06-01T00:00:30Z").unwrap());
        broker
    }

    pub(super) fn temp_store_dir(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "gui-shell-broker-{test_name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn seed_agent_task_scratch_for_crash(root: &Path, session_id: &str) -> (Broker, String) {
        let store_root = root.join("store");
        let workspace_root = root.join("workspace");
        fs::create_dir_all(&workspace_root).expect("作業領域を作成");
        let protected = vec![store_root.clone()];
        let workspace_config = super::super::workspace_root::WorkspaceStartup {
            runtime_id: "fixture-runtime".into(),
            workspace_id: "fixture-workspace".into(),
            root_path: workspace_root.to_string_lossy().into_owned(),
            secret_paths: Vec::new(),
        };

        let mut broker = Broker::new_persistent(session_id, &store_root).unwrap();
        broker
            .実行系登録(
                "fixture-runtime",
                Arc::new(
                    crate::adapters::minidora::MinidoraAdapter::new("127.0.0.1:9").unwrap(),
                ),
            )
            .unwrap();
        broker
            .作業領域起動登録(&workspace_config, &protected)
            .expect("BrokerでWorkspaceを登録");
        let binding = broker
            .作業領域
            .dialogue_binding("fixture-runtime", "fixture-workspace")
            .expect("登録Workspace binding");
        let journal = broker.agent_task_scratch.clone().expect("永続回復journal");
        let scratch_name = format!(".d4p-tmp-{}", "a".repeat(32));
        let record_id = journal
            .reserve(
                "fixture-task",
                "fixture-runtime",
                "fixture-workspace",
                binding.recovery_binding_hash(),
                binding.root_directory_identity(),
                &scratch_name,
            )
            .expect("scratch作成前に回復記録を予約");
        let workspace = cap_std::fs::Dir::open_ambient_dir(
            &workspace_root,
            cap_std::ambient_authority(),
        )
        .expect("Workspaceを開く");
        workspace
            .create_dir(&scratch_name)
            .expect("scratchを作成");
        let scratch = workspace
            .open_dir_nofollow(&scratch_name)
            .expect("nofollowでscratchを開く");
        let metadata = scratch.dir_metadata().expect("scratch属性");
        journal
            .activate(
                &record_id,
                super::super::workspace_root::DirectoryIdentity {
                    device: cap_fs_ext::MetadataExt::dev(&metadata),
                    file_id: cap_fs_ext::MetadataExt::ino(&metadata),
                },
            )
            .expect("実体識別後に回復記録を有効化");
        drop(scratch);
        drop(workspace);
        (broker, scratch_name)
    }

    pub(super) fn persistent_test_broker(store_dir: &Path) -> Broker {
        let mut broker = Broker::new_persistent("session-1", store_dir).unwrap();
        broker.current_epoch_seconds_override =
            Some(parse_issued_at_epoch_seconds("2026-06-01T00:00:30Z").unwrap());
        broker
    }

    fn setup_doctor_request(request_id: &str, payload: Value) -> BrokerRequestEnvelope {
        let mut request = BrokerRequestEnvelope::health(request_id, &format!("nonce-{request_id}"));
        request.session_id = Some("setup-doctor-session".to_string());
        request.operation = Some(BrokerOperation::SetupDoctor報告取得);
        request.payload = Some(payload);
        request.issued_at = Some(BrokerRequestEnvelope::current_issued_at());
        request.refresh_payload_hash();
        request
    }

    fn first_run_configuration_request(
        request_id: &str,
        payload: Value,
    ) -> BrokerRequestEnvelope {
        let mut request = BrokerRequestEnvelope::health(request_id, &format!("nonce-{request_id}"));
        request.session_id = Some("first-run-session".to_string());
        request.operation = Some(BrokerOperation::初回設定取得);
        request.payload = Some(payload);
        request.issued_at = Some(BrokerRequestEnvelope::current_issued_at());
        request.refresh_payload_hash();
        request
    }

    // 子processはBroker library試験用fixtureであり、production broker-server／IPCの証拠ではない。
    #[test]
    fn broker強制終了後の別process起動登録で永続scratchを監査付き回収する() {
        const CHILD_ROOT_ENV: &str = "GUI_SHELL_AGENT_TASK_SCRATCH_CRASH_TEST_ROOT";
        if let Some(root) = std::env::var_os(CHILD_ROOT_ENV).map(PathBuf::from) {
            let (broker, _) =
                seed_agent_task_scratch_for_crash(&root, "scratch-session-crash-child");
            fs::write(
                root.join("child-ready.pid"),
                std::process::id().to_string(),
            )
            .expect("journal保存後の準備完了札");
            loop {
                std::hint::black_box(&broker);
                std::thread::sleep(Duration::from_secs(1));
            }
        }

        let root = temp_store_dir("agent-task-scratch-startup-recovery");
        let store_root = root.join("store");
        let workspace_root = root.join("workspace");
        fs::create_dir(&workspace_root).expect("作業領域を作成");
        let protected = vec![store_root.clone()];
        let workspace_config = || super::super::workspace_root::WorkspaceStartup {
            runtime_id: "fixture-runtime".into(),
            workspace_id: "fixture-workspace".into(),
            root_path: workspace_root.to_string_lossy().into_owned(),
            secret_paths: Vec::new(),
        };
        let scratch_name = format!(".d4p-tmp-{}", "a".repeat(32));

        let child = Command::new(std::env::current_exe().expect("現在の試験実行file"))
            .arg("broker強制終了後の別process起動登録で永続scratchを監査付き回収する")
            .arg("--nocapture")
            .env(CHILD_ROOT_ENV, &root)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Broker fixture子processを起動");
        let mut child = TestChildProcess(child);
        let child_pid = child.0.id();
        let ready_path = root.join("child-ready.pid");
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Ok(contents) = fs::read_to_string(&ready_path) {
                let ready_pid = contents.parse::<u32>().expect("子processの準備完了PID");
                assert_eq!(ready_pid, child_pid, "別processの準備完了札を拒否する");
                break;
            }
            if let Some(status) = child.0.try_wait().expect("子process状態") {
                panic!("子Brokerはscratchを有効化する前に終了した: {status}");
            }
            assert!(
                Instant::now() < deadline,
                "子Brokerのscratch準備が期限内に完了しない"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        child.0.kill().expect("Broker子processを強制終了");
        let status = child.0.wait().expect("強制終了したBroker子processを回収");
        assert!(!status.success(), "子Brokerが通常終了しておりcrash試験になっていない");

        let mut restarted = Broker::new_persistent("scratch-session-restarted", &store_root)
            .expect("同じ永続storeからBrokerを再起動");
        restarted
            .実行系登録(
                "fixture-runtime",
                Arc::new(
                    crate::adapters::minidora::MinidoraAdapter::new("127.0.0.1:9").unwrap(),
                ),
            )
            .unwrap();
        restarted
            .作業領域起動登録(&workspace_config(), &protected)
            .expect("再起動時Workspace登録とscratch回復");

        assert!(!workspace_root.join(&scratch_name).exists());
        assert!(!restarted
            .agent_task_scratch
            .as_ref()
            .expect("再読込した回復journal")
            .has_pending_workspace("fixture-workspace"));
        assert!(restarted.audit_events().iter().any(|event| {
            event.request_id == "agent-task-scratch-recovery:0"
                && event.operation == "Agent Task scratch回復"
                && event.decision == "recorded"
                && event.evidence_source == EVIDENCE_SOURCE_LIVE_RUNTIME
        }));
        drop(restarted);
        fs::remove_dir_all(root).expect("試験用Broker storeとWorkspaceを除去");
    }

    #[test]
    fn first_run_configuration_is_create_only_broker_audited_and_used_by_setup_doctor() {
        let root = temp_store_dir("first-run-config");
        let mut broker = Broker::new_persistent("first-run-session", &root).unwrap();
        assert!(broker.initialize_desktop_first_run_configuration().is_err());
        assert!(!root.join("first_run_configuration.json").exists());

        broker.set_desktop_setup_doctor_runtime_evidence(true, true);
        broker.initialize_desktop_first_run_configuration().unwrap();
        let bytes = fs::read(root.join("first_run_configuration.json")).unwrap();
        let created_hash = crate::audit_hash::sha256_tagged(&bytes);
        let created_event = broker
            .audit_events()
            .iter()
            .find(|event| {
                event.operation == "D4 Pocket初回設定生成"
                    && event.decision == "accepted"
                    && event.reason.starts_with("first_run_configuration_created")
            })
            .expect("初回設定生成をhash結合したAuditEvent");
        assert_eq!(created_event.payload_hash, created_hash);

        let response = broker.handle(first_run_configuration_request(
            "first-config-1",
            json!({"version": 1}),
        ));
        assert_eq!(response.status, BrokerStatus::Accepted);
        let configuration = response.body.expect("UI設定projection");
        assert_eq!(configuration["ui_preferences"]["theme"], "system");
        let read_hash = crate::audit_hash::sha256_tagged(&bytes);
        let read_event = broker
            .audit_events()
            .iter()
            .find(|event| event.event_id == response.audit_event_id)
            .expect("UI設定取得のaccepted AuditEvent");
        assert_eq!(read_event.operation, "初回設定取得");
        assert_eq!(read_event.payload_hash, read_hash);

        let mut report_request = setup_doctor_request(
            "setup-report-config",
            json!({"version": 1}),
        );
        report_request.session_id = Some("first-run-session".to_string());
        let report = broker.handle(report_request);
        assert_eq!(report.status, BrokerStatus::Accepted, "{:?}", report.error);
        let report = report.body.expect("Setup Doctor報告");
        assert_eq!(report["checks"][6]["status"], "pass");
        assert_eq!(report["checks"][6]["evidence_class"], "LIVE_RUNTIME");
        assert_eq!(report["status"], "pass");

        let invalid = broker.handle(first_run_configuration_request(
            "first-config-2",
            json!({"version": 1, "output_path": "C:/outside.json"}),
        ));
        assert_eq!(invalid.status, BrokerStatus::Rejected);
        assert_eq!(
            invalid.error.unwrap().code,
            "first_run_configuration_request_invalid"
        );
        assert_eq!(
            fs::read(root.join("first_run_configuration.json")).unwrap(),
            bytes
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_existing_first_run_configuration_is_not_replaced() {
        let root = temp_store_dir("first-run-config-preserve");
        let target = root.join("first_run_configuration.json");
        let original = br#"{"version":1,"product":"D4 Pocket","ui_preferences":{"theme":"dark","density":"compact","locale":"ja-JP"}}"#;
        fs::write(&target, original).unwrap();
        let mut broker = Broker::new_persistent("first-run-session", &root).unwrap();
        broker.set_desktop_setup_doctor_runtime_evidence(true, false);
        assert!(broker.initialize_desktop_first_run_configuration().is_err());
        assert_eq!(fs::read(&target).unwrap(), original);
        assert!(broker.audit_events().iter().any(|event| {
            event.operation == "D4 Pocket初回設定生成" && event.decision == "rejected"
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn setup_doctor_report_is_broker_generated_audited_and_keeps_unknown_unknown() {
        let root = temp_store_dir("setup-doctor-report");
        let mut broker = Broker::new_persistent("setup-doctor-session", &root).unwrap();
        broker.set_desktop_setup_doctor_runtime_evidence(true, true);

        let response = broker.handle(setup_doctor_request(
            "setup-report-1",
            json!({"version": 1}),
        ));
        assert_eq!(response.status, BrokerStatus::Accepted);
        let report = response.body.expect("Brokerが生成した報告");
        assert_eq!(report["evidence_source"], EVIDENCE_SOURCE_LIVE_RUNTIME);
        assert_eq!(report["status"], "warning");
        assert_eq!(report["checks"].as_array().unwrap().len(), 7);
        assert_eq!(report["checks"][0]["status"], "pass");
        assert_eq!(report["checks"][3]["status"], "pass");
        assert_eq!(report["checks"][6]["status"], "unknown");
        assert_eq!(report["checks"][6]["evidence_class"], "CONFIG");
        assert!(report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|check| check["grants_authority"] == false));

        let bytes = serde_json::to_vec(&report).unwrap();
        let report_hash = crate::audit_hash::sha256_tagged(&bytes);
        let exported = broker
            .audit_events()
            .iter()
            .find(|event| {
                event.operation == "Setup Doctor報告取得"
                    && event.decision == "accepted"
                    && event.reason == "setup_doctor_report_exported"
            })
            .expect("製品報告を受理した監査記録");
        assert_eq!(exported.payload_hash, report_hash);
        assert_eq!(response.audit_event_id, exported.event_id);
        assert_eq!(fs::read(root.join("setup_doctor_report.json")).unwrap(), bytes);

        let invalid = broker.handle(setup_doctor_request(
            "setup-report-2",
            json!({"version": 1, "output_path": "C:/secret"}),
        ));
        assert_eq!(invalid.status, BrokerStatus::Rejected);
        assert_eq!(
            invalid.error.as_ref().map(|error| error.code.as_str()),
            Some("setup_doctor_request_invalid")
        );
        assert_eq!(fs::read(root.join("setup_doctor_report.json")).unwrap(), bytes);
        fs::remove_dir_all(root).unwrap();
    }

    fn command_request_with_operation(
        request_id: &str,
        nonce: &str,
        operation: &str,
    ) -> BrokerRequestEnvelope {
        let mut envelope = BrokerRequestEnvelope::command_envelope(request_id, "session-1", nonce);
        envelope.payload = Some(serde_json::json!({
            "action": {
                "operation": operation,
                "capability_id": operation,
                "permission_id": format!("permission.{operation}"),
                "payload": {
                    "target": operation
                }
            }
        }));
        envelope.refresh_payload_hash();
        envelope
    }

    fn production_authority_request(
        request_id: &str,
        nonce: &str,
        action: serde_json::Value,
    ) -> BrokerRequestEnvelope {
        let mut envelope = BrokerRequestEnvelope::command_envelope(request_id, "session-1", nonce);
        envelope.operation = Some(BrokerOperation::AuthorityEvaluate);
        envelope.payload = Some(serde_json::json!({"action": action}));
        envelope.refresh_payload_hash();
        envelope
    }

    fn broker_command_action() -> serde_json::Value {
        serde_json::json!({
            "operation": "command_envelope.dispatch",
            "runtime_id": "gui_shell_rust_broker",
            "capability_id": "command_envelope.dispatch",
            "permission_id": "permission.broker.command_envelope",
            "approval_id": "broker-projected-approval",
            "target_scope": "broker_command",
            "recovery_action": {"recovery_id": "recover-command-dispatch"},
            "adapter_metadata": {"client": "test"}
        })
    }

    #[test]
    fn tray_stop_request_is_broker_receipt_and_rejects_unknown_fields() {
        let mut broker = test_broker();
        let mut request = BrokerRequestEnvelope::command_envelope(
            "tray-stop-request",
            "session-1",
            "tray-stop-request-nonce",
        );
        request.operation = Some(BrokerOperation::全Runtime停止要求);
        request.payload = Some(json!({"版": 1}));
        request.refresh_payload_hash();
        let response = broker.handle(request);
        assert_eq!(response.status, BrokerStatus::Accepted);
        assert_eq!(response.operation, "全Runtime停止要求");
        let body = response.body.unwrap();
        assert_eq!(body["停止実行済み"], false);
        assert_eq!(body["承認状態"], "owner_reapproval_required");
        assert_eq!(body["権限生成"], "なし");

        let mut malformed = BrokerRequestEnvelope::command_envelope(
            "tray-stop-request-invalid",
            "session-1",
            "tray-stop-request-invalid-nonce",
        );
        malformed.operation = Some(BrokerOperation::全Runtime停止要求);
        malformed.payload = Some(json!({"版": 1, "authority": "owner"}));
        malformed.refresh_payload_hash();
        let rejected = broker.handle(malformed);
        assert_eq!(rejected.status, BrokerStatus::Rejected);
        assert_eq!(rejected.error.unwrap().code, "tray_stop_request_invalid");
    }

    fn authority_error_codes(body: &serde_json::Value) -> Vec<&str> {
        body.get("errors")
            .and_then(serde_json::Value::as_array)
            .unwrap()
            .iter()
            .filter_map(|error| error.get("code").and_then(serde_json::Value::as_str))
            .collect()
    }

    #[test]
    fn resource_observation_rejects_runtime_id_outside_contract_before_registry_lookup() {
        let mut broker = Broker::new("resource-id-session");
        let cases = vec![
            ("empty", String::new()),
            ("too-long", "a".repeat(129)),
            ("leading-dot", ".local".to_string()),
            ("leading-underscore", "_local".to_string()),
            ("space", "local runtime".to_string()),
            ("newline", "local\nnext".to_string()),
            ("non-ascii", "実行系".to_string()),
            ("pid-like", "pid:16496".to_string()),
            ("path", "local/..".to_string()),
        ];
        for (index, (name, runtime_id)) in cases.into_iter().enumerate() {
            let request_id = format!("resource-invalid-id-{index}");
            let nonce = format!("resource-invalid-id-nonce-{index}");
            let response = broker.handle(resource_request(
                &request_id,
                "resource-id-session",
                &nonce,
                json!({"版": 1, "実行系ID": runtime_id}),
            ));
            assert_eq!(response.status, BrokerStatus::Rejected, "case: {name}");
            assert_eq!(response.error.unwrap().code, "要求不正", "case: {name}");
            assert!(
                !broker
                    .audit_events()
                    .iter()
                    .any(|event| event.request_id == request_id && event.decision == "received"),
                "契約外の実行系IDを通常観測として受信記録しない: {name}"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_resource_observation_binds_only_registered_loopback_owner_and_never_substitutes_unknown_metrics() {
        use std::thread;
        use std::time::Duration;

        let (mut broker, _listener) = resource_broker_with_live_loopback_listener();
        let first = broker.handle(resource_request(
            "resource-first",
            "resource-session",
            "resource-nonce-first",
            json!({"版": 1, "実行系ID": "local"}),
        ));
        assert_eq!(first.status, BrokerStatus::Accepted);
        assert_eq!(first.operation, "実行系資源観測");
        assert_eq!(first.evidence_source, EVIDENCE_SOURCE_LIVE_RUNTIME);
        let first_body = first.body.unwrap();
        assert_eq!(first_body["観測監査ID"].as_str(), Some(first.audit_event_id.as_str()));
        let first_audit = broker.audit_events().last().unwrap();
        assert_eq!(first_audit.event_id, first.audit_event_id);
        assert_eq!(
            first_audit.payload_hash,
            canonical_payload_hash(Some(&first_body)),
            "accepted監査eventは観測body全体を結合する"
        );
        assert_eq!(first_body["結合"]["状態"], "bound");
        assert_eq!(first_body["結合"]["PID"].as_u64(), Some(u64::from(std::process::id())));
        assert_eq!(first_body["統治"]["能力ID"], "runtime.resource.observe");
        assert_eq!(first_body["計測"]["稼働時間Millis"]["状態"], "measured");
        assert_eq!(first_body["計測"]["RAMWorkingSetBytes"]["状態"], "measured");
        assert_eq!(first_body["計測"]["CPU利用率Percent"]["状態"], "unknown");
        for field in [
            "DiskIOBytes",
            "NetworkIOBytes",
            "GPU利用率Percent",
            "VRAMBytes",
            "平均応答Millis",
        ] {
            assert_eq!(first_body["計測"][field]["状態"], "unknown");
            assert!(first_body["計測"][field]["値"].is_null());
            assert_eq!(first_body["計測"][field]["証拠種別"], "INTERNAL_STATE");
        }

        thread::sleep(Duration::from_millis(40));
        let second = broker.handle(resource_request(
            "resource-second",
            "resource-session",
            "resource-nonce-second",
            json!({"版": 1, "実行系ID": "local"}),
        ));
        assert_eq!(second.status, BrokerStatus::Accepted);
        let second_body = second.body.unwrap();
        assert_eq!(
            second_body["計測"]["CPU利用率Percent"]["状態"],
            "measured",
            "second CPU observation: {:?}",
            second_body["計測"]["CPU利用率Percent"],
        );
        let cpu_percent = second_body["計測"]["CPU利用率Percent"]["値"]
            .as_f64()
            .expect("CPU利用率Percentは有限numberで返す");
        assert!((0.0..=100.0).contains(&cpu_percent));
        assert_eq!(second_body["短期履歴"].as_array().unwrap().len(), 2);

        let caller_pid = broker.handle(resource_request(
            "resource-caller-pid",
            "resource-session",
            "resource-nonce-caller-pid",
            json!({"版": 1, "実行系ID": "local", "PID": std::process::id()}),
        ));
        assert_eq!(caller_pid.status, BrokerStatus::Rejected);
        assert_eq!(caller_pid.error.unwrap().code, "要求不正");

        let unknown_runtime = broker.handle(resource_request(
            "resource-unknown-runtime",
            "resource-session",
            "resource-nonce-unknown-runtime",
            json!({"版": 1, "実行系ID": "other"}),
        ));
        assert_eq!(unknown_runtime.status, BrokerStatus::Rejected);
        assert_eq!(unknown_runtime.error.unwrap().code, "実行系不在");
    }

    #[cfg(windows)]
    #[test]
    fn windows_resource_observation_invalidates_when_registered_listener_disappears() {
        use std::thread;
        use std::time::Duration;

        let (mut broker, listener) = resource_broker_with_live_loopback_listener();
        let initial = broker.handle(resource_request(
            "resource-listener-present",
            "resource-session",
            "resource-listener-present-nonce",
            json!({"版": 1, "実行系ID": "local"}),
        ));
        assert_eq!(initial.status, BrokerStatus::Accepted);
        assert_eq!(initial.evidence_source, EVIDENCE_SOURCE_LIVE_RUNTIME);
        drop(listener);

        let mut invalidated = None;
        for attempt in 0..50 {
            let request_id = format!("resource-listener-gone-{attempt}");
            let nonce = format!("resource-listener-gone-nonce-{attempt}");
            let response = broker.handle(resource_request(
                &request_id,
                "resource-session",
                &nonce,
                json!({"版": 1, "実行系ID": "local"}),
            ));
            if response
                .body
                .as_ref()
                .is_some_and(|body| body["結合"]["状態"] == "binding_mismatch")
            {
                invalidated = Some(response);
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        let invalidated = invalidated.expect("listener消失後に登録済みPIDの値を返し続けた");
        assert_eq!(invalidated.status, BrokerStatus::Accepted);
        assert_eq!(invalidated.evidence_source, EVIDENCE_SOURCE_INTERNAL_STATE);
        let body = invalidated.body.unwrap();
        assert_eq!(body["結合"]["状態"], "binding_mismatch");
        assert!(body["結合"]["PID"].is_null());
        assert!(body["結合"]["PID作成時刻UnixMillis"].is_null());
        assert_eq!(body["短期履歴"], json!([]));
        assert!(body["計測"].as_object().unwrap().values().all(|metric| {
            metric["状態"] == "unknown" && metric["値"].is_null()
        }));
    }

    #[cfg(windows)]
    #[test]
    fn resource_observation_audit_failure_suspends_and_clears_registered_binding() {
        use std::net::{Ipv4Addr, TcpListener};

        let store = temp_store_dir("resource-audit-failure");
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let mut broker = Broker::new_persistent("resource-audit-session", &store).unwrap();
        let adapter = crate::adapters::minidora::MinidoraAdapter::new(&address.to_string()).unwrap();
        broker.実行系登録("local", Arc::new(adapter)).unwrap();

        let audit_path = store.join("audit.jsonl");
        let saved_audit_path = store.join("audit.saved");
        fs::rename(&audit_path, &saved_audit_path).unwrap();
        fs::create_dir(&audit_path).unwrap();
        let failed = broker.handle(resource_request(
            "resource-audit-failure",
            "resource-audit-session",
            "resource-audit-failure-nonce",
            json!({"版": 1, "実行系ID": "local"}),
        ));
        assert_eq!(failed.status, BrokerStatus::Suspended);
        assert!(failed.body.is_none());
        fs::remove_dir(&audit_path).unwrap();
        fs::rename(&saved_audit_path, &audit_path).unwrap();

        let after_repair = broker.handle(resource_request(
            "resource-audit-after-repair",
            "resource-audit-session",
            "resource-audit-after-repair-nonce",
            json!({"版": 1, "実行系ID": "local"}),
        ));
        assert_eq!(after_repair.status, BrokerStatus::Rejected);
        assert_eq!(after_repair.error.unwrap().code, "実行系不在");
        drop(broker);
        drop(listener);
        fs::remove_dir_all(store).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn runtime_registration_audit_failure_rolls_back_the_dialogue_adapter() {
        use std::net::{Ipv4Addr, TcpListener};

        let store = temp_store_dir("runtime-registration-audit-failure");
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let mut broker = Broker::new_persistent("registration-audit-session", &store).unwrap();

        let audit_path = store.join("audit.jsonl");
        let saved_audit_path = store.join("audit.saved");
        fs::rename(&audit_path, &saved_audit_path).unwrap();
        fs::create_dir(&audit_path).unwrap();

        let adapter = crate::adapters::minidora::MinidoraAdapter::new(&address.to_string()).unwrap();
        assert_eq!(
            broker.実行系登録("local", Arc::new(adapter)),
            Err(対話失敗::監査失敗)
        );
        assert!(
            !broker.対話.登録済み("local"),
            "監査へ記録できない実行系adapterを対話registryへ残さない"
        );

        fs::remove_dir(&audit_path).unwrap();
        fs::rename(&saved_audit_path, &audit_path).unwrap();
        drop(broker);
        drop(listener);
        fs::remove_dir_all(store).unwrap();
    }

    use std::sync::atomic::{AtomicUsize, Ordering};

    struct LifecycleTestAdapter {
        calls: Arc<AtomicUsize>,
        fail_closed_calls: Arc<AtomicUsize>,
        sabotage_audit_path: Option<PathBuf>,
        fail_action: Option<LifecycleAction>,
        actions: &'static [LifecycleAction],
    }

    impl LifecycleAdapter for LifecycleTestAdapter {
        fn supported_actions(&self) -> &'static [LifecycleAction] {
            self.actions
        }

        fn transition(
            &self,
            action: LifecycleAction,
        ) -> Result<crate::broker::runtime_lifecycle::LifecycleAdapterResult, crate::broker::runtime_lifecycle::LifecycleAdapterFailure> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_action == Some(action) {
                return Err(crate::broker::runtime_lifecycle::LifecycleAdapterFailure::new(
                    "ライフサイクル試験Adapterの失敗",
                ));
            }
            if let Some(audit_path) = &self.sabotage_audit_path {
                let saved = audit_path.with_extension("saved");
                fs::rename(audit_path, &saved).expect("final監査前にaudit logを退避できる");
                fs::create_dir(audit_path).expect("final監査経路をdirectoryへ置換できる");
            }
            let next_state = match action {
                LifecycleAction::Start | LifecycleAction::Restart | LifecycleAction::Resume => crate::broker::runtime_lifecycle::LifecycleState::Ready,
                LifecycleAction::Stop => crate::broker::runtime_lifecycle::LifecycleState::Stopped,
                LifecycleAction::Pause => crate::broker::runtime_lifecycle::LifecycleState::Paused,
                LifecycleAction::Quarantine => crate::broker::runtime_lifecycle::LifecycleState::Quarantined,
            };
            Ok(crate::broker::runtime_lifecycle::LifecycleAdapterResult {
                next_state,
                acknowledgement: "test-lifecycle-ack",
            })
        }

        fn fail_closed(&self) {
            self.fail_closed_calls.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn lifecycle_request(
        operation: BrokerOperation,
        request_id: &str,
        nonce: &str,
        issued_at: &str,
        payload: Value,
    ) -> BrokerRequestEnvelope {
        let mut request = BrokerRequestEnvelope::command_envelope_at(
            request_id,
            "session-1",
            nonce,
            issued_at,
        );
        request.operation = Some(operation);
        request.payload = Some(payload);
        request.refresh_payload_hash();
        request
    }

    fn lifecycle_test_broker(
        sabotage_audit_path: Option<PathBuf>,
    ) -> (Broker, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let mut broker = test_broker();
        let calls = Arc::new(AtomicUsize::new(0));
        let fail_closed_calls = Arc::new(AtomicUsize::new(0));
        broker
            .ライフサイクル試験登録(
                "fixture",
                Arc::new(LifecycleTestAdapter {
                    calls: Arc::clone(&calls),
                    fail_closed_calls: Arc::clone(&fail_closed_calls),
                    sabotage_audit_path,
                    fail_action: None,
                    actions: &LifecycleAction::ALL,
                }),
            )
            .unwrap();
        (broker, calls, fail_closed_calls)
    }

    fn lifecycle_pending(broker: &mut Broker, action: &str, nonce: &str) -> Value {
        let response = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル承認要求,
            &format!("pending-{nonce}"),
            nonce,
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "fixture", "操作": action}),
        ));
        assert_eq!(response.status, BrokerStatus::Accepted, "{response:?}");
        let body = response.body.unwrap();
        assert_eq!(body["状態"], "pending");
        body
    }

    fn lifecycle_owner_approve(broker: &mut Broker, approval: &Value, nonce: &str) -> Value {
        let response = broker.処理(
            lifecycle_request(
                BrokerOperation::実行系ライフサイクル承認,
                &format!("owner-{nonce}"),
                &format!("owner-{nonce}"),
                "2026-06-01T00:00:00Z",
                json!({
                    "版": 1,
                    "承認ID": approval["承認ID"],
                    "承認hash": approval["承認hash"],
                }),
            ),
            true,
        );
        assert_eq!(response.status, BrokerStatus::Accepted, "{response:?}");
        let body = response.body.unwrap();
        assert_eq!(body["状態"], "approved");
        body
    }

    fn lifecycle_execute(broker: &mut Broker, action: &str, approval: &Value, nonce: &str) -> BrokerResponse {
        broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル操作,
            &format!("execute-{nonce}"),
            nonce,
            "2026-06-01T00:00:00Z",
            json!({
                "版": 1,
                "実行系ID": "fixture",
                "操作": action,
                "承認ID": approval["承認ID"],
            }),
        ))
    }

    #[test]
    fn lifecycle_requires_owner_approval_revalidates_replay_and_quarantines() {
        let (mut broker, calls, _) = lifecycle_test_broker(None);
        let unknown = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル承認要求,
            "unknown-runtime",
            "unknown-runtime-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "unknown", "操作": "start"}),
        ));
        assert_eq!(unknown.status, BrokerStatus::Rejected);
        assert_eq!(unknown.error.unwrap().code, "lifecycle_runtime_unknown");

        let owner_status = broker.処理(
            lifecycle_request(
                BrokerOperation::実行系ライフサイクル状態,
                "owner-status",
                "owner-status-nonce",
                "2026-06-01T00:00:00Z",
                json!({"版": 1, "実行系ID": "fixture"}),
            ),
            true,
        );
        assert_eq!(owner_status.status, BrokerStatus::Rejected);
        assert_eq!(
            owner_status.error.unwrap().code,
            "lifecycle_normal_channel_required"
        );

        let missing = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル操作,
            "missing-approval",
            "missing-approval-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "fixture", "操作": "start", "承認ID": "missing"}),
        ));
        assert_eq!(missing.status, BrokerStatus::Rejected);
        assert_eq!(missing.error.unwrap().code, "lifecycle_approval_missing");

        let pending = lifecycle_pending(&mut broker, "start", "start-pending");
        let normal_approval = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル承認,
            "normal-owner-forgery",
            "normal-owner-forgery-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "承認ID": pending["承認ID"], "承認hash": pending["承認hash"]}),
        ));
        assert_eq!(normal_approval.status, BrokerStatus::Rejected);
        assert_eq!(normal_approval.error.unwrap().code, "lifecycle_owner_approval_required");

        let stale = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル操作,
            "stale-execute",
            "stale-execute-nonce",
            "2000-01-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "fixture", "操作": "start", "承認ID": pending["承認ID"]}),
        ));
        assert_eq!(stale.status, BrokerStatus::Rejected);
        assert_eq!(stale.error.unwrap().code, "broker_issued_at_invalid");
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        let approved = lifecycle_owner_approve(&mut broker, &pending, "start-approval");
        let executed = lifecycle_execute(&mut broker, "start", &approved, "start-execute");
        assert_eq!(executed.status, BrokerStatus::Accepted, "{executed:?}");
        let transition = executed.body.unwrap();
        assert_eq!(transition["遷移前状態"], "stopped");
        assert_eq!(transition["遷移後状態"], "ready");
        assert_eq!(transition["統治"]["承認状態"], "consumed");
        assert_eq!(transition["ライフサイクル監査ID"], executed.audit_event_id);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let replay = lifecycle_execute(&mut broker, "start", &approved, "start-execute");
        assert_eq!(replay.status, BrokerStatus::Rejected);
        assert_eq!(replay.error.unwrap().code, "broker_replay_detected");
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let quarantine_pending = lifecycle_pending(&mut broker, "quarantine", "quarantine-pending");
        let quarantine_approved = lifecycle_owner_approve(&mut broker, &quarantine_pending, "quarantine-approval");
        let quarantined = lifecycle_execute(&mut broker, "quarantine", &quarantine_approved, "quarantine-execute");
        assert_eq!(quarantined.status, BrokerStatus::Accepted, "{quarantined:?}");
        let status = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル状態,
            "quarantine-status",
            "quarantine-status-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "fixture"}),
        ));
        assert_eq!(status.status, BrokerStatus::Accepted);
        let status = status.body.unwrap();
        assert_eq!(status["状態"], "quarantined");
        assert_eq!(status["操作一覧"], json!([]));
        assert_eq!(status["承認一覧"], json!([]));
        let blocked = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル承認要求,
            "quarantine-blocked",
            "quarantine-blocked-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "fixture", "操作": "start"}),
        ));
        assert_eq!(blocked.status, BrokerStatus::Rejected);
        assert_eq!(blocked.error.unwrap().code, "lifecycle_invalid_transition");
    }

    #[test]
    fn lifecycle_protocol_rejects_caller_control_fields_and_current_governance_failures() {
        for (index, (field, value)) in [
            ("PID", json!(1234)),
            ("endpoint", json!("127.0.0.1:1")),
            ("command", json!("cmd.exe")),
            ("argv", json!(["/c", "whoami"])),
            ("env", json!({"X":"Y"})),
            ("state", json!("ready")),
            ("authority", json!("owner")),
            ("metadata", json!({"authority":"owner"})),
            ("統治ID", json!("caller-controlled")),
        ]
        .into_iter()
        .enumerate()
        {
            let (mut broker, calls, _) = lifecycle_test_broker(None);
            let mut payload = json!({"版": 1, "実行系ID": "fixture", "操作": "start"});
            payload
                .as_object_mut()
                .unwrap()
                .insert(field.to_string(), value);
            let response = broker.handle(lifecycle_request(
                BrokerOperation::実行系ライフサイクル承認要求,
                &format!("caller-field-{index}"),
                &format!("caller-field-nonce-{index}"),
                "2026-06-01T00:00:00Z",
                payload,
            ));
            assert_eq!(response.status, BrokerStatus::Rejected);
            assert_eq!(response.error.unwrap().code, "lifecycle_request_invalid");
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        }

        let (mut broker, calls, _) = lifecycle_test_broker(None);
        let invalid_transition = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル承認要求,
            "invalid-transition",
            "invalid-transition-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "fixture", "操作": "pause"}),
        ));
        assert_eq!(invalid_transition.status, BrokerStatus::Rejected);
        assert_eq!(invalid_transition.error.unwrap().code, "lifecycle_invalid_transition");
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        let (mut broker, calls, _) = lifecycle_test_broker(None);
        broker.ライフサイクル試験能力削除("fixture", LifecycleAction::Start);
        let response = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル承認要求,
            "capability-missing",
            "capability-missing-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "fixture", "操作": "start"}),
        ));
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "lifecycle_capability_missing");
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        let (mut broker, calls, _) = lifecycle_test_broker(None);
        broker.ライフサイクル試験permission設定("fixture", LifecycleAction::Start, false);
        let response = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル承認要求,
            "permission-denied",
            "permission-denied-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "fixture", "操作": "start"}),
        ));
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "lifecycle_permission_denied");
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        let (mut broker, calls, _) = lifecycle_test_broker(None);
        broker.ライフサイクル試験復旧削除("fixture", LifecycleAction::Start);
        let response = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル承認要求,
            "recovery-missing",
            "recovery-missing-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "fixture", "操作": "start"}),
        ));
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "lifecycle_recovery_missing");
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        let (mut broker, calls, _) = lifecycle_test_broker(None);
        let pending = lifecycle_pending(&mut broker, "start", "stale-session-pending");
        let approved = lifecycle_owner_approve(&mut broker, &pending, "stale-session-approval");
        let mut stale_session = lifecycle_request(
            BrokerOperation::実行系ライフサイクル操作,
            "stale-session-execute",
            "stale-session-execute-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "fixture", "操作": "start", "承認ID": approved["承認ID"]}),
        );
        stale_session.session_id = Some("stale-session".to_string());
        let response = broker.handle(stale_session);
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "broker_stale_session");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn lifecycle_quarantine_reservation_closes_dialogue_resource_and_workspace_before_adapter_result() {
        let store = temp_store_dir("lifecycle-terminal-runtime-closure");
        let workspace_root = store.join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        let mut broker = persistent_test_broker(&store);
        let dialogue_adapter = crate::adapters::minidora::MinidoraAdapter::new("127.0.0.1:9").unwrap();
        broker
            .実行系登録("fixture", Arc::new(dialogue_adapter))
            .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let fail_closed_calls = Arc::new(AtomicUsize::new(0));
        broker
            .ライフサイクル試験登録(
                "fixture",
                Arc::new(LifecycleTestAdapter {
                    calls: Arc::clone(&calls),
                    fail_closed_calls: Arc::clone(&fail_closed_calls),
                    sabotage_audit_path: None,
                    fail_action: Some(LifecycleAction::Quarantine),
                    actions: &LifecycleAction::ALL,
                }),
            )
            .unwrap();
        broker
            .作業領域登録(
                "fixture",
                "fixture-workspace",
                cap_std::fs::Dir::open_ambient_dir(
                    &workspace_root,
                    cap_std::ambient_authority(),
                )
                .unwrap(),
                &[],
            )
            .unwrap();

        let dialogue_start = broker.handle(lifecycle_request(
            BrokerOperation::対話開始,
            "terminal-dialogue-start",
            "terminal-dialogue-start-nonce",
            "2026-06-01T00:00:00Z",
            json!({"実行系ID": "fixture"}),
        ));
        assert_eq!(dialogue_start.status, BrokerStatus::Accepted);
        let session_id = dialogue_start.body.unwrap()["対話セッションID"]
            .as_str()
            .unwrap()
            .to_string();

        let started_pending = lifecycle_pending(&mut broker, "start", "terminal-start-pending");
        let started_approval = lifecycle_owner_approve(&mut broker, &started_pending, "terminal-start-approval");
        let started = lifecycle_execute(&mut broker, "start", &started_approval, "terminal-start-execute");
        assert_eq!(started.status, BrokerStatus::Accepted);

        let quarantine_pending = lifecycle_pending(&mut broker, "quarantine", "terminal-failing-pending");
        let quarantine_approval = lifecycle_owner_approve(
            &mut broker,
            &quarantine_pending,
            "terminal-failing-approval",
        );
        let quarantine = lifecycle_execute(
            &mut broker,
            "quarantine",
            &quarantine_approval,
            "terminal-failing-execute",
        );
        assert_eq!(quarantine.status, BrokerStatus::Rejected);
        assert_eq!(quarantine.error.unwrap().code, "lifecycle_adapter_failed");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(fail_closed_calls.load(Ordering::SeqCst) >= 1);

        let terminal_status = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル状態,
            "terminal-status-after-adapter-failure",
            "terminal-status-after-adapter-failure-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "fixture"}),
        ));
        assert_eq!(terminal_status.status, BrokerStatus::Accepted);
        let terminal_status = terminal_status.body.unwrap();
        assert_eq!(terminal_status["状態"], "quarantined");
        assert_eq!(terminal_status["操作一覧"], json!([]));
        assert_eq!(terminal_status["承認一覧"], json!([]));

        let listed = broker.handle(lifecycle_request(
            BrokerOperation::実行系列挙,
            "terminal-runtime-list",
            "terminal-runtime-list-nonce",
            "2026-06-01T00:00:00Z",
            json!({}),
        ));
        assert_eq!(listed.status, BrokerStatus::Accepted);
        assert_eq!(listed.body.unwrap()["実行系"], json!([]));

        let old_session_send = broker.handle(lifecycle_request(
            BrokerOperation::対話送信,
            "terminal-old-session-send",
            "terminal-old-session-send-nonce",
            "2026-06-01T00:00:00Z",
            json!({"対話セッションID": session_id, "入力": "隔離後送信"}),
        ));
        assert_eq!(old_session_send.status, BrokerStatus::Rejected);
        assert_eq!(old_session_send.error.unwrap().code, "セッション不一致");

        let mut resource_request = resource_request(
            "terminal-resource-observation",
            "session-1",
            "terminal-resource-observation-nonce",
            json!({"版": 1, "実行系ID": "fixture"}),
        );
        resource_request.issued_at = Some("2026-06-01T00:00:00Z".to_string());
        let resource = broker.handle(resource_request);
        assert_eq!(resource.status, BrokerStatus::Rejected);
        assert_eq!(resource.error.unwrap().code, "実行系不在");

        let workspaces = broker.handle(lifecycle_request(
            BrokerOperation::作業領域一覧,
            "terminal-workspace-list",
            "terminal-workspace-list-nonce",
            "2026-06-01T00:00:00Z",
            json!({}),
        ));
        assert_eq!(workspaces.status, BrokerStatus::Accepted);
        assert_eq!(workspaces.body.unwrap()["作業領域"], json!([]));
        assert_eq!(
            broker.実行系登録(
                "fixture",
                Arc::new(crate::adapters::minidora::MinidoraAdapter::new("127.0.0.1:9").unwrap()),
            ),
            Err(対話失敗::隔離済み)
        );
        assert_eq!(
            broker.作業領域登録(
                "fixture",
                "fixture-workspace-replacement",
                cap_std::fs::Dir::open_ambient_dir(
                    &workspace_root,
                    cap_std::ambient_authority(),
                )
                .unwrap(),
                &[],
            ),
            Err("実行系はterminal隔離中")
        );
        assert_eq!(
            broker.作業領域起動登録(
                &super::super::workspace_root::WorkspaceStartup {
                    runtime_id: "fixture".to_string(),
                    workspace_id: "fixture-workspace-startup".to_string(),
                    root_path: workspace_root.to_string_lossy().to_string(),
                    secret_paths: vec![],
                },
                &[],
            ),
            Err("実行系はterminal隔離中")
        );

        drop(broker);
        let mut restarted = persistent_test_broker(&store);
        let restarted_status = restarted.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル状態,
            "terminal-status-after-restart",
            "terminal-status-after-restart-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "fixture"}),
        ));
        assert_eq!(restarted_status.status, BrokerStatus::Accepted);
        assert_eq!(restarted_status.body.unwrap()["状態"], "quarantined");
        assert_eq!(
            restarted.実行系登録(
                "fixture",
                Arc::new(crate::adapters::minidora::MinidoraAdapter::new("127.0.0.1:9").unwrap()),
            ),
            Err(対話失敗::隔離済み)
        );
        drop(restarted);
        fs::remove_dir_all(store).unwrap();
    }

    #[test]
    fn minidora_dialogue_adapter_is_reported_without_lifecycle_capability() {
        let mut broker = test_broker();
        let minidora = crate::adapters::minidora::MinidoraAdapter::new("127.0.0.1:9").unwrap();
        broker.実行系登録("minidora", Arc::new(minidora)).unwrap();
        let status = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル状態,
            "minidora-lifecycle-status",
            "minidora-lifecycle-status-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "minidora"}),
        ));
        assert_eq!(status.status, BrokerStatus::Accepted);
        let body = status.body.unwrap();
        assert_eq!(body["対応"], false);
        assert_eq!(body["状態"], "not_supported");
        assert_eq!(body["操作一覧"], json!([]));
        assert_eq!(body["承認一覧"], json!([]));
        let request = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル承認要求,
            "minidora-lifecycle-request",
            "minidora-lifecycle-request-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "minidora", "操作": "start"}),
        ));
        assert_eq!(request.status, BrokerStatus::Rejected);
        assert_eq!(request.error.unwrap().code, "lifecycle_runtime_unknown");
    }

    #[test]
    fn lifecycle_audit_failure_never_leaves_approval_or_successful_transition_usable() {
        let fixed_now = parse_issued_at_epoch_seconds("2026-06-01T00:00:30Z").unwrap();

        let store = temp_store_dir("lifecycle-request-audit-failure");
        let (mut broker, calls, _) = lifecycle_test_broker(None);
        broker.state_store = BrokerStateStore::durable_file_store(
            BrokerPersistentStore::open_or_create(&store, "session-1").unwrap().0,
        );
        broker.current_epoch_seconds_override = Some(fixed_now);
        let audit_path = store.join("audit.jsonl");
        let saved = audit_path.with_extension("saved");
        fs::rename(&audit_path, &saved).unwrap();
        fs::create_dir(&audit_path).unwrap();
        let request_failure = broker.handle(lifecycle_request(
            BrokerOperation::実行系ライフサイクル承認要求,
            "request-audit-failure",
            "request-audit-failure-nonce",
            "2026-06-01T00:00:00Z",
            json!({"版": 1, "実行系ID": "fixture", "操作": "start"}),
        ));
        assert_eq!(request_failure.status, BrokerStatus::Suspended);
        let after_request_failure = broker.ライフサイクル.status_body("fixture", fixed_now).unwrap();
        assert_eq!(after_request_failure["状態"], "unknown");
        assert_eq!(after_request_failure["承認一覧"], json!([]));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        fs::remove_dir(&audit_path).unwrap();
        fs::rename(&saved, &audit_path).unwrap();
        drop(broker);
        fs::remove_dir_all(&store).unwrap();

        let store = temp_store_dir("lifecycle-owner-approval-audit-failure");
        let (mut broker, calls, _) = lifecycle_test_broker(None);
        broker.state_store = BrokerStateStore::durable_file_store(
            BrokerPersistentStore::open_or_create(&store, "session-1").unwrap().0,
        );
        broker.current_epoch_seconds_override = Some(fixed_now);
        let pending = lifecycle_pending(&mut broker, "start", "owner-audit-pending");
        let audit_path = store.join("audit.jsonl");
        let saved = audit_path.with_extension("saved");
        fs::rename(&audit_path, &saved).unwrap();
        fs::create_dir(&audit_path).unwrap();
        let owner_failure = broker.処理(
            lifecycle_request(
                BrokerOperation::実行系ライフサイクル承認,
                "owner-audit-failure",
                "owner-audit-failure-nonce",
                "2026-06-01T00:00:00Z",
                json!({"版": 1, "承認ID": pending["承認ID"], "承認hash": pending["承認hash"]}),
            ),
            true,
        );
        assert_eq!(owner_failure.status, BrokerStatus::Suspended);
        let after_owner_failure = broker.ライフサイクル.status_body("fixture", fixed_now).unwrap();
        assert_eq!(after_owner_failure["状態"], "unknown");
        assert_eq!(after_owner_failure["承認一覧"], json!([]));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        fs::remove_dir(&audit_path).unwrap();
        fs::rename(&saved, &audit_path).unwrap();
        drop(broker);
        fs::remove_dir_all(&store).unwrap();

        let store = temp_store_dir("lifecycle-pre-action-audit-failure");
        let (mut broker, calls, _) = lifecycle_test_broker(None);
        broker.state_store = BrokerStateStore::durable_file_store(
            BrokerPersistentStore::open_or_create(&store, "session-1").unwrap().0,
        );
        broker.current_epoch_seconds_override = Some(fixed_now);
        let pending = lifecycle_pending(&mut broker, "start", "pre-action-pending");
        let approved = lifecycle_owner_approve(&mut broker, &pending, "pre-action-approval");
        let audit_path = store.join("audit.jsonl");
        let saved = audit_path.with_extension("saved");
        fs::rename(&audit_path, &saved).unwrap();
        fs::create_dir(&audit_path).unwrap();
        let pre_action_failure = lifecycle_execute(&mut broker, "start", &approved, "pre-action-execute");
        assert_eq!(pre_action_failure.status, BrokerStatus::Suspended);
        let after_pre_action_failure = broker.ライフサイクル.status_body("fixture", fixed_now).unwrap();
        assert_eq!(after_pre_action_failure["状態"], "unknown");
        assert_eq!(after_pre_action_failure["承認一覧"], json!([]));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        fs::remove_dir(&audit_path).unwrap();
        fs::rename(&saved, &audit_path).unwrap();
        drop(broker);
        fs::remove_dir_all(&store).unwrap();

        let store = temp_store_dir("lifecycle-final-audit-failure");
        let audit_path = store.join("audit.jsonl");
        let (mut broker, calls, fail_closed) = lifecycle_test_broker(Some(audit_path.clone()));
        broker.state_store = BrokerStateStore::durable_file_store(
            BrokerPersistentStore::open_or_create(&store, "session-1").unwrap().0,
        );
        broker.current_epoch_seconds_override = Some(fixed_now);
        let pending = lifecycle_pending(&mut broker, "start", "final-pending");
        let approved = lifecycle_owner_approve(&mut broker, &pending, "final-approval");
        let final_failure = lifecycle_execute(&mut broker, "start", &approved, "final-execute");
        assert_eq!(final_failure.status, BrokerStatus::Suspended);
        assert_eq!(calls.load(Ordering::SeqCst), 1, "最終監査前に一回だけadapterが実行される");
        assert!(fail_closed.load(Ordering::SeqCst) >= 1, "最終監査障害でadapterをfail-closedする");
        let after_final_failure = broker.ライフサイクル.status_body("fixture", fixed_now).unwrap();
        assert_eq!(after_final_failure["状態"], "unknown");
        assert_eq!(after_final_failure["承認一覧"], json!([]));
        let saved = audit_path.with_extension("saved");
        fs::remove_dir(&audit_path).unwrap();
        fs::rename(&saved, &audit_path).unwrap();
        drop(broker);
        fs::remove_dir_all(&store).unwrap();
    }

    #[test]
    fn broker_state_store_declares_all_in_memory_scopes() {
        let store = BrokerStateStore::in_memory_skeleton();
        assert_eq!(store.health_status(), "ready");
        assert_eq!(store.audit_persistence(), "in_memory_skeleton");
        assert_eq!(store.replay_persistence(), "in_memory_session_only");
        assert_eq!(store.session_persistence(), "in_memory_session_only");
        assert!(!store.persistence_required());
        assert!(!store.persistence_ready());
    }

    #[test]
    fn persistent_store_health_reports_durable_ready() {
        let store_dir = temp_store_dir("persistent-health");
        let mut broker = persistent_test_broker(&store_dir);
        let response = broker.handle(BrokerRequestEnvelope::health("request-1", "nonce-1"));
        assert_eq!(response.status, BrokerStatus::Accepted);
        let health = response.health.unwrap();
        assert_eq!(health.status, "ready");
        assert_eq!(health.audit_persistence, "durable_file_store");
        assert_eq!(health.replay_persistence, "durable_file_store");
        assert_eq!(health.session_persistence, "durable_file_store");
        assert!(health.persistence_required);
        assert!(health.persistence_ready);
        assert_eq!(health.authority_cutover_status, "not_active");
        assert!(store_dir.join("audit.jsonl").exists());
        assert!(store_dir.join("replay_nonces.jsonl").exists());
        assert!(store_dir.join("session.json").exists());
    }

    #[test]
    fn persistent_store_rejects_replayed_nonce_after_restart() {
        let store_dir = temp_store_dir("persistent-replay");
        {
            let mut broker = persistent_test_broker(&store_dir);
            let response = broker.handle(BrokerRequestEnvelope::health("request-1", "nonce-1"));
            assert_eq!(response.status, BrokerStatus::Accepted);
        }

        let mut restarted = persistent_test_broker(&store_dir);
        let response = restarted.handle(BrokerRequestEnvelope::health("request-2", "nonce-1"));
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "broker_replay_detected");
    }

    #[test]
    fn persistent_store_verifies_audit_chain_after_restart() {
        let store_dir = temp_store_dir("persistent-audit-chain");
        {
            let mut broker = persistent_test_broker(&store_dir);
            let first = broker.handle(BrokerRequestEnvelope::health("request-1", "nonce-1"));
            let second = broker.handle(BrokerRequestEnvelope::command_envelope(
                "request-2",
                "session-1",
                "nonce-2",
            ));
            assert_eq!(first.status, BrokerStatus::Accepted);
            assert_eq!(second.status, BrokerStatus::Suspended);
            assert_eq!(broker.audit_events().len(), 2);
        }

        let restarted = persistent_test_broker(&store_dir);
        assert_eq!(restarted.audit_events().len(), 2);
        assert_eq!(
            restarted.audit_events()[1].previous_event_hash,
            Some(restarted.audit_events()[0].event_hash.clone())
        );
    }

    #[test]
    fn persistent_store_rejects_tampered_audit_chain() {
        let store_dir = temp_store_dir("persistent-audit-tamper");
        {
            let mut broker = persistent_test_broker(&store_dir);
            let response = broker.handle(BrokerRequestEnvelope::health("request-1", "nonce-1"));
            assert_eq!(response.status, BrokerStatus::Accepted);
        }
        let audit_path = store_dir.join("audit.jsonl");
        let tampered = fs::read_to_string(&audit_path)
            .unwrap()
            .replace("\"decision\":\"accepted\"", "\"decision\":\"rejected\"");
        fs::write(&audit_path, tampered).unwrap();

        let result = Broker::new_persistent("session-1", &store_dir);
        assert!(result.is_err());
    }

    #[test]
    fn persistent_store_rejects_truncated_audit_anchor() {
        let store_dir = temp_store_dir("persistent-audit-anchor-truncate");
        {
            let mut broker = persistent_test_broker(&store_dir);
            let first = broker.handle(BrokerRequestEnvelope::health("request-1", "nonce-1"));
            let second = broker.handle(BrokerRequestEnvelope::health("request-2", "nonce-2"));
            assert_eq!(first.status, BrokerStatus::Accepted);
            assert_eq!(second.status, BrokerStatus::Accepted);
        }
        let audit_path = store_dir.join("audit.jsonl");
        let first_line = fs::read_to_string(&audit_path)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .to_string();
        fs::write(&audit_path, format!("{first_line}\n")).unwrap();

        let result = Broker::new_persistent("session-1", &store_dir);
        assert!(result.is_err());
    }

    #[test]
    fn persistent_store_rejects_malformed_replay_state() {
        let store_dir = temp_store_dir("persistent-replay-malformed");
        {
            let mut broker = persistent_test_broker(&store_dir);
            let response = broker.handle(BrokerRequestEnvelope::health("request-1", "nonce-1"));
            assert_eq!(response.status, BrokerStatus::Accepted);
        }
        fs::write(store_dir.join("replay_nonces.jsonl"), "{not-json}\n").unwrap();

        let result = Broker::new_persistent("session-1", &store_dir);
        assert!(result.is_err());
    }

    #[test]
    fn broker_health_is_accepted_and_audited() {
        let mut broker = test_broker();
        let response = broker.handle(BrokerRequestEnvelope::health("request-1", "nonce-1"));
        assert_eq!(response.status, BrokerStatus::Accepted);
        let health = response.health.unwrap();
        assert_eq!(health.boundary_role, "rust_security_broker_candidate");
        assert_eq!(health.authority_cutover_status, "not_active");
        assert_eq!(health.audit_persistence, "in_memory_skeleton");
        assert_eq!(health.replay_persistence, "in_memory_session_only");
        assert_eq!(health.session_persistence, "in_memory_session_only");
        assert!(!health.persistence_required);
        assert!(!health.persistence_ready);
        assert_eq!(broker.audit_events().len(), 1);
        assert_eq!(broker.audit_events()[0].decision, "accepted");
        assert_eq!(
            broker.audit_events()[0].payload_hash,
            canonical_payload_hash(None)
        );
    }

    #[test]
    fn persistence_required_health_suspends_without_store() {
        let mut broker = persistence_required_broker();
        let response = broker.handle(BrokerRequestEnvelope::health("request-1", "nonce-1"));
        assert_eq!(response.status, BrokerStatus::Suspended);
        assert_eq!(
            response.error.unwrap().code,
            "broker_persistence_unavailable"
        );
        let health = response.health.unwrap();
        assert_eq!(health.status, "suspend");
        assert_eq!(health.audit_persistence, "in_memory_skeleton");
        assert_eq!(health.replay_persistence, "in_memory_session_only");
        assert_eq!(health.session_persistence, "in_memory_session_only");
        assert!(health.persistence_required);
        assert!(!health.persistence_ready);
        assert_eq!(broker.audit_events()[0].decision, "suspended");
    }

    #[test]
    fn persistence_required_command_rejects_without_store() {
        let mut broker = persistence_required_broker();
        let response = broker.handle(BrokerRequestEnvelope::command_envelope(
            "request-1",
            "session-1",
            "nonce-1",
        ));
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "broker_persistence_unavailable"
        );
        assert_eq!(broker.audit_events()[0].decision, "rejected");
        assert!(broker.audit_events()[0]
            .reason
            .contains("broker_persistence_unavailable"));
    }

    #[test]
    fn json_health_request_is_accepted_and_serialized() {
        let mut broker = test_broker();
        let response = broker.handle_json(
            r#"{
                "request_id": "json-request-1",
                "operation": "health",
                "payload_hash": "sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b",
                "nonce": "json-nonce-1",
                "issued_at": "2026-06-01T00:00:00Z",
                "metadata": {"client": "desktop_flutter"}
            }"#,
        );
        assert_eq!(response.status, BrokerStatus::Accepted);
        let encoded = response.to_json_string().unwrap();
        assert!(encoded.contains(r#""status":"accepted""#));
        assert!(encoded.contains(r#""boundary_role":"rust_security_broker_candidate""#));
        assert!(encoded.contains(r#""authority_cutover_status":"not_active""#));
        assert!(encoded.contains(r#""session_persistence":"in_memory_session_only""#));
        assert!(encoded.contains(r#""shutdown_requested":false"#));
    }

    #[test]
    fn payload_hash_mismatch_is_rejected_and_audited() {
        let mut broker = test_broker();
        let response = broker.handle_json(
            r#"{
                "request_id": "json-request-1",
                "operation": "normalize_payload",
                "payload_hash": "sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b",
                "nonce": "json-nonce-1",
                "issued_at": "2026-06-01T00:00:00Z",
                "metadata": {"client": "desktop_flutter"},
                "payload": {"client_payload": "desktop_flutter_authority_probe"}
            }"#,
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "broker_payload_hash_mismatch");
        assert_eq!(
            broker.audit_events()[0].payload_hash,
            "sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b"
        );
    }

    #[test]
    fn payload_hash_matches_dart_known_vectors() {
        assert_eq!(
            canonical_payload_hash(None),
            "sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b"
        );
        assert_eq!(
            canonical_payload_hash(Some(&json!({"b": 1, "a": 2}))),
            "sha256:d3626ac30a87e6f7a6428233b3c68299976865fa5508e4267c5415c76af7a772"
        );
        assert_eq!(
            canonical_payload_hash(Some(&json!({
                "z": [{"b": 1, "a": 2}, null, true],
                "a": {"d": "text", "c": [3, 2, 1]}
            }))),
            "sha256:8895d6e5b558a29b870d1156bfb1e95fcbab9933f2360c35edaa78d734c8c87a"
        );
        assert_eq!(
            canonical_payload_hash(Some(&json!({
                "client_payload": "desktop_flutter_authority_probe"
            }))),
            "sha256:787a213a62a6dd88756a81d1b68234f88759d36308adc933625aa48a4507a93b"
        );
    }

    #[test]
    fn invalid_json_is_rejected_and_audited() {
        let mut broker = test_broker();
        let response = broker.handle_json("{not-json");
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "broker_request_malformed".to_string()
        );
        assert_eq!(broker.audit_events().len(), 1);
    }

    #[test]
    fn json_missing_metadata_is_rejected_and_audited() {
        let mut broker = test_broker();
        let response = broker.handle_json(
            r#"{
                "request_id": "json-request-1",
                "operation": "health",
                "payload_hash": "sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b",
                "nonce": "json-nonce-1",
                "issued_at": "2026-06-01T00:00:00Z"
            }"#,
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "broker_request_malformed".to_string()
        );
    }

    #[test]
    fn json_authority_metadata_is_rejected_and_audited() {
        let mut broker = test_broker();
        let response = broker.handle_json(
            r#"{
                "request_id": "json-request-1",
                "operation": "health",
                "payload_hash": "sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b",
                "nonce": "json-nonce-1",
                "issued_at": "2026-06-01T00:00:00Z",
                "metadata": {"trustLevel": "root"}
            }"#,
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "broker_authority_metadata_rejected".to_string()
        );
    }

    #[test]
    fn malformed_request_is_rejected_and_audited() {
        let mut broker = test_broker();
        let response = broker.handle(BrokerRequestEnvelope {
            request_id: None,
            session_id: None,
            operation: Some(BrokerOperation::Health),
            payload_hash: Some(
                "sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b"
                    .to_string(),
            ),
            nonce: Some("nonce-1".to_string()),
            issued_at: Some("2026-06-01T00:00:00Z".to_string()),
            metadata: vec![],
            metadata_present: true,
            payload: None,
        });
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "broker_request_malformed".to_string()
        );
        assert_eq!(broker.audit_events().len(), 1);
    }

    #[test]
    fn stale_issued_at_is_rejected_and_audited() {
        let mut broker = test_broker();
        let mut request = BrokerRequestEnvelope::health("request-1", "nonce-1");
        request.issued_at = Some("2026-06-01T00:10:00Z".to_string());
        let response = broker.handle(request);
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "broker_issued_at_invalid");
        assert_eq!(broker.audit_events().len(), 1);
        assert_eq!(broker.audit_events()[0].reason, "broker_issued_at_invalid");
    }

    #[test]
    fn malformed_issued_at_is_rejected_and_audited() {
        let mut broker = test_broker();
        let mut request = BrokerRequestEnvelope::health("request-1", "nonce-1");
        request.issued_at = Some("not-a-timestamp".to_string());
        let response = broker.handle(request);
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "broker_issued_at_invalid");
        assert_eq!(broker.audit_events().len(), 1);
    }

    #[test]
    fn issued_at_parser_handles_utc_offsets() {
        assert_eq!(
            parse_issued_at_epoch_seconds("2026-06-01T09:00:30+09:00"),
            parse_issued_at_epoch_seconds("2026-06-01T00:00:30Z")
        );
        assert_eq!(
            parse_issued_at_epoch_seconds("2026-05-31T19:00:30-05:00"),
            parse_issued_at_epoch_seconds("2026-06-01T00:00:30Z")
        );
        assert!(parse_issued_at_epoch_seconds("2026-02-29T00:00:00Z").is_none());
        assert!(parse_issued_at_epoch_seconds("2024-02-29T00:00:00Z").is_some());
    }

    #[test]
    fn current_issued_at_formatter_uses_utc_rfc3339_seconds() {
        assert_eq!(
            epoch_seconds_to_rfc3339(
                parse_issued_at_epoch_seconds("2026-06-01T00:00:30Z").unwrap()
            ),
            "2026-06-01T00:00:30Z"
        );
    }

    #[test]
    fn replayed_nonce_is_rejected_and_audited() {
        let mut broker = test_broker();
        let first = broker.handle(BrokerRequestEnvelope::health("request-1", "nonce-1"));
        let second = broker.handle(BrokerRequestEnvelope::health("request-2", "nonce-1"));
        assert_eq!(first.status, BrokerStatus::Accepted);
        assert_eq!(second.status, BrokerStatus::Rejected);
        assert_eq!(second.error.unwrap().code, "broker_replay_detected");
        assert_eq!(broker.audit_events().len(), 2);
    }

    #[test]
    fn stale_session_is_rejected_and_audited() {
        let mut broker = test_broker();
        let response = broker.handle(BrokerRequestEnvelope::shutdown(
            "request-1",
            "stale-session",
            "nonce-1",
        ));
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "broker_stale_session");
        assert!(!broker.shutdown_requested());
    }

    #[test]
    fn authority_metadata_is_rejected_and_audited() {
        let mut broker = test_broker();
        let mut request = BrokerRequestEnvelope::health("request-1", "nonce-1");
        request.metadata.push(BrokerMetadata {
            key: "trust\u{200b}Level".to_string(),
            value: "root".to_string(),
        });
        let response = broker.handle(request);
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "broker_authority_metadata_rejected"
        );
    }

    #[test]
    fn unicode_nfkc_authority_metadata_is_rejected_and_audited() {
        let mut broker = test_broker();
        let response = broker.handle_json(
            r#"{
                "request_id": "json-request-1",
                "operation": "health",
                "payload_hash": "sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b",
                "nonce": "json-nonce-1",
                "issued_at": "2026-06-01T00:00:00Z",
                "metadata": {"ｔｒｕｓｔ＿ｌｅｖｅｌ": "ｒｏｏｔ"}
            }"#,
        );
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "broker_authority_metadata_rejected"
        );
    }

    #[test]
    fn authority_alias_and_separator_variants_are_rejected() {
        let mut broker = test_broker();
        for (index, key) in [
            "Trust-Level",
            "TRUST LEVEL",
            "permissionGrant",
            "permissiongrant",
            "permissions_granted",
            "privilege",
        ]
        .iter()
        .enumerate()
        {
            let mut request = BrokerRequestEnvelope::health(&format!("request-{}", index + 1), key);
            request.metadata.push(BrokerMetadata {
                key: (*key).to_string(),
                value: "operator".to_string(),
            });
            let response = broker.handle(request);
            assert_eq!(response.status, BrokerStatus::Rejected);
            assert_eq!(
                response.error.unwrap().code,
                "broker_authority_metadata_rejected"
            );
        }
    }

    #[test]
    fn value_only_and_nested_authority_metadata_are_rejected() {
        let mut broker = test_broker();
        let value_only = broker.handle_json(
            r#"{
                "request_id": "json-request-1",
                "operation": "health",
                "payload_hash": "sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b",
                "nonce": "json-nonce-1",
                "issued_at": "2026-06-01T00:00:00Z",
                "metadata": {"safe_label": "ｒｏｏｔ"}
            }"#,
        );
        assert_eq!(value_only.status, BrokerStatus::Rejected);
        assert_eq!(
            value_only.error.unwrap().code,
            "broker_authority_metadata_rejected"
        );

        let nested = broker.handle_json(
            r#"{
                "request_id": "json-request-2",
                "operation": "health",
                "payload_hash": "sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b",
                "nonce": "json-nonce-2",
                "issued_at": "2026-06-01T00:00:00Z",
                "metadata": {"safe_label": {"authority": "admin"}}
            }"#,
        );
        assert_eq!(nested.status, BrokerStatus::Rejected);
        assert_eq!(
            nested.error.unwrap().code,
            "broker_authority_metadata_rejected"
        );
    }

    #[test]
    fn production_authority_rejects_caller_supplied_fixture_state() {
        let mut broker = test_broker();
        let mut request =
            production_authority_request("request-1", "nonce-1", broker_command_action());
        request.payload = Some(serde_json::json!({
            "state": {
                "runtimes": [{"runtime_id": "gui_shell_rust_broker"}],
                "capabilities": [{
                    "capability_id": "command_envelope.dispatch",
                    "runtime_id": "gui_shell_rust_broker",
                    "operations": ["command_envelope.dispatch"]
                }],
                "permissions": [{
                    "permission_id": "permission.broker.command_envelope",
                    "runtime_id": "gui_shell_rust_broker",
                    "capability_id": "command_envelope.dispatch",
                    "operation": "command_envelope.dispatch",
                    "target_scope": "broker_command",
                    "decision": "allow"
                }],
                "approvals": [{
                    "approval_id": "broker-projected-approval",
                    "runtime_id": "gui_shell_rust_broker",
                    "operation": "command_envelope.dispatch",
                    "target_scope": "broker_command",
                    "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "status": "approved"
                }],
                "audit_events": [{
                    "event_id": "fixture-audit",
                    "action": "command_envelope.dispatch",
                    "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                }],
                "recovery_actions": [{
                    "recovery_id": "recover-command-dispatch",
                    "runtime_id": "gui_shell_rust_broker",
                    "operation": "command_envelope.dispatch"
                }]
            },
            "action": broker_command_action()
        }));
        request.refresh_payload_hash();
        let response = broker.handle(request);
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "broker_authority_state_rejected"
        );
    }

    #[test]
    fn production_authority_uses_broker_owned_registry_and_denies_missing_records() {
        let mut broker = test_broker();
        let mut action = broker_command_action();
        action["runtime_id"] = serde_json::Value::String("caller-runtime".to_string());
        let response = broker.handle(production_authority_request("request-1", "nonce-1", action));
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.unwrap();
        assert_eq!(body["allowed"], false);
        assert_eq!(body["decision"], "denied");
        assert!(authority_error_codes(&body).contains(&"unknown_runtime"));
        assert_eq!(broker.audit_events()[0].decision, "denied");
    }

    #[test]
    fn production_authority_rejects_caller_forged_authority_source() {
        let mut broker = test_broker();
        let mut action = broker_command_action();
        action["authority_source"] = serde_json::Value::String("rust_security_broker".to_string());
        let response = broker.handle(production_authority_request("request-1", "nonce-1", action));
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.unwrap();
        assert_eq!(body["allowed"], false);
        assert!(authority_error_codes(&body).contains(&"caller_authority_source_rejected"));
        assert_eq!(broker.audit_events()[0].decision, "denied");
    }

    #[test]
    fn production_authority_rejects_caller_audit_mapping() {
        let mut broker = test_broker();
        let mut action = broker_command_action();
        action["audit_event"] = serde_json::json!({
            "event_id": "caller-audit",
            "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        });
        let response = broker.handle(production_authority_request("request-1", "nonce-1", action));
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.unwrap();
        assert_eq!(body["allowed"], false);
        assert!(authority_error_codes(&body).contains(&"caller_audit_mapping_rejected"));
        assert_eq!(broker.audit_events()[0].decision, "denied");
    }

    #[test]
    fn fixture_authority_operation_is_isolated_from_production_operation() {
        let mut broker = test_broker();
        let mut request =
            production_authority_request("request-1", "nonce-1", broker_command_action());
        request.operation = Some(BrokerOperation::AuthorityFixtureEvaluate);
        request.payload = Some(serde_json::json!({
            "state": {
                "runtimes": [{"runtime_id": "gui_shell_rust_broker"}],
                "capabilities": [{
                    "capability_id": "command_envelope.dispatch",
                    "runtime_id": "gui_shell_rust_broker",
                    "operations": ["command_envelope.dispatch"]
                }],
                "permissions": [{
                    "permission_id": "permission.broker.command_envelope",
                    "runtime_id": "gui_shell_rust_broker",
                    "capability_id": "command_envelope.dispatch",
                    "operation": "command_envelope.dispatch",
                    "target_scope": "broker_command",
                    "decision": "allow"
                }],
                "approvals": [{
                    "approval_id": "broker-projected-approval",
                    "runtime_id": "gui_shell_rust_broker",
                    "operation": "command_envelope.dispatch",
                    "target_scope": "broker_command",
                    "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "status": "approved"
                }],
                "audit_events": [{
                    "event_id": "fixture-audit",
                    "action": "command_envelope.dispatch",
                    "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                }],
                "recovery_actions": [{
                    "recovery_id": "recover-command-dispatch",
                    "runtime_id": "gui_shell_rust_broker",
                    "operation": "command_envelope.dispatch"
                }]
            },
            "action": {
                "operation": "command_envelope.dispatch",
                "runtime_id": "gui_shell_rust_broker",
                "capability_id": "command_envelope.dispatch",
                "permission_id": "permission.broker.command_envelope",
                "approval_id": "broker-projected-approval",
                "target_scope": "broker_command",
                "audit_event": {
                    "event_id": "fixture-audit",
                    "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                },
                "recovery_action": {"recovery_id": "recover-command-dispatch"}
            }
        }));
        request.refresh_payload_hash();
        let response = broker.handle(request);
        assert_eq!(response.status, BrokerStatus::Accepted);
        assert_eq!(response.operation, "authority_fixture_evaluate");
        assert_eq!(response.evidence_source, EVIDENCE_SOURCE_INTERNAL_STATE);
        assert_eq!(response.body.unwrap()["allowed"], true);
    }

    #[test]
    fn command_envelope_is_suspended_without_dispatch() {
        let mut broker = test_broker();
        let response = broker.handle(BrokerRequestEnvelope::command_envelope(
            "request-1",
            "session-1",
            "nonce-1",
        ));
        assert_eq!(response.status, BrokerStatus::Suspended);
        assert_eq!(
            response.error.unwrap().code,
            "broker_command_dispatch_disabled"
        );
        let body = response.body.unwrap();
        assert_eq!(body["dispatch_enabled"], false);
        assert_eq!(body["dispatch_decision"], "suspended");
        assert_eq!(body["execution_gate"]["dispatch"], "suspended");
        assert_eq!(broker.audit_events()[0].decision, "suspended");
    }

    #[test]
    fn command_envelope_reports_process_credential_and_update_gates() {
        for (index, (operation, target_kind)) in [
            ("process.spawn", "process"),
            ("credential.read", "credential"),
            ("update.apply", "update"),
        ]
        .iter()
        .enumerate()
        {
            let mut broker = test_broker();
            let response = broker.handle(command_request_with_operation(
                &format!("request-{index}"),
                &format!("nonce-{index}"),
                operation,
            ));
            assert_eq!(response.status, BrokerStatus::Suspended);
            let body = response.body.unwrap();
            assert_eq!(body["dispatch_enabled"], false);
            assert_eq!(body["execution_gate"]["status"], "suspended");
            assert_eq!(body["execution_gate"]["target_kind"], *target_kind);
            assert_eq!(body["execution_gate"]["dispatch"], "suspended");
            assert_eq!(body["execution_gate"][*target_kind], "suspended");
        }
    }

    #[test]
    fn shutdown_sets_lifecycle_flag() {
        let mut broker = test_broker();
        let response = broker.handle(BrokerRequestEnvelope::shutdown(
            "request-1",
            "session-1",
            "nonce-1",
        ));
        assert_eq!(response.status, BrokerStatus::Accepted);
        assert!(response.shutdown_requested);
        assert!(broker.shutdown_requested());
    }
}

#[cfg(test)]
mod 端末統治試験 {
    use super::*;
    use serde_json::json;
    use crate::broker::dialogue::識別子生成;
    struct 環境 {broker: Broker, path: std::path::PathBuf}
    impl Drop for 環境 {fn drop(&mut self){assert_eq!(self.path.parent(),Some(std::env::temp_dir().as_path()));let _=std::fs::remove_dir_all(&self.path);}}
    fn 環境生成()->環境 {
        let path=std::env::temp_dir().join(format!("gui-shell-link-test-{}",識別子生成().unwrap()));
        let mut broker=Broker::new_persistent("device-test",&path).unwrap();
        broker.端末経路設定("b".repeat(64),7443).unwrap();
        broker.実行系登録("local",Arc::new(crate::adapters::minidora::MinidoraAdapter::new("127.0.0.1:9").unwrap())).unwrap();
        環境 {broker,path}
    }
    fn 制御(b:&mut Broker,op:&str,p:Value)->BrokerResponse{
        let id=識別子生成().unwrap();
        b.owner要求処理(&json!({"request_id":id,"nonce":id,"session_id":"device-test","operation":op,"payload":p,
            "payload_hash":sha256_tagged(p.to_string().as_bytes()),"issued_at":BrokerRequestEnvelope::current_issued_at(),"metadata":{}}).to_string())
    }
    fn 通常要求(b: &mut Broker, operation: BrokerOperation, payload: Value) -> BrokerResponse {
        let id=識別子生成().unwrap();
        let nonce=識別子生成().unwrap();
        let mut request=BrokerRequestEnvelope::health(&id,&nonce);
        request.session_id=Some("device-test".into());
        request.operation=Some(operation);
        request.payload=Some(payload);
        request.issued_at=Some(BrokerRequestEnvelope::current_issued_at());
        request.refresh_payload_hash();
        b.handle(request)
    }
    fn 通信(b:&mut Broker,c:&Value,op:&str,p:Value)->BrokerResponse{
        b.端末要求処理(&json!({"版":1,"HostID":c["HostID"],"端末ID":c["端末ID"],"資格ID":c.get("結合ID").unwrap_or(&c["招待ID"]),
            "資格秘密":c.get("端末秘密").unwrap_or(&c["招待秘密"]),"nonce":識別子生成().unwrap(),"発行時刻":b.current_epoch_seconds(),"操作":op,"内容":p}).to_string())
    }
    struct AgentMetadataAdapter {
        metadata: Value,
        workspace_identity: Option<crate::broker::dialogue::AgentTaskWorkspaceIdentity>,
    }
    impl AgentMetadataAdapter {
        fn new(metadata: Value) -> Self {
            Self {
                metadata,
                workspace_identity: None,
            }
        }
        fn bound_to_workspace(
            metadata: Value,
            workspace_identity: crate::broker::dialogue::AgentTaskWorkspaceIdentity,
        ) -> Self {
            Self {
                metadata,
                workspace_identity: Some(workspace_identity),
            }
        }
    }
    impl 実行系Adapter for AgentMetadataAdapter {
        fn 接続対象(&self) -> String {
            "agent-metadata-test://read-only".into()
        }
        fn agent_metadata(&self) -> Option<Value> {
            Some(self.metadata.clone())
        }
        fn 作業領域実体識別子(
            &self,
        ) -> Option<crate::broker::dialogue::AgentTaskWorkspaceIdentity> {
            self.workspace_identity
        }
        fn 応答(
            &self,
            _: &crate::broker::dialogue::対話要求,
            _: &std::sync::atomic::AtomicBool,
            _: Instant,
            _: &mut Vec<Vec<u8>>,
        ) -> Result<crate::broker::dialogue::実行結果, 対話失敗> {
            Err(対話失敗::応答不正)
        }
    }
    fn 準備(b:&mut Broker)->(Value,Value){
        let i=制御(b,"端末招待",json!({"端末ID":"c".repeat(32),"接続先Host":"127.0.0.1"})).body.unwrap();
        let c=通信(b,&i,"端末結合",json!({})).body.unwrap();
        let session=通信(b,&c,"対話開始",json!({"実行系ID":"local"})).body.unwrap();
        let pending=通信(b,&c,"対話送信",json!({"対話セッションID":session["対話セッションID"],"入力":"こんにちは"})).body.unwrap();
        (c,pending)
    }
    #[test]
    fn agent_list_rejects_untrusted_adapter_metadata_with_audit_and_no_leak() {
        let secret_marker = "TEST_ONLY_SECRET_SENTINEL";
        for (index, metadata) in [
            json!({"permission": "all"}),
            json!({"api_key": secret_marker}),
            {
                let mut metadata: Value = serde_json::from_str(include_str!(
                    "../../../../examples/contracts/agent_adapter.valid.json"
                ))
                .unwrap();
                metadata["tool_support"]["reason"] =
                    json!(format!("sk-{secret_marker}"));
                metadata
            },
        ]
        .into_iter()
        .enumerate()
        {
            let mut environment = 環境生成();
            environment
                .broker
                .実行系登録(
                    "hostile-agent",
                    Arc::new(AgentMetadataAdapter::new(metadata)),
                )
                .unwrap();
            let mut request = BrokerRequestEnvelope::health(
                &format!("agent-list-request-{index}"),
                &format!("agent-list-nonce-{index}"),
            );
            request.session_id = Some("device-test".into());
            request.operation = Some(BrokerOperation::Agent一覧);
            request.payload = Some(json!({}));
            request.issued_at = Some(BrokerRequestEnvelope::current_issued_at());
            request.refresh_payload_hash();

            let response = environment.broker.handle(request);
            assert_eq!(response.status, BrokerStatus::Rejected);
            assert_eq!(response.error.as_ref().map(|error| error.code.as_str()), Some("応答不正"));

            let response_json = serde_json::to_string(&response).unwrap();
            let audit_json = serde_json::to_string(environment.broker.audit_events()).unwrap();
            assert!(!response_json.contains(secret_marker));
            assert!(!audit_json.contains(secret_marker));
            assert!(audit_json.contains("Agent一覧"));
            assert!(audit_json.contains("\"decision\":\"rejected\""));
        }
    }
    #[test]
    fn 対話セッション一覧は通常要求経路で監査済み内部状態だけを返す() {
        let mut e=環境生成();
        let metadata: Value = serde_json::from_str(include_str!(
            "../../../../examples/contracts/agent_adapter.valid.json"
        ))
        .expect("Agent接続例を読み込む");
        e.broker
            .実行系登録(
                "fixture-agent",
                Arc::new(AgentMetadataAdapter::new(metadata.clone())),
            )
            .unwrap();
        let workspace_root=e.path.join("fixture-agent-workspace");
        std::fs::create_dir(&workspace_root).unwrap();
        for workspace_id in ["fixture-workspace", "second-fixture-workspace"] {
            let (root, _, ancestry) =
                super::super::workspace_root::open_isolated_root_with_ancestry(
                    &workspace_root,
                    &[],
                )
                .unwrap();
            e.broker.作業領域登録範囲付き(
                "fixture-agent",
                workspace_id,
                root,
                &[],
                Some(ancestry),
            ).unwrap();
        }
        let unbound=通常要求(&mut e.broker,BrokerOperation::対話開始,json!({"実行系ID":"fixture-agent"}));
        assert_eq!(unbound.status,BrokerStatus::Rejected);
        assert_eq!(unbound.error.as_ref().map(|error|error.code.as_str()),Some("作業領域不在"));
        let unregistered=通常要求(&mut e.broker,BrokerOperation::対話開始,json!({"実行系ID":"fixture-agent","作業領域ID":"not-registered"}));
        assert_eq!(unregistered.status,BrokerStatus::Rejected);
        assert_eq!(unregistered.error.as_ref().map(|error|error.code.as_str()),Some("作業領域不在"));
        let other_runtime_root=e.path.join("other-runtime-workspace");
        std::fs::create_dir(&other_runtime_root).unwrap();
        e.broker
            .実行系登録(
                "other-fixture-agent",
                Arc::new(AgentMetadataAdapter::new(metadata.clone())),
            )
            .unwrap();
        let (other_root, _, other_ancestry) =
            super::super::workspace_root::open_isolated_root_with_ancestry(
                &other_runtime_root,
                &[],
            )
            .unwrap();
        e.broker.作業領域登録範囲付き(
            "other-fixture-agent",
            "other-runtime-workspace",
            other_root,
            &[],
            Some(other_ancestry),
        ).unwrap();
        let cross_runtime=通常要求(&mut e.broker,BrokerOperation::対話開始,json!({"実行系ID":"fixture-agent","作業領域ID":"other-runtime-workspace"}));
        assert_eq!(cross_runtime.status,BrokerStatus::Rejected);
        assert_eq!(cross_runtime.error.as_ref().map(|error|error.code.as_str()),Some("作業領域不在"));
        let started=通常要求(&mut e.broker,BrokerOperation::対話開始,json!({"実行系ID":"fixture-agent","作業領域ID":"fixture-workspace"}));
        assert_eq!(started.status,BrokerStatus::Accepted);
        let listed=通常要求(&mut e.broker,BrokerOperation::対話セッション一覧,json!({}));
        assert_eq!(listed.status,BrokerStatus::Accepted);
        assert_eq!(listed.evidence_source,EVIDENCE_SOURCE_INTERNAL_STATE);
        let body=listed.body.as_ref().expect("対話セッション一覧");
        assert_eq!(body["版"],1);
        assert_eq!(body["対話セッション"].as_array().unwrap().len(),1);
        assert_eq!(body["対話セッション"][0]["作業領域ID"],"fixture-workspace");
        assert_eq!(body["対話セッション"][0]["作業領域結合監査ID"],started.audit_event_id);
        assert_ne!(body["対話セッション"][0]["作成監査ID"],started.audit_event_id);
        assert_eq!(body["対話セッション"][0].as_object().unwrap().len(),6);
        assert!(e.broker.audit_events().iter().any(|event|
            event.event_id==started.audit_event_id && event.reason=="対話Sessionと登録済みWorkspaceの明示結合"));
        assert_eq!(
            通常要求(&mut e.broker,BrokerOperation::対話セッション一覧,json!({"authority":"owner"})).status,
            BrokerStatus::Rejected
        );
    }
    #[test]
    #[allow(non_snake_case)]
    fn Agent作業要求検査は現行SessionとWorkspaceだけを照合し本文を露出せず未実行を明示する() {
        let mut e = 環境生成();
        let mut metadata: Value = serde_json::from_str(include_str!(
            "../../../../examples/contracts/agent_adapter.valid.json"
        ))
        .expect("Agent Adapterのfixture");
        metadata["capabilities"].as_array_mut().unwrap().push(json!({
            "capability_id":"task_execution",
            "support":{"status":"supported","reason":"Broker Task権限経路の試験fixture。実Task実行の証拠ではない"}
        }));
        let root = e.path.join("agent-task-workspace");
        std::fs::create_dir(&root).unwrap();
        let root_identity = super::super::workspace_root::pin_workspace_path(&root)
            .expect("登録対象Workspace rootを固定")
            .identity;
        e.broker
            .実行系登録(
                "fixture-agent",
                Arc::new(AgentMetadataAdapter::bound_to_workspace(
                    metadata,
                    crate::broker::dialogue::AgentTaskWorkspaceIdentity::from_directory_identity(
                        root_identity,
                    ),
                )),
            )
            .unwrap();
        let (workspace_root, _, ancestry) =
            super::super::workspace_root::open_isolated_root_with_ancestry(&root, &[]).unwrap();
        e.broker
            .作業領域登録範囲付き(
                "fixture-agent",
                "fixture-task-workspace",
                workspace_root,
                &[],
                Some(ancestry),
            )
            .unwrap();
        let started = 通常要求(
            &mut e.broker,
            BrokerOperation::対話開始,
            json!({"実行系ID":"fixture-agent","作業領域ID":"fixture-task-workspace"}),
        );
        assert_eq!(started.status, BrokerStatus::Accepted);
        let session_id = started.body.as_ref().unwrap()["対話セッションID"]
            .as_str()
            .unwrap();
        let instruction = "監査へ本文を漏らさない検査用一意文言";
        let payload = json!({
            "agent_runtime_id":"fixture-agent",
            "session_id":session_id,
            "workspace_id":"fixture-task-workspace",
            "instruction":instruction,
        });
        let accepted = 通常要求(
            &mut e.broker,
            BrokerOperation::Agent作業要求検査,
            payload.clone(),
        );
        assert_eq!(accepted.status, BrokerStatus::Accepted);
        assert_eq!(accepted.evidence_source, EVIDENCE_SOURCE_INTERNAL_STATE);
        let body = accepted.body.as_ref().unwrap();
        assert_eq!(body["状態"], "要求検査済み");
        assert_eq!(body["実行状態"], "未実行");
        assert_eq!(body["Permission状態"], "未付与");
        assert_eq!(body["Approval状態"], "未取得");
        assert_eq!(
            body["指示hash"],
            crate::audit_hash::sha256_tagged(instruction.as_bytes())
        );
        let output = serde_json::to_string(&accepted).unwrap();
        let audit = serde_json::to_string(e.broker.audit_events()).unwrap();
        assert!(!output.contains(instruction));
        assert!(!audit.contains(instruction));

        let permission_payload = json!({
            "agent_runtime_id":"fixture-agent",
            "session_id":session_id,
            "workspace_id":"fixture-task-workspace"
        });
        let ordinary_permission = 通常要求(
            &mut e.broker,
            BrokerOperation::AgentTaskWorkspacePermissionGrant,
            permission_payload.clone(),
        );
        assert_eq!(ordinary_permission.status, BrokerStatus::Rejected);
        assert_eq!(
            ordinary_permission.error.as_ref().map(|error| error.code.as_str()),
            Some("desktop_native_owner_confirmation_required")
        );
        let approval_without_permission = json!({
            "request_id": "agent-task-owner-approval-without-permission",
            "session_id": "device-test",
            "operation": "AgentTaskOwnerApprovalGrant",
            "payload": payload.clone(),
            "payload_hash": canonical_payload_hash(Some(&payload)),
            "nonce": "agent-task-owner-approval-without-permission-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        let denied_without_permission = e
            .broker
            .desktop_owner_operation_json(&approval_without_permission.to_string());
        assert_eq!(denied_without_permission.status, BrokerStatus::Rejected);
        assert_eq!(
            denied_without_permission.error.as_ref().map(|error| error.code.as_str()),
            Some("権限拒否")
        );

        let native_permission_request = |request_id: &str, nonce: &str| {
            json!({
                "request_id": request_id,
                "session_id": "device-test",
                "operation": "AgentTaskWorkspacePermissionGrant",
                "payload": permission_payload,
                "payload_hash": canonical_payload_hash(Some(&permission_payload)),
                "nonce": nonce,
                "issued_at": BrokerRequestEnvelope::current_issued_at(),
                "metadata": {"client": "desktop_flutter"}
            })
        };
        let native_request = native_permission_request(
            "agent-task-permission-native",
            "agent-task-permission-native-nonce",
        );
        let permission = e
            .broker
            .desktop_owner_operation_json(&native_request.to_string());
        assert_eq!(permission.status, BrokerStatus::Accepted);
        let receipt = permission.body.as_ref().unwrap();
        assert_eq!(receipt["operation"], "agent_task.execute");
        assert_eq!(receipt["scope"], "session_workspace_once");
        assert_eq!(receipt["source"], "owner");
        assert_eq!(receipt["use_limit"], 1);
        assert_eq!(receipt["uses_remaining"], 1);
        assert_eq!(receipt["status"], "active");
        assert_eq!(
            receipt["expires_at_epoch_seconds"].as_i64().unwrap(),
            e.broker.current_epoch_seconds() + 300
        );
        assert!(e.broker.audit_events().iter().any(|event| {
            event.operation == "AgentTaskWorkspacePermissionGrant"
                && event.reason.contains("Task未実行")
        }));

        let approval_before_grant = 通常要求(
            &mut e.broker,
            BrokerOperation::Agent作業要求検査,
            payload.clone(),
        );
        assert_eq!(
            approval_before_grant.body.as_ref().unwrap()["Approval状態"],
            "未取得"
        );
        let ordinary_approval = 通常要求(
            &mut e.broker,
            BrokerOperation::AgentTaskOwnerApprovalGrant,
            payload.clone(),
        );
        assert_eq!(ordinary_approval.status, BrokerStatus::Rejected);
        assert_eq!(
            ordinary_approval.error.as_ref().map(|error| error.code.as_str()),
            Some("desktop_native_owner_confirmation_required")
        );
        let native_approval_request = json!({
            "request_id": "agent-task-owner-approval-native",
            "session_id": "device-test",
            "operation": "AgentTaskOwnerApprovalGrant",
            "payload": payload.clone(),
            "payload_hash": canonical_payload_hash(Some(&payload)),
            "nonce": "agent-task-owner-approval-native-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "metadata": {"client": "desktop_flutter"}
        });
        let approval = e
            .broker
            .desktop_owner_operation_json(&native_approval_request.to_string());
        assert_eq!(approval.status, BrokerStatus::Accepted);
        let approval_receipt = approval.body.as_ref().unwrap();
        assert_eq!(approval_receipt["実行状態"], "未実行");
        assert_eq!(approval_receipt["指示hash"], crate::audit_hash::sha256_tagged(instruction.as_bytes()));
        assert_eq!(approval_receipt["use_limit"], 1);
        assert_eq!(approval_receipt["uses_remaining"], 1);
        assert_eq!(approval_receipt["status"], "issued_unconsumed");
        assert!(approval_receipt.get("approval_id").is_none());
        let approval_output = serde_json::to_string(&approval).unwrap();
        let approval_audit = serde_json::to_string(e.broker.audit_events()).unwrap();
        assert!(!approval_output.contains(instruction));
        assert!(!approval_audit.contains(instruction));
        assert!(e.broker.audit_events().iter().any(|event| {
            event.operation == "AgentTaskOwnerApprovalGrant"
                && event.reason.contains("Task未実行")
        }));
        let replayed_approval = e
            .broker
            .desktop_owner_operation_json(&native_approval_request.to_string());
        assert_eq!(replayed_approval.status, BrokerStatus::Rejected);
        assert_eq!(
            replayed_approval.error.as_ref().map(|error| error.code.as_str()),
            Some("broker_replay_detected")
        );

        let permission_preflight = 通常要求(
            &mut e.broker,
            BrokerOperation::Agent作業要求検査,
            payload.clone(),
        );
        assert_eq!(permission_preflight.status, BrokerStatus::Accepted);
        assert_eq!(
            permission_preflight.body.as_ref().unwrap()["Permission状態"],
            "有効"
        );
        assert_eq!(
            permission_preflight.body.as_ref().unwrap()["Approval状態"],
            "有効"
        );
        let mut changed_instruction = payload.clone();
        changed_instruction["instruction"] = json!("変更されたTask本文");
        let mismatched_approval = 通常要求(
            &mut e.broker,
            BrokerOperation::Agent作業要求検査,
            changed_instruction,
        );
        assert_eq!(mismatched_approval.status, BrokerStatus::Accepted);
        assert_eq!(
            mismatched_approval.body.as_ref().unwrap()["Approval状態"],
            "未取得"
        );
        e.broker.current_epoch_seconds_override =
            receipt["expires_at_epoch_seconds"].as_i64();
        let expired_permission_preflight = 通常要求(
            &mut e.broker,
            BrokerOperation::Agent作業要求検査,
            payload.clone(),
        );
        assert_eq!(expired_permission_preflight.status, BrokerStatus::Accepted);
        assert_eq!(
            expired_permission_preflight.body.as_ref().unwrap()["Permission状態"],
            "未付与"
        );
        assert_eq!(
            expired_permission_preflight.body.as_ref().unwrap()["Approval状態"],
            "未取得"
        );
        let replacement_permission_request = native_permission_request(
            "agent-task-permission-native-replacement",
            "agent-task-permission-native-replacement-nonce",
        );
        let replacement_permission = e
            .broker
            .desktop_owner_operation_json(&replacement_permission_request.to_string());
        assert_eq!(replacement_permission.status, BrokerStatus::Accepted);
        e.broker.current_epoch_seconds_override = None;

        let after_permission_replacement = 通常要求(
            &mut e.broker,
            BrokerOperation::Agent作業要求検査,
            payload.clone(),
        );
        assert_eq!(after_permission_replacement.status, BrokerStatus::Accepted);
        assert_eq!(
            after_permission_replacement.body.as_ref().unwrap()["Permission状態"],
            "有効"
        );
        assert_eq!(
            after_permission_replacement.body.as_ref().unwrap()["Approval状態"],
            "未取得"
        );

        let duplicate_native_request = native_permission_request(
            "agent-task-permission-native-duplicate",
            "agent-task-permission-native-duplicate-nonce",
        );
        let duplicate = e
            .broker
            .desktop_owner_operation_json(&duplicate_native_request.to_string());
        assert_eq!(duplicate.status, BrokerStatus::Rejected);
        assert_eq!(
            duplicate.error.as_ref().map(|error| error.code.as_str()),
            Some("権限拒否")
        );

        let mut stale_session = payload.clone();
        stale_session["session_id"] = json!("stale-agent-session");
        let rejected = 通常要求(
            &mut e.broker,
            BrokerOperation::Agent作業要求検査,
            stale_session,
        );
        assert_eq!(rejected.status, BrokerStatus::Rejected);
        assert_eq!(
            rejected.error.as_ref().map(|error| error.code.as_str()),
            Some("セッション不一致")
        );

        let mut authority_injection = payload.clone();
        authority_injection["permission_id"] = json!("attacker-controlled");
        let rejected = 通常要求(
            &mut e.broker,
            BrokerOperation::Agent作業要求検査,
            authority_injection,
        );
        assert_eq!(rejected.status, BrokerStatus::Rejected);
        assert_eq!(
            rejected.error.as_ref().map(|error| error.code.as_str()),
            Some("要求不正")
        );

        let ended_session = 通常要求(
            &mut e.broker,
            BrokerOperation::対話終了,
            json!({"対話セッションID": session_id}),
        );
        assert_eq!(ended_session.status, BrokerStatus::Accepted);
        assert_eq!(ended_session.body.as_ref().unwrap()["状態"], "終了");
        let after_session_end = 通常要求(
            &mut e.broker,
            BrokerOperation::Agent作業要求検査,
            payload,
        );
        assert_eq!(after_session_end.status, BrokerStatus::Rejected);
        assert_eq!(
            after_session_end.error.as_ref().map(|error| error.code.as_str()),
            Some("セッション不一致")
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn AgentTask能力が未対応なら検査とOwner権限発行を拒否する() {
        let mut e = 環境生成();
        let mut metadata: Value = serde_json::from_str(include_str!(
            "../../../../examples/contracts/agent_adapter.valid.json"
        ))
        .expect("Agent Adapter試験構造を読み込む");
        metadata["capabilities"].as_array_mut().unwrap().push(json!({
            "capability_id":"task_execution",
            "support":{"status":"unsupported","reason":"Broker統治済み書込Task経路なし"}
        }));
        e.broker
            .実行系登録(
                "unsupported-task-agent",
                Arc::new(AgentMetadataAdapter::new(metadata)),
            )
            .unwrap();
        let root = e.path.join("unsupported-task-workspace");
        std::fs::create_dir(&root).unwrap();
        let (workspace_root, _, ancestry) =
            super::super::workspace_root::open_isolated_root_with_ancestry(&root, &[]).unwrap();
        e.broker
            .作業領域登録範囲付き(
                "unsupported-task-agent",
                "unsupported-task-workspace",
                workspace_root,
                &[],
                Some(ancestry),
            )
            .unwrap();
        let started = 通常要求(
            &mut e.broker,
            BrokerOperation::対話開始,
            json!({"実行系ID":"unsupported-task-agent","作業領域ID":"unsupported-task-workspace"}),
        );
        assert_eq!(started.status, BrokerStatus::Accepted);
        let session_id = started.body.as_ref().unwrap()["対話セッションID"]
            .as_str()
            .unwrap()
            .to_owned();
        let task_payload = json!({
            "agent_runtime_id":"unsupported-task-agent",
            "session_id":session_id,
            "workspace_id":"unsupported-task-workspace",
            "instruction":"隔離Task経路が未対応なら拒否する"
        });
        let preflight = 通常要求(
            &mut e.broker,
            BrokerOperation::Agent作業要求検査,
            task_payload.clone(),
        );
        assert_eq!(preflight.status, BrokerStatus::Rejected);
        assert_eq!(
            preflight.error.as_ref().map(|error| error.code.as_str()),
            Some("AgentTask実行非対応")
        );

        let permission_payload = json!({
            "agent_runtime_id":"unsupported-task-agent",
            "session_id":session_id,
            "workspace_id":"unsupported-task-workspace"
        });
        let native_request = json!({
            "request_id":"unsupported-task-permission",
            "session_id":"device-test",
            "operation":"AgentTaskWorkspacePermissionGrant",
            "payload":permission_payload,
            "payload_hash":canonical_payload_hash(Some(&permission_payload)),
            "nonce":"unsupported-task-permission-nonce",
            "issued_at":BrokerRequestEnvelope::current_issued_at(),
            "metadata":{"client":"desktop_flutter"}
        });
        let denied = e
            .broker
            .desktop_owner_operation_json(&native_request.to_string());
        assert_eq!(denied.status, BrokerStatus::Rejected);
        assert_eq!(
            denied.error.as_ref().map(|error| error.code.as_str()),
            Some("AgentTask実行非対応")
        );
        assert!(!e.broker.audit_events().iter().any(|event| {
            event.operation == "AgentTaskWorkspacePermissionGrant"
                && event.reason.contains("発行（native Owner確認・Task未実行）")
        }));

        let approval_request = json!({
            "request_id":"unsupported-task-approval",
            "session_id":"device-test",
            "operation":"AgentTaskOwnerApprovalGrant",
            "payload":task_payload.clone(),
            "payload_hash":canonical_payload_hash(Some(&task_payload)),
            "nonce":"unsupported-task-approval-nonce",
            "issued_at":BrokerRequestEnvelope::current_issued_at(),
            "metadata":{"client":"desktop_flutter"}
        });
        let denied = e
            .broker
            .desktop_owner_operation_json(&approval_request.to_string());
        assert_eq!(denied.status, BrokerStatus::Rejected);
        assert_eq!(
            denied.error.as_ref().map(|error| error.code.as_str()),
            Some("AgentTask実行非対応")
        );
    }
    #[test]
    fn 監査障害時も失効資格の保留送信を隔離する(){
        let mut e=環境生成();let (c,p)=準備(&mut e.broker);
        let audit=e.path.join("audit.jsonl");let saved=e.path.join("audit.saved");
        std::fs::rename(&audit,&saved).unwrap();std::fs::create_dir(&audit).unwrap();
        let r=制御(&mut e.broker,"端末失効",json!({"結合ID":c["結合ID"]}));assert_ne!(r.status,BrokerStatus::Accepted);
        std::fs::remove_dir(&audit).unwrap();std::fs::rename(&saved,&audit).unwrap();
        assert_ne!(通信(&mut e.broker,&c,"端末確認",json!({})).status,BrokerStatus::Accepted);
        assert_ne!(制御(&mut e.broker,"対話承認",json!({"要求ID":p["要求ID"],"要求hash":p["要求hash"],"表示範囲":"full"})).status,BrokerStatus::Accepted);
    }
    #[test]
    fn 期限処理は実制御の保留要求を隔離し監査する(){
        let mut e=環境生成();let (c,p)=準備(&mut e.broker);
        e.broker.current_epoch_seconds_override=Some(c["有効期限"].as_i64().unwrap());
        e.broker.端末期限処理();
        assert_ne!(通信(&mut e.broker,&c,"端末確認",json!({})).status,BrokerStatus::Accepted);
        // 時刻を戻しても、失効した要求のowner承認は復活しない。
        e.broker.current_epoch_seconds_override=None;
        assert_ne!(制御(&mut e.broker,"対話承認",json!({"要求ID":p["要求ID"],"要求hash":p["要求hash"],"表示範囲":"full"})).status,BrokerStatus::Accepted);
        let (_, reopened) = BrokerPersistentStore::open_or_create(&e.path, "audit-check").unwrap();
        assert!(reopened.audit_log.events().iter().any(|event| event.reason == "期限超過で対話を隔離"));
    }

    #[test]
    #[allow(non_snake_case)]
    fn Mobileの読み取り投影は既存Broker統治経路だけを通りowner作用を拒否する(){
        let mut e=環境生成();
        let agent_fixture: Value = serde_json::from_str(include_str!(
            "../../../../examples/contracts/agent_adapter.valid.json"
        ))
        .expect("正常Agent Adapter fixture");
        e.broker
            .実行系登録(
                "fixture-agent",
                Arc::new(AgentMetadataAdapter::new(agent_fixture.clone())),
            )
            .unwrap();
        let agent_workspace=e.path.join("mobile-agent-workspace");
        std::fs::create_dir(&agent_workspace).unwrap();
        e.broker.作業領域登録(
            "fixture-agent",
            "mobile-fixture-workspace",
            cap_std::fs::Dir::open_ambient_dir(&agent_workspace,cap_std::ambient_authority()).unwrap(),
            &[],
        ).unwrap();
        let invitation=制御(&mut e.broker,"端末招待",json!({"端末ID":"c".repeat(32),"接続先Host":"127.0.0.1"})).body.unwrap();
        let credential=通信(&mut e.broker,&invitation,"端末結合",json!({})).body.unwrap();
        for (operation,payload) in [
            ("Agent一覧",json!({})),
            ("作業領域一覧",json!({})),
            ("実行系ライフサイクル状態",json!({"版":1,"実行系ID":"local"})),
            ("実行系資源観測",json!({"版":1,"実行系ID":"local"})),
            ("通知一覧",json!({"版":1,"未読のみ":false,"上限":64})),
            ("全Runtime停止要求",json!({"版":1})),
            ("対話履歴閲覧状態",json!({})),
        ] {
            let response=通信(&mut e.broker,&credential,operation,payload);
            assert_eq!(response.status,BrokerStatus::Accepted,"{operation}");
            assert_eq!(response.evidence_source,EVIDENCE_SOURCE_INTERNAL_STATE,"{operation} evidence source");
        }
        let agent_list=通信(&mut e.broker,&credential,"Agent一覧",json!({}));
        let agent_body=agent_list.body.as_ref().expect("Agent一覧projection");
        assert_eq!(agent_body.as_object().map(serde_json::Map::len),Some(1));
        assert!(agent_body.get("Agent").is_some_and(Value::is_array));
        assert_eq!(agent_body["Agent"], json!([agent_fixture]));
        let workspace_list=通信(&mut e.broker,&credential,"作業領域一覧",json!({}));
        assert_eq!(workspace_list.status,BrokerStatus::Accepted);
        assert_eq!(workspace_list.evidence_source,EVIDENCE_SOURCE_INTERNAL_STATE);
        let workspace_entries=workspace_list.body.as_ref().unwrap()["作業領域"].as_array().unwrap();
        assert_eq!(workspace_entries.len(),1);
        assert_eq!(workspace_entries[0],json!({"作業領域ID":"mobile-fixture-workspace","実行系ID":"fixture-agent"}));
        assert!(!workspace_list.body.as_ref().unwrap().to_string().contains("登録hash"));
        assert!(!workspace_list.body.as_ref().unwrap().to_string().contains("approval_id"));
        assert!(!workspace_list.body.as_ref().unwrap().to_string().contains("mobile-agent-workspace"));
        assert_eq!(通信(&mut e.broker,&credential,"対話セッション一覧",json!({})).status,BrokerStatus::Rejected);
        assert_eq!(通信(&mut e.broker,&credential,"対話開始",json!({"実行系ID":"fixture-agent"})).status,BrokerStatus::Rejected);
        let started=制御(&mut e.broker,"対話開始",json!({"実行系ID":"fixture-agent","作業領域ID":"mobile-fixture-workspace"}));
        assert_eq!(started.status,BrokerStatus::Accepted);
        let created_audit_id=started.audit_event_id.clone();
        let listed=制御(&mut e.broker,"対話セッション一覧",json!({}));
        assert_eq!(listed.status,BrokerStatus::Accepted);
        assert_eq!(listed.evidence_source,EVIDENCE_SOURCE_INTERNAL_STATE);
        let listed_body=listed.body.as_ref().expect("対話セッション一覧");
        assert_eq!(listed_body["版"],1);
        assert_eq!(listed_body["対話セッション"].as_array().unwrap().len(),1);
        assert_eq!(listed_body["対話セッション"][0]["作業領域ID"],"mobile-fixture-workspace");
        assert_eq!(listed_body["対話セッション"][0]["作業領域結合監査ID"],created_audit_id);
        assert_ne!(listed_body["対話セッション"][0]["作成監査ID"],created_audit_id);
        assert_eq!(listed_body["対話セッション"][0].as_object().unwrap().len(),6);
        assert_eq!(通信(&mut e.broker,&credential,"対話履歴閲覧",json!({"approval_id":"a","query":{}})).status,BrokerStatus::Rejected);
        assert_eq!(通信(&mut e.broker,&credential,"Agent一覧",json!({"authority":"owner"})).status,BrokerStatus::Rejected);
        assert_eq!(通信(&mut e.broker,&credential,"作業領域一覧",json!({"実行系ID":"fixture-agent"})).status,BrokerStatus::Rejected);
        assert_eq!(通信(&mut e.broker,&credential,"対話履歴承認",json!({"実行系ID":"local"})).status,BrokerStatus::Rejected);
        assert_eq!(通信(&mut e.broker,&credential,"MCP接続一覧",json!({"版":1})).status,BrokerStatus::Rejected);
    }
}

#[cfg(test)]
#[path = "../../tests/unit/workspace_protocol.rs"]
mod workspace_tests;

#[path = "replay.rs"]
mod replay;

#[cfg(all(test, windows))]
#[path = "../../tests/unit/content_save.rs"]
mod content_save_tests;

#[path = "content_control.rs"]
mod content_control;

#[path = "content_delete.rs"]
mod content_delete;

#[path = "content_inventory.rs"]
mod content_inventory;

#[path = "content_recovery.rs"]
mod content_recovery;

#[path = "content_discard.rs"]
mod content_discard;

#[path = "content_discard_recovery.rs"]
mod content_discard_recovery;
