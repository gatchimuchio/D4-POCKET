"""開発時にだけ使用するWindows向けGUI Shell ModulePlan build。

Flutterや製品Runtimeからは呼び出さない。Receipt JSONは画面選択としてだけ
読み、Owner AuthorityやReceiptの出所を検証せず、非製品build証拠を記録する。
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import stat
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from pathlib import PureWindowsPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))
FLUTTER_PROJECT = ROOT / "apps" / "desktop_flutter"
CATALOG_PATH = ROOT / "specs" / "gui_shell_module_catalog.json"
BUILD_EVIDENCE_SCHEMA = ROOT / "specs" / "gui_shell_module_build_evidence.schema.json"
BUILD_OUTPUT = FLUTTER_PROJECT / "build" / "windows" / "x64" / "runner" / "Release"

EXPECTED_CORE_IDS = (
    "core.security_broker",
    "core.cryptography",
    "core.audit_finality",
    "core.credentials",
    "core.os_controls",
    "core.permission_enforcement",
    "core.approval_enforcement",
    "core.content_exposure",
)
EXPECTED_REQUIRED_IDS = (
    "shell.dashboard",
    "shell.runtime_operation",
    "shell.agent_operation",
    "shell.authority",
    "shell.approval",
    "shell.audit",
    "shell.recovery",
    "shell.problems",
    "shell.evidence",
    "shell.settings",
)
EXPECTED_OPTIONAL_IDS = (
    "shell.setup_doctor",
    "shell.history",
    "shell.evaluation_lab",
    "shell.host_capabilities",
    "shell.notifications",
    "shell.observability",
    "shell.trace_inspector",
    "shell.host_operations",
)
MODULE_DEFINE_NAMES = {
    "shell.setup_doctor": "SETUP_DOCTOR",
    "shell.history": "HISTORY",
    "shell.evaluation_lab": "EVALUATION_LAB",
    "shell.host_capabilities": "HOST_CAPABILITIES",
    "shell.notifications": "NOTIFICATIONS",
    "shell.observability": "OBSERVABILITY",
    "shell.trace_inspector": "TRACE_INSPECTOR",
    "shell.host_operations": "HOST_OPERATIONS",
}


@dataclass(frozen=True)
class ResolvedModulePlan:
    selection_mode: str
    requested_optional_ids: tuple[str, ...]
    included_optional_ids: tuple[str, ...]
    included_module_ids: tuple[str, ...]
    excluded_optional_ids: tuple[str, ...]


def _string_list(value: Any, label: str) -> list[str]:
    if not isinstance(value, list) or any(not isinstance(item, str) for item in value):
        raise ValueError(f"{label}は文字列の一覧でなければならない")
    if len(value) != len(set(value)):
        raise ValueError(f"{label}に重複識別子がある")
    return value


def _reject_duplicate_json_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("JSON object keyが重複している")
        value[key] = item
    return value


def _parse_json_bytes(raw: bytes) -> Any:
    return json.loads(
        raw.decode("utf-8"),
        object_pairs_hook=_reject_duplicate_json_keys,
        parse_constant=lambda _token: (_ for _ in ()).throw(
            ValueError("JSON数値が有限値ではない")
        ),
    )


def _validate_catalog(catalog: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(catalog, dict) or catalog.get("version") != 1:
        raise ValueError("Module一覧の版が未対応である")
    if catalog.get("unprunable_core_ids") != list(EXPECTED_CORE_IDS):
        raise ValueError("Module一覧が固定安全Coreを保持していない")
    if catalog.get("required_module_ids") != list(EXPECTED_REQUIRED_IDS):
        raise ValueError("Module一覧が固定必須画面を保持していない")
    modules = catalog.get("optional_modules")
    if not isinstance(modules, list):
        raise ValueError("Module一覧の任意Module定義が不正である")

    optional_ids: set[str] = set()
    dependency_map: dict[str, list[str]] = {}
    for module in modules:
        if not isinstance(module, dict):
            raise ValueError("任意Module定義が不正である")
        module_id = module.get("module_id")
        dependencies = _string_list(module.get("depends_on"), f"{module_id}.depends_on")
        surfaces = _string_list(module.get("source_surfaces"), f"{module_id}.source_surfaces")
        if (
            not isinstance(module_id, str)
            or not module_id.startswith("shell.")
            or not surfaces
        ):
            raise ValueError("任意Module識別子または画面pathが不正である")
        if module_id in optional_ids or module_id in EXPECTED_CORE_IDS or module_id in EXPECTED_REQUIRED_IDS:
            raise ValueError("任意Module識別子が重複するか保護対象と衝突している")
        optional_ids.add(module_id)
        dependency_map[module_id] = dependencies
        for surface in surfaces:
            posix_surface = PurePosixPath(surface)
            windows_surface = PureWindowsPath(surface)
            if (
                posix_surface.is_absolute()
                or windows_surface.drive
                or "\\" in surface
                or ".." in posix_surface.parts
            ):
                raise ValueError("Module画面pathがRepository相対pathではない")
            candidate = (ROOT / PurePosixPath(surface)).resolve()
            try:
                candidate.relative_to(ROOT.resolve())
            except ValueError as exc:
                raise ValueError("Module画面pathがRepository外を指している") from exc
            if not candidate.is_file():
                raise ValueError("Module画面pathが存在しない")
    if tuple(module["module_id"] for module in modules) != EXPECTED_OPTIONAL_IDS:
        raise ValueError("Module一覧が固定Desktop選択面と一致しない")
    for dependencies in dependency_map.values():
        if any(dep not in optional_ids and dep not in EXPECTED_REQUIRED_IDS for dep in dependencies):
            raise ValueError("Module一覧に未知の依存先がある")

    complete: set[str] = set()

    def visit(module_id: str, active: set[str]) -> None:
        if module_id in active:
            raise ValueError("Module一覧の依存関係に循環がある")
        if module_id in complete:
            return
        active.add(module_id)
        for dependency in dependency_map[module_id]:
            if dependency in dependency_map:
                visit(dependency, active)
        active.remove(module_id)
        complete.add(module_id)

    for module_id in dependency_map:
        visit(module_id, set())
    return catalog


def _load_catalog(path: Path = CATALOG_PATH) -> dict[str, Any]:
    return _validate_catalog(_parse_json_bytes(path.read_bytes()))


def _validate_export_receipt(receipt: dict[str, Any]) -> None:
    from tooling.schema_check.check_schemas import validate_instance

    schema = json.loads((ROOT / "specs" / "gui_shell_export_receipt.schema.json").read_text(encoding="utf-8"))
    if validate_instance(receipt, schema):
        raise ValueError("Export Receiptが現行Schemaに適合しない")


def resolve_module_plan(
    receipt: dict[str, Any], catalog: dict[str, Any] | None = None
) -> ResolvedModulePlan:
    """書出しReceiptを画面選択として検証する。authorityは検証も継承もしない。"""
    _validate_export_receipt(receipt)
    catalog = _validate_catalog(catalog) if catalog is not None else _load_catalog()
    if receipt.get("status") != "accepted" or receipt.get("operation") != "GUI Shell書出し":
        raise ValueError("accepted状態のGUI Shell Export Receiptが必要である")
    if receipt.get("authority_strip") is not True:
        raise ValueError("Export Receiptのauthority_strip markerがtrueではない")
    for field in (
        "credential_inherited",
        "permission_inherited",
        "approval_inherited",
        "audit_chain_inherited",
    ):
        if receipt.get(field) is not False:
            raise ValueError(f"Export Receiptの非継承条件が不正である: {field}")

    export_manifest = receipt.get("export_manifest")
    if not isinstance(export_manifest, dict):
        raise ValueError("Export ReceiptにManifestがない")
    identity = export_manifest.get("app_identity")
    distribution = export_manifest.get("distribution_metadata")
    if (
        not isinstance(identity, dict)
        or not isinstance(identity.get("app_id"), str)
        or identity.get("target_platform") != "windows"
        or not isinstance(distribution, dict)
        or distribution.get("artifact_status") != "not_built"
        or distribution.get("signed") is not False
    ):
        raise ValueError("Export ReceiptがWindows manifest-only build入力ではない")

    plan = export_manifest.get("module_plan")
    if not isinstance(plan, dict) or plan.get("catalog_version") != catalog["version"]:
        raise ValueError("Export ReceiptのModulePlanがないかModule一覧の版が未知である")
    if plan.get("binary_pruning_status") != "not_applied":
        raise ValueError("Manifest-only Export Receiptがbinary除去済みを主張している")
    if plan.get("unprunable_core_ids") != catalog["unprunable_core_ids"]:
        raise ValueError("ModulePlanが固定安全Coreを欠落または変更している")
    if plan.get("mandatory_module_ids") != catalog["required_module_ids"]:
        raise ValueError("ModulePlanが必須画面を欠落または変更している")

    optional_modules = catalog["optional_modules"]
    optional_ids = [module["module_id"] for module in optional_modules]
    optional_set = set(optional_ids)
    requested = _string_list(
        plan.get("requested_optional_module_ids"), "requested_optional_module_ids"
    )
    if not set(requested).issubset(optional_set):
        raise ValueError("ModulePlanが未知の任意Moduleを要求している")
    selection_mode = plan.get("selection_mode")
    if selection_mode not in {"all_optional", "explicit_optional"}:
        raise ValueError("ModulePlanの選択方式が未対応である")
    if selection_mode == "all_optional" and set(requested) != optional_set:
        raise ValueError("既定選択ではすべての任意Moduleを保持しなければならない")

    dependency_map = {module["module_id"]: module["depends_on"] for module in optional_modules}
    included_optional: set[str] = set()
    active: set[str] = set()

    def include(module_id: str) -> None:
        if module_id in EXPECTED_CORE_IDS or module_id in EXPECTED_REQUIRED_IDS:
            return
        if module_id in included_optional:
            return
        if module_id in active:
            raise ValueError("Module依存関係に循環がある")
        if module_id not in dependency_map:
            raise ValueError("Module依存先が一覧に存在しない")
        active.add(module_id)
        for dependency in dependency_map[module_id]:
            include(dependency)
        active.remove(module_id)
        included_optional.add(module_id)

    for module_id in requested:
        include(module_id)

    expected_included = (
        set(EXPECTED_CORE_IDS)
        | set(EXPECTED_REQUIRED_IDS)
        | included_optional
    )
    included = _string_list(plan.get("included_module_ids"), "included_module_ids")
    excluded = _string_list(
        plan.get("excluded_optional_module_ids"), "excluded_optional_module_ids"
    )
    if set(included) != expected_included:
        raise ValueError("ModulePlanの包含一覧が必須Moduleと依存閉包に一致しない")
    if set(excluded) != optional_set - included_optional:
        raise ValueError("ModulePlanの除外一覧が選択結果に一致しない")

    ordered_included = tuple(
        [*EXPECTED_CORE_IDS, *EXPECTED_REQUIRED_IDS]
        + [module_id for module_id in optional_ids if module_id in included_optional]
    )
    ordered_excluded = tuple(
        module_id for module_id in optional_ids if module_id not in included_optional
    )
    return ResolvedModulePlan(
        selection_mode=selection_mode,
        requested_optional_ids=tuple(module_id for module_id in optional_ids if module_id in requested),
        included_optional_ids=tuple(
            module_id for module_id in optional_ids if module_id in included_optional
        ),
        included_module_ids=ordered_included,
        excluded_optional_ids=ordered_excluded,
    )


def dart_defines(plan: ResolvedModulePlan, catalog: dict[str, Any] | None = None) -> dict[str, bool]:
    catalog = _validate_catalog(catalog) if catalog is not None else _load_catalog()
    _validate_dart_define_bindings(catalog)
    enabled = set(plan.included_optional_ids)
    return {
        "GUI_SHELL_MODULE_" + MODULE_DEFINE_NAMES[module["module_id"]]: module["module_id"] in enabled
        for module in catalog["optional_modules"]
    }


def _validate_dart_define_bindings(catalog: dict[str, Any]) -> None:
    source = (FLUTTER_PROJECT / "lib" / "main.dart").read_text(encoding="utf-8")
    for module in catalog["optional_modules"]:
        module_id = module["module_id"]
        flag = MODULE_DEFINE_NAMES[module_id]
        dart_name = "kGuiShellModule" + "".join(part.title() for part in flag.lower().split("_"))
        define = "GUI_SHELL_MODULE_" + flag
        match = re.search(
            rf"const\s+bool\s+{re.escape(dart_name)}\s*=\s*bool\.fromEnvironment\(\s*['\"]{re.escape(define)}['\"].*?defaultValue:\s*true",
            source,
            re.DOTALL,
        )
        if match is None or source.count(dart_name) < 2:
            raise ValueError("Module一覧に対応するcompile-time UI surfaceがない")
        if module_id == "shell.trace_inspector":
            declaration_end = source.find(";", match.start())
            declaration = source[match.start() : declaration_end + 1]
            if not re.search(r"&&\s*kGuiShellModuleObservability", declaration):
                raise ValueError("Trace Inspectorが依存先Observabilityへ結合されていない")


def _git_value(*arguments: str) -> str:
    completed = subprocess.run(
        ["git", *arguments],
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return completed.stdout.strip()


def _flutter_versions() -> dict[str, str]:
    completed = subprocess.run(
        ["flutter", "--version", "--machine"],
        cwd=FLUTTER_PROJECT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    value = json.loads(completed.stdout)
    versions = {
        "flutter": str(value.get("frameworkVersion", "unknown")),
        "dart": str(value.get("dartSdkVersion", "unknown")),
        "framework_revision": str(value.get("frameworkRevision", "unknown")),
    }
    if any(not version or version == "unknown" for version in versions.values()):
        raise ValueError("Flutter toolchainが完全な版情報を返さなかった")
    return versions


def _artifact_files(root: Path) -> tuple[list[dict[str, Any]], int, str]:
    entries: list[dict[str, Any]] = []
    total_bytes = 0
    tree_digest = hashlib.sha256()
    for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix().lower()):
        metadata = path.lstat()
        if stat.S_ISLNK(metadata.st_mode) or getattr(metadata, "st_file_attributes", 0) & 0x400:
            raise ValueError(f"build artifact内にreparse pointがある: {path}")
        if not path.is_file():
            continue
        relative = path.relative_to(root).as_posix()
        if ".." in PurePosixPath(relative).parts:
            raise ValueError(f"artifact pathがroot外を指している: {relative}")
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        size = metadata.st_size
        entries.append({"path": relative, "size_bytes": size, "sha256": digest})
        total_bytes += size
        tree_digest.update(f"{relative}\0{size}\0{digest}\n".encode("utf-8"))
    if not entries:
        raise ValueError("Windows build outputにfileがない")
    return entries, total_bytes, tree_digest.hexdigest()


def _write_build(receipt_path: Path, artifact_dir: Path) -> dict[str, Any]:
    if sys.platform != "win32":
        raise ValueError("このDeveloper build toolはWindows専用である")
    if _git_value("status", "--porcelain", "--untracked-files=all"):
        raise ValueError("build証拠の生成にはcleanなcommit済みworktreeが必要である")
    if artifact_dir.exists():
        raise ValueError("artifact出力先が既に存在するため上書きを拒否した")
    try:
        artifact_dir.relative_to(ROOT)
    except ValueError:
        pass
    else:
        raise ValueError("build artifactはRepository外へ出力しなければならない")

    metadata = receipt_path.lstat()
    if stat.S_ISLNK(metadata.st_mode) or getattr(metadata, "st_file_attributes", 0) & 0x400:
        raise ValueError("Receipt入力にsymbolic linkまたはreparse pointを使えない")
    if metadata.st_size > 1_000_000:
        raise ValueError("Receipt入力が1 MB上限を超えている")
    receipt_bytes = receipt_path.read_bytes()
    if len(receipt_bytes) > 1_000_000:
        raise ValueError("Receipt入力が1 MB上限を超えている")
    receipt = _parse_json_bytes(receipt_bytes)
    if not isinstance(receipt, dict):
        raise ValueError("Receipt入力はJSON objectでなければならない")
    catalog = _load_catalog()
    plan = resolve_module_plan(receipt, catalog)
    define_map = dart_defines(plan, catalog)
    versions = _flutter_versions()
    command = [
        "flutter",
        "--suppress-analytics",
        "build",
        "windows",
        "--release",
        "--no-pub",
        *[f"--dart-define={name}={str(enabled).lower()}" for name, enabled in define_map.items()],
    ]
    build_started = time.perf_counter()
    subprocess.run(command, cwd=FLUTTER_PROJECT, check=True)
    build_duration_ms = round((time.perf_counter() - build_started) * 1000)
    app_executable = BUILD_OUTPUT / "gui_shell_desktop.exe"
    app_code = BUILD_OUTPUT / "data" / "app.so"
    if not app_executable.is_file() or not app_code.is_file():
        raise ValueError("Windows release出力にapp executableまたはDart AOT imageがない")

    source_files, source_bytes, source_tree_hash = _artifact_files(BUILD_OUTPUT)
    artifact_dir.mkdir(parents=True, exist_ok=False)
    app_destination = artifact_dir / "app"
    shutil.copytree(BUILD_OUTPUT, app_destination)
    files, total_bytes, tree_hash = _artifact_files(app_destination)
    if (files, total_bytes, tree_hash) != (source_files, source_bytes, source_tree_hash):
        raise ValueError("複製artifactが検証済みFlutter build outputと一致しない")
    evidence = {
        "version": 1,
        "evidence_type": "development_module_pruning_build",
        "evidence_source": "INTERNAL_STATE",
        "product_artifact_claimed": False,
        "standalone_app_claimed": False,
        "selection_input_trust": "unverified_receipt_json_selection_only",
        "authority_verified": False,
        "source_commit": _git_value("rev-parse", "HEAD"),
        "source_receipt_sha256": hashlib.sha256(receipt_bytes).hexdigest(),
        "worktree_clean": True,
        "toolchain": versions,
        "target_platform": "windows",
        "build_mode": "release",
        "build_scope": "desktop_flutter_ui_only",
        "catalog_version": catalog["version"],
        "selection_mode": plan.selection_mode,
        "requested_optional_module_ids": list(plan.requested_optional_ids),
        "included_module_ids": list(plan.included_module_ids),
        "excluded_optional_module_ids": list(plan.excluded_optional_ids),
        "dart_defines": define_map,
        "build_command": command,
        "build_duration_ms": build_duration_ms,
        "artifact_root": "app",
        "artifact_files": files,
        "artifact_total_bytes": total_bytes,
        "artifact_tree_sha256": tree_hash,
        "binary_pruning_verified": False,
        "pruning_status": "compile_time_defines_applied_binary_comparison_pending",
        "signed": False,
    }
    from tooling.schema_check.check_schemas import validate_instance

    schema = json.loads(BUILD_EVIDENCE_SCHEMA.read_text(encoding="utf-8"))
    errors = validate_instance(evidence, schema)
    if errors:
        raise ValueError("build証拠がContractに適合しない: " + "; ".join(errors))
    (artifact_dir / "build_evidence.json").write_text(
        json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    return evidence


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Developerが明示実行するWindows Module選択build。独立製品ではなくOwner権限も扱わない。"
    )
    parser.add_argument("--receipt", type=Path, required=True, help="書出しReceipt JSON。権限の検証には使わない")
    parser.add_argument("--artifact-dir", type=Path, help="まだ存在しないRepository外の出力directory")
    parser.add_argument("--plan-only", action="store_true", help="artifactをbuildせず選択とDart defineだけを検査")
    arguments = parser.parse_args()
    try:
        receipt_metadata = arguments.receipt.lstat()
        if stat.S_ISLNK(receipt_metadata.st_mode) or getattr(receipt_metadata, "st_file_attributes", 0) & 0x400:
            raise ValueError("Receipt入力にsymbolic linkまたはreparse pointを使えない")
        if receipt_metadata.st_size > 1_000_000:
            raise ValueError("Receipt入力が1 MB上限を超えている")
        receipt_path = arguments.receipt.resolve(strict=True)
        receipt_bytes = receipt_path.read_bytes()
        if len(receipt_bytes) > 1_000_000:
            raise ValueError("Receipt入力が1 MB上限を超えている")
        receipt = _parse_json_bytes(receipt_bytes)
        if not isinstance(receipt, dict):
            raise ValueError("Receipt入力はJSON objectでなければならない")
        catalog = _load_catalog()
        plan = resolve_module_plan(receipt, catalog)
        define_map = dart_defines(plan, catalog)
        if arguments.plan_only:
            print(json.dumps({
                "selection_input_trust": "unverified_receipt_json_selection_only",
                "authority_verified": False,
                "requested_optional_module_ids": list(plan.requested_optional_ids),
                "included_module_ids": list(plan.included_module_ids),
                "excluded_optional_module_ids": list(plan.excluded_optional_ids),
                "dart_defines": define_map,
                "binary_pruning_verified": False,
            }, ensure_ascii=False, indent=2))
            return 0
        if arguments.artifact_dir is None:
            raise ValueError("--plan-only以外では--artifact-dirが必要である")
        evidence = _write_build(receipt_path, arguments.artifact_dir.resolve())
    except (OSError, ValueError, json.JSONDecodeError, subprocess.CalledProcessError) as exc:
        print(f"Module選択buildを停止した: {exc}", file=sys.stderr)
        return 1
    print(
        json.dumps(
            {
                "artifact_root": str(arguments.artifact_dir.resolve() / "app"),
                "artifact_tree_sha256": evidence["artifact_tree_sha256"],
                "artifact_total_bytes": evidence["artifact_total_bytes"],
                "pruning_status": evidence["pruning_status"],
                "product_artifact_claimed": False,
            },
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
