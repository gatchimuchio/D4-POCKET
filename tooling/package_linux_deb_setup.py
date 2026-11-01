"""Ubuntu 24.04向けD4 Pocket Setup bootstrapを.debへ梱包する開発用tool。"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import stat
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.export_linux_product import validate_build_evidence
from tooling.export_windows_product import (
    _artifact_inventory,
    _read_regular_file,
    product_version_from_source,
)
from tooling.package_linux_product import EXECUTABLE_PATHS

PACKAGE_NAME = "d4-pocket-setup"
TARGET = "ubuntu-24.04-amd64"
INSTALL_ROOT = PurePosixPath("opt/d4-pocket/setup")
DESKTOP_ENTRY = PurePosixPath("usr/share/applications/d4-pocket-setup.desktop")
MAX_EVIDENCE_BYTES = 4 * 1024 * 1024
MAX_MANIFEST_BYTES = 64 * 1024
MAX_CONTROL_BYTES = 64 * 1024
DEPENDENCY_ALTERNATIVE = re.compile(
    r"[a-z0-9][a-z0-9+.-]*(?::[a-z0-9-]+)?"
    r"(?:\s*\(\s*(?:<<|<=|=|>=|>>)\s*[A-Za-z0-9.+:~\-]+\s*\))?\Z"
)
VERSION_COMPONENT = re.compile(r"[0-9][A-Za-z0-9.+:~]*\Z")
EXECUTABLES = (
    "gui_shell_desktop",
    "gui_shell_desktop_launcher",
    "libexec/gui_shell_linux_broker",
    "lib/libflutter_linux_gtk.so",
)
DESKTOP_CONTENT = (
    "[Desktop Entry]\n"
    "Type=Application\n"
    "Name=D4 Pocket Setup\n"
    "Comment=Install or open D4 Pocket\n"
    "Exec=/opt/d4-pocket/setup/gui_shell_desktop_launcher\n"
    "TryExec=/opt/d4-pocket/setup/gui_shell_desktop_launcher\n"
    "Terminal=false\n"
    "Categories=Utility;Development;\n"
    "DBusActivatable=false\n"
).encode("utf-8")


def _safe_environment() -> dict[str, str]:
    return {
        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
        "LANG": "C.UTF-8",
        "LC_ALL": "C.UTF-8",
        "TZ": "UTC",
        "SOURCE_DATE_EPOCH": "0",
    }


def _run(command: list[str], *, cwd: Path, timeout: int = 120) -> subprocess.CompletedProcess[str]:
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            env=_safe_environment(),
            capture_output=True,
            check=False,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise ValueError(f"Linux package toolを安全に実行できない: {Path(command[0]).name}") from error
    return result


def _require_supported_target() -> None:
    if not sys.platform.startswith("linux") or not hasattr(os, "O_NOFOLLOW"):
        raise ValueError("Debian Setup packageはLinux host上でのみ作成できる")
    if platform.machine().casefold() not in {"x86_64", "amd64"}:
        raise ValueError("Debian Setup packageのbuild architectureはx86_64だけを許可する")
    try:
        release_lines = Path("/etc/os-release").read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise ValueError("Linux distributionを確認できない") from error
    release: dict[str, str] = {}
    for line in release_lines:
        key, separator, value = line.partition("=")
        if separator:
            release[key] = value.strip().strip('"').strip("'")
    if release.get("ID") != "ubuntu" or release.get("VERSION_ID") != "24.04":
        raise ValueError("Debian Setup packageの受入れtargetはUbuntu 24.04だけである")
    architecture = _run(["dpkg", "--print-architecture"], cwd=ROOT)
    if architecture.returncode != 0 or architecture.stdout.strip() != "amd64":
        raise ValueError("dpkg architectureがamd64ではない")
    for tool in ("dpkg-deb", "dpkg-shlibdeps"):
        result = _run([tool, "--version"], cwd=ROOT)
        if result.returncode != 0:
            raise ValueError(f"必要なbuild toolを利用できない: {tool}")


def _parse_dependencies(output: str) -> str:
    output_lines = [line for line in output.splitlines() if line.strip()]
    if len(output_lines) != 1 or not output_lines[0].startswith("shlibs:Depends="):
        raise ValueError("dpkg-shlibdeps出力に予期しない行がある")
    values = [
        line.removeprefix("shlibs:Depends=").strip()
        for line in output_lines
        if line.startswith("shlibs:Depends=")
    ]
    if len(values) != 1 or not values[0] or "\r" in values[0] or "\n" in values[0]:
        raise ValueError("dpkg-shlibdepsが一意なruntime Dependsを返さなかった")
    groups = [group.strip() for group in values[0].split(",")]
    if not groups or any(not group for group in groups):
        raise ValueError("runtime Dependsに空groupがある")
    for group in groups:
        alternatives = [item.strip() for item in group.split("|")]
        if not alternatives or any(not DEPENDENCY_ALTERNATIVE.fullmatch(item) for item in alternatives):
            raise ValueError("runtime Dependsに不正なpackage expressionがある")
    normalized = ", ".join(groups)
    if len(normalized) > 32_000:
        raise ValueError("runtime Dependsが上限を超える")
    return normalized


def _control_text(package_version: str, dependencies: str, installed_kib: int) -> str:
    fields = [
        "Package: d4-pocket-setup",
        f"Version: {package_version}",
        "Section: utils",
        "Priority: optional",
        "Architecture: amd64",
        "Maintainer: D4 Pocket Development <packages@example.invalid>",
        f"Installed-Size: {installed_kib}",
    ]
    pieces = dependencies.split(", ")
    lines: list[str] = []
    current = "Depends: "
    for index, piece in enumerate(pieces):
        suffix = piece if index == 0 else ", " + piece
        if len(current) + len(suffix) > 78 and current != "Depends: ":
            lines.append(current + ",")
            current = " " + piece
        else:
            current += suffix
    lines.append(current)
    fields.extend(lines)
    fields.extend(
        [
            "Description: D4 Pocket first-run setup entry",
            " This package supplies a bootstrap UI for the Broker-controlled per-user install.",
            " Product install, update, rollback, and uninstall remain Owner-confirmed Broker actions.",
            " Removing this package removes only package-owned files.",
        ]
    )
    value = "\n".join(fields) + "\n"
    if len(value.encode("utf-8")) > MAX_CONTROL_BYTES:
        raise ValueError("Debian control metadataが上限を超える")
    return value


def _copy_regular_file(
    source: Path,
    destination: Path,
    expected_size: int,
    expected_hash: str,
    package_mode: int,
    relative: str,
) -> None:
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | os.O_NOFOLLOW
    source_fd = os.open(source, flags)
    try:
        before = os.fstat(source_fd)
        source_mode = stat.S_IMODE(before.st_mode)
        if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1 or before.st_size != expected_size:
            raise ValueError(f"Linux bundle inputが通常の単独fileではない: {relative}")
        if (
            source_mode & 0o7022
            or not source_mode & 0o400
            or (package_mode == 0o755 and not source_mode & 0o100)
        ):
            raise ValueError(f"Linux bundle input permissionが安全でない: {relative}")
        destination.parent.mkdir(mode=0o755, parents=True, exist_ok=True)
        output_fd = os.open(
            destination,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_CLOEXEC", 0) | os.O_NOFOLLOW,
            package_mode,
        )
        digest = hashlib.sha256()
        count = 0
        try:
            os.fchmod(output_fd, package_mode)
            while True:
                chunk = os.read(source_fd, 1024 * 1024)
                if not chunk:
                    break
                digest.update(chunk)
                count += len(chunk)
                view = memoryview(chunk)
                while view:
                    written = os.write(output_fd, view)
                    if written <= 0:
                        raise OSError("Debian staging file write did not progress")
                    view = view[written:]
            os.fsync(output_fd)
        finally:
            os.close(output_fd)
        after = os.fstat(source_fd)
        if (
            count != expected_size
            or digest.hexdigest() != expected_hash
            or before.st_dev != after.st_dev
            or before.st_ino != after.st_ino
            or before.st_size != after.st_size
            or stat.S_IMODE(before.st_mode) != stat.S_IMODE(after.st_mode)
            or before.st_mtime_ns != after.st_mtime_ns
            or before.st_ctime_ns != after.st_ctime_ns
            or before.st_nlink != after.st_nlink
        ):
            raise ValueError("Linux bundle file changed while creating Debian package")
    finally:
        os.close(source_fd)


def _collect_dependencies(stage: Path, bundle_stage: Path) -> tuple[str, str]:
    control_root = stage / "debian"
    control_root.mkdir(mode=0o700)
    control = (
        "Source: d4-pocket-setup\n"
        "Section: utils\n"
        "Priority: optional\n"
        "Maintainer: D4 Pocket Development <packages@example.invalid>\n\n"
        "Package: d4-pocket-setup\n"
        "Architecture: amd64\n"
        "Depends: libc6\n"
        "Description: temporary shlibdeps input\n"
        " temporary build metadata\n"
    )
    (control_root / "control").write_text(control, encoding="utf-8", newline="\n")
    command = [
        "dpkg-shlibdeps",
        "-O",
        "-dDepends",
        f"-S{bundle_stage.parents[2]}",
        f"-l{bundle_stage / 'lib'}",
    ]
    command.extend(f"-e{bundle_stage / item}" for item in EXECUTABLES)
    result = _run(command, cwd=stage)
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).strip()[:1000]
        raise ValueError("runtime dependencyをbundleから解決できない: " + detail)
    dependencies = _parse_dependencies(result.stdout)
    warnings = "\n".join(line for line in result.stderr.splitlines() if line.strip())
    return dependencies, warnings


def _safe_output_directory(path: Path, bundle: Path) -> Path:
    candidate = path.expanduser()
    if not candidate.is_absolute():
        candidate = Path.cwd() / candidate
    if candidate.exists() or candidate.is_symlink():
        metadata = candidate.lstat()
        if not stat.S_ISDIR(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode):
            raise ValueError("Debian package output rootが通常directoryではない")
        output = candidate.resolve(strict=True)
    else:
        output = candidate.parent.resolve(strict=True) / candidate.name
    resolved_bundle = bundle.resolve(strict=True)
    if output == resolved_bundle or output.is_relative_to(resolved_bundle):
        raise ValueError("Debian package outputをsource bundle内へ作成できない")
    if output == ROOT.resolve() or output.is_relative_to(ROOT.resolve()):
        raise ValueError("Debian package outputをRepository内へ作成できない")
    if any(part.casefold() == "onedrive" or part.casefold().startswith("onedrive -") for part in output.parts):
        raise ValueError("Debian package outputをOneDrive内へ作成できない")
    if not candidate.exists():
        output.mkdir(mode=0o700)
    return output


def _require_source_checkout(source_commit: str) -> None:
    checkout = _run(["git", "rev-parse", "HEAD"], cwd=ROOT)
    dirty = _run(["git", "status", "--porcelain=v1", "--untracked-files=all"], cwd=ROOT)
    if (
        checkout.returncode != 0
        or checkout.stdout.strip() != source_commit
        or dirty.returncode != 0
        or dirty.stdout.strip()
    ):
        raise ValueError("Export evidence source commitとclean checkoutが一致しない")


def _load_export(evidence_path: Path, bundle_path: Path) -> tuple[dict[str, Any], Path, bytes, list[dict[str, Any]], int, str]:
    evidence_candidate = evidence_path.expanduser()
    evidence_metadata = evidence_candidate.lstat()
    if not stat.S_ISREG(evidence_metadata.st_mode) or stat.S_ISLNK(evidence_metadata.st_mode):
        raise ValueError("Linux Export evidenceが通常fileではない")
    evidence_path = evidence_candidate.resolve(strict=True)
    if evidence_path.name != "d4_pocket_linux_export_build.json":
        raise ValueError("Linux Export evidence filenameが正本名と一致しない")
    evidence_raw = _read_regular_file(evidence_path, MAX_EVIDENCE_BYTES, "Linux Export evidence")
    try:
        evidence = json.loads(evidence_raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("Linux Export evidenceがUTF-8 JSONではない") from error
    if not isinstance(evidence, dict) or evidence.get("evidence_type") != "d4_pocket_linux_export_build":
        raise ValueError("Linux Export evidence typeが一致しない")
    build_root = evidence_path.parent
    bundle_candidate = bundle_path.expanduser()
    bundle_metadata = bundle_candidate.lstat()
    if not stat.S_ISDIR(bundle_metadata.st_mode) or stat.S_ISLNK(bundle_metadata.st_mode):
        raise ValueError("Linux Export bundle rootが通常directoryではない")
    bundle = bundle_candidate.resolve(strict=True)
    if bundle != (build_root / "bundle").resolve(strict=True):
        raise ValueError("bundleとbuild evidenceのrootが一致しない")
    package_record = evidence.get("package")
    if not isinstance(package_record, dict):
        raise ValueError("Linux D4PKG01 evidenceがない")
    package_name = package_record.get("file_name")
    if not isinstance(package_name, str) or Path(package_name).name != package_name:
        raise ValueError("Linux D4PKG01 filenameが固定されていない")
    product_package = build_root / package_name
    product_manifest = _read_regular_file(bundle / "product_manifest.json", MAX_MANIFEST_BYTES, "Linux Product Manifest")
    archive_record = evidence.get("portable_archive")
    archive_path: Path | None = None
    readme: bytes | None = None
    if archive_record is not None:
        if not isinstance(archive_record, dict):
            raise ValueError("portable archive evidenceが不正である")
        archive_name = archive_record.get("file_name")
        if not isinstance(archive_name, str) or Path(archive_name).name != archive_name:
            raise ValueError("portable archive filenameが固定されていない")
        archive_path = build_root / archive_name
        readme = _read_regular_file(ROOT / "installer/linux/PORTABLE_README_ja.txt", 128 * 1024, "Linux portable README")
    validate_build_evidence(
        evidence,
        bundle,
        product_package,
        product_manifest,
        archive_path,
        readme,
    )
    records, total_bytes, tree_hash = _artifact_inventory(bundle)
    return evidence, bundle, product_manifest, records, total_bytes, tree_hash


def write_debian_setup_package(
    source_dir: Path,
    build_evidence_path: Path,
    output_dir: Path,
    *,
    package_revision: int,
) -> dict[str, Any]:
    """検証済みLinux bundleをUbuntu用のOwner非権限bootstrap .debへ梱包する。"""
    _require_supported_target()
    if type(package_revision) is not int or package_revision < 1 or package_revision > 999_999:
        raise ValueError("Debian package revisionは1以上の整数でなければならない")
    evidence, bundle, _manifest_raw, records, total_bytes, tree_hash = _load_export(
        build_evidence_path, source_dir
    )
    source_commit = evidence.get("source_commit")
    if not isinstance(source_commit, str) or not re.fullmatch(r"[a-f0-9]{40}", source_commit):
        raise ValueError("Linux Export source commitが固定されていない")
    _require_source_checkout(source_commit)
    product_version = product_version_from_source(ROOT)
    if not VERSION_COMPONENT.fullmatch(product_version):
        raise ValueError("Product versionをDebian package versionへ安全に写せない")
    package_version = f"{product_version}-{package_revision}"
    output_root = _safe_output_directory(output_dir, bundle)
    output_path = output_root / f"{PACKAGE_NAME}_{package_version}_amd64.deb"
    if output_path.exists() or output_path.is_symlink():
        raise ValueError("Debian package output already exists")

    with tempfile.TemporaryDirectory(prefix="d4-pocket-deb-build-", dir=output_root) as temporary:
        temporary_root = Path(temporary)
        # dpkg-shlibdeps owns temporary_root/debian for its control metadata.
        # Keep the actual package tree separate so both staging contracts can coexist.
        package_root = temporary_root / "package-root" / PACKAGE_NAME
        payload_root = package_root.joinpath(*INSTALL_ROOT.parts)
        payload_root.mkdir(mode=0o755, parents=True)
        os.chmod(package_root, 0o755)
        os.chmod(payload_root, 0o755)
        for record in records:
            relative = record["path"]
            relative_path = PurePosixPath(relative)
            if relative_path.is_absolute() or any(part in {"", ".", ".."} for part in relative_path.parts):
                raise ValueError("Linux bundle pathがpackage rootから逸脱する")
            package_mode = 0o755 if relative in EXECUTABLE_PATHS else 0o644
            _copy_regular_file(
                bundle / relative,
                payload_root.joinpath(*relative_path.parts),
                record["size_bytes"],
                record["sha256"],
                package_mode,
                relative,
            )
        desktop_path = package_root.joinpath(*DESKTOP_ENTRY.parts)
        desktop_path.parent.mkdir(mode=0o755, parents=True, exist_ok=True)
        os.chmod(desktop_path.parent, 0o755)
        desktop_path.write_bytes(DESKTOP_CONTENT)
        os.chmod(desktop_path, 0o644)

        staged_records, staged_bytes, staged_tree = _artifact_inventory(payload_root)
        if staged_records != records or staged_bytes != total_bytes or staged_tree != tree_hash:
            raise ValueError("Debian stageと検証済みLinux bundleが一致しない")

        dependencies, dependency_warnings = _collect_dependencies(temporary_root, payload_root)
        package_debian_version = _run(["dpkg", "--compare-versions", package_version, "gt", "0"], cwd=temporary_root)
        if package_debian_version.returncode != 0:
            raise ValueError("dpkgがDebian package versionを解釈できない")

        control_dir = package_root / "DEBIAN"
        control_dir.mkdir(mode=0o755)
        installed_bytes = total_bytes + len(DESKTOP_CONTENT)
        control = _control_text(package_version, dependencies, (installed_bytes + 1023) // 1024)
        (control_dir / "control").write_text(control, encoding="utf-8", newline="\n")
        os.chmod(control_dir, 0o755)
        os.chmod(control_dir / "control", 0o644)

        for item in package_root.rglob("*"):
            if item.is_dir():
                os.chmod(item, 0o755)
        os.chmod(package_root, 0o755)

        for item in sorted(package_root.rglob("*"), key=lambda value: len(value.parts), reverse=True):
            os.utime(item, (0, 0), follow_symlinks=False)
        os.utime(package_root, (0, 0), follow_symlinks=False)
        staged_package = temporary_root / output_path.name
        build = _run(
            ["dpkg-deb", "--build", "--root-owner-group", str(package_root), str(staged_package)],
            cwd=temporary_root,
            timeout=300,
        )
        if build.returncode != 0 or not staged_package.is_file():
            detail = (build.stderr or build.stdout).strip()[:1000]
            raise ValueError("dpkg-debがSetup packageを作成できない: " + detail)
        os.chmod(staged_package, 0o644)

        fields: dict[str, str] = {}
        for field in ("Package", "Version", "Architecture", "Depends"):
            result = _run(["dpkg-deb", "--field", str(staged_package), field], cwd=temporary_root)
            if result.returncode != 0:
                raise ValueError(f"生成packageのcontrol fieldを読めない: {field}")
            fields[field] = result.stdout.strip()
        normalized_dependencies = re.sub(r"\n[ \t]+", " ", fields["Depends"]).strip()
        if (
            fields["Package"] != PACKAGE_NAME
            or fields["Version"] != package_version
            or fields["Architecture"] != "amd64"
            or _parse_dependencies("shlibs:Depends=" + normalized_dependencies) != dependencies
        ):
            raise ValueError("生成packageのmetadataが期待値と一致しない")

        listing = _run(["dpkg-deb", "--contents", str(staged_package)], cwd=temporary_root)
        if listing.returncode != 0 or any("root/root" not in line for line in listing.stdout.splitlines() if line.strip()):
            raise ValueError("生成packageのfile ownershipがroot/rootではない")
        extracted = temporary_root / "verify-extract"
        extracted.mkdir(mode=0o700)
        extract = _run(["dpkg-deb", "--extract", str(staged_package), str(extracted)], cwd=temporary_root)
        if extract.returncode != 0:
            raise ValueError("生成packageを独立展開して検証できない")
        extracted_bundle = extracted.joinpath(*INSTALL_ROOT.parts)
        extracted_records, extracted_bytes, extracted_tree = _artifact_inventory(extracted_bundle)
        if extracted_records != records or extracted_bytes != total_bytes or extracted_tree != tree_hash:
            raise ValueError("生成packageのpayloadが検証済みbundleと一致しない")
        extracted_desktop = extracted.joinpath(*DESKTOP_ENTRY.parts)
        if _read_regular_file(extracted_desktop, 4096, "D4 Pocket Setup desktop entry") != DESKTOP_CONTENT:
            raise ValueError("生成packageのdesktop entryが固定内容ではない")
        allowed_files = {
            extracted_bundle.joinpath(*PurePosixPath(record["path"]).parts)
            for record in records
        } | {extracted_desktop}
        allowed_dirs = {
            extracted / "opt",
            extracted / "opt/d4-pocket",
            extracted / "usr",
            extracted / "usr/share",
            extracted / "usr/share/applications",
        }
        for file_path in allowed_files:
            parent = file_path.parent
            while parent != extracted:
                allowed_dirs.add(parent)
                parent = parent.parent
        for path in extracted.rglob("*"):
            metadata = path.lstat()
            if stat.S_ISLNK(metadata.st_mode):
                raise ValueError("生成packageにsymlinkがある")
            if stat.S_ISREG(metadata.st_mode):
                if path not in allowed_files:
                    raise ValueError("生成packageに固定root外のfileがある")
            elif stat.S_ISDIR(metadata.st_mode):
                if path not in allowed_dirs:
                    raise ValueError("生成packageに固定root外のdirectoryがある")
            else:
                raise ValueError("生成packageに通常file／directory以外がある")

        control_extract = temporary_root / "verify-control"
        control_extract.mkdir(mode=0o700)
        control_result = _run(["dpkg-deb", "--control", str(staged_package), str(control_extract)], cwd=temporary_root)
        if control_result.returncode != 0:
            raise ValueError("生成packageのcontrol archiveを読めない")
        control_members = {path.name for path in control_extract.iterdir()}
        if control_members - {"control", "md5sums"} or "control" not in control_members:
            raise ValueError("生成packageにmaintainer scriptまたは未許可control fileがある")

        digest = hashlib.sha256()
        with staged_package.open("rb") as package_file:
            for chunk in iter(lambda: package_file.read(1024 * 1024), b""):
                digest.update(chunk)
        package_hash = digest.hexdigest()
        size = staged_package.stat().st_size
        try:
            os.link(staged_package, output_path, follow_symlinks=False)
        except FileExistsError as error:
            raise ValueError("Debian package output already exists") from error
        output_metadata = output_path.lstat()
        if not stat.S_ISREG(output_metadata.st_mode) or stat.S_ISLNK(output_metadata.st_mode):
            output_path.unlink(missing_ok=True)
            raise ValueError("Debian package outputが通常fileではない")

    return {
        "evidence_type": "d4_pocket_linux_debian_setup_package",
        "evidence_source": "INTERNAL_STATE",
        "package_name": PACKAGE_NAME,
        "package_version": package_version,
        "product_version": product_version,
        "target": TARGET,
        "source_commit": source_commit,
        "artifact_tree_sha256": tree_hash,
        "package_sha256": package_hash,
        "package_size_bytes": size,
        "runtime_dependencies": dependencies,
        "dependency_tool_warnings": dependency_warnings,
        "installed_root": "/opt/d4-pocket/setup",
        "desktop_entry": "/usr/share/applications/d4-pocket-setup.desktop",
        "package_manager_scripts": [],
        "scope": "first_run_setup_bootstrap_only",
        "signed": False,
        "owner_authorization_created": False,
        "permission_created": False,
        "approval_created": False,
        "credential_values_included": False,
        "user_xdg_data_modified_by_package": False,
        "output_path": str(output_path),
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Ubuntu 24.04 amd64用D4 Pocket first-run setup .debを作成する開発専用tool。"
    )
    parser.add_argument("--source-dir", type=Path, required=True, help="検証済みLinux Export bundle")
    parser.add_argument("--build-evidence", type=Path, required=True, help="d4_pocket_linux_export_build.json")
    parser.add_argument("--output-dir", type=Path, required=True, help="Repository／OneDrive外の新規package directory")
    parser.add_argument("--package-revision", type=int, required=True, help="同一製品版内での正整数package revision")
    args = parser.parse_args()
    try:
        result = write_debian_setup_package(
            args.source_dir,
            args.build_evidence,
            args.output_dir,
            package_revision=args.package_revision,
        )
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"D4 Pocket Debian Setup package作成を停止した: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
