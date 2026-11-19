//! Broker内部でboundedに保持するSpan、Trace、Metricの観測センター。
//!
//! 観測は性能・挙動・運用状態のための内部状態であり、Auditの責任・安全・証拠を
//! 置き換えない。callerが観測を登録する経路、権限を生成する経路、外部export経路は
//! 提供しない。現段階の観測範囲は、BrokerがAudit eventを確定した処理だけである。
#![allow(non_snake_case)]

use super::audit::BrokerAuditEvent;
use super::protocol::{
    canonical_payload_hash, Broker, BrokerOperation, BrokerResponse, BrokerStatus,
    EVIDENCE_SOURCE_INTERNAL_STATE,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

const VERSION: u64 = 1;
const MAX_SPANS: usize = 1024;
const MAX_RESPONSE_SPANS: usize = 256;
const MAX_TASK_TRACE_BINDINGS: usize = 128;
const MAX_PENDING_FOLLOWUPS: usize = 8;
const PENDING_FOLLOWUP_TTL: Duration = Duration::from_secs(300);
#[cfg(feature = "module_trace_inspector")]
const MAX_TRACE_ID_LENGTH: usize = 128;
const OP_LIST: &str = "観測一覧";

#[derive(Debug, Default)]
pub(crate) struct ObservationCenter {
    next_sequence: u64,
    next_trace_sequence: u64,
    spans: VecDeque<ObservationSpan>,
    task_trace_ids: HashMap<String, String>,
    task_trace_order: VecDeque<String>,
    trace_parent_ids: HashMap<String, String>,
    pending_followups: HashMap<String, PendingFollowup>,
    pending_followup_order: VecDeque<String>,
}

#[derive(Debug)]
struct PendingFollowup {
    instruction_hash: String,
    source_task_id: String,
    expires_at: Instant,
}

#[derive(Debug)]
struct ObservationSpan {
    span_id: String,
    trace_id: String,
    parent_span_id: Option<String>,
    target: String,
    operation: String,
    started_at_epoch_millis: i64,
    ended_at_epoch_millis: i64,
    duration_millis: u64,
    status: String,
    error_kind: Option<String>,
    related_audit_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListRequest {
    #[serde(rename = "版")]
    version: u64,
    #[serde(rename = "上限", default)]
    limit: Option<usize>,
    #[cfg(feature = "module_trace_inspector")]
    #[serde(rename = "TraceID", default)]
    trace_id: Option<String>,
}

impl ObservationCenter {
    /// Audit確定後にだけ呼び出す。失敗したAuditは観測済みとして記録しない。
    pub(crate) fn record_completed_span(
        &mut self,
        event: &BrokerAuditEvent,
        started_at_epoch_millis: i64,
        duration_millis: u64,
    ) {
        self.next_sequence = self.next_sequence.saturating_add(1);
        let sequence = self.next_sequence;
        let span_id = format!("observation-span-{sequence}");
        let task_record = super::execution_history::agent_task_audit_record(event)
            .ok()
            .flatten();
        let trace_id = match task_record.as_ref() {
            Some(record) => {
                let parent_task_id = (record.stage == "start")
                    .then(|| self.take_followup_parent(&record.instruction_hash))
                    .flatten();
                self.trace_for_task(&record.task_id, parent_task_id.as_deref())
            }
            None => self.next_trace_id(),
        };
        let ended_at_epoch_millis = started_at_epoch_millis
            .saturating_add(i64::try_from(duration_millis).unwrap_or(i64::MAX));
        let error_kind = match event.decision.as_str() {
            "rejected" | "suspended" => Some("operation_not_accepted".to_string()),
            _ => None,
        };
        self.spans.push_back(ObservationSpan {
            span_id,
            trace_id,
            parent_span_id: None,
            target: "Broker".to_string(),
            operation: event.operation.chars().take(128).collect(),
            started_at_epoch_millis,
            ended_at_epoch_millis,
            duration_millis,
            status: event.decision.chars().take(32).collect(),
            error_kind,
            related_audit_id: event.event_id.clone(),
        });
        while self.spans.len() > MAX_SPANS {
            self.spans.pop_front();
        }
    }

    fn response_body(&self, request: &ListRequest) -> Value {
        let limit = request.limit.unwrap_or(MAX_RESPONSE_SPANS);
        let spans = self.spans.iter().rev();
        #[cfg(feature = "module_trace_inspector")]
        let spans = spans.filter(|span| {
            request
                .trace_id
                .as_deref()
                .map(|trace_id| span.trace_id == trace_id)
                .unwrap_or(true)
        });
        let mut spans: Vec<&ObservationSpan> = spans.take(limit).collect();
        // 直近を選び、応答内は従来どおり確定順に並べる。
        spans.reverse();
        let span_values: Vec<Value> = spans.iter().map(|span| span.to_value()).collect();
        let trace_values = traces_for(&spans, &self.trace_parent_ids);
        let metric_values = metrics_for(&spans);
        json!({
            "版": VERSION,
            "Span一覧": span_values,
            "Trace一覧": trace_values,
            "Metric一覧": metric_values,
            "件数": spans.len(),
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
            "観測範囲": "broker_audit_finalization",
            "Auditとの責任分離": "Audit=責任・安全・証拠; Observation=性能・挙動・運用状態",
            "OpenTelemetry export": "unsupported",
            "権限生成": "なし"
        })
    }

    pub(super) fn associate_audit_event_with_task(&mut self, audit_event_id: &str, task_id: &str) {
        if !valid_correlation_id(task_id) {
            return;
        }
        let Some(index) = self.spans.iter().rposition(|span| {
            span.related_audit_id == audit_event_id
                && span.operation == "MCP Tool実行"
                && matches!(span.status.as_str(), "started" | "accepted")
        }) else {
            return;
        };
        let trace_id = self.trace_for_task(task_id, None);
        if let Some(span) = self.spans.get_mut(index) {
            span.trace_id = trace_id;
        }
    }

    pub(super) fn register_followup(
        &mut self,
        result_id: &str,
        instruction_hash: &str,
        source_task_id: &str,
    ) {
        if !valid_result_id(result_id)
            || !valid_tagged_hash(instruction_hash)
            || !valid_correlation_id(source_task_id)
        {
            return;
        }
        self.clear_pending_followup_instruction(instruction_hash);
        self.clear_pending_followup_result(result_id);
        while self.pending_followups.len() >= MAX_PENDING_FOLLOWUPS {
            let Some(oldest) = self.pending_followup_order.pop_front() else {
                break;
            };
            self.pending_followups.remove(&oldest);
        }
        self.pending_followup_order.push_back(result_id.to_owned());
        self.pending_followups.insert(
            result_id.to_owned(),
            PendingFollowup {
                instruction_hash: instruction_hash.to_owned(),
                source_task_id: source_task_id.to_owned(),
                expires_at: Instant::now() + PENDING_FOLLOWUP_TTL,
            },
        );
    }

    pub(super) fn clear_pending_followup_result(&mut self, result_id: &str) {
        self.pending_followups.remove(result_id);
        self.pending_followup_order
            .retain(|pending_id| pending_id != result_id);
    }

    pub(super) fn clear_pending_followup_instruction(&mut self, instruction_hash: &str) {
        let matching = self
            .pending_followups
            .iter()
            .filter(|(_, pending)| pending.instruction_hash == instruction_hash)
            .map(|(result_id, _)| result_id.clone())
            .collect::<Vec<_>>();
        for result_id in matching {
            self.clear_pending_followup_result(&result_id);
        }
    }

    fn take_followup_parent(&mut self, instruction_hash: &str) -> Option<String> {
        let now = Instant::now();
        let expired = self
            .pending_followups
            .iter()
            .filter(|(_, pending)| now >= pending.expires_at)
            .map(|(result_id, _)| result_id.clone())
            .collect::<Vec<_>>();
        for result_id in expired {
            self.clear_pending_followup_result(&result_id);
        }
        let matching = self
            .pending_followups
            .iter()
            .filter(|(_, pending)| pending.instruction_hash == instruction_hash)
            .map(|(result_id, pending)| (result_id.clone(), pending.source_task_id.clone()))
            .collect::<Vec<_>>();
        let mut unique_parents = matching
            .iter()
            .map(|(_, task_id)| task_id.clone())
            .collect::<Vec<_>>();
        unique_parents.sort_unstable();
        unique_parents.dedup();
        for (result_id, _) in matching {
            self.clear_pending_followup_result(&result_id);
        }
        (unique_parents.len() == 1).then(|| unique_parents.remove(0))
    }

    fn trace_for_task(&mut self, task_id: &str, parent_task_id: Option<&str>) -> String {
        if let Some(trace_id) = self.task_trace_ids.get(task_id).cloned() {
            self.task_trace_order.retain(|known| known != task_id);
            self.task_trace_order.push_back(task_id.to_owned());
            return trace_id;
        }
        let trace_id = self.next_trace_id();
        if let Some(parent_trace_id) = parent_task_id
            .filter(|parent| *parent != task_id)
            .and_then(|parent| self.task_trace_ids.get(parent))
            .filter(|parent_trace_id| **parent_trace_id != trace_id)
        {
            self.trace_parent_ids
                .insert(trace_id.clone(), parent_trace_id.clone());
        }
        while self.task_trace_ids.len() >= MAX_TASK_TRACE_BINDINGS {
            let Some(oldest) = self.task_trace_order.pop_front() else {
                break;
            };
            if let Some(evicted_trace_id) = self.task_trace_ids.remove(&oldest) {
                self.trace_parent_ids.remove(&evicted_trace_id);
            }
        }
        self.task_trace_order.push_back(task_id.to_owned());
        self.task_trace_ids
            .insert(task_id.to_owned(), trace_id.clone());
        trace_id
    }

    fn next_trace_id(&mut self) -> String {
        self.next_trace_sequence = self.next_trace_sequence.saturating_add(1);
        format!("observation-trace-{}", self.next_trace_sequence)
    }
}

impl ObservationSpan {
    fn to_value(&self) -> Value {
        json!({
            "版": VERSION,
            "SpanID": self.span_id,
            "TraceID": self.trace_id,
            "親SpanID": self.parent_span_id,
            "対象": self.target,
            "操作": self.operation,
            "開始EpochMillis": self.started_at_epoch_millis,
            "終了EpochMillis": self.ended_at_epoch_millis,
            "所要Millis": self.duration_millis,
            "状態": self.status,
            "エラー分類": self.error_kind,
            "関連監査ID": self.related_audit_id,
            "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE
        })
    }
}

fn traces_for(
    spans: &[&ObservationSpan],
    parent_trace_ids: &HashMap<String, String>,
) -> Vec<Value> {
    let mut indexes = HashMap::<String, usize>::new();
    let mut grouped = Vec::<(String, Vec<String>)>::new();
    for span in spans {
        let index = match indexes.get(&span.trace_id) {
            Some(index) => *index,
            None => {
                let index = grouped.len();
                indexes.insert(span.trace_id.clone(), index);
                grouped.push((span.trace_id.clone(), Vec::new()));
                index
            }
        };
        grouped[index].1.push(span.span_id.clone());
    }
    grouped
        .into_iter()
        .map(|(trace_id, span_ids)| {
            let count = span_ids.len();
            json!({
                "版": VERSION,
                "TraceID": trace_id,
                "SpanID一覧": span_ids,
                "件数": count,
                "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
                "生成源": "broker_audit_finalization",
                "親TraceID": parent_trace_ids.get(&trace_id)
            })
        })
        .collect()
}

fn valid_correlation_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.:-".contains(&byte))
}

fn valid_tagged_hash(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_result_id(value: &str) -> bool {
    value.len() == "mcp-agent-result-".len() + 32
        && value.starts_with("mcp-agent-result-")
        && value["mcp-agent-result-".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn metrics_for(spans: &[&ObservationSpan]) -> Vec<Value> {
    let count = spans.len() as u64;
    let error_count = spans
        .iter()
        .filter(|span| span.error_kind.is_some())
        .count() as u64;
    let total_duration: u128 = spans
        .iter()
        .map(|span| u128::from(span.duration_millis))
        .sum();
    let average_duration = (!spans.is_empty()).then(|| (total_duration / u128::from(count)) as u64);
    let max_duration = spans.iter().map(|span| span.duration_millis).max();
    let state = if spans.is_empty() {
        "unknown"
    } else {
        "observed"
    };
    vec![
        metric(
            "broker.operation.count",
            "操作観測数",
            count,
            "count",
            count,
            state,
        ),
        metric(
            "broker.operation.error_count",
            "非受理操作観測数",
            error_count,
            "count",
            count,
            state,
        ),
        metric_optional(
            "broker.audit_finalize.duration_ms_avg",
            "Audit確定所要時間平均",
            average_duration,
            "ms",
            count,
            state,
        ),
        metric_optional(
            "broker.audit_finalize.duration_ms_max",
            "Audit確定所要時間最大",
            max_duration,
            "ms",
            count,
            state,
        ),
    ]
}

fn metric(
    metric_id: &str,
    name: &str,
    value: u64,
    unit: &str,
    sample_count: u64,
    state: &str,
) -> Value {
    json!({
        "版": VERSION,
        "MetricID": metric_id,
        "名称": name,
        "値": value,
        "単位": unit,
        "種別": "counter",
        "観測件数": sample_count,
        "状態": state,
        "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE
    })
}

fn metric_optional(
    metric_id: &str,
    name: &str,
    value: Option<u64>,
    unit: &str,
    sample_count: u64,
    state: &str,
) -> Value {
    json!({
        "版": VERSION,
        "MetricID": metric_id,
        "名称": name,
        "値": value,
        "単位": unit,
        "種別": "gauge",
        "観測件数": sample_count,
        "状態": state,
        "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE
    })
}

pub(super) fn dispatch(
    broker: &mut Broker,
    operation: BrokerOperation,
    payload: &Value,
    request_id: &str,
    payload_hash: &str,
) -> BrokerResponse {
    if operation.as_str() != OP_LIST {
        return broker.reject_with_payload_hash(
            request_id,
            operation.as_str(),
            "observation_operation_unknown",
            "観測操作が未定義",
            true,
            payload_hash,
        );
    }
    let request = match serde_json::from_value::<ListRequest>(payload.clone()) {
        Ok(request) if request.version == VERSION => request,
        _ => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_LIST,
                "observation_list_invalid",
                "観測一覧の要求が不正",
                true,
                payload_hash,
            )
        }
    };
    let limit = request.limit.unwrap_or(MAX_RESPONSE_SPANS);
    if !(1..=MAX_RESPONSE_SPANS).contains(&limit) {
        return broker.reject_with_payload_hash(
            request_id,
            OP_LIST,
            "observation_limit_invalid",
            "観測一覧の上限は1から256までである",
            true,
            payload_hash,
        );
    }
    #[cfg(feature = "module_trace_inspector")]
    if request
        .trace_id
        .as_deref()
        .is_some_and(|trace_id| trace_id.is_empty() || trace_id.len() > MAX_TRACE_ID_LENGTH)
    {
        return broker.reject_with_payload_hash(
            request_id,
            OP_LIST,
            "observation_trace_id_invalid",
            "TraceIDの長さが不正",
            true,
            payload_hash,
        );
    }
    let body = broker.observations.response_body(&request);
    let audit_payload_hash = canonical_payload_hash(Some(&body));
    let audit_event = match broker.append_audit(
        request_id,
        OP_LIST,
        "accepted",
        "観測一覧を返却。Auditと責任を分離し権限を生成しない",
        EVIDENCE_SOURCE_INTERNAL_STATE,
        &audit_payload_hash,
    ) {
        Ok(event) => event,
        Err(error) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_LIST,
                "observation_audit_append_failed",
                &error.message(),
            )
        }
    };
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OP_LIST.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
        audit_event_id: audit_event.event_id,
        error: None,
        health: None,
        body: Some(body),
        shutdown_requested: false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::protocol::BrokerRequestEnvelope;
    use super::*;

    fn event_for(
        operation: &str,
        decision: &str,
        index: usize,
        request_id: &str,
    ) -> BrokerAuditEvent {
        BrokerAuditEvent {
            event_id: format!("broker-audit-{index}"),
            request_id: request_id.to_string(),
            operation: operation.to_string(),
            decision: decision.to_string(),
            reason: "秘密の理由を観測へ露出しない".to_string(),
            evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
            payload_hash: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            previous_event_hash: None,
            event_hash: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                .to_string(),
        }
    }

    fn event(decision: &str, index: usize) -> BrokerAuditEvent {
        event_for("通知一覧", decision, index, &format!("request-{index}"))
    }

    fn task_event(
        task_id: &str,
        stage: &str,
        instruction_hash: &str,
        index: usize,
    ) -> BrokerAuditEvent {
        let request_hash = crate::audit_hash::sha256_tagged(format!("request-{index}").as_bytes());
        let record = super::super::execution_history::AgentTaskHistoryAuditRecord {
            version: 1,
            stage: stage.to_string(),
            task_id: task_id.to_string(),
            runtime_id: "runtime-a".to_string(),
            session_id: "session-a".to_string(),
            workspace_id: "workspace-a".to_string(),
            instruction_hash: instruction_hash.to_string(),
            request_hash,
            created_at: 100,
            status: if stage == "start" {
                "running".to_string()
            } else {
                "completed".to_string()
            },
            updated_at: if stage == "start" { 100 } else { 200 },
            result_hash: (stage == "terminal")
                .then(|| crate::audit_hash::sha256_tagged(b"task result")),
            failure_class: None,
            start_audit_event_id: (stage == "terminal").then(|| "broker-audit-1".to_string()),
            buffer_intervention_record_hash: None,
            buffer_intervention_status: None,
            buffer_intervention_audit_event_id: None,
        };
        let (reason, payload_hash) =
            super::super::execution_history::encode_agent_task_audit(&record)
                .expect("valid Task audit marker");
        let mut event = event_for(
            if stage == "start" {
                "AgentTask実行"
            } else {
                "AgentTask状態"
            },
            "recorded",
            index,
            task_id,
        );
        event.reason = reason;
        event.payload_hash = payload_hash;
        event
    }

    #[test]
    fn 完了Spanは内部観測であり監査内容を露出しない() {
        let mut center = ObservationCenter::default();
        center.record_completed_span(&event("accepted", 1), 1_000, 3);
        let span = center
            .spans
            .into_iter()
            .next()
            .expect("観測Spanが生成される");
        let value = span.to_value();
        assert_eq!(value["証拠種別"], "INTERNAL_STATE");
        assert_eq!(value["所要Millis"], 3);
        assert!(value.get("秘密の理由を観測へ露出しない").is_none());
    }

    #[test]
    fn Span保持数はboundedで古い観測を破棄する() {
        let mut center = ObservationCenter::default();
        for index in 1..=(MAX_SPANS + 1) {
            center.record_completed_span(&event("accepted", index), index as i64, 1);
        }
        assert_eq!(center.spans.len(), MAX_SPANS);
        assert_eq!(
            center.spans.front().unwrap().related_audit_id,
            "broker-audit-2"
        );
    }

    #[test]
    fn AgentTaskと検証済みMCPTool結果を同じTraceへ結ぶ() {
        let instruction_hash = crate::audit_hash::sha256_tagged(b"task instruction");
        let mut center = ObservationCenter::default();
        center.record_completed_span(
            &task_event("task-source", "start", &instruction_hash, 1),
            1_000,
            3,
        );
        let task_trace_id = center.spans.back().unwrap().trace_id.clone();

        let tool_started = event_for("MCP Tool実行", "started", 2, "mcp-request");
        center.record_completed_span(&tool_started, 1_010, 5);
        center.associate_audit_event_with_task(&tool_started.event_id, "task-source");

        let tool_result = event_for("MCP Tool実行", "accepted", 3, "mcp-request");
        center.record_completed_span(&tool_result, 1_020, 7);
        center.associate_audit_event_with_task(&tool_result.event_id, "task-source");

        center.record_completed_span(
            &task_event("task-source", "terminal", &instruction_hash, 4),
            1_030,
            2,
        );

        assert!(center
            .spans
            .iter()
            .all(|span| span.trace_id == task_trace_id));
        let body = center.response_body(&ListRequest {
            version: VERSION,
            limit: None,
            trace_id: None,
        });
        assert_eq!(body["Trace一覧"].as_array().unwrap().len(), 1);
        assert_eq!(body["Trace一覧"][0]["件数"], 4);
        assert_eq!(body["Trace一覧"][0]["親TraceID"], Value::Null);
    }

    #[test]
    fn 検証済み一回MCP結果だけをFollowupTraceの親へ結び期限後は結ばない() {
        let source_instruction_hash = crate::audit_hash::sha256_tagged(b"source task");
        let followup_instruction_hash = crate::audit_hash::sha256_tagged(b"effective followup");
        let result_id = "mcp-agent-result-0123456789abcdef0123456789abcdef";
        let mut center = ObservationCenter::default();
        center.record_completed_span(
            &task_event("task-source", "start", &source_instruction_hash, 1),
            1_000,
            3,
        );
        let parent_trace_id = center.spans.back().unwrap().trace_id.clone();
        center.register_followup(result_id, &followup_instruction_hash, "task-source");
        center.record_completed_span(
            &task_event("task-followup", "start", &followup_instruction_hash, 2),
            1_010,
            4,
        );
        let child_trace_id = center.spans.back().unwrap().trace_id.clone();
        let body = center.response_body(&ListRequest {
            version: VERSION,
            limit: None,
            trace_id: None,
        });
        let child_trace = body["Trace一覧"]
            .as_array()
            .unwrap()
            .iter()
            .find(|trace| trace["TraceID"] == child_trace_id)
            .unwrap();
        assert_eq!(child_trace["親TraceID"], parent_trace_id);
        assert!(center.pending_followups.is_empty());

        let mut unlinked = ObservationCenter::default();
        unlinked.record_completed_span(
            &task_event("task-source", "start", &source_instruction_hash, 1),
            1_000,
            3,
        );
        unlinked.register_followup(result_id, &followup_instruction_hash, "task-source");
        unlinked
            .pending_followups
            .get_mut(result_id)
            .expect("pending followup")
            .expires_at = Instant::now() - Duration::from_secs(1);
        unlinked.record_completed_span(
            &task_event("task-followup", "start", &followup_instruction_hash, 2),
            1_010,
            4,
        );
        assert!(unlinked.trace_parent_ids.is_empty());
        assert!(unlinked.pending_followups.is_empty());

        unlinked.register_followup(
            "untrusted-result-id",
            &followup_instruction_hash,
            "task-source",
        );
        assert!(unlinked.pending_followups.is_empty());
    }

    #[test]
    fn Brokerに束縛されたMCP結果だけが実FollowupTaskの親Traceになる() {
        let mut broker = Broker::new("observation-followup-binding-test");
        let result_id = "mcp-agent-result-0123456789abcdef0123456789abcdef";
        let result_hash = crate::audit_hash::sha256_tagged(b"bound MCP result");
        let source_instruction_hash = crate::audit_hash::sha256_tagged(b"source task");
        broker.mcp_agent_results.insert(
            result_id.to_string(),
            super::super::mcp_center::McpAgentResultContext {
                server_id: "mcp-server-a".to_string(),
                tool_id: "tool-a".to_string(),
                source_task_id: "task-source".to_string(),
                source_task_result_hash: source_instruction_hash.clone(),
                agent_runtime_id: "runtime-a".to_string(),
                session_id: "session-a".to_string(),
                workspace_id: "workspace-a".to_string(),
                result_hash: result_hash.clone(),
                tool_result_hash: crate::audit_hash::sha256_tagged(b"tool result"),
                text: zeroize::Zeroizing::new("private fixture result".to_string()),
                expires_at: Instant::now() + Duration::from_secs(60),
            },
        );
        broker.observations.record_completed_span(
            &task_event("task-source", "start", &source_instruction_hash, 1),
            1_000,
            3,
        );
        let source_trace_id = broker.observations.spans.back().unwrap().trace_id.clone();
        let payload = json!({
            "agent_runtime_id": "runtime-a",
            "session_id": "session-a",
            "workspace_id": "workspace-a",
            "instruction": "Use the verified Tool result as data",
            "mcp_result_id": result_id,
            "mcp_result_hash": result_hash,
        });

        let (enriched, consume_id) = super::super::mcp_center::enrich_agent_task_payload(
            &mut broker,
            "AgentTask実行",
            &payload,
        )
        .expect("Broker内で束縛されたTool結果をFollowup Taskへ渡す");
        assert_eq!(consume_id.as_deref(), Some(result_id));
        let effective_instruction = enriched["instruction"].as_str().unwrap();
        let effective_instruction_hash =
            crate::audit_hash::sha256_tagged(effective_instruction.as_bytes());
        let pending = broker
            .observations
            .pending_followups
            .get(result_id)
            .expect("bounded correlation binding");
        assert_eq!(pending.instruction_hash, effective_instruction_hash);
        assert_eq!(pending.source_task_id, "task-source");
        assert!(!format!("{:?}", broker.observations).contains("private fixture result"));

        broker.observations.record_completed_span(
            &task_event("task-followup", "start", &effective_instruction_hash, 2),
            1_010,
            4,
        );
        let followup_trace_id = broker.observations.spans.back().unwrap().trace_id.clone();
        let response = broker.observations.response_body(&ListRequest {
            version: VERSION,
            limit: None,
            trace_id: None,
        });
        let followup_trace = response["Trace一覧"]
            .as_array()
            .unwrap()
            .iter()
            .find(|trace| trace["TraceID"] == followup_trace_id)
            .unwrap();
        assert_eq!(followup_trace["親TraceID"], source_trace_id);
        assert!(!response.to_string().contains("private fixture result"));

        super::super::mcp_center::consume_agent_result_context_after_task(
            &mut broker,
            consume_id.as_deref(),
            true,
        );
        assert!(broker.mcp_agent_results.is_empty());
        assert!(broker.observations.pending_followups.is_empty());
    }

    #[test]
    fn Metricは保持中の実観測だけから算出する() {
        let mut center = ObservationCenter::default();
        center.record_completed_span(&event("accepted", 1), 1_000, 4);
        center.record_completed_span(&event("rejected", 2), 1_000, 8);
        let body = center.response_body(&ListRequest {
            version: VERSION,
            limit: None,
            trace_id: None,
        });
        let metrics = body["Metric一覧"].as_array().unwrap();
        assert_eq!(metrics[0]["値"], 2);
        assert_eq!(metrics[1]["値"], 1);
        assert_eq!(metrics[2]["値"], 6);
        assert_eq!(metrics[3]["値"], 8);
        assert_eq!(body["OpenTelemetry export"], "unsupported");
    }

    #[test]
    fn 空の観測は不明を表しゼロの実測へ偽装しない() {
        let body = ObservationCenter::default().response_body(&ListRequest {
            version: VERSION,
            limit: None,
            trace_id: None,
        });
        let metrics = body["Metric一覧"].as_array().unwrap();
        assert_eq!(metrics[2]["値"], Value::Null);
        assert_eq!(metrics[2]["状態"], "unknown");
    }

    #[test]
    fn 観測一覧は通常Broker経路から内部観測だけを返す() {
        let mut broker = Broker::new("observation-session");
        for index in 1..=9 {
            let decision = if index == 9 { "rejected" } else { "accepted" };
            broker.observations.record_completed_span(
                &event(decision, index),
                index as i64,
                index as u64,
            );
        }
        let issued_at = BrokerRequestEnvelope::current_issued_at();
        let mut request = BrokerRequestEnvelope::command_envelope(
            "observation-request",
            "observation-session",
            "observation-nonce",
        );
        request.issued_at = Some(issued_at);
        request.operation = Some(BrokerOperation::観測一覧);
        request.payload = Some(json!({"版": VERSION, "上限": 8}));
        request.refresh_payload_hash();

        let response = broker.handle(request);
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.expect("観測一覧bodyが返る");
        assert_eq!(body["証拠種別"], "INTERNAL_STATE");
        assert_eq!(body["権限生成"], "なし");
        assert_eq!(body["OpenTelemetry export"], "unsupported");
        let spans = body["Span一覧"].as_array().unwrap();
        assert_eq!(spans.len(), 8);
        assert_eq!(spans[0]["関連監査ID"], "broker-audit-2");
        assert_eq!(spans[7]["関連監査ID"], "broker-audit-9");
        assert_eq!(body["Metric一覧"][1]["値"], 1);
        assert_eq!(body["Metric一覧"][3]["値"], 9);
    }

    #[cfg(feature = "module_trace_inspector")]
    #[test]
    fn Trace指定は該当する直近Spanだけを確定順で返す() {
        let mut center = ObservationCenter::default();
        let instruction_hash = crate::audit_hash::sha256_tagged(b"task instruction");
        for index in [1, 3, 5] {
            let stage = if index == 1 { "start" } else { "terminal" };
            center.record_completed_span(
                &task_event("task-a", stage, &instruction_hash, index),
                1_000,
                3,
            );
            center.record_completed_span(&event("accepted", index + 1), 1_000, 1);
        }
        let trace_id = center.spans.front().unwrap().trace_id.clone();
        let body = center.response_body(&ListRequest {
            version: VERSION,
            limit: Some(2),
            trace_id: Some(trace_id.clone()),
        });
        assert_eq!(body["件数"], 2);
        assert_eq!(body["Trace一覧"][0]["TraceID"], trace_id);
        assert_eq!(
            body["Trace一覧"][0]["SpanID一覧"],
            json!(["observation-span-3", "observation-span-5"])
        );
        assert_eq!(body["Span一覧"][0]["関連監査ID"], "broker-audit-3");
        assert_eq!(body["Span一覧"][1]["関連監査ID"], "broker-audit-5");
    }
}
