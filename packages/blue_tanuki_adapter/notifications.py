def notifications() -> list[dict]:
    return [
        {
            "notification_id": "blue-tanuki-runtime-ready",
            "runtime_id": "blue_tanuki",
            "severity": "info",
            "message": "BLUE-TANUKI 参照 runtime mock は準備済みです。",
        }
    ]
