//! Brokerが所有するAgent Task scratchのbounded回復記録。
//! 記録にはWorkspace内の直接子名とdirectory identityだけを保持し、本文・資格・絶対pathを保持しない。

use std::sync::{Arc, Mutex};

use cap_fs_ext::DirExt;
use cap_std::fs::Dir;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::store::{BrokerPersistentStore, BrokerStoreError};
use super::workspace_root::DirectoryIdentity;

const MAX_RECORDS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ScratchState {
    Reserved,
    Active,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScratchRecord {
    record_id: String,
    task_id: String,
    runtime_id: String,
    workspace_id: String,
    recovery_binding_hash: String,
    root_device: u64,
    root_file_id: u64,
    scratch_name: String,
    scratch_device: Option<u64>,
    scratch_file_id: Option<u64>,
    state: ScratchState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JournalState {
    version: u8,
    entries: Vec<ScratchRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentTaskScratchJournal {
    inner: Arc<JournalInner>,
}

#[derive(Debug)]
struct JournalInner {
    store: Option<BrokerPersistentStore>,
    state: Mutex<JournalState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecoveryOutcome {
    Removed,
    MissingReconciled,
    ReservedMissingReconciled,
    ReservedPathPreserved,
    IdentityMismatchPreserved,
    RegistrationMismatchPreserved,
    UnsafePathPreserved,
    JournalUpdateFailed,
}

#[derive(Debug, Clone)]
pub struct AgentTaskScratchContext {
    pub(crate) task_id: String,
    pub(crate) runtime_id: String,
    pub(crate) workspace_id: String,
    pub(crate) recovery_binding_hash: String,
    pub(crate) root_identity: DirectoryIdentity,
    pub(crate) journal: AgentTaskScratchJournal,
}

impl AgentTaskScratchJournal {
    pub(crate) fn open(store: BrokerPersistentStore) -> Result<Self, BrokerStoreError> {
        let value = store.load_agent_task_scratch_state()?;
        let state: JournalState = serde_json::from_value(value).map_err(|_| {
            BrokerStoreError::MalformedAgentTaskScratchState(
                "Agent Task scratch回復記録のpayload構造が不正".to_string(),
            )
        })?;
        validate_state(&state).map_err(BrokerStoreError::MalformedAgentTaskScratchState)?;
        Ok(Self {
            inner: Arc::new(JournalInner {
                store: Some(store),
                state: Mutex::new(state),
            }),
        })
    }

    #[cfg(test)]
    pub(crate) fn in_memory() -> Self {
        Self {
            inner: Arc::new(JournalInner {
                store: None,
                state: Mutex::new(JournalState {
                    version: 1,
                    entries: Vec::new(),
                }),
            }),
        }
    }

    pub(crate) fn reserve(
        &self,
        task_id: &str,
        runtime_id: &str,
        workspace_id: &str,
        recovery_binding_hash: &str,
        root_identity: DirectoryIdentity,
        scratch_name: &str,
    ) -> Result<String, &'static str> {
        if !identifier(task_id)
            || !identifier(runtime_id)
            || !identifier(workspace_id)
            || !recovery_binding_hash.starts_with("sha256:")
            || recovery_binding_hash.len() != 71
            || root_identity.file_id == 0
            || !valid_scratch_name(scratch_name)
        {
            return Err("Agent Task scratch回復記録のscopeが不正");
        }
        let record_id = random_record_id()?;
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "Agent Task scratch回復記録が利用不能")?;
        if state.entries.len() >= MAX_RECORDS {
            return Err("Agent Task scratch回復記録の上限に到達");
        }
        state.entries.push(ScratchRecord {
            record_id: record_id.clone(),
            task_id: task_id.to_owned(),
            runtime_id: runtime_id.to_owned(),
            workspace_id: workspace_id.to_owned(),
            recovery_binding_hash: recovery_binding_hash.to_owned(),
            root_device: root_identity.device,
            root_file_id: root_identity.file_id,
            scratch_name: scratch_name.to_owned(),
            scratch_device: None,
            scratch_file_id: None,
            state: ScratchState::Reserved,
        });
        if let Err(error) = self.persist(&state) {
            state.entries.retain(|entry| entry.record_id != record_id);
            return Err(error);
        }
        Ok(record_id)
    }

    pub(crate) fn activate(
        &self,
        record_id: &str,
        scratch_identity: DirectoryIdentity,
    ) -> Result<(), &'static str> {
        if scratch_identity.file_id == 0 {
            return Err("Agent Task用一時領域の実体識別情報を確認できない");
        }
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "Agent Task scratch回復記録が利用不能")?;
        let record = state
            .entries
            .iter_mut()
            .find(|entry| entry.record_id == record_id)
            .ok_or("Agent Task scratch予約が不在")?;
        if record.state != ScratchState::Reserved {
            return Err("Agent Task scratch予約状態が不正");
        }
        record.scratch_device = Some(scratch_identity.device);
        record.scratch_file_id = Some(scratch_identity.file_id);
        record.state = ScratchState::Active;
        self.persist(&state)
    }

    pub(crate) fn complete(&self, record_id: &str) -> Result<(), &'static str> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "Agent Task scratch回復記録が利用不能")?;
        let old_len = state.entries.len();
        let previous = state.entries.clone();
        state.entries.retain(|entry| entry.record_id != record_id);
        if state.entries.len() == old_len {
            return Err("Agent Task scratch回復記録が不在");
        }
        if let Err(error) = self.persist(&state) {
            state.entries = previous;
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn has_pending_workspace(&self, workspace_id: &str) -> bool {
        self.inner
            .state
            .lock()
            .map(|state| {
                state
                    .entries
                    .iter()
                    .any(|entry| entry.workspace_id == workspace_id)
            })
            .unwrap_or(true)
    }

    pub(crate) fn recover_workspace(
        &self,
        runtime_id: &str,
        workspace_id: &str,
        recovery_binding_hash: &str,
        root: &Dir,
        root_identity: DirectoryIdentity,
    ) -> Vec<RecoveryOutcome> {
        let Ok(mut state) = self.inner.state.lock() else {
            return vec![RecoveryOutcome::UnsafePathPreserved];
        };
        let candidates = state
            .entries
            .iter()
            .filter(|entry| entry.workspace_id == workspace_id)
            .cloned()
            .collect::<Vec<_>>();
        let mut outcomes = Vec::with_capacity(candidates.len());
        for record in candidates {
            if record.runtime_id != runtime_id {
                outcomes.push(RecoveryOutcome::RegistrationMismatchPreserved);
                continue;
            }
            if record.recovery_binding_hash != recovery_binding_hash
                || record.root_device != root_identity.device
                || record.root_file_id != root_identity.file_id
            {
                outcomes.push(RecoveryOutcome::RegistrationMismatchPreserved);
                continue;
            }
            if !valid_scratch_name(&record.scratch_name) {
                outcomes.push(RecoveryOutcome::UnsafePathPreserved);
                continue;
            }
            let child = match root.open_dir_nofollow(&record.scratch_name) {
                Ok(child) => child,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    let outcome = if record.state == ScratchState::Reserved {
                        RecoveryOutcome::ReservedMissingReconciled
                    } else {
                        RecoveryOutcome::MissingReconciled
                    };
                    if self
                        .remove_record_and_persist(&mut state, &record.record_id)
                        .is_err()
                    {
                        outcomes.push(RecoveryOutcome::JournalUpdateFailed);
                    } else {
                        outcomes.push(outcome);
                    }
                    continue;
                }
                Err(_) => {
                    outcomes.push(if record.state == ScratchState::Reserved {
                        RecoveryOutcome::ReservedPathPreserved
                    } else {
                        RecoveryOutcome::UnsafePathPreserved
                    });
                    continue;
                }
            };
            if record.state == ScratchState::Reserved {
                outcomes.push(RecoveryOutcome::ReservedPathPreserved);
                continue;
            }
            let Ok(metadata) = child.dir_metadata() else {
                outcomes.push(RecoveryOutcome::UnsafePathPreserved);
                continue;
            };
            let actual = DirectoryIdentity {
                device: cap_fs_ext::MetadataExt::dev(&metadata),
                file_id: cap_fs_ext::MetadataExt::ino(&metadata),
            };
            if Some(actual.device) != record.scratch_device
                || Some(actual.file_id) != record.scratch_file_id
            {
                outcomes.push(RecoveryOutcome::IdentityMismatchPreserved);
                continue;
            }
            if child.remove_open_dir_all().is_err() {
                outcomes.push(RecoveryOutcome::UnsafePathPreserved);
                continue;
            }
            if self
                .remove_record_and_persist(&mut state, &record.record_id)
                .is_err()
            {
                outcomes.push(RecoveryOutcome::JournalUpdateFailed);
            } else {
                outcomes.push(RecoveryOutcome::Removed);
            }
        }
        outcomes
    }

    fn remove_record_and_persist(
        &self,
        state: &mut JournalState,
        record_id: &str,
    ) -> Result<(), &'static str> {
        let previous = state.entries.clone();
        state.entries.retain(|entry| entry.record_id != record_id);
        if state.entries.len() == previous.len() {
            return Err("Agent Task scratch回復記録が不在");
        }
        if let Err(error) = self.persist(state) {
            state.entries = previous;
            return Err(error);
        }
        Ok(())
    }

    fn persist(&self, state: &JournalState) -> Result<(), &'static str> {
        validate_state(state).map_err(|_| "Agent Task scratch回復記録が不正")?;
        if let Some(store) = &self.inner.store {
            store
                .write_agent_task_scratch_state(&json!({
                    "version": state.version,
                    "entries": state.entries,
                }))
                .map_err(|_| "Agent Task scratch回復記録を耐久保存できない")?;
        }
        Ok(())
    }
}

fn validate_state(state: &JournalState) -> Result<(), String> {
    if state.version != 1 || state.entries.len() > MAX_RECORDS {
        return Err("回復記録の版または件数が不正".to_string());
    }
    let mut ids = std::collections::BTreeSet::new();
    for entry in &state.entries {
        if !identifier(&entry.record_id)
            || !identifier(&entry.task_id)
            || !identifier(&entry.runtime_id)
            || !identifier(&entry.workspace_id)
            || entry.recovery_binding_hash.len() != 71
            || !entry.recovery_binding_hash.starts_with("sha256:")
            || entry.root_file_id == 0
            || !valid_scratch_name(&entry.scratch_name)
            || !ids.insert(entry.record_id.as_str())
            || matches!(entry.state, ScratchState::Reserved)
                && (entry.scratch_device.is_some() || entry.scratch_file_id.is_some())
            || matches!(entry.state, ScratchState::Active)
                && (entry.scratch_device.is_none()
                    || entry.scratch_file_id.is_none()
                    || entry.scratch_file_id == Some(0))
        {
            return Err("回復記録entryが不正".to_string());
        }
    }
    Ok(())
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
}

fn valid_scratch_name(name: &str) -> bool {
    name.strip_prefix(".d4p-tmp-").is_some_and(|suffix| {
        suffix.len() == 32
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn random_record_id() -> Result<String, &'static str> {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).map_err(|_| "Agent Task scratch回復IDを生成できない")?;
    Ok(hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cap_std::ambient_authority;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct Fixture {
        root: std::path::PathBuf,
        store: BrokerPersistentStore,
        workspace_path: std::path::PathBuf,
        root_identity: DirectoryIdentity,
    }

    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "d4p-task-scratch-journal-{}-{nonce}",
                std::process::id()
            ));
            let workspace_path = root.join("workspace");
            fs::create_dir_all(&workspace_path).expect("試験用作業領域を作成");
            let store = BrokerPersistentStore::open_or_create(root.join("store"), "fixture")
                .expect("試験用保管領域を開く")
                .0;
            let workspace = Dir::open_ambient_dir(&workspace_path, ambient_authority())
                .expect("試験用作業領域を開く");
            let metadata = workspace.dir_metadata().expect("作業領域の属性を取得");
            let root_identity = DirectoryIdentity {
                device: cap_fs_ext::MetadataExt::dev(&metadata),
                file_id: cap_fs_ext::MetadataExt::ino(&metadata),
            };
            Self {
                root,
                store,
                workspace_path,
                root_identity,
            }
        }

        fn open_workspace(&self) -> Dir {
            Dir::open_ambient_dir(&self.workspace_path, ambient_authority())
                .expect("作業領域を開く")
        }

        fn reserve(&self, journal: &AgentTaskScratchJournal, name: &str) -> String {
            journal
                .reserve(
                    "task-1",
                    "runtime-1",
                    "workspace-1",
                    &format!("sha256:{}", "a".repeat(64)),
                    self.root_identity,
                    name,
                )
                .expect("reserve")
        }

        fn create_active(&self, journal: &AgentTaskScratchJournal, name: &str) -> String {
            let record_id = self.reserve(journal, name);
            let workspace = self.open_workspace();
            workspace.create_dir(name).expect("一時領域を作成");
            let child = workspace.open_dir_nofollow(name).expect("一時領域を開く");
            let metadata = child.dir_metadata().expect("一時領域の属性を取得");
            journal
                .activate(
                    &record_id,
                    DirectoryIdentity {
                        device: cap_fs_ext::MetadataExt::dev(&metadata),
                        file_id: cap_fs_ext::MetadataExt::ino(&metadata),
                    },
                )
                .expect("回復記録を有効化");
            record_id
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn scratch_name(suffix: char) -> String {
        format!(".d4p-tmp-{}{}", "0".repeat(31), suffix)
    }

    #[test]
    fn 永続journalは再起動後も認証され一致するscratchだけを回収する() {
        let fixture = Fixture::new();
        let journal = AgentTaskScratchJournal::open(fixture.store.clone()).expect("回復記録を開く");
        let name = scratch_name('1');
        fixture.create_active(&journal, &name);
        let reopened_store =
            BrokerPersistentStore::open_or_create(fixture.store.root().to_path_buf(), "restarted")
                .expect("保管領域を再度開く")
                .0;
        let reopened = AgentTaskScratchJournal::open(reopened_store).expect("回復記録を再読込");
        let outcomes = reopened.recover_workspace(
            "runtime-1",
            "workspace-1",
            &format!("sha256:{}", "a".repeat(64)),
            &fixture.open_workspace(),
            fixture.root_identity,
        );
        assert_eq!(outcomes, vec![RecoveryOutcome::Removed]);
        assert!(!fixture.workspace_path.join(name).exists());
        assert!(!reopened.has_pending_workspace("workspace-1"));
    }

    #[test]
    fn recovery_binding不一致は既存scratchを削除せず保留する() {
        let fixture = Fixture::new();
        let journal = AgentTaskScratchJournal::in_memory();
        let active_name = scratch_name('2');
        fixture.create_active(&journal, &active_name);

        let outcomes = journal.recover_workspace(
            "runtime-1",
            "workspace-1",
            &format!("sha256:{}", "b".repeat(64)),
            &fixture.open_workspace(),
            fixture.root_identity,
        );
        assert_eq!(outcomes, vec![RecoveryOutcome::RegistrationMismatchPreserved]);
        assert!(fixture.workspace_path.join(&active_name).exists());
        assert!(journal.has_pending_workspace("workspace-1"));
    }

    #[test]
    fn identity未確認のreserved既存pathは削除せず保留する() {
        let fixture = Fixture::new();
        let journal = AgentTaskScratchJournal::in_memory();
        let name = scratch_name('3');
        fixture.reserve(&journal, &name);
        fixture
            .open_workspace()
            .create_dir(&name)
            .expect("未確認の予約先を作成");

        let outcomes = journal.recover_workspace(
            "runtime-1",
            "workspace-1",
            &format!("sha256:{}", "a".repeat(64)),
            &fixture.open_workspace(),
            fixture.root_identity,
        );
        assert_eq!(outcomes, vec![RecoveryOutcome::ReservedPathPreserved]);
        assert!(fixture.workspace_path.join(name).exists());
        assert!(journal.has_pending_workspace("workspace-1"));
    }

    #[test]
    fn root_identity不一致ではactive_scratchを削除しない() {
        let fixture = Fixture::new();
        let journal = AgentTaskScratchJournal::in_memory();
        let name = scratch_name('4');
        fixture.create_active(&journal, &name);
        let outcomes = journal.recover_workspace(
            "runtime-1",
            "workspace-1",
            &format!("sha256:{}", "a".repeat(64)),
            &fixture.open_workspace(),
            DirectoryIdentity {
                device: fixture.root_identity.device,
                file_id: fixture.root_identity.file_id + 1,
            },
        );
        assert_eq!(
            outcomes,
            vec![RecoveryOutcome::RegistrationMismatchPreserved]
        );
        assert!(fixture.workspace_path.join(name).exists());
    }

    #[test]
    fn scratch名が再利用されても別directory_identityは削除しない() {
        let fixture = Fixture::new();
        let journal = AgentTaskScratchJournal::in_memory();
        let name = scratch_name('7');
        fixture.create_active(&journal, &name);
        let replacement = fixture.workspace_path.join("replacement-holder");
        let preserved = fixture.workspace_path.join("preserved-original");
        fs::create_dir(&replacement).expect("置換用フォルダーを作成");
        fs::rename(fixture.workspace_path.join(&name), &preserved).expect("記録対象を退避");
        fs::rename(&replacement, fixture.workspace_path.join(&name)).expect("記録名を再利用");

        let outcomes = journal.recover_workspace(
            "runtime-1",
            "workspace-1",
            &format!("sha256:{}", "a".repeat(64)),
            &fixture.open_workspace(),
            fixture.root_identity,
        );
        assert_eq!(outcomes, vec![RecoveryOutcome::IdentityMismatchPreserved]);
        assert!(fixture.workspace_path.join(name).exists());
        assert!(preserved.exists());
        assert!(journal.has_pending_workspace("workspace-1"));
    }

    #[test]
    fn 認証journalの改竄と本文path秘密値の混入を拒否する() {
        let fixture = Fixture::new();
        let journal = AgentTaskScratchJournal::open(fixture.store.clone()).expect("回復記録を開く");
        let name = scratch_name('5');
        fixture.create_active(&journal, &name);
        let journal_path = fixture
            .store
            .root()
            .join("agent_task_scratch_recovery.json");
        let raw = fs::read_to_string(&journal_path).expect("回復記録を読む");
        assert!(!raw.contains("TASK_PRIVATE_OUTPUT_SENTINEL"));
        assert!(!raw.contains("secret-value"));
        assert!(!raw.contains(&fixture.workspace_path.to_string_lossy().to_string()));
        let tampered = raw.replace("workspace-1", "workspace-x");
        fs::write(&journal_path, tampered).expect("回復記録を改変");
        assert!(matches!(
            AgentTaskScratchJournal::open(fixture.store.clone()),
            Err(BrokerStoreError::TamperedAgentTaskScratchState(_))
        ));
    }

    #[test]
    fn 消失したscratchは記録だけを再照合して解消する() {
        let fixture = Fixture::new();
        let journal = AgentTaskScratchJournal::in_memory();
        let name = scratch_name('6');
        fixture.reserve(&journal, &name);
        let outcomes = journal.recover_workspace(
            "runtime-1",
            "workspace-1",
            &format!("sha256:{}", "a".repeat(64)),
            &fixture.open_workspace(),
            fixture.root_identity,
        );
        assert_eq!(outcomes, vec![RecoveryOutcome::ReservedMissingReconciled]);
        assert!(!journal.has_pending_workspace("workspace-1"));
    }
}
