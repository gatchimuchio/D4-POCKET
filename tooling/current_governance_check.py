"""v11五文書の採用同一性・意味参照・要求/結線/業務所管を検査。製品品質検査を実行しない。"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
INDEX_PATH = "規定/正本索引.json"
CONTINUING_ACCEPTANCE_LEDGER = "docs/QE_2026-11-14/進捗.md"
ADOPTION_ID = "D4Pocket-v11-20261124"
DIRECTORY = "docs/V11_2026-11-24"
ROLES = ("要求仕様", "設計仕様", "実装開発工程", "品質工程", "実装開発実行指示")
EXPECTED_RECORDS = (344, 320, 311, 401, 488)
WIRE_COUNTS = {"A": 7, "B": 6, "C": 10, "D": 4, "E": 4,
               "F": 7, "G": 3, "H": 4, "I": 5, "J": 4}
OS = ("Windows", "Linux", "macOS", "Android", "iOS")


class Invalid(ValueError):
    pass


def require(ok: bool, message: str) -> None:
    if not ok:
        raise Invalid(message)


def unique_object(pairs: list[tuple[str, object]]) -> dict:
    value = {}
    for key, content in pairs:
        require(key not in value, f"重複JSON key: {key}")
        value[key] = content
    return value


def reject_constant(value: str):
    raise Invalid(f"不正JSON定数: {value}")


def parse_json(raw: str):
    return json.loads(raw, object_pairs_hook=unique_object,
                      parse_constant=reject_constant)


def read_nlc(raw: bytes) -> dict[str, dict]:
    require(raw and not raw.startswith(b"\xef\xbb\xbf"), "空/BOMを拒否")
    require(b"\r" not in raw and raw.endswith(b"\n"), "LFのみ/最終LF必須")
    text = raw.decode("utf-8")
    records = {}
    for index, line in enumerate(text[:-1].split("\n"), 1):
        require(bool(line) and line == line.strip(), f"JSONL物理行: {index}")
        item = parse_json(line)
        require(type(item) is dict, f"object以外のJSONL行: {index}")
        ident = item.get("識別")
        require(type(ident) is str and bool(ident), f"識別不正: {index}")
        require(ident not in records, f"識別重複: {ident}")
        require(type(item.get("種類")) is str, f"種類欠落: {ident}")
        records[ident] = item
    for ident, item in records.items():
        refs = item.get("意味参照", [])
        require(type(refs) is list, f"意味参照の型: {ident}")
        for target in refs:
            require(type(target) is str and target in records,
                    f"文書内意味参照未解決: {ident} -> {target}")
    return records


def ids_with_prefix(records: dict, prefix: str) -> set[str]:
    return {key for key in records if key.startswith(prefix)}


def validate_population(docs: dict[str, dict[str, dict]]) -> None:
    requirements, design, implementation, quality, instructions = (docs[k] for k in ROLES)
    req = ids_with_prefix(requirements, "要求/")
    wire = ids_with_prefix(requirements, "結線/")
    require(len(req) == 188 and len(wire) == 54, "原要求188/結線54が不一致")
    require({f"設計対応/{key}" for key in req} == ids_with_prefix(design, "設計対応/"),
            "設計の188要求対応が不一致")
    require({f"施工要求/{key}" for key in req} == ids_with_prefix(implementation, "施工要求/"),
            "実装工程の188要求対応が不一致")
    require({f"品質対応/{key}" for key in req} == ids_with_prefix(quality, "品質対応/"),
            "品質工程の188要求対応が不一致")
    required_wires = {f"結線/W-{family}{i:02}" for family, count in WIRE_COUNTS.items()
                      for i in range(1, count + 1)}
    require(wire == required_wires, "W-A..J 54線の母集団が不一致")
    require({f"設計結線/{key}" for key in wire} == ids_with_prefix(design, "設計結線/"),
            "設計の54線が不一致")
    require({f"施工結線/{key}" for key in wire} == ids_with_prefix(implementation, "施工結線/"),
            "工程の54線が不一致")
    require({f"品質結線/{key}" for key in wire} == ids_with_prefix(quality, "品質結線/"),
            "品質の54線が不一致")
    require({f"施工結線詳細/{key.removeprefix('結線/')}" for key in wire}
            == ids_with_prefix(instructions, "施工結線詳細/"),
            "指示書の54線が不一致")
    construction = {key.removeprefix("要求施工/") for key in
                    ids_with_prefix(instructions, "要求施工/")}
    quality_only = {key.removeprefix("品質所管分離/") for key in
                    ids_with_prefix(instructions, "品質所管分離/")}
    require(len(construction) == 152 and len(quality_only) == 36, "実装152/品質36が不一致")
    require(construction.isdisjoint(quality_only) and construction | quality_only == req,
            "実装/品質の所管分割が不一致")
    for req_id in construction:
        require(instructions[f"要求施工/{req_id}"]["種類"] == "施工命令",
                f"実装命令の型: {req_id}")
    for req_id in quality_only:
        require(instructions[f"品質所管分離/{req_id}"]["種類"] == "所管分離",
                f"品質専用が施工命令へ変換: {req_id}")
    cases = {f"検査/Q{group}-{i:02}" for group, count in ((1, 23), (2, 24), (3, 24))
             for i in range(1, count + 1)}
    require({f"品質検査/{item}" for item in cases}
            == ids_with_prefix(quality, "品質検査/"), "品質71ケースの母集団が不一致")
    require(len(ids_with_prefix(instructions, "品質検査/")) == 0,
            "品質検査を実装指示に混入した")
    require({f"製品/OS/{name}" for name in OS}.issubset(requirements),
            "必須五OS要求の欠落")
    require({f"設計OS/{name}" for name in OS}.issubset(design),
            "必須五OS設計の欠落")
    require({f"品質OS/{name}" for name in OS}.issubset(quality),
            "必須五OS品質の欠落")
    require({f"OS実装/{name}" for name in OS}.issubset(instructions)
            and {f"OSビルド/{name}" for name in OS}.issubset(instructions),
            "五OSの実装/Builder命令欠落")
    require(len(ids_with_prefix(requirements, "出荷阻害/")) == 18
            and len(ids_with_prefix(quality, "品質阻害/")) == 18, "歴史阻害18不一致")


def validate_source_references(root: Path, docs: dict[str, dict[str, dict]]) -> None:
    """工程表と施工指示のSource候補を照合し、Repo内の実在fileへ束縛する。"""
    plan = docs["実装開発工程"]
    instructions = docs["実装開発実行指示"]
    construction_ids = {
        ident.removeprefix("要求施工/")
        for ident in ids_with_prefix(instructions, "要求施工/")
    }
    wire_ids = {
        ident.removeprefix("施工結線詳細/")
        for ident in ids_with_prefix(instructions, "施工結線詳細/")
    }
    expected_wire_ids = {f"W-{family}{index:02}"
                         for family, count in WIRE_COUNTS.items()
                         for index in range(1, count + 1)}
    require(len(construction_ids) == 152, "Source対応: 実装要求152件が不一致")
    require(wire_ids == expected_wire_ids, "Source対応: 必須結線54件が不一致")

    repository = Path(root).resolve(strict=True)
    pairs = (
        (construction_ids, "施工要求/", "主要Source", "要求施工/", "既存Source候補"),
        (wire_ids, "施工結線/結線/", "候補Source", "施工結線詳細/", "施工Source候補"),
    )
    for identifiers, plan_prefix, plan_field, instruction_prefix, instruction_field in pairs:
        for suffix in identifiers:
            plan_key = f"{plan_prefix}{suffix}"
            instruction_key = f"{instruction_prefix}{suffix}"
            planned = plan.get(plan_key, {}).get(plan_field)
            instructed = instructions.get(instruction_key, {}).get(instruction_field)
            require(type(planned) is list and bool(planned),
                    f"Source対応: {plan_key} の{plan_field}が空")
            require(type(instructed) is list and bool(instructed),
                    f"Source対応: {instruction_key} の{instruction_field}が空")
            require(set(planned) == set(instructed),
                    f"Source対応: {plan_key} と {instruction_key} が不一致")
            for source in planned:
                require(type(source) is str and bool(source),
                        f"Source対応: {plan_key} に不正path")
                relative = PurePosixPath(source)
                require(
                    not relative.is_absolute()
                    and relative.as_posix() == source
                    and "\\" not in source
                    and ":" not in source
                    and all(part not in ("", ".", "..") for part in source.split("/")),
                    f"Source対応: Repo相対pathではない {source}",
                )
                candidate = repository.joinpath(*relative.parts)
                try:
                    resolved = candidate.resolve(strict=True)
                except (OSError, RuntimeError) as error:
                    raise Invalid(f"Source対応: 候補が存在しない {source}: {error}") from error
                require(resolved.is_relative_to(repository),
                        f"Source対応: Repo外を指すpath {source}")
                require(resolved.is_file(), f"Source対応: fileではない {source}")


def governance_errors(root: Path = ROOT) -> list[str]:
    """現行v11だけを判定。v9履歴検査はnlc08_governance_check_legacyへ分離。"""
    try:
        root = Path(root)
        index = parse_json((root / INDEX_PATH).read_text(encoding="utf-8"))
        adoption = index.get("現行施工採用")
        require(type(adoption) is dict and adoption.get("識別") == ADOPTION_ID,
                "v11採用識別が現行でない")
        require(type(index.get("履歴施工採用")) is dict
                and index["履歴施工採用"].get("識別") == "D4Pocket-v9-NLC08-20261123",
                "v9履歴採用を保持していない")
        entries = adoption.get("五文書")
        require(type(entries) is list and len(entries) == 5, "五文書の採用定義が不成立")
        require(tuple(entry.get("役割") for entry in entries) == ROLES,
                "五文書の所管・順序が不一致")
        adopt_path = adoption.get("採用記録")
        require(adopt_path == f"{DIRECTORY}/採用記録.md", "採用記録の位置が不一致")
        adopted = (root / adopt_path).read_text(encoding="utf-8")
        agents = (root / "AGENTS.md").read_text(encoding="utf-8")
        readme = (root / "README.md").read_text(encoding="utf-8")
        require(adopt_path in agents and ADOPTION_ID in (root / INDEX_PATH).read_text(encoding="utf-8"),
                "AGENTS/索引の採用接続が不成立")
        docs: dict[str, dict[str, dict]] = {}
        indexed = {item.get("path") for item in index.get("現行正本", [])
                   if type(item) is dict and item.get("現在区分") == "現行"}
        for role, count, entry in zip(ROLES, EXPECTED_RECORDS, entries):
            path = entry.get("path")
            require(type(path) is str and path.startswith(f"{DIRECTORY}/")
                    and ".." not in Path(path).parts, f"{role}: pathの範囲不正")
            raw = (root / path).read_bytes()
            digest = hashlib.sha256(raw).hexdigest()
            require(digest == entry.get("SHA256") and len(raw) == entry.get("バイト数"),
                    f"{role}: SHA256/byteが採用値と不一致")
            records = read_nlc(raw)
            require(len(records) == count == entry.get("記録数"),
                    f"{role}: record件数不一致")
            require(records["文書/宣言"].get("基底言語") == "日本語"
                    and str(records["文書/宣言"].get("版", "")).startswith("v11"),
                    f"{role}: 版/言語の不一致")
            require(path in agents and path in readme and path in indexed
                    and entry["SHA256"] in adopted, f"{role}: 索引/AGENTS/README/採用記録断絶")
            docs[role] = records
        validate_population(docs)
        validate_source_references(root, docs)
        require("品質" in adoption.get("完成規則", "")
                and "実装" in adoption.get("完成規則", ""),
                "実装と品質の完成状態が混線")
        require(index.get("現行品質採用", {}).get("工程表") == entries[3]["path"],
                "品質工程の独立入口が不成立")
    except (OSError, UnicodeError, ValueError, KeyError, IndexError, TypeError, RecursionError) as error:
        return [f"現行v11五文書整合: {error}"]
    return []


def main() -> int:
    errors = governance_errors()
    if errors:
        for error in errors:
            print(error)
        return 1
    print("現行v11五文書整合PASS: 344/320/311/401/488行、原要求188、実装152/品質36、結線54、品質71、OS5。製品QA/出荷GOは未判定。")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
