"""Run the isolated Linux W-E03 Export → independent Runtime → Tool path."""

from __future__ import annotations

import argparse
from contextlib import redirect_stdout
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import stat
import struct
import subprocess
import sys
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

SOURCE_APP_ID = "d4-pocket-app-11111111111111111111111111111111"
SOURCE_AUDIT_ID = "audit-store-22222222222222222222222222222222"
APP_ID_PATTERN = re.compile(r"d4-pocket-app-[0-9a-f]{32}\Z")
AUDIT_ID_PATTERN = re.compile(r"audit-store-[0-9a-f]{32}\Z")
TEST_UPDATE_PUBLIC_KEY_DER_HEX = (
    "302a300506032b6570032100"
    "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
)
TEST_UPDATE_SOURCE_URL = "https://updates.example.com/d4-pocket/stable"


def validate_manifest_bytes(
    raw: bytes,
    *,
    source_app_id: str = SOURCE_APP_ID,
    source_audit_id: str = SOURCE_AUDIT_ID,
) -> dict[str, str]:
    try:
        document = json.loads(raw.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ValueError("Broker Export Manifest is not valid UTF-8 JSON") from error
    if not isinstance(document, dict) or document.get("version") != 1:
        raise ValueError("Broker Export Manifest version is unsupported")
    if document.get("product") != "D4 Pocket":
        raise ValueError("Broker Export Manifest product is invalid")
    manifest = document.get("manifest")
    if not isinstance(manifest, dict):
        raise ValueError("Broker Export Manifest payload is missing")
    app_identity = manifest.get("app_identity")
    audit_store = manifest.get("audit_store")
    policies = manifest.get("inheritance_policy")
    distribution = manifest.get("distribution_metadata")
    if not isinstance(app_identity, dict) or not isinstance(audit_store, dict):
        raise ValueError("Export identity fields are missing")
    if not isinstance(policies, dict) or set(policies.values()) != {"none"}:
        raise ValueError("Export inheritance policy must deny all inheritance")
    if not isinstance(distribution, dict):
        raise ValueError("Export distribution metadata is missing")
    app_id = app_identity.get("app_id")
    audit_id = audit_store.get("store_id")
    if (
        app_identity.get("target_platform") != "linux"
        or not isinstance(app_id, str)
        or APP_ID_PATTERN.fullmatch(app_id) is None
        or not isinstance(audit_id, str)
        or AUDIT_ID_PATTERN.fullmatch(audit_id) is None
        or app_id == source_app_id
        or audit_id == source_audit_id
        or audit_store.get("inherited") is not False
        or distribution.get("signed") is not False
    ):
        raise ValueError("Export identity, inheritance, or unsigned boundary is invalid")
    return {
        "app_id": app_id,
        "audit_store_id": audit_id,
        "manifest_sha256": hashlib.sha256(raw).hexdigest(),
    }


def validate_developer_update_trust(raw: bytes) -> str:
    """W-E03用の隔離test trustがUI Export Manifestに固定されたことを確認する。"""
    try:
        document = json.loads(raw.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ValueError("Export Manifestのtest update trustがJSONではありません") from error
    trust = document.get("update_trust") if isinstance(document, dict) else None
    expected_key = bytes.fromhex(TEST_UPDATE_PUBLIC_KEY_DER_HEX)
    fingerprint = f"sha256:{hashlib.sha256(expected_key[12:]).hexdigest()}"
    if (
        not isinstance(trust, dict)
        or trust.get("版") != 2
        or trust.get("algorithm") != "Ed25519"
        or trust.get("public_key_der_hex") != TEST_UPDATE_PUBLIC_KEY_DER_HEX
        or trust.get("public_key_fingerprint") != fingerprint
        or trust.get("package_sources")
        != [{"channel": "stable", "base_url": TEST_UPDATE_SOURCE_URL}]
    ):
        raise ValueError(
            "installed UI Export Manifestにrun-only test update trustが固定されていません"
        )
    return fingerprint


def _read_regular_file(path: Path) -> bytes:
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise RuntimeError(f"Expected a private single-link regular file: {path.name}")
    return path.read_bytes()


def _copy_ui_manifest(
    export_root: Path, capture_root: Path, verified: dict[str, str]
) -> dict[str, str]:
    candidates = list(export_root.glob("d4-pocket-app-*.json"))
    if len(candidates) != 1:
        raise RuntimeError("Installed UI did not produce exactly one Export Manifest")
    source = candidates[0]
    raw = _read_regular_file(source)
    digest = hashlib.sha256(raw).hexdigest()
    if verified.get("file") != source.name or verified.get("sha256") != digest:
        raise RuntimeError("Installed UI Manifest changed after Broker verification")
    destination = capture_root / source.name
    descriptor = os.open(
        destination,
        os.O_WRONLY | os.O_CREAT | os.O_EXCL,
        0o600,
    )
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    if destination.read_bytes() != raw:
        raise RuntimeError("Captured Manifest differs from the UI Export bytes")
    identity = validate_manifest_bytes(raw)
    return {"file": destination.name, **identity}


def _last_smoke_result(output: str, expected_scope: str) -> dict[str, Any]:
    for line in reversed(output.splitlines()):
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(value, dict):
            continue
        if expected_scope == "update_lifecycle":
            if all(key in value for key in ("manifest", "uninstall", "live_audit")):
                return value
        elif value.get("acceptance_scope") == expected_scope:
            return value
    raise RuntimeError(f"Installed smoke did not emit {expected_scope} evidence")


def _run_smoke(smoke_module: Any, *, scope: str, **kwargs: Any) -> dict[str, Any]:
    captured = io.StringIO()
    try:
        with redirect_stdout(captured):
            smoke_module.run_installed_smoke(acceptance_scope=scope, **kwargs)
    except Exception as error:
        raise RuntimeError(
            f"Installed {scope} run failed: {type(error).__name__}: {error}"
        ) from error
    return _last_smoke_result(captured.getvalue(), scope)


def _validate_package_identity(
    package_path: Path, expected_app_id: str, expected_audit_id: str
) -> dict[str, Any]:
    raw = _read_regular_file(package_path)
    if len(raw) < 12 or raw[:8] != b"D4PKG01\n":
        raise RuntimeError("Independent Export output is not a D4PKG01 package")
    manifest_length = struct.unpack("<I", raw[8:12])[0]
    manifest_end = 12 + manifest_length
    if manifest_end > len(raw):
        raise RuntimeError("D4PKG01 outer manifest length exceeds the package")
    try:
        manifest = json.loads(raw[12:manifest_end].decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise RuntimeError("D4PKG01 outer manifest is invalid") from error
    if (
        not isinstance(manifest, dict)
        or manifest.get("app_id") != expected_app_id
        or manifest.get("audit_store_id") != expected_audit_id
        or manifest.get("target_platform") != "linux"
        or not isinstance(manifest.get("files"), list)
    ):
        raise RuntimeError("D4PKG01 package identity differs from the Export Manifest")
    offset = manifest_end
    for row in manifest["files"]:
        if not isinstance(row, dict) or type(row.get("byte_length")) is not int:
            raise RuntimeError("D4PKG01 file entry is malformed")
        offset += row["byte_length"]
        if offset > len(raw):
            raise RuntimeError("D4PKG01 file payload exceeds the package")
    if offset != len(raw):
        raise RuntimeError("D4PKG01 package has unclaimed trailing bytes")
    return {
        "package_sha256": hashlib.sha256(raw).hexdigest(),
        "package_byte_length": len(raw),
        "file_count": len(manifest["files"]),
    }


def validate_builder_output(
    evidence: dict[str, Any],
    *,
    output_root: Path,
    manifest_identity: dict[str, str],
) -> dict[str, Any]:
    boundary = evidence.get("authority_boundary")
    compile_identity = evidence.get("compile_time_identity")
    package = evidence.get("package")
    if (
        evidence.get("evidence_type") != "d4_pocket_linux_export_build"
        or evidence.get("evidence_source") != "INTERNAL_STATE"
        or evidence.get("target_platform") != "linux"
        or evidence.get("build_mode") != "release"
        or evidence.get("selection_input_source") != "manifest_file_untrusted"
        or evidence.get("source_manifest_sha256") != manifest_identity["manifest_sha256"]
        or evidence.get("source_receipt_sha256") is not None
        or evidence.get("manifest_copied_byte_for_byte") is not True
        or not isinstance(boundary, dict)
        or boundary.get("source_authority_verified") is not False
        or boundary.get("owner_authorization_verified") is not False
        or boundary.get("authority_strip") is not True
        or boundary.get("credential_values_explicitly_supplied") is not False
        or any(
            boundary.get(key) is not False
            for key in (
                "credential_inherited",
                "permission_inherited",
                "approval_inherited",
                "audit_chain_inherited",
            )
        )
        or not isinstance(compile_identity, dict)
        or compile_identity.get("source") != "schema_validated_manifest"
        or compile_identity.get("app_id") != manifest_identity["app_id"]
        or compile_identity.get("audit_store_id") != manifest_identity["audit_store_id"]
        or compile_identity.get("manifest_sha256") != manifest_identity["manifest_sha256"]
        or not isinstance(package, dict)
        or package.get("signed") is not False
        or package.get("installed_product_verified") is not False
    ):
        raise RuntimeError("Manifest-only Export build evidence violates its contract")
    filename = package.get("file_name")
    if not isinstance(filename, str) or Path(filename).name != filename:
        raise RuntimeError("Export build evidence package filename is unsafe")
    package_path = output_root / filename
    package_identity = _validate_package_identity(
        package_path,
        manifest_identity["app_id"],
        manifest_identity["audit_store_id"],
    )
    if (
        package.get("sha256") != package_identity["package_sha256"]
        or package.get("byte_length") != package_identity["package_byte_length"]
    ):
        raise RuntimeError("Export build evidence package hash or size differs")
    return {"path": package_path, **package_identity}


def _assert_source_uninstalled(result: dict[str, Any]) -> None:
    uninstall = result.get("uninstall")
    if (
        result.get("gtk_window") != "opened_and_normally_closed"
        or result.get("owned_processes") != "all_stopped"
        or not isinstance(uninstall, dict)
        or uninstall.get("product_root") != "removed"
        or uninstall.get("desktop_entry") != "removed"
        or uninstall.get("audit_store") != "retained_outside_product_root"
        or uninstall.get("user_data") != "retained_outside_product_root"
        or result.get("runtime_audit") != "durable_after_broker_restart"
    ):
        raise RuntimeError("Source D4 uninstall or retained-Audit evidence is incomplete")


def _assert_independent_task(result: dict[str, Any], package_sha256: str) -> dict[str, Any]:
    task = result.get("agent_task_normal_release")
    if (
        result.get("package_sha256") != package_sha256
        or result.get("installed_app_exit")
        != "normal_window_close_and_owned_process_cleanup"
        or not isinstance(task, dict)
        or task.get("task_scenarios") != ["complete"]
        or task.get("terminal_statuses") != ["completed"]
        or task.get("normal_release_task") is not True
        or task.get("workspace_write") is not True
        or task.get("secret_and_outside_reads_denied") is not True
        or task.get("workspace_task_scratch_removed") is not True
        or task.get("descendant_groups_stopped") is not True
        or task.get("completed_fixture_reports") != 1
        or not isinstance(task.get("positive_result_hash"), str)
        or re.fullmatch(r"sha256:[0-9a-f]{64}", task["positive_result_hash"]) is None
        or not isinstance(task.get("live_audit"), dict)
    ):
        raise RuntimeError("Independent Export installed Tool Task evidence is incomplete")
    return task


def _assert_independent_crash_recovery(
    result: dict[str, Any], package_sha256: str
) -> dict[str, Any]:
    crash = result.get("agent_task_normal_release_crash_recovery")
    loopback = crash.get("loopback") if isinstance(crash, dict) else None
    scenarios = loopback.get("scenarios") if isinstance(loopback, dict) else None
    if (
        result.get("acceptance_scope") != "agent_task_normal_release_crash_recovery"
        or result.get("package_sha256") != package_sha256
        or result.get("installed_app_exit")
        != "normal_window_close_and_owned_process_cleanup"
        or not isinstance(crash, dict)
        or crash.get("normal_release_codex_configuration") is not True
        or not isinstance(crash.get("task_id"), str)
        or crash.get("task_descendant_group_stopped") is not True
        or crash.get("scratch_retained_until_recovery") is not True
        or crash.get("scratch_removed_after_restart_and_workspace_registration") is not True
        or crash.get("task_recovery_status") != "suspended"
        or crash.get("old_approval_reused") is not False
        or crash.get("positive_result_after_crash") is not False
        or crash.get("codex_home_after_broker_restart")
        != "fresh_run_owned_empty_fixture"
        or not isinstance(crash.get("audit_chain"), dict)
        or not isinstance(result.get("durable_audit_after_broker_restart"), dict)
        or not isinstance(scenarios, list)
        or len(scenarios) != 1
        or not isinstance(scenarios[0], dict)
        or scenarios[0].get("name") != "crash"
        or (
            scenarios[0].get("posts"),
            scenarios[0].get("tool_calls"),
            scenarios[0].get("tool_outputs"),
        )
        != (1, 1, 0)
    ):
        raise RuntimeError(
            "Independent Export installed crash-recovery evidence is incomplete"
        )
    return crash


def _run_command(command: list[str], *, cwd: Path) -> str:
    completed = subprocess.run(
        command,
        cwd=cwd,
        text=True,
        encoding="utf-8",
        errors="replace",
        capture_output=True,
        check=False,
        timeout=1200,
    )
    if completed.returncode != 0:
        details = (completed.stderr or completed.stdout)[-5000:]
        raise RuntimeError(
            f"Command exited {completed.returncode}: {Path(command[1]).name}; {details}"
        )
    return completed.stdout


def run(temp_root: Path, private_key: Path, codex_cli: Path) -> dict[str, Any]:
    import tooling.tests.linux_installed_product_smoke as smoke

    root_meta = temp_root.lstat()
    if not stat.S_ISDIR(root_meta.st_mode) or temp_root.is_symlink():
        raise RuntimeError("D4P_EXPORT_ROOT must be an existing nonsymlink directory")
    temp_root = temp_root.resolve(strict=True)
    if root_meta.st_mode & 0o077:
        raise RuntimeError("D4P_EXPORT_ROOT permissions must be private (0700)")
    if private_key.resolve(strict=True) != private_key or private_key.is_symlink():
        raise RuntimeError("Run-only signing key must be a direct regular file")
    if temp_root not in private_key.parents or private_key.stat().st_mode & 0o077:
        raise RuntimeError("Run-only signing key is outside D4P_EXPORT_ROOT or not private")
    if codex_cli.resolve(strict=True) != codex_cli or codex_cli.is_symlink():
        raise RuntimeError("Pinned Codex CLI must be a direct executable file")
    if temp_root not in codex_cli.parents or not os.access(codex_cli, os.X_OK):
        raise RuntimeError("Pinned Codex CLI is outside D4P_EXPORT_ROOT or not executable")

    output_root = temp_root / "output"
    source_package = output_root / "D4Pocket-linux.d4pkg"
    rollback_package = output_root / "D4Pocket-linux-rollback.d4pkg"
    for path in (source_package, rollback_package):
        _read_regular_file(path)

    capture_root = temp_root / "manifest-only-capture"
    capture_root.mkdir(mode=0o700)
    capture: list[dict[str, str]] = []
    original_verify = smoke._verify_export_manifest

    def capture_verified_manifest(export_root: Path) -> dict[str, str]:
        verified = original_verify(export_root)
        capture.append(_copy_ui_manifest(export_root, capture_root, verified))
        return verified

    smoke._verify_export_manifest = capture_verified_manifest
    try:
        source_result = _run_smoke(
            smoke,
            scope="update_lifecycle",
            package=source_package,
            temp_root=temp_root,
            acceptance_root=temp_root / "source-product-acceptance",
            private_key_path=private_key,
            update_package=rollback_package,
            export_update_trust=(
                TEST_UPDATE_PUBLIC_KEY_DER_HEX,
                TEST_UPDATE_SOURCE_URL,
            ),
        )
    finally:
        smoke._verify_export_manifest = original_verify
    if len(capture) != 1:
        raise RuntimeError("Source product did not yield exactly one verified UI Export Manifest")
    _assert_source_uninstalled(source_result)
    source_manifest_evidence = source_result.get("manifest")
    if (
        not isinstance(source_manifest_evidence, dict)
        or source_manifest_evidence.get("file") != capture[0]["file"]
        or source_manifest_evidence.get("sha256") != capture[0]["manifest_sha256"]
    ):
        raise RuntimeError("Installed UI Export evidence differs from captured Manifest bytes")
    manifest_file = capture_root / capture[0]["file"]
    manifest_raw = _read_regular_file(manifest_file)
    manifest_identity = validate_manifest_bytes(manifest_raw)
    update_trust_fingerprint = validate_developer_update_trust(manifest_raw)
    if manifest_identity != {
        key: capture[0][key]
        for key in ("app_id", "audit_store_id", "manifest_sha256")
    }:
        raise RuntimeError("Captured UI Manifest identity changed before Builder input")
    if (temp_root / "source-product-acceptance").exists():
        raise RuntimeError("Source D4 installed acceptance root survived its cleanup")

    candidate_output = temp_root / "independent-export-output"
    command = [
        sys.executable,
        str(ROOT / "tooling/export_linux_product.py"),
        "--manifest-file",
        str(manifest_file),
        "--output-dir",
        str(candidate_output),
        "--allow-verification-branch",
    ]
    _run_command(command, cwd=ROOT)
    build_evidence_path = candidate_output / "d4_pocket_linux_export_build.json"
    evidence_bytes = _read_regular_file(build_evidence_path)
    try:
        build_evidence = json.loads(evidence_bytes.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise RuntimeError("Independent Export build evidence is invalid JSON") from error
    if not isinstance(build_evidence, dict):
        raise RuntimeError("Independent Export build evidence must be an object")
    package_info = validate_builder_output(
        build_evidence,
        output_root=candidate_output,
        manifest_identity=manifest_identity,
    )
    if package_info["package_sha256"] == hashlib.sha256(source_package.read_bytes()).hexdigest():
        raise RuntimeError("Independent Export package unexpectedly matches the source D4 package")

    # The smoke harness binds Cargo test constants from these values. Remove its run-owned
    # target first so the custom identity is compiled into the ignored install-stage test.
    cargo_target = temp_root / "linux-installed-cargo-target"
    if cargo_target.exists() or cargo_target.is_symlink():
        if cargo_target.is_symlink() or not cargo_target.is_dir():
            raise RuntimeError("Run-owned installed-test Cargo target is not a plain directory")
        if temp_root not in cargo_target.resolve(strict=True).parents:
            raise RuntimeError("Run-owned installed-test Cargo target escaped D4P_EXPORT_ROOT")
        shutil.rmtree(cargo_target)
    smoke.APP_ID = manifest_identity["app_id"]
    smoke.AUDIT_ID = manifest_identity["audit_store_id"]
    try:
        independent_result = _run_smoke(
            smoke,
            scope="agent_task_normal_release",
            package=Path(package_info["path"]),
            temp_root=temp_root,
            acceptance_root=temp_root / "independent-product-acceptance",
            private_key_path=private_key,
            update_package=Path(package_info["path"]),
            codex_cli=codex_cli,
        )
        independent_crash_result = _run_smoke(
            smoke,
            scope="agent_task_normal_release_crash_recovery",
            package=Path(package_info["path"]),
            temp_root=temp_root,
            acceptance_root=temp_root / "independent-product-crash-acceptance",
            private_key_path=private_key,
            update_package=Path(package_info["path"]),
            codex_cli=codex_cli,
        )
    finally:
        smoke.APP_ID = SOURCE_APP_ID
        smoke.AUDIT_ID = SOURCE_AUDIT_ID
    task = _assert_independent_task(independent_result, package_info["package_sha256"])
    crash = _assert_independent_crash_recovery(
        independent_crash_result, package_info["package_sha256"]
    )
    if (temp_root / "independent-product-acceptance").exists():
        raise RuntimeError("Independent product installed acceptance root survived cleanup")
    if (temp_root / "independent-product-crash-acceptance").exists():
        raise RuntimeError("Independent product crash acceptance root survived cleanup")

    return {
        "version": 1,
        "evidence_class": "HOSTED_OS_PRODUCT",
        "runtime_evidence_class": "LIVE_RUNTIME",
        "source_commit": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
        ).strip(),
        "platform": "GitHub-hosted Ubuntu 22.04 x86_64, X11/Xvfb",
        "authority_verified": False,
        "real_owner_authority": False,
        "external_provider_requests": 0,
        "source_product": {
            "package_sha256": source_result.get("package_sha256"),
            "ui_generated_manifest_sha256": manifest_identity["manifest_sha256"],
            "export_identity": {
                "app_id": manifest_identity["app_id"],
                "audit_store_id": manifest_identity["audit_store_id"],
            },
            "update_lifecycle": "passed",
            "uninstall": source_result["uninstall"],
            "owned_processes": source_result["owned_processes"],
            "audit_durable_after_broker_restart": source_result["runtime_audit"],
        },
        "independent_export": {
            "selection_input_source": build_evidence["selection_input_source"],
            "source_receipt_sha256": build_evidence["source_receipt_sha256"],
            "source_manifest_sha256": build_evidence["source_manifest_sha256"],
            "build_evidence_sha256": hashlib.sha256(evidence_bytes).hexdigest(),
            "package_sha256": package_info["package_sha256"],
            "package_byte_length": package_info["package_byte_length"],
            "app_id": manifest_identity["app_id"],
            "audit_store_id": manifest_identity["audit_store_id"],
            "update_trust": {
                "source": "installed_ui_test_owner_fixture",
                "public_key_fingerprint": update_trust_fingerprint,
                "package_source": TEST_UPDATE_SOURCE_URL,
                "private_key_in_product": False,
            },
            "credential_permission_approval_audit_inherited": False,
        },
        "independent_runtime": {
            "source_app_uninstalled_before_runtime": True,
            "acceptance_scope": independent_result["acceptance_scope"],
            "package_sha256": independent_result["package_sha256"],
            "task_scenarios": task["task_scenarios"],
            "terminal_statuses": task["terminal_statuses"],
            "positive_result_hash": task["positive_result_hash"],
            "workspace_write": task["workspace_write"],
            "secret_and_outside_reads_denied": task["secret_and_outside_reads_denied"],
            "workspace_task_scratch_removed": task["workspace_task_scratch_removed"],
            "descendant_groups_stopped": task["descendant_groups_stopped"],
            "durable_audit_after_normal_exit": isinstance(
                independent_result.get("durable_audit_after_normal_exit"), dict
            ),
            "installed_app_exit": independent_result["installed_app_exit"],
        },
        "independent_crash_recovery": {
            "acceptance_scope": independent_crash_result["acceptance_scope"],
            "package_sha256": independent_crash_result["package_sha256"],
            "task_id": crash["task_id"],
            "task_descendant_group_stopped": crash["task_descendant_group_stopped"],
            "scratch_retained_until_recovery": crash["scratch_retained_until_recovery"],
            "scratch_removed_after_restart_and_workspace_registration": crash[
                "scratch_removed_after_restart_and_workspace_registration"
            ],
            "task_recovery_status": crash["task_recovery_status"],
            "old_approval_reused": crash["old_approval_reused"],
            "positive_result_after_crash": crash["positive_result_after_crash"],
            "loopback": crash["loopback"],
            "durable_audit_after_broker_restart": isinstance(
                independent_crash_result.get("durable_audit_after_broker_restart"), dict
            ),
            "installed_app_exit": independent_crash_result["installed_app_exit"],
        },
        "limitations": [
            "Automated test-only native Owner path; no real Owner authority was exercised.",
            "Codex CLI used the run-owned loopback Responses fixture; no external Provider was contacted.",
            "This does not close W-E03, E2/E3/E4/E5/E6, formal distribution, or Release.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--temp-root", type=Path, required=True)
    parser.add_argument("--private-key", type=Path, required=True)
    parser.add_argument("--codex-cli", type=Path, required=True)
    args = parser.parse_args()
    try:
        evidence = run(args.temp_root, args.private_key, args.codex_cli)
        output_path = args.temp_root / "w-e03-runtime-evidence.json"
        with output_path.open("x", encoding="utf-8", newline="\n") as stream:
            json.dump(evidence, stream, ensure_ascii=False, indent=2)
            stream.write("\n")
        os.chmod(output_path, 0o600)
        rendered = json.dumps(evidence, ensure_ascii=False, sort_keys=True)
        print(rendered)
        summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
        if summary_path:
            with Path(summary_path).open("a", encoding="utf-8") as stream:
                stream.write("\n## W-E03 isolated Linux Export Runtime\n\n")
                stream.write("```json\n" + json.dumps(evidence, ensure_ascii=False, indent=2) + "\n```\n")
        return 0
    except Exception as error:
        print(f"W-E03 developer verification failed: {type(error).__name__}: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
