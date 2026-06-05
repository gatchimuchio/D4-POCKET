use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Map, Value};

use crate::audit_hash::sha256_tagged;
use crate::broker::protocol::{metadata_attempts_authority_value, normalize_authority_token};

const NON_AUTHORITY_SOURCES: [&str; 16] = [
    "adapter_metadata",
    "cache",
    "diagnostics",
    "external_metadata",
    "generated_config",
    "generated_output",
    "gui_state",
    "history",
    "local_ui_state",
    "memory",
    "metadata",
    "model_output",
    "previous_state",
    "tool_output",
    "tool_response",
    "ui_state",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerAuthorityRegistry {
    state: Value,
}

impl BrokerAuthorityRegistry {
    pub fn production_default() -> Self {
        Self {
            state: json!({
                "runtimes": [{
                    "runtime_id": "gui_shell_rust_broker",
                    "issuer": "gui-shell-rust-broker",
                    "authority_source": "rust_security_broker"
                }],
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
                    "decision": "deny",
                    "source": "broker_internal_policy"
                }],
                "approvals": [],
                "recovery_actions": [{
                    "recovery_id": "recover-command-dispatch",
                    "runtime_id": "gui_shell_rust_broker"
                }]
            }),
        }
    }

    fn state(&self) -> &Value {
        &self.state
    }
}

pub fn evaluate_authority(payload: &Value) -> Value {
    let action = payload.get("action").unwrap_or(&Value::Null);
    let operation = action
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    evaluate_authority_from_state(
        payload.get("state").unwrap_or(&Value::Null),
        action,
        operation,
        EvaluationMode::Fixture,
    )
}

pub fn evaluate_broker_authority(registry: &BrokerAuthorityRegistry, payload: &Value) -> Value {
    let action = payload.get("action").unwrap_or(payload);
    let operation = action
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let mut caller_errors = Vec::new();
    if payload.get("state").is_some() {
        caller_errors.push(shell_error(
            "caller_state_rejected",
            "production authority evaluation does not accept caller-supplied state",
            operation,
        ));
    }
    if action.get("audit_event").is_some() {
        caller_errors.push(shell_error(
            "caller_audit_mapping_rejected",
            "production authority evaluation emits broker audit and does not accept caller audit mappings",
            operation,
        ));
    }
    if let Some(source) = action.get("authority_source").and_then(Value::as_str) {
        caller_errors.push(shell_error(
            "caller_authority_source_rejected",
            &format!("{source} cannot grant authority from a caller request"),
            operation,
        ));
    }
    for source in NON_AUTHORITY_SOURCES {
        let source_flag = format!("{source}_grants_authority");
        if action.get(&source_flag).and_then(Value::as_bool) == Some(true) {
            caller_errors.push(shell_error(
                "caller_authority_source_rejected",
                &format!("{source} cannot grant authority"),
                operation,
            ));
        }
    }

    let mut result = evaluate_authority_from_state(
        registry.state(),
        action,
        operation,
        EvaluationMode::Production,
    );
    let result_errors = result
        .get_mut("errors")
        .and_then(Value::as_array_mut)
        .expect("authority result errors must be an array");
    result_errors.splice(0..0, caller_errors);
    let allowed = result_errors.is_empty();
    result["allowed"] = Value::Bool(allowed);
    result["decision"] = Value::String(if allowed { "authorized" } else { "denied" }.to_string());
    result["authority_source"] = Value::String("rust_security_broker".to_string());
    result["issuer"] = Value::String("gui-shell-rust-broker".to_string());
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvaluationMode {
    Fixture,
    Production,
}

fn evaluate_authority_from_state(
    state: &Value,
    action: &Value,
    operation: &str,
    mode: EvaluationMode,
) -> Value {
    let mut errors = Vec::new();

    let runtime_id = action.get("runtime_id").and_then(Value::as_str);
    if let Some(runtime_id) = runtime_id {
        if !contains_id(state, "runtimes", "runtime_id", runtime_id) {
            errors.push(shell_error(
                "unknown_runtime",
                &format!("unknown runtime: {runtime_id}"),
                operation,
            ));
        }
    } else {
        errors.push(shell_error(
            "unknown_runtime",
            "runtime_id is required",
            operation,
        ));
    }

    let capability_id = action.get("capability_id").and_then(Value::as_str);
    if let Some(capability_id) = capability_id {
        if let Some(capability) = find_record(state, "capabilities", "capability_id", capability_id)
        {
            if runtime_id.is_some()
                && capability.get("runtime_id").and_then(Value::as_str) != runtime_id
            {
                errors.push(shell_error(
                    "relation_mismatch",
                    "capability does not belong to requested runtime",
                    operation,
                ));
            }
            if capability
                .get("operations")
                .and_then(Value::as_array)
                .is_some_and(|operations| {
                    !operations
                        .iter()
                        .any(|candidate| candidate.as_str() == Some(operation))
                })
            {
                errors.push(shell_error(
                    "relation_mismatch",
                    "capability does not authorize requested operation",
                    operation,
                ));
            }
        } else {
            errors.push(shell_error(
                "unknown_capability",
                &format!("unknown capability: {capability_id}"),
                operation,
            ));
        }
    } else {
        errors.push(shell_error(
            "unknown_capability",
            "unknown capability: null",
            operation,
        ));
    }

    let permission_id = action.get("permission_id").and_then(Value::as_str);
    let permission =
        permission_id.and_then(|id| find_record(state, "permissions", "permission_id", id));
    match (permission_id, permission) {
        (None, _) | (_, None) => errors.push(shell_error(
            "unknown_permission",
            &format!("unknown permission: {}", permission_id.unwrap_or("null")),
            operation,
        )),
        (Some(_), Some(permission)) => {
            let decision = permission
                .get("decision")
                .and_then(Value::as_str)
                .unwrap_or("deny");
            if !matches!(decision, "allow" | "approved") {
                errors.push(shell_error(
                    "permission_denied",
                    &format!("permission decision is not allow or approved: {decision}"),
                    operation,
                ));
            }
            if let Some(capability_id) = capability_id {
                if permission.get("capability_id").and_then(Value::as_str) != Some(capability_id) {
                    errors.push(shell_error(
                        "permission_denied",
                        "permission does not authorize the requested capability",
                        operation,
                    ));
                }
            }
            for field in ["runtime_id", "operation", "target_scope"] {
                if action.get(field).and_then(Value::as_str)
                    != permission.get(field).and_then(Value::as_str)
                {
                    errors.push(shell_error(
                        "relation_mismatch",
                        &format!("permission {field} does not match requested action"),
                        operation,
                    ));
                }
            }
        }
    }

    match action.get("approval_id").and_then(Value::as_str) {
        None => errors.push(shell_error(
            "approval_missing",
            "approval_id is required",
            operation,
        )),
        Some(approval_id) => match find_record(state, "approvals", "approval_id", approval_id) {
            None => errors.push(shell_error(
                "approval_missing",
                &format!("unknown approval: {approval_id}"),
                operation,
            )),
            Some(approval)
                if approval.get("status").and_then(Value::as_str) != Some("approved") =>
            {
                errors.push(shell_error(
                    "approval_not_valid",
                    &format!("approval is not approved: {approval_id}"),
                    operation,
                ));
            }
            Some(approval) => {
                for field in ["runtime_id", "operation", "target_scope"] {
                    if action.get(field).and_then(Value::as_str)
                        != approval.get(field).and_then(Value::as_str)
                    {
                        errors.push(shell_error(
                            "relation_mismatch",
                            &format!("approval {field} does not match requested action"),
                            operation,
                        ));
                    }
                }
                if action_has_payload(action) {
                    let expected_hash =
                        canonical_hash(action.get("payload").unwrap_or(&Value::Null));
                    if approval.get("payload_hash").and_then(Value::as_str)
                        != Some(expected_hash.as_str())
                    {
                        errors.push(shell_error(
                            "payload_hash_mismatch",
                            "approval payload_hash does not match canonical action payload",
                            operation,
                        ));
                    }
                }
            }
        },
    }

    if mode == EvaluationMode::Fixture {
        match action.get("audit_event") {
            Some(Value::Object(audit_event)) => {
                if !audit_event
                    .get("event_id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| !id.is_empty())
                {
                    errors.push(shell_error(
                        "audit_mapping_missing",
                        "audit_event.event_id is required",
                        operation,
                    ));
                }
                if action_has_payload(action)
                    && !audit_event
                        .get("payload_hash")
                        .and_then(Value::as_str)
                        .is_some_and(|hash| !hash.is_empty())
                {
                    errors.push(shell_error(
                        "audit_mapping_missing",
                        "audit_event.payload_hash is required when payload exists",
                        operation,
                    ));
                }
                let stored_audit = audit_event
                    .get("event_id")
                    .and_then(Value::as_str)
                    .and_then(|id| find_record(state, "audit_events", "event_id", id));
                if stored_audit.is_none() {
                    errors.push(shell_error(
                        "audit_mapping_missing",
                        "audit_event is not present in broker-owned audit store",
                        operation,
                    ));
                }
                if action_has_payload(action) {
                    let expected_hash =
                        canonical_hash(action.get("payload").unwrap_or(&Value::Null));
                    if audit_event.get("payload_hash").and_then(Value::as_str)
                        != Some(expected_hash.as_str())
                        || stored_audit
                            .and_then(|audit| audit.get("payload_hash").and_then(Value::as_str))
                            != Some(expected_hash.as_str())
                    {
                        errors.push(shell_error(
                            "payload_hash_mismatch",
                            "audit payload_hash does not match canonical action payload",
                            operation,
                        ));
                    }
                }
                if stored_audit
                    .and_then(|audit| audit.get("action").and_then(Value::as_str))
                    .is_some_and(|action_name| action_name != operation)
                {
                    errors.push(shell_error(
                        "relation_mismatch",
                        "audit action does not match requested operation",
                        operation,
                    ));
                }
            }
            _ => errors.push(shell_error(
                "audit_mapping_missing",
                "audit_event is required",
                operation,
            )),
        }
    }

    let recovery_action = action.get("recovery_action");
    match recovery_action {
        Some(Value::Object(recovery))
            if recovery
                .get("recovery_id")
                .and_then(Value::as_str)
                .is_some() =>
        {
            let recovery_id = recovery.get("recovery_id").and_then(Value::as_str).unwrap();
            if let Some(stored_recovery) =
                find_record(state, "recovery_actions", "recovery_id", recovery_id)
            {
                for field in ["runtime_id", "operation"] {
                    if action.get(field).and_then(Value::as_str)
                        != stored_recovery.get(field).and_then(Value::as_str)
                    {
                        errors.push(shell_error(
                            "relation_mismatch",
                            &format!("recovery {field} does not match requested action"),
                            operation,
                        ));
                    }
                }
            } else {
                errors.push(shell_error(
                    "recovery_mapping_missing",
                    &format!("unknown recovery action: {recovery_id}"),
                    operation,
                ));
            }
        }
        _ => errors.push(shell_error(
            "recovery_mapping_missing",
            "recovery_action.recovery_id is required",
            operation,
        )),
    }

    if metadata_attempts_authority_value(action.get("adapter_metadata").unwrap_or(&Value::Null)) {
        errors.push(shell_error(
            "adapter_metadata_escalation_attempt",
            "adapter metadata attempted to claim authority",
            operation,
        ));
    }

    if mode == EvaluationMode::Fixture {
        for source in NON_AUTHORITY_SOURCES {
            let source_flag = format!("{source}_grants_authority");
            if action.get("authority_source").and_then(Value::as_str) == Some(source)
                || action.get(&source_flag).and_then(Value::as_bool) == Some(true)
            {
                errors.push(shell_error(
                    "non_authority_source_attempt",
                    &format!("{source} cannot grant authority"),
                    operation,
                ));
            }
        }
    }

    let required_recovery = if errors.is_empty() {
        Value::Null
    } else {
        recovery_action.cloned().unwrap_or(Value::Null)
    };

    json!({
        "allowed": errors.is_empty(),
        "errors": errors,
        "required_recovery": required_recovery,
        "audit_required": true
    })
}

pub fn edit_approval(payload: &Value) -> Value {
    let approval = payload.get("approval").cloned().unwrap_or(Value::Null);
    let field = payload.get("field").and_then(Value::as_str).unwrap_or("");
    let value = payload.get("value").cloned().unwrap_or(Value::Null);
    let Some(mut approval_object) = approval.as_object().cloned() else {
        return json!({"ok": false, "error": "approval is required"});
    };
    if !can_edit(&approval_object, field) {
        return json!({
            "ok": false,
            "error": format!("field is not editable: {field}")
        });
    }
    let mut full_payload = approval_object
        .get("full_payload")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    full_payload.insert(field.to_string(), value);
    let full_payload_value = Value::Object(full_payload);
    approval_object.insert("full_payload".to_string(), full_payload_value.clone());
    approval_object.insert(
        "payload_hash".to_string(),
        Value::String(canonical_hash(&full_payload_value)),
    );
    approval_object.insert(
        "status".to_string(),
        Value::String("requires_validation".to_string()),
    );
    json!({"ok": true, "approval": Value::Object(approval_object)})
}

pub fn project_approval_content(approval: &Value) -> Value {
    let visibility = approval
        .get("content_visibility")
        .and_then(Value::as_str)
        .unwrap_or("");
    match visibility {
        "none" => json!({}),
        "hash_only" => {
            json!({"payload_hash": approval.get("payload_hash").cloned().unwrap_or(Value::Null)})
        }
        "summary" => {
            json!({"summary": approval.get("summary").cloned().unwrap_or_else(|| Value::String(String::new()))})
        }
        "redacted" => {
            json!({"redacted_payload": approval.get("redacted_payload").cloned().unwrap_or_else(|| json!({}))})
        }
        "full" => {
            json!({"full_payload": approval.get("full_payload").cloned().unwrap_or_else(|| json!({}))})
        }
        _ => json!({"error": format!("unknown content visibility: {visibility}")}),
    }
}

pub fn verify_audit_chain(events: &Value) -> Value {
    let Some(events) = events.as_array() else {
        return json!({"ok": false, "event_count": 0, "latest_event_hash": null, "errors": ["audit events must be an array"]});
    };
    let mut previous = Value::Null;
    let mut latest = Value::Null;
    let mut errors = Vec::new();
    let mut seen_event_ids = BTreeSet::new();
    for (index, event) in events.iter().enumerate() {
        let Some(object) = event.as_object() else {
            errors.push(format!("event {index} is not an object"));
            continue;
        };
        if let Some(event_id) = object.get("event_id").and_then(Value::as_str) {
            if !seen_event_ids.insert(event_id.to_string()) {
                errors.push(format!("event {index} duplicate event_id {event_id}"));
            }
        } else {
            errors.push(format!("event {index} missing event_id"));
        }
        if object
            .get("previous_event_hash")
            .cloned()
            .unwrap_or(Value::Null)
            != previous
        {
            errors.push(format!("event {index} previous hash mismatch"));
        }
        let mut expected_object = object.clone();
        expected_object.remove("event_hash");
        expected_object.insert("previous_event_hash".to_string(), previous.clone());
        let expected_hash = canonical_hash(&Value::Object(expected_object));
        if object.get("event_hash").and_then(Value::as_str) != Some(expected_hash.as_str()) {
            errors.push(format!("event {index} hash mismatch"));
        }
        latest = object.get("event_hash").cloned().unwrap_or(Value::Null);
        previous = latest.clone();
    }
    json!({
        "ok": errors.is_empty(),
        "event_count": events.len(),
        "latest_event_hash": latest,
        "errors": errors
    })
}

pub fn normalize_inbound_payload(payload: &Value) -> Value {
    let normalized_payload = normalize_payload(payload);
    let stripped_payload = strip_authority_keys(payload);
    let key_findings = authority_keys_in(payload, "");
    let value_findings = authority_values_in(&stripped_payload, "");
    let collision_findings = normalization_collisions_in(payload, "");
    json!({
        "raw_payload": payload,
        "normalized_payload": normalized_payload,
        "stripped_payload": stripped_payload,
        "quarantined": !key_findings.is_empty() || !value_findings.is_empty() || !collision_findings.is_empty(),
        "authority_key_findings": key_findings,
        "authority_value_findings": value_findings,
        "normalization_collision_findings": collision_findings,
        "audit_event": {
            "event_type": if key_findings.is_empty() && value_findings.is_empty() && collision_findings.is_empty() { "normalization.pass" } else { "normalization.quarantine" },
            "authority_key_count": key_findings.len(),
            "authority_value_count": value_findings.len(),
            "normalization_collision_count": collision_findings.len(),
            "raw_payload_preserved": true
        }
    })
}

pub fn canonical_hash(payload: &Value) -> String {
    sha256_tagged(canonical_json(payload).as_bytes())
}

fn shell_error(code: &str, message: &str, operation: &str) -> Value {
    json!({
        "code": code,
        "message": message,
        "operation": operation,
        "recoverable": true,
        "recovery_hint": recovery_hint(code)
    })
}

fn recovery_hint(code: &str) -> &'static str {
    match code {
        "unknown_runtime" => "Register the runtime before routing this operation.",
        "unknown_capability" => "Register the capability in Shell Core before use.",
        "unknown_permission" => "Record an explicit permission decision before use.",
        "permission_denied" => "Request or grant permission through an authority source.",
        "approval_missing" => "Create an approval and wait for an approved state.",
        "approval_not_valid" => "Revalidate or approve the current approval request.",
        "audit_mapping_missing" => "Attach an AuditEvent with an event_id and required payload_hash.",
        "recovery_mapping_missing" => "Attach a RecoveryAction with a recovery_id.",
        "adapter_metadata_escalation_attempt" => "Remove authority claims from adapter metadata.",
        "non_authority_source_attempt" => {
            "Use an authority source; generated output, tool output, metadata, UI state, memory, cache, previous_state, and local_ui_state cannot grant authority."
        }
        _ => "",
    }
}

fn contains_id(state: &Value, collection: &str, id_field: &str, id: &str) -> bool {
    find_record(state, collection, id_field, id).is_some()
}

fn find_record<'a>(
    state: &'a Value,
    collection: &str,
    id_field: &str,
    id: &str,
) -> Option<&'a Value> {
    state
        .get(collection)
        .and_then(Value::as_array)
        .and_then(|records| {
            records
                .iter()
                .find(|record| record.get(id_field).and_then(Value::as_str) == Some(id))
        })
}

fn action_has_payload(action: &Value) -> bool {
    ["payload", "full_payload", "redacted_payload"]
        .iter()
        .any(|key| action.get(*key).is_some())
}

fn can_edit(approval: &Map<String, Value>, field: &str) -> bool {
    let editable_fields = string_set(approval.get("editable_fields"));
    editable_fields.contains(field) && !protected_fields(approval).contains(field)
}

fn protected_fields(approval: &Map<String, Value>) -> BTreeSet<String> {
    let mut fields = BTreeSet::from([
        "audit_event_id".to_string(),
        "audit_id".to_string(),
        "payload_hash".to_string(),
        "permission_id".to_string(),
        "runtime_id".to_string(),
    ]);
    for key in [
        "authority_fields",
        "sealed_fields",
        "hidden_fields",
        "sacred_fields",
    ] {
        fields.extend(string_set(approval.get(key)));
    }
    fields
}

fn string_set(value: Option<&Value>) -> BTreeSet<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn normalize_payload(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| (normalize_authority_token(key), normalize_payload(value)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(normalize_payload).collect()),
        Value::String(value) => Value::String(value.nfkc().collect::<String>().trim().to_string()),
        _ => value.clone(),
    }
}

fn strip_authority_keys(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .filter_map(|(key, value)| {
                    if canonical_authority_key(&normalize_authority_token(key)).is_some() {
                        None
                    } else {
                        Some((normalize_authority_token(key), strip_authority_keys(value)))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(strip_authority_keys).collect()),
        _ => value.clone(),
    }
}

fn normalization_collisions_in(value: &Value, path: &str) -> Vec<Value> {
    let mut findings = Vec::new();
    match value {
        Value::Object(object) => {
            let mut seen: BTreeMap<String, String> = BTreeMap::new();
            for (key, item) in object {
                let normalized = normalize_authority_token(key);
                let child_path = if path.is_empty() {
                    normalized.clone()
                } else {
                    format!("{path}.{normalized}")
                };
                if let Some(first_key) = seen.get(&normalized) {
                    if first_key != key {
                        findings.push(json!({
                            "path": child_path,
                            "normalized_key": normalized,
                            "first_key": first_key,
                            "colliding_key": key
                        }));
                    }
                } else {
                    seen.insert(normalized, key.clone());
                }
                findings.extend(normalization_collisions_in(item, &child_path));
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                findings.extend(normalization_collisions_in(
                    item,
                    &format!("{path}[{index}]"),
                ));
            }
        }
        _ => {}
    }
    findings
}

fn authority_keys_in(value: &Value, path: &str) -> Vec<Value> {
    let mut findings = Vec::new();
    match value {
        Value::Object(object) => {
            for (key, item) in object {
                let normalized = normalize_authority_token(key);
                let child_path = if path.is_empty() {
                    normalized.clone()
                } else {
                    format!("{path}.{normalized}")
                };
                if let Some(canonical) = canonical_authority_key(&normalized) {
                    findings
                        .push(json!({"path": child_path, "key": key, "canonical_key": canonical}));
                }
                findings.extend(authority_keys_in(item, &child_path));
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                findings.extend(authority_keys_in(item, &format!("{path}[{index}]")));
            }
        }
        _ => {}
    }
    findings
}

fn authority_values_in(value: &Value, path: &str) -> Vec<Value> {
    let mut findings = Vec::new();
    match value {
        Value::Object(object) => {
            for (key, item) in object {
                let normalized = normalize_authority_token(key);
                let child_path = if path.is_empty() {
                    normalized
                } else {
                    format!("{path}.{normalized}")
                };
                findings.extend(authority_values_in(item, &child_path));
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                findings.extend(authority_values_in(item, &format!("{path}[{index}]")));
            }
        }
        Value::String(value) => {
            let normalized = normalize_authority_token(value);
            if matches!(
                normalized.as_str(),
                "admin" | "all" | "approved" | "elevated" | "root"
            ) {
                findings.push(json!({"path": path, "value": value, "canonical_value": normalized}));
            }
        }
        _ => {}
    }
    findings
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

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => serde_json::to_string(value).unwrap(),
        Value::Array(items) => {
            let encoded: Vec<String> = items.iter().map(canonical_json).collect();
            format!("[{}]", encoded.join(","))
        }
        Value::Object(object) => {
            let sorted: BTreeMap<&String, &Value> = object.iter().collect();
            let encoded: Vec<String> = sorted
                .into_iter()
                .map(|(key, value)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap(),
                        canonical_json(value)
                    )
                })
                .collect();
            format!("{{{}}}", encoded.join(","))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{evaluate_authority, NON_AUTHORITY_SOURCES};

    fn authority_payload(source: &str) -> serde_json::Value {
        json!({
            "state": {
                "runtimes": [{"runtime_id": "runtime-1"}],
                "capabilities": [{
                    "capability_id": "filesystem.write",
                    "runtime_id": "runtime-1",
                    "operations": ["filesystem.write"]
                }],
                "permissions": [{
                    "permission_id": "permission-allow",
                    "runtime_id": "runtime-1",
                    "capability_id": "filesystem.write",
                    "operation": "filesystem.write",
                    "target_scope": "workspace",
                    "decision": "allow"
                }],
                "approvals": [{
                    "approval_id": "approval-approved",
                    "runtime_id": "runtime-1",
                    "operation": "filesystem.write",
                    "target_scope": "workspace",
                    "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "status": "approved"
                }],
                "audit_events": [{
                    "event_id": "audit-1",
                    "action": "filesystem.write",
                    "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                }],
                "recovery_actions": [{
                    "recovery_id": "recover-1",
                    "runtime_id": "runtime-1",
                    "operation": "filesystem.write"
                }]
            },
            "action": {
                "operation": "filesystem.write",
                "runtime_id": "runtime-1",
                "capability_id": "filesystem.write",
                "permission_id": "permission-allow",
                "approval_id": "approval-approved",
                "target_scope": "workspace",
                "payload": {"path": "notes/today.md"},
                "audit_event": {
                    "event_id": "audit-1",
                    "payload_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                },
                "recovery_action": {"recovery_id": "recover-1"},
                "adapter_metadata": {"label": "safe"},
                "authority_source": source
            }
        })
    }

    #[test]
    fn non_authority_sources_cannot_grant_authority() {
        for source in NON_AUTHORITY_SOURCES {
            let result = evaluate_authority(&authority_payload(source));
            assert_eq!(result["allowed"], false, "{source} was allowed");
            let error_codes: Vec<&str> = result["errors"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|error| error.get("code").and_then(|code| code.as_str()))
                .collect();
            assert!(
                error_codes.contains(&"non_authority_source_attempt"),
                "{source} was not rejected as a non-authority source"
            );
        }
    }
}

use unicode_normalization::UnicodeNormalization;
