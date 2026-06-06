use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::audit_hash::hmac_sha256_tagged;
use crate::broker::audit::{BrokerAuditEvent, BrokerAuditLog};

const REPLAY_NONCE_RETENTION_SECONDS: i64 = 24 * 60 * 60;
const MAX_REPLAY_NONCE_RECORDS: usize = 100_000;

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
            audit_anchor_path: root.join("audit_anchor.json"),
            audit_anchor_key_path: root.join("audit_anchor.key"),
            replay_path: root.join("replay_nonces.jsonl"),
            session_path: root.join("session.json"),
            audit_anchor_key: load_or_create_anchor_key(&root.join("audit_anchor.key"))?,
            root,
        };
        store.ensure_file_exists(&store.audit_path)?;
        store.ensure_file_exists(&store.replay_path)?;
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
                "failed to serialize broker audit event: {error}"
            ))
        })?;
        append_jsonl_line(&self.audit_path, &serialized).map_err(|error| {
            BrokerStoreError::Io(format!("failed to append audit event: {error}"))
        })?;
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
                "failed to serialize replay nonce: {error}"
            ))
        })?;
        append_jsonl_line(&self.replay_path, &serialized).map_err(|error| {
            BrokerStoreError::Io(format!("failed to append replay nonce: {error}"))
        })?;
        let nonces = self.load_replay_nonces(recorded_at_epoch_seconds)?;
        self.compact_replay_nonces(&nonces)?;
        Ok(nonces)
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

    fn load_replay_nonces(
        &self,
        now_epoch_seconds: i64,
    ) -> Result<HashMap<String, i64>, BrokerStoreError> {
        let file = File::open(&self.replay_path).map_err(|error| {
            BrokerStoreError::Io(format!("failed to open broker replay store: {error}"))
        })?;
        let reader = BufReader::new(file);
        let mut nonces = BTreeMap::new();
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
                    "failed to serialize compacted replay nonce: {error}"
                ))
            })?;
            serialized.push_str(&line);
            serialized.push('\n');
        }
        atomic_write(&self.replay_path, serialized.as_bytes()).map_err(|error| {
            BrokerStoreError::Io(format!("failed to compact replay nonce store: {error}"))
        })
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
        atomic_write(&self.session_path, serialized.as_bytes()).map_err(|error| {
            BrokerStoreError::Io(format!("failed to write broker session state: {error}"))
        })
    }

    fn verify_audit_anchor(&self, audit_log: &BrokerAuditLog) -> Result<(), BrokerStoreError> {
        if !self.audit_anchor_path.exists() {
            return if audit_log.events().is_empty() {
                Ok(())
            } else {
                Err(BrokerStoreError::TamperedAuditState(
                    "broker audit anchor is missing for non-empty audit log".to_string(),
                ))
            };
        }
        let raw = fs::read_to_string(&self.audit_anchor_path).map_err(|error| {
            BrokerStoreError::MalformedAuditState(format!(
                "failed to read broker audit anchor: {error}"
            ))
        })?;
        let record: AuditAnchorRecord = serde_json::from_str(&raw).map_err(|error| {
            BrokerStoreError::MalformedAuditState(format!("malformed broker audit anchor: {error}"))
        })?;
        let expected = self.build_audit_anchor(audit_log);
        if record != expected {
            return Err(BrokerStoreError::TamperedAuditState(
                "broker audit anchor HMAC does not match audit head".to_string(),
            ));
        }
        Ok(())
    }

    fn write_audit_anchor(&self, audit_log: &BrokerAuditLog) -> Result<(), BrokerStoreError> {
        let record = self.build_audit_anchor(audit_log);
        let serialized = serde_json::to_string_pretty(&record).map_err(|error| {
            BrokerStoreError::MalformedAuditState(format!(
                "failed to serialize broker audit anchor: {error}"
            ))
        })?;
        atomic_write(&self.audit_anchor_path, serialized.as_bytes()).map_err(|error| {
            BrokerStoreError::Io(format!("failed to write broker audit anchor: {error}"))
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

fn nonce_is_retained(recorded_at_epoch_seconds: i64, now_epoch_seconds: i64) -> bool {
    now_epoch_seconds.saturating_sub(recorded_at_epoch_seconds) <= REPLAY_NONCE_RETENTION_SECONDS
}

fn load_or_create_anchor_key(path: &Path) -> Result<Vec<u8>, BrokerStoreError> {
    if path.exists() {
        let raw = fs::read_to_string(path).map_err(|error| {
            BrokerStoreError::MalformedAuditState(format!(
                "failed to read broker audit anchor key: {error}"
            ))
        })?;
        return hex::decode(raw.trim()).map_err(|error| {
            BrokerStoreError::MalformedAuditState(format!(
                "malformed broker audit anchor key: {error}"
            ))
        });
    }
    let mut key = vec![0u8; 32];
    getrandom::getrandom(&mut key).map_err(|error| {
        BrokerStoreError::Io(format!(
            "failed to generate broker audit anchor key: {error}"
        ))
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
            BrokerStoreError::Io(format!("failed to create broker audit anchor key: {error}"))
        })?;
        file.write_all(encoded.as_bytes()).map_err(|error| {
            BrokerStoreError::Io(format!("failed to write broker audit anchor key: {error}"))
        })?;
        file.sync_data().map_err(|error| {
            BrokerStoreError::Io(format!("failed to sync broker audit anchor key: {error}"))
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
