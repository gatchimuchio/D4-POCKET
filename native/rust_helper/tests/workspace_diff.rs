use gui_shell_rust_helper::workspace_diff::generate;
use std::{fs, path::PathBuf, process::Command};

struct TestDirectory(PathBuf);
impl TestDirectory {
    fn new() -> Self {
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).unwrap();
        let path =
            std::env::temp_dir().join(format!("gui-shell-diff-test-{}", hex::encode(random)));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn git_consumes_generated_patch_and_reverse_restores_original() {
    let samples: &[Option<&[u8]>] = &[
        None,
        Some(b""),
        Some(b"same\nold\nlast"),
        Some(b"same\nnew\nextra\nlast\n"),
        Some("日本語\r\n\n終端".as_bytes()),
    ];
    for before in samples {
        for after in samples {
            if before == after {
                continue;
            }
            let directory = TestDirectory::new();
            if let Some(bytes) = before {
                fs::write(directory.0.join("file"), bytes).unwrap();
            }
            let diff = generate(*before, *after);
            fs::write(directory.0.join("change.patch"), diff.unified.unwrap()).unwrap();
            // Gitを独立consumerとして、正方向と逆方向の双方を実適用する。
            for (reverse, expected) in [(false, after), (true, before)] {
                let mut command = Command::new("git");
                command
                    .current_dir(&directory.0)
                    // byte完全一致を検査するため、hostの改行自動変換を適用しない。
                    .args(["-c", "core.autocrlf=false", "apply", "--whitespace=nowarn"]);
                if reverse {
                    command.arg("--reverse");
                }
                let output = command
                    .arg("change.patch")
                    .output()
                    .expect("Gitの実適用検証が必要");
                assert!(
                    output.status.success(),
                    "変更前={before:?} 変更後={after:?} 逆適用={reverse}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                match expected {
                    Some(bytes) => assert_eq!(fs::read(directory.0.join("file")).unwrap(), *bytes),
                    None => assert!(!directory.0.join("file").exists()),
                }
            }
        }
    }
}

#[test]
fn actual_serialized_results_match_workspace_schema() {
    let directory = TestDirectory::new();
    let oversized = vec![65; 65_537];
    let results = vec![
        generate(None, Some(b"text\n")),
        generate(Some(b"text\n"), None),
        generate(Some(b"a\n"), Some(b"b\n")),
        generate(Some(b"same"), Some(b"same")),
        generate(None, Some(b"\0binary")),
        generate(None, Some(&oversized)),
        generate(None, Some(b"")),
    ];
    let data = directory.0.join("results.json");
    fs::write(&data, serde_json::to_vec(&results).unwrap()).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = "import json,sys; from pathlib import Path; sys.path.insert(0,sys.argv[1]); from tooling.schema_check.check_schemas import validate_instance; schema=json.loads((Path(sys.argv[1])/'specs/workspace_diff.schema.json').read_text(encoding='utf-8')); results=json.loads(Path(sys.argv[2]).read_text(encoding='utf-8')); errors=[e for result in results for e in validate_instance(result,schema)]; assert not errors, errors";
    let output = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .args(["-c", script])
        .arg(root)
        .arg(data)
        .output()
        .expect("既存Schema検証用Pythonが必要");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
