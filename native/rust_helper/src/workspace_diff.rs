//! 作業領域の内部差分。取得権限と表示権限は呼出し元の統治経路が所有する。
use serde::Serialize;

use crate::audit_hash::sha256_tagged;

const MAX_BYTES: usize = 65_536;
const MAX_LINES: usize = 2_000;
const MAX_CELLS: usize = 1_000_000;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct FileVersion {
    pub bytes: usize,
    pub sha256: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    Unchanged,
    Binary,
    Oversized,
    Text,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiffLine {
    pub number: usize,
    pub text: String,
    pub newline: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RowKind {
    Same,
    Added,
    Deleted,
    Changed,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DiffRow {
    pub kind: RowKind,
    pub before: Option<DiffLine>,
    pub after: Option<DiffLine>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct WorkspaceDiff {
    pub version: u8,
    pub kind: DiffKind,
    pub before: Option<FileVersion>,
    pub after: Option<FileVersion>,
    pub unified: Option<String>,
    pub rows: Vec<DiffRow>,
}

fn version(bytes: Option<&[u8]>) -> Option<FileVersion> {
    bytes.map(|value| FileVersion {
        bytes: value.len(),
        sha256: sha256_tagged(value),
    })
}

fn lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').collect()
}

fn row_line(number: usize, text: &str) -> DiffLine {
    DiffLine {
        number,
        text: text.strip_suffix('\n').unwrap_or(text).to_owned(),
        newline: text.ends_with('\n'),
    }
}

fn patch_line(output: &mut String, prefix: char, text: &str) {
    output.push(prefix);
    output.push_str(text);
    if !text.ends_with('\n') {
        output.push_str("\n\\ No newline at end of file\n");
    }
}

fn flush_rows(rows: &mut Vec<DiffRow>, deleted: &mut Vec<DiffLine>, added: &mut Vec<DiffLine>) {
    let count = deleted.len().max(added.len());
    let mut before = deleted.drain(..);
    let mut after = added.drain(..);
    for _ in 0..count {
        let left = before.next();
        let right = after.next();
        let kind = match (&left, &right) {
            (Some(_), Some(_)) => RowKind::Changed,
            (Some(_), None) => RowKind::Deleted,
            (None, Some(_)) => RowKind::Added,
            (None, None) => unreachable!(),
        };
        rows.push(DiffRow {
            kind,
            before: left,
            after: right,
        });
    }
}

/// 取得器が全fileを検査した比較版。巨大fileの本文を復元しない。
pub fn generate_versions(before: Option<&crate::workspace_reader::ComparedFile>, after: Option<&crate::workspace_reader::ComparedFile>) -> WorkspaceDiff {
    let metadata=|value:Option<&crate::workspace_reader::ComparedFile>|value.map(|v|FileVersion {bytes:v.bytes,sha256:v.sha256.clone()});
    let left=metadata(before);let right=metadata(after);
    if left==right {
        return WorkspaceDiff {version:1,kind:DiffKind::Unchanged,before:left,after:right,unified:None,rows:vec![]};
    }
    if before.is_some_and(|v|v.content.is_none()) || after.is_some_and(|v|v.content.is_none()) {
        return WorkspaceDiff {version:1,kind:DiffKind::Oversized,before:left,after:right,unified:None,rows:vec![]};
    }
    generate(before.and_then(|v|v.content.as_deref()),after.and_then(|v|v.content.as_deref()))
}

/// bytesは取得済みの内部内容。結果をUIへ渡す前に現在の表示範囲を強制する。
/// Noneはfile不在であり、Some(b"")とは異なる。
pub fn generate(before: Option<&[u8]>, after: Option<&[u8]>) -> WorkspaceDiff {
    let mut result = WorkspaceDiff {
        version: 1,
        kind: DiffKind::Unchanged,
        before: version(before),
        after: version(after),
        unified: None,
        rows: Vec::new(),
    };
    if before == after {
        return result;
    }
    let left = before.unwrap_or_default();
    let right = after.unwrap_or_default();
    if left.len() > MAX_BYTES || right.len() > MAX_BYTES {
        result.kind = DiffKind::Oversized;
        return result;
    }
    let (Ok(left_text), Ok(right_text)) = (std::str::from_utf8(left), std::str::from_utf8(right))
    else {
        result.kind = DiffKind::Binary;
        return result;
    };
    if left.contains(&0) || right.contains(&0) {
        result.kind = DiffKind::Binary;
        return result;
    }
    let old = lines(left_text);
    let new = lines(right_text);
    let width = new.len() + 1;
    let cells = (old.len() + 1).saturating_mul(width);
    if old.len() > MAX_LINES || new.len() > MAX_LINES || cells > MAX_CELLS {
        result.kind = DiffKind::Oversized;
        return result;
    }
    // 上限確認済みのLCS表。等長候補では削除を先に選び決定論を保つ。
    let mut lcs = vec![0u16; cells];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            lcs[i * width + j] = if old[i] == new[j] {
                1 + lcs[(i + 1) * width + j + 1]
            } else {
                lcs[(i + 1) * width + j].max(lcs[i * width + j + 1])
            };
        }
    }
    let old_label = if before.is_some() {
        "a/file"
    } else {
        "/dev/null"
    };
    let new_label = if after.is_some() {
        "b/file"
    } else {
        "/dev/null"
    };
    let mut unified = String::from("diff --git a/file b/file\n");
    if before.is_none() {
        unified.push_str("new file mode 100644\n");
    } else if after.is_none() {
        unified.push_str("deleted file mode 100644\n");
    }
    // 空fileの追加・削除はGitの存在差headerで表す。無効な空hunkを作らない。
    if !old.is_empty() || !new.is_empty() {
        unified.push_str(&format!("--- {old_label}\n+++ {new_label}\n"));
        unified.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            usize::from(!old.is_empty()),
            old.len(),
            usize::from(!new.is_empty()),
            new.len()
        ));
    }
    let (mut i, mut j) = (0, 0);
    let mut deleted = Vec::new();
    let mut added = Vec::new();
    while i < old.len() || j < new.len() {
        if i < old.len() && j < new.len() && old[i] == new[j] {
            flush_rows(&mut result.rows, &mut deleted, &mut added);
            patch_line(&mut unified, ' ', old[i]);
            result.rows.push(DiffRow {
                kind: RowKind::Same,
                before: Some(row_line(i + 1, old[i])),
                after: Some(row_line(j + 1, new[j])),
            });
            i += 1;
            j += 1;
        } else if i < old.len()
            && (j == new.len() || lcs[(i + 1) * width + j] >= lcs[i * width + j + 1])
        {
            patch_line(&mut unified, '-', old[i]);
            deleted.push(row_line(i + 1, old[i]));
            i += 1;
        } else {
            patch_line(&mut unified, '+', new[j]);
            added.push(row_line(j + 1, new[j]));
            j += 1;
        }
    }
    flush_rows(&mut result.rows, &mut deleted, &mut added);
    result.kind = DiffKind::Text;
    result.unified = Some(unified);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_preserve_unified_and_paired_rows() {
        let result = generate(
            Some(b"same\nold\nlast\n"),
            Some(b"same\nnew\nextra\nlast\n"),
        );
        assert_eq!(result.kind, DiffKind::Text);
        assert_eq!(
            result.unified.as_deref(),
            Some("diff --git a/file b/file\n--- a/file\n+++ b/file\n@@ -1,3 +1,4 @@\n same\n-old\n+new\n+extra\n last\n")
        );
        assert_eq!(
            result.rows.iter().map(|row| &row.kind).collect::<Vec<_>>(),
            vec![
                &RowKind::Same,
                &RowKind::Changed,
                &RowKind::Added,
                &RowKind::Same
            ]
        );
        assert_eq!(result.rows[3].after.as_ref().unwrap().number, 4);
        assert_eq!(
            result,
            generate(
                Some(b"same\nold\nlast\n"),
                Some(b"same\nnew\nextra\nlast\n")
            )
        );
    }

    #[test]
    fn newline_and_crlf_are_not_normalized_away() {
        let result = generate(Some(b"a\r\nlast"), Some(b"a\nlast\n"));
        assert_eq!(result.rows[0].before.as_ref().unwrap().text, "a\r");
        assert!(!result.rows[1].before.as_ref().unwrap().newline);
        assert!(result.rows[1].after.as_ref().unwrap().newline);
        assert!(result
            .unified
            .unwrap()
            .contains("-last\n\\ No newline at end of file\n"));
    }

    #[test]
    fn absent_and_empty_are_distinct() {
        let added = generate(None, Some(b""));
        assert_eq!(added.kind, DiffKind::Text);
        assert!(added.before.is_none());
        assert_eq!(added.after.unwrap().bytes, 0);
        assert_eq!(generate(Some(b""), Some(b"")).kind, DiffKind::Unchanged);
        let deleted = generate(Some(b"a\n"), None);
        assert_eq!(deleted.rows[0].kind, RowKind::Deleted);
        assert!(deleted
            .unified
            .unwrap()
            .contains("+++ /dev/null\n@@ -1,1 +0,0 @@\n-a\n"));
    }

    #[test]
    fn binary_and_oversized_never_return_content() {
        for bytes in [vec![0, 65], vec![255, 65], vec![65; MAX_BYTES + 1]] {
            let result = generate(None, Some(&bytes));
            assert!(matches!(
                result.kind,
                DiffKind::Binary | DiffKind::Oversized
            ));
            assert!(result.unified.is_none() && result.rows.is_empty());
            assert_eq!(result.after.unwrap().sha256, sha256_tagged(&bytes));
        }
        let too_many_lines = "x\n".repeat(MAX_LINES + 1);
        assert_eq!(
            generate(None, Some(too_many_lines.as_bytes())).kind,
            DiffKind::Oversized
        );
        let large = "x\n".repeat(1000);
        let changed = "y\n".repeat(1000);
        assert_eq!(
            generate(Some(large.as_bytes()), Some(changed.as_bytes())).kind,
            DiffKind::Oversized
        );
        assert_eq!(
            generate(None, Some(&vec![65; MAX_BYTES])).kind,
            DiffKind::Text
        );
    }

    #[test]
    fn side_rows_reconstruct_exact_inputs_with_repeated_unicode_lines() {
        let samples = [
            "",
            "同じ\n同じ\n終端",
            "同じ\n追加\n同じ\n",
            "\r\n\n",
            "終端\n",
        ];
        for before in samples {
            for after in samples {
                let result = generate(Some(before.as_bytes()), Some(after.as_bytes()));
                if before == after {
                    assert_eq!(result.kind, DiffKind::Unchanged);
                    continue;
                }
                let reconstruct = |old| {
                    result
                        .rows
                        .iter()
                        .filter_map(|row| {
                            if old {
                                row.before.as_ref()
                            } else {
                                row.after.as_ref()
                            }
                        })
                        .map(|line| {
                            format!("{}{}", line.text, if line.newline { "\n" } else { "" })
                        })
                        .collect::<String>()
                };
                assert_eq!(reconstruct(true), before);
                assert_eq!(reconstruct(false), after);
            }
        }
    }
}
