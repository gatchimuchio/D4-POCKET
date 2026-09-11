"""release専用。収集済み合格宣言を信用せず、現在fileをRustで再検証する。"""
import json
import os
from pathlib import Path
import subprocess
import hashlib

ROOT = Path(__file__).resolve().parents[1]


def verify_collected(evidence: dict) -> dict:
    inputs = evidence.get("checkpoint_inputs")
    if not isinstance(inputs, dict):
        raise ValueError("署名checkpointの実検証入力がない")
    # 継続性記録は証拠束の自己申告から採用しない。ownerが別に指定する。
    floor = os.environ.get("GUI_SHELL_AUDIT_TRUSTED_HEAD")
    if not floor:
        raise ValueError("owner管理のGUI_SHELL_AUDIT_TRUSTED_HEADが未指定")
    root = Path(inputs["installed_root"]).resolve(strict=True)
    store = Path(inputs["audit_dir"]).resolve(strict=True)
    if not store.is_relative_to(root):
        raise ValueError("監査storeがinstalled root外")
    manifest = json.loads((root / "installed_manifest.json").read_text(encoding="utf-8-sig"))
    source = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, timeout=10).strip()
    if manifest.get("source_commit") != source or manifest.get("source_worktree_clean") is not True:
        raise ValueError("現在source commitとinstalled sourceが不一致")
    artifact = Path(manifest["app_exe"]).resolve(strict=True)
    if not artifact.is_relative_to(root):
        raise ValueError("artifactがinstalled root外")
    with artifact.open("rb") as stream:
        actual_hash = "sha256:" + hashlib.file_digest(stream, "sha256").hexdigest()
    if actual_hash != manifest.get("app_artifact_sha256"):
        raise ValueError("installed artifact hash不一致")
    helper = ROOT / "native/rust_helper/target/debug" / ("gui_shell_rust_helper.exe" if os.name == "nt" else "gui_shell_rust_helper")
    result = subprocess.run([str(helper), "監査チェックポイント", "verify", str(store), str(root), source,
        str(ROOT / "config/audit_signing_trust.json"), floor, inputs["bundle"], inputs.get("previous_bundle", "-")],
        capture_output=True, text=True, encoding="utf-8", timeout=60, cwd=ROOT)
    if result.returncode != 0:
        raise ValueError("Rust署名checkpoint検証失敗: " + result.stderr.strip())
    verified = json.loads(result.stdout)
    if verified.get("status") != "passed" or verified.get("verification_kind") != "offline_ed25519_checkpoint_v2":
        raise ValueError("署名checkpoint実検証結果不正")
    if verified != evidence.get("checkpoint_verification"):
        raise ValueError("Collector結果と現在の再検証が不一致")
    return verified
