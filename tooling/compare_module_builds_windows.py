"""同一commitにおける全画面／選択画面のWindows AOT比較補助。"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import stat
import subprocess
import sys
import time
from pathlib import Path, PurePosixPath, PureWindowsPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.build_module_pruned_windows import (
    BUILD_OUTPUT,
    EXPECTED_OPTIONAL_IDS,
    MODULE_DEFINE_NAMES,
    _artifact_files,
    _flutter_executable,
    _flutter_versions,
    _git_value,
    _load_catalog,
    _parse_json_bytes,
    dart_defines,
    resolve_module_plan,
)


COMPARISON_SCHEMA = ROOT / "specs" / "gui_shell_module_comparison_evidence.schema.json"
ANALYSIS_SENTINEL = "A summary of your Windows bundle analysis can be found at:"
MAX_RECEIPT_BYTES = 1_000_000
MAX_ANALYSIS_BYTES = 50_000_000
APP_CODE_PATH = "data/app.so"
APP_EXECUTABLE = "gui_shell_desktop.exe"
AOT_REPORT_ROOT = "app.so (Dart AOT)"
DART_PACKAGE_ROOT = ("package:gui_shell_desktop", "package:gui_shell_desktop")


def _safe_console_text(value: str, encoding: str | None = None) -> str:
    """現在のWindows console code pageで表せないtool出力をescapeして保持する。"""
    active_encoding = encoding or getattr(sys.stdout, "encoding", None) or "utf-8"
    return value.encode(active_encoding, errors="backslashreplace").decode(active_encoding)


def _console_write(value: str) -> None:
    sys.stdout.write(_safe_console_text(value))


def all_optional_defines() -> dict[str, bool]:
    """Flutterの既定動作と同じ、全任意画面を含む基準buildを表す。"""
    return {
        "GUI_SHELL_MODULE_" + MODULE_DEFINE_NAMES[module_id]: True
        for module_id in EXPECTED_OPTIONAL_IDS
    }


def comparison_summary(
    baseline_files: list[dict[str, Any]], selected_files: list[dict[str, Any]]
) -> dict[str, Any]:
    baseline = {item["path"]: item for item in baseline_files}
    selected = {item["path"]: item for item in selected_files}
    changed = sorted(
        path
        for path in baseline.keys() | selected.keys()
        if baseline.get(path) != selected.get(path)
    )
    baseline_total = sum(item["size_bytes"] for item in baseline_files)
    selected_total = sum(item["size_bytes"] for item in selected_files)
    baseline_code = baseline.get(APP_CODE_PATH)
    selected_code = selected.get(APP_CODE_PATH)
    if baseline_code is None or selected_code is None:
        raise ValueError("両buildにdata/app.soが必要である")
    if APP_EXECUTABLE not in baseline or APP_EXECUTABLE not in selected:
        raise ValueError("両buildにWindows executableが必要である")
    non_aot_changed = [path for path in changed if path != APP_CODE_PATH]
    return {
        "baseline_total_bytes": baseline_total,
        "selected_total_bytes": selected_total,
        "total_bytes_reduced": baseline_total - selected_total,
        "baseline_app_code_bytes": baseline_code["size_bytes"],
        "selected_app_code_bytes": selected_code["size_bytes"],
        "app_code_bytes_reduced": baseline_code["size_bytes"] - selected_code["size_bytes"],
        "baseline_app_code_sha256": baseline_code["sha256"],
        "selected_app_code_sha256": selected_code["sha256"],
        "changed_artifact_paths": changed,
        "non_aot_changed_artifact_paths": non_aot_changed,
        "non_aot_artifacts_identical": not non_aot_changed,
    }


def _aot_surface_libraries(report: dict[str, Any], catalog: dict[str, Any]) -> list[str]:
    """Flutter AOT size report内に存在するCatalog対象Dart libraryを列挙する。"""
    aot_roots: list[dict[str, Any]] = []
    pending: list[Any] = [report]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            if value.get("n") == AOT_REPORT_ROOT:
                aot_roots.append(value)
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)
    if len(aot_roots) != 1:
        raise ValueError("Flutter size-analysis reportにDart AOT treeが一意に存在しない")

    targets: dict[str, tuple[str, ...]] = {}
    lib_root = PurePosixPath("apps/desktop_flutter/lib")
    for module in catalog["optional_modules"]:
        for surface in module["source_surfaces"]:
            source = PurePosixPath(surface)
            try:
                relative = source.relative_to(lib_root)
            except ValueError as exc:
                raise ValueError(f"Catalog surfaceがDesktop Flutter lib外にある: {surface}") from exc
            if not relative.parts or source.suffix != ".dart":
                raise ValueError(f"Catalog surfaceがDart library pathではない: {surface}")
            targets[surface] = (*DART_PACKAGE_ROOT, *relative.parts)

    found: set[str] = set()
    pending_paths: list[tuple[Any, tuple[str, ...]]] = [(aot_roots[0], ())]
    while pending_paths:
        value, ancestors = pending_paths.pop()
        if isinstance(value, dict):
            name = value.get("n")
            path = (*ancestors, name) if isinstance(name, str) else ancestors
            for surface, target in targets.items():
                if len(path) >= len(target) and path[-len(target) :] == target:
                    found.add(surface)
            pending_paths.extend((child, path) for child in value.values())
        elif isinstance(value, list):
            pending_paths.extend((child, ancestors) for child in value)
    return sorted(found)


def validate_comparison_evidence(
    evidence: dict[str, Any],
    schema: dict[str, Any],
    *,
    receipt: dict[str, Any] | None = None,
    catalog: dict[str, Any] | None = None,
) -> list[str]:
    """Schemaに加え、2 buildの選択・file記録・比較値の相互整合を検査する。"""
    from tooling.schema_check.check_schemas import validate_instance

    errors = validate_instance(evidence, schema)
    if errors:
        return errors

    baseline = evidence["baseline"]
    selected = evidence["selected"]
    expected_baseline_defines = all_optional_defines()
    if baseline["effective_dart_defines"] != expected_baseline_defines:
        errors.append("baselineがFlutterの全任意画面既定有効状態と一致しない")
    if baseline["dart_define_arguments"] != {}:
        errors.append("baselineへ任意Moduleのdart-defineを渡している")
    if selected["dart_define_arguments"] != selected["effective_dart_defines"]:
        errors.append("選択buildのdart-define引数と有効値が一致しない")
    if all(selected["effective_dart_defines"].values()):
        errors.append("選択buildが任意Moduleを一つも除外していない")

    active_catalog = catalog if catalog is not None else _load_catalog()
    try:
        all_surfaces = sorted(
            surface
            for module in active_catalog["optional_modules"]
            for surface in module["source_surfaces"]
        )
        expected_baseline_surfaces = all_surfaces
        expected_selected_surfaces = sorted(
            surface
            for module in active_catalog["optional_modules"]
            if selected["effective_dart_defines"].get(
                "GUI_SHELL_MODULE_" + MODULE_DEFINE_NAMES.get(module["module_id"], "")
            )
            for surface in module["source_surfaces"]
        )
        if baseline["aot_surface_libraries"] != expected_baseline_surfaces:
            errors.append("baselineのFlutter AOT reportに全Catalog surface libraryが存在しない")
        if selected["aot_surface_libraries"] != expected_selected_surfaces:
            errors.append("選択buildのFlutter AOT report libraryがModule選択と一致しない")
    except (KeyError, TypeError, ValueError) as exc:
        errors.append(f"Flutter AOT surface library記録を照合できない: {exc}")

    def common_build_arguments(run: dict[str, Any]) -> tuple[list[str], list[str]]:
        command = run["build_command"]
        directories = [
            item.split("=", 1)[1]
            for item in command
            if item.startswith("--code-size-directory=")
        ]
        if len(directories) != 1:
            errors.append(f"{run['label']}にcode-size directoryが一意でない")
        elif (
            PureWindowsPath(directories[0]).name != "size-analysis-inputs"
            or PureWindowsPath(directories[0]).parent.name != run["label"]
        ):
            errors.append(f"{run['label']}のcode-size directoryが証拠出力位置と一致しない")
        if any(item == "--dart-define=" for item in command):
            errors.append(f"{run['label']}に空のdart-define argumentがある")
        common = [
            item
            for item in command
            if not item.startswith("--dart-define=")
            and not item.startswith("--code-size-directory=")
        ]
        required = ["--suppress-analytics", "build", "windows", "--release", "--no-pub", "--analyze-size"]
        if not all(item in command for item in required):
            errors.append(f"{run['label']}の共通Windows Release build条件が不足している")
        return common, directories

    baseline_common, baseline_dirs = common_build_arguments(baseline)
    selected_common, selected_dirs = common_build_arguments(selected)
    if baseline_common != selected_common:
        errors.append("baselineと選択buildの共通command条件が一致しない")
    if baseline_dirs and selected_dirs and baseline_dirs[0] == selected_dirs[0]:
        errors.append("2 buildが同じsize-analysis入力directoryを再利用している")
    expected_selected_args = [
        f"--dart-define={name}={str(value).lower()}"
        for name, value in selected["effective_dart_defines"].items()
    ]
    actual_selected_args = [
        item for item in selected["build_command"] if item.startswith("--dart-define=")
    ]
    if actual_selected_args != expected_selected_args:
        errors.append("選択build commandのdart-defineが記録済み値と一致しない")
    if any(item.startswith("--dart-define=") for item in baseline["build_command"]):
        errors.append("all-enabled baselineはDart defineなしでbuildしなければならない")

    for run in (baseline, selected):
        files = run["artifact_files"]
        by_path = {item["path"]: item for item in files}
        if len(by_path) != len(files):
            errors.append(f"{run['label']} artifact pathが重複している")
        if not {APP_EXECUTABLE, APP_CODE_PATH}.issubset(by_path):
            errors.append(f"{run['label']}にWindows executableまたはDart AOT imageがない")
        if sum(item["size_bytes"] for item in files) != run["artifact_total_bytes"]:
            errors.append(f"{run['label']} artifact合計byte数がfile記録と一致しない")
        tree = hashlib.sha256()
        for item in sorted(files, key=lambda item: item["path"].lower()):
            tree.update(
                f"{item['path']}\0{item['size_bytes']}\0{item['sha256']}\n".encode("utf-8")
            )
        if tree.hexdigest() != run["artifact_tree_sha256"]:
            errors.append(f"{run['label']} artifact tree hashがfile記録と一致しない")
        report_path = run["size_analysis_report"]["path"]
        expected_prefix = run["label"] + "/analysis/"
        if not report_path.startswith(expected_prefix) or not report_path.endswith("/report.json"):
            errors.append(f"{run['label']} size-analysis reportの採取位置が不正である")
        analysis_inputs = run["size_analysis_inputs"]
        input_names = {Path(item["path"]).name for item in analysis_inputs}
        if input_names != {"snapshot.windows-x64.json", "trace.windows-x64.json"}:
            errors.append(f"{run['label']} snapshotとprecompiler traceが揃っていない")
        if any(not item["path"].startswith(expected_prefix + "inputs/") for item in analysis_inputs):
            errors.append(f"{run['label']} size-analysis入力の採取位置が不正である")

    try:
        expected_summary = comparison_summary(
            baseline["artifact_files"], selected["artifact_files"]
        )
        if evidence["comparison"] != expected_summary:
            errors.append("comparison summaryがartifact file記録から再計算した値と一致しない")
    except (KeyError, TypeError, ValueError) as exc:
        errors.append(f"artifact比較を再計算できない: {exc}")

    if receipt is not None:
        try:
            plan = resolve_module_plan(receipt, active_catalog)
            expected_selected_defines = dart_defines(plan, active_catalog)
            if evidence["selected"]["effective_dart_defines"] != expected_selected_defines:
                errors.append("選択buildがReceiptのModulePlan・依存閉包と一致しない")
            if evidence["requested_optional_module_ids"] != list(plan.requested_optional_ids):
                errors.append("証拠の要求Module一覧がReceiptと一致しない")
            if evidence["included_module_ids"] != list(plan.included_module_ids):
                errors.append("証拠の包含Module一覧がReceiptと一致しない")
            if evidence["excluded_optional_module_ids"] != list(plan.excluded_optional_ids):
                errors.append("証拠の除外Module一覧がReceiptと一致しない")
            if evidence["selection_mode"] != plan.selection_mode:
                errors.append("証拠の選択modeがReceiptと一致しない")
        except (OSError, ValueError, KeyError, TypeError) as exc:
            errors.append(f"Receiptとbuild証拠の対応検査に失敗した: {exc}")
    return errors


def _reject_link_or_reparse(path: Path, label: str) -> os.stat_result:
    metadata = path.lstat()
    if stat.S_ISLNK(metadata.st_mode) or getattr(metadata, "st_file_attributes", 0) & 0x400:
        raise ValueError(f"{label}にsymbolic linkまたはreparse pointを使えない")
    return metadata


def _read_receipt(receipt_path: Path) -> tuple[bytes, dict[str, Any]]:
    metadata = _reject_link_or_reparse(receipt_path, "Receipt")
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > MAX_RECEIPT_BYTES:
        raise ValueError("Receiptは通常fileであり1 MB以下でなければならない")
    raw = receipt_path.read_bytes()
    if len(raw) > MAX_RECEIPT_BYTES:
        raise ValueError("Receiptが1 MB上限を超えている")
    value = _parse_json_bytes(raw)
    if not isinstance(value, dict):
        raise ValueError("Receipt入力はJSON objectでなければならない")
    return raw, value


def _destination_path(path: Path) -> Path:
    if path.exists() or path.is_symlink():
        raise ValueError("artifact出力先が既に存在するため上書きを拒否した")
    resolved = path.resolve()
    try:
        resolved.relative_to(ROOT.resolve())
    except ValueError:
        return resolved
    raise ValueError("比較artifactはRepository外へ出力しなければならない")


def _file_record(path: Path, evidence_root: Path) -> dict[str, Any]:
    metadata = _reject_link_or_reparse(path, "size analysis出力")
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > MAX_ANALYSIS_BYTES:
        raise ValueError("size analysis出力が通常fileでないか50 MB上限を超えている")
    raw = path.read_bytes()
    relative = path.relative_to(evidence_root).as_posix()
    return {"path": relative, "size_bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def _analysis_report_path(output: str) -> Path:
    matches = [
        line.split(ANALYSIS_SENTINEL, 1)[1].strip()
        for line in output.splitlines()
        if ANALYSIS_SENTINEL in line
    ]
    if not matches or not matches[-1]:
        raise ValueError("FlutterがWindows size-analysis report pathを出力しなかった")
    report_path = Path(matches[-1].strip('"'))
    if not report_path.is_absolute():
        raise ValueError("Flutter size-analysis report pathが絶対pathではない")
    metadata = _reject_link_or_reparse(report_path, "Flutter size-analysis report")
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > MAX_ANALYSIS_BYTES:
        raise ValueError("Flutter size-analysis reportが通常fileでないか50 MB上限を超えている")
    analysis_root = Path.home() / ".flutter-devtools"
    _reject_link_or_reparse(analysis_root, "Flutter size-analysis directory")
    try:
        report_path.resolve(strict=True).relative_to(analysis_root.resolve(strict=True))
    except (OSError, ValueError) as exc:
        raise ValueError("Flutter size-analysis reportが管理対象directoryの外にある") from exc
    return report_path


def _clean_source_commit(expected_commit: str | None = None) -> str:
    if _git_value("status", "--porcelain", "--untracked-files=all"):
        raise ValueError("比較証拠の生成にはcleanなcommit済みworktreeが必要である")
    commit = _git_value("rev-parse", "HEAD")
    if expected_commit is not None and commit != expected_commit:
        raise ValueError("比較build中にsource commitが変化した")
    return commit


def _run_build(
    *,
    executable: str,
    defines: dict[str, bool],
    profile_input_dir: Path,
    cwd: Path,
) -> tuple[list[str], int, Path, list[Path], int, str]:
    profile_input_dir.mkdir(parents=True, exist_ok=False)
    command = [
        executable,
        "--suppress-analytics",
        "build",
        "windows",
        "--release",
        "--no-pub",
        "--analyze-size",
        f"--code-size-directory={profile_input_dir}",
    ]
    if defines:
        command.extend(
            f"--dart-define={name}={str(enabled).lower()}"
            for name, enabled in defines.items()
        )
    started = time.perf_counter()
    completed = subprocess.run(
        command,
        cwd=cwd,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        encoding="utf-8",
        errors="replace",
    )
    elapsed_ms = round((time.perf_counter() - started) * 1000)
    if completed.stdout:
        _console_write(completed.stdout)
    if completed.returncode != 0:
        raise subprocess.CalledProcessError(completed.returncode, command)

    report_path = _analysis_report_path(completed.stdout)
    report_bytes = report_path.read_bytes()
    report = _parse_json_bytes(report_bytes)
    if not isinstance(report, dict) or report.get("type") != "windows":
        raise ValueError("Flutter size-analysis reportがWindows形式でない")

    snapshots = sorted(profile_input_dir.glob("snapshot.*.json"))
    traces = sorted(profile_input_dir.glob("trace.*.json"))
    if len(snapshots) != 1 or len(traces) != 1 or snapshots[0].stem != traces[0].stem.replace("trace", "snapshot", 1):
        raise ValueError("Flutter size-analysisのsnapshotとprecompiler traceが一意に揃わない")
    for path in [*snapshots, *traces]:
        metadata = _reject_link_or_reparse(path, "size-analysis入力")
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > MAX_ANALYSIS_BYTES:
            raise ValueError("size-analysis入力が通常fileでないか50 MB上限を超えている")

    return (
        command,
        elapsed_ms,
        report_path,
        [*snapshots, *traces],
        len(report_bytes),
        hashlib.sha256(report_bytes).hexdigest(),
    )


def _capture_run(
    *,
    label: str,
    output_root: Path,
    command_defines: dict[str, bool],
    effective_defines: dict[str, bool],
    executable: str,
    evidence_root: Path,
    catalog: dict[str, Any],
) -> dict[str, Any]:
    run_root = output_root / label
    run_root.mkdir(parents=True, exist_ok=False)
    profile_input_dir = run_root / "size-analysis-inputs"
    command, duration_ms, report_source, input_files, report_size, report_hash = _run_build(
        executable=executable,
        defines=command_defines,
        profile_input_dir=profile_input_dir,
        cwd=ROOT / "apps" / "desktop_flutter",
    )
    aot_surface_libraries = _aot_surface_libraries(
        _parse_json_bytes(report_source.read_bytes()), catalog
    )
    if not (BUILD_OUTPUT / APP_EXECUTABLE).is_file() or not (BUILD_OUTPUT / APP_CODE_PATH).is_file():
        raise ValueError("Windows release出力にapp executableまたはDart AOT imageがない")
    build_metadata = _reject_link_or_reparse(BUILD_OUTPUT, "Windows build output")
    if not stat.S_ISDIR(build_metadata.st_mode):
        raise ValueError("Windows build outputがdirectoryではない")

    source_files, total_bytes, tree_hash = _artifact_files(BUILD_OUTPUT)
    app_destination = run_root / "app"
    shutil.copytree(BUILD_OUTPUT, app_destination)
    copied_files, copied_bytes, copied_tree_hash = _artifact_files(app_destination)
    if (copied_files, copied_bytes, copied_tree_hash) != (source_files, total_bytes, tree_hash):
        raise ValueError(f"{label} artifact copyがFlutter build outputと一致しない")

    analysis_dir = run_root / "analysis"
    analysis_dir.mkdir(exist_ok=False)
    report_destination = analysis_dir / "report.json"
    shutil.copyfile(report_source, report_destination)
    report_record = _file_record(report_destination, evidence_root)
    if (report_record["size_bytes"], report_record["sha256"]) != (report_size, report_hash):
        raise ValueError(f"{label} size-analysis report copyが元fileと一致しない")

    input_records: list[dict[str, Any]] = []
    copied_input_root = analysis_dir / "inputs"
    copied_input_root.mkdir(exist_ok=False)
    for source in input_files:
        destination = copied_input_root / source.name
        shutil.copyfile(source, destination)
        record = _file_record(destination, evidence_root)
        if hashlib.sha256(source.read_bytes()).hexdigest() != record["sha256"]:
            raise ValueError(f"{label} size-analysis input copyが元fileと一致しない")
        input_records.append(record)

    return {
        "label": label,
        "dart_define_arguments": command_defines,
        "effective_dart_defines": effective_defines,
        "build_command": command,
        "build_duration_ms": duration_ms,
        "artifact_root": (run_root / "app").relative_to(evidence_root).as_posix(),
        "artifact_files": copied_files,
        "artifact_total_bytes": copied_bytes,
        "artifact_tree_sha256": copied_tree_hash,
        "aot_surface_libraries": aot_surface_libraries,
        "size_analysis_report": report_record,
        "size_analysis_inputs": input_records,
    }


def _comparison_evidence(receipt_path: Path, output_dir: Path) -> dict[str, Any]:
    if sys.platform != "win32":
        raise ValueError("Windows comparison buildはWindows専用である")
    source_commit = _clean_source_commit()
    output_dir = _destination_path(output_dir)

    receipt_bytes, receipt = _read_receipt(receipt_path)
    catalog = _load_catalog()
    plan = resolve_module_plan(receipt, catalog)
    selected_defines = dart_defines(plan, catalog)
    if not plan.excluded_optional_ids:
        raise ValueError("比較buildでは1つ以上の任意Moduleを除外するReceiptが必要である")
    baseline_defines = all_optional_defines()
    if set(baseline_defines) != set(selected_defines):
        raise ValueError("baselineと選択buildのcompile-time define集合が異なる")

    executable = _flutter_executable()
    versions = _flutter_versions(executable)
    output_dir.mkdir(parents=True, exist_ok=False)
    baseline = _capture_run(
        label="baseline",
        output_root=output_dir,
        command_defines={},
        effective_defines=baseline_defines,
        executable=executable,
        evidence_root=output_dir,
        catalog=catalog,
    )
    _clean_source_commit(source_commit)
    selected = _capture_run(
        label="selected",
        output_root=output_dir,
        command_defines=selected_defines,
        effective_defines=selected_defines,
        executable=executable,
        evidence_root=output_dir,
        catalog=catalog,
    )
    _clean_source_commit(source_commit)
    if _flutter_versions(executable) != versions:
        raise ValueError("比較build中にFlutter toolchain versionが変化した")
    summary = comparison_summary(baseline["artifact_files"], selected["artifact_files"])
    evidence = {
        "version": 1,
        "evidence_type": "development_module_build_comparison",
        "evidence_source": "INTERNAL_STATE",
        "product_artifact_claimed": False,
        "standalone_app_claimed": False,
        "selection_input_trust": "unverified_receipt_json_selection_only",
        "authority_verified": False,
        "source_commit": source_commit,
        "source_receipt_sha256": hashlib.sha256(receipt_bytes).hexdigest(),
        "worktree_clean": True,
        "target_platform": "windows",
        "build_mode": "release",
        "build_scope": "desktop_flutter_ui_only",
        "same_source_commit": True,
        "same_toolchain": True,
        "toolchain": versions,
        "catalog_version": catalog["version"],
        "selection_mode": plan.selection_mode,
        "requested_optional_module_ids": list(plan.requested_optional_ids),
        "included_module_ids": list(plan.included_module_ids),
        "excluded_optional_module_ids": list(plan.excluded_optional_ids),
        "baseline_selection": "all_optional_default_enabled",
        "baseline": baseline,
        "selected": selected,
        "comparison": summary,
        "binary_pruning_verified": False,
        "semantic_pruning_status": "flutter_aot_report_surface_libraries_match_selection; runtime_unverified",
        "cold_start_status": "not_measured",
        "resource_comparison_status": "not_measured",
        "signed": False,
    }
    schema = json.loads(COMPARISON_SCHEMA.read_text(encoding="utf-8"))
    errors = validate_comparison_evidence(evidence, schema, receipt=receipt, catalog=catalog)
    if errors:
        raise ValueError("比較証拠がContractに適合しない: " + "; ".join(errors))
    (output_dir / "comparison_evidence.json").write_text(
        json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    return evidence


def main() -> int:
    parser = argparse.ArgumentParser(
        description="同一commitの全任意画面baselineと選択画面のWindows Release AOTを比較するDeveloper専用tool。"
    )
    parser.add_argument("--receipt", type=Path, required=True, help="画面選択としてのみ使う未信頼Export Receipt JSON")
    parser.add_argument("--artifact-dir", type=Path, required=True, help="まだ存在しないRepository外の出力directory")
    args = parser.parse_args()
    receipt_path = args.receipt if args.receipt.is_absolute() else ROOT / args.receipt
    output_dir = args.artifact_dir if args.artifact_dir.is_absolute() else ROOT / args.artifact_dir
    try:
        evidence = _comparison_evidence(receipt_path, output_dir)
    except (OSError, ValueError, json.JSONDecodeError, subprocess.CalledProcessError) as exc:
        message = f"Module比較buildを停止した: {exc}\n"
        encoding = getattr(sys.stderr, "encoding", None) or "utf-8"
        sys.stderr.write(message.encode(encoding, errors="backslashreplace").decode(encoding))
        return 1
    _console_write(json.dumps({
        "comparison_evidence": str(output_dir.resolve() / "comparison_evidence.json"),
        "source_commit": evidence["source_commit"],
        "baseline_bytes": evidence["comparison"]["baseline_total_bytes"],
        "selected_bytes": evidence["comparison"]["selected_total_bytes"],
        "bytes_reduced": evidence["comparison"]["total_bytes_reduced"],
        "app_code_bytes_reduced": evidence["comparison"]["app_code_bytes_reduced"],
        "binary_pruning_verified": False,
        "cold_start_status": "not_measured",
        "resource_comparison_status": "not_measured",
    }, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
