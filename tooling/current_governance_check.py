"""現行採用の三点同一性・内部参照・要求対応を検査する。製品完成は判定しない。"""
from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path, PurePosixPath


ROOT = Path(__file__).resolve().parents[1]
INDEX_PATH = "規定/正本索引.json"
CONTINUING_ACCEPTANCE_LEDGER = "docs/QE_2026-11-14/進捗.md"
CURRENT_ADOPTION_ID = "D4Pocket-v9-NLC06-20261122"
ROLES = {"製品仕様": "仕様", "統合工程": "工程", "施工指示": "施工"}
REFERENCE_FIELDS = {
    "意味参照", "構成参照", "根拠参照", "来歴参照", "参考参照", "対応参照",
    "要求参照", "工程参照", "施工参照", "検査参照", "対象参照", "出所参照",
}
COMMON_FIELDS = {
    "保存表現", "内容規格", "原資料位置", "原資料参照", "名前空間", "名称", "固定版",
    "基底言語", "定義", "対象", "意味参照", "意味正本", "採用状態", "操作入力", "改訂日",
    "時間基準", "根拠時点", "構成参照", "機能", "注記", "版", "用語", "確認範囲",
    "禁止事項", "表記契約", "規定", "規範強度", "観測", "観測区分", "解釈投入表現",
    "読取単位", "識別", "適用域", "適用条件",
}
FUNCTIONS = {"宣言", "定義", "規則", "索引", "観測", "監査所見", "履歴", "採用判断"}
STRENGTHS = {"必須", "禁止", "許可", "推奨"}
FIELD_TYPES = {"文字列": str, "数値": int, "配列": list, "対応表": dict}
WORK_COUNTS = {"A": 7, "B": 6, "C": 10, "D": 4, "E": 4, "F": 7, "G": 3, "H": 4, "I": 5, "J": 4}
WORK_IDS = {f"W-{family}{index:02}" for family, count in WORK_COUNTS.items() for index in range(1, count + 1)}
TEST_IDS = {f"検査/Q{group}-{index:02}" for group, count in ((1, 23), (2, 24), (3, 24)) for index in range(1, count + 1)}


class Invalid(ValueError):
    """採用・構造・束縛の不適合を表す。"""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise Invalid(message)


def unique_pairs(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        require(key not in result, f"JSONキー重複: {key}")
        result[key] = value
    return result


def reject_constant(value: str) -> None:
    raise Invalid(f"JSON外の数値: {value}")


def parse_json(text: str) -> object:
    try:
        return json.loads(text, object_pairs_hook=unique_pairs, parse_constant=reject_constant)
    except (ValueError, TypeError) as error:
        raise Invalid(f"JSON不適合: {error}") from error


def canonical(value: object) -> str:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def nonimplicit(value: object) -> None:
    require(value is not None and type(value) is not bool, "null/真偽値は未規定の暗黙値")
    if type(value) is str:
        require(bool(value), "空文字列")
    elif type(value) is dict:
        require(bool(value), "空の対応表")
        for key, item in value.items():
            require(type(key) is str and bool(key), "対応表キーの型/空値")
            nonimplicit(item)
    elif type(value) is list:
        for item in value:
            nonimplicit(item)
    else:
        require(type(value) is int and value >= 0, "未規定の値型")


def condition(value: object) -> None:
    require(type(value) is dict and len(value) == 1, "条件式は一演算")
    operator, argument = next(iter(value.items()))
    if operator == "判定":
        require(type(argument) is str and bool(argument), "判定文が不正")
    elif operator in {"全て", "いずれか"}:
        require(type(argument) is list and bool(argument), "空の量化条件")
        for item in argument:
            condition(item)
    elif operator == "否定":
        condition(argument)
    else:
        raise Invalid(f"未知条件演算: {operator}")


def validate_nlc(raw: bytes) -> dict[str, dict]:
    """外部文書を復号条件にせず一文書のJSONL構造と内部参照を検査する。"""
    require(raw and not raw.startswith(b"\xef\xbb\xbf"), "空文書/BOM")
    require(b"\r" not in raw and raw.endswith(b"\n"), "LF/最終LF不適合")
    try:
        lines = raw.decode("utf-8").split("\n")[:-1]
    except UnicodeDecodeError as error:
        raise Invalid("UTF-8不適合") from error
    by: dict[str, dict] = {}
    for number, line in enumerate(lines, 1):
        require(bool(line.strip()), f"空の物理行: {number}")
        row = parse_json(line)
        require(type(row) is dict, f"行{number}が対応表でない")
        ident = row.get("識別")
        require(type(ident) is str and bool(ident), f"行{number}の識別が不正")
        require(ident not in by, f"識別重複: {ident}")
        by[ident] = row
    field_types = {field: str for field in COMMON_FIELDS}
    field_types.update({"意味参照": list, "構成参照": list, "禁止事項": list, "適用条件": dict})
    instruction = "施工読取/欄契約" in by
    if instruction:
        definitions = by["施工読取/欄契約"].get("欄定義")
        require(type(definitions) is list and bool(definitions), "施工欄契約なし")
        field_types = {}
        for field in definitions:
            require(type(field) is dict and set(field) == {"欄名", "型", "意味"}, "欄定義構造")
            require(type(field["欄名"]) is str and field["欄名"] not in field_types, "欄定義重複/型")
            require(type(field["型"]) is list and len(field["型"]) == 1 and field["型"][0] in FIELD_TYPES, "欄定義型")
            field_types[field["欄名"]] = FIELD_TYPES[field["型"][0]]
        require(REFERENCE_FIELDS <= field_types.keys(), "参照欄型の宣言欠落")
    for ident, row in by.items():
        nonimplicit(row)
        require(set(row) <= field_types.keys(), f"{ident}: 未知構造欄")
        require({"識別", "機能", "適用域"} <= row.keys(), f"{ident}: 共通必須欄不足")
        for field, value in row.items():
            require(type(value) is field_types[field], f"{ident}/{field}: 型不一致")
        require(row["機能"] in FUNCTIONS, f"{ident}: 未知機能")
        if row["機能"] == "規則":
            require({"規範強度", "適用条件", "対象", "規定"} <= row.keys(), f"{ident}: 規則必須欄不足")
            require(row["規範強度"] in STRENGTHS, f"{ident}: 規範強度不正")
            condition(row["適用条件"])
        if row["機能"] == "定義":
            require({"用語", "定義"} <= row.keys(), f"{ident}: 定義必須欄不足")
        for field in REFERENCE_FIELDS & row.keys():
            refs = row[field]
            require(type(refs) is list and all(type(item) is str for item in refs), f"{ident}: 参照型")
            require(len(set(refs)) == len(refs), f"{ident}: 重複参照")
            require(set(refs) <= by.keys(), f"{ident}: 未解決参照")
        if "識別対応" in row:
            require(all(type(item) is str and item in by for item in row["識別対応"].values()), f"{ident}: 対応先未解決")
    declarations = [row for row in by.values() if row["機能"] == "宣言"]
    require(len(declarations) == 1 and declarations[0]["識別"] == "文書/宣言", "文書宣言の一意性")
    require(declarations[0].get("基底言語") == "日本語", "日本語基底でない")
    require(declarations[0].get("内容規格") == "自然言語コード/上位規則/0.4", "NLC版不一致")
    full_index = "索引/施工指示全文" if instruction else "索引/全文"
    require(full_index in by, "全文索引欠落")
    require(set(by[full_index].get("構成参照", [])) == set(by) - {full_index}, "全文索引不一致")
    colors: dict[str, int] = {}

    def visit(ident: str) -> None:
        if colors.get(ident) == 2:
            return
        require(colors.get(ident) != 1, f"意味参照の循環: {ident}")
        colors[ident] = 1
        for target in by[ident].get("意味参照", []):
            visit(target)
        colors[ident] = 2

    for ident in by:
        visit(ident)
    return by


def validate_bindings(documents: dict[str, dict[str, dict]]) -> None:
    """確定入力の意味欄・内部参照・明示変換履歴と施工対応を別々に照合する。"""
    require(set(documents) == set(ROLES), "三点の役割が不一致")
    out = documents["施工指示"]
    maps = {}
    for role in ("製品仕様", "統合工程"):
        kind = ROLES[role]
        mapping = {}
        for row in out.values():
            if row["識別"].startswith(f"対応/入力/{kind}/"):
                for old, new in row["識別対応"].items():
                    require(old not in mapping, f"{kind}: 入力対応重複")
                    mapping[old] = new
        originals = documents[role]
        require(set(mapping) == set(originals), f"{kind}: 入力全数対応不足")
        maps[role] = mapping
        for ident, original in originals.items():
            target = out[mapping[ident]]
            origins = [item for item in target.get("由来", []) if item.get("資料") == f"入力/{kind}" and item.get("元識別") == ident]
            require(len(origins) == 1, f"{kind}/{ident}: 由来の一意性")
            require(origins[0].get("原記録SHA256") == digest(canonical(original).encode("utf-8")), f"{kind}/{ident}: 原記録ハッシュ不一致")
            allowed = set()
            if ident in {"読取/現在", "読取/採否", "履歴ID/D4-AUD-F07"}:
                allowed = {"規定"}
            if ident.startswith("出荷阻害/"):
                allowed = {"規定", "根拠時点"}
            if (kind, ident) in {("仕様", "要求/N06"), ("仕様", "製品/Release"), ("工程", "工程/E0"), ("工程", "統治/R10")}:
                allowed = {"規定"}
            if kind == "工程" and ident in {"受入/Q5", "受入/Q7"}:
                allowed = {"適用域"}
            if ident == "文書/宣言":
                require(target.get("機能") == "観測" and target.get("適用域") == "説明資料", f"{kind}: 旧宣言の効力混入")
                require(parse_json(target.get("原文", "")) == original, f"{kind}: 旧宣言の引用相違")
            else:
                fields = (set(original) | set(target)) - REFERENCE_FIELDS - {"識別", "由来"}
                changed = {field for field in fields if original.get(field) != target.get(field)}
                require(changed == allowed, f"{kind}/{ident}: 無断意味変更")
                extras = {}
                if (kind, ident) == ("仕様", "要求/N06"):
                    extras = {"意味参照": ["具体化/Export選択集合"]}
                elif (kind, ident) == ("工程", "工程/E0"):
                    extras = {"意味参照": ["施工/開始固定点"]}
                elif kind == "工程" and ident in {"受入/Q5", "受入/Q7"}:
                    extras = {"意味参照": ["具体化/受入段階"]}
                for field in REFERENCE_FIELDS & (original.keys() | target.keys()):
                    expected = [mapping[item] for item in original.get(field, [])] + extras.get(field, [])
                    require(target.get(field, []) == expected, f"{kind}/{ident}: 内部参照の誤束縛")
            if allowed or ident == "文書/宣言":
                history = [row for row in out.values() if row["識別"].startswith("変換履歴/") and row.get("対象") == f"{kind}/{ident}"]
                require(len(history) == 1, f"{kind}/{ident}: 変換履歴不足/重複")
                require(parse_json(history[0].get("原文", "")) == original, f"{kind}/{ident}: 変換前原文不一致")
                require(history[0].get("対応参照") == [mapping[ident]], f"{kind}/{ident}: 変換先不一致")
    spec = documents["製品仕様"]
    plan = documents["統合工程"]
    for source in (spec, plan):
        require({ident.removeprefix("結線/") for ident in source if ident.startswith("結線/")} == WORK_IDS, "原54結線の母集団不一致")
        require({ident for ident in source if ident.startswith("検査/")} == TEST_IDS, "原71検査の母集団不一致")
    require({ident.removeprefix("施工/") for ident in out if ident.startswith("施工/W-")} == WORK_IDS, "54施工線不足")
    require({ident.removeprefix("実施/") for ident in out if ident.startswith("実施/検査/")} == TEST_IDS, "71検査手順不足")
    require({ident for ident in out if ident.startswith("施工工程/")} == {f"施工工程/E{index}" for index in range(7)}, "七工程不足")
    require({ident for ident in out if ident.startswith("実施/OS/")} == {f"実施/OS/{name}" for name in ("Windows", "macOS", "Linux", "Android", "iOS")}, "五OS手順不足")
    requirements = {ident for ident in spec if ident.split("/")[0] in {"要求", "履歴ID", "受入", "製品", "出荷阻害"}}
    require(len(requirements) == 257 and {ident.removeprefix("対応/要件/") for ident in out if ident.startswith("対応/要件/")} == requirements, "257要件対応不足")
    for ident in requirements:
        require(out["対応/要件/" + ident].get("要求参照") == [maps["製品仕様"][ident]], f"{ident}: 要件対応誤束縛")
    operations = {ident for ident in out if ident.startswith("操作/")}
    require(len(operations) == 46, "46具体操作の母集団不一致")
    used_operations = set()
    for work in WORK_IDS:
        row = out["施工/" + work]
        require({"主体", "必要入力", "操作列", "受渡し", "保持条件", "完了条件", "停止条件", "再開条件", "工程別判定", "施工先", "拒否境界"} <= row.keys(), f"{work}: 施工契約不足")
        require(len(row["操作列"]) >= 3 and row["施工先"], f"{work}: 施工手順/対象不足")
        require(set(row["工程別判定"]) == {f"E{index}" for index in range(1, 7)}, f"{work}: 工程別判定不足")
        require("禁止事項" not in row, f"{work}: 拒否境界の再否定")
        match = re.fullmatch(r"本番経路=(.*?)。成立=(.*?)。負例=(.*?)。Producer/Contract/Broker権限/Native/Consumer/Effect/Audit/Recoveryを現在Productで欠落なく結合し、各境界を独立照合する。", spec["結線/" + work]["規定"])
        require(match is not None, f"{work}: 原結線の構造相違")
        chain, goal, negative = match.groups()
        require(row["対象"] == chain and row["目的"] == goal and row["拒否境界"] == [negative], f"{work}: 経路/目的/負例の意味相違")
        require(negative not in row["停止条件"], f"{work}: 正常な拒否を停止へ転記")
        require(maps["製品仕様"]["結線/" + work] in row["要求参照"] and maps["統合工程"]["結線/" + work] in row["工程参照"], f"{work}: 原結線束縛不足")
        used_operations.update(row["施工参照"])
    require(used_operations == operations, "到達しない具体操作/不正な操作参照")
    for ident in TEST_IDS:
        row = out["実施/" + ident]
        require(row["対象"] == spec[ident]["規定"], f"{ident}: 原検査条件相違")
        require(row["要求参照"] == [maps["製品仕様"][ident]] and row["工程参照"] == [maps["統合工程"][ident]], f"{ident}: 検査の入力束縛相違")
    require(out[maps["統合工程"]["受入/Q5"]]["適用域"] == "五OS/E5", "Q5工程束縛相違")
    require(out[maps["統合工程"]["受入/Q7"]]["適用域"] == "出荷審査E6", "Q7工程束縛相違")
    require(sum(ident.startswith("対応/監査/") for ident in out) == 28, "監査帰還対応不足")


def repository_file(root: Path, relative: object) -> Path:
    require(type(relative) is str and bool(relative) and "\\" not in relative, "相対pathの型/区切り")
    pure = PurePosixPath(relative)
    require(not pure.is_absolute() and not any(part in {".", ".."} for part in relative.split("/")), "相対path逸脱")
    path = root / relative
    require(path.is_file() and path.resolve().is_relative_to(root.resolve()), f"対象fileなし/範囲外: {relative}")
    return path


def governance_errors(root: Path = ROOT) -> list[str]:
    try:
        index = parse_json((root / INDEX_PATH).read_text(encoding="utf-8"))
        require(type(index) is dict, "正本索引の型")
        adoption = index.get("現行施工採用")
        require(type(adoption) is dict and adoption.get("識別") == CURRENT_ADOPTION_ID, "現行採用接続なし/版不一致")
        require(adoption.get("継続受入証拠台帳") == CONTINUING_ACCEPTANCE_LEDGER, "継続受入証拠台帳の接続不一致")
        adoption_path = repository_file(root, adoption.get("採用記録"))
        adopted_text = adoption_path.read_text(encoding="utf-8")
        agents = repository_file(root, "AGENTS.md").read_text(encoding="utf-8")
        repository_file(root, adoption.get("継続施工記録"))
        repository_file(root, adoption.get("継続受入証拠台帳"))
        require(adoption_path.name in agents and adoption_path.parent.as_posix().endswith("docs/QE_2026-11-22"), "AGENTS採用記録接続不一致")
        entries = adoption.get("三点")
        require(type(entries) is list and len(entries) == 3, "三点の件数不一致")
        require({entry.get("役割") for entry in entries if type(entry) is dict} == set(ROLES), "三点の役割重複/不足")
        require(len({entry["path"] for entry in entries}) == 3, "三点path重複")
        current_paths = {entry.get("path") for entry in index.get("現行正本", []) if type(entry) is dict}
        require(adoption["採用記録"] in current_paths, "採用記録の索引登録不足")
        documents = {}
        for entry in entries:
            role = entry["役割"]
            path = repository_file(root, entry.get("path"))
            raw = path.read_bytes()
            require(type(entry.get("SHA256")) is str and digest(raw) == entry["SHA256"], f"{role}: 受領SHA-256不一致")
            require(type(entry.get("バイト数")) is int and len(raw) == entry["バイト数"], f"{role}: バイト数不一致")
            by = validate_nlc(raw)
            require(type(entry.get("記録数")) is int and len(by) == entry["記録数"], f"{role}: 記録数不一致")
            require(by["文書/宣言"].get("名前空間") == entry.get("名前空間"), f"{role}: 名前空間不一致")
            require(entry["path"] in current_paths and entry["path"] in agents, f"{role}: 索引/AGENTS接続不足")
            require(path.name in adopted_text and entry["SHA256"] in adopted_text, f"{role}: 採用記録との同一性不足")
            documents[role] = by
        validate_bindings(documents)
        require(set(adoption.get("明示具体化", [])) == {"具体化/Export選択集合", "具体化/受入段階", "施工読取/採用と時点", "具体化/開発検証分離"}, "明示具体化の採用接続不足")
        for ident in adoption["明示具体化"]:
            require(ident in documents["施工指示"], f"採用した明示具体化なし: {ident}")
        for role, kind in (("製品仕様", "仕様"), ("統合工程", "工程")):
            entry = next(item for item in entries if item["役割"] == role)
            require(documents["施工指示"]["入力/" + kind]["ハッシュ値"] == entry["SHA256"], f"{role}: 施工指示入力ハッシュ不一致")
    except (OSError, UnicodeError, Invalid, KeyError, TypeError, RecursionError) as error:
        return [f"現行三点整合: {error}"]
    return []


def main() -> int:
    errors = governance_errors()
    if errors:
        print("現行三点整合検査が失敗:")
        for error in errors:
            print(f"  - {error}")
        return 1
    print("現行三点整合検査が合格: 438/289/1453記録、54施工線、46操作、71検査、257要件対応。NLC06の開発検証/製品Owner境界を確認。製品QA・出荷GOは未判定。")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
