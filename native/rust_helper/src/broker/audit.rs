use crate::audit_hash::sha256_tagged;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokerAuditEvent {
    pub event_id: String,
    pub request_id: String,
    pub operation: String,
    pub decision: String,
    pub reason: String,
    pub evidence_source: String,
    pub payload_hash: String,
    pub previous_event_hash: Option<String>,
    pub event_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BrokerAuditLog {
    events: Vec<BrokerAuditEvent>,
}

impl BrokerAuditLog {
    pub fn persistence_scope(&self) -> &'static str {
        "in_memory_skeleton"
    }

    pub fn append(
        &mut self,
        request_id: &str,
        operation: &str,
        decision: &str,
        reason: &str,
        evidence_source: &str,
        payload_hash: &str,
    ) -> BrokerAuditEvent {
        let previous_event_hash = self.events.last().map(|event| event.event_hash.clone());
        let event = Self::build_event(
            self.events.len() + 1,
            request_id,
            operation,
            decision,
            reason,
            evidence_source,
            payload_hash,
            previous_event_hash,
        );
        self.events.push(event.clone());
        event
    }

    pub fn build_next(
        &self,
        request_id: &str,
        operation: &str,
        decision: &str,
        reason: &str,
        evidence_source: &str,
        payload_hash: &str,
    ) -> BrokerAuditEvent {
        let previous_event_hash = self.events.last().map(|event| event.event_hash.clone());
        Self::build_event(
            self.events.len() + 1,
            request_id,
            operation,
            decision,
            reason,
            evidence_source,
            payload_hash,
            previous_event_hash,
        )
    }

    pub fn push_verified(&mut self, event: BrokerAuditEvent) -> Result<(), String> {
        let expected = self.build_next(
            &event.request_id,
            &event.operation,
            &event.decision,
            &event.reason,
            &event.evidence_source,
            &event.payload_hash,
        );
        if expected != event {
            return Err("broker audit event does not extend the current hash chain".to_string());
        }
        self.events.push(event);
        Ok(())
    }

    pub fn from_verified_events(events: Vec<BrokerAuditEvent>) -> Result<Self, String> {
        let mut log = Self::default();
        for event in events {
            log.push_verified(event)?;
        }
        Ok(log)
    }

    fn build_event(
        event_index: usize,
        request_id: &str,
        operation: &str,
        decision: &str,
        reason: &str,
        evidence_source: &str,
        payload_hash: &str,
        previous_event_hash: Option<String>,
    ) -> BrokerAuditEvent {
        let event_id = format!("broker-audit-{event_index}");
        let hash_input = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            event_id,
            request_id,
            operation,
            decision,
            reason,
            evidence_source,
            payload_hash,
            previous_event_hash.as_deref().unwrap_or("")
        );
        let event_hash = sha256_tagged(hash_input.as_bytes());
        BrokerAuditEvent {
            event_id,
            request_id: request_id.to_string(),
            operation: operation.to_string(),
            decision: decision.to_string(),
            reason: reason.to_string(),
            evidence_source: evidence_source.to_string(),
            payload_hash: payload_hash.to_string(),
            previous_event_hash,
            event_hash,
        }
    }

    pub fn events(&self) -> &[BrokerAuditEvent] {
        &self.events
    }
}
