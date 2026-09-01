from __future__ import annotations

import re

from packages.shell_contracts import load_default_catalog


TYPE_MAP = {
    "object": dict,
    "array": list,
    "string": str,
    "integer": int,
    "boolean": bool,
    "null": type(None),
}


def type_matches(value, expected_type: str) -> bool:
    if expected_type == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if expected_type == "boolean":
        return isinstance(value, bool)
    return isinstance(value, TYPE_MAP[expected_type])


def validate_instance(value, schema: dict, path: str = "$") -> list[str]:
    errors: list[str] = []
    expected_type = schema.get("type")
    if isinstance(expected_type, list):
        if not any(type_matches(value, item) for item in expected_type):
            return [f"{path}: {expected_type} のいずれかが必要です"]
    elif isinstance(expected_type, str):
        if not type_matches(value, expected_type):
            return [f"{path}: {expected_type} が必要です"]

    if "const" in schema and value != schema["const"]:
        errors.append(f"{path}: const {schema['const']!r} が必要です")
    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{path}: 値 {value!r} が enum にありません")

    if isinstance(value, str):
        if "minLength" in schema and len(value) < schema["minLength"]:
            errors.append(f"{path}: minLength {schema['minLength']} より短い値です")
        if "pattern" in schema and re.match(schema["pattern"], value) is None:
            errors.append(f"{path}: pattern {schema['pattern']} と一致しません")

    if isinstance(value, list):
        if "minItems" in schema and len(value) < schema["minItems"]:
            errors.append(f"{path}: minItems {schema['minItems']} より項目が少ない値です")
        item_schema = schema.get("items")
        if isinstance(item_schema, dict):
            for index, item in enumerate(value):
                errors.extend(validate_instance(item, item_schema, f"{path}[{index}]"))

    if isinstance(value, dict):
        required = schema.get("required", [])
        for key in required:
            if key not in value:
                errors.append(f"{path}: 必須 key {key} がありません")
        properties = schema.get("properties", {})
        additional = schema.get("additionalProperties", True)
        if additional is False:
            for key in value:
                if key not in properties:
                    errors.append(f"{path}: 追加 property {key} は許可されません")
        for key, item in value.items():
            if key in properties:
                errors.extend(validate_instance(item, properties[key], f"{path}.{key}"))
            elif isinstance(additional, dict):
                errors.extend(validate_instance(item, additional, f"{path}.{key}"))
    return errors


def validate_contract(record: dict, schema_name: str) -> None:
    schema = load_default_catalog().get(schema_name)
    errors = validate_instance(record, schema)
    if errors:
        raise ValueError(f"{schema_name} の検証に失敗しました: {'; '.join(errors)}")
