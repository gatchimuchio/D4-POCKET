//! 対話の要求・承認・結果を所有する。実行系固有の通信やUIは所有しない。
#![allow(non_snake_case)]

use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::time::{Duration, Instant};

use crate::audit_hash::sha256_tagged;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 対話失敗 {
    要求不正,
    実行系不在,
    権限拒否,
    セッション不一致,
    通信失敗,
    期限超過,
    応答不正,
    監査失敗,
    取消,
    隔離済み,
    作業領域不在,
    AgentTask非対応,
}
impl 対話失敗 {
    pub fn 分類(self) -> &'static str {
        match self {
            Self::要求不正 => "要求不正",
            Self::実行系不在 => "実行系不在",
            Self::権限拒否 => "権限拒否",
            Self::セッション不一致 => "セッション不一致",
            Self::通信失敗 => "通信失敗",
            Self::期限超過 => "期限超過",
            Self::応答不正 => "応答不正",
            Self::監査失敗 => "監査失敗",
            Self::取消 => "取消",
            Self::隔離済み => "実行系隔離済み",
            Self::作業領域不在 => "作業領域不在",
            Self::AgentTask非対応 => "AgentTask実行非対応",
        }
    }
    pub fn 復旧(self) -> &'static str {
        match self {
            Self::要求不正 => "入力修正",
            Self::実行系不在 => "実行系再確認",
            Self::権限拒否 => "権限再確認",
            Self::監査失敗 => "監査修復",
            Self::セッション不一致 | Self::取消 | Self::期限超過 => "新規セッション",
            Self::通信失敗 | Self::応答不正 => "接続再確認",
            Self::隔離済み => "RecoveryActionによる隔離判断を確認",
            Self::作業領域不在 => "RuntimeとWorkspaceの登録対応を再確認",
            Self::AgentTask非対応 => "Agent Task実行対応済みRuntimeを選択",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct 対話要求 {
    pub 要求ID: String,
    pub 実行系ID: String,
    pub 対話セッションID: String,
    pub 入力: String,
}

pub struct 実行結果 {
    pub 対話セッションID: String,
    pub 本文: String,
    pub 参照: Vec<String>,
    pub 能力: Vec<String>,
    pub 経路: String,
    pub 追跡ID: String,
    pub 追跡hash: String,
    pub 保留: bool,
    pub 生応答: Vec<u8>,
}

/// Agent Adapterが起動時に固定したWorkspace directoryの実体識別子。
/// Broker登録rootとの一致照合にのみ使い、Permissionやpathアクセスを表さない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentTaskWorkspaceIdentity {
    device: u64,
    file_id: u64,
}

impl AgentTaskWorkspaceIdentity {
    pub const fn new(device: u64, file_id: u64) -> Self {
        Self { device, file_id }
    }

    pub(crate) const fn from_directory_identity(
        identity: super::workspace_root::DirectoryIdentity,
    ) -> Self {
        Self::new(identity.device, identity.file_id)
    }
}

pub trait 実行系Adapter: Send + Sync {
    fn 接続対象(&self) -> String;
    /// Agent Taskの対象WorkspaceとAdapterが固定した実体rootを照合する。
    /// 未提供はTask操作の拒否条件とし、metadata宣言で代替しない。
    fn 作業領域実体識別子(&self) -> Option<AgentTaskWorkspaceIdentity> {
        None
    }
    /// Agent Adapterとして表示できる宣言だけを返す。Noneは通常Runtimeであり、
    /// Runtime metadataをAgent authorityへ昇格させない。
    fn agent_metadata(&self) -> Option<Value> {
        None
    }
    /// Agent Taskを実行する固定Adapter経路。既定では非対応とし、metadataだけで
    /// 実行能力を追加できない。実装は期限・取消を守り、本文を外部へ記録しない。
    fn AgentTask実行対応(&self) -> bool {
        false
    }
    fn AgentTask実行(
        &self,
        _instruction: &str,
        _cancel: &AtomicBool,
        _deadline: Instant,
        _context: Option<super::agent_task_scratch::AgentTaskScratchContext>,
    ) -> Result<String, 対話失敗> {
        Err(対話失敗::AgentTask非対応)
    }
    /// OS資源観測のためにBrokerだけが読むloopback接続先。権限や操作対象を生成しない。
    fn 観測対象(&self) -> Option<SocketAddr> {
        None
    }
    fn 応答(
        &self,
        要求: &対話要求,
        取消: &AtomicBool,
        期限: Instant,
        生受信: &mut Vec<Vec<u8>>,
    ) -> Result<実行結果, 対話失敗>;
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentAdapterMetadata {
    adapter_id: String,
    agent_id: String,
    provider: String,
    version: String,
    model: String,
    status: String,
    capabilities: Vec<AgentCapabilityMetadata>,
    workspace_requirements: AgentWorkspaceMetadata,
    tool_support: AgentSupportMetadata,
    mcp_support: AgentSupportMetadata,
    session_support: AgentSupportMetadata,
    cancellation_support: AgentSupportMetadata,
    usage_metrics_support: AgentSupportMetadata,
    cost_metrics_support: AgentSupportMetadata,
    authentication: AgentAuthenticationMetadata,
    host_requirements: AgentHostMetadata,
    #[serde(default, deserialize_with = "deserialize_non_null_optional_string")]
    evidence_source: Option<String>,
    #[serde(default, deserialize_with = "deserialize_non_null_optional_string")]
    evidence_reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentCapabilityMetadata {
    capability_id: String,
    support: AgentSupportMetadata,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentSupportMetadata {
    status: String,
    reason: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentWorkspaceMetadata {
    mode: String,
    boundary_policy: String,
    secret_paths: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentAuthenticationMetadata {
    method: String,
    secret_value_present: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentHostMetadata {
    platforms: Vec<String>,
    network_scope: String,
    process_spawn: AgentSupportMetadata,
}

fn deserialize_non_null_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer).map(Some)
}

impl AgentAdapterMetadata {
    /// Adapter宣言は実行許可を与えない。未対応・不明な経路へOwner権限を発行しないための拒否条件にだけ使う。
    fn task_execution_supported(&self) -> bool {
        self.capabilities.iter().any(|capability| {
            capability.capability_id == "task_execution" && capability.support.status == "supported"
        })
    }

    fn read(value: &Value) -> Result<Self, 対話失敗> {
        if agent_metadata_contains_credential_marker(value)
            || super::protocol::metadata_attempts_authority_value(value)
        {
            return Err(対話失敗::応答不正);
        }
        let metadata: Self =
            serde_json::from_value(value.clone()).map_err(|_| 対話失敗::応答不正)?;
        if !agent_text_valid(&metadata.adapter_id)
            || !agent_text_valid(&metadata.agent_id)
            || !agent_text_valid(&metadata.provider)
            || !agent_text_valid(&metadata.version)
            || !agent_text_valid(&metadata.model)
            || !["ready", "degraded", "unavailable", "unsupported"]
                .contains(&metadata.status.as_str())
            || metadata.capabilities.len() > 64
            || metadata.capabilities.iter().any(|capability| {
                !agent_text_valid(&capability.capability_id)
                    || !agent_support_valid(&capability.support)
            })
            || metadata
                .capabilities
                .iter()
                .map(|capability| capability.capability_id.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                != metadata.capabilities.len()
            || !["required", "optional"].contains(&metadata.workspace_requirements.mode.as_str())
            || metadata.workspace_requirements.boundary_policy != "deny_outside_workspace"
            || metadata.workspace_requirements.secret_paths.len() > 64
            || metadata
                .workspace_requirements
                .secret_paths
                .iter()
                .any(|path| !agent_text_valid(path))
            || !agent_support_valid(&metadata.tool_support)
            || !agent_support_valid(&metadata.mcp_support)
            || !agent_support_valid(&metadata.session_support)
            || !agent_support_valid(&metadata.cancellation_support)
            || !agent_support_valid(&metadata.usage_metrics_support)
            || !agent_support_valid(&metadata.cost_metrics_support)
            || ![
                "none",
                "api_key_reference",
                "oauth_reference",
                "local_credential_reference",
                "unsupported",
                "unknown",
            ]
            .contains(&metadata.authentication.method.as_str())
            || metadata.authentication.secret_value_present
            || metadata.host_requirements.platforms.is_empty()
            || metadata.host_requirements.platforms.len() > 8
            || metadata.host_requirements.platforms.iter().any(|platform| {
                !["windows", "macos", "linux", "android", "ios", "unknown"]
                    .contains(&platform.as_str())
            })
            || ![
                "loopback_only",
                "outbound_allowed",
                "unsupported",
                "unknown",
            ]
            .contains(&metadata.host_requirements.network_scope.as_str())
            || !agent_support_valid(&metadata.host_requirements.process_spawn)
            || metadata.evidence_source.as_deref().is_some_and(|source| {
                ![
                    "CONFIG",
                    "INTERNAL_STATE",
                    "LIVE_RUNTIME",
                    "EXTERNAL_EVIDENCE",
                    "FIXTURE",
                ]
                .contains(&source)
            })
            || metadata
                .evidence_reason
                .as_deref()
                .is_some_and(|reason| !agent_text_valid(reason))
        {
            return Err(対話失敗::応答不正);
        }
        Ok(metadata)
    }
}

/// 全fieldから既知のcredential形式を拒否する。未知形式の秘密値不存在までは証明しない。
fn agent_metadata_contains_credential_marker(value: &Value) -> bool {
    match value {
        Value::Object(object) => object
            .values()
            .any(agent_metadata_contains_credential_marker),
        Value::Array(items) => items.iter().any(agent_metadata_contains_credential_marker),
        Value::String(value) => {
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
        _ => false,
    }
}

fn agent_text_valid(value: &str) -> bool {
    !value.is_empty() && value.chars().count() <= 256
}

fn agent_support_valid(value: &AgentSupportMetadata) -> bool {
    ["supported", "unsupported", "unknown"].contains(&value.status.as_str())
        && agent_text_valid(&value.reason)
}

fn agent_adapter_projection(
    adapters: &BTreeMap<String, Arc<dyn 実行系Adapter>>,
) -> Result<Vec<(String, Value)>, 対話失敗> {
    let mut adapter_ids = BTreeSet::new();
    let mut agent_ids = BTreeSet::new();
    let mut projection = Vec::new();
    for (runtime_id, adapter) in adapters {
        let Some(metadata) = adapter.agent_metadata() else {
            continue;
        };
        let validated = AgentAdapterMetadata::read(&metadata)?;
        if !adapter_ids.insert(validated.adapter_id) || !agent_ids.insert(validated.agent_id) {
            return Err(対話失敗::応答不正);
        }
        projection.push((runtime_id.clone(), metadata));
    }
    Ok(projection)
}

fn agent_metadata_projection(
    adapters: &BTreeMap<String, Arc<dyn 実行系Adapter>>,
) -> Result<Vec<Value>, 対話失敗> {
    Ok(agent_adapter_projection(adapters)?
        .into_iter()
        .map(|(_, metadata)| metadata)
        .collect())
}

/// Broker内の対話記録から得る統計。OS測定値ではなく、現在processの権限や健全性を示さない。
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct 実行系資源統計 {
    pub 処理中要求数: u64,
    pub 完了要求数: u64,
    pub 失敗要求数: u64,
}

/// IPCの資源観測契約と起動制御面で共用する実行系IDの境界。
/// ASCII以外、PIDや接続先を埋め込む区切り文字、空白、制御文字は許可しない。
pub(crate) fn 実行系ID妥当(id: &str) -> bool {
    let bytes = id.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 128
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(byte))
}

struct 受信結果 {
    結果: Result<実行結果, 対話失敗>,
    生受信: Vec<Vec<u8>>,
    /// Adapter呼出しが返った直後にworkerが採取する単調時刻。
    /// poll時刻を応答時間に混ぜない。
    応答完了: Option<Instant>,
}

#[derive(Serialize)]
struct セッション {
    対話セッションID: String,
    実行系ID: String,
    状態: String,
    作成監査ID: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    作業領域ID: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    作業領域結合監査ID: Option<String>,
    #[serde(skip)]
    作業領域登録hash: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Agent作業要求 {
    agent_runtime_id: String,
    session_id: String,
    workspace_id: String,
    instruction: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentTaskWorkspacePermission要求 {
    agent_runtime_id: String,
    session_id: String,
    workspace_id: String,
}

struct AgentTaskWorkspacePermission記録 {
    permission_id: String,
    agent_runtime_id: String,
    workspace_id: String,
    workspace_registration_hash: String,
    expires_at_epoch_seconds: i64,
    monotonic_expiry: Instant,
    owner_approval: Option<AgentTaskOwnerApproval記録>,
}

struct AgentTaskOwnerApproval記録 {
    instruction_hash: String,
    execution_conditions_hash: String,
    expires_at_epoch_seconds: i64,
    monotonic_expiry: Instant,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentTask識別子要求 {
    task_id: String,
}

struct AgentTask受信結果 {
    result: Result<String, 対話失敗>,
    completed_at: Instant,
}

struct AgentTask作業 {
    task_id: String,
    runtime_id: String,
    session_id: String,
    workspace_id: String,
    workspace_root_identity: AgentTaskWorkspaceIdentity,
    instruction_hash: String,
    status: &'static str,
    audit_event_id: String,
    result_hash: Option<String>,
    cancel: Arc<AtomicBool>,
    owner_cancel_requested_at: Option<Instant>,
    receiver: Option<mpsc::Receiver<AgentTask受信結果>>,
    created_at: Instant,
    deadline: Instant,
}

const AGENT_TASK_RECORD_LIMIT: usize = 128;
const AGENT_TASK_EXECUTION_LIMIT: Duration = Duration::from_secs(900);

const AGENT_TASK_EXECUTION_POLICY: &str = "gui-shell-agent-task-sandbox-v1-max-runtime-900s";

fn AgentTask結果hash化(
    result: Result<String, 対話失敗>,
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<String, 対話失敗> {
    if Instant::now() >= deadline {
        return Err(対話失敗::期限超過);
    }
    if cancel.load(Ordering::SeqCst) {
        return Err(対話失敗::取消);
    }
    result.and_then(|output| {
        if output.len() > 1_048_576 {
            Err(対話失敗::応答不正)
        } else {
            Ok(sha256_tagged(output.as_bytes()))
        }
    })
}

fn AgentTask結果完了時刻検査(
    result: Result<String, 対話失敗>,
    completed_at: Instant,
    owner_cancel_requested_at: Option<Instant>,
    deadline: Instant,
) -> Result<String, 対話失敗> {
    if completed_at >= deadline {
        return match result {
            Ok(_) | Err(対話失敗::取消) => Err(対話失敗::期限超過),
            Err(error) => Err(error),
        };
    }
    if owner_cancel_requested_at.is_some_and(|requested_at| completed_at >= requested_at) {
        return match result {
            Ok(_) => Err(対話失敗::取消),
            Err(error) => Err(error),
        };
    }
    result
}

fn AgentTask実行条件hash(
    runtime_id: &str,
    session_id: &str,
    workspace_id: &str,
    registration_hash: &str,
    permission_id: &str,
) -> String {
    let material = format!(
        "gui-shell-agent-task-execution-conditions-v1\0{runtime_id}\0{session_id}\0{workspace_id}\0{registration_hash}\0{permission_id}\0{AGENT_TASK_EXECUTION_POLICY}"
    );
    sha256_tagged(material.as_bytes())
}

fn Agent作業要求識別子妥当(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn Agent実行系ID取得(
    adapters: &BTreeMap<String, Arc<dyn 実行系Adapter>>,
    sessions: &BTreeMap<String, セッション>,
) -> Result<BTreeSet<String>, 対話失敗> {
    let candidates = sessions
        .values()
        .map(|session| session.実行系ID.as_str())
        .collect::<BTreeSet<_>>();
    let mut adapter_ids = BTreeSet::new();
    let mut agent_ids = BTreeSet::new();
    let mut agent_runtime_ids = BTreeSet::new();
    for runtime_id in candidates {
        let Some(adapter) = adapters.get(runtime_id) else {
            continue;
        };
        let Some(metadata) = adapter.agent_metadata() else {
            continue;
        };
        let validated = AgentAdapterMetadata::read(&metadata)?;
        if !adapter_ids.insert(validated.adapter_id) || !agent_ids.insert(validated.agent_id) {
            return Err(対話失敗::応答不正);
        }
        agent_runtime_ids.insert(runtime_id.to_owned());
    }
    Ok(agent_runtime_ids)
}

/// 対話開始を監査へ結び付ける正規の安全な射影。
/// Session作成と後続のC5復元検証が同じ表現を使うことで、監査IDの隣接関係を
/// 推測せずにSessionと実行系を検証できる。
fn 対話開始監査射影(対話セッションID: &str, 実行系ID: &str) -> Value {
    json!({
        "対話セッションID": 対話セッションID,
        "実行系ID": 実行系ID,
        "状態": "利用中",
    })
}

/// C5の復元検証が共有する対話開始監査hash。private入力や接続先を含まない。
pub(crate) fn 対話開始監査hash(対話セッションID: &str, 実行系ID: &str) -> String {
    let body = 対話開始監査射影(対話セッションID, 実行系ID);
    sha256_tagged(body.to_string().as_bytes())
}
struct 作業 {
    要求: 対話要求,
    要求hash: String,
    作成時刻: i64,
    開始時刻: Option<i64>,
    終了時刻: Option<i64>,
    保存済み記録hash: Option<String>,
    保存済み結果証跡: bool,
    作成監査ID: String,
    開始監査ID: Option<String>,
    終了監査ID: Option<String>,
    状態: &'static str,
    表示範囲: String,
    /// C5評価が既存の対話経路を使うための内部区別。
    /// 通常の`対話取得`からはownerを含めて結果を返さない。
    評価隔離: bool,
    取消: Arc<AtomicBool>,
    受信: Option<mpsc::Receiver<受信結果>>,
    生受信: Vec<Vec<u8>>,
    結果: Option<Result<実行結果, 対話失敗>>,
    実行期限: Option<Instant>,
    /// C5の遅延評価だけが使う単調時計。監査の壁時計やC3の資源観測値を代用しない。
    単調開始: Option<Instant>,
    単調応答Millis: Option<u64>,
}

/// 評価ラボが既存の対話統治経路から受け取る内部射影。
/// 生応答、入力、Adapter接続先は含めない。`結果` は既存の表示範囲射影である。
#[derive(Clone)]
pub(crate) struct 評価対話進捗 {
    pub 要求ID: String,
    pub 実行系ID: String,
    pub 対話セッションID: String,
    pub 状態: String,
    pub 結果: Option<Value>,
    pub 実行記録: Value,
    pub 単調応答Millis: Option<u64>,
}

/// C6が対話制御から受け取る、結果本文を含まない内部射影。
/// 要求hash、結果証跡、終了監査の相関だけをBroker内の独立登録経路へ渡す。
pub(crate) struct 回帰Case情報 {
    pub 要求ID: String,
    pub 要求hash: String,
    pub 実行系ID: String,
    pub 対話セッションID: String,
    pub 結果状態: String,
    pub 応答hash: String,
    pub 終了監査ID: String,
}

#[derive(Default)]
pub struct 対話制御 {
    実行系: BTreeMap<String, Arc<dyn 実行系Adapter>>,
    セッション: BTreeMap<String, セッション>,
    作業: BTreeMap<String, 作業>,
    失効セッション: BTreeSet<String>,
    agent_task_permissions: BTreeMap<String, AgentTaskWorkspacePermission記録>,
    agent_tasks: BTreeMap<String, AgentTask作業>,
}
impl std::fmt::Debug for 対話制御 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("対話制御")
            .field("実行系数", &self.実行系.len())
            .field("要求数", &self.作業.len())
            .finish()
    }
}
impl Drop for 対話制御 {
    fn drop(&mut self) {
        for 作業 in self.作業.values() {
            作業.取消.store(true, Ordering::SeqCst);
        }
        for 作業 in self.agent_tasks.values() {
            作業.cancel.store(true, Ordering::SeqCst);
        }
    }
}

type 監査器<'a> = dyn FnMut(&str, &str, &str) -> Result<String, 対話失敗> + 'a;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 実行系指定 {
    実行系ID: String,
    #[serde(default)]
    作業領域ID: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 送信指定 {
    対話セッションID: String,
    入力: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 要求指定 {
    要求ID: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 終了指定 {
    対話セッションID: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 承認指定 {
    要求ID: String,
    要求hash: String,
    表示範囲: String,
}

fn 読取<T: serde::de::DeserializeOwned>(値: &Value) -> Result<T, 対話失敗> {
    serde_json::from_value(値.clone()).map_err(|_| 対話失敗::要求不正)
}
fn 空入力(値: &Value) -> Result<(), 対話失敗> {
    if 値.as_object().is_some_and(|v| v.is_empty()) {
        Ok(())
    } else {
        Err(対話失敗::要求不正)
    }
}
pub fn 識別子生成() -> Result<String, 対話失敗> {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).map_err(|_| 対話失敗::要求不正)?;
    Ok(hex::encode(bytes))
}
pub(crate) fn 要求hash(要求: &対話要求) -> String {
    let 値 = serde_json::to_value(要求).expect("対話要求のJSON変換");
    sha256_tagged(
        serde_json::to_string(&値)
            .expect("対話要求の正本化")
            .as_bytes(),
    )
}

impl 対話制御 {
    /// owner保存制御だけが消費する。本文を通常IPCへ返すAPIではない。
    /// 評価隔離済み対話は評価専用ProtectedStoreから外へ移さないため拒否する。
    #[cfg(any(windows, test))]
    pub(crate) fn 保存対象(&self, payload: &Value) -> Result<Value, 対話失敗> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct 指定 {
            要求ID: String,
            要求hash: String,
        }
        let p: 指定 = 読取(payload)?;
        let work = self.作業.get(&p.要求ID).ok_or(対話失敗::要求不正)?;
        if work.評価隔離 || p.要求hash != work.要求hash || work.表示範囲 != "full" {
            return Err(対話失敗::権限拒否);
        }
        if work.状態 != "完了"
            || !work.保存済み結果証跡
            || work.保存済み記録hash.is_none()
            || work.終了監査ID.is_none()
            || !matches!(work.結果, Some(Ok(_)))
        {
            return Err(対話失敗::要求不正);
        }
        Ok(json!({"版":1,"要求":work.要求,"要求hash":work.要求hash,
            "結果":表示射影(work, work.結果.as_ref().ok_or(対話失敗::要求不正)?),
            "実行記録":実行記録(work),"結果証跡":結果証跡(work).ok_or(対話失敗::要求不正)?}))
    }

    /// C6のowner登録だけが消費する。元の入力・応答本文を返さず、現在の
    /// request/hashと永続化済み結果証跡の一致だけを証明する。
    pub(crate) fn 回帰Case情報(
        &self,
        request_id: &str,
        request_hash: &str,
    ) -> Result<回帰Case情報, 対話失敗> {
        let work = self.作業.get(request_id).ok_or(対話失敗::要求不正)?;
        if work.評価隔離
            || work.要求hash != request_hash
            || work.状態 != "完了"
            || work.表示範囲 != "full"
            || !work.保存済み結果証跡
            || work.終了監査ID.is_none()
        {
            return Err(対話失敗::権限拒否);
        }
        let result = work
            .結果
            .as_ref()
            .ok_or(対話失敗::要求不正)?
            .as_ref()
            .map_err(|_| 対話失敗::要求不正)?;
        let 結果状態 = if result.保留 { "保留" } else { "成功" };
        Ok(回帰Case情報 {
            要求ID: work.要求.要求ID.clone(),
            要求hash: work.要求hash.clone(),
            実行系ID: work.要求.実行系ID.clone(),
            対話セッションID: work.要求.対話セッションID.clone(),
            結果状態: 結果状態.to_string(),
            応答hash: sha256_tagged(&result.生応答),
            終了監査ID: work.終了監査ID.clone().ok_or(対話失敗::要求不正)?,
        })
    }

    pub(crate) fn 登録済み(&self, id: &str) -> bool {
        self.実行系.contains_key(id)
    }

    /// C5が一括の承認待ち対話を作る前に、既存対話を追い出さずに確認する上限。
    /// この値は権限・Approval・送信許可を生成しない。
    pub(crate) fn 評価要求可能数(&self) -> usize {
        128usize
            .saturating_sub(self.作業.len())
            .min(64usize.saturating_sub(self.セッション.len()))
    }

    /// C4のterminal隔離後、評価対話のworker受信側とsessionの双方が実際に
    /// 解放されたことだけをC5が確認する。外部実行の停止やC4の成功は示さない。
    pub(crate) fn 評価対話回収済み(
        &self, 要求ID: &str, 対話セッションID: &str
    ) -> bool {
        !self.作業.contains_key(要求ID) && !self.セッション.contains_key(対話セッションID)
    }

    /// C5評価だけが、送信済みでまだowner承認されていない対話を隔離する。
    /// 隔離後も既存のowner対話承認は個別に行うが、通常の`対話取得`へは公開しない。
    pub(crate) fn 評価隔離(&mut self, 要求ID: &str) -> Result<(), 対話失敗> {
        let work = self.作業.get_mut(要求ID).ok_or(対話失敗::要求不正)?;
        if work.評価隔離 || work.状態 != "承認待ち" || work.受信.is_some() || work.結果.is_some()
        {
            return Err(対話失敗::要求不正);
        }
        work.評価隔離 = true;
        Ok(())
    }

    /// C5の評価結果収集用。既存の対話状態遷移・監査保存を進めてから、
    /// Content Exposure Boundaryを適用済みの結果だけを返す。
    /// 評価器や呼出し元に生応答・入力・接続先を渡さない。
    pub(crate) fn 評価進捗(
        &mut self,
        要求ID: &str,
        現在: i64,
        監査: &mut 監査器<'_>,
    ) -> Result<評価対話進捗, 対話失敗> {
        self.進捗反映(現在, 監査)?;
        let work = self.作業.get(要求ID).ok_or(対話失敗::要求不正)?;
        if !work.評価隔離 {
            return Err(対話失敗::要求不正);
        }
        if work.状態 == "監査失敗" {
            return Err(対話失敗::監査失敗);
        }
        Ok(評価対話進捗 {
            要求ID: work.要求.要求ID.clone(),
            実行系ID: work.要求.実行系ID.clone(),
            対話セッションID: work.要求.対話セッションID.clone(),
            状態: work.状態.to_string(),
            結果: work.結果.as_ref().map(|result| 表示射影(work, result)),
            実行記録: 実行記録(work),
            単調応答Millis: work.単調応答Millis,
        })
    }

    /// C5の非隣接監査回帰だけが、workerから到着済みの結果を進捗反映前の同じ
    /// receiverへ戻すための試験用同期点。production binaryには含めない。
    #[cfg(test)]
    pub(crate) fn 試験用受信済みを進捗前に再投入(
        &mut self,
        要求ID: &str,
        deadline: Instant,
    ) -> Result<(), 対話失敗> {
        loop {
            let received = {
                let work = self.作業.get(要求ID).ok_or(対話失敗::要求不正)?;
                let receiver = work.受信.as_ref().ok_or(対話失敗::要求不正)?;
                match receiver.try_recv() {
                    Ok(value) => Some(value),
                    Err(mpsc::TryRecvError::Empty) => None,
                    Err(mpsc::TryRecvError::Disconnected) => return Err(対話失敗::通信失敗),
                }
            };
            if let Some(value) = received {
                let (sender, receiver) = mpsc::sync_channel(1);
                sender.send(value).map_err(|_| 対話失敗::通信失敗)?;
                self.作業.get_mut(要求ID).ok_or(対話失敗::要求不正)?.受信 = Some(receiver);
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(対話失敗::通信失敗);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// これはin-memory対話記録の射影であり、実行系processからのtelemetryではない。
    pub(crate) fn 資源統計(&self, 実行系ID: &str) -> 実行系資源統計 {
        let mut stats = 実行系資源統計::default();
        for work in self
            .作業
            .values()
            .filter(|work| work.要求.実行系ID == 実行系ID)
        {
            if work.状態 == "実行中" {
                stats.処理中要求数 = stats.処理中要求数.saturating_add(1);
            }
            let 完了 = matches!(work.結果, Some(_))
                && !matches!(work.状態, "承認待ち" | "実行中" | "監査失敗");
            if !完了 {
                continue;
            }
            stats.完了要求数 = stats.完了要求数.saturating_add(1);
            if matches!(work.結果, Some(Err(error)) if error != 対話失敗::取消) {
                stats.失敗要求数 = stats.失敗要求数.saturating_add(1);
            }
        }
        // 開始時刻・終了時刻は監査用の秒精度であり、ミリ秒応答時間の実測値ではない。
        // 高精度の単調時計を記録するまで、この統計に平均応答時間を持たせない。
        stats
    }

    fn agent_task_permission_is_active(
        &self,
        session_id: &str,
        runtime_id: &str,
        workspace_id: &str,
        registration_hash: &str,
        now: i64,
    ) -> bool {
        self.agent_task_permissions
            .get(session_id)
            .is_some_and(|grant| {
                now < grant.expires_at_epoch_seconds
                    && Instant::now() < grant.monotonic_expiry
                    && grant.permission_id.len() == 32
                    && grant.agent_runtime_id == runtime_id
                    && grant.workspace_id == workspace_id
                    && grant.workspace_registration_hash == registration_hash
            })
    }

    fn agent_task_owner_approval_is_active(
        &self,
        session_id: &str,
        runtime_id: &str,
        workspace_id: &str,
        registration_hash: &str,
        instruction_hash: &str,
        now: i64,
    ) -> bool {
        self.agent_task_permissions
            .get(session_id)
            .is_some_and(|grant| {
                self.agent_task_permission_is_active(
                    session_id,
                    runtime_id,
                    workspace_id,
                    registration_hash,
                    now,
                ) && grant.owner_approval.as_ref().is_some_and(|approval| {
                    now < approval.expires_at_epoch_seconds
                        && Instant::now() < approval.monotonic_expiry
                        && approval.instruction_hash == instruction_hash
                        && approval.execution_conditions_hash
                            == AgentTask実行条件hash(
                                runtime_id,
                                session_id,
                                workspace_id,
                                registration_hash,
                                &grant.permission_id,
                            )
                })
            })
    }

    /// 資格失効は監査障害時も採用停止を優先する。外部計算の停止は保証しない。
    pub(crate) fn 資格隔離(&mut self, sessions: &[String]) {
        self.失効セッション.extend(sessions.iter().cloned());
        for session in sessions {
            self.agent_task_permissions.remove(session);
        }
        for work in self.作業.values_mut() {
            if sessions.contains(&work.要求.対話セッションID) {
                work.取消.store(true, Ordering::SeqCst);
                work.状態 = "中止";
                work.結果 = Some(Err(対話失敗::取消));
                work.単調応答Millis = None;
            }
        }
        for task in self.agent_tasks.values_mut() {
            if sessions.contains(&task.session_id) && matches!(task.status, "pending" | "running") {
                task.cancel.store(true, Ordering::SeqCst);
            }
        }
        for id in sessions {
            if let Some(session) = self.セッション.get_mut(id) {
                session.状態 = "中止後隔離".into();
            }
        }
        self.失効資源解放();
    }

    fn 失効資源解放(&mut self) {
        let finished: Vec<_> = self
            .失効セッション
            .iter()
            .filter(|id| {
                !self
                    .作業
                    .values()
                    .any(|w| &w.要求.対話セッションID == *id && w.受信.is_some())
            })
            .cloned()
            .collect();
        for id in finished {
            self.作業.retain(|_, w| w.要求.対話セッションID != id);
            self.セッション.remove(&id);
            self.失効セッション.remove(&id);
        }
    }

    /// 起動制御面だけが登録する。登録は通信や送信許可を発生させない。
    pub fn 登録(
        &mut self,
        ID: &str,
        adapter: Arc<dyn 実行系Adapter>,
    ) -> Result<(), 対話失敗> {
        if !実行系ID妥当(ID) || self.実行系.contains_key(ID) {
            return Err(対話失敗::要求不正);
        }
        self.実行系.insert(ID.to_owned(), adapter);
        Ok(())
    }

    /// 起動制御面が、直後の登録監査に失敗した新規登録だけを取り消す。
    /// 対話sessionや作業を持つ実行系の管理操作には使わない。
    pub(crate) fn 直後登録取消(&mut self, ID: &str) -> bool {
        self.実行系.remove(ID).is_some()
    }

    /// terminal lifecycle隔離の後に、同じ実行系へ通常対話を再接続・再実行させない。
    /// 既存sessionと進行中要求も資格隔離と同じfail-closed処理で停止する。
    pub(crate) fn 実行系隔離(&mut self, 実行系ID: &str) {
        let sessions = self
            .セッション
            .iter()
            .filter_map(|(session_id, session)| {
                (session.実行系ID == 実行系ID).then(|| session_id.clone())
            })
            .collect::<Vec<_>>();
        self.資格隔離(&sessions);
        self.実行系.remove(実行系ID);
    }

    pub fn 操作(
        &mut self,
        操作: &str,
        値: &Value,
        owner: bool,
        現在: i64,
        監査: &mut 監査器<'_>,
    ) -> Result<Value, 対話失敗> {
        self.操作_作業領域結合済み(操作, 値, owner, 現在, None, 監査)
    }

    pub(crate) fn 操作_作業領域結合済み(
        &mut self,
        操作: &str,
        値: &Value,
        owner: bool,
        現在: i64,
        作業領域結合: Option<&super::workspace::DialogueWorkspaceBinding>,
        監査: &mut 監査器<'_>,
    ) -> Result<Value, 対話失敗> {
        #[cfg(test)]
        let scratch_journal = Some(super::agent_task_scratch::AgentTaskScratchJournal::in_memory());
        #[cfg(not(test))]
        let scratch_journal = None;
        self.操作_作業領域結合済み_scratch(
            操作,
            値,
            owner,
            現在,
            作業領域結合,
            scratch_journal,
            監査,
        )
    }

    pub(crate) fn 操作_作業領域結合済み_scratch(
        &mut self,
        操作: &str,
        値: &Value,
        owner: bool,
        現在: i64,
        作業領域結合: Option<&super::workspace::DialogueWorkspaceBinding>,
        scratch_journal: Option<super::agent_task_scratch::AgentTaskScratchJournal>,
        監査: &mut 監査器<'_>,
    ) -> Result<Value, 対話失敗> {
        if matches!(操作, "対話承認" | "対話承認待ち") && !owner {
            return Err(対話失敗::権限拒否);
        }
        self.進捗反映(現在, 監査)?;
        self.AgentTask進捗反映(現在, 監査)?;
        let result = match 操作 {
            "実行系列挙" => {
                空入力(値)?;
                Ok(json!({"実行系": self.実行系.keys().collect::<Vec<_>>()}))
            }
            "対話セッション一覧" => {
                空入力(値)?;
                let agent_runtime_ids = Agent実行系ID取得(&self.実行系, &self.セッション)?;
                let agent_sessions = self
                    .セッション
                    .values()
                    .filter(|session| agent_runtime_ids.contains(&session.実行系ID))
                    .collect::<Vec<_>>();
                if agent_sessions.iter().any(|session| {
                    session.作業領域ID.is_none() || session.作業領域結合監査ID.is_none()
                }) {
                    return Err(対話失敗::応答不正);
                }
                Ok(json!({
                    "版": 1,
                    "対話セッション": agent_sessions,
                }))
            }
            "Agent一覧" => {
                空入力(値)?;
                Ok(json!({"Agent": agent_metadata_projection(&self.実行系)?}))
            }
            "Agent作業要求検査" => {
                let request: Agent作業要求 = 読取(値)?;
                if !Agent作業要求識別子妥当(&request.agent_runtime_id)
                    || !Agent作業要求識別子妥当(&request.session_id)
                    || !Agent作業要求識別子妥当(&request.workspace_id)
                    || request.instruction.trim().is_empty()
                    || request.instruction.chars().count() > 32_768
                {
                    return Err(対話失敗::要求不正);
                }
                let adapter = self
                    .実行系
                    .get(&request.agent_runtime_id)
                    .ok_or(対話失敗::実行系不在)?;
                let metadata = adapter.agent_metadata().ok_or(対話失敗::実行系不在)?;
                let metadata = AgentAdapterMetadata::read(&metadata)?;
                if !metadata.task_execution_supported() {
                    return Err(対話失敗::AgentTask非対応);
                }
                let binding = 作業領域結合.ok_or(対話失敗::作業領域不在)?;
                if !binding.matches(&request.agent_runtime_id, &request.workspace_id) {
                    return Err(対話失敗::作業領域不在);
                }
                if adapter.作業領域実体識別子() != Some(binding.root_identity()) {
                    return Err(対話失敗::作業領域不在);
                }
                let session = self
                    .セッション
                    .get(&request.session_id)
                    .ok_or(対話失敗::セッション不一致)?;
                if session.状態 != "利用中"
                    || session.実行系ID != request.agent_runtime_id
                    || session.作業領域ID.as_deref() != Some(request.workspace_id.as_str())
                    || session.作業領域登録hash.as_deref() != Some(binding.registration_hash())
                {
                    return Err(対話失敗::セッション不一致);
                }
                let permission_state = if self.agent_task_permission_is_active(
                    &request.session_id,
                    &request.agent_runtime_id,
                    &request.workspace_id,
                    binding.registration_hash(),
                    現在,
                ) {
                    "有効"
                } else {
                    "未付与"
                };
                let instruction_hash = sha256_tagged(request.instruction.as_bytes());
                let approval_state = if self.agent_task_owner_approval_is_active(
                    &request.session_id,
                    &request.agent_runtime_id,
                    &request.workspace_id,
                    binding.registration_hash(),
                    &instruction_hash,
                    現在,
                ) {
                    "有効"
                } else {
                    "未取得"
                };
                Ok(json!({
                    "版": 1,
                    "状態": "要求検査済み",
                    "実行状態": "未実行",
                    "Permission状態": permission_state,
                    "Approval状態": approval_state,
                    "実行系ID": request.agent_runtime_id,
                    "対話セッションID": request.session_id,
                    "作業領域ID": request.workspace_id,
                    "指示hash": instruction_hash,
                }))
            }
            "AgentTaskWorkspacePermissionGrant" => {
                if !owner {
                    return Err(対話失敗::権限拒否);
                }
                let request: AgentTaskWorkspacePermission要求 = 読取(値)?;
                if !Agent作業要求識別子妥当(&request.agent_runtime_id)
                    || !Agent作業要求識別子妥当(&request.session_id)
                    || !Agent作業要求識別子妥当(&request.workspace_id)
                {
                    return Err(対話失敗::要求不正);
                }
                let adapter = self
                    .実行系
                    .get(&request.agent_runtime_id)
                    .ok_or(対話失敗::実行系不在)?;
                let metadata = adapter.agent_metadata().ok_or(対話失敗::実行系不在)?;
                let metadata = AgentAdapterMetadata::read(&metadata)?;
                if !metadata.task_execution_supported() {
                    return Err(対話失敗::AgentTask非対応);
                }
                let binding = 作業領域結合.ok_or(対話失敗::作業領域不在)?;
                if !binding.matches(&request.agent_runtime_id, &request.workspace_id) {
                    return Err(対話失敗::作業領域不在);
                }
                if adapter.作業領域実体識別子() != Some(binding.root_identity()) {
                    return Err(対話失敗::作業領域不在);
                }
                let session = self
                    .セッション
                    .get(&request.session_id)
                    .ok_or(対話失敗::セッション不一致)?;
                if session.状態 != "利用中"
                    || session.実行系ID != request.agent_runtime_id
                    || session.作業領域ID.as_deref() != Some(request.workspace_id.as_str())
                    || session.作業領域登録hash.as_deref() != Some(binding.registration_hash())
                    || self.失効セッション.contains(&request.session_id)
                {
                    return Err(対話失敗::セッション不一致);
                }
                if self.agent_task_permission_is_active(
                    &request.session_id,
                    &request.agent_runtime_id,
                    &request.workspace_id,
                    binding.registration_hash(),
                    現在,
                ) {
                    return Err(対話失敗::権限拒否);
                }
                let permission_id = 識別子生成()?;
                let expires_at_epoch_seconds = 現在.saturating_add(300);
                let registration_hash = binding.registration_hash().to_owned();
                self.agent_task_permissions.insert(
                    request.session_id.clone(),
                    AgentTaskWorkspacePermission記録 {
                        permission_id: permission_id.clone(),
                        agent_runtime_id: request.agent_runtime_id.clone(),
                        workspace_id: request.workspace_id.clone(),
                        workspace_registration_hash: registration_hash.clone(),
                        expires_at_epoch_seconds,
                        monotonic_expiry: Instant::now() + Duration::from_secs(300),
                        owner_approval: None,
                    },
                );
                if let Err(error) = 監査(
                    "Agent Task用Workspace Permission発行（native Owner確認・Task未実行）",
                    &request.session_id,
                    &sha256_tagged(permission_id.as_bytes()),
                ) {
                    self.agent_task_permissions.remove(&request.session_id);
                    return Err(error);
                }
                Ok(json!({
                    "permission_id": permission_id,
                    "agent_runtime_id": request.agent_runtime_id,
                    "session_id": request.session_id,
                    "workspace_id": request.workspace_id,
                    "workspace_registration_hash": registration_hash,
                    "operation": "agent_task.execute",
                    "scope": "session_workspace_once",
                    "decision": "allow",
                    "source": "owner",
                    "expires_at_epoch_seconds": expires_at_epoch_seconds,
                    "use_limit": 1,
                    "uses_remaining": 1,
                    "status": "active"
                }))
            }
            "AgentTaskOwnerApprovalGrant" => {
                if !owner {
                    return Err(対話失敗::権限拒否);
                }
                let request: Agent作業要求 = 読取(値)?;
                if !Agent作業要求識別子妥当(&request.agent_runtime_id)
                    || !Agent作業要求識別子妥当(&request.session_id)
                    || !Agent作業要求識別子妥当(&request.workspace_id)
                    || request.instruction.trim().is_empty()
                    || request.instruction.chars().count() > 32_768
                {
                    return Err(対話失敗::要求不正);
                }
                let adapter = self
                    .実行系
                    .get(&request.agent_runtime_id)
                    .ok_or(対話失敗::実行系不在)?;
                let metadata = adapter.agent_metadata().ok_or(対話失敗::実行系不在)?;
                let metadata = AgentAdapterMetadata::read(&metadata)?;
                if !metadata.task_execution_supported() {
                    return Err(対話失敗::AgentTask非対応);
                }
                let binding = 作業領域結合.ok_or(対話失敗::作業領域不在)?;
                if !binding.matches(&request.agent_runtime_id, &request.workspace_id) {
                    return Err(対話失敗::作業領域不在);
                }
                if adapter.作業領域実体識別子() != Some(binding.root_identity()) {
                    return Err(対話失敗::作業領域不在);
                }
                let session = self
                    .セッション
                    .get(&request.session_id)
                    .ok_or(対話失敗::セッション不一致)?;
                if session.状態 != "利用中"
                    || session.実行系ID != request.agent_runtime_id
                    || session.作業領域ID.as_deref() != Some(request.workspace_id.as_str())
                    || session.作業領域登録hash.as_deref() != Some(binding.registration_hash())
                    || self.失効セッション.contains(&request.session_id)
                {
                    return Err(対話失敗::セッション不一致);
                }
                let permission = self
                    .agent_task_permissions
                    .get(&request.session_id)
                    .filter(|grant| {
                        self.agent_task_permission_is_active(
                            &request.session_id,
                            &request.agent_runtime_id,
                            &request.workspace_id,
                            binding.registration_hash(),
                            現在,
                        ) && grant.agent_runtime_id == request.agent_runtime_id
                            && grant.workspace_id == request.workspace_id
                    })
                    .ok_or(対話失敗::権限拒否)?;
                let instruction_hash = sha256_tagged(request.instruction.as_bytes());
                let execution_conditions_hash = AgentTask実行条件hash(
                    &request.agent_runtime_id,
                    &request.session_id,
                    &request.workspace_id,
                    binding.registration_hash(),
                    &permission.permission_id,
                );
                let expires_at_epoch_seconds = 現在.saturating_add(300);
                監査(
                    "Agent Task本文hash・実行条件hashへのOwner Approval発行（native確認・Task未実行）",
                    &request.session_id,
                    &execution_conditions_hash,
                )?;
                let approval = AgentTaskOwnerApproval記録 {
                    instruction_hash: instruction_hash.clone(),
                    execution_conditions_hash: execution_conditions_hash.clone(),
                    expires_at_epoch_seconds,
                    monotonic_expiry: Instant::now() + Duration::from_secs(300),
                };
                self.agent_task_permissions
                    .get_mut(&request.session_id)
                    .ok_or(対話失敗::権限拒否)?
                    .owner_approval = Some(approval);
                Ok(json!({
                    "状態": "Owner Approval発行済み",
                    "実行状態": "未実行",
                    "実行系ID": request.agent_runtime_id,
                    "対話セッションID": request.session_id,
                    "作業領域ID": request.workspace_id,
                    "指示hash": instruction_hash,
                    "実行条件hash": execution_conditions_hash,
                    "適用ポリシー": AGENT_TASK_EXECUTION_POLICY,
                    "expires_at_epoch_seconds": expires_at_epoch_seconds,
                    "use_limit": 1,
                    "uses_remaining": 1,
                    "status": "issued_unconsumed"
                }))
            }
            "AgentTask実行" => {
                let request: Agent作業要求 = 読取(値)?;
                if !Agent作業要求識別子妥当(&request.agent_runtime_id)
                    || !Agent作業要求識別子妥当(&request.session_id)
                    || !Agent作業要求識別子妥当(&request.workspace_id)
                    || request.instruction.trim().is_empty()
                    || request.instruction.chars().count() > 32_768
                {
                    return Err(対話失敗::要求不正);
                }
                let adapter = Arc::clone(
                    self.実行系
                        .get(&request.agent_runtime_id)
                        .ok_or(対話失敗::実行系不在)?,
                );
                let metadata = adapter.agent_metadata().ok_or(対話失敗::実行系不在)?;
                if !AgentAdapterMetadata::read(&metadata)?.task_execution_supported()
                    || !adapter.AgentTask実行対応()
                {
                    return Err(対話失敗::AgentTask非対応);
                }
                let binding = 作業領域結合.ok_or(対話失敗::作業領域不在)?;
                if !binding.matches(&request.agent_runtime_id, &request.workspace_id)
                    || adapter.作業領域実体識別子() != Some(binding.root_identity())
                {
                    return Err(対話失敗::作業領域不在);
                }
                let scratch_journal = scratch_journal.clone().ok_or(対話失敗::AgentTask非対応)?;
                if scratch_journal.has_pending_workspace(&request.workspace_id) {
                    return Err(対話失敗::権限拒否);
                }
                let session = self
                    .セッション
                    .get(&request.session_id)
                    .ok_or(対話失敗::セッション不一致)?;
                if session.状態 != "利用中"
                    || session.実行系ID != request.agent_runtime_id
                    || session.作業領域ID.as_deref() != Some(request.workspace_id.as_str())
                    || session.作業領域登録hash.as_deref() != Some(binding.registration_hash())
                    || self.失効セッション.contains(&request.session_id)
                {
                    return Err(対話失敗::セッション不一致);
                }
                if self.agent_tasks.values().any(|task| {
                    matches!(task.status, "pending" | "running")
                        && (task.session_id == request.session_id
                            || task.workspace_id == request.workspace_id
                            || task.workspace_root_identity == binding.root_identity())
                }) {
                    return Err(対話失敗::権限拒否);
                }
                let instruction_hash = sha256_tagged(request.instruction.as_bytes());
                if !self.agent_task_owner_approval_is_active(
                    &request.session_id,
                    &request.agent_runtime_id,
                    &request.workspace_id,
                    binding.registration_hash(),
                    &instruction_hash,
                    現在,
                ) {
                    return Err(対話失敗::権限拒否);
                }
                let grant = self
                    .agent_task_permissions
                    .get(&request.session_id)
                    .ok_or(対話失敗::権限拒否)?;
                // Grant期限は開始許可の期限。消費後の実行時間は別の固定上限でboundedにする。
                let deadline = Instant::now() + AGENT_TASK_EXECUTION_LIMIT;
                let permission_id = grant.permission_id.clone();
                if Instant::now() >= deadline
                    || self
                        .agent_tasks
                        .values()
                        .filter(|task| task.status == "running")
                        .count()
                        >= 4
                {
                    return Err(対話失敗::権限拒否);
                }
                while self.agent_tasks.len() >= AGENT_TASK_RECORD_LIMIT {
                    let oldest_terminal = self
                        .agent_tasks
                        .iter()
                        .filter(|(_, task)| !matches!(task.status, "pending" | "running"))
                        .min_by_key(|(_, task)| task.created_at)
                        .map(|(task_id, _)| task_id.clone());
                    if let Some(task_id) = oldest_terminal {
                        self.agent_tasks.remove(&task_id);
                    } else {
                        return Err(対話失敗::要求不正);
                    }
                }
                let task_id = 識別子生成()?;
                let start_hash = sha256_tagged(
                    json!({
                        "task_id": task_id,
                        "agent_runtime_id": request.agent_runtime_id,
                        "session_id": request.session_id,
                        "workspace_id": request.workspace_id,
                        "instruction_hash": instruction_hash,
                        "execution_conditions_hash": AgentTask実行条件hash(
                            &request.agent_runtime_id,
                            &request.session_id,
                            &request.workspace_id,
                            binding.registration_hash(),
                            &permission_id,
                        ),
                    })
                    .to_string()
                    .as_bytes(),
                );
                let start_audit_id = 監査(
                    "Agent Task実行開始（Permission／Owner Approval一回消費）",
                    &task_id,
                    &start_hash,
                )?;
                // Audit確定後、同じBroker排他区間内でgrantを消費してからworkerを起動する。
                self.agent_task_permissions.remove(&request.session_id);
                let cancel = Arc::new(AtomicBool::new(false));
                let worker_cancel = Arc::clone(&cancel);
                let instruction = request.instruction;
                let execution_context = super::agent_task_scratch::AgentTaskScratchContext {
                    task_id: task_id.clone(),
                    runtime_id: request.agent_runtime_id.clone(),
                    workspace_id: request.workspace_id.clone(),
                    recovery_binding_hash: binding.recovery_binding_hash().to_string(),
                    root_identity: binding.root_directory_identity(),
                    secret_paths: binding.secret_paths().to_vec(),
                    journal: scratch_journal,
                };
                let (send, receive) = mpsc::sync_channel(1);
                let spawn_result = std::thread::Builder::new()
                    .name("AgentTask実行".into())
                    .spawn(move || {
                        let result = if Instant::now() >= deadline {
                            Err(対話失敗::期限超過)
                        } else if worker_cancel.load(Ordering::SeqCst) {
                            Err(対話失敗::取消)
                        } else {
                            AgentTask結果hash化(
                                adapter.AgentTask実行(
                                    &instruction,
                                    &worker_cancel,
                                    deadline,
                                    Some(execution_context),
                                ),
                                &worker_cancel,
                                deadline,
                            )
                        };
                        let completed_at = Instant::now();
                        let _ = send.send(AgentTask受信結果 {
                            result,
                            completed_at,
                        });
                    });
                let (status, receiver, audit_event_id) = match spawn_result {
                    Ok(_) => ("running", Some(receive), start_audit_id),
                    Err(_) => {
                        let failure_hash = sha256_tagged(
                            json!({"task_id": task_id, "status": "failed", "reason": "worker_start_failed"})
                                .to_string()
                                .as_bytes(),
                        );
                        let failure_audit = 監査(
                            "Agent Task worker起動失敗 RecoveryAction=再試行前にRuntime状態確認",
                            &task_id,
                            &failure_hash,
                        );
                        let failure_audit_id = match failure_audit {
                            Ok(event_id) => event_id,
                            Err(error) => {
                                self.agent_tasks.insert(
                                    task_id.clone(),
                                    AgentTask作業 {
                                        task_id: task_id.clone(),
                                        runtime_id: request.agent_runtime_id,
                                        session_id: request.session_id,
                                        workspace_id: request.workspace_id,
                                        workspace_root_identity: binding.root_identity(),
                                        instruction_hash,
                                        status: "quarantined",
                                        audit_event_id: start_audit_id,
                                        result_hash: None,
                                        cancel,
                                        owner_cancel_requested_at: None,
                                        receiver: None,
                                        created_at: Instant::now(),
                                        deadline,
                                    },
                                );
                                return Err(error);
                            }
                        };
                        ("failed", None, failure_audit_id)
                    }
                };
                let task = AgentTask作業 {
                    task_id: task_id.clone(),
                    runtime_id: request.agent_runtime_id,
                    session_id: request.session_id,
                    workspace_id: request.workspace_id,
                    workspace_root_identity: binding.root_identity(),
                    instruction_hash,
                    status,
                    audit_event_id,
                    result_hash: None,
                    cancel,
                    owner_cancel_requested_at: None,
                    receiver,
                    created_at: Instant::now(),
                    deadline,
                };
                let body = AgentTask状態射影(&task);
                self.agent_tasks.insert(task_id, task);
                Ok(body)
            }
            "AgentTask状態" => {
                let request: AgentTask識別子要求 = 読取(値)?;
                if !Agent作業要求識別子妥当(&request.task_id) {
                    return Err(対話失敗::要求不正);
                }
                let task = self
                    .agent_tasks
                    .get(&request.task_id)
                    .ok_or(対話失敗::要求不正)?;
                Ok(AgentTask状態射影(task))
            }
            "AgentTask取消" => {
                let request: AgentTask識別子要求 = 読取(値)?;
                if !Agent作業要求識別子妥当(&request.task_id) {
                    return Err(対話失敗::要求不正);
                }
                let task = self
                    .agent_tasks
                    .get(&request.task_id)
                    .filter(|task| matches!(task.status, "pending" | "running"))
                    .ok_or(対話失敗::要求不正)?;
                let event_hash = sha256_tagged(
                    json!({"task_id": request.task_id, "instruction_hash": task.instruction_hash})
                        .to_string()
                        .as_bytes(),
                );
                監査(
                    "Agent Task取消要求（Adapter終了確認待ち）",
                    &request.task_id,
                    &event_hash,
                )?;
                let task = self
                    .agent_tasks
                    .get_mut(&request.task_id)
                    .ok_or(対話失敗::要求不正)?;
                task.owner_cancel_requested_at
                    .get_or_insert_with(Instant::now);
                task.cancel.store(true, Ordering::SeqCst);
                Ok(AgentTask状態射影(task))
            }
            "対話開始" => {
                let 指定: 実行系指定 = 読取(値)?;
                let adapter = self
                    .実行系
                    .get(&指定.実行系ID)
                    .ok_or(対話失敗::実行系不在)?;
                let agent = adapter
                    .agent_metadata()
                    .map(|metadata| AgentAdapterMetadata::read(&metadata))
                    .transpose()?;
                let workspace_id = if agent.is_some() {
                    let binding = 作業領域結合.ok_or(対話失敗::作業領域不在)?;
                    let workspace_id = 指定.作業領域ID.as_deref().ok_or(対話失敗::作業領域不在)?;
                    if !binding.matches(&指定.実行系ID, workspace_id) {
                        return Err(対話失敗::作業領域不在);
                    }
                    Some(binding.workspace_id().to_owned())
                } else {
                    match (指定.作業領域ID.as_deref(), 作業領域結合) {
                        (None, None) => None,
                        (Some(workspace_id), Some(binding))
                            if binding.matches(&指定.実行系ID, workspace_id) =>
                        {
                            Some(binding.workspace_id().to_owned())
                        }
                        _ => return Err(対話失敗::作業領域不在),
                    }
                };
                if self.セッション.len() >= 64 {
                    return Err(対話失敗::要求不正);
                }
                let ID = 識別子生成()?;
                let mut session = セッション {
                    対話セッションID: ID.clone(),
                    実行系ID: 指定.実行系ID,
                    状態: "利用中".into(),
                    作成監査ID: String::new(),
                    作業領域ID: workspace_id,
                    作業領域結合監査ID: None,
                    作業領域登録hash: 作業領域結合
                        .map(|binding| binding.registration_hash().to_owned()),
                };
                let body = 対話開始監査射影(&session.対話セッションID, &session.実行系ID);
                let 作成監査ID = 監査(
                    "対話開始",
                    &ID,
                    &対話開始監査hash(&session.対話セッションID, &session.実行系ID),
                )?;
                session.作成監査ID = 作成監査ID;
                if let Some(binding) = 作業領域結合 {
                    session.作業領域結合監査ID = Some(監査(
                        "対話Sessionと登録済みWorkspaceの明示結合",
                        &ID,
                        &binding.audit_hash(&ID),
                    )?);
                }
                self.セッション.insert(ID, session);
                Ok(body)
            }
            "対話送信" => {
                let 指定: 送信指定 = 読取(値)?;
                if 指定.入力.trim().is_empty()
                    || 指定.入力.chars().count() > 4096
                    || self.作業.len() >= 128
                {
                    return Err(対話失敗::要求不正);
                }
                let session = self
                    .セッション
                    .get(&指定.対話セッションID)
                    .ok_or(対話失敗::セッション不一致)?;
                if session.状態 != "利用中"
                    || self.作業.values().any(|v| {
                        v.要求.対話セッションID == 指定.対話セッションID
                            && matches!(v.状態, "承認待ち" | "実行中")
                    })
                {
                    return Err(対話失敗::セッション不一致);
                }
                let 要求 = 対話要求 {
                    要求ID: 識別子生成()?,
                    実行系ID: session.実行系ID.clone(),
                    対話セッションID: 指定.対話セッションID,
                    入力: 指定.入力,
                };
                let hash = 要求hash(&要求);
                let 作成監査ID = 監査("対話承認待ち作成", &要求.要求ID, &hash)?;
                let body = json!({"要求ID": 要求.要求ID, "要求hash": hash, "状態": "承認待ち", "期限": 現在 + 300});
                self.作業.insert(
                    要求.要求ID.clone(),
                    作業 {
                        要求,
                        要求hash: hash,
                        作成時刻: 現在,
                        開始時刻: None,
                        終了時刻: None,
                        保存済み記録hash: None,
                        保存済み結果証跡: false,
                        作成監査ID,
                        開始監査ID: None,
                        終了監査ID: None,
                        状態: "承認待ち",
                        表示範囲: "none".into(),
                        評価隔離: false,
                        取消: Arc::new(AtomicBool::new(false)),
                        受信: None,
                        生受信: Vec::new(),
                        結果: None,
                        実行期限: None,
                        単調開始: None,
                        単調応答Millis: None,
                    },
                );
                Ok(body)
            }
            "対話承認待ち" => {
                空入力(値)?;
                Ok(
                    json!({"要求": self.作業.values().filter(|v| v.状態 == "承認待ち").map(|v| {
                        if v.評価隔離 {
                            // C5のprivate Case inputは、owner承認待ち一覧にも再投影しない。
                            // 承認に必要なrequest ID/hash、Runtime、期限だけを返す。
                            json!({
                                "要求": {
                                    "要求ID": v.要求.要求ID,
                                    "実行系ID": v.要求.実行系ID,
                                },
                                "要求hash": v.要求hash,
                                "期限": v.作成時刻 + 300,
                                "評価隔離": true,
                            })
                        } else {
                            json!({
                                "要求": v.要求,
                                "要求hash": v.要求hash,
                                "期限": v.作成時刻 + 300,
                                "接続先": self.実行系[&v.要求.実行系ID].接続対象(),
                            })
                        }
                    }).collect::<Vec<_>>()}),
                )
            }
            "対話承認" => {
                let 指定: 承認指定 = 読取(値)?;
                if !["none", "hash_only", "summary", "redacted", "full"]
                    .contains(&指定.表示範囲.as_str())
                {
                    return Err(対話失敗::要求不正);
                }
                if self.作業.values().filter(|v| v.受信.is_some()).count() >= 8 {
                    return Err(対話失敗::要求不正);
                }
                let work = self.作業.get_mut(&指定.要求ID).ok_or(対話失敗::要求不正)?;
                if work.状態 != "承認待ち"
                    || work.要求hash != 指定.要求hash
                    || 現在 >= work.作成時刻 + 300
                {
                    return Err(対話失敗::権限拒否);
                }
                let adapter = Arc::clone(
                    self.実行系
                        .get(&work.要求.実行系ID)
                        .ok_or(対話失敗::実行系不在)?,
                );
                let 理由 = if work.評価隔離 {
                    format!(
                        "対話送信承認 Capability=対話送信 Permission={} Approval={} 表示範囲={} 評価隔離=true RecoveryAction=接続再確認",
                        work.要求.実行系ID, work.要求hash, 指定.表示範囲,
                    )
                } else {
                    format!(
                        "対話送信承認 Capability=対話送信 Permission={}:{} Approval={} 表示範囲={} RecoveryAction=接続再確認",
                        work.要求.実行系ID,
                        adapter.接続対象(),
                        work.要求hash,
                        指定.表示範囲,
                    )
                };
                work.開始監査ID = Some(監査(&理由, &work.要求.要求ID, &work.要求hash)?);
                work.開始時刻 = Some(現在);
                work.表示範囲 = 指定.表示範囲;
                let 要求 = work.要求.clone();
                let 取消 = Arc::clone(&work.取消);
                let 単調開始 = Instant::now();
                let 期限 =
                    単調開始 + Duration::from_secs(10.min((work.作成時刻 + 300 - 現在) as u64));
                work.単調開始 = Some(単調開始);
                work.単調応答Millis = None;
                work.実行期限 = Some(期限);
                let (送信, 受信) = mpsc::sync_channel(1);
                work.状態 = "実行中";
                if std::thread::Builder::new()
                    .name("実行系対話".into())
                    .spawn(move || {
                        let mut 生受信 = Vec::new();
                        let 結果 = if 取消.load(Ordering::SeqCst) {
                            Err(対話失敗::取消)
                        } else {
                            adapter.応答(&要求, &取消, 期限, &mut 生受信)
                        };
                        let _ = 送信.send(受信結果 {
                            結果,
                            生受信,
                            応答完了: Some(Instant::now()),
                        });
                    })
                    .is_err()
                {
                    work.状態 = "完了";
                    work.結果 = Some(Err(対話失敗::通信失敗));
                    self.agent_task_permissions
                        .remove(&work.要求.対話セッションID);
                    if let Some(s) = self.セッション.get_mut(&work.要求.対話セッションID)
                    {
                        s.状態 = "中止後隔離".into();
                    }
                    work.終了監査ID = Some(
                        監査("対話worker起動失敗", &work.要求.要求ID, &work.要求hash).map_err(
                            |e| {
                                work.状態 = "監査失敗";
                                e
                            },
                        )?,
                    );
                    work.終了時刻 = Some(現在);
                    return Err(対話失敗::通信失敗);
                }
                work.受信 = Some(受信);
                work.状態 = "実行中";
                Ok(json!({"要求ID": 指定.要求ID, "状態": "実行中"}))
            }
            "対話取得" => {
                let 指定: 要求指定 = 読取(値)?;
                let work = self.作業.get(&指定.要求ID).ok_or(対話失敗::要求不正)?;
                if work.状態 == "監査失敗" {
                    return Err(対話失敗::監査失敗);
                }
                if work.評価隔離 {
                    return Err(対話失敗::権限拒否);
                }
                Ok(
                    json!({"要求ID": 指定.要求ID, "状態": work.状態, "結果": work.結果.as_ref().map(|r| 表示射影(work, r)), "実行記録": 実行記録(work)}),
                )
            }
            "対話中止" => {
                let 指定: 要求指定 = 読取(値)?;
                let work = self.作業.get_mut(&指定.要求ID).ok_or(対話失敗::要求不正)?;
                if !matches!(work.状態, "承認待ち" | "実行中") {
                    return Err(対話失敗::要求不正);
                }
                let 終了監査ID = 監査(
                    "対話中止 実行系の停止は保証しない",
                    &指定.要求ID,
                    &work.要求hash,
                )?;
                work.取消.store(true, Ordering::SeqCst);
                work.状態 = "中止";
                work.終了時刻 = Some(現在);
                work.終了監査ID = Some(終了監査ID);
                work.結果 = Some(Err(対話失敗::取消));
                work.単調応答Millis = None;
                self.セッション
                    .get_mut(&work.要求.対話セッションID)
                    .ok_or(対話失敗::セッション不一致)?
                    .状態 = "中止後隔離".into();
                self.agent_task_permissions
                    .remove(&work.要求.対話セッションID);
                Ok(json!({"要求ID": 指定.要求ID, "状態": "中止"}))
            }
            "対話終了" => {
                let 指定: 終了指定 = 読取(値)?;
                if !self.セッション.contains_key(&指定.対話セッションID) {
                    return Err(対話失敗::セッション不一致);
                }
                if self
                    .作業
                    .values()
                    .any(|v| v.要求.対話セッションID == 指定.対話セッションID && v.受信.is_some())
                    || self.agent_tasks.values().any(|task| {
                        task.session_id == 指定.対話セッションID
                            && matches!(task.status, "pending" | "running")
                    })
                {
                    return Err(対話失敗::セッション不一致);
                }
                監査(
                    "対話終了",
                    &指定.対話セッションID,
                    &sha256_tagged(指定.対話セッションID.as_bytes()),
                )?;
                self.作業
                    .retain(|_, v| v.要求.対話セッションID != 指定.対話セッションID);
                self.agent_task_permissions.remove(&指定.対話セッションID);
                self.セッション.remove(&指定.対話セッションID);
                Ok(json!({"対話セッションID": 指定.対話セッションID, "状態": "終了"}))
            }
            _ => Err(対話失敗::要求不正),
        };
        self.記録保存(監査)?;
        result
    }

    fn 記録保存(&mut self, 監査: &mut 監査器<'_>) -> Result<(), 対話失敗> {
        for work in self.作業.values_mut() {
            if work.状態 == "監査失敗" {
                continue;
            }
            if !work.保存済み結果証跡 {
                if let Some(proof) = 結果証跡(work) {
                    let encoded = proof.to_string();
                    if 監査(
                        &format!("対話結果証跡:{encoded}"),
                        &work.要求.要求ID,
                        &sha256_tagged(encoded.as_bytes()),
                    )
                    .is_err()
                    {
                        work.取消.store(true, Ordering::SeqCst);
                        work.状態 = "監査失敗";
                        work.結果 = Some(Err(対話失敗::監査失敗));
                        self.agent_task_permissions
                            .remove(&work.要求.対話セッションID);
                        if let Some(session) = self.セッション.get_mut(&work.要求.対話セッションID)
                        {
                            session.状態 = "中止後隔離".into();
                        }
                        return Err(対話失敗::監査失敗);
                    }
                    work.保存済み結果証跡 = true;
                }
            }
            let (状態, 失敗分類) = match &work.結果 {
                Some(Ok(v)) => (if v.保留 { "保留" } else { "成功" }, None),
                Some(Err(e)) => (
                    if *e == 対話失敗::取消 {
                        "中止"
                    } else {
                        "失敗"
                    },
                    Some(e.分類()),
                ),
                None => (work.状態, None),
            };
            let body = json!({"版":2, "状態":状態, "失敗分類":失敗分類,
                "実行記録":実行記録(work), "入力概要":入力概要(work)})
            .to_string();
            let hash = sha256_tagged(body.as_bytes());
            if work.保存済み記録hash.as_ref() == Some(&hash) {
                continue;
            }
            if 監査(&format!("対話実行記録:{body}"), &work.要求.要求ID, &hash).is_err()
            {
                work.取消.store(true, Ordering::SeqCst);
                work.状態 = "監査失敗";
                work.結果 = Some(Err(対話失敗::監査失敗));
                self.agent_task_permissions
                    .remove(&work.要求.対話セッションID);
                if let Some(session) = self.セッション.get_mut(&work.要求.対話セッションID)
                {
                    session.状態 = "中止後隔離".into();
                }
                return Err(対話失敗::監査失敗);
            }
            work.保存済み記録hash = Some(hash);
        }
        Ok(())
    }

    fn 進捗反映(
        &mut self, 現在: i64, 監査: &mut 監査器<'_>
    ) -> Result<(), 対話失敗> {
        for work in self.作業.values_mut() {
            let 完了 = if let Some(受信) = &work.受信 {
                match 受信.try_recv() {
                    Ok(r) => Some(r),
                    Err(mpsc::TryRecvError::Disconnected) => Some(受信結果 {
                        結果: Err(対話失敗::通信失敗),
                        生受信: Vec::new(),
                        応答完了: None,
                    }),
                    Err(mpsc::TryRecvError::Empty) => None,
                }
            } else if work.状態 == "承認待ち" && 現在 >= work.作成時刻 + 300 {
                Some(受信結果 {
                    結果: Err(対話失敗::期限超過),
                    生受信: Vec::new(),
                    応答完了: None,
                })
            } else {
                None
            };
            let 期限内応答 = 完了
                .as_ref()
                .and_then(|結果| 結果.応答完了)
                .is_some_and(|応答完了| work.実行期限.map_or(true, |期限| 応答完了 <= 期限));
            let 期限超過 = work.状態 == "実行中"
                && work.実行期限.is_some_and(|期限| {
                    match 完了.as_ref().and_then(|結果| 結果.応答完了) {
                        Some(応答完了) => 応答完了 > 期限,
                        None => Instant::now() >= 期限,
                    }
                });
            if 期限超過 && !期限内応答 {
                work.取消.store(true, Ordering::SeqCst);
                work.結果 = Some(Err(対話失敗::期限超過));
                work.状態 = "完了";
                work.単調応答Millis = None;
                self.agent_task_permissions
                    .remove(&work.要求.対話セッションID);
                if let Some(s) = self.セッション.get_mut(&work.要求.対話セッションID)
                {
                    s.状態 = "中止後隔離".into();
                }
                let 終了監査 = 監査("対話期限超過", &work.要求.要求ID, &work.要求hash);
                if 終了監査.is_err() {
                    work.状態 = "監査失敗";
                    work.結果 = Some(Err(対話失敗::監査失敗));
                    return Err(対話失敗::監査失敗);
                }
                work.終了時刻 = Some(現在);
                work.終了監査ID = 終了監査.ok();
            }
            if let Some(mut 受信結果) = 完了 {
                work.受信 = None;
                work.単調応答Millis = if work.結果.is_none() && 期限内応答 {
                    match (work.単調開始, 受信結果.応答完了) {
                        (Some(開始), Some(応答完了)) => Some(
                            応答完了
                                .saturating_duration_since(開始)
                                .as_millis()
                                .min(u128::from(u64::MAX)) as u64,
                        ),
                        _ => None,
                    }
                } else {
                    None
                };
                if 受信結果.生受信.len() > 4
                    || 受信結果.生受信.iter().any(|v| v.len() > 1024 * 1024)
                {
                    受信結果.結果 = Err(対話失敗::応答不正);
                    受信結果.生受信.truncate(4);
                    for v in &mut 受信結果.生受信 {
                        v.truncate(1024 * 1024);
                    }
                }
                work.生受信 = 受信結果.生受信;
                let 結果 = 受信結果.結果.and_then(|v| 結果検査(&work.要求, v));
                let hashes: Vec<_> = work.生受信.iter().map(|v| sha256_tagged(v)).collect();
                let hash = sha256_tagged(
                    json!({"要求hash": work.要求hash, "受信hash": hashes})
                        .to_string()
                        .as_bytes(),
                );
                let 種別 = if work.結果.is_some() {
                    "採用終了後応答破棄"
                } else {
                    "対話完了"
                };
                let 終了監査 = 監査(種別, &work.要求.要求ID, &hash);
                if 終了監査.is_err() {
                    work.取消.store(true, Ordering::SeqCst);
                    work.状態 = "監査失敗";
                    work.結果 = Some(Err(対話失敗::監査失敗));
                    self.agent_task_permissions
                        .remove(&work.要求.対話セッションID);
                    if let Some(s) = self.セッション.get_mut(&work.要求.対話セッションID)
                    {
                        s.状態 = "中止後隔離".into();
                    }
                    return Err(対話失敗::監査失敗);
                }
                if work.結果.is_none() {
                    work.終了時刻 = Some(現在);
                    work.終了監査ID = 終了監査.ok();
                    if 結果.is_err() {
                        self.agent_task_permissions
                            .remove(&work.要求.対話セッションID);
                        if let Some(s) = self.セッション.get_mut(&work.要求.対話セッションID)
                        {
                            s.状態 = "中止後隔離".into();
                        }
                    }
                    work.状態 = "完了";
                    work.結果 = Some(結果);
                }
            }
        }
        self.記録保存(監査)?;
        self.失効資源解放();
        Ok(())
    }

    fn AgentTask進捗反映(
        &mut self,
        現在: i64,
        監査: &mut 監査器<'_>,
    ) -> Result<(), 対話失敗> {
        let task_ids = self.agent_tasks.keys().cloned().collect::<Vec<_>>();
        for task_id in task_ids {
            let completed = {
                let Some(task) = self.agent_tasks.get_mut(&task_id) else {
                    continue;
                };
                if task.status != "running" {
                    continue;
                }
                let deadline = task.deadline;
                match task.receiver.as_ref().map(mpsc::Receiver::try_recv) {
                    Some(Ok(result)) => Some(AgentTask結果完了時刻検査(
                        result.result,
                        result.completed_at,
                        task.owner_cancel_requested_at,
                        deadline,
                    )),
                    Some(Err(mpsc::TryRecvError::Disconnected)) => Some(Err(対話失敗::通信失敗)),
                    Some(Err(mpsc::TryRecvError::Empty)) | None => {
                        if Instant::now() >= deadline {
                            task.cancel.store(true, Ordering::SeqCst);
                        }
                        None
                    }
                }
            };
            let Some(result) = completed else {
                continue;
            };
            let (status, result_hash, failure) = match result {
                Ok(hash) if hash.starts_with("sha256:") && hash.len() == 71 => {
                    ("completed", Some(hash), None)
                }
                Ok(_) => ("failed", None, Some(対話失敗::応答不正)),
                Err(error) if error == 対話失敗::取消 => ("cancelled", None, Some(error)),
                Err(error) => ("failed", None, Some(error)),
            };
            let event_hash = sha256_tagged(
                json!({
                    "task_id": task_id,
                    "status": status,
                    "instruction_hash": self.agent_tasks[&task_id].instruction_hash,
                    "result_hash": result_hash,
                    "failure_class": failure.map(対話失敗::分類),
                    "completed_at": 現在,
                })
                .to_string()
                .as_bytes(),
            );
            let event_id = match 監査(
                if status == "completed" {
                    "Agent Task完了（結果本文非保存・hashのみ）"
                } else {
                    "Agent Task失敗・取消（RecoveryAction=Workspace差分を確認）"
                },
                &task_id,
                &event_hash,
            ) {
                Ok(event_id) => event_id,
                Err(error) => {
                    let Some(task) = self.agent_tasks.get_mut(&task_id) else {
                        return Err(対話失敗::要求不正);
                    };
                    task.cancel.store(true, Ordering::SeqCst);
                    task.receiver = None;
                    task.status = "quarantined";
                    return Err(error);
                }
            };
            let Some(task) = self.agent_tasks.get_mut(&task_id) else {
                return Err(対話失敗::要求不正);
            };
            task.receiver = None;
            task.status = status;
            task.result_hash = result_hash;
            task.audit_event_id = event_id;
        }
        Ok(())
    }
}

fn AgentTask状態射影(task: &AgentTask作業) -> Value {
    let mut projection = json!({
        "task_id": task.task_id,
        "record_version": 2,
        "agent_runtime_id": task.runtime_id,
        "session_id": task.session_id,
        "workspace_id": task.workspace_id,
        "description": "Agent作業Task（内容は別のWorkspace差分経路で確認）",
        "instruction_hash": task.instruction_hash,
        "status": task.status,
        "audit_event_id": task.audit_event_id,
    });
    if let Some(result_hash) = &task.result_hash {
        projection["result_hash"] = json!(result_hash);
    }
    projection
}

fn 実行記録(work: &作業) -> Value {
    json!({"要求ID": work.要求.要求ID, "実行系ID": work.要求.実行系ID,
        "対話セッションID": work.要求.対話セッションID, "作成時刻": work.作成時刻,
        "開始時刻": work.開始時刻, "終了時刻": work.終了時刻,
        "作成監査ID": work.作成監査ID, "開始監査ID": work.開始監査ID,
        "終了監査ID": work.終了監査ID})
}

/// 永続履歴のmetadata用。入力本文を復元・公開する権限や正しさの証拠にはしない。
fn 入力概要(work: &作業) -> Value {
    json!({"表示範囲":"hash_only", "入力hash":sha256_tagged(work.要求.入力.as_bytes())})
}

fn 結果検査(要求: &対話要求, v: 実行結果) -> Result<実行結果, 対話失敗> {
    if v.対話セッションID != 要求.対話セッションID {
        return Err(対話失敗::セッション不一致);
    }
    let hex = |s: &str, n: usize| {
        s.len() == n
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    if v.本文.chars().count() > 65536
        || v.参照.len() > 64
        || v.参照.iter().any(|s| s.chars().count() > 2048)
        || v.能力.len() > 64
        || v.能力.iter().any(|s| s.chars().count() > 256)
        || v.経路.chars().count() > 256
        || !hex(&v.追跡ID, 32)
        || !v
            .追跡hash
            .strip_prefix("sha256:")
            .is_some_and(|s| hex(s, 64))
        || v.生応答.is_empty()
        || v.生応答.len() > 1024 * 1024
    {
        return Err(対話失敗::応答不正);
    }
    Ok(v)
}

fn 結果証跡(work: &作業) -> Option<Value> {
    let result = work.結果.as_ref()?.as_ref().ok()?;
    let ended = work.終了監査ID.as_ref()?;
    if work.表示範囲 == "none" {
        return None;
    }
    let full = work.表示範囲 == "full";
    let hash = |v: Value| sha256_tagged(v.to_string().as_bytes());
    Some(
        json!({"版":1,"要求ID":work.要求.要求ID,"対話セッションID":work.要求.対話セッションID,
        "実行系ID":work.要求.実行系ID,"要求hash":work.要求hash,"終了監査ID":ended,
        "表示範囲":work.表示範囲,"応答hash":sha256_tagged(&result.生応答),
        "能力申告hash":full.then(|| hash(json!(result.能力))),
        "経路申告hash":full.then(|| hash(json!(result.経路))),
        "追跡参照hash":full.then(|| hash(json!({"追跡ID":result.追跡ID,"追跡hash":result.追跡hash}))),
        "証拠種別":"INTERNAL_STATE"}),
    )
}

fn 表示射影(work: &作業, 結果: &Result<実行結果, 対話失敗>) -> Value {
    let mut body = json!({"要求ID": work.要求.要求ID, "実行系ID": work.要求.実行系ID, "対話セッションID": work.要求.対話セッションID,
        "状態": "成功", "表示範囲": work.表示範囲, "本文": "", "参照": [], "能力": [], "経路": "", "追跡ID": "", "追跡hash": "", "応答hash": "", "失敗分類": "", "復旧": ""});
    match 結果 {
        Ok(v) => {
            if v.保留 {
                body["状態"] = json!("保留");
            }
            if work.表示範囲 != "none" {
                body["応答hash"] = json!(sha256_tagged(&v.生応答));
            }
            if work.表示範囲 == "full" {
                body["本文"] = json!(v.本文);
                body["参照"] = json!(v.参照);
                body["能力"] = json!(v.能力);
                body["経路"] = json!(v.経路);
                body["追跡ID"] = json!(v.追跡ID);
                body["追跡hash"] = json!(v.追跡hash);
            }
        }
        Err(e) => {
            body["状態"] = json!(if *e == 対話失敗::取消 {
                "中止"
            } else {
                "失敗"
            });
            body["失敗分類"] = json!(e.分類());
            body["復旧"] = json!(e.復旧());
        }
    }
    body
}

#[cfg(test)]
#[path = "../../tests/unit/agent_task_fixture_fs.rs"]
mod agent_task_fixture_fs;

#[cfg(all(test, windows))]
#[path = "../../tests/support/broker_codex_fixture.rs"]
mod broker_codex_fixture_support;

#[cfg(all(test, windows))]
#[path = "../../tests/support/codex_loopback_responses.rs"]
mod broker_codex_loopback_support;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[cfg(windows)]
    const CANCEL_FIXTURE_OWNER_APPROVAL_OPERATION: &str = "AgentTaskOwnerApprovalGrant";

    #[test]
    fn AgentAdapter投影は既存SchemaとAuthority境界を検証する() {
        let valid: Value = serde_json::from_str(include_str!(
            "../../../../examples/contracts/agent_adapter.valid.json"
        ))
        .expect("正常Agent Adapter fixture");
        let parsed = AgentAdapterMetadata::read(&valid).expect("正常Agent Adapter metadata");
        assert!(!parsed.task_execution_supported(), "宣言の欠落は未対応扱い");

        let mut task_unknown = valid.clone();
        task_unknown["capabilities"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "capability_id": "task_execution",
                "support": {"status": "unknown", "reason": "実装未確認"}
            }));
        assert!(!AgentAdapterMetadata::read(&task_unknown)
            .unwrap()
            .task_execution_supported());

        let mut task_supported = valid.clone();
        task_supported["capabilities"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "capability_id": "task_execution",
                "support": {"status": "supported", "reason": "試験用宣言"}
            }));
        assert!(AgentAdapterMetadata::read(&task_supported)
            .unwrap()
            .task_execution_supported());

        let mut authority = valid.clone();
        authority["permission"] = json!("all");
        assert_eq!(
            AgentAdapterMetadata::read(&authority).err(),
            Some(対話失敗::応答不正)
        );

        let mut credential = valid.clone();
        credential["api_key"] = json!("TEST_ONLY_SECRET_SENTINEL");
        assert_eq!(
            AgentAdapterMetadata::read(&credential).err(),
            Some(対話失敗::応答不正)
        );

        let mut nested_credential = valid.clone();
        nested_credential["authentication"]["api_key"] = json!("TEST_ONLY_SECRET_SENTINEL");
        assert_eq!(
            AgentAdapterMetadata::read(&nested_credential).err(),
            Some(対話失敗::応答不正)
        );

        let mut embedded_credential = valid.clone();
        embedded_credential["tool_support"]["reason"] = json!("Bearer TEST_ONLY_SECRET_SENTINEL");
        assert_eq!(
            AgentAdapterMetadata::read(&embedded_credential).err(),
            Some(対話失敗::応答不正)
        );

        let mut secret_flag = valid.clone();
        secret_flag["authentication"]["secret_value_present"] = json!(true);
        assert_eq!(
            AgentAdapterMetadata::read(&secret_flag).err(),
            Some(対話失敗::応答不正)
        );

        let mut malformed_optional = valid;
        malformed_optional["evidence_source"] = Value::Null;
        assert_eq!(
            AgentAdapterMetadata::read(&malformed_optional).err(),
            Some(対話失敗::応答不正)
        );
    }

    #[test]
    fn 対話開始監査hashは既存session射影と一致する() {
        let session = セッション {
            対話セッションID: "session-fixture".into(),
            実行系ID: "runtime-fixture".into(),
            状態: "利用中".into(),
            作成監査ID: "audit-fixture".into(),
            作業領域ID: None,
            作業領域結合監査ID: None,
            作業領域登録hash: None,
        };
        let legacy_body = json!({
            "対話セッションID": &session.対話セッションID,
            "実行系ID": &session.実行系ID,
            "状態": "利用中",
        });
        assert_eq!(
            対話開始監査hash(&session.対話セッションID, &session.実行系ID),
            sha256_tagged(legacy_body.to_string().as_bytes())
        );
    }

    struct 試験Adapter {
        回数: Arc<AtomicUsize>,
        失敗: bool,
        別session: bool,
        遅延: bool,
        Agentmetadata有効: bool,
    }
    impl 実行系Adapter for 試験Adapter {
        fn 接続対象(&self) -> String {
            "試験専用".into()
        }
        fn 作業領域実体識別子(&self) -> Option<AgentTaskWorkspaceIdentity> {
            Some(AgentTaskWorkspaceIdentity::new(1, 1))
        }
        fn agent_metadata(&self) -> Option<Value> {
            if !self.Agentmetadata有効 {
                return None;
            }
            let mut metadata: Value = serde_json::from_str(include_str!(
                "../../../../examples/contracts/agent_adapter.valid.json"
            ))
            .expect("Agent接続例を読み込む");
            metadata["adapter_id"] = json!("dialogue-test-adapter");
            metadata["agent_id"] = json!("dialogue-test-agent");
            metadata["capabilities"].as_array_mut().unwrap().push(json!({
                "capability_id":"task_execution",
                "support":{"status":"supported","reason":"Broker権限経路の試験専用宣言。実Taskの実行証拠ではない"}
            }));
            Some(metadata)
        }
        fn AgentTask実行対応(&self) -> bool {
            true
        }
        fn AgentTask実行(
            &self,
            _: &str,
            cancel: &AtomicBool,
            deadline: Instant,
            _: Option<crate::broker::agent_task_scratch::AgentTaskScratchContext>,
        ) -> Result<String, 対話失敗> {
            self.回数.fetch_add(1, Ordering::SeqCst);
            if self.遅延 {
                let end = Instant::now() + Duration::from_secs(1);
                while Instant::now() < end {
                    if cancel.load(Ordering::SeqCst) {
                        return Err(対話失敗::取消);
                    }
                    if Instant::now() >= deadline {
                        return Err(対話失敗::期限超過);
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
            if cancel.load(Ordering::SeqCst) {
                return Err(対話失敗::取消);
            }
            if self.失敗 {
                return Err(対話失敗::通信失敗);
            }
            Ok("TASK_PRIVATE_OUTPUT_SENTINEL".into())
        }
        fn 応答(
            &self,
            r: &対話要求,
            _: &AtomicBool,
            _: Instant,
            raw: &mut Vec<Vec<u8>>,
        ) -> Result<実行結果, 対話失敗> {
            self.回数.fetch_add(1, Ordering::SeqCst);
            if self.遅延 {
                std::thread::sleep(Duration::from_millis(100));
            }
            raw.push(b"raw-private".to_vec());
            if self.失敗 {
                return Err(対話失敗::通信失敗);
            }
            Ok(実行結果 {
                対話セッションID: if self.別session {
                    "別session".into()
                } else {
                    r.対話セッションID.clone()
                },
                本文: r.入力.clone(),
                参照: vec!["秘密参照".into()],
                能力: vec!["fixture".into()],
                経路: "fixture".into(),
                追跡ID: "a".repeat(32),
                追跡hash: format!("sha256:{}", "b".repeat(64)),
                保留: false,
                生応答: b"raw-private".to_vec(),
            })
        }
    }

    struct AgentTask隔離FixtureAdapter {
        runtime_id: String,
        workspace_id: String,
        adapter_identity: AgentTaskWorkspaceIdentity,
        root_identity: super::super::workspace_root::DirectoryIdentity,
        recovery_binding_hash: String,
        marker_path: std::path::PathBuf,
        marker_content: &'static str,
        calls: Arc<AtomicUsize>,
    }

    impl 実行系Adapter for AgentTask隔離FixtureAdapter {
        fn 接続対象(&self) -> String {
            self.runtime_id.clone()
        }

        fn 作業領域実体識別子(&self) -> Option<AgentTaskWorkspaceIdentity> {
            Some(self.adapter_identity)
        }

        fn agent_metadata(&self) -> Option<Value> {
            let mut metadata: Value = serde_json::from_str(include_str!(
                "../../../../examples/contracts/agent_adapter.valid.json"
            ))
            .expect("Agent接続例を読み込む");
            metadata["adapter_id"] = json!(format!("fixture-{}", self.runtime_id));
            metadata["agent_id"] = json!(format!("agent-{}", self.runtime_id));
            metadata["capabilities"].as_array_mut()?.push(json!({
                "capability_id": "task_execution",
                "support": {"status": "supported", "reason": "分離routing fixture専用。実Agent実行の証拠ではない"}
            }));
            Some(metadata)
        }

        fn AgentTask実行対応(&self) -> bool {
            true
        }

        fn AgentTask実行(
            &self,
            _: &str,
            cancel: &AtomicBool,
            deadline: Instant,
            context: Option<crate::broker::agent_task_scratch::AgentTaskScratchContext>,
        ) -> Result<String, 対話失敗> {
            if cancel.load(Ordering::SeqCst) {
                return Err(対話失敗::取消);
            }
            if Instant::now() >= deadline {
                return Err(対話失敗::期限超過);
            }
            let context = context.ok_or(対話失敗::作業領域不在)?;
            if context.runtime_id != self.runtime_id
                || context.workspace_id != self.workspace_id
                || context.root_identity != self.root_identity
                || context.recovery_binding_hash != self.recovery_binding_hash
            {
                return Err(対話失敗::作業領域不在);
            }
            super::agent_task_fixture_fs::write_marker(&self.marker_path, self.marker_content)
                .map_err(|_| 対話失敗::通信失敗)?;
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.marker_content.to_owned())
        }

        fn 応答(
            &self,
            request: &対話要求,
            _: &AtomicBool,
            _: Instant,
            raw: &mut Vec<Vec<u8>>,
        ) -> Result<実行結果, 対話失敗> {
            raw.push(Vec::new());
            Ok(実行結果 {
                対話セッションID: request.対話セッションID.clone(),
                本文: String::new(),
                参照: Vec::new(),
                能力: Vec::new(),
                経路: "fixture".into(),
                追跡ID: "a".repeat(32),
                追跡hash: format!("sha256:{}", "b".repeat(64)),
                保留: false,
                生応答: Vec::new(),
            })
        }
    }

    #[cfg(windows)]
    struct Broker統合CodexFixtureAdapter {
        inner: crate::adapters::codex_cli::CodexCliAdapter,
    }

    #[cfg(windows)]
    impl 実行系Adapter for Broker統合CodexFixtureAdapter {
        fn 接続対象(&self) -> String {
            self.inner.接続対象()
        }

        fn 作業領域実体識別子(&self) -> Option<AgentTaskWorkspaceIdentity> {
            self.inner.作業領域実体識別子()
        }

        fn agent_metadata(&self) -> Option<Value> {
            let mut metadata = self.inner.agent_metadata()?;
            let capability = metadata["capabilities"]
                .as_array_mut()?
                .iter_mut()
                .find(|item| item["capability_id"] == "task_execution")?;
            capability["support"]["status"] = json!("supported");
            capability["support"]["reason"] =
                json!("Brokerとfake Codex CLIの縦断試験専用。製品能力の証拠ではない");
            metadata["evidence_source"] = json!("FIXTURE");
            metadata["evidence_reason"] = json!("試験専用Adapterが能力宣言を上書きした合成経路");
            Some(metadata)
        }

        fn AgentTask実行対応(&self) -> bool {
            self.inner.AgentTask実行対応()
        }

        fn AgentTask実行(
            &self,
            instruction: &str,
            cancel: &AtomicBool,
            deadline: Instant,
            context: Option<crate::broker::agent_task_scratch::AgentTaskScratchContext>,
        ) -> Result<String, 対話失敗> {
            self.inner
                .AgentTask実行(instruction, cancel, deadline, context)
        }

        fn 応答(
            &self,
            request: &対話要求,
            cancel: &AtomicBool,
            deadline: Instant,
            raw: &mut Vec<Vec<u8>>,
        ) -> Result<実行結果, 対話失敗> {
            self.inner.応答(request, cancel, deadline, raw)
        }
    }

    fn 準備(失敗: bool, 別session: bool, 遅延: bool) -> (対話制御, Arc<AtomicUsize>) {
        let mut c = 対話制御::default();
        let n = Arc::new(AtomicUsize::new(0));
        c.登録(
            "left",
            Arc::new(試験Adapter {
                回数: n.clone(),
                失敗,
                別session,
                遅延,
                Agentmetadata有効: true,
            }),
        )
        .unwrap();
        (c, n)
    }
    fn 操作(
        c: &mut 対話制御, op: &str, v: Value, owner: bool
    ) -> Result<Value, 対話失敗> {
        let mut payload = v;
        let binding = if op == "対話開始" {
            let runtime = payload
                .get("実行系ID")
                .and_then(Value::as_str)
                .map(str::to_owned);
            runtime.and_then(|runtime| {
                let adapter = c.実行系.get(&runtime)?;
                adapter.agent_metadata()?;
                let workspace_id = format!("fixture-workspace-{runtime}");
                payload["作業領域ID"] = json!(workspace_id);
                Some(super::super::workspace::DialogueWorkspaceBinding::for_test(
                    &runtime,
                    &workspace_id,
                ))
            })
        } else {
            None
        };
        c.操作_作業領域結合済み(
            op,
            &payload,
            owner,
            100,
            binding.as_ref(),
            &mut |_, _, _| Ok("fixture-audit".into()),
        )
    }
    fn AgentTask操作(
        c: &mut 対話制御,
        op: &str,
        payload: Value,
        owner: bool,
    ) -> Result<Value, 対話失敗> {
        let binding = payload
            .get("agent_runtime_id")
            .and_then(Value::as_str)
            .zip(payload.get("workspace_id").and_then(Value::as_str))
            .map(|(runtime, workspace)| {
                super::super::workspace::DialogueWorkspaceBinding::for_test(runtime, workspace)
            });
        c.操作_作業領域結合済み(
            op,
            &payload,
            owner,
            100,
            binding.as_ref(),
            &mut |_, _, _| Ok("fixture-task-audit".into()),
        )
    }

    fn AgentTask操作結合済み(
        c: &mut 対話制御,
        op: &str,
        payload: Value,
        owner: bool,
        binding: Option<&super::super::workspace::DialogueWorkspaceBinding>,
    ) -> Result<Value, 対話失敗> {
        c.操作_作業領域結合済み(op, &payload, owner, 100, binding, &mut |_, _, _| {
            Ok("fixture-bound-task-audit".into())
        })
    }

    fn AgentTask要求(session_id: &str, instruction: &str) -> Value {
        json!({
            "agent_runtime_id": "left",
            "session_id": session_id,
            "workspace_id": "fixture-workspace-left",
            "instruction": instruction,
        })
    }

    fn AgentTask権限とApprovalを発行(
        c: &mut 対話制御, session_id: &str, instruction: &str
    ) {
        let request = AgentTask要求(session_id, instruction);
        AgentTask操作(
            c,
            "AgentTaskWorkspacePermissionGrant",
            json!({
                "agent_runtime_id": request["agent_runtime_id"],
                "session_id": request["session_id"],
                "workspace_id": request["workspace_id"],
            }),
            true,
        )
        .unwrap();
        AgentTask操作(c, "AgentTaskOwnerApprovalGrant", request, true).unwrap();
    }
    fn 開始(c: &mut 対話制御, id: &str) -> String {
        操作(c, "対話開始", json!({"実行系ID":id}), false).unwrap()["対話セッションID"]
            .as_str()
            .unwrap()
            .into()
    }

    #[test]
    fn AgentTask取消後または期限後に届いた成功応答を採用しない() {
        let cancelled = AtomicBool::new(true);
        assert_eq!(
            AgentTask結果hash化(
                Ok("遅延応答試験用秘密本文".into()),
                &cancelled,
                Instant::now() + Duration::from_secs(60),
            ),
            Err(対話失敗::取消)
        );

        let not_cancelled = AtomicBool::new(false);
        assert_eq!(
            AgentTask結果hash化(
                Ok("遅延応答試験用秘密本文".into()),
                &not_cancelled,
                Instant::now() - Duration::from_secs(1),
            ),
            Err(対話失敗::期限超過)
        );

        assert_eq!(
            AgentTask結果hash化(
                Ok("同時期限・取消試験用本文".into()),
                &cancelled,
                Instant::now() - Duration::from_secs(1),
            ),
            Err(対話失敗::期限超過),
            "期限到達時にBrokerが取消flagを立てても期限超過を保持する"
        );
    }

    #[test]
    fn AgentTask完了結果は期限前の取消と期限後の停止を区別する() {
        let deadline = Instant::now();
        assert_eq!(
            AgentTask結果完了時刻検査(
                Err(対話失敗::取消),
                deadline - Duration::from_millis(1),
                None,
                deadline,
            ),
            Err(対話失敗::取消),
            "期限前に完了したOwner取消は取消のまま保持する"
        );
        assert_eq!(
            AgentTask結果完了時刻検査(Err(対話失敗::取消), deadline, None, deadline,),
            Err(対話失敗::期限超過),
            "deadlineで停止要求したworkerの遅延取消応答は期限超過へ分類する"
        );
        assert_eq!(
            AgentTask結果完了時刻検査(
                Ok("sha256:fixture".into()),
                deadline + Duration::from_millis(1),
                None,
                deadline,
            ),
            Err(対話失敗::期限超過),
            "期限後の成功hashを採用しない"
        );
        assert_eq!(
            AgentTask結果完了時刻検査(
                Err(対話失敗::通信失敗),
                deadline + Duration::from_millis(1),
                None,
                deadline,
            ),
            Err(対話失敗::通信失敗),
            "process終了を確認できない通信失敗を期限超過で隠さない"
        );

        let cancellation_requested_at = deadline - Duration::from_millis(20);
        assert_eq!(
            AgentTask結果完了時刻検査(
                Ok("sha256:fixture".into()),
                cancellation_requested_at + Duration::from_millis(1),
                Some(cancellation_requested_at),
                deadline,
            ),
            Err(対話失敗::取消),
            "Owner取消の受理後に競合して届いた成功hashを採用しない"
        );
        assert_eq!(
            AgentTask結果完了時刻検査(
                Ok("sha256:fixture".into()),
                cancellation_requested_at - Duration::from_millis(1),
                Some(cancellation_requested_at),
                deadline,
            ),
            Ok("sha256:fixture".into()),
            "Owner取消受理前に完了したworker結果は後続poll遅延で取消へ書き換えない"
        );
    }

    #[test]
    fn AgentTask実行はPermissionとApprovalを再検証して一回消費し出力本文を残さない() {
        let (mut c, calls) = 準備(false, false, false);
        let session_id = 開始(&mut c, "left");
        let instruction = "workspace内の安全な変更を行う";
        AgentTask権限とApprovalを発行(&mut c, &session_id, instruction);

        let altered = AgentTask要求(&session_id, "別の指示へ差し替える");
        assert_eq!(
            AgentTask操作(&mut c, "AgentTask実行", altered, false),
            Err(対話失敗::権限拒否)
        );
        assert!(c.agent_task_permissions.contains_key(&session_id));

        let started = AgentTask操作(
            &mut c,
            "AgentTask実行",
            AgentTask要求(&session_id, instruction),
            false,
        )
        .unwrap();
        assert_eq!(started["record_version"], 2);
        assert_eq!(started["status"], "running");
        assert!(!c.agent_task_permissions.contains_key(&session_id));
        assert_eq!(
            AgentTask操作(
                &mut c,
                "AgentTask実行",
                AgentTask要求(&session_id, instruction),
                false,
            ),
            Err(対話失敗::権限拒否)
        );

        let task_id = started["task_id"].as_str().unwrap().to_owned();
        let mut state = Value::Null;
        for _ in 0..50 {
            state =
                AgentTask操作(&mut c, "AgentTask状態", json!({"task_id": task_id}), false).unwrap();
            if state["status"] != "running" {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(state["status"], "completed");
        assert!(state["result_hash"]
            .as_str()
            .unwrap()
            .starts_with("sha256:"));
        assert!(!state.to_string().contains("TASK_PRIVATE_OUTPUT_SENTINEL"));
        assert!(!state.as_object().unwrap().contains_key("instruction"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn AgentTask実行は期限切れOwnerApprovalを有効Permissionから分離して拒否する() {
        for wall_clock_expired in [true, false] {
            let (mut c, calls) = 準備(false, false, false);
            let session_id = 開始(&mut c, "left");
            let instruction = "期限切れOwner Approvalでは実行しない";
            AgentTask権限とApprovalを発行(&mut c, &session_id, instruction);

            let (registration_hash, instruction_hash) = {
                let grant = c.agent_task_permissions.get_mut(&session_id).unwrap();
                assert!(grant.expires_at_epoch_seconds > 100);
                assert!(Instant::now() < grant.monotonic_expiry);
                let approval = grant.owner_approval.as_mut().unwrap();
                if wall_clock_expired {
                    approval.expires_at_epoch_seconds = 100;
                } else {
                    assert!(approval.expires_at_epoch_seconds > 100);
                    approval.monotonic_expiry = Instant::now() - Duration::from_secs(1);
                }
                (
                    grant.workspace_registration_hash.clone(),
                    sha256_tagged(instruction.as_bytes()),
                )
            };

            assert!(c.agent_task_permission_is_active(
                &session_id,
                "left",
                "fixture-workspace-left",
                &registration_hash,
                100,
            ));
            assert!(!c.agent_task_owner_approval_is_active(
                &session_id,
                "left",
                "fixture-workspace-left",
                &registration_hash,
                &instruction_hash,
                100,
            ));
            assert_eq!(
                AgentTask操作(
                    &mut c,
                    "AgentTask実行",
                    AgentTask要求(&session_id, instruction),
                    false,
                ),
                Err(対話失敗::権限拒否),
            );
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert!(c.agent_task_permissions.contains_key(&session_id));
        }
    }

    #[test]
    fn AgentTask実行は有効OwnerApprovalを期限切れPermissionから分離して拒否する() {
        for wall_clock_expired in [true, false] {
            let (mut c, calls) = 準備(false, false, false);
            let session_id = 開始(&mut c, "left");
            let instruction = "期限切れPermissionでは実行しない";
            AgentTask権限とApprovalを発行(&mut c, &session_id, instruction);

            let (registration_hash, instruction_hash) = {
                let grant = c.agent_task_permissions.get(&session_id).unwrap();
                (
                    grant.workspace_registration_hash.clone(),
                    sha256_tagged(instruction.as_bytes()),
                )
            };
            assert!(c.agent_task_owner_approval_is_active(
                &session_id,
                "left",
                "fixture-workspace-left",
                &registration_hash,
                &instruction_hash,
                100,
            ));

            {
                let grant = c.agent_task_permissions.get_mut(&session_id).unwrap();
                assert!(grant.expires_at_epoch_seconds > 100);
                assert!(Instant::now() < grant.monotonic_expiry);
                let approval = grant.owner_approval.as_ref().unwrap();
                assert!(approval.expires_at_epoch_seconds > 100);
                assert!(Instant::now() < approval.monotonic_expiry);
                assert_eq!(approval.instruction_hash, instruction_hash);
                assert_eq!(
                    approval.execution_conditions_hash,
                    AgentTask実行条件hash(
                        "left",
                        &session_id,
                        "fixture-workspace-left",
                        &grant.workspace_registration_hash,
                        &grant.permission_id,
                    ),
                );

                if wall_clock_expired {
                    grant.expires_at_epoch_seconds = 100;
                } else {
                    grant.monotonic_expiry = Instant::now() - Duration::from_secs(1);
                }
            }

            assert!(!c.agent_task_permission_is_active(
                &session_id,
                "left",
                "fixture-workspace-left",
                &registration_hash,
                100,
            ));
            assert!(!c.agent_task_owner_approval_is_active(
                &session_id,
                "left",
                "fixture-workspace-left",
                &registration_hash,
                &instruction_hash,
                100,
            ));
            assert_eq!(
                AgentTask操作(
                    &mut c,
                    "AgentTask実行",
                    AgentTask要求(&session_id, instruction),
                    false,
                ),
                Err(対話失敗::権限拒否),
            );
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert!(c
                .agent_task_permissions
                .get(&session_id)
                .is_some_and(|grant| grant.owner_approval.is_some()));
        }
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "明示指定した実Codex CLIを資格情報なしloopback API経由でBroker Taskへ接続するときに実行する"]
    fn Broker承認経路から実CodexCLIをloopback偽APIで実行し隔離とcleanupを確認する_LIVE_RUNTIME() {
        use std::io::{Read, Write};

        let executable = std::env::var_os("GUI_SHELL_CODEX_TASK_BROKER_TEST_EXE")
            .map(std::path::PathBuf::from)
            .expect("GUI_SHELL_CODEX_TASK_BROKER_TEST_EXEにCodex CLI絶対pathを指定する");
        assert!(executable.is_absolute(), "Codex CLIは絶対pathで指定する");
        assert!(
            executable.is_file(),
            "指定したCodex CLIが通常fileとして存在する"
        );

        let fixture = super::broker_codex_fixture_support::BrokerCodexFixture::create();
        let workspace_path = fixture.workspace_path();
        let private = workspace_path.join("private");
        std::fs::create_dir_all(&private).expect("合成登録secret directory");
        let mut secret_file = std::fs::File::create(private.join("credential-backup.txt"))
            .expect("合成登録secret file");
        secret_file
            .write_all(b"synthetic-secret-content-never-returned")
            .expect("合成登録secret本文");
        let workspace_parent = workspace_path.parent().expect("専用fixture root");
        let mut outside_marker =
            std::fs::File::create(workspace_parent.join("outside-read-marker.txt"))
                .expect("合成Workspace外read marker");
        outside_marker
            .write_all(b"synthetic-outside-workspace-marker")
            .expect("合成Workspace外marker本文");
        let outside_write = workspace_parent.join("outside-write-marker.txt");
        assert!(!outside_write.exists(), "Workspace外write targetは未作成");

        let codex_home = workspace_parent.join("isolated-codex-home");
        std::fs::create_dir(&codex_home).expect("資格情報を持たない専用CODEX_HOME");
        let server =
            super::broker_codex_loopback_support::CodexLoopbackResponses::start(workspace_path)
                .expect("localhost限定Responses API試験用応答器");
        let mut inner =
            crate::adapters::codex_cli::CodexCliAdapter::new(&executable, workspace_path)
                .expect("実Codex CLIのversion/help interfaceを確認する");
        assert!(
            inner.AgentTask実行対応(),
            "Windows実CLIのworkspace-write interface"
        );
        let product_metadata = inner.agent_metadata().expect("製品Adapter metadata");
        assert_eq!(
            product_metadata["capabilities"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["capability_id"] == "task_execution")
                .unwrap()["support"]["status"],
            "unsupported",
            "統合試験は製品AdapterのTask能力宣言を変更しない"
        );
        inner.use_test_loopback_responses_api(server.port(), codex_home);

        let runtime_id = "codex-loopback-integration";
        let workspace_id = "codex-loopback-integration-workspace";
        let (workspace_handle, _, ancestry) =
            super::super::workspace_root::open_isolated_root_with_ancestry(workspace_path, &[])
                .unwrap();
        let mut workspaces = super::super::workspace::WorkspaceRegistry::default();
        let secret_paths = ["private/credential-backup.txt".to_owned()];
        workspaces
            .register(
                runtime_id,
                workspace_id,
                workspace_handle,
                &secret_paths,
                Some(ancestry),
                &mut |_, _| Ok(()),
            )
            .unwrap();
        let binding = workspaces
            .dialogue_binding(runtime_id, workspace_id)
            .expect("Brokerが登録WorkspaceへAdapterを結合");

        let mut control = 対話制御::default();
        control
            .登録(
                runtime_id,
                Arc::new(Broker統合CodexFixtureAdapter { inner }),
            )
            .unwrap();
        let mut audit_events = Vec::new();
        let mut audit = |operation: &str, session: &str, hash: &str| {
            audit_events.push((operation.to_owned(), session.to_owned(), hash.to_owned()));
            Ok(format!("loopback-integration-audit-{}", audit_events.len()))
        };

        let started = control
            .操作_作業領域結合済み(
                "対話開始",
                &json!({"実行系ID":runtime_id,"作業領域ID":workspace_id}),
                false,
                100,
                Some(&binding),
                &mut audit,
            )
            .unwrap();
        let session_id = started["対話セッションID"].as_str().unwrap().to_owned();
        let instruction = "INTEGRATION_TASK_INSTRUCTION_SENTINEL: perform only the bounded synthetic workspace probe";
        let task_request = json!({
            "agent_runtime_id":runtime_id,
            "session_id":session_id,
            "workspace_id":workspace_id,
            "instruction":instruction,
        });
        let permission_request = json!({
            "agent_runtime_id":runtime_id,
            "session_id":session_id,
            "workspace_id":workspace_id,
        });

        assert_eq!(
            control.操作_作業領域結合済み(
                "AgentTaskWorkspacePermissionGrant",
                &permission_request,
                false,
                100,
                Some(&binding),
                &mut audit,
            ),
            Err(対話失敗::権限拒否),
            "Owner No相当ではWorkspace Permissionを発行しない"
        );
        assert_eq!(
            control.操作_作業領域結合済み(
                "AgentTask実行",
                &task_request,
                false,
                100,
                Some(&binding),
                &mut audit,
            ),
            Err(対話失敗::権限拒否),
            "Permissionと個別Owner Approvalがなければ実CLIを起動しない"
        );
        assert_eq!(
            server.accepted_connections(),
            0,
            "Owner Permission前の拒否要求ではCodex CLIを起動しない"
        );
        control
            .操作_作業領域結合済み(
                "AgentTaskWorkspacePermissionGrant",
                &permission_request,
                true,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("試験用Owner確認でWorkspace Permissionを発行");
        assert_eq!(
            control.操作_作業領域結合済み(
                "AgentTask実行",
                &task_request,
                false,
                100,
                Some(&binding),
                &mut audit,
            ),
            Err(対話失敗::権限拒否),
            "Task本文hashへ結合したApprovalがなければ実CLIを起動しない"
        );
        assert_eq!(
            server.accepted_connections(),
            0,
            "Task Approval前の拒否要求ではCodex CLIを起動しない"
        );
        control
            .操作_作業領域結合済み(
                "AgentTaskOwnerApprovalGrant",
                &task_request,
                true,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("試験用Owner確認で一回Approvalを発行");
        assert_eq!(
            server.post_requests(),
            0,
            "Approval発行はRuntime要求を送らない"
        );
        assert_eq!(
            server.accepted_connections(),
            0,
            "Owner PermissionとTask Approvalが揃うまでCodex CLIを起動しない"
        );

        let task = control
            .操作_作業領域結合済み(
                "AgentTask実行",
                &task_request,
                false,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("Brokerが承認済み実Codex Taskを開始");
        assert!(matches!(
            task["status"].as_str(),
            Some("running" | "completed")
        ));
        assert_eq!(
            control.操作_作業領域結合済み(
                "AgentTask実行",
                &task_request,
                false,
                100,
                Some(&binding),
                &mut audit,
            ),
            Err(対話失敗::権限拒否),
            "Permission／ApprovalはTask開始時に一回消費する"
        );

        let task_id = task["task_id"].as_str().unwrap();
        let deadline = Instant::now() + Duration::from_secs(60);
        let result = loop {
            let state = control
                .操作_作業領域結合済み(
                    "AgentTask状態",
                    &json!({"task_id":task_id}),
                    false,
                    100,
                    None,
                    &mut audit,
                )
                .unwrap();
            if state["status"] != "running" {
                break state;
            }
            assert!(
                Instant::now() < deadline,
                "実Codex Broker Taskの終了待ち期限"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        assert_eq!(
            result["status"],
            "completed",
            "Task state: {result}; accepted_connections={}; incomplete_requests=({}); model_list_gets={}; responses_post_attempts={}; invalid_post_bodies={}; response_write=({}); responses_posts={}; tool_offered={}; tool_call_sent={}; workspace_marker={}; outside_write={}; blocked_proxy_requests={} ({})",
            server.accepted_connections(),
            server.incomplete_request_summary(),
            server.model_list_requests(),
            server.response_post_attempts(),
            server.invalid_post_bodies(),
            server.response_write_summary(),
            server.post_requests(),
            server.tool_was_offered(),
            server.tool_call_was_sent(),
            workspace_path.join("broker-real-codex-marker.txt").exists(),
            outside_write.exists(),
            server.blocked_external_requests(),
            server.blocked_external_summary(),
        );
        assert!(result["result_hash"]
            .as_str()
            .unwrap()
            .starts_with("sha256:"));
        assert!(!result.to_string().contains(instruction));
        assert!(!result
            .to_string()
            .contains("synthetic-secret-content-never-returned"));
        assert!(!result.to_string().contains("合成試験Taskが完了しました"));
        let mut workspace_marker =
            std::fs::File::open(workspace_path.join("broker-real-codex-marker.txt"))
                .expect("実MxC tool childのWorkspace marker");
        let mut workspace_marker_bytes = Vec::new();
        workspace_marker
            .read_to_end(&mut workspace_marker_bytes)
            .expect("Workspace marker読取");
        assert_eq!(
            workspace_marker_bytes, b"synthetic-task-write",
            "実MxC tool childが登録Workspaceへだけ書き込む"
        );
        let mut secret_file = std::fs::File::open(private.join("credential-backup.txt")).unwrap();
        let mut secret_bytes = Vec::new();
        secret_file.read_to_end(&mut secret_bytes).unwrap();
        assert_eq!(
            secret_bytes, b"synthetic-secret-content-never-returned",
            "登録secretは変更されず、試験結果へ本文を出さない"
        );
        assert!(!outside_write.exists(), "Workspace外への書込がない");
        assert!(server.post_requests() >= 2, "Responses APIのtool／終端往復");
        assert_eq!(
            server.invalid_post_bodies(),
            0,
            "Responses API要求JSONが正しい"
        );
        assert_eq!(
            server.response_write_failures(),
            0,
            "Responses API応答を全送信"
        );
        assert_eq!(
            server.incomplete_request_count(),
            0,
            "途中切断したHTTP要求がない"
        );
        assert!(server.tool_was_offered(), "実CLIがBroker Task toolを公開");
        assert!(
            server.tool_call_was_sent(),
            "実CLIが偽APIの固定tool callを処理"
        );
        for entry in std::fs::read_dir(workspace_path).unwrap() {
            assert!(
                !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".d4p-tmp-"),
                "Broker管理WorkspaceTaskScratchをTask終端後に残さない"
            );
        }

        let cancel_instruction =
            "INTEGRATION_CANCEL_INSTRUCTION_SENTINEL: keep the bounded synthetic probe running";
        let heartbeat = workspace_path.join("broker-real-codex-cancel-heartbeat.txt");
        let heartbeat_literal = heartbeat.to_string_lossy().replace('\'', "''");
        server.set_command(format!(
            "$ErrorActionPreference='Stop'; <# 合成Workspace内で取消試験の稼働状態を記録する #> $heartbeat='{heartbeat_literal}'; [IO.File]::WriteAllText($heartbeat,'started'); while ($true) {{ [IO.File]::AppendAllText($heartbeat,'x'); Start-Sleep -Milliseconds 50 }}"
        ));
        let cancellation_marker = workspace_path.join("broker-real-codex-marker.txt");
        std::fs::remove_file(&cancellation_marker)
            .expect("次の合成Task用に成功Task markerを除去する");
        let cancel_task_request = json!({
            "agent_runtime_id":runtime_id,
            "session_id":session_id,
            "workspace_id":workspace_id,
            "instruction":cancel_instruction,
        });
        control
            .操作_作業領域結合済み(
                "AgentTaskWorkspacePermissionGrant",
                &permission_request,
                true,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("実CLI取消Task用の合成Workspace Permission発行");
        control
            .操作_作業領域結合済み(
                "AgentTaskOwnerApprovalGrant",
                &cancel_task_request,
                true,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("実CLI取消Task本文hashへ結合した合成Owner Approval発行");
        let cancel_task = control
            .操作_作業領域結合済み(
                "AgentTask実行",
                &cancel_task_request,
                false,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("Broker承認経路から実Codex CLIの取消probeを開始");
        assert_eq!(cancel_task["status"], "running");
        let heartbeat_deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if std::fs::metadata(&heartbeat).is_ok_and(|metadata| metadata.len() >= 4) {
                break;
            }
            assert!(
                Instant::now() < heartbeat_deadline,
                "実MxC tool childが取消前にheartbeatを更新する"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            server.tool_call_was_sent(),
            "実CLIへ取消用tool callを送信した"
        );
        let cancel_task_id = cancel_task["task_id"].as_str().unwrap();
        let cancelling = control
            .操作_作業領域結合済み(
                "AgentTask取消",
                &json!({"task_id":cancel_task_id}),
                false,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("実CLI取消をBroker Audit付きで要求");
        assert_eq!(
            cancelling["status"], "running",
            "取消要求だけでは実CLI process群の停止前にterminal化しない"
        );
        let cancellation_deadline = Instant::now() + Duration::from_secs(20);
        let cancelled = loop {
            let state = control
                .操作_作業領域結合済み(
                    "AgentTask状態",
                    &json!({"task_id":cancel_task_id}),
                    false,
                    100,
                    None,
                    &mut audit,
                )
                .expect("実CLI取消Task状態取得");
            if state["status"] != "running" {
                break state;
            }
            assert!(
                Instant::now() < cancellation_deadline,
                "実CLI取消後のterminal状態への遷移待ち期限"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        assert_eq!(cancelled["status"], "cancelled");
        assert!(cancelled.get("result_hash").is_none());
        let heartbeat_after_terminal = std::fs::metadata(&heartbeat)
            .expect("停止済みtool child heartbeatをstatする")
            .len();
        std::thread::sleep(Duration::from_millis(250));
        assert_eq!(
            std::fs::metadata(&heartbeat)
                .expect("terminal後heartbeatを再確認する")
                .len(),
            heartbeat_after_terminal,
            "Broker terminal取消後に実MxC tool childが稼働を継続しない"
        );
        for entry in std::fs::read_dir(workspace_path).unwrap() {
            assert!(
                !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".d4p-tmp-"),
                "実CLI取消後にBroker管理WorkspaceTaskScratchを残さない"
            );
        }
        drop(audit);
        assert!(audit_events.iter().any(|(operation, _, _)| {
            operation == "Agent Task実行開始（Permission／Owner Approval一回消費）"
        }));
        assert!(audit_events.iter().any(|(operation, _, _)| {
            operation == "Agent Task完了（結果本文非保存・hashのみ）"
        }));
        assert!(audit_events.iter().any(|(operation, id, _)| {
            operation.contains("取消要求") && id == cancel_task_id
        }));
        assert!(audit_events.iter().any(|(operation, id, _)| {
            operation.contains("失敗・取消") && id == cancel_task_id
        }));
        assert!(!format!("{audit_events:?}").contains(instruction));
        assert!(!format!("{audit_events:?}").contains(cancel_instruction));
        assert!(!cancelled.to_string().contains(cancel_instruction));
        eprintln!(
            "loopback_proxy_blocked_external_requests={}",
            server.blocked_external_requests()
        );
    }

    #[cfg(windows)]
    #[test]
    fn Broker制御からCodexAdapterを通るfakeTaskは成功・取消・期限後停止を区別してscratchを片付ける_fixture(
    ) {
        let fixture = super::broker_codex_fixture_support::BrokerCodexFixture::create();
        let workspace_path = fixture.workspace_path();

        let inner = crate::adapters::codex_cli::CodexCliAdapter::new(
            fixture.executable_path(),
            workspace_path,
        )
        .expect("fake Codex CLI interfaceを登録時検査する");
        let product_metadata = inner.agent_metadata().expect("製品Adapter metadata");
        assert_eq!(
            product_metadata["capabilities"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["capability_id"] == "task_execution")
                .unwrap()["support"]["status"],
            "unsupported",
            "縦断fixtureが製品Adapterの能力宣言を変更してはならない"
        );

        let runtime_id = "codex-fixture";
        let workspace_id = "codex-fixture-workspace";
        let (workspace_handle, _, ancestry) =
            super::super::workspace_root::open_isolated_root_with_ancestry(&workspace_path, &[])
                .unwrap();
        let mut workspaces = super::super::workspace::WorkspaceRegistry::default();
        let secret_paths = ["private/credential-backup.txt".to_owned()];
        workspaces
            .register(
                runtime_id,
                workspace_id,
                workspace_handle,
                &secret_paths,
                Some(ancestry),
                &mut |_, _| Ok(()),
            )
            .unwrap();
        let binding = workspaces
            .dialogue_binding(runtime_id, workspace_id)
            .expect("登録WorkspaceとのBroker結合");

        let mut control = 対話制御::default();
        control
            .登録(
                runtime_id,
                Arc::new(Broker統合CodexFixtureAdapter { inner }),
            )
            .unwrap();
        let mut audit_events = Vec::new();
        let mut audit = |operation: &str, session: &str, hash: &str| {
            audit_events.push((operation.to_owned(), session.to_owned(), hash.to_owned()));
            Ok(format!("fixture-audit-{}", audit_events.len()))
        };

        let started = control
            .操作_作業領域結合済み(
                "対話開始",
                &json!({"実行系ID":runtime_id,"作業領域ID":workspace_id}),
                false,
                100,
                Some(&binding),
                &mut audit,
            )
            .unwrap();
        let session_id = started["対話セッションID"].as_str().unwrap().to_owned();
        let instruction = "fixture内の一回限りTask";
        let task_request = json!({
            "agent_runtime_id":runtime_id,
            "session_id":session_id,
            "workspace_id":workspace_id,
            "instruction":instruction,
        });
        let permission_request = json!({
            "agent_runtime_id":runtime_id,
            "session_id":session_id,
            "workspace_id":workspace_id,
        });

        assert_eq!(
            control.操作_作業領域結合済み(
                "AgentTaskWorkspacePermissionGrant",
                &permission_request,
                false,
                100,
                Some(&binding),
                &mut audit,
            ),
            Err(対話失敗::権限拒否),
            "Owner No相当ではPermissionを発行しない"
        );
        assert_eq!(
            control.操作_作業領域結合済み(
                "AgentTask実行",
                &task_request,
                false,
                100,
                Some(&binding),
                &mut audit,
            ),
            Err(対話失敗::権限拒否),
            "承認なしではAdapterを起動しない"
        );

        control
            .操作_作業領域結合済み(
                "AgentTaskWorkspacePermissionGrant",
                &permission_request,
                true,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("test fixture内のOwner Permission発行");
        assert_eq!(
            control.操作_作業領域結合済み(
                "AgentTask実行",
                &task_request,
                false,
                100,
                Some(&binding),
                &mut audit,
            ),
            Err(対話失敗::権限拒否),
            "本文ごとのOwner Approvalなしでは起動しない"
        );
        control
            .操作_作業領域結合済み(
                "AgentTaskOwnerApprovalGrant",
                &task_request,
                true,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("test fixture内の本文hash結合Approval発行");

        let task = control
            .操作_作業領域結合済み(
                "AgentTask実行",
                &task_request,
                false,
                100,
                Some(&binding),
                &mut audit,
            )
            .unwrap();
        assert_eq!(task["status"], "running");
        assert_eq!(
            control.操作_作業領域結合済み(
                "AgentTask実行",
                &task_request,
                false,
                100,
                Some(&binding),
                &mut audit,
            ),
            Err(対話失敗::権限拒否),
            "ApprovalはTask開始時に一回消費する"
        );

        let task_id = task["task_id"].as_str().unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let result = loop {
            let state = control
                .操作_作業領域結合済み(
                    "AgentTask状態",
                    &json!({"task_id":task_id}),
                    false,
                    100,
                    None,
                    &mut audit,
                )
                .unwrap();
            if state["status"] != "running" {
                break state;
            }
            assert!(Instant::now() < deadline, "fake Codex Taskの終了待ち期限");
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(result["status"], "completed");
        assert!(result["result_hash"]
            .as_str()
            .unwrap()
            .starts_with("sha256:"));
        assert!(!result.to_string().contains(instruction));
        assert!(!result.to_string().contains("fixture-task-completed"));

        assert!(!workspace_path.join("fixture-canary").exists());
        for entry in std::fs::read_dir(&workspace_path).unwrap() {
            assert!(
                !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".d4p-tmp-"),
                "正常完了後にBroker管理Task scratchを残さない"
            );
        }

        let heartbeat = fixture.descendant_heartbeat_path();
        let cancel_instruction = format!("FIXTURE_TIMEOUT_WITH_DESCENDANT {}", heartbeat.display());
        let cancel_task_request = json!({
            "agent_runtime_id":runtime_id,
            "session_id":session_id,
            "workspace_id":workspace_id,
            "instruction":cancel_instruction.clone(),
        });
        control
            .操作_作業領域結合済み(
                "AgentTaskWorkspacePermissionGrant",
                &permission_request,
                true,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("取消fixture専用Permission発行");
        control
            .操作_作業領域結合済み(
                CANCEL_FIXTURE_OWNER_APPROVAL_OPERATION,
                &cancel_task_request,
                true,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("取消fixture専用本文hash結合Approval発行");
        let cancel_task = control
            .操作_作業領域結合済み(
                "AgentTask実行",
                &cancel_task_request,
                false,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("Brokerから偽Codex CLIの停止fixtureを開始");
        assert_eq!(cancel_task["status"], "running");
        assert!(
            fixture.wait_for_descendant_heartbeat(Duration::from_secs(5)) >= 2,
            "取消要求前にAdapter配下の子孫processが稼働している"
        );
        let cancel_task_id = cancel_task["task_id"].as_str().unwrap();
        let cancelling = control
            .操作_作業領域結合済み(
                "AgentTask取消",
                &json!({"task_id":cancel_task_id}),
                false,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("Broker取消要求を監査して送る");
        assert_eq!(
            cancelling["status"], "running",
            "取消要求だけではprocess終了前にterminal状態へ進めない"
        );
        let cancel_deadline = Instant::now() + Duration::from_secs(10);
        let cancelled = loop {
            let state = control
                .操作_作業領域結合済み(
                    "AgentTask状態",
                    &json!({"task_id":cancel_task_id}),
                    false,
                    100,
                    None,
                    &mut audit,
                )
                .expect("取消fixtureの状態取得");
            if state["status"] != "running" {
                break state;
            }
            assert!(
                Instant::now() < cancel_deadline,
                "取消後terminal状態への遷移待ち期限"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(cancelled["status"], "cancelled");
        assert!(cancelled.get("result_hash").is_none());
        let heartbeat_after_terminal = fixture.descendant_heartbeat_len();
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(
            fixture.descendant_heartbeat_len(),
            heartbeat_after_terminal,
            "terminal取消後に子孫processが稼働を継続しない"
        );
        fixture.assert_no_workspace_task_scratch();

        let deadline_heartbeat = workspace_path.join("deadline-descendant-heartbeat");
        let deadline_instruction = format!(
            "FIXTURE_TIMEOUT_WITH_DESCENDANT {}",
            deadline_heartbeat.display()
        );
        let deadline_task_request = json!({
            "agent_runtime_id":runtime_id,
            "session_id":session_id,
            "workspace_id":workspace_id,
            "instruction":deadline_instruction.clone(),
        });
        control
            .操作_作業領域結合済み(
                "AgentTaskWorkspacePermissionGrant",
                &permission_request,
                true,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("期限fixture専用Permission発行");
        control
            .操作_作業領域結合済み(
                CANCEL_FIXTURE_OWNER_APPROVAL_OPERATION,
                &deadline_task_request,
                true,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("期限fixture専用本文hash結合Approval発行");
        let deadline_task = control
            .操作_作業領域結合済み(
                "AgentTask実行",
                &deadline_task_request,
                false,
                100,
                Some(&binding),
                &mut audit,
            )
            .expect("期限fixtureをBroker Adapter経路で開始");
        assert_eq!(deadline_task["status"], "running");
        let heartbeat_deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if std::fs::metadata(&deadline_heartbeat).is_ok_and(|metadata| metadata.len() >= 2) {
                break;
            }
            assert!(
                Instant::now() < heartbeat_deadline,
                "期限要求前に子孫processが稼働markerを書く"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let deadline_task_id = deadline_task["task_id"].as_str().unwrap();
        control
            .agent_tasks
            .get_mut(deadline_task_id)
            .expect("Broker内Task記録")
            .deadline = Instant::now() - Duration::from_millis(1);
        let task_completion_deadline = Instant::now() + Duration::from_secs(10);
        let deadline_failed = loop {
            let state = control
                .操作_作業領域結合済み(
                    "AgentTask状態",
                    &json!({"task_id":deadline_task_id}),
                    false,
                    100,
                    None,
                    &mut audit,
                )
                .expect("期限fixtureの状態取得");
            if state["status"] != "running" {
                break state;
            }
            assert!(
                Instant::now() < task_completion_deadline,
                "期限停止後terminal状態への遷移待ち期限"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(
            deadline_failed["status"], "failed",
            "Broker deadlineによる停止をOwner取消へ分類しない"
        );
        assert!(deadline_failed.get("result_hash").is_none());
        let deadline_heartbeat_after_terminal =
            std::fs::metadata(&deadline_heartbeat).unwrap().len();
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(
            std::fs::metadata(&deadline_heartbeat).unwrap().len(),
            deadline_heartbeat_after_terminal,
            "deadline terminal後に子孫processが稼働を継続しない"
        );
        fixture.assert_no_workspace_task_scratch();
        drop(audit);
        assert!(audit_events
            .iter()
            .any(|event| event.0.contains("Owner Approval発行")));
        assert!(audit_events
            .iter()
            .any(|event| event.0.contains("一回消費")));
        assert!(audit_events
            .iter()
            .any(|event| event.0.contains("取消要求") && event.1 == cancel_task_id));
        assert!(audit_events
            .iter()
            .any(|event| { event.0.contains("失敗・取消") && event.1 == cancel_task_id }));
        assert!(audit_events
            .iter()
            .any(|event| { event.0.contains("失敗・取消") && event.1 == deadline_task_id }));
        let audit_text = serde_json::to_string(&audit_events).unwrap();
        assert!(!audit_text.contains(instruction));
        assert!(!audit_text.contains(&cancel_instruction));
        assert!(!audit_text.contains(&deadline_instruction));
        assert!(!cancelled.to_string().contains(&cancel_instruction));
        assert!(!deadline_failed.to_string().contains(&deadline_instruction));

        drop(control);
        drop(workspaces);
        drop(fixture);
    }

    #[test]
    fn AgentTaskは登録Workspace結合とAdapterをAgent間で混同しない_fixture() {
        let base = std::env::temp_dir().join(format!(
            "gui-shell-agent-task-isolation-{}",
            識別子生成().unwrap()
        ));
        let root_a = base.join("workspace-a");
        let root_b = base.join("workspace-b");
        std::fs::create_dir_all(&root_a).unwrap();
        std::fs::create_dir(&root_b).unwrap();

        let mut workspaces = super::super::workspace::WorkspaceRegistry::default();
        let (handle_a, _, ancestry_a) =
            super::super::workspace_root::open_isolated_root_with_ancestry(&root_a, &[]).unwrap();
        workspaces
            .register(
                "agent-a",
                "workspace-a",
                handle_a,
                &[],
                Some(ancestry_a),
                &mut |_, _| Ok(()),
            )
            .unwrap();
        let (handle_b, _, ancestry_b) =
            super::super::workspace_root::open_isolated_root_with_ancestry(&root_b, &[]).unwrap();
        workspaces
            .register(
                "agent-b",
                "workspace-b",
                handle_b,
                &[],
                Some(ancestry_b),
                &mut |_, _| Ok(()),
            )
            .unwrap();
        let binding_a = workspaces
            .dialogue_binding("agent-a", "workspace-a")
            .expect("Agent Aの現行登録結合");
        let binding_b = workspaces
            .dialogue_binding("agent-b", "workspace-b")
            .expect("Agent Bの現行登録結合");
        assert_ne!(binding_a.root_identity(), binding_b.root_identity());

        let calls_a = Arc::new(AtomicUsize::new(0));
        let calls_b = Arc::new(AtomicUsize::new(0));
        let mut control = 対話制御::default();
        control
            .登録(
                "agent-a",
                Arc::new(AgentTask隔離FixtureAdapter {
                    runtime_id: "agent-a".into(),
                    workspace_id: "workspace-a".into(),
                    adapter_identity: binding_a.root_identity(),
                    root_identity: binding_a.root_directory_identity(),
                    recovery_binding_hash: binding_a.recovery_binding_hash().into(),
                    marker_path: root_a.join("agent-a.marker"),
                    marker_content: "AGENT_A_ONLY_SENTINEL",
                    calls: calls_a.clone(),
                }),
            )
            .unwrap();
        control
            .登録(
                "agent-b",
                Arc::new(AgentTask隔離FixtureAdapter {
                    runtime_id: "agent-b".into(),
                    workspace_id: "workspace-b".into(),
                    adapter_identity: binding_b.root_identity(),
                    root_identity: binding_b.root_directory_identity(),
                    recovery_binding_hash: binding_b.recovery_binding_hash().into(),
                    marker_path: root_b.join("agent-b.marker"),
                    marker_content: "AGENT_B_ONLY_SENTINEL",
                    calls: calls_b.clone(),
                }),
            )
            .unwrap();

        let start_session =
            |control: &mut 対話制御,
             runtime: &str,
             workspace: &str,
             binding: &super::super::workspace::DialogueWorkspaceBinding| {
                control
                    .操作_作業領域結合済み(
                        "対話開始",
                        &json!({"実行系ID": runtime, "作業領域ID": workspace}),
                        false,
                        100,
                        Some(binding),
                        &mut |_, _, _| Ok("fixture-session-audit".into()),
                    )
                    .unwrap()["対話セッションID"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            };
        let session_a = start_session(&mut control, "agent-a", "workspace-a", &binding_a);
        let session_b = start_session(&mut control, "agent-b", "workspace-b", &binding_b);
        let request_a = json!({
            "agent_runtime_id":"agent-a",
            "session_id":session_a,
            "workspace_id":"workspace-a",
            "instruction":"fixture task A",
        });
        let request_b = json!({
            "agent_runtime_id":"agent-b",
            "session_id":session_b,
            "workspace_id":"workspace-b",
            "instruction":"fixture task B",
        });
        for (request, binding) in [(&request_a, &binding_a), (&request_b, &binding_b)] {
            AgentTask操作結合済み(
                &mut control,
                "AgentTaskWorkspacePermissionGrant",
                json!({
                    "agent_runtime_id": request["agent_runtime_id"],
                    "session_id": request["session_id"],
                    "workspace_id": request["workspace_id"],
                }),
                true,
                Some(binding),
            )
            .unwrap();
            AgentTask操作結合済み(
                &mut control,
                "AgentTaskOwnerApprovalGrant",
                request.clone(),
                true,
                Some(binding),
            )
            .unwrap();
        }

        assert_eq!(
            AgentTask操作結合済み(
                &mut control,
                "AgentTask実行",
                request_a.clone(),
                false,
                Some(&binding_b),
            ),
            Err(対話失敗::作業領域不在),
            "他Agentの実Workspace結合ではTaskを開始しない"
        );
        assert_eq!(calls_a.load(Ordering::SeqCst), 0);
        assert_eq!(calls_b.load(Ordering::SeqCst), 0);
        assert!(!root_a.join("agent-a.marker").exists());
        assert!(!root_b.join("agent-b.marker").exists());

        let started_a = AgentTask操作結合済み(
            &mut control,
            "AgentTask実行",
            request_a,
            false,
            Some(&binding_a),
        )
        .unwrap();
        let started_b = AgentTask操作結合済み(
            &mut control,
            "AgentTask実行",
            request_b,
            false,
            Some(&binding_b),
        )
        .unwrap();
        let task_a = started_a["task_id"].as_str().unwrap().to_owned();
        let task_b = started_b["task_id"].as_str().unwrap().to_owned();

        let wait_terminal = |control: &mut 対話制御, task_id: &str| {
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                let state = AgentTask操作結合済み(
                    control,
                    "AgentTask状態",
                    json!({"task_id": task_id}),
                    false,
                    None,
                )
                .unwrap();
                if state["status"] != "running" {
                    return state;
                }
                assert!(
                    Instant::now() < deadline,
                    "AgentTask terminal状態の待機期限"
                );
                std::thread::sleep(Duration::from_millis(2));
            }
        };
        let state_a = wait_terminal(&mut control, &task_a);
        let state_b = wait_terminal(&mut control, &task_b);
        assert_eq!(state_a["status"], "completed");
        assert_eq!(state_b["status"], "completed");
        assert!(!state_a.to_string().contains("AGENT_A_ONLY_SENTINEL"));
        assert!(!state_b.to_string().contains("AGENT_B_ONLY_SENTINEL"));
        assert_eq!(
            super::agent_task_fixture_fs::read_marker(&root_a.join("agent-a.marker")).unwrap(),
            "AGENT_A_ONLY_SENTINEL"
        );
        assert_eq!(
            super::agent_task_fixture_fs::read_marker(&root_b.join("agent-b.marker")).unwrap(),
            "AGENT_B_ONLY_SENTINEL"
        );
        assert!(!root_a.join("agent-b.marker").exists());
        assert!(!root_b.join("agent-a.marker").exists());
        assert_eq!(calls_a.load(Ordering::SeqCst), 1);
        assert_eq!(calls_b.load(Ordering::SeqCst), 1);

        drop(control);
        drop(workspaces);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn AgentTask取消はworker終了までrunningを保ちterminal監査後だけcancelledとなる() {
        let (mut c, calls) = 準備(false, false, true);
        let session_id = 開始(&mut c, "left");
        let instruction = "取消境界の試験";
        AgentTask権限とApprovalを発行(&mut c, &session_id, instruction);
        let started = AgentTask操作(
            &mut c,
            "AgentTask実行",
            AgentTask要求(&session_id, instruction),
            false,
        )
        .unwrap();
        let task_id = started["task_id"].as_str().unwrap().to_owned();
        for _ in 0..2_000 {
            if calls.load(Ordering::SeqCst) == 1 {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "Task workerの開始を確認する"
        );
        let cancelling =
            AgentTask操作(&mut c, "AgentTask取消", json!({"task_id": task_id}), false).unwrap();
        assert_eq!(cancelling["status"], "running");

        let mut state = Value::Null;
        for _ in 0..100 {
            state =
                AgentTask操作(&mut c, "AgentTask状態", json!({"task_id": task_id}), false).unwrap();
            if state["status"] != "running" {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(state["status"], "cancelled");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn 対話セッション一覧は開始監査に結合した有界metadataだけを返す() {
        let (mut c, adapter_calls) = 準備(false, false, false);
        let session_id = 開始(&mut c, "left");
        assert_eq!(adapter_calls.load(Ordering::SeqCst), 0);
        let listing = 操作(&mut c, "対話セッション一覧", json!({}), false).unwrap();

        assert_eq!(listing["版"], 1);
        assert_eq!(listing["対話セッション"].as_array().unwrap().len(), 1);
        assert_eq!(listing["対話セッション"][0]["対話セッションID"], session_id);
        assert_eq!(listing["対話セッション"][0]["実行系ID"], "left");
        assert_eq!(listing["対話セッション"][0]["状態"], "利用中");
        assert_eq!(listing["対話セッション"][0]["作成監査ID"], "fixture-audit");
        assert_eq!(
            listing["対話セッション"][0]["作業領域ID"],
            "fixture-workspace-left"
        );
        assert_eq!(
            listing["対話セッション"][0]["作業領域結合監査ID"],
            "fixture-audit"
        );
        assert_eq!(listing["対話セッション"][0].as_object().unwrap().len(), 6);
        assert_eq!(adapter_calls.load(Ordering::SeqCst), 0);
        assert!(操作(
            &mut c,
            "対話セッション一覧",
            json!({"authority":"owner"}),
            false
        )
        .is_err());

        for index in 1..64 {
            let _ = 開始(&mut c, "left");
            assert_eq!(index, c.セッション.len() - 1);
        }
        assert_eq!(
            操作(&mut c, "対話セッション一覧", json!({}), false).unwrap()["対話セッション"]
                .as_array()
                .unwrap()
                .len(),
            64
        );
        assert_eq!(
            操作(&mut c, "対話開始", json!({"実行系ID":"left"}), false),
            Err(対話失敗::要求不正)
        );
    }

    #[test]
    fn Agent対話Sessionは同一RuntimeへのBroker検証済みWorkspace結合を必須とする() {
        let (mut c, adapter_calls) = 準備(false, false, false);
        let mut audit_reasons = Vec::new();
        let mut audit = |reason: &str, _: &str, _: &str| {
            audit_reasons.push(reason.to_owned());
            Ok(format!("audit-{}", audit_reasons.len()))
        };
        let payload = json!({"実行系ID":"left","作業領域ID":"workspace-left"});
        assert_eq!(
            c.操作_作業領域結合済み("対話開始", &payload, false, 100, None, &mut audit),
            Err(対話失敗::作業領域不在)
        );
        let wrong_runtime = super::super::workspace::DialogueWorkspaceBinding::for_test(
            "other-runtime",
            "workspace-left",
        );
        assert_eq!(
            c.操作_作業領域結合済み(
                "対話開始",
                &payload,
                false,
                100,
                Some(&wrong_runtime),
                &mut audit,
            ),
            Err(対話失敗::作業領域不在)
        );
        assert!(c.セッション.is_empty());

        let valid =
            super::super::workspace::DialogueWorkspaceBinding::for_test("left", "workspace-left");
        let result = c
            .操作_作業領域結合済み(
                "対話開始",
                &payload,
                false,
                100,
                Some(&valid),
                &mut audit,
            )
            .unwrap();
        assert!(result["対話セッションID"].is_string());
        drop(audit);
        assert_eq!(
            audit_reasons,
            ["対話開始", "対話Sessionと登録済みWorkspaceの明示結合",]
        );
        let session = c.セッション.values().next().unwrap();
        assert_eq!(session.作業領域ID.as_deref(), Some("workspace-left"));
        assert!(session.作業領域結合監査ID.is_some());
        assert_eq!(adapter_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn 通常RuntimeはWorkspaceなしで開始でき登録Workspace指定時はmetadataだけを結合する() {
        let (mut c, _) = 準備(false, false, false);
        assert!(操作(&mut c, "対話開始", json!({"実行系ID":"left"}), false).is_ok());

        let binding =
            super::super::workspace::DialogueWorkspaceBinding::for_test("left", "workspace-left");
        let result = c
            .操作_作業領域結合済み(
                "対話開始",
                &json!({"実行系ID":"left","作業領域ID":"workspace-left"}),
                false,
                100,
                Some(&binding),
                &mut |_, _, _| Ok("workspace-audit".into()),
            )
            .unwrap();
        assert!(result["対話セッションID"].is_string());
        let session = c
            .セッション
            .values()
            .find(|session| session.作業領域ID.as_deref() == Some("workspace-left"))
            .expect("選択Workspaceへのmetadata対応");
        assert!(session.作業領域結合監査ID.is_some());
    }

    #[test]
    fn AgentSessionはWorkspace結合監査に失敗したら登録されない() {
        let (mut c, _) = 準備(false, false, false);
        let binding =
            super::super::workspace::DialogueWorkspaceBinding::for_test("left", "workspace-left");
        let mut audit_count = 0;
        let result = c.操作_作業領域結合済み(
            "対話開始",
            &json!({"実行系ID":"left","作業領域ID":"workspace-left"}),
            false,
            100,
            Some(&binding),
            &mut |_, _, _| {
                audit_count += 1;
                if audit_count == 2 {
                    Err(対話失敗::監査失敗)
                } else {
                    Ok("dialogue-start-audit".into())
                }
            },
        );
        assert_eq!(result, Err(対話失敗::監査失敗));
        assert_eq!(audit_count, 2);
        assert!(c.セッション.is_empty());
    }

    #[test]
    fn 対話セッション一覧は一般Runtimeと未知実行系をAgent表示へ混ぜない() {
        let (mut c, _) = 準備(false, false, false);
        let agent_session = 開始(&mut c, "left");
        c.登録(
            "runtime-only",
            Arc::new(試験Adapter {
                回数: Arc::new(AtomicUsize::new(0)),
                失敗: false,
                別session: false,
                遅延: false,
                Agentmetadata有効: false,
            }),
        )
        .unwrap();
        let _runtime_session = 開始(&mut c, "runtime-only");
        c.セッション.insert(
            "f".repeat(32),
            セッション {
                対話セッションID: "f".repeat(32),
                実行系ID: "unregistered-runtime".into(),
                状態: "利用中".into(),
                作成監査ID: "audit-unregistered-runtime".into(),
                作業領域ID: None,
                作業領域結合監査ID: None,
                作業領域登録hash: None,
            },
        );

        let listing = 操作(&mut c, "対話セッション一覧", json!({}), false).unwrap();
        assert_eq!(listing["対話セッション"].as_array().unwrap().len(), 1);
        assert_eq!(
            listing["対話セッション"][0]["対話セッションID"],
            agent_session
        );
        assert_eq!(listing["対話セッション"][0]["実行系ID"], "left");
    }
    fn 要求(c: &mut 対話制御, s: &str) -> Value {
        操作(
            c,
            "対話送信",
            json!({"対話セッションID":s,"入力":"こんにちは"}),
            false,
        )
        .unwrap()
    }
    fn 承認(p: &Value, scope: &str) -> Value {
        json!({"要求ID":p["要求ID"],"要求hash":p["要求hash"],"表示範囲":scope})
    }
    fn 評価成功受信(対話セッションID: String, 応答完了: Instant) -> 受信結果 {
        受信結果 {
            結果: Ok(実行結果 {
                対話セッションID,
                本文: "評価結果".into(),
                参照: Vec::new(),
                能力: Vec::new(),
                経路: "fixture".into(),
                追跡ID: "a".repeat(32),
                追跡hash: format!("sha256:{}", "b".repeat(64)),
                保留: false,
                生応答: b"evaluation-raw".to_vec(),
            }),
            生受信: vec![b"evaluation-raw".to_vec()],
            応答完了: Some(応答完了),
        }
    }
    fn 完了(c: &mut 対話制御, p: &Value) -> Value {
        for _ in 0..200 {
            let v = 操作(c, "対話取得", json!({"要求ID":p["要求ID"]}), false).unwrap();
            if v["状態"] == "完了" {
                assert!(v["実行記録"]["終了時刻"].is_i64());
                assert!(v["実行記録"]["終了監査ID"].is_string());
                assert_eq!(v["実行記録"]["要求ID"], p["要求ID"]);
                return v["結果"].clone();
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("対話完了の待機期限");
    }

    #[test]
    fn 対話終了はsession上限を回復する() {
        let (mut c, _) = 準備(false, false, false);
        let sessions = (0..64).map(|_| 開始(&mut c, "left")).collect::<Vec<_>>();

        assert_eq!(c.評価要求可能数(), 0);
        assert_eq!(
            操作(&mut c, "対話開始", json!({"実行系ID":"left"}), false),
            Err(対話失敗::要求不正)
        );

        操作(
            &mut c,
            "対話終了",
            json!({"対話セッションID": sessions[0]}),
            false,
        )
        .unwrap();
        assert_eq!(c.評価要求可能数(), 1);
        assert!(!開始(&mut c, "left").is_empty());
    }

    #[test]
    fn 実行系IDはIPC契約と同じ境界を使う() {
        for accepted in ["local", "local-1", "A.b_c-9"] {
            assert!(実行系ID妥当(accepted), "許可するID: {accepted}");
        }
        for rejected in [
            "",
            ".local",
            "_local",
            "local runtime",
            "local\nnext",
            "実行系",
            "pid:1234",
            "local/..",
        ] {
            assert!(!実行系ID妥当(rejected), "拒否するID: {rejected:?}");
        }
        assert!(!実行系ID妥当(&"a".repeat(129)));
    }

    #[test]
    fn 資源統計は高精度時計を持たず完了数と失敗数だけを返す() {
        let (mut c, _) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let pending = 要求(&mut c, &session);
        操作(&mut c, "対話承認", 承認(&pending, "full"), true).unwrap();
        完了(&mut c, &pending);

        let stats = c.資源統計("left");
        assert_eq!(stats.完了要求数, 1);
        assert_eq!(stats.失敗要求数, 0);
    }

    #[test]
    fn 評価隔離済み対話は通常取得を拒否し内部進捗だけを返す() {
        let (mut c, _) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let pending = 要求(&mut c, &session);
        let request_id = pending["要求ID"].as_str().unwrap();

        assert!(matches!(
            c.評価進捗(request_id, 100, &mut |_, _, _| Ok("fixture-audit".into())),
            Err(対話失敗::要求不正)
        ));
        assert_eq!(c.評価隔離(request_id), Ok(()));
        assert_eq!(c.評価隔離(request_id), Err(対話失敗::要求不正));
        for owner in [false, true] {
            assert_eq!(
                操作(&mut c, "対話取得", json!({"要求ID": request_id}), owner),
                Err(対話失敗::権限拒否)
            );
        }
        let pending_progress = c
            .評価進捗(request_id, 100, &mut |_, _, _| Ok("fixture-audit".into()))
            .unwrap();
        assert_eq!(pending_progress.状態, "承認待ち");
        assert!(pending_progress.結果.is_none());

        let mut reasons = Vec::new();
        c.操作(
            "対話承認",
            &承認(&pending, "full"),
            true,
            101,
            &mut |reason, _, _| {
                reasons.push(reason.to_owned());
                Ok("fixture-audit".into())
            },
        )
        .unwrap();
        let evaluation_reason = reasons
            .iter()
            .find(|reason| reason.starts_with("対話送信承認"))
            .unwrap();
        assert!(evaluation_reason.contains("評価隔離=true"));
        assert!(!evaluation_reason.contains("試験専用"));

        let limit = Instant::now() + Duration::from_secs(2);
        let completed = loop {
            let progress = c
                .評価進捗(request_id, 102, &mut |_, _, _| Ok("fixture-audit".into()))
                .unwrap();
            if progress.状態 == "完了" {
                break progress;
            }
            assert!(Instant::now() < limit, "評価対話完了の待機期限");
            std::thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(completed.結果.unwrap()["本文"], "こんにちは");
        assert_eq!(
            c.保存対象(&json!({"要求ID": request_id, "要求hash": pending["要求hash"]})),
            Err(対話失敗::権限拒否)
        );
        for owner in [false, true] {
            assert_eq!(
                操作(&mut c, "対話取得", json!({"要求ID": request_id}), owner),
                Err(対話失敗::権限拒否)
            );
        }

        let normal_session = 開始(&mut c, "left");
        let normal_pending = 要求(&mut c, &normal_session);
        let mut normal_reasons = Vec::new();
        c.操作(
            "対話承認",
            &承認(&normal_pending, "full"),
            true,
            103,
            &mut |reason, _, _| {
                normal_reasons.push(reason.to_owned());
                Ok("fixture-audit".into())
            },
        )
        .unwrap();
        assert!(normal_reasons
            .iter()
            .any(|reason| reason.contains("Permission=left:試験専用")
                && !reason.contains("評価隔離=true")));
    }

    #[test]
    fn 評価遅延はworker応答完了時刻から求めpoll待機を含めない() {
        let (mut c, _) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let pending = 要求(&mut c, &session);
        let request_id = pending["要求ID"].as_str().unwrap();
        c.評価隔離(request_id).unwrap();

        let (sender, receiver) = mpsc::sync_channel(1);
        let started = Instant::now() - Duration::from_millis(600);
        let completed = Instant::now() - Duration::from_millis(400);
        {
            let work = c.作業.get_mut(request_id).unwrap();
            work.状態 = "実行中";
            work.単調開始 = Some(started);
            work.実行期限 = Some(Instant::now() + Duration::from_secs(1));
            work.受信 = Some(receiver);
        }
        sender.send(評価成功受信(session, completed)).unwrap();
        std::thread::sleep(Duration::from_millis(250));

        let progress = c
            .評価進捗(request_id, 101, &mut |_, _, _| Ok("fixture-audit".into()))
            .unwrap();
        let latency = progress.単調応答Millis.unwrap();
        assert!(
            latency >= 100 && latency < 350,
            "poll時刻を含む遅延: {latency}"
        );
    }

    #[test]
    fn 期限前に完了した評価対話は期限後pollでも採用する() {
        let (mut c, _) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let pending = 要求(&mut c, &session);
        let request_id = pending["要求ID"].as_str().unwrap();
        c.評価隔離(request_id).unwrap();

        let (sender, receiver) = mpsc::sync_channel(1);
        let now = Instant::now();
        let deadline = now - Duration::from_millis(100);
        let response_completed = now - Duration::from_millis(200);
        {
            let work = c.作業.get_mut(request_id).unwrap();
            work.状態 = "実行中";
            work.単調開始 = Some(now - Duration::from_millis(300));
            work.実行期限 = Some(deadline);
            work.受信 = Some(receiver);
        }
        sender
            .send(評価成功受信(session, response_completed))
            .unwrap();

        let mut reasons = Vec::new();
        let progress = c
            .評価進捗(request_id, 101, &mut |reason, _, _| {
                reasons.push(reason.to_owned());
                Ok("fixture-audit".into())
            })
            .unwrap();
        assert_eq!(progress.状態, "完了");
        assert_eq!(progress.結果.unwrap()["状態"], "成功");
        assert!(progress.単調応答Millis.is_some());
        assert!(!reasons.iter().any(|reason| reason == "対話期限超過"));
    }

    #[test]
    fn 中止後にworker応答を回収しても評価遅延を残さない() {
        let (mut c, calls) = 準備(false, false, true);
        let session = 開始(&mut c, "left");
        let workspace_id = "fixture-workspace-left";
        let binding =
            super::super::workspace::DialogueWorkspaceBinding::for_test("left", workspace_id);
        let task_permission = json!({
            "agent_runtime_id":"left",
            "session_id":session,
            "workspace_id":workspace_id,
        });
        c.操作_作業領域結合済み(
            "AgentTaskWorkspacePermissionGrant",
            &task_permission,
            true,
            100,
            Some(&binding),
            &mut |_, _, _| Ok("fixture-task-permission-audit".into()),
        )
        .expect("中止前はSession結合Permissionを発行できる");
        assert!(c.agent_task_permissions.contains_key(&session));
        let pending = 要求(&mut c, &session);
        let request_id = pending["要求ID"].as_str().unwrap();
        c.評価隔離(request_id).unwrap();
        操作(&mut c, "対話承認", 承認(&pending, "full"), true).unwrap();
        let limit = Instant::now() + Duration::from_secs(2);
        while calls.load(Ordering::SeqCst) == 0 {
            assert!(Instant::now() < limit, "worker開始待機期限");
            std::thread::sleep(Duration::from_millis(1));
        }
        操作(&mut c, "対話中止", json!({"要求ID": request_id}), false).unwrap();
        assert!(
            !c.agent_task_permissions.contains_key(&session),
            "隔離されたSessionのTask Permissionは直ちに失効する"
        );
        std::thread::sleep(Duration::from_millis(150));

        let progress = c
            .評価進捗(request_id, 101, &mut |_, _, _| Ok("fixture-audit".into()))
            .unwrap();
        assert_eq!(progress.状態, "中止");
        assert_eq!(progress.単調応答Millis, None);
    }

    #[test]
    #[allow(non_snake_case)]
    fn AgentTask発行監査失敗ではPermissionを残さずApprovalを発行しない() {
        let (mut c, _) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let workspace_id = "fixture-workspace-left";
        let binding =
            super::super::workspace::DialogueWorkspaceBinding::for_test("left", workspace_id);
        let permission_request = json!({
            "agent_runtime_id": "left",
            "session_id": session.clone(),
            "workspace_id": workspace_id,
        });

        assert_eq!(
            c.操作_作業領域結合済み(
                "AgentTaskWorkspacePermissionGrant",
                &permission_request,
                true,
                100,
                Some(&binding),
                &mut |_, _, _| Err(対話失敗::監査失敗),
            ),
            Err(対話失敗::監査失敗)
        );
        assert!(
            !c.agent_task_permissions.contains_key(&session),
            "発行監査が確定しないPermissionは失効させる"
        );

        c.操作_作業領域結合済み(
            "AgentTaskWorkspacePermissionGrant",
            &permission_request,
            true,
            100,
            Some(&binding),
            &mut |_, _, _| Ok("permission-audit".into()),
        )
        .expect("監査可能になればPermissionを発行できる");

        let instruction = "この本文は監査へ出さない試験用Task";
        let owner_approval_request = json!({
            "agent_runtime_id": "left",
            "session_id": session.clone(),
            "workspace_id": workspace_id,
            "instruction": instruction.to_owned(),
        });
        let mut audit_reason = String::new();
        assert_eq!(
            c.操作_作業領域結合済み(
                "AgentTaskOwnerApprovalGrant",
                &owner_approval_request,
                true,
                100,
                Some(&binding),
                &mut |reason, _, _| {
                    audit_reason = reason.to_owned();
                    Err(対話失敗::監査失敗)
                },
            ),
            Err(対話失敗::監査失敗)
        );
        assert!(!audit_reason.contains(instruction));
        assert!(c.agent_task_permissions[&session].owner_approval.is_none());

        let preflight = c
            .操作_作業領域結合済み(
                "Agent作業要求検査",
                &owner_approval_request,
                false,
                100,
                Some(&binding),
                &mut |_, _, _| Ok("preflight-audit".into()),
            )
            .expect("発行失敗後も状態を安全に再検査できる");
        assert_eq!(preflight["Permission状態"], "有効");
        assert_eq!(preflight["Approval状態"], "未取得");
        assert_eq!(preflight["実行状態"], "未実行");
    }

    #[test]
    #[allow(non_snake_case)]
    fn AgentTaskOwnerApprovalはWorkspace登録hash差替後に再利用できない() {
        const OWNER_APPROVAL_GRANT_OPERATION: &str = "AgentTaskOwnerApprovalGrant";
        let (mut c, _) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let workspace_id = "fixture-workspace-left";
        let binding =
            super::super::workspace::DialogueWorkspaceBinding::for_test("left", workspace_id);
        let permission_request = json!({
            "agent_runtime_id": "left",
            "session_id": session.clone(),
            "workspace_id": workspace_id,
        });
        let task_request = json!({
            "agent_runtime_id": "left",
            "session_id": session.clone(),
            "workspace_id": workspace_id,
            "instruction": "登録hash差替後に承認を再利用しない試験",
        });

        c.操作_作業領域結合済み(
            "AgentTaskWorkspacePermissionGrant",
            &permission_request,
            true,
            100,
            Some(&binding),
            &mut |_, _, _| Ok("試験Task監査".into()),
        )
        .expect("現在のWorkspace登録hashへTask Permissionを結合できる");
        c.操作_作業領域結合済み(
            OWNER_APPROVAL_GRANT_OPERATION,
            &task_request,
            true,
            100,
            Some(&binding),
            &mut |_, _, _| Ok("試験Task監査".into()),
        )
        .expect("現在のPermissionと登録hashへOwner Approvalを結合できる");

        let current = c
            .操作_作業領域結合済み(
                "Agent作業要求検査",
                &task_request,
                false,
                100,
                Some(&binding),
                &mut |_, _, _| Ok("試験Task監査".into()),
            )
            .expect("発行時と同じ登録hashのpreflight");
        assert_eq!(current["Permission状態"], "有効");
        assert_eq!(current["Approval状態"], "有効");

        let replacement_binding =
            super::super::workspace::DialogueWorkspaceBinding::for_test_with_registration_hash(
                "left",
                workspace_id,
                "sha256:replacement-registration",
            );
        let after_registration_change = c.操作_作業領域結合済み(
            "Agent作業要求検査",
            &task_request,
            false,
            100,
            Some(&replacement_binding),
            &mut |_, _, _| Ok("試験Task監査".into()),
        );
        assert_eq!(after_registration_change, Err(対話失敗::セッション不一致));
    }

    #[test]
    #[allow(non_snake_case)]
    fn AgentTask操作は同じWorkspaceIDでもAdapter実体rootが異なれば拒否する() {
        const OWNER_APPROVAL_GRANT_OPERATION: &str = "AgentTaskOwnerApprovalGrant";
        let (mut c, _) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let workspace_id = "fixture-workspace-left";
        let binding =
            super::super::workspace::DialogueWorkspaceBinding::for_test_with_root_identity(
                "left",
                workspace_id,
                "sha256:fixture-registration",
                super::super::workspace_root::DirectoryIdentity {
                    device: 1,
                    file_id: 2,
                },
            );
        let request = json!({
            "agent_runtime_id":"left",
            "session_id":session.clone(),
            "workspace_id":workspace_id,
            "instruction":"別rootへのPermission発行を拒否する"
        });
        assert_eq!(
            c.操作_作業領域結合済み(
                "Agent作業要求検査",
                &request,
                false,
                100,
                Some(&binding),
                &mut |_, _, _| Ok("fixture-audit".into()),
            ),
            Err(対話失敗::作業領域不在)
        );
        assert!(!c.agent_task_permissions.contains_key(&session));

        let permission_request = json!({
            "agent_runtime_id":"left",
            "session_id":session.clone(),
            "workspace_id":workspace_id
        });
        assert_eq!(
            c.操作_作業領域結合済み(
                "AgentTaskWorkspacePermissionGrant",
                &permission_request,
                true,
                100,
                Some(&binding),
                &mut |_, _, _| Ok("fixture-audit".into()),
            ),
            Err(対話失敗::作業領域不在)
        );
        assert!(!c.agent_task_permissions.contains_key(&session));

        let matching_binding =
            super::super::workspace::DialogueWorkspaceBinding::for_test("left", workspace_id);
        c.操作_作業領域結合済み(
            "AgentTaskWorkspacePermissionGrant",
            &permission_request,
            true,
            100,
            Some(&matching_binding),
            &mut |_, _, _| Ok("fixture-audit".into()),
        )
        .expect("一致するrootだけでPermissionを発行する");
        assert_eq!(
            c.操作_作業領域結合済み(
                OWNER_APPROVAL_GRANT_OPERATION,
                &request,
                true,
                100,
                Some(&binding),
                &mut |_, _, _| Ok("fixture-audit".into()),
            ),
            Err(対話失敗::作業領域不在)
        );
        assert!(c.agent_task_permissions[&session].owner_approval.is_none());
    }

    #[test]
    fn 保存対象は全文の完了と確定記録を要求する() {
        for (scope, fail) in [
            ("none", false),
            ("hash_only", false),
            ("summary", false),
            ("redacted", false),
            ("full", true),
            ("full", false),
        ] {
            let (mut c, _) = 準備(fail, false, false);
            let session = 開始(&mut c, "left");
            let p = 要求(&mut c, &session);
            let select = json!({"要求ID":p["要求ID"],"要求hash":p["要求hash"]});
            assert!(c.保存対象(&select).is_err());
            操作(&mut c, "対話承認", 承認(&p, scope), true).unwrap();
            完了(&mut c, &p);
            if scope != "full" || fail {
                assert!(c.保存対象(&select).is_err());
                continue;
            }
            let content = c.保存対象(&select).unwrap();
            assert_eq!(content["要求"]["入力"], "こんにちは");
            assert_eq!(content["結果"]["本文"], "こんにちは");
            assert!(!content.to_string().contains("raw-private"));
            assert!(c
                .保存対象(&json!({"要求ID":p["要求ID"],"要求hash":"wrong"}))
                .is_err());
            let mut bad = select.clone();
            bad["本文"] = json!("注入");
            assert!(c.保存対象(&bad).is_err());
            c.作業
                .get_mut(p["要求ID"].as_str().unwrap())
                .unwrap()
                .保存済み結果証跡 = false;
            assert!(c.保存対象(&select).is_err());
            c.資格隔離(&[session]);
            assert!(c.保存対象(&select).is_err());
        }
    }

    #[test]
    fn 回帰Case情報は完了した全文結果のhashと監査だけを返す() {
        let (mut c, _) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let pending = 要求(&mut c, &session);
        assert!(c
            .回帰Case情報(
                pending["要求ID"].as_str().unwrap(),
                pending["要求hash"].as_str().unwrap()
            )
            .is_err());
        操作(&mut c, "対話承認", 承認(&pending, "full"), true).unwrap();
        完了(&mut c, &pending);
        let source = c
            .回帰Case情報(
                pending["要求ID"].as_str().unwrap(),
                pending["要求hash"].as_str().unwrap(),
            )
            .unwrap();
        assert_eq!(source.要求ID, pending["要求ID"].as_str().unwrap());
        assert_eq!(source.要求hash, pending["要求hash"].as_str().unwrap());
        assert_eq!(source.結果状態, "成功");
        assert!(source.応答hash.starts_with("sha256:"));
        assert!(!source.終了監査ID.is_empty());
        assert!(c
            .回帰Case情報(
                pending["要求ID"].as_str().unwrap(),
                &format!("{}x", pending["要求hash"])
            )
            .is_err());
    }
    #[test]
    fn 結果証跡は表示範囲を保持し一度だけ保存する() {
        for (scope, fail) in [
            ("none", false),
            ("hash_only", false),
            ("summary", false),
            ("redacted", false),
            ("full", false),
            ("full", true),
        ] {
            let (mut c, _) = 準備(fail, false, false);
            let session = 開始(&mut c, "left");
            let p = 要求(&mut c, &session);
            操作(&mut c, "対話承認", 承認(&p, scope), true).unwrap();
            let mut proofs = Vec::new();
            let mut finished = false;
            for _ in 0..200 {
                let response = c
                    .操作(
                        "対話取得",
                        &json!({"要求ID":p["要求ID"]}),
                        false,
                        101,
                        &mut |reason, _, hash| {
                            if let Some(body) = reason.strip_prefix("対話結果証跡:") {
                                assert_eq!(sha256_tagged(body.as_bytes()), hash);
                                assert!(!body.contains("fixture") && !body.contains("raw-private"));
                                proofs.push(serde_json::from_str::<Value>(body).unwrap());
                            }
                            Ok("proof-audit".into())
                        },
                    )
                    .unwrap();
                if response["状態"] == "完了" {
                    finished = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(finished);
            c.操作(
                "対話取得",
                &json!({"要求ID":p["要求ID"]}),
                false,
                102,
                &mut |reason, _, _| {
                    assert!(!reason.starts_with("対話結果証跡:"));
                    Ok("next-audit".into())
                },
            )
            .unwrap();
            if scope == "none" || fail {
                assert!(proofs.is_empty());
            } else {
                assert_eq!(proofs.len(), 1);
                let proof = &proofs[0];
                assert_eq!(proof["要求hash"], p["要求hash"]);
                assert_eq!(proof["応答hash"], sha256_tagged(b"raw-private"));
                assert_eq!(proof["能力申告hash"].is_string(), scope == "full");
                assert_eq!(proof["経路申告hash"].is_string(), scope == "full");
                assert_eq!(proof["追跡参照hash"].is_string(), scope == "full");
            }
        }
    }
    #[test]
    fn 結果証跡の書込失敗で本文返却と再送を止める() {
        let (mut c, _) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let p = 要求(&mut c, &session);
        操作(&mut c, "対話承認", 承認(&p, "full"), true).unwrap();
        let mut rejected = false;
        for _ in 0..200 {
            let response = c.操作(
                "対話取得",
                &json!({"要求ID":p["要求ID"]}),
                false,
                101,
                &mut |reason, _, _| {
                    if reason.starts_with("対話結果証跡:") {
                        Err(対話失敗::監査失敗)
                    } else {
                        Ok("proof-audit".into())
                    }
                },
            );
            if response == Err(対話失敗::監査失敗) {
                rejected = true;
                break;
            }
            assert_ne!(response.unwrap()["状態"], "完了");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(rejected);
        assert_eq!(
            操作(&mut c, "対話取得", json!({"要求ID":p["要求ID"]}), false),
            Err(対話失敗::監査失敗)
        );
        assert!(操作(
            &mut c,
            "対話送信",
            json!({"対話セッションID":session,"入力":"再送しない"}),
            false
        )
        .is_err());
    }

    #[test]
    fn 履歴保存は変更時だけ行い書込失敗で採用を止める() {
        let (mut c, _) = 準備(false, false, true);
        let session = 開始(&mut c, "left");
        let mut records = Vec::new();
        let mut audit = |reason: &str, _: &str, hash: &str| {
            if let Some(body) = reason.strip_prefix("対話実行記録:") {
                assert_eq!(sha256_tagged(body.as_bytes()), hash);
                records.push(serde_json::from_str::<Value>(body).unwrap());
            }
            Ok("history-test-event".into())
        };
        let p = c
            .操作(
                "対話送信",
                &json!({"対話セッションID":session,"入力":"保存しない本文"}),
                false,
                100,
                &mut audit,
            )
            .unwrap();
        for _ in 0..3 {
            c.操作(
                "対話取得",
                &json!({"要求ID":p["要求ID"]}),
                false,
                101,
                &mut audit,
            )
            .unwrap();
        }
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["版"], 2);
        assert_eq!(records[0]["状態"], "承認待ち");
        assert_eq!(
            records[0]["入力概要"],
            json!({"表示範囲":"hash_only",
            "入力hash":sha256_tagged("保存しない本文".as_bytes())})
        );
        assert!(!records[0].to_string().contains("保存しない本文"));
        assert_eq!(
            c.操作(
                "対話承認",
                &承認(&p, "full"),
                true,
                110,
                &mut |reason, _, _| if reason.starts_with("対話実行記録:") {
                    Err(対話失敗::監査失敗)
                } else {
                    Ok("approved-event".into())
                }
            ),
            Err(対話失敗::監査失敗)
        );
        assert_eq!(
            操作(&mut c, "対話取得", json!({"要求ID":p["要求ID"]}), false),
            Err(対話失敗::監査失敗)
        );
        assert!(操作(
            &mut c,
            "対話送信",
            json!({"対話セッションID":session,"入力":"再送しない"}),
            false
        )
        .is_err());
    }

    #[test]
    fn 実行記録は未実行と終了を区別し遅延応答で上書きしない() {
        for approve in [false, true] {
            let (mut c, _) = 準備(false, false, true);
            let session = 開始(&mut c, "left");
            let p = 要求(&mut c, &session);
            let id = p["要求ID"].as_str().unwrap();
            let before = 実行記録(&c.作業[id]);
            assert!(before["開始時刻"].is_null());
            assert!(before["終了監査ID"].is_null());
            if approve {
                c.操作(
                    "対話承認",
                    &承認(&p, "full"),
                    true,
                    110,
                    &mut |_, _, _| Ok("approved-event".into()),
                )
                .unwrap();
            }
            c.操作(
                "対話中止",
                &json!({"要求ID":id}),
                false,
                120,
                &mut |_, _, _| Ok("cancel-event".into()),
            )
            .unwrap();
            let record = 実行記録(&c.作業[id]);
            assert_eq!(record["作成時刻"], 100);
            assert_eq!(
                record["開始時刻"],
                if approve { json!(110) } else { Value::Null }
            );
            assert_eq!(
                record["開始監査ID"],
                if approve {
                    json!("approved-event")
                } else {
                    Value::Null
                }
            );
            assert_eq!(record["終了時刻"], 120);
            assert_eq!(record["終了監査ID"], "cancel-event");
            std::thread::sleep(Duration::from_millis(150));
            c.進捗反映(130, &mut |_, _, _| Ok("late-event".into()))
                .unwrap();
            assert_eq!(実行記録(&c.作業[id]), record);
            assert!(!record.to_string().contains("こんにちは"));
        }
        let (mut c, count) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let p = 要求(&mut c, &session);
        let v = c
            .操作(
                "対話取得",
                &json!({"要求ID":p["要求ID"]}),
                false,
                400,
                &mut |_, _, _| Ok("expired-event".into()),
            )
            .unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert!(v["実行記録"]["開始時刻"].is_null());
        assert_eq!(v["実行記録"]["終了時刻"], 400);
        assert_eq!(v["実行記録"]["終了監査ID"], "expired-event");
        assert_eq!(v["結果"]["失敗分類"], "期限超過");
    }

    #[test]
    fn 失効後の遅延応答は監査してから資源解放する() {
        let (mut c, count) = 準備(false, false, true);
        let session = 開始(&mut c, "left");
        let pending = 要求(&mut c, &session);
        操作(&mut c, "対話承認", 承認(&pending, "full"), true).unwrap();
        let limit = Instant::now() + Duration::from_secs(2);
        while count.load(Ordering::SeqCst) == 0 {
            assert!(Instant::now() < limit, "worker開始待機期限");
            std::thread::sleep(Duration::from_millis(1));
        }
        c.資格隔離(&[session.clone()]);
        assert!(c.作業.values().any(|v| v.受信.is_some()));
        let mut discarded = false;
        for _ in 0..200 {
            c.進捗反映(100, &mut |reason, _, _| {
                if reason == "採用終了後応答破棄" {
                    discarded = true;
                }
                Ok("fixture-audit".into())
            })
            .unwrap();
            if c.作業.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(discarded);
        assert!(c.作業.is_empty());
        assert!(c.セッション.is_empty());
    }

    #[test]
    fn 端末失効の反復で対話枠を占有し続けない() {
        let (mut c, _) = 準備(false, false, false);
        for _ in 0..70 {
            let session = 開始(&mut c, "left");
            let pending = 要求(&mut c, &session);
            c.資格隔離(&[session.clone()]);
            assert!(操作(&mut c, "対話承認", 承認(&pending, "full"), true).is_err());
            assert!(c.セッション.is_empty());
            assert!(c.作業.is_empty());
        }
    }

    #[test]
    fn Adapterが期限を無視してもCoreは遅延応答を採用しない() {
        let (mut c, _) = 準備(false, false, true);
        let s = 開始(&mut c, "left");
        let p = 要求(&mut c, &s);
        操作(&mut c, "対話承認", 承認(&p, "full"), true).unwrap();
        c.作業
            .get_mut(p["要求ID"].as_str().unwrap())
            .unwrap()
            .実行期限 = Some(Instant::now());
        assert_eq!(完了(&mut c, &p)["失敗分類"], "期限超過");
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(完了(&mut c, &p)["失敗分類"], "期限超過");
    }
    #[test]
    fn 承認資格とhashと一回性を強制する() {
        let (mut c, n) = 準備(false, false, false);
        let s = 開始(&mut c, "left");
        let p = 要求(&mut c, &s);
        assert_eq!(n.load(Ordering::SeqCst), 0);
        assert_eq!(
            操作(&mut c, "対話承認待ち", json!({}), false),
            Err(対話失敗::権限拒否)
        );
        assert_eq!(
            操作(&mut c, "対話承認", 承認(&p, "full"), false),
            Err(対話失敗::権限拒否)
        );
        let mut bad = 承認(&p, "full");
        bad["要求hash"] = json!("sha256:wrong");
        assert_eq!(操作(&mut c, "対話承認", bad, true), Err(対話失敗::権限拒否));
        操作(&mut c, "対話承認", 承認(&p, "full"), true).unwrap();
        assert_eq!(完了(&mut c, &p)["本文"], "こんにちは");
        assert_eq!(
            操作(&mut c, "対話承認", 承認(&p, "full"), true),
            Err(対話失敗::権限拒否)
        );
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn 非全文scopeは本文も参照も追跡も公開しない() {
        for scope in ["none", "hash_only", "summary", "redacted"] {
            let (mut c, _) = 準備(false, false, false);
            let s = 開始(&mut c, "left");
            let p = 要求(&mut c, &s);
            操作(&mut c, "対話承認", 承認(&p, scope), true).unwrap();
            let v = 完了(&mut c, &p);
            for key in ["本文", "経路", "追跡ID", "追跡hash"] {
                assert_eq!(v[key], "");
            }
            assert_eq!(v["参照"], json!([]));
            assert_eq!(v["能力"], json!([]));
            assert_eq!(v["応答hash"] == "", scope == "none");
            assert!(!v.to_string().contains("raw-private"));
        }
    }
    #[test]
    fn 監査失敗なら送信せず結果確定失敗なら本文を返さない() {
        let (mut c, n) = 準備(false, false, false);
        let s = 開始(&mut c, "left");
        let p = 要求(&mut c, &s);
        assert_eq!(
            c.操作(
                "対話承認",
                &承認(&p, "full"),
                true,
                100,
                &mut |_, _, _| Err(対話失敗::監査失敗)
            ),
            Err(対話失敗::監査失敗)
        );
        assert_eq!(n.load(Ordering::SeqCst), 0);
        操作(&mut c, "対話承認", 承認(&p, "full"), true).unwrap();
        let limit = Instant::now() + Duration::from_secs(2);
        loop {
            let result = c.操作(
                "対話取得",
                &json!({"要求ID":p["要求ID"]}),
                false,
                100,
                &mut |_, _, _| Err(対話失敗::監査失敗),
            );
            if result == Err(対話失敗::監査失敗) {
                break;
            }
            assert!(Instant::now() < limit, "完了監査の待機期限");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            操作(&mut c, "対話取得", json!({"要求ID":p["要求ID"]}), false),
            Err(対話失敗::監査失敗)
        );
    }
    #[test]
    fn 期限切れと未知fieldと空白入力を拒否する() {
        let (mut c, n) = 準備(false, false, false);
        let s = 開始(&mut c, "left");
        for v in [
            json!({"対話セッションID":s,"入力":" "}),
            json!({"対話セッションID":s,"入力":"x","authority":"owner"}),
            json!({"対話セッションID":s,"入力":"x".repeat(4097)}),
        ] {
            assert_eq!(操作(&mut c, "対話送信", v, false), Err(対話失敗::要求不正));
        }
        let p = 要求(&mut c, &s);
        assert_eq!(
            c.操作(
                "対話承認",
                &承認(&p, "full"),
                true,
                400,
                &mut |_, _, _| Ok("fixture-audit".into())
            ),
            Err(対話失敗::権限拒否)
        );
        assert_eq!(n.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn 取消後の応答は保持しても採用せずsessionを隔離する() {
        for running in [false, true] {
            let (mut c, n) = 準備(false, false, true);
            let s = 開始(&mut c, "left");
            let p = 要求(&mut c, &s);
            if running {
                操作(&mut c, "対話承認", 承認(&p, "full"), true).unwrap();
                let limit = Instant::now() + Duration::from_secs(2);
                while n.load(Ordering::SeqCst) == 0 {
                    assert!(Instant::now() < limit, "worker開始の待機期限");
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            操作(&mut c, "対話中止", json!({"要求ID":p["要求ID"]}), false).unwrap();
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                let v = 操作(&mut c, "対話取得", json!({"要求ID":p["要求ID"]}), false).unwrap();
                assert_eq!(v["結果"]["状態"], "中止");
                assert_eq!(v["結果"]["本文"], "");
                if c.作業[p["要求ID"].as_str().unwrap()].受信.is_none() {
                    break;
                }
                assert!(Instant::now() < deadline, "取消後応答の受信待機期限");
                std::thread::sleep(Duration::from_millis(5));
            }
            assert_eq!(
                操作(
                    &mut c,
                    "対話送信",
                    json!({"対話セッションID":s,"入力":"再送"}),
                    false
                ),
                Err(対話失敗::セッション不一致)
            );
            assert_eq!(n.load(Ordering::SeqCst), usize::from(running));
            if running {
                assert_eq!(c.作業[p["要求ID"].as_str().unwrap()].生受信.len(), 1);
            }
        }
    }
    #[test]
    fn 左右の成功と片側失敗と両側失敗を分離する() {
        for (left, right) in [(false, false), (true, false), (false, true), (true, true)] {
            let (mut c, _) = 準備(left, false, false);
            c.登録(
                "right",
                Arc::new(試験Adapter {
                    回数: Arc::new(AtomicUsize::new(0)),
                    失敗: right,
                    別session: false,
                    遅延: false,
                    Agentmetadata有効: true,
                }),
            )
            .unwrap();
            let a = 開始(&mut c, "left");
            let b = 開始(&mut c, "right");
            assert_ne!(a, b);
            let p = 要求(&mut c, &a);
            let q = 要求(&mut c, &b);
            操作(&mut c, "対話承認", 承認(&p, "full"), true).unwrap();
            操作(&mut c, "対話承認", 承認(&q, "hash_only"), true).unwrap();
            let x = 完了(&mut c, &p);
            let y = 完了(&mut c, &q);
            assert_eq!(x["実行系ID"], "left");
            assert_eq!(y["実行系ID"], "right");
            assert_eq!(x["状態"], if left { "失敗" } else { "成功" });
            assert_eq!(y["状態"], if right { "失敗" } else { "成功" });
            assert_eq!(y["本文"], "");
        }
    }
    #[test]
    fn Adapterの別session応答をCoreでも拒否する() {
        let (mut c, _) = 準備(false, true, false);
        let s = 開始(&mut c, "left");
        let p = 要求(&mut c, &s);
        操作(&mut c, "対話承認", 承認(&p, "full"), true).unwrap();
        let v = 完了(&mut c, &p);
        assert_eq!(v["失敗分類"], "セッション不一致");
        assert_eq!(v["本文"], "");
    }
}
