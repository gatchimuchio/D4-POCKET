import copy

from .error_taxonomy import CONTENT_VISIBILITY_VIOLATION, shell_error


def project_approval_content(approval: dict) -> dict:
    visibility = approval.get("content_visibility")
    if visibility == "none":
        return {}
    if visibility == "hash_only":
        return {"payload_hash": approval["payload_hash"]}
    if visibility == "summary":
        return {"summary": approval.get("summary", "")}
    if visibility == "redacted":
        return {"redacted_payload": copy.deepcopy(approval.get("redacted_payload", {}))}
    if visibility == "full":
        return {"full_payload": copy.deepcopy(approval.get("full_payload", {}))}
    return {
        "error": shell_error(
            CONTENT_VISIBILITY_VIOLATION,
            f"未知または欠落した content_visibility です: {visibility}",
            "content_projection",
            recoverable=True,
        )
    }
