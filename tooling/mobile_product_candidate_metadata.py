"""Shared provenance helpers for Android and iOS mobile candidates."""

from __future__ import annotations

import hashlib
import json
import stat
from pathlib import Path
from typing import Any

DEVICE_LINK_SCHEMA_FILES = (
    "specs/device_link_control.schema.json",
    "specs/device_link_credential.schema.json",
    "specs/device_link_invitation.schema.json",
    "specs/device_link_request.schema.json",
)
MAX_DEVICE_LINK_SCHEMA_BYTES = 1_000_000


def device_link_schema_evidence(root: Path) -> dict[str, Any]:
    """Bind a candidate to the exact four Device Link contracts in its source."""
    records: dict[str, str] = {}
    for relative in DEVICE_LINK_SCHEMA_FILES:
        path = root / relative
        try:
            before = path.lstat()
        except OSError as error:
            raise ValueError("DeviceLink Schemaを確認できない") from error
        if not stat.S_ISREG(before.st_mode) or stat.S_ISLNK(before.st_mode):
            raise ValueError("DeviceLink Schemaが通常fileではない")
        if before.st_size < 1 or before.st_size > MAX_DEVICE_LINK_SCHEMA_BYTES:
            raise ValueError("DeviceLink Schemaがsize範囲外")
        try:
            content = path.read_bytes()
            after = path.lstat()
        except OSError as error:
            raise ValueError("DeviceLink Schemaを読み取れない") from error
        if (
            (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns)
            != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns)
            or len(content) != before.st_size
        ):
            raise ValueError("DeviceLink Schemaが読み取り中に変化した")
        records[Path(relative).name] = hashlib.sha256(content).hexdigest()

    canonical = json.dumps(records, sort_keys=True, separators=(",", ":")).encode("ascii")
    return {"sha256": hashlib.sha256(canonical).hexdigest(), "files": records}
