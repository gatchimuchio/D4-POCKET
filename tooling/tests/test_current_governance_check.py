"""現行三点の保存形式・意味束縛・採用入口を、実原本と隔離した破損例で検査する。"""
from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from tooling.current_governance_check import (
    Invalid,
    governance_errors,
    validate_bindings,
    validate_nlc,
)


REPO_ROOT = Path(__file__).resolve().parents[2]
ADOPTION_DIRECTORY = "docs/QE_2026-11-22"
DOCUMENT_PATHS = {
    "製品仕様": f"{ADOPTION_DIRECTORY}/01_D4Pocket_SPEC_v9_NLC06.jsonl",
    "統合工程": f"{ADOPTION_DIRECTORY}/02_D4Pocket_PLAN_v9_NLC06.jsonl",
    "施工指示": f"{ADOPTION_DIRECTORY}/03_D4Pocket_CODEX_v9_NLC06.jsonl",
}
EXPECTED_RECORD_COUNTS = {"製品仕様": 438, "統合工程": 289, "施工指示": 1453}
INDEX_PATH = "規定/正本索引.json"


def encode_records(records: dict[str, dict]) -> bytes:
    """有効な保存形式だけを保ち、個別の破損要因を分離する。"""
    return (
        "\n".join(
            json.dumps(record, ensure_ascii=False, separators=(",", ":"))
            for record in records.values()
        )
        + "\n"
    ).encode("utf-8")


def remove_record_and_references(records: dict[str, dict], ident: str) -> None:
    """未解決参照ではなく母集団欠落だけを検査できるよう参照も整える。"""
    del records[ident]
    for record in records.values():
        for field, value in record.items():
            if field.endswith("参照") and isinstance(value, list):
                record[field] = [item for item in value if item != ident]


class ActualDocumentsMixin:
    @classmethod
    def setUpClass(cls) -> None:
        super().setUpClass()
        cls.raw_documents = {
            role: (REPO_ROOT / relative).read_bytes()
            for role, relative in DOCUMENT_PATHS.items()
        }
        cls.documents = {
            role: validate_nlc(raw) for role, raw in cls.raw_documents.items()
        }

    def changed_instruction(self) -> dict[str, dict]:
        return copy.deepcopy(self.documents["施工指示"])


class NLC構造検査Tests(ActualDocumentsMixin, unittest.TestCase):
    def test_実三点の保存形式と記録数が成立する(self) -> None:
        for role, raw in self.raw_documents.items():
            with self.subTest(role=role):
                records = validate_nlc(raw)
                self.assertEqual(EXPECTED_RECORD_COUNTS[role], len(records))
                self.assertEqual("日本語", records["文書/宣言"]["基底言語"])

    def test_開発検証と実製品Owner受入が分離されている(self) -> None:
        spec = self.documents["製品仕様"]["統治/開発検証と製品Owner受入の分離"]
        plan = self.documents["統合工程"]["工程/開発検証と製品Owner受入の分離"]
        instruction = self.documents["施工指示"]["具体化/開発検証分離"]
        self.assertIn("実製品Owner画面のクリックを要求してはならない", spec["規定"])
        self.assertIn("native Owner確認", spec["規定"])
        self.assertIn("実製品Permission/Approval", spec["規定"])
        self.assertIn("E1〜E4", plan["規定"])
        self.assertIn("W-E03の独立施工はW-E02の実製品Owner受入待ちで停止しない", plan["規定"])
        self.assertIn("real_owner_authority=false", instruction["規定"])
        self.assertIn("test-only制御は通常Release", instruction["規定"])
        export = self.documents["施工指示"]["操作/Export計画"]
        self.assertIn("Owner本人の画面操作を待たない", export["規定"])
        self.assertIn("--manifest-file", export["操作列"][2])
        self.assertIn("実製品の書出し作用", export["操作列"][3])
        stop = self.documents["施工指示"]["施工/停止再開"]
        self.assertNotIn("本人Gate", stop["規定"])
        self.assertIn("独立施工を続ける", stop["規定"])
        for key in ("結線/W-E02", "結線/W-E03"):
            self.assertIn(spec["識別"], self.documents["製品仕様"][key]["意味参照"])
            self.assertIn(plan["識別"], self.documents["統合工程"][key]["意味参照"])
        for key in ("施工/W-E02", "施工/W-E03"):
            self.assertIn(instruction["識別"], self.documents["施工指示"][key]["意味参照"])

    def test_BOMを拒否する(self) -> None:
        with self.assertRaises(Invalid):
            validate_nlc(b"\xef\xbb\xbf" + self.raw_documents["施工指示"])

    def test_CRLFを拒否する(self) -> None:
        with self.assertRaises(Invalid):
            validate_nlc(self.raw_documents["施工指示"].replace(b"\n", b"\r\n"))

    def test_末尾LF欠落を拒否する(self) -> None:
        with self.assertRaises(Invalid):
            validate_nlc(self.raw_documents["施工指示"][:-1])

    def test_空の物理行を拒否する(self) -> None:
        with self.assertRaises(Invalid):
            validate_nlc(self.raw_documents["施工指示"].replace(b"\n", b"\n\n", 1))

    def test_同一JSONオブジェクトの重複キーを拒否する(self) -> None:
        raw = self.raw_documents["施工指示"]
        duplicate_key = '{"識別":"重複キー検査",'.encode("utf-8") + raw[1:]
        with self.assertRaises(Invalid):
            validate_nlc(duplicate_key)

    def test_同一文書の重複識別を拒否する(self) -> None:
        raw = self.raw_documents["施工指示"]
        duplicate_record = raw + raw.split(b"\n", 1)[0] + b"\n"
        with self.assertRaises(Invalid):
            validate_nlc(duplicate_record)

    def test_未解決の意味参照を拒否する(self) -> None:
        records = self.changed_instruction()
        records["施工/単位"]["意味参照"].append("存在しない/意味")
        with self.assertRaises(Invalid):
            validate_nlc(encode_records(records))

    def test_空の全てといずれかを拒否する(self) -> None:
        for operator in ("全て", "いずれか"):
            with self.subTest(operator=operator):
                records = self.changed_instruction()
                records["施工/単位"]["適用条件"] = {operator: []}
                with self.assertRaises(Invalid):
                    validate_nlc(encode_records(records))

    def test_未知の構造欄を拒否する(self) -> None:
        records = self.changed_instruction()
        records["施工/単位"]["未定義の試験専用欄"] = "暗黙に受理しない"
        with self.assertRaises(Invalid):
            validate_nlc(encode_records(records))

    def test_整数欄に置いたJSON真偽値を拒否する(self) -> None:
        for value in (True, False):
            with self.subTest(value=value):
                records = self.changed_instruction()
                # bool が Python の int の派生型であることを受理理由にしない。
                records["入力/仕様"]["記録数"] = value
                with self.assertRaises(Invalid):
                    validate_nlc(encode_records(records))

    def test_未規定のnullと空文字と空対応表を拒否する(self) -> None:
        for value in (None, "", {}):
            with self.subTest(value=value):
                records = self.changed_instruction()
                records["施工/単位"]["規定"] = value
                with self.assertRaises(Invalid):
                    validate_nlc(encode_records(records))

    def test_文字列内の実wire真偽値はJSON真偽値と混同しない(self) -> None:
        records = self.documents["施工指示"]
        wire_text = "\n".join(records["操作/AIEdit提案審査"]["操作列"])
        self.assertIn("self_approval=false", wire_text)
        self.assertIn("repository_rules_acknowledged=true", wire_text)
        self.assertIn("操作/AIEdit提案審査", validate_nlc(encode_records(records)))


class 三点意味束縛検査Tests(ActualDocumentsMixin, unittest.TestCase):
    def assert_bindings_reject(self, documents: dict[str, dict[str, dict]]) -> None:
        # 各文書は先に構造検査を通し、単なるJSON/未解決参照ではない差を検査する。
        parsed = {
            role: validate_nlc(encode_records(records))
            for role, records in documents.items()
        }
        with self.assertRaises(Invalid):
            validate_bindings(parsed)

    def test_実三点の原要求と施工対応が成立する(self) -> None:
        self.assertIsNone(validate_bindings(copy.deepcopy(self.documents)))

    def test_参照を整えても個別検査の欠落を拒否する(self) -> None:
        documents = copy.deepcopy(self.documents)
        remove_record_and_references(
            documents["施工指示"], "実施/検査/Q1-01"
        )
        self.assert_bindings_reject(documents)

    def test_仕様の原要求の無断変更を拒否する(self) -> None:
        documents = copy.deepcopy(self.documents)
        documents["製品仕様"]["要求/N01"]["規定"] += "。無断の条件変更。"
        self.assert_bindings_reject(documents)

    def test_内在化された原要求の無断変更を拒否する(self) -> None:
        documents = copy.deepcopy(self.documents)
        documents["施工指示"]["仕様/要求/N01"]["規定"] += "。無断の条件変更。"
        self.assert_bindings_reject(documents)

    def test_解決済みIDでも工程索引の仕様側への誤束縛を拒否する(self) -> None:
        documents = copy.deepcopy(self.documents)
        instruction = documents["施工指示"]
        instruction["工程表/索引/旧W"]["構成参照"] = list(
            instruction["仕様/索引/旧W"]["構成参照"]
        )
        self.assert_bindings_reject(documents)

    def test_正しいAuthRequiredを禁止事項へ移す再否定を拒否する(self) -> None:
        documents = copy.deepcopy(self.documents)
        work = documents["施工指示"]["施工/W-B01"]
        self.assertEqual(["Provider資格欠はAuthRequired"], work["拒否境界"])
        work["禁止事項"] = work.pop("拒否境界")
        self.assert_bindings_reject(documents)

    def test_正規の拒否本文を停止条件へ直接転記する誤りを拒否する(self) -> None:
        documents = copy.deepcopy(self.documents)
        work = documents["施工指示"]["施工/W-B01"]
        work["停止条件"].append(work["拒否境界"][0])
        self.assert_bindings_reject(documents)

    def test_元記録を削った変換履歴を拒否する(self) -> None:
        documents = copy.deepcopy(self.documents)
        history = documents["施工指示"]["変換履歴/002"]
        original = json.loads(history["原文"])
        original["規定"] = "原要求を保持しない差替え"
        history["原文"] = json.dumps(original, ensure_ascii=False)
        self.assert_bindings_reject(documents)


class 統治入口検査Tests(unittest.TestCase):
    """実Repoは読み取りだけにし、破損はOS一時領域の独立コピーで作る。"""

    @classmethod
    def setUpClass(cls) -> None:
        super().setUpClass()
        cls.source_index = json.loads((REPO_ROOT / INDEX_PATH).read_text(encoding="utf-8"))
        adoption = cls.source_index["現行施工採用"]
        relative_paths = {
            INDEX_PATH,
            "AGENTS.md",
            adoption["採用記録"],
            adoption["継続施工記録"],
            adoption["継続受入証拠台帳"],
            *(entry["path"] for entry in adoption["三点"]),
        }
        cls.source_files = {
            relative: (REPO_ROOT / relative).read_bytes()
            for relative in relative_paths
        }

    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory(prefix="d4-governance-test-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        for relative, raw in self.source_files.items():
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(raw)
        self.assertEqual([], governance_errors(self.root), "隔離元の正本が不成立")

    def write_index(self, index: dict) -> None:
        (self.root / INDEX_PATH).write_text(
            json.dumps(index, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )

    def assert_governance_rejects(self) -> None:
        errors = governance_errors(self.root)
        self.assertIsInstance(errors, list)
        self.assertTrue(errors, "破損した統治接続を受理している")
        self.assertTrue(all(isinstance(error, str) and error for error in errors))

    def test_実Repoの三点と採用入口が成立する(self) -> None:
        self.assertEqual([], governance_errors(REPO_ROOT))

    def test_三点それぞれの受領Hash不一致を拒否する(self) -> None:
        for role in DOCUMENT_PATHS:
            with self.subTest(role=role):
                index = copy.deepcopy(self.source_index)
                entry = next(
                    row for row in index["現行施工採用"]["三点"] if row["役割"] == role
                )
                entry["SHA256"] = "0" * 64
                self.write_index(index)
                self.assert_governance_rejects()

    def test_構造が有効でも三点の実bytes差替えを拒否する(self) -> None:
        for role, relative in DOCUMENT_PATHS.items():
            with self.subTest(role=role):
                original = self.source_files[relative]
                changed = original[:-1] + b" \n"
                validate_nlc(changed)
                path = self.root / relative
                try:
                    path.write_bytes(changed)
                    self.assert_governance_rejects()
                finally:
                    path.write_bytes(original)

    def test_採用記録の所在断絶を拒否する(self) -> None:
        index = copy.deepcopy(self.source_index)
        index["現行施工採用"]["採用記録"] = f"{ADOPTION_DIRECTORY}/存在しない採用記録.md"
        self.write_index(index)
        self.assert_governance_rejects()

    def test_採用記録から三点の同一性Hashが消えた場合を拒否する(self) -> None:
        adoption = self.source_index["現行施工採用"]
        path = self.root / adoption["採用記録"]
        text = path.read_text(encoding="utf-8")
        digest = adoption["三点"][0]["SHA256"]
        self.assertIn(digest, text)
        path.write_text(text.replace(digest, "Hash記載欠落"), encoding="utf-8")
        self.assert_governance_rejects()

    def test_AGENTSから各三点への参照断絶を拒否する(self) -> None:
        path = self.root / "AGENTS.md"
        original = self.source_files["AGENTS.md"].decode("utf-8")
        for role, relative in DOCUMENT_PATHS.items():
            with self.subTest(role=role):
                self.assertIn(relative, original)
                path.write_text(original.replace(relative, "三点所在の記載欠落"), encoding="utf-8")
                self.assert_governance_rejects()

    def test_AGENTSから採用記録への参照断絶を拒否する(self) -> None:
        path = self.root / "AGENTS.md"
        original = path.read_text(encoding="utf-8")
        basename = Path(self.source_index["現行施工採用"]["採用記録"]).name
        self.assertIn(basename, original)
        path.write_text(original.replace(basename, "採用所在の記載欠落"), encoding="utf-8")
        self.assert_governance_rejects()

    def test_正本索引の現行登録から三点が消えた場合を拒否する(self) -> None:
        index = copy.deepcopy(self.source_index)
        relative = DOCUMENT_PATHS["施工指示"]
        index["現行正本"] = [
            row for row in index["現行正本"] if row.get("path") != relative
        ]
        self.write_index(index)
        self.assert_governance_rejects()

    def test_明示具体化の採用接続欠落を拒否する(self) -> None:
        index = copy.deepcopy(self.source_index)
        index["現行施工採用"]["明示具体化"].remove("具体化/Export選択集合")
        self.write_index(index)
        self.assert_governance_rejects()


if __name__ == "__main__":
    unittest.main()
