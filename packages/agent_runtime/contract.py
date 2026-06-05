from pathlib import Path


SECRET_PATH_PARTS = {".env", ".ssh", ".gnupg", "secrets"}


class AgentRuntimeContract:
    def __init__(self, workspace: dict):
        self.workspace = dict(workspace)
        self.root_path = Path(workspace["root_path"]).resolve()
        self.secret_paths = tuple(workspace.get("secret_paths", []))
        self.resolved_secret_paths = tuple(self._resolve_candidate(secret) for secret in self.secret_paths)

    def path_allowed(self, candidate: str) -> bool:
        path = self._resolve_candidate(candidate)
        try:
            path.relative_to(self.root_path)
        except ValueError:
            return False
        return not self.is_secret_path(path)

    def is_secret_path(self, candidate: str | Path) -> bool:
        path = self._resolve_candidate(candidate)
        parts = set(path.parts)
        if parts & SECRET_PATH_PARTS:
            return True
        for secret in self.resolved_secret_paths:
            if path == secret or secret in path.parents:
                return True
        return False

    def _resolve_candidate(self, candidate: str | Path) -> Path:
        path = Path(candidate)
        if not path.is_absolute():
            path = self.root_path / path
        return path.resolve()

    def shell_command_requires_permission(self, tool_call: dict) -> bool:
        return tool_call.get("tool_name") == "shell.command" and bool(tool_call.get("permission_id"))

    def git_push_requires_explicit_approval(self, tool_call: dict) -> bool:
        if tool_call.get("tool_name") != "git.push":
            return True
        return tool_call.get("approval_required") is True and bool(tool_call.get("permission_id"))

    def diff_is_auditable(self, diff: dict) -> bool:
        return bool(diff.get("audit_event_id")) and bool(diff.get("payload_hash"))

    def auto_permission_is_advisory_only(self, runtime: dict) -> bool:
        return runtime.get("auto_permission_mode") in {"disabled", "advisory_only"}

    def state_change_has_rollback(self, record: dict) -> bool:
        return bool(record.get("rollback_candidate_id"))
