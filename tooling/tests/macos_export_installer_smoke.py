from __future__ import annotations

import json
import hashlib
import os
import pathlib
import plistlib
import re
import shutil
import signal
import stat
import subprocess
import sys
import time
from typing import Any

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from tooling.tests.macos_installed_launch_smoke import verify_installed_launch


def validate_smoke_record(record: dict[str, Any]) -> dict[str, str]:
    app_id = record.get("app_id", "")
    audit_id = record.get("audit_store_id", "")
    if re.fullmatch(r"d4-pocket-app-[0-9a-f]{32}", app_id) is None:
        raise ValueError("macOS installer smoke app_idがfixture形式ではない")
    if re.fullmatch(r"audit-store-[0-9a-f]{32}", audit_id) is None:
        raise ValueError("macOS installer smoke audit_store_idがfixture形式ではない")
    suffix = app_id.removeprefix("d4-pocket-app-")[:12]
    bundle_name = f"D4Pocket-{suffix}.app"
    bundle_id = f"com.d4p.export.{app_id.removeprefix('d4-pocket-app-')}"
    package_name = f"D4Pocket-{suffix}.pkg"
    uninstaller_name = f"D4Pocket-{suffix}-Uninstaller.pkg"
    if (
        record.get("bundle_name") != bundle_name
        or record.get("bundle_identifier") != bundle_id
        or record.get("installer_identifier") != f"{bundle_id}.installer"
        or record.get("formal_distribution_claimed") is not False
        or record.get("installed_product_verified") is not False
    ):
        raise ValueError("macOS installer smokeのapp identity／release claimが不正")
    package = record.get("package", {})
    uninstaller = record.get("uninstaller_package", {})
    if (
        package.get("file_name") != package_name
        or package.get("signed") is not False
        or uninstaller.get("file_name") != uninstaller_name
        or uninstaller.get("identifier") != f"{bundle_id}.uninstaller"
        or uninstaller.get("signed") is not False
        or uninstaller.get("removed_scope") != "matching_app_bundle_only"
        or uninstaller.get("user_data_preserved") is not True
        or uninstaller.get("keychain_preserved") is not True
    ):
        raise ValueError("macOS installer smoke package metadataが不正")
    source_commit = record.get("source_commit", "")
    if re.fullmatch(r"[0-9a-f]{40}", source_commit) is None:
        raise ValueError("macOS installer smoke source commitが不正")
    return {
        "bundle_name": bundle_name,
        "bundle_id": bundle_id,
        "app_id": app_id,
        "audit_id": audit_id,
        "package_name": package_name,
        "uninstaller_name": uninstaller_name,
    }


def _ensure_owned_directory(root: pathlib.Path, home: pathlib.Path) -> None:
    if root.is_symlink() or not root.is_dir():
        raise RuntimeError("installer smoke cleanup対象directoryが不正")
    resolved_home = home.resolve(strict=True)
    resolved_root = root.resolve(strict=True)
    if not resolved_root.is_relative_to(resolved_home):
        raise RuntimeError("installer smoke cleanup対象がuser home外へ解決された")


def _require_protected_install_path(path: pathlib.Path, *, directory: bool) -> None:
    metadata = path.lstat()
    type_matches = stat.S_ISDIR(metadata.st_mode) if directory else stat.S_ISREG(metadata.st_mode)
    if not type_matches or (metadata.st_uid, metadata.st_gid) != (0, 0) or metadata.st_mode & 0o022:
        raise RuntimeError(f"macOS installed App Bundleの所有者・modeまたは種別が不正: {path.name}")


def _verify_installed_bundle(app_path: pathlib.Path, bundle_id: str) -> None:
    contents = app_path / "Contents"
    macos = contents / "MacOS"
    for path in (app_path, contents, macos):
        _require_protected_install_path(path, directory=True)
    info_path = contents / "Info.plist"
    _require_protected_install_path(info_path, directory=False)
    with info_path.open("rb") as stream:
        installed_info = plistlib.load(stream)
    if installed_info.get("CFBundleIdentifier") != bundle_id:
        raise RuntimeError("native Installer後のBundle IDが一致しない")
    executable = installed_info.get("CFBundleExecutable")
    if not isinstance(executable, str) or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", executable) is None:
        raise RuntimeError("native Installer後のApp実行file名が不正")
    for path in (macos / executable, macos / "gui_shell_macos_broker"):
        _require_protected_install_path(path, directory=False)


def _process_snapshot() -> list[tuple[int, int, str]]:
    result = subprocess.run(
        ["/bin/ps", "-ww", "-axo", "pid=,ppid=,command="],
        check=True,
        capture_output=True,
        text=True,
    )
    processes = []
    for line in result.stdout.splitlines():
        fields = line.strip().split(maxsplit=2)
        if len(fields) != 3:
            continue
        try:
            pid, parent = int(fields[0]), int(fields[1])
        except ValueError:
            continue
        executable = fields[2].split(maxsplit=1)[0]
        processes.append((pid, parent, pathlib.Path(executable).name))
    return processes


def _stop_smoke_processes(bundle_id: str, executable_name: str) -> None:
    names = {executable_name, "gui_shell_macos_broker"}
    if not any(name in names for _pid, _parent, name in _process_snapshot()):
        return
    if any(name == executable_name for _pid, _parent, name in _process_snapshot()):
        try:
            subprocess.run(
                ["/usr/bin/osascript", "-e", f'tell application id "{bundle_id}" to quit'],
                capture_output=True,
                text=True,
                timeout=20,
                check=False,
            )
        except (OSError, subprocess.TimeoutExpired):
            pass
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        if not any(name in names for _pid, _parent, name in _process_snapshot()):
            return
        time.sleep(0.5)
    for pid, _parent, name in _process_snapshot():
        if name in names:
            try:
                os.kill(pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        if not any(name in names for _pid, _parent, name in _process_snapshot()):
            return
        time.sleep(0.5)
    raise RuntimeError("installer smokeのapp/helper processを停止できない")


def run_installer_smoke(
    output_dir: pathlib.Path,
    evidence_dir: pathlib.Path,
    *,
    macos_version: str,
    architecture: str,
) -> dict[str, Any]:
    if output_dir.is_symlink() or not output_dir.is_dir():
        raise ValueError("macOS installer output directoryが通常directoryではない")
    record_path = output_dir / "d4_pocket_macos_export_build.json"
    if record_path.is_symlink() or not record_path.is_file():
        raise ValueError("macOS installer build evidenceが通常fileではない")
    record = json.loads(record_path.read_text(encoding="utf-8"))
    identity = validate_smoke_record(record)
    package_paths = {
        "package": output_dir / identity["package_name"],
        "uninstaller_package": output_dir / identity["uninstaller_name"],
    }
    for field, package_path in package_paths.items():
        if package_path.is_symlink() or not package_path.is_file():
            raise ValueError("macOS installer packageが通常fileではない")
        actual_hash = hashlib.sha256(package_path.read_bytes()).hexdigest()
        if actual_hash != record[field]["sha256"]:
            raise ValueError("macOS installer package hashがbuild evidenceと一致しない")
    home = pathlib.Path.home().resolve(strict=True)
    if evidence_dir.is_symlink() or not evidence_dir.is_dir():
        raise ValueError("macOS installer evidence directoryが通常directoryではない")

    app_path = pathlib.Path("/Applications") / identity["bundle_name"]
    app_data_root = home / "Library/Application Support/D4Pocket/apps" / identity["app_id"]
    state_root = app_data_root / "stores" / identity["audit_id"]
    container_root = home / "Library/Containers" / identity["bundle_id"]
    launch_evidence_path = evidence_dir / "macos-installed-launch.json"
    smoke_evidence_path = evidence_dir / "macos-installer-smoke.json"
    paths = (app_path, app_data_root, container_root, launch_evidence_path, smoke_evidence_path)
    if any(path.exists() or path.is_symlink() for path in paths):
        raise RuntimeError("macOS installer smokeの一意fixture pathが既存")
    for path in (app_data_root, container_root):
        if not path.resolve(strict=False).is_relative_to(home):
            raise RuntimeError("macOS installer smoke data pathがhome外へ解決された")

    marker = state_root / "uninstaller-smoke-marker.txt"
    marker_bytes = b"D4P-MACOS-UNINSTALL-KEEP\n"
    app_install_attempted = False
    user_data_created = False
    try:
        state_root.mkdir(parents=True)
        user_data_created = True
        marker.write_bytes(marker_bytes)
        app_install_attempted = True
        subprocess.run(
            ["/usr/bin/sudo", "/usr/sbin/installer", "-pkg", str(output_dir / identity["package_name"]), "-target", "/"],
            check=True,
        )
        _verify_installed_bundle(app_path, identity["bundle_id"])

        launch = verify_installed_launch(app_path, identity["bundle_id"], launch_evidence_path)
        subprocess.run(
            ["/usr/bin/sudo", "/bin/rm", "-rf", str(app_path)], check=True
        )
        if app_path.exists() or app_path.is_symlink():
            raise RuntimeError("repair前のfixture App Bundleを撤去できない")
        subprocess.run(
            ["/usr/bin/sudo", "/usr/sbin/installer", "-pkg", str(output_dir / identity["package_name"]), "-target", "/"],
            check=True,
        )
        _verify_installed_bundle(app_path, identity["bundle_id"])

        subprocess.run(
            ["/usr/bin/sudo", "/usr/sbin/installer", "-pkg", str(output_dir / identity["uninstaller_name"]), "-target", "/"],
            check=True,
        )
        if app_path.exists() or app_path.is_symlink():
            raise RuntimeError("native Uninstallerがmatching App Bundleを削除しなかった")
        if marker.is_symlink() or not marker.is_file() or marker.read_bytes() != marker_bytes:
            raise RuntimeError("native Uninstallerがuser data markerを保持しなかった")

        result = {
            "version": 1,
            "evidence_type": "d4_pocket_macos_installer_smoke",
            "evidence_source": "LIVE_RUNTIME",
            "source_commit": record["source_commit"],
            "macos_version": macos_version,
            "architecture": architecture,
            "bundle_identifier": identity["bundle_id"],
            "install_package_sha256": record["package"]["sha256"],
            "uninstaller_package_sha256": record["uninstaller_package"]["sha256"],
            "install": {
                "native_installer_completed": True,
                "bundle_identifier_verified": True,
                "protected_bundle_entrypoints_verified": True,
            },
            "installed_launch": launch,
            "repair": {
                "same_package_reapplied_after_app_removal": True,
                "bundle_restored": True,
                "protected_bundle_entrypoints_verified": True,
            },
            "uninstall": {
                "native_installer_completed": True,
                "matching_bundle_removed": True,
                "user_data_marker_preserved": True,
            },
            "publisher_signed": False,
            "launch_verified": all(
                launch.get(field) is True
                for field in (
                    "app_bundle_signature_verified",
                    "helper_signature_verified",
                    "app_sandbox_entitlements_verified",
                    "helper_inherited_sandbox_entitlements_verified",
                    "app_launched",
                    "broker_helper_started_as_app_child",
                    "graceful_quit_request_sent",
                    "app_process_stopped",
                    "broker_helper_process_stopped",
                )
            ),
            "helper_exit_status_observed": launch.get("helper_exit_status_observed") is True,
            "formal_distribution_claimed": False,
        }
        smoke_evidence_path.write_text(
            json.dumps(result, separators=(",", ":")) + "\n", encoding="utf-8"
        )
        return result
    finally:
        executable_name = "gui_shell_desktop"
        if app_install_attempted or any(
            name == executable_name for _pid, _parent, name in _process_snapshot()
        ):
            _stop_smoke_processes(identity["bundle_id"], executable_name)
        if app_path.exists() or app_path.is_symlink():
            if app_path.is_symlink() or not app_path.is_dir():
                raise RuntimeError("installer smoke cleanup App Bundle pathが不正")
            if app_install_attempted:
                subprocess.run(
                    ["/usr/bin/sudo", "/bin/rm", "-rf", str(app_path)], check=True
                )
            if app_path.exists() or app_path.is_symlink():
                raise RuntimeError("installer smoke cleanup App Bundle撤去に失敗")
        if user_data_created:
            for root in (app_data_root, container_root):
                if root.exists() or root.is_symlink():
                    _ensure_owned_directory(root, home)
                    shutil.rmtree(root)
                    if root.exists() or root.is_symlink():
                        raise RuntimeError("installer smoke cleanup data rootが残存")


def main() -> int:
    if len(sys.argv) != 5:
        raise SystemExit(
            "usage: macos_export_installer_smoke.py OUTPUT_DIR EVIDENCE_DIR MACOS_VERSION ARCH"
        )
    result = run_installer_smoke(
        pathlib.Path(sys.argv[1]),
        pathlib.Path(sys.argv[2]),
        macos_version=sys.argv[3],
        architecture=sys.argv[4],
    )
    print(json.dumps(result, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
