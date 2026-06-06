use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

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

const EVIDENCE_SOURCE_LIVE_RUNTIME: &str = "LIVE_RUNTIME";
const EVIDENCE_SOURCE_INTERNAL_STATE: &str = "INTERNAL_STATE";
const BROKER_ID: &str = "gui-shell-rust-broker";
const REQUEST_FRESHNESS_WINDOW_SECONDS: u64 = 300;
const ZERO_PAYLOAD_HASH: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerOperation {
    Health,
    Shutdown,
    CommandEnvelope,
    AuthorityEvaluate,
    AuthorityFixtureEvaluate,
    ApprovalEdit,
    ContentProjection,
    AuditVerify,
    NormalizePayload,
}

impl BrokerOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            BrokerOperation::Health => "health",
            BrokerOperation::Shutdown => "shutdown",
            BrokerOperation::CommandEnvelope => "command_envelope",
            BrokerOperation::AuthorityEvaluate => "authority_evaluate",
            BrokerOperation::AuthorityFixtureEvaluate => "authority_fixture_evaluate",
            BrokerOperation::ApprovalEdit => "approval_edit",
            BrokerOperation::ContentProjection => "content_projection",
            BrokerOperation::AuditVerify => "audit_verify",
            BrokerOperation::NormalizePayload => "normalize_payload",
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

impl BrokerRequestEnvelope {
    pub fn from_json_str(input: &str) -> Result<Self, serde_json::Error> {
        let raw: JsonRequestEnvelope = serde_json::from_str(input)?;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Broker {
    session_id: String,
    seen_nonces: HashMap<String, i64>,
    audit_log: BrokerAuditLog,
    authority_registry: BrokerAuthorityRegistry,
    shutdown_requested: bool,
    current_epoch_seconds_override: Option<i64>,
    state_store: BrokerStateStore,
}

impl Broker {
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            seen_nonces: HashMap::new(),
            audit_log: BrokerAuditLog::default(),
            authority_registry: BrokerAuthorityRegistry::production_default(),
            shutdown_requested: false,
            current_epoch_seconds_override: None,
            state_store: BrokerStateStore::in_memory_skeleton(),
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
        Ok(Self {
            session_id: session_id.to_string(),
            seen_nonces: persistent_state.seen_nonces,
            audit_log: persistent_state.audit_log,
            authority_registry: BrokerAuthorityRegistry::production_default(),
            shutdown_requested: false,
            current_epoch_seconds_override: None,
            state_store: BrokerStateStore::durable_file_store(persistent_store),
        })
    }

    pub fn handle(&mut self, envelope: BrokerRequestEnvelope) -> BrokerResponse {
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

    pub fn handle_json(&mut self, input: &str) -> BrokerResponse {
        match BrokerRequestEnvelope::from_json_str(input) {
            Ok(envelope) => self.handle(envelope),
            Err(_) => self.reject(
                "malformed-request",
                "unknown",
                "broker_request_malformed",
                "broker request JSON failed to parse or included unknown fields",
                true,
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

    fn reject_with_payload_hash(
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

    fn append_audit(
        &mut self,
        request_id: &str,
        operation: &str,
        decision: &str,
        reason: &str,
        evidence_source: &str,
        payload_hash: &str,
    ) -> Result<BrokerAuditEvent, BrokerStoreError> {
        let event = self.audit_log.build_next(
            request_id,
            operation,
            decision,
            reason,
            evidence_source,
            payload_hash,
        );
        self.state_store.append_audit_event(&event)?;
        self.audit_log
            .push_verified(event.clone())
            .map_err(BrokerStoreError::TamperedAuditState)?;
        Ok(event)
    }

    fn record_nonce(&mut self, nonce: &str) -> Result<(), BrokerStoreError> {
        let recorded_at = current_epoch_seconds();
        if let Some(nonces) = self.state_store.append_replay_nonce(nonce, recorded_at)? {
            self.seen_nonces = nonces;
        } else {
            self.seen_nonces.insert(nonce.to_string(), recorded_at);
        }
        Ok(())
    }

    fn audit_store_failed_response(
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

fn canonical_payload_hash(payload: Option<&Value>) -> String {
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
    use serde_json::json;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_broker() -> Broker {
        Broker::new_with_current_epoch_seconds(
            "session-1",
            parse_issued_at_epoch_seconds("2026-06-01T00:00:30Z").unwrap(),
        )
    }

    fn persistence_required_broker() -> Broker {
        let mut broker = Broker::new_requiring_persistence("session-1");
        broker.current_epoch_seconds_override =
            Some(parse_issued_at_epoch_seconds("2026-06-01T00:00:30Z").unwrap());
        broker
    }

    fn temp_store_dir(test_name: &str) -> PathBuf {
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

    fn persistent_test_broker(store_dir: &Path) -> Broker {
        let mut broker = Broker::new_persistent("session-1", store_dir).unwrap();
        broker.current_epoch_seconds_override =
            Some(parse_issued_at_epoch_seconds("2026-06-01T00:00:30Z").unwrap());
        broker
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

    fn authority_error_codes(body: &serde_json::Value) -> Vec<&str> {
        body.get("errors")
            .and_then(serde_json::Value::as_array)
            .unwrap()
            .iter()
            .filter_map(|error| error.get("code").and_then(serde_json::Value::as_str))
            .collect()
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
