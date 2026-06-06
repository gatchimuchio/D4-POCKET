NON_AUTHORITY_SOURCES = {
    "adapter_metadata",
    "cache",
    "diagnostics",
    "external_metadata",
    "generated_config",
    "generated_output",
    "gui_state",
    "history",
    "local_ui_state",
    "memory",
    "metadata",
    "model_output",
    "previous_state",
    "tool_output",
    "tool_response",
    "ui_state",
}

AUTHORITY_SOURCES = {
    "broker_internal_policy",
    "policy",
    "runtime",
    "rust_security_broker",
}


class PermissionLedger:
    def __init__(self):
        self._permissions: dict[str, dict] = {}

    def record(self, permission: dict) -> None:
        self._permissions[permission["permission_id"]] = dict(permission)

    def decision_for(self, permission_id: str) -> str:
        permission = self._permissions.get(permission_id)
        if permission is None:
            return "deny"
        return permission.get("decision", "deny")

    def can_grant_authority_from_source(self, source: str) -> bool:
        return source in AUTHORITY_SOURCES
