from packages.shell_core.content_exposure import project_approval_content


def normalize_approval(source: dict) -> dict:
    status = source.get("status", "pending")
    if source.get("adapter_approved") is True:
        status = "pending"

    approval = {
        "approval_id": source.get("approval_id", "blue-tanuki-approval-1"),
        "runtime_id": "blue_tanuki",
        "operation": source.get("operation", "runtime.read"),
        "target_scope": source.get("target_scope", "runtime_status"),
        "status": status,
        "content_visibility": source.get("content_visibility", "redacted"),
        "payload_hash": source.get(
            "payload_hash",
            "sha256:3333333333333333333333333333333333333333333333333333333333333333",
        ),
        "summary": source.get("summary", "Review BLUE-TANUKI runtime operation."),
        "redacted_payload": source.get("redacted_payload", {"target": "runtime", "details": "[redacted]"}),
        "full_payload": source.get("full_payload", {"target": "runtime", "details": "mock"}),
        "editable_fields": source.get("editable_fields", ["target"]),
        "sealed_fields": source.get("sealed_fields", ["runtime_id"]),
        "hidden_fields": source.get("hidden_fields", []),
        "sacred_fields": source.get("sacred_fields", ["authority_context"]),
        "authority_fields": source.get("authority_fields", ["permission_id"]),
    }
    if approval["status"] == "approved" and source.get("approved_by") == "adapter":
        approval["status"] = "pending"
    return approval


def projected_approval(source: dict) -> dict:
    return project_approval_content(normalize_approval(source))
