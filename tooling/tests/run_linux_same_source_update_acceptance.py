#!/usr/bin/env python3
"""Run the installed Linux product update path against a same-source D4PKG."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SERVER_SCRIPT = ROOT / "tooling/tests/linux_update_https_server.py"
SMOKE_SCRIPT = ROOT / "tooling/tests/linux_installed_product_smoke.py"
CA_ENV = "GUI_SHELL_R2_E2E_UPDATE_CA_PEM"
ACCEPTANCE_SCOPE = "debian_setup_lifecycle"


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _read_d4pkg(path: Path, expected_version: str) -> dict[str, object]:
    with path.open("rb") as package:
        header = package.read(12)
        if len(header) != 12 or header[:8] != b"D4PKG01\n":
            raise ValueError("update candidateがD4PKG01ではありません")
        manifest_length = int.from_bytes(header[8:12], "little")
        if not 1 <= manifest_length <= 4 * 1024 * 1024:
            raise ValueError("update candidate manifest長が不正です")
        manifest_bytes = package.read(manifest_length)
    if len(manifest_bytes) != manifest_length:
        raise ValueError("update candidate manifestが途中で終了しています")
    value = json.loads(manifest_bytes.decode("utf-8"))
    if (
        value.get("product") != "D4 Pocket"
        or value.get("target_platform") != "linux"
        or value.get("product_version") != expected_version
        or not isinstance(value.get("files"), list)
        or not value["files"]
    ):
        raise ValueError("update candidateはLinux D4 Pocket product packageではありません")
    return value


def _server_process_matches(pid: int, update_root: Path) -> bool:
    try:
        command = [
            argument
            for argument in Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
            if argument
        ]
        owner = Path(f"/proc/{pid}").stat().st_uid
    except OSError:
        return False
    return (
        owner == os.getuid()
        and any(part.endswith(b"linux_update_https_server.py") for part in command)
        and any(
            command[index] == b"--root"
            and command[index + 1] == os.fsencode(str(update_root))
            for index in range(len(command) - 1)
        )
    )


def _stop_server(
    server: subprocess.Popen[bytes], pid_file: Path, update_root: Path
) -> None:
    try:
        pid = int(pid_file.read_text(encoding="ascii").strip())
    except (OSError, ValueError):
        pid = None
    if pid is not None and _server_process_matches(pid, update_root):
        os.kill(pid, signal.SIGTERM)
    try:
        server.wait(timeout=8)
    except subprocess.TimeoutExpired:
        if pid is not None and _server_process_matches(pid, update_root):
            os.kill(pid, signal.SIGKILL)
        server.wait(timeout=5)
    if pid is not None and _server_process_matches(pid, update_root):
        raise RuntimeError("run-owned Update HTTPS server processが残っています")


def _tail(path: Path, limit: int = 16000) -> str:
    try:
        with path.open("rb") as source:
            source.seek(0, os.SEEK_END)
            source.seek(max(0, source.tell() - limit))
            return source.read(limit).decode("utf-8", errors="replace")
    except OSError:
        return ""


def _run(args: argparse.Namespace) -> dict[str, object]:
    if args.temp_root.is_symlink() or not args.temp_root.is_dir():
        raise ValueError("acceptance temp rootは通常directoryではありません")
    root = args.temp_root.resolve(strict=True)
    update_root = root / "update-server"
    input_paths = (
        args.candidate_package,
        args.catalog,
        args.initial_package,
        args.build_evidence,
        args.candidate_evidence,
        args.update_signing_key,
    )
    if any(path.is_symlink() or not path.is_file() for path in input_paths):
        raise ValueError("same-source acceptance input contains a non-regular file")
    candidate, catalog, initial, evidence_path, candidate_evidence_path, signing_key = (
        path.resolve(strict=True) for path in input_paths
    )
    build_evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
    candidate_evidence = json.loads(candidate_evidence_path.read_text(encoding="utf-8"))
    if args.acceptance_root.is_symlink():
        raise ValueError("acceptance root cannot be a symlink")
    acceptance_root = args.acceptance_root.resolve(strict=False)
    if (
        root.is_relative_to(ROOT)
        or update_root.is_symlink()
        or not update_root.is_dir()
        or root not in candidate.parents
        or root not in catalog.parents
        or root not in initial.parents
        or root not in evidence_path.parents
        or root not in candidate_evidence_path.parents
        or root not in signing_key.parents
        or root not in acceptance_root.parents
        or not SERVER_SCRIPT.is_file()
        or not SMOKE_SCRIPT.is_file()
    ):
        raise ValueError("same-source acceptance input is outside the run-owned temporary root")
    if (
        build_evidence.get("source_commit") != args.source_commit
        or candidate_evidence.get("source_commit") != args.source_commit
        or build_evidence.get("artifact_tree_sha256")
        != candidate_evidence.get("bundle_artifact_tree_sha256")
        or candidate_evidence.get("payload_is_product_d4pkg") is not True
        or candidate_evidence.get("product_version") != "0.1.2"
        or candidate_evidence.get("package_sha256") != _sha256(candidate)
        or candidate_evidence.get("package_size_bytes") != candidate.stat().st_size
    ):
        raise ValueError("release bundle・candidate・run source commitのprovenanceが一致しません")
    candidate_manifest = _read_d4pkg(candidate, "0.1.2")
    initial_manifest = _read_d4pkg(initial, "0.1.1")
    if (
        candidate_manifest.get("app_id") != initial_manifest.get("app_id")
        or candidate_manifest.get("audit_store_id") != initial_manifest.get("audit_store_id")
    ):
        raise ValueError("初回導入packageと同一source updateのApp／Audit identityが異なります")
    catalog_document = json.loads(catalog.read_text(encoding="utf-8"))
    candidates = catalog_document.get("候補")
    if not isinstance(candidates, list) or len(candidates) != 1:
        raise ValueError("acceptance catalogは単一の候補だけを持つ必要があります")
    signed_bytes = bytes.fromhex(candidates[0]["署名対象"])
    signed_candidate = json.loads(signed_bytes)
    if (
        signed_candidate.get("package_sha256") != _sha256(candidate)
        or signed_candidate.get("package_size_bytes") != candidate.stat().st_size
        or signed_candidate.get("提供版") != "0.1.2"
        or hashlib.sha256(catalog.read_bytes()).hexdigest()
        != candidate_evidence.get("catalog_sha256")
    ):
        raise ValueError("署名catalogと同一source update packageのhash／sizeが一致しません")

    server_log = root / "update-server.log"
    smoke_log = root / "debian-setup-acceptance-output.log"
    pid_file = update_root / "server.pid"
    result_file = root / "same-source-update-acceptance.json"
    environment = os.environ.copy()
    environment[CA_ENV] = str((update_root / "ca.pem").resolve(strict=True))
    server_log_handle = server_log.open("wb")
    server: subprocess.Popen[bytes] | None = None
    try:
        server = subprocess.Popen(
            [
                "sudo", "-n", sys.executable, str(SERVER_SCRIPT),
                "--root", str(update_root),
                "--certificate", str(update_root / "server.pem"),
                "--private-key", str(update_root / "server.key"),
                "--pid-file", str(pid_file),
                "--drop-uid", str(os.getuid()),
                "--drop-gid", str(os.getgid()),
            ],
            stdout=server_log_handle,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        ready = False
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if server.poll() is not None:
                break
            if "READY 443" in _tail(server_log, 4000):
                ready = True
                break
            time.sleep(0.2)
        if not ready:
            raise RuntimeError("run-owned TLS update server did not become ready: " + _tail(server_log, 4000))
        subprocess.run(
            [
                "curl", "--fail", "--silent", "--show-error",
                "--cacert", str(update_root / "ca.pem"),
                "--resolve", "updates.d4p-test.example:443:127.0.0.1",
                "https://updates.d4p-test.example/d4/stable/updates.json",
                "--output", "/dev/null",
            ],
            check=True,
            timeout=20,
        )
        with smoke_log.open("wb") as smoke_log_handle:
            smoke = subprocess.run(
                [
                    "dbus-run-session", "--", "xvfb-run", "-a", sys.executable,
                    str(SMOKE_SCRIPT),
                    "--package", str(initial),
                    "--temp-root", str(root),
                    "--acceptance-root", str(args.acceptance_root),
                    "--private-key", str(signing_key),
                    "--update-package", str(initial),
                    "--update-candidate-package", str(candidate),
                    "--update-catalog", str(catalog),
                    "--acceptance-scope", ACCEPTANCE_SCOPE,
                ],
                env=environment,
                stdout=smoke_log_handle,
                stderr=subprocess.STDOUT,
                check=False,
                timeout=30 * 60,
            )
        if smoke.returncode != 0:
            raise RuntimeError(f"installed product acceptance failed with exit={smoke.returncode}")
    finally:
        try:
            if server is not None:
                _stop_server(server, pid_file, update_root)
        finally:
            server_log_handle.close()

    output = _tail(smoke_log, 64000)
    if output:
        print(output, end="" if output.endswith("\n") else "\n")
    result_records = []
    for line in output.splitlines():
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict) and value.get("acceptance_scope") == ACCEPTANCE_SCOPE:
            result_records.append(value)
    if not result_records:
        raise RuntimeError("installed product acceptance JSON resultがありません")
    result = result_records[-1]
    update = result.get("update_lifecycle")
    first = result.get("first_install")
    if (
        not isinstance(update, dict)
        or not isinstance(first, dict)
        or first.get("product_version") != "0.1.1"
        or first.get("package_sha256") != _sha256(initial)
        or update.get("active_record", {}).get("product_version") != "0.1.2"
        or update.get("package_sha256") != candidate_evidence["package_sha256"]
        or update.get("package_size_bytes") != candidate_evidence["package_size_bytes"]
        or update.get("payload_is_product_d4pkg") is not True
        or result.get("active_version") != "0.1.1"
        or not isinstance(result.get("uninstall"), dict)
        or not isinstance(result.get("live_audit"), dict)
        or update.get("download_rejected_then_owner_approved") is not True
        or update.get("stage_rejected_then_owner_approved") is not True
        or update.get("activation_rejected_then_owner_approved") is not True
        or result.get("runtime_audit") != "durable_after_broker_restart"
    ):
        raise RuntimeError("same-source installed update acceptance result did not meet its finite contract")
    acceptance = {
        "acceptance_scope": ACCEPTANCE_SCOPE,
        "source_commit": args.source_commit,
        "bundle_artifact_tree_sha256": candidate_evidence["bundle_artifact_tree_sha256"],
        "initial_package_sha256": _sha256(initial),
        "update_package_sha256": candidate_evidence["package_sha256"],
        "update_package_size_bytes": candidate_evidence["package_size_bytes"],
        "update_catalog_sha256": candidate_evidence["catalog_sha256"],
        "payload_is_product_d4pkg": True,
        "installed_ui_broker_https_signature_owner_stage_activate_rollback": True,
        "active_version_after_rollback": "0.1.1",
        "uninstall_and_live_audit": True,
        "tls_and_update_signing_identity": "run-scoped published test identities",
        "formal_distribution_claimed": False,
    }
    result_file.write_text(json.dumps(acceptance, ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8")
    return acceptance


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--temp-root", type=Path, required=True)
    parser.add_argument("--acceptance-root", type=Path, required=True)
    parser.add_argument("--initial-package", type=Path, required=True)
    parser.add_argument("--candidate-package", type=Path, required=True)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--update-signing-key", type=Path, required=True)
    parser.add_argument("--build-evidence", type=Path, required=True)
    parser.add_argument("--candidate-evidence", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    args = parser.parse_args()
    try:
        result = _run(args)
    except Exception as error:
        temp_root = args.temp_root
        for name in ("update-server.log", "debian-setup-acceptance-output.log"):
            tail = _tail(temp_root / name, 12000)
            if tail:
                print(f"--- {name} (tail) ---\n{tail}", file=sys.stderr)
        print(f"same-source installed update acceptance failed: {type(error).__name__}: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
