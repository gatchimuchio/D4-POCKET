"""Run an isolated Windows Export-runtime-to-Codex developer verification."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import uuid
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.export_windows_product import (
    RECEIPT_NAME,
    _artifact_inventory,
    validate_build_evidence,
    validate_export_manifest_only_input,
)
from tooling.flutter_workspace import safe_flutter_temp_parent

APP_ID_PATTERN = re.compile(r"d4-pocket-app-[0-9a-f]{32}\Z")
AUDIT_ID_PATTERN = re.compile(r"audit-store-[0-9a-f]{32}\Z")
SOURCE_APP_ID = "d4-pocket-app-11111111111111111111111111111111"
SOURCE_AUDIT_ID = "audit-store-22222222222222222222222222222222"
EXPECTED_CODEX_VERSION = "codex-cli 0.166.0"
TEST_NAME = "exported_windows_runtime_completes_real_codex_task_with_isolated_identity"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def git_text(*arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(ROOT), *arguments],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    return result.stdout.strip()


def _fresh_identity() -> tuple[str, str]:
    suffix = uuid.uuid4().hex
    return f"d4-pocket-app-{suffix}", f"audit-store-{uuid.uuid4().hex}"


def _credential_free_environment() -> dict[str, str]:
    environment = os.environ.copy()
    exact_names = {
        "GH_TOKEN",
        "GITHUB_TOKEN",
        "ACTIONS_RUNTIME_TOKEN",
        "ACTIONS_ID_TOKEN_REQUEST_TOKEN",
        "NODE_AUTH_TOKEN",
        "NPM_TOKEN",
        "CODEX_HOME",
    }
    credential_prefixes = (
        "OPENAI_",
        "ANTHROPIC_",
        "GEMINI_",
        "GOOGLE_",
        "AZURE_OPENAI_",
        "AWS_",
    )
    for name in tuple(environment):
        upper_name = name.upper()
        if upper_name in exact_names or upper_name.startswith(credential_prefixes):
            environment.pop(name, None)
    return environment


def _task_e2e_environment(
    base_environment: dict[str, str],
    *,
    codex_cli: Path,
    runtime_root: Path,
    manifest_path: Path,
    manifest_sha256: str,
    source_commit: str,
    app_id: str,
    audit_store_id: str,
    evidence_path: Path,
) -> dict[str, str]:
    environment = base_environment.copy()
    environment.update(
        {
            "GUI_SHELL_CODEX_TASK_CRASH_E2E_CLI": str(codex_cli),
            "GUI_SHELL_W_E03_EXPORT_RUNTIME_ROOT": str(runtime_root),
            "GUI_SHELL_W_E03_MANIFEST_PATH": str(manifest_path),
            "GUI_SHELL_W_E03_MANIFEST_SHA256": manifest_sha256,
            "GUI_SHELL_W_E03_SOURCE_COMMIT": source_commit,
            "GUI_SHELL_W_E03_APP_ID": app_id,
            "GUI_SHELL_W_E03_AUDIT_STORE_ID": audit_store_id,
            "GUI_SHELL_W_E03_EVIDENCE_PATH": str(evidence_path),
        }
    )
    return environment


def create_manifest_only_input(
    path: Path, *, app_id: str, audit_store_id: str
) -> tuple[bytes, str]:
    if APP_ID_PATTERN.fullmatch(app_id) is None or app_id == SOURCE_APP_ID:
        raise ValueError("Developer Export App ID is invalid or reuses source identity")
    if AUDIT_ID_PATTERN.fullmatch(audit_store_id) is None or audit_store_id == SOURCE_AUDIT_ID:
        raise ValueError("Developer Export Audit Store ID is invalid or reuses source identity")
    if path.exists() or path.is_symlink():
        raise FileExistsError("Refusing to overwrite a Manifest-only input")

    source = ROOT / "examples/contracts/gui_shell_export_manifest.valid.json"
    document = json.loads(source.read_text(encoding="utf-8"))
    manifest = document["manifest"]
    manifest["app_identity"]["app_id"] = app_id
    manifest["audit_store"]["store_id"] = audit_store_id
    raw = (json.dumps(document, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    validate_export_manifest_only_input(
        raw,
        f"{app_id}.json",
        target_platform="windows",
    )
    path.write_bytes(raw)
    if path.read_bytes() != raw:
        raise RuntimeError("Manifest-only input changed after write")
    return raw, hashlib.sha256(raw).hexdigest()


def validate_task_evidence(
    evidence: Any,
    *,
    source_commit: str,
    manifest_sha256: str,
    app_id: str,
    audit_store_id: str,
    expected_helper_relative_path: str,
) -> dict[str, Any]:
    if not isinstance(evidence, dict):
        raise ValueError("W-E03 Rust Task evidence must be a JSON object")
    result_hash = evidence.get("task_result_hash")
    audit_hash = evidence.get("durable_audit_sha256")
    helper_hash = evidence.get("export_runtime_broker_sha256")
    workspace_hash = evidence.get("workspace_marker_sha256")
    audit_event_count = evidence.get("durable_audit_event_count")
    if (
        evidence.get("evidence_type") != "d4_pocket_w_e03_windows_export_runtime_developer"
        or evidence.get("evidence_class") != "LIVE_RUNTIME"
        or evidence.get("source_commit") != source_commit
        or re.fullmatch(r"[0-9a-f]{40}", source_commit) is None
        or evidence.get("manifest_sha256") != manifest_sha256
        or evidence.get("app_id") != app_id
        or evidence.get("audit_store_id") != audit_store_id
        or evidence.get("export_runtime_broker_relative_path") != expected_helper_relative_path
        or evidence.get("real_owner_authority") is not False
        or evidence.get("synthetic_owner_fixture") is not True
        or evidence.get("external_provider_requests") != 0
        or evidence.get("task_status") != "completed"
        or not isinstance(result_hash, str)
        or re.fullmatch(r"sha256:[0-9a-f]{64}", result_hash) is None
        or not isinstance(audit_hash, str)
        or re.fullmatch(r"[0-9a-f]{64}", audit_hash) is None
        or not isinstance(helper_hash, str)
        or re.fullmatch(r"[0-9a-f]{64}", helper_hash) is None
        or not isinstance(workspace_hash, str)
        or re.fullmatch(r"[0-9a-f]{64}", workspace_hash) is None
        or type(audit_event_count) is not int
        or audit_event_count < 1
        or evidence.get("outside_workspace_write") != "denied"
        or evidence.get("secret_value_exposed") is not False
        or evidence.get("approval_replay") != "rejected"
        or evidence.get("scratch_cleanup") != "passed"
        or evidence.get("codex_process_tree_cleanup") != "passed"
        or evidence.get("durable_audit_chain") != "passed"
        or evidence.get("task_body_in_audit") is not False
    ):
        raise ValueError("W-E03 developer Task evidence violates its bounded contract")
    return evidence


def _safe_cleanup(run_root: Path, temp_parent: Path) -> None:
    if not run_root.exists() and not run_root.is_symlink():
        return
    if (
        run_root.name.startswith("d4p-windows-we03-") is False
        or run_root.parent.resolve(strict=True) != temp_parent.resolve(strict=True)
        or run_root.is_symlink()
        or not run_root.is_dir()
    ):
        raise RuntimeError("Refusing cleanup outside the owned W-E03 temporary root")
    shutil.rmtree(run_root)
    if run_root.exists() or run_root.is_symlink():
        raise RuntimeError("Run-owned W-E03 temporary root survived cleanup")


def _write_actions_summary(record: dict[str, Any]) -> None:
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if not summary:
        return
    export = record["export"]
    task = record["independent_runtime"]
    lines = [
        "### Windows W-E03 Export Runtime developer verification",
        "",
        f"- Source commit: `{record['source_commit']}`.",
        f"- Untrusted Manifest-only SHA-256: `{export['source_manifest_sha256']}`.",
        f"- Export tree SHA-256: `{export['artifact_tree_sha256']}`; bytes: {export['artifact_total_bytes']}.",
        f"- Fresh App ID / Audit Store ID: `{export['app_id']}` / `{export['audit_store_id']}`.",
        f"- Test-only Broker SHA-256: `{task['test_broker_sha256']}`; Codex CLI: `{record['codex_cli_version']}`.",
        f"- Real Codex Task: `{task['task_status']}`; result hash: `{task['task_result_hash']}`.",
        f"- Workspace boundary / one-shot Approval / process and scratch cleanup / durable Audit: `{task['outside_workspace_write']}` / `{task['approval_replay']}` / `{task['codex_process_tree_cleanup']}` + `{task['scratch_cleanup']}` / `{task['audit_chain']}`.",
        f"- Run-owned temporary root cleanup: `{task['run_root_cleanup']}`.",
        "- Evidence scope: isolated `r2-e2e` Broker overlay and synthetic Owner fixture. Real Owner authority, normal Release UI, and source-product stop/uninstall acceptance are not established.",
        "- Export, runtime overlay, Task fixture, and Cargo target were removed after evidence validation; no artifact was uploaded.",
    ]
    with Path(summary).open("a", encoding="utf-8") as stream:
        stream.write("\n".join(lines) + "\n")


def run(codex_cli: Path) -> dict[str, Any]:
    if os.name != "nt":
        raise RuntimeError("Windows W-E03 developer Runtime verification requires Windows")
    source_commit = git_text("rev-parse", "HEAD")
    if git_text("branch", "--show-current") != "main":
        raise RuntimeError("Windows Export builder requires main")
    if git_text("status", "--porcelain", "--untracked-files=all"):
        raise RuntimeError("Windows Export builder requires a clean source tree")
    remote = git_text("ls-remote", "origin", "refs/heads/main").splitlines()
    if len(remote) != 1 or remote[0].split() != [source_commit, "refs/heads/main"]:
        raise RuntimeError("Local main does not match GitHub origin/main")

    codex_cli = codex_cli.expanduser().resolve(strict=True)
    if not codex_cli.is_file():
        raise RuntimeError("Codex CLI path is not a file")
    version_result = subprocess.run(
        [str(codex_cli), "--version"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
    )
    codex_version = version_result.stdout.strip()
    if codex_version != EXPECTED_CODEX_VERSION:
        raise RuntimeError(f"Codex CLI version mismatch: {codex_version}")

    temp_parent = safe_flutter_temp_parent(ROOT)
    run_root = temp_parent / f"d4p-windows-we03-{uuid.uuid4().hex}"
    run_root.mkdir()
    try:
        app_id, audit_store_id = _fresh_identity()
        manifest_input = run_root / f"{app_id}.json"
        manifest_raw, manifest_sha256 = create_manifest_only_input(
            manifest_input,
            app_id=app_id,
            audit_store_id=audit_store_id,
        )
        bundle = run_root / "export"
        subprocess.run(
            [
                sys.executable,
                "-X",
                "utf8",
                str(ROOT / "tooling/export_windows_product.py"),
                "--manifest-file",
                str(manifest_input),
                "--output-dir",
                str(bundle),
            ],
            cwd=ROOT,
            check=True,
            env=_credential_free_environment(),
        )
        receipt_path = bundle / RECEIPT_NAME
        build_evidence = json.loads(receipt_path.read_text(encoding="utf-8"))
        validate_build_evidence(build_evidence, bundle)
        if (
            build_evidence.get("source_commit") != source_commit
            or build_evidence.get("selection_input_source") != "manifest_file_untrusted"
            or build_evidence.get("source_manifest_sha256") != manifest_sha256
            or build_evidence.get("source_receipt_sha256") is not None
            or build_evidence.get("app_id") != app_id
            or build_evidence.get("audit_store_id") != audit_store_id
            or build_evidence.get("portable_bundle_assembled") is not True
            or build_evidence.get("signed") is not False
            or build_evidence.get("formal_distribution_claimed") is not False
        ):
            raise RuntimeError("Windows Export build receipt does not match the untrusted Manifest input")
        normal_inventory, normal_bytes, normal_tree_hash = _artifact_inventory(
            bundle, excluded_paths=frozenset({RECEIPT_NAME})
        )

        runtime_root = run_root / "runtime-copy"
        shutil.copytree(bundle, runtime_root, symlinks=False)
        runtime_manifest = runtime_root / "product_manifest.json"
        if runtime_manifest.read_bytes() != manifest_raw:
            raise RuntimeError("Export Runtime copy Manifest bytes differ from the builder input")
        helper_relative = ".developer-verification/broker/gui_shell_rust_helper.exe"
        helper_target = runtime_root.joinpath(*Path(helper_relative).parts)
        evidence_dir = run_root / "evidence"
        evidence_dir.mkdir()
        evidence_path = evidence_dir / "windows-we03-runtime.json"

        environment = _task_e2e_environment(
            _credential_free_environment(),
            codex_cli=codex_cli,
            runtime_root=runtime_root,
            manifest_path=runtime_manifest,
            manifest_sha256=manifest_sha256,
            source_commit=source_commit,
            app_id=app_id,
            audit_store_id=audit_store_id,
            evidence_path=evidence_path,
        )
        cargo_command = [
            sys.executable,
            "-X",
            "utf8",
            str(ROOT / "tooling/run_cargo_isolated.py"),
            "test",
            "--locked",
            "--offline",
            "--manifest-path",
            "native/rust_helper/Cargo.toml",
            "--features",
            "r2-e2e",
            "--test",
            "agent_task_crash_recovery_e2e",
            TEST_NAME,
            "--",
            "--ignored",
            "--exact",
            "--nocapture",
        ]
        cargo = subprocess.run(cargo_command, cwd=ROOT, env=environment, check=False)
        if cargo.returncode != 0:
            raise RuntimeError(f"Windows Export Runtime Codex Task E2E failed with {cargo.returncode}")
        evidence = validate_task_evidence(
            json.loads(evidence_path.read_text(encoding="utf-8")),
            source_commit=source_commit,
            manifest_sha256=manifest_sha256,
            app_id=app_id,
            audit_store_id=audit_store_id,
            expected_helper_relative_path=helper_relative.replace("/", "\\"),
        )
        if sha256_file(helper_target) != evidence["export_runtime_broker_sha256"]:
            raise RuntimeError("Developer Broker overlay changed after Task verification")
        final = {
            "evidence_type": "d4_pocket_windows_we03_export_runtime_developer",
            "source_commit": source_commit,
            "codex_cli_version": codex_version,
            "real_owner_authority": False,
            "external_provider_requests": 0,
            "export": {
                "selection_input_source": build_evidence["selection_input_source"],
                "source_manifest_sha256": manifest_sha256,
                "app_id": app_id,
                "audit_store_id": audit_store_id,
                "artifact_tree_sha256": normal_tree_hash,
                "artifact_total_bytes": normal_bytes,
                "artifact_file_count": len(normal_inventory),
                "normal_release_broker_sha256": next(
                    record["sha256"]
                    for record in normal_inventory
                    if record["path"] == "broker/gui_shell_rust_helper.exe"
                ),
            },
            "independent_runtime": {
                "task_status": evidence["task_status"],
                "task_result_hash": evidence["task_result_hash"],
                "test_broker_sha256": evidence["export_runtime_broker_sha256"],
                "test_broker_is_r2_e2e_overlay": True,
                "workspace_marker_sha256": evidence["workspace_marker_sha256"],
                "outside_workspace_write": evidence["outside_workspace_write"],
                "secret_value_exposed": evidence["secret_value_exposed"],
                "approval_replay": evidence["approval_replay"],
                "scratch_cleanup": evidence["scratch_cleanup"],
                "codex_process_tree_cleanup": evidence["codex_process_tree_cleanup"],
                "audit_chain": evidence["durable_audit_chain"],
                "audit_sha256": evidence["durable_audit_sha256"],
            },
            "scope_limit": evidence["scope_limit"],
        }
    finally:
        _safe_cleanup(run_root, temp_parent)

    final["independent_runtime"]["run_root_cleanup"] = "passed"
    if git_text("status", "--porcelain", "--untracked-files=all"):
        raise RuntimeError("Windows Export verification left changes in the source worktree")
    _write_actions_summary(final)
    return final


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--codex-cli", required=True, type=Path)
    args = parser.parse_args()
    try:
        result = run(args.codex_cli)
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        print(f"Windows W-E03 developer verification failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
