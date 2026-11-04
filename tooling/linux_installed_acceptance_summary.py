from __future__ import annotations

import sys
from pathlib import Path

def summarize_runtime_provenance() -> None:
    import json, pathlib, sys
    matches = []
    for line in pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines():
        try:
            candidate = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(candidate, dict) and candidate.get("acceptance_scope") == "runtime_provenance":
            matches.append(candidate)
    if len(matches) != 1:
        raise SystemExit(f"Runtime provenanceの結果は1件必要です。取得数: {len(matches)}")
    result = matches[0]
    runtime = result["runtime_identity"]
    audit = result["audit_store_observation"]
    components = result["component_sha256"]
    print("\n### 導入済みRuntimeの来歴（`LIVE_RUNTIME`）")
    print(f"- 対象source commit: `{result['source_commit']}`")
    print(f"- package hashと容量: `{result['package_sha256']}`（{result['package_byte_length']} byte）")
    print(f"- artifact tree: `{result['artifact_tree_sha256']}`、導入済みtree: `{result['installed_artifact_tree_sha256']}`（{result['artifact_count']} file）")
    print(f"- 導入先root: `{result['installed_root']}`")
    print(f"- 有効stage: `{result['active_stage']}`")
    print(f"- 実行identity: App `{runtime['app_id']}`、Audit Store `{runtime['audit_store_id']}`")
    print(f"- 構成部品: Runner `{components['runner']}`、Launcher `{components['launcher']}`、Broker `{components['broker']}`")
    print(f"- 稼働中Runner: `{result['live_processes']['runner']['sha256']}`（{result['live_processes']['runner']['process_count']} process）、稼働中Broker: `{result['live_processes']['broker']['sha256']}`（{result['live_processes']['broker']['process_count']} process）")
    print(f"- Audit Store path: `{audit['path']}`、Audit file `{audit['audit.jsonl']['sha256']}`、anchor `{audit['audit_anchor.json']['sha256']}`、秘密鍵fileの存在: `{audit['anchor_key_private_file']}`")
    print("- 証拠範囲: Ubuntu 24.04 hosted runnerのXvfb／X11と未署名合成test identity。導入artifactと稼働中Runtime pathをこのsource commitへ結合する。実desktop session、Wayland描画、署名、正式配布、Linux全体のrelease readinessは示さない。")

def summarize_wayland_rendering() -> None:
    import json, pathlib, sys
    matches = []
    for line in pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines():
        try:
            candidate = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(candidate, dict) and candidate.get("acceptance_scope") == "wayland_rendering":
            matches.append(candidate)
    if len(matches) != 1:
        raise SystemExit(f"Wayland描画の結果は1件必要です。取得数: {len(matches)}")
    result = matches[0]
    frame = result["compositor_output"]
    runner = result["live_installed_runner"]["runner"]
    print("\n### 導入済み製品のWayland画面描画（`LIVE_RUNTIME`）")
    print(f"- 対象source commit: `{result['source_commit']}`")
    print(f"- 導入package SHA-256: `{result['installed_package_sha256']}`")
    print(f"- 表示方式: {result['display_server']}、window表示: `{result['visible_window']}`")
    print(f"- compositor出力: `{frame['width']}×{frame['height']}`、変化pixel `{frame['changed_pixels']}`（下限 `{frame['required_changed_pixels']}`）")
    print(f"- 基準PNG SHA-256: `{frame['baseline_png_sha256']}`、描画PNG SHA-256: `{frame['rendered_png_sha256']}`、画像file保持: `{frame['image_files_retained']}`")
    print(f"- 稼働中の導入済みRunner: SHA-256 `{runner['sha256']}`、process数 `{runner['process_count']}`")
    print(f"- 管理対象processのcleanup: `{result['owned_process_cleanup']}`")
    print("- 証拠範囲: 隔離Weston headless／Pixman compositor上の導入済みUI画面出力。実desktop session、Wayland compositor間の差、正式配布／署名、Linux全体のrelease readinessは示さない。")

def summarize_wayland_workspace_selection() -> None:
    import json, pathlib, sys
    matches = []
    for line in pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines():
        try:
            candidate = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(candidate, dict) and candidate.get("acceptance_scope") == "wayland_workspace_selection":
            matches.append(candidate)
    if len(matches) != 1:
        raise SystemExit(f"Wayland Workspace選択の結果は1件必要です。取得数: {len(matches)}")
    result = matches[0]
    evidence = result["workspace_selection"]
    runner = result["live_installed_runner"]["runner"]
    print("\n### Waylandでの導入済み製品Workspace選択とOwner境界（`LIVE_RUNTIME`）")
    print(f"- 対象source commit: `{result['source_commit']}`")
    print(f"- package識別値 SHA-256: `{result['installed_package_sha256']}`、製品版 `{result['product_version']}`")
    print(f"- 表示方式: {result['display_server']}、window表示 `{result['visible_window']}`")
    print(f"- 入力方式: `{result['keyboard_input']}`")
    print(f"- native選択: `{evidence['workspace_selection']}`")
    print(f"- Owner境界: `{evidence['registration']}`、範囲付きCLI probe `{evidence['cli_interface_probe_calls']}`")
    print(f"- Task実行能力: `{evidence['task_execution']}`、Workspace Permission `{evidence['workspace_read_permission']}`")
    print(f"- 登録の永続Audit: `{evidence['accepted_registration_audit']}`、secret／root露出 `{evidence['audit_secret_and_root_exposure']}`")
    print(f"- 稼働中Runner SHA-256: `{runner['sha256']}`、process数 `{runner['process_count']}`")
    print(f"- 管理対象processのcleanup: `{result['owned_process_cleanup']}`")
    print("- 証拠範囲: X11 DISPLAYを未設定にしたheadless Weston／Pixman Wayland session。実desktop session、compositor間の差、署名／正式配布、Linux全体のrelease readinessは示さない。")

OPERATIONS = {
    'summarize-runtime-provenance': summarize_runtime_provenance,
    'summarize-wayland-rendering': summarize_wayland_rendering,
    'summarize-wayland-workspace-selection': summarize_wayland_workspace_selection,
}


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] not in OPERATIONS:
        allowed = ", ".join(sorted(OPERATIONS))
        raise SystemExit(f"使い方: `{Path(__file__).name}` <{allowed}> [引数]")
    operation = OPERATIONS[sys.argv[1]]
    sys.argv = [sys.argv[0], *sys.argv[2:]]
    operation()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
