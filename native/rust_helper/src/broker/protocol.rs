use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use unicode_normalization::UnicodeNormalization;

use crate::broker::audit::{BrokerAuditEvent, BrokerAuditLog};

const EVIDENCE_SOURCE_LIVE_RUNTIME: &str = "LIVE_RUNTIME";
const EVIDENCE_SOURCE_INTERNAL_STATE: &str = "INTERNAL_STATE";
const BROKER_ID: &str = "gui-shell-rust-broker";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerOperation {
    Health,
    Shutdown,
    CommandEnvelope,
}

impl BrokerOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            BrokerOperation::Health => "health",
            BrokerOperation::Shutdown => "shutdown",
            BrokerOperation::CommandEnvelope => "command_envelope",
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
        })
    }

    pub fn health(request_id: &str, nonce: &str) -> Self {
        Self {
            request_id: Some(request_id.to_string()),
            session_id: None,
            operation: Some(BrokerOperation::Health),
            payload_hash: Some(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
            ),
            nonce: Some(nonce.to_string()),
            issued_at: Some("2026-06-01T00:00:00Z".to_string()),
            metadata: vec![],
            metadata_present: true,
        }
    }

    pub fn shutdown(request_id: &str, session_id: &str, nonce: &str) -> Self {
        Self {
            request_id: Some(request_id.to_string()),
            session_id: Some(session_id.to_string()),
            operation: Some(BrokerOperation::Shutdown),
            payload_hash: Some(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
            ),
            nonce: Some(nonce.to_string()),
            issued_at: Some("2026-06-01T00:00:00Z".to_string()),
            metadata: vec![],
            metadata_present: true,
        }
    }

    pub fn command_envelope(request_id: &str, session_id: &str, nonce: &str) -> Self {
        Self {
            request_id: Some(request_id.to_string()),
            session_id: Some(session_id.to_string()),
            operation: Some(BrokerOperation::CommandEnvelope),
            payload_hash: Some(
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_string(),
            ),
            nonce: Some(nonce.to_string()),
            issued_at: Some("2026-06-01T00:00:00Z".to_string()),
            metadata: vec![],
            metadata_present: true,
        }
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
    seen_nonces: HashSet<String>,
    audit_log: BrokerAuditLog,
    shutdown_requested: bool,
}

impl Broker {
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            seen_nonces: HashSet::new(),
            audit_log: BrokerAuditLog::default(),
            shutdown_requested: false,
        }
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

        if !is_tagged_sha256(envelope.payload_hash.as_deref().unwrap_or("")) {
            return self.reject(
                &request_id,
                &operation,
                "broker_payload_hash_invalid",
                "payload_hash must be tagged sha256",
                true,
            );
        }

        if envelope.operation != Some(BrokerOperation::Health)
            && envelope.session_id.as_deref() != Some(self.session_id.as_str())
        {
            return self.reject(
                &request_id,
                &operation,
                "broker_stale_session",
                "broker session is missing or stale",
                true,
            );
        }

        let nonce = envelope.nonce.clone().unwrap_or_default();
        if self.seen_nonces.contains(&nonce) {
            return self.reject(
                &request_id,
                &operation,
                "broker_replay_detected",
                "broker request nonce was replayed",
                true,
            );
        }

        if metadata_attempts_authority(&envelope.metadata) {
            self.seen_nonces.insert(nonce);
            return self.reject(
                &request_id,
                &operation,
                "broker_authority_metadata_rejected",
                "broker metadata attempted to carry authority",
                true,
            );
        }

        self.seen_nonces.insert(nonce);

        match envelope.operation.unwrap() {
            BrokerOperation::Health => self.accept_health(&request_id),
            BrokerOperation::Shutdown => self.accept_shutdown(&request_id),
            BrokerOperation::CommandEnvelope => self.suspend_command(&request_id),
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

    fn accept_health(&mut self, request_id: &str) -> BrokerResponse {
        let audit_event = self.audit_log.append(
            request_id,
            BrokerOperation::Health.as_str(),
            "accepted",
            "health status returned",
            EVIDENCE_SOURCE_LIVE_RUNTIME,
        );
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: BrokerOperation::Health.as_str().to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_LIVE_RUNTIME.to_string(),
            audit_event_id: audit_event.event_id,
            error: None,
            health: Some(BrokerHealth {
                broker_id: BROKER_ID.to_string(),
                status: "ready".to_string(),
                boundary_role: "rust_security_broker_candidate".to_string(),
                authority_cutover_status: "not_active".to_string(),
                command_dispatch_enabled: false,
                audit_append_enabled: true,
                audit_persistence: self.audit_log.persistence_scope().to_string(),
                replay_persistence: "in_memory_session_only".to_string(),
                evidence_source: EVIDENCE_SOURCE_LIVE_RUNTIME.to_string(),
            }),
            shutdown_requested: false,
        }
    }

    fn accept_shutdown(&mut self, request_id: &str) -> BrokerResponse {
        self.shutdown_requested = true;
        let audit_event = self.audit_log.append(
            request_id,
            BrokerOperation::Shutdown.as_str(),
            "accepted",
            "shutdown requested",
            EVIDENCE_SOURCE_LIVE_RUNTIME,
        );
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: BrokerOperation::Shutdown.as_str().to_string(),
            status: BrokerStatus::Accepted,
            evidence_source: EVIDENCE_SOURCE_LIVE_RUNTIME.to_string(),
            audit_event_id: audit_event.event_id,
            error: None,
            health: None,
            shutdown_requested: true,
        }
    }

    fn suspend_command(&mut self, request_id: &str) -> BrokerResponse {
        let audit_event = self.audit_log.append(
            request_id,
            BrokerOperation::CommandEnvelope.as_str(),
            "suspended",
            "external command dispatch disabled in broker skeleton",
            EVIDENCE_SOURCE_INTERNAL_STATE,
        );
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
        let audit_event = self.audit_log.append(
            request_id,
            operation,
            "rejected",
            code,
            EVIDENCE_SOURCE_INTERNAL_STATE,
        );
        BrokerResponse {
            request_id: request_id.to_string(),
            operation: operation.to_string(),
            status: BrokerStatus::Rejected,
            evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
            audit_event_id: audit_event.event_id,
            error: Some(error(code, message, recoverable)),
            health: None,
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

fn is_tagged_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value
            .as_bytes()
            .iter()
            .skip(7)
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
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

fn normalize_key(value: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broker_health_is_accepted_and_audited() {
        let mut broker = Broker::new("session-1");
        let response = broker.handle(BrokerRequestEnvelope::health("request-1", "nonce-1"));
        assert_eq!(response.status, BrokerStatus::Accepted);
        let health = response.health.unwrap();
        assert_eq!(health.boundary_role, "rust_security_broker_candidate");
        assert_eq!(health.authority_cutover_status, "not_active");
        assert_eq!(health.audit_persistence, "in_memory_skeleton");
        assert_eq!(health.replay_persistence, "in_memory_session_only");
        assert_eq!(broker.audit_events().len(), 1);
        assert_eq!(broker.audit_events()[0].decision, "accepted");
    }

    #[test]
    fn json_health_request_is_accepted_and_serialized() {
        let mut broker = Broker::new("session-1");
        let response = broker.handle_json(
            r#"{
                "request_id": "json-request-1",
                "operation": "health",
                "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
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
        assert!(encoded.contains(r#""shutdown_requested":false"#));
    }

    #[test]
    fn invalid_json_is_rejected_and_audited() {
        let mut broker = Broker::new("session-1");
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
        let mut broker = Broker::new("session-1");
        let response = broker.handle_json(
            r#"{
                "request_id": "json-request-1",
                "operation": "health",
                "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
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
        let mut broker = Broker::new("session-1");
        let response = broker.handle_json(
            r#"{
                "request_id": "json-request-1",
                "operation": "health",
                "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
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
        let mut broker = Broker::new("session-1");
        let response = broker.handle(BrokerRequestEnvelope {
            request_id: None,
            session_id: None,
            operation: Some(BrokerOperation::Health),
            payload_hash: Some(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
            ),
            nonce: Some("nonce-1".to_string()),
            issued_at: Some("2026-06-01T00:00:00Z".to_string()),
            metadata: vec![],
            metadata_present: true,
        });
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.unwrap().code,
            "broker_request_malformed".to_string()
        );
        assert_eq!(broker.audit_events().len(), 1);
    }

    #[test]
    fn replayed_nonce_is_rejected_and_audited() {
        let mut broker = Broker::new("session-1");
        let first = broker.handle(BrokerRequestEnvelope::health("request-1", "nonce-1"));
        let second = broker.handle(BrokerRequestEnvelope::health("request-2", "nonce-1"));
        assert_eq!(first.status, BrokerStatus::Accepted);
        assert_eq!(second.status, BrokerStatus::Rejected);
        assert_eq!(second.error.unwrap().code, "broker_replay_detected");
        assert_eq!(broker.audit_events().len(), 2);
    }

    #[test]
    fn stale_session_is_rejected_and_audited() {
        let mut broker = Broker::new("session-1");
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
        let mut broker = Broker::new("session-1");
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
        let mut broker = Broker::new("session-1");
        let response = broker.handle_json(
            r#"{
                "request_id": "json-request-1",
                "operation": "health",
                "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
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
        let mut broker = Broker::new("session-1");
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
        let mut broker = Broker::new("session-1");
        let value_only = broker.handle_json(
            r#"{
                "request_id": "json-request-1",
                "operation": "health",
                "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
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
                "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
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
    fn command_envelope_is_suspended_without_dispatch() {
        let mut broker = Broker::new("session-1");
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
        assert_eq!(broker.audit_events()[0].decision, "suspended");
    }

    #[test]
    fn shutdown_sets_lifecycle_flag() {
        let mut broker = Broker::new("session-1");
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
