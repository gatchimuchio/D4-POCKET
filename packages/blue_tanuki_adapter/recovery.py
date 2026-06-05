def recovery_candidates(reason: str) -> list[dict]:
    recovery_class = "runtime_down" if reason == "runtime_down" else "unknown"
    return [
        {
            "recovery_id": f"blue-tanuki-{recovery_class}",
            "runtime_id": "blue_tanuki",
            "operation": "runtime.read",
            "class": recovery_class,
            "severity": "warning",
            "user_visible_message": "Check BLUE-TANUKI runtime connectivity and retry after it is ready.",
            "safe_to_retry": True,
            "steps": [
                "Open Runtime Center.",
                "Check BLUE-TANUKI adapter health.",
                "Retry after runtime status is ready.",
            ],
            "requires_user_action": True,
        }
    ]
