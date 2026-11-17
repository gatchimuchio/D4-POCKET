"""同一commitにおけるFlutter AOTとRust helperのWindows Release比較補助。"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import time
from pathlib import Path, PurePosixPath, PureWindowsPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.build_module_pruned_windows import (
    BUILD_OUTPUT,
    EXPECTED_CORE_IDS,
    EXPECTED_OPTIONAL_IDS,
    EXPECTED_REQUIRED_IDS,
    MODULE_DEFINE_NAMES,
    ResolvedModulePlan,
    _artifact_files,
    _flutter_executable,
    _flutter_versions,
    _git_value,
    _load_catalog,
    _parse_json_bytes,
    cargo_feature_arguments,
    dart_defines,
    resolve_module_plan_for_optional_ids,
    resolve_module_plan,
)
from tooling.build_environment import build_child_environment
from tooling.flutter_workspace import safe_flutter_temp_parent


COMPARISON_SCHEMA = ROOT / "specs" / "gui_shell_module_comparison_evidence.schema.json"
COMPARISON_EVIDENCE_VERSION = 4
ANALYSIS_SENTINEL = "A summary of your Windows bundle analysis can be found at:"
MAX_RECEIPT_BYTES = 1_000_000
MAX_ANALYSIS_BYTES = 50_000_000
APP_CODE_PATH = "data/app.so"
APP_EXECUTABLE = "gui_shell_desktop.exe"
AOT_REPORT_ROOT = "app.so (Dart AOT)"
DART_PACKAGE_ROOT = ("package:gui_shell_desktop", "package:gui_shell_desktop")
RUST_HELPER_BINARY = "gui_shell_rust_helper.exe"
RUST_HELPER_MANIFEST = "native/rust_helper/Cargo.toml"
MAX_NATIVE_BINARY_BYTES = 512_000_000
MAP_SYMBOL_LINE = re.compile(
    r"^\s*[0-9a-fA-F]+:[0-9a-fA-F]+\s+(\S+)\s+[0-9a-fA-F]+(?:\s|$)"
)


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


def _resource_files(artifact_files: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Windows Flutter bundleに実在するflutter_assets fileを返す。"""
    return sorted(
        (
            item
            for item in artifact_files
            if item["path"].startswith("data/flutter_assets/")
        ),
        key=lambda item: item["path"],
    )


def resource_inventory_comparison(
    baseline_files: list[dict[str, Any]], selected_files: list[dict[str, Any]]
) -> dict[str, Any]:
    baseline = _resource_files(baseline_files)
    selected = _resource_files(selected_files)
    baseline_by_path = {item["path"]: item for item in baseline}
    selected_by_path = {item["path"]: item for item in selected}
    changed = sorted(
        path
        for path in baseline_by_path.keys() | selected_by_path.keys()
        if baseline_by_path.get(path) != selected_by_path.get(path)
    )
    return {
        "baseline_resource_bytes": sum(item["size_bytes"] for item in baseline),
        "selected_resource_bytes": sum(item["size_bytes"] for item in selected),
        "changed_resource_paths": changed,
        "resource_inventory_identical": not changed,
    }


def _runtime_dependency_tree_from_json(value: dict[str, Any]) -> dict[str, Any]:
    """dart pub deps JSONからrootのruntime依存閉包を正規化する。"""
    packages = value.get("packages")
    if not isinstance(packages, list):
        raise ValueError("dart pub deps JSONにpackages一覧がない")
    if any(
        not isinstance(item, dict) or not isinstance(item.get("name"), str)
        for item in packages
    ):
        raise ValueError("dart pub deps JSONに不正なpackage entryがある")
    entries = {item["name"]: item for item in packages}
    if len(entries) != len(packages):
        raise ValueError("dart pub deps JSONに重複package nodeがある")
    root = entries.get(value.get("root"))
    if (
        root is None
        or root.get("kind") != "root"
        or root.get("name") != value.get("root")
    ):
        raise ValueError("dart pub deps JSONのroot packageが特定できない")
    direct = root.get("directDependencies")
    if not isinstance(direct, list) or any(not isinstance(item, str) for item in direct):
        raise ValueError("dart pub deps JSONのdirectDependenciesが不正である")
    if len(direct) != len(set(direct)):
        raise ValueError("dart pub deps JSONのdirectDependenciesに重複がある")

    pending = list(direct)
    visited: set[str] = set()
    normalized: list[dict[str, Any]] = []
    while pending:
        name = pending.pop()
        if name in visited:
            continue
        entry = entries.get(name)
        if entry is None:
            raise ValueError(f"runtime依存packageがpub deps graphに存在しない: {name}")
        dependencies = entry.get("dependencies", [])
        if not isinstance(dependencies, list) or any(
            not isinstance(item, str) for item in dependencies
        ):
            raise ValueError(f"runtime依存packageのedgeが不正である: {name}")
        if len(dependencies) != len(set(dependencies)):
            raise ValueError(f"runtime依存packageのedgeが重複している: {name}")
        if not isinstance(entry.get("source"), str) or not entry["source"]:
            raise ValueError(f"runtime依存packageのsourceが不正である: {name}")
        if not isinstance(entry.get("version"), str):
            raise ValueError(f"runtime依存packageのversionが不正である: {name}")
        visited.add(name)
        pending.extend(dependencies)
        normalized.append(
            {
                "name": name,
                "version": str(entry.get("version", "")),
                "source": str(entry.get("source", "")),
                "dependencies": sorted(set(dependencies)),
            }
        )

    direct_dependencies = sorted(set(direct))
    normalized.sort(key=lambda item: item["name"])
    digest_input = {
        "root_package": root["name"],
        "direct_runtime_dependencies": direct_dependencies,
        "runtime_packages": normalized,
    }
    digest = hashlib.sha256(
        json.dumps(
            digest_input, ensure_ascii=False, sort_keys=True, separators=(",", ":")
        ).encode("utf-8")
    ).hexdigest()
    return {
        "source": "dart_pub_deps_json",
        "command": "dart pub deps --json",
        **digest_input,
        "runtime_package_count": len(normalized),
        "runtime_tree_sha256": digest,
    }


def _runtime_dependency_tree(dart_executable: str) -> dict[str, Any]:
    completed = subprocess.run(
        [dart_executable, "pub", "deps", "--json"],
        cwd=ROOT / "apps" / "desktop_flutter",
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        encoding="utf-8",
        errors="replace",
    )
    if completed.returncode != 0:
        raise ValueError(
            "dart pub deps --jsonに失敗した: "
            + completed.stderr.strip()[-1000:]
        )
    value = _parse_json_bytes(completed.stdout.encode("utf-8"))
    if not isinstance(value, dict):
        raise ValueError("dart pub deps --jsonがobjectを返さなかった")
    return _runtime_dependency_tree_from_json(value)


def _dart_executable(flutter_executable: str, expected_version: str) -> str:
    flutter_bin = Path(flutter_executable).resolve().parent
    candidates = [
        flutter_bin / "dart.bat",
        flutter_bin / "dart.cmd",
        flutter_bin / "dart.exe",
        flutter_bin / "cache" / "dart-sdk" / "bin" / "dart.exe",
    ]
    path_dart = shutil.which("dart")
    if path_dart is not None:
        candidates.append(Path(path_dart))

    seen: set[str] = set()
    for candidate in candidates:
        normalized = str(candidate.resolve()) if candidate.exists() else str(candidate)
        if normalized.lower() in seen:
            continue
        seen.add(normalized.lower())
        if not candidate.is_file() and shutil.which(str(candidate)) is None:
            continue
        completed = subprocess.run(
            [str(candidate), "--version"],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            encoding="utf-8",
            errors="replace",
        )
        version_output = completed.stdout + completed.stderr
        if completed.returncode == 0 and expected_version in version_output:
            return str(candidate)
    raise ValueError("Flutter toolchainと同じDart SDK executableを特定できない")


def _aot_package_libraries(report: dict[str, Any]) -> list[str]:
    """Flutter AOT treeに含まれるpackage配下のDart library pathを列挙する。"""
    roots: list[dict[str, Any]] = []
    pending: list[Any] = [report]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            if value.get("n") == AOT_REPORT_ROOT:
                roots.append(value)
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)
    if len(roots) != 1:
        raise ValueError("Flutter size-analysis reportにDart AOT treeが一意に存在しない")

    found: set[str] = set()
    path_stack: list[tuple[Any, str | None]] = [(roots[0], None)]
    while path_stack:
        value, package_path = path_stack.pop()
        if isinstance(value, dict):
            name = value.get("n")
            if isinstance(name, str):
                if name.startswith("package:"):
                    package_path = name
                    if name.endswith(".dart"):
                        found.add(name)
                elif package_path is not None and name.endswith(".dart"):
                    found.add(f"{package_path.rstrip('/')}/{name}")
            children = value.get("children", [])
            if isinstance(children, list):
                path_stack.extend((child, package_path) for child in children)
    return sorted(found)


def package_dependency_comparison(
    baseline: dict[str, Any], selected: dict[str, Any]
) -> dict[str, Any]:
    baseline_tree = baseline["runtime_dependency_tree"]
    selected_tree = selected["runtime_dependency_tree"]
    baseline_libraries = set(baseline["aot_package_libraries"])
    selected_libraries = set(selected["aot_package_libraries"])
    return {
        "baseline_runtime_tree_sha256": baseline_tree["runtime_tree_sha256"],
        "selected_runtime_tree_sha256": selected_tree["runtime_tree_sha256"],
        "runtime_package_tree_identical": baseline_tree == selected_tree,
        "removed_aot_package_libraries": sorted(baseline_libraries - selected_libraries),
        "added_aot_package_libraries": sorted(selected_libraries - baseline_libraries),
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
    """Schemaと全構成の選択・file記録・比較値の相互整合を検査する。"""
    from tooling.schema_check.check_schemas import validate_instance

    errors = validate_instance(evidence, schema)
    if errors:
        return errors

    baseline = evidence["baseline"]
    selected = evidence["selected"]
    active_catalog = catalog if catalog is not None else _load_catalog()
    expected_variants = _pruning_variant_plans(active_catalog)
    variants = evidence["pruning_variants"]
    if len(variants) != len(expected_variants):
        errors.append("比較variant数がModule Catalogから導出した必要数と一致しない")
    for (expected_label, expected_plan, expected_omitted), variant in zip(
        expected_variants, variants
    ):
        if variant["label"] != expected_label:
            errors.append("比較variant labelが固定Module Catalog順と一致しない")
        if variant["omitted_optional_module_ids"] != list(expected_omitted):
            errors.append(f"{expected_label}の除外Module一覧が必要条件と一致しない")
        if variant["included_optional_module_ids"] != list(
            expected_plan.included_optional_ids
        ):
            errors.append(f"{expected_label}の包含Module一覧が依存閉包と一致しない")
        if variant["excluded_optional_module_ids"] != list(
            expected_plan.excluded_optional_ids
        ):
            errors.append(f"{expected_label}の除外Module一覧が依存閉包と一致しない")
        expected_defines = dart_defines(expected_plan, active_catalog)
        run = variant["run"]
        if run["label"] != expected_label:
            errors.append(f"{expected_label}のFlutter run labelが一致しない")
        if run["artifact_root"] != f"{expected_label}/app":
            errors.append(f"{expected_label}のFlutter artifact rootが一致しない")
        if run["effective_dart_defines"] != expected_defines:
            errors.append(f"{expected_label}のFlutter defineがModulePlanと一致しない")
        if run["dart_define_arguments"] != expected_defines:
            errors.append(f"{expected_label}のFlutter command defineがModulePlanと一致しない")

    expected_baseline_defines = all_optional_defines()
    if baseline["effective_dart_defines"] != expected_baseline_defines:
        errors.append("baselineがFlutterの全任意画面既定有効状態と一致しない")
    if baseline["dart_define_arguments"] != {}:
        errors.append("baselineへ任意Moduleのdart-defineを渡している")
    if selected["dart_define_arguments"] != selected["effective_dart_defines"]:
        errors.append("選択buildのdart-define引数と有効値が一致しない")
    if all(selected["effective_dart_defines"].values()):
        errors.append("選択buildが任意Moduleを一つも除外していない")

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
        for variant in variants:
            run = variant["run"]
            expected_variant_surfaces = sorted(
                surface
                for module in active_catalog["optional_modules"]
                if run["effective_dart_defines"].get(
                    "GUI_SHELL_MODULE_" + MODULE_DEFINE_NAMES.get(module["module_id"], "")
                )
                for surface in module["source_surfaces"]
            )
            if run["aot_surface_libraries"] != expected_variant_surfaces:
                errors.append(f"{variant['label']}のFlutter AOT surfaceがModulePlanと一致しない")
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

    all_runs = [baseline, selected, *(variant["run"] for variant in variants)]
    baseline_common, baseline_dirs = common_build_arguments(baseline)
    all_profile_dirs = [baseline_dirs[0]] if baseline_dirs else []
    for run in all_runs[1:]:
        run_common, run_dirs = common_build_arguments(run)
        if baseline_common != run_common:
            errors.append(f"{run['label']}の共通command条件がbaselineと一致しない")
        if run_dirs:
            all_profile_dirs.extend(run_dirs)
    if len(all_profile_dirs) != len(set(all_profile_dirs)):
        errors.append("複数buildが同じsize-analysis入力directoryを再利用している")
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

    for run in all_runs:
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
        dependency_tree = run["runtime_dependency_tree"]
        dependency_names = [
            item["name"] for item in dependency_tree["runtime_packages"]
        ]
        if dependency_tree["runtime_package_count"] != len(
            dependency_tree["runtime_packages"]
        ):
            errors.append(f"{run['label']} runtime package countが一致しない")
        if dependency_names != sorted(set(dependency_names)):
            errors.append(f"{run['label']} runtime package nodeが一意・整列されていない")
        dependency_name_set = set(dependency_names)
        if not set(dependency_tree["direct_runtime_dependencies"]).issubset(
            dependency_name_set
        ):
            errors.append(f"{run['label']} direct runtime dependency nodeが欠けている")
        if any(
            not set(item["dependencies"]).issubset(dependency_name_set)
            for item in dependency_tree["runtime_packages"]
        ):
            errors.append(f"{run['label']} runtime dependency edgeの参照先が欠けている")
        dependency_digest_input = {
            "root_package": dependency_tree["root_package"],
            "direct_runtime_dependencies": dependency_tree[
                "direct_runtime_dependencies"
            ],
            "runtime_packages": dependency_tree["runtime_packages"],
        }
        dependency_digest = hashlib.sha256(
            json.dumps(
                dependency_digest_input,
                ensure_ascii=False,
                sort_keys=True,
                separators=(",", ":"),
            ).encode("utf-8")
        ).hexdigest()
        if dependency_digest != dependency_tree["runtime_tree_sha256"]:
            errors.append(f"{run['label']} runtime dependency tree hashが一致しない")
        if run["aot_package_libraries"] != sorted(
            set(run["aot_package_libraries"])
        ):
            errors.append(f"{run['label']} AOT package libraryが一意・整列されていない")

    if any(
        baseline["runtime_dependency_tree"] != run["runtime_dependency_tree"]
        for run in all_runs[1:]
    ):
        errors.append("同一sourceのModule選択間でruntime dependency treeが変化した")

    try:
        expected_summary = comparison_summary(
            baseline["artifact_files"], selected["artifact_files"]
        )
        if evidence["comparison"] != expected_summary:
            errors.append("comparison summaryがartifact file記録から再計算した値と一致しない")
    except (KeyError, TypeError, ValueError) as exc:
        errors.append(f"artifact比較を再計算できない: {exc}")

    try:
        expected_resources = resource_inventory_comparison(
            baseline["artifact_files"], selected["artifact_files"]
        )
        if evidence["resource_inventory_comparison"] != expected_resources:
            errors.append("resource inventory比較がartifact file記録と一致しない")
        expected_dependencies = package_dependency_comparison(baseline, selected)
        if evidence["package_dependency_comparison"] != expected_dependencies:
            errors.append("package dependency比較がrun記録から再計算した値と一致しない")
        for variant in variants:
            run = variant["run"]
            label = variant["label"]
            if variant["comparison_to_baseline"] != comparison_summary(
                baseline["artifact_files"], run["artifact_files"]
            ):
                errors.append(f"{label} Flutter比較summaryをartifact recordから再計算できない")
            if variant["resource_inventory_comparison_to_baseline"] != resource_inventory_comparison(
                baseline["artifact_files"], run["artifact_files"]
            ):
                errors.append(f"{label} resource比較がartifact recordと一致しない")
            if variant["package_dependency_comparison_to_baseline"] != package_dependency_comparison(
                baseline, run
            ):
                errors.append(f"{label} package依存比較がrun記録と一致しない")
    except (KeyError, TypeError, ValueError) as exc:
        errors.append(f"resource／dependency比較を再計算できない: {exc}")

    native_plan: ResolvedModulePlan | None = None
    if receipt is not None:
        try:
            plan = resolve_module_plan(receipt, active_catalog)
            native_plan = plan
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
    errors.extend(
        validate_rust_binary_comparison(
            evidence["rust_binary_comparison"],
            selected_plan=native_plan,
            catalog=active_catalog,
        )
    )
    return errors


def validate_rust_binary_comparison(
    comparison: dict[str, Any],
    *,
    selected_plan: ResolvedModulePlan | None = None,
    catalog: dict[str, Any] | None = None,
) -> list[str]:
    errors: list[str] = []
    active_catalog = catalog if catalog is not None else _load_catalog()
    if comparison["always_included_module_ids"] != [
        *EXPECTED_CORE_IDS,
        *EXPECTED_REQUIRED_IDS,
    ]:
        errors.append("Rust binary比較でSecurity Core／Required Moduleが維持されていない")
    baseline = comparison["baseline"]
    selected = comparison["selected"]
    summary = comparison["comparison"]
    full_plan = _full_optional_plan(active_catalog)
    expected_baseline_arguments = cargo_feature_arguments(
        full_plan, target_platform="windows"
    )
    if baseline["included_optional_module_ids"] != list(full_plan.included_optional_ids):
        errors.append("Rust baselineが全Optional Moduleを含んでいない")
    if baseline["cargo_feature_arguments"] != expected_baseline_arguments:
        errors.append("Rust baseline Cargo feature argumentsが全Optional Moduleと一致しない")
    if selected_plan is not None:
        if selected["included_optional_module_ids"] != list(
            selected_plan.included_optional_ids
        ):
            errors.append("Rust selected Module一覧が検証済みExport Planと一致しない")
        if selected["cargo_feature_arguments"] != cargo_feature_arguments(
            selected_plan, target_platform="windows"
        ):
            errors.append("Rust selected Cargo feature argumentsがExport Planと一致しない")

    expected_variants = _pruning_variant_plans(active_catalog)
    variants = comparison["pruning_variants"]
    if len(variants) != len(expected_variants):
        errors.append("Rust比較variant数がModule Catalogから導出した必要数と一致しない")
    variant_runs: list[dict[str, Any]] = []
    for (expected_label, expected_plan, expected_omitted), variant in zip(
        expected_variants, variants
    ):
        if variant["label"] != expected_label:
            errors.append("Rust比較variant labelが固定Module Catalog順と一致しない")
        if variant["omitted_optional_module_ids"] != list(expected_omitted):
            errors.append(f"Rust {expected_label}の除外Module一覧が必要条件と一致しない")
        if variant["included_optional_module_ids"] != list(
            expected_plan.included_optional_ids
        ):
            errors.append(f"Rust {expected_label}の包含Module一覧が依存閉包と一致しない")
        run = variant["run"]
        expected_features = cargo_feature_arguments(
            expected_plan, target_platform="windows"
        )
        if run["label"] != expected_label:
            errors.append(f"Rust {expected_label} run labelが一致しない")
        if run["included_optional_module_ids"] != list(
            expected_plan.included_optional_ids
        ):
            errors.append(f"Rust {expected_label} runがModule依存閉包と一致しない")
        if run["cargo_feature_arguments"] != expected_features:
            errors.append(f"Rust {expected_label} Cargo featuresがModulePlanと一致しない")
        variant_runs.append(run)

    for label, run in (
        ("baseline", baseline),
        ("selected", selected),
        *[(variant["label"], run) for variant, run in zip(variants, variant_runs)],
    ):
        feature_arguments = run["cargo_feature_arguments"]
        for command_name, command in (
            ("build", run["cargo_command"]),
            ("dependency tree", run["runtime_dependency_tree"]["command"]),
        ):
            if not _contains_argument_sequence(command, feature_arguments):
                errors.append(
                    f"Rust {label} {command_name} commandが記録済みfeature選択と一致しない"
                )

    def validate_pair(label: str, run: dict[str, Any], pair: dict[str, Any]) -> None:
        expected_reduction = baseline["binary"]["size_bytes"] - run["binary"]["size_bytes"]
        if pair["baseline_binary_bytes"] != baseline["binary"]["size_bytes"]:
            errors.append(f"Rust {label} baseline byte数がartifact記録と一致しない")
        if pair["selected_binary_bytes"] != run["binary"]["size_bytes"]:
            errors.append(f"Rust {label} variant byte数がartifact記録と一致しない")
        if pair["binary_bytes_reduced"] != expected_reduction:
            errors.append(f"Rust {label} byte削減量をartifact記録から再計算できない")
        expected_hash_changed = baseline["binary"]["sha256"] != run["binary"]["sha256"]
        if pair["baseline_binary_sha256"] != baseline["binary"]["sha256"]:
            errors.append(f"Rust {label} baseline SHA-256がartifact記録と一致しない")
        if pair["selected_binary_sha256"] != run["binary"]["sha256"]:
            errors.append(f"Rust {label} variant SHA-256がartifact記録と一致しない")
        if pair["binary_hash_changed"] is not expected_hash_changed:
            errors.append(f"Rust {label} hash変更状態がartifact記録と一致しない")
        expected_tree_identical = (
            baseline["runtime_dependency_tree"]["tree_file"]["sha256"]
            == run["runtime_dependency_tree"]["tree_file"]["sha256"]
        )
        if pair["runtime_dependency_tree_identical"] is not expected_tree_identical:
            errors.append(f"Rust {label}依存tree比較が保存tree hashと一致しない")

    validate_pair("selected", selected, summary)
    for variant in variants:
        validate_pair(
            variant["label"], variant["run"], variant["comparison_to_baseline"]
        )
    return errors


def _contains_argument_sequence(arguments: list[str], expected: list[str]) -> bool:
    if not expected or len(expected) > len(arguments):
        return False
    return any(
        arguments[index : index + len(expected)] == expected
        for index in range(len(arguments) - len(expected) + 1)
    )


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


def _native_binary_record(path: Path, evidence_root: Path) -> dict[str, Any]:
    metadata = _reject_link_or_reparse(path, "Rust helper binary")
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_size < 1:
        raise ValueError("Rust helper binaryが空または通常fileではない")
    if metadata.st_size > MAX_NATIVE_BINARY_BYTES:
        raise ValueError("Rust helper binaryが512 MB上限を超えている")
    try:
        path.resolve(strict=True).relative_to(evidence_root.resolve(strict=True))
    except (OSError, ValueError) as exc:
        raise ValueError("Rust helper binaryが証拠directoryの外にある") from exc
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return {
        "path": path.relative_to(evidence_root).as_posix(),
        "size_bytes": metadata.st_size,
        "sha256": digest.hexdigest(),
    }


def _parse_linker_map_symbols(raw: bytes) -> list[str]:
    """MSVC/LLD mapのPublics by Value領域からsymbol名を抽出する。"""
    text = raw.decode("utf-8-sig", errors="replace")
    in_public_symbols = False
    symbols: set[str] = set()
    for line in text.splitlines():
        if "publics by value" in line.casefold():
            in_public_symbols = True
            continue
        if in_public_symbols and line.strip().casefold().startswith("static symbols"):
            break
        if not in_public_symbols:
            continue
        match = MAP_SYMBOL_LINE.match(line)
        if match:
            symbols.add(match.group(1))
    if not symbols:
        raise ValueError("Rust helper linker mapからPublic symbolを採取できない")
    return sorted(symbols)


def symbol_set_comparison(
    baseline_symbols: list[str], selected_symbols: list[str]
) -> dict[str, list[str]]:
    baseline = set(baseline_symbols)
    selected = set(selected_symbols)
    return {
        "removed_symbols": sorted(baseline - selected),
        "added_symbols": sorted(selected - baseline),
    }


def _symbol_inventory_record(
    symbol_file_record: dict[str, Any], symbols: list[str]
) -> dict[str, Any]:
    return {"file": symbol_file_record, "symbol_count": len(symbols)}


def _redact_local_paths(value: str) -> str:
    candidates = {
        str(ROOT.resolve()): "{repository}",
        str(ROOT.resolve()).replace("\\", "/"): "{repository}",
        str(Path.home()): "{user-profile}",
        str(Path.home()).replace("\\", "/"): "{user-profile}",
    }
    for path, replacement in sorted(candidates.items(), key=lambda item: len(item[0]), reverse=True):
        if path:
            value = re.sub(re.escape(path), replacement, value, flags=re.IGNORECASE)
    return value


def _cargo_process_output(
    command: list[str], environment: dict[str, str], *, label: str, timeout: int = 1800
) -> str:
    try:
        completed = subprocess.run(
            command,
            cwd=ROOT,
            env=environment,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as exc:
        raw_output = exc.stdout if exc.stdout is not None else exc.output
        if isinstance(raw_output, bytes):
            partial_output = raw_output.decode("utf-8", errors="replace")
        else:
            partial_output = raw_output or ""
        detail = "\n".join(_redact_local_paths(partial_output).splitlines()[-20:])
        raise ValueError(
            f"{label}が{timeout}秒でtimeoutした" + (f":\n{detail}" if detail else "")
        ) from exc
    output = _redact_local_paths(completed.stdout or "")
    if completed.returncode != 0:
        detail = "\n".join(output.splitlines()[-20:])
        raise ValueError(
            f"{label}がexit={completed.returncode}で失敗した"
            + (f":\n{detail}" if detail else "")
        )
    return output


def _rust_toolchain(environment: dict[str, str]) -> tuple[str, str, str]:
    path = environment.get("PATH")
    cargo = shutil.which("cargo", path=path)
    rustc = shutil.which("rustc", path=path)
    if cargo is None or rustc is None:
        raise ValueError("隔離Build環境でCargo／Rust compilerを解決できない")
    cargo_version_output = _cargo_process_output(
        [cargo, "--version"], environment, label="Cargo version"
    )
    rustc_output = _cargo_process_output(
        [rustc, "-vV"], environment, label="rustc toolchain identity"
    )
    cargo_version = cargo_version_output.strip().splitlines()[0]
    rustc_version = next(
        (line.strip() for line in rustc_output.splitlines() if line.startswith("rustc ")), ""
    )
    target = next(
        (line.split(":", 1)[1].strip() for line in rustc_output.splitlines() if line.startswith("host:")),
        "",
    )
    if not cargo_version or not rustc_version or target != "x86_64-pc-windows-msvc":
        raise ValueError("Windows x64 MSVC Cargo／Rust toolchainを確定できない")
    return cargo, rustc, json.dumps(
        {"cargo": cargo_version, "rustc": rustc_version, "target": target},
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    )


def _full_optional_plan(catalog: dict[str, Any]) -> ResolvedModulePlan:
    optional_ids = tuple(module["module_id"] for module in catalog["optional_modules"])
    if optional_ids != EXPECTED_OPTIONAL_IDS:
        raise ValueError("Full Rust比較のOptional Module集合が固定Catalogと一致しない")
    return ResolvedModulePlan(
        selection_mode="all_optional",
        requested_optional_ids=optional_ids,
        included_optional_ids=optional_ids,
        included_module_ids=(
            *EXPECTED_CORE_IDS,
            *EXPECTED_REQUIRED_IDS,
            *optional_ids,
        ),
        excluded_optional_ids=(),
    )


def _pruning_variant_plans(
    catalog: dict[str, Any],
) -> list[tuple[str, ResolvedModulePlan, tuple[str, ...]]]:
    """Return deterministic one-module removals plus the all-optional-free plan."""
    optional_ids = tuple(module["module_id"] for module in catalog["optional_modules"])
    full_ids = set(optional_ids)
    variants: list[tuple[str, ResolvedModulePlan, tuple[str, ...]]] = []
    seen: set[tuple[str, ...]] = {optional_ids}

    if optional_ids:
        plan = resolve_module_plan_for_optional_ids((), catalog)
        if plan.included_optional_ids not in seen:
            variants.append(("no_optional_modules", plan, optional_ids))
            seen.add(plan.included_optional_ids)

    if len(optional_ids) < 2:
        candidates = [optional_ids[:index] + optional_ids[index + 1 :] for index in range(len(optional_ids))]
        target_count = len({
            resolve_module_plan_for_optional_ids(candidate, catalog).included_optional_ids
            for candidate in [optional_ids, *candidates, ()]
        }) - 1
    else:
        candidates = [
            tuple(module_id for module_id in optional_ids if module_id != omitted_id)
            for omitted_id in optional_ids
        ]
        target_count = 3

    for requested in candidates:
        plan = resolve_module_plan_for_optional_ids(requested, catalog)
        included = plan.included_optional_ids
        if included in seen:
            continue
        omitted = tuple(module_id for module_id in optional_ids if module_id not in set(included))
        if len(omitted) == len(optional_ids):
            label = "no_optional_modules"
        elif len(omitted) == 1:
            label = "without_" + omitted[0].removeprefix("shell.").replace(".", "_")
        else:
            label = "without_optional_" + "_".join(
                module_id.removeprefix("shell.").replace(".", "_") for module_id in omitted
            )
        variants.append((label, plan, omitted))
        seen.add(included)
        if len(variants) >= target_count:
            break

    if len(optional_ids) >= 2 and len(variants) != 3:
        raise ValueError("Module Catalogから独立した除外比較構成を3件作れない")
    if len(optional_ids) < 2 and len(variants) != target_count:
        raise ValueError("少数Moduleの全構成を比較planへ展開できない")
    if any(set(plan.included_optional_ids) | set(omitted) != full_ids for _, plan, omitted in variants):
        raise ValueError("Module比較variantがOptional Module全集合を分割していない")
    return variants


def _cargo_feature_tree(
    *,
    cargo: str,
    environment: dict[str, str],
    target: str,
    feature_arguments: list[str],
) -> tuple[list[str], str]:
    command = [
        cargo,
        "tree",
        "--color",
        "never",
        "--manifest-path",
        RUST_HELPER_MANIFEST,
        "--locked",
        "--offline",
        "--target",
        target,
        "--edges",
        "normal",
        "--prefix",
        "none",
        *feature_arguments,
    ]
    output = _cargo_process_output(command, environment, label="Cargo runtime dependency tree")
    normalized = "\n".join(line.rstrip() for line in output.splitlines()).strip() + "\n"
    return command, normalized


def _capture_rust_helper_run(
    *,
    label: str,
    plan: ResolvedModulePlan,
    cargo: str,
    environment: dict[str, str],
    target: str,
    target_dir: Path,
    shared_linker_map_path: Path,
    evidence_root: Path,
    expected_commit: str,
) -> dict[str, Any]:
    run_root = evidence_root / label / "rust"
    run_root.mkdir(parents=True, exist_ok=False)
    feature_arguments = cargo_feature_arguments(plan, target_platform="windows")
    _cargo_tree_command, dependency_tree = _cargo_feature_tree(
        cargo=cargo,
        environment=environment,
        target=target,
        feature_arguments=feature_arguments,
    )
    tree_path = run_root / "cargo-runtime-dependency-tree.txt"
    tree_path.write_text(dependency_tree, encoding="utf-8", newline="\n")

    linker_map_path = run_root / f"{RUST_HELPER_BINARY}.map"
    started = time.perf_counter()
    command = [
        cargo,
        "rustc",
        "--color",
        "never",
        "--manifest-path",
        RUST_HELPER_MANIFEST,
        "--locked",
        "--offline",
        "--release",
        "--target",
        target,
        *feature_arguments,
        "--target-dir",
        str(target_dir),
        "--bin",
        "gui_shell_rust_helper",
        "--",
        "-C",
        f"link-arg=/MAP:{shared_linker_map_path}",
    ]
    _cargo_process_output(command, environment, label=f"{label} Rust helper Release build")
    _clean_source_commit(expected_commit)
    binary_source = target_dir / target / "release" / RUST_HELPER_BINARY
    binary_path = run_root / RUST_HELPER_BINARY
    if not binary_source.is_file():
        raise ValueError(f"{label} Rust helper Release binaryが生成されない")
    shutil.copy2(binary_source, binary_path)
    if not shared_linker_map_path.is_file():
        raise ValueError(f"{label} Rust helper linker mapが生成されない")
    shutil.copy2(shared_linker_map_path, linker_map_path)
    binary = _native_binary_record(binary_path, evidence_root)
    map_record = _file_record(linker_map_path, evidence_root)
    symbols = _parse_linker_map_symbols(linker_map_path.read_bytes())
    symbol_path = run_root / "public-symbols.txt"
    symbol_path.write_text("".join(f"{symbol}\n" for symbol in symbols), encoding="utf-8", newline="\n")
    symbol_record = _file_record(symbol_path, evidence_root)

    duration_ms = round((time.perf_counter() - started) * 1000)
    tree_record = _file_record(tree_path, evidence_root)
    tree_entries = [line.strip() for line in dependency_tree.splitlines() if line.strip()]
    command_template = [
        "cargo",
        "rustc",
        "--color",
        "never",
        "--manifest-path",
        RUST_HELPER_MANIFEST,
        "--locked",
        "--offline",
        "--release",
        "--target",
        target,
        *feature_arguments,
        "--target-dir",
        "{isolated-target-dir}",
        "--bin",
        "gui_shell_rust_helper",
        "--",
        "-C",
        f"link-arg=/MAP:{{comparison-temporary-directory}}/{RUST_HELPER_BINARY}.map",
    ]
    return {
        "label": label,
        "included_optional_module_ids": list(plan.included_optional_ids),
        "cargo_feature_arguments": feature_arguments,
        "cargo_command": command_template,
        "build_duration_ms": duration_ms,
        "binary": binary,
        "linker_map": map_record,
        "symbol_inventory": _symbol_inventory_record(symbol_record, symbols),
        "runtime_dependency_tree": {
            "command": [
                "cargo",
                "tree",
                "--color",
                "never",
                "--manifest-path",
                RUST_HELPER_MANIFEST,
                "--locked",
                "--offline",
                "--target",
                target,
                "--edges",
                "normal",
                "--prefix",
                "none",
                *feature_arguments,
            ],
            "entry_count": len(tree_entries),
            "tree_file": tree_record,
        },
        "_symbols": symbols,
        "_dependency_tree": dependency_tree,
    }


def _native_pair_comparison(
    baseline: dict[str, Any],
    selected: dict[str, Any],
    evidence_root: Path,
    *,
    removed_path: Path,
    added_path: Path,
) -> dict[str, Any]:
    symbols = symbol_set_comparison(baseline["_symbols"], selected["_symbols"])
    baseline_tree = baseline["_dependency_tree"]
    selected_tree = selected["_dependency_tree"]
    removed_path.write_text(
        "".join(f"{symbol}\n" for symbol in symbols["removed_symbols"]),
        encoding="utf-8",
        newline="\n",
    )
    added_path.write_text(
        "".join(f"{symbol}\n" for symbol in symbols["added_symbols"]),
        encoding="utf-8",
        newline="\n",
    )
    return {
        "baseline_binary_bytes": baseline["binary"]["size_bytes"],
        "selected_binary_bytes": selected["binary"]["size_bytes"],
        "binary_bytes_reduced": baseline["binary"]["size_bytes"]
        - selected["binary"]["size_bytes"],
        "baseline_binary_sha256": baseline["binary"]["sha256"],
        "selected_binary_sha256": selected["binary"]["sha256"],
        "binary_hash_changed": baseline["binary"]["sha256"]
        != selected["binary"]["sha256"],
        "runtime_dependency_tree_identical": hashlib.sha256(
            baseline_tree.encode("utf-8")
        ).hexdigest()
        == hashlib.sha256(selected_tree.encode("utf-8")).hexdigest(),
        "removed_symbol_count": len(symbols["removed_symbols"]),
        "added_symbol_count": len(symbols["added_symbols"]),
        "removed_symbols_file": _file_record(removed_path, evidence_root),
        "added_symbols_file": _file_record(added_path, evidence_root),
    }


def _native_binary_comparison(
    *,
    plan: ResolvedModulePlan,
    pruning_plans: list[tuple[str, ResolvedModulePlan, tuple[str, ...]]],
    catalog: dict[str, Any],
    evidence_root: Path,
    source_commit: str,
) -> dict[str, Any]:
    inherited_environment = dict(os.environ)
    cargo_home = inherited_environment.get("CARGO_HOME") or str(Path.home() / ".cargo")
    environment = build_child_environment(
        {"CARGO_HOME": cargo_home, "CARGO_TERM_COLOR": "never"},
        inherited=inherited_environment,
    )
    cargo, _rustc, toolchain_json = _rust_toolchain(environment)
    toolchain = json.loads(toolchain_json)
    target_parent = Path(safe_flutter_temp_parent(ROOT)).resolve(strict=True)
    with tempfile.TemporaryDirectory(
        prefix="d4p-module-rust-target-", dir=target_parent
    ) as temp:
        target_dir = Path(temp) / "target"
        shared_linker_map_path = Path(temp) / f"{RUST_HELPER_BINARY}.map"
        baseline = _capture_rust_helper_run(
            label="baseline",
            plan=_full_optional_plan(catalog),
            cargo=cargo,
            environment=environment,
            target=toolchain["target"],
            target_dir=target_dir,
            shared_linker_map_path=shared_linker_map_path,
            evidence_root=evidence_root,
            expected_commit=source_commit,
        )
        _clean_source_commit(source_commit)
        selected = _capture_rust_helper_run(
            label="selected",
            plan=plan,
            cargo=cargo,
            environment=environment,
            target=toolchain["target"],
            target_dir=target_dir,
            shared_linker_map_path=shared_linker_map_path,
            evidence_root=evidence_root,
            expected_commit=source_commit,
        )
        _clean_source_commit(source_commit)
        variant_runs: list[dict[str, Any]] = []
        for label, variant_plan, omitted in pruning_plans:
            run = _capture_rust_helper_run(
                label=label,
                plan=variant_plan,
                cargo=cargo,
                environment=environment,
                target=toolchain["target"],
                target_dir=target_dir,
                shared_linker_map_path=shared_linker_map_path,
                evidence_root=evidence_root,
                expected_commit=source_commit,
            )
            _clean_source_commit(source_commit)
            pair = _native_pair_comparison(
                baseline,
                run,
                evidence_root,
                removed_path=evidence_root / label / "rust" / "removed-symbols.txt",
                added_path=evidence_root / label / "rust" / "added-symbols.txt",
            )
            run.pop("_symbols")
            run.pop("_dependency_tree")
            variant_runs.append(
                {
                    "label": label,
                    "omitted_optional_module_ids": list(omitted),
                    "included_optional_module_ids": list(
                        variant_plan.included_optional_ids
                    ),
                    "run": run,
                    "comparison_to_baseline": pair,
                }
            )
    _cargo_after, _rustc_after, final_toolchain_json = _rust_toolchain(environment)
    if final_toolchain_json != toolchain_json:
        raise ValueError("複数Rust helper構成build中にToolchain identityが変化した")

    comparison = _native_pair_comparison(
        baseline,
        selected,
        evidence_root,
        removed_path=evidence_root / "removed-rust-symbols.txt",
        added_path=evidence_root / "added-rust-symbols.txt",
    )
    baseline.pop("_symbols")
    baseline.pop("_dependency_tree")
    selected.pop("_symbols")
    selected.pop("_dependency_tree")
    return {
        "build_target": toolchain["target"],
        "always_included_module_ids": [*EXPECTED_CORE_IDS, *EXPECTED_REQUIRED_IDS],
        "same_source_commit": True,
        "same_toolchain": True,
        "toolchain": toolchain,
        "baseline": baseline,
        "selected": selected,
        "comparison": comparison,
        "pruning_variants": variant_runs,
        "binary_pruning_verified": False,
    }


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
    dart_executable: str,
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
    report = _parse_json_bytes(report_source.read_bytes())
    aot_surface_libraries = _aot_surface_libraries(report, catalog)
    aot_package_libraries = _aot_package_libraries(report)
    runtime_dependency_tree = _runtime_dependency_tree(dart_executable)
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
        "aot_package_libraries": aot_package_libraries,
        "runtime_dependency_tree": runtime_dependency_tree,
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
    dart_executable = _dart_executable(executable, versions["dart"])
    pruning_plans = _pruning_variant_plans(catalog)
    output_dir.mkdir(parents=True, exist_ok=False)
    baseline = _capture_run(
        label="baseline",
        output_root=output_dir,
        command_defines={},
        effective_defines=baseline_defines,
        executable=executable,
        evidence_root=output_dir,
        catalog=catalog,
        dart_executable=dart_executable,
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
        dart_executable=dart_executable,
    )
    _clean_source_commit(source_commit)
    pruning_variants: list[dict[str, Any]] = []
    for label, variant_plan, omitted in pruning_plans:
        variant_defines = dart_defines(variant_plan, catalog)
        run = _capture_run(
            label=label,
            output_root=output_dir,
            command_defines=variant_defines,
            effective_defines=variant_defines,
            executable=executable,
            evidence_root=output_dir,
            catalog=catalog,
            dart_executable=dart_executable,
        )
        _clean_source_commit(source_commit)
        pruning_variants.append(
            {
                "label": label,
                "omitted_optional_module_ids": list(omitted),
                "included_optional_module_ids": list(
                    variant_plan.included_optional_ids
                ),
                "excluded_optional_module_ids": list(
                    variant_plan.excluded_optional_ids
                ),
                "run": run,
                "comparison_to_baseline": comparison_summary(
                    baseline["artifact_files"], run["artifact_files"]
                ),
                "resource_inventory_comparison_to_baseline": resource_inventory_comparison(
                    baseline["artifact_files"], run["artifact_files"]
                ),
                "package_dependency_comparison_to_baseline": package_dependency_comparison(
                    baseline, run
                ),
            }
        )
    if _flutter_versions(executable) != versions:
        raise ValueError("比較build中にFlutter toolchain versionが変化した")
    rust_comparison = _native_binary_comparison(
        plan=plan,
        pruning_plans=pruning_plans,
        catalog=catalog,
        evidence_root=output_dir,
        source_commit=source_commit,
    )
    summary = comparison_summary(baseline["artifact_files"], selected["artifact_files"])
    evidence = {
        "version": COMPARISON_EVIDENCE_VERSION,
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
        "build_scope": "desktop_flutter_and_rust_helper",
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
        "pruning_variants": pruning_variants,
        "comparison": summary,
        "rust_binary_comparison": rust_comparison,
        "resource_inventory_comparison": resource_inventory_comparison(
            baseline["artifact_files"], selected["artifact_files"]
        ),
        "package_dependency_comparison": package_dependency_comparison(
            baseline, selected
        ),
        "binary_pruning_verified": False,
        "semantic_pruning_status": "flutter_aot_and_rust_helper_binary_observed; standalone_runtime_unverified",
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
        description="同一commitの全Optional、Receipt選択、個別除外、OptionalなしをWindows Releaseで測定するDeveloper専用tool。"
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
        "resource_inventory_identical": evidence["resource_inventory_comparison"][
            "resource_inventory_identical"
        ],
        "runtime_package_tree_identical": evidence[
            "package_dependency_comparison"
        ]["runtime_package_tree_identical"],
        "rust_helper_binary_bytes_reduced": evidence["rust_binary_comparison"][
            "comparison"
        ]["binary_bytes_reduced"],
        "rust_helper_removed_symbol_count": evidence["rust_binary_comparison"][
            "comparison"
        ]["removed_symbol_count"],
        "binary_pruning_verified": False,
        "cold_start_status": "not_measured",
        "resource_comparison_status": "not_measured",
    }, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
