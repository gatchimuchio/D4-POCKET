use crate::audit_hash::sha256_tagged;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerAuditEvent {
    pub event_id: String,
    pub request_id: String,
    pub operation: String,
    pub decision: String,
    pub reason: String,
    pub evidence_source: String,
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
    ) -> BrokerAuditEvent {
        let previous_event_hash = self.events.last().map(|event| event.event_hash.clone());
        let event_id = format!("broker-audit-{}", self.events.len() + 1);
        let hash_input = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            event_id,
            request_id,
            operation,
            decision,
            reason,
            evidence_source,
            previous_event_hash.as_deref().unwrap_or("")
        );
        let event_hash = sha256_tagged(hash_input.as_bytes());
        let event = BrokerAuditEvent {
            event_id,
            request_id: request_id.to_string(),
            operation: operation.to_string(),
            decision: decision.to_string(),
            reason: reason.to_string(),
            evidence_source: evidence_source.to_string(),
            previous_event_hash,
            event_hash,
        };
        self.events.push(event.clone());
        event
    }

    pub fn events(&self) -> &[BrokerAuditEvent] {
        &self.events
    }
}
