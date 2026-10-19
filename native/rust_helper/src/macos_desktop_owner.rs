//! Desktopの限定Owner入口。UIデータを承認へ昇格せず、既存Brokerへ現在要求を渡す。
use crate::broker::ipc_server::DesktopOwnerOperationRequest;
use crate::broker::protocol::{canonical_payload_hash, request_issued_at_is_current};
use crate::broker::{adapter_center, BrokerEndpoint, BrokerRequestEnvelope};
use std::sync::mpsc::{self, SyncSender};
use std::time::Duration;

struct Candidate {
    request: String,
    summary: String,
    response_timeout: Duration,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentTaskWorkspacePermissionRequest {
    agent_runtime_id: String,
    session_id: String,
    workspace_id: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentTaskOwnerApprovalRequest {
    agent_runtime_id: String,
    session_id: String,
    workspace_id: String,
    instruction: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentTaskResultExposureRequest {
    task_id: String,
    result_hash: String,
    content_visibility: String,
}

fn valid_task_scope_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn valid_task_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
fn macos_owner_summary(operation: &str, payload: &serde_json::Value) -> Option<String> {
    crate::broker::mcp_center::desktop_owner_summary(operation, payload)
}

fn candidate(frame: &[u8], endpoint: &BrokerEndpoint) -> Option<Candidate> {
    if frame.len() >= endpoint.max_request_bytes {
        return None;
    }
    let raw = std::str::from_utf8(frame).ok()?;
    let envelope = BrokerRequestEnvelope::from_json_str(raw).ok()?;
    if envelope.session_id.is_some()
        || envelope.request_id.as_deref().is_none_or(str::is_empty)
        || envelope.nonce.as_deref().is_none_or(str::is_empty)
        || !envelope
            .issued_at
            .as_deref()
            .is_some_and(request_issued_at_is_current)
        || envelope.metadata.len() != 1
        || envelope.metadata[0].key != "client"
        || envelope.metadata[0].value != "desktop_flutter"
    {
        return None;
    }
    let hash = canonical_payload_hash(envelope.payload.as_ref());
    if envelope.payload_hash.as_deref() != Some(&hash) {
        return None;
    }
    let operation = envelope.operation?.as_str();
    let payload = envelope.payload.as_ref()?;
    let registration = operation == "AgentCLI実行系作業領域登録";
    let text = if registration {
        let summary = crate::broker::protocol::macos_agent_registration_summary(payload)?;
        format!("{summary}\n要求hash: {hash}")
    } else if operation == "AgentTaskWorkspacePermissionGrant" {
        let request: AgentTaskWorkspacePermissionRequest =
            serde_json::from_value(payload.clone()).ok()?;
        if !valid_task_scope_identifier(&request.agent_runtime_id)
            || !valid_task_scope_identifier(&request.session_id)
            || !valid_task_scope_identifier(&request.workspace_id)
        {
            return None;
        }
        format!(
            "このAgent Sessionに限定した、一回限りのTask用Workspace Permissionを発行しますか？\n\nRuntime ID: {}\nSession ID: {}\nWorkspace ID: {}\n許可operation: agent_task.execute\n\nこの操作はTaskを実行・保存・変更しません。Task本文と実行条件を示す別個のOwner Approvalは必要です。許可はこのSessionとWorkspaceに束縛され、再利用できません。\n\n要求hash: {hash}",
            request.agent_runtime_id, request.session_id, request.workspace_id
        )
    } else if operation == "AgentTaskOwnerApprovalGrant" {
        let request: AgentTaskOwnerApprovalRequest =
            serde_json::from_value(payload.clone()).ok()?;
        if !valid_task_scope_identifier(&request.agent_runtime_id)
            || !valid_task_scope_identifier(&request.session_id)
            || !valid_task_scope_identifier(&request.workspace_id)
            || request.instruction.trim().is_empty()
            || request.instruction.chars().count() > 32_768
        {
            return None;
        }
        let instruction_hash = crate::audit_hash::sha256_tagged(request.instruction.as_bytes());
        format!(
            "このAgent Taskに一回限りのOwner Approvalを発行しますか？\n\nRuntime ID: {}\nSession ID: {}\nWorkspace ID: {}\n指示文字数: {}\n指示hash: {}\n実行上限: 最大15分\n\nCompose画面でTask本文を確認してから判断してください。この確認画面には本文を表示しません。Approvalは本文hashと現在の実行条件に束縛され、Permission・Credential・Trustを付与せず、Taskを実行しません。\n\n要求hash: {hash}",
            request.agent_runtime_id,
            request.session_id,
            request.workspace_id,
            request.instruction.chars().count(),
            instruction_hash
        )
    } else if operation == "AgentTask結果表示承認" {
        let request: AgentTaskResultExposureRequest =
            serde_json::from_value(payload.clone()).ok()?;
        if !valid_task_id(&request.task_id)
            || !valid_sha256(&request.result_hash)
            || !["none", "hash_only", "summary", "redacted", "full"]
                .contains(&request.content_visibility.as_str())
        {
            return None;
        }
        format!(
            "完了したAgent Taskの結果を一回だけ表示するApprovalを発行しますか？\n\nTask ID: {}\n確定結果hash: {}\nContent Exposure: {}\n\n`full`はAgent本文を表示し、秘密情報や危険な指示を含む可能性があります。表示前に範囲を確認してください。このApprovalはTask実行権・Workspace読取許可・Permission・Credential・Authorityを与えず、Auditへ本文を保存しません。\n\n要求hash: {hash}",
            request.task_id, request.result_hash, request.content_visibility
        )
    } else if matches!(
        operation,
        "作業領域承認" | "作業領域失効" | "作業領域全体基準点保存"
    ) {
        let summary = crate::broker::workspace::owner_confirmation_summary(operation, payload)?;
        format!("{summary}\n要求hash: {hash}")
    } else if operation == "GUI Shell書出し" {
        let summary = crate::broker::export_center::owner_confirmation_summary(payload, &hash).ok()?;
        let target = crate::broker::export_center::target_platform_label(&summary.target_platform)?;
        let update_trust = summary.update_trust.as_ref().map_or_else(
            || "未設定（製品Updateはfail-closed）".to_string(),
            |trust| {
                format!(
                    "Ed25519公開鍵fingerprint: {} / HTTPS配布元: {}",
                    trust.public_key_fingerprint,
                    trust.package_source_details.join("; ")
                )
            },
        );
        format!(
            "GUI Shell構成から独立Manifest fileを作成します。\n\nアプリ名: {}\n対象platform: {}\nExport ID: {}\n配布channel: {}\n選択任意Module数: {}\n更新trust: {}\n保存先: 実行中Brokerの固定Export directory（要求pathから変更不可）\n\n実行可能App、Module除去、build、Installer、署名は作成しません。Credential、Permission、Approval、Audit chainは継承しません。\n要求hash: {}",
            summary.display_name,
            target,
            summary.export_id,
            summary.distribution_channel,
            summary.optional_module_count,
            update_trust,
            summary.payload_hash
        )
    } else if operation == "Host登録" {
        let summary = crate::broker::host_center::owner_confirmation_summary(payload)?;
        format!("{summary}\n要求hash: {hash}")
    } else if operation == "A2A接続" {
        let (agent, target) =
            crate::broker::a2a_center::owner_confirmation_summary(payload).ok()?;
        format!("外部Agentの宣言情報だけを取得します。\nAgent ID: {agent}\n接続先: {target}\n要求hash: {hash}\n\nAgent Cardは未信頼metadataです。Trust・Task Permission・Approval・Credentialを生成せず、Task実行を行いません。Brokerが現在の要求と永続Auditを再評価します。")
    } else if matches!(operation, "MCP接続" | "MCP切断" | "MCP Tool実行") {
        let summary = macos_owner_summary(operation, payload)?;
        format!("{summary}\n要求hash: {hash}")
    } else if operation == "資格情報失効" {
        let p = payload.as_object()?;
        let fields = [
            "版",
            "資格情報ID",
            "用途",
            "接続対象",
            "暗号文hash",
            "作成監査ID",
        ];
        let text = |s: &str| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control);
        let id = p.get("資格情報ID")?.as_str()?;
        let ciphertext_hash = p.get("暗号文hash")?.as_str()?;
        if p.len() != fields.len()
            || !fields.iter().all(|f| p.contains_key(*f))
            || p.get("版")? != &serde_json::json!(1)
            || id.len() != 32
            || !id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || ciphertext_hash.len() != 71
            || !ciphertext_hash.starts_with("sha256:")
            || !["用途", "接続対象", "作成監査ID"].iter().all(|f| {
                p.get(*f)
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(text)
            })
        {
            return None;
        }
        format!("資格情報を論理失効します。取消できません。\nID: {id}\n用途: {}\n接続対象: {}\n暗号文hash: {ciphertext_hash}\n作成監査ID: {}\n要求hash: {hash}\nBrokerが現在metadataを再照合します。秘密値は表示・物理削除しません。",
            p["用途"].as_str()?, p["接続対象"].as_str()?, p["作成監査ID"].as_str()?)
    } else if let Some(summary) =
        adapter_center::owner_confirmation_summary(operation.strip_prefix("アダプター")?, payload)
    {
        format!(
            "Adapter {}をこのBrokerのcatalogへ要求します。\nAdapter: {}\n現在Adapter hash: {}\n要求hash: {}\n\nBrokerが現在のrecord、hash、署名と状態条件を再評価します。外部codeの起動・Permission・Approval・Credential・trustの付与は行いません。削除はcatalogのrecordだけが対象です。",
            summary.operation, summary.adapter_id, summary.adapter_hash, hash,
        )
    } else {
        let summary = adapter_center::owner_manifest_confirmation_summary(
            operation.strip_prefix("アダプター")?,
            payload,
        )?;
        format!(
        "Adapter {}をこのBrokerのcatalogへ登録します。\nAdapter: {}\nRuntime: {}\n発行者: {}\n版: {}\n接続: {}\n内容露出: {}\n要求Capability: {}\n許可差分（要求のみ）: {}\n既知の危険: {}\n互換性: {}\n署名者: {}\n署名対象hash: {}\n現在Adapter hash: {}\n要求hash: {}\n\n外部codeの起動・Permission・Approval・Credential・trustの付与は行いません。署名の検証・有効化は別操作です。",
        summary.operation, summary.adapter_id, summary.runtime_id, summary.publisher,
        summary.adapter_version, summary.transport, summary.content_exposure,
        summary.requested_capabilities.join(" / "), summary.permission_diff.join(" / "),
        summary.known_risks.join(" / "), summary.compatibility, summary.signer_fingerprint,
        summary.signed_bytes_hash, summary.current_adapter_hash.as_deref().unwrap_or("新規"), hash,
        )
    };
    let normalized = super::macos_desktop_worker::normalize(frame, &endpoint.session_id);
    Some(Candidate {
        request: String::from_utf8(normalized).ok()?,
        summary: text,
        // 登録は既存5秒probeを2回実行する。OS確認300秒とは別の限定待機。
        response_timeout: Duration::from_secs(if registration {
            15
        } else if matches!(operation, "MCP接続" | "MCP切断" | "MCP Tool実行") {
            35
        } else if operation == "A2A接続" {
            // 既存の接続2秒＋読取5秒とAuditの有限待機。再送しない。
            10
        } else {
            4
        }),
    })
}

/// Noneはnormal IPCへ戻す。戻った要求は既存Brokerが拒否・監査し、承認にならない。
pub(super) fn dispatch(
    frame: &[u8],
    endpoint: &BrokerEndpoint,
    sender: &SyncSender<DesktopOwnerOperationRequest>,
    confirm: &mut impl FnMut(&str) -> bool,
) -> std::io::Result<Option<Vec<u8>>> {
    let Some(candidate) = candidate(frame, endpoint) else {
        return Ok(None);
    };
    if !confirm(&candidate.summary) {
        return Ok(None);
    }
    let (reply, response) = mpsc::sync_channel(1);
    sender
        .try_send(DesktopOwnerOperationRequest {
            request_json: candidate.request,
            download_confirmation: None,
            apply_confirmation: None,
            activation_confirmation: None,
            product_repair_confirmation: None,
            product_uninstall_confirmation: None,
            reply,
        })
        .map_err(|_| std::io::Error::other("Owner操作のBroker配送が不能"))?;
    let response = response
        .recv_timeout(candidate.response_timeout)
        .map_err(|_| std::io::Error::other("Owner操作の応答が未確定。再送しない"))?;
    Ok(Some(
        response
            .to_json_string()
            .map_err(std::io::Error::other)?
            .into_bytes(),
    ))
}

#[cfg(test)]
mod workspace_inspector_tests {
    use super::*;
    use crate::broker::ipc_server::BrokerCredentialRole;
    use crate::broker::protocol::{BrokerResponse, BrokerStatus};
    use serde_json::json;

    #[test]
    fn linux_and_macos_export_owner_confirmation_binds_linux_target_and_payload() {
        let endpoint = BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 1,
            session_id: "fixture-session".into(),
            session_secret: "synthetic-only".into(),
            credential_role: BrokerCredentialRole::Normal,
            transport: "tcp".into(),
            max_request_bytes: 65536,
        };
        let payload = json!({
            "version": 1,
            "export_id": "linux-export-fixture",
            "compose_manifest": {
                "version": 1,
                "compose_id": "linux-fixture",
                "display_name": "Linux fixture",
                "runtime_ids": ["gui_shell_rust_broker"],
                "agent_ids": [],
                "tool_ids": [],
                "mcp_connection_ids": [],
                "theme": {"theme_id": "d4-pocket", "mode": "system"},
                "capability_requirements": ["runtime.read"],
                "settings": {"locale": "ja-JP", "density": "comfortable", "content_visibility": "summary"},
                "inheritance_policy": {"authority": "none", "permission": "none", "approval": "none", "credential": "none", "audit_chain": "none"},
                "output_mode": "manifest_only"
            },
            "target_platform": "linux",
            "export_mode": "manifest_file",
            "distribution_channel": "local"
        });
        let request = json!({
            "request_id": "linux-export-request",
            "nonce": "linux-export-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "operation": "GUI Shell書出し",
            "metadata": {"client": "desktop_flutter"},
            "payload_hash": canonical_payload_hash(Some(&payload)),
            "payload": payload
        });
        let bytes = serde_json::to_vec(&request).unwrap();
        let candidate_value =
            candidate(&bytes, &endpoint).expect("Linux export is an Owner operation");
        assert!(candidate_value.summary.contains("対象platform: Linux"));
        assert!(candidate_value.summary.contains("linux-export-fixture"));
        assert!(candidate_value
            .summary
            .contains(request["payload_hash"].as_str().unwrap()));
        assert!(candidate_value.summary.contains("固定Export directory"));
        assert!(candidate_value
            .summary
            .contains("Permission、Approval、Audit chainは継承しません"));

        let (sender, receiver) = mpsc::sync_channel(1);
        assert!(dispatch(&bytes, &endpoint, &sender, &mut |_| false)
            .unwrap()
            .is_none());
        assert!(receiver.try_recv().is_err());

        let (sender, receiver) = mpsc::sync_channel::<DesktopOwnerOperationRequest>(1);
        let worker = std::thread::spawn(move || {
            let owner_request: DesktopOwnerOperationRequest =
                receiver.recv_timeout(Duration::from_secs(2)).unwrap();
            let normalized: serde_json::Value =
                serde_json::from_str(&owner_request.request_json).unwrap();
            assert_eq!(normalized["operation"], "GUI Shell書出し");
            assert_eq!(normalized["payload"]["target_platform"], "linux");
            owner_request
                .reply
                .send(BrokerResponse {
                    request_id: "linux-export-request".into(),
                    operation: "GUI Shell書出し".into(),
                    status: BrokerStatus::Accepted,
                    evidence_source: "INTERNAL_STATE".into(),
                    audit_event_id: "fixture-audit".into(),
                    error: None,
                    health: None,
                    body: Some(json!({"manifest_file_status":"written"})),
                    shutdown_requested: false,
                })
                .unwrap();
        });
        let response = dispatch(&bytes, &endpoint, &sender, &mut |summary| {
            summary.contains("対象platform: Linux")
        })
        .unwrap()
        .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&response).unwrap()["status"],
            "accepted"
        );
        worker.join().unwrap();

        let mut modified = request.clone();
        modified["payload"]["target_platform"] = json!("android");
        modified["payload_hash"] = json!(canonical_payload_hash(modified.get("payload")));
        assert!(candidate(&serde_json::to_vec(&modified).unwrap(), &endpoint).is_none());
        modified = request;
        modified["payload"]["target_platform"] = json!("windows");
        assert!(candidate(&serde_json::to_vec(&modified).unwrap(), &endpoint).is_none());
    }

    #[test]
    fn macos_workspace内容露出_同一要求だけを確認し未承認と権限注入を拒否する() {
        let endpoint = BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 1,
            session_id: "fixture-session".into(),
            session_secret: "synthetic-only".into(),
            credential_role: BrokerCredentialRole::Normal,
            transport: "tcp".into(),
            max_request_bytes: 65536,
        };
        for operation in ["作業領域承認", "作業領域失効", "作業領域全体基準点保存"]
        {
            let mut payload = json!({"作業領域ID":"macos-product-workspace", "登録hash":format!("sha256:{}", "a".repeat(64))});
            if operation == "作業領域承認" {
                payload["表示範囲"] = json!("full");
            }
            let valid = json!({"request_id":"workspace-request", "nonce":"workspace-nonce",
                "issued_at":BrokerRequestEnvelope::current_issued_at(), "operation":operation,
                "metadata":{"client":"desktop_flutter"}, "payload_hash":canonical_payload_hash(Some(&payload)), "payload":payload});
            let bytes = serde_json::to_vec(&valid).unwrap();
            let current = candidate(&bytes, &endpoint).unwrap();
            assert!(current.summary.contains("macos-product-workspace"));
            assert!(current
                .summary
                .contains(valid["payload_hash"].as_str().unwrap()));
            assert!(current.summary.contains("Task実行・書込権は付与しません"));
            let (sender, receiver) = mpsc::sync_channel(1);
            assert!(dispatch(&bytes, &endpoint, &sender, &mut |_| false)
                .unwrap()
                .is_none());
            assert!(receiver.try_recv().is_err());
            for field in ["session_id", "approval"] {
                let mut invalid = valid.clone();
                invalid[field] = json!(true);
                assert!(candidate(&serde_json::to_vec(&invalid).unwrap(), &endpoint).is_none());
            }
            let mut authority = valid.clone();
            authority["metadata"]["authority"] = json!("owner");
            assert!(candidate(&serde_json::to_vec(&authority).unwrap(), &endpoint).is_none());
            let mut changed = valid.clone();
            changed["payload"]["作業領域ID"] = json!("another-workspace");
            assert!(candidate(&serde_json::to_vec(&changed).unwrap(), &endpoint).is_none());
            for (field, value) in [
                ("root", json!("/outside")),
                ("登録hash", json!("sha256:invalid")),
                ("表示範囲", json!("trusted")),
            ] {
                let mut invalid = valid.clone();
                invalid["payload"][field] = value;
                invalid["payload_hash"] = json!(canonical_payload_hash(invalid.get("payload")));
                assert!(candidate(&serde_json::to_vec(&invalid).unwrap(), &endpoint).is_none());
            }
            let original = payload.clone();
            let worker = std::thread::spawn(move || {
                let request = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
                let normalized: serde_json::Value =
                    serde_json::from_str(&request.request_json).unwrap();
                assert_eq!(normalized["payload"], original);
                assert_eq!(normalized["session_id"], "fixture-session");
                request
                    .reply
                    .send(BrokerResponse {
                        request_id: "workspace-request".into(),
                        operation: operation.into(),
                        status: BrokerStatus::Rejected,
                        evidence_source: "INTERNAL_STATE".into(),
                        audit_event_id: "fixture-audit".into(),
                        error: None,
                        health: None,
                        body: None,
                        shutdown_requested: false,
                    })
                    .unwrap();
            });
            let response = dispatch(&bytes, &endpoint, &sender, &mut |_| true)
                .unwrap()
                .unwrap();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&response).unwrap()["status"],
                "rejected"
            );
            worker.join().unwrap();
        }
    }
}

#[cfg(test)]
mod host_tests {
    use super::*;
    use crate::broker::ipc_server::BrokerCredentialRole;
    use crate::broker::protocol::{BrokerResponse, BrokerStatus};
    use serde_json::json;

    #[test]
    fn macos_host_同一要求だけを確認し未承認と権限注入を拒否する() {
        let endpoint = BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 1,
            session_id: "fixture-session".into(),
            session_secret: "synthetic-only".into(),
            credential_role: BrokerCredentialRole::Normal,
            transport: "tcp".into(),
            max_request_bytes: 65536,
        };
        let payload = json!({"版":1,"操作":"登録","Host ID":"mac-host-fixture",
            "表示名":"Mac公開Host","Platform":"macos","接続状態":"pending_review","Trust":"pending_review",
            "証明書/identity":{"種別":"identity_hash","hash":format!("sha256:{}", "a".repeat(64))},
            "Runtime summary":{"runtime_count":2,"agent_count":1,"evidence_source":"INTERNAL_STATE"},"最終接続":null});
        let valid = json!({"request_id":"host-request","nonce":"host-nonce",
            "issued_at":BrokerRequestEnvelope::current_issued_at(),"operation":"Host登録",
            "metadata":{"client":"desktop_flutter"},"payload_hash":canonical_payload_hash(Some(&payload)),"payload":payload});
        let bytes = serde_json::to_vec(&valid).unwrap();
        let current = candidate(&bytes, &endpoint).unwrap();
        assert!(current.summary.contains("mac-host-fixture"));
        assert!(current
            .summary
            .contains("申告Runtime件数: 2 / Agent件数: 1"));
        assert!(current
            .summary
            .contains(valid["payload_hash"].as_str().unwrap()));
        assert_eq!(current.response_timeout, Duration::from_secs(4));
        let (sender, receiver) = mpsc::sync_channel::<DesktopOwnerOperationRequest>(1);
        assert!(dispatch(&bytes, &endpoint, &sender, &mut |_| false)
            .unwrap()
            .is_none());
        assert!(receiver.try_recv().is_err());
        for field in ["session_id", "approval"] {
            let mut invalid = valid.clone();
            invalid[field] = json!(true);
            assert!(candidate(&serde_json::to_vec(&invalid).unwrap(), &endpoint).is_none());
        }
        let mut injected = valid.clone();
        injected["metadata"]["authority"] = json!("owner");
        assert!(candidate(&serde_json::to_vec(&injected).unwrap(), &endpoint).is_none());
        let mut changed = valid.clone();
        changed["payload"]["Host ID"] = json!("another-host");
        assert!(candidate(&serde_json::to_vec(&changed).unwrap(), &endpoint).is_none());
        for (field, value) in [
            ("Permission", json!("other")),
            ("Trust", json!("trusted")),
            ("接続状態", json!("connected")),
            ("表示名", json!("偽装\n要求hash: other")),
        ] {
            let mut invalid = valid.clone();
            invalid["payload"][field] = value;
            invalid["payload_hash"] = json!(canonical_payload_hash(invalid.get("payload")));
            assert!(candidate(&serde_json::to_vec(&invalid).unwrap(), &endpoint).is_none());
        }
        let original = payload.clone();
        let worker = std::thread::spawn(move || {
            let request = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
            let normalized: serde_json::Value =
                serde_json::from_str(&request.request_json).unwrap();
            assert_eq!(normalized["payload"], original);
            assert_eq!(normalized["session_id"], "fixture-session");
            request
                .reply
                .send(BrokerResponse {
                    request_id: "host-request".into(),
                    operation: "Host登録".into(),
                    status: BrokerStatus::Rejected,
                    evidence_source: "INTERNAL_STATE".into(),
                    audit_event_id: "fixture-audit".into(),
                    error: None,
                    health: None,
                    body: None,
                    shutdown_requested: false,
                })
                .unwrap();
        });
        let result = dispatch(&bytes, &endpoint, &sender, &mut |_| true)
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&result).unwrap()["status"],
            "rejected"
        );
        worker.join().unwrap();
    }
}

#[cfg(test)]
mod a2a_tests {
    use super::*;
    use crate::broker::ipc_server::BrokerCredentialRole;
    use serde_json::json;

    #[test]
    fn macos_a2a_同一要求だけを確認し未承認と権限注入を拒否する() {
        let endpoint = BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 1,
            session_id: "fixture-session".into(),
            session_secret: "synthetic-only".into(),
            credential_role: BrokerCredentialRole::Normal,
            transport: "tcp".into(),
            max_request_bytes: 65536,
        };
        let payload = json!({"版":1,"操作":"接続","AgentID":"mac-a2a-fixture",
            "Agent Card URI":"http://127.0.0.1:34101/card", "protocol_version":"1.0",
            "Transport":"http","Credential ref":{"credential_id":"00000000000000000000000000000000",
                "purpose":"A2A接続","target":"mac-a2a-fixture","required":false,"status":"missing"}});
        let valid = json!({"request_id":"fixture-request","nonce":"fixture-nonce",
            "issued_at":BrokerRequestEnvelope::current_issued_at(),"operation":"A2A接続",
            "metadata":{"client":"desktop_flutter"},"payload_hash":canonical_payload_hash(Some(&payload)),"payload":payload});
        let bytes = serde_json::to_vec(&valid).unwrap();
        let current = candidate(&bytes, &endpoint).unwrap();
        assert!(current.summary.contains("mac-a2a-fixture"));
        assert!(current.summary.contains("http://127.0.0.1:34101/card"));
        assert!(current
            .summary
            .contains(valid["payload_hash"].as_str().unwrap()));
        assert!(current.summary.contains("未信頼metadata"));
        assert_eq!(current.response_timeout, Duration::from_secs(10));
        let normalized: serde_json::Value = serde_json::from_str(&current.request).unwrap();
        assert_eq!(normalized["payload"], valid["payload"]);
        assert_eq!(normalized["session_id"], "fixture-session");
        for key in ["session_id", "approval"] {
            let mut invalid = valid.clone();
            invalid[key] = json!(true);
            assert!(candidate(&serde_json::to_vec(&invalid).unwrap(), &endpoint).is_none());
        }
        let mut invalid = valid.clone();
        invalid["metadata"]["authority"] = json!("owner");
        assert!(candidate(&serde_json::to_vec(&invalid).unwrap(), &endpoint).is_none());
        let mut invalid = valid.clone();
        invalid["payload"]["AgentID"] = json!("other");
        assert!(candidate(&serde_json::to_vec(&invalid).unwrap(), &endpoint).is_none());
        for field in ["Approval", "secret"] {
            let mut invalid = valid.clone();
            invalid["payload"][field] = json!("injected");
            invalid["payload_hash"] = json!(canonical_payload_hash(Some(&invalid["payload"])));
            assert!(candidate(&serde_json::to_vec(&invalid).unwrap(), &endpoint).is_none());
        }
        let mut invalid = valid.clone();
        invalid["payload"]["Agent Card URI"] = json!("http://192.0.2.1/card");
        invalid["payload_hash"] = json!(canonical_payload_hash(Some(&invalid["payload"])));
        assert!(candidate(&serde_json::to_vec(&invalid).unwrap(), &endpoint).is_none());
        let (sender, receiver) = mpsc::sync_channel(1);
        assert!(dispatch(&bytes, &endpoint, &sender, &mut |_| false)
            .unwrap()
            .is_none());
        assert!(receiver.try_recv().is_err());
        // 実確認後に同じpayloadを既存receiverへ一回だけ配送する。
        let worker = std::thread::spawn(move || {
            let received: DesktopOwnerOperationRequest = receiver.recv().unwrap();
            let request: serde_json::Value = serde_json::from_str(&received.request_json).unwrap();
            assert_eq!(request["payload"], valid["payload"]);
            received
                .reply
                .send(crate::broker::protocol::BrokerResponse {
                    request_id: "fixture-request".into(),
                    operation: "A2A接続".into(),
                    status: crate::broker::protocol::BrokerStatus::Accepted,
                    evidence_source: "FIXTURE".into(),
                    audit_event_id: "fixture-audit".into(),
                    error: None,
                    health: None,
                    body: None,
                    shutdown_requested: false,
                })
                .unwrap();
        });
        assert!(dispatch(&bytes, &endpoint, &sender, &mut |_| true)
            .unwrap()
            .is_some());
        worker.join().unwrap();
    }
}

#[cfg(test)]
mod mcp_tests {
    use super::*;
    use crate::broker::ipc_server::BrokerCredentialRole;
    use serde_json::{json, Value};

    #[test]
    fn macos_mcp_candidate_現在hashへ束縛し_ui承認とsession注入を拒否する() {
        let endpoint = BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 1,
            session_id: "fixture-session".into(),
            session_secret: "synthetic-only".into(),
            credential_role: BrokerCredentialRole::Normal,
            transport: "tcp".into(),
            max_request_bytes: 65536,
        };
        let payload = json!({"版":1,"操作":"切断","ServerID":"fixture-mcp"});
        let frame = |payload: Value| {
            json!({"request_id":"fixture-request","nonce":"fixture-nonce",
            "issued_at":BrokerRequestEnvelope::current_issued_at(),"operation":"MCP切断",
            "metadata":{"client":"desktop_flutter"},"payload_hash":canonical_payload_hash(Some(&payload)),"payload":payload})
        };
        let valid = frame(payload.clone());
        let candidate_value = candidate(&serde_json::to_vec(&valid).unwrap(), &endpoint).unwrap();
        assert!(candidate_value.summary.contains("fixture-mcp"));
        assert!(candidate_value
            .summary
            .contains(valid["payload_hash"].as_str().unwrap()));
        assert_eq!(candidate_value.response_timeout, Duration::from_secs(35));
        for key in ["session_id", "approval"] {
            let mut invalid = valid.clone();
            invalid[key] = json!(true);
            assert!(candidate(&serde_json::to_vec(&invalid).unwrap(), &endpoint).is_none());
        }
        let mut metadata = valid.clone();
        metadata["metadata"]["authority"] = json!("owner");
        assert!(candidate(&serde_json::to_vec(&metadata).unwrap(), &endpoint).is_none());
        let mut mismatch = valid.clone();
        mismatch["payload"]["ServerID"] = json!("other");
        assert!(candidate(&serde_json::to_vec(&mismatch).unwrap(), &endpoint).is_none());
        let (sender, receiver) = mpsc::sync_channel(1);
        assert!(dispatch(
            &serde_json::to_vec(&valid).unwrap(),
            &endpoint,
            &sender,
            &mut |_| false
        )
        .unwrap()
        .is_none());
        assert!(receiver.try_recv().is_err());
    }
}

#[cfg(test)]
mod task_owner_tests {
    use super::*;
    use crate::broker::ipc_server::BrokerCredentialRole;
    use crate::broker::protocol::{BrokerResponse, BrokerStatus};
    use serde_json::{json, Value};

    fn endpoint() -> BrokerEndpoint {
        BrokerEndpoint {
            host: "127.0.0.1".into(),
            port: 1,
            session_id: "fixture-session".into(),
            session_secret: "synthetic-only".into(),
            credential_role: BrokerCredentialRole::Normal,
            transport: "tcp".into(),
            max_request_bytes: 65536,
        }
    }

    fn request(operation: &str, payload: Value) -> Value {
        json!({
            "request_id": "fixture-task-owner-request",
            "nonce": "fixture-task-owner-nonce",
            "issued_at": BrokerRequestEnvelope::current_issued_at(),
            "operation": operation,
            "metadata": {"client": "desktop_flutter"},
            "payload_hash": canonical_payload_hash(Some(&payload)),
            "payload": payload
        })
    }

    #[test]
    fn task_owner_confirmations_bind_payload_and_never_show_task_body() {
        let instruction = "PRIVATE_TASK_INSTRUCTION_MARKER";
        let payload = json!({
            "agent_runtime_id": "codex-runtime",
            "session_id": "session-0123456789abcdef",
            "workspace_id": "workspace-project",
            "instruction": instruction
        });
        let valid = request("AgentTaskOwnerApprovalGrant", payload.clone());
        let bytes = serde_json::to_vec(&valid).unwrap();
        let summary = candidate(&bytes, &endpoint()).unwrap().summary;
        assert!(summary.contains("codex-runtime"));
        assert!(summary.contains("workspace-project"));
        assert!(summary.contains(&crate::audit_hash::sha256_tagged(instruction.as_bytes())));
        assert!(summary.contains(valid["payload_hash"].as_str().unwrap()));
        assert!(!summary.contains(instruction));

        let mut extra = payload;
        extra["authority"] = json!("owner");
        let invalid = request("AgentTaskOwnerApprovalGrant", extra);
        assert!(candidate(&serde_json::to_vec(&invalid).unwrap(), &endpoint()).is_none());

        let mut authority_metadata = valid.clone();
        authority_metadata["metadata"]["authority"] = json!("owner");
        assert!(candidate(
            &serde_json::to_vec(&authority_metadata).unwrap(),
            &endpoint()
        )
        .is_none());

        let mut caller_session = valid.clone();
        caller_session["session_id"] = json!("attacker-session");
        assert!(candidate(&serde_json::to_vec(&caller_session).unwrap(), &endpoint()).is_none());

        let (sender, receiver) = mpsc::sync_channel(1);
        assert!(dispatch(&bytes, &endpoint(), &sender, &mut |_| false)
            .unwrap()
            .is_none());
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn task_permission_and_result_exposure_confirmations_are_scoped_and_bounded() {
        let permission_payload = json!({
            "agent_runtime_id": "codex-runtime",
            "session_id": "session-0123456789abcdef",
            "workspace_id": "workspace-project"
        });
        let permission = request("AgentTaskWorkspacePermissionGrant", permission_payload);
        let permission_candidate =
            candidate(&serde_json::to_vec(&permission).unwrap(), &endpoint()).unwrap();
        assert!(permission_candidate.summary.contains("agent_task.execute"));
        assert!(permission_candidate.summary.contains("workspace-project"));
        assert!(permission_candidate
            .summary
            .contains(permission["payload_hash"].as_str().unwrap()));

        let result_payload = json!({
            "task_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "result_hash": format!("sha256:{}", "b".repeat(64)),
            "content_visibility": "full"
        });
        let result = request("AgentTask結果表示承認", result_payload.clone());
        let result_candidate =
            candidate(&serde_json::to_vec(&result).unwrap(), &endpoint()).unwrap();
        assert!(result_candidate.summary.contains("Content Exposure: full"));
        assert!(result_candidate
            .summary
            .contains(result_payload["result_hash"].as_str().unwrap()));
        assert!(result_candidate.summary.contains("秘密情報"));

        let mut invalid_result = result_payload;
        invalid_result["content_visibility"] = json!("trusted");
        let invalid = request("AgentTask結果表示承認", invalid_result);
        assert!(candidate(&serde_json::to_vec(&invalid).unwrap(), &endpoint()).is_none());
    }

    #[test]
    fn confirmed_task_approval_is_forwarded_once_to_internal_broker_queue() {
        let payload = json!({
            "agent_runtime_id": "codex-runtime",
            "session_id": "session-0123456789abcdef",
            "workspace_id": "workspace-project",
            "instruction": "Review this bounded test task"
        });
        let valid = request("AgentTaskOwnerApprovalGrant", payload.clone());
        let bytes = serde_json::to_vec(&valid).unwrap();
        let (sender, receiver) = mpsc::sync_channel::<DesktopOwnerOperationRequest>(1);
        let worker = std::thread::spawn(move || {
            let request = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
            let parsed: Value = serde_json::from_str(&request.request_json).unwrap();
            assert_eq!(parsed["payload"], payload);
            assert_eq!(parsed["session_id"], "fixture-session");
            request
                .reply
                .send(BrokerResponse {
                    request_id: "fixture-task-owner-request".into(),
                    operation: "AgentTaskOwnerApprovalGrant".into(),
                    status: BrokerStatus::Accepted,
                    evidence_source: "FIXTURE".into(),
                    audit_event_id: "fixture-audit".into(),
                    error: None,
                    health: None,
                    body: None,
                    shutdown_requested: false,
                })
                .unwrap();
            assert!(receiver.try_recv().is_err());
        });
        let response = dispatch(&bytes, &endpoint(), &sender, &mut |_| true)
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&response).unwrap()["status"],
            "accepted"
        );
        worker.join().unwrap();
    }
}
