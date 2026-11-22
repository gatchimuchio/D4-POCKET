"""Run the bounded W-E04 Windows Setup and Broker-managed installed lifecycle."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling import (  # noqa: E402
    build_windows_installer,
    export_windows_product,
    package_windows_product,
)
from tooling.tests.windows_broker_export_manifest_compare import (  # noqa: E402
    _assert_export_capture,
)

EXPORT_TEST = (
    "desktop_launcher::tests::"
    "nativeOwner確認dialogのExportNoYesをBrokerReceiptManifestまで接続する"
)
SEED_TEST = (
    "broker::update_center::tests::"
    "windows_installed_acceptance_seeds_signed_candidates_and_packages"
)
TEST_PRIVATE_KEY_DER_HEX = (
    "302e020100300506032b657004220420"
    "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
)
TEST_PUBLIC_KEY_DER_HEX = (
    "302a300506032b6570032100"
    "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
)
TEST_SOURCE_URL = "https://updates.example.com/d4-pocket/stable"
CURRENT_STAGE = "initialize"
_SENSITIVE_ENV_MARKERS = (
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "CREDENTIAL",
    "PRIVATE_KEY",
    "ACCESS_KEY",
    "ID_TOKEN_REQUEST",
)


def _sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def _write_json(path: Path, value: dict[str, Any]) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def _child_environment(overrides: dict[str, str] | None = None) -> dict[str, str]:
    env = os.environ.copy()
    for name in tuple(env):
        if any(marker in name.upper() for marker in _SENSITIVE_ENV_MARKERS):
            env.pop(name, None)
    if overrides:
        env.update(overrides)
    return env


def _run(command: list[str], *, env: dict[str, str], cwd: Path = ROOT) -> str:
    safe_command = [Path(command[0]).name, *command[1:]]
    print(f"::group::{' '.join(safe_command)}", flush=True)
    result = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    if result.stdout:
        print(result.stdout, end="" if result.stdout.endswith("\n") else "\n")
    print("::endgroup::", flush=True)
    if result.returncode != 0:
        raise RuntimeError(f"{Path(command[0]).name} exited with {result.returncode}")
    return result.stdout


def _next_patch(version: str) -> str:
    match = re.fullmatch(r"([0-9]+)\.([0-9]+)\.([0-9]+)([+-][0-9A-Za-z.-]+)?", version)
    if match is None:
        raise ValueError("source product version is not a bounded numeric SemVer")
    major, minor, patch, suffix = match.groups()
    return f"{major}.{minor}.{int(patch) + 1}{suffix or ''}"


def _assert_run_root(run_root: Path, runner_temp: Path) -> Path:
    runner_temp = runner_temp.resolve(strict=True)
    if run_root.exists() or run_root.is_symlink():
        raise FileExistsError("W-E04 run root already exists; refusing overwrite")
    parent = run_root.parent.resolve(strict=True)
    candidate = parent / run_root.name
    if candidate == runner_temp or not candidate.is_relative_to(runner_temp):
        raise ValueError("W-E04 run root must be a new direct descendant of RUNNER_TEMP")
    candidate.mkdir()
    return candidate.resolve(strict=True)


def _cargo_test(
    command_name: str,
    test_filter: str,
    env: dict[str, str],
    *,
    show_test_output: bool = False,
) -> str:
    command = [
        "cargo",
        "test",
        "--locked",
        "--manifest-path",
        "native/rust_helper/Cargo.toml",
        "--lib",
        "--",
        "--ignored",
        "--exact",
        test_filter,
        "--test-threads=1",
    ]
    if show_test_output:
        command.append("--nocapture")
    output = _run(command, env=env)
    if not re.search(r"test result: ok\. 1 passed; 0 failed;", output):
        raise RuntimeError(f"{command_name} did not report exactly one passing test")
    return output


_EXPECTED_OWNER_DECISIONS = (
    ("first_install_download_decline", "decline"),
    ("first_install_download", "approve"),
    ("first_install_stage", "approve"),
    ("first_install_activation", "approve"),
    ("update_download", "approve"),
    ("update_stage", "approve"),
    ("update_activation", "approve"),
    ("rollback", "approve"),
    ("repair", "approve"),
    ("uninstall", "approve"),
)

_REQUIRED_LIFECYCLE_FLAGS = (
    "setup_executed",
    "setup_first_launch",
    "setup_process_exited_after_handoff",
    "initial_install",
    "update",
    "repair",
    "rollback",
    "uninstall",
    "staged_state_not_reported_installed",
    "product_root_absent_before_test_cleanup",
    "start_menu_shortcut_absent_before_test_cleanup",
    "product_root_absent_after_uninstall",
    "start_menu_shortcut_absent_after_uninstall",
    "user_data_preserved_after_uninstall",
    "audit_store_preserved_after_uninstall",
    "test_identity_cleanup",
    "setup_temp_and_process_cleanup",
)


def _assert_lifecycle_evidence(
    lifecycle: dict[str, Any],
    *,
    source_commit: str,
    app_id: str,
    audit_store_id: str,
    initial_version: str,
    update_version: str,
    initial_package_sha256: str,
    update_package_sha256: str,
) -> None:
    if lifecycle.get("result") != "passed" or lifecycle.get("failure_stage") is not None:
        raise ValueError("Windows UI lifecycle did not report an unfailed pass")
    if lifecycle.get("source_commit") != source_commit:
        raise ValueError("Windows UI lifecycle evidence has a different source commit")
    if lifecycle.get("app_id") != app_id or lifecycle.get("audit_store_id") != audit_store_id:
        raise ValueError("Windows UI lifecycle evidence has a different Product identity")
    if lifecycle.get("real_owner_authority") is not False:
        raise ValueError("Developer UI automation was misrepresented as real Owner authority")
    if lifecycle.get("evidence_class") != "LIVE_RUNTIME":
        raise ValueError("Windows UI lifecycle evidence has the wrong evidence class")
    for field in _REQUIRED_LIFECYCLE_FLAGS:
        if lifecycle.get(field) is not True:
            raise ValueError(f"Windows UI lifecycle did not verify {field}")
    if lifecycle.get("initial_version") != initial_version or lifecycle.get(
        "update_version"
    ) != update_version:
        raise ValueError("Windows UI lifecycle versions differ from the packaged candidates")
    if lifecycle.get("initial_package_sha256") != initial_package_sha256 or lifecycle.get(
        "update_package_sha256"
    ) != update_package_sha256:
        raise ValueError("Windows UI lifecycle package hashes differ from the packaged candidates")

    owner_decisions = lifecycle.get("owner_decisions")
    if not isinstance(owner_decisions, list):
        raise ValueError("Windows UI lifecycle omitted its test Owner decision sequence")
    observed_decisions: list[tuple[str, str]] = []
    for item in owner_decisions:
        if not isinstance(item, dict) or item.get("test_identity") is not True:
            raise ValueError("Owner dialog decision was not bound to the isolated test identity")
        operation = item.get("operation")
        decision = item.get("decision")
        if not isinstance(operation, str) or not isinstance(decision, str):
            raise ValueError("Owner dialog decision has an invalid shape")
        observed_decisions.append((operation, decision))
    if tuple(observed_decisions) != _EXPECTED_OWNER_DECISIONS:
        raise ValueError("Windows UI lifecycle Owner decision sequence is incomplete or unexpected")

    audit_before = lifecycle.get("audit_before_uninstall")
    audit_after = lifecycle.get("audit_after_uninstall")
    if not isinstance(audit_before, dict) or not isinstance(audit_after, dict):
        raise ValueError("Windows UI lifecycle omitted the before/after durable Audit summary")
    for label, audit in (("before", audit_before), ("after", audit_after)):
        if type(audit.get("events")) is not int or audit["events"] < 1:
            raise ValueError(f"Windows UI lifecycle Audit {label} event count is invalid")
        if type(audit.get("bytes")) is not int or audit["bytes"] < 1:
            raise ValueError(f"Windows UI lifecycle Audit {label} byte count is invalid")
        if not isinstance(audit.get("sha256"), str) or not re.fullmatch(
            r"[0-9a-f]{64}", audit["sha256"]
        ):
            raise ValueError(f"Windows UI lifecycle Audit {label} hash is invalid")
        if audit.get("contains_test_candidate_ids") is not True or audit.get(
            "private_test_key_absent"
        ) is not True:
            raise ValueError(f"Windows UI lifecycle Audit {label} scope/content checks failed")
    if audit_after["events"] <= audit_before["events"]:
        raise ValueError("Uninstall did not append a durable Audit event")
    if lifecycle.get("audit_event_count_after_uninstall") != audit_after["events"]:
        raise ValueError("Post-uninstall Audit event count projection differs from the file")
    if lifecycle.get("audit_bytes_after_uninstall") != audit_after["bytes"]:
        raise ValueError("Post-uninstall Audit byte count projection differs from the file")
    if lifecycle.get("audit_sha256_after_uninstall") != audit_after["sha256"]:
        raise ValueError("Post-uninstall Audit hash projection differs from the file")
    if not isinstance(lifecycle.get("sentinel_sha256"), str) or not re.fullmatch(
        r"[0-9a-f]{64}", lifecycle["sentinel_sha256"]
    ):
        raise ValueError("Uninstall preservation sentinel hash is missing or invalid")
    cleanup_errors = lifecycle.get("cleanup_errors")
    if cleanup_errors != []:
        raise ValueError("Windows UI lifecycle reported process or test identity cleanup errors")


def _make_bundle_and_packages(
    *,
    receipt_path: Path,
    manifest_path: Path,
    bundle_path: Path,
    initial_package_path: Path,
    update_package_path: Path,
) -> dict[str, Any]:
    export_evidence = export_windows_product.build_portable_bundle(
        receipt_path, manifest_path, bundle_path
    )
    if (
        export_evidence.get("source_commit") != _git(["rev-parse", "HEAD"])
        or export_evidence.get("target_platform") != "windows"
        or export_evidence.get("portable_bundle_assembled") is not True
        or export_evidence.get("signed") is not False
        or export_evidence.get("formal_distribution_claimed") is not False
    ):
        raise ValueError("Export bundle evidence identity or scope is invalid")

    product_manifest = json.loads(
        (bundle_path / "product_manifest.json").read_text(encoding="utf-8")
    )
    manifest_body = product_manifest.get("manifest")
    identity = manifest_body.get("app_identity") if isinstance(manifest_body, dict) else None
    audit_store = manifest_body.get("audit_store") if isinstance(manifest_body, dict) else None
    trust = product_manifest.get("update_trust")
    if not isinstance(identity, dict) or not isinstance(audit_store, dict):
        raise ValueError("Broker Export Product Manifest lacks app or Audit identity")
    if not isinstance(trust, dict):
        raise ValueError("Broker Export Product Manifest lacks UpdateTrust")
    app_id = identity.get("app_id")
    audit_store_id = audit_store.get("store_id")
    if not isinstance(app_id, str) or not re.fullmatch(r"d4-pocket-app-[0-9a-f]{32}", app_id):
        raise ValueError("Export App ID is not a newly generated Windows Product identity")
    if not isinstance(audit_store_id, str) or not re.fullmatch(
        r"audit-store-[0-9a-f]{32}", audit_store_id
    ):
        raise ValueError("Export Audit Store ID is not a newly generated identity")
    if (
        trust.get("public_key_der_hex") != TEST_PUBLIC_KEY_DER_HEX
        or len(trust.get("package_sources", [])) != 1
        or trust["package_sources"][0].get("base_url") != TEST_SOURCE_URL
    ):
        raise ValueError("Export Product Manifest contains unexpected test UpdateTrust")
    if TEST_PRIVATE_KEY_DER_HEX in json.dumps(product_manifest):
        raise ValueError("test signing key leaked into Product Manifest")

    source_version = export_windows_product.product_version_from_source(ROOT)
    update_version = _next_patch(source_version)
    package_initial = package_windows_product.write_product_package(
        bundle_path,
        initial_package_path,
        product_version=source_version,
        app_id=app_id,
        audit_store_id=audit_store_id,
    )
    package_update = package_windows_product.write_product_package(
        bundle_path,
        update_package_path,
        product_version=update_version,
        app_id=app_id,
        audit_store_id=audit_store_id,
    )
    if package_initial["app_id"] != package_update["app_id"] or package_initial[
        "audit_store_id"
    ] != package_update["audit_store_id"]:
        raise ValueError("initial and update D4PKG identities differ")
    return {
        "app_id": app_id,
        "audit_store_id": audit_store_id,
        "source_version": source_version,
        "update_version": update_version,
        "export": {
            "source_commit": export_evidence["source_commit"],
            "manifest_sha256": _sha256(manifest_path.read_bytes()),
            "app_id": app_id,
            "audit_store_id": audit_store_id,
            "update_trust_fingerprint": trust.get("public_key_fingerprint"),
            "bundle_tree_sha256": export_evidence.get("artifact_tree_sha256"),
            "bundle_file_count": export_evidence.get("artifact_file_count"),
        },
        "packages": {
            "initial": {
                "product_version": source_version,
                "sha256": package_initial["package_sha256"],
                "size_bytes": package_initial["package_size_bytes"],
            },
            "update": {
                "product_version": update_version,
                "sha256": package_update["package_sha256"],
                "size_bytes": package_update["package_size_bytes"],
            },
            "payload_tree_same": package_initial["file_count"]
            == package_update["file_count"],
        },
        "trust": {
            "public_key_fingerprint": trust.get("public_key_fingerprint"),
            "package_source": TEST_SOURCE_URL,
            "private_key_bundled": False,
        },
    }


def _git(args: list[str]) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=ROOT,
        env=_child_environment(),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} exited with {result.returncode}")
    return result.stdout.strip()


def run(source_commit: str, evidence_dir: Path, run_root: Path) -> dict[str, Any]:
    global CURRENT_STAGE
    if sys.platform != "win32":
        raise RuntimeError("W-E04 Windows Setup lifecycle is Windows-only")
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise ValueError("source commit must be a full lowercase SHA")
    if _git(["rev-parse", "HEAD"]) != source_commit:
        raise RuntimeError("checkout HEAD differs from the requested source commit")
    if _git(["branch", "--show-current"]) != "main":
        raise RuntimeError("W-E04 manual lifecycle must run from main")
    if _git(["rev-parse", "refs/remotes/origin/main"]) != source_commit:
        raise RuntimeError("origin/main differs from the checked-out source commit")
    if _git(["status", "--porcelain", "--untracked-files=all"]):
        raise RuntimeError("source tree must be clean before W-E04 lifecycle")

    runner_temp = Path(os.environ["RUNNER_TEMP"]).resolve(strict=True)
    evidence_dir = evidence_dir.resolve(strict=False)
    if not evidence_dir.is_relative_to(runner_temp) or evidence_dir == runner_temp:
        raise ValueError("evidence directory must be inside RUNNER_TEMP")
    if evidence_dir.exists():
        raise FileExistsError("evidence directory already exists")
    evidence_dir.mkdir(parents=True)
    run_root = _assert_run_root(run_root, runner_temp)
    capture_temp_dir: Path | None = None

    result: dict[str, Any] = {
        "version": 1,
        "evidence_id": "W-E04-WINDOWS-SETUP-INSTALLED-LIFECYCLE",
        "source": {"source_commit": source_commit},
        "classification": {
            "evidence_class": "HOSTED_OS_PRODUCT",
            "runtime_evidence_class": "LIVE_RUNTIME",
            "platform": "GitHub-hosted Windows Server 2025 interactive session",
            "real_owner_authority": False,
            "owner_decisions": "native dialogs driven by isolated Developer test automation",
            "formal_distribution_claimed": False,
            "release_claimed": False,
        },
        "scope": {
            "normal_setup_executable_executed": False,
            "normal_setup_first_launch": False,
            "initial_install": False,
            "update": False,
            "repair": False,
            "rollback": False,
            "uninstall": False,
            "stage_or_portable_misrepresented_as_installed": False,
            "external_catalog_https_fetched": False,
            "e5_or_e6_claimed": False,
        },
        "failure_stage": CURRENT_STAGE,
    }
    try:
        CURRENT_STAGE = "prepare_public_test_update_trust"
        trust_input_path = run_root / "update-trust-input.json"
        _write_json(
            trust_input_path,
            {
                "public_key_der_hex": TEST_PUBLIC_KEY_DER_HEX,
                "package_sources": [
                    {"channel": "stable", "base_url": TEST_SOURCE_URL}
                ],
            },
        )
        capture_temp_root = Path(tempfile.gettempdir()).resolve(strict=True)
        capture_temp_dir = Path(
            tempfile.mkdtemp(
                prefix="d4p-w-e04-owner-capture-",
                dir=capture_temp_root,
            )
        ).resolve(strict=True)
        if capture_temp_dir.parent != capture_temp_root:
            raise ValueError("Owner capture directory must be a direct child of the Windows temp root")
        test_env = _child_environment(
            {
                "GUI_SHELL_EXPORT_E2E_CAPTURE_DIR": str(capture_temp_dir),
                "GUI_SHELL_EXPORT_E2E_UPDATE_TRUST_FILE": str(trust_input_path),
                "TEMP": str(capture_temp_root),
                "TMP": str(capture_temp_root),
            }
        )

        CURRENT_STAGE = "broker_native_export_owner_test"
        _cargo_test("Broker Export Owner test", EXPORT_TEST, test_env)
        captured_receipt_path = capture_temp_dir / "d4_pocket_build_receipt.json"
        captured_manifest_path = capture_temp_dir / "broker-generated-manifest.json"
        if not captured_receipt_path.is_file() or not captured_manifest_path.is_file():
            raise FileNotFoundError("Broker Export test did not write its Receipt and Manifest")
        capture_names = {item.name for item in capture_temp_dir.iterdir()}
        if capture_names != {captured_receipt_path.name, captured_manifest_path.name}:
            raise ValueError("Broker Export capture contains unrequested files")
        receipt = json.loads(captured_receipt_path.read_text(encoding="utf-8"))
        manifest_bytes = captured_manifest_path.read_bytes()
        manifest = json.loads(manifest_bytes.decode("utf-8"))
        export_summary = _assert_export_capture(receipt, manifest, manifest_bytes)
        trust = manifest.get("update_trust")
        if not isinstance(trust, dict) or trust.get("public_key_der_hex") != TEST_PUBLIC_KEY_DER_HEX:
            raise ValueError("native Broker Export did not bind the public test key")

        capture_dir = run_root / "broker-export-capture"
        capture_dir.mkdir()
        receipt_path = capture_dir / captured_receipt_path.name
        manifest_path = capture_dir / export_summary["manifest_file_name"]
        shutil.copy2(captured_receipt_path, receipt_path)
        shutil.copy2(captured_manifest_path, manifest_path)
        if _sha256(manifest_path.read_bytes()) != export_summary["manifest_sha256"]:
            raise ValueError("copied Broker Manifest differs from the validated capture")
        resolved_capture_temp = capture_temp_dir.resolve(strict=True)
        if resolved_capture_temp.parent != capture_temp_root:
            raise RuntimeError("refusing to remove Owner capture outside the Windows temp root")
        shutil.rmtree(resolved_capture_temp)
        if resolved_capture_temp.exists():
            raise RuntimeError("Owner capture temp cleanup failed")
        capture_temp_dir = None

        CURRENT_STAGE = "build_normal_release_bundle_and_setup"
        bundle_path = run_root / "normal-release-export"
        initial_package_path = run_root / "initial.d4pkg"
        update_package_path = run_root / "update.d4pkg"
        package_summary = _make_bundle_and_packages(
            receipt_path=receipt_path,
            manifest_path=manifest_path,
            bundle_path=bundle_path,
            initial_package_path=initial_package_path,
            update_package_path=update_package_path,
        )
        setup_output = run_root / "setup-output"
        setup_output.mkdir()
        iscc_path = os.environ.get("D4P_W_E04_ISCC_PATH")
        setup_summary = build_windows_installer.build_windows_installer(
            bundle_path,
            setup_output,
            iscc_path=Path(iscc_path) if iscc_path else None,
        )
        setup_path = Path(setup_summary["setup_path"]).resolve(strict=True)
        setup_bytes = setup_path.read_bytes()
        if setup_summary.get("source_commit") != source_commit:
            raise ValueError("Setup receipt source commit differs from the clean checkout")
        if setup_summary.get("app_id") != package_summary["app_id"] or setup_summary.get(
            "audit_store_id"
        ) != package_summary["audit_store_id"]:
            raise ValueError("Setup and D4PKG identity differ")
        if setup_summary.get("authenticode") != "not_signed_by_this_build_recipe":
            raise ValueError("Setup signature scope is not the expected unsigned developer candidate")

        CURRENT_STAGE = "seed_isolated_broker_candidates"
        compile_env = _child_environment(
            {
                "GUI_SHELL_PRODUCT_APP_ID": package_summary["app_id"],
                "GUI_SHELL_PRODUCT_AUDIT_STORE_ID": package_summary["audit_store_id"],
                "GUI_SHELL_PRODUCT_MANIFEST_SHA256": _sha256(
                    (bundle_path / "product_manifest.json").read_bytes()
                ),
                "GUI_SHELL_UPDATE_TRUST_JSON": json.dumps(
                    trust, ensure_ascii=False, separators=(",", ":")
                ),
                "D4P_WINDOWS_ACCEPTANCE_INITIAL_PACKAGE": str(initial_package_path),
                "D4P_WINDOWS_ACCEPTANCE_UPDATE_PACKAGE": str(update_package_path),
            }
        )
        seed_output = _cargo_test(
            "Windows installed package seed",
            SEED_TEST,
            compile_env,
            show_test_output=True,
        )
        seed_match = re.search(
            r"W-E04_WINDOWS_SEED app_id=(\S+) audit_store_id=(\S+) initial_version=(\S+) update_version=(\S+) initial_package_sha256=([0-9a-f]{64}) update_package_sha256=([0-9a-f]{64}) candidates_signature_verified=true package_cache_verified=true installed_root_absent=true",
            seed_output,
        )
        if seed_match is None:
            raise RuntimeError("Windows Broker seed did not report its bounded acceptance record")
        if seed_match.group(1) != package_summary["app_id"] or seed_match.group(2) != package_summary[
            "audit_store_id"
        ]:
            raise RuntimeError("Windows Broker seed identity differs from the Export package")

        CURRENT_STAGE = "run_normal_installer_product_ui_lifecycle"
        lifecycle_evidence_path = run_root / "windows-ui-lifecycle.json"
        try:
            _run(
                [
                "pwsh",
                "-NoLogo",
                "-NoProfile",
                "-File",
                str(ROOT / "tooling/tests/windows_installed_setup_lifecycle.ps1"),
                "-SetupPath",
                str(setup_path),
                "-RunRoot",
                str(run_root),
                "-EvidencePath",
                str(lifecycle_evidence_path),
                "-AppId",
                package_summary["app_id"],
                "-AuditStoreId",
                package_summary["audit_store_id"],
                "-InitialVersion",
                package_summary["source_version"],
                "-UpdateVersion",
                package_summary["update_version"],
                "-SourceCommit",
                source_commit,
                "-InitialPackageSha256",
                package_summary["packages"]["initial"]["sha256"],
                "-UpdatePackageSha256",
                package_summary["packages"]["update"]["sha256"],
                ],
                env=compile_env,
            )
        except Exception:
            if lifecycle_evidence_path.is_file():
                try:
                    result["lifecycle"] = json.loads(
                        lifecycle_evidence_path.read_text(encoding="utf-8-sig")
                    )
                except (OSError, json.JSONDecodeError):
                    result["lifecycle_evidence_parse_error"] = True
            raise
        lifecycle = json.loads(lifecycle_evidence_path.read_text(encoding="utf-8"))
        _assert_lifecycle_evidence(
            lifecycle,
            source_commit=source_commit,
            app_id=package_summary["app_id"],
            audit_store_id=package_summary["audit_store_id"],
            initial_version=package_summary["source_version"],
            update_version=package_summary["update_version"],
            initial_package_sha256=package_summary["packages"]["initial"]["sha256"],
            update_package_sha256=package_summary["packages"]["update"]["sha256"],
        )

        CURRENT_STAGE = "assemble_scope_bound_evidence"
        result["source"].update(
            {
                "workflow_ref": os.environ.get("GITHUB_REF"),
                "workflow_run_id": os.environ.get("GITHUB_RUN_ID"),
                "workflow_run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
                "runner_os": os.environ.get("RUNNER_OS"),
                "runner_image_os": os.environ.get("ImageOS"),
                "runner_image_version": os.environ.get("ImageVersion"),
                "rustc_version": os.environ.get("D4P_W_E04_RUSTC_VERSION"),
                "cargo_version": os.environ.get("D4P_W_E04_CARGO_VERSION"),
                "flutter_version": os.environ.get("D4P_W_E04_FLUTTER_VERSION"),
                "flutter_revision": os.environ.get("D4P_W_E04_FLUTTER_REVISION"),
                "inno_setup_version": os.environ.get("D4P_W_E04_ISCC_VERSION"),
                "source_manifest_sha256": package_summary["export"]["manifest_sha256"],
            }
        )
        result["export"] = {
            **package_summary["export"],
            "receipt_export_id": export_summary.get("export_id"),
            "receipt_audit_event_id": export_summary.get("audit_event_id"),
        }
        result["setup"] = {
            "setup_sha256": _sha256(setup_bytes),
            "setup_size_bytes": len(setup_bytes),
            "authenticode": setup_summary.get("authenticode"),
            "setup_architecture": setup_summary.get("setup_architecture"),
            "product_app_id": setup_summary.get("app_id"),
            "audit_store_id": setup_summary.get("audit_store_id"),
        }
        result["packages"] = package_summary["packages"]
        result["trust"] = package_summary["trust"]
        result["lifecycle"] = lifecycle
        result["scope"].update(
            {
                "normal_setup_executable_executed": lifecycle.get("setup_executed") is True,
                "normal_setup_first_launch": lifecycle.get("setup_first_launch") is True,
                "initial_install": lifecycle.get("initial_install") is True,
                "update": lifecycle.get("update") is True,
                "repair": lifecycle.get("repair") is True,
                "rollback": lifecycle.get("rollback") is True,
                "uninstall": lifecycle.get("uninstall") is True,
                "stage_or_portable_misrepresented_as_installed": lifecycle.get(
                    "staged_state_not_reported_installed"
                )
                is not True,
                "external_catalog_https_fetched": False,
                "e5_or_e6_claimed": False,
            }
        )
        result["cleanup"] = {
            "runner_scoped_workspace_removed": False,
            "product_identity_state_removed_after_lifecycle_checks": lifecycle.get(
                "test_identity_cleanup"
            )
            is True,
            "setup_temp_and_owned_processes_removed": lifecycle.get(
                "setup_temp_and_process_cleanup"
            )
            is True,
        }
        result["limitations"] = [
            "Update candidates and exact D4PKG bytes were seeded into this unique Product identity's Broker cache; HTTPS catalog retrieval and external package download were not exercised.",
            "Owner dialogs were driven by isolated Developer UI automation. This is not real Owner authority, a separate human Windows profile, formal signing, E5/E6, or release readiness.",
            "The two package envelopes carry successive test versions over the same normal Release bundle payload, so this confirms lifecycle mechanics and rollback selection, not a changed-code update rollout.",
        ]
        result["phase_status"] = {
            "developer_verification": "PASS",
            "w_e04": "OPEN",
            "e2_e6_and_release": "NOT_CLOSED",
        }
        result["failure_stage"] = None
        evidence_path = evidence_dir / "w-e04-windows-setup-installed-lifecycle.json"
        _write_json(evidence_path, result)
        summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
        if summary_path:
            with open(summary_path, "a", encoding="utf-8", newline="\n") as stream:
                stream.write(
                    "\n### W-E04 Windows normal Setup lifecycle\n"
                    f"- Result: `passed`; source `{source_commit}`; App ID `{package_summary['app_id']}`.\n"
                    f"- Setup SHA-256: `{result['setup']['setup_sha256']}`; initial/update D4PKG: `{package_summary['packages']['initial']['sha256']}` / `{package_summary['packages']['update']['sha256']}`.\n"
                    "- Setup launch, Broker-managed install/update/repair/rollback/uninstall, native confirmation dialogs and cleanup were observed by isolated Developer automation.\n"
                    "- Cached signed candidates were used; no remote catalog or package download. Real Owner authority, E5/E6, formal signing and release are not claimed.\n"
                )
        print(f"W-E04_WINDOWS_RESULT={evidence_path}")
        return result
    except Exception as exc:
        result["failure_stage"] = CURRENT_STAGE
        result["failure_type"] = type(exc).__name__
        result["result"] = "failed"
        _write_json(evidence_dir / "w-e04-windows-setup-installed-lifecycle.json", result)
        raise
    finally:
        if capture_temp_dir is not None and capture_temp_dir.exists():
            capture_temp_root = Path(tempfile.gettempdir()).resolve(strict=True)
            resolved_capture_temp = capture_temp_dir.resolve(strict=True)
            if resolved_capture_temp.parent != capture_temp_root:
                raise RuntimeError("refusing to remove Owner capture outside the Windows temp root")
            shutil.rmtree(resolved_capture_temp)
            if resolved_capture_temp.exists():
                raise RuntimeError("Owner capture temp cleanup failed")
        resolved = run_root.resolve(strict=True)
        if not resolved.is_relative_to(runner_temp) or resolved == runner_temp:
            raise RuntimeError("refusing to remove W-E04 run root outside RUNNER_TEMP")
        shutil.rmtree(resolved)
        if resolved.exists():
            raise RuntimeError("W-E04 run root cleanup failed")
        result.setdefault("cleanup", {})["runner_scoped_workspace_removed"] = True
        evidence_path = evidence_dir / "w-e04-windows-setup-installed-lifecycle.json"
        if evidence_path.exists():
            _write_json(evidence_path, result)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--evidence-dir", type=Path, required=True)
    parser.add_argument("--run-root", type=Path, required=True)
    args = parser.parse_args()
    try:
        run(args.source_commit, args.evidence_dir, args.run_root)
        return 0
    except Exception as exc:
        print(f"W-E04 Windows lifecycle failed: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
