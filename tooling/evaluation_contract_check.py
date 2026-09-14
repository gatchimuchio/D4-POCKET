"""C5評価ラボのdevelopment-only関係検査。

このmoduleは公開projectionとowner登録payloadの構造関係を検査するだけである。
Authority、Permission、Approval、Auditの有効性、runtime実行、release/security判断を生成しない。
"""

from __future__ import annotations

import math
import re
from typing import Any


公開禁止field = frozenset(
    {
        "入力",
        "本文",
        "期待本文",
        "期待断片",
        "パターン",
        "期待Schema",
        "設定",
        "非公開CasePayload一覧",
        "raw",
        "raw_input",
        "expected_body",
        "regex",
        "json_schema",
        "secret",
        "credential",
        "token",
        "endpoint",
        "command",
        "argv",
        "env",
    }
)

評価用途に不許可のfield = frozenset(
    {
        "Authority",
        "authority",
        "Permission",
        "permission",
        "Approval",
        "approval",
        "release",
        "security_decision",
        "権限ID",
        "承認ID",
        "release可否",
        "security判断",
    }
)

評価器設定field = {
    "exact": frozenset({"期待本文"}),
    "contains": frozenset({"期待断片"}),
    "regex": frozenset({"パターン"}),
    "json_schema": frozenset({"期待Schema"}),
    "reference_count": frozenset({"最小数", "最大数"}),
    "route": frozenset({"期待経路"}),
    "status": frozenset({"期待状態"}),
    "capability": frozenset({"必要能力一覧"}),
    "latency_threshold": frozenset({"最大Millis"}),
}


def Dataset改版列検査(records: object) -> list[str]:
    """時間順のDataset manifestまたはowner登録について改版の単調性を検査する。

    JSON Schemaは一つのrecordだけを検査するため、既存の永続監査recordとの
    revision比較はここで行う。呼出側は検証済みのaccepted recordを古い順に渡す。
    同じ評価DatasetIDでは、revisionは厳密に増加しなければならない。
    """

    if not isinstance(records, list):
        return ["Dataset改版列がarrayではない"]

    errors: list[str] = []
    latest_revision: dict[str, int] = {}
    for index, record in enumerate(records):
        path = f"$[{index}]"
        if not isinstance(record, dict):
            errors.append(f"{path}がobjectではない")
            continue
        dataset_id = record.get("評価DatasetID")
        revision = record.get("revision")
        if not isinstance(dataset_id, str) or not dataset_id:
            errors.append(f"{path}.評価DatasetIDが不正")
            continue
        if not isinstance(revision, int) or isinstance(revision, bool) or revision < 1:
            errors.append(f"{path}.revisionが不正")
            continue
        previous = latest_revision.get(dataset_id)
        if previous is not None:
            if revision == previous:
                errors.append(f"{path}の評価DatasetID/revisionが既存recordと重複する")
            elif revision < previous:
                errors.append(f"{path}のrevisionが同一評価DatasetIDの確定revisionより後退している")
        if previous is None or revision > previous:
            latest_revision[dataset_id] = revision
    return errors


def _field_paths(value: Any, fields: frozenset[str], path: str = "$") -> list[str]:
    paths: list[str] = []
    if isinstance(value, dict):
        for key, item in value.items():
            item_path = f"{path}.{key}"
            if key in fields:
                paths.append(item_path)
            paths.extend(_field_paths(item, fields, item_path))
    elif isinstance(value, list):
        for index, item in enumerate(value):
            paths.extend(_field_paths(item, fields, f"{path}[{index}]"))
    return paths


def _external_ref_paths(value: Any, path: str = "$") -> list[str]:
    paths: list[str] = []
    if isinstance(value, dict):
        for key, item in value.items():
            item_path = f"{path}.{key}"
            if key == "$ref" and (not isinstance(item, str) or not item.startswith("#")):
                paths.append(item_path)
            paths.extend(_external_ref_paths(item, item_path))
    elif isinstance(value, list):
        for index, item in enumerate(value):
            paths.extend(_external_ref_paths(item, f"{path}[{index}]"))
    return paths


def 公開projection検査(*projection: object) -> list[str]:
    """公開Dataset/Case/Evaluator/Experiment/Result/Comparisonのraw fieldを拒否する。"""

    errors: list[str] = []
    for index, value in enumerate(projection):
        for path in _field_paths(value, 公開禁止field):
            errors.append(f"公開projection[{index}]にrawまたは秘密fieldがある: {path}")
    return errors


def 登録検査(registration: object) -> list[str]:
    """owner登録payloadが決定論の運用観測に限定されることを検査する。"""

    errors: list[str] = []
    if not isinstance(registration, dict):
        return ["owner登録payloadがobjectではない"]
    if registration.get("評価用途") != "運用観測":
        errors.append("評価用途が運用観測に固定されていない")
    method = registration.get("評価方式")
    if method != "決定論":
        errors.append("評価方式が決定論に固定されていない")
    if isinstance(method, str) and "llm" in method.casefold():
        errors.append("LLM-as-judgeをAuthority用途または決定論の代替にできない")
    for path in _field_paths(registration, 評価用途に不許可のfield):
        errors.append(f"評価登録にAuthority/Permission/Approval/release fieldがある: {path}")

    cases = registration.get("非公開CasePayload一覧")
    if not isinstance(cases, list):
        return errors
    case_ids: set[object] = set()
    case_orders: set[object] = set()
    for case_index, case in enumerate(cases):
        case_path = f"$.非公開CasePayload一覧[{case_index}]"
        if not isinstance(case, dict):
            errors.append(f"{case_path}がobjectではない")
            continue
        case_id = case.get("評価CaseID")
        if case_id in case_ids:
            errors.append(f"{case_path}の評価CaseIDが重複する")
        case_ids.add(case_id)
        order = case.get("順序")
        if order in case_orders:
            errors.append(f"{case_path}の順序が重複する")
        case_orders.add(order)
        evaluators = case.get("評価器設定")
        if not isinstance(evaluators, list):
            errors.append(f"{case_path}.評価器設定がarrayではない")
            continue
        evaluator_ids: set[object] = set()
        for evaluator_index, evaluator in enumerate(evaluators):
            evaluator_path = f"{case_path}.評価器設定[{evaluator_index}]"
            if not isinstance(evaluator, dict):
                errors.append(f"{evaluator_path}がobjectではない")
                continue
            evaluator_id = evaluator.get("評価器ID")
            if evaluator_id in evaluator_ids:
                errors.append(f"{evaluator_path}の評価器IDが重複する")
            evaluator_ids.add(evaluator_id)
            kind = evaluator.get("種類")
            config = evaluator.get("設定")
            expected_fields = 評価器設定field.get(kind)
            if expected_fields is None:
                errors.append(f"{evaluator_path}の評価器種類が未対応")
                continue
            if not isinstance(config, dict) or set(config) != expected_fields:
                errors.append(f"{evaluator_path}の設定が種類{kind!r}と一致しない")
                continue
            if kind == "reference_count":
                minimum = config.get("最小数")
                maximum = config.get("最大数")
                if (
                    not isinstance(minimum, int)
                    or isinstance(minimum, bool)
                    or not isinstance(maximum, int)
                    or isinstance(maximum, bool)
                    or minimum > maximum
                ):
                    errors.append(f"{evaluator_path}の参照数上下限が不正")
            if kind == "latency_threshold":
                latency = config.get("最大Millis")
                if (
                    not isinstance(latency, int)
                    or isinstance(latency, bool)
                    or latency < 0
                    or latency > 86400000
                ):
                    errors.append(f"{evaluator_path}の最大Millisが不正")
            if kind == "json_schema":
                for path in _external_ref_paths(config.get("期待Schema")):
                    errors.append(f"{evaluator_path}のJSON Schemaが外部refを含む: {path}")
    return errors


def 実験検査(experiment: object) -> list[str]:
    """Experimentの公開projection、計画監査相関、Runtime一意性を検査する。"""

    errors = 公開projection検査(experiment)
    if not isinstance(experiment, dict):
        return [*errors, "Experimentがobjectではない"]
    runtimes = experiment.get("対象Runtime一覧")
    if not isinstance(runtimes, list):
        return [*errors, "対象Runtime一覧がarrayではない"]
    planned_case_count = experiment.get("計画Case数")
    result_count = experiment.get("結果数")
    state = experiment.get("状態")
    if (
        not isinstance(planned_case_count, int)
        or isinstance(planned_case_count, bool)
        or planned_case_count < 1
        or not isinstance(result_count, int)
        or isinstance(result_count, bool)
        or result_count < 0
    ):
        errors.append("Experimentの計画Case数または結果数が不正")
    else:
        expected_result_count = planned_case_count * len(runtimes)
        if result_count > expected_result_count:
            errors.append("Experimentの結果数がCase×Runtime計画数を超える")
        if state == "完了":
            if result_count != expected_result_count:
                errors.append("完了Experimentの結果数がCase×Runtime計画数と一致しない")
            for field in ("開始時刻UnixMillis", "終了時刻UnixMillis"):
                timestamp = experiment.get(field)
                if (
                    not isinstance(timestamp, int)
                    or isinstance(timestamp, bool)
                    or timestamp < 0
                ):
                    errors.append(f"完了Experimentの{field}がない")
    plan_audit_id = experiment.get("計画監査ID")
    if not isinstance(plan_audit_id, str) or not plan_audit_id:
        errors.append("計画監査IDがない")
    started_at = experiment.get("開始時刻UnixMillis")
    finished_at = experiment.get("終了時刻UnixMillis")
    if (
        isinstance(started_at, int)
        and not isinstance(started_at, bool)
        and isinstance(finished_at, int)
        and not isinstance(finished_at, bool)
        and finished_at < started_at
    ):
        errors.append("Experimentの終了時刻が開始時刻より前である")
    seen: set[str] = set()
    for index, runtime in enumerate(runtimes):
        if not isinstance(runtime, str) or not runtime:
            errors.append(f"対象Runtime一覧[{index}]の実行系IDが文字列ではない")
            continue
        if runtime in seen:
            errors.append(f"対象Runtime一覧[{index}]の実行系IDが重複する")
        seen.add(runtime)
    return errors


def 結果検査(result: object) -> list[str]:
    """Resultの公開projection、latency、Evaluator相関を検査する。"""

    errors = 公開projection検査(result)
    if not isinstance(result, dict):
        return [*errors, "Resultがobjectではない"]
    latency = result.get("LatencyMillis")
    if latency is not None:
        if (
            not isinstance(latency, int)
            or isinstance(latency, bool)
            or not math.isfinite(float(latency))
            or latency < 0
        ):
            errors.append("LatencyMillisは非負の有限integerまたはnullでなければならない")
    evaluators = result.get("評価器判定一覧")
    if isinstance(evaluators, list):
        seen: set[object] = set()
        for index, evaluator in enumerate(evaluators):
            evaluator_id = evaluator.get("評価器ID") if isinstance(evaluator, dict) else None
            if evaluator_id in seen:
                errors.append(f"評価器判定一覧[{index}]の評価器IDが重複する")
            seen.add(evaluator_id)
    status = result.get("結果状態")
    determination = result.get("判定")
    if status == "中止" and determination != "中断":
        errors.append("中止結果は中断判定でなければならない")
    if status == "評価不能" and determination != "評価不能":
        errors.append("評価不能結果は評価不能判定でなければならない")
    if status == "成功":
        for field in ("開始監査ID", "終了監査ID", "応答hash"):
            value = result.get(field)
            if not isinstance(value, str) or not value:
                errors.append(f"成功結果の{field}がない")
    quarantine_reservation = result.get("実行系隔離予約監査ID")
    if status != "中止" and quarantine_reservation is not None:
        errors.append("中止以外のResultは実行系隔離予約監査IDをnullにしなければならない")
    if quarantine_reservation is not None and (
        not isinstance(quarantine_reservation, str)
        or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.:-]{0,255}", quarantine_reservation) is None
    ):
        errors.append("実行系隔離予約監査IDはnullまたは監査IDでなければならない")
    response_hash = result.get("応答hash")
    if response_hash is not None and (
        not isinstance(response_hash, str)
        or re.fullmatch(r"sha256:[a-f0-9]{64}", response_hash) is None
    ):
        errors.append("応答hashはnullまたはtagged SHA-256でなければならない")
    return errors


def 公開結果検査(result: object) -> list[str]:
    """通常IPCへ返すResult要約が内部監査fieldを持たないことを検査する。"""

    errors = 公開projection検査(result)
    if not isinstance(result, dict):
        return [*errors, "公開Resultがobjectではない"]
    forbidden = {
        "結果状態",
        "応答hash",
        "開始監査ID",
        "終了監査ID",
        "実行系隔離予約監査ID",
        "作成監査ID",
        "対話要求ID",
        "対話セッションID",
        "証拠種別",
    }
    for field in sorted(forbidden.intersection(result)):
        errors.append(f"公開Resultに内部fieldがある: {field}")
    evaluators = result.get("評価器判定一覧")
    if not isinstance(evaluators, list):
        return [*errors, "公開Resultの評価器判定一覧がarrayではない"]
    if len(evaluators) == 0 or len(evaluators) > 16:
        errors.append("公開Resultの評価器判定一覧件数が不正")
    seen: set[object] = set()
    for index, evaluator in enumerate(evaluators):
        evaluator_id = evaluator.get("評価器ID") if isinstance(evaluator, dict) else None
        if evaluator_id in seen:
            errors.append(f"公開Resultの評価器判定一覧[{index}]の評価器IDが重複する")
        seen.add(evaluator_id)
    latency = result.get("LatencyMillis")
    if latency is not None and (
        not isinstance(latency, int)
        or isinstance(latency, bool)
        or latency < 0
    ):
        errors.append("公開ResultのLatencyMillisが不正")
    return errors


def 比較検査(comparison: object) -> list[str]:
    """一つのExperimentに対するComparisonのDataset、Runtime、latencyを検査する。"""

    errors = 公開projection検査(comparison)
    if not isinstance(comparison, dict):
        return [*errors, "Comparisonがobjectではない"]
    dataset_hash = comparison.get("Dataset定義hash")
    experiments = comparison.get("実験一覧")
    if not isinstance(experiments, list):
        return [*errors, "実験一覧がarrayではない"]
    planned_case_count = comparison.get("計画Case数")
    if (
        not isinstance(planned_case_count, int)
        or isinstance(planned_case_count, bool)
        or planned_case_count < 1
    ):
        errors.append("Comparisonの計画Case数が不正")
        planned_case_count = None
    count_fields = ("成立数", "不成立数", "評価不能数", "中断数")
    entry_totals = {field: 0 for field in count_fields}
    runtimes: set[str] = set()
    experiment_id: object | None = None
    experiment_id_seen = False
    for index, experiment in enumerate(experiments):
        path = f"$.実験一覧[{index}]"
        if not isinstance(experiment, dict):
            errors.append(f"{path}がobjectではない")
            continue
        if experiment.get("Dataset定義hash") != dataset_hash:
            errors.append(f"{path}のDataset定義hashが比較対象と一致しない")
        entry_experiment_id = experiment.get("評価ExperimentID")
        if experiment_id_seen and entry_experiment_id != experiment_id:
            errors.append(f"{path}の評価ExperimentIDが一つの比較Experimentと一致しない")
        else:
            experiment_id = entry_experiment_id
            experiment_id_seen = True
        runtime = experiment.get("実行系ID")
        if not isinstance(runtime, str) or not runtime:
            errors.append(f"{path}の実行系IDが文字列ではない")
            runtime = None
        if runtime is None:
            continue
        if runtime in runtimes:
            errors.append(f"{path}の実行系IDが重複する")
        runtimes.add(runtime)
        entry_count = 0
        valid_counts = True
        for field in count_fields:
            count = experiment.get(field)
            if not isinstance(count, int) or isinstance(count, bool) or count < 0:
                errors.append(f"{path}.{field}が非負integerではない")
                valid_counts = False
                continue
            entry_count += count
            entry_totals[field] += count
        if valid_counts and planned_case_count is not None and entry_count != planned_case_count:
            errors.append(f"{path}の四分類合計が計画Case数と一致しない")
        latency = experiment.get("平均LatencyMillis")
        if latency is not None and (
            not isinstance(latency, (int, float))
            or isinstance(latency, bool)
            or not math.isfinite(latency)
            or latency < 0
        ):
            errors.append(f"{path}の平均LatencyMillisが非負の有限numberまたはnullではない")
    for field in count_fields:
        root_count = comparison.get(field)
        if not isinstance(root_count, int) or isinstance(root_count, bool) or root_count < 0:
            errors.append(f"Comparisonの{field}が非負integerではない")
        elif root_count != entry_totals[field]:
            errors.append(f"Comparisonの{field}がRuntime entry合計と一致しない")
    return errors
