#!/usr/bin/env python3
"""Build a same-bundle D4PKG and run-scoped signed HTTPS catalog for Linux acceptance."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.package_linux_product import write_product_package
from tooling.export_linux_product import validate_build_evidence
from tooling.tests.create_linux_update_catalog_fixture import (
    write_product_acceptance_catalog,
)


def _safe_exception_summary(error: Exception) -> str:
    message = re.sub(
        r"(?i)(?:[a-z]:[\\/]|\\\\|/)[^\s\"'<>]+",
        "<path>",
        str(error),
    )
    return message[:500] or type(error).__name__


def _regular_file(path: Path, label: str) -> Path:
    if path.is_symlink() or not path.is_file():
        raise ValueError(f"{label}は通常fileではありません")
    return path.resolve(strict=True)


def _prepare(bundle: Path, output_root: Path, private_key: Path, build_evidence: Path, source_commit: str) -> dict[str, object]:
    if bundle.is_symlink() or not bundle.is_dir():
        raise ValueError("Linux release bundleは通常directoryではありません")
    if output_root.is_symlink() or not output_root.is_dir():
        raise ValueError("run output rootは通常directoryではありません")
    bundle = bundle.resolve(strict=True)
    output_root = output_root.resolve(strict=True)
    private_key = _regular_file(private_key, "RFC 8032 test key")
    build_evidence = _regular_file(build_evidence, "Linux build evidence")
    if (
        output_root.is_relative_to(ROOT)
        or bundle.is_relative_to(ROOT)
        or output_root not in bundle.parents
        or output_root not in private_key.parents
        or output_root not in build_evidence.parents
        or bundle != output_root / "output" / "bundle"
        or build_evidence != output_root / "output" / "d4_pocket_linux_export_build.json"
        or len(source_commit) != 40
        or any(character not in "0123456789abcdef" for character in source_commit)
    ):
        raise ValueError("same-source update packageとbundleはRepository外のrun rootに限ります")
    evidence = json.loads(build_evidence.read_text(encoding="utf-8"))
    if evidence.get("target_platform") != "linux" or evidence.get("source_commit") != source_commit:
        raise ValueError("Linux release bundle evidenceが指定source commitと一致しません")
    validate_build_evidence(
        evidence,
        bundle,
        output_root / "output" / "D4Pocket-linux.d4pkg",
        (bundle / "product_manifest.json").read_bytes(),
    )
    update_server_root = output_root / "update-server"
    if update_server_root.exists() or update_server_root.is_symlink():
        raise ValueError("run-scoped update server rootは新規でなければなりません")
    update_server_root.mkdir(mode=0o700)
    package_path = output_root / "D4Pocket-linux-update.d4pkg"
    package_record = write_product_package(
        bundle, package_path, product_version="0.1.2"
    )
    catalog_record = write_product_acceptance_catalog(
        package_path, update_server_root, private_key
    )
    if (
        package_record["target_platform"] != "linux"
        or package_record["product_version"] != "0.1.2"
        or package_record["installed_product_verified"] is not False
        or package_record["package_sha256"] != catalog_record["package_sha256"]
        or package_record["package_size_bytes"] != catalog_record["package_size_bytes"]
        or catalog_record["payload_is_d4pkg_with_product_manifest"] is not True
    ):
        raise ValueError("同一bundle D4PKGと署名catalogのidentityが一致しません")

    certificate = update_server_root / "server.pem"
    private_tls_key = update_server_root / "server.key"
    subprocess.run(
        [
            "openssl", "req", "-x509", "-newkey", "rsa:2048", "-sha256",
            "-nodes", "-days", "2", "-keyout", str(private_tls_key),
            "-out", str(certificate), "-subj", "/CN=updates.d4p-test.example",
            "-addext", "subjectAltName=DNS:updates.d4p-test.example",
            "-addext", "basicConstraints=critical,CA:TRUE",
            "-addext", "keyUsage=critical,digitalSignature,keyCertSign,keyEncipherment",
            "-addext", "extendedKeyUsage=serverAuth",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    private_key_metadata = private_tls_key.lstat()
    if private_key_metadata.st_mode & 0o077:
        raise ValueError("run-scoped TLS private keyのfile permissionが広すぎます")
    return {
        "evidence_type": "linux_same_source_update_candidate",
        "source_commit": source_commit,
        "bundle_artifact_tree_sha256": evidence.get("artifact_tree_sha256"),
        "product_version": package_record["product_version"],
        "target_platform": package_record["target_platform"],
        "package_sha256": package_record["package_sha256"],
        "package_size_bytes": package_record["package_size_bytes"],
        "payload_is_product_d4pkg": True,
        "catalog_sha256": hashlib.sha256(
            Path(catalog_record["catalog_path"]).read_bytes()
        ).hexdigest(),
        "signing_identity": catalog_record["signing_identity"],
        "tls_identity": "run-scoped self-signed test CA for exact acceptance host",
        "formal_distribution_claimed": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--private-key", type=Path, required=True)
    parser.add_argument("--build-evidence", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    args = parser.parse_args()
    try:
        result = _prepare(
            args.bundle, args.output_root, args.private_key, args.build_evidence, args.source_commit
        )
    except (OSError, ValueError, json.JSONDecodeError, subprocess.CalledProcessError) as error:
        print(
            "same-source Linux update candidate作成を停止: "
            f"{type(error).__name__}: {_safe_exception_summary(error)}",
            file=sys.stderr,
        )
        return 1
    print(json.dumps(result, ensure_ascii=False, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
