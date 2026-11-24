//! D4 PocketのGUI Shell構成Manifestを生成するBroker経路。
//!
//! この段階はManifest-onlyであり、build、独立App identity、Credential、
//! Permission、Approval、Audit chainの継承を行わない。構成入力はBrokerで
//! Schema相当の構造と境界を再検証し、受理結果だけを監査へ記録する。
#![allow(non_snake_case)]

use super::protocol::{Broker, BrokerResponse, BrokerStatus, EVIDENCE_SOURCE_INTERNAL_STATE};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const VERSION: u64 = 1;
const CATALOG_VERSION: u64 = 3;
const OP_COMPOSE: &str = "GUI Shell構成";
const OP_CATALOG: &str = "GUI Shell構成Catalog";
const OP_PREVIEW: &str = "GUI Shell構成Preview";
const MAX_COMPOSE_TOOLS: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Theme {
    theme_id: String,
    mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Settings {
    locale: String,
    density: String,
    content_visibility: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InheritancePolicy {
    authority: String,
    permission: String,
    approval: String,
    credential: String,
    audit_chain: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ComposeManifest {
    version: u64,
    compose_id: String,
    display_name: String,
    runtime_ids: Vec<String>,
    agent_ids: Vec<String>,
    tool_ids: Vec<String>,
    mcp_connection_ids: Vec<String>,
    theme: Theme,
    capability_requirements: Vec<String>,
    settings: Settings,
    inheritance_policy: InheritancePolicy,
    output_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct McpToolComposition {
    pub(super) compose_id: String,
    pub(super) manifest_hash: String,
    pub(super) runtime_count: usize,
    pub(super) agent_count: usize,
    pub(super) tool_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct McpToolCompositionPreview {
    pub(super) composition: McpToolComposition,
    pub(super) catalog_snapshot_hash: String,
    pub(super) connection_binding_hash: String,
    pub(super) host_switch_generation: String,
    pub(super) dependency_preview: Value,
}

fn latest_host_switch_generation(broker: &Broker) -> String {
    broker
        .audit_events()
        .iter()
        .rev()
        .find(|event| {
            matches!(event.operation.as_str(), "Host切替" | "Host停止")
                && event.decision == "accepted"
        })
        .map(|event| crate::audit_hash::sha256_tagged(event.event_id.as_bytes()))
        .unwrap_or_else(|| crate::audit_hash::sha256_tagged(b"host-context:no-switch"))
}

#[cfg(feature = "module_host_capabilities")]
fn local_host_catalog_snapshot() -> Value {
    super::host_capability::current()
}

#[cfg(not(feature = "module_host_capabilities"))]
fn local_host_catalog_snapshot() -> Value {
    let platform = match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        "linux" => "linux",
        "android" => "android",
        "ios" => "ios",
        _ => "unknown",
    };
    json!({
        "ホストID": format!("gui-shell-local-{platform}"),
        "表示名": format!("ローカル {platform}"),
        "プラットフォーム": platform,
        "状態": "unknown",
        "能力": [],
    })
}

/// Manifest選択候補の現在metadataを一度だけ正規化する。
/// capability宣言、接続、Host metadataはいずれもPermissionや実行Authorityを発行しない。
fn catalog_snapshot(broker: &mut Broker) -> Result<Value, String> {
    let runtime_agent_catalog = broker
        .compose_runtime_agent_catalog()
        .map_err(|_| "現在のRuntime／Agent Catalogを安全に投影できない".to_string())?;
    if runtime_agent_catalog.runtime_ids.len() > 64
        || runtime_agent_catalog.agents.len() > 64
        || broker.mcp_connections.len() > 64
        || broker.hosts.len() > 64
    {
        return Err("Runtime／Agent／Tool／Host Catalogの件数上限を超過した".to_string());
    }

    let agents = runtime_agent_catalog
        .agents
        .iter()
        .map(|agent| json!({
            "runtime_id": agent.runtime_id,
            "metadata": agent.metadata,
            "dependencies": {"status":"partial", "items":[{"kind":"runtime", "id":agent.runtime_id}]}
        }))
        .collect::<Vec<_>>();
    let runtimes = runtime_agent_catalog
        .runtime_ids
        .iter()
        .map(|runtime_id| {
            let metadata = runtime_agent_catalog
                .agents
                .iter()
                .find(|agent| agent.runtime_id == *runtime_id)
                .map(|agent| &agent.metadata);
            let host_requirements = metadata
                .and_then(|value| value.get("host_requirements"))
                .cloned()
                .unwrap_or_else(|| json!({
                    "platforms": ["unknown"],
                    "network_scope": "unknown",
                    "process_spawn": {"status":"unknown", "reason":"Runtime metadataがない"}
                }));
            let workspace_requirements = metadata
                .and_then(|value| value.get("workspace_requirements"))
                .cloned()
                .unwrap_or_else(|| json!({"status":"unknown"}));
            json!({
                "runtime_id": runtime_id,
                "adapter_id": metadata.and_then(|value| value.get("adapter_id")).cloned().unwrap_or(Value::Null),
                "version": metadata.and_then(|value| value.get("version")).and_then(Value::as_str).unwrap_or("unknown"),
                "abi": "unknown",
                "architecture": "unknown",
                "os": host_requirements.get("platforms").cloned().unwrap_or_else(|| json!(["unknown"])),
                "status": metadata.and_then(|value| value.get("status")).and_then(Value::as_str).unwrap_or("unknown"),
                "capabilities": metadata.and_then(|value| value.get("capabilities")).cloned().unwrap_or_else(|| json!([])),
                "permissions": {"status":"not_granted", "requirements":[]},
                "dependencies": metadata
                    .and_then(|value| value.get("adapter_id"))
                    .and_then(Value::as_str)
                    .map(|adapter_id| json!({"status":"partial", "items":[{"kind":"adapter", "id":adapter_id}]}))
                    .unwrap_or_else(|| json!({"status":"unknown", "items":[]})),
                "scope": {"workspace":workspace_requirements, "host":host_requirements},
                "evidence_source": metadata.and_then(|value| value.get("evidence_source")).and_then(Value::as_str).unwrap_or(EVIDENCE_SOURCE_INTERNAL_STATE),
            })
        })
        .collect::<Vec<_>>();

    let credential_status_by_server = broker
        .mcp_connections
        .iter()
        .map(|(server_id, entry)| {
            let reference = entry.projection.get("Credential ref");
            let status = match reference
                .and_then(|value| value.get("status"))
                .and_then(Value::as_str)
            {
                Some("missing") => "missing",
                Some("configured") => {
                    let credential_id = reference
                        .and_then(|value| value.get("credential_id"))
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if !credential_id.is_empty()
                        && broker.MCP資格情報有効確認(credential_id, server_id)
                    {
                        "configured"
                    } else {
                        "revoked"
                    }
                }
                _ => "unknown",
            };
            (server_id.clone(), status.to_string())
        })
        .collect::<BTreeMap<_, _>>();

    let mut tools = Vec::new();
    for (server_id, entry) in &mut broker.mcp_connections {
        let connection_status = if entry.quarantined {
            "quarantined"
        } else if entry.connection.is_alive() {
            "connected"
        } else {
            return Err("未隔離MCP Serverのprocess状態を確認できない".to_string());
        };
        let catalog = entry.connection.catalog();
        let server = entry.projection.get("Server");
        let transport = entry
            .projection
            .get("Transport")
            .and_then(|value| value.get("kind"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let credential_status = credential_status_by_server
            .get(server_id)
            .map(String::as_str)
            .unwrap_or("unknown");
        for tool in &catalog.tools {
            tools.push(json!({
                "server_id": server_id,
                "server_name": server.and_then(|value| value.get("表示名")).and_then(Value::as_str).unwrap_or("unknown"),
                "server_version": catalog.discovery.server_version.as_deref().unwrap_or("unknown"),
                "protocol_version": catalog.discovery.protocol_version,
                "tool_id": tool.tool_id,
                "name": tool.name,
                "input_schema_hash": tool.input_schema_hash,
                "status": connection_status,
                "authentication_status": credential_status,
                "abi": "unknown",
                "os": ["unknown"],
                "dependencies": {"status":"partial", "items":[{"kind":"mcp_server", "id":server_id}]},
                "scope": {"transport":transport, "boundary":"current_registered_connection"},
                "permission_requirement": "separate_owner_confirmation_and_one_shot_permission",
                "permission_generated": false,
                "evidence_source": "LIVE_RUNTIME",
            }));
            if tools.len() > MAX_COMPOSE_TOOLS {
                return Err("MCP Tool Catalogの件数上限を超過した".to_string());
            }
        }
    }

    let current_host = local_host_catalog_snapshot();
    let local_host = json!({
        "host_id": current_host.get("ホストID").cloned().unwrap_or(Value::Null),
        "display_name": current_host.get("表示名").cloned().unwrap_or(Value::Null),
        "kind": "local",
        "version": "unknown",
        "abi": "unknown",
        "architecture": std::env::consts::ARCH,
        "os": current_host.get("プラットフォーム").cloned().unwrap_or_else(|| json!("unknown")),
        "status": current_host.get("状態").cloned().unwrap_or_else(|| json!("unknown")),
        "connection_state": "local",
        "trust_state": "local_runtime_only",
        "permissions": {"status":"not_granted", "requirements":[]},
        "dependencies": {"status":"unknown", "items":[]},
        "scope": {"capabilities":current_host.get("能力").cloned().unwrap_or_else(|| json!([]))},
        "evidence_source": "LIVE_RUNTIME",
        "authority_strip": true,
    });
    let mut hosts = vec![local_host];
    hosts.extend(broker.hosts.iter().map(|(host_id, receipt)| {
        json!({
            "host_id": host_id,
            "display_name": receipt.get("表示名").cloned().unwrap_or(Value::Null),
            "kind": "registered_metadata",
            "version": "unknown",
            "abi": "unknown",
            "architecture": "unknown",
            "os": receipt.get("Platform").cloned().unwrap_or_else(|| json!("unknown")),
            "status": "metadata_only",
            "connection_state": receipt.get("接続状態").cloned().unwrap_or_else(|| json!("unknown")),
            "trust_state": receipt.get("Trust").and_then(|value| value.get("state")).cloned().unwrap_or_else(|| json!("unknown")),
            "permissions": {"status":"not_granted", "requirements":[]},
            "dependencies": {"status":"unknown", "items":[]},
            "scope": {"boundary":"metadata_only", "runtime_summary":receipt.get("Runtime summary").cloned().unwrap_or(Value::Null)},
            "evidence_source": EVIDENCE_SOURCE_INTERNAL_STATE,
            "authority_strip": true,
        })
    }));

    let host_switch_generation = latest_host_switch_generation(broker);
    let content = json!({
        "version": CATALOG_VERSION,
        "runtime_ids": runtime_agent_catalog.runtime_ids,
        "runtimes": runtimes,
        "agents": agents,
        "tools": tools,
        "hosts": hosts,
        "host_switch_generation": host_switch_generation,
        "evidence_source": EVIDENCE_SOURCE_INTERNAL_STATE,
        "authority_strip": true,
        "permission_generated": false,
    });
    validate_dependency_catalog(&content)?;
    let encoded =
        serde_json::to_vec(&content).map_err(|_| "Compose Catalogを正規化できない".to_string())?;
    if encoded.len() > 1024 * 1024 {
        return Err("Compose Catalogが1 MiBの応答上限を超過した".to_string());
    }
    let snapshot_hash = crate::audit_hash::sha256_tagged(&encoded);
    let mut body = content
        .as_object()
        .cloned()
        .ok_or_else(|| "Compose Catalogがobjectではない".to_string())?;
    body.insert("snapshot_hash".to_string(), Value::String(snapshot_hash));
    Ok(Value::Object(body))
}

fn dependency_node_key(kind: &str, id: &str) -> String {
    format!("{kind}\0{id}")
}

fn valid_catalog_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'))
}

fn validate_dependency_edges(
    nodes: &BTreeSet<String>,
    edges: &BTreeSet<(String, String)>,
) -> Result<(), String> {
    let mut indegree = nodes
        .iter()
        .map(|node| (node.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = BTreeMap::<String, Vec<String>>::new();
    for (source, target) in edges {
        if !nodes.contains(source) || !nodes.contains(target) {
            return Err("Compose Catalogの依存先が現在の候補Catalogにありません".to_string());
        }
        *indegree
            .get_mut(target)
            .ok_or_else(|| "Compose Catalogの依存先がありません".to_string())? += 1;
        outgoing
            .entry(source.clone())
            .or_default()
            .push(target.clone());
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(node, degree)| (*degree == 0).then_some(node.clone()))
        .collect::<Vec<_>>();
    let mut visited = 0usize;
    while let Some(node) = ready.pop() {
        visited += 1;
        for dependency in outgoing.get(&node).into_iter().flatten() {
            let degree = indegree
                .get_mut(dependency)
                .ok_or_else(|| "Compose Catalogの依存先がありません".to_string())?;
            *degree -= 1;
            if *degree == 0 {
                ready.push(dependency.clone());
            }
        }
    }
    if visited != nodes.len() {
        return Err("Compose Catalogの依存関係に循環があります".to_string());
    }
    Ok(())
}

fn validate_dependency_catalog(catalog: &Value) -> Result<(), String> {
    let runtimes = catalog
        .get("runtimes")
        .and_then(Value::as_array)
        .ok_or_else(|| "Compose CatalogのRuntime一覧が不正です".to_string())?;
    let agents = catalog
        .get("agents")
        .and_then(Value::as_array)
        .ok_or_else(|| "Compose CatalogのAgent一覧が不正です".to_string())?;
    let tools = catalog
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| "Compose CatalogのTool一覧が不正です".to_string())?;
    let hosts = catalog
        .get("hosts")
        .and_then(Value::as_array)
        .ok_or_else(|| "Compose CatalogのHost一覧が不正です".to_string())?;

    let mut nodes = BTreeSet::new();
    let mut edges = BTreeSet::new();
    let mut runtime_ids = BTreeSet::new();
    let mut adapter_ids = BTreeSet::new();
    for runtime in runtimes {
        let runtime_id = runtime
            .get("runtime_id")
            .and_then(Value::as_str)
            .filter(|value| valid_catalog_id(value))
            .ok_or_else(|| "Compose CatalogのRuntime依存元IDが不正です".to_string())?;
        if !runtime_ids.insert(runtime_id.to_string())
            || !nodes.insert(dependency_node_key("runtime", runtime_id))
        {
            return Err("Compose CatalogのRuntime依存元が重複しています".to_string());
        }
    }
    for agent in agents {
        let runtime_id = agent
            .get("runtime_id")
            .and_then(Value::as_str)
            .filter(|value| valid_catalog_id(value))
            .ok_or_else(|| "Compose CatalogのAgent依存元IDが不正です".to_string())?;
        let adapter_id = agent
            .get("metadata")
            .and_then(|metadata| metadata.get("adapter_id"))
            .and_then(Value::as_str)
            .filter(|value| valid_catalog_id(value))
            .ok_or_else(|| "Compose CatalogのAgent Adapter依存IDが不正です".to_string())?;
        if !runtime_ids.contains(runtime_id) {
            return Err("Compose CatalogのAgentが存在しないRuntimeを参照しています".to_string());
        }
        let runtime = runtimes
            .iter()
            .find(|runtime| runtime.get("runtime_id").and_then(Value::as_str) == Some(runtime_id))
            .ok_or_else(|| "Compose CatalogのAgent Runtime依存先が存在しません".to_string())?;
        if runtime.get("adapter_id").and_then(Value::as_str) != Some(adapter_id) {
            return Err("Compose CatalogのRuntimeとAgent Adapter依存が一致しません".to_string());
        }
        adapter_ids.insert(adapter_id.to_string());
        if !nodes.insert(dependency_node_key("agent_instance", runtime_id)) {
            return Err("Compose CatalogのAgent依存元が重複しています".to_string());
        }
    }
    for adapter_id in &adapter_ids {
        nodes.insert(dependency_node_key("adapter", adapter_id));
    }
    for runtime in runtimes {
        let runtime_id = runtime
            .get("runtime_id")
            .and_then(Value::as_str)
            .ok_or_else(|| "Compose CatalogのRuntime依存元IDが不正です".to_string())?;
        match runtime.get("adapter_id").and_then(Value::as_str) {
            Some(adapter_id) if adapter_ids.contains(adapter_id) => {
                let expected = json!({
                    "status":"partial",
                    "items":[{"kind":"adapter", "id":adapter_id}]
                });
                if runtime.get("dependencies") != Some(&expected) {
                    return Err("Compose CatalogのRuntime Adapter依存が不一致です".to_string());
                }
                edges.insert((
                    dependency_node_key("runtime", runtime_id),
                    dependency_node_key("adapter", adapter_id),
                ));
            }
            Some(_) => {
                return Err("Compose CatalogのRuntime Adapter依存先が存在しません".to_string())
            }
            None => {
                if runtime.get("dependencies") != Some(&json!({"status":"unknown", "items":[]})) {
                    return Err("Compose CatalogのRuntime未知依存状態が不正です".to_string());
                }
            }
        }
    }
    for agent in agents {
        let runtime_id = agent
            .get("runtime_id")
            .and_then(Value::as_str)
            .ok_or_else(|| "Compose CatalogのAgent依存元IDが不正です".to_string())?;
        if agent.get("dependencies")
            != Some(&json!({
                "status":"partial",
                "items":[{"kind":"runtime", "id":runtime_id}]
            }))
        {
            return Err("Compose CatalogのAgent Runtime依存が不一致です".to_string());
        }
        edges.insert((
            dependency_node_key("agent_instance", runtime_id),
            dependency_node_key("runtime", runtime_id),
        ));
    }
    for tool in tools {
        let server_id = tool
            .get("server_id")
            .and_then(Value::as_str)
            .filter(|value| valid_catalog_id(value))
            .ok_or_else(|| "Compose CatalogのTool Server依存IDが不正です".to_string())?;
        let tool_id = tool
            .get("tool_id")
            .and_then(Value::as_str)
            .filter(|value| valid_catalog_id(value))
            .ok_or_else(|| "Compose CatalogのTool依存元IDが不正です".to_string())?;
        let node_id = format!(
            "{}_{}_{}_{}",
            server_id.len(),
            server_id,
            tool_id.len(),
            tool_id
        );
        nodes.insert(dependency_node_key("mcp_server", server_id));
        if !nodes.insert(dependency_node_key("tool", &node_id)) {
            return Err("Compose CatalogのMCP Tool依存元が重複しています".to_string());
        }
        let expected = json!({
            "status":"partial",
            "items":[{"kind":"mcp_server", "id":server_id}]
        });
        if tool.get("dependencies") != Some(&expected) {
            return Err("Compose CatalogのMCP Tool Server依存が不一致です".to_string());
        }
        edges.insert((
            dependency_node_key("tool", &node_id),
            dependency_node_key("mcp_server", server_id),
        ));
    }
    for host in hosts {
        let host_id = host
            .get("host_id")
            .and_then(Value::as_str)
            .filter(|value| valid_catalog_id(value))
            .ok_or_else(|| "Compose CatalogのHost依存IDが不正です".to_string())?;
        if !nodes.insert(dependency_node_key("host", host_id)) {
            return Err("Compose CatalogのHost依存元が重複しています".to_string());
        }
        if host.get("dependencies") != Some(&json!({"status":"unknown", "items":[]})) {
            return Err("Compose CatalogのHost依存はunknownでなければなりません".to_string());
        }
    }
    validate_dependency_edges(&nodes, &edges)
}

pub(super) fn catalog(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    payload_hash: &str,
) -> BrokerResponse {
    if !payload.as_object().is_some_and(|object| object.is_empty()) {
        return broker.reject_with_payload_hash(
            request_id,
            OP_CATALOG,
            "gui_shell_compose_catalog_invalid",
            "Compose Catalog要求の構造が不正または禁止fieldがある",
            true,
            payload_hash,
        );
    }
    let body = match catalog_snapshot(broker) {
        Ok(body) => body,
        Err(reason) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_CATALOG,
                "gui_shell_compose_catalog_unavailable",
                &reason,
                true,
                payload_hash,
            )
        }
    };
    let evidence_source = EVIDENCE_SOURCE_INTERNAL_STATE;
    let audit = match broker.append_audit(
        request_id,
        OP_CATALOG,
        "accepted",
            "現在のRuntime／Agent／Tool／Host metadataをCompose候補へ投影。snapshot hashで更新後の変更を拒否し、Permission／Trust／Authorityを生成しない",
        evidence_source,
        payload_hash,
    ) {
        Ok(event) => event,
        Err(error) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_CATALOG,
                "broker_audit_append_failed",
                &error.message(),
            )
        }
    };
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OP_CATALOG.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: evidence_source.to_string(),
        audit_event_id: audit.event_id.clone(),
        error: None,
        health: None,
        body: Some(body),
        shutdown_requested: broker.shutdown_requested,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreviewRequest {
    version: u64,
    current_manifest: Value,
    candidate_manifest: ComposeManifest,
    build_target: String,
    preview_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreviewInput {
    current_manifest: Option<ComposeManifest>,
    candidate_manifest: ComposeManifest,
    build_target: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ComposeRequest {
    version: u64,
    catalog_snapshot_hash: String,
    compose_manifest: Value,
}

fn valid_snapshot_hash(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn compose(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    _owner: bool,
    payload_hash: &str,
) -> BrokerResponse {
    let request = match serde_json::from_value::<ComposeRequest>(payload.clone()) {
        Ok(request)
            if request.version == VERSION
                && valid_snapshot_hash(&request.catalog_snapshot_hash) =>
        {
            request
        }
        _ => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_COMPOSE,
                "gui_shell_compose_invalid",
                "Compose要求はManifestと現在Catalog snapshot hashを必要とします",
                true,
                payload_hash,
            )
        }
    };
    let current_catalog = match catalog_snapshot(broker) {
        Ok(catalog) => catalog,
        Err(reason) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_COMPOSE,
                "gui_shell_compose_catalog_unavailable",
                &reason,
                true,
                payload_hash,
            )
        }
    };
    if current_catalog.get("snapshot_hash").and_then(Value::as_str)
        != Some(request.catalog_snapshot_hash.as_str())
    {
        return broker.reject_with_payload_hash(
            request_id,
            OP_COMPOSE,
            "gui_shell_compose_catalog_stale",
            "Runtime／Agent／Tool／Hostまたは認証状態が更新されています。現在Catalogを再取得してください",
            true,
            payload_hash,
        );
    }
    let manifest = match parse_manifest(&request.compose_manifest) {
        Ok(manifest) => manifest,
        Err(reason) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_COMPOSE,
                "gui_shell_compose_invalid",
                &reason,
                true,
                payload_hash,
            )
        }
    };
    if let Err(reason) = validate_catalog_selection(broker, &manifest) {
        return broker.reject_with_payload_hash(
            request_id,
            OP_COMPOSE,
            "gui_shell_compose_catalog_mismatch",
            &reason,
            true,
            payload_hash,
        );
    }
    let evidence_source = EVIDENCE_SOURCE_INTERNAL_STATE;

    let manifest_value = match serde_json::to_value(&manifest) {
        Ok(value) => value,
        Err(_) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_COMPOSE,
                "gui_shell_compose_serialize_failed",
                "構成Manifestを正規化できない",
                true,
                payload_hash,
            )
        }
    };
    let audit = match broker.append_audit(
        request_id,
        OP_COMPOSE,
        "accepted",
        "構成Manifestだけを生成。build、App identity、Credential、Permission、Approval、Audit chainを継承しない",
        evidence_source,
        payload_hash,
    ) {
        Ok(event) => event,
        Err(error) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_COMPOSE,
                "broker_audit_append_failed",
                &error.message(),
            )
        }
    };

    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OP_COMPOSE.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: evidence_source.to_string(),
        audit_event_id: audit.event_id.clone(),
        error: None,
        health: None,
        body: Some(json!({
            "version": VERSION,
            "operation": OP_COMPOSE,
            "status": "accepted",
            "compose_manifest": manifest_value,
            "build_status": "not_started",
            "app_identity_status": "not_generated",
            "permission_generated": false,
            "authority_strip": true,
            "inheritance_policy": {
                "authority": "none",
                "permission": "none",
                "approval": "none",
                "credential": "none",
                "audit_chain": "none"
            },
            "evidence_source": evidence_source,
            "audit_id": audit.event_id,
        })),
        shutdown_requested: broker.shutdown_requested,
    }
}

pub(super) fn preview(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    _owner: bool,
    payload_hash: &str,
) -> BrokerResponse {
    let input = match parse_preview(payload) {
        Ok(input) => input,
        Err(reason) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_PREVIEW,
                "gui_shell_preview_invalid",
                &reason,
                true,
                payload_hash,
            )
        }
    };
    if let Err(reason) = validate_catalog_selection(broker, &input.candidate_manifest) {
        return broker.reject_with_payload_hash(
            request_id,
            OP_PREVIEW,
            "gui_shell_preview_catalog_mismatch",
            &reason,
            true,
            payload_hash,
        );
    }
    let evidence_source = EVIDENCE_SOURCE_INTERNAL_STATE;
    let candidate_value = match serde_json::to_value(&input.candidate_manifest) {
        Ok(value) => value,
        Err(_) => {
            return broker.reject_with_payload_hash(
                request_id,
                OP_PREVIEW,
                "gui_shell_preview_serialize_failed",
                "Preview対象Manifestを正規化できない",
                true,
                payload_hash,
            )
        }
    };
    let current_value = input
        .current_manifest
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .ok()
        .flatten()
        .unwrap_or(Value::Null);
    let audit = match broker.append_audit(
        request_id,
        OP_PREVIEW,
        "accepted",
        "構成、機能要件、差分、版rollbackをPreview。build、Export、rollback、権限生成は実行しない",
        evidence_source,
        payload_hash,
    ) {
        Ok(event) => event,
        Err(error) => {
            return broker.audit_store_failed_response(
                request_id,
                OP_PREVIEW,
                "broker_audit_append_failed",
                &error.message(),
            )
        }
    };
    let permission_requirements: Vec<Value> = input
        .candidate_manifest
        .capability_requirements
        .iter()
        .map(|capability| {
            json!({
                "capability": capability,
                "permission_status": "not_generated",
                "approval_status": "not_requested"
            })
        })
        .collect();
    let diff = manifest_diff(input.current_manifest.as_ref(), &input.candidate_manifest);
    let current_version = input
        .current_manifest
        .as_ref()
        .map(|manifest| manifest.version);
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OP_PREVIEW.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: evidence_source.to_string(),
        audit_event_id: audit.event_id.clone(),
        error: None,
        health: None,
        body: Some(json!({
            "version": VERSION,
            "operation": OP_PREVIEW,
            "status": "accepted",
            "current_manifest": current_value,
            "candidate_manifest": candidate_value,
            "build_target": input.build_target,
            "preview_mode": "version_rollback",
            "diff": diff,
            "permission_requirements": permission_requirements,
            "version_preview": {
                "current_version": current_version,
                "candidate_version": VERSION,
                "rollback_available": false,
                "rollback_status": "not_executable",
                "rollback_reason": "Previewはrollback要求を実行しない"
            },
            "build_status": "not_started",
            "export_status": "not_started",
            "authority_strip": true,
            "inheritance_policy": {
                "authority": "none",
                "permission": "none",
                "approval": "none",
                "credential": "none",
                "audit_chain": "none"
            },
            "evidence_source": evidence_source,
            "audit_id": audit.event_id,
        })),
        shutdown_requested: broker.shutdown_requested,
    }
}

pub(super) fn parse_manifest(value: &Value) -> Result<ComposeManifest, String> {
    let manifest: ComposeManifest = serde_json::from_value(value.clone())
        .map_err(|_| "GUI Shell構成の構造が不正または禁止fieldがある".to_string())?;
    validate_manifest(manifest)
}

fn parse_mcp_tool_composition(
    value: &Value,
    server_id: &str,
    tool_id: &str,
) -> Result<(ComposeManifest, McpToolComposition), String> {
    let manifest = parse_manifest(value)?;
    if !manifest
        .mcp_connection_ids
        .iter()
        .any(|selected| selected == server_id)
        || !manifest.tool_ids.iter().any(|selected| selected == tool_id)
    {
        return Err("Compose構成に現在のMCP Server／Toolの組合せが含まれていません".to_string());
    }
    let bytes = serde_json::to_vec(&manifest)
        .map_err(|_| "Compose構成のhashを確定できません".to_string())?;
    let binding = McpToolComposition {
        compose_id: manifest.compose_id.clone(),
        manifest_hash: crate::audit_hash::sha256_tagged(&bytes),
        runtime_count: manifest.runtime_ids.len(),
        agent_count: manifest.agent_ids.len(),
        tool_count: manifest.tool_ids.len(),
    };
    Ok((manifest, binding))
}

pub(super) fn validate_mcp_tool_composition(
    broker: &mut Broker,
    value: &Value,
    server_id: &str,
    tool_id: &str,
) -> Result<McpToolComposition, String> {
    let (manifest, binding) = parse_mcp_tool_composition(value, server_id, tool_id)?;
    validate_catalog_selection(broker, &manifest)?;
    Ok(binding)
}

pub(super) fn preview_mcp_tool_composition(
    broker: &mut Broker,
    value: &Value,
    server_id: &str,
    tool_id: &str,
) -> Result<McpToolCompositionPreview, String> {
    let (manifest, composition) = parse_mcp_tool_composition(value, server_id, tool_id)?;
    let before = catalog_snapshot(broker)?;
    validate_catalog_selection(broker, &manifest)?;
    let current = catalog_snapshot(broker)?;
    let catalog_snapshot_hash = current
        .get("snapshot_hash")
        .and_then(Value::as_str)
        .ok_or_else(|| "現在Compose Catalogのhashを確定できない".to_string())?;
    if before.get("snapshot_hash").and_then(Value::as_str) != Some(catalog_snapshot_hash) {
        return Err("Compose Preview中に現在Catalogが変化した".to_string());
    }
    let connection_binding_hash = broker
        .mcp_connections
        .get(server_id)
        .and_then(|entry| entry.projection.get("Transport"))
        .and_then(|transport| transport.get("接続先hash"))
        .and_then(Value::as_str)
        .filter(|hash| valid_snapshot_hash(hash))
        .ok_or_else(|| "MCP接続のWorkspaceを含む接続束縛hashを確認できない".to_string())?;
    let host_switch_generation = current
        .get("host_switch_generation")
        .and_then(Value::as_str)
        .filter(|hash| valid_snapshot_hash(hash))
        .ok_or_else(|| "現在Host切替generationを確認できない".to_string())?;
    let dependency_preview = selected_dependency_preview(&current, &manifest, server_id, tool_id)?;

    Ok(McpToolCompositionPreview {
        composition,
        catalog_snapshot_hash: catalog_snapshot_hash.to_string(),
        connection_binding_hash: connection_binding_hash.to_string(),
        host_switch_generation: host_switch_generation.to_string(),
        dependency_preview,
    })
}

pub(super) fn validate_mcp_tool_composition_preview(
    broker: &mut Broker,
    value: &Value,
    server_id: &str,
    tool_id: &str,
    expected_manifest_hash: &str,
    expected_catalog_snapshot_hash: &str,
    expected_connection_binding_hash: &str,
) -> Result<McpToolComposition, String> {
    let preview = preview_mcp_tool_composition(broker, value, server_id, tool_id)?;
    if preview.composition.manifest_hash != expected_manifest_hash
        || preview.catalog_snapshot_hash != expected_catalog_snapshot_hash
        || preview.connection_binding_hash != expected_connection_binding_hash
    {
        return Err(
            "Compose Tool Previewと現在Manifest／Catalog／Workspaceが一致しない".to_string(),
        );
    }
    Ok(preview.composition)
}

fn selected_dependency_preview(
    catalog: &Value,
    manifest: &ComposeManifest,
    server_id: &str,
    tool_id: &str,
) -> Result<Value, String> {
    let mut sources = Vec::<(String, String, Value)>::new();
    for runtime_id in &manifest.runtime_ids {
        let runtime = catalog
            .get("runtimes")
            .and_then(Value::as_array)
            .and_then(|items| {
                items.iter().find(|item| {
                    item.get("runtime_id").and_then(Value::as_str) == Some(runtime_id.as_str())
                })
            })
            .ok_or_else(|| "Compose Preview対象Runtimeが現在Catalogにありません".to_string())?;
        sources.push((
            "runtime".to_string(),
            runtime_id.clone(),
            runtime.get("dependencies").cloned().unwrap_or(Value::Null),
        ));
    }
    let agents = catalog
        .get("agents")
        .and_then(Value::as_array)
        .ok_or_else(|| "Compose PreviewのAgent依存Catalogが不正".to_string())?;
    for agent_id in &manifest.agent_ids {
        let selected = agents
            .iter()
            .filter(|entry| {
                manifest.runtime_ids.iter().any(|runtime_id| {
                    entry.get("runtime_id").and_then(Value::as_str) == Some(runtime_id.as_str())
                }) && entry
                    .get("metadata")
                    .and_then(|metadata| metadata.get("agent_id"))
                    .and_then(Value::as_str)
                    == Some(agent_id.as_str())
            })
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return Err("Compose Preview対象Agentが現在Catalogにありません".to_string());
        }
        for agent in selected {
            let runtime_id = agent
                .get("runtime_id")
                .and_then(Value::as_str)
                .ok_or_else(|| "Compose Preview対象AgentのRuntime参照が不正".to_string())?;
            sources.push((
                "agent".to_string(),
                format!("{agent_id}@{runtime_id}"),
                agent.get("dependencies").cloned().unwrap_or(Value::Null),
            ));
        }
    }
    let tools = catalog
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| "Compose PreviewのTool依存Catalogが不正".to_string())?;
    for selected_tool_id in &manifest.tool_ids {
        let tool = tools
            .iter()
            .find(|entry| {
                entry.get("tool_id").and_then(Value::as_str) == Some(selected_tool_id.as_str())
                    && manifest.mcp_connection_ids.iter().any(|selected_server| {
                        entry.get("server_id").and_then(Value::as_str)
                            == Some(selected_server.as_str())
                    })
            })
            .ok_or_else(|| "Compose Preview対象Toolが現在Catalogにありません".to_string())?;
        let source_server = tool
            .get("server_id")
            .and_then(Value::as_str)
            .ok_or_else(|| "Compose Preview対象ToolのServer参照が不正".to_string())?;
        sources.push((
            "tool".to_string(),
            format!(
                "{selected_server}:{selected_tool_id}",
                selected_server = source_server
            ),
            tool.get("dependencies").cloned().unwrap_or(Value::Null),
        ));
    }
    if !manifest.tool_ids.iter().any(|selected| selected == tool_id)
        || !manifest
            .mcp_connection_ids
            .iter()
            .any(|selected| selected == server_id)
    {
        return Err("Compose Preview対象Server／ToolがManifestに含まれていない".to_string());
    }
    if sources.len() > 256 {
        return Err("Compose Previewの依存項目上限を超過した".to_string());
    }

    let mut observed_bindings = Vec::new();
    let mut incomplete_sources = 0usize;
    let mut unknown_sources = 0usize;
    for (source_kind, source_id, dependencies) in &sources {
        let status = dependencies
            .get("status")
            .and_then(Value::as_str)
            .filter(|status| ["known", "partial", "unknown"].contains(status))
            .ok_or_else(|| "Compose Previewの依存状態が不正".to_string())?;
        if status != "known" {
            incomplete_sources += 1;
        }
        if status == "unknown" {
            unknown_sources += 1;
        }
        let items = dependencies
            .get("items")
            .and_then(Value::as_array)
            .ok_or_else(|| "Compose Previewの依存参照一覧が不正".to_string())?;
        for item in items {
            let target_kind = item
                .get("kind")
                .and_then(Value::as_str)
                .ok_or_else(|| "Compose Previewの依存種別が不正".to_string())?;
            let target_id = item
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "Compose Previewの依存IDが不正".to_string())?;
            observed_bindings.push(json!({
                "source_kind": source_kind,
                "source_id": source_id,
                "target_kind": target_kind,
                "target_id": target_id,
            }));
        }
    }
    let status = if unknown_sources > 0 && observed_bindings.is_empty() {
        "unknown"
    } else if incomplete_sources > 0 {
        "partial"
    } else {
        "known"
    };
    Ok(json!({
        "status": status,
        "observed_bindings": observed_bindings,
        "incomplete_source_count": incomplete_sources,
        "unknown_source_count": unknown_sources,
    }))
}

pub(super) fn mcp_tool_composition_owner_summary(
    value: &Value,
    server_id: &str,
    tool_id: &str,
) -> Result<String, String> {
    let (_, binding) = parse_mcp_tool_composition(value, server_id, tool_id)?;
    Ok(format!(
        "Compose ID: {} / Runtime {}件・Agent {}件・Tool {}件 / 構成hash: {}",
        binding.compose_id,
        binding.runtime_count,
        binding.agent_count,
        binding.tool_count,
        binding.manifest_hash
    ))
}

pub(super) fn validate_catalog_selection(
    broker: &mut Broker,
    manifest: &ComposeManifest,
) -> Result<(), String> {
    catalog_snapshot(broker)?;
    let runtime_agent_catalog = broker
        .compose_runtime_agent_catalog()
        .map_err(|_| "現在のRuntime／Agent Catalogを検証できない".to_string())?;
    let available_runtime_ids = runtime_agent_catalog
        .runtime_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if manifest
        .runtime_ids
        .iter()
        .any(|runtime_id| !available_runtime_ids.contains(runtime_id.as_str()))
    {
        return Err("選択したRuntimeが現在のBroker登録Catalogにありません".to_string());
    }
    let mut agents_by_id: BTreeMap<&str, Vec<_>> = BTreeMap::new();
    for entry in &runtime_agent_catalog.agents {
        if let Some(agent_id) = entry.metadata.get("agent_id").and_then(Value::as_str) {
            agents_by_id.entry(agent_id).or_default().push(entry);
        }
    }
    let mut selected_agent_instances = Vec::with_capacity(manifest.agent_ids.len());
    for agent_id in &manifest.agent_ids {
        let entries = agents_by_id
            .get(agent_id.as_str())
            .ok_or_else(|| "選択したAgentが現在のBroker登録Catalogにありません".to_string())?;
        let selected_instances = entries
            .iter()
            .copied()
            .filter(|entry| {
                available_runtime_ids.contains(entry.runtime_id.as_str())
                    && manifest.runtime_ids.contains(&entry.runtime_id)
                    && matches!(
                        entry.metadata["status"].as_str(),
                        Some("ready" | "degraded")
                    )
            })
            .collect::<Vec<_>>();
        if selected_instances.is_empty() {
            return Err("選択Agentに対応する現在Runtimeが構成へ含まれていません".to_string());
        }
        if !manifest.tool_ids.is_empty()
            && !selected_instances.iter().any(|agent| {
                let task_execution =
                    agent.metadata["capabilities"]
                        .as_array()
                        .is_some_and(|capabilities| {
                            capabilities.iter().any(|capability| {
                                capability["capability_id"].as_str() == Some("task_execution")
                                    && capability["support"]["status"].as_str() == Some("supported")
                            })
                        });
                task_execution
                    && agent.metadata["tool_support"]["status"].as_str() == Some("supported")
                    && agent.metadata["mcp_support"]["status"].as_str() == Some("supported")
            })
        {
            return Err(
                "選択Agentの同一Runtime instanceがtask_execution・tool_support・mcp_supportをすべてsupportedとして宣言していません"
                    .to_string(),
            );
        }
        selected_agent_instances.push(selected_instances);
    }
    if !manifest.capability_requirements.is_empty() && selected_agent_instances.is_empty() {
        return Err("Capability互換条件には少なくとも1つの現在Agent選択が必要です".to_string());
    }
    for capability_id in &manifest.capability_requirements {
        for instances in &selected_agent_instances {
            let supported = instances.iter().any(|agent| {
                agent.metadata["capabilities"]
                    .as_array()
                    .is_some_and(|capabilities| {
                        capabilities.iter().any(|capability| {
                            capability["capability_id"].as_str() == Some(capability_id)
                                && capability["support"]["status"].as_str() == Some("supported")
                        })
                    })
            });
            if !supported {
                return Err(format!(
                    "選択AgentがCapability互換条件をsupportedとして宣言していません: {capability_id}"
                ));
            }
        }
    }
    let mut available_tools = BTreeSet::new();
    let credential_valid_by_server = broker
        .mcp_connections
        .iter()
        .map(|(server_id, entry)| {
            let reference = entry.projection.get("Credential ref");
            let required = reference
                .and_then(|value| value.get("required"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let credential_id = reference
                .and_then(|value| value.get("credential_id"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let status = reference
                .and_then(|value| value.get("status"))
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let valid = !required
                || (status == "configured"
                    && !credential_id.is_empty()
                    && broker.MCP資格情報有効確認(credential_id, server_id));
            (server_id.clone(), valid)
        })
        .collect::<BTreeMap<_, _>>();
    for server_id in &manifest.mcp_connection_ids {
        let entry = broker
            .mcp_connections
            .get_mut(server_id)
            .ok_or_else(|| "選択したMCP接続が現在のBroker Catalogにありません".to_string())?;
        if !credential_valid_by_server
            .get(server_id)
            .copied()
            .unwrap_or(false)
        {
            return Err("選択したMCP接続のCredentialが失効または利用不能です".to_string());
        }
        if entry.quarantined || !entry.connection.is_alive() {
            return Err("選択したMCP接続が利用可能状態ではありません".to_string());
        }
        available_tools.extend(
            entry
                .connection
                .catalog()
                .tools
                .iter()
                .map(|tool| tool.tool_id.clone()),
        );
    }
    if manifest
        .tool_ids
        .iter()
        .any(|tool_id| !available_tools.contains(tool_id))
    {
        return Err("選択したToolが選択中の現在MCP Catalogにありません".to_string());
    }
    Ok(())
}

fn parse_preview(value: &Value) -> Result<PreviewInput, String> {
    let request: PreviewRequest = serde_json::from_value(value.clone())
        .map_err(|_| "GUI Shell構成Previewの構造が不正または禁止fieldがある".to_string())?;
    if request.version != VERSION {
        return Err("GUI Shell構成Previewの版が不正".to_string());
    }
    if request.preview_mode != "version_rollback" {
        return Err("Preview modeはversion_rollbackだけを受け付ける".to_string());
    }
    if !["windows", "desktop", "mobile"].contains(&request.build_target.as_str()) {
        return Err("Preview対象platformが不正".to_string());
    }
    let current_manifest = if request.current_manifest.is_null() {
        None
    } else {
        Some(validate_manifest(
            serde_json::from_value(request.current_manifest)
                .map_err(|_| "現在の構成Manifestが不正".to_string())?,
        )?)
    };
    Ok(PreviewInput {
        current_manifest,
        candidate_manifest: validate_manifest(request.candidate_manifest)?,
        build_target: request.build_target,
    })
}

fn manifest_diff(base: Option<&ComposeManifest>, candidate: &ComposeManifest) -> Value {
    let (runtime_added, runtime_removed) = list_delta(
        base.map(|manifest| manifest.runtime_ids.as_slice()),
        candidate.runtime_ids.as_slice(),
    );
    let (agent_added, agent_removed) = list_delta(
        base.map(|manifest| manifest.agent_ids.as_slice()),
        candidate.agent_ids.as_slice(),
    );
    let (tool_added, tool_removed) = list_delta(
        base.map(|manifest| manifest.tool_ids.as_slice()),
        candidate.tool_ids.as_slice(),
    );
    let (mcp_added, mcp_removed) = list_delta(
        base.map(|manifest| manifest.mcp_connection_ids.as_slice()),
        candidate.mcp_connection_ids.as_slice(),
    );
    let (capability_added, capability_removed) = list_delta(
        base.map(|manifest| manifest.capability_requirements.as_slice()),
        candidate.capability_requirements.as_slice(),
    );
    let mut changed_fields = Vec::new();
    match base {
        None => changed_fields.push("initial_configuration".to_string()),
        Some(base) => {
            if base.compose_id != candidate.compose_id {
                changed_fields.push("compose_id".to_string());
            }
            if base.display_name != candidate.display_name {
                changed_fields.push("display_name".to_string());
            }
            if base.theme != candidate.theme {
                changed_fields.push("theme".to_string());
            }
            if base.settings != candidate.settings {
                changed_fields.push("settings".to_string());
            }
        }
    }
    json!({
        "runtime_added": runtime_added,
        "runtime_removed": runtime_removed,
        "agent_added": agent_added,
        "agent_removed": agent_removed,
        "tool_added": tool_added,
        "tool_removed": tool_removed,
        "mcp_added": mcp_added,
        "mcp_removed": mcp_removed,
        "capability_added": capability_added,
        "capability_removed": capability_removed,
        "changed_fields": changed_fields,
    })
}

fn list_delta(base: Option<&[String]>, candidate: &[String]) -> (Vec<String>, Vec<String>) {
    let old: BTreeSet<&String> = base.unwrap_or(&[]).iter().collect();
    let new: BTreeSet<&String> = candidate.iter().collect();
    (
        new.difference(&old).map(|value| (*value).clone()).collect(),
        old.difference(&new).map(|value| (*value).clone()).collect(),
    )
}

fn validate_manifest(manifest: ComposeManifest) -> Result<ComposeManifest, String> {
    if manifest.version != VERSION
        || !valid_identifier(&manifest.compose_id)
        || manifest.display_name.trim().is_empty()
        || manifest.display_name.chars().count() > 128
    {
        return Err("構成ID、版、表示名が不正".to_string());
    }
    for (label, values, limit, max_length) in [
        ("実行基盤", &manifest.runtime_ids, 32usize, 128usize),
        ("エージェント", &manifest.agent_ids, 32usize, 128usize),
        ("ツール", &manifest.tool_ids, 64usize, 147usize),
        ("MCP接続", &manifest.mcp_connection_ids, 32usize, 128usize),
        (
            "機能要件",
            &manifest.capability_requirements,
            64usize,
            128usize,
        ),
    ] {
        if values.len() > limit
            || values
                .iter()
                .any(|value| value.trim().is_empty() || value.chars().count() > max_length)
            || values.iter().collect::<BTreeSet<_>>().len() != values.len()
        {
            return Err(format!("{label}選択が不正または重複している"));
        }
    }
    if manifest.theme.theme_id.trim().is_empty()
        || manifest.theme.theme_id.chars().count() > 64
        || !["system", "light", "dark"].contains(&manifest.theme.mode.as_str())
    {
        return Err("Theme選択が不正".to_string());
    }
    if !valid_locale(&manifest.settings.locale)
        || !["compact", "comfortable"].contains(&manifest.settings.density.as_str())
        || !["none", "hash_only", "summary", "redacted", "full"]
            .contains(&manifest.settings.content_visibility.as_str())
    {
        return Err("Settings選択が不正".to_string());
    }
    let policy = &manifest.inheritance_policy;
    if [
        &policy.authority,
        &policy.permission,
        &policy.approval,
        &policy.credential,
        &policy.audit_chain,
    ]
    .iter()
    .any(|value| value.as_str() != "none")
    {
        return Err(
            "Credential、Permission、Approval、Authority、Audit chainの継承を拒否".to_string(),
        );
    }
    if manifest.output_mode != "manifest_only" {
        return Err("構成出力はmanifest_onlyだけを受け付ける".to_string());
    }
    Ok(manifest)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= 64
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

fn valid_locale(value: &str) -> bool {
    let mut parts = value.split('-');
    let language = parts.next().unwrap_or_default();
    let region = parts.next().unwrap_or_default();
    parts.next().is_none()
        && language.len() == 2
        && region.len() == 2
        && language
            .chars()
            .all(|character| character.is_ascii_alphabetic())
        && region
            .chars()
            .all(|character| character.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::dialogue::{実行系Adapter, 実行結果, 対話失敗, 対話要求};
    use crate::broker::protocol::{Broker, BrokerOperation, BrokerRequestEnvelope, BrokerStatus};
    use std::sync::{atomic::AtomicBool, Arc};
    use std::time::Instant;

    struct ComposeCatalogAdapter {
        metadata: Option<Value>,
    }

    impl 実行系Adapter for ComposeCatalogAdapter {
        fn 接続対象(&self) -> String {
            "fixture://compose-catalog".to_string()
        }

        fn agent_metadata(&self) -> Option<Value> {
            self.metadata.clone()
        }

        fn 応答(
            &self,
            _request: &対話要求,
            _cancel: &AtomicBool,
            _deadline: Instant,
            _raw_received: &mut Vec<Vec<u8>>,
        ) -> Result<実行結果, 対話失敗> {
            Err(対話失敗::AgentTask非対応)
        }
    }

    fn register_agent(
        broker: &mut Broker,
        runtime_id: &str,
        agent_id: &str,
        capability_status: &str,
    ) {
        register_agent_with_model(
            broker,
            runtime_id,
            agent_id,
            capability_status,
            "fixture-model",
        );
    }

    fn register_agent_with_model(
        broker: &mut Broker,
        runtime_id: &str,
        agent_id: &str,
        capability_status: &str,
        model: &str,
    ) {
        register_agent_with_tool_compatibility_and_model(
            broker,
            runtime_id,
            agent_id,
            capability_status,
            model,
            "unsupported",
            "unsupported",
        );
    }

    fn register_agent_with_tool_compatibility(
        broker: &mut Broker,
        runtime_id: &str,
        agent_id: &str,
        task_execution_status: &str,
        tool_status: &str,
        mcp_status: &str,
    ) {
        register_agent_with_tool_compatibility_and_model(
            broker,
            runtime_id,
            agent_id,
            task_execution_status,
            "fixture-model",
            tool_status,
            mcp_status,
        );
    }

    fn register_agent_with_tool_compatibility_and_model(
        broker: &mut Broker,
        runtime_id: &str,
        agent_id: &str,
        task_execution_status: &str,
        model: &str,
        tool_status: &str,
        mcp_status: &str,
    ) {
        let mut metadata: Value = serde_json::from_str(include_str!(
            "../../../../examples/contracts/agent_adapter.valid.json"
        ))
        .expect("Agent Adapter試験資料");
        metadata["adapter_id"] = json!(format!("{agent_id}-adapter"));
        metadata["agent_id"] = json!(agent_id);
        metadata["model"] = json!(model);
        metadata["status"] = json!("ready");
        metadata["capabilities"] = json!([{
            "capability_id": "task_execution",
            "support": {"status": task_execution_status, "reason": "Compose互換性試験"}
        }]);
        metadata["tool_support"] = json!({
            "status": tool_status,
            "reason": "Compose互換性試験"
        });
        metadata["mcp_support"] = json!({
            "status": mcp_status,
            "reason": "Compose互換性試験"
        });
        broker
            .実行系登録(
                runtime_id,
                Arc::new(ComposeCatalogAdapter {
                    metadata: Some(metadata),
                }),
            )
            .expect("Compose用Agent Runtime登録");
    }

    fn call_catalog(broker: &mut Broker, payload: Value) -> BrokerResponse {
        let mut request = BrokerRequestEnvelope::command_envelope_at(
            "compose-catalog-test",
            "session-1",
            &format!("catalog-nonce-{}", broker.audit_events().len()),
            &BrokerRequestEnvelope::current_issued_at(),
        );
        request.operation = Some(BrokerOperation::GuiShell構成Catalog);
        request.payload = Some(payload);
        request.refresh_payload_hash();
        broker.handle(request)
    }

    fn manifest() -> Value {
        json!({
            "version": 1,
            "compose_id": "d4-pocket-local",
            "display_name": "D4 Pocket ローカル構成",
            "runtime_ids": [],
            "agent_ids": [],
            "tool_ids": [],
            "mcp_connection_ids": [],
            "theme": {"theme_id": "d4-pocket", "mode": "system"},
            "capability_requirements": [],
            "settings": {"locale": "ja-JP", "density": "comfortable", "content_visibility": "summary"},
            "inheritance_policy": {
                "authority": "none", "permission": "none", "approval": "none",
                "credential": "none", "audit_chain": "none"
            },
            "output_mode": "manifest_only"
        })
    }

    fn call(broker: &mut Broker, payload: Value) -> super::super::protocol::BrokerResponse {
        let payload = if payload.get("compose_id").is_some() {
            let snapshot = catalog_snapshot(broker).expect("現在Compose Catalog");
            json!({
                "version": VERSION,
                "catalog_snapshot_hash": snapshot["snapshot_hash"],
                "compose_manifest": payload,
            })
        } else {
            payload
        };
        let mut request = BrokerRequestEnvelope::command_envelope_at(
            "compose-test",
            "session-1",
            &format!("nonce-{}", broker.audit_events().len()),
            &BrokerRequestEnvelope::current_issued_at(),
        );
        request.operation = Some(BrokerOperation::GuiShell構成);
        request.payload = Some(payload);
        request.refresh_payload_hash();
        broker.handle(request)
    }

    fn call_with_snapshot(
        broker: &mut Broker,
        manifest: Value,
        snapshot_hash: &str,
    ) -> super::super::protocol::BrokerResponse {
        call(
            broker,
            json!({
                "version": VERSION,
                "catalog_snapshot_hash": snapshot_hash,
                "compose_manifest": manifest,
            }),
        )
    }

    fn preview_payload(current: Value, candidate: Value) -> Value {
        json!({
            "version": 1,
            "current_manifest": current,
            "candidate_manifest": candidate,
            "build_target": "windows",
            "preview_mode": "version_rollback"
        })
    }

    fn call_preview(broker: &mut Broker, payload: Value) -> super::super::protocol::BrokerResponse {
        let mut request = BrokerRequestEnvelope::command_envelope_at(
            "preview-test",
            "session-1",
            &format!("preview-nonce-{}", broker.audit_events().len()),
            &BrokerRequestEnvelope::current_issued_at(),
        );
        request.operation = Some(BrokerOperation::GuiShell構成Preview);
        request.payload = Some(payload);
        request.refresh_payload_hash();
        broker.handle(request)
    }

    #[test]
    fn ComposeCatalogは現在RuntimeとAgentの対応を投影しAuthorityを生成しない() {
        let mut broker = Broker::new("session-1");
        register_agent(&mut broker, "fixture-runtime", "fixture-agent", "supported");
        let response = call_catalog(&mut broker, json!({}));
        assert_eq!(response.status, BrokerStatus::Accepted);
        assert_eq!(response.evidence_source, EVIDENCE_SOURCE_INTERNAL_STATE);
        let body = response.body.expect("Catalog body");
        assert_eq!(body["runtime_ids"], json!(["fixture-runtime"]));
        assert_eq!(body["version"], CATALOG_VERSION);
        assert!(body["snapshot_hash"]
            .as_str()
            .is_some_and(valid_snapshot_hash));
        assert_eq!(body["runtimes"][0]["runtime_id"], "fixture-runtime");
        assert_eq!(body["runtimes"][0]["abi"], "unknown");
        assert_eq!(body["runtimes"][0]["permissions"]["status"], "not_granted");
        assert_eq!(body["runtimes"][0]["dependencies"]["status"], "partial");
        assert_eq!(
            body["runtimes"][0]["dependencies"]["items"][0]["kind"],
            "adapter"
        );
        assert_eq!(
            body["agents"][0]["dependencies"]["items"][0]["kind"],
            "runtime"
        );
        assert!(body["hosts"]
            .as_array()
            .is_some_and(|hosts| !hosts.is_empty()));
        assert_eq!(body["tools"], json!([]));
        assert_eq!(body["agents"][0]["runtime_id"], "fixture-runtime");
        assert_eq!(body["agents"][0]["metadata"]["agent_id"], "fixture-agent");
        assert_eq!(body["authority_strip"], true);
        assert_eq!(body["permission_generated"], false);
        assert_eq!(
            call_catalog(&mut broker, json!({"owner": true})).status,
            BrokerStatus::Rejected
        );
    }

    #[test]
    fn Compose依存validatorは欠落先と循環を拒否する() {
        assert!(valid_catalog_id("runtime-a"));
        assert!(!valid_catalog_id("_runtime-a"));
        assert!(!valid_catalog_id("runtime/../a"));
        assert!(!valid_catalog_id(&"a".repeat(257)));

        let first = dependency_node_key("runtime", "runtime-a");
        let second = dependency_node_key("adapter", "adapter-a");
        let nodes = BTreeSet::from([first.clone(), second.clone()]);
        let missing = BTreeSet::from([(first.clone(), dependency_node_key("host", "missing"))]);
        assert!(validate_dependency_edges(&nodes, &missing).is_err());
        let cycle = BTreeSet::from([(first.clone(), second.clone()), (second, first)]);
        assert!(validate_dependency_edges(&nodes, &cycle)
            .expect_err("循環を拒否")
            .contains("循環"));

        let mut broker = Broker::new("session-1");
        register_agent(&mut broker, "fixture-runtime", "fixture-agent", "supported");
        let mut catalog = catalog_snapshot(&mut broker).expect("Catalog");
        catalog["agents"][0]["dependencies"]["items"][0]["id"] = json!("missing-runtime");
        assert!(validate_dependency_catalog(&catalog).is_err());
    }

    #[cfg(not(feature = "module_host_capabilities"))]
    #[test]
    fn ComposeCatalogはHost能力Module除外時も能力を推測せずLocalHostを投影する() {
        let mut broker = Broker::new("session-1");
        let response = call_catalog(&mut broker, json!({}));
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.expect("Catalog body");
        let platform = match std::env::consts::OS {
            "windows" => "windows",
            "macos" => "macos",
            "linux" => "linux",
            "android" => "android",
            "ios" => "ios",
            _ => "unknown",
        };
        assert_eq!(
            body["hosts"][0]["host_id"],
            format!("gui-shell-local-{platform}")
        );
        assert_eq!(
            body["hosts"][0]["display_name"],
            format!("ローカル {platform}")
        );
        assert_eq!(body["hosts"][0]["status"], "unknown");
        assert_eq!(body["hosts"][0]["scope"]["capabilities"], json!([]));
        assert_eq!(body["hosts"][0]["permissions"]["status"], "not_granted");
        assert_eq!(body["hosts"][0]["authority_strip"], true);
    }

    #[test]
    fn ComposeはHost切替後に古いCatalogSnapshotを拒否する() {
        let mut broker = Broker::new("session-1");
        let snapshot = catalog_snapshot(&mut broker).expect("切替前Catalog");
        let stale_hash = snapshot["snapshot_hash"].as_str().unwrap().to_string();
        broker
            .append_audit(
                "host-switch-test",
                "Host切替",
                "accepted",
                "Host表示コンテキストを切替",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .expect("Host切替Audit");

        let response = call_with_snapshot(&mut broker, manifest(), &stale_hash);
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.as_ref().map(|error| error.code.as_str()),
            Some("gui_shell_compose_catalog_stale")
        );
    }

    #[test]
    fn ComposeはHost表示停止後に古いCatalogSnapshotを拒否する() {
        let mut broker = Broker::new("session-1");
        let snapshot = catalog_snapshot(&mut broker).expect("停止前Catalog");
        let stale_hash = snapshot["snapshot_hash"].as_str().unwrap().to_string();
        broker
            .append_audit(
                "host-stop-test",
                "Host停止",
                "accepted",
                "Host表示コンテキストを解除",
                EVIDENCE_SOURCE_INTERNAL_STATE,
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .expect("Host停止Audit");

        let response = call_with_snapshot(&mut broker, manifest(), &stale_hash);
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.as_ref().map(|error| error.code.as_str()),
            Some("gui_shell_compose_catalog_stale")
        );
    }

    #[test]
    fn ComposeCatalogは同じAgent型を複数Runtimeに対応付ける() {
        let mut broker = Broker::new("session-1");
        register_agent_with_model(&mut broker, "codex-a", "codex", "supported", "model-a");
        register_agent_with_model(&mut broker, "codex-b", "codex", "supported", "model-b");

        let response = call_catalog(&mut broker, json!({}));
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.expect("複数Runtime Catalog本文");
        assert_eq!(body["runtime_ids"], json!(["codex-a", "codex-b"]));
        assert_eq!(body["agents"].as_array().unwrap().len(), 2);
        assert_eq!(body["agents"][0]["runtime_id"], "codex-a");
        assert_eq!(body["agents"][1]["runtime_id"], "codex-b");
        assert_eq!(body["agents"][0]["metadata"]["adapter_id"], "codex-adapter");
        assert_eq!(body["agents"][1]["metadata"]["agent_id"], "codex");
        assert_eq!(body["agents"][0]["metadata"]["model"], "model-a");
        assert_eq!(body["agents"][1]["metadata"]["model"], "model-b");
        assert_eq!(body["authority_strip"], true);
        assert_eq!(body["permission_generated"], false);
    }

    #[test]
    fn ComposeCatalogは同じAgentIDを異なるAdapter型のRuntimeへ束縛する登録を拒否する() {
        let mut broker = Broker::new("session-1");
        register_agent(&mut broker, "codex-a", "codex", "supported");
        let mut metadata: Value = serde_json::from_str(include_str!(
            "../../../../examples/contracts/agent_adapter.valid.json"
        ))
        .expect("Agent Adapter試験資料");
        metadata["adapter_id"] = json!("unrelated-adapter");
        metadata["agent_id"] = json!("codex");
        broker
            .実行系登録(
                "other-runtime",
                Arc::new(ComposeCatalogAdapter {
                    metadata: Some(metadata),
                }),
            )
            .expect("別Runtime登録自体は成功");

        let response = call_catalog(&mut broker, json!({}));
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.body, None);
    }

    #[test]
    fn ComposeCatalogは投影上限を超えるRuntime一覧を拒否する() {
        let mut broker = Broker::new("session-1");
        for index in 0..65 {
            broker
                .実行系登録(
                    &format!("fixture-runtime-{index:02}"),
                    Arc::new(ComposeCatalogAdapter { metadata: None }),
                )
                .expect("Catalog上限試験用Runtime登録");
        }
        let response = call_catalog(&mut broker, json!({}));
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.body, None);
    }

    #[test]
    fn Composeは現在CatalogとAgentRuntime対応およびCapability互換性を再検査する() {
        let mut broker = Broker::new("session-1");
        register_agent(&mut broker, "fixture-runtime", "fixture-agent", "supported");

        let mut compatible = manifest();
        compatible["runtime_ids"] = json!(["fixture-runtime"]);
        compatible["agent_ids"] = json!(["fixture-agent"]);
        compatible["capability_requirements"] = json!(["task_execution"]);
        assert_eq!(
            call(&mut broker, compatible.clone()).status,
            BrokerStatus::Accepted
        );

        let mut stale_runtime = compatible.clone();
        stale_runtime["runtime_ids"] = json!(["stale-runtime"]);
        assert_eq!(
            call(&mut broker, stale_runtime).status,
            BrokerStatus::Rejected
        );

        let mut missing_runtime_pair = compatible.clone();
        missing_runtime_pair["runtime_ids"] = json!([]);
        assert_eq!(
            call(&mut broker, missing_runtime_pair).status,
            BrokerStatus::Rejected
        );

        let mut unknown_capability = compatible;
        unknown_capability["capability_requirements"] = json!(["provider_selection"]);
        assert_eq!(
            call(&mut broker, unknown_capability).status,
            BrokerStatus::Rejected
        );

        let mut unsupported = Broker::new("session-unsupported");
        register_agent(
            &mut unsupported,
            "fixture-runtime",
            "fixture-agent",
            "unknown",
        );
        let mut unsupported_requirement = manifest();
        unsupported_requirement["runtime_ids"] = json!(["fixture-runtime"]);
        unsupported_requirement["agent_ids"] = json!(["fixture-agent"]);
        unsupported_requirement["capability_requirements"] = json!(["task_execution"]);
        assert_eq!(
            call(&mut unsupported, unsupported_requirement).status,
            BrokerStatus::Rejected
        );
    }

    #[test]
    fn Composeは選択した複数Runtimeの同一Agent型から利用可能な対応先を解決する() {
        let mut broker = Broker::new("session-1");
        register_agent(&mut broker, "codex-a", "codex", "supported");
        register_agent(&mut broker, "codex-b", "codex", "unknown");

        let mut supported_instance = manifest();
        supported_instance["runtime_ids"] = json!(["codex-a"]);
        supported_instance["agent_ids"] = json!(["codex"]);
        supported_instance["capability_requirements"] = json!(["task_execution"]);
        assert_eq!(
            call(&mut broker, supported_instance).status,
            BrokerStatus::Accepted,
            "未選択instanceのmetadataが選択instanceへ干渉しない"
        );

        let mut unknown_instance = manifest();
        unknown_instance["runtime_ids"] = json!(["codex-b"]);
        unknown_instance["agent_ids"] = json!(["codex"]);
        unknown_instance["capability_requirements"] = json!(["task_execution"]);
        assert_eq!(
            call(&mut broker, unknown_instance).status,
            BrokerStatus::Rejected,
            "選択instance自身がCapabilityをsupportedと宣言しない場合は拒否する"
        );
    }

    #[test]
    fn ComposeのMCP_Tool要件は選択Agent同一Runtimeの実行・Tool・MCP対応を要求する() {
        let manifest_with_tool = || {
            let mut value = manifest();
            value["runtime_ids"] = json!(["fixture-runtime"]);
            value["agent_ids"] = json!(["fixture-agent"]);
            value["tool_ids"] = json!(["tool-fixture"]);
            value
        };

        for (task_status, tool_status, mcp_status) in [
            ("supported", "unsupported", "supported"),
            ("supported", "unknown", "supported"),
            ("supported", "supported", "unsupported"),
            ("supported", "supported", "unknown"),
            ("unknown", "supported", "supported"),
            ("unsupported", "supported", "supported"),
        ] {
            let mut broker = Broker::new("session-incompatible");
            register_agent_with_tool_compatibility(
                &mut broker,
                "fixture-runtime",
                "fixture-agent",
                task_status,
                tool_status,
                mcp_status,
            );
            let error = validate_catalog_selection(
                &mut broker,
                &parse_manifest(&manifest_with_tool()).unwrap(),
            )
            .expect_err("選択Agentに未成立の依存があればComposeを拒否する");
            assert!(
                error.contains("task_execution・tool_support・mcp_support"),
                "互換性を拒否した理由が明確: {error}"
            );
        }

        let mut split = Broker::new("session-split-runtime-support");
        register_agent_with_tool_compatibility(
            &mut split,
            "runtime-tool-only",
            "fixture-agent",
            "supported",
            "supported",
            "unsupported",
        );
        register_agent_with_tool_compatibility(
            &mut split,
            "runtime-mcp-only",
            "fixture-agent",
            "supported",
            "unsupported",
            "supported",
        );
        let mut split_manifest = manifest_with_tool();
        split_manifest["runtime_ids"] = json!(["runtime-tool-only", "runtime-mcp-only"]);
        let error =
            validate_catalog_selection(&mut split, &parse_manifest(&split_manifest).unwrap())
                .expect_err("別Runtime instanceの能力を寄せ集めてTool互換と判定しない");
        assert!(
            error.contains("task_execution・tool_support・mcp_support"),
            "Runtimeをまたいだ能力の寄せ集めを拒否した理由が明確: {error}"
        );

        let mut compatible = Broker::new("session-compatible");
        register_agent_with_tool_compatibility(
            &mut compatible,
            "fixture-runtime",
            "fixture-agent",
            "supported",
            "supported",
            "supported",
        );
        let error = validate_catalog_selection(
            &mut compatible,
            &parse_manifest(&manifest_with_tool()).unwrap(),
        )
        .expect_err("Catalog未登録Toolは後段の現在MCP Catalog検査で拒否する");
        assert_eq!(
            error, "選択したToolが選択中の現在MCP Catalogにありません",
            "3能力を同一instanceが宣言すれば互換性検査を通過する"
        );
    }

    #[test]
    fn 構成はmanifestだけを生成し権限を継承しない() {
        let mut broker = Broker::new("session-1");
        let response = call(&mut broker, manifest());
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.unwrap();
        assert_eq!(body["build_status"], "not_started");
        assert_eq!(body["app_identity_status"], "not_generated");
        assert_eq!(body["permission_generated"], false);
        assert_eq!(body["authority_strip"], true);
        assert_eq!(body["inheritance_policy"]["credential"], "none");
        assert_eq!(body["compose_manifest"]["output_mode"], "manifest_only");
    }

    #[test]
    fn 構成は権限継承と未知fieldを拒否する() {
        let mut broker = Broker::new("session-1");
        let mut authority = manifest();
        authority["inheritance_policy"]["permission"] = Value::from("permission.read");
        assert_eq!(call(&mut broker, authority).status, BrokerStatus::Rejected);

        let mut unknown = manifest();
        unknown["credential"] = Value::from("secret");
        assert_eq!(call(&mut broker, unknown).status, BrokerStatus::Rejected);
    }

    #[test]
    fn 構成のlocaleと重複選択を検査する() {
        let mut broker = Broker::new("session-1");
        let mut invalid_locale = manifest();
        invalid_locale["settings"]["locale"] = Value::from("ja");
        assert_eq!(
            call(&mut broker, invalid_locale).status,
            BrokerStatus::Rejected
        );

        let mut duplicate = manifest();
        duplicate["agent_ids"] = json!(["codex", "codex"]);
        assert_eq!(call(&mut broker, duplicate).status, BrokerStatus::Rejected);
    }

    #[test]
    fn mcp_catalog_tool_ids_accept_147_and_reject_unknown_server() {
        let valid_tool_id = format!("tool-{}", "a".repeat(142));
        let oversized_tool_id = format!("tool-{}", "a".repeat(143));
        let mut valid = manifest();
        valid["tool_ids"] = json!([valid_tool_id]);
        assert!(
            parse_manifest(&valid).is_ok(),
            "147文字の実Tool IDを受理する"
        );

        let mut oversized = manifest();
        oversized["tool_ids"] = json!([oversized_tool_id]);
        assert!(parse_manifest(&oversized).is_err(), "148文字は拒否する");

        let mut broker = Broker::new("session-1");
        let mut unregistered = valid;
        unregistered["mcp_connection_ids"] = json!(["unknown-mcp"]);
        let response = call(&mut broker, unregistered);
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.as_ref().map(|error| error.code.as_str()),
            Some("gui_shell_compose_catalog_mismatch")
        );
    }

    #[test]
    fn 構成のpayload_hashが監査へ結合される() {
        let mut broker = Broker::new("session-1");
        let payload = manifest();
        let response = call(&mut broker, payload.clone());
        let event = broker.audit_events().last().unwrap();
        assert_eq!(event.operation, OP_COMPOSE);
        assert_eq!(
            event.payload_hash,
            crate::broker::protocol::canonical_payload_hash(Some(&payload))
        );
        assert_eq!(response.audit_event_id, event.event_id);
    }

    #[test]
    fn Previewは差分と機能要件を表示するがbuildとrollbackを実行しない() {
        let mut broker = Broker::new("session-1");
        register_agent(&mut broker, "fixture-runtime", "fixture-agent", "supported");
        let mut candidate = manifest();
        candidate["runtime_ids"] = json!(["fixture-runtime"]);
        candidate["agent_ids"] = json!(["fixture-agent"]);
        candidate["capability_requirements"] = json!(["task_execution"]);
        let response = call_preview(&mut broker, preview_payload(Value::Null, candidate));
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.unwrap();
        assert_eq!(body["build_status"], "not_started");
        assert_eq!(body["export_status"], "not_started");
        assert_eq!(body["version_preview"]["rollback_available"], false);
        assert_eq!(body["diff"]["runtime_added"][0], "fixture-runtime");
        assert_eq!(
            body["permission_requirements"][0]["permission_status"],
            "not_generated"
        );
    }

    #[test]
    fn Previewは現在構成との差分を決定論的に返す() {
        let mut broker = Broker::new("session-1");
        register_agent(&mut broker, "fixture-runtime", "fixture-agent", "supported");
        let current = manifest();
        let mut candidate = manifest();
        candidate["display_name"] = Value::from("変更後構成");
        candidate["runtime_ids"] = json!(["fixture-runtime"]);
        candidate["agent_ids"] = json!(["fixture-agent"]);
        candidate["capability_requirements"] = json!(["task_execution"]);
        let response = call_preview(&mut broker, preview_payload(current, candidate));
        let body = response.body.unwrap();
        assert_eq!(body["diff"]["runtime_added"][0], "fixture-runtime");
        assert_eq!(body["diff"]["agent_added"][0], "fixture-agent");
        assert_eq!(body["diff"]["capability_added"][0], "task_execution");
        assert_eq!(body["diff"]["changed_fields"][0], "display_name");
    }

    #[test]
    fn Previewは権限継承と未知fieldを拒否する() {
        let mut broker = Broker::new("session-1");
        let mut invalid = preview_payload(Value::Null, manifest());
        invalid["candidate_manifest"]["inheritance_policy"]["permission"] =
            Value::from("permission.read");
        assert_eq!(
            call_preview(&mut broker, invalid).status,
            BrokerStatus::Rejected
        );

        let mut unknown = preview_payload(Value::Null, manifest());
        unknown["candidate_manifest"]["secret"] = Value::from("credential");
        assert_eq!(
            call_preview(&mut broker, unknown).status,
            BrokerStatus::Rejected
        );
    }
}
