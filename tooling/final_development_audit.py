"""C32の開発監査対応表を検査する。

このtoolは静的な対応表の完全性だけを検査し、Runtime、Agent、端末、権限、
Approvalを起動しない。終了値0は対応表の成立を示すだけで、release_readyを意味しない。
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
AUDIT_PATH = ROOT / "docs" / "specs" / "final-development-audit.json"
RECEIPT_PATH = ROOT / "docs" / "総合機能拡張_rev1" / "受領記録.json"
PROGRESS_PATH = ROOT / "docs" / "総合機能拡張_rev1" / "進捗.md"
REQUIRED_FIELDS = [
    "意味正本",
    "Contract",
    "Code",
    "Production path",
    "Test",
    "Negative",
    "Recovery",
    "Audit",
    "UI",
    "Evidence",
]
ALLOWED_CLASSIFICATIONS = {"release_blocker", "post_v1_scope", "known_limitation"}


def _load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise ValueError(f"JSONがobjectではない: {path}")
    return value


def _is_present(value: Any) -> bool:
    if value is None:
        return False
    if isinstance(value, str):
        return bool(value.strip())
    if isinstance(value, list):
        return bool(value) and all(_is_present(item) for item in value)
    if isinstance(value, dict):
        return bool(value)
    return True


def _relative_path(value: str) -> Path:
    return ROOT / value.replace("/", "\\")


def _check_reference_paths(paths: list[Any], errors: list[str], label: str) -> None:
    for raw_path in paths:
        if not isinstance(raw_path, str) or not raw_path.strip():
            errors.append(f"{label}の参照pathが文字列ではない")
            continue
        path = _relative_path(raw_path)
        if not path.exists():
            errors.append(f"{label}の参照pathが存在しない: {raw_path}")


def _progress_numbers() -> set[str]:
    text = PROGRESS_PATH.read_text(encoding="utf-8")
    return {f"C{number}" for number in range(35) if f"| C{number} |" in text}


def main() -> int:
    errors: list[str] = []
    try:
        data = _load_json(AUDIT_PATH)
        receipt = _load_json(RECEIPT_PATH)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(json.dumps({"状態": "failed", "エラー": [f"監査入力を読めない: {exc}"]}, ensure_ascii=False, indent=2))
        return 1

    declared_fields = data.get("必須項目")
    if declared_fields != REQUIRED_FIELDS:
        errors.append("必須監査項目の定義が規定と一致しない")

    common = data.get("共通")
    if not isinstance(common, dict):
        errors.append("共通監査項目がobjectではない")
        common = {}
    for field in REQUIRED_FIELDS:
        if not _is_present(common.get(field)) and field in {"意味正本", "Negative", "Recovery", "Audit"}:
            errors.append(f"共通監査項目が空: {field}")

    common_refs = data.get("共通参照")
    if not isinstance(common_refs, list):
        errors.append("共通参照が配列ではない")
        common_refs = []
    _check_reference_paths(common_refs, errors, "共通参照")

    rows = data.get("工程")
    if not isinstance(rows, list):
        errors.append("工程対応表が配列ではない")
        rows = []

    expected = [f"C{number}" for number in range(32)]
    actual_numbers: list[str] = []
    results: list[dict[str, Any]] = []
    residual_counts = {classification: 0 for classification in sorted(ALLOWED_CLASSIFICATIONS)}
    for row in rows:
        if not isinstance(row, dict):
            errors.append("工程対応表にobjectでない行がある")
            continue
        number = row.get("番号")
        if not isinstance(number, str):
            errors.append("工程行の番号が文字列ではない")
            continue
        actual_numbers.append(number)
        merged = dict(common)
        merged.update(row)
        missing = [field for field in REQUIRED_FIELDS if not _is_present(merged.get(field))]
        if missing:
            errors.append(f"{number}の必須対応が空: {', '.join(missing)}")
        references = list(common_refs)
        row_refs = row.get("参照", [])
        if not isinstance(row_refs, list):
            errors.append(f"{number}の参照が配列ではない")
        else:
            references.extend(row_refs)
        _check_reference_paths(references, errors, number)

        risks = row.get("残存risk")
        if not isinstance(risks, list) or not risks:
            errors.append(f"{number}の残存riskが空")
            risks = []
        for risk in risks:
            if not isinstance(risk, dict):
                errors.append(f"{number}の残存riskがobjectではない")
                continue
            classification = risk.get("classification")
            if classification not in ALLOWED_CLASSIFICATIONS:
                errors.append(f"{number}の残存risk分類が不正: {classification}")
            else:
                residual_counts[classification] += 1
            for key in ("reason", "required_action"):
                if not _is_present(risk.get(key)):
                    errors.append(f"{number}の残存riskに{key}がない")
        results.append(
            {
                "番号": number,
                "名称": row.get("名称", ""),
                "状態": row.get("状態", ""),
                "必須項目": "成立" if not missing else "欠落",
                "残存risk件数": len(risks),
            }
        )

    if actual_numbers != expected:
        errors.append(f"C0〜C31の工程番号が一致しない: {actual_numbers}")
    if len(actual_numbers) != len(set(actual_numbers)):
        errors.append("工程番号が重複している")

    receipt_items = receipt.get("工程")
    receipt_numbers = {
        item.get("番号")
        for item in receipt_items
        if isinstance(item, dict) and isinstance(item.get("番号"), str)
    } if isinstance(receipt_items, list) else set()
    missing_receipt = [number for number in [f"C{n}" for n in range(35)] if number not in receipt_numbers]
    if missing_receipt:
        errors.append(f"受領記録に工程がない: {missing_receipt}")

    progress_numbers = _progress_numbers()
    missing_progress = [number for number in [f"C{n}" for n in range(35)] if number not in progress_numbers]
    if missing_progress:
        errors.append(f"進捗表に工程がない: {missing_progress}")

    unaudited = data.get("未監査工程")
    if unaudited != ["C32", "C33", "C34"]:
        errors.append("未監査工程はC32、C33、C34を明示しなければならない")

    report = {
        "版": 1,
        "状態": "failed" if errors else "passed",
        "監査結論": "対応表の構造が成立した。正式releaseは未成立。" if not errors else "対応表に不整合がある。該当工程を未成立として修正する。",
        "監査対象": "C0〜C31",
        "工程数": len(results),
        "未監査工程": unaudited,
        "必須項目": REQUIRED_FIELDS,
        "工程結果": results,
        "残存risk分類件数": residual_counts,
        "release_ready": False,
        "release_blocker": True,
        "raw outputを保存しない": True,
        "エラー": errors,
    }
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
