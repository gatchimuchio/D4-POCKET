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
    registration_hash: String,
    reader: WorkspaceReader,
    grant: Option<Grant>,
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
    pub(crate) fn response_current(&self, body: &Value, now: i64) -> bool {
        if self.last_now.is_some_and(|last| now < last) {return false;}
        body["作業領域ID"].as_str().and_then(|id| self.entries.get(id))
            .and_then(|entry| entry.grant.as_ref()).is_some_and(|grant|
                now < grant.expires && Instant::now() < grant.deadline && body["approval_id"] == grant.id)
    }
    /// 信頼済み起動制御面が渡すhandleを保持する。登録だけでは読取許可を作らない。
    pub(crate) fn register(&mut self, runtime: &str, id: &str, root: Dir, secrets: &[String], audit: &mut Audit<'_>) -> Result<(), &'static str> {
        if !identifier(id) || !identifier(runtime) || self.entries.contains_key(id) || self.entries.len() >= 16 {
            return Err("作業領域の登録指定が不正または上限超過");
        }
        let identity=root.dir_metadata().map_err(|_| "作業領域のroot識別子を確認できない")?;
        let reader = WorkspaceReader::from_registered_dir(root, secrets).map_err(|_| "作業領域の除外指定が不正")?;
        let nonce = 識別子生成().map_err(|_| "登録識別子を生成できない")?;
        let registration_hash = digest(&json!({"実行系ID":runtime,"作業領域ID":id,"登録識別子":nonce,"除外":secrets,
            "root_device":cap_fs_ext::MetadataExt::dev(&identity),"root_file_id":cap_fs_ext::MetadataExt::ino(&identity)}));
        audit("作業領域登録・Permission拒否", &registration_hash)?;
        self.entries.insert(id.into(), RegisteredWorkspace {runtime:runtime.into(),registration_hash,reader,grant:None});
        Ok(())
    }

    pub(crate) fn operate(&mut self, operation: &str, payload: &Value, owner: bool, now: i64, audit: &mut Audit<'_>) -> Result<Value, &'static str> {
        if self.last_now.is_some_and(|last| now < last) {
            for entry in self.entries.values_mut() {entry.grant = None;}
            return Err("時計後退を検出したため読取承認を失効");
        }
        self.last_now = Some(now);
        for entry in self.entries.values_mut() {
            if entry.grant.as_ref().is_some_and(|g| now >= g.expires || Instant::now() >= g.deadline) {entry.grant = None;}
        }
        if matches!(operation, "作業領域承認" | "作業領域失効") && !owner {
            return Err("owner制御資格が必要");
        }
        if operation == "作業領域一覧" {
            if !payload.as_object().is_some_and(|v| v.is_empty()) {return Err("作業領域一覧の入力が不正");}
            return Ok(json!({"作業領域":self.entries.iter().map(|(id,e)| {
                let grant = e.grant.as_ref().filter(|g| now < g.expires);
                json!({"作業領域ID":id,"実行系ID":e.runtime,"登録hash":e.registration_hash,
                    "承認状態":if grant.is_some(){"approved"}else{"denied"},"有効期限":grant.map(|g|g.expires),
                    "表示範囲":grant.map(|g|g.visibility.as_str()).unwrap_or("none")})
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
            audit("作業領域PermissionとApprovalを失効", &digest(payload))?;
            return Ok(json!({"作業領域ID":p.作業領域ID,"承認状態":"revoked"}));
        }
        if !matches!(operation, "作業領域ツリー" | "作業領域読取") {return Err("作業領域操作が不正");}
        let p: Selection = parse(payload)?;
        let entry = self.entries.get(&p.作業領域ID).ok_or("作業領域が未登録")?;
        let grant = entry.grant.as_ref().filter(|g| now < g.expires).ok_or("現在の作業領域読取承認が必要")?;
        entry.reader.validate_relative_path(&p.相対path, operation == "作業領域ツリー").map_err(|_| "相対pathが不正または除外対象")?;
        let binding = json!({"作業領域ID":p.作業領域ID,"実行系ID":entry.runtime,"登録hash":entry.registration_hash,
            "approval_id":grant.id,"permission_id":format!("workspace.inspect.{}",grant.id),
            "capability_id":"workspace.inspect","recovery_id":"workspace.reapprove","要求":payload});
        audit("現在の読取PermissionとApprovalを照合", &digest(&binding))?;
        // 内容を許可しないprojectionでは、filesystemへの不要な取得も行わない。
        let projection = match grant.visibility.as_str() {
            "none" => Value::Null,
            "summary" | "redacted" => json!({"説明":"この表示範囲に提供できる承認済み内容はありません"}),
            "full" | "hash_only" => {
                let data = if operation == "作業領域ツリー" {
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
        let body = json!({"作業領域ID":p.作業領域ID,"表示範囲":grant.visibility,"approval_id":grant.id,"projection":projection});
        audit("作業領域の取得projectionを確定", &digest(&body))?;
        if Instant::now() >= grant.deadline {return Err("取得中に読取承認が期限超過");}
        Ok(body)
    }
}
