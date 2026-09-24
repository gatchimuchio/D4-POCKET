//! Windows向けGUI Shell書出しの独立manifestを生成するBroker境界。
//!
//! 現行単位は新規identity、監査store、設定、Runtime／Adapter manifestと配布
//! metadataの生成までであり、実build、installer、process、filesystem書込み、
//! Credential／Permission／Approval／Audit chain継承は行わない。
#![allow(non_snake_case)]

use super::compose_center;
use super::dialogue::識別子生成;
use super::protocol::{Broker, BrokerResponse, BrokerStatus, EVIDENCE_SOURCE_INTERNAL_STATE};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

const VERSION: u64 = 1;
const OPERATION: &str = "GUI Shell書出し";
const UNPRUNABLE_CORE_IDS: [&str; 8] = [
    "core.security_broker",
    "core.cryptography",
    "core.audit_finality",
    "core.credentials",
    "core.os_controls",
    "core.permission_enforcement",
    "core.approval_enforcement",
    "core.content_exposure",
];
const REQUIRED_MODULE_IDS: [&str; 10] = [
    "shell.dashboard",
    "shell.runtime_operation",
    "shell.agent_operation",
    "shell.authority",
    "shell.approval",
    "shell.audit",
    "shell.recovery",
    "shell.problems",
    "shell.evidence",
    "shell.settings",
];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportRequest {
    version: u64,
    export_id: String,
    compose_manifest: Value,
    target_platform: String,
    export_mode: String,
    distribution_channel: String,
    module_selection: Option<ModuleSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModuleSelection {
    optional_module_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModuleCatalog {
    version: u64,
    unprunable_core_ids: Vec<String>,
    required_module_ids: Vec<String>,
    optional_modules: Vec<OptionalModule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct OptionalModule {
    module_id: String,
    depends_on: Vec<String>,
    source_surfaces: Vec<String>,
}

#[derive(Debug, Clone)]
struct ModulePlan {
    receipt: Value,
}

const MODULE_CATALOG_JSON: &str = include_str!("../../../../specs/gui_shell_module_catalog.json");

pub(super) fn export(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    owner: bool,
    payload_hash: &str,
) -> BrokerResponse {
    if !owner {
        return broker.reject_with_payload_hash(
            request_id,
            OPERATION,
            "owner_required",
            "GUI Shell書出しはowner制御資格が必要",
            true,
            payload_hash,
        );
    }
    let request = match parse_request(payload) {
        Ok(request) => request,
        Err(reason) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_export_invalid",
                &reason,
                true,
                payload_hash,
            )
        }
    };
    let manifest = match compose_center::parse_manifest(&request.compose_manifest) {
        Ok(manifest) => manifest,
        Err(reason) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_export_manifest_invalid",
                &reason,
                true,
                payload_hash,
            )
        }
    };
    let manifest_value = match serde_json::to_value(manifest) {
        Ok(value) => value,
        Err(_) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_export_serialize_failed",
                "書出し元Manifestを正規化できない",
                true,
                payload_hash,
            )
        }
    };
    let module_plan = match resolve_module_plan(request.module_selection.as_ref()) {
        Ok(plan) => plan,
        Err(reason) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_export_module_selection_invalid",
                &reason,
                true,
                payload_hash,
            )
        }
    };
    let app_id = match 識別子生成() {
        Ok(id) => format!("d4-pocket-app-{id}"),
        Err(_) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_export_identity_failed",
                "新規App identityを生成できない",
                true,
                payload_hash,
            )
        }
    };
    let audit_store_id = match 識別子生成() {
        Ok(id) => format!("audit-store-{id}"),
        Err(_) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_export_audit_store_failed",
                "新規監査store identityを生成できない",
                true,
                payload_hash,
            )
        }
    };
    let audit = match broker.append_audit(
        request_id,
        OPERATION,
        "accepted",
        "Windows向け独立manifestとModule選択計画を生成。実binaryからの除去、build、installerは開始せず、Credential、Permission、Approval、Audit chainは継承しない",
        EVIDENCE_SOURCE_INTERNAL_STATE,
        payload_hash,
    ) {
        Ok(event) => event,
        Err(error) => {
            return broker.audit_store_failed_response(
                request_id,
                OPERATION,
                "broker_audit_append_failed",
                &error.message(),
            )
        }
    };
    let display_name = manifest_value["display_name"].clone();
    let settings = manifest_value["settings"].clone();
    let runtime_ids = manifest_value["runtime_ids"].clone();
    let agent_ids = manifest_value["agent_ids"].clone();
    let tool_ids = manifest_value["tool_ids"].clone();
    let mcp_connection_ids = manifest_value["mcp_connection_ids"].clone();
    let capability_requirements = manifest_value["capability_requirements"].clone();
    BrokerResponse {
        request_id: request_id.to_string(),
        operation: OPERATION.to_string(),
        status: BrokerStatus::Accepted,
        evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
        audit_event_id: audit.event_id.clone(),
        error: None,
        health: None,
        body: Some(json!({
            "version": VERSION,
            "operation": OPERATION,
            "status": "accepted",
            "export_id": request.export_id,
            "export_manifest": {
                "app_identity": {"app_id": app_id, "display_name": display_name, "target_platform": "windows"},
                "audit_store": {"store_id": audit_store_id, "chain_status": "new", "inherited": false},
                "settings": settings,
                "runtime_manifest": {"runtime_ids": runtime_ids},
                "adapter_configuration": {"agent_ids": agent_ids, "tool_ids": tool_ids, "mcp_connection_ids": mcp_connection_ids},
                "capability_requirements": capability_requirements,
                "distribution_metadata": {"target_platform": "windows", "artifact_status": "not_built", "installer_status": "not_started", "signed": false, "channel": request.distribution_channel},
                "inheritance_policy": {"authority": "none", "permission": "none", "approval": "none", "credential": "none", "audit_chain": "none"},
                "module_plan": module_plan.receipt
            },
            "build_status": "not_started",
            "artifact_status": "not_built",
            "credential_inherited": false,
            "permission_inherited": false,
            "approval_inherited": false,
            "audit_chain_inherited": false,
            "authority_strip": true,
            "evidence_source": EVIDENCE_SOURCE_INTERNAL_STATE,
            "audit_id": audit.event_id,
        })),
        shutdown_requested: broker.shutdown_requested,
    }
}

fn resolve_module_plan(selection: Option<&ModuleSelection>) -> Result<ModulePlan, String> {
    let catalog: ModuleCatalog = serde_json::from_str(MODULE_CATALOG_JSON)
        .map_err(|_| "Module一覧の正本を読み取れない".to_string())?;
    if catalog.version != 1
        || catalog.unprunable_core_ids.is_empty()
        || catalog.required_module_ids.is_empty()
    {
        return Err("Module一覧の版または必須Module定義が不正".to_string());
    }
    if !catalog_matches_fixed_modules(&catalog) {
        return Err("安全保持または必須Module一覧が固定Contractと一致しない".to_string());
    }

    let unprunable_core: HashSet<&str> = catalog
        .unprunable_core_ids
        .iter()
        .map(String::as_str)
        .collect();
    let required: HashSet<&str> = catalog
        .required_module_ids
        .iter()
        .map(String::as_str)
        .collect();
    if unprunable_core.len() != catalog.unprunable_core_ids.len()
        || required.len() != catalog.required_module_ids.len()
        || unprunable_core.iter().any(|id| required.contains(id))
    {
        return Err("必須Moduleまたは安全保持IDが重複している".to_string());
    }

    let mut optional_by_id = HashMap::new();
    for module in &catalog.optional_modules {
        if module.module_id.is_empty()
            || module.source_surfaces.is_empty()
            || module
                .source_surfaces
                .iter()
                .any(|path| path.starts_with('/') || path.contains(".."))
            || required.contains(module.module_id.as_str())
            || unprunable_core.contains(module.module_id.as_str())
            || optional_by_id
                .insert(module.module_id.as_str(), module)
                .is_some()
        {
            return Err("任意Module定義が重複または不正".to_string());
        }
    }
    for module in &catalog.optional_modules {
        let mut dependencies = HashSet::new();
        for dependency in &module.depends_on {
            if !dependencies.insert(dependency.as_str())
                || (!required.contains(dependency.as_str())
                    && !optional_by_id.contains_key(dependency.as_str()))
            {
                return Err("Module依存先が重複または一覧に存在しない".to_string());
            }
        }
    }

    let selection_is_explicit = selection.is_some();
    let requested: Vec<String> = match selection {
        Some(selection) => selection.optional_module_ids.clone(),
        None => catalog
            .optional_modules
            .iter()
            .map(|module| module.module_id.clone())
            .collect(),
    };
    if requested.len() > 32 {
        return Err("Module選択数が上限を超えている".to_string());
    }
    let mut requested_set = HashSet::new();
    for module_id in &requested {
        if !requested_set.insert(module_id.as_str()) {
            return Err("Module選択に重複IDがある".to_string());
        }
        if required.contains(module_id.as_str()) {
            return Err("必須Moduleを任意選択へ指定してはならない".to_string());
        }
        if !optional_by_id.contains_key(module_id.as_str()) {
            return Err("未知のModule IDがある".to_string());
        }
    }

    fn include_with_dependencies(
        module_id: &str,
        required: &HashSet<&str>,
        optional_by_id: &HashMap<&str, &OptionalModule>,
        included: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
    ) -> Result<(), String> {
        if required.contains(module_id) || included.contains(module_id) {
            return Ok(());
        }
        let module = optional_by_id
            .get(module_id)
            .ok_or_else(|| "Module依存先が一覧に存在しない".to_string())?;
        if !visiting.insert(module_id.to_string()) {
            return Err("Module依存に循環がある".to_string());
        }
        for dependency in &module.depends_on {
            include_with_dependencies(dependency, required, optional_by_id, included, visiting)?;
        }
        visiting.remove(module_id);
        included.insert(module_id.to_string());
        Ok(())
    }

    for module in &catalog.optional_modules {
        let mut catalog_included = HashSet::new();
        let mut catalog_visiting = HashSet::new();
        include_with_dependencies(
            &module.module_id,
            &required,
            &optional_by_id,
            &mut catalog_included,
            &mut catalog_visiting,
        )?;
    }

    let mut included_optional = HashSet::new();
    let mut visiting = HashSet::new();
    for module_id in &requested {
        include_with_dependencies(
            module_id,
            &required,
            &optional_by_id,
            &mut included_optional,
            &mut visiting,
        )?;
    }

    let mandatory_ids = catalog.required_module_ids.clone();
    let included_ids: Vec<String> = catalog
        .unprunable_core_ids
        .iter()
        .cloned()
        .chain(
            mandatory_ids.iter().cloned().chain(
                catalog
                    .optional_modules
                    .iter()
                    .filter(|module| included_optional.contains(&module.module_id))
                    .map(|module| module.module_id.clone()),
            ),
        )
        .collect();
    let excluded_ids: Vec<String> = catalog
        .optional_modules
        .iter()
        .filter(|module| !included_optional.contains(&module.module_id))
        .map(|module| module.module_id.clone())
        .collect();
    let requested_ids: Vec<String> = catalog
        .optional_modules
        .iter()
        .filter(|module| requested_set.contains(module.module_id.as_str()))
        .map(|module| module.module_id.clone())
        .collect();

    Ok(ModulePlan {
        receipt: json!({
            "catalog_version": catalog.version,
            "selection_mode": if selection_is_explicit { "explicit_optional" } else { "all_optional" },
            "requested_optional_module_ids": requested_ids,
            "unprunable_core_ids": catalog.unprunable_core_ids,
            "mandatory_module_ids": mandatory_ids,
            "included_module_ids": included_ids,
            "excluded_optional_module_ids": excluded_ids,
            "binary_pruning_status": "not_applied"
        }),
    })
}

fn catalog_matches_fixed_modules(catalog: &ModuleCatalog) -> bool {
    let configured_core_ids: Vec<&str> = catalog
        .unprunable_core_ids
        .iter()
        .map(String::as_str)
        .collect();
    let configured_required_ids: Vec<&str> = catalog
        .required_module_ids
        .iter()
        .map(String::as_str)
        .collect();
    configured_core_ids.as_slice() == UNPRUNABLE_CORE_IDS.as_slice()
        && configured_required_ids.as_slice() == REQUIRED_MODULE_IDS.as_slice()
}

fn parse_request(value: &Value) -> Result<ExportRequest, String> {
    let request: ExportRequest = serde_json::from_value(value.clone())
        .map_err(|_| "GUI Shell書出しの構造が不正または禁止fieldがある".to_string())?;
    if request.version != VERSION
        || !valid_identifier(&request.export_id)
        || request.target_platform != "windows"
        || request.export_mode != "manifest_only"
        || !["local", "installer_candidate"].contains(&request.distribution_channel.as_str())
    {
        return Err("書出し版、識別子、対象platform、出力mode、配布channelが不正".to_string());
    }
    Ok(request)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= 64
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::protocol::{Broker, BrokerOperation, BrokerRequestEnvelope, BrokerStatus};

    fn manifest() -> Value {
        json!({
            "version": 1, "compose_id": "d4-pocket-local", "display_name": "D4 Pocket ローカル構成",
            "runtime_ids": ["gui_shell_rust_broker"], "agent_ids": ["codex"], "tool_ids": [], "mcp_connection_ids": [],
            "theme": {"theme_id": "d4-pocket", "mode": "system"}, "capability_requirements": ["runtime.read"],
            "settings": {"locale": "ja-JP", "density": "comfortable", "content_visibility": "summary"},
            "inheritance_policy": {"authority": "none", "permission": "none", "approval": "none", "credential": "none", "audit_chain": "none"},
            "output_mode": "manifest_only"
        })
    }

    fn payload() -> Value {
        json!({"version": 1, "export_id": "export-test", "compose_manifest": manifest(), "target_platform": "windows", "export_mode": "manifest_only", "distribution_channel": "local"})
    }

    fn owner_request(broker: &mut Broker, payload: Value) -> String {
        let nonce = format!("owner-export-nonce-{}", broker.audit_events().len());
        let payload_hash = crate::broker::protocol::canonical_payload_hash(Some(&payload));
        json!({"request_id": "export-owner-test", "session_id": "session-1", "operation": OPERATION, "payload": payload, "payload_hash": payload_hash, "nonce": nonce, "issued_at": BrokerRequestEnvelope::current_issued_at(), "metadata": {}}).to_string()
    }

    #[test]
    fn Module一覧から必須安全境界または画面を削れない() {
        let mut catalog: ModuleCatalog = serde_json::from_str(MODULE_CATALOG_JSON).unwrap();
        assert!(catalog_matches_fixed_modules(&catalog));

        catalog.unprunable_core_ids.pop();
        assert!(!catalog_matches_fixed_modules(&catalog));

        let mut catalog: ModuleCatalog = serde_json::from_str(MODULE_CATALOG_JSON).unwrap();
        catalog.required_module_ids.pop();
        assert!(!catalog_matches_fixed_modules(&catalog));
    }

    #[test]
    fn 書出しは新規identityと監査storeを生成するが権限を継承しない() {
        let mut broker = Broker::new("session-1");
        let raw = owner_request(&mut broker, payload());
        let response = broker.owner要求処理(&raw);
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.unwrap();
        assert!(body["export_manifest"]["app_identity"]["app_id"]
            .as_str()
            .unwrap()
            .starts_with("d4-pocket-app-"));
        assert_eq!(body["export_manifest"]["audit_store"]["inherited"], false);
        assert_eq!(body["build_status"], "not_started");
        assert_eq!(body["credential_inherited"], false);
        assert_eq!(body["permission_inherited"], false);
        assert_eq!(body["approval_inherited"], false);
        assert_eq!(body["audit_chain_inherited"], false);
        assert_eq!(
            body["export_manifest"]["module_plan"]["selection_mode"],
            "all_optional"
        );
        assert_eq!(
            body["export_manifest"]["module_plan"]["binary_pruning_status"],
            "not_applied"
        );
        let included = body["export_manifest"]["module_plan"]["included_module_ids"]
            .as_array()
            .unwrap();
        assert!(included.contains(&Value::from("shell.audit")));
        for protected in [
            "core.security_broker",
            "core.cryptography",
            "core.audit_finality",
            "core.credentials",
            "core.os_controls",
            "core.permission_enforcement",
            "core.approval_enforcement",
            "core.content_exposure",
        ] {
            assert!(included.contains(&Value::from(protected)));
        }
    }

    #[test]
    fn 明示選択は必須Moduleを保持し依存Moduleを閉包する() {
        let mut broker = Broker::new("session-1");
        let mut request = payload();
        request["module_selection"] = json!({"optional_module_ids": ["shell.trace_inspector"]});
        let raw = owner_request(&mut broker, request);
        let response = broker.owner要求処理(&raw);
        assert_eq!(response.status, BrokerStatus::Accepted);
        let plan = &response.body.unwrap()["export_manifest"]["module_plan"];
        assert_eq!(plan["selection_mode"], "explicit_optional");
        assert_eq!(plan["binary_pruning_status"], "not_applied");
        let included = plan["included_module_ids"].as_array().unwrap();
        for required in [
            "shell.authority",
            "shell.approval",
            "shell.audit",
            "shell.recovery",
            "core.security_broker",
            "core.cryptography",
            "core.audit_finality",
            "core.credentials",
            "core.os_controls",
            "core.permission_enforcement",
            "core.approval_enforcement",
            "core.content_exposure",
        ] {
            assert!(included.contains(&Value::from(required)));
        }
        assert!(included.contains(&Value::from("shell.observability")));
        assert!(included.contains(&Value::from("shell.trace_inspector")));
        assert!(plan["excluded_optional_module_ids"]
            .as_array()
            .unwrap()
            .contains(&Value::from("shell.history")));
    }

    #[test]
    fn 空選択でも必須Moduleだけは除去できない() {
        let mut broker = Broker::new("session-1");
        let mut request = payload();
        request["module_selection"] = json!({"optional_module_ids": []});
        let raw = owner_request(&mut broker, request);
        let response = broker.owner要求処理(&raw);
        assert_eq!(response.status, BrokerStatus::Accepted);
        let plan = &response.body.unwrap()["export_manifest"]["module_plan"];
        let included = plan["included_module_ids"].as_array().unwrap();
        assert_eq!(included.len(), 18);
        for protected in plan["unprunable_core_ids"].as_array().unwrap() {
            assert!(included.contains(protected));
        }
        for mandatory in plan["mandatory_module_ids"].as_array().unwrap() {
            assert!(included.contains(mandatory));
        }
        assert_eq!(
            plan["excluded_optional_module_ids"]
                .as_array()
                .unwrap()
                .len(),
            8
        );
    }

    #[test]
    fn 未知重複または必須Moduleの任意指定を拒否する() {
        for selection in [
            json!({"optional_module_ids": ["shell.unknown"]}),
            json!({"optional_module_ids": ["shell.history", "shell.history"]}),
            json!({"optional_module_ids": ["shell.audit"]}),
        ] {
            let mut broker = Broker::new("session-1");
            let mut request = payload();
            request["module_selection"] = selection;
            let raw = owner_request(&mut broker, request);
            assert_eq!(broker.owner要求処理(&raw).status, BrokerStatus::Rejected);
        }
    }

    #[test]
    fn 書出しは通常資格と継承要求を拒否する() {
        let mut broker = Broker::new("session-1");
        let mut normal = BrokerRequestEnvelope::command_envelope_at(
            "export-normal",
            "session-1",
            "export-normal-nonce",
            &BrokerRequestEnvelope::current_issued_at(),
        );
        normal.operation = Some(BrokerOperation::GuiShell書出し);
        normal.payload = Some(payload());
        normal.refresh_payload_hash();
        assert_eq!(broker.handle(normal).status, BrokerStatus::Rejected);

        let mut invalid = payload();
        invalid["compose_manifest"]["inheritance_policy"]["credential"] =
            Value::from("credential.value");
        let raw = owner_request(&mut broker, invalid);
        assert_eq!(broker.owner要求処理(&raw).status, BrokerStatus::Rejected);
    }
}
