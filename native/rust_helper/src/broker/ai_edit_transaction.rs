//! 審査で固定された版を適用し、処理途中の停止から復旧する。
use super::{ai_edit_center::ApplyFile, store::BrokerPersistentStore};
use crate::workspace_reader::{EditFileVersion, WorkspaceReader, MAX_COMPARISON_BYTES};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum EditStatus {
    Applying,
    Applied,
    Reverting,
    Reverted,
    RecoveryRequired,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct EditRecord {
    version: u8,
    plan_hash: String,
    target: (u64, u64),
    vault: (u64, u64),
    files: Vec<EditEntry>,
    status: EditStatus,
    #[serde(default)]
    apply_request: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct EditEntry {
    path: String,
    before: Option<EditFileVersion>,
    after: Option<EditFileVersion>,
}

pub(super) struct EditCandidateRecord {
    record: EditRecord,
    pub(super) plan_hash: String,
    pub(super) target: (u64, u64),
    pub(super) vault: (u64, u64),
    pub(super) apply_request: Option<serde_json::Value>,
    pub(super) status: EditStatus,
    pub(super) files: Vec<EditCandidateFile>,
}

pub(super) struct EditCandidateFile {
    pub(super) path: String,
    pub(super) before_hash: Option<String>,
    pub(super) after_hash: Option<String>,
}

/// Brokerが現在認可した操作と、隔離済みの同一volume退避先を渡す。
pub(super) struct EditTransaction<'a> {
    pub(super) store: &'a BrokerPersistentStore,
    pub(super) target: &'a WorkspaceReader,
    pub(super) vault: &'a WorkspaceReader,
}

impl EditTransaction<'_> {
    #[cfg(test)]
    pub(super) fn apply(
        &self,
        id: &str,
        plan_hash: &str,
        files: &[ApplyFile],
        source: &WorkspaceReader,
        current: impl Fn() -> bool,
    ) -> Result<EditStatus, &'static str> {
        self.apply_with_request(id, plan_hash, files, source, None, current)
    }

    pub(super) fn apply_with_request(
        &self,
        id: &str,
        plan_hash: &str,
        files: &[ApplyFile],
        source: &WorkspaceReader,
        apply_request: Option<&serde_json::Value>,
        current: impl Fn() -> bool,
    ) -> Result<EditStatus, &'static str> {
        if self.store.read_workspace_edit_record(id)?.is_some() {
            return Err("編集復旧記録が存在するため同じ適用を再実行できない");
        }
        if !valid_hash(plan_hash) || files.is_empty() || files.len() > 32 || !current() {
            return Err("編集適用の審査結合、対象件数または承認期限が不正");
        }
        if apply_request.is_some_and(|request| {
            super::protocol::canonical_payload_hash(Some(request)) != plan_hash
        }) {
            return Err("編集Journalへ保存する元要求hashが不一致");
        }
        let (target, vault) = self.roots()?;
        if source
            .edit_root_identity()
            .ok()
            .is_none_or(|identity| identity == target || identity == vault)
        {
            return Err("審査元、適用先と退避先は別の実体を必要とする");
        }
        let mut seen = BTreeSet::new();
        let mut before_budget = MAX_COMPARISON_BYTES;
        let mut after_budget = MAX_COMPARISON_BYTES;
        let mut entries = Vec::with_capacity(files.len());
        for (index, file) in files.iter().enumerate() {
            if !current()
                || !seen.insert(file.path.to_lowercase())
                || file.before_hash == file.after_hash
                || [file.before_hash.as_deref(), file.after_hash.as_deref()]
                    .into_iter()
                    .flatten()
                    .any(|hash| !valid_hash(hash))
            {
                return Err("編集対象の版、重複または承認期限が不正");
            }
            let before = self
                .target
                .edit_file_version(&file.path, before_budget)
                .map_err(|_| "適用前の版を安全に読めない")?;
            if before.as_ref().map(|(version, _)| version.hash.as_str())
                != file.before_hash.as_deref()
            {
                return Err("適用前の版が審査時のhashと不一致");
            }
            if let Some((_, bytes)) = &before {
                before_budget -= bytes;
            }
            let after = match &file.after_hash {
                Some(hash) => {
                    let (version, bytes) = source
                        .stage_edit_version(
                            &file.path,
                            hash,
                            self.vault,
                            &new_slot(index),
                            after_budget,
                        )
                        .map_err(|_| "審査済みの変更後版を退避先へ準備できない")?;
                    after_budget -= bytes;
                    Some(version)
                }
                None if version_at(source, &file.path)?.is_none() => None,
                None => return Err("削除対象が審査元に再作成された"),
            };
            entries.push(EditEntry {
                path: file.path.clone(),
                before: before.map(|(version, _)| version),
                after,
            });
        }
        if !current() {
            return Err("編集適用の着手前に承認期限を超過");
        }
        let mut record = EditRecord {
            version: if apply_request.is_some() { 2 } else { 1 },
            plan_hash: plan_hash.into(),
            target,
            vault,
            files: entries,
            status: EditStatus::Applying,
            apply_request: apply_request.cloned(),
        };
        self.save(id, &record, true)?;
        let ran = record
            .files
            .iter()
            .enumerate()
            .try_for_each(|(index, entry)| {
                if !current() {
                    return Err("編集中に承認期限を超過");
                }
                self.apply_file(index, entry, &current)
            });
        if ran.is_ok() && current() && self.verify_applied(&record).is_ok() && current() {
            record.status = EditStatus::Applied;
            if self.save(id, &record, false).is_ok() {
                return Ok(EditStatus::Applied);
            }
            // 保存の失敗が置換前か後かを断定できない。補償の着手を先に耐久記録する。
            record.status = EditStatus::Reverting;
            self.save(id, &record, false)?;
        }
        // 最初の記録から中断位置を再観測できる。途中で保存が止まっても元の版を保持する。
        self.revert_files(id, &mut record, &|| true)
    }

    pub(super) fn rollback(
        &self,
        id: &str,
        plan_hash: &str,
        current: impl Fn() -> bool,
    ) -> Result<EditStatus, &'static str> {
        let mut record = self.load(id, plan_hash)?;
        if !current() {
            return Err("ロールバックに現在の独立承認が必要");
        }
        match record.status {
            EditStatus::Applied => self.verify_applied(&record)?,
            EditStatus::Reverting | EditStatus::RecoveryRequired => {}
            EditStatus::Applying | EditStatus::Reverted => {
                return Err("ロールバックに適用済みまたは回復可能な記録が必要");
            }
        }
        if !current() {
            return Err("ロールバックの着手前に承認期限を超過");
        }
        if record.status == EditStatus::Applied {
            record.status = EditStatus::Reverting;
            self.save(id, &record, false)?;
        }
        self.revert_files(id, &mut record, &current)
    }

    /// 既に着手した操作だけを補償する。確定済みの適用は独立承認を待つ。
    pub(super) fn recover(&self, id: &str, plan_hash: &str) -> Result<EditStatus, &'static str> {
        let mut record = self.load(id, plan_hash)?;
        match record.status {
            EditStatus::Applying | EditStatus::Reverting | EditStatus::RecoveryRequired => {
                self.revert_files(id, &mut record, &|| true)
            }
            status => Ok(status),
        }
    }

    /// HMACで認証された既存Journal自身の計画hashへ束縛して回復する。
    /// 再発行された現在の操作hashを旧Journalへ誤って流用しない。
    pub(super) fn recover_existing(&self, id: &str) -> Result<EditStatus, &'static str> {
        let value = self
            .store
            .read_workspace_edit_record(id)?
            .ok_or("編集復旧記録が存在しない")?;
        let record: EditRecord =
            serde_json::from_value(value).map_err(|_| "編集復旧記録の形式が不正")?;
        if !matches!(record.version, 1 | 2)
            || (record.version == 2 && record.apply_request.is_none())
            || !valid_hash(&record.plan_hash)
            || self.roots()? != (record.target, record.vault)
        {
            return Err("編集復旧記録の適用先または退避先が不一致");
        }
        self.recover(id, &record.plan_hash)
    }

    fn apply_file(
        &self,
        index: usize,
        entry: &EditEntry,
        current: &impl Fn() -> bool,
    ) -> Result<(), &'static str> {
        if version_at(self.target, &entry.path)? != entry.before {
            return Err("適用直前に対象版が変更された");
        }
        if let Some(before) = &entry.before {
            self.target
                .move_planned_edit_version(
                    &entry.path,
                    before,
                    self.vault,
                    &old_slot(index),
                    current,
                )
                .map_err(|_| "適用前版の退避結果を確定できない")?;
        }
        if let Some(after) = &entry.after {
            self.vault
                .move_planned_edit_version(
                    &new_slot(index),
                    after,
                    self.target,
                    &entry.path,
                    current,
                )
                .map_err(|_| "適用後版の設置結果を確定できない")?;
        }
        if version_at(self.target, &entry.path)? != entry.after {
            return Err("適用後の対象版が不一致");
        }
        Ok(())
    }

    fn verify_applied(&self, record: &EditRecord) -> Result<(), &'static str> {
        let mut target_budget = MAX_COMPARISON_BYTES;
        let mut vault_budget = MAX_COMPARISON_BYTES;
        for (index, entry) in record.files.iter().enumerate() {
            let (target_version, target_bytes) = self
                .target
                .edit_file_version(&entry.path, target_budget)
                .map_err(|_| "適用後の対象版を確認できない")?
                .map_or((None, 0), |(version, bytes)| (Some(version), bytes));
            target_budget = target_budget
                .checked_sub(target_bytes)
                .ok_or("適用後の対象版が検証上限を超過")?;
            let (old_version, old_bytes) = self
                .vault
                .edit_file_version(&old_slot(index), vault_budget)
                .map_err(|_| "適用前の退避版を確認できない")?
                .map_or((None, 0), |(version, bytes)| (Some(version), bytes));
            vault_budget = vault_budget
                .checked_sub(old_bytes)
                .ok_or("適用前の退避版が検証上限を超過")?;
            let new_present = self
                .vault
                .edit_file_version(&new_slot(index), 0)
                .map_err(|_| "未設置の変更後退避版を確認できない")?
                .is_some();
            let reverted_present = self
                .vault
                .edit_file_version(&reverted_slot(index), 0)
                .map_err(|_| "未使用のRollback退避版を確認できない")?
                .is_some();
            if target_version != entry.after
                || old_version != entry.before
                || new_present
                || reverted_present
            {
                return Err("適用後の対象版または退避版が変更された");
            }
        }
        Ok(())
    }

    fn revert_files(
        &self,
        id: &str,
        record: &mut EditRecord,
        current: &impl Fn() -> bool,
    ) -> Result<EditStatus, &'static str> {
        let mut complete = true;
        for (index, entry) in record.files.iter().enumerate().rev() {
            if !current() {
                complete = false;
                break;
            }
            if self.revert_file(index, entry, current).is_err() {
                complete = false;
                if !current() {
                    break;
                }
            }
        }
        record.status = if complete {
            EditStatus::Reverted
        } else {
            EditStatus::RecoveryRequired
        };
        self.save(id, record, false)?;
        Ok(record.status)
    }

    fn revert_file(
        &self,
        index: usize,
        entry: &EditEntry,
        current: &impl Fn() -> bool,
    ) -> Result<(), &'static str> {
        if !current() {
            return Err("ロールバックの現在承認が失効");
        }
        let actual = version_at(self.target, &entry.path)?;
        let backup = version_at(self.vault, &old_slot(index))?;
        if actual == entry.before && backup.is_none() {
            return Ok(());
        }
        if backup != entry.before || (actual.is_some() && actual != entry.after) {
            return Err("競合した対象版または想定外の退避版を保持");
        }
        if let Some(after) = &actual {
            if version_at(self.vault, &new_slot(index))?.is_some() {
                return Err("設置前と設置後の版が同時に存在");
            }
            self.target
                .move_planned_edit_version(
                    &entry.path,
                    after,
                    self.vault,
                    &reverted_slot(index),
                    current,
                )
                .map_err(|_| "適用後版の退避を確定できない")?;
        }
        if let Some(before) = &backup {
            if !current() {
                return Err("ロールバック中に現在承認が失効");
            }
            self.vault
                .move_planned_edit_version(
                    &old_slot(index),
                    before,
                    self.target,
                    &entry.path,
                    current,
                )
                .map_err(|_| "適用前版の復元を確定できない")?;
        }
        if version_at(self.target, &entry.path)? != entry.before {
            return Err("復元後の版が不一致");
        }
        Ok(())
    }

    fn roots(&self) -> Result<((u64, u64), (u64, u64)), &'static str> {
        let target = self
            .target
            .edit_root_identity()
            .map_err(|_| "適用先rootが不正")?;
        let vault = self
            .vault
            .edit_root_identity()
            .map_err(|_| "退避先rootが不正")?;
        if target == vault || target.0 != vault.0 {
            return Err("適用先と異なる同一volumeの退避先が必要");
        }
        Ok((target, vault))
    }

    fn load(&self, id: &str, plan_hash: &str) -> Result<EditRecord, &'static str> {
        let value = self
            .store
            .read_workspace_edit_record(id)?
            .ok_or("編集復旧記録が存在しない")?;
        let record: EditRecord =
            serde_json::from_value(value).map_err(|_| "編集復旧記録の形式が不正")?;
        if !matches!(record.version, 1 | 2)
            || (record.version == 2 && record.apply_request.is_none())
            || record.plan_hash != plan_hash
            || self.roots()? != (record.target, record.vault)
        {
            return Err("編集復旧記録の審査結合、適用先または退避先が不一致");
        }
        Ok(record)
    }

    pub(super) fn inspect_stored_record(
        store: &BrokerPersistentStore,
        id: &str,
    ) -> Result<Option<EditCandidateRecord>, &'static str> {
        let Some(value) = store.read_workspace_edit_record(id)? else {
            return Ok(None);
        };
        let record: EditRecord =
            serde_json::from_value(value).map_err(|_| "編集復旧記録の形式が不正")?;
        if !matches!(record.version, 1 | 2)
            || (record.version == 2 && record.apply_request.is_none())
            || !valid_hash(&record.plan_hash)
            || record.target.1 == 0
            || record.vault.1 == 0
            || record.target == record.vault
            || record.target.0 != record.vault.0
            || record.files.is_empty()
            || record.files.len() > 32
        {
            return Err("編集復旧記録の識別、rootまたは対象数が不正");
        }
        let mut seen = BTreeSet::new();
        let mut files = Vec::with_capacity(record.files.len());
        for entry in &record.files {
            if entry.path.is_empty()
                || entry.path.len() > 200
                || !seen.insert(entry.path.to_lowercase())
                || entry.before == entry.after
                || entry
                    .before
                    .iter()
                    .chain(entry.after.iter())
                    .any(|version| !valid_hash(&version.hash) || version.file_id == 0)
            {
                return Err("編集復旧記録のfile bindingが不正");
            }
            files.push(EditCandidateFile {
                path: entry.path.clone(),
                before_hash: entry.before.as_ref().map(|version| version.hash.clone()),
                after_hash: entry.after.as_ref().map(|version| version.hash.clone()),
            });
        }
        Ok(Some(EditCandidateRecord {
            plan_hash: record.plan_hash.clone(),
            target: record.target,
            vault: record.vault,
            apply_request: record.apply_request.clone(),
            status: record.status,
            files,
            record,
        }))
    }

    pub(super) fn candidate_matches(
        &self,
        candidate: &EditCandidateRecord,
        files: &[ApplyFile],
    ) -> Result<bool, &'static str> {
        if self.roots()? != (candidate.target, candidate.vault)
            || files.len() != candidate.record.files.len()
            || files
                .iter()
                .zip(&candidate.record.files)
                .any(|(request, entry)| {
                    request.path != entry.path
                        || request.before_hash.as_deref()
                            != entry.before.as_ref().map(|version| version.hash.as_str())
                        || request.after_hash.as_deref()
                            != entry.after.as_ref().map(|version| version.hash.as_str())
                })
        {
            return Err("編集Journalと元適用要求のfile bindingが不一致");
        }
        match candidate.status {
            EditStatus::Applied => Ok(self.verify_applied(&candidate.record).is_ok()),
            EditStatus::Reverting | EditStatus::RecoveryRequired => Ok(true),
            EditStatus::Applying | EditStatus::Reverted => Ok(false),
        }
    }

    fn save(&self, id: &str, record: &EditRecord, initialize: bool) -> Result<(), &'static str> {
        let value = serde_json::to_value(record).map_err(|_| "編集復旧記録を変換できない")?;
        self.store
            .write_workspace_edit_record(id, &value, initialize)
    }
}

fn version_at(
    reader: &WorkspaceReader,
    path: &str,
) -> Result<Option<EditFileVersion>, &'static str> {
    reader
        .edit_file_version(path, MAX_COMPARISON_BYTES)
        .map(|result| result.map(|(version, _)| version))
        .map_err(|_| "対象版を再観測できない")
}

fn valid_hash(hash: &str) -> bool {
    hash.strip_prefix("sha256:").is_some_and(|value| {
        value.len() == 64
            && value
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    })
}

fn new_slot(index: usize) -> String {
    format!("new-{index}")
}

fn old_slot(index: usize) -> String {
    format!("old-{index}")
}

fn reverted_slot(index: usize) -> String {
    format!("reverted-{index}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit_hash::sha256_tagged;
    use cap_std::{ambient_authority, fs::Dir};
    use std::{fs, path::PathBuf};

    const ID: &str = "0000000000000000000000000000000000000000000000000000000000000001";

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let mut nonce = [0u8; 16];
            getrandom::getrandom(&mut nonce).unwrap();
            let root = std::env::temp_dir().join(format!("d4p-edit-test-{}", hex::encode(nonce)));
            fs::create_dir(&root).unwrap();
            for name in ["source", "target", "vault"] {
                fs::create_dir(root.join(name)).unwrap();
            }
            Self(root)
        }

        fn store(&self) -> BrokerPersistentStore {
            BrokerPersistentStore::open_or_create(self.0.join("store"), "edit-fixture")
                .unwrap()
                .0
        }

        fn reader(&self, name: &str) -> WorkspaceReader {
            WorkspaceReader::from_registered_dir(
                Dir::open_ambient_dir(self.0.join(name), ambient_authority()).unwrap(),
                &[],
            )
            .unwrap()
        }

        fn write(&self, path: &str, bytes: &[u8]) {
            fs::write(self.0.join(path), bytes).unwrap();
        }

        fn read(&self, path: &str) -> Vec<u8> {
            fs::read(self.0.join(path)).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn file(path: &str, before: Option<&[u8]>, after: Option<&[u8]>) -> ApplyFile {
        ApplyFile {
            path: path.into(),
            before_hash: before.map(sha256_tagged),
            after_hash: after.map(sha256_tagged),
        }
    }

    #[test]
    fn edit_transaction_applies_and_reverts_add_replace_delete_after_store_reopen() {
        let fixture = Fixture::new();
        let large = vec![b'x'; 70_000];
        fixture.write("target/replace", b"old");
        fixture.write("source/replace", &large);
        fixture.write("source/add", b"add");
        fixture.write("target/delete", b"delete");
        let target = fixture.reader("target");
        let vault = fixture.reader("vault");
        let source = fixture.reader("source");
        let plan = sha256_tagged(b"fixture plan");
        let files = [
            file("replace", Some(b"old"), Some(&large)),
            file("add", None, Some(b"add")),
            file("delete", Some(b"delete"), None),
        ];
        {
            let store = fixture.store();
            let transaction = EditTransaction {
                store: &store,
                target: &target,
                vault: &vault,
            };
            assert_eq!(
                transaction.apply(ID, &plan, &files, &source, || true),
                Ok(EditStatus::Applied)
            );
            assert!(transaction
                .apply(ID, &plan, &files, &source, || true)
                .is_err());
        }
        assert_eq!(fixture.read("target/replace"), large);
        assert_eq!(fixture.read("target/add"), b"add");
        assert!(!fixture.0.join("target/delete").exists());
        assert_eq!(fixture.read("source/replace"), large);

        let store = fixture.store();
        let transaction = EditTransaction {
            store: &store,
            target: &target,
            vault: &vault,
        };
        assert_eq!(transaction.recover(ID, &plan), Ok(EditStatus::Applied));
        assert!(transaction.rollback(ID, &plan, || false).is_err());
        assert!(transaction
            .rollback(ID, &sha256_tagged(b"other plan"), || true)
            .is_err());
        assert_eq!(
            transaction.rollback(ID, &plan, || true),
            Ok(EditStatus::Reverted)
        );
        assert_eq!(fixture.read("target/replace"), b"old");
        assert_eq!(fixture.read("target/delete"), b"delete");
        assert!(!fixture.0.join("target/add").exists());
        assert_eq!(fixture.read("vault/reverted-0"), large);
        assert!(transaction.rollback(ID, &plan, || true).is_err());
    }

    #[test]
    fn edit_transaction_compensates_completed_files_and_preserves_concurrent_edits() {
        let fixture = Fixture::new();
        for name in ["first", "second"] {
            fixture.write(&format!("target/{name}"), b"old");
            fixture.write(&format!("source/{name}"), b"new");
        }
        let store = fixture.store();
        let target = fixture.reader("target");
        let vault = fixture.reader("vault");
        let transaction = EditTransaction {
            store: &store,
            target: &target,
            vault: &vault,
        };
        let plan = sha256_tagged(b"fixture plan");
        let files = [
            file("first", Some(b"old"), Some(b"new")),
            file("second", Some(b"old"), Some(b"new")),
        ];
        let changed = std::cell::Cell::new(false);
        let result = transaction.apply(ID, &plan, &files, &fixture.reader("source"), || {
            if !changed.get()
                && fs::read(fixture.0.join("target/first")).ok().as_deref() == Some(b"new")
            {
                fixture.write("target/second", b"other writer");
                changed.set(true);
            }
            true
        });
        assert_eq!(result, Ok(EditStatus::RecoveryRequired));
        assert_eq!(fixture.read("target/first"), b"old");
        assert_eq!(fixture.read("target/second"), b"other writer");
        assert_eq!(fixture.read("vault/reverted-0"), b"new");
        assert_eq!(
            transaction.recover(ID, &plan),
            Ok(EditStatus::RecoveryRequired)
        );
        assert_eq!(fixture.read("target/second"), b"other writer");
    }

    #[test]
    fn edit_transaction_recovers_interrupted_apply_and_refuses_a_tampered_record() {
        let fixture = Fixture::new();
        for name in ["first", "second"] {
            fixture.write(&format!("target/{name}"), b"old");
            fixture.write(&format!("source/{name}"), b"new");
        }
        let target = fixture.reader("target");
        let vault = fixture.reader("vault");
        let plan = sha256_tagged(b"fixture plan");
        let files = [
            file("first", Some(b"old"), Some(b"new")),
            file("second", Some(b"old"), Some(b"new")),
        ];
        {
            let store = fixture.store();
            let transaction = EditTransaction {
                store: &store,
                target: &target,
                vault: &vault,
            };
            let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _result =
                    transaction.apply(ID, &plan, &files, &fixture.reader("source"), || {
                        assert_ne!(
                            fs::read(fixture.0.join("target/first")).ok().as_deref(),
                            Some(b"new".as_slice()),
                            "試験用の適用中断",
                        );
                        true
                    });
            }));
            assert!(interrupted.is_err());
            assert_eq!(
                transaction.load(ID, &plan).unwrap().status,
                EditStatus::Applying
            );
        }
        let store = fixture.store();
        let transaction = EditTransaction {
            store: &store,
            target: &target,
            vault: &vault,
        };
        let path = fixture.0.join(format!("store/workspace-edit-{ID}.json"));
        let recorded = fs::read_to_string(&path).unwrap();
        fs::write(&path, recorded.replace("applying", "applied")).unwrap();
        assert!(transaction.recover(ID, &plan).is_err());
        assert_eq!(fixture.read("target/first"), b"new");
        fs::write(&path, recorded).unwrap();
        assert_eq!(transaction.recover(ID, &plan), Ok(EditStatus::Reverted));
        assert_eq!(fixture.read("target/first"), b"old");
        assert_eq!(fixture.read("target/second"), b"old");
    }

    #[test]
    fn edit_transaction_refuses_to_modify_workspace_without_durable_record() {
        let fixture = Fixture::new();
        fixture.write("target/file", b"old");
        fixture.write("source/file", b"new");
        let store = fixture.store();
        let target = fixture.reader("target");
        let vault = fixture.reader("vault");
        let transaction = EditTransaction {
            store: &store,
            target: &target,
            vault: &vault,
        };
        fs::rename(fixture.0.join("store"), fixture.0.join("blocked-store")).unwrap();
        let plan = sha256_tagged(b"fixture plan");
        let files = [file("file", Some(b"old"), Some(b"new"))];
        assert!(transaction
            .apply(ID, &plan, &files, &fixture.reader("source"), || true)
            .is_err());
        assert_eq!(fixture.read("target/file"), b"old");
        assert!(!fixture.0.join("vault/old-0").exists());
    }

    #[test]
    fn edit_transaction_revoked_between_moves_restores_previous_version() {
        let fixture = Fixture::new();
        fixture.write("target/file", b"old");
        fixture.write("source/file", b"new");
        let store = fixture.store();
        let target = fixture.reader("target");
        let vault = fixture.reader("vault");
        let transaction = EditTransaction {
            store: &store,
            target: &target,
            vault: &vault,
        };
        let plan = sha256_tagged(b"fixture plan");
        let files = [file("file", Some(b"old"), Some(b"new"))];
        let result = transaction.apply(ID, &plan, &files, &fixture.reader("source"), || {
            !fixture.0.join("vault/old-0").exists()
        });
        assert_eq!(result, Ok(EditStatus::Reverted));
        assert_eq!(fixture.read("target/file"), b"old");
        assert_eq!(fixture.read("vault/new-0"), b"new");
        assert!(!fixture.0.join("vault/reverted-0").exists());
    }
}
