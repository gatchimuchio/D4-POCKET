from __future__ import annotations

from pathlib import Path

from .adapter_loader import load_adapter
from .content_exposure import project_approval_content
from .permission_ledger import NON_AUTHORITY_SOURCES, PermissionLedger


class InvariantEvaluator:
    def __init__(self, root: Path | None = None):
        self.root = root or Path(__file__).resolve().parents[2]

    def evaluate(self) -> dict[str, bool]:
        return {
            "flutter_imported_by_shell_core": self.shell_core_imports_forbidden("flutter"),
            "blue_tanuki_imported_by_shell_core": self.shell_core_imports_forbidden("blue_tanuki"),
            "adapter_metadata_can_escalate_authority": self.adapter_metadata_can_escalate_authority(),
            "memory_cache_previous_state_can_grant_authority": self.non_authority_sources_can_grant_authority(),
            "full_payload_projected_without_full_visibility": self.full_payload_projected_without_full_visibility(),
            "installer_setup_state_can_grant_authority": self.installer_setup_state_can_grant_authority(),
            "mobile_device_state_can_grant_authority": self.mobile_device_state_can_grant_authority(),
        }

    def shell_core_imports_forbidden(self, forbidden_module: str) -> bool:
        shell_core = self.root / "packages" / "shell_core"
        if not shell_core.exists():
            return True
        for path in sorted(shell_core.rglob("*.py")):
            if "__pycache__" in path.parts:
                continue
            text = path.read_text(encoding="utf-8")
            for line in text.splitlines():
                normalized = line.strip().lower()
                if normalized.startswith(f"import {forbidden_module}") or normalized.startswith(
                    f"from {forbidden_module}"
                ):
                    return True
        return False

    def adapter_metadata_can_escalate_authority(self) -> bool:
        adapter = {
            "adapter_id": "invariant_probe",
            "runtime_id": "runtime_probe",
            "contract_version": "1.0.0",
            "authority_strip": True,
            "declared_capabilities": ["runtime.read"],
            "metadata": {
                "Trust_Level": "root",
                "permissionGrant": "all",
                "safe_label": "probe",
            },
        }
        record = load_adapter(adapter)
        return record.metadata != {"safe_label": "probe"} or record.effective_capabilities() != ("runtime.read",)

    def non_authority_sources_can_grant_authority(self) -> bool:
        ledger = PermissionLedger()
        return any(ledger.can_grant_authority_from_source(source) for source in NON_AUTHORITY_SOURCES)

    def full_payload_projected_without_full_visibility(self) -> bool:
        base = {
            "content_visibility": "redacted",
            "payload_hash": "sha256:" + "1" * 64,
            "summary": "summary",
            "redacted_payload": {"path": "notes/today.md"},
            "full_payload": {"path": "notes/today.md", "content": "secret"},
        }
        for visibility in ("none", "hash_only", "summary", "redacted"):
            projected = project_approval_content({**base, "content_visibility": visibility})
            if "full_payload" in projected:
                return True
        return False

    def installer_setup_state_can_grant_authority(self) -> bool:
        try:
            from installer.setup_doctor import setup_doctor_report
        except Exception:
            return True
        report = setup_doctor_report()
        if report.get("installer_grants_authority") is not False:
            return True
        if report.get("installer_silently_approves_permissions") is not False:
            return True
        return any(check.get("grants_authority") is not False for check in report.get("checks", []))

    def mobile_device_state_can_grant_authority(self) -> bool:
        mobile_root = self.root / "apps" / "mobile_flutter" / "lib"
        if not mobile_root.exists():
            return False
        forbidden = (
            "independent authority: true",
            "silently pair",
            "adapter_can_approve: true",
            "metadata_trusted: true",
        )
        combined = "\n".join(path.read_text(encoding="utf-8").lower() for path in sorted(mobile_root.glob("**/*.dart")))
        return any(pattern in combined for pattern in forbidden)
