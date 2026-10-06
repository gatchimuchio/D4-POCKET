//! Windows向けGUI Shell書出しの独立manifestを生成するBroker境界。
//!
//! 現行単位は新規identity、監査store、設定、Runtime／Adapter manifestと配布
//! metadataを固定Export directoryへ保存するまでであり、実build、installer、process、
//! Credential／Permission／Approval／Audit chain継承は行わない。
#![allow(non_snake_case)]

use super::compose_center;
use super::dialogue::識別子生成;
use super::protocol::{Broker, BrokerResponse, BrokerStatus, OwnerConfirmationSource, EVIDENCE_SOURCE_INTERNAL_STATE};
use super::update_center::{self, UpdatePackageSource, UpdateTrust};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptions};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use unicode_normalization::UnicodeNormalization;

const VERSION: u64 = 1;
const MAX_MANIFEST_FILE_BYTES: usize = 64 * 1024;
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
    update_trust: Option<ExportUpdateTrustInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportUpdateTrustInput {
    public_key_der_hex: String,
    package_sources: Vec<UpdatePackageSource>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnerConfirmationSummary {
    pub display_name: String,
    pub export_id: String,
    pub distribution_channel: String,
    pub optional_module_count: usize,
    pub update_trust: Option<UpdateTrustConfirmationSummary>,
    pub payload_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UpdateTrustConfirmationSummary {
    pub public_key_fingerprint: String,
    pub package_source_details: Vec<String>,
}

/// Owner dialogへ渡す表示情報をBrokerと同じContract検査から作る。
/// 任意のpayload文字列やmetadataをダイアログ本文へ直接埋め込まない。
pub(crate) fn owner_confirmation_summary(payload: &Value, payload_hash: &str) -> Result<OwnerConfirmationSummary, String> {
    let request = parse_request(payload)?;
    let manifest = compose_center::parse_manifest(&request.compose_manifest)?;
    let manifest_value = serde_json::to_value(manifest)
        .map_err(|_| "書出し元Manifestを正規化できない".to_string())?;
    let display_name = manifest_value["display_name"]
        .as_str()
        .ok_or_else(|| "書出し表示名を確認できない".to_string())?;
    let safe_display_name: String = display_name.nfkc().filter(|character| {
        !character.is_control()
            && !matches!(character, '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{feff}' | '\u{2028}' | '\u{2029}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
    }).take(96).collect();
    if safe_display_name.trim().is_empty() {
        return Err("Owner確認に表示できる書出し名がない".to_string());
    }
    let module_plan = resolve_module_plan(request.module_selection.as_ref())?;
    let optional_module_count = module_plan.receipt["requested_optional_module_ids"]
        .as_array()
        .ok_or_else(|| "書出しModule計画を確認できない".to_string())?
        .len();
    let update_trust = normalize_update_trust(request.update_trust.as_ref())?.map(|trust| {
        UpdateTrustConfirmationSummary {
            public_key_fingerprint: trust.public_key_fingerprint,
            package_source_details: trust
                .package_sources
                .iter()
                .map(|source| format!("{}: {}", source.channel, source.base_url))
                .collect(),
        }
    });
    Ok(OwnerConfirmationSummary {
        display_name: safe_display_name,
        export_id: request.export_id,
        distribution_channel: request.distribution_channel,
        optional_module_count,
        update_trust,
        payload_hash: payload_hash.to_string(),
    })
}

pub(super) fn export(
    broker: &mut Broker,
    request_id: &str,
    payload: &Value,
    owner: bool,
    owner_confirmation: OwnerConfirmationSource,
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
    let update_trust = match normalize_update_trust(request.update_trust.as_ref()) {
        Ok(trust) => trust,
        Err(reason) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_export_update_trust_invalid",
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
    let display_name = manifest_value["display_name"].clone();
    let settings = manifest_value["settings"].clone();
    let runtime_ids = manifest_value["runtime_ids"].clone();
    let agent_ids = manifest_value["agent_ids"].clone();
    let tool_ids = manifest_value["tool_ids"].clone();
    let mcp_connection_ids = manifest_value["mcp_connection_ids"].clone();
    let capability_requirements = manifest_value["capability_requirements"].clone();
    let export_manifest = json!({
        "app_identity": {"app_id": app_id, "display_name": display_name, "target_platform": "windows"},
        "audit_store": {"store_id": audit_store_id, "chain_status": "new", "inherited": false},
        "settings": settings,
        "runtime_manifest": {"runtime_ids": runtime_ids},
        "adapter_configuration": {"agent_ids": agent_ids, "tool_ids": tool_ids, "mcp_connection_ids": mcp_connection_ids},
        "capability_requirements": capability_requirements,
        "distribution_metadata": {"target_platform": "windows", "artifact_status": "not_built", "installer_status": "not_started", "signed": false, "channel": request.distribution_channel},
        "inheritance_policy": {"authority": "none", "permission": "none", "approval": "none", "credential": "none", "audit_chain": "none"},
        "module_plan": module_plan.receipt
    });
    let mut manifest_document = json!({
        "version": VERSION,
        "product": "D4 Pocket",
        "export_id": request.export_id.clone(),
        "manifest": export_manifest
    });
    if let Some(update_trust) = update_trust {
        let normalized = match serde_json::to_value(update_trust) {
            Ok(value) => value,
            Err(_) => {
                return broker.reject_with_payload_hash(
                    request_id,
                    OPERATION,
                    "gui_shell_export_serialize_failed",
                    "更新trust設定を正規化できない",
                    true,
                    payload_hash,
                )
            }
        };
        manifest_document["update_trust"] = normalized;
    }
    let manifest_bytes = match serde_json::to_vec_pretty(&manifest_document) {
        Ok(mut bytes) => {
            bytes.push(b'\n');
            bytes
        }
        Err(_) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_export_serialize_failed",
                "独立Manifest fileを正規化できない",
                true,
                payload_hash,
            )
        }
    };
    if manifest_bytes.is_empty() || manifest_bytes.len() > MAX_MANIFEST_FILE_BYTES {
        return broker.reject_with_payload_hash(
            request_id,
            OPERATION,
            "gui_shell_export_manifest_too_large",
            "独立Manifest fileが保存上限を超えています。構成を縮小してください。",
            true,
            payload_hash,
        );
    }
    if broker.desktop_export_root.is_none() {
        return broker.reject_with_payload_hash(
            request_id,
            OPERATION,
            "gui_shell_export_target_unavailable",
            "Rust Desktop起動器が固定Manifest保存先を用意していないため、書出しを停止しました。",
            true,
            payload_hash,
        );
    }
    let app_id = export_manifest["app_identity"]["app_id"]
        .as_str()
        .expect("Brokerが生成したApp identity");
    let file_name = format!("{app_id}.json");
    let prepared_audit = match broker.append_audit(
        &format!("{request_id}:manifest-prepared"),
        OPERATION,
        "received",
        &format!("Capability=gui_shell.export_manifest_file Permission=固定Export directoryへの新規単一file作成 Approval=Owner確認済み RecoveryAction=file作成前の要求受理記録。file hash={} file_name={file_name}", crate::audit_hash::sha256_tagged(&manifest_bytes)),
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
    let (export_root_path, export_root) = broker
        .desktop_export_root
        .as_ref()
        .expect("固定Export保存先を事前確認済み");
    let manifest_file = match write_manifest_file(
        export_root,
        export_root_path,
        app_id,
        &manifest_bytes,
    ) {
        Ok(file) => file,
        Err(reason) => {
            return broker.reject_with_payload_hash(
                request_id,
                OPERATION,
                "gui_shell_export_file_write_failed",
                &reason,
                true,
                payload_hash,
            )
        }
    };
    let confirmation_reason = match owner_confirmation {
        OwnerConfirmationSource::DesktopNativeConfirmation => "OwnerがRust Desktop起動器のネイティブ確認でpayload hashを確認して明示許可。新規Manifest fileを固定Export directoryに作成し、file hashをAuditEventへ結合。実行可能App、binary pruning、build、installer、署名は未実行。Credential、Permission、Approval、Audit chainは継承しない",
        OwnerConfirmationSource::OwnerCredential => "Owner制御資格による書出し操作を受理。新規Manifest fileを固定Export directoryに作成し、file hashをAuditEventへ結合。実行可能App、binary pruning、build、installer、署名は未実行。Credential、Permission、Approval、Audit chainは継承しない",
        OwnerConfirmationSource::NotOwner => "Owner制御がない書出し要求を拒否すべき経路へ到達した",
    };
    let audit = match broker.append_audit(
        request_id,
        OPERATION,
        "accepted",
        &format!(
            "{confirmation_reason}; 保存path={} file hash={} 一時file状態={} 一時file名={} 復旧操作={}",
            manifest_file.path,
            manifest_file.sha256,
            manifest_file.temporary_file_status,
            manifest_file.temporary_file_name,
            manifest_file.recovery_action,
        ),
        EVIDENCE_SOURCE_INTERNAL_STATE,
        &manifest_file.sha256,
    ) {
        Ok(event) => event,
        Err(_error) => {
            return super::protocol::BrokerResponse {
                request_id: request_id.to_string(),
                operation: OPERATION.to_string(),
                status: BrokerStatus::Suspended,
                evidence_source: EVIDENCE_SOURCE_INTERNAL_STATE.to_string(),
                audit_event_id: prepared_audit.event_id,
                error: Some(super::protocol::BrokerError {
                    code: "gui_shell_export_audit_finalize_failed".to_string(),
                    message: format!(
                        "Manifest fileは生成されましたが、完了Auditを確定できませんでした。配布へ進まず、上書きや削除をせず、保存path={} と file hash={} を照合してAudit状態を復旧してください。一時file状態={} 一時file名={} 復旧操作={}",
                        manifest_file.path,
                        manifest_file.sha256,
                        manifest_file.temporary_file_status,
                        manifest_file.temporary_file_name,
                        manifest_file.recovery_action,
                    ),
                    recoverable: true,
                    audit_event_required: true,
                    fail_closed: true,
                }),
                health: None,
                body: None,
                shutdown_requested: broker.shutdown_requested,
            };
        }
    };
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
            "export_manifest": export_manifest,
            "manifest_file_status": "written",
            "manifest_file": {
                "file_name": manifest_file.file_name,
                "path": manifest_file.path,
                "sha256": manifest_file.sha256,
                "byte_length": manifest_file.byte_length,
                "temporary_file_status": manifest_file.temporary_file_status,
                "temporary_file_name": manifest_file.temporary_file_name,
                "recovery_action": manifest_file.recovery_action,
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

struct ManifestFile {
    file_name: String,
    path: String,
    sha256: String,
    byte_length: usize,
    temporary_file_status: &'static str,
    temporary_file_name: String,
    recovery_action: &'static str,
}

fn write_manifest_file(
    root: &Dir,
    root_path: &Path,
    app_id: &str,
    bytes: &[u8],
) -> Result<ManifestFile, String> {
    write_manifest_file_with_cleanup(root, root_path, app_id, bytes, |root, name| {
        root.remove_file(name)
    })
}

fn write_manifest_file_with_cleanup(
    root: &Dir,
    root_path: &Path,
    app_id: &str,
    bytes: &[u8],
    remove_temporary: impl FnOnce(&Dir, &str) -> std::io::Result<()>,
) -> Result<ManifestFile, String> {
    let random_suffix = app_id
        .strip_prefix("d4-pocket-app-")
        .filter(|suffix| suffix.len() == 32 && suffix.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()))
        .ok_or_else(|| "Broker生成のApp identityが不正なためfileを作成しない".to_string())?;
    if bytes.is_empty() || bytes.len() > MAX_MANIFEST_FILE_BYTES {
        return Err("Manifest fileが空または保存上限超過のため作成しない".to_string());
    }
    let file_name = format!("d4-pocket-app-{random_suffix}.json");
    let temporary_name = format!(".{random_suffix}.manifest.tmp");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).follow(FollowSymlinks::No);
    let mut file = match root.open_with(&temporary_name, &options) {
        Ok(file) => file,
        Err(_) => return Err("Manifest一時fileを新規作成できない".to_string()),
    };
    let write_error = file
        .write_all(bytes)
        .err()
        .map(|_| "Manifest一時fileを書き込めない".to_string())
        .or_else(|| {
            file.sync_all()
                .err()
                .map(|_| "Manifest一時fileを永続化できない".to_string())
        });
    drop(file);
    if let Some(error) = write_error {
        return Err(with_temporary_cleanup_hint(root, &temporary_name, error));
    }
    if root.hard_link(&temporary_name, root, &file_name).is_err() {
        return Err(with_temporary_cleanup_hint(
            root,
            &temporary_name,
            "既存fileを置換せずManifest fileを確定できない".to_string(),
        ));
    }
    let cleanup_pending = remove_temporary(root, &temporary_name).is_err();
    Ok(ManifestFile {
        file_name: file_name.clone(),
        path: root_path.join(file_name).to_string_lossy().into_owned(),
        sha256: crate::audit_hash::sha256_tagged(bytes),
        byte_length: bytes.len(),
        temporary_file_status: if cleanup_pending { "cleanup_pending" } else { "removed" },
        temporary_file_name: temporary_name,
        recovery_action: if cleanup_pending {
            "owner_review_and_remove_after_hash_verification"
        } else {
            "none"
        },
    })
}

fn with_temporary_cleanup_hint(root: &Dir, temporary_name: &str, error: String) -> String {
    if root.remove_file(temporary_name).is_err() {
        format!("{error}。残存の可能性がある一時file名: {temporary_name}。配布せずOwnerが確認してください")
    } else {
        error
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

fn normalize_update_trust(
    input: Option<&ExportUpdateTrustInput>,
) -> Result<Option<UpdateTrust>, String> {
    let Some(input) = input else {
        return Ok(None);
    };
    if input.public_key_der_hex.len() != 88
        || !input
            .public_key_der_hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || input.package_sources.is_empty()
    {
        return Err("更新trustは小文字hexのEd25519公開鍵と配布元が必要".to_string());
    }
    let der = hex::decode(&input.public_key_der_hex)
        .map_err(|_| "更新trust公開鍵の形式が不正".to_string())?;
    let public_key = crate::checkpoint::public_key(&der)
        .map_err(|_| "更新trustにはEd25519 SPKI公開鍵が必要".to_string())?;
    let trust = UpdateTrust {
        version: 2,
        algorithm: "Ed25519".to_string(),
        public_key_der_hex: input.public_key_der_hex.clone(),
        public_key_fingerprint: crate::audit_hash::sha256_tagged(public_key),
        package_sources: input.package_sources.clone(),
    };
    update_center::validate_trust(&trust)
        .map_err(|_| "更新trustのchannelまたはHTTPS配布元が不正".to_string())?;
    Ok(Some(trust))
}

fn parse_request(value: &Value) -> Result<ExportRequest, String> {
    if value
        .as_object()
        .is_some_and(|object| object.contains_key("update_trust") && value["update_trust"].is_null())
    {
        return Err("update_trustは省略するか、有効な設定objectを指定する".to_string());
    }
    let request: ExportRequest = serde_json::from_value(value.clone())
        .map_err(|_| "GUI Shell書出しの構造が不正または禁止fieldがある".to_string())?;
    if request.version != VERSION
        || !valid_identifier(&request.export_id)
        || request.target_platform != "windows"
        || request.export_mode != "manifest_file"
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
    use std::fs;

    fn update_trust_input() -> Value {
        json!({
            "public_key_der_hex": "302a300506032b6570032100d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
            "package_sources": [{
                "channel": "stable",
                "base_url": "https://updates.example.com/d4-pocket/stable"
            }]
        })
    }

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
        json!({"version": 1, "export_id": "export-test", "compose_manifest": manifest(), "target_platform": "windows", "export_mode": "manifest_file", "distribution_channel": "local"})
    }

    fn configure_export_root(broker: &mut Broker, label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "gui-shell-export-{label}-{}",
            crate::broker::dialogue::識別子生成().unwrap()
        ));
        fs::create_dir(&path).unwrap();
        let canonical = fs::canonicalize(&path).unwrap();
        let root = Dir::open_ambient_dir(&canonical, cap_std::ambient_authority()).unwrap();
        broker.set_desktop_export_root(canonical.clone(), root);
        canonical
    }

    fn owner_request(broker: &mut Broker, payload: Value) -> String {
        let nonce = format!("owner-export-nonce-{}", broker.audit_events().len());
        let payload_hash = crate::broker::protocol::canonical_payload_hash(Some(&payload));
        json!({"request_id": "export-owner-test", "session_id": "session-1", "operation": OPERATION, "payload": payload, "payload_hash": payload_hash, "nonce": nonce, "issued_at": BrokerRequestEnvelope::current_issued_at(), "metadata": {}}).to_string()
    }

    fn desktop_owner_request(broker: &mut Broker, payload: Value) -> String {
        let mut request: Value = serde_json::from_str(&owner_request(broker, payload)).unwrap();
        request["metadata"] = json!({"client": "desktop_flutter"});
        request.to_string()
    }

    #[test]
    fn Manifest_fileは新規作成だけを許可し既存fileを置換しない() {
        let root_path = std::env::temp_dir().join(format!(
            "gui-shell-export-write-{}",
            crate::broker::dialogue::識別子生成().unwrap()
        ));
        fs::create_dir(&root_path).unwrap();
        let canonical = fs::canonicalize(&root_path).unwrap();
        let root = Dir::open_ambient_dir(&canonical, cap_std::ambient_authority()).unwrap();
        let app_id = "d4-pocket-app-0123456789abcdef0123456789abcdef";
        let first = write_manifest_file(&root, &canonical, app_id, b"first manifest").unwrap();
        let second = write_manifest_file(&root, &canonical, app_id, b"replacement");
        assert!(second.is_err());
        assert_eq!(fs::read(&first.path).unwrap(), b"first manifest");
        drop(root);
        fs::remove_dir_all(root_path).unwrap();
    }

    #[test]
    fn Manifest確定後の一時file削除失敗は生成成功と復旧要求を記録する() {
        let root_path = std::env::temp_dir().join(format!(
            "gui-shell-export-cleanup-{}",
            crate::broker::dialogue::識別子生成().unwrap()
        ));
        fs::create_dir(&root_path).unwrap();
        let canonical = fs::canonicalize(&root_path).unwrap();
        let root = Dir::open_ambient_dir(&canonical, cap_std::ambient_authority()).unwrap();
        let app_id = "d4-pocket-app-abcdef0123456789abcdef0123456789";
        let artifact = write_manifest_file_with_cleanup(
            &root,
            &canonical,
            app_id,
            b"manifest with cleanup pending",
            |_root, _name| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "試験用一時file削除失敗",
                ))
            },
        )
        .unwrap();
        assert_eq!(artifact.temporary_file_status, "cleanup_pending");
        assert_eq!(
            artifact.recovery_action,
            "owner_review_and_remove_after_hash_verification"
        );
        assert_eq!(fs::read(&artifact.path).unwrap(), b"manifest with cleanup pending");
        assert!(canonical.join(&artifact.temporary_file_name).exists());
        root.remove_file(&artifact.temporary_file_name).unwrap();
        drop(root);
        fs::remove_dir_all(root_path).unwrap();
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
        let export_root = configure_export_root(&mut broker, "fresh-identity");
        let raw = desktop_owner_request(&mut broker, payload());
        let response = broker.desktop_owner_operation_json(&raw);
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.unwrap();
        let app_id = body["export_manifest"]["app_identity"]["app_id"]
            .as_str()
            .unwrap();
        let audit_store_id = body["export_manifest"]["audit_store"]["store_id"]
            .as_str()
            .unwrap();
        for (identifier, prefix) in [(app_id, "d4-pocket-app-"), (audit_store_id, "audit-store-")] {
            let suffix = identifier.strip_prefix(prefix).unwrap();
            assert_eq!(suffix.len(), 32);
            assert!(suffix
                .bytes()
                .all(|byte| { byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte) }));
        }
        assert_eq!(body["export_manifest"]["audit_store"]["inherited"], false);
        assert_eq!(body["build_status"], "not_started");
        assert_eq!(body["manifest_file_status"], "written");
        assert_eq!(body["manifest_file"]["temporary_file_status"], "removed");
        assert_eq!(body["manifest_file"]["recovery_action"], "none");
        let manifest_path = body["manifest_file"]["path"].as_str().unwrap();
        let manifest_bytes = fs::read(manifest_path).unwrap();
        assert_eq!(
            crate::audit_hash::sha256_tagged(&manifest_bytes),
            body["manifest_file"]["sha256"]
        );
        let file_document: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        assert_eq!(file_document["manifest"]["app_identity"], body["export_manifest"]["app_identity"]);
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
        drop(broker);
        fs::remove_dir_all(export_root).unwrap();
    }

    #[test]
    fn Owner確認summaryはManifest検証済みの表示値とhashだけを返す() {
        let mut request_payload = payload();
        request_payload["compose_manifest"]["display_name"] =
            Value::from("D4 Pocket\n\u{202e}\u{2028}危険名");
        request_payload["module_selection"] = json!({"optional_module_ids": []});
        let summary = owner_confirmation_summary(
            &request_payload,
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ).unwrap();
        assert_eq!(summary.display_name, "D4 Pocket危険名");
        assert_eq!(summary.export_id, "export-test");
        assert_eq!(summary.optional_module_count, 0);
        assert!(!summary.display_name.contains('\n'));
        assert!(!summary.display_name.contains('\u{202e}'));
        assert!(!summary.display_name.contains('\u{2028}'));
        assert_eq!(summary.payload_hash.len(), 71);

        request_payload["module_selection"] = json!({"optional_module_ids": ["shell.unknown"]});
        assert!(owner_confirmation_summary(&request_payload, "sha256:invalid").is_err());
    }

    #[test]
    fn 更新trustはBroker検証後にManifestへ固定しOwner確認summaryへfingerprintと配布元を示す() {
        let mut request_payload = payload();
        request_payload["update_trust"] = update_trust_input();
        let summary = owner_confirmation_summary(
            &request_payload,
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap();
        let trust_summary = summary.update_trust.unwrap();
        assert_eq!(
            trust_summary.package_source_details,
            vec!["stable: https://updates.example.com/d4-pocket/stable"]
        );
        assert!(trust_summary.public_key_fingerprint.starts_with("sha256:"));

        let mut broker = Broker::new("session-1");
        let export_root = configure_export_root(&mut broker, "update-trust");
        let request_json = desktop_owner_request(&mut broker, request_payload);
        let response = broker.desktop_owner_operation_json(&request_json);
        assert_eq!(response.status, BrokerStatus::Accepted);
        let body = response.body.unwrap();
        let manifest_path = body["manifest_file"]["path"].as_str().unwrap();
        let manifest: Value =
            serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
        assert_eq!(manifest["update_trust"]["版"], 2);
        assert_eq!(manifest["update_trust"]["algorithm"], "Ed25519");
        assert_eq!(
            manifest["update_trust"]["public_key_fingerprint"],
            trust_summary.public_key_fingerprint
        );
        assert_eq!(
            manifest["update_trust"]["package_sources"][0]["base_url"],
            "https://updates.example.com/d4-pocket/stable"
        );
        drop(broker);
        fs::remove_dir_all(export_root).unwrap();
    }

    #[test]
    fn 不正な更新trustと危険な配布元はExport前に拒否する() {
        let mut null_trust = payload();
        null_trust["update_trust"] = Value::Null;
        assert!(owner_confirmation_summary(&null_trust, "sha256:invalid").is_err());

        let mut malformed_key = payload();
        malformed_key["update_trust"] = update_trust_input();
        malformed_key["update_trust"]["public_key_der_hex"] = Value::from("not-a-key");
        assert!(owner_confirmation_summary(&malformed_key, "sha256:invalid").is_err());

        let mut unsafe_source = payload();
        unsafe_source["update_trust"] = update_trust_input();
        unsafe_source["update_trust"]["package_sources"][0]["base_url"] =
            Value::from("https://user@updates.example.com/d4-pocket/stable");
        assert!(owner_confirmation_summary(&unsafe_source, "sha256:invalid").is_err());
    }

    #[test]
    fn RustネイティブOwner確認だけがDesktop書出しを受理し監査へ起点を残す() {
        let mut broker = Broker::new("session-1");
        let export_root = configure_export_root(&mut broker, "native-owner");
        let mut request: Value = serde_json::from_str(&owner_request(&mut broker, payload())).unwrap();
        request["metadata"] = json!({"client": "desktop_flutter"});
        let response = broker.desktop_owner_operation_json(&request.to_string());
        assert_eq!(response.status, BrokerStatus::Accepted);
        let audit = broker.audit_events().last().unwrap();
        assert_eq!(audit.operation, OPERATION);
        assert!(audit.reason.contains("Rust Desktop起動器のネイティブ確認"));
        assert_eq!(audit.evidence_source, EVIDENCE_SOURCE_INTERNAL_STATE);
        assert_eq!(response.body.unwrap()["authority_strip"], true);
        drop(broker);
        fs::remove_dir_all(export_root).unwrap();
    }

    #[test]
    fn DesktopOwner内部経路は別operationと不正client_metadataを拒否する() {
        let mut broker = Broker::new("session-1");
        let unrelated = json!({
            "request_id": "desktop-health-not-export",
            "operation": "health",
            "payload_hash": crate::broker::protocol::canonical_payload_hash(None),
            "nonce": "desktop-health-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "session_id": "session-1",
            "metadata": {"client": "desktop_flutter"}
        });
        let rejected = broker.desktop_owner_operation_json(&unrelated.to_string());
        assert_eq!(rejected.status, BrokerStatus::Rejected);
        assert_eq!(rejected.error.unwrap().code, "desktop_owner_operation_invalid");

        let mut request: Value = serde_json::from_str(&owner_request(&mut broker, payload())).unwrap();
        request["metadata"] = json!({"client": "desktop_flutter", "owner": true});
        let rejected = broker.desktop_owner_operation_json(&request.to_string());
        assert_eq!(rejected.status, BrokerStatus::Rejected);
        assert_eq!(rejected.error.unwrap().code, "desktop_owner_operation_invalid");
    }

    #[test]
    fn 明示選択は必須Moduleを保持し依存Moduleを閉包する() {
        let mut broker = Broker::new("session-1");
        let export_root = configure_export_root(&mut broker, "module-closure");
        let mut request = payload();
        request["module_selection"] = json!({"optional_module_ids": ["shell.trace_inspector"]});
        let raw = desktop_owner_request(&mut broker, request);
        let response = broker.desktop_owner_operation_json(&raw);
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
        drop(broker);
        fs::remove_dir_all(export_root).unwrap();
    }

    #[test]
    fn 空選択でも必須Moduleだけは除去できない() {
        let mut broker = Broker::new("session-1");
        let export_root = configure_export_root(&mut broker, "empty-selection");
        let mut request = payload();
        request["module_selection"] = json!({"optional_module_ids": []});
        let raw = desktop_owner_request(&mut broker, request);
        let response = broker.desktop_owner_operation_json(&raw);
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
        drop(broker);
        fs::remove_dir_all(export_root).unwrap();
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
            let raw = desktop_owner_request(&mut broker, request);
            assert_eq!(
                broker.desktop_owner_operation_json(&raw).status,
                BrokerStatus::Rejected
            );
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
        let raw = desktop_owner_request(&mut broker, invalid);
        assert_eq!(
            broker.desktop_owner_operation_json(&raw).status,
            BrokerStatus::Rejected
        );
    }

    #[test]
    fn Owner資格だけのExport要求はManifestを作らずnative確認必須として拒否する() {
        let mut broker = Broker::new("session-1");
        let export_root = configure_export_root(&mut broker, "owner-credential-denied");
        let raw = owner_request(&mut broker, payload());
        let response = broker.owner要求処理(&raw);
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(
            response.error.as_ref().map(|error| error.code.as_str()),
            Some("desktop_native_owner_confirmation_required")
        );
        assert_eq!(fs::read_dir(&export_root).unwrap().count(), 0);
        assert!(broker
            .audit_events()
            .iter()
            .any(|event| { event.operation == OPERATION && event.decision == "rejected" }));
        drop(broker);
        fs::remove_dir_all(export_root).unwrap();
    }

    #[test]
    fn 固定Export保存先がない要求はfileを作らず拒否する() {
        let mut broker = Broker::new("session-1");
        let raw = desktop_owner_request(&mut broker, payload());
        let response = broker.desktop_owner_operation_json(&raw);
        assert_eq!(response.status, BrokerStatus::Rejected);
        assert_eq!(response.error.unwrap().code, "gui_shell_export_target_unavailable");
        assert!(broker.audit_events().iter().any(|event| {
            event.operation == OPERATION && event.decision == "rejected"
        }));
    }
}
