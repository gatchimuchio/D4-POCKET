def recovery_candidates(reason: str) -> list[dict]:
    recovery_class = "runtime_down" if reason == "runtime_down" else "unknown"
    return [
        {
            "recovery_id": f"blue-tanuki-{recovery_class}",
            "runtime_id": "blue_tanuki",
            "operation": "runtime.read",
            "class": recovery_class,
            "severity": "warning",
            "user_visible_message": "BLUE-TANUKI runtime の接続を確認し、準備完了後に再実行してください。",
            "safe_to_retry": True,
            "steps": [
                "実行系センターを開いてください。",
                "BLUE-TANUKI adapter の health を確認してください。",
                "runtime 状態が ready になってから再実行してください。",
            ],
            "requires_user_action": True,
        }
    ]
