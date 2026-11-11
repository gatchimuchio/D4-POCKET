"""Broker Export Manifestから未署名Linux Release bundle／D4PKG01を組み立てる開発tool。"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
import platform
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.build_module_pruned_windows import _load_catalog, dart_defines
from tooling.export_windows_product import (
    CREDENTIAL_SCAN_STATUS,
    MAX_INPUT_BYTES,
    MAX_MANIFEST_BYTES,
    _artifact_inventory,
    _parse_json_bytes,
    _read_regular_file,
    _resolve_build_temp_root,
    _resolve_output_directory,
    _safe_extract_source,
    _scan_credential_inventory,
    _schema,
    _update_trust_summary,
    product_version_from_source,
    validate_export_inputs,
)
from tooling.package_linux_product import EXECUTABLE_PATHS, write_product_package
from tooling.schema_check.check_schemas import validate_instance


RECEIPT_NAME = "d4_pocket_linux_export_build.json"
BUILD_SCHEMA_NAME = "gui_shell_linux_export_build.schema.json"
BUILD_TEMP_PREFIX = "d4p-linux-build-"
OUTPUT_TEMP_PREFIX = "d4p-linux-output-"
PORTABLE_ARCHIVE_ROOT = "D4Pocket"
PORTABLE_ARCHIVE_TARGET = "ubuntu-24.04-x86_64"
PORTABLE_README_MEMBER = "README-linux-ja.txt"
PORTABLE_README_MAX_BYTES = 32 * 1024
SIZE_ANALYSIS_SENTINEL = "A summary of your Linux bundle analysis can be found at:"
SIZE_ANALYSIS_MAX_BYTES = 50_000_000
AOT_REPORT_ROOT = "libapp.so (Dart AOT)"
PACKAGE_REPORT_ROOT = ("package:gui_shell_desktop", "package:gui_shell_desktop")
ISOLATED_ENVIRONMENT_ALLOWLIST = {
    "PATH",
    "LANG",
    "LC_ALL",
    "TZ",
    "FLUTTER_ROOT",
    "RUSTUP_HOME",
    "PKG_CONFIG_PATH",
    "PKG_CONFIG_LIBDIR",
    "PKG_CONFIG_SYSROOT_DIR",
    "CC",
    "CXX",
}


def _git_value(*arguments: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(ROOT), *arguments],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        env=_linux_build_environment({}),
    )
    return completed.stdout.strip()


def _require_clean_source(*, allow_verification_branch: bool = False) -> tuple[str, str]:
    branch = _git_value("branch", "--show-current")
    if not branch or (branch != "main" and not allow_verification_branch):
        raise ValueError("Linux Export buildはmainのclean commitから実行する")
    if branch != "main" and (
        re.fullmatch(r"codex/[A-Za-z0-9._/-]+", branch) is None
        or ".." in branch
        or "//" in branch
        or branch.endswith(("/", "."))
    ):
        raise ValueError("手動検証branchはcodex/配下の安全な名前に限る")
    if _git_value("status", "--porcelain", "--untracked-files=all"):
        raise ValueError("Linux Export buildはcleanなcommit済みsourceから実行する")
    commit = _git_value("rev-parse", "HEAD")
    remote_ref = f"refs/heads/{branch}"
    remote_refs = _git_value("ls-remote", "origin", remote_ref).splitlines()
    remote_parts = remote_refs[0].split() if len(remote_refs) == 1 else []
    if len(remote_parts) != 2 or remote_parts[0] != commit or remote_parts[1] != remote_ref:
        raise ValueError("Linux Export build sourceとGitHub remote branch HEADが一致しない")
    return branch, commit


def _linux_build_environment(
    overrides: dict[str, str], *, inherited: dict[str, str] | None = None
) -> dict[str, str]:
    source = os.environ if inherited is None else inherited
    result = {
        name: value
        for name, value in source.items()
        if name.upper() in ISOLATED_ENVIRONMENT_ALLOWLIST
    }
    for name, value in overrides.items():
        for existing in tuple(result):
            if existing.upper() == name.upper():
                del result[existing]
        result[name] = value
    return result


def _flutter_build_environment(environment: dict[str, str]) -> dict[str, str]:
    # Flutter's first-run analytics notice is appended to stdout unless it detects CI;
    # that breaks the machine-readable JSON emitted by `flutter --version --machine`.
    result = {name: value for name, value in environment.items() if name.upper() != "CI"}
    result["CI"] = "true"
    return result


def _rustup_home() -> str:
    configured = os.environ.get("RUSTUP_HOME", "").strip()
    if configured:
        return str(Path(configured).expanduser().resolve(strict=True))
    rustup = shutil.which("rustup")
    if rustup is None:
        raise ValueError("Rust toolchainのRUSTUP_HOMEを特定できない")
    completed = subprocess.run(
        [rustup, "show", "home"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        env=_linux_build_environment({}),
    )
    value = completed.stdout.strip()
    if not value:
        raise ValueError("rustupがhome pathを返さなかった")
    return str(Path(value).resolve(strict=True))


def _tool_paths() -> dict[str, str]:
    names = ("flutter", "cargo", "rustc", "cmake", "ninja", "pkg-config", "clang", "clang++")
    paths = {name: shutil.which(name) for name in names}
    if any(value is None for value in paths.values()):
        missing = [name for name, value in paths.items() if value is None]
        raise ValueError("Linux Export toolchain commandがPATHにない: " + ", ".join(missing))
    return {name: str(value) for name, value in paths.items()}


def _tool_version(command: list[str], environment: dict[str, str]) -> str:
    completed = subprocess.run(
        command,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        env=environment,
    )
    lines = completed.stdout.splitlines()
    if not lines or not lines[0].strip():
        raise ValueError("Linux build toolがversionを返さなかった")
    return lines[0].strip()


def _toolchain(paths: dict[str, str], environment: dict[str, str]) -> dict[str, str]:
    flutter_environment = _flutter_build_environment(environment)
    flutter_result = subprocess.run(
        [paths["flutter"], "--version", "--machine"],
        cwd=ROOT / "apps" / "desktop_flutter",
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        env=flutter_environment,
    )
    flutter_value = json.loads(flutter_result.stdout)
    versions = {
        "flutter": str(flutter_value.get("frameworkVersion", "unknown")),
        "dart": str(flutter_value.get("dartSdkVersion", "unknown")),
        "framework_revision": str(flutter_value.get("frameworkRevision", "unknown")),
    }
    versions.update(
        {
            "cargo": _tool_version([paths["cargo"], "--version"], environment),
            "rustc": _tool_version([paths["rustc"], "--version"], environment),
            "cmake": _tool_version([paths["cmake"], "--version"], environment),
            "ninja": _tool_version([paths["ninja"], "--version"], environment),
            "pkg_config": _tool_version([paths["pkg-config"], "--version"], environment),
            "clang": _tool_version([paths["clang"], "--version"], environment),
            "clangxx": _tool_version([paths["clang++"], "--version"], environment),
            "architecture": platform.machine() or "unknown",
        }
    )
    if hasattr(platform, "freedesktop_os_release"):
        release = platform.freedesktop_os_release()
        versions["distribution_id"] = release.get("ID", "unknown")
        versions["distribution_version"] = release.get("VERSION_ID", "unknown")
    else:
        versions["distribution_id"] = "unknown"
        versions["distribution_version"] = "unknown"
    if any(not value or value == "unknown" for value in versions.values()):
        raise ValueError("Linux build toolchain／host identityのversion情報が不足している")
    return versions


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _aot_report_root(report: dict[str, Any]) -> dict[str, Any]:
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
        raise ValueError("Flutter Linux size-analysis reportにDart AOT treeが一意に存在しない")
    return roots[0]


def _aot_surface_libraries(
    report: dict[str, Any], catalog: dict[str, Any]
) -> list[str]:
    """Flutter Linux AOT report内のCatalog対象Dart libraryだけを列挙する。"""
    targets: dict[str, tuple[str, ...]] = {}
    lib_root = PurePosixPath("apps/desktop_flutter/lib")
    for module in catalog["optional_modules"]:
        for surface in module["source_surfaces"]:
            source = PurePosixPath(surface)
            try:
                relative = source.relative_to(lib_root)
            except ValueError as error:
                raise ValueError(
                    f"Catalog surfaceがDesktop Flutter lib外にある: {surface}"
                ) from error
            if not relative.parts or source.suffix != ".dart":
                raise ValueError(f"Catalog surfaceがDart library pathではない: {surface}")
            targets[surface] = (*PACKAGE_REPORT_ROOT, *relative.parts)

    found: set[str] = set()
    pending: list[tuple[Any, tuple[str, ...]]] = [(_aot_report_root(report), ())]
    while pending:
        value, ancestors = pending.pop()
        if isinstance(value, dict):
            name = value.get("n")
            path = (*ancestors, name) if isinstance(name, str) else ancestors
            for surface, target in targets.items():
                if len(path) >= len(target) and path[-len(target) :] == target:
                    found.add(surface)
            pending.extend((child, path) for child in value.values())
        elif isinstance(value, list):
            pending.extend((child, ancestors) for child in value)
    return sorted(found)


def _module_source_surfaces(
    catalog: dict[str, Any], included_module_ids: set[str]
) -> tuple[list[str], list[str]]:
    included = sorted(
        surface
        for module in catalog["optional_modules"]
        if module["module_id"] in included_module_ids
        for surface in module["source_surfaces"]
    )
    excluded = sorted(
        surface
        for module in catalog["optional_modules"]
        if module["module_id"] not in included_module_ids
        for surface in module["source_surfaces"]
    )
    return included, excluded


def _validate_aot_surface_selection(
    report: dict[str, Any],
    catalog: dict[str, Any],
    included_module_ids: set[str],
) -> tuple[list[str], list[str]]:
    actual = _aot_surface_libraries(report, catalog)
    expected, excluded = _module_source_surfaces(catalog, included_module_ids)
    if actual != expected:
        missing = sorted(set(expected) - set(actual))
        unexpected = sorted(set(actual) - set(expected))
        raise ValueError(
            "Flutter Linux Release AOT libraryがModule選択と一致しない: "
            f"missing={missing}, unexpected={unexpected}"
        )
    return actual, excluded


def _size_analysis_report_path(output: str, isolated_home: Path) -> Path:
    matches = [
        line.split(SIZE_ANALYSIS_SENTINEL, 1)[1].strip()
        for line in output.splitlines()
        if SIZE_ANALYSIS_SENTINEL in line
    ]
    if not matches or not matches[-1]:
        raise ValueError("FlutterがLinux size-analysis report pathを出力しなかった")
    report_path = Path(matches[-1].strip('"'))
    if not report_path.is_absolute():
        raise ValueError("Flutter size-analysis report pathが絶対pathではない")
    devtools_root = isolated_home / ".flutter-devtools"
    root_metadata = devtools_root.lstat()
    if stat.S_ISLNK(root_metadata.st_mode) or not stat.S_ISDIR(root_metadata.st_mode):
        raise ValueError("Flutter size-analysis directoryが通常directoryではない")
    metadata = report_path.lstat()
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        raise ValueError("Flutter size-analysis reportが通常fileではない")
    if metadata.st_size <= 0 or metadata.st_size > SIZE_ANALYSIS_MAX_BYTES:
        raise ValueError("Flutter size-analysis reportが空か50 MB上限を超えている")
    try:
        report_path.resolve(strict=True).relative_to(devtools_root.resolve(strict=True))
    except (OSError, ValueError) as error:
        raise ValueError("Flutter size-analysis reportが隔離DevTools directory外にある") from error
    return report_path


def _size_analysis_input_records(directory: Path) -> list[dict[str, Any]]:
    snapshots = sorted(directory.glob("snapshot.*.json"))
    traces = sorted(directory.glob("trace.*.json"))
    if (
        len(snapshots) != 1
        or len(traces) != 1
        or snapshots[0].name != "snapshot.linux-x64.json"
        or traces[0].name != "trace.linux-x64.json"
    ):
        raise ValueError("Flutter Linux size-analysis snapshot／traceが一意に揃わない")
    records: list[dict[str, Any]] = []
    for path in [*snapshots, *traces]:
        metadata = path.lstat()
        if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
            raise ValueError("Flutter size-analysis入力が通常fileではない")
        if metadata.st_size <= 0 or metadata.st_size > SIZE_ANALYSIS_MAX_BYTES:
            raise ValueError("Flutter size-analysis入力が空か50 MB上限を超えている")
        records.append(
            {
                "name": path.name,
                "size_bytes": metadata.st_size,
                "sha256": _sha256_file(path),
            }
        )
    return records


def _capture_linux_aot_surface_analysis(
    *,
    output: str,
    isolated_home: Path,
    input_directory: Path,
    build_directory: Path,
    staging_root: Path,
    catalog: dict[str, Any],
    included_module_ids: set[str],
) -> dict[str, Any]:
    report_path = _size_analysis_report_path(output, isolated_home)
    report_raw = _read_regular_file(
        report_path, SIZE_ANALYSIS_MAX_BYTES, "Flutter Linux size-analysis report"
    )
    report = _parse_json_bytes(report_raw)
    if not isinstance(report, dict) or report.get("type") != "linux":
        raise ValueError("Flutter size-analysis reportがLinux形式ではない")
    aot_root = _aot_report_root(report)
    profile_aot = build_directory / "bundle" / "lib" / "libapp.so"
    profile_metadata = profile_aot.lstat()
    if stat.S_ISLNK(profile_metadata.st_mode) or not stat.S_ISREG(profile_metadata.st_mode):
        raise ValueError("Flutter Release outputに通常のlibapp.soがない")
    if aot_root.get("value") != profile_metadata.st_size:
        raise ValueError("Flutter AOT reportのsizeが同じbuildのlibapp.soと一致しない")
    surfaces, excluded_surfaces = _validate_aot_surface_selection(
        report, catalog, included_module_ids
    )
    analysis_directory = staging_root / "analysis"
    analysis_directory.mkdir(mode=0o700, exist_ok=True)
    report_destination = analysis_directory / "linux-code-size-analysis.json"
    with report_destination.open("xb") as destination:
        destination.write(report_raw)
        destination.flush()
        os.fsync(destination.fileno())
    report_record = {
        "path": "analysis/linux-code-size-analysis.json",
        "size_bytes": len(report_raw),
        "sha256": hashlib.sha256(report_raw).hexdigest(),
    }
    included_surfaces, _ = _module_source_surfaces(catalog, included_module_ids)
    return {
        "scope": "flutter_linux_x64_release_aot_catalog_surfaces",
        "report": report_record,
        "inputs": _size_analysis_input_records(input_directory),
        "aot_surface_libraries": surfaces,
        "included_source_surfaces": included_surfaces,
        "excluded_source_surfaces": excluded_surfaces,
        "profiled_aot_image_size_bytes": profile_metadata.st_size,
        "profiled_aot_image_sha256": _sha256_file(profile_aot),
    }


def _write_source_archive(commit: str, archive_path: Path) -> str:
    with archive_path.open("xb") as archive:
        subprocess.run(
            ["git", "-C", str(ROOT), "archive", "--format=tar", commit],
            check=True,
            stdout=archive,
            env=_linux_build_environment({}),
        )
    return _sha256_file(archive_path)


def _portable_archive_plan(
    bundle: Path, readme_raw: bytes, artifact_records: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    root_metadata = bundle.lstat()
    if stat.S_ISLNK(root_metadata.st_mode) or not stat.S_ISDIR(root_metadata.st_mode):
        raise ValueError("Linux portable archiveのbundle rootが通常directoryではない")
    if not readme_raw or len(readme_raw) > PORTABLE_README_MAX_BYTES:
        raise ValueError("Linux portable archiveのREADMEが空または上限超過")

    current_records, _total_bytes, _tree_hash = _artifact_inventory(bundle)
    if current_records != artifact_records:
        raise ValueError("Linux portable archiveのbundle inventoryが変化した")
    expected_files = {record["path"]: record for record in artifact_records}
    plan: list[dict[str, Any]] = [
        {
            "name": PORTABLE_ARCHIVE_ROOT,
            "kind": "directory",
            "path": None,
            "payload": None,
            "mode": 0o755,
            "size": 0,
            "sha256": None,
        },
        {
            "name": f"{PORTABLE_ARCHIVE_ROOT}/{PORTABLE_README_MEMBER}",
            "kind": "file",
            "path": None,
            "payload": readme_raw,
            "mode": 0o644,
            "size": len(readme_raw),
            "sha256": hashlib.sha256(readme_raw).hexdigest(),
        },
    ]
    seen_files: set[str] = set()
    children = sorted(bundle.rglob("*"), key=lambda item: item.relative_to(bundle).as_posix())
    for path in children:
        relative = path.relative_to(bundle).as_posix()
        pure = PurePosixPath(relative)
        if (
            not relative
            or "\\" in relative
            or pure.is_absolute()
            or pure.as_posix() != relative
            or any(part in {"", ".", ".."} for part in relative.split("/"))
        ):
            raise ValueError("Linux portable archiveに不正な相対pathがある")
        if relative == PORTABLE_README_MEMBER:
            raise ValueError("Linux portable archiveのbundleが予約済みREADME名を使用している")
        metadata = path.lstat()
        if stat.S_ISLNK(metadata.st_mode):
            raise ValueError("Linux portable archiveはsymlinkを含められない")
        member_name = f"{PORTABLE_ARCHIVE_ROOT}/{relative}"
        if stat.S_ISDIR(metadata.st_mode):
            plan.append(
                {
                    "name": member_name,
                    "kind": "directory",
                    "path": None,
                    "payload": None,
                    "mode": 0o755,
                    "size": 0,
                    "sha256": None,
                }
            )
            continue
        if not stat.S_ISREG(metadata.st_mode):
            raise ValueError("Linux portable archiveに通常file／directory以外がある")
        record = expected_files.get(relative)
        if record is None:
            raise ValueError("Linux portable archive fileがbundle inventoryにない")
        expected_mode = 0o755 if relative in EXECUTABLE_PATHS else 0o644
        source_mode = stat.S_IMODE(metadata.st_mode)
        if source_mode & 0o7022 or not source_mode & 0o400:
            raise ValueError(
                f"Linux portable archive source permissionが安全でない: {relative}"
            )
        if relative in EXECUTABLE_PATHS and not source_mode & 0o100:
            raise ValueError(
                f"Linux portable archiveの実行fileにowner execute bitがない: {relative}"
            )
        seen_files.add(relative)
        plan.append(
            {
                "name": member_name,
                "kind": "file",
                "path": path,
                "payload": None,
                "mode": expected_mode,
                "size": record["size_bytes"],
                "sha256": record["sha256"],
            }
        )
    if seen_files != set(expected_files):
        raise ValueError("Linux portable archive file setがbundle inventoryと異なる")
    return [plan[0], *sorted(plan[1:], key=lambda item: item["name"])]


def _write_portable_archive(
    bundle: Path,
    archive_path: Path,
    readme_raw: bytes,
    artifact_records: list[dict[str, Any]],
) -> dict[str, Any]:
    plan = _portable_archive_plan(bundle, readme_raw, artifact_records)
    with archive_path.open("xb") as raw_output:
        with gzip.GzipFile(
            filename="", fileobj=raw_output, mode="wb", compresslevel=9, mtime=0
        ) as compressed:
            with tarfile.open(fileobj=compressed, mode="w|", format=tarfile.PAX_FORMAT) as archive:
                for item in plan:
                    info = tarfile.TarInfo(item["name"])
                    info.uid = 0
                    info.gid = 0
                    info.uname = ""
                    info.gname = ""
                    info.mtime = 0
                    info.mode = item["mode"]
                    if item["kind"] == "directory":
                        info.type = tarfile.DIRTYPE
                        info.size = 0
                        archive.addfile(info)
                    else:
                        info.type = tarfile.REGTYPE
                        info.size = item["size"]
                        if item["path"] is None:
                            source = io.BytesIO(item["payload"])
                        else:
                            source = item["path"].open("rb")
                        with source:
                            archive.addfile(info, source)
        raw_output.flush()
        os.fsync(raw_output.fileno())

    metadata = archive_path.lstat()
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        raise ValueError("Linux portable archive outputが通常fileではない")
    return {
        "file_name": archive_path.name,
        "format": "tar.gz",
        "target": PORTABLE_ARCHIVE_TARGET,
        "byte_length": metadata.st_size,
        "sha256": _sha256_file(archive_path),
        "entry_count": len(plan),
        "readme_sha256": hashlib.sha256(readme_raw).hexdigest(),
        "signed": False,
    }


def _validate_portable_archive(
    bundle: Path,
    archive_path: Path,
    readme_raw: bytes,
    artifact_records: list[dict[str, Any]],
    record: dict[str, Any],
) -> None:
    metadata = archive_path.lstat()
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        raise ValueError("Linux portable archiveが通常fileではない")
    if (
        metadata.st_size != record["byte_length"]
        or _sha256_file(archive_path) != record["sha256"]
        or record["format"] != "tar.gz"
        or record["target"] != PORTABLE_ARCHIVE_TARGET
        or record["entry_count"] < 2
        or record["readme_sha256"] != hashlib.sha256(readme_raw).hexdigest()
        or record["signed"] is not False
    ):
        raise ValueError("Linux portable archive identity／hashがbuild evidenceと一致しない")

    plan = _portable_archive_plan(bundle, readme_raw, artifact_records)
    try:
        with tarfile.open(archive_path, mode="r:gz") as archive:
            members = archive.getmembers()
            if len(members) != len(plan) or len(members) != record["entry_count"]:
                raise ValueError("Linux portable archive entry数が一致しない")
            names = [member.name for member in members]
            if names != [item["name"] for item in plan] or len(set(names)) != len(names):
                raise ValueError("Linux向け可搬packageの構成要素の位置と並び順が一致しない")
            for member, item in zip(members, plan, strict=True):
                pure = PurePosixPath(member.name)
                if (
                    "\\" in member.name
                    or pure.is_absolute()
                    or pure.as_posix() != member.name
                    or any(part in {"", ".", ".."} for part in member.name.split("/"))
                    or member.uid != 0
                    or member.gid != 0
                    or member.uname != ""
                    or member.gname != ""
                    or member.mtime != 0
                    or member.mode != item["mode"]
                    or member.pax_headers.keys() - {"path"}
                ):
                    raise ValueError("Linux portable archive metadataが安全な固定値ではない")
                if item["kind"] == "directory":
                    if not member.isdir() or member.size != 0:
                        raise ValueError("Linux portable archive directory entryが不正")
                    continue
                if not member.isfile() or member.size != item["size"]:
                    raise ValueError("Linux portable archive file entryが不正")
                source = archive.extractfile(member)
                if source is None:
                    raise ValueError("Linux portable archive fileを読めない")
                digest = hashlib.sha256()
                total = 0
                with source:
                    for chunk in iter(lambda: source.read(1024 * 1024), b""):
                        total += len(chunk)
                        digest.update(chunk)
                if total != item["size"] or digest.hexdigest() != item["sha256"]:
                    raise ValueError("Linux portable archive内容がbundle／READMEと不一致")
    except (tarfile.TarError, OSError) as error:
        raise ValueError("Linux portable archiveを安全に検証できない") from error


def _validate_aot_surface_evidence(
    evidence: dict[str, Any], bundle: Path
) -> None:
    analysis = evidence["aot_surface_analysis"]
    output_root = bundle.parent
    analysis_directory = output_root / "analysis"
    directory_metadata = analysis_directory.lstat()
    if stat.S_ISLNK(directory_metadata.st_mode) or not stat.S_ISDIR(directory_metadata.st_mode):
        raise ValueError("Linux Export AOT analysis directoryが通常directoryではない")
    report_record = analysis["report"]
    relative = PurePosixPath(report_record["path"])
    if (
        relative.is_absolute()
        or not relative.parts
        or any(part in {"", ".", ".."} for part in relative.parts)
    ):
        raise ValueError("Linux Export AOT report pathが安全な相対pathではない")
    report_path = output_root.joinpath(*relative.parts)
    report_raw = _read_regular_file(
        report_path, SIZE_ANALYSIS_MAX_BYTES, "Linux Export AOT report"
    )
    if (
        len(report_raw) != report_record["size_bytes"]
        or hashlib.sha256(report_raw).hexdigest() != report_record["sha256"]
    ):
        raise ValueError("Linux Export AOT report hash／sizeがevidenceと一致しない")
    report = _parse_json_bytes(report_raw)
    if not isinstance(report, dict) or report.get("type") != "linux":
        raise ValueError("Linux Export AOT reportのplatformが一致しない")
    catalog = _load_catalog()
    included_ids = set(evidence["module_plan"]["included_module_ids"])
    actual, excluded = _validate_aot_surface_selection(report, catalog, included_ids)
    included, _ = _module_source_surfaces(catalog, included_ids)
    if (
        analysis["aot_surface_libraries"] != actual
        or analysis["included_source_surfaces"] != included
        or analysis["excluded_source_surfaces"] != excluded
    ):
        raise ValueError("Linux Export AOT source surface記録がModulePlan／reportと一致しない")
    aot_record = next(
        (record for record in evidence["artifact_files"] if record["path"] == "lib/libapp.so"),
        None,
    )
    if aot_record is None:
        raise ValueError("Linux Export artifact inventoryにlibapp.soがない")
    aot_root = _aot_report_root(report)
    if (
        analysis["profiled_aot_image_size_bytes"] != aot_record["size_bytes"]
        or analysis["exported_aot_image_size_bytes"] != aot_record["size_bytes"]
        or analysis["profiled_aot_image_sha256"] != aot_record["sha256"]
        or analysis["exported_aot_image_sha256"] != aot_record["sha256"]
        or analysis["image_matches_export"] is not True
        or aot_root.get("value") != aot_record["size_bytes"]
    ):
        raise ValueError("Linux Export AOT report／profile／bundle libapp.so identityが一致しない")
    input_records = analysis["inputs"]
    if {record["name"] for record in input_records} != {
        "snapshot.linux-x64.json",
        "trace.linux-x64.json",
    }:
        raise ValueError("Linux Export AOT analysis input setが不完全")
    if len({record["name"] for record in input_records}) != len(input_records):
        raise ValueError("Linux Export AOT analysis input nameが重複している")


def validate_build_evidence(
    evidence: dict[str, Any],
    bundle: Path,
    package_path: Path,
    manifest_raw: bytes,
    portable_archive_path: Path | None = None,
    portable_readme_raw: bytes | None = None,
) -> None:
    schema = _schema(BUILD_SCHEMA_NAME)
    errors = validate_instance(evidence, schema)
    if errors:
        raise ValueError("Linux Export build evidenceがSchemaに適合しない")
    app_id = evidence["app_id"]
    audit_store_id = evidence["audit_store_id"]
    expected_manifest_hash = hashlib.sha256(manifest_raw).hexdigest()
    if evidence["source_manifest_sha256"] != expected_manifest_hash:
        raise ValueError("build evidenceのsource Manifest hashが入力fileと一致しない")
    if evidence["compile_time_identity"] != {
        "source": "schema_validated_manifest",
        "app_id": app_id,
        "audit_store_id": audit_store_id,
        "manifest_sha256": evidence["source_manifest_sha256"],
    }:
        raise ValueError("compile-time identityがBroker Export Manifestと一致しない")
    records, total_bytes, tree_hash = _artifact_inventory(bundle)
    if (
        records != evidence["artifact_files"]
        or total_bytes != evidence["artifact_total_bytes"]
        or tree_hash != evidence["artifact_tree_sha256"]
    ):
        raise ValueError("Linux Export evidenceとbundle inventoryが一致しない")
    product_manifest = _read_regular_file(
        bundle / "product_manifest.json", MAX_MANIFEST_BYTES, "Linux Product Manifest"
    )
    if product_manifest != manifest_raw:
        raise ValueError("Linux Product Manifestのbyte列がBroker入力から変化した")
    parsed_manifest = _parse_json_bytes(product_manifest)
    if (
        parsed_manifest["manifest"]["app_identity"]["app_id"] != app_id
        or parsed_manifest["manifest"]["audit_store"]["store_id"] != audit_store_id
        or parsed_manifest["manifest"]["app_identity"]["target_platform"] != "linux"
    ):
        raise ValueError("bundle Manifest identity／targetがbuild evidenceと一致しない")
    manifest_module_plan = parsed_manifest["manifest"]["module_plan"]
    for evidence_key, manifest_key in (
        ("selection_mode", "selection_mode"),
        ("requested_optional_module_ids", "requested_optional_module_ids"),
        ("unprunable_core_ids", "unprunable_core_ids"),
        ("included_module_ids", "included_module_ids"),
        ("excluded_optional_module_ids", "excluded_optional_module_ids"),
    ):
        if evidence["module_plan"][evidence_key] != manifest_module_plan[manifest_key]:
            raise ValueError("Linux Export ModulePlanがManifestと一致しない")
    _validate_aot_surface_evidence(evidence, bundle)
    if _update_trust_summary(parsed_manifest) != evidence["compile_time_update_trust"]:
        raise ValueError("Rust compile-time update trustがManifestと一致しない")
    scan = _scan_credential_inventory(bundle, records, total_bytes, tree_hash)
    if (
        scan != evidence["credential_artifact_scan"]
        or evidence["authority_boundary"]["credential_artifact_scan_status"] != scan["status"]
    ):
        raise ValueError("bounded Credential scanがLinux bundle hashへ結合されない")
    metadata = package_path.lstat()
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        raise ValueError("Linux D4PKG01 outputが通常fileではない")
    package_size = metadata.st_size
    package_hash = _sha256_file(package_path)
    package_record = evidence["package"]
    if (
        package_record["byte_length"] != package_size
        or package_record["sha256"] != package_hash
        or package_record["file_name"] != package_path.name
    ):
        raise ValueError("Linux D4PKG01がbuild evidenceと一致しない")
    portable_record = evidence["portable_archive"]
    if portable_record is None:
        if portable_archive_path is not None or portable_readme_raw is not None:
            raise ValueError("Linux portable archiveの対象外buildにarchive入力がある")
    else:
        if portable_archive_path is None or portable_readme_raw is None:
            raise ValueError("Linux portable archive evidenceにarchive／README入力がない")
        if portable_record["file_name"] != portable_archive_path.name:
            raise ValueError("Linux portable archive file nameがevidenceと一致しない")
        _validate_portable_archive(
            bundle,
            portable_archive_path,
            portable_readme_raw,
            records,
            portable_record,
        )


def build_linux_product(
    receipt_path: Path,
    manifest_path: Path,
    output_dir: Path,
    *,
    allow_verification_branch: bool = False,
    r2_e2e_test_build: bool = False,
) -> dict[str, Any]:
    if not sys.platform.startswith("linux") or not hasattr(os, "O_NOFOLLOW"):
        raise ValueError("Linux Export buildはLinux host専用である")
    source_branch, commit = _require_clean_source(
        allow_verification_branch=allow_verification_branch
    )
    output = _resolve_output_directory(output_dir)
    receipt_raw = _read_regular_file(receipt_path, MAX_INPUT_BYTES, "Export Receipt")
    manifest_raw = _read_regular_file(manifest_path, MAX_MANIFEST_BYTES, "Export Manifest file")
    receipt, manifest_file, plan, receipt_hash, manifest_hash = validate_export_inputs(
        receipt_raw, manifest_raw, target_platform="linux"
    )
    if manifest_path.name != receipt["manifest_file"]["file_name"]:
        raise ValueError("入力Manifest file名がBroker Receiptと一致しない")
    app_id = manifest_file["manifest"]["app_identity"]["app_id"]
    audit_store_id = manifest_file["manifest"]["audit_store"]["store_id"]
    if r2_e2e_test_build:
        if not allow_verification_branch:
            raise ValueError("R2 E2E buildは明示された手動検証buildだけで有効化できる")
        if (
            app_id != "d4-pocket-app-11111111111111111111111111111111"
            or audit_store_id != "audit-store-22222222222222222222222222222222"
        ):
            raise ValueError("R2 E2E buildは固定test identityだけを受け付ける")
    version = product_version_from_source(ROOT)
    paths = _tool_paths()
    rustup_home = _rustup_home()
    defines = dart_defines(plan)

    build_started = time.perf_counter()
    with tempfile.TemporaryDirectory(prefix=OUTPUT_TEMP_PREFIX, dir=output.parent) as output_temp:
        staging_root = Path(output_temp) / "product"
        staging_root.mkdir()
        bundle = staging_root / "bundle"
        package_path = staging_root / "D4Pocket-linux.d4pkg"
        with tempfile.TemporaryDirectory(
            prefix=BUILD_TEMP_PREFIX, dir=_resolve_build_temp_root()
        ) as build_temp:
            temporary_root = Path(build_temp)
            isolated_home = temporary_root / "home"
            isolated_home.mkdir(mode=0o700)
            source_root = temporary_root / "source"
            source_root.mkdir()
            archive_path = temporary_root / "source.tar"
            archive_hash = _write_source_archive(commit, archive_path)
            _safe_extract_source(archive_path, source_root)

            flutter_project = source_root / "apps" / "desktop_flutter"
            build_dir = flutter_project / "build" / "linux" / "x64" / "release"
            product_version = product_version_from_source(source_root)
            update_trust = manifest_file.get("update_trust")
            build_overrides = {
                "HOME": str(isolated_home),
                "XDG_CACHE_HOME": str(temporary_root / "cache"),
                "TMPDIR": str(temporary_root / "tmp"),
                "PUB_CACHE": str(temporary_root / "pub"),
                "CARGO_HOME": str(temporary_root / "cargo"),
                "RUSTUP_HOME": rustup_home,
                "GUI_SHELL_UPDATE_TRUST_JSON": json.dumps(
                    update_trust, ensure_ascii=False, sort_keys=True, separators=(",", ":")
                ) if update_trust is not None else "",
            }
            (temporary_root / "cache").mkdir()
            (temporary_root / "tmp").mkdir()
            (temporary_root / "pub").mkdir()
            (temporary_root / "cargo").mkdir()
            if update_trust is None:
                build_overrides.pop("GUI_SHELL_UPDATE_TRUST_JSON")
            child_environment = _linux_build_environment(build_overrides)
            flutter_environment = _flutter_build_environment(child_environment)
            toolchain = _toolchain(paths, child_environment)
            pub_arguments = ["pub", "get", "--enforce-lockfile"]
            subprocess.run(
                [paths["flutter"], *pub_arguments],
                cwd=flutter_project,
                env=flutter_environment,
                check=True,
            )
            catalog = _load_catalog()
            included_module_ids = set(plan.included_optional_ids)
            dart_define_arguments = [
                f"--dart-define={name}={str(enabled).lower()}"
                for name, enabled in defines.items()
            ]
            size_analysis_input_directory = temporary_root / "size-analysis-inputs"
            size_analysis_input_directory.mkdir(mode=0o700)
            flutter_size_analysis_arguments = [
                "build",
                "linux",
                "--release",
                "--no-pub",
                "--target-platform=linux-x64",
                "--analyze-size",
                f"--code-size-directory={size_analysis_input_directory}",
                *dart_define_arguments,
                f"--dart-define=GUI_SHELL_PRODUCT_VERSION={product_version}",
            ]
            size_analysis_result = subprocess.run(
                [paths["flutter"], *flutter_size_analysis_arguments],
                cwd=flutter_project,
                env=flutter_environment,
                check=False,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                encoding="utf-8",
                errors="replace",
            )
            size_analysis_output = size_analysis_result.stdout or ""
            if size_analysis_result.returncode != 0:
                if size_analysis_output:
                    sys.stdout.write(size_analysis_output)
                raise subprocess.CalledProcessError(
                    size_analysis_result.returncode,
                    [paths["flutter"], *flutter_size_analysis_arguments],
                    output=size_analysis_output,
                )
            for line in size_analysis_output.splitlines():
                if (
                    SIZE_ANALYSIS_SENTINEL in line
                    or "libapp.so (Dart AOT)" in line
                    or line.startswith("✓ Built ")
                ):
                    print(line)
            aot_surface_analysis = _capture_linux_aot_surface_analysis(
                output=size_analysis_output,
                isolated_home=isolated_home,
                input_directory=size_analysis_input_directory,
                build_directory=build_dir,
                staging_root=staging_root,
                catalog=catalog,
                included_module_ids=included_module_ids,
            )

            flutter_arguments = [
                "build",
                "linux",
                "--release",
                "--no-pub",
                "--target-platform=linux-x64",
                "--config-only",
                *dart_define_arguments,
                f"--dart-define=GUI_SHELL_PRODUCT_VERSION={product_version}",
            ]
            subprocess.run(
                [paths["flutter"], *flutter_arguments],
                cwd=flutter_project,
                env=flutter_environment,
                check=True,
            )

            cmake_configure_arguments = [
                "-S", str(flutter_project / "linux"),
                "-B", str(build_dir),
                "-G", "Ninja",
                "-DCMAKE_BUILD_TYPE=Release",
                "-DFLUTTER_TARGET_PLATFORM=linux-x64",
                f"-DCMAKE_INSTALL_PREFIX={bundle}",
                f"-DD4_PRODUCT_APP_ID={app_id}",
                f"-DD4_PRODUCT_AUDIT_STORE_ID={audit_store_id}",
                f"-DD4_PRODUCT_MANIFEST_SHA256={manifest_hash}",
                f"-DD4_LINUX_R2_E2E_TEST_BUILD={'ON' if r2_e2e_test_build else 'OFF'}",
            ]
            subprocess.run(
                [paths["cmake"], *cmake_configure_arguments],
                cwd=source_root,
                env={**child_environment, "CC": "clang", "CXX": "clang++"},
                check=True,
            )
            cmake_build_arguments = ["--build", str(build_dir), "--target", "install"]
            subprocess.run(
                [paths["cmake"], *cmake_build_arguments],
                cwd=source_root,
                env=child_environment,
                check=True,
            )
            if not bundle.is_dir() or bundle.is_symlink():
                raise ValueError("CMake installが固定Linux bundleを生成しなかった")
            exported_aot = bundle / "lib" / "libapp.so"
            exported_aot_metadata = exported_aot.lstat()
            if (
                stat.S_ISLNK(exported_aot_metadata.st_mode)
                or not stat.S_ISREG(exported_aot_metadata.st_mode)
            ):
                raise ValueError("Linux Export bundleに通常のlibapp.soがない")
            exported_aot_hash = _sha256_file(exported_aot)
            if (
                exported_aot_metadata.st_size
                != aot_surface_analysis["profiled_aot_image_size_bytes"]
                or exported_aot_hash
                != aot_surface_analysis["profiled_aot_image_sha256"]
            ):
                raise ValueError(
                    "size-analysis対象libapp.soと最終Export bundleのfile identityが一致しない"
                )
            aot_surface_analysis["exported_aot_image_size_bytes"] = (
                exported_aot_metadata.st_size
            )
            aot_surface_analysis["exported_aot_image_sha256"] = exported_aot_hash
            aot_surface_analysis["image_matches_export"] = True
            manifest_target = bundle / "product_manifest.json"
            if manifest_target.exists() or manifest_target.is_symlink():
                raise ValueError("build bundleに入力Manifest名と衝突するfileがある")
            with manifest_target.open("xb") as destination:
                destination.write(manifest_raw)
                destination.flush()
                os.fsync(destination.fileno())

            package_result = write_product_package(
                bundle, package_path, product_version=product_version
            )
            artifact_files, total_bytes, tree_hash = _artifact_inventory(bundle)
            portable_archive_record = None
            portable_archive_path = None
            portable_readme_raw = None
            if (
                toolchain["distribution_id"] == "ubuntu"
                and toolchain["distribution_version"] == "24.04"
                and toolchain["architecture"] == "x86_64"
            ):
                portable_readme_path = source_root / "installer" / "linux" / "PORTABLE_README_ja.txt"
                portable_readme_raw = _read_regular_file(
                    portable_readme_path,
                    PORTABLE_README_MAX_BYTES,
                    "Linux portable archive README",
                )
                portable_archive_path = staging_root / (
                    f"D4Pocket-{product_version}-linux-x64-portable.tar.gz"
                )
                portable_archive_record = _write_portable_archive(
                    bundle, portable_archive_path, portable_readme_raw, artifact_files
                )
            credential_scan = _scan_credential_inventory(
                bundle, artifact_files, total_bytes, tree_hash
            )
            elapsed_ms = round((time.perf_counter() - build_started) * 1000)
            evidence = {
                "version": 1,
                "evidence_type": "d4_pocket_linux_export_build",
                "evidence_source": "INTERNAL_STATE",
                "product": "D4 Pocket",
                "target_platform": "linux",
                "build_mode": "release",
                "source_ref": f"refs/heads/{source_branch}",
                "source_commit": commit,
                "source_archive_sha256": archive_hash,
                "source_receipt_sha256": receipt_hash,
                "source_manifest_sha256": manifest_hash,
                "export_id": receipt["export_id"],
                "app_id": app_id,
                "audit_store_id": audit_store_id,
                "authority_boundary": {
                    "source_authority_verified": False,
                    "owner_authorization_verified": False,
                    "authority_strip": True,
                    "credential_values_explicitly_supplied": False,
                    "credential_artifact_scan_status": CREDENTIAL_SCAN_STATUS,
                    "credential_inherited": False,
                    "permission_inherited": False,
                    "approval_inherited": False,
                    "audit_chain_inherited": False,
                },
                "module_plan": {
                    "catalog_version": 1,
                    "selection_mode": plan.selection_mode,
                    "requested_optional_module_ids": list(plan.requested_optional_ids),
                    "unprunable_core_ids": receipt["export_manifest"]["module_plan"]["unprunable_core_ids"],
                    "included_module_ids": list(plan.included_module_ids),
                    "excluded_optional_module_ids": list(plan.excluded_optional_ids),
                },
                "aot_surface_analysis": aot_surface_analysis,
                "toolchain": toolchain,
                "pub_get_arguments": pub_arguments,
                "flutter_build_arguments": flutter_arguments,
                "flutter_size_analysis_arguments": flutter_size_analysis_arguments,
                "cmake_configure_arguments": cmake_configure_arguments,
                "cmake_build_arguments": cmake_build_arguments,
                "compile_time_identity": {
                    "source": "schema_validated_manifest",
                    "app_id": app_id,
                    "audit_store_id": audit_store_id,
                    "manifest_sha256": manifest_hash,
                },
                "compile_time_update_trust": _update_trust_summary(manifest_file),
                "build_duration_ms": elapsed_ms,
                "runtime_storage_root_template": "$XDG_DATA_HOME/apps/d4-pocket/<App ID>/stores/<Audit store ID>",
                "portable_bundle_assembled": True,
                "manifest_copied_byte_for_byte": True,
                "runtime_manifest_consumed": False,
                "binary_pruning_verified": False,
                "formal_distribution_claimed": False,
                "signed": False,
                "installer_status": "not_started",
                "launch_status": "not_verified",
                "artifact_root": "bundle",
                "artifact_files": artifact_files,
                "artifact_total_bytes": total_bytes,
                "artifact_tree_sha256": tree_hash,
                "package": {
                    "file_name": package_path.name,
                    "format": "D4PKG01",
                    "byte_length": package_result["package_size_bytes"],
                    "sha256": package_result["package_sha256"],
                    "signed": False,
                    "installed_product_verified": False,
                },
                "portable_archive": portable_archive_record,
                "credential_artifact_scan": credential_scan,
            }
            if portable_archive_record is None:
                validate_build_evidence(evidence, bundle, package_path, manifest_raw)
            else:
                if portable_archive_path is None or portable_readme_raw is None:
                    raise ValueError("portable archiveの検証入力が不足している")
                validate_build_evidence(
                    evidence,
                    bundle,
                    package_path,
                    manifest_raw,
                    portable_archive_path,
                    portable_readme_raw,
                )
            (staging_root / RECEIPT_NAME).write_text(
                json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
            )
        if output.exists() or output.is_symlink():
            raise ValueError("build中にoutput directoryが作成されたため公開を停止した")
        os.rename(staging_root, output)
    return evidence


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Broker Receipt／ManifestからLinux Release bundle、D4PKG01、build provenanceを作る"
    )
    parser.add_argument("--receipt", type=Path, required=True, help="Brokerが返したExport Receipt")
    parser.add_argument("--manifest-file", type=Path, required=True, help="Receiptが束縛するManifest file")
    parser.add_argument("--output-dir", type=Path, required=True, help="Repository／OneDrive外の新規出力root")
    parser.add_argument(
        "--allow-verification-branch",
        action="store_true",
        help="cleanでremote HEAD一致のcodex/* branchを手動検証buildに限り許可する",
    )
    parser.add_argument(
        "--r2-e2e-test-build",
        action="store_true",
        help="固定test identity限定でLinux installed Agent Task acceptance buildを生成する",
    )
    args = parser.parse_args()
    try:
        evidence = build_linux_product(
            args.receipt,
            args.manifest_file,
            args.output_dir,
            allow_verification_branch=args.allow_verification_branch,
            r2_e2e_test_build=args.r2_e2e_test_build,
        )
    except Exception as error:
        message = f": {error}" if isinstance(error, ValueError) else ""
        print(
            f"Linux Export buildを停止した: {type(error).__name__}{message}",
            file=sys.stderr,
        )
        return 1
    print(
        json.dumps(
            {
                "output_dir": str(args.output_dir.resolve()),
                "bundle": str((args.output_dir / "bundle").resolve()),
                "package_sha256": evidence["package"]["sha256"],
                "artifact_tree_sha256": evidence["artifact_tree_sha256"],
                "source_commit": evidence["source_commit"],
                "signed": False,
                "installed_product_verified": False,
                "r2_e2e_test_build": args.r2_e2e_test_build,
            },
            ensure_ascii=False,
            separators=(",", ":"),
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
