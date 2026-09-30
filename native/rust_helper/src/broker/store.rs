use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::audit_hash::hmac_sha256_tagged;
use crate::broker::audit::{BrokerAuditEvent, BrokerAuditLog};

const REPLAY_NONCE_RETENTION_SECONDS: i64 = 24 * 60 * 60;
const MAX_REPLAY_NONCE_RECORDS: usize = 100_000;
const MAX_A2A_STATE_BYTES: usize = 64 * 1024 * 1024;
const MAX_HOST_STATE_BYTES: usize = 8 * 1024 * 1024;
const MAX_ADAPTER_STATE_BYTES: usize = 8 * 1024 * 1024;
const MAX_AGENT_TASK_SCRATCH_STATE_BYTES: usize = 512 * 1024;
const MAX_SETUP_DOCTOR_REPORT_BYTES: usize = 64 * 1024;
const MAX_FIRST_RUN_CONFIGURATION_BYTES: usize = 16 * 1024;
const MAX_UPDATE_TRUST_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokerStoreError {
    Io(String),
    MalformedAuditState(String),
    TamperedAuditState(String),
    MalformedReplayState(String),
    MalformedSessionState(String),
    MalformedProfileState(String),
    MalformedUpdateState(String),
    MalformedUpdateTrust(String),
    MalformedNotificationState(String),
    MalformedA2aState(String),
    MalformedHostState(String),
    MalformedAdapterState(String),
    MalformedAgentTaskScratchState(String),
    TamperedAgentTaskScratchState(String),
    MalformedFirstRunConfiguration(String),
    MissingFirstRunConfiguration,
}

impl BrokerStoreError {
    pub fn message(&self) -> String {
        match self {
            BrokerStoreError::Io(message)
            | BrokerStoreError::MalformedAuditState(message)
            | BrokerStoreError::TamperedAuditState(message)
            | BrokerStoreError::MalformedReplayState(message)
            | BrokerStoreError::MalformedSessionState(message)
            | BrokerStoreError::MalformedProfileState(message)
            | BrokerStoreError::MalformedUpdateState(message)
            | BrokerStoreError::MalformedUpdateTrust(message)
            | BrokerStoreError::MalformedNotificationState(message)
            | BrokerStoreError::MalformedA2aState(message)
            | BrokerStoreError::MalformedHostState(message)
            | BrokerStoreError::MalformedAdapterState(message) => message.clone(),
            BrokerStoreError::MalformedAgentTaskScratchState(message)
            | BrokerStoreError::TamperedAgentTaskScratchState(message) => message.clone(),
            BrokerStoreError::MalformedFirstRunConfiguration(message) => message.clone(),
            BrokerStoreError::MissingFirstRunConfiguration => {
                "初回設定fileが存在しない".to_string()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerPersistentState {
    pub audit_log: BrokerAuditLog,
    pub seen_nonces: HashMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerPersistentStore {
    root: PathBuf,
    audit_path: PathBuf,
    audit_anchor_path: PathBuf,
    audit_anchor_key_path: PathBuf,
    replay_path: PathBuf,
    session_path: PathBuf,
    profile_path: PathBuf,
    update_path: PathBuf,
    update_trust_path: PathBuf,
    notification_path: PathBuf,
    a2a_path: PathBuf,
    host_path: PathBuf,
    adapter_path: PathBuf,
    agent_task_scratch_path: PathBuf,
    setup_doctor_report_path: PathBuf,
    first_run_configuration_path: PathBuf,
    audit_anchor_key: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ReplayNonceRecord {
    nonce: String,
    recorded_at_epoch_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SessionRecord {
    session_id: String,
    state: String,
    evidence_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AuditAnchorRecord {
    version: u32,
    event_count: usize,
    head_event_hash: Option<String>,
    anchor_hmac: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedAgentTaskScratchState {
    version: u8,
    state: Value,
    hmac: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct FirstRunConfigurationDocument {
    version: u8,
    product: String,
    ui_preferences: FirstRunUiPreferences,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct FirstRunUiPreferences {
    theme: String,
    density: String,
    locale: String,
}

impl BrokerPersistentStore {
    pub fn open_or_create(
        root: impl AsRef<Path>,
        session_id: &str,
    ) -> Result<(Self, BrokerPersistentState), BrokerStoreError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(|error| {
            BrokerStoreError::Io(format!("broker store directoryの作成に失敗: {error}"))
        })?;

        let store = Self {
            audit_path: root.join("audit.jsonl"),
            audit_anchor_path: root.join("audit_anchor.json"),
            audit_anchor_key_path: root.join("audit_anchor.key"),
            replay_path: root.join("replay_nonces.jsonl"),
            session_path: root.join("session.json"),
            profile_path: root.join("profiles.json"),
            update_path: root.join("updates.json"),
            update_trust_path: root.join("update_trust.json"),
            notification_path: root.join("notifications.json"),
            a2a_path: root.join("a2a_connections.json"),
            host_path: root.join("hosts.json"),
            adapter_path: root.join("adapters.json"),
            agent_task_scratch_path: root.join("agent_task_scratch_recovery.json"),
            setup_doctor_report_path: root.join("setup_doctor_report.json"),
            first_run_configuration_path: root.join("first_run_configuration.json"),
            audit_anchor_key: load_or_create_anchor_key(&root.join("audit_anchor.key"))?,
            root,
        };
        store.ensure_file_exists(&store.audit_path)?;
        store.ensure_file_exists(&store.replay_path)?;
        store.ensure_profile_file_exists()?;
        store.ensure_update_files_exist()?;
        store.ensure_notification_file_exists()?;
        store.ensure_a2a_file_exists()?;
        store.ensure_host_file_exists()?;
        store.ensure_adapter_file_exists()?;
        let audit_log = store.load_audit_log()?;
        store.verify_audit_anchor(&audit_log)?;
        let seen_nonces = store.load_replay_nonces(current_epoch_seconds())?;
        store.compact_replay_nonces(&seen_nonces)?;
        store.load_existing_session_if_present()?;
        store.write_session(session_id)?;
        Ok((
            store,
            BrokerPersistentState {
                audit_log,
                seen_nonces,
            },
        ))
    }

    pub fn append_audit_event(&self, event: &BrokerAuditEvent) -> Result<(), BrokerStoreError> {
        let serialized = serde_json::to_string(event).map_err(|error| {
            BrokerStoreError::MalformedAuditState(format!(
                "broker audit eventのserializeに失敗: {error}"
            ))
        })?;
        append_jsonl_line(&self.audit_path, &serialized)
            .map_err(|error| BrokerStoreError::Io(format!("audit eventの追記に失敗: {error}")))?;
        let anchored_log = self.load_audit_log()?;
        self.write_audit_anchor(&anchored_log)
    }

    pub fn append_replay_nonce(
        &self,
        nonce: &str,
        recorded_at_epoch_seconds: i64,
    ) -> Result<HashMap<String, i64>, BrokerStoreError> {
        let serialized = serde_json::to_string(&ReplayNonceRecord {
            nonce: nonce.into(),
            recorded_at_epoch_seconds: Some(recorded_at_epoch_seconds),
        })
        .map_err(|error| {
            BrokerStoreError::MalformedReplayState(format!(
                "replay nonceのserializeに失敗: {error}"
            ))
        })?;
        append_jsonl_line(&self.replay_path, &serialized)
            .map_err(|error| BrokerStoreError::Io(format!("replay nonceの追記に失敗: {error}")))?;
        let nonces = self.load_replay_nonces(recorded_at_epoch_seconds)?;
        self.compact_replay_nonces(&nonces)?;
        Ok(nonces)
    }

    pub(crate) fn verified_audit_log(&self) -> Result<BrokerAuditLog, BrokerStoreError> {
        let log = self.load_audit_log()?;
        self.verify_audit_anchor(&log)?;
        Ok(log)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn load_profile_state(&self) -> Result<Value, BrokerStoreError> {
        let raw = fs::read_to_string(&self.profile_path).map_err(|error| {
            BrokerStoreError::MalformedProfileState(format!(
                "broker profile stateの読取りに失敗: {error}"
            ))
        })?;
        if raw.trim().is_empty() {
            return Err(BrokerStoreError::MalformedProfileState(
                "broker profile stateが空である".to_string(),
            ));
        }
        serde_json::from_str(&raw).map_err(|error| {
            BrokerStoreError::MalformedProfileState(format!(
                "broker profile stateがmalformed: {error}"
            ))
        })
    }

    pub fn write_profile_state(&self, state: &Value) -> Result<(), BrokerStoreError> {
        let serialized = serde_json::to_string_pretty(state).map_err(|error| {
            BrokerStoreError::MalformedProfileState(format!(
                "broker profile stateのserializeに失敗: {error}"
            ))
        })?;
        atomic_write(&self.profile_path, serialized.as_bytes()).map_err(|error| {
            BrokerStoreError::Io(format!("broker profile stateの書込みに失敗: {error}"))
        })
    }

    pub fn load_update_state(&self) -> Result<Value, BrokerStoreError> {
        let raw = fs::read_to_string(&self.update_path).map_err(|error| {
            BrokerStoreError::MalformedUpdateState(format!(
                "broker update stateの読取りに失敗: {error}"
            ))
        })?;
        if raw.trim().is_empty() {
            return Err(BrokerStoreError::MalformedUpdateState(
                "broker update stateが空である".to_string(),
            ));
        }
        serde_json::from_str(&raw).map_err(|error| {
            BrokerStoreError::MalformedUpdateState(format!(
                "broker update stateがmalformed: {error}"
            ))
        })
    }

    pub fn write_update_state(&self, state: &Value) -> Result<(), BrokerStoreError> {
        let serialized = serde_json::to_string_pretty(state).map_err(|error| {
            BrokerStoreError::MalformedUpdateState(format!(
                "broker update stateのserializeに失敗: {error}"
            ))
        })?;
        atomic_write(&self.update_path, serialized.as_bytes()).map_err(|error| {
            BrokerStoreError::Io(format!("broker update stateの書込みに失敗: {error}"))
        })
    }

    pub fn load_notification_state(&self) -> Result<Value, BrokerStoreError> {
        let raw = fs::read_to_string(&self.notification_path).map_err(|error| {
            BrokerStoreError::MalformedNotificationState(format!(
                "broker notification stateの読取りに失敗: {error}"
            ))
        })?;
        if raw.trim().is_empty() {
            return Err(BrokerStoreError::MalformedNotificationState(
                "broker notification stateが空である".to_string(),
            ));
        }
        serde_json::from_str(&raw).map_err(|error| {
            BrokerStoreError::MalformedNotificationState(format!(
                "broker notification stateがmalformed: {error}"
            ))
        })
    }

    pub fn write_notification_state(&self, state: &Value) -> Result<(), BrokerStoreError> {
        let serialized = serde_json::to_string_pretty(state).map_err(|error| {
            BrokerStoreError::MalformedNotificationState(format!(
                "broker notification stateのserializeに失敗: {error}"
            ))
        })?;
        atomic_write(&self.notification_path, serialized.as_bytes()).map_err(|error| {
            BrokerStoreError::Io(format!("broker notification stateの書込みに失敗: {error}"))
        })
    }

    pub fn load_a2a_state(&self) -> Result<Value, BrokerStoreError> {
        let metadata = fs::metadata(&self.a2a_path).map_err(|error| {
            BrokerStoreError::MalformedA2aState(format!(
                "broker A2A connection stateのmetadata読取りに失敗: {error}"
            ))
        })?;
        if metadata.len() > MAX_A2A_STATE_BYTES as u64 {
            return Err(BrokerStoreError::MalformedA2aState(
                "broker A2A connection stateがbounded上限を超過".to_string(),
            ));
        }
        let raw = fs::read_to_string(&self.a2a_path).map_err(|error| {
            BrokerStoreError::MalformedA2aState(format!(
                "broker A2A connection stateの読取りに失敗: {error}"
            ))
        })?;
        if raw.trim().is_empty() {
            return Err(BrokerStoreError::MalformedA2aState(
                "broker A2A connection stateが空である".to_string(),
            ));
        }
        crate::broker::json_input::read_unique(&raw).map_err(|error| {
            BrokerStoreError::MalformedA2aState(format!(
                "broker A2A connection stateがmalformed: {error}"
            ))
        })
    }

    pub fn write_a2a_state(&self, state: &Value) -> Result<(), BrokerStoreError> {
        let serialized = serde_json::to_string_pretty(state).map_err(|error| {
            BrokerStoreError::MalformedA2aState(format!(
                "broker A2A connection stateのserializeに失敗: {error}"
            ))
        })?;
        if serialized.len() > MAX_A2A_STATE_BYTES {
            return Err(BrokerStoreError::MalformedA2aState(
                "broker A2A connection stateがbounded上限を超過".to_string(),
            ));
        }
        atomic_write(&self.a2a_path, serialized.as_bytes()).map_err(|error| {
            BrokerStoreError::Io(format!(
                "broker A2A connection stateの書込みに失敗: {error}"
            ))
        })
    }

    pub fn load_host_state(&self) -> Result<Value, BrokerStoreError> {
        let metadata = fs::metadata(&self.host_path).map_err(|error| {
            BrokerStoreError::MalformedHostState(format!(
                "broker Host registry stateのmetadata読取りに失敗: {error}"
            ))
        })?;
        if metadata.len() > MAX_HOST_STATE_BYTES as u64 {
            return Err(BrokerStoreError::MalformedHostState(
                "broker Host registry stateがbounded上限を超過".to_string(),
            ));
        }
        let raw = fs::read_to_string(&self.host_path).map_err(|error| {
            BrokerStoreError::MalformedHostState(format!(
                "broker Host registry stateの読取りに失敗: {error}"
            ))
        })?;
        if raw.trim().is_empty() {
            return Err(BrokerStoreError::MalformedHostState(
                "broker Host registry stateが空である".to_string(),
            ));
        }
        crate::broker::json_input::read_unique(&raw).map_err(|error| {
            BrokerStoreError::MalformedHostState(format!(
                "broker Host registry stateがmalformed: {error}"
            ))
        })
    }

    pub fn write_host_state(&self, state: &Value) -> Result<(), BrokerStoreError> {
        let serialized = serde_json::to_string_pretty(state).map_err(|error| {
            BrokerStoreError::MalformedHostState(format!(
                "broker Host registry stateのserializeに失敗: {error}"
            ))
        })?;
        if serialized.len() > MAX_HOST_STATE_BYTES {
            return Err(BrokerStoreError::MalformedHostState(
                "broker Host registry stateがbounded上限を超過".to_string(),
            ));
        }
        atomic_write(&self.host_path, serialized.as_bytes()).map_err(|error| {
            BrokerStoreError::Io(format!("broker Host registry stateの書込みに失敗: {error}"))
        })
    }

    pub fn load_adapter_state(&self) -> Result<Value, BrokerStoreError> {
        let metadata = fs::metadata(&self.adapter_path).map_err(|error| {
            BrokerStoreError::MalformedAdapterState(format!(
                "broker Adapter stateのmetadata読取りに失敗: {error}"
            ))
        })?;
        if metadata.len() > MAX_ADAPTER_STATE_BYTES as u64 {
            return Err(BrokerStoreError::MalformedAdapterState(
                "broker Adapter stateがbounded上限を超過".to_string(),
            ));
        }
        let raw = fs::read_to_string(&self.adapter_path).map_err(|error| {
            BrokerStoreError::MalformedAdapterState(format!(
                "broker Adapter stateの読取りに失敗: {error}"
            ))
        })?;
        if raw.trim().is_empty() {
            return Err(BrokerStoreError::MalformedAdapterState(
                "broker Adapter stateが空である".to_string(),
            ));
        }
        crate::broker::json_input::read_unique(&raw).map_err(|error| {
            BrokerStoreError::MalformedAdapterState(format!(
                "broker Adapter stateがmalformed: {error}"
            ))
        })
    }

    pub fn write_adapter_state(&self, state: &Value) -> Result<(), BrokerStoreError> {
        let serialized = serde_json::to_string_pretty(state).map_err(|error| {
            BrokerStoreError::MalformedAdapterState(format!(
                "broker Adapter stateのserializeに失敗: {error}"
            ))
        })?;
        if serialized.len() > MAX_ADAPTER_STATE_BYTES {
            return Err(BrokerStoreError::MalformedAdapterState(
                "broker Adapter stateがbounded上限を超過".to_string(),
            ));
        }
        atomic_write(&self.adapter_path, serialized.as_bytes()).map_err(|error| {
            BrokerStoreError::Io(format!("broker Adapter stateの書込みに失敗: {error}"))
        })
    }

    /// Broker Task scratch回復記録を既存store keyで認証して読む。
    /// 未作成時だけ空記録を初期化し、破損・改竄を空状態へ読み替えない。
    pub(crate) fn load_agent_task_scratch_state(&self) -> Result<Value, BrokerStoreError> {
        let metadata = match fs::symlink_metadata(&self.agent_task_scratch_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let state = serde_json::json!({"version": 1, "entries": []});
                self.write_agent_task_scratch_state(&state)?;
                return Ok(state);
            }
            Err(error) => {
                return Err(BrokerStoreError::MalformedAgentTaskScratchState(format!(
                    "Agent Task scratch回復記録のmetadataを確認できない: {error}"
                )));
            }
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
            return Err(BrokerStoreError::MalformedAgentTaskScratchState(
                "Agent Task scratch回復記録は通常fileに限る".to_string(),
            ));
        }
        if metadata.len() > MAX_AGENT_TASK_SCRATCH_STATE_BYTES as u64 {
            return Err(BrokerStoreError::MalformedAgentTaskScratchState(
                "Agent Task scratch回復記録がbounded上限を超過".to_string(),
            ));
        }
        let raw = fs::read_to_string(&self.agent_task_scratch_path).map_err(|error| {
            BrokerStoreError::MalformedAgentTaskScratchState(format!(
                "Agent Task scratch回復記録を読めない: {error}"
            ))
        })?;
        let value = crate::broker::json_input::read_unique(&raw).map_err(|error| {
            BrokerStoreError::MalformedAgentTaskScratchState(format!(
                "Agent Task scratch回復記録のJSONが不正: {error}"
            ))
        })?;
        let signed: SignedAgentTaskScratchState = serde_json::from_value(value).map_err(|_| {
            BrokerStoreError::MalformedAgentTaskScratchState(
                "Agent Task scratch回復記録の構造が不正".to_string(),
            )
        })?;
        if signed.version != 1 {
            return Err(BrokerStoreError::MalformedAgentTaskScratchState(
                "Agent Task scratch回復記録の版が非対応".to_string(),
            ));
        }
        let expected = agent_task_scratch_state_hmac(&self.audit_anchor_key, &signed.state);
        if !constant_time_equal(expected.as_bytes(), signed.hmac.as_bytes()) {
            return Err(BrokerStoreError::TamperedAgentTaskScratchState(
                "Agent Task scratch回復記録の認証に失敗".to_string(),
            ));
        }
        Ok(signed.state)
    }

    pub(crate) fn write_agent_task_scratch_state(
        &self,
        state: &Value,
    ) -> Result<(), BrokerStoreError> {
        let signed = SignedAgentTaskScratchState {
            version: 1,
            state: state.clone(),
            hmac: agent_task_scratch_state_hmac(&self.audit_anchor_key, state),
        };
        let serialized = serde_json::to_vec(&signed).map_err(|error| {
            BrokerStoreError::MalformedAgentTaskScratchState(format!(
                "Agent Task scratch回復記録を正本化できない: {error}"
            ))
        })?;
        if serialized.len() > MAX_AGENT_TASK_SCRATCH_STATE_BYTES {
            return Err(BrokerStoreError::MalformedAgentTaskScratchState(
                "Agent Task scratch回復記録がbounded上限を超過".to_string(),
            ));
        }
        atomic_write(&self.agent_task_scratch_path, &serialized).map_err(|error| {
            BrokerStoreError::Io(format!(
                "Agent Task scratch回復記録を耐久保存できない: {error}"
            ))
        })
    }

    pub fn write_setup_doctor_report(&self, bytes: &[u8]) -> Result<(), BrokerStoreError> {
        if bytes.is_empty() || bytes.len() > MAX_SETUP_DOCTOR_REPORT_BYTES {
            return Err(BrokerStoreError::Io(
                "Setup Doctor reportのsizeが許容範囲外".to_string(),
            ));
        }
        let root_metadata = fs::symlink_metadata(&self.root).map_err(|_| {
            BrokerStoreError::Io("Setup Doctor storeのrootを確認できない".to_string())
        })?;
        if !root_metadata.is_dir()
            || root_metadata.file_type().is_symlink()
            || is_reparse_point(&root_metadata)
        {
            return Err(BrokerStoreError::Io(
                "Setup Doctor storeのrootが通常directoryではない".to_string(),
            ));
        }
        match fs::symlink_metadata(&self.setup_doctor_report_path) {
            Ok(metadata)
                if !metadata.is_file()
                    || metadata.file_type().is_symlink()
                    || is_reparse_point(&metadata) =>
            {
                return Err(BrokerStoreError::Io(
                    "Setup Doctor reportの既存targetが通常fileではない".to_string(),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(BrokerStoreError::Io(
                    "Setup Doctor reportの既存targetを確認できない".to_string(),
                ));
            }
        }

        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).map_err(|_| {
            BrokerStoreError::Io("Setup Doctor一時file識別子を生成できない".to_string())
        })?;
        let temporary_path = self
            .root
            .join(format!(".setup_doctor_report-{}.tmp", hex::encode(random)));
        let write_result = (|| {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary_path)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary_path, &self.setup_doctor_report_path)
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&temporary_path);
            return Err(BrokerStoreError::Io(
                "Setup Doctor reportを安全に保存できない".to_string(),
            ));
        }
        Ok(())
    }

    pub fn initialize_first_run_configuration(&self) -> Result<(Vec<u8>, bool), BrokerStoreError> {
        let root_metadata = fs::symlink_metadata(&self.root).map_err(|_| {
            BrokerStoreError::MalformedFirstRunConfiguration(
                "初回設定の固定storeを確認できない".to_string(),
            )
        })?;
        if !root_metadata.is_dir()
            || root_metadata.file_type().is_symlink()
            || is_reparse_point(&root_metadata)
        {
            return Err(BrokerStoreError::MalformedFirstRunConfiguration(
                "初回設定の固定storeが通常directoryではない".to_string(),
            ));
        }

        match self.read_first_run_configuration() {
            Ok(bytes) => return Ok((bytes, false)),
            Err(BrokerStoreError::MissingFirstRunConfiguration) => {}
            Err(error) => return Err(error),
        }

        let bytes = br#"{
  "version": 1,
  "product": "D4 Pocket",
  "ui_preferences": {
    "theme": "system",
    "density": "compact",
    "locale": "ja-JP"
  }
}"#
        .to_vec();
        validate_first_run_configuration(&bytes)?;

        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).map_err(|_| {
            BrokerStoreError::Io("初回設定の一時file識別子を生成できない".to_string())
        })?;
        let temporary_path = self.root.join(format!(
            ".first_run_configuration-{}.tmp",
            hex::encode(random)
        ));
        let write_result = (|| {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary_path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::hard_link(&temporary_path, &self.first_run_configuration_path)?;
            fs::remove_file(&temporary_path)?;
            Ok::<(), std::io::Error>(())
        })();
        match write_result {
            Ok(()) => {
                let stored = self.read_first_run_configuration()?;
                Ok((stored, true))
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let _ = fs::remove_file(&temporary_path);
                let stored = self.read_first_run_configuration()?;
                Ok((stored, false))
            }
            Err(_) => {
                let _ = fs::remove_file(&temporary_path);
                Err(BrokerStoreError::Io(
                    "初回設定を既存fileを置換せず安全に生成できない".to_string(),
                ))
            }
        }
    }

    fn read_first_run_configuration(&self) -> Result<Vec<u8>, BrokerStoreError> {
        let metadata = match fs::symlink_metadata(&self.first_run_configuration_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(BrokerStoreError::MissingFirstRunConfiguration);
            }
            Err(_) => {
                return Err(BrokerStoreError::MalformedFirstRunConfiguration(
                    "初回設定fileを確認できない".to_string(),
                ));
            }
        };
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || is_reparse_point(&metadata)
            || metadata.len() > MAX_FIRST_RUN_CONFIGURATION_BYTES as u64
        {
            return Err(BrokerStoreError::MalformedFirstRunConfiguration(
                "初回設定fileの種類またはsizeが不正".to_string(),
            ));
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        File::open(&self.first_run_configuration_path)
            .map_err(|_| {
                BrokerStoreError::MalformedFirstRunConfiguration(
                    "初回設定fileを読めない".to_string(),
                )
            })?
            .take((MAX_FIRST_RUN_CONFIGURATION_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| {
                BrokerStoreError::MalformedFirstRunConfiguration(
                    "初回設定fileを読めない".to_string(),
                )
            })?;
        validate_first_run_configuration(&bytes)?;
        Ok(bytes)
    }

    pub fn load_update_trust(&self) -> Result<Option<Value>, BrokerStoreError> {
        let metadata = fs::symlink_metadata(&self.update_trust_path).map_err(|error| {
            BrokerStoreError::MalformedUpdateTrust(format!(
                "broker update trustのmetadata読取りに失敗: {error}"
            ))
        })?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || is_reparse_point(&metadata)
            || metadata.len() > MAX_UPDATE_TRUST_BYTES as u64
        {
            return Err(BrokerStoreError::MalformedUpdateTrust(
                "broker update trustの種類またはsizeが不正".to_string(),
            ));
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        File::open(&self.update_trust_path)
            .map_err(|_| {
                BrokerStoreError::MalformedUpdateTrust("broker update trustを読めない".to_string())
            })?
            .take((MAX_UPDATE_TRUST_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| {
                BrokerStoreError::MalformedUpdateTrust("broker update trustを読めない".to_string())
            })?;
        if bytes.len() > MAX_UPDATE_TRUST_BYTES {
            return Err(BrokerStoreError::MalformedUpdateTrust(
                "broker update trustがbounded上限を超過".to_string(),
            ));
        }
        let raw = String::from_utf8(bytes).map_err(|_| {
            BrokerStoreError::MalformedUpdateTrust("broker update trustがUTF-8ではない".to_string())
        })?;
        if raw.trim().is_empty() {
            return Err(BrokerStoreError::MalformedUpdateTrust(
                "broker update trustが空である".to_string(),
            ));
        }
        let value: Value = crate::broker::json_input::read_unique(&raw).map_err(|error| {
            BrokerStoreError::MalformedUpdateTrust(format!(
                "broker update trustがmalformed: {error}"
            ))
        })?;
        let object = value.as_object().ok_or_else(|| {
            BrokerStoreError::MalformedUpdateTrust(
                "broker update trustはobjectでなければならない".to_string(),
            )
        })?;
        let version = object.get("版").and_then(Value::as_u64);
        let base_keys = [
            "版",
            "algorithm",
            "public_key_der_hex",
            "public_key_fingerprint",
        ];
        let expected_keys: &[&str] = match version {
            Some(1) => &base_keys,
            Some(2) => &[
                "版",
                "algorithm",
                "public_key_der_hex",
                "public_key_fingerprint",
                "package_sources",
            ],
            _ => &[],
        };
        if object
            .keys()
            .any(|key| !expected_keys.contains(&key.as_str()))
            || expected_keys.iter().any(|key| !object.contains_key(*key))
        {
            return Err(BrokerStoreError::MalformedUpdateTrust(
                "broker update trustに未知または欠落fieldがある".to_string(),
            ));
        }
        if object.get("algorithm") != Some(&Value::from("Ed25519")) {
            return Err(BrokerStoreError::MalformedUpdateTrust(
                "broker update trustのalgorithmが不正".to_string(),
            ));
        }
        if object.get("public_key_der_hex") == Some(&Value::Null)
            && object.get("public_key_fingerprint") == Some(&Value::Null)
        {
            if version == Some(2)
                && object
                    .get("package_sources")
                    .and_then(Value::as_array)
                    .is_none_or(|sources| !sources.is_empty())
            {
                return Err(BrokerStoreError::MalformedUpdateTrust(
                    "公開鍵未設定時にpackage sourceを構成できない".to_string(),
                ));
            }
            return Ok(None);
        }
        Ok(Some(value))
    }

    fn ensure_profile_file_exists(&self) -> Result<(), BrokerStoreError> {
        if self.profile_path.exists() {
            return Ok(());
        }
        self.write_profile_state(&serde_json::json!({"版": 1, "profiles": []}))
    }

    fn ensure_update_files_exist(&self) -> Result<(), BrokerStoreError> {
        if !self.update_path.exists() {
            self.write_update_state(&serde_json::json!({"版": 1, "updates": []}))?;
        }
        if !self.update_trust_path.exists() {
            let trust = serde_json::json!({
                "版": 2,
                "algorithm": "Ed25519",
                "public_key_der_hex": null,
                "public_key_fingerprint": null,
                "package_sources": []
            });
            let serialized = serde_json::to_string_pretty(&trust).map_err(|error| {
                BrokerStoreError::MalformedUpdateTrust(format!(
                    "broker update trustの初期化serializeに失敗: {error}"
                ))
            })?;
            atomic_write(&self.update_trust_path, serialized.as_bytes()).map_err(|error| {
                BrokerStoreError::Io(format!("broker update trustの初期化に失敗: {error}"))
            })?;
        }
        Ok(())
    }

    fn ensure_notification_file_exists(&self) -> Result<(), BrokerStoreError> {
        if self.notification_path.exists() {
            return Ok(());
        }
        self.write_notification_state(&serde_json::json!({"版": 1, "states": []}))
    }

    fn ensure_a2a_file_exists(&self) -> Result<(), BrokerStoreError> {
        if self.a2a_path.exists() {
            return Ok(());
        }
        self.write_a2a_state(&serde_json::json!({"版": 1, "connections": []}))
    }

    fn ensure_host_file_exists(&self) -> Result<(), BrokerStoreError> {
        if self.host_path.exists() {
            return Ok(());
        }
        self.write_host_state(&serde_json::json!({"版": 1, "hosts": []}))
    }

    fn ensure_adapter_file_exists(&self) -> Result<(), BrokerStoreError> {
        if self.adapter_path.exists() {
            return Ok(());
        }
        self.write_adapter_state(&serde_json::json!({"版": 1, "adapters": []}))
    }

    fn ensure_file_exists(&self, path: &Path) -> Result<(), BrokerStoreError> {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map(|_| ())
            .map_err(|error| {
                BrokerStoreError::Io(format!(
                    "broker store file {}の初期化に失敗: {error}",
                    path.display()
                ))
            })
    }

    fn load_audit_log(&self) -> Result<BrokerAuditLog, BrokerStoreError> {
        let file = File::open(&self.audit_path).map_err(|error| {
            BrokerStoreError::Io(format!("broker audit storeのopenに失敗: {error}"))
        })?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for (index, line) in reader.lines().enumerate() {
            let line = line.map_err(|error| {
                BrokerStoreError::MalformedAuditState(format!(
                    "broker audit eventのline {}の読取りに失敗: {error}",
                    index + 1
                ))
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let event: BrokerAuditEvent = serde_json::from_str(&line).map_err(|error| {
                BrokerStoreError::MalformedAuditState(format!(
                    "broker audit eventのline {}がmalformed: {error}",
                    index + 1
                ))
            })?;
            events.push(event);
        }
        BrokerAuditLog::from_verified_events(events).map_err(BrokerStoreError::TamperedAuditState)
    }

    fn load_replay_nonces(
        &self,
        now_epoch_seconds: i64,
    ) -> Result<HashMap<String, i64>, BrokerStoreError> {
        let file = File::open(&self.replay_path).map_err(|error| {
            BrokerStoreError::Io(format!("broker replay storeのopenに失敗: {error}"))
        })?;
        let reader = BufReader::new(file);
        let mut nonces = BTreeMap::new();
        for (index, line) in reader.lines().enumerate() {
            let line = line.map_err(|error| {
                BrokerStoreError::MalformedReplayState(format!(
                    "replay nonceのline {}の読取りに失敗: {error}",
                    index + 1
                ))
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let record: ReplayNonceRecord = serde_json::from_str(&line).map_err(|error| {
                BrokerStoreError::MalformedReplayState(format!(
                    "replay nonceのline {}がmalformed: {error}",
                    index + 1
                ))
            })?;
            if record.nonce.trim().is_empty() {
                return Err(BrokerStoreError::MalformedReplayState(format!(
                    "replay nonceのline {}が空である",
                    index + 1
                )));
            }
            let recorded_at = record
                .recorded_at_epoch_seconds
                .unwrap_or(now_epoch_seconds);
            if nonce_is_retained(recorded_at, now_epoch_seconds) {
                nonces.insert(record.nonce, recorded_at);
            }
        }
        while nonces.len() > MAX_REPLAY_NONCE_RECORDS {
            let Some(oldest_key) = nonces
                .iter()
                .min_by_key(|(_, recorded_at)| *recorded_at)
                .map(|(nonce, _)| nonce.clone())
            else {
                break;
            };
            nonces.remove(&oldest_key);
        }
        Ok(nonces.into_iter().collect())
    }

    fn compact_replay_nonces(&self, nonces: &HashMap<String, i64>) -> Result<(), BrokerStoreError> {
        let mut records: Vec<_> = nonces.iter().collect();
        records.sort_by(|left, right| left.1.cmp(right.1).then_with(|| left.0.cmp(right.0)));
        let mut serialized = String::new();
        for (nonce, recorded_at) in records {
            let line = serde_json::to_string(&ReplayNonceRecord {
                nonce: nonce.to_string(),
                recorded_at_epoch_seconds: Some(*recorded_at),
            })
            .map_err(|error| {
                BrokerStoreError::MalformedReplayState(format!(
                    "compact済みreplay nonceのserializeに失敗: {error}"
                ))
            })?;
            serialized.push_str(&line);
            serialized.push('\n');
        }
        atomic_write(&self.replay_path, serialized.as_bytes()).map_err(|error| {
            BrokerStoreError::Io(format!("replay nonce storeのcompactに失敗: {error}"))
        })
    }

    fn load_existing_session_if_present(&self) -> Result<(), BrokerStoreError> {
        if !self.session_path.exists() {
            return Ok(());
        }
        let raw = fs::read_to_string(&self.session_path).map_err(|error| {
            BrokerStoreError::MalformedSessionState(format!(
                "broker session stateの読取りに失敗: {error}"
            ))
        })?;
        if raw.trim().is_empty() {
            return Err(BrokerStoreError::MalformedSessionState(
                "broker session stateが空である".to_string(),
            ));
        }
        let record: SessionRecord = serde_json::from_str(&raw).map_err(|error| {
            BrokerStoreError::MalformedSessionState(format!(
                "broker session stateがmalformed: {error}"
            ))
        })?;
        if record.session_id.trim().is_empty() || record.state != "active" {
            return Err(BrokerStoreError::MalformedSessionState(
                "broker session stateがinvalidである".to_string(),
            ));
        }
        Ok(())
    }

    fn write_session(&self, session_id: &str) -> Result<(), BrokerStoreError> {
        let record = SessionRecord {
            session_id: session_id.to_string(),
            state: "active".to_string(),
            evidence_source: "LIVE_RUNTIME".to_string(),
        };
        let serialized = serde_json::to_string_pretty(&record).map_err(|error| {
            BrokerStoreError::MalformedSessionState(format!(
                "broker session stateのserializeに失敗: {error}"
            ))
        })?;
        atomic_write(&self.session_path, serialized.as_bytes()).map_err(|error| {
            BrokerStoreError::Io(format!("broker session stateの書込みに失敗: {error}"))
        })
    }

    fn verify_audit_anchor(&self, audit_log: &BrokerAuditLog) -> Result<(), BrokerStoreError> {
        if !self.audit_anchor_path.exists() {
            return if audit_log.events().is_empty() {
                Ok(())
            } else {
                Err(BrokerStoreError::TamperedAuditState(
                    "空でないaudit logにbroker audit anchorがない".to_string(),
                ))
            };
        }
        let raw = fs::read_to_string(&self.audit_anchor_path).map_err(|error| {
            BrokerStoreError::MalformedAuditState(format!(
                "broker audit anchorの読取りに失敗: {error}"
            ))
        })?;
        let record: AuditAnchorRecord = serde_json::from_str(&raw).map_err(|error| {
            BrokerStoreError::MalformedAuditState(format!(
                "broker audit anchorがmalformed: {error}"
            ))
        })?;
        let expected = self.build_audit_anchor(audit_log);
        if record != expected {
            return Err(BrokerStoreError::TamperedAuditState(
                "broker audit anchor HMACがaudit headと一致しない".to_string(),
            ));
        }
        Ok(())
    }

    fn write_audit_anchor(&self, audit_log: &BrokerAuditLog) -> Result<(), BrokerStoreError> {
        let record = self.build_audit_anchor(audit_log);
        let serialized = serde_json::to_string_pretty(&record).map_err(|error| {
            BrokerStoreError::MalformedAuditState(format!(
                "broker audit anchorのserializeに失敗: {error}"
            ))
        })?;
        atomic_write(&self.audit_anchor_path, serialized.as_bytes()).map_err(|error| {
            BrokerStoreError::Io(format!("broker audit anchorの書込みに失敗: {error}"))
        })
    }

    fn build_audit_anchor(&self, audit_log: &BrokerAuditLog) -> AuditAnchorRecord {
        let event_count = audit_log.events().len();
        let head_event_hash = audit_log
            .events()
            .last()
            .map(|event| event.event_hash.clone());
        let input = format!(
            "version=1|event_count={event_count}|head_event_hash={}",
            head_event_hash.as_deref().unwrap_or("")
        );
        AuditAnchorRecord {
            version: 1,
            event_count,
            head_event_hash,
            anchor_hmac: hmac_sha256_tagged(&self.audit_anchor_key, input.as_bytes()),
        }
    }
}

fn append_jsonl_line(path: &Path, serialized: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(serialized.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_data()
}

fn agent_task_scratch_state_hmac(key: &[u8], state: &Value) -> String {
    let input = format!("version=1|state={state}");
    hmac_sha256_tagged(key, input.as_bytes())
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary_path = path.with_extension("tmp");
    {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary_path)?;
        file.write_all(bytes)?;
        file.sync_data()?;
    }
    fs::rename(&temporary_path, path)
}

fn validate_first_run_configuration(bytes: &[u8]) -> Result<(), BrokerStoreError> {
    if bytes.is_empty() || bytes.len() > MAX_FIRST_RUN_CONFIGURATION_BYTES {
        return Err(BrokerStoreError::MalformedFirstRunConfiguration(
            "初回設定fileが空または上限超過である".to_string(),
        ));
    }
    let document: FirstRunConfigurationDocument = serde_json::from_slice(bytes).map_err(|_| {
        BrokerStoreError::MalformedFirstRunConfiguration("初回設定fileのJSON構造が不正".to_string())
    })?;
    if document.version != 1
        || document.product != "D4 Pocket"
        || document.ui_preferences.theme != "system"
        || document.ui_preferences.density != "compact"
        || document.ui_preferences.locale != "ja-JP"
    {
        return Err(BrokerStoreError::MalformedFirstRunConfiguration(
            "初回設定fileが固定既定値contractに適合しない".to_string(),
        ));
    }
    Ok(())
}

fn nonce_is_retained(recorded_at_epoch_seconds: i64, now_epoch_seconds: i64) -> bool {
    now_epoch_seconds.saturating_sub(recorded_at_epoch_seconds) <= REPLAY_NONCE_RETENTION_SECONDS
}

fn load_or_create_anchor_key(path: &Path) -> Result<Vec<u8>, BrokerStoreError> {
    if path.exists() {
        let raw = fs::read_to_string(path).map_err(|error| {
            BrokerStoreError::MalformedAuditState(format!(
                "broker audit anchor keyの読取りに失敗: {error}"
            ))
        })?;
        return hex::decode(raw.trim()).map_err(|error| {
            BrokerStoreError::MalformedAuditState(format!(
                "broker audit anchor keyがmalformed: {error}"
            ))
        });
    }
    let mut key = vec![0u8; 32];
    getrandom::getrandom(&mut key).map_err(|error| {
        BrokerStoreError::Io(format!("broker audit anchor keyの生成に失敗: {error}"))
    })?;
    let encoded = hex::encode(&key);
    {
        let mut options = OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path).map_err(|error| {
            BrokerStoreError::Io(format!("broker audit anchor keyの作成に失敗: {error}"))
        })?;
        file.write_all(encoded.as_bytes()).map_err(|error| {
            BrokerStoreError::Io(format!("broker audit anchor keyの書込みに失敗: {error}"))
        })?;
        file.sync_data().map_err(|error| {
            BrokerStoreError::Io(format!("broker audit anchor keyのsyncに失敗: {error}"))
        })?;
    }
    Ok(key)
}

fn current_epoch_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod first_run_configuration_tests {
    use super::*;

    fn temp_store(label: &str) -> (PathBuf, BrokerPersistentStore) {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "gui-shell-first-run-{label}-{}-{unique}",
            std::process::id()
        ));
        let (store, _) = BrokerPersistentStore::open_or_create(&root, "first-run-test")
            .expect("永続storeを初期化する");
        (root, store)
    }

    #[test]
    fn fixed_default_configuration_is_create_only_and_idempotent() {
        let (root, store) = temp_store("create-only");
        let (first_bytes, first_created) = store
            .initialize_first_run_configuration()
            .expect("初回設定を生成する");
        assert!(first_created);
        validate_first_run_configuration(&first_bytes).expect("生成設定を検証する");
        let target = root.join("first_run_configuration.json");
        assert_eq!(fs::read(&target).unwrap(), first_bytes);

        let before = fs::read(&target).unwrap();
        let (second_bytes, second_created) = store
            .initialize_first_run_configuration()
            .expect("既存設定を検証する");
        assert!(!second_created);
        assert_eq!(second_bytes, before);
        assert_eq!(fs::read(&target).unwrap(), before);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn malformed_or_modified_configuration_is_preserved_and_rejected() {
        let (root, store) = temp_store("preserve-invalid");
        let target = root.join("first_run_configuration.json");
        let original = br#"{"version":1,"product":"D4 Pocket","ui_preferences":{"theme":"dark","density":"compact","locale":"ja-JP"}}"#;
        fs::write(&target, original).unwrap();
        assert!(store.initialize_first_run_configuration().is_err());
        assert_eq!(fs::read(&target).unwrap(), original);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unknown_configuration_fields_are_rejected_without_replacement() {
        let (root, store) = temp_store("unknown-field");
        let target = root.join("first_run_configuration.json");
        let original = br#"{"version":1,"product":"D4 Pocket","ui_preferences":{"theme":"system","density":"compact","locale":"ja-JP"},"permission":"filesystem.write"}"#;
        fs::write(&target, original).unwrap();
        assert!(store.initialize_first_run_configuration().is_err());
        assert_eq!(fs::read(&target).unwrap(), original);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn oversized_configuration_is_rejected_without_replacement() {
        let (root, store) = temp_store("oversized");
        let target = root.join("first_run_configuration.json");
        let original = vec![b'x'; MAX_FIRST_RUN_CONFIGURATION_BYTES + 1];
        fs::write(&target, &original).unwrap();
        assert!(store.initialize_first_run_configuration().is_err());
        assert_eq!(fs::read(&target).unwrap(), original);
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod setup_doctor_report_tests {
    use super::*;

    #[test]
    fn setup_doctor_report_is_bounded_and_only_latest_report_is_retained() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "gui-shell-setup-doctor-store-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let (store, _) = BrokerPersistentStore::open_or_create(&root, "setup-doctor-test")
            .expect("永続storeを作成する");
        let target = root.join("setup_doctor_report.json");

        store
            .write_setup_doctor_report(br#"{"version":1,"status":"warning"}"#)
            .unwrap();
        assert_eq!(
            fs::read(&target).unwrap(),
            br#"{"version":1,"status":"warning"}"#
        );
        store
            .write_setup_doctor_report(br#"{"version":1,"status":"unknown"}"#)
            .unwrap();
        assert_eq!(
            fs::read(&target).unwrap(),
            br#"{"version":1,"status":"unknown"}"#
        );
        assert_eq!(
            fs::read_dir(&root)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".setup_doctor_report-"))
                .count(),
            0
        );
        assert!(store
            .write_setup_doctor_report(&vec![0; MAX_SETUP_DOCTOR_REPORT_BYTES + 1])
            .is_err());
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod update_trust_store_tests {
    use super::*;

    fn temp_store(label: &str) -> (PathBuf, BrokerPersistentStore) {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "gui-shell-update-trust-{label}-{}-{unique}",
            std::process::id()
        ));
        let (store, _) = BrokerPersistentStore::open_or_create(&root, "update-trust-test")
            .expect("永続storeを初期化する");
        (root, store)
    }

    #[test]
    fn legacy_v1_and_empty_v2_trust_remain_unconfigured() {
        let (root, store) = temp_store("compatibility");
        let path = root.join("update_trust.json");
        for trust in [
            serde_json::json!({
                "版": 1,
                "algorithm": "Ed25519",
                "public_key_der_hex": null,
                "public_key_fingerprint": null
            }),
            serde_json::json!({
                "版": 2,
                "algorithm": "Ed25519",
                "public_key_der_hex": null,
                "public_key_fingerprint": null,
                "package_sources": []
            }),
        ] {
            fs::write(&path, serde_json::to_vec(&trust).unwrap()).unwrap();
            assert!(store.load_update_trust().unwrap().is_none());
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn v2_requires_source_field_and_unconfigured_trust_cannot_enable_a_source() {
        let (root, store) = temp_store("source-requirements");
        let path = root.join("update_trust.json");
        fs::write(
            &path,
            r#"{"版":2,"algorithm":"Ed25519","public_key_der_hex":null,"public_key_fingerprint":null}"#.as_bytes(),
        )
        .unwrap();
        assert!(store.load_update_trust().is_err());

        let trust_with_source = serde_json::json!({
            "版": 2,
            "algorithm": "Ed25519",
            "public_key_der_hex": null,
            "public_key_fingerprint": null,
            "package_sources": [{
                "channel": "stable",
                "base_url": "https://updates.example.invalid/d4/stable"
            }]
        });
        fs::write(&path, serde_json::to_vec(&trust_with_source).unwrap()).unwrap();
        assert!(store.load_update_trust().is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_fields_and_oversized_trust_are_rejected() {
        let (root, store) = temp_store("bounded-unique");
        let path = root.join("update_trust.json");
        fs::write(
            &path,
            r#"{"版":2,"版":1,"algorithm":"Ed25519","public_key_der_hex":null,"public_key_fingerprint":null,"package_sources":[]}"#.as_bytes(),
        )
        .unwrap();
        assert!(store.load_update_trust().is_err());

        fs::write(&path, vec![b' '; MAX_UPDATE_TRUST_BYTES + 1]).unwrap();
        assert!(store.load_update_trust().is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
