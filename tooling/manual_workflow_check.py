from __future__ import annotations

from pathlib import Path

import yaml


class 重複拒否読取器(yaml.BaseLoader):
    """起動名を真偽値に変換せず、重複鍵を拒否する開発専用読取器。"""

    def construct_object(self, node, deep=False):
        if node.tag not in {
            "tag:yaml.org,2002:str", "tag:yaml.org,2002:seq", "tag:yaml.org,2002:map"
        }:
            raise ValueError("workflow の独自タグは許可しない")
        return super().construct_object(node, deep=deep)

    def construct_mapping(self, node, deep=False):
        対応 = {}
        for 鍵節点, 値節点 in node.value:
            if not isinstance(鍵節点, yaml.ScalarNode):
                raise ValueError("workflow の鍵は文字列に限る")
            鍵 = self.construct_object(鍵節点, deep=deep)
            if 鍵 in 対応 or 鍵 == "<<":
                raise ValueError("workflow の重複鍵・合成鍵は許可しない")
            対応[鍵] = self.construct_object(値節点, deep=deep)
        return 対応


def 手動起動検査(本文: str) -> list[str]:
    """CONFIG 証拠: 起動条件だけを検査し、実行結果や権限を保証しない。"""
    try:
        文書 = yaml.load(本文, Loader=重複拒否読取器)
    except (yaml.YAMLError, ValueError, RecursionError) as 例外:
        return [f"workflow の YAML を解釈できない: {type(例外).__name__}"]
    if not isinstance(文書, dict):
        return ["workflow は対応形式でなければならない"]
    起動 = 文書.get("on")
    if 起動 == "workflow_dispatch" or 起動 == ["workflow_dispatch"]:
        return []
    if isinstance(起動, dict) and set(起動) == {"workflow_dispatch"}:
        設定 = 起動["workflow_dispatch"]
        if 設定 in ("", "null", "~") or isinstance(設定, dict):
            return []
    return ["workflow の起動は workflow_dispatch のみ許可する"]


def 手動補助一覧検査(ルート: Path) -> list[str]:
    不整合 = []
    ディレクトリ = ルート / ".github" / "workflows"
    if ディレクトリ.is_symlink():
        return ["workflow ディレクトリの外部参照は許可しない"]
    for ファイル in sorted(ディレクトリ.glob("*")):
        if ファイル.suffix.lower() not in {".yml", ".yaml"}:
            continue
        if ファイル.is_symlink() or not ファイル.is_file():
            不整合.append(f"workflow は通常ファイルに限る: {ファイル.name}")
            continue
        try:
            本文 = ファイル.read_text(encoding="utf-8")
            if len(本文) > 65536:
                不整合.append(f"workflow の検査上限を超過: {ファイル.name}")
                continue
            不整合.extend(f"{ファイル.name}: {理由}" for 理由 in 手動起動検査(本文))
        except (OSError, UnicodeError) as 例外:
            不整合.append(f"workflow を読めない: {ファイル.name}: {type(例外).__name__}")
    return 不整合
