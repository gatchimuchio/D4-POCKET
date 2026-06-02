use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::broker::audit::{BrokerAuditEvent, BrokerAuditLog};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokerStoreError {
    Io(String),
    MalformedAuditState(String),
    TamperedAuditState(String),
    MalformedReplayState(String),
    MalformedSessionState(String),
}

impl BrokerStoreError {
    pub fn message(&self) -> String {
        match self {
            BrokerStoreError::Io(message)
            | BrokerStoreError::MalformedAuditState(message)
            | BrokerStoreError::TamperedAuditState(message)
            | BrokerStoreError::MalformedReplayState(message)
            | BrokerStoreError::MalformedSessionState(message) => message.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerPersistentState {
    pub audit_log: BrokerAuditLog,
    pub seen_nonces: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerPersistentStore {
    root: PathBuf,
    audit_path: PathBuf,
    replay_path: PathBuf,
    session_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ReplayNonceRecord {
    nonce: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SessionRecord {
    session_id: String,
    state: String,
    evidence_source: String,
}

impl BrokerPersistentStore {
    pub fn open_or_create(
        root: impl AsRef<Path>,
        session_id: &str,
    ) -> Result<(Self, BrokerPersistentState), BrokerStoreError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(|error| {
            BrokerStoreError::Io(format!("failed to create broker store directory: {error}"))
        })?;

        let store = Self {
            audit_path: root.join("audit.jsonl"),
            replay_path: root.join("replay_nonces.jsonl"),
            session_path: root.join("session.json"),
            root,
        };
        store.ensure_file_exists(&store.audit_path)?;
        store.ensure_file_exists(&store.replay_path)?;
        let audit_log = store.load_audit_log()?;
        let seen_nonces = store.load_replay_nonces()?;
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
                "failed to serialize broker audit event: {error}"
            ))
        })?;
        append_jsonl_line(&self.audit_path, &serialized)
            .map_err(|error| BrokerStoreError::Io(format!("failed to append audit event: {error}")))
    }

    pub fn append_replay_nonce(&self, nonce: &str) -> Result<(), BrokerStoreError> {
        let serialized = serde_json::to_string(&ReplayNonceRecord {
            nonce: nonce.into(),
        })
        .map_err(|error| {
            BrokerStoreError::MalformedReplayState(format!(
                "failed to serialize replay nonce: {error}"
            ))
        })?;
        append_jsonl_line(&self.replay_path, &serialized).map_err(|error| {
            BrokerStoreError::Io(format!("failed to append replay nonce: {error}"))
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn ensure_file_exists(&self, path: &Path) -> Result<(), BrokerStoreError> {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map(|_| ())
            .map_err(|error| {
                BrokerStoreError::Io(format!(
                    "failed to initialize broker store file {}: {error}",
                    path.display()
                ))
            })
    }

    fn load_audit_log(&self) -> Result<BrokerAuditLog, BrokerStoreError> {
        let file = File::open(&self.audit_path).map_err(|error| {
            BrokerStoreError::Io(format!("failed to open broker audit store: {error}"))
        })?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for (index, line) in reader.lines().enumerate() {
            let line = line.map_err(|error| {
                BrokerStoreError::MalformedAuditState(format!(
                    "failed to read broker audit event line {}: {error}",
                    index + 1
                ))
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let event: BrokerAuditEvent = serde_json::from_str(&line).map_err(|error| {
                BrokerStoreError::MalformedAuditState(format!(
                    "malformed broker audit event line {}: {error}",
                    index + 1
                ))
            })?;
            events.push(event);
        }
        BrokerAuditLog::from_verified_events(events).map_err(BrokerStoreError::TamperedAuditState)
    }

    fn load_replay_nonces(&self) -> Result<HashSet<String>, BrokerStoreError> {
        let file = File::open(&self.replay_path).map_err(|error| {
            BrokerStoreError::Io(format!("failed to open broker replay store: {error}"))
        })?;
        let reader = BufReader::new(file);
        let mut nonces = HashSet::new();
        for (index, line) in reader.lines().enumerate() {
            let line = line.map_err(|error| {
                BrokerStoreError::MalformedReplayState(format!(
                    "failed to read replay nonce line {}: {error}",
                    index + 1
                ))
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let record: ReplayNonceRecord = serde_json::from_str(&line).map_err(|error| {
                BrokerStoreError::MalformedReplayState(format!(
                    "malformed replay nonce line {}: {error}",
                    index + 1
                ))
            })?;
            if record.nonce.trim().is_empty() {
                return Err(BrokerStoreError::MalformedReplayState(format!(
                    "empty replay nonce line {}",
                    index + 1
                )));
            }
            nonces.insert(record.nonce);
        }
        Ok(nonces)
    }

    fn load_existing_session_if_present(&self) -> Result<(), BrokerStoreError> {
        if !self.session_path.exists() {
            return Ok(());
        }
        let raw = fs::read_to_string(&self.session_path).map_err(|error| {
            BrokerStoreError::MalformedSessionState(format!(
                "failed to read broker session state: {error}"
            ))
        })?;
        if raw.trim().is_empty() {
            return Err(BrokerStoreError::MalformedSessionState(
                "broker session state is empty".to_string(),
            ));
        }
        let record: SessionRecord = serde_json::from_str(&raw).map_err(|error| {
            BrokerStoreError::MalformedSessionState(format!(
                "malformed broker session state: {error}"
            ))
        })?;
        if record.session_id.trim().is_empty() || record.state != "active" {
            return Err(BrokerStoreError::MalformedSessionState(
                "broker session state is invalid".to_string(),
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
                "failed to serialize broker session state: {error}"
            ))
        })?;
        let temporary_path = self.session_path.with_extension("json.tmp");
        fs::write(&temporary_path, serialized).map_err(|error| {
            BrokerStoreError::Io(format!("failed to write broker session state: {error}"))
        })?;
        fs::rename(&temporary_path, &self.session_path).map_err(|error| {
            BrokerStoreError::Io(format!("failed to commit broker session state: {error}"))
        })
    }
}

fn append_jsonl_line(path: &Path, serialized: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(serialized.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_data()
}
