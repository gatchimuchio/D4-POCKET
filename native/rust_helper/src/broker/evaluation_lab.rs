//! C5 評価ラボの純粋・決定論的な評価器。
//!
//! このmoduleは、既に公開可能な対話結果projectionと、callerが単調時計から得た
//! latencyだけを入力にする。privateな期待値、本文、JSON Schemaは出力へ含めない。
//! Authority、Permission、Approval、Audit、Recovery、release、security、LLMの
//! 判断は一切持たず、filesystem、process、network、IPCにも到達しない。
//!
//! 正規表現は、線形時間で評価する標準的な`regex` crateだけを用いる。独自実装、
//! 外部process、外部参照は使わない。patternが不正またはresource上限を超える場合は、
//! 推測で成立させず評価器不正として扱う。
//! JSON Schemaは外部参照を解決しない有界subsetだけを扱う。

use regex::RegexBuilder;
use serde_json::{json, Map, Value};

const MAX_EVALUATORS: usize = 64;
const MAX_BODY_CHARS: usize = 65_536;
const MAX_REFERENCES: usize = 64;
const MAX_CAPABILITIES: usize = 64;
const MAX_REFERENCE_CHARS: usize = 2_048;
const MAX_CAPABILITY_CHARS: usize = 256;
const MAX_ROUTE_CHARS: usize = 256;
const MAX_KIND_CHARS: usize = 64;
const MAX_REGEX_CHARS: usize = 8_192;
const MAX_LATENCY_MILLIS: u64 = 86_400_000;
const MAX_JSON_DOCUMENT_BYTES: usize = 65_536;
const MAX_JSON_DEPTH: usize = 16;
const MAX_JSON_COLLECTION_ITEMS: usize = 64;
const MAX_JSON_KEY_CHARS: usize = 256;

/// 一つの評価器またはcase全体の公開判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvaluationVerdict {
    /// 評価可能であり、期待条件を満たした。
    成立,
    /// 評価可能であり、期待条件を満たさなかった。
    不成立,
    /// 必要な公開範囲、入力、または対応実装がない。
    評価不能,
}

impl EvaluationVerdict {
    fn as_public_label(self) -> &'static str {
        match self {
            Self::成立 => "成立",
            Self::不成立 => "不成立",
            Self::評価不能 => "評価不能",
        }
    }
}

/// private evaluator設定を露出しない、一評価器分の公開結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvaluatorEvaluation {
    pub(crate) evaluator_id: String,
    pub(crate) kind: String,
    pub(crate) verdict: EvaluationVerdict,
    pub(crate) reason_code: &'static str,
}

/// 評価器結果のAND集約。権限やrelease判断ではない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaseEvaluation {
    pub(crate) verdict: EvaluationVerdict,
    pub(crate) evaluator_results: Vec<EvaluatorEvaluation>,
}

impl CaseEvaluation {
    /// UIその他へ渡せる、期待値や本文を含まない公開projectionを返す。
    pub(crate) fn public_projection(&self) -> Value {
        let evaluator_results = self
            .evaluator_results
            .iter()
            .map(|result| {
                json!({
                    "評価器ID": result.evaluator_id,
                    "種類": result.kind,
                    "判定": result.verdict.as_public_label(),
                    "理由code": result.reason_code,
                })
            })
            .collect::<Vec<_>>();

        json!({
            "case判定": self.verdict.as_public_label(),
            "評価結果": evaluator_results,
        })
    }
}

/// evaluate_caseの公開projectionだけを必要とするcaller用のshortcut。
pub(crate) fn evaluate_case_public_projection(
    dialogue_projection: &Value,
    latency_millis: Option<u64>,
    evaluator_configs: &Value,
) -> Value {
    evaluate_case(dialogue_projection, latency_millis, evaluator_configs).public_projection()
}

/// 一つのC5 caseを純粋に評価する。
///
/// latency_millisはcallerが単調時計で観測した値だけを渡す。値を得られない場合は
/// Noneとし、wall-clockや自己生成値で補完してはならない。
pub(crate) fn evaluate_case(
    dialogue_projection: &Value,
    latency_millis: Option<u64>,
    evaluator_configs: &Value,
) -> CaseEvaluation {
    let Some(configs) = evaluator_configs.as_array() else {
        return CaseEvaluation {
            verdict: EvaluationVerdict::評価不能,
            evaluator_results: vec![invalid_result("", "", "evaluator_config_not_array")],
        };
    };

    if configs.is_empty() {
        return CaseEvaluation {
            verdict: EvaluationVerdict::評価不能,
            evaluator_results: vec![invalid_result("", "", "evaluator_config_empty")],
        };
    }

    if configs.len() > MAX_EVALUATORS {
        return CaseEvaluation {
            verdict: EvaluationVerdict::評価不能,
            evaluator_results: vec![invalid_result("", "", "evaluator_config_limit_exceeded")],
        };
    }

    let projection = PublicDialogueProjection::parse(dialogue_projection);
    let evaluator_results = configs
        .iter()
        .map(|raw_config| {
            let (safe_id, safe_kind) = safe_config_identity(raw_config);
            let config = match EvaluatorConfig::parse(raw_config) {
                Ok(config) => config,
                Err(reason_code) => return invalid_result(&safe_id, &safe_kind, reason_code),
            };

            if let Err(reason_code) = validate_evaluator_settings(&config) {
                return invalid_result(config.id, config.kind, reason_code);
            }

            match &projection {
                Ok(projection) => evaluate_valid_config(&config, projection, latency_millis),
                Err(reason_code) => invalid_result(config.id, config.kind, *reason_code),
            }
        })
        .collect::<Vec<_>>();

    CaseEvaluation {
        verdict: aggregate_verdict(&evaluator_results),
        evaluator_results,
    }
}

struct PublicDialogueProjection<'a> {
    state: &'a str,
    visibility: &'a str,
    body: &'a str,
    references: &'a [Value],
    capabilities: &'a [Value],
    route: &'a str,
}

impl<'a> PublicDialogueProjection<'a> {
    fn parse(value: &'a Value) -> Result<Self, &'static str> {
        let object = value.as_object().ok_or("dialogue_projection_invalid")?;
        let state = required_string(object, "状態")?;
        let visibility = required_string(object, "表示範囲")?;
        let body = required_string(object, "本文")?;
        let references = object
            .get("参照")
            .and_then(Value::as_array)
            .ok_or("dialogue_projection_invalid")?;
        let capabilities = object
            .get("能力")
            .and_then(Value::as_array)
            .ok_or("dialogue_projection_invalid")?;
        let route = required_string(object, "経路")?;
        let response_hash = required_string(object, "応答hash")?;

        if !valid_status(state)
            || !valid_visibility(visibility)
            || !within_chars(body, MAX_BODY_CHARS)
            || !valid_string_array(references, MAX_REFERENCES, MAX_REFERENCE_CHARS)
            || !valid_string_array(capabilities, MAX_CAPABILITIES, MAX_CAPABILITY_CHARS)
            || !within_chars(route, MAX_ROUTE_CHARS)
            || !valid_response_hash(response_hash)
        {
            return Err("dialogue_projection_invalid");
        }

        Ok(Self {
            state,
            visibility,
            body,
            references,
            capabilities,
            route,
        })
    }
}

struct EvaluatorConfig<'a> {
    id: &'a str,
    kind: &'a str,
    settings: &'a Map<String, Value>,
}

impl<'a> EvaluatorConfig<'a> {
    fn parse(value: &'a Value) -> Result<Self, &'static str> {
        let object = value.as_object().ok_or("evaluator_config_invalid")?;
        if object.len() != 3
            || !object.contains_key("評価器ID")
            || !object.contains_key("種類")
            || !object.contains_key("設定")
        {
            return Err("evaluator_config_invalid");
        }

        let id = required_string(object, "評価器ID").map_err(|_| "evaluator_config_invalid")?;
        let kind = required_string(object, "種類").map_err(|_| "evaluator_config_invalid")?;
        let settings = object
            .get("設定")
            .and_then(Value::as_object)
            .ok_or("evaluator_config_invalid")?;

        if !valid_evaluator_id(id) || !valid_kind_label(kind) {
            return Err("evaluator_config_invalid");
        }

        Ok(Self { id, kind, settings })
    }
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
) -> Result<&'a str, &'static str> {
    object
        .get(field)
        .and_then(Value::as_str)
        .ok_or("dialogue_projection_invalid")
}

fn safe_config_identity(value: &Value) -> (String, String) {
    let Some(object) = value.as_object() else {
        return (String::new(), String::new());
    };
    let id = object
        .get("評価器ID")
        .and_then(Value::as_str)
        .filter(|value| valid_evaluator_id(value))
        .unwrap_or_default()
        .to_owned();
    let kind = object
        .get("種類")
        .and_then(Value::as_str)
        .filter(|value| valid_kind_label(value))
        .unwrap_or_default()
        .to_owned();
    (id, kind)
}

fn invalid_result(id: &str, kind: &str, reason_code: &'static str) -> EvaluatorEvaluation {
    EvaluatorEvaluation {
        evaluator_id: id.to_owned(),
        kind: kind.to_owned(),
        verdict: EvaluationVerdict::評価不能,
        reason_code: normalize_reason_code(reason_code),
    }
}

fn result(
    config: &EvaluatorConfig<'_>,
    verdict: EvaluationVerdict,
    reason_code: &'static str,
) -> EvaluatorEvaluation {
    EvaluatorEvaluation {
        evaluator_id: config.id.to_owned(),
        kind: config.kind.to_owned(),
        verdict,
        reason_code: normalize_reason_code(reason_code),
    }
}

/// 公開contractが許可するreason codeだけを返す。
///
/// 評価器内部の詳細な失敗原因は、private設定や応答内容を含み得るため公開しない。
/// ここで粗い正規化を行い、Result Schemaの固定enumを破らない。
fn normalize_reason_code(reason_code: &'static str) -> &'static str {
    match reason_code {
        "exact_match"
        | "contains_match"
        | "regex_match"
        | "json_schema_match"
        | "reference_count_match"
        | "route_match"
        | "status_match"
        | "capability_match"
        | "latency_threshold_met" => "一致",
        "exact_mismatch"
        | "contains_mismatch"
        | "regex_mismatch"
        | "json_schema_mismatch"
        | "reference_count_mismatch"
        | "route_mismatch"
        | "status_mismatch"
        | "capability_mismatch" => "不一致",
        "latency_threshold_exceeded" => "閾値超過",
        "content_visibility_not_full" | "latency_unavailable" => "根拠不足",
        "dialogue_projection_invalid"
        | "json_schema_subject_invalid"
        | "json_schema_subject_limit_exceeded" => "応答不正",
        "missing_dialogue_input" => "入力不足",
        "cancelled" => "中止",
        "audit_failure" => "監査失敗",
        _ => "評価器不正",
    }
}

fn aggregate_verdict(results: &[EvaluatorEvaluation]) -> EvaluationVerdict {
    if results
        .iter()
        .any(|result| result.verdict == EvaluationVerdict::不成立)
    {
        EvaluationVerdict::不成立
    } else if !results.is_empty()
        && results
            .iter()
            .all(|result| result.verdict == EvaluationVerdict::成立)
    {
        EvaluationVerdict::成立
    } else {
        EvaluationVerdict::評価不能
    }
}

fn validate_evaluator_settings(config: &EvaluatorConfig<'_>) -> Result<(), &'static str> {
    match config.kind {
        "exact" => {
            string_setting(config.settings, "期待本文", MAX_BODY_CHARS, false)?;
        }
        "contains" => {
            string_setting(config.settings, "期待断片", MAX_REGEX_CHARS, false)?;
        }
        "regex" => {
            let pattern = string_setting(config.settings, "パターン", MAX_REGEX_CHARS, false)?;
            compile_regex(pattern)?;
        }
        "json_schema" => {
            let schema = value_setting(config.settings, "期待Schema")?;
            if !schema.is_object() {
                return Err("json_schema_invalid");
            }
            validate_json_schema(schema, 0)?;
        }
        "reference_count" => {
            let (minimum, maximum) = reference_count_range(config.settings)?;
            if minimum > MAX_REFERENCES as u64 || maximum > MAX_REFERENCES as u64 {
                return Err("evaluator_config_invalid");
            }
        }
        "route" => {
            string_setting(config.settings, "期待経路", MAX_ROUTE_CHARS, false)?;
        }
        "status" => {
            let expected = string_setting(config.settings, "期待状態", MAX_KIND_CHARS, false)?;
            if !valid_status(expected) {
                return Err("evaluator_config_invalid");
            }
        }
        "capability" => {
            capability_settings(config.settings)?;
        }
        "latency_threshold" => {
            let maximum = u64_setting(config.settings, "最大Millis")?;
            if maximum > MAX_LATENCY_MILLIS {
                return Err("evaluator_config_invalid");
            }
        }
        _ => return Err("unknown_evaluator_kind"),
    }
    Ok(())
}

fn evaluate_valid_config(
    config: &EvaluatorConfig<'_>,
    projection: &PublicDialogueProjection<'_>,
    latency_millis: Option<u64>,
) -> EvaluatorEvaluation {
    if requires_full_visibility(config.kind) && projection.visibility != "full" {
        return invalid_result(config.id, config.kind, "content_visibility_not_full");
    }

    match config.kind {
        "exact" => match string_setting(config.settings, "期待本文", MAX_BODY_CHARS, false) {
            Ok(expected) if projection.body == expected => {
                result(config, EvaluationVerdict::成立, "exact_match")
            }
            Ok(_) => result(config, EvaluationVerdict::不成立, "exact_mismatch"),
            Err(reason_code) => invalid_result(config.id, config.kind, reason_code),
        },
        "contains" => match string_setting(config.settings, "期待断片", MAX_REGEX_CHARS, false)
        {
            Ok(expected) if projection.body.contains(expected) => {
                result(config, EvaluationVerdict::成立, "contains_match")
            }
            Ok(_) => result(config, EvaluationVerdict::不成立, "contains_mismatch"),
            Err(reason_code) => invalid_result(config.id, config.kind, reason_code),
        },
        "regex" => match string_setting(config.settings, "パターン", MAX_REGEX_CHARS, false)
            .and_then(compile_regex)
        {
            Ok(regex) if regex.is_match(projection.body) => {
                result(config, EvaluationVerdict::成立, "regex_match")
            }
            Ok(_) => result(config, EvaluationVerdict::不成立, "regex_mismatch"),
            Err(reason_code) => invalid_result(config.id, config.kind, reason_code),
        },
        "json_schema" => evaluate_json_schema(config, projection.body),
        "reference_count" => match reference_count_range(config.settings) {
            Ok((minimum, maximum))
                if (minimum..=maximum).contains(&(projection.references.len() as u64)) =>
            {
                result(config, EvaluationVerdict::成立, "reference_count_match")
            }
            Ok(_) => result(
                config,
                EvaluationVerdict::不成立,
                "reference_count_mismatch",
            ),
            Err(reason_code) => invalid_result(config.id, config.kind, reason_code),
        },
        "route" => match string_setting(config.settings, "期待経路", MAX_ROUTE_CHARS, false) {
            Ok(expected) if projection.route == expected => {
                result(config, EvaluationVerdict::成立, "route_match")
            }
            Ok(_) => result(config, EvaluationVerdict::不成立, "route_mismatch"),
            Err(reason_code) => invalid_result(config.id, config.kind, reason_code),
        },
        "status" => match string_setting(config.settings, "期待状態", MAX_KIND_CHARS, false) {
            Ok(expected) if projection.state == expected => {
                result(config, EvaluationVerdict::成立, "status_match")
            }
            Ok(_) => result(config, EvaluationVerdict::不成立, "status_mismatch"),
            Err(reason_code) => invalid_result(config.id, config.kind, reason_code),
        },
        "capability" => match capability_settings(config.settings) {
            Ok(required)
                if required.iter().all(|expected| {
                    projection
                        .capabilities
                        .iter()
                        .any(|capability| capability.as_str() == Some(*expected))
                }) =>
            {
                result(config, EvaluationVerdict::成立, "capability_match")
            }
            Ok(_) => result(config, EvaluationVerdict::不成立, "capability_mismatch"),
            Err(reason_code) => invalid_result(config.id, config.kind, reason_code),
        },
        "latency_threshold" => match (latency_millis, u64_setting(config.settings, "最大Millis"))
        {
            (Some(latency), Ok(limit)) if latency <= limit => {
                result(config, EvaluationVerdict::成立, "latency_threshold_met")
            }
            (Some(_), Ok(_)) => result(
                config,
                EvaluationVerdict::不成立,
                "latency_threshold_exceeded",
            ),
            (None, Ok(_)) => invalid_result(config.id, config.kind, "latency_unavailable"),
            (_, Err(reason_code)) => invalid_result(config.id, config.kind, reason_code),
        },
        _ => invalid_result(config.id, config.kind, "unknown_evaluator_kind"),
    }
}

fn requires_full_visibility(kind: &str) -> bool {
    matches!(
        kind,
        "exact" | "contains" | "regex" | "json_schema" | "reference_count" | "route" | "capability"
    )
}

fn value_setting<'a>(
    settings: &'a Map<String, Value>,
    expected_field: &str,
) -> Result<&'a Value, &'static str> {
    if settings.len() != 1 {
        return Err("evaluator_config_invalid");
    }
    settings
        .get(expected_field)
        .ok_or("evaluator_config_invalid")
}

fn string_setting<'a>(
    settings: &'a Map<String, Value>,
    expected_field: &str,
    maximum_chars: usize,
    allow_empty: bool,
) -> Result<&'a str, &'static str> {
    let value = value_setting(settings, expected_field)?
        .as_str()
        .ok_or("evaluator_config_invalid")?;
    if !within_chars(value, maximum_chars) || (!allow_empty && value.is_empty()) {
        return Err("evaluator_config_invalid");
    }
    Ok(value)
}

fn u64_setting(settings: &Map<String, Value>, expected_field: &str) -> Result<u64, &'static str> {
    value_setting(settings, expected_field)?
        .as_u64()
        .ok_or("evaluator_config_invalid")
}

fn reference_count_range(settings: &Map<String, Value>) -> Result<(u64, u64), &'static str> {
    if settings.len() != 2 {
        return Err("evaluator_config_invalid");
    }
    let minimum = settings
        .get("最小数")
        .and_then(Value::as_u64)
        .ok_or("evaluator_config_invalid")?;
    let maximum = settings
        .get("最大数")
        .and_then(Value::as_u64)
        .ok_or("evaluator_config_invalid")?;
    if minimum > maximum || maximum > MAX_REFERENCES as u64 {
        return Err("evaluator_config_invalid");
    }
    Ok((minimum, maximum))
}

fn capability_settings<'a>(settings: &'a Map<String, Value>) -> Result<Vec<&'a str>, &'static str> {
    let values = value_setting(settings, "必要能力一覧")
        .and_then(|value| value.as_array().ok_or("evaluator_config_invalid"))?;
    if values.is_empty() || values.len() > MAX_CAPABILITIES {
        return Err("evaluator_config_invalid");
    }

    let mut required = Vec::with_capacity(values.len());
    for value in values {
        let capability = value.as_str().ok_or("evaluator_config_invalid")?;
        if capability.is_empty() || !within_chars(capability, MAX_CAPABILITY_CHARS) {
            return Err("evaluator_config_invalid");
        }
        required.push(capability);
    }
    Ok(required)
}

fn compile_regex(pattern: &str) -> Result<regex::Regex, &'static str> {
    RegexBuilder::new(pattern)
        .size_limit(1 << 20)
        .dfa_size_limit(1 << 20)
        .build()
        .map_err(|_| "regex_invalid")
}

fn evaluate_json_schema(config: &EvaluatorConfig<'_>, body: &str) -> EvaluatorEvaluation {
    if body.len() > MAX_JSON_DOCUMENT_BYTES {
        return invalid_result(config.id, config.kind, "json_schema_subject_limit_exceeded");
    }

    let subject = match super::json_input::read_unique::<Value>(body) {
        Ok(subject) => subject,
        Err(_) => return invalid_result(config.id, config.kind, "json_schema_subject_invalid"),
    };
    if !bounded_json_value(&subject, 0) {
        return invalid_result(config.id, config.kind, "json_schema_subject_limit_exceeded");
    }

    let schema = match value_setting(config.settings, "期待Schema") {
        Ok(schema) => schema,
        Err(reason_code) => return invalid_result(config.id, config.kind, reason_code),
    };
    if let Err(reason_code) = validate_json_schema(schema, 0) {
        return invalid_result(config.id, config.kind, reason_code);
    }

    match json_schema_matches(schema, &subject, 0) {
        Ok(true) => result(config, EvaluationVerdict::成立, "json_schema_match"),
        Ok(false) => result(config, EvaluationVerdict::不成立, "json_schema_mismatch"),
        Err(reason_code) => invalid_result(config.id, config.kind, reason_code),
    }
}

fn validate_json_schema(schema: &Value, depth: usize) -> Result<(), &'static str> {
    if depth > MAX_JSON_DEPTH || !bounded_json_value(schema, depth) {
        return Err("json_schema_limit_exceeded");
    }
    let object = schema.as_object().ok_or("json_schema_invalid")?;

    for (keyword, value) in object {
        match keyword.as_str() {
            "type" => {
                let type_name = value.as_str().ok_or("json_schema_invalid")?;
                if !valid_json_type(type_name) {
                    return Err("json_schema_invalid");
                }
            }
            "required" => {
                let required = value.as_array().ok_or("json_schema_invalid")?;
                if required.len() > MAX_JSON_COLLECTION_ITEMS
                    || required.iter().any(|item| {
                        item.as_str()
                            .map(|field| !within_chars(field, MAX_JSON_KEY_CHARS))
                            .unwrap_or(true)
                    })
                {
                    return Err("json_schema_invalid");
                }
            }
            "properties" => {
                let properties = value.as_object().ok_or("json_schema_invalid")?;
                if properties.len() > MAX_JSON_COLLECTION_ITEMS
                    || properties
                        .keys()
                        .any(|field| !within_chars(field, MAX_JSON_KEY_CHARS))
                {
                    return Err("json_schema_limit_exceeded");
                }
                for property_schema in properties.values() {
                    validate_json_schema(property_schema, depth + 1)?;
                }
            }
            "items" => {
                validate_json_schema(value, depth + 1)?;
            }
            "enum" => {
                let options = value.as_array().ok_or("json_schema_invalid")?;
                if options.is_empty()
                    || options.len() > MAX_JSON_COLLECTION_ITEMS
                    || options
                        .iter()
                        .any(|option| !bounded_json_value(option, depth + 1))
                {
                    return Err("json_schema_invalid");
                }
            }
            "const" => {
                if !bounded_json_value(value, depth + 1) {
                    return Err("json_schema_limit_exceeded");
                }
            }
            "additionalProperties" => {
                if !value.is_boolean() {
                    return Err("json_schema_invalid");
                }
            }
            "$ref" => return Err("json_schema_ref_unsupported"),
            _ => return Err("json_schema_keyword_unsupported"),
        }
    }
    Ok(())
}

fn json_schema_matches(
    schema: &Value,
    subject: &Value,
    depth: usize,
) -> Result<bool, &'static str> {
    if depth > MAX_JSON_DEPTH {
        return Err("json_schema_limit_exceeded");
    }
    let object = schema.as_object().ok_or("json_schema_invalid")?;

    if let Some(type_name) = object.get("type").and_then(Value::as_str) {
        if !json_type_matches(subject, type_name) {
            return Ok(false);
        }
    }
    if let Some(options) = object.get("enum").and_then(Value::as_array) {
        if !options.iter().any(|option| option == subject) {
            return Ok(false);
        }
    }
    if let Some(expected) = object.get("const") {
        if expected != subject {
            return Ok(false);
        }
    }

    if let Some(subject_object) = subject.as_object() {
        if let Some(required) = object.get("required").and_then(Value::as_array) {
            if required.iter().any(|field| {
                field
                    .as_str()
                    .map(|field| !subject_object.contains_key(field))
                    .unwrap_or(true)
            }) {
                return Ok(false);
            }
        }

        let properties = object.get("properties").and_then(Value::as_object);
        if let Some(properties) = properties {
            for (field, property_schema) in properties {
                if let Some(value) = subject_object.get(field) {
                    if !json_schema_matches(property_schema, value, depth + 1)? {
                        return Ok(false);
                    }
                }
            }
        }
        if object.get("additionalProperties").and_then(Value::as_bool) == Some(false) {
            if subject_object.keys().any(|field| {
                properties
                    .map(|properties| !properties.contains_key(field))
                    .unwrap_or(true)
            }) {
                return Ok(false);
            }
        }
    }

    if let Some(subject_array) = subject.as_array() {
        if subject_array.len() > MAX_JSON_COLLECTION_ITEMS {
            return Err("json_schema_subject_limit_exceeded");
        }
        if let Some(item_schema) = object.get("items") {
            for item in subject_array {
                if !json_schema_matches(item_schema, item, depth + 1)? {
                    return Ok(false);
                }
            }
        }
    }

    Ok(true)
}

fn bounded_json_value(value: &Value, depth: usize) -> bool {
    if depth > MAX_JSON_DEPTH {
        return false;
    }
    match value {
        Value::Object(object) => {
            object.len() <= MAX_JSON_COLLECTION_ITEMS
                && object.iter().all(|(key, value)| {
                    within_chars(key, MAX_JSON_KEY_CHARS) && bounded_json_value(value, depth + 1)
                })
        }
        Value::Array(items) => {
            items.len() <= MAX_JSON_COLLECTION_ITEMS
                && items.iter().all(|item| bounded_json_value(item, depth + 1))
        }
        Value::String(value) => within_chars(value, MAX_BODY_CHARS),
        Value::Null | Value::Bool(_) | Value::Number(_) => true,
    }
}

fn valid_json_type(value: &str) -> bool {
    matches!(
        value,
        "null" | "boolean" | "object" | "array" | "number" | "integer" | "string"
    )
}

fn json_type_matches(value: &Value, type_name: &str) -> bool {
    match type_name {
        "null" => value.is_null(),
        "boolean" => value.is_boolean(),
        "object" => value.is_object(),
        "array" => value.is_array(),
        "number" => value.is_number(),
        "integer" => value
            .as_number()
            .map(|number| number.as_i64().is_some() || number.as_u64().is_some())
            .unwrap_or(false),
        "string" => value.is_string(),
        _ => false,
    }
}

fn valid_status(value: &str) -> bool {
    matches!(value, "成功" | "保留" | "失敗" | "中止")
}

fn valid_visibility(value: &str) -> bool {
    matches!(
        value,
        "none" | "hash_only" | "summary" | "redacted" | "full"
    )
}

fn valid_response_hash(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    let Some(hash) = value.strip_prefix("sha256:") else {
        return false;
    };
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_evaluator_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_kind_label(value: &str) -> bool {
    !value.is_empty() && within_chars(value, MAX_KIND_CHARS) && value.is_ascii()
}

fn valid_string_array(values: &[Value], maximum_items: usize, maximum_chars: usize) -> bool {
    values.len() <= maximum_items
        && values.iter().all(|value| {
            value
                .as_str()
                .map(|value| within_chars(value, maximum_chars))
                .unwrap_or(false)
        })
}

fn within_chars(value: &str, maximum: usize) -> bool {
    value.chars().count() <= maximum
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evaluator(id: u32, kind: &str, settings: Value) -> Value {
        json!({
            "評価器ID": format!("{id:032x}"),
            "種類": kind,
            "設定": settings,
        })
    }

    fn full_projection(body: &str) -> Value {
        json!({
            "状態": "成功",
            "表示範囲": "full",
            "本文": body,
            "参照": ["ref-a", "ref-b"],
            "能力": ["read", "search"],
            "経路": "runtime.dialogue",
            "応答hash": format!("sha256:{}", "a".repeat(64)),
        })
    }

    fn non_full_projection(body: &str) -> Value {
        let mut value = full_projection(body);
        value["表示範囲"] = json!("summary");
        value
    }

    fn one(
        kind: &str,
        settings: Value,
        projection: Value,
        latency_millis: Option<u64>,
    ) -> EvaluatorEvaluation {
        let configs = Value::Array(vec![evaluator(1, kind, settings)]);
        evaluate_case(&projection, latency_millis, &configs)
            .evaluator_results
            .into_iter()
            .next()
            .expect("評価器結果を一件取得できる")
    }

    #[test]
    fn executable_evaluators_pass_and_fail_deterministically() {
        let body = r#"{"answer":"ok","items":[1,2],"constant":7}"#;
        let projection = full_projection(body);
        let schema = json!({
            "type": "object",
            "required": ["answer", "items", "constant"],
            "properties": {
                "answer": {"type": "string", "enum": ["ok"], "const": "ok"},
                "items": {"type": "array", "items": {"type": "integer"}},
                "constant": {"const": 7}
            },
            "additionalProperties": false
        });

        let cases = [
            (
                "exact",
                json!({"期待本文": body}),
                json!({"期待本文": "different"}),
                Some(100),
            ),
            (
                "contains",
                json!({"期待断片": "answer"}),
                json!({"期待断片": "missing"}),
                Some(100),
            ),
            (
                "regex",
                json!({"パターン": r#""answer":"ok""#}),
                json!({"パターン": r#""answer":"missing""#}),
                Some(100),
            ),
            (
                "json_schema",
                json!({"期待Schema": schema}),
                json!({"期待Schema": {"type": "array"}}),
                Some(100),
            ),
            (
                "reference_count",
                json!({"最小数": 1, "最大数": 2}),
                json!({"最小数": 3, "最大数": 4}),
                Some(100),
            ),
            (
                "route",
                json!({"期待経路": "runtime.dialogue"}),
                json!({"期待経路": "other.route"}),
                Some(100),
            ),
            (
                "status",
                json!({"期待状態": "成功"}),
                json!({"期待状態": "失敗"}),
                Some(100),
            ),
            (
                "capability",
                json!({"必要能力一覧": ["read", "search"]}),
                json!({"必要能力一覧": ["read", "write"]}),
                Some(100),
            ),
            (
                "latency_threshold",
                json!({"最大Millis": 100}),
                json!({"最大Millis": 99}),
                Some(100),
            ),
        ];

        for (kind, passing_settings, failing_settings, latency) in cases {
            assert_eq!(
                one(kind, passing_settings, projection.clone(), latency).verdict,
                EvaluationVerdict::成立,
                "{kind} should pass"
            );
            assert_eq!(
                one(kind, failing_settings, projection.clone(), latency).verdict,
                EvaluationVerdict::不成立,
                "{kind} should fail"
            );
        }
    }

    #[test]
    fn full_only_evaluators_are_unavailable_outside_full_visibility() {
        let body = r#"{"answer":"ok"}"#;
        let projection = non_full_projection(body);
        let settings = [
            ("exact", json!({"期待本文": body})),
            ("contains", json!({"期待断片": "answer"})),
            ("regex", json!({"パターン": "answer"})),
            ("json_schema", json!({"期待Schema": {"type": "object"}})),
            ("reference_count", json!({"最小数": 2, "最大数": 2})),
            ("route", json!({"期待経路": "runtime.dialogue"})),
            ("capability", json!({"必要能力一覧": ["read"]})),
        ];

        for (kind, settings) in settings {
            let result = one(kind, settings, projection.clone(), Some(100));
            assert_eq!(result.verdict, EvaluationVerdict::評価不能, "{kind}");
            assert_eq!(result.reason_code, "根拠不足", "{kind}");
        }
    }

    #[test]
    fn status_is_available_without_full_and_latency_needs_measurement() {
        let projection = non_full_projection("summary only");

        assert_eq!(
            one(
                "status",
                json!({"期待状態": "成功"}),
                projection.clone(),
                None
            )
            .verdict,
            EvaluationVerdict::成立
        );
        assert_eq!(
            one(
                "status",
                json!({"期待状態": "失敗"}),
                projection.clone(),
                None
            )
            .verdict,
            EvaluationVerdict::不成立
        );
        assert_eq!(
            one(
                "status",
                json!({"期待状態": "unknown"}),
                projection.clone(),
                None
            )
            .verdict,
            EvaluationVerdict::評価不能
        );
        let latency = one(
            "latency_threshold",
            json!({"最大Millis": 100}),
            projection,
            None,
        );
        assert_eq!(latency.verdict, EvaluationVerdict::評価不能);
        assert_eq!(latency.reason_code, "根拠不足");
    }

    #[test]
    fn regex_matches_with_standard_crate_and_rejects_invalid_patterns() {
        let matched = one(
            "regex",
            json!({"パターン": r#""answer":"ok""#}),
            full_projection(r#"{"answer":"ok"}"#),
            Some(100),
        );
        assert_eq!(matched.verdict, EvaluationVerdict::成立);
        assert_eq!(matched.reason_code, "一致");

        let mismatched = one(
            "regex",
            json!({"パターン": r#""answer":"missing""#}),
            full_projection(r#"{"answer":"ok"}"#),
            Some(100),
        );
        assert_eq!(mismatched.verdict, EvaluationVerdict::不成立);
        assert_eq!(mismatched.reason_code, "不一致");

        let invalid = one(
            "regex",
            json!({"パターン": "("}),
            full_projection(r#"{"answer":"ok"}"#),
            Some(100),
        );
        assert_eq!(invalid.verdict, EvaluationVerdict::評価不能);
        assert_eq!(invalid.reason_code, "評価器不正");
    }

    #[test]
    fn json_schema_rejects_invalid_json_unknown_keywords_and_references() {
        let invalid_subject = one(
            "json_schema",
            json!({"期待Schema": {"type": "object"}}),
            full_projection("{"),
            Some(100),
        );
        assert_eq!(invalid_subject.verdict, EvaluationVerdict::評価不能);
        assert_eq!(invalid_subject.reason_code, "応答不正");

        let unsupported_keyword = one(
            "json_schema",
            json!({"期待Schema": {"pattern": "x"}}),
            full_projection(r#""x""#),
            Some(100),
        );
        assert_eq!(unsupported_keyword.verdict, EvaluationVerdict::評価不能);
        assert_eq!(unsupported_keyword.reason_code, "評価器不正");

        let reference = one(
            "json_schema",
            json!({"期待Schema": {"$ref": "https://example.invalid/schema"}}),
            full_projection(r#""x""#),
            Some(100),
        );
        assert_eq!(reference.verdict, EvaluationVerdict::評価不能);
        assert_eq!(reference.reason_code, "評価器不正");

        let additional_property = one(
            "json_schema",
            json!({"期待Schema": {
                "type": "object",
                "properties": {"allowed": {"type": "string"}},
                "additionalProperties": false
            }}),
            full_projection(r#"{"allowed":"ok","extra":true}"#),
            Some(100),
        );
        assert_eq!(additional_property.verdict, EvaluationVerdict::不成立);
        assert_eq!(additional_property.reason_code, "不一致");
    }

    #[test]
    fn invalid_config_unknown_kind_and_large_values_fail_closed() {
        let malformed = json!({
            "評価器ID": "00000000000000000000000000000001",
            "種類": "exact"
        });
        let malformed_configs = Value::Array(vec![malformed]);
        let malformed_result = evaluate_case(&full_projection("ok"), Some(1), &malformed_configs);
        assert_eq!(malformed_result.verdict, EvaluationVerdict::評価不能);
        assert_eq!(
            malformed_result.evaluator_results[0].reason_code,
            "評価器不正"
        );

        let unknown = one(
            "unsupported",
            json!({"anything": true}),
            full_projection("ok"),
            Some(1),
        );
        assert_eq!(unknown.verdict, EvaluationVerdict::評価不能);
        assert_eq!(unknown.reason_code, "評価器不正");

        let huge_projection = full_projection(&"x".repeat(MAX_BODY_CHARS + 1));
        let huge = one("exact", json!({"期待本文": "x"}), huge_projection, Some(1));
        assert_eq!(huge.verdict, EvaluationVerdict::評価不能);
        assert_eq!(huge.reason_code, "応答不正");

        let too_many = Value::Array(
            (0..(MAX_EVALUATORS + 1))
                .map(|id| evaluator(id as u32, "status", json!({"期待状態": "成功"})))
                .collect(),
        );
        let too_many_result = evaluate_case(&full_projection("ok"), Some(1), &too_many);
        assert_eq!(too_many_result.verdict, EvaluationVerdict::評価不能);
        assert_eq!(
            too_many_result.evaluator_results[0].reason_code,
            "評価器不正"
        );
    }

    #[test]
    fn schema_private_config_fields_and_ranges_are_enforced() {
        let projection = full_projection("response");

        let old_kind_field = json!({
            "評価器ID": format!("{:032x}", 1),
            "種別": "contains",
            "設定": {"期待断片": "response"},
        });
        let old_kind_result =
            evaluate_case(&projection, Some(1), &Value::Array(vec![old_kind_field]));
        assert_eq!(old_kind_result.verdict, EvaluationVerdict::評価不能);
        assert_eq!(
            old_kind_result.evaluator_results[0].reason_code,
            "評価器不正"
        );

        for (kind, settings) in [
            ("reference_count", json!({"最小数": 3, "最大数": 2})),
            ("capability", json!({"必要能力一覧": "read"})),
            (
                "latency_threshold",
                json!({"最大Millis": MAX_LATENCY_MILLIS + 1}),
            ),
        ] {
            let result = one(kind, settings, projection.clone(), Some(1));
            assert_eq!(result.verdict, EvaluationVerdict::評価不能, "{kind}");
            assert_eq!(result.reason_code, "評価器不正", "{kind}");
        }
    }

    #[test]
    fn multiple_evaluators_use_and_and_preserve_input_order() {
        let configs = Value::Array(vec![
            evaluator(3, "status", json!({"期待状態": "成功"})),
            evaluator(1, "contains", json!({"期待断片": "missing"})),
            evaluator(2, "latency_threshold", json!({"最大Millis": 10})),
        ]);
        let first = evaluate_case(&full_projection(r#"{"answer":"ok"}"#), None, &configs);
        let second = evaluate_case(&full_projection(r#"{"answer":"ok"}"#), None, &configs);

        assert_eq!(first.verdict, EvaluationVerdict::不成立);
        assert_eq!(first, second);
        assert_eq!(
            first
                .evaluator_results
                .iter()
                .map(|result| result.evaluator_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "00000000000000000000000000000003",
                "00000000000000000000000000000001",
                "00000000000000000000000000000002"
            ]
        );
        assert_eq!(
            first
                .evaluator_results
                .iter()
                .map(|result| result.verdict)
                .collect::<Vec<_>>(),
            vec![
                EvaluationVerdict::成立,
                EvaluationVerdict::不成立,
                EvaluationVerdict::評価不能
            ]
        );
    }

    #[test]
    fn public_projection_never_includes_expected_or_body_values() {
        let secret_expected = "private expected phrase";
        let body = "private result body";
        let configs = Value::Array(vec![evaluator(
            1,
            "exact",
            json!({"期待本文": secret_expected}),
        )]);
        let public = evaluate_case_public_projection(&full_projection(body), Some(1), &configs);
        let serialized = serde_json::to_string(&public).expect("公開結果を直列化できる");

        assert!(!serialized.contains(secret_expected));
        assert!(!serialized.contains(body));
        assert_eq!(public["case判定"], json!("不成立"));
        assert_eq!(
            public["評価結果"][0]["評価器ID"],
            json!(format!("{:032x}", 1))
        );
        assert_eq!(public["評価結果"][0]["種類"], json!("exact"));
        assert_eq!(public["評価結果"][0]["理由code"], json!("不一致"));
        assert!(public["評価結果"][0].get("理由コード").is_none());
    }
}
