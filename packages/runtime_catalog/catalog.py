import copy

from packages.shell_core.normalization import authority_keys_in, authority_values_in, strip_authority_keys


class RuntimeCatalog:
    def __init__(self):
        self._runtimes: dict[str, dict] = {}
        self._adapters: dict[str, dict] = {}

    def register_runtime_manifest(self, manifest: dict) -> None:
        if manifest.get("signed_manifest") is not True:
            raise ValueError("runtime manifest には署名が必要です")
        self._runtimes[manifest["runtime_id"]] = copy.deepcopy(manifest)

    def register_adapter_manifest(self, manifest: dict) -> None:
        if manifest.get("authority_strip") is not True:
            raise ValueError("adapter manifest は authority_strip=true を必須としなければなりません")
        if manifest.get("signed_manifest") is not True:
            raise ValueError("adapter manifest には署名が必要です")
        self._adapters[manifest["adapter_id"]] = copy.deepcopy(manifest)

    def runtime_manifests(self) -> list[dict]:
        return [copy.deepcopy(self._runtimes[key]) for key in sorted(self._runtimes)]

    def adapter_manifests(self) -> list[dict]:
        return [copy.deepcopy(self._adapters[key]) for key in sorted(self._adapters)]

    def can_grant_authority(self, manifest: dict) -> bool:
        return False

    def metadata_attempts_authority(self, metadata: dict) -> bool:
        stripped = strip_authority_keys(metadata)
        return bool(authority_keys_in(metadata) or authority_values_in(stripped))
