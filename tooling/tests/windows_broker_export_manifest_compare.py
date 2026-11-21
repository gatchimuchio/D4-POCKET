from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any, Iterator


ROOT = Path(__file__).resolve().parents[2]
TEST_FILTER = (
    "desktop_launcher::tests::"
    "nativeOwner確認dialogのExportNoYesをBrokerReceiptManifestまで接続する"
)
EVIDENCE_NAME = "w-e02-windows-broker-export-manifest-consumer.json"
CURRENT_STAGE = "source_identity"


def _run(command: list[str], *, cwd: Path, env: dict[str, str] | None = None) -> str:
    print(f"::group::{Path(command[0]).name} {' '.join(command[1:])}", flush=True)
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


def _git(args: list[str]) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=ROOT,
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


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _write_json(path: Path, value: dict[str, Any]) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def _symbol_records(value: Any) -> Iterator[dict[str, Any]]:
    if isinstance(value, dict):
        for key, item in value.items():
            if key in {"removed_symbols_file", "added_symbols_file"}:
                if isinstance(item, dict):
                    yield item
            else:
                yield from _symbol_records(item)
    elif isinstance(value, list):
        for item in value:
            yield from _symbol_records(item)


def _copy_symbol_evidence(source_root: Path, destination_root: Path, evidence: dict[str, Any]) -> int:
    copied: set[str] = set()
    source_prefix = source_root.resolve(strict=True)
    for record in _symbol_records(evidence):
        relative = PurePosixPath(str(record.get("path", "")))
        if relative.is_absolute() or not relative.parts or ".." in relative.parts:
            raise ValueError("比較証拠のsymbol file pathが不正です")
        source = (source_root / Path(*relative.parts)).resolve(strict=True)
        if not source.is_relative_to(source_prefix) or not source.is_file():
            raise ValueError("比較証拠のsymbol fileがartifact directory外です")
        raw = source.read_bytes()
        if _sha256(raw) != record.get("sha256") or len(raw) != record.get("size_bytes"):
            raise ValueError("比較証拠のsymbol file hash／sizeが一致しません")
        key = relative.as_posix()
        if key in copied:
            continue
        destination = destination_root / Path(*relative.parts)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        copied.add(key)
    return len(copied)


def _assert_export_capture(receipt: Any, manifest: Any, raw: bytes) -> dict[str, Any]:
    if not isinstance(receipt, dict) or not isinstance(manifest, dict):
        raise ValueError("Broker Export captureのJSON構造が不正です")
    file_receipt = receipt.get("manifest_file")
    if not isinstance(file_receipt, dict):
        raise ValueError("Broker ReceiptにManifest file情報がありません")
    manifest_body = manifest.get("manifest")
    if not isinstance(manifest_body, dict):
        raise ValueError("Broker生成Manifestのbodyがありません")
    identity = manifest_body.get("app_identity")
    audit_store = manifest_body.get("audit_store")
    inheritance = manifest_body.get("inheritance_policy")
    if not isinstance(identity, dict) or not isinstance(audit_store, dict):
        raise ValueError("Broker生成ManifestのidentityまたはAudit storeがありません")
    if not isinstance(inheritance, dict):
        raise ValueError("Broker生成Manifestのauthority inheritance policyがありません")

    app_id = identity.get("app_id")
    file_name = file_receipt.get("file_name")
    raw_sha256 = _sha256(raw)
    if receipt.get("status") != "accepted" or receipt.get("operation") != "GUI Shell書出し":
        raise ValueError("Broker Export Receiptはacceptedではありません")
    if receipt.get("manifest_file_status") != "written":
        raise ValueError("BrokerがManifest fileを書き出していません")
    if receipt.get("build_status") != "not_started" or receipt.get("artifact_status") != "not_built":
        raise ValueError("Export ProducerがBuilderを先行実行しています")
    if receipt.get("authority_strip") is not True:
        raise ValueError("Export Receiptのauthority stripが成立しません")
    for field in (
        "credential_inherited",
        "permission_inherited",
        "approval_inherited",
        "audit_chain_inherited",
    ):
        if receipt.get(field) is not False:
            raise ValueError(f"Export Receiptで{field}がfalseではありません")
    if not isinstance(app_id, str) or not re.fullmatch(r"d4-pocket-app-[0-9a-f]{32}", app_id):
        raise ValueError("Export Manifestの新規App ID形式が不正です")
    if file_name != f"{app_id}.json":
        raise ValueError("Receipt filenameとManifest App IDが一致しません")
    if identity.get("target_platform") != "windows":
        raise ValueError("Export Manifestのtarget platformがWindowsではありません")
    if receipt.get("export_manifest", {}).get("app_identity", {}).get("app_id") != app_id:
        raise ValueError("ReceiptとBroker生成ManifestのApp IDが一致しません")
    if receipt.get("export_manifest", {}).get("audit_store", {}).get("store_id") != audit_store.get("store_id"):
        raise ValueError("ReceiptとBroker生成ManifestのAudit store IDが一致しません")
    if file_receipt.get("byte_length") != len(raw):
        raise ValueError("ReceiptとBroker生成Manifestのbyte lengthが一致しません")
    if file_receipt.get("sha256") != f"sha256:{raw_sha256}":
        raise ValueError("Broker ReceiptのManifest file hashが一致しません")
    for policy in ("authority", "permission", "approval", "credential", "audit_chain"):
        if inheritance.get(policy) != "none":
            raise ValueError(f"Export Manifestで{policy}が継承されています")
    audit_id = receipt.get("audit_id")
    if not isinstance(audit_id, str) or not audit_id:
        raise ValueError("Export ReceiptにAudit event IDがありません")

    return {
        "export_id": receipt.get("export_id"),
        "app_id": app_id,
        "audit_store_id": audit_store.get("store_id"),
        "manifest_file_name": file_name,
        "manifest_byte_length": len(raw),
        "manifest_sha256": raw_sha256,
        "audit_event_id": audit_id,
    }


def _run_acceptance(source_commit: str, evidence_dir: Path) -> dict[str, Any]:
    global CURRENT_STAGE
    CURRENT_STAGE = "source_identity"
    if sys.platform != "win32":
        raise RuntimeError("この検収scriptはWindows runner専用です")
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise ValueError("source commitの形式が不正です")
    if _git(["rev-parse", "HEAD"]) != source_commit:
        raise RuntimeError("checkout HEADと要求source commitが一致しません")
    if _git(["branch", "--show-current"]) != "main":
        raise RuntimeError("このworkflowはmainからだけ実行できます")
    if _git(["rev-parse", "refs/remotes/origin/main"]) != source_commit:
        raise RuntimeError("origin/mainとcheckout HEADが一致しません")
    if _git(["status", "--porcelain", "--untracked-files=all"]):
        raise RuntimeError("検収開始時のsource treeがcleanではありません")

    runner_temp = Path(os.environ["RUNNER_TEMP"]).resolve(strict=True)
    evidence_dir = evidence_dir.resolve(strict=False)
    if not evidence_dir.is_relative_to(runner_temp) or evidence_dir == runner_temp:
        raise ValueError("evidence directoryはRUNNER_TEMPの専用childに限定します")
    if evidence_dir.exists():
        raise FileExistsError("evidence directoryが既にあります")
    evidence_dir.mkdir(parents=True)

    test_summary: dict[str, Any]
    export_summary: dict[str, Any]
    builder_summary: dict[str, Any]
    symbol_file_count: int

    with tempfile.TemporaryDirectory(
        prefix="d4p-w-e02-native-export-", dir=tempfile.gettempdir()
    ) as capture_name:
        CURRENT_STAGE = "native_win32_export_test"
        capture_dir = Path(capture_name)
        env = os.environ.copy()
        env["GUI_SHELL_EXPORT_E2E_CAPTURE_DIR"] = str(capture_dir)
        cargo_output = _run(
            [
                "cargo",
                "test",
                "--locked",
                "--manifest-path",
                "native/rust_helper/Cargo.toml",
                "--lib",
                "--",
                "--ignored",
                "--exact",
                TEST_FILTER,
                "--test-threads=1",
            ],
            cwd=ROOT,
            env=env,
        )
        if not re.search(r"test result: ok\. 1 passed; 0 failed;", cargo_output):
            raise RuntimeError("Win32 Export Owner dialog testは1件成功していません")
        receipt_path = capture_dir / "d4_pocket_build_receipt.json"
        manifest_source = capture_dir / "broker-generated-manifest.json"
        if not receipt_path.is_file() or not manifest_source.is_file():
            raise FileNotFoundError("成功したBroker testのReceipt／Manifest captureがありません")
        unexpected = {item.name for item in capture_dir.iterdir()} - {
            receipt_path.name,
            manifest_source.name,
        }
        if unexpected:
            raise ValueError("Broker test captureに未定義fileがあります")
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        manifest_bytes = manifest_source.read_bytes()
        manifest = json.loads(manifest_bytes)
        CURRENT_STAGE = "broker_manifest_receipt_binding"
        export_summary = _assert_export_capture(receipt, manifest, manifest_bytes)

        with tempfile.TemporaryDirectory(
            prefix="d4p-w-e02-builder-", dir=runner_temp
        ) as build_name:
            CURRENT_STAGE = "windows_full_pruned_builder"
            build_root = Path(build_name)
            manifest_input = build_root / export_summary["manifest_file_name"]
            manifest_input.write_bytes(manifest_bytes)
            comparison_dir = build_root / "windows-comparison"
            compare_output = _run(
                [
                    sys.executable,
                    "-X",
                    "utf8",
                    "tooling/compare_module_builds_windows.py",
                    "--manifest-file",
                    str(manifest_input),
                    "--artifact-dir",
                    str(comparison_dir),
                ],
                cwd=ROOT,
                env=os.environ.copy(),
            )
            comparison_path = comparison_dir / "comparison_evidence.json"
            if not comparison_path.is_file():
                raise FileNotFoundError("Windows比較Builderの証拠JSONがありません")
            comparison_bytes = comparison_path.read_bytes()
            comparison = json.loads(comparison_bytes)
            if comparison.get("evidence_type") != "development_module_build_comparison":
                raise ValueError("Windows比較Builderのevidence typeが不正です")
            if comparison.get("source_commit") != source_commit:
                raise ValueError("Builder source commitが検収candidateと異なります")
            if comparison.get("source_manifest_sha256") != export_summary["manifest_sha256"]:
                raise ValueError("Broker Manifest SHAとBuilder input SHAが一致しません")
            if comparison.get("selection_input_trust") != "unverified_manifest_json_selection_only":
                raise ValueError("BuilderがManifest selectionをuntrusted入力として扱っていません")
            if comparison.get("authority_verified") is not False:
                raise ValueError("BuilderがManifestにAuthorityを与えています")
            if comparison.get("source_receipt_sha256") is not None:
                raise ValueError("Manifest-only Builderへ別Receiptが混入しました")
            if comparison.get("same_source_commit") is not True or comparison.get("same_toolchain") is not True:
                raise ValueError("Full／Pruned buildのsourceまたはtoolchain同一性がありません")
            if comparison.get("worktree_clean") is not True:
                raise ValueError("比較Builderがclean sourceで実行されていません")
            requested = comparison.get("requested_optional_module_ids")
            included = comparison.get("included_module_ids")
            excluded = comparison.get("excluded_optional_module_ids")
            if requested != ["shell.history"] or not isinstance(included, list) or not isinstance(excluded, list):
                raise ValueError("Broker Export選択がWindows Builderへ正しく伝わっていません")
            if "shell.history" not in included or "shell.history" in excluded or not excluded:
                raise ValueError("選択Moduleと除外Moduleの実Planが不正です")

            comparison_out = evidence_dir / "comparison"
            comparison_out.mkdir()
            (comparison_out / comparison_path.name).write_bytes(comparison_bytes)
            symbol_file_count = _copy_symbol_evidence(
                comparison_dir, comparison_out, comparison
            )
            export_summary["builder_manifest_sha256"] = comparison[
                "source_manifest_sha256"
            ]
            builder_summary = {
                "tool": "tooling/compare_module_builds_windows.py",
                "evidence_type": comparison["evidence_type"],
                "evidence_source": comparison.get("evidence_source"),
                "selection_input_trust": comparison["selection_input_trust"],
                "authority_verified": comparison["authority_verified"],
                "same_source_commit": comparison["same_source_commit"],
                "same_toolchain": comparison["same_toolchain"],
                "worktree_clean": comparison["worktree_clean"],
                "requested_optional_module_ids": requested,
                "included_module_ids": included,
                "excluded_optional_module_ids": excluded,
                "comparison": comparison.get("comparison"),
                "resource_inventory_comparison": comparison.get(
                    "resource_inventory_comparison"
                ),
                "package_dependency_comparison": comparison.get(
                    "package_dependency_comparison"
                ),
                "binary_pruning_verified": comparison.get(
                    "binary_pruning_verified"
                ),
                "comparison_evidence_sha256": _sha256(comparison_bytes),
                "symbol_evidence_file_count": symbol_file_count,
            }
            test_summary = {
                "command": "cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib -- --ignored --exact <Export Owner dialog test> --test-threads=1",
                "test_filter": TEST_FILTER,
                "result": "passed",
                "passed_test_count": 1,
                "real_owner_authority": False,
                "confirmation_actor": "isolated test harness controlling the native Win32 No/Yes dialog",
                "assertions_executed_by_test": [
                    "No拒否時owner_requiredとなりManifestを作らない",
                    "Yes選択時にproduction BrokerがManifestを生成する",
                    "Receiptと生成ManifestのApp ID／Audit store ID／Manifest SHAを照合する",
                    "Broker Audit fileに拒否／受理要求を確認し、private marker不在を確認する",
                    "Authority／Permission／Approval／Credential／Audit chainを継承しない",
                ],
            }

    CURRENT_STAGE = "source_cleanup"
    if _git(["status", "--porcelain", "--untracked-files=all"]):
        raise RuntimeError("検収後にsource treeがcleanではありません")

    CURRENT_STAGE = "evidence_finalize"
    result = {
        "evidence_type": "w_e02_windows_broker_export_manifest_to_compare_builder",
        "result": "passed",
        "evidence_source": "INTERNAL_STATE",
        "source_commit": source_commit,
        "runner": {
            "os": platform.platform(),
            "image": f"{os.environ.get('ImageOS', 'unknown')}/{os.environ.get('ImageVersion', 'unknown')}",
            "python": platform.python_version(),
        },
        "producer": test_summary,
        "export": export_summary,
        "builder": builder_summary,
        "scope": {
            "same_run_broker_manifest_to_windows_builder": True,
            "real_installed_product": False,
            "real_flutter_settings_ui": False,
            "real_owner_authority": False,
            "normal_release_capability": False,
            "binary_pruning_verified": False,
        },
        "cleanup": {
            "broker_test_capture_removed": True,
            "manifest_staging_removed": True,
            "comparison_build_outputs_removed": True,
            "durable_evidence_contains_build_binaries": False,
        },
    }
    evidence_path = evidence_dir / EVIDENCE_NAME
    _write_json(evidence_path, result)
    test_summary["result"] = "passed"
    print(f"W-E02_RESULT={evidence_path}")
    print(f"W-E02_MANIFEST_SHA256={export_summary['manifest_sha256']}")
    print(f"W-E02_COMPARISON_SHA256={builder_summary['comparison_evidence_sha256']}")
    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_path:
        summary = [
            "### W-E02 Broker Export Manifest → Windows Builder",
            f"- Result: `passed`; source: `{source_commit}`.",
            f"- Broker-generated Manifest: `{export_summary['manifest_file_name']}`, SHA-256 `{export_summary['manifest_sha256']}`.",
            f"- Builder input SHA-256: `{builder_summary['comparison_evidence_sha256']}`; selection trust `{builder_summary['selection_input_trust']}`; Authority verified `{builder_summary['authority_verified']}`.",
            f"- Requested/included/excluded optional modules: `{requested}` / `{included}` / `{excluded}`.",
            "- Scope: hosted Windows internal-state evidence. No installed-product, Flutter Settings UI, real Owner authority, normal Release capability, or binary-pruning claim.",
        ]
        with open(summary_path, "a", encoding="utf-8", newline="\n") as stream:
            stream.write("\n".join(summary) + "\n")
    return result


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run the native Broker Export test and feed its exact Manifest to the Windows Full/Pruned Builder."
    )
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--evidence-dir", type=Path, required=True)
    args = parser.parse_args()

    try:
        if args.evidence_dir.exists():
            raise FileExistsError("evidence output directory already exists")
        # The stage is included in a failure record without copying test payloads or logs.
        result = _run_acceptance(args.source_commit, args.evidence_dir)
        return 0 if result["result"] == "passed" else 1
    except Exception as exc:
        try:
            if args.evidence_dir.exists() and args.evidence_dir.is_dir():
                failure = {
                    "evidence_type": "w_e02_windows_broker_export_manifest_to_compare_builder",
                    "result": "failed",
                    "source_commit": args.source_commit,
                    "failure_type": type(exc).__name__,
                    "failure_stage": CURRENT_STAGE,
                }
                _write_json(args.evidence_dir / EVIDENCE_NAME, failure)
        except OSError:
            pass
        print(f"W-E02 acceptance failed: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
