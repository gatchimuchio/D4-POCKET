//! 作業領域の現在Permissionと読取。rootの選択やowner認証は所有しない。
#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use cap_std::fs::Dir;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::{audit_hash::sha256_tagged, workspace_reader::{EntryKind, WorkspaceReader}};
use super::dialogue::識別子生成;

type Audit<'a> = dyn FnMut(&str, &str) -> Result<(), &'static str> + 'a;

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
        f.debug_struct("WorkspaceRegistry").field("登録数", &self.entries.len()).finish()
    }
}

struct RegisteredWorkspace {
    runtime: String,
    root_device: u64,
    root_file_id: u64,
    root_ancestry: Vec<super::workspace_root::DirectoryIdentity>,
    root_ancestry_complete: bool,
    registration_hash: String,
    reader: WorkspaceReader,
    grant: Option<Grant>,
    baseline: Option<Baseline>,
}

/// Broker内で照合済みのSession-to-Workspace参照。
/// Permissionやfilesystem accessを保持せず、登録関係の監査相関にだけ使う。
pub(crate) struct DialogueWorkspaceBinding {
    runtime: String,
    workspace_id: String,
    registration_hash: String,
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
        Self::for_test_with_registration_hash(
            runtime,
            workspace_id,
            "sha256:fixture-registration",
        )
    }

    #[cfg(test)]
    pub(crate) fn for_test_with_registration_hash(
        runtime: &str,
        workspace_id: &str,
        registration_hash: &str,
    ) -> Self {
        Self {
            runtime: runtime.to_owned(),
            workspace_id: workspace_id.to_owned(),
            registration_hash: registration_hash.to_owned(),
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
fn identifier(id: &str) -> bool {
    !id.is_empty() && id.len() <= 128 && id.as_bytes()[0].is_ascii_alphanumeric()
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
}
fn digest(value: &Value) -> String {
    sha256_tagged(value.to_string().as_bytes())
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
        })
    }

    /// terminal lifecycle隔離後に、同じ実行系へ結合した作業領域root、承認、基準点を残さない。
    pub(crate) fn remove_runtime(&mut self, runtime: &str) {
        self.entries.retain(|_, entry| entry.runtime != runtime);
    }

    pub(crate) fn response_current(&self, body: &Value, now: i64) -> bool {
        if self.last_now.is_some_and(|last| now < last) {return false;}
        body["作業領域ID"].as_str().and_then(|id| self.entries.get(id))
            .and_then(|entry| entry.grant.as_ref()).is_some_and(|grant|
                now < grant.expires && Instant::now() < grant.deadline && body["approval_id"] == grant.id)
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
        if !identifier(id) || !identifier(runtime) || self.entries.contains_key(id) || self.entries.len() >= 16 {
            return Err("作業領域の登録指定が不正または上限超過");
        }
        let identity=root.dir_metadata().map_err(|_| "作業領域のroot識別子を確認できない")?;
        let root_device=cap_fs_ext::MetadataExt::dev(&identity);
        let root_file_id=cap_fs_ext::MetadataExt::ino(&identity);
        if root_file_id == 0 {
            return Err("root file IDが未観測のため作業領域登録を拒否");
        }
        let root_identity=super::workspace_root::DirectoryIdentity {device:root_device,file_id:root_file_id};
        let (root_ancestry,root_ancestry_complete)=match ancestry {
            Some(ancestry)
                if !ancestry.is_empty()
                    && ancestry.len()>=2
                    && ancestry.len()<=65
                    && ancestry.last()==Some(&root_identity)
                    && ancestry.iter().all(|identity|identity.file_id!=0) => (ancestry,true),
            Some(_) => return Err("作業領域の親子識別範囲が不正または未観測"),
            None => (vec![root_identity],false),
        };
        let other_runtimes:Vec<&RegisteredWorkspace>=self.entries.values()
            .filter(|entry|entry.runtime!=runtime).collect();
        if other_runtimes.iter().any(|entry| {
            entry.root_device == root_device && entry.root_file_id == root_file_id
        }) {
            return Err("別Agentとの同一物理作業領域共有を拒否");
        }
        if other_runtimes.iter().any(|entry|!entry.root_ancestry_complete) || (!other_runtimes.is_empty()&&!root_ancestry_complete) {
            return Err("別AgentとのWorkspace分離範囲を確認できないため登録を拒否");
        }
        if other_runtimes.iter().any(|entry| {
            let existing_root=super::workspace_root::DirectoryIdentity {
                device:entry.root_device,
                file_id:entry.root_file_id,
            };
            entry.root_ancestry.contains(&root_identity)
                || root_ancestry.contains(&existing_root)
        }) {
            return Err("別Agentとの親子Workspace範囲重複を拒否");
        }
        let reader = WorkspaceReader::from_registered_dir(root, secrets).map_err(|_| "作業領域の除外指定が不正")?;
        let nonce = 識別子生成().map_err(|_| "登録識別子を生成できない")?;
        let registration_hash = digest(&json!({"実行系ID":runtime,"作業領域ID":id,"登録識別子":nonce,"除外":secrets,
            "root_device":root_device,"root_file_id":root_file_id,
            "root_ancestry":root_ancestry.iter().map(|identity|json!({"device":identity.device,"file_id":identity.file_id})).collect::<Vec<_>>()}));
        audit("作業領域登録・Permission拒否", &registration_hash)?;
        self.entries.insert(id.into(), RegisteredWorkspace {
            runtime:runtime.into(),root_device,root_file_id,root_ancestry,root_ancestry_complete,registration_hash,reader,grant:None,baseline:None,
        });
        Ok(())
    }

    pub(crate) fn operate(&mut self, operation: &str, payload: &Value, owner: bool, now: i64, audit: &mut Audit<'_>) -> Result<Value, &'static str> {
        if self.last_now.is_some_and(|last| now < last) {
            for entry in self.entries.values_mut() {entry.grant = None;entry.baseline = None;}
            return Err("時計後退を検出したため読取承認を失効");
        }
        self.last_now = Some(now);
        for entry in self.entries.values_mut() {
            if entry.grant.as_ref().is_some_and(|g| now >= g.expires || Instant::now() >= g.deadline) {entry.grant = None;}
        }
        if matches!(operation, "作業領域承認" | "作業領域失効" | "作業領域基準点保存" | "作業領域全体基準点保存") && !owner {
            return Err("owner制御資格が必要");
        }
        if operation == "作業領域一覧" {
            if !payload.as_object().is_some_and(|v| v.is_empty()) {return Err("作業領域一覧の入力が不正");}
            return Ok(json!({"作業領域":self.entries.iter().map(|(id,e)| {
                let grant = e.grant.as_ref().filter(|g| now < g.expires);
                json!({"作業領域ID":id,"実行系ID":e.runtime,"登録hash":e.registration_hash,
                    "承認状態":if grant.is_some(){"approved"}else{"denied"},"有効期限":grant.map(|g|g.expires),
                    "表示範囲":grant.map(|g|g.visibility.as_str()).unwrap_or("none"),"approval_id":grant.map(|g|g.id.as_str())})
            }).collect::<Vec<_>>()}));
        }
        if operation == "作業領域承認" {
            let p: Approval = parse(payload)?;
            if !["none","hash_only","summary","redacted","full"].contains(&p.表示範囲.as_str()) {return Err("表示範囲が不正");}
            let entry = self.entries.get_mut(&p.作業領域ID).ok_or("作業領域が未登録")?;
            if p.登録hash != entry.registration_hash {return Err("登録hashが一致しない");}
            let id = 識別子生成().map_err(|_| "承認識別子を生成できない")?;
            let expires = now.checked_add(300).ok_or("承認期限が不正")?;
            let body = json!({"作業領域ID":p.作業領域ID,"approval_id":id,"permission_id":format!("workspace.inspect.{id}"),
                "capability_id":"workspace.inspect","recovery_id":"workspace.reapprove","有効期限":expires,"表示範囲":p.表示範囲});
            audit("ownerが現在登録の読取を承認", &digest(&body))?;
            entry.grant = Some(Grant {id,visibility:p.表示範囲,expires,deadline:Instant::now()+Duration::from_secs(300)});
            return Ok(body);
        }
        if operation == "作業領域失効" {
            let p: Revocation = parse(payload)?;
            let entry = self.entries.get_mut(&p.作業領域ID).ok_or("作業領域が未登録")?;
            if p.登録hash != entry.registration_hash {return Err("登録hashが一致しない");}
            entry.grant = None;
            entry.baseline = None;
            audit("作業領域PermissionとApprovalを失効", &digest(payload))?;
            return Ok(json!({"作業領域ID":p.作業領域ID,"承認状態":"revoked"}));
        }
        if matches!(operation,"作業領域基準点保存" | "作業領域全体基準点保存") {
            let whole=operation=="作業領域全体基準点保存";
            let p: Capture = if whole {let v:Revocation=parse(payload)?;Capture {作業領域ID:v.作業領域ID,登録hash:v.登録hash,相対paths:vec![]}} else {parse(payload)?};
            let entry=self.entries.get_mut(&p.作業領域ID).ok_or("作業領域が未登録")?;
            let grant=entry.grant.as_ref().filter(|g| now < g.expires && Instant::now() < g.deadline && g.visibility == "full").ok_or("基準点保存には現在のfull読取承認が必要")?;
            if p.登録hash != entry.registration_hash || (!whole && p.相対paths.is_empty()) || p.相対paths.len() > 128 {return Err("基準点の登録指定または範囲が不正");}
            let mut paths=std::collections::BTreeSet::new();
            for path in &p.相対paths {
                entry.reader.validate_relative_path(path,false).map_err(|_| "基準点pathが不正または除外対象")?;
                if !paths.insert(path.clone()) {return Err("基準点pathが重複");}
            }
            entry.baseline=None;
            audit("ownerの現在承認と基準点取得範囲を照合", &digest(payload))?;
            let inventory=if whole {Some(entry.reader.comparison_inventory().map_err(|_| "全体探索が不成立")?)} else {None};
            if let Some(v)=&inventory {paths.extend(v.files.iter().cloned());}
            let mut retained=0usize;
            let mut files=BTreeMap::new();
            let mut remaining=crate::workspace_reader::MAX_COMPARISON_BYTES;
            for path in paths {
                if Instant::now() >= grant.deadline {return Err("基準点取得中に承認期限超過");}
                let mut bytes=entry.reader.read_comparison_version(&path,remaining).map_err(|_| "基準点fileの取得拒否または上限超過")?;
                remaining-=bytes.as_ref().map(|v|v.bytes as u64).unwrap_or(0);
                if whole && bytes.is_none() {return Err("全体基準点の取得中にfileが消失");}
                if let Some(v)=bytes.as_mut() {
                    let size=v.content.as_ref().map(|b|b.len()).unwrap_or(0);
                    if retained+size>8*1024*1024 {v.content=None;} else {retained+=size;}
                }
                files.insert(path,bytes);
            }
            if let Some(before)=&inventory {
                if &entry.reader.comparison_inventory().map_err(|_| "基準点取得後の探索が不成立")? != before {return Err("全体基準点の探索範囲が変化");}
            }
            let metadata:BTreeMap<_,_>=files.iter().map(|(path,bytes)|(path,bytes.as_ref().map(|v|json!({"bytes":v.bytes,"sha256":v.sha256})))).collect();
            let nonce=識別子生成().map_err(|_| "基準点識別子を生成できない")?;
            let hash=digest(&json!({"version":1,"登録hash":entry.registration_hash,"nonce":nonce,"作成時刻":now,"whole":whole,"excluded_secrets":inventory.as_ref().map(|v|v.excluded_secrets),"files":metadata}));
            let body=json!({"version":1,"operation":operation,"要求hash":digest(payload),"作業領域ID":p.作業領域ID,
                "実行系ID":entry.runtime,"登録hash":entry.registration_hash,"approval_id":grant.id,"有効期限":grant.expires,
                "表示範囲":grant.visibility,"projection":{"基準点hash":hash,"対象数":files.len()}});
            audit("指定範囲の基準点を確定", &digest(&body))?;
            if Instant::now() >= grant.deadline {return Err("基準点監査中に承認期限超過");}
            entry.baseline=Some(Baseline {whole,hash,files});
            return Ok(body);
        }
        if !matches!(operation, "作業領域ツリー" | "作業領域読取" | "作業領域差分" | "作業領域復旧プレビュー" | "作業領域比較範囲" | "作業領域変更一覧") {return Err("作業領域操作が不正");}
        let (p,baseline_hash) = if matches!(operation,"作業領域差分" | "作業領域復旧プレビュー" | "作業領域変更一覧") {
            let value:DiffSelection=parse(payload)?;
            (Selection {作業領域ID:value.作業領域ID,相対path:value.相対path},Some(value.基準点hash))
        } else {(parse::<Selection>(payload)?,None)};
        let entry = self.entries.get(&p.作業領域ID).ok_or("作業領域が未登録")?;
        let grant = entry.grant.as_ref().filter(|g| now < g.expires).ok_or("現在の作業領域読取承認が必要")?;
        if matches!(operation,"作業領域比較範囲" | "作業領域変更一覧") && !p.相対path.is_empty() {return Err("比較範囲にpath指定は不要");}
        entry.reader.validate_relative_path(&p.相対path, matches!(operation, "作業領域ツリー" | "作業領域比較範囲" | "作業領域変更一覧")).map_err(|_| "相対pathが不正または除外対象")?;
        let baseline=if let Some(hash)=baseline_hash.as_ref() {
            let baseline=entry.baseline.as_ref().ok_or("現在登録の基準点が必要")?;
            if hash != &baseline.hash || (!baseline.whole && !baseline.files.contains_key(&p.相対path)) {return Err("基準点hashまたは指定範囲が不一致");}
            if operation=="作業領域変更一覧" && !baseline.whole {return Err("全体基準点が必要");}
            Some(baseline)
        } else {None};
        let binding = json!({"作業領域ID":p.作業領域ID,"実行系ID":entry.runtime,"登録hash":entry.registration_hash,
            "approval_id":grant.id,"permission_id":format!("workspace.inspect.{}",grant.id),
            "capability_id":"workspace.inspect","recovery_id":"workspace.reapprove","要求":payload});
        audit("現在の読取PermissionとApprovalを照合", &digest(&binding))?;
        // 内容を許可しないprojectionでは、filesystemへの不要な取得も行わない。
        let projection = match grant.visibility.as_str() {
            "none" => Value::Null,
            "summary" | "redacted" => json!({"説明":"この表示範囲に提供できる承認済み内容はありません"}),
            "full" | "hash_only" => {
                let data = if operation == "作業領域比較範囲" {
                    json!({"基準点hash":entry.baseline.as_ref().map(|v|v.hash.as_str()),"相対paths":entry.baseline.as_ref().map(|v|v.files.keys().collect::<Vec<_>>()).unwrap_or_default()})
                } else if operation=="作業領域変更一覧" {
                    let baseline=baseline.ok_or("全体基準点が必要")?;
                    let inventory=entry.reader.comparison_inventory().map_err(|_| "変更探索が不成立")?;
                    let mut paths:std::collections::BTreeSet<_>=inventory.files.iter().cloned().collect();
                    paths.extend(baseline.files.keys().cloned());
                    let mut remaining=crate::workspace_reader::MAX_COMPARISON_BYTES;
                    let mut changes=Vec::new();let mut unchanged=0;
                    for path in paths {
                        if Instant::now()>=grant.deadline {return Err("変更探索中に承認期限超過");}
                        let after=entry.reader.read_comparison_version(&path,remaining).map_err(|_| "変更対象の取得拒否")?;
                        remaining-=after.as_ref().map(|v|v.bytes as u64).unwrap_or(0);
                        let before=baseline.files.get(&path).and_then(|v|v.as_ref());
                        let state=match (before,after.as_ref()) {
                            (None,None)=>return Err("探索中に新規fileが消失"),
                            (None,Some(_))=>Some("added"),
                            (Some(_),None)=>Some("deleted"),
                            (Some(a),Some(b)) if a.bytes!=b.bytes || a.sha256!=b.sha256=>Some("modified"),
                            _=>None,
                        };
                        if let Some(status)=state {changes.push(json!({"path":path,"status":status}));} else {unchanged+=1;}
                    }
                    if entry.reader.comparison_inventory().map_err(|_| "変更取得後の探索が不成立")? != inventory {return Err("変更取得中に探索範囲が変化");}
                    json!({"基準点hash":baseline.hash,"changes":changes,"unchanged":unchanged,"excluded_secrets":inventory.excluded_secrets})
                } else if let Some(baseline)=baseline {
                    let after=entry.reader.read_comparison_version(&p.相対path,crate::workspace_reader::MAX_COMPARISON_BYTES).map_err(|_| "比較fileの取得拒否または上限超過")?;
                    let before=baseline.files.get(&p.相対path).and_then(|v|v.as_ref());
                    if operation=="作業領域復旧プレビュー" {
                        let diff=crate::workspace_diff::generate_versions(after.as_ref(),before);
                        let action=if diff.kind==crate::workspace_diff::DiffKind::Unchanged {"none"}
                            else if before.is_none() {"remove"} else if after.is_none() {"recreate"} else {"replace"};
                        json!({"基準点hash":baseline.hash,"diff":diff,"action":action,
                            "baseline_content_available":before.is_none_or(|v|v.content.is_some()),"execution_permitted":false})
                    } else {json!({"基準点hash":baseline.hash,"diff":crate::workspace_diff::generate_versions(before,after.as_ref())})}
                } else if operation == "作業領域ツリー" {
                    let items = entry.reader.list(&p.相対path).map_err(|_| "作業領域を安全に列挙できない")?;
                    json!({"entries":items.into_iter().map(|v|json!({"path":v.path,"kind":match v.kind {EntryKind::File=>"file",EntryKind::Directory=>"directory"},"bytes":v.bytes})).collect::<Vec<_>>()})
                } else {
                    let bytes = entry.reader.read(&p.相対path).map_err(|_| "作業領域fileを安全に読めない")?;
                    let text = std::str::from_utf8(&bytes).ok().filter(|_| !bytes.contains(&0));
                    json!({"path":p.相対path,"bytes":bytes.len(),"sha256":sha256_tagged(&bytes),"binary":text.is_none(),"text":text})
                };
                if grant.visibility == "full" {data} else {json!({"sha256":digest(&data)})}
            }
            _ => return Err("現在の表示範囲が不正"),
        };
        let body = json!({"version":1,"operation":operation,"要求hash":digest(payload),
            "作業領域ID":p.作業領域ID,"実行系ID":entry.runtime,"登録hash":entry.registration_hash,
            "有効期限":grant.expires,"表示範囲":grant.visibility,"approval_id":grant.id,"projection":projection});
        audit("作業領域の取得projectionを確定", &digest(&body))?;
        if Instant::now() >= grant.deadline {return Err("取得中に読取承認が期限超過");}
        Ok(body)
    }
}
