from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from installer.first_run import first_run_smoke
from packages.agent_runtime import AgentRuntimeContract
from packages.runtime_catalog import RuntimeCatalog
from packages.shell_core.release_smoke import run_shell_core_release_smoke


def run_release_smokes(root: Path | None = None) -> dict:
    if root is None:
        with tempfile.TemporaryDirectory() as tmp:
            return run_release_smokes(Path(tmp))

    shell_core = run_shell_core_release_smoke(root / "shell_core")
    first_run = first_run_smoke(root / "first_run")
    runtime_catalog = run_runtime_catalog_smoke()
    agent_runtime = run_agent_runtime_smoke()
    errors = []
    if not shell_core["ok"]:
        errors.extend(f"shell_core: {error}" for error in shell_core["errors"])
    if not first_run["ok"]:
        errors.extend(f"first_run: {error}" for error in first_run["errors"])
    if not runtime_catalog["ok"]:
        errors.extend(f"runtime_catalog: {error}" for error in runtime_catalog["errors"])
    if not agent_runtime["ok"]:
        errors.extend(f"agent_runtime: {error}" for error in agent_runtime["errors"])
    return {
        "ok": not errors,
        "errors": errors,
        "shell_core": shell_core,
        "first_run": first_run,
        "runtime_catalog": runtime_catalog,
        "agent_runtime": agent_runtime,
    }


def _example(name: str) -> dict:
    return json.loads((ROOT / "examples" / "contracts" / name).read_text(encoding="utf-8"))


def run_runtime_catalog_smoke() -> dict:
    errors: list[str] = []
    catalog = RuntimeCatalog()
    runtime_manifest = _example("runtime_manifest.valid.json")
    adapter_manifest = _example("adapter_manifest.valid.json")
    catalog.register_runtime_manifest(runtime_manifest)
    catalog.register_adapter_manifest(adapter_manifest)
    if len(catalog.runtime_manifests()) != 1:
        errors.append("実行系 manifest が登録されなかった")
    if len(catalog.adapter_manifests()) != 1:
        errors.append("Adapter manifest が登録されなかった")
    if catalog.can_grant_authority(runtime_manifest):
        errors.append("実行系台帳が権限を付与した")
    return {"ok": not errors, "errors": errors, "runtime_count": len(catalog.runtime_manifests()), "adapter_count": len(catalog.adapter_manifests())}


def run_agent_runtime_smoke() -> dict:
    errors: list[str] = []
    contract = AgentRuntimeContract(_example("agent_workspace.valid.json"))
    if not contract.path_allowed("/workspace/project/src/main.py"):
        errors.append("エージェント実行系が作業空間内のパスを拒否した")
    if contract.path_allowed("/workspace/project/.env"):
        errors.append("エージェント実行系が機密パスを許可した")
    if contract.path_allowed("/outside/project/file.txt"):
        errors.append("エージェント実行系が作業空間外のパスを許可した")
    if not contract.shell_command_requires_permission(_example("agent_tool_call.valid.json")):
        errors.append("エージェント実行系が shell command に権限対応付けを要求しなかった")
    if not contract.diff_is_auditable(_example("agent_diff.valid.json")):
        errors.append("エージェント実行系が監査可能な差分を拒否した")
    return {"ok": not errors, "errors": errors}


def main() -> int:
    result = run_release_smokes()
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
