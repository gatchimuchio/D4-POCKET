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
use std::collections::VecDeque;

const VERSION: u64 = 1;
const MAX_SPANS: usize = 1024;
const MAX_RESPONSE_SPANS: usize = 256;
const MAX_TRACE_ID_LENGTH: usize = 128;
const OP_LIST: &str = "観測一覧";

#[derive(Debug, Default)]
pub(crate) struct ObservationCenter {
    next_sequence: u64,
    spans: VecDeque<ObservationSpan>,
}

#[derive(Debug, Clone)]
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
        let trace_id = format!("observation-trace-{sequence}");
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
        let mut spans: Vec<ObservationSpan> = self
            .spans
            .iter()
            .filter(|span| {
                request
                    .trace_id
                    .as_deref()
                    .map(|trace_id| span.trace_id == trace_id)
                    .unwrap_or(true)
            })
            .cloned()
            .collect();
        spans.truncate(limit);
        let span_values: Vec<Value> = spans.iter().map(ObservationSpan::to_value).collect();
        let trace_values = traces_for(&spans);
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

fn traces_for(spans: &[ObservationSpan]) -> Vec<Value> {
    spans
        .iter()
        .map(|span| {
            json!({
                "版": VERSION,
                "TraceID": span.trace_id,
                "SpanID一覧": [span.span_id],
                "件数": 1,
                "証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE,
                "生成源": "broker_audit_finalization",
                "親TraceID": Value::Null
            })
        })
        .collect()
}

fn metrics_for(spans: &[ObservationSpan]) -> Vec<Value> {
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

    fn event(decision: &str, index: usize) -> BrokerAuditEvent {
        BrokerAuditEvent {
            event_id: format!("broker-audit-{index}"),
            request_id: format!("request-{index}"),
            operation: "通知一覧".to_string(),
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
    }
}
