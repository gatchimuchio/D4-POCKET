from pathlib import Path
import json
import re

ROOT = Path(__file__).resolve().parents[2]
SPECS = ROOT / "specs"
EXAMPLES = ROOT / "examples" / "contracts"
INVALID_EXAMPLES = EXAMPLES / "invalid"

REQUIRED = {
    "action_envelope.schema.json",
    "runtime.schema.json",
    "adapter.schema.json",
    "capability.schema.json",
    "permission.schema.json",
    "approval.schema.json",
    "audit.schema.json",
    "recovery.schema.json",
    "diagnostic.schema.json",
    "update.schema.json",
    "content_exposure.schema.json",
    "framework_risk_profile.schema.json",
    "runtime_manifest.schema.json",
    "adapter_manifest.schema.json",
    "agent_runtime.schema.json",
    "agent_session.schema.json",
    "agent_workspace.schema.json",
    "agent_task.schema.json",
    "agent_tool_call.schema.json",
    "agent_diff.schema.json",
    "ipc_request.schema.json",
    "ipc_response.schema.json",
    "broker_error.schema.json",
    "broker_session.schema.json",
    "broker_health.schema.json",
    "broker_command_envelope.schema.json",
}

TYPE_MAP = {
    "object": dict,
    "array": list,
    "string": str,
    "integer": int,
    "boolean": bool,
    "null": type(None),
}


def load_json(path: Path) -> tuple[object | None, str | None]:
    try:
        return json.loads(path.read_text(encoding="utf-8")), None
    except Exception as exc:
        return None, str(exc)


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
            errors.append(f"{path}: expected one of {expected_type}")
            return errors
    elif isinstance(expected_type, str):
        if not type_matches(value, expected_type):
            errors.append(f"{path}: expected {expected_type}")
            return errors

    if "const" in schema and value != schema["const"]:
        errors.append(f"{path}: expected const {schema['const']!r}")

    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{path}: value {value!r} not in enum")

    if isinstance(value, str):
        if "minLength" in schema and len(value) < schema["minLength"]:
            errors.append(f"{path}: shorter than minLength {schema['minLength']}")
        if "pattern" in schema and re.match(schema["pattern"], value) is None:
            errors.append(f"{path}: does not match pattern {schema['pattern']}")

    if isinstance(value, int) and not isinstance(value, bool):
        if "minimum" in schema and value < schema["minimum"]:
            errors.append(f"{path}: below minimum {schema['minimum']}")

    if isinstance(value, list):
        if "minItems" in schema and len(value) < schema["minItems"]:
            errors.append(f"{path}: fewer than minItems {schema['minItems']}")
        item_schema = schema.get("items")
        if isinstance(item_schema, dict):
            for index, item in enumerate(value):
                errors.extend(validate_instance(item, item_schema, f"{path}[{index}]"))

    if isinstance(value, dict):
        required = schema.get("required", [])
        for key in required:
            if key not in value:
                errors.append(f"{path}: missing required key {key}")

        properties = schema.get("properties", {})
        additional = schema.get("additionalProperties", True)

        if additional is False:
            for key in value:
                if key not in properties:
                    errors.append(f"{path}: additional property {key} not allowed")

        for key, item in value.items():
            if key in properties:
                errors.extend(validate_instance(item, properties[key], f"{path}.{key}"))
            elif isinstance(additional, dict):
                errors.extend(validate_instance(item, additional, f"{path}.{key}"))

    return errors


def valid_example_path(schema_name: str) -> Path:
    return EXAMPLES / schema_name.replace(".schema.json", ".valid.json")


def schema_name_from_invalid_fixture(path: Path) -> str:
    stem = path.name.removesuffix(".invalid.json")
    schema_bases = sorted(
        (name.removesuffix(".schema.json") for name in REQUIRED),
        key=len,
        reverse=True,
    )
    for base in schema_bases:
        if stem == base or stem.startswith(f"{base}_"):
            return f"{base}.schema.json"
    return f"{stem.split('_', 1)[0]}.schema.json"


def main() -> int:
    existing = {p.name for p in SPECS.glob("*.schema.json")}
    missing = sorted(REQUIRED - existing)
    errors: list[str] = []
    if missing:
        for name in missing:
            errors.append(f"missing schema: {name}")

    schemas: dict[str, dict] = {}
    for path in sorted(SPECS.glob("*.schema.json")):
        data, err = load_json(path)
        if err:
            errors.append(f"{path}: invalid json: {err}")
            continue
        if not isinstance(data, dict):
            errors.append(f"{path}: schema root must be object")
            continue
        schemas[path.name] = data
        for key in ["$schema", "$id", "title", "type"]:
            if key not in data:
                errors.append(f"{path}: missing {key}")

    for schema_name in sorted(REQUIRED):
        schema = schemas.get(schema_name)
        if not schema:
            continue
        example_path = valid_example_path(schema_name)
        example, err = load_json(example_path)
        if err:
            errors.append(f"{example_path}: invalid or missing valid example: {err}")
            continue
        for failure in validate_instance(example, schema):
            errors.append(f"{example_path}: {failure}")

    invalid_count = 0
    invalid_schema_names: set[str] = set()
    for invalid_path in sorted(INVALID_EXAMPLES.glob("*.invalid.json")):
        invalid_count += 1
        schema_name = schema_name_from_invalid_fixture(invalid_path)
        invalid_schema_names.add(schema_name)
        schema = schemas.get(schema_name)
        if not schema:
            errors.append(f"{invalid_path}: cannot resolve schema {schema_name}")
            continue
        instance, err = load_json(invalid_path)
        if err:
            errors.append(f"{invalid_path}: invalid json: {err}")
            continue
        failures = validate_instance(instance, schema)
        if not failures:
            errors.append(f"{invalid_path}: invalid fixture unexpectedly passed {schema_name}")

    missing_invalid = sorted(REQUIRED - invalid_schema_names)
    for schema_name in missing_invalid:
        errors.append(f"missing negative fixture for {schema_name}")

    if errors:
        print("schema check failed:")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(f"schema check passed: {len(schemas)} schemas, {len(REQUIRED)} examples, {invalid_count} negative fixtures")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
