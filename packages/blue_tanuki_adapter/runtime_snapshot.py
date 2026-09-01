def runtime_snapshot() -> dict:
    return {
        "runtime_id": "blue_tanuki",
        "name": "BLUE-TANUKI",
        "kind": "reference_runtime",
        "status": "ready",
        "adapter_id": "blue_tanuki_reference",
        "version": "mock-contract",
        "endpoints": {
            "health": "/health",
            "ready": "/ready",
            "runtime_snapshot": "/runtime/snapshot",
            "authority_trace": "/authority/trace",
            "notifications": "/notifications",
            "approvals": "/approvals",
            "audit_events": "/audit/events",
            "diagnostics_export": "/diagnostics/export",
            "recovery": "/recovery",
        },
        "capabilities": [
            "runtime.read",
            "approval.review",
            "audit.read",
            "diagnostics.read",
            "recovery.read",
        ],
        "diagnostic_summary": "mock adapter 契約を利用できます",
    }
