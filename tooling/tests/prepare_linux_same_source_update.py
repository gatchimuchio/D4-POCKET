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


def _build_evidence_validation_inputs(
    evidence: dict[str, object], output_root: Path
) -> tuple[Path, Path | None, bytes | None]:
    product_output = output_root / "output"
    if product_output.is_symlink() or not product_output.is_dir():
        raise ValueError("Linux build outputは通常directoryではありません")
    product_output = product_output.resolve(strict=True)

    package_record = evidence.get("package")
    if not isinstance(package_record, dict):
        raise ValueError("Linux build evidenceにD4PKG recordがありません")
    package_name = package_record.get("file_name")
    if (
        not isinstance(package_name, str)
        or not package_name
        or Path(package_name).name != package_name
        or "/" in package_name
        or "\\" in package_name
    ):
        raise ValueError("Linux build evidenceのD4PKG file名が不正です")
    package_path = _regular_file(product_output / package_name, "Linux build D4PKG")

    archive_record = evidence.get("portable_archive")
    if archive_record is None:
        return package_path, None, None
    if not isinstance(archive_record, dict):
        raise ValueError("Linux build evidenceのportable archive recordが不正です")
    archive_name = archive_record.get("file_name")
    if (
        not isinstance(archive_name, str)
        or not archive_name
        or Path(archive_name).name != archive_name
        or "/" in archive_name
        or "\\" in archive_name
    ):
        raise ValueError("Linux build evidenceのportable archive file名が不正です")
    archive_path = _regular_file(
        product_output / archive_name, "Linux portable archive"
    )
    readme_path = _regular_file(
        ROOT / "installer" / "linux" / "PORTABLE_README_ja.txt",
        "Linux portable archive README",
    )
    return package_path, archive_path, readme_path.read_bytes()


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
    if not isinstance(evidence, dict):
        raise ValueError("Linux build evidenceはobjectではありません")
    if evidence.get("target_platform") != "linux" or evidence.get("source_commit") != source_commit:
        raise ValueError("Linux release bundle evidenceが指定source commitと一致しません")
    package_path, archive_path, readme_raw = _build_evidence_validation_inputs(
        evidence, output_root
    )
    validate_build_evidence(
        evidence,
        bundle,
        package_path,
        (bundle / "product_manifest.json").read_bytes(),
        archive_path,
        readme_raw,
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

    certificate_authority = update_server_root / "ca.pem"
    private_ca_key = update_server_root / "ca.key"
    certificate = update_server_root / "server.pem"
    private_tls_key = update_server_root / "server.key"
    certificate_request = update_server_root / "server.csr"
    certificate_extensions = update_server_root / "server-extensions.cnf"
    ca_serial = update_server_root / "ca.srl"
    subprocess.run(
        [
            "openssl", "req", "-x509", "-newkey", "rsa:2048", "-sha256",
            "-nodes", "-days", "2", "-keyout", str(private_ca_key),
            "-out", str(certificate_authority), "-subj", "/CN=D4 Pocket Linux Acceptance CA",
            "-addext", "basicConstraints=critical,CA:TRUE,pathlen:0",
            "-addext", "keyUsage=critical,keyCertSign,cRLSign",
            "-addext", "subjectKeyIdentifier=hash",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    subprocess.run(
        [
            "openssl", "req", "-new", "-newkey", "rsa:2048", "-sha256", "-nodes",
            "-keyout", str(private_tls_key), "-out", str(certificate_request),
            "-subj", "/CN=updates.d4p-test.example",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    certificate_extensions.write_text(
        "[server_cert]\n"
        "basicConstraints=critical,CA:FALSE\n"
        "keyUsage=critical,digitalSignature,keyEncipherment\n"
        "extendedKeyUsage=serverAuth\n"
        "subjectAltName=DNS:updates.d4p-test.example\n"
        "subjectKeyIdentifier=hash\n"
        "authorityKeyIdentifier=keyid,issuer\n",
        encoding="ascii",
    )
    subprocess.run(
        [
            "openssl", "x509", "-req", "-in", str(certificate_request),
            "-CA", str(certificate_authority), "-CAkey", str(private_ca_key),
            "-CAcreateserial", "-out", str(certificate), "-days", "2", "-sha256",
            "-extfile", str(certificate_extensions), "-extensions", "server_cert",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    subprocess.run(
        [
            "openssl", "verify", "-CAfile", str(certificate_authority),
            "-purpose", "sslserver", "-verify_hostname", "updates.d4p-test.example",
            str(certificate),
        ],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    for private_key in (private_ca_key, private_tls_key):
        if private_key.lstat().st_mode & 0o077:
            raise ValueError("run-scoped TLS private keyのfile permissionが広すぎます")
    private_ca_key.unlink()
    certificate_request.unlink()
    certificate_extensions.unlink()
    ca_serial.unlink()
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
        "tls_identity": "run-scoped test CA with host-specific server leaf certificate",
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
