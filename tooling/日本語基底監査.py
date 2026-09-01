from __future__ import annotations

import argparse
import ast
import bisect
import fnmatch
import hashlib
import io
import json
import re
import subprocess
import sys
import tempfile
import tokenize
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable, Iterator


ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = "規定/00_日本語基底規定.md"
INDEX_PATH = "規定/正本索引.json"
REGISTRY_PATH = "規定/日本語基底例外.json"
AUDITOR_PATH = "tooling/日本語基底監査.py"
WINDOWS_PROOF_PACK_SCOPE = "openai_submission_assets/windows_proof_pack/**"
SELF_TEST_ENGLISH_MESSAGE = "Visible contract diagnostic"

JAPANESE_RE = re.compile(r"[ぁ-ゖァ-ヺ一-鿿々〆〇ー]")
LATIN_WORD_RE = re.compile(r"[A-Za-z][A-Za-z'\-]*")
ENGLISH_CLAUSE_RE = re.compile(
    r"(?:\b[A-Za-z][A-Za-z'\-]*\b[ \t]+){3,}\b[A-Za-z][A-Za-z'\-]*\b"
)
URL_RE = re.compile(r"(?:https?|wss?)://[^\s)>\]}]+", re.IGNORECASE)
HASH_RE = re.compile(r"\b(?:sha(?:-?256)?[:=]?)?[0-9a-f]{40,64}\b", re.IGNORECASE)
INLINE_CODE_RE = re.compile(r"`[^`]*`")
MARKDOWN_LINK_RE = re.compile(r"!?\[([^\]]*)\]\([^)]*\)")
HTML_TAG_RE = re.compile(r"<[^>]+>")
INTERPOLATION_RE = re.compile(r"\$\{[^}]*\}|\$[A-Za-z_][A-Za-z0-9_]*|\{[^{}]*\}")

SCANNED_CODE_SUFFIXES = {".py", ".dart", ".rs", ".sh", ".ps1", ".cpp", ".cc", ".h"}
JSON_HUMAN_KEYS = {
    "title",
    "description",
    "$comment",
    "message",
    "reason",
    "required_action",
    "recovery_instruction",
    "summary",
    "diagnostic_summary",
    "user_visible_message",
    "display_name",
    "window_title",
    "note",
    "release_ready_reason",
}
MACHINE_VALUES = {
    "true",
    "false",
    "yes",
    "no",
    "none",
    "null",
    "root",
    "pass",
    "passed",
    "fail",
    "failed",
    "pending",
    "blocked",
    "unknown",
    "ok",
    "error",
}

MARKDOWN_MACHINE_FIELDS = {
    "classification": re.compile(
        r"(?:release_blocker|post_v1_scope|known_limitation|required_for_v1|none)"
    ),
    "blocks_release": re.compile(r"(?:yes|no|true|false)"),
    "registry_id": re.compile(r"[A-Za-z0-9][A-Za-z0-9_.:/#@+\-]*"),
    "aggregate_of": re.compile(
        r"(?:none|[A-Za-z0-9_.:/#@+\-]+(?:\s*,\s*[A-Za-z0-9_.:/#@+\-]+)*|"
        r"\[[A-Za-z0-9_.:/#@+\-,\"' ]*\])"
    ),
}

CLI_COMMAND_RE = re.compile(
    r"(?:"
    r"(?:python(?:3(?:\.\d+)?)?|bash|sh|pwsh|powershell)\s+"
    r"(?:[A-Za-z0-9_.-]+[/\\])+[A-Za-z0-9_.-]+"
    r"|cargo\s+(?:test|build|check|fmt|clippy|run)"
    r"|flutter\s+(?:analyze|test|build|run|doctor|pub)"
    r"|git\s+(?:status|diff|show|log|add|commit|push|fetch|ls-files|ls-remote)"
    r")(?:\s+[-A-Za-z0-9_./\\:=@]+)*"
)

REQUIRED_EXCEPTION_CATEGORIES = {
    "法的原文",
    "生成物・依存固定",
    "保存証拠",
    "機械構文・外部互換",
    "固有名・固定表記",
}

LEXICAL_EXCEPTION_CATEGORIES = {"機械構文・外部互換", "固有名・固定表記"}
FILE_EXCEPTION_CATEGORIES = {"法的原文", "生成物・依存固定", "保存証拠"}

AUDIT_LIMITATIONS = [
    "本監査は Git 追跡済みおよび非 ignore 未追跡 file の静的ヒューリスティック検査であり、日本語文字の存在を意味・仕様・権限境界の適合証明とは扱わない。",
    "Dart などの動的生成文字列、runtime から受け取る外部文字列、画面の実描画、アクセシビリティ、翻訳の自然さは実行時・人間のレビューが別途必要である。",
    "コード検査は明白な comment、docstring、CLI、診断文、UI 文字列を対象とするが、各言語の完全な AST 解析ではないため偽陽性・偽陰性が残り得る。",
    "ignore 対象、submodule 内容、binary 内部の文字列、外部配布物は走査対象外である。",
    "局所例外は台帳に記録された範囲だけに適用し、例外資産を現行の規範・権限・製品完了の根拠へ昇格させない。",
]


@dataclass(frozen=True)
class Finding:
    path: str
    surface: str
    line: int
    excerpt: str


@dataclass(frozen=True)
class ExceptionEntry:
    id: str
    scopes: tuple[str, ...]
    region: str
    category: str
    handling: str
    integrity: str
    terms: tuple[str, ...]


@dataclass(frozen=True)
class FrozenAsset:
    path: str
    sha256: str
    exception_id: str


@dataclass(frozen=True)
class Registry:
    entries: tuple[ExceptionEntry, ...]
    frozen_assets: tuple[FrozenAsset, ...]


@dataclass(frozen=True)
class AuditResult:
    ok: bool
    state: str
    strict: bool
    repository_files: int
    scanned_files: dict[str, int]
    excluded_files: tuple[str, ...]
    debt_files: int
    debt_findings: int
    findings: tuple[Finding, ...]
    failures: tuple[str, ...]
    warnings: tuple[str, ...]
    limitations: tuple[str, ...]


def has_japanese(text: str) -> bool:
    return JAPANESE_RE.search(text) is not None


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def path_matches(path: str, scopes: Iterable[str]) -> bool:
    return any(
        fnmatch.fnmatchcase(path, scope)
        or (scope.startswith("**/") and fnmatch.fnmatchcase(path, scope[3:]))
        for scope in scopes
    )


def repository_paths(root: Path) -> tuple[list[str], list[str]]:
    failures: list[str] = []
    try:
        completed = subprocess.run(
            [
                "git",
                "-C",
                str(root),
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
                "-z",
            ],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except OSError as exc:
        return [], [f"repository file 一覧を取得できない: {exc.__class__.__name__}: {exc}"]
    if completed.returncode != 0:
        stderr = completed.stderr.decode("utf-8", errors="replace").strip()
        return [], [f"git ls-files が失敗した: {stderr}"]
    try:
        decoded = completed.stdout.decode("utf-8")
    except UnicodeDecodeError as exc:
        return [], [f"repository file 一覧を UTF-8 として読めない: {exc}"]
    paths = sorted(path for path in decoded.split("\0") if path)
    root_resolved = root.resolve()
    for rel in paths:
        candidate = root / rel
        try:
            resolved = candidate.resolve(strict=False)
        except OSError as exc:
            failures.append(f"repository path を解決できない: {rel}: {exc}")
            continue
        if not resolved.is_relative_to(root_resolved):
            failures.append(f"repository path が repository 外を指している: {rel}")
    return paths, failures


def read_utf8(path: Path, rel: str, failures: list[str]) -> str | None:
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as exc:
        failures.append(f"走査対象を UTF-8 で読めない: {rel}: {exc}")
        return None


def _valid_relative_scope(scope: object) -> bool:
    return (
        isinstance(scope, str)
        and bool(scope)
        and not Path(scope).is_absolute()
        and ".." not in Path(scope).parts
    )


def _scope_has_wildcard(scope: str) -> bool:
    return any(char in scope for char in "*?[")


def _allowed_generated_scope(scope: str) -> bool:
    name = Path(scope).name.lower()
    return (
        scope.endswith(".lock")
        or name == ".metadata"
        or "generated" in name
        or scope == "MANIFEST.sha256.json"
        or re.fullmatch(r"apps/[A-Za-z0-9_.-]+/(?:linux|windows|macos)/\*\*", scope)
        is not None
    )


def _allowed_evidence_scope(scope: str) -> bool:
    lowered = scope.lower()
    return (
        lowered.startswith(("docs/evidence/", "docs/source/", "docs/reports/", "docs/history/"))
        or lowered.startswith("openai_submission_assets/")
        or "ledger" in Path(lowered).name
    )


def load_registry(root: Path, repository_files: list[str], failures: list[str]) -> Registry:
    registry_file = root / REGISTRY_PATH
    try:
        raw = json.loads(registry_file.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        failures.append(f"{REGISTRY_PATH} を読めない: {exc}")
        return Registry((), ())

    if raw.get("schema_version") != 1:
        failures.append(f"{REGISTRY_PATH}: schema_version は 1 でなければならない")
    if raw.get("policy") != POLICY_PATH:
        failures.append(f"{REGISTRY_PATH}: policy は {POLICY_PATH} を参照しなければならない")
    purpose = raw.get("purpose_japanese")
    if not isinstance(purpose, str) or not has_japanese(purpose):
        failures.append(f"{REGISTRY_PATH}: purpose_japanese は日本語で必要")

    entries_raw = raw.get("exceptions")
    if not isinstance(entries_raw, list):
        failures.append(f"{REGISTRY_PATH}: exceptions は配列でなければならない")
        entries_raw = []
    entries: list[ExceptionEntry] = []
    seen_ids: set[str] = set()
    for index, item in enumerate(entries_raw):
        label = f"{REGISTRY_PATH}: exceptions[{index}]"
        if not isinstance(item, dict):
            failures.append(f"{label} は object でなければならない")
            continue
        exception_id = item.get("id")
        if not isinstance(exception_id, str) or not re.fullmatch(r"JBE-[0-9]{3}", exception_id):
            failures.append(f"{label}.id は JBE-NNN 形式で必要")
            continue
        if exception_id in seen_ids:
            failures.append(f"{label}.id が重複している: {exception_id}")
        seen_ids.add(exception_id)
        scopes_raw = item.get("scopes")
        if not isinstance(scopes_raw, list) or not scopes_raw or not all(
            _valid_relative_scope(scope) for scope in scopes_raw
        ):
            failures.append(f"{label}.scopes は repository 内の相対 scope 配列で必要")
            scopes_raw = []
        region = item.get("region")
        category = item.get("category")
        handling = item.get("handling")
        integrity = item.get("integrity")
        reason = item.get("unavoidable_reason_japanese")
        reference = item.get("japanese_reference")
        effect = item.get("authority_effect")
        trigger = item.get("review_trigger")
        for field_name, value in (
            ("region", region),
            ("unavoidable_reason_japanese", reason),
            ("japanese_reference", reference),
            ("review_trigger", trigger),
        ):
            if not isinstance(value, str) or not has_japanese(value):
                failures.append(f"{label}.{field_name} は日本語で必要")
        if reference != POLICY_PATH:
            failures.append(f"{label}.japanese_reference は {POLICY_PATH} へ接続しなければならない")
        if category not in REQUIRED_EXCEPTION_CATEGORIES:
            failures.append(f"{label}.category が未定義: {category}")
        if handling not in {"exclude_file", "lexical_only"}:
            failures.append(f"{label}.handling が不正: {handling}")
        if integrity not in {"sha256", "review_on_change"}:
            failures.append(f"{label}.integrity が不正: {integrity}")
        if effect != "none":
            failures.append(f"{label}.authority_effect は none でなければならない")
        terms_raw = item.get("terms", [])
        if not isinstance(terms_raw, list) or not all(isinstance(term, str) and term for term in terms_raw):
            failures.append(f"{label}.terms は非空 string の配列でなければならない")
            terms_raw = []
        if handling == "exclude_file" and category not in FILE_EXCEPTION_CATEGORIES:
            failures.append(f"{label}: file 全体除外は法的原文・生成物・保存証拠に限る")
        if handling == "lexical_only" and category not in LEXICAL_EXCEPTION_CATEGORIES:
            failures.append(f"{label}: 字句例外は機械構文・外部互換または固有名・固定表記に限る")
        if category in {"法的原文", "保存証拠"}:
            if handling != "exclude_file" or integrity != "sha256":
                failures.append(f"{label}: 法的原文・保存証拠は sha256 固定の file 全体除外で必要")
        if category == "法的原文" and any(
            _scope_has_wildcard(str(scope)) for scope in scopes_raw
        ):
            failures.append(f"{label}: 法的原文に wildcard scope を使えない")
        if category == "保存証拠":
            for scope in scopes_raw:
                if _scope_has_wildcard(str(scope)) and scope != WINDOWS_PROOF_PACK_SCOPE:
                    failures.append(f"{label}: 保存証拠の wildcard scope が局所 proof pack ではない: {scope}")
        if category == "法的原文":
            for scope in scopes_raw:
                if Path(str(scope)).name.upper() not in {"LICENSE", "COPYING", "NOTICE"}:
                    failures.append(f"{label}: 法的原文 scope の種類が不明: {scope}")
        if category == "保存証拠":
            for scope in scopes_raw:
                if not _allowed_evidence_scope(str(scope)):
                    failures.append(f"{label}: 保存証拠の局所性を確認できない scope: {scope}")
        if category == "生成物・依存固定":
            if handling != "exclude_file" or integrity != "review_on_change":
                failures.append(f"{label}: 生成物・依存固定は変更時再監査の file 全体除外で必要")
            for scope in scopes_raw:
                if not _allowed_generated_scope(str(scope)):
                    failures.append(f"{label}: 生成物と証明できない scope: {scope}")
        if handling == "exclude_file":
            for scope in scopes_raw:
                if not any(path_matches(rel, (str(scope),)) for rel in repository_files):
                    failures.append(f"{label}: 現在資産に一致しない全体除外 scope: {scope}")
        if category == "機械構文・外部互換" and terms_raw:
            failures.append(f"{label}: 機械構文例外に任意の terms allowlist を設けてはならない")
        if category == "固有名・固定表記":
            if not terms_raw:
                failures.append(f"{label}: 固有名・固定表記は個別 terms を必要とする")
            for term in terms_raw:
                if len(str(term)) > 64 or len(str(term).split()) > 4 or "\n" in str(term):
                    failures.append(f"{label}: 固有名の局所表記として広すぎる term: {term}")
        entries.append(
            ExceptionEntry(
                exception_id,
                tuple(str(scope) for scope in scopes_raw),
                str(region or ""),
                str(category or ""),
                str(handling or ""),
                str(integrity or ""),
                tuple(str(term) for term in terms_raw),
            )
        )

    missing_categories = REQUIRED_EXCEPTION_CATEGORIES - {entry.category for entry in entries}
    if missing_categories:
        failures.append(
            f"{REGISTRY_PATH}: 必須例外分類を欠く: {', '.join(sorted(missing_categories))}"
        )

    frozen_raw = raw.get("frozen_assets")
    if not isinstance(frozen_raw, list):
        failures.append(f"{REGISTRY_PATH}: frozen_assets は配列でなければならない")
        frozen_raw = []
    frozen_assets: list[FrozenAsset] = []
    frozen_paths: set[str] = set()
    by_id = {entry.id: entry for entry in entries}
    for index, item in enumerate(frozen_raw):
        label = f"{REGISTRY_PATH}: frozen_assets[{index}]"
        if not isinstance(item, dict):
            failures.append(f"{label} は object でなければならない")
            continue
        rel = item.get("path")
        expected = item.get("sha256")
        exception_id = item.get("exception_id")
        if not _valid_relative_scope(rel) or any(char in str(rel) for char in "*?["):
            failures.append(f"{label}.path は固定した相対 path で必要")
            continue
        rel = str(rel)
        if rel in frozen_paths:
            failures.append(f"{label}.path が重複している: {rel}")
        frozen_paths.add(rel)
        if not isinstance(expected, str) or not re.fullmatch(r"[0-9a-f]{64}", expected):
            failures.append(f"{label}.sha256 が不正")
            continue
        entry = by_id.get(str(exception_id))
        if entry is None or entry.integrity != "sha256" or not path_matches(rel, entry.scopes):
            failures.append(f"{label}.exception_id が sha256 固定例外と scope 一致しない")
            continue
        asset = root / rel
        if not asset.is_file():
            failures.append(f"{label}: 固定資産が存在しない: {rel}")
        elif sha256_file(asset) != expected:
            failures.append(f"{label}: 保存証拠または法的原文の hash が変化した: {rel}")
        frozen_assets.append(FrozenAsset(rel, expected, str(exception_id)))

    for entry in entries:
        if entry.handling != "exclude_file" or entry.integrity != "sha256":
            continue
        for rel in repository_files:
            if path_matches(rel, entry.scopes) and rel not in frozen_paths:
                failures.append(f"{entry.id}: sha256 固定 scope の資産が frozen_assets にない: {rel}")

    return Registry(tuple(entries), tuple(frozen_assets))


def validate_governance(root: Path, failures: list[str]) -> None:
    policy_file = root / POLICY_PATH
    policy = read_utf8(policy_file, POLICY_PATH, failures) if policy_file.is_file() else None
    if policy is None:
        if not policy_file.is_file():
            failures.append(f"必須正本が存在しない: {POLICY_PATH}")
    else:
        for phrase in ("日本語", "基底", "局所例外", "監査"):
            if phrase not in policy:
                failures.append(f"{POLICY_PATH}: 必須の意味接続を欠く: {phrase}")

    index_file = root / INDEX_PATH
    try:
        index = json.loads(index_file.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        failures.append(f"{INDEX_PATH} を読めない: {exc}")
        return
    serialized = json.dumps(index, ensure_ascii=False, sort_keys=True)
    for needle in ("日本語", POLICY_PATH, REGISTRY_PATH, AUDITOR_PATH):
        if needle not in serialized:
            failures.append(f"{INDEX_PATH}: 正本接続を欠く: {needle}")


def exception_for_path(path: str, registry: Registry) -> ExceptionEntry | None:
    for entry in registry.entries:
        if entry.handling == "exclude_file" and path_matches(path, entry.scopes):
            return entry
    return None


def lexical_terms_for_path(path: str, registry: Registry) -> tuple[str, ...]:
    terms: set[str] = set()
    for entry in registry.entries:
        if entry.handling == "lexical_only" and path_matches(path, entry.scopes):
            terms.update(entry.terms)
    return tuple(sorted(terms, key=len, reverse=True))


def line_number(newlines: list[int], offset: int) -> int:
    return bisect.bisect_right(newlines, offset) + 1


def compact_excerpt(text: str, limit: int = 180) -> str:
    collapsed = re.sub(r"\s+", " ", text).strip()
    return collapsed if len(collapsed) <= limit else f"{collapsed[: limit - 1]}…"


def remove_machine_exceptions(text: str) -> str:
    cleaned = URL_RE.sub(" ", text)
    cleaned = HASH_RE.sub(" ", cleaned)
    return INTERPOLATION_RE.sub("", cleaned)


def remove_lexical_exceptions(text: str, terms: Iterable[str]) -> str:
    cleaned = remove_machine_exceptions(text)
    for term in terms:
        cleaned = cleaned.replace(term, " ")
    return cleaned


def machine_only(text: str) -> bool:
    raw = text.strip()
    if not raw:
        return True
    if re.match(r"^(?:\{\{?|\[)\\?[\"']", raw):
        return True
    if CLI_COMMAND_RE.fullmatch(raw):
        return True
    if re.fullmatch(
        r"[A-Za-z_][A-Za-z0-9_.-]*\s*=\s*[A-Za-z0-9_.:/#@+\-{}$\\]*"
        r"(?:\s*[|,;]\s*[A-Za-z_][A-Za-z0-9_.-]*\s*=\s*"
        r"[A-Za-z0-9_.:/#@+\-{}$\\]*)*",
        raw,
    ):
        return True
    if re.fullmatch(r":?\s*[a-z_][a-z0-9_]*=", raw):
        return True
    if re.fullmatch(r"(?:-\s*)?[a-z_][a-z0-9_]*(?::\s*\|?)?", raw):
        return True
    if re.fullmatch(r"[A-Z][A-Za-z0-9_.]*\([A-Za-z_][A-Za-z0-9_]*:\s*\)", raw):
        return True
    if re.fullmatch(r"[A-Za-z0-9_.:/#@+%$\\\-]+", raw) and any(
        char in raw for char in "_./:#@%$\\-"
    ):
        return True
    stripped = text.strip(" \t\r\n:;,.()[]{}<>/\\|=+*#@!?“”‘’\"")
    if not stripped:
        return True
    lowered = stripped.lower()
    if lowered in MACHINE_VALUES:
        return True
    if re.fullmatch(r"[A-Z][A-Z0-9_-]{1,31}", stripped):
        return True
    if re.fullmatch(r"[A-Za-z0-9_.:/#@+%$\\\-]+", stripped) and any(
        char in stripped for char in "_./:#@%$\\-"
    ):
        return True
    if re.fullmatch(r"[A-Za-z][A-Za-z0-9]*", stripped) and any(char.isdigit() for char in stripped):
        return True
    return False


def is_non_japanese_prose(text: str, terms: Iterable[str]) -> bool:
    if machine_only(text):
        return False
    machine_cleaned = remove_machine_exceptions(text)
    if machine_only(machine_cleaned):
        return False
    cleaned = remove_lexical_exceptions(text, terms)
    if has_japanese(cleaned):
        return ENGLISH_CLAUSE_RE.search(cleaned) is not None
    words = LATIN_WORD_RE.findall(cleaned)
    if not words:
        return False
    if machine_only(cleaned):
        stripped = cleaned.strip(" \t\r\n:;,.()[]{}<>/\\|=+*#@!?“”‘’\"")
        if stripped.lower() in MACHINE_VALUES and cleaned != machine_cleaned:
            return True
        return False
    return True


def markdown_segments(line: str) -> list[str]:
    if re.fullmatch(r"\s*\|?(?:\s*:?-{3,}:?\s*\|)+\s*", line):
        return []
    line = MARKDOWN_LINK_RE.sub(lambda match: match.group(1), line)
    line = INLINE_CODE_RE.sub(" ", line)
    line = HTML_TAG_RE.sub(" ", line)
    if "|" in line:
        segments = line.split("|")
    else:
        segments = [line]
    cleaned: list[str] = []
    for segment in segments:
        segment = re.sub(r"^\s{0,3}(?:#{1,6}|>|[-+*]|\d+[.)])\s*", "", segment)
        segment = re.sub(r"^\s*[-:]{3,}\s*$", "", segment)
        field_match = re.fullmatch(r"\s*([a-z_]+)\s*:\s*(.*?)\s*", segment)
        if field_match is not None:
            value_pattern = MARKDOWN_MACHINE_FIELDS.get(field_match.group(1))
            if value_pattern is not None and value_pattern.fullmatch(field_match.group(2)):
                continue
        if segment.strip():
            cleaned.append(segment)
    return cleaned


def scan_markdown(path: str, text: str, terms: tuple[str, ...]) -> list[Finding]:
    findings: list[Finding] = []
    in_fence = False
    for number, line in enumerate(text.splitlines(), start=1):
        if re.match(r"^\s*(```|~~~)", line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        for segment in markdown_segments(line):
            if is_non_japanese_prose(segment, terms):
                findings.append(Finding(path, "active_markdown_prose", number, compact_excerpt(segment)))
    return findings


@dataclass(frozen=True)
class CLikeToken:
    kind: str
    start: int
    end: int
    body: str


def _skip_quoted_expression(text: str, start: int) -> int:
    quote = text[start]
    delimiter = quote * 3 if text.startswith(quote * 3, start) else quote
    cursor = start + len(delimiter)
    while cursor < len(text):
        if text.startswith(delimiter, cursor):
            return cursor + len(delimiter)
        if text[cursor] == "\\" and cursor + 1 < len(text):
            cursor += 2
        else:
            cursor += 1
    return len(text)


def _skip_dart_interpolation(text: str, start: int) -> int:
    cursor = start
    depth = 1
    while cursor < len(text) and depth:
        char = text[cursor]
        if char in {"'", '"'}:
            cursor = _skip_quoted_expression(text, cursor)
        elif char == "{":
            depth += 1
            cursor += 1
        elif char == "}":
            depth -= 1
            cursor += 1
        else:
            cursor += 1
    return cursor


def iter_c_like_tokens(
    text: str,
    *,
    single_quote_strings: bool = True,
    dart_interpolation: bool = False,
) -> Iterator[CLikeToken]:
    index = 0
    length = len(text)
    while index < length:
        if text.startswith("//", index):
            end = text.find("\n", index + 2)
            end = length if end < 0 else end
            yield CLikeToken("comment", index, end, text[index + 2 : end])
            index = end
            continue
        if text.startswith("/*", index):
            end_marker = text.find("*/", index + 2)
            end = length if end_marker < 0 else end_marker + 2
            body_end = length if end_marker < 0 else end_marker
            yield CLikeToken("comment", index, end, text[index + 2 : body_end])
            index = end
            continue
        quote = text[index]
        allowed_quotes = {"'", '"'} if single_quote_strings else {'"'}
        if quote not in allowed_quotes:
            index += 1
            continue
        triple = text.startswith(quote * 3, index)
        delimiter = quote * 3 if triple else quote
        body_start = index + len(delimiter)
        cursor = body_start
        raw = index > 0 and text[index - 1] in {"r", "R"}
        while cursor < length:
            if dart_interpolation and text.startswith("${", cursor):
                cursor = _skip_dart_interpolation(text, cursor + 2)
                continue
            if text.startswith(delimiter, cursor):
                end = cursor + len(delimiter)
                yield CLikeToken("string", index, end, text[body_start:cursor])
                index = end
                break
            if not raw and text[cursor] == "\\" and cursor + 1 < length:
                cursor += 2
            else:
                cursor += 1
        else:
            index += len(delimiter)


DART_CONTEXT_RE = re.compile(
    r"(?:\bText\s*\(|\bSelectableText\s*\(|\bTooltip\s*\(|"
    r"\b(?:title|subtitle|label|labelText|hintText|helperText|errorText|semanticLabel|"
    r"tooltip|message|reason|requiredAction|recoveryInstruction|diagnosticSummary|description)\s*:)",
    re.MULTILINE,
)
DART_DIAGNOSTIC_RE = re.compile(r"\b(?:throw|return|Exception|Error|print|debugPrint|log)\b")
DART_MACHINE_ARGUMENT_RE = re.compile(r"\b(?:evidenceLabel|evidenceTitle)\s*:\s*$")


def scan_c_like_comments(
    path: str,
    text: str,
    terms: tuple[str, ...],
    tokens: list[CLikeToken],
) -> list[Finding]:
    findings: list[Finding] = []
    newlines = [index for index, char in enumerate(text) if char == "\n"]
    for token in tokens:
        if token.kind != "comment":
            continue
        base_line = line_number(newlines, token.start)
        for offset, raw_line in enumerate(token.body.splitlines() or [token.body]):
            candidate = raw_line.strip().lstrip("*!/").strip()
            if is_non_japanese_prose(candidate, terms):
                findings.append(
                    Finding(path, "code_comment", base_line + offset, compact_excerpt(candidate))
                )
    return findings


def scan_dart(path: str, text: str, terms: tuple[str, ...]) -> list[Finding]:
    tokens = list(iter_c_like_tokens(text, dart_interpolation=True))
    findings = scan_c_like_comments(path, text, terms, tokens)
    newlines = [index for index, char in enumerate(text) if char == "\n"]
    for token in tokens:
        if token.kind != "string":
            continue
        body = token.body
        if not body.strip():
            continue
        line_start = text.rfind("\n", 0, token.start) + 1
        line_end = text.find("\n", token.end)
        line_end = len(text) if line_end < 0 else line_end
        line_text = text[line_start:line_end]
        if re.match(r"\s*(?:import|export|part)\b", line_text):
            continue
        after = text[token.end : min(len(text), token.end + 12)]
        if re.match(r"\s*:", after):
            continue
        before = text[max(0, token.start - 220) : token.start]
        if DART_MACHINE_ARGUMENT_RE.search(before):
            continue
        context = f"{before}\n{line_text[: max(0, token.start - line_start)]}"
        user_facing = DART_CONTEXT_RE.search(context) is not None
        if not user_facing and len(LATIN_WORD_RE.findall(body)) >= 2:
            user_facing = DART_DIAGNOSTIC_RE.search(context[-120:]) is not None
        if user_facing and is_non_japanese_prose(body, terms):
            findings.append(
                Finding(
                    path,
                    "dart_user_facing_string",
                    line_number(newlines, token.start),
                    compact_excerpt(body),
                )
            )
    return findings


def _python_call_name(node: ast.Call) -> str:
    target = node.func
    if isinstance(target, ast.Name):
        return target.id
    if isinstance(target, ast.Attribute):
        return target.attr
    return ""


def _visible_string_constants(node: ast.AST) -> Iterator[ast.Constant]:
    if isinstance(node, ast.Constant):
        if isinstance(node.value, str):
            yield node
        return
    if isinstance(node, ast.JoinedStr):
        visible = " ".join(
            child.value
            for child in node.values
            if isinstance(child, ast.Constant) and isinstance(child.value, str)
        )
        if visible:
            yield ast.copy_location(ast.Constant(value=visible), node)
        return
    if isinstance(node, (ast.Subscript, ast.Attribute, ast.Name)):
        return
    for child in ast.iter_child_nodes(node):
        yield from _visible_string_constants(child)


def _returned_string_constants(node: ast.AST) -> Iterator[ast.Constant]:
    if isinstance(node, (ast.Constant, ast.JoinedStr)):
        yield from _visible_string_constants(node)
        return
    if isinstance(node, (ast.List, ast.Tuple, ast.Set)):
        for child in node.elts:
            yield from _returned_string_constants(child)
        return
    if isinstance(node, ast.IfExp):
        yield from _returned_string_constants(node.body)
        yield from _returned_string_constants(node.orelse)


def scan_python(
    path: str,
    text: str,
    terms: tuple[str, ...],
) -> tuple[list[Finding], list[str]]:
    findings: list[Finding] = []
    failures: list[str] = []
    seen: set[tuple[int, str]] = set()
    try:
        for token in tokenize.generate_tokens(io.StringIO(text).readline):
            if token.type != tokenize.COMMENT:
                continue
            candidate = token.string[1:].strip()
            if candidate.startswith("!") or re.match(r".*coding[:=]", candidate):
                continue
            if is_non_japanese_prose(candidate, terms):
                key = (token.start[0], candidate)
                if key not in seen:
                    seen.add(key)
                    findings.append(Finding(path, "code_comment", token.start[0], compact_excerpt(candidate)))
    except (tokenize.TokenError, IndentationError) as exc:
        failures.append(f"Python token 解析が失敗した: {path}: {exc}")

    try:
        tree = ast.parse(text, filename=path)
    except SyntaxError as exc:
        failures.append(f"Python AST 解析が失敗した: {path}:{exc.lineno}: {exc.msg}")
        return findings, failures

    for node in ast.walk(tree):
        if isinstance(node, (ast.Module, ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            body = getattr(node, "body", [])
            if body and isinstance(body[0], ast.Expr):
                value = body[0].value
                if isinstance(value, ast.Constant) and isinstance(value.value, str):
                    if is_non_japanese_prose(value.value, terms):
                        key = (getattr(value, "lineno", 1), value.value)
                        if key not in seen:
                            seen.add(key)
                            findings.append(
                                Finding(path, "code_docstring", key[0], compact_excerpt(value.value))
                            )
        if isinstance(node, ast.Call):
            call_name = _python_call_name(node)
            diagnostic_append = (
                isinstance(node.func, ast.Attribute)
                and node.func.attr == "append"
                and isinstance(node.func.value, ast.Name)
                and node.func.value.id
                in {"errors", "failures", "warnings", "issues", "violations", "diagnostics"}
            )
            diagnostic_call = call_name in {
                "print",
                "echo",
                "exit",
                "info",
                "debug",
                "log",
                "warning",
                "error",
                "exception",
                "critical",
                "ValueError",
                "RuntimeError",
                "AssertionError",
                "SystemExit",
                "ArgumentParser",
                "_failed",
                "_passed",
            }
            candidates: list[ast.Constant] = []
            if diagnostic_call or diagnostic_append:
                for argument in node.args:
                    candidates.extend(_visible_string_constants(argument))
            if call_name == "EvidenceCheck" and len(node.args) >= 6:
                for argument in node.args[4:6]:
                    candidates.extend(_visible_string_constants(argument))
            for keyword in node.keywords:
                if keyword.arg in {
                    "help",
                    "description",
                    "reason",
                    "required_action",
                    "message",
                    "post_v1_reason",
                }:
                    candidates.extend(_visible_string_constants(keyword.value))
            for value in candidates:
                if is_non_japanese_prose(value.value, terms):
                    key = (getattr(value, "lineno", 1), value.value)
                    if key not in seen:
                        seen.add(key)
                        findings.append(
                            Finding(path, "cli_or_diagnostic_string", key[0], compact_excerpt(value.value))
                        )
        if isinstance(node, ast.Dict):
            for key_node, value_node in zip(node.keys, node.values):
                if not isinstance(key_node, ast.Constant) or key_node.value not in {
                    "message",
                    "reason",
                    "required_action",
                    "description",
                }:
                    continue
                for value in _visible_string_constants(value_node):
                    if is_non_japanese_prose(value.value, terms):
                        key = (getattr(value, "lineno", 1), value.value)
                        if key not in seen:
                            seen.add(key)
                            findings.append(
                                Finding(path, "cli_or_diagnostic_string", key[0], compact_excerpt(value.value))
                            )
        if isinstance(node, ast.Raise) and node.exc is not None:
            for value in _visible_string_constants(node.exc):
                if is_non_japanese_prose(value.value, terms):
                    key = (getattr(value, "lineno", 1), value.value)
                    if key not in seen:
                        seen.add(key)
                        findings.append(
                            Finding(path, "cli_or_diagnostic_string", key[0], compact_excerpt(value.value))
                        )
        if isinstance(node, ast.Assert) and node.msg is not None:
            for value in _visible_string_constants(node.msg):
                if is_non_japanese_prose(value.value, terms):
                    key = (getattr(value, "lineno", 1), value.value)
                    if key not in seen:
                        seen.add(key)
                        findings.append(
                            Finding(path, "cli_or_diagnostic_string", key[0], compact_excerpt(value.value))
                        )
        if isinstance(node, ast.Return) and node.value is not None:
            for value in _returned_string_constants(node.value):
                if len(LATIN_WORD_RE.findall(value.value)) < 2:
                    continue
                if is_non_japanese_prose(value.value, terms):
                    key = (getattr(value, "lineno", 1), value.value)
                    if key not in seen:
                        seen.add(key)
                        findings.append(
                            Finding(path, "cli_or_diagnostic_string", key[0], compact_excerpt(value.value))
                        )
    return findings, failures


RUST_DIAGNOSTIC_RE = re.compile(
    r"(?:println!|eprintln!|format!|panic!|bail!|ensure!|anyhow!|Err\s*\(|expect\s*\(|"
    r"message\s*:|reason\s*:|description\s*:)",
    re.MULTILINE,
)
SHELL_DIAGNOSTIC_RE = re.compile(
    r"\b(?:echo|printf|throw|Write-(?:Host|Output|Error|Warning|Verbose)|System\.out\.print)\b",
    re.IGNORECASE,
)
POWERSHELL_VALUE_CONTEXT_RE = re.compile(
    r"(?P<human>"
    r"-(?:Message|Reason|RequiredAction|RecoveryInstruction)\b"
    r"|\b(?:message|reason|required_action|recovery_instruction|"
    r"reason_not_formal_product_evidence)\s*="
    r"|\$(?:errors|failures|warnings|issues|violations|diagnostics)\s*"
    r"(?:\+=|\.Add\s*\()"
    r")"
    r"|(?P<machine>"
    r"-[A-Za-z][A-Za-z0-9-]*\b"
    r"|\b[A-Za-z_][A-Za-z0-9_]*\s*="
    r")",
    re.IGNORECASE,
)
QUOTED_RE = re.compile(r"(['\"])(.*?)(?<!\\)\1")


def scan_c_like_code(path: str, text: str, terms: tuple[str, ...]) -> list[Finding]:
    tokens = list(iter_c_like_tokens(text, single_quote_strings=not path.endswith(".rs")))
    findings = scan_c_like_comments(path, text, terms, tokens)
    newlines = [index for index, char in enumerate(text) if char == "\n"]
    for token in tokens:
        if token.kind != "string":
            continue
        before = text[max(0, token.start - 120) : token.start]
        if RUST_DIAGNOSTIC_RE.search(before) and is_non_japanese_prose(token.body, terms):
            findings.append(
                Finding(
                    path,
                    "cli_or_diagnostic_string",
                    line_number(newlines, token.start),
                    compact_excerpt(token.body),
                )
            )
    return findings


def scan_shell_like(path: str, text: str, terms: tuple[str, ...]) -> list[Finding]:
    findings: list[Finding] = []
    for number, line in enumerate(text.splitlines(), start=1):
        stripped = line.strip()
        if stripped.startswith("#") and not stripped.startswith("#!"):
            comment = stripped[1:].strip()
            if is_non_japanese_prose(comment, terms):
                findings.append(Finding(path, "code_comment", number, compact_excerpt(comment)))
        for match in QUOTED_RE.finditer(line):
            body = match.group(2)
            direct_diagnostic = SHELL_DIAGNOSTIC_RE.search(line[: match.start()]) is not None
            powershell_human_value = False
            if path.lower().endswith(".ps1"):
                contexts = list(POWERSHELL_VALUE_CONTEXT_RE.finditer(line[: match.start()]))
                powershell_human_value = bool(
                    contexts and contexts[-1].group("human") is not None
                )
            if (direct_diagnostic or powershell_human_value) and is_non_japanese_prose(
                body, terms
            ):
                findings.append(
                    Finding(path, "cli_or_diagnostic_string", number, compact_excerpt(body))
                )
    return findings


def scan_schema(path: str, text: str, terms: tuple[str, ...]) -> tuple[list[Finding], list[str]]:
    failures: list[str] = []
    findings: list[Finding] = []
    try:
        document = json.loads(text)
    except json.JSONDecodeError as exc:
        return [], [f"JSON Schema を解析できない: {path}:{exc.lineno}: {exc.msg}"]

    def visit(value: object, pointer: str) -> None:
        if isinstance(value, dict):
            for key, child in value.items():
                child_pointer = f"{pointer}/{key.replace('~', '~0').replace('/', '~1')}"
                if key in {"title", "description", "$comment"} and isinstance(child, str):
                    if is_non_japanese_prose(child, terms):
                        findings.append(
                            Finding(
                                path,
                                "json_schema_human_metadata",
                                1,
                                compact_excerpt(f"{child_pointer}: {child}"),
                            )
                        )
                visit(child, child_pointer)
        elif isinstance(value, list):
            for index, child in enumerate(value):
                visit(child, f"{pointer}/{index}")

    visit(document, "")
    return findings, failures


def scan_json_human_metadata(
    path: str,
    text: str,
    terms: tuple[str, ...],
) -> tuple[list[Finding], list[str]]:
    try:
        document = json.loads(text)
    except json.JSONDecodeError as exc:
        return [], [f"JSON を解析できない: {path}:{exc.lineno}: {exc.msg}"]

    findings: list[Finding] = []

    def visit(value: object, pointer: str) -> None:
        if isinstance(value, dict):
            for key, child in value.items():
                child_pointer = f"{pointer}/{key.replace('~', '~0').replace('/', '~1')}"
                if key in JSON_HUMAN_KEYS and isinstance(child, str):
                    if is_non_japanese_prose(child, terms):
                        findings.append(
                            Finding(
                                path,
                                "json_human_metadata",
                                1,
                                compact_excerpt(f"{child_pointer}: {child}"),
                            )
                        )
                visit(child, child_pointer)
        elif isinstance(value, list):
            for index, child in enumerate(value):
                visit(child, f"{pointer}/{index}")

    visit(document, "")
    return findings, []


def audit_repository(root: Path, strict: bool = False) -> AuditResult:
    failures: list[str] = []
    warnings: list[str] = []
    repository_files, git_failures = repository_paths(root)
    failures.extend(git_failures)
    registry = load_registry(root, repository_files, failures)
    validate_governance(root, failures)

    findings: list[Finding] = []
    excluded: list[str] = []
    scanned = {
        "active_markdown": 0,
        "dart": 0,
        "json_schema": 0,
        "json_human": 0,
        "other_code": 0,
    }
    for rel in repository_files:
        exception = exception_for_path(rel, registry)
        if exception is not None:
            excluded.append(f"{rel} ({exception.id})")
            continue
        suffix = Path(rel).suffix.lower()
        is_markdown = suffix == ".md"
        is_dart = suffix == ".dart"
        is_schema = rel.endswith(".schema.json")
        is_json = suffix == ".json"
        is_other_code = suffix in SCANNED_CODE_SUFFIXES - {".dart"}
        if not (is_markdown or is_dart or is_json or is_other_code):
            continue
        candidate = root / rel
        if not candidate.is_file():
            failures.append(f"repository 走査対象が working tree に存在しない: {rel}")
            continue
        text = read_utf8(candidate, rel, failures)
        if text is None:
            continue
        terms = lexical_terms_for_path(rel, registry)
        if is_markdown:
            scanned["active_markdown"] += 1
            findings.extend(scan_markdown(rel, text, terms))
        elif is_dart:
            scanned["dart"] += 1
            findings.extend(scan_dart(rel, text, terms))
        elif is_schema:
            scanned["json_schema"] += 1
            schema_findings, schema_failures = scan_schema(rel, text, terms)
            findings.extend(schema_findings)
            failures.extend(schema_failures)
        elif is_json:
            scanned["json_human"] += 1
            json_findings, json_failures = scan_json_human_metadata(rel, text, terms)
            findings.extend(json_findings)
            failures.extend(json_failures)
        else:
            scanned["other_code"] += 1
            if suffix == ".py":
                code_findings, code_failures = scan_python(rel, text, terms)
                findings.extend(code_findings)
                failures.extend(code_failures)
            elif suffix in {".rs", ".cpp", ".cc", ".h"}:
                findings.extend(scan_c_like_code(rel, text, terms))
            else:
                findings.extend(scan_shell_like(rel, text, terms))

    findings = sorted(findings, key=lambda item: (item.path, item.line, item.surface, item.excerpt))
    unique_findings: list[Finding] = []
    seen_findings: set[tuple[str, str, int, str]] = set()
    for finding in findings:
        key = (finding.path, finding.surface, finding.line, finding.excerpt)
        if key not in seen_findings:
            seen_findings.add(key)
            unique_findings.append(finding)
    findings = unique_findings

    debt_paths = {finding.path for finding in findings}
    if findings:
        warnings.append(
            f"日本語基底負債が {len(debt_paths)} files / {len(findings)} findings 残っている"
        )
    if strict and findings:
        failures.append(
            f"厳格監査: 日本語基底負債 {len(debt_paths)} files / {len(findings)} findings が未解消"
        )
    ok = not failures
    if failures:
        state = "不整合"
    elif findings:
        state = "監査報告・負債あり"
    else:
        state = "厳格成立"
    return AuditResult(
        ok=ok,
        state=state,
        strict=strict,
        repository_files=len(repository_files),
        scanned_files=scanned,
        excluded_files=tuple(sorted(excluded)),
        debt_files=len(debt_paths),
        debt_findings=len(findings),
        findings=tuple(findings),
        failures=tuple(failures),
        warnings=tuple(warnings),
        limitations=tuple(AUDIT_LIMITATIONS),
    )


def result_to_json(result: AuditResult) -> dict[str, object]:
    payload = asdict(result)
    payload["findings"] = [asdict(finding) for finding in result.findings]
    return payload


def print_result(result: AuditResult) -> None:
    if not result.ok:
        status = "FAIL"
    elif result.findings:
        status = "REPORT"
    else:
        status = "PASS"
    scans = " ".join(f"{key}={value}" for key, value in result.scanned_files.items())
    print(
        f"[日本語基底監査] {status} 状態={result.state} strict={str(result.strict).lower()} "
        f"repository_files={result.repository_files} excluded={len(result.excluded_files)} "
        f"{scans} 負債files={result.debt_files} 負債findings={result.debt_findings}"
    )
    grouped: dict[str, list[Finding]] = {}
    for finding in result.findings:
        grouped.setdefault(finding.path, []).append(finding)
    for path, path_findings in grouped.items():
        surfaces = sorted({finding.surface for finding in path_findings})
        lines = sorted({finding.line for finding in path_findings})
        line_text = ",".join(str(line) for line in lines[:24])
        if len(lines) > 24:
            line_text += f",他{len(lines) - 24}"
        print(
            f"  負債 {path}: findings={len(path_findings)} "
            f"surface={','.join(surfaces)} lines={line_text}"
        )
    for warning in result.warnings:
        print(f"  注意 {warning}")
    for failure in result.failures:
        print(f"  失敗 {failure}", file=sys.stderr)
    print("  監査限界:")
    for limitation in result.limitations:
        print(f"    - {limitation}")


def run_self_tests() -> int:
    tests = 0

    def check(condition: bool, message: str) -> None:
        nonlocal tests
        tests += 1
        if not condition:
            raise AssertionError(message)

    terms = ("GUI Shell", "Flutter")
    check(not is_non_japanese_prose("権限境界は日本語で定める。", terms), "日本語散文の誤検出")
    check(is_non_japanese_prose("Authority must never come from UI state.", terms), "英語散文を検出できない")
    check(not is_non_japanese_prose("GUI Shell / Flutter", terms), "台帳登録された固有名の誤検出")
    check(
        is_non_japanese_prose("判断は Authority must never come from UI state. と定める。", terms),
        "日本語文字を添えた英語散文を見逃した",
    )
    check(
        not is_non_japanese_prose("recover-broker-persistence", ()),
        "kebab-case 契約 ID を誤検出した",
    )
    check(
        not is_non_japanese_prose("installer_grants_authority=false", ()),
        "key=value 固定値を誤検出した",
    )
    check(
        is_non_japanese_prose("broker request failed", ()),
        "人間向け診断文を機械値として見逃した",
    )
    check(
        is_non_japanese_prose("Approval failed", ("Approval",)),
        "固定契約名の周囲にある診断文を見逃した",
    )
    check(
        not is_non_japanese_prose("python3 tooling/validate_all.py", ()),
        "CLI command 固定構文を誤検出した",
    )
    check(
        is_non_japanese_prose("python validation failed", ()),
        "CLI に似た人間向け診断を見逃した",
    )

    markdown = "# 判断境界\n\n```text\nEnglish fixture\n```\n\nEnglish active prose.\n"
    markdown_findings = scan_markdown("README.md", markdown, ())
    check(len(markdown_findings) == 1, "Markdown の active prose / code fence 分離失敗")
    check(markdown_findings[0].line == 7, "Markdown 行番取得失敗")

    ledger = (
        "- classification: required_for_v1\n"
        "- blocks_release: no\n"
        "- registry_id: R2-WIN-001\n"
        "- aggregate_of: R2-WIN-001, R2-WIN-002\n"
        "- status: unresolved\n"
        "- reason: External evidence is still missing\n"
        "- item: Windows evidence\n"
    )
    ledger_findings = scan_markdown("ROADMAP.md", ledger, ())
    check(
        [finding.line for finding in ledger_findings] == [5, 6, 7],
        "Markdown 機械 field と人間向け台帳値の分離失敗",
    )

    dart = (
        "const Text('Settings');\n"
        "const Text('設定');\n"
        "const key = 'runtime_id';\n"
        "SurfaceSemantics(label: '設定', evidenceLabel: 'NavigationRail');\n"
        "Text(\"${flag ? 'PASS' : 'FAIL'}\");\n"
    )
    dart_findings = scan_dart("lib/main.dart", dart, ())
    check(
        any(finding.excerpt == "Settings" for finding in dart_findings),
        "Dart の利用者向け文字列を検出できない",
    )
    check(
        all(finding.excerpt != "runtime_id" for finding in dart_findings),
        "Dart の固定識別子を誤検出した",
    )
    check(
        all(finding.excerpt != "NavigationRail" for finding in dart_findings),
        "Dart の evidence label を表示文字列と誤認した",
    )
    check(
        all("PASS" not in finding.excerpt for finding in dart_findings),
        "Dart の interpolation-only 文字列を誤検出した",
    )

    rust = (
        'format!("{{\\"request_id\\":\\"{}\\"}}");\n'
        'format!("broker-audit-{event_index}");\n'
        'panic!("visible broker failure message");\n'
    )
    rust_findings = scan_c_like_code("native/example.rs", rust, ())
    check(
        [finding.excerpt for finding in rust_findings] == ["visible broker failure message"],
        "Rust の JSON・契約 ID と人間向け診断の分離失敗",
    )

    schema = json.dumps(
        {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "Approval",
            "description": "承認境界を表す。",
        },
        ensure_ascii=False,
    )
    schema_findings, schema_failures = scan_schema("specs/test.schema.json", schema, ())
    check(not schema_failures, "JSON Schema 自己試験の解析失敗")
    check(len(schema_findings) == 1 and "/title" in schema_findings[0].excerpt, "JSON Schema title 検出失敗")

    json_document = json.dumps(
        {
            "message": SELF_TEST_ENGLISH_MESSAGE,
            "reason": "owner-use",
            "required_action": "所有者が再検証する。",
        },
        ensure_ascii=False,
    )
    json_findings, json_failures = scan_json_human_metadata(
        "examples/contracts/example.json",
        json_document,
        (),
    )
    check(not json_failures, "JSON 人間向け metadata 自己試験の解析失敗")
    check(
        len(json_findings) == 1 and "/message" in json_findings[0].excerpt,
        "JSON 人間向け metadata と固定機械値の分離失敗",
    )

    python_source = (
        '# Human-facing comment\n'
        'print("Visible diagnostic message")\n'
        'errors = []\n'
        'errors.append("Appended failure message")\n'
        'def probe():\n'
        '    return ["Returned failure message"]\n'
        'raise CustomError("Custom failure message")\n'
        'assert value, "Assertion failure message"\n'
        'value = "runtime_id"\n'
    )
    python_findings, python_failures = scan_python("tooling/example.py", python_source, ())
    check(not python_failures, "Python 自己試験の解析失敗")
    check(
        {finding.surface for finding in python_findings}
        == {"code_comment", "cli_or_diagnostic_string"},
        "Python comment / diagnostic 分類失敗",
    )
    check(
        {
            "Appended failure message",
            "Returned failure message",
            "Custom failure message",
            "Assertion failure message",
        }
        <= {finding.excerpt for finding in python_findings},
        "Python の append / return / custom raise / assert 診断を検出できない",
    )

    powershell_source = (
        'New-DoctorCheck -CheckId "windows.machine_id" -Message "Visible doctor message" '
        '-RecoveryInstruction "Operator recovery instruction"\n'
        '$record = [ordered]@{ status = "passed"; reason = "Visible record reason"; '
        'command = "powershell -File tooling\\probe.ps1" }\n'
        '$errors += "Accumulated failure message"\n'
        '$errors.Add("Added failure message")\n'
    )
    powershell_findings = scan_shell_like(
        "installer/windows/example.ps1",
        powershell_source,
        (),
    )
    check(
        {finding.excerpt for finding in powershell_findings}
        == {
            "Visible doctor message",
            "Operator recovery instruction",
            "Visible record reason",
            "Accumulated failure message",
            "Added failure message",
        },
        "PowerShell の人間向け引数・台帳値・error 蓄積診断の分類失敗",
    )
    check(
        all(
            finding.excerpt not in {
                "windows.machine_id",
                "passed",
                "powershell -File tooling\\probe.ps1",
            }
            for finding in powershell_findings
        ),
        "PowerShell の機械 field/value/path/command を人間向け診断と誤認した",
    )

    entry = ExceptionEntry(
        "JBE-999",
        ("LICENSE",),
        "法的原文",
        "法的原文",
        "exclude_file",
        "sha256",
        (),
    )
    registry = Registry((entry,), ())
    check(exception_for_path("LICENSE", registry) == entry, "固定 path 例外の一致失敗")
    check(exception_for_path("README.md", registry) is None, "局所例外が能動文書へ漏れた")
    check(_allowed_generated_scope("**/*.lock"), "lockfile scope を生成物として分類できない")
    check(
        not _allowed_generated_scope("docs/**"),
        "能動文書の広域 scope を生成物として許した",
    )
    check(
        not _allowed_evidence_scope("README.md"),
        "能動文書を保存証拠 scope として許した",
    )

    with tempfile.TemporaryDirectory(prefix="gui-shell-ja-base-") as temporary:
        temporary_root = Path(temporary)
        subprocess.run(["git", "init", "-q", str(temporary_root)], check=True)
        (temporary_root / "tracked.md").write_text("追跡済み\n", encoding="utf-8")
        (temporary_root / ".gitignore").write_text("ignored.md\n", encoding="utf-8")
        (temporary_root / "untracked.md").write_text("未追跡\n", encoding="utf-8")
        (temporary_root / "ignored.md").write_text("English ignored prose.\n", encoding="utf-8")
        subprocess.run(
            ["git", "-C", str(temporary_root), "add", ".gitignore", "tracked.md"],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        inventory, inventory_failures = repository_paths(temporary_root)
        check(not inventory_failures, "repository file 自己試験の inventory 失敗")
        check(
            inventory == [".gitignore", "tracked.md", "untracked.md"],
            "追跡済みと非 ignore 未追跡を正しく列挙できない",
        )
        check("ignored.md" not in inventory, "ignore 対象が走査基準へ混入した")

    validate_all = (ROOT / "tooling" / "validate_all.py").read_text(encoding="utf-8")
    check(
        'python_step("tooling/日本語基底監査.py", "--strict")' in validate_all,
        "validate_all の常時 strict 接続が存在しない",
    )
    current_inventory, current_inventory_failures = repository_paths(ROOT)
    check(not current_inventory_failures, "現行 repository file 一覧の取得失敗")
    current_registry_failures: list[str] = []
    current_registry = load_registry(ROOT, current_inventory, current_registry_failures)
    check(not current_registry_failures, "現行局所例外台帳の整合性失敗")
    check(
        "Approval" not in lexical_terms_for_path("apps/example.dart", current_registry),
        "契約識別名例外が Dart 利用者表示へ漏れた",
    )
    check(
        "Approval" in lexical_terms_for_path("specs/example.schema.json", current_registry),
        "JSON Schema の固定契約識別名との接続を失った",
    )
    current_governance_failures: list[str] = []
    validate_governance(ROOT, current_governance_failures)
    check(not current_governance_failures, "現行規定・正本索引の接続失敗")
    own_source = Path(__file__).read_text(encoding="utf-8")
    own_findings, own_failures = scan_python(
        "tooling/日本語基底監査.py",
        own_source,
        lexical_terms_for_path("tooling/日本語基底監査.py", current_registry),
    )
    check(not own_failures, "監査器自身の Python 解析が失敗した")
    check(not own_findings, "監査器自身に未登録の非日本語散文が残っている")

    print(f"[日本語基底監査:自己試験] PASS tests={tests}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description="GUI-Shell の追跡済みおよび非 ignore 未追跡資産を、日本語基底の局所例外台帳と照合する。"
    )
    parser.add_argument("--root", type=Path, default=ROOT, help="監査対象 repository の root")
    parser.add_argument(
        "--strict",
        action="store_true",
        help="日本語基底負債が一件でも残る場合に失敗する。",
    )
    parser.add_argument("--json", action="store_true", help="全 finding を JSON で出力する。")
    parser.add_argument("--self-test", action="store_true", help="監査器の局所自己試験を実行する。")
    args = parser.parse_args()
    if args.self_test:
        try:
            return run_self_tests()
        except AssertionError as exc:
            print(f"[日本語基底監査:自己試験] FAIL {exc}", file=sys.stderr)
            return 1

    result = audit_repository(args.root.resolve(), strict=args.strict)
    if args.json:
        print(json.dumps(result_to_json(result), ensure_ascii=False, indent=2))
    else:
        print_result(result)
    return 0 if result.ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
