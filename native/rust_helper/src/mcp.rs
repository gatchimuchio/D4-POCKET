//! MCP stdio wireの検証とmetadata-only catalog射影。
//!
//! 外部Serverの応答は未信頼入力であり、Server metadata、Tool description、Trust、
//! Capability diffから権限を生成しない。Process起動・Credential注入・Tool実行は
//! このparserの責任外であり、Brokerの別の統治経路からだけ呼び出す。
#![allow(non_snake_case)]

use crate::audit_hash::sha256_tagged;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::fmt;

pub const MODERN_PROTOCOL_VERSION: &str = "2026-07-28";
pub const LEGACY_PROTOCOL_VERSION: &str = "2025-11-25";
const MAX_WIRE_LINE_BYTES: usize = 256 * 1024;
const MAX_CATALOG_ITEMS: usize = 256;
const MAX_NAME_BYTES: usize = 256;
const MAX_TOOL_SCHEMA_BYTES: usize = 128 * 1024;
const MAX_TOOL_SCHEMA_NODES: usize = 4096;
const MAX_TOOL_SCHEMA_DEPTH: usize = 64;
const MAX_TOOL_ARGUMENT_BYTES: usize = 32 * 1024;
const MAX_TOOL_ARGUMENT_NODES: usize = 2048;
const MAX_TOOL_ARGUMENT_DEPTH: usize = 32;
#[cfg(test)]
const MCP_JSON_SCHEMA_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

fn is_mcp_json_schema_dialect(value: &str) -> bool {
    matches!(
        value,
        "https://json-schema.org/draft/2020-12/schema"
            | "https://json-schema.org/draft/2020-12/schema#"
            | "http://json-schema.org/draft/2020-12/schema"
            | "http://json-schema.org/draft/2020-12/schema#"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpProtocolEra {
    Modern,
    Legacy,
}

impl McpProtocolEra {
    pub fn version(self) -> &'static str {
        match self {
            Self::Modern => MODERN_PROTOCOL_VERSION,
            Self::Legacy => LEGACY_PROTOCOL_VERSION,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpError {
    pub code: &'static str,
    pub message: &'static str,
}

impl McpError {
    pub(crate) fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}

impl fmt::Display for McpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for McpError {}

#[derive(Debug, Clone, PartialEq)]
pub struct McpJsonRpcMessage {
    pub id: Option<Value>,
    pub method: Option<String>,
    pub params: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<Value>,
    pub metadata: Option<Value>,
}

impl McpJsonRpcMessage {
    pub fn parse_line(line: &[u8]) -> Result<Self, McpError> {
        if line.len() > MAX_WIRE_LINE_BYTES {
            return Err(McpError::new(
                "mcp_wire_oversized",
                "MCP wire lineが上限を超過した",
            ));
        }
        let text = std::str::from_utf8(line)
            .map_err(|_| McpError::new("mcp_wire_invalid_utf8", "MCP wire lineがUTF-8ではない"))?;
        let value: Value = serde_json::from_str(text)
            .map_err(|_| McpError::new("mcp_wire_malformed_json", "MCP wire JSONが不正"))?;
        let object = value.as_object().ok_or_else(|| {
            McpError::new(
                "mcp_wire_not_object",
                "MCP JSON-RPC messageがobjectではない",
            )
        })?;
        for key in object.keys() {
            if !matches!(
                key.as_str(),
                "jsonrpc" | "id" | "method" | "params" | "result" | "error" | "_meta"
            ) {
                return Err(McpError::new(
                    "mcp_wire_unknown_field",
                    "MCP JSON-RPC fieldが不明",
                ));
            }
        }
        if object.get("jsonrpc") != Some(&Value::String("2.0".to_string())) {
            return Err(McpError::new(
                "mcp_wire_version_invalid",
                "JSON-RPC versionが2.0ではない",
            ));
        }
        let id = object.get("id").cloned();
        if id.as_ref().is_some_and(Value::is_null) {
            return Err(McpError::new(
                "mcp_wire_id_invalid",
                "JSON-RPC idがnullである",
            ));
        }
        if let Some(id) = &id {
            if !(id.is_string() || id.is_u64() || id.is_i64()) {
                return Err(McpError::new(
                    "mcp_wire_id_invalid",
                    "JSON-RPC idの型が不正",
                ));
            }
            if id
                .as_str()
                .is_some_and(|value| value.as_bytes().len() > 128)
            {
                return Err(McpError::new(
                    "mcp_wire_id_oversized",
                    "JSON-RPC idが上限を超過した",
                ));
            }
        }
        let method = object
            .get("method")
            .and_then(Value::as_str)
            .map(str::to_string);
        if method.as_deref().is_some_and(|value| {
            value.is_empty()
                || value.as_bytes().len() > MAX_NAME_BYTES
                || value.chars().any(char::is_control)
        }) {
            return Err(McpError::new("mcp_method_invalid", "MCP methodが不正"));
        }
        let has_result = object.contains_key("result");
        let has_error = object.contains_key("error");
        if has_result && has_error {
            return Err(McpError::new(
                "mcp_response_ambiguous",
                "MCP responseがresultとerrorを同時に持つ",
            ));
        }
        let is_request = method.is_some();
        let is_response = has_result || has_error;
        if is_request == is_response {
            return Err(McpError::new(
                "mcp_wire_shape_invalid",
                "MCP JSON-RPC messageの形が不正",
            ));
        }
        if is_request && method.is_some() && id.is_none() && object.contains_key("result") {
            return Err(McpError::new(
                "mcp_wire_shape_invalid",
                "MCP notificationがresponse fieldを持つ",
            ));
        }
        if let Some(error) = object.get("error") {
            let error_object = error
                .as_object()
                .ok_or_else(|| McpError::new("mcp_error_invalid", "MCP errorがobjectではない"))?;
            if !error_object.get("code").is_some_and(Value::is_i64)
                || !error_object.get("message").is_some_and(Value::is_string)
            {
                return Err(McpError::new(
                    "mcp_error_invalid",
                    "MCP errorのcodeまたはmessageが不正",
                ));
            }
        }
        Ok(Self {
            id,
            method,
            params: object.get("params").cloned(),
            result: object.get("result").cloned(),
            error: object.get("error").cloned(),
            metadata: object.get("_meta").cloned(),
        })
    }

    pub fn is_notification(&self) -> bool {
        self.method.is_some() && self.id.is_none()
    }

    pub fn response_for(&self, id: u64) -> bool {
        self.id.as_ref().and_then(Value::as_u64) == Some(id) && self.method.is_none()
    }

    pub fn is_method_not_found_error(&self) -> bool {
        self.error
            .as_ref()
            .and_then(Value::as_object)
            .and_then(|error| error.get("code"))
            .and_then(Value::as_i64)
            == Some(-32601)
    }

    pub fn validate_complete_result(&self) -> Result<(), McpError> {
        let Some(result) = self.result.as_ref() else {
            return Ok(());
        };
        let result = result
            .as_object()
            .ok_or_else(|| McpError::new("mcp_result_invalid", "MCP resultがobjectではない"))?;
        if result
            .get("resultType")
            .is_some_and(|result_type| result_type != "complete")
        {
            return Err(McpError::new(
                "mcp_result_type_unsupported",
                "MCP resultTypeが未対応",
            ));
        }
        Ok(())
    }
}

pub fn request_line(
    id: u64,
    era: McpProtocolEra,
    method: &str,
    params: Value,
) -> Result<Vec<u8>, McpError> {
    if method.is_empty()
        || method.as_bytes().len() > MAX_NAME_BYTES
        || method.chars().any(char::is_control)
    {
        return Err(McpError::new("mcp_method_invalid", "MCP methodが不正"));
    }
    let params = match era {
        McpProtocolEra::Modern => {
            let mut object = params.as_object().cloned().ok_or_else(|| {
                McpError::new(
                    "mcp_params_invalid",
                    "modern MCP paramsはobjectでなければならない",
                )
            })?;
            let metadata = object
                .entry("_meta")
                .or_insert_with(|| Value::Object(Map::new()));
            let metadata = metadata.as_object_mut().ok_or_else(|| {
                McpError::new("mcp_metadata_invalid", "MCP _metaがobjectではない")
            })?;
            metadata.insert(
                "io.modelcontextprotocol/protocolVersion".to_string(),
                Value::String(era.version().to_string()),
            );
            metadata.insert(
                "io.modelcontextprotocol/clientInfo".to_string(),
                json!({"name": "gui-shell", "version": "0.1.0"}),
            );
            metadata.insert(
                "io.modelcontextprotocol/clientCapabilities".to_string(),
                json!({}),
            );
            Value::Object(object)
        }
        McpProtocolEra::Legacy => params,
    };
    serde_json::to_vec(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    }))
    .map(|mut bytes| {
        bytes.push(b'\n');
        bytes
    })
    .map_err(|_| McpError::new("mcp_wire_encode_failed", "MCP requestのJSON化に失敗した"))
}

pub fn notification_line(method: &str, params: Value) -> Result<Vec<u8>, McpError> {
    if method.is_empty()
        || method.as_bytes().len() > MAX_NAME_BYTES
        || method.chars().any(char::is_control)
    {
        return Err(McpError::new("mcp_method_invalid", "MCP methodが不正"));
    }
    serde_json::to_vec(&json!({"jsonrpc": "2.0", "method": method, "params": params}))
        .map(|mut bytes| {
            bytes.push(b'\n');
            bytes
        })
        .map_err(|_| {
            McpError::new(
                "mcp_wire_encode_failed",
                "MCP notificationのJSON化に失敗した",
            )
        })
}

pub fn modern_discover_request(id: u64) -> Result<Vec<u8>, McpError> {
    request_line(id, McpProtocolEra::Modern, "server/discover", json!({}))
}

pub fn legacy_initialize_request(id: u64) -> Result<Vec<u8>, McpError> {
    request_line(
        id,
        McpProtocolEra::Legacy,
        "initialize",
        json!({
            "protocolVersion": LEGACY_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {"name": "gui-shell", "version": "0.1.0"}
        }),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpDiscovery {
    pub era: McpProtocolEra,
    pub protocol_version: String,
    pub server_name: Option<String>,
    pub server_version: Option<String>,
    pub capabilities: Value,
    pub metadata_hash: String,
}

pub fn parse_discovery(
    message: &McpJsonRpcMessage,
    era: McpProtocolEra,
) -> Result<McpDiscovery, McpError> {
    if message.error.is_some() || message.result.is_none() {
        return Err(McpError::new(
            "mcp_discovery_failed",
            "MCP discovery responseが失敗した",
        ));
    }
    let result = message
        .result
        .as_ref()
        .and_then(Value::as_object)
        .ok_or_else(|| {
            McpError::new(
                "mcp_discovery_invalid",
                "MCP discovery resultがobjectではない",
            )
        })?;
    let protocol_version = match era {
        McpProtocolEra::Modern => {
            let versions = result
                .get("supportedVersions")
                .and_then(Value::as_array)
                .ok_or_else(|| McpError::new("mcp_discovery_invalid", "supportedVersionsがない"))?;
            let versions: Vec<&str> = versions.iter().filter_map(Value::as_str).collect();
            if !versions.contains(&MODERN_PROTOCOL_VERSION) {
                return Err(McpError::new(
                    "mcp_protocol_unsupported",
                    "modern MCP protocol versionを選択できない",
                ));
            }
            MODERN_PROTOCOL_VERSION.to_string()
        }
        McpProtocolEra::Legacy => result
            .get("protocolVersion")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty() && value.as_bytes().len() <= 32)
            .ok_or_else(|| McpError::new("mcp_discovery_invalid", "legacy protocolVersionがない"))?
            .to_string(),
    };
    let capabilities = result
        .get("capabilities")
        .filter(|value| value.is_object())
        .cloned()
        .ok_or_else(|| {
            McpError::new(
                "mcp_capabilities_invalid",
                "MCP capabilitiesがobjectではない",
            )
        })?;
    let server_info = if era == McpProtocolEra::Modern {
        message
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("io.modelcontextprotocol/serverInfo"))
    } else {
        result.get("serverInfo")
    };
    let (server_name, server_version) = parse_server_info(server_info)?;
    let metadata_hash = sha256_tagged(&serde_json::to_vec(result).map_err(|_| {
        McpError::new(
            "mcp_discovery_invalid",
            "MCP discovery resultをhash化できない",
        )
    })?);
    Ok(McpDiscovery {
        era,
        protocol_version,
        server_name,
        server_version,
        capabilities,
        metadata_hash,
    })
}

fn parse_server_info(value: Option<&Value>) -> Result<(Option<String>, Option<String>), McpError> {
    let Some(value) = value else {
        return Ok((None, None));
    };
    let object = value.as_object().ok_or_else(|| {
        McpError::new("mcp_server_info_invalid", "MCP serverInfoがobjectではない")
    })?;
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.as_bytes().len() <= MAX_NAME_BYTES)
        .map(str::to_string);
    let version = object
        .get("version")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.as_bytes().len() <= MAX_NAME_BYTES)
        .map(str::to_string);
    if name.is_none() || version.is_none() {
        return Err(McpError::new(
            "mcp_server_info_invalid",
            "MCP serverInfoのnameまたはversionがない",
        ));
    }
    Ok((name, version))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpTool {
    pub name: String,
    pub tool_id: String,
    pub input_schema_hash: String,
    input_schema: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpResource {
    pub name: String,
    pub resource_id: String,
    pub uri_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpPrompt {
    pub name: String,
    pub prompt_id: String,
    pub argument_schema_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpCatalog {
    pub discovery: McpDiscovery,
    pub tools: Vec<McpTool>,
    pub resources: Vec<McpResource>,
    pub prompts: Vec<McpPrompt>,
}

impl McpCatalog {
    pub fn to_metadata_projection(
        &self,
        server_id: &str,
        transport_kind: &str,
        endpoint_hash: &str,
        credential_ref: Value,
    ) -> Result<Value, McpError> {
        if server_id.is_empty() || !safe_text(server_id, MAX_NAME_BYTES) {
            return Err(McpError::new(
                "mcp_server_id_invalid",
                "MCP server IDが不正",
            ));
        }
        if !matches!(transport_kind, "stdio" | "streamable_http" | "oauth") {
            return Err(McpError::new(
                "mcp_transport_invalid",
                "MCP transport種別が不正",
            ));
        }
        if !is_hash(endpoint_hash) {
            return Err(McpError::new(
                "mcp_endpoint_hash_invalid",
                "MCP接続先hashが不正",
            ));
        }
        validate_credential_ref(&credential_ref)?;
        let tool_ids: Vec<String> = self.tools.iter().map(|tool| tool.tool_id.clone()).collect();
        let mut tool_projection = Vec::with_capacity(self.tools.len());
        for tool in &self.tools {
            tool_projection.push(json!({
                "tool_id": tool.tool_id,
                "name": tool.name,
                "description_summary": "",
                "input_schema_hash": tool.input_schema_hash,
                "risk": "unknown",
                "status": {"status": "supported", "reason": "JSON Schema 2020-12を検証済み"}
            }));
        }
        let resource_projection: Vec<Value> = self
            .resources
            .iter()
            .map(|resource| {
                json!({
                    "resource_id": resource.resource_id,
                    "name": resource.name,
                    "uri_template_hash": resource.uri_hash,
                    "mime_type": "application/octet-stream",
                    "status": {"status": "supported", "reason": "MCP resource metadataを検証済み"}
                })
            })
            .collect();
        let prompt_projection: Vec<Value> = self
            .prompts
            .iter()
            .map(|prompt| {
                json!({
                    "prompt_id": prompt.prompt_id,
                    "name": prompt.name,
                    "description_summary": "",
                    "argument_schema_hash": prompt.argument_schema_hash,
                    "status": {"status": "supported", "reason": "MCP prompt metadataを検証済み"}
                })
            })
            .collect();
        Ok(json!({
            "版": 1,
            "契約種別": "MCP外部概念射影",
            "Server": {
                "server_id": server_id,
                "表示名": self.discovery.server_name.clone().unwrap_or_else(|| "匿名MCP Server".to_string()),
                "origin": "live_runtime",
                "version": self.discovery.server_version.clone().unwrap_or_else(|| self.discovery.protocol_version.clone()),
                "metadata_hash": self.discovery.metadata_hash,
                "status": "discovered"
            },
            "Transport": {
                "kind": transport_kind,
                "status": {"status": "supported", "reason": "MCP transport wireを検証済み"},
                "接続先hash": endpoint_hash,
                "reason": "接続先実値をmetadataへ投影しない"
            },
            "Tool": tool_projection,
            "Resource": resource_projection,
            "Prompt": prompt_projection,
            "Credential ref": credential_ref,
            "Trust": {
                "state": "unverified",
                "evidence_source": "LIVE_RUNTIME",
                "reason": "MCP metadataはTrustまたはAuthorityを自動生成しない"
            },
            "Capability diff": {
                "status": if tool_ids.is_empty() { "no_change" } else { "added" },
                "added": tool_ids,
                "removed": [],
                "changed": [],
                "requires_operator_review": !self.tools.is_empty(),
                "evidence_source": "LIVE_RUNTIME"
            },
            "権限生成": "なし",
            "公開範囲": "metadata_only",
            "証拠種別": "LIVE_RUNTIME"
        }))
    }

    pub fn validate_tool_call(&self, name: &str, arguments: &Value) -> Result<(), McpError> {
        let tool = self
            .tools
            .iter()
            .find(|tool| tool.name == name)
            .ok_or_else(|| McpError::new("mcp_unknown_tool", "MCP Toolがcatalogへ未登録"))?;
        if !arguments.is_object() {
            return Err(McpError::new(
                "mcp_tool_arguments_invalid",
                "MCP Tool argumentsがobjectではない",
            ));
        }
        if !json_value_within_limits(arguments, MAX_TOOL_ARGUMENT_NODES, MAX_TOOL_ARGUMENT_DEPTH) {
            return Err(McpError::new(
                "mcp_tool_arguments_complexity_exceeded",
                "MCP Tool argumentsが構造上限を超過した",
            ));
        }
        let encoded = serde_json::to_vec(arguments).map_err(|_| {
            McpError::new(
                "mcp_tool_arguments_invalid",
                "MCP Tool argumentsを正本化できない",
            )
        })?;
        if encoded.len() > MAX_TOOL_ARGUMENT_BYTES {
            return Err(McpError::new(
                "mcp_tool_arguments_complexity_exceeded",
                "MCP Tool argumentsがbyte上限を超過した",
            ));
        }
        if contains_authority_key(arguments) {
            return Err(McpError::new(
                "mcp_tool_authority_injection",
                "MCP Tool argumentsへ権限fieldを持ち込めない",
            ));
        }
        let validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .build(&tool.input_schema)
            .map_err(|_| {
                McpError::new(
                    "mcp_tool_schema_invalid",
                    "MCP Tool inputSchema validatorを再構築できない",
                )
            })?;
        if !validator.is_valid(arguments) {
            return Err(McpError::new(
                "mcp_tool_arguments_schema_invalid",
                "MCP Tool argumentsがCatalogのinputSchemaに適合しない",
            ));
        }
        Ok(())
    }
}

pub fn parse_tools_response(message: &McpJsonRpcMessage) -> Result<Vec<McpTool>, McpError> {
    let array = list_array(message, "tools")?;
    if array.len() > MAX_CATALOG_ITEMS {
        return Err(McpError::new(
            "mcp_catalog_oversized",
            "MCP Tool catalogが上限を超過した",
        ));
    }
    let mut names = BTreeSet::new();
    let mut tools = Vec::with_capacity(array.len());
    for value in array {
        let object = value
            .as_object()
            .ok_or_else(|| McpError::new("mcp_tool_invalid", "MCP Toolがobjectではない"))?;
        reject_authority_fields(object)?;
        let name = required_safe_string(object, "name")?;
        if !names.insert(name.to_string()) {
            return Err(McpError::new(
                "mcp_tool_duplicate",
                "MCP Tool nameが重複した",
            ));
        }
        let input_schema = object.get("inputSchema").ok_or_else(|| {
            McpError::new("mcp_tool_schema_invalid", "MCP Tool inputSchemaがない")
        })?;
        validate_tool_input_schema(input_schema)?;
        let tool_id = format!(
            "tool-{}",
            hex::encode(sha256_tagged(name.as_bytes()).as_bytes())
        );
        tools.push(McpTool {
            name: name.to_string(),
            tool_id,
            input_schema_hash: hash_value(input_schema)?,
            input_schema: input_schema.clone(),
        });
    }
    Ok(tools)
}

fn validate_tool_input_schema(schema: &Value) -> Result<(), McpError> {
    let invalid = || {
        McpError::new(
            "mcp_tool_schema_invalid",
            "MCP Tool inputSchemaがJSON Schema 2020-12として不正",
        )
    };
    let object = schema.as_object().ok_or_else(invalid)?;
    if object.get("type") != Some(&Value::String("object".to_string())) {
        return Err(McpError::new(
            "mcp_tool_schema_invalid",
            "MCP Tool inputSchemaのroot型がobjectではない",
        ));
    }
    if let Some(dialect) = object.get("$schema") {
        if !dialect.as_str().is_some_and(is_mcp_json_schema_dialect) {
            return Err(McpError::new(
                "mcp_tool_schema_dialect_unsupported",
                "MCP Tool inputSchemaのJSON Schema dialectは未対応",
            ));
        }
    }

    let encoded = serde_json::to_vec(schema).map_err(|_| invalid())?;
    if encoded.len() > MAX_TOOL_SCHEMA_BYTES {
        return Err(McpError::new(
            "mcp_tool_schema_oversized",
            "MCP Tool inputSchemaが上限を超過した",
        ));
    }

    let mut pending = vec![(schema, 1usize)];
    let mut nodes = 0usize;
    while let Some((value, depth)) = pending.pop() {
        nodes += 1;
        if nodes > MAX_TOOL_SCHEMA_NODES || depth > MAX_TOOL_SCHEMA_DEPTH {
            return Err(McpError::new(
                "mcp_tool_schema_complexity_exceeded",
                "MCP Tool inputSchemaの複雑度が上限を超過した",
            ));
        }
        match value {
            Value::Object(properties) => {
                for reference_keyword in ["$ref", "$dynamicRef"] {
                    if let Some(reference) = properties.get(reference_keyword) {
                        let reference = reference.as_str().ok_or_else(invalid)?;
                        if !reference.starts_with('#') {
                            return Err(McpError::new(
                                "mcp_tool_schema_external_reference_unsupported",
                                "MCP Tool inputSchemaの外部参照は取得しない",
                            ));
                        }
                    }
                }
                if properties.contains_key("$recursiveRef") {
                    return Err(McpError::new(
                        "mcp_tool_schema_dialect_unsupported",
                        "MCP Tool inputSchemaに旧dialect専用参照がある",
                    ));
                }
                for keyword in [
                    "additionalProperties",
                    "contains",
                    "contentSchema",
                    "else",
                    "if",
                    "items",
                    "not",
                    "propertyNames",
                    "then",
                    "unevaluatedItems",
                    "unevaluatedProperties",
                ] {
                    if let Some(child) = properties.get(keyword) {
                        if keyword == "items" {
                            if let Some(items) = child.as_array() {
                                pending.extend(items.iter().map(|item| (item, depth + 1)));
                            } else {
                                pending.push((child, depth + 1));
                            }
                        } else {
                            pending.push((child, depth + 1));
                        }
                    }
                }
                for keyword in [
                    "$defs",
                    "definitions",
                    "dependentSchemas",
                    "patternProperties",
                    "properties",
                ] {
                    if let Some(children) = properties.get(keyword).and_then(Value::as_object) {
                        pending.extend(children.values().map(|child| (child, depth + 1)));
                    }
                }
                for keyword in ["allOf", "anyOf", "oneOf", "prefixItems"] {
                    if let Some(children) = properties.get(keyword).and_then(Value::as_array) {
                        pending.extend(children.iter().map(|child| (child, depth + 1)));
                    }
                }
            }
            Value::Array(values) => {
                pending.extend(values.iter().map(|child| (child, depth + 1)));
            }
            _ => {}
        }
    }

    jsonschema::draft202012::meta::validate(schema).map_err(|_| invalid())?;
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(schema)
        .map_err(|_| invalid())?;
    Ok(())
}

pub fn parse_resources_response(message: &McpJsonRpcMessage) -> Result<Vec<McpResource>, McpError> {
    let array = list_array(message, "resources")?;
    if array.len() > MAX_CATALOG_ITEMS {
        return Err(McpError::new(
            "mcp_catalog_oversized",
            "MCP Resource catalogが上限を超過した",
        ));
    }
    let mut names = BTreeSet::new();
    let mut resources = Vec::with_capacity(array.len());
    for value in array {
        let object = value
            .as_object()
            .ok_or_else(|| McpError::new("mcp_resource_invalid", "MCP Resourceがobjectではない"))?;
        reject_authority_fields(object)?;
        let name = required_safe_string(object, "name")?;
        let uri = required_safe_string(object, "uri")?;
        if !names.insert(name.to_string()) {
            return Err(McpError::new(
                "mcp_resource_duplicate",
                "MCP Resource nameが重複した",
            ));
        }
        resources.push(McpResource {
            name: name.to_string(),
            resource_id: format!(
                "resource-{}",
                hex::encode(sha256_tagged(uri.as_bytes()).as_bytes())
            ),
            uri_hash: sha256_tagged(uri.as_bytes()),
        });
    }
    Ok(resources)
}

pub fn parse_prompts_response(message: &McpJsonRpcMessage) -> Result<Vec<McpPrompt>, McpError> {
    let array = list_array(message, "prompts")?;
    if array.len() > MAX_CATALOG_ITEMS {
        return Err(McpError::new(
            "mcp_catalog_oversized",
            "MCP Prompt catalogが上限を超過した",
        ));
    }
    let mut names = BTreeSet::new();
    let mut prompts = Vec::with_capacity(array.len());
    for value in array {
        let object = value
            .as_object()
            .ok_or_else(|| McpError::new("mcp_prompt_invalid", "MCP Promptがobjectではない"))?;
        reject_authority_fields(object)?;
        let name = required_safe_string(object, "name")?;
        if !names.insert(name.to_string()) {
            return Err(McpError::new(
                "mcp_prompt_duplicate",
                "MCP Prompt nameが重複した",
            ));
        }
        let arguments = object
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new()));
        if !arguments.is_array() {
            return Err(McpError::new(
                "mcp_prompt_schema_invalid",
                "MCP Prompt argumentsがarrayではない",
            ));
        }
        prompts.push(McpPrompt {
            name: name.to_string(),
            prompt_id: format!(
                "prompt-{}",
                hex::encode(sha256_tagged(name.as_bytes()).as_bytes())
            ),
            argument_schema_hash: hash_value(&arguments)?,
        });
    }
    Ok(prompts)
}

fn list_array<'a>(message: &'a McpJsonRpcMessage, key: &str) -> Result<&'a [Value], McpError> {
    if message.error.is_some() || message.result.is_none() {
        return Err(McpError::new(
            "mcp_list_failed",
            "MCP list responseが失敗した",
        ));
    }
    let result = message
        .result
        .as_ref()
        .and_then(Value::as_object)
        .ok_or_else(|| McpError::new("mcp_list_invalid", "MCP list resultがobjectではない"))?;
    if result
        .get("nextCursor")
        .is_some_and(|value| !value.is_null())
    {
        return Err(McpError::new(
            "mcp_list_pagination_unhandled",
            "MCP list paginationを未処理のまま受理しない",
        ));
    }
    result
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| McpError::new("mcp_list_invalid", "MCP list arrayがない"))
}

fn required_safe_string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a str, McpError> {
    let value = object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::new("mcp_metadata_invalid", "MCP metadata stringがない"))?;
    if !safe_text(value, MAX_NAME_BYTES) {
        return Err(McpError::new(
            "mcp_metadata_invalid",
            "MCP metadata stringが不正",
        ));
    }
    Ok(value)
}

fn reject_authority_fields(object: &Map<String, Value>) -> Result<(), McpError> {
    const FORBIDDEN: &[&str] = &[
        "authority",
        "authority_id",
        "permission_id",
        "approval_id",
        "capability_grant",
        "secret",
        "secret_value",
        "token",
        "password",
        "credential_value",
    ];
    if object.keys().any(|key| FORBIDDEN.contains(&key.as_str())) {
        return Err(McpError::new(
            "mcp_metadata_authority_injection",
            "MCP metadataへ権限または秘密値を持ち込めない",
        ));
    }
    Ok(())
}

fn contains_authority_key(value: &Value) -> bool {
    match value {
        Value::Object(object) => {
            object.keys().any(|key| {
                matches!(
                    key.as_str(),
                    "authority"
                        | "authority_id"
                        | "permission_id"
                        | "approval_id"
                        | "capability_grant"
                        | "secret"
                        | "secret_value"
                        | "token"
                        | "password"
                        | "credential_value"
                )
            }) || object.values().any(contains_authority_key)
        }
        Value::Array(values) => values.iter().any(contains_authority_key),
        _ => false,
    }
}

fn json_value_within_limits(root: &Value, maximum_nodes: usize, maximum_depth: usize) -> bool {
    let mut pending = vec![(root, 1usize)];
    let mut nodes = 0usize;
    while let Some((value, depth)) = pending.pop() {
        nodes += 1;
        if nodes > maximum_nodes || depth > maximum_depth {
            return false;
        }
        match value {
            Value::Object(object) => {
                pending.extend(object.values().map(|child| (child, depth + 1)));
            }
            Value::Array(values) => {
                pending.extend(values.iter().map(|child| (child, depth + 1)));
            }
            _ => {}
        }
    }
    true
}

fn hash_value(value: &Value) -> Result<String, McpError> {
    let bytes = serde_json::to_vec(value).map_err(|_| {
        McpError::new(
            "mcp_metadata_hash_failed",
            "MCP metadata hashを生成できない",
        )
    })?;
    Ok(sha256_tagged(&bytes))
}

fn validate_credential_ref(value: &Value) -> Result<(), McpError> {
    let object = value.as_object().ok_or_else(|| {
        McpError::new(
            "mcp_credential_ref_invalid",
            "MCP Credential refがobjectではない",
        )
    })?;
    if object.keys().any(|key| {
        matches!(
            key.as_str(),
            "secret" | "secret_value" | "token" | "password" | "credential_value"
        )
    }) {
        return Err(McpError::new(
            "mcp_credential_leak",
            "MCP Credential refへ実値を投影しない",
        ));
    }
    Ok(())
}

fn safe_text(value: &str, max_bytes: usize) -> bool {
    !value.is_empty() && value.as_bytes().len() <= max_bytes && !value.chars().any(char::is_control)
}

fn is_hash(value: &str) -> bool {
    let Some(hex_part) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex_part.len() == 64
        && hex_part
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(id: u64, result: Value) -> McpJsonRpcMessage {
        McpJsonRpcMessage::parse_line(
            serde_json::to_string(&json!({"jsonrpc": "2.0", "id": id, "result": result}))
                .expect("応答JSON")
                .as_bytes(),
        )
        .expect("応答解析")
    }

    fn catalog_with_tools(tools: Value) -> McpCatalog {
        McpCatalog {
            discovery: parse_discovery(
                &response(
                    1,
                    json!({
                        "supportedVersions": [MODERN_PROTOCOL_VERSION],
                        "capabilities": {"tools": {}}
                    }),
                ),
                McpProtocolEra::Modern,
            )
            .expect("discovery"),
            tools: parse_tools_response(&response(2, tools)).expect("Tool一覧"),
            resources: Vec::new(),
            prompts: Vec::new(),
        }
    }

    #[test]
    fn modern_discovery_and_catalog_are_metadata_only() {
        let discovered = response(
            1,
            json!({
                "supportedVersions": [MODERN_PROTOCOL_VERSION],
                "capabilities": {"tools": {}},
            }),
        );
        let mut discovered = discovered;
        discovered.metadata = Some(
            json!({"io.modelcontextprotocol/serverInfo": {"name": "fixture", "version": "1"}}),
        );
        let discovery =
            parse_discovery(&discovered, McpProtocolEra::Modern).expect("現行discovery");
        let tools = parse_tools_response(&response(
            2,
            json!({"tools": [{
                "name": "read",
                    "description": "表示してはいけない説明",
                "inputSchema": {
                    "type": "object",
                    "description": "表示してはいけないSchema本文",
                    "properties": {}
                }
            }]}),
        ))
        .expect("tools");
        let catalog = McpCatalog {
            discovery,
            tools,
            resources: Vec::new(),
            prompts: Vec::new(),
        };
        let projection = catalog
            .to_metadata_projection(
                "mcp-fixture",
                "stdio",
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                json!({
                    "credential_id": "11111111111111111111111111111111",
                    "purpose": "mcp_transport",
                    "target": "fixture",
                    "required": false,
                    "status": "missing"
                }),
            )
            .expect("射影");
        let encoded = serde_json::to_string(&projection).expect("射影JSON");
        assert!(!encoded.contains("表示してはいけない説明"));
        assert!(!encoded.contains("表示してはいけないSchema本文"));
        assert_eq!(projection["権限生成"], "なし");
        assert_eq!(projection["Trust"]["state"], "unverified");
    }

    #[test]
    fn modern_request_declares_required_client_capabilities_and_classifies_fallback_error() {
        let request = modern_discover_request(7).expect("新protocol要求");
        let request: Value = serde_json::from_slice(&request).expect("要求JSON");
        assert_eq!(
            request["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"],
            MODERN_PROTOCOL_VERSION
        );
        assert_eq!(
            request["params"]["_meta"]["io.modelcontextprotocol/clientInfo"]["name"],
            "gui-shell"
        );
        assert_eq!(
            request["params"]["_meta"]["io.modelcontextprotocol/clientCapabilities"],
            json!({})
        );

        let unsupported = McpJsonRpcMessage::parse_line(
            br#"{"jsonrpc":"2.0","id":7,"error":{"code":-32601,"message":"method not found"}}"#,
        )
        .expect("未対応method応答");
        assert!(unsupported.is_method_not_found_error());

        let invalid_params = McpJsonRpcMessage::parse_line(
            br#"{"jsonrpc":"2.0","id":7,"error":{"code":-32602,"message":"invalid params"}}"#,
        )
        .expect("parameter不正応答");
        assert!(!invalid_params.is_method_not_found_error());

        let unknown_result_type = McpJsonRpcMessage::parse_line(
            br#"{"jsonrpc":"2.0","id":7,"result":{"resultType":"input_required"}}"#,
        )
        .expect("未対応resultType応答");
        assert_eq!(
            unknown_result_type
                .validate_complete_result()
                .expect_err("unsupported resultType")
                .code,
            "mcp_result_type_unsupported"
        );
    }

    #[test]
    fn malformed_unknown_and_injected_mcp_messages_are_rejected() {
        assert_eq!(
            McpJsonRpcMessage::parse_line(br#"{"jsonrpc":"2.0","id":1,"result":{}} trailing"#)
                .expect_err("malformed")
                .code,
            "mcp_wire_malformed_json"
        );
        let injected = response(
            1,
            json!({"tools": [{
                "name": "danger",
                "inputSchema": {"type": "object"},
                "permission_id": "permission.injected"
            }]}),
        );
        assert_eq!(
            parse_tools_response(&injected)
                .expect_err("authority injection")
                .code,
            "mcp_metadata_authority_injection"
        );
        let unknown = McpCatalog {
            discovery: parse_discovery(
                &response(
                    1,
                    json!({"protocolVersion": LEGACY_PROTOCOL_VERSION, "capabilities": {}}),
                ),
                McpProtocolEra::Legacy,
            )
            .expect("旧discovery"),
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
        };
        assert_eq!(
            unknown
                .validate_tool_call("missing", &json!({}))
                .expect_err("未知Tool")
                .code,
            "mcp_unknown_tool"
        );
    }

    #[test]
    fn tool_arguments_must_match_the_registered_input_schema() {
        let catalog = catalog_with_tools(json!({"tools": [{
            "name": "search",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "minLength": 1},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 20}
                },
                "required": ["query"],
                "additionalProperties": false
            }
        }]}));

        assert!(catalog
            .validate_tool_call("search", &json!({"query": "needle", "limit": 5}))
            .is_ok());
        for (arguments, expected_code) in [
            (json!({}), "mcp_tool_arguments_schema_invalid"),
            (json!({"query": 7}), "mcp_tool_arguments_schema_invalid"),
            (
                json!({"query": "needle", "limit": 21}),
                "mcp_tool_arguments_schema_invalid",
            ),
            (
                json!({"query": "needle", "extra": true}),
                "mcp_tool_arguments_schema_invalid",
            ),
            (
                json!({"query": "needle", "context": {"credential_value": "marker"}}),
                "mcp_tool_authority_injection",
            ),
        ] {
            assert_eq!(
                catalog
                    .validate_tool_call("search", &arguments)
                    .expect_err("Schema不適合または権限field注入")
                    .code,
                expected_code
            );
        }
    }

    #[test]
    fn tool_arguments_are_bounded_before_schema_validation() {
        let catalog = catalog_with_tools(json!({"tools": [{
            "name": "accept",
            "inputSchema": {"type": "object"}
        }]}));
        let oversized = json!({"value": "x".repeat(MAX_TOOL_ARGUMENT_BYTES)});
        assert_eq!(
            catalog
                .validate_tool_call("accept", &oversized)
                .expect_err("byte上限")
                .code,
            "mcp_tool_arguments_complexity_exceeded"
        );

        let mut deeply_nested = Value::Null;
        for _ in 0..MAX_TOOL_ARGUMENT_DEPTH {
            deeply_nested = json!([deeply_nested]);
        }
        assert_eq!(
            catalog
                .validate_tool_call("accept", &json!({"value": deeply_nested}))
                .expect_err("深さ上限")
                .code,
            "mcp_tool_arguments_complexity_exceeded"
        );

        let mut many_values = Map::new();
        for index in 0..MAX_TOOL_ARGUMENT_NODES {
            many_values.insert(format!("p{index}"), Value::Null);
        }
        assert_eq!(
            catalog
                .validate_tool_call("accept", &Value::Object(many_values))
                .expect_err("node上限")
                .code,
            "mcp_tool_arguments_complexity_exceeded"
        );
    }

    #[test]
    fn tool_input_schema_is_meta_validated_bounded_and_never_fetches_external_refs() {
        let valid = response(
            1,
            json!({"tools": [{
                "name": "read",
                "inputSchema": {
                    "$schema": MCP_JSON_SCHEMA_DIALECT,
                    "type": "object",
                    "properties": {"path": {"$ref": "#/$defs/nonempty"}},
                    "$defs": {"nonempty": {"type": "string", "minLength": 1}},
                    "default": {"$ref": "https://example.invalid/ordinary-data.json"},
                    "required": ["path"],
                    "additionalProperties": false
                }
            }]}),
        );
        assert_eq!(parse_tools_response(&valid).expect("正しいSchema").len(), 1);

        let default_dialect = response(
            7,
            json!({"tools": [{
                "name": "default-dialect",
                "inputSchema": {
                    "type": "object",
                    "properties": {"value": {"type": "string", "minLength": 1}}
                }
            }]}),
        );
        assert_eq!(
            parse_tools_response(&default_dialect)
                .expect("$schema省略時はDraft 2020-12")
                .len(),
            1
        );

        let standard_http_alias = response(
            6,
            json!({"tools": [{
                "name": "http-alias",
                "inputSchema": {
                    "$schema": "http://json-schema.org/draft/2020-12/schema#",
                    "type": "object"
                }
            }]}),
        );
        assert_eq!(
            parse_tools_response(&standard_http_alias)
                .expect("既知の2020-12 URI表記")
                .len(),
            1
        );

        let malformed = response(
            2,
            json!({"tools": [{
                "name": "bad",
                "inputSchema": {"type": "object", "properties": {"x": {"type": "not-a-type"}}}
            }]}),
        );
        assert_eq!(
            parse_tools_response(&malformed)
                .expect_err("不正なJSON Schema")
                .code,
            "mcp_tool_schema_invalid"
        );

        let external_ref = response(
            3,
            json!({"tools": [{
                "name": "remote",
                "inputSchema": {"type": "object", "$ref": "https://example.invalid/schema.json"}
            }]}),
        );
        assert_eq!(
            parse_tools_response(&external_ref)
                .expect_err("外部参照は取得しない")
                .code,
            "mcp_tool_schema_external_reference_unsupported"
        );

        let nested_external_ref = response(
            5,
            json!({"tools": [{
                "name": "nested-remote",
                "inputSchema": {"type": "object", "properties": {"value": {
                    "$ref": "https://example.invalid/schema.json"
                }}}
            }]}),
        );
        assert_eq!(
            parse_tools_response(&nested_external_ref)
                .expect_err("Schema内の外部参照")
                .code,
            "mcp_tool_schema_external_reference_unsupported"
        );

        let unsupported_dialect = response(
            4,
            json!({"tools": [{
                "name": "legacy",
                "inputSchema": {
                    "$schema": "http://json-schema.org/draft-07/schema#",
                    "type": "object"
                }
            }]}),
        );
        assert_eq!(
            parse_tools_response(&unsupported_dialect)
                .expect_err("未対応dialect")
                .code,
            "mcp_tool_schema_dialect_unsupported"
        );

        let mut deep_schema = json!({"type": "object"});
        for _ in 0..MAX_TOOL_SCHEMA_DEPTH {
            deep_schema = json!({"type": "object", "properties": {"nested": deep_schema}});
        }
        assert_eq!(
            validate_tool_input_schema(&deep_schema)
                .expect_err("複雑度上限")
                .code,
            "mcp_tool_schema_complexity_exceeded"
        );

        let oversized = json!({
            "type": "object",
            "description": "x".repeat(MAX_TOOL_SCHEMA_BYTES)
        });
        assert_eq!(
            validate_tool_input_schema(&oversized)
                .expect_err("byte上限")
                .code,
            "mcp_tool_schema_oversized"
        );

        let mut properties = Map::new();
        for index in 0..MAX_TOOL_SCHEMA_NODES {
            properties.insert(format!("p{index}"), Value::Bool(true));
        }
        let too_many_nodes = json!({"type": "object", "properties": properties});
        assert_eq!(
            validate_tool_input_schema(&too_many_nodes)
                .expect_err("node上限")
                .code,
            "mcp_tool_schema_complexity_exceeded"
        );
    }

    #[test]
    fn pagination_and_credential_value_fail_closed() {
        let paginated = response(1, json!({"tools": [], "nextCursor": "next"}));
        assert_eq!(
            parse_tools_response(&paginated)
                .expect_err("pagination")
                .code,
            "mcp_list_pagination_unhandled"
        );
        let discovery = parse_discovery(
            &response(
                1,
                json!({"protocolVersion": LEGACY_PROTOCOL_VERSION, "capabilities": {}}),
            ),
            McpProtocolEra::Legacy,
        )
        .expect("旧discovery");
        let catalog = McpCatalog {
            discovery,
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
        };
        assert_eq!(
            catalog
                .to_metadata_projection(
                    "mcp-fixture",
                    "stdio",
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    json!({"credential_id": "1", "secret_value": "never"}),
                )
                .expect_err("credential leak")
                .code,
            "mcp_credential_leak"
        );
    }
}
