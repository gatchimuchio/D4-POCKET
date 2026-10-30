"""公開RFC 8032鍵だけを使うLinux installed Update Center受入れfixture生成器。"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
FIXTURE_DIR = ROOT / "tooling/tests/fixtures/linux-update-catalog"
APP_ID = "d4-pocket-app-11111111111111111111111111111111"
AUDIT_ID = "audit-store-22222222222222222222222222222222"
UPDATE_ID = "linux-installed-catalog-display"
OFFERED_VERSION = "0.1.2"
CURRENT_VERSION = "0.1.1"
SUMMARY = "Linux installed Update Center download and apply acceptance fixture"
PUBLIC_KEY_DER = bytes.fromhex(
    "302a300506032b6570032100"
    "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
)
PUBLISHED_TEST_PRIVATE_KEY_DER = bytes.fromhex(
    "302e020100300506032b657004220420"
    "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
)
FINGERPRINT = "sha256:21fe31dfa154a261626bf854046fd2271b7bed4b6abe45aa58877ef47f9721b9"


def _compact_json(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")


def build_package() -> bytes:
    """小型の非実行payload。stage／activation受入れ専用で起動しない。"""
    product_manifest = _compact_json(
        {
            "version": 1,
            "product": "D4 Pocket",
            "export_id": "linux-update-fixture",
            "manifest": {
                "app_identity": {"app_id": APP_ID, "target_platform": "linux"},
                "audit_store": {
                    "store_id": AUDIT_ID,
                    "chain_status": "new",
                    "inherited": False,
                },
                "inheritance_policy": {
                    "authority": "none",
                    "permission": "none",
                    "approval": "none",
                    "credential": "none",
                    "audit_chain": "none",
                },
                "distribution_metadata": {"target_platform": "linux"},
            },
        }
    )
    payloads = [
        (
            "data/icudtl.dat",
            b"D4P Linux update fixture data; not a product runtime.\n",
            0o644,
        ),
        ("gui_shell_desktop", b"D4P update fixture; deliberately not executable ELF.\n", 0o755),
        (
            "gui_shell_desktop_launcher",
            b"D4P update fixture; deliberately not executable ELF.\n",
            0o755,
        ),
        (
            "lib/libflutter_linux_gtk.so",
            b"D4P update fixture shared-library placeholder; do not load.\n",
            0o644,
        ),
        (
            "libexec/gui_shell_linux_broker",
            b"D4P update fixture broker placeholder; do not execute.\n",
            0o755,
        ),
        ("product_manifest.json", product_manifest, 0o644),
    ]
    files = [
        {
            "path": name,
            "byte_length": len(payload),
            "sha256": hashlib.sha256(payload).hexdigest(),
            "mode": mode,
        }
        for name, payload, mode in payloads
    ]
    manifest = _compact_json(
        {
            "version": 1,
            "product": "D4 Pocket",
            "product_version": OFFERED_VERSION,
            "app_id": APP_ID,
            "audit_store_id": AUDIT_ID,
            "target_platform": "linux",
            "files": files,
        }
    )
    return b"D4PKG01\n" + len(manifest).to_bytes(4, "little") + manifest + b"".join(
        payload for _, payload, _ in payloads
    )


def _sign_with_public_test_key(private_key: Path, signed_bytes: bytes) -> bytes:
    metadata = private_key.lstat()
    if private_key.is_symlink() or not private_key.is_file() or metadata.st_size != 48:
        raise ValueError("RFC 8032試験鍵は48バイトの通常DERファイルである必要があります")
    with tempfile.TemporaryDirectory(prefix="d4p-linux-update-sign-") as temporary:
        temporary_path = Path(temporary)
        input_path = temporary_path / "candidate.bin"
        output_path = temporary_path / "signature.bin"
        input_path.write_bytes(signed_bytes)
        public_result = subprocess.run(
            [
                "openssl",
                "pkey",
                "-inform",
                "DER",
                "-in",
                str(private_key),
                "-pubout",
                "-outform",
                "DER",
            ],
            check=True,
            capture_output=True,
        )
        if public_result.stdout != PUBLIC_KEY_DER:
            raise ValueError("秘密鍵が公開済みRFC 8032試験ベクトルと一致しません")
        subprocess.run(
            [
                "openssl",
                "pkeyutl",
                "-sign",
                "-rawin",
                "-keyform",
                "DER",
                "-inkey",
                str(private_key),
                "-in",
                str(input_path),
                "-out",
                str(output_path),
            ],
            check=True,
            capture_output=True,
        )
        signature = output_path.read_bytes()
    if len(signature) != 64:
        raise ValueError("OpenSSLが不正なEd25519署名長を返しました")
    return signature


def build_catalog(package: bytes, private_key: Path) -> bytes:
    package_hash = hashlib.sha256(package).hexdigest()
    signed_document = {
        "版": 2,
        "更新ID": UPDATE_ID,
        "現行版": CURRENT_VERSION,
        "提供版": OFFERED_VERSION,
        "channel": "stable",
        "内容概要": SUMMARY,
        "package_sha256": package_hash,
        "package_size_bytes": len(package),
        "rollback可能": True,
    }
    signed_bytes = _compact_json(signed_document)
    candidate = {
        **signed_document,
        "署名対象": signed_bytes.hex(),
        "署名": _sign_with_public_test_key(private_key, signed_bytes).hex(),
        "署名者fingerprint": FINGERPRINT,
    }
    return (
        json.dumps({"版": 1, "候補": [candidate]}, ensure_ascii=False, indent=2) + "\n"
    ).encode("utf-8")


def _atomic_write(path: Path, content: bytes) -> None:
    if path.is_symlink() or (path.exists() and not path.is_file()):
        raise ValueError(f"fixture出力が通常fileではありません: {path.name}")
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.", dir=path.parent
    )
    temporary_path = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as output:
            output.write(content)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary_path, path)
    finally:
        if temporary_path.exists():
            temporary_path.unlink()


def write_fixture(private_key: Path | None = None) -> dict[str, Any]:
    if not FIXTURE_DIR.is_dir() or FIXTURE_DIR.is_symlink():
        raise ValueError("追跡対象Linux更新fixture用directoryを利用できません")
    package = build_package()
    if private_key is None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-update-key-") as temporary:
            key_path = Path(temporary) / "published-rfc8032-test.der"
            key_path.write_bytes(PUBLISHED_TEST_PRIVATE_KEY_DER)
            catalog = build_catalog(package, key_path)
    else:
        catalog = build_catalog(package, private_key.resolve(strict=True))
    package_path = FIXTURE_DIR / f"{UPDATE_ID}.pkg"
    catalog_path = FIXTURE_DIR / "updates.json"
    _atomic_write(package_path, package)
    _atomic_write(catalog_path, catalog)
    return {
        "fixture_only": True,
        "product_payload_executable": False,
        "package_path": package_path.relative_to(ROOT).as_posix(),
        "package_sha256": hashlib.sha256(package).hexdigest(),
        "package_size_bytes": len(package),
        "catalog_path": catalog_path.relative_to(ROOT).as_posix(),
        "signing_identity": "published RFC 8032 test vector; never production trust",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--private-key",
        type=Path,
        help="RFC 8032試験ベクトル用DER鍵（任意）。省略時は公開試験ベクトルを使います",
    )
    args = parser.parse_args()
    print(
        json.dumps(
            write_fixture(args.private_key),
            ensure_ascii=False,
            separators=(",", ":"),
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
