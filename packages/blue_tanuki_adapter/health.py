def health() -> dict:
    return {
        "runtime_id": "blue_tanuki",
        "status": "ready",
        "message": "参照 runtime mock の health は準備済みです",
    }


def ready() -> dict:
    return {
        "runtime_id": "blue_tanuki",
        "ready": True,
    }
