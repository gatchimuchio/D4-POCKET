"""実行系対話の開発専用契約検査。許可や承認を発行しない。"""

import hashlib

from packages.shell_contracts import load_default_catalog
from tooling.schema_check.check_schemas import validate_instance


def 構造検査(名前: str, 値: object) -> list[str]:
    return validate_instance(値, load_default_catalog().get(f"runtime_dialogue_{名前}.schema.json"))


def 要求関係検査(要求: dict, セッション: dict) -> list[str]:
    不整合 = 構造検査("request", 要求) + 構造検査("session", セッション)
    if 不整合:
        return 不整合
    if not 要求["入力"].strip():
        不整合.append("空白だけの入力は送信しない")
    for 鍵 in ("実行系ID", "対話セッションID"):
        if 要求[鍵] != セッション[鍵]:
            不整合.append(f"要求とセッションの{鍵}が一致しない")
    if セッション["状態"] != "利用中":
        不整合.append("終了・中止後隔離のセッションは再利用しない")
    return 不整合


def 応答関係検査(要求: dict, 応答: dict, 許可表示範囲: str) -> list[str]:
    不整合 = 構造検査("request", 要求) + 構造検査("response", 応答)
    if 不整合:
        return 不整合
    if not 要求["入力"].strip():
        不整合.append("空白だけの要求に対する応答を採用しない")
    for 鍵 in ("要求ID", "実行系ID", "対話セッションID"):
        if 要求[鍵] != 応答[鍵]:
            不整合.append(f"要求と応答の{鍵}が一致しない")
    if 許可表示範囲 not in {"none", "hash_only", "summary", "redacted", "full"}:
        不整合.append("表示許可が不明である")
    if 応答["表示範囲"] != 許可表示範囲:
        不整合.append("応答が指定された表示許可と異なる")
    if 許可表示範囲 != "full":
        for 鍵 in ("本文", "参照", "能力", "経路", "追跡ID", "追跡hash"):
            if 応答[鍵]:
                不整合.append(f"明示的な射影のない非全文表示へ{鍵}が漏れている")
    if 許可表示範囲 == "none" and 応答["応答hash"]:
        不整合.append("非表示の応答へhashが漏れている")
    if 応答["状態"] in {"成功", "保留"}:
        if 応答["失敗分類"] or 応答["復旧"]:
            不整合.append("正常・保留結果へ失敗分類が混入している")
    elif not 応答["失敗分類"] or not 応答["復旧"]:
        不整合.append("失敗・中止結果に失敗分類と復旧が必要である")
    if 応答["状態"] == "中止" and (応答["失敗分類"], 応答["復旧"]) != ("取消", "新規セッション"):
        不整合.append("中止後は取消としてセッションを隔離する")
    return 不整合


def 比較関係検査(比較: dict, 左要求: dict, 右要求: dict, 左応答: dict, 右応答: dict,
             左表示範囲: str, 右表示範囲: str) -> list[str]:
    不整合 = 構造検査("comparison", 比較)
    不整合 += 応答関係検査(左要求, 左応答, 左表示範囲)
    不整合 += 応答関係検査(右要求, 右応答, 右表示範囲)
    if 不整合:
        return 不整合
    for 鍵 in ("要求ID", "実行系ID", "対話セッションID"):
        if 左要求[鍵] == 右要求[鍵]:
            不整合.append(f"比較の左右で{鍵}を共有しない")
    if 左要求["入力"] != 右要求["入力"]:
        不整合.append("比較入力が一致しない")
    入力hash = "sha256:" + hashlib.sha256(左要求["入力"].encode("utf-8")).hexdigest()
    if 比較["入力hash"] != 入力hash:
        不整合.append("比較の入力hashが一致しない")
    for 側, 要求, 応答 in (("左", 左要求, 左応答), ("右", 右要求, 右応答)):
        for 鍵 in ("要求ID", "実行系ID"):
            if 比較[側 + 鍵] != 要求[鍵]:
                不整合.append(f"比較の{側}{鍵}が一致しない")
        if 比較[側 + "状態"] != 応答["状態"]:
            不整合.append(f"比較の{側}状態が一致しない")
    return 不整合
