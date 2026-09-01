import copy


class RecoveryCatalog:
    def __init__(self):
        self._actions: dict[str, dict] = {}

    def register(self, recovery_action: dict) -> None:
        self._actions[recovery_action["recovery_id"]] = copy.deepcopy(recovery_action)

    def get(self, recovery_id: str) -> dict:
        if recovery_id not in self._actions:
            raise KeyError(f"未登録の recovery action です: {recovery_id}")
        return copy.deepcopy(self._actions[recovery_id])
