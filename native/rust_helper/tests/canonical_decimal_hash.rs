use std::fs;
use std::path::PathBuf;
use std::process::Command;

use gui_shell_rust_helper::broker::BrokerRequestEnvelope;
use serde_json::Value;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository rootを解決")
}

#[test]
fn runtime_resource_observation_decimal_vector_matches_rust_and_python_payload_hash() {
    let root = repository_root();
    let vector_path = root.join("tooling/canonicalization_vectors/runtime_resource_observation_decimal.json");
    let vector: Value = serde_json::from_str(
        &fs::read_to_string(&vector_path).expect("小数正本化vectorを読取"),
    )
    .expect("小数正本化vectorをJSONとして読取");
    let expected = vector["expected_rust_payload_hash"]
        .as_str()
        .expect("固定Rust hash")
        .to_string();
    let body = vector["body"].clone();

    let mut request = BrokerRequestEnvelope::command_envelope(
        "canonical-decimal-vector",
        "canonical-decimal-session",
        "canonical-decimal-nonce",
    );
    request.payload = Some(body);
    request.refresh_payload_hash();
    assert_eq!(request.payload_hash.as_deref(), Some(expected.as_str()));

    let python = if cfg!(windows) { "python" } else { "python3" };
    let script = r#"
import hashlib
import sys
from pathlib import Path

sys.path.insert(0, sys.argv[1])
from tooling.minidora_live_check import 正本JSON読取, 正本化

vector = 正本JSON読取(Path(sys.argv[2]).read_bytes())
encoded = 正本化(vector["body"]).encode("utf-8")
print("sha256:" + hashlib.sha256(encoded).hexdigest())
"#;
    let output = Command::new(python)
        .args(["-c", script])
        .arg(&root)
        .arg(&vector_path)
        .output()
        .expect("Python小数正本化相互検証を起動");
    assert!(
        output.status.success(),
        "Python小数正本化相互検証が失敗: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        expected,
        "Python側のC3小数正本化hashがRust固定値と異なる"
    );
}
