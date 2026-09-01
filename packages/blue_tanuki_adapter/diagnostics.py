def diagnostics_export() -> dict:
    return {
        "diagnostic_id": "blue-tanuki-diagnostic-1",
        "runtime_id": "blue_tanuki",
        "status": "pass",
        "checks": [
            {
                "check_id": "adapter_contract",
                "status": "pass",
                "message": "mock adapter 契約を利用できます",
            }
        ],
    }
