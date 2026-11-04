from __future__ import annotations

import sys
from pathlib import Path

def verify_package_pair() -> None:
    import hashlib, json, pathlib, sys
    one_path, two_path = map(pathlib.Path, sys.argv[1:3])
    one, two = json.loads(one_path.read_text()), json.loads(two_path.read_text())
    expected_commit = sys.argv[3]
    manifest = json.loads(pathlib.Path(sys.argv[4]).read_text())["manifest"]
    assert one["evidence_type"] == two["evidence_type"] == "d4_pocket_linux_debian_setup_package"
    assert one["target"] == two["target"] == "ubuntu-24.04-amd64"
    assert one["package_version"].rsplit("-", 1)[0] == two["package_version"].rsplit("-", 1)[0]
    assert one["package_version"].endswith("-1") and two["package_version"].endswith("-2")
    assert one["source_commit"] == two["source_commit"] == expected_commit
    assert one["artifact_tree_sha256"] == two["artifact_tree_sha256"]
    assert one["scope"] == "first_run_setup_bootstrap_only"
    assert one["signed"] is False and one["owner_authorization_created"] is False
    assert one["permission_created"] is False and one["approval_created"] is False
    assert one["credential_values_included"] is False and one["user_xdg_data_modified_by_package"] is False
    assert one["package_manager_scripts"] == [] and two["package_manager_scripts"] == []
    for record in (one, two):
        package = pathlib.Path(record["output_path"])
        assert package.is_file() and package.stat().st_size == record["package_size_bytes"]
        assert hashlib.sha256(package.read_bytes()).hexdigest() == record["package_sha256"]
    print(one["output_path"], two["output_path"], one["source_commit"], one["artifact_tree_sha256"], manifest["app_identity"]["app_id"], manifest["audit_store"]["store_id"])

def verify_first_install_package() -> None:
    import hashlib, json, pathlib, sys
    record = json.loads(pathlib.Path(sys.argv[1]).read_text())
    manifest = json.loads(pathlib.Path(sys.argv[3]).read_text())["manifest"]
    package = pathlib.Path(record["output_path"])
    assert record["evidence_type"] == "d4_pocket_linux_debian_setup_package"
    assert record["target"] == "ubuntu-24.04-amd64"
    assert record["source_commit"] == sys.argv[2]
    assert record["scope"] == "first_run_setup_bootstrap_only"
    assert record["signed"] is False
    assert record["owner_authorization_created"] is False
    assert record["permission_created"] is False and record["approval_created"] is False
    assert record["credential_values_included"] is False
    assert record["package_manager_scripts"] == []
    assert package.is_file() and package.stat().st_size == record["package_size_bytes"]
    digest = hashlib.sha256(package.read_bytes()).hexdigest()
    assert digest == record["package_sha256"]
    print(record["output_path"], record["source_commit"], record["artifact_tree_sha256"], manifest["app_identity"]["app_id"], manifest["audit_store"]["store_id"], digest)

def validate_installed_acceptance() -> None:
    import json, pathlib, sys
    records = []
    for line in pathlib.Path(sys.argv[1]).read_text().splitlines():
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict) and value.get("acceptance_scope") == sys.argv[2]:
            records.append(value)
    assert records, "installed smokeのJSON acceptance resultを取得できません"
    result = records[-1]
    scope = sys.argv[2]
    assert result["acceptance_scope"] == scope
    if scope == "debian_setup_lifecycle":
        first = result["first_install"]
        assert first["product_version"] == "0.1.1"
        assert first["package_sha256"] == sys.argv[3]
        assert result["update_lifecycle"]["active_record"]["product_version"] == "0.1.2"
        assert result["active_version"] == "0.1.1"
        assert result["uninstall"] and result["live_audit"]

OPERATIONS = {
    'verify-package-pair': verify_package_pair,
    'verify-first-install-package': verify_first_install_package,
    'validate-installed-acceptance': validate_installed_acceptance,
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
