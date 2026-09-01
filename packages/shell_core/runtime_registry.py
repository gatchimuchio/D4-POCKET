import copy


class RuntimeRegistry:
    def __init__(self):
        self._runtimes: dict[str, dict] = {}

    def register(self, runtime: dict) -> None:
        runtime_id = runtime["runtime_id"]
        self._runtimes[runtime_id] = copy.deepcopy(runtime)

    def get(self, runtime_id: str) -> dict:
        if runtime_id not in self._runtimes:
            raise KeyError(f"未登録の runtime です: {runtime_id}")
        return copy.deepcopy(self._runtimes[runtime_id])

    def snapshot(self) -> list[dict]:
        return [copy.deepcopy(self._runtimes[key]) for key in sorted(self._runtimes)]
