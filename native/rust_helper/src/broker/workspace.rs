//! 作業領域の現在Permissionと読取。rootの選択やowner認証は所有しない。
#![allow(non_snake_case)]

use super::dialogue::識別子生成;
use crate::{
    audit_hash::sha256_tagged,
    workspace_reader::{EntryKind, WorkspaceReader},
};
use cap_std::fs::Dir;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use super::ai_edit_center::{
    ApplyRequest, ApplyTargetRequest, ReviewRequest, RollbackCandidatesRequest, RollbackRequest,
    APPLY_TARGET_OPERATION, REVIEW_OPERATION, ROLLBACK_CANDIDATES_OPERATION, ROLLBACK_OPERATION,
};
use super::audit::BrokerAuditLog;

type Audit<'a> = dyn FnMut(&str, &str) -> Result<(), &'static str> + 'a;
const ROLLBACK_CANDIDATE_PAGE_SIZE: usize = 4;

#[derive(Default)]
pub(crate) struct WorkspaceRegistry {
    entries: BTreeMap<String, RegisteredWorkspace>,
    last_now: Option<i64>,
}

#[cfg(test)]
#[path = "../../tests/unit/workspace_registry.rs"]
mod tests;

impl std::fmt::Debug for WorkspaceRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceRegistry")
            .field("登録数", &self.entries.len())
            .finish()
    }
}

struct RegisteredWorkspace {
    runtime: String,
    root_device: u64,
    root_file_id: u64,
    root_ancestry: Vec<super::workspace_root::DirectoryIdentity>,
    root_ancestry_complete: bool,
    registration_hash: String,
    recovery_binding_hash: String,
    reader: WorkspaceReader,
    grant: Option<Grant>,
    baseline: Option<Baseline>,
}

impl RegisteredWorkspace {
    fn compare_whole(
        &self,
        baseline: &Baseline,
        grant: &Grant,
        mut changed: impl FnMut(
            String,
            &'static str,
            Option<&crate::workspace_reader::ComparedFile>,
            Option<&crate::workspace_reader::ComparedFile>,
        ) -> Result<(), &'static str>,
    ) -> Result<(usize, usize), &'static str> {
        let inventory = self
            .reader
            .comparison_inventory()
            .map_err(|_| "変更探索が不成立")?;
        let mut paths: std::collections::BTreeSet<_> = inventory.files.iter().cloned().collect();
        paths.extend(baseline.files.keys().cloned());
        let mut remaining = crate::workspace_reader::MAX_COMPARISON_BYTES;
        let mut unchanged = 0;
        for path in paths {
            if Instant::now() >= grant.deadline {
                return Err("変更探索中に承認期限超過");
            }
            let after = self
                .reader
                .read_comparison_version(&path, remaining)
                .map_err(|_| "変更対象の取得拒否")?;
            remaining -= after.as_ref().map(|v| v.bytes as u64).unwrap_or(0);
            let before = baseline.files.get(&path).and_then(|v| v.as_ref());
            let state = match (before, after.as_ref()) {
                (None, None) => return Err("探索中に新規fileが消失"),
                (None, Some(_)) => Some("added"),
                (Some(_), None) => Some("deleted"),
                (Some(a), Some(b)) if a.bytes != b.bytes || a.sha256 != b.sha256 => {
                    Some("modified")
                }
                _ => None,
            };
            if let Some(status) = state {
                changed(path, status, before, after.as_ref())?;
            } else {
                unchanged += 1;
            }
        }
        if self
            .reader
            .comparison_inventory()
            .map_err(|_| "変更取得後の探索が不成立")?
            != inventory
        {
            return Err("変更取得中に探索範囲が変化");
        }
        Ok((unchanged, inventory.excluded_secrets))
    }
}

/// Broker内で照合済みのSession-to-Workspace参照。
/// Permissionやfilesystem accessを保持せず、登録関係の監査相関にだけ使う。
pub(crate) struct DialogueWorkspaceBinding {
    runtime: String,
    workspace_id: String,
    registration_hash: String,
    recovery_binding_hash: String,
    root_identity: super::workspace_root::DirectoryIdentity,
    secret_paths: Vec<String>,
}

impl DialogueWorkspaceBinding {
    pub(crate) fn matches(&self, runtime: &str, workspace_id: &str) -> bool {
        self.runtime == runtime && self.workspace_id == workspace_id
    }

    pub(crate) fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub(crate) fn registration_hash(&self) -> &str {
        &self.registration_hash
    }

    pub(crate) fn recovery_binding_hash(&self) -> &str {
        &self.recovery_binding_hash
    }

    pub(crate) fn root_identity(&self) -> super::dialogue::AgentTaskWorkspaceIdentity {
        super::dialogue::AgentTaskWorkspaceIdentity::from_directory_identity(self.root_identity)
    }

    pub(crate) fn root_directory_identity(&self) -> super::workspace_root::DirectoryIdentity {
        self.root_identity
    }

    pub(crate) fn secret_paths(&self) -> &[String] {
        &self.secret_paths
    }

    pub(crate) fn audit_hash(&self, session_id: &str) -> String {
        digest(&json!({
            "対話セッションID": session_id,
            "実行系ID": self.runtime,
            "作業領域ID": self.workspace_id,
            "登録hash": self.registration_hash,
        }))
    }

    #[cfg(test)]
    pub(crate) fn for_test(runtime: &str, workspace_id: &str) -> Self {
        Self::for_test_with_registration_hash(runtime, workspace_id, "sha256:fixture-registration")
    }

    #[cfg(test)]
    pub(crate) fn for_test_with_registration_hash(
        runtime: &str,
        workspace_id: &str,
        registration_hash: &str,
    ) -> Self {
        Self::for_test_with_root_identity(
            runtime,
            workspace_id,
            registration_hash,
            super::workspace_root::DirectoryIdentity {
                device: 1,
                file_id: 1,
            },
        )
    }

    #[cfg(test)]
    pub(crate) fn for_test_with_root_identity(
        runtime: &str,
        workspace_id: &str,
        registration_hash: &str,
        root_identity: super::workspace_root::DirectoryIdentity,
    ) -> Self {
        Self {
            runtime: runtime.to_owned(),
            workspace_id: workspace_id.to_owned(),
            registration_hash: registration_hash.to_owned(),
            recovery_binding_hash: format!("sha256:{}", "b".repeat(64)),
            root_identity,
            secret_paths: Vec::new(),
        }
    }
}
struct Baseline {
    whole: bool,
    hash: String,
    files: BTreeMap<String, Option<crate::workspace_reader::ComparedFile>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Capture {
    作業領域ID: String,
    登録hash: String,
    相対paths: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiffSelection {
    作業領域ID: String,
    相対path: String,
    基準点hash: String,
}
struct Grant {
    id: String,
    visibility: String,
    expires: i64,
    deadline: Instant,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    作業領域ID: String,
    相対path: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Approval {
    作業領域ID: String,
    登録hash: String,
    表示範囲: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Revocation {
    作業領域ID: String,
    登録hash: String,
}

fn parse<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, &'static str> {
    serde_json::from_value(value.clone()).map_err(|_| "作業領域要求の構造が不正")
}
pub(super) fn identifier(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
}
fn digest(value: &Value) -> String {
    sha256_tagged(value.to_string().as_bytes())
}

/// 現在要求の公開参照だけをDesktop native確認へ射影する。登録・Permissionの採否はBrokerが所有する。
#[cfg(any(test, target_os = "linux", target_os = "macos"))]
pub(crate) fn owner_confirmation_summary(operation: &str, payload: &Value) -> Option<String> {
    let (id, hash, detail) = match operation {
        "作業領域承認" => {
            let p: Approval = parse(payload).ok()?;
            if !["none", "hash_only", "summary", "redacted", "full"].contains(&p.表示範囲.as_str())
            {
                return None;
            }
            let detail = format!("現在登録に限定した5分の読取許可を要求します。\n内容露出: {}\nfullは登録済みsecret除外以外の本文を画面へ返せます。一般的な秘密検出ではありません。", p.表示範囲);
            (p.作業領域ID, p.登録hash, detail)
        }
        "作業領域失効" | "作業領域全体基準点保存" => {
            let p: Revocation = parse(payload).ok()?;
            let detail = if operation == "作業領域失効" {
                "現在の読取許可とBroker内baselineを失効します。Workspace fileは変更・削除しません。"
            } else {
                "現在のfull読取許可で取得できる範囲だけを、上限付きのBroker内baselineへ保存します。既存baselineを置換します。secret除外を守り、Task固有成果やRecovery適用にはしません。"
            };
            (p.作業領域ID, p.登録hash, detail.to_owned())
        }
        _ => return None,
    };
    if !identifier(&id)
        || hash.len() != 71
        || !hash.starts_with("sha256:")
        || !hash[7..]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    Some(format!("Workspace操作: {operation}\nWorkspace ID: {id}\n登録hash: {hash}\n{detail}\n\nBrokerが現在登録・期限・登録handle・secret境界・Auditを再評価します。Task実行・書込権は付与しません。Credential・Agent trust・OSアクセス範囲を生成しません。"))
}

impl WorkspaceRegistry {
    /// 指定Runtimeに属する現在登録だけから、内容・root path・Permissionを含まない結合参照を作る。
    pub(crate) fn dialogue_binding(
        &self,
        runtime: &str,
        workspace_id: &str,
    ) -> Option<DialogueWorkspaceBinding> {
        let entry = self.entries.get(workspace_id)?;
        (entry.runtime == runtime).then(|| DialogueWorkspaceBinding {
            runtime: runtime.to_owned(),
            workspace_id: workspace_id.to_owned(),
            registration_hash: entry.registration_hash.clone(),
            recovery_binding_hash: entry.recovery_binding_hash.clone(),
            root_identity: super::workspace_root::DirectoryIdentity {
                device: entry.root_device,
                file_id: entry.root_file_id,
            },
            secret_paths: entry.reader.registered_secret_paths().to_vec(),
        })
    }

    /// terminal lifecycle隔離後に、同じ実行系へ結合した作業領域root、承認、基準点を残さない。
    pub(crate) fn remove_runtime(&mut self, runtime: &str) {
        self.entries.retain(|_, entry| entry.runtime != runtime);
    }

    pub(crate) fn response_current(&self, body: &Value, now: i64) -> bool {
        if self.last_now.is_some_and(|last| now < last) {
            return false;
        }
        let current = |workspace: &Value, approval: &Value| {
            workspace
                .as_str()
                .and_then(|id| self.entries.get(id))
                .and_then(|entry| entry.grant.as_ref())
                .is_some_and(|grant| {
                    now < grant.expires
                        && Instant::now() < grant.deadline
                        && approval.as_str() == Some(grant.id.as_str())
                })
        };
        current(&body["作業領域ID"], &body["approval_id"])
            && (body["operation"] != APPLY_TARGET_OPERATION
                || current(
                    &body["projection"]["source_workspace_id"],
                    &body["projection"]["source_approval_id"],
                ))
    }
    /// 信頼済み起動制御面が渡すhandleを保持する。登録だけでは読取許可を作らない。
    pub(crate) fn register(
        &mut self,
        runtime: &str,
        id: &str,
        root: Dir,
        secrets: &[String],
        ancestry: Option<Vec<super::workspace_root::DirectoryIdentity>>,
        audit: &mut Audit<'_>,
    ) -> Result<(), &'static str> {
        if !identifier(id)
            || !identifier(runtime)
            || self.entries.contains_key(id)
            || self.entries.len() >= 16
        {
            return Err("作業領域の登録指定が不正または上限超過");
        }
        let identity = root
            .dir_metadata()
            .map_err(|_| "作業領域のroot識別子を確認できない")?;
        let root_device = cap_fs_ext::MetadataExt::dev(&identity);
        let root_file_id = cap_fs_ext::MetadataExt::ino(&identity);
        if root_file_id == 0 {
            return Err("root file IDが未観測のため作業領域登録を拒否");
        }
        let root_identity = super::workspace_root::DirectoryIdentity {
            device: root_device,
            file_id: root_file_id,
        };
        let (root_ancestry, root_ancestry_complete) = match ancestry {
            Some(ancestry)
                if !ancestry.is_empty()
                    && ancestry.len() >= 2
                    && ancestry.len() <= 65
                    && ancestry.last() == Some(&root_identity)
                    && ancestry.iter().all(|identity| identity.file_id != 0) =>
            {
                (ancestry, true)
            }
            Some(_) => return Err("作業領域の親子識別範囲が不正または未観測"),
            None => (vec![root_identity], false),
        };
        let other_runtimes: Vec<&RegisteredWorkspace> = self
            .entries
            .values()
            .filter(|entry| entry.runtime != runtime)
            .collect();
        if other_runtimes
            .iter()
            .any(|entry| entry.root_device == root_device && entry.root_file_id == root_file_id)
        {
            return Err("別Agentとの同一物理作業領域共有を拒否");
        }
        if other_runtimes
            .iter()
            .any(|entry| !entry.root_ancestry_complete)
            || (!other_runtimes.is_empty() && !root_ancestry_complete)
        {
            return Err("別AgentとのWorkspace分離範囲を確認できないため登録を拒否");
        }
        if other_runtimes.iter().any(|entry| {
            let existing_root = super::workspace_root::DirectoryIdentity {
                device: entry.root_device,
                file_id: entry.root_file_id,
            };
            entry.root_ancestry.contains(&root_identity) || root_ancestry.contains(&existing_root)
        }) {
            return Err("別Agentとの親子Workspace範囲重複を拒否");
        }
        let reader = WorkspaceReader::from_registered_dir(root, secrets)
            .map_err(|_| "作業領域の除外指定が不正")?;
        let recovery_binding_hash = digest(
            &json!({"実行系ID":runtime,"作業領域ID":id,"除外":secrets,
            "root_device":root_device,"root_file_id":root_file_id,
            "root_ancestry":root_ancestry.iter().map(|identity|json!({"device":identity.device,"file_id":identity.file_id})).collect::<Vec<_>>()}),
        );
        let nonce = 識別子生成().map_err(|_| "登録識別子を生成できない")?;
        let registration_hash = digest(
            &json!({"実行系ID":runtime,"作業領域ID":id,"登録識別子":nonce,"除外":secrets,
            "root_device":root_device,"root_file_id":root_file_id,
            "root_ancestry":root_ancestry.iter().map(|identity|json!({"device":identity.device,"file_id":identity.file_id})).collect::<Vec<_>>()}),
        );
        audit("作業領域登録・Permission拒否", &registration_hash)?;
        self.entries.insert(
            id.into(),
            RegisteredWorkspace {
                runtime: runtime.into(),
                root_device,
                root_file_id,
                root_ancestry,
                root_ancestry_complete,
                registration_hash,
                recovery_binding_hash,
                reader,
                grant: None,
                baseline: None,
            },
        );
        Ok(())
    }

    pub(crate) fn operate(
        &mut self,
        operation: &str,
        payload: &Value,
        owner: bool,
        now: i64,
        audit: &mut Audit<'_>,
    ) -> Result<Value, &'static str> {
        if self.last_now.is_some_and(|last| now < last) {
            for entry in self.entries.values_mut() {
                entry.grant = None;
                entry.baseline = None;
            }
            return Err("時計後退を検出したため読取承認を失効");
        }
        self.last_now = Some(now);
        for entry in self.entries.values_mut() {
            if entry
                .grant
                .as_ref()
                .is_some_and(|g| now >= g.expires || Instant::now() >= g.deadline)
            {
                entry.grant = None;
            }
        }
        if matches!(
            operation,
            "作業領域承認" | "作業領域失効" | "作業領域基準点保存" | "作業領域全体基準点保存"
        ) && !owner
        {
            return Err("owner制御資格が必要");
        }
        if operation == "作業領域一覧" {
            if !payload.as_object().is_some_and(|v| v.is_empty()) {
                return Err("作業領域一覧の入力が不正");
            }
            return Ok(json!({"作業領域":self.entries.iter().map(|(id,e)| {
                let grant = e.grant.as_ref().filter(|g| now < g.expires);
                json!({"作業領域ID":id,"実行系ID":e.runtime,"登録hash":e.registration_hash,
                    "承認状態":if grant.is_some(){"approved"}else{"denied"},"有効期限":grant.map(|g|g.expires),
                    "表示範囲":grant.map(|g|g.visibility.as_str()).unwrap_or("none"),"approval_id":grant.map(|g|g.id.as_str())})
            }).collect::<Vec<_>>()}));
        }
        if operation == "作業領域承認" {
            let p: Approval = parse(payload)?;
            if !["none", "hash_only", "summary", "redacted", "full"].contains(&p.表示範囲.as_str())
            {
                return Err("表示範囲が不正");
            }
            let entry = self
                .entries
                .get_mut(&p.作業領域ID)
                .ok_or("作業領域が未登録")?;
            if p.登録hash != entry.registration_hash {
                return Err("登録hashが一致しない");
            }
            let id = 識別子生成().map_err(|_| "承認識別子を生成できない")?;
            let expires = now.checked_add(300).ok_or("承認期限が不正")?;
            let body = json!({"作業領域ID":p.作業領域ID,"approval_id":id,"permission_id":format!("workspace.inspect.{id}"),
                "capability_id":"workspace.inspect","recovery_id":"workspace.reapprove","有効期限":expires,"表示範囲":p.表示範囲});
            audit("ownerが現在登録の読取を承認", &digest(&body))?;
            entry.grant = Some(Grant {
                id,
                visibility: p.表示範囲,
                expires,
                deadline: Instant::now() + Duration::from_secs(300),
            });
            return Ok(body);
        }
        if operation == "作業領域失効" {
            let p: Revocation = parse(payload)?;
            let entry = self
                .entries
                .get_mut(&p.作業領域ID)
                .ok_or("作業領域が未登録")?;
            if p.登録hash != entry.registration_hash {
                return Err("登録hashが一致しない");
            }
            entry.grant = None;
            entry.baseline = None;
            audit("作業領域PermissionとApprovalを失効", &digest(payload))?;
            return Ok(json!({"作業領域ID":p.作業領域ID,"承認状態":"revoked"}));
        }
        if matches!(operation, "作業領域基準点保存" | "作業領域全体基準点保存")
        {
            let whole = operation == "作業領域全体基準点保存";
            let p: Capture = if whole {
                let v: Revocation = parse(payload)?;
                Capture {
                    作業領域ID: v.作業領域ID,
                    登録hash: v.登録hash,
                    相対paths: vec![],
                }
            } else {
                parse(payload)?
            };
            let entry = self
                .entries
                .get_mut(&p.作業領域ID)
                .ok_or("作業領域が未登録")?;
            let grant = entry
                .grant
                .as_ref()
                .filter(|g| {
                    now < g.expires && Instant::now() < g.deadline && g.visibility == "full"
                })
                .ok_or("基準点保存には現在のfull読取承認が必要")?;
            if p.登録hash != entry.registration_hash
                || (!whole && p.相対paths.is_empty())
                || p.相対paths.len() > 128
            {
                return Err("基準点の登録指定または範囲が不正");
            }
            let mut paths = std::collections::BTreeSet::new();
            for path in &p.相対paths {
                entry
                    .reader
                    .validate_relative_path(path, false)
                    .map_err(|_| "基準点pathが不正または除外対象")?;
                if !paths.insert(path.clone()) {
                    return Err("基準点pathが重複");
                }
            }
            entry.baseline = None;
            audit("ownerの現在承認と基準点取得範囲を照合", &digest(payload))?;
            let inventory = if whole {
                Some(
                    entry
                        .reader
                        .comparison_inventory()
                        .map_err(|_| "全体探索が不成立")?,
                )
            } else {
                None
            };
            if let Some(v) = &inventory {
                paths.extend(v.files.iter().cloned());
            }
            let mut retained = 0usize;
            let mut files = BTreeMap::new();
            let mut remaining = crate::workspace_reader::MAX_COMPARISON_BYTES;
            for path in paths {
                if Instant::now() >= grant.deadline {
                    return Err("基準点取得中に承認期限超過");
                }
                let mut bytes = entry
                    .reader
                    .read_comparison_version(&path, remaining)
                    .map_err(|_| "基準点fileの取得拒否または上限超過")?;
                remaining -= bytes.as_ref().map(|v| v.bytes as u64).unwrap_or(0);
                if whole && bytes.is_none() {
                    return Err("全体基準点の取得中にfileが消失");
                }
                if let Some(v) = bytes.as_mut() {
                    let size = v.content.as_ref().map(|b| b.len()).unwrap_or(0);
                    if retained + size > 8 * 1024 * 1024 {
                        v.content = None;
                    } else {
                        retained += size;
                    }
                }
                files.insert(path, bytes);
            }
            if let Some(before) = &inventory {
                if &entry
                    .reader
                    .comparison_inventory()
                    .map_err(|_| "基準点取得後の探索が不成立")?
                    != before
                {
                    return Err("全体基準点の探索範囲が変化");
                }
            }
            let metadata: BTreeMap<_, _> = files
                .iter()
                .map(|(path, bytes)| {
                    (
                        path,
                        bytes
                            .as_ref()
                            .map(|v| json!({"bytes":v.bytes,"sha256":v.sha256})),
                    )
                })
                .collect();
            let nonce = 識別子生成().map_err(|_| "基準点識別子を生成できない")?;
            let hash = digest(
                &json!({"version":1,"登録hash":entry.registration_hash,"nonce":nonce,"作成時刻":now,"whole":whole,"excluded_secrets":inventory.as_ref().map(|v|v.excluded_secrets),"files":metadata}),
            );
            let body = json!({"version":1,"operation":operation,"要求hash":digest(payload),"作業領域ID":p.作業領域ID,
                "実行系ID":entry.runtime,"登録hash":entry.registration_hash,"approval_id":grant.id,"有効期限":grant.expires,
                "表示範囲":grant.visibility,"projection":{"基準点hash":hash,"対象数":files.len()}});
            audit("指定範囲の基準点を確定", &digest(&body))?;
            if Instant::now() >= grant.deadline {
                return Err("基準点監査中に承認期限超過");
            }
            entry.baseline = Some(Baseline { whole, hash, files });
            return Ok(body);
        }
        if operation == REVIEW_OPERATION {
            let p = ReviewRequest::parse(payload)?;
            let (proposal_id, paths) = p.proposal_scope()?;
            let entry = self
                .entries
                .get(&p.workspace_id)
                .ok_or("作業領域が未登録")?;
            let grant = entry
                .grant
                .as_ref()
                .filter(|g| g.id == p.approval_id && g.visibility == "full")
                .ok_or("審査には現在のfull読取承認が必要")?;
            let baseline = entry
                .baseline
                .as_ref()
                .filter(|v| v.whole && v.hash == p.baseline_hash)
                .ok_or("審査には現在の全体基準点が必要")?;
            if p.registration_hash != entry.registration_hash {
                return Err("作業領域の登録hashが不一致");
            }
            for path in &paths {
                entry
                    .reader
                    .validate_relative_path(path, false)
                    .map_err(|_| "提案の対象pathが不正または除外対象")?;
            }
            audit(
                "編集提案、作業領域、現在の読取承認と全体基準点を照合",
                &digest(payload),
            )?;
            let mut changes = Vec::new();
            let mut projection_bytes = 0;
            let (_, excluded_secrets) =
                entry.compare_whole(baseline, grant, |path, status, before, after| {
                    if !paths.contains(&path) {
                        return Err("提案の対象外に変更がある");
                    }
                    let change = json!({
                        "path": path,
                        "status": status,
                        "diff": crate::workspace_diff::generate_versions(before, after)
                    });
                    projection_bytes += serde_json::to_vec(&change)
                        .map_err(|_| "審査差分を整形できない")?
                        .len();
                    if projection_bytes > 2 * 1024 * 1024 {
                        return Err("審査差分が表示上限を超えた");
                    }
                    changes.push(change);
                    Ok(())
                })?;
            if changes.is_empty() {
                return Err("審査対象の変更がない");
            }
            let diff_hash = digest(&json!({"changes": &changes}));
            return Ok(json!({
                "version": 1, "operation": REVIEW_OPERATION, "要求hash": digest(payload),
                "作業領域ID": p.workspace_id, "実行系ID": entry.runtime,
                "登録hash": entry.registration_hash, "approval_id": grant.id,
                "有効期限": grant.expires, "表示範囲": grant.visibility,
                "projection": {
                    "proposal_id": proposal_id, "proposal_audit_id": p.proposal_audit_id,
                    "proposal_hash": digest(&p.proposal), "基準点hash": baseline.hash,
                    "diff_hash": diff_hash, "changes": changes,
                    "excluded_secrets": excluded_secrets
                }
            }));
        }
        if operation == APPLY_TARGET_OPERATION {
            return self.prepare_edit_target(payload, audit);
        }
        if !matches!(
            operation,
            "作業領域ツリー"
                | "作業領域読取"
                | "作業領域差分"
                | "作業領域復旧プレビュー"
                | "作業領域比較範囲"
                | "作業領域変更一覧"
        ) {
            return Err("作業領域操作が不正");
        }
        let (p, baseline_hash) = if matches!(
            operation,
            "作業領域差分" | "作業領域復旧プレビュー" | "作業領域変更一覧"
        ) {
            let value: DiffSelection = parse(payload)?;
            (
                Selection {
                    作業領域ID: value.作業領域ID,
                    相対path: value.相対path,
                },
                Some(value.基準点hash),
            )
        } else {
            (parse::<Selection>(payload)?, None)
        };
        let entry = self.entries.get(&p.作業領域ID).ok_or("作業領域が未登録")?;
        let grant = entry
            .grant
            .as_ref()
            .filter(|g| now < g.expires)
            .ok_or("現在の作業領域読取承認が必要")?;
        if matches!(operation, "作業領域比較範囲" | "作業領域変更一覧") && !p.相対path.is_empty()
        {
            return Err("比較範囲にpath指定は不要");
        }
        entry
            .reader
            .validate_relative_path(
                &p.相対path,
                matches!(
                    operation,
                    "作業領域ツリー" | "作業領域比較範囲" | "作業領域変更一覧"
                ),
            )
            .map_err(|_| "相対pathが不正または除外対象")?;
        let baseline = if let Some(hash) = baseline_hash.as_ref() {
            let baseline = entry.baseline.as_ref().ok_or("現在登録の基準点が必要")?;
            if hash != &baseline.hash
                || (!baseline.whole && !baseline.files.contains_key(&p.相対path))
            {
                return Err("基準点hashまたは指定範囲が不一致");
            }
            if operation == "作業領域変更一覧" && !baseline.whole {
                return Err("全体基準点が必要");
            }
            Some(baseline)
        } else {
            None
        };
        let binding = json!({"作業領域ID":p.作業領域ID,"実行系ID":entry.runtime,"登録hash":entry.registration_hash,
            "approval_id":grant.id,"permission_id":format!("workspace.inspect.{}",grant.id),
            "capability_id":"workspace.inspect","recovery_id":"workspace.reapprove","要求":payload});
        audit("現在の読取PermissionとApprovalを照合", &digest(&binding))?;
        // 内容を許可しないprojectionでは、filesystemへの不要な取得も行わない。
        let projection = match grant.visibility.as_str() {
            "none" => Value::Null,
            "summary" | "redacted" => {
                json!({"説明":"この表示範囲に提供できる承認済み内容はありません"})
            }
            "full" | "hash_only" => {
                let data = if operation == "作業領域比較範囲" {
                    json!({"基準点hash":entry.baseline.as_ref().map(|v|v.hash.as_str()),"相対paths":entry.baseline.as_ref().map(|v|v.files.keys().collect::<Vec<_>>()).unwrap_or_default()})
                } else if operation == "作業領域変更一覧" {
                    let baseline = baseline.ok_or("全体基準点が必要")?;
                    let mut changes = Vec::new();
                    let (unchanged, excluded_secrets) =
                        entry.compare_whole(baseline, grant, |path, status, _, _| {
                            changes.push(json!({"path":path,"status":status}));
                            Ok(())
                        })?;
                    json!({"基準点hash":baseline.hash,"changes":changes,"unchanged":unchanged,"excluded_secrets":excluded_secrets})
                } else if let Some(baseline) = baseline {
                    let after = entry
                        .reader
                        .read_comparison_version(
                            &p.相対path,
                            crate::workspace_reader::MAX_COMPARISON_BYTES,
                        )
                        .map_err(|_| "比較fileの取得拒否または上限超過")?;
                    let before = baseline.files.get(&p.相対path).and_then(|v| v.as_ref());
                    if operation == "作業領域復旧プレビュー" {
                        let diff = crate::workspace_diff::generate_versions(after.as_ref(), before);
                        let action = if diff.kind == crate::workspace_diff::DiffKind::Unchanged {
                            "none"
                        } else if before.is_none() {
                            "remove"
                        } else if after.is_none() {
                            "recreate"
                        } else {
                            "replace"
                        };
                        json!({"基準点hash":baseline.hash,"diff":diff,"action":action,
                            "baseline_content_available":before.is_none_or(|v|v.content.is_some()),"execution_permitted":false})
                    } else {
                        json!({"基準点hash":baseline.hash,"diff":crate::workspace_diff::generate_versions(before,after.as_ref())})
                    }
                } else if operation == "作業領域ツリー" {
                    let items = entry
                        .reader
                        .list(&p.相対path)
                        .map_err(|_| "作業領域を安全に列挙できない")?;
                    json!({"entries":items.into_iter().map(|v|json!({"path":v.path,"kind":match v.kind {EntryKind::File=>"file",EntryKind::Directory=>"directory"},"bytes":v.bytes})).collect::<Vec<_>>()})
                } else {
                    let bytes = entry
                        .reader
                        .read(&p.相対path)
                        .map_err(|_| "作業領域fileを安全に読めない")?;
                    let text = std::str::from_utf8(&bytes)
                        .ok()
                        .filter(|_| !bytes.contains(&0));
                    json!({"path":p.相対path,"bytes":bytes.len(),"sha256":sha256_tagged(&bytes),"binary":text.is_none(),"text":text})
                };
                if grant.visibility == "full" {
                    data
                } else {
                    json!({"sha256":digest(&data)})
                }
            }
            _ => return Err("現在の表示範囲が不正"),
        };
        let body = json!({"version":1,"operation":operation,"要求hash":digest(payload),
            "作業領域ID":p.作業領域ID,"実行系ID":entry.runtime,"登録hash":entry.registration_hash,
            "有効期限":grant.expires,"表示範囲":grant.visibility,"approval_id":grant.id,"projection":projection});
        audit("作業領域の取得projectionを確定", &digest(&body))?;
        if Instant::now() >= grant.deadline {
            return Err("取得中に読取承認が期限超過");
        }
        Ok(body)
    }

    fn prepare_edit_target(
        &self,
        payload: &Value,
        audit: &mut Audit<'_>,
    ) -> Result<Value, &'static str> {
        let request = ApplyTargetRequest::parse(payload)?;
        let binding = request.binding()?;
        let source = self
            .entries
            .get(&binding.workspace_id)
            .ok_or("審査元の作業領域が未登録")?;
        let target = self
            .entries
            .get(&request.workspace_id)
            .ok_or("適用先の作業領域が未登録")?;
        let identity = |entry: &RegisteredWorkspace| super::workspace_root::DirectoryIdentity {
            device: entry.root_device,
            file_id: entry.root_file_id,
        };
        if !source.root_ancestry_complete
            || !target.root_ancestry_complete
            || source.root_ancestry.contains(&identity(target))
            || target.root_ancestry.contains(&identity(source))
            || source.registration_hash != binding.registration_hash
            || target.registration_hash != request.registration_hash
        {
            return Err("現在の審査元と適用先は相互に独立した登録rootである必要がある");
        }
        let source_grant = source
            .grant
            .as_ref()
            .filter(|grant| {
                grant.visibility == "full"
                    && grant.id == binding.approval_id
                    && grant.expires == binding.expires_at
            })
            .ok_or("審査時の読取承認が失効または置換された")?;
        let target_grant = target
            .grant
            .as_ref()
            .filter(|grant| grant.visibility == "full" && grant.id == request.approval_id)
            .ok_or("適用先には現在のfull読取承認が必要")?;
        if !source
            .baseline
            .as_ref()
            .is_some_and(|baseline| baseline.whole && baseline.hash == binding.baseline_hash)
        {
            return Err("審査元の全体基準点が変わった");
        }
        audit("審査監査、独立rootと現在の読取承認を照合", &digest(payload))?;
        let mut budgets = [crate::workspace_reader::MAX_COMPARISON_BYTES; 2];
        let mut files = Vec::with_capacity(binding.files.len());
        for file in binding.files {
            for (index, (reader, grant, expected, error)) in [
                (
                    &source.reader,
                    source_grant,
                    &file.after_hash,
                    "審査元の版が変更された",
                ),
                (
                    &target.reader,
                    target_grant,
                    &file.before_hash,
                    "適用先の版が審査時の変更前版と一致しない",
                ),
            ]
            .into_iter()
            .enumerate()
            {
                if Instant::now() >= grant.deadline {
                    return Err("適用先照合中に承認期限超過");
                }
                let observed = reader
                    .read_comparison_version(&file.path, budgets[index])
                    .map_err(|_| "対象版を安全に確認できない")?;
                budgets[index] -= observed.as_ref().map(|v| v.bytes as u64).unwrap_or(0);
                if observed.as_ref().map(|v| &v.sha256) != expected.as_ref() {
                    return Err(error);
                }
            }
            files.push(json!({"path": file.path, "before_hash": file.before_hash, "after_hash": file.after_hash}));
        }
        Ok(json!({
            "version": 1, "operation": APPLY_TARGET_OPERATION, "要求hash": digest(payload),
            "作業領域ID": request.workspace_id, "実行系ID": target.runtime,
            "登録hash": target.registration_hash, "approval_id": target_grant.id,
            "有効期限": target_grant.expires, "表示範囲": target_grant.visibility,
            "projection": {
                "review_audit_id": request.review_audit_id, "proposal_id": binding.proposal_id,
                "diff_hash": binding.diff_hash, "source_workspace_id": binding.workspace_id,
                "source_approval_id": source_grant.id, "files": files
            }
        }))
    }

    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    pub(super) fn edit_workspace_identity(
        &self,
        id: &str,
    ) -> Option<super::dialogue::AgentTaskWorkspaceIdentity> {
        let entry = self.entries.get(id)?;
        Some(super::dialogue::AgentTaskWorkspaceIdentity::new(
            entry.root_device,
            entry.root_file_id,
        ))
    }

    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    pub(super) fn apply_reviewed_edit(
        &mut self,
        request: super::ai_edit_center::ApplyRequest,
        target_request: super::ai_edit_center::ApplyTargetRequest,
        binding: super::ai_edit_center::ApplyBinding,
        applied_request: &Value,
        checked_target_hash: &str,
        plan_hash: &str,
        now: i64,
        store: &super::store::BrokerPersistentStore,
        audit: &mut dyn FnMut(&str, &str, &str) -> Result<String, &'static str>,
    ) -> Result<(Value, String), &'static str> {
        use super::ai_edit_center::APPLY_OPERATION;
        use super::ai_edit_transaction::{EditStatus, EditTransaction};
        use cap_fs_ext::DirExt;
        use cap_std::fs::DirBuilder;

        let checked = self.operate(
            APPLY_TARGET_OPERATION,
            &request.target_request,
            false,
            now,
            &mut |_, _| Ok(()),
        )?;
        if digest(&checked) != checked_target_hash {
            return Err("適用先照合の監査と現在の照合結果が不一致");
        }
        let source = self
            .entries
            .get(&binding.workspace_id)
            .ok_or("審査元が未登録")?;
        let target = self
            .entries
            .get(&target_request.workspace_id)
            .ok_or("適用先が未登録")?;
        let source_grant = source.grant.as_ref().ok_or("審査元の承認が期限超過")?;
        let target_grant = target.grant.as_ref().ok_or("適用先の承認が期限超過")?;
        let current =
            || Instant::now() < source_grant.deadline && Instant::now() < target_grant.deadline;

        let (store_root, _, store_ancestry) =
            super::workspace_root::open_isolated_root_with_ancestry(store.root(), &[])?;
        let store_identity = store_ancestry.last().ok_or("退避先の実体を確認できない")?;
        if store_identity.device != target.root_device {
            return Err("適用先と同じvolumeにBrokerの保護された退避先が必要");
        }
        for workspace in [source, target] {
            let identity = super::workspace_root::DirectoryIdentity {
                device: workspace.root_device,
                file_id: workspace.root_file_id,
            };
            if store_ancestry.contains(&identity)
                || workspace.root_ancestry.contains(store_identity)
            {
                return Err("Brokerの退避先と作業領域の重複を拒否");
            }
        }
        let key = digest(&json!({
            "review_audit_id": target_request.review_audit_id,
            "target_device": target.root_device,
            "target_file_id": target.root_file_id,
        }));
        let transaction_id = key
            .strip_prefix("sha256:")
            .ok_or("編集適用IDの生成に失敗")?;
        let vault_name = format!("edit-files-{transaction_id}");
        if !current() {
            return Err("現在の読取承認が失効");
        }
        if store.read_workspace_edit_record(transaction_id)?.is_some() {
            let vault = WorkspaceReader::from_registered_dir(
                store_root
                    .open_dir_nofollow(&vault_name)
                    .map_err(|_| "既存編集Journalの保護退避先を開けない")?,
                &[],
            )
            .map_err(|_| "既存編集Journalの退避先実体が不正")?;
            let transaction = EditTransaction {
                store,
                target: &target.reader,
                vault: &vault,
            };
            audit(
                "recorded",
                "既存編集Journalを再読し、再適用せず中断分だけを補償復旧する",
                plan_hash,
            )?;
            if !current() {
                return Err("補償復旧の着手前に現在の読取承認が失効");
            }
            let status = transaction.recover_existing(transaction_id)?;
            audit(
                "recorded",
                &format!("既存編集Journalの補償復旧状態を確定: {status:?}"),
                plan_hash,
            )?;
            return match status {
                EditStatus::Reverted => Err(
                    "既存編集Journalの補償復旧状態を確認しました。今回の適用は行っていません。新しい提案・審査・一回Approvalが必要です",
                ),
                EditStatus::RecoveryRequired => {
                    Err("補償復旧を完了できません。Workspace差分と復旧記録を確認してください")
                }
                EditStatus::Applied => {
                    Err("同じ審査の編集は既に適用済みです。再適用しません")
                }
                EditStatus::Applying | EditStatus::Reverting => {
                    Err("編集の補償復旧状態を確定できません。復旧記録を保持します")
                }
            };
        }
        let mut builder = DirBuilder::new();
        #[cfg(unix)]
        {
            use cap_std::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        #[cfg(not(unix))]
        builder.recursive(false);
        // 適用先・審査に固有の未使用名だけを確保し、確認と適用の二重実行を拒否する。
        let reason = format!("編集適用の一回認可を消費。復旧記録 {transaction_id}");
        audit("recorded", &reason, plan_hash)?;
        store_root
            .create_dir_with(&vault_name, &builder)
            .map_err(|_| "適用が使用済みか退避先を新規確保できない")?;
        let vault = WorkspaceReader::from_registered_dir(
            store_root
                .open_dir_nofollow(&vault_name)
                .map_err(|_| "退避先を開けない")?,
            &[],
        )
        .map_err(|_| "退避先の実体が不正")?;
        let transaction = EditTransaction {
            store,
            target: &target.reader,
            vault: &vault,
        };
        match transaction.apply_with_request(
            transaction_id,
            plan_hash,
            &binding.files,
            &source.reader,
            Some(applied_request),
            current,
        )? {
            EditStatus::Applied => (),
            EditStatus::Reverted => return Err("編集を適用できなかったため元の版へ戻した"),
            _ => return Err("編集を停止した。保護した版の復旧記録を確認"),
        }
        let body = json!({
            "version": 1, "operation": APPLY_OPERATION, "request_hash": plan_hash,
            "workspace_id": target_request.workspace_id, "review_audit_id": target_request.review_audit_id,
            "diff_hash": binding.diff_hash, "transaction_id": transaction_id,
            "changed_files": binding.files.len(), "status": "applied"
        });
        let terminal = audit("accepted", "編集適用と復旧記録の耐久確定", &digest(&body));
        if terminal.is_err() {
            let _result = transaction.rollback(transaction_id, plan_hash, || true);
        }
        terminal.map(|event_id| (body, event_id))
    }

    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    pub(super) fn list_rollback_candidates(
        &mut self,
        payload: &Value,
        now: i64,
        store: &super::store::BrokerPersistentStore,
        audit_log: &BrokerAuditLog,
        audit: &mut Audit<'_>,
    ) -> Result<Value, &'static str> {
        use super::ai_edit_center::find_applied_receipt;
        use super::ai_edit_transaction::{EditStatus, EditTransaction};
        use cap_fs_ext::DirExt;

        let request = RollbackCandidatesRequest::parse(payload)?;
        if self.last_now.is_some_and(|last| now < last) {
            for entry in self.entries.values_mut() {
                entry.grant = None;
                entry.baseline = None;
            }
            return Err("時計後退を検出したためWorkspace Permissionを失効");
        }
        self.last_now = Some(now);
        for entry in self.entries.values_mut() {
            if entry
                .grant
                .as_ref()
                .is_some_and(|grant| now >= grant.expires || Instant::now() >= grant.deadline)
            {
                entry.grant = None;
            }
        }
        let entry = self
            .entries
            .get(&request.workspace_id)
            .ok_or("Rollback候補のWorkspaceが未登録")?;
        let root = entry
            .reader
            .edit_root_identity()
            .map_err(|_| "Rollback候補のWorkspace rootを確認できない")?;
        if entry.registration_hash != request.registration_hash
            || root != (entry.root_device, entry.root_file_id)
        {
            return Err("Rollback候補の現在登録またはroot実体が一致しない");
        }
        let grant = entry
            .grant
            .as_ref()
            .filter(|grant| {
                grant.id == request.approval_id
                    && grant.visibility == "full"
                    && now < grant.expires
                    && Instant::now() < grant.deadline
            })
            .ok_or("Rollback候補一覧には現在のfull Workspace Permissionが必要")?;
        let current = || now < grant.expires && Instant::now() < grant.deadline;
        if !current() {
            return Err("Rollback候補の読取開始前にPermissionが失効");
        }
        let permission_binding = json!({
            "workspace_id": request.workspace_id,
            "registration_hash": request.registration_hash,
            "approval_id": grant.id,
            "capability_id": "workspace.inspect",
            "visibility": grant.visibility,
            "request": payload
        });
        audit(
            "現在のfull Workspace PermissionとRollback候補要求を照合",
            &digest(&permission_binding),
        )?;

        let (ids, next_cursor) = store
            .workspace_edit_record_page(request.cursor.as_deref(), ROLLBACK_CANDIDATE_PAGE_SIZE)?;
        let (store_root, _, store_ancestry) =
            super::workspace_root::open_isolated_root_with_ancestry(store.root(), &[])?;
        let store_identity = store_ancestry
            .last()
            .ok_or("Rollback候補の保管rootを確認できない")?;
        let target_identity = super::workspace_root::DirectoryIdentity {
            device: entry.root_device,
            file_id: entry.root_file_id,
        };
        if store_identity.device != entry.root_device
            || store_ancestry.contains(&target_identity)
            || entry.root_ancestry.contains(store_identity)
        {
            return Err("Rollback候補のJournal保管先とWorkspaceが分離されていない");
        }

        let mut candidates = Vec::new();
        let mut unavailable_count = 0usize;
        for id in ids {
            if !current() {
                return Err("Rollback候補の確認中にWorkspace Permissionが失効");
            }
            let Some(journal) = EditTransaction::inspect_stored_record(store, &id)? else {
                continue;
            };
            if journal.target != root {
                continue;
            }
            if !matches!(
                journal.status,
                EditStatus::Applied | EditStatus::Reverting | EditStatus::RecoveryRequired
            ) {
                continue;
            }
            let Some(applied_request) = journal.apply_request.as_ref() else {
                unavailable_count += 1;
                continue;
            };
            let (_apply, target_request, binding) = ApplyRequest::parse(applied_request)?;
            let request_hash = super::protocol::canonical_payload_hash(Some(applied_request));
            if request_hash != journal.plan_hash
                || target_request.workspace_id != request.workspace_id
            {
                return Err("Rollback候補Journalと元適用要求が一致しない");
            }
            let expected_id = digest(&json!({
                "review_audit_id": target_request.review_audit_id,
                "target_device": root.0,
                "target_file_id": root.1
            }));
            if expected_id.strip_prefix("sha256:") != Some(id.as_str()) {
                return Err("Rollback候補のTransaction IDが現在rootと一致しない");
            }
            if journal.files.len() != binding.files.len()
                || journal
                    .files
                    .iter()
                    .zip(&binding.files)
                    .any(|(record, request)| {
                        record.path != request.path
                            || record.before_hash != request.before_hash
                            || record.after_hash != request.after_hash
                    })
            {
                return Err("Rollback候補Journalと適用file一覧が一致しない");
            }
            let receipt = match find_applied_receipt(
                audit_log,
                &request_hash,
                &target_request.workspace_id,
                &target_request.review_audit_id,
                &binding.diff_hash,
                &id,
                binding.files.len(),
            ) {
                Ok(receipt) => receipt,
                Err("同じ編集適用に複数のAudit receiptがある") => {
                    return Err("Rollback候補に重複した適用Audit receiptがある");
                }
                Err(_) => {
                    unavailable_count += 1;
                    continue;
                }
            };
            let vault_name = format!("edit-files-{id}");
            let vault = WorkspaceReader::from_registered_dir(
                store_root
                    .open_dir_nofollow(&vault_name)
                    .map_err(|_| "Rollback候補の保護退避先を開けない")?,
                &[],
            )
            .map_err(|_| "Rollback候補の保護退避先実体が不正")?;
            let transaction = EditTransaction {
                store,
                target: &entry.reader,
                vault: &vault,
            };
            let matches = transaction.candidate_matches(&journal, &binding.files)?;
            let availability = match journal.status {
                EditStatus::Applied if matches => "available",
                EditStatus::Applied => "conflict",
                EditStatus::Reverting | EditStatus::RecoveryRequired => "resume",
                EditStatus::Applying | EditStatus::Reverted => continue,
            };
            candidates.push(json!({
                "transaction_id": id,
                "applied_request_hash": request_hash,
                "applied_audit_id": receipt.event_id,
                "applied_request": applied_request,
                "diff_hash": binding.diff_hash,
                "changed_files": binding.files.len(),
                "status": match journal.status {
                    EditStatus::Applied => "applied",
                    EditStatus::Reverting => "reverting",
                    EditStatus::RecoveryRequired => "recovery_required",
                    EditStatus::Applying | EditStatus::Reverted => continue,
                },
                "availability": availability
            }));
            if !current() {
                return Err("Rollback候補の確認完了前にWorkspace Permissionが失効");
            }
        }
        let body = json!({
            "version": 1,
            "operation": ROLLBACK_CANDIDATES_OPERATION,
            "要求hash": digest(payload),
            "作業領域ID": request.workspace_id,
            "実行系ID": entry.runtime,
            "登録hash": entry.registration_hash,
            "approval_id": grant.id,
            "有効期限": grant.expires,
            "表示範囲": grant.visibility,
            "projection": {
                "candidates": candidates,
                "unavailable_count": unavailable_count,
                "next_cursor": next_cursor
            }
        });
        audit(
            "Broker確認済みRollback候補のbounded projectionを確定",
            &digest(&body),
        )?;
        if !current() {
            return Err("Rollback候補projectionの確定前にWorkspace Permissionが失効");
        }
        Ok(body)
    }

    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    pub(super) fn rollback_reviewed_edit(
        &mut self,
        payload: &Value,
        rollback_request_hash: &str,
        now: i64,
        store: &super::store::BrokerPersistentStore,
        audit: &mut dyn FnMut(&str, &str, &str) -> Result<String, &'static str>,
    ) -> Result<(Value, String), &'static str> {
        use super::ai_edit_transaction::{EditStatus, EditTransaction};
        use cap_fs_ext::DirExt;

        let request = RollbackRequest::parse(payload)?;
        if super::protocol::canonical_payload_hash(Some(payload)) != rollback_request_hash {
            return Err("Rollbackの現在要求hashが一致しない");
        }
        let (_, target_request, binding) = ApplyRequest::parse(&request.applied_request)?;
        if self.last_now.is_some_and(|last| now < last) {
            for entry in self.entries.values_mut() {
                entry.grant = None;
                entry.baseline = None;
            }
            return Err("時計後退を検出したためWorkspace Permissionを失効");
        }
        self.last_now = Some(now);
        let target = self
            .entries
            .get_mut(&request.workspace_id)
            .ok_or("Rollback対象Workspaceが未登録")?;
        if target.registration_hash != request.registration_hash
            || target_request.workspace_id != request.workspace_id
        {
            return Err("Rollback対象Workspaceの現在登録が一致しない");
        }
        let expired = target
            .grant
            .as_ref()
            .is_none_or(|grant| now >= grant.expires || Instant::now() >= grant.deadline);
        if expired {
            target.grant = None;
            return Err("Rollbackには現在のWorkspace Permissionが必要");
        }
        let grant = target.grant.as_ref().ok_or("Workspace Permissionがない")?;
        if grant.id != request.approval_id || grant.visibility != "full" {
            return Err("Rollback要求は現在のfull Workspace Permissionと一致しない");
        }
        let current = || now < grant.expires && Instant::now() < grant.deadline;
        if !current() {
            return Err("Rollbackの着手前にWorkspace Permissionが失効");
        }

        let expected_id = digest(&json!({
            "review_audit_id": target_request.review_audit_id,
            "target_device": target.root_device,
            "target_file_id": target.root_file_id,
        }));
        if expected_id.strip_prefix("sha256:") != Some(request.transaction_id.as_str())
            || super::protocol::canonical_payload_hash(Some(&request.applied_request))
                != request.applied_request_hash
        {
            return Err("Rollback要求が元の適用Workspace・審査・hashと一致しない");
        }

        let (store_root, _, store_ancestry) =
            super::workspace_root::open_isolated_root_with_ancestry(store.root(), &[])?;
        let store_identity = store_ancestry
            .last()
            .ok_or("編集退避先の実体を確認できない")?;
        let target_identity = super::workspace_root::DirectoryIdentity {
            device: target.root_device,
            file_id: target.root_file_id,
        };
        if store_identity.device != target.root_device
            || store_ancestry.contains(&target_identity)
            || target.root_ancestry.contains(store_identity)
        {
            return Err("編集Journalと退避先が現在のWorkspace rootから分離されていない");
        }
        let vault_name = format!("edit-files-{}", request.transaction_id);
        let vault = WorkspaceReader::from_registered_dir(
            store_root
                .open_dir_nofollow(&vault_name)
                .map_err(|_| "Rollbackの保護退避先を開けない")?,
            &[],
        )
        .map_err(|_| "Rollbackの保護退避先実体が不正")?;
        let transaction = EditTransaction {
            store,
            target: &target.reader,
            vault: &vault,
        };
        let intent = format!(
            "独立Owner確認済みRollbackを開始。transaction={} apply_hash={} files={}",
            request.transaction_id,
            request.applied_request_hash,
            binding.files.len(),
        );
        audit("recorded", &intent, rollback_request_hash)?;
        if !current() {
            return Err("Rollbackの耐久意図記録後にWorkspace Permissionが失効");
        }
        let status = transaction.rollback(
            &request.transaction_id,
            &request.applied_request_hash,
            current,
        )?;
        if status != EditStatus::Reverted {
            audit(
                "rejected",
                &format!(
                    "Rollbackが未完了。transaction={} status={status:?}",
                    request.transaction_id
                ),
                rollback_request_hash,
            )?;
            return Err("Rollbackを完了できないためJournalとWorkspace差分を保持");
        }
        let body = json!({
            "version": 1,
            "operation": ROLLBACK_OPERATION,
            "request_hash": rollback_request_hash,
            "applied_request_hash": request.applied_request_hash,
            "applied_audit_id": request.applied_audit_id,
            "workspace_id": request.workspace_id,
            "review_audit_id": target_request.review_audit_id,
            "diff_hash": binding.diff_hash,
            "transaction_id": request.transaction_id,
            "changed_files": binding.files.len(),
            "status": "rolled_back"
        });
        let event_id = audit(
            "accepted",
            "独立Approvalによる編集Rollbackと耐久Journalを確定",
            &digest(&body),
        )?;
        Ok((body, event_id))
    }
}
