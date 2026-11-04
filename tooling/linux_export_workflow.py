from __future__ import annotations

import sys
from pathlib import Path

def verify_build_result() -> None:
    import hashlib, json, pathlib, sys
    root = pathlib.Path(sys.argv[1])
    evidence = json.loads((root / "d4_pocket_linux_export_build.json").read_text())
    package = root / "D4Pocket-linux.d4pkg"
    expected_test_build = sys.argv[3] in {"agent_task_supervision", "agent_task_crash_recovery", "provider_credential_binding"}
    assert evidence["target_platform"] == "linux"
    assert evidence["source_commit"] == sys.argv[2]
    assert evidence["signed"] is False and evidence["formal_distribution_claimed"] is False
    assert evidence["package"]["installed_product_verified"] is False
    assert evidence["package"]["sha256"] == hashlib.sha256(package.read_bytes()).hexdigest()
    if sys.argv[3] == "portable_archive":
        archive = evidence["portable_archive"]
        assert archive["target"] == "ubuntu-24.04-x86_64"
        assert archive["signed"] is False
        archive_path = root / archive["file_name"]
        assert archive["byte_length"] == archive_path.stat().st_size
        assert archive["sha256"] == hashlib.sha256(archive_path.read_bytes()).hexdigest()
    assert evidence["cmake_configure_arguments"].count(
        "-DD4_LINUX_R2_E2E_TEST_BUILD=ON" if expected_test_build
        else "-DD4_LINUX_R2_E2E_TEST_BUILD=OFF"
    ) == 1
    assert evidence["cmake_configure_arguments"].count(
        "-DD4_LINUX_R2_E2E_TEST_BUILD=ON" if not expected_test_build
        else "-DD4_LINUX_R2_E2E_TEST_BUILD=OFF"
    ) == 0
    print(json.dumps({"source_commit": evidence["source_commit"], "artifact_tree_sha256": evidence["artifact_tree_sha256"], "package_sha256": evidence["package"]["sha256"], "r2_e2e_test_build": expected_test_build, "signed": False, "installed_product_verified": False}, separators=(",", ":")))

def verify_portable_archive() -> None:
    import hashlib, json, pathlib, stat, sys
    root = pathlib.Path(sys.argv[1]).resolve()
    evidence = json.loads((root / "d4_pocket_linux_export_build.json").read_text())
    record = evidence["portable_archive"]
    archive = root / sys.argv[2]
    assert record["target"] == "ubuntu-24.04-x86_64" and record["format"] == "tar.gz"
    assert record["file_name"] == archive.name and archive.parent.resolve() == root
    assert stat.S_ISREG(archive.lstat().st_mode)
    assert archive.stat().st_size == record["byte_length"]
    assert hashlib.sha256(archive.read_bytes()).hexdigest() == record["sha256"]

OPERATIONS = {
    'verify-build-result': verify_build_result,
    'verify-portable-archive': verify_portable_archive,
}


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] not in OPERATIONS:
        allowed = ", ".join(sorted(OPERATIONS))
        raise SystemExit(f"使い方: `{Path(__file__).name}` <{allowed}> [引数]")
    operation = OPERATIONS[sys.argv[1]]
    sys.argv = [sys.argv[0], *sys.argv[2:]]
    operation()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
