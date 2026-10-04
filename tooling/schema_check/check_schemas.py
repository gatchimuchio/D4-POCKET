from pathlib import Path
from copy import deepcopy
from datetime import datetime
import json
import math
import re
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[2]
SPECS = ROOT / "specs"
EXAMPLES = ROOT / "examples" / "contracts"
INVALID_EXAMPLES = EXAMPLES / "invalid"
DATE_TIME_PATTERN = re.compile(
    r"[0-9]{4}-[0-9]{2}-[0-9]{2}[Tt][0-9]{2}:[0-9]{2}:[0-9]{2}"
    r"(?:\.[0-9]+)?(?:[Zz]|[+-][0-9]{2}:[0-9]{2})"
)


def is_date_time(value: str) -> bool:
    if DATE_TIME_PATTERN.fullmatch(value) is None:
        return False
    normalized = value.replace("t", "T").replace("z", "Z")
    if normalized.endswith("Z"):
        normalized = normalized[:-1] + "+00:00"
    try:
        parsed = datetime.fromisoformat(normalized)
    except ValueError:
        return False
    return parsed.tzinfo is not None and parsed.utcoffset() is not None

REQUIRED = {
    "runtime_content_discard.schema.json",
    "runtime_content_reconciliation.schema.json",
    "runtime_content_discard_reconciliation.schema.json",
    "runtime_content_state.schema.json",
    "runtime_content_deletion.schema.json",
    "runtime_content_access.schema.json",
    "runtime_content_archive.schema.json",
    "runtime_content_receipt.schema.json",
    "runtime_replay_response.schema.json",
    "runtime_result_evidence.schema.json",
    "runtime_history_access.schema.json",
    "runtime_execution_history_page.schema.json",
    "runtime_execution_history.schema.json",
    "runtime_execution_record.schema.json",
    "runtime_resource_query.schema.json",
    "runtime_resource_observation.schema.json",
    "runtime_lifecycle_request.schema.json",
    "runtime_lifecycle_result.schema.json",
    "workspace_inspection_response.schema.json",
    "workspace_startup.schema.json",
    "workspace_inspection_request.schema.json",
    "workspace_diff.schema.json",
    "audit_checkpoint.schema.json",
    "device_link_control.schema.json",
    "device_link_invitation.schema.json",
    "device_link_credential.schema.json",
    "device_link_request.schema.json",
    "mobile_device_link_channel_request.schema.json",
    "mobile_local_recovery_audit.schema.json",
    "mobile_agent_list.schema.json",
    "mobile_workspace_selection.schema.json",
    "runtime_dialogue_operation.schema.json",
    "runtime_dialogue_request.schema.json",
    "runtime_dialogue_submission_receipt.schema.json",
    "runtime_dialogue_session.schema.json",
    "runtime_dialogue_session_list.schema.json",
    "runtime_dialogue_response.schema.json",
    "runtime_dialogue_comparison.schema.json",
    "evaluation_dataset.schema.json",
    "evaluation_case.schema.json",
    "evaluation_evaluator.schema.json",
    "evaluation_dataset_registration.schema.json",
    "regression_case_registration.schema.json",
    "regression_case_receipt.schema.json",
    "regression_case_list_request.schema.json",
    "regression_case_list.schema.json",
    "regression_case_delete_request.schema.json",
    "regression_case_delete_receipt.schema.json",
    "regression_case_delete_recovery_request.schema.json",
    "regression_case_delete_recovery_receipt.schema.json",
    "credential_registration.schema.json",
    "credential_revocation_request.schema.json",
    "credential_receipt.schema.json",
    "credential_list.schema.json",
    "mcp_contract.schema.json",
    "a2a_contract.schema.json",
    "mcp_connection.schema.json",
    "mcp_connection_receipt.schema.json",
    "mcp_tool_call.schema.json",
    "mcp_tool_call_receipt.schema.json",
    "mcp_connection_list.schema.json",
    "mcp_disconnect.schema.json",
    "mcp_disconnect_receipt.schema.json",
    "a2a_connection.schema.json",
    "a2a_connection_receipt.schema.json",
    "a2a_connection_list.schema.json",
    "host_registration.schema.json",
    "host_receipt.schema.json",
    "host_list.schema.json",
    "host_switch.schema.json",
    "host_switch_receipt.schema.json",
    "adapter_management_manifest.schema.json",
    "adapter_management_request.schema.json",
    "adapter_management_receipt.schema.json",
    "adapter_management_list.schema.json",
    "tray_stop_request.schema.json",
    "tray_stop_response.schema.json",
    "windows_tray_projection.schema.json",
    "profile.schema.json",
    "profile_receipt.schema.json",
    "profile_list.schema.json",
    "evaluation_experiment.schema.json",
    "evaluation_result.schema.json",
    "evaluation_public_result.schema.json",
    "evaluation_comparison.schema.json",

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
    "update_candidate.schema.json",
    "update_receipt.schema.json",
    "update_list.schema.json",
    "update_download_job.schema.json",
    "update_trust.schema.json",
    "notification.schema.json",
    "notification_list.schema.json",
    "notification_action.schema.json",
    "observation_span.schema.json",
    "observation_trace.schema.json",
    "observation_metric.schema.json",
    "observation_list.schema.json",
    "content_exposure.schema.json",
    "framework_risk_profile.schema.json",
    "runtime_manifest.schema.json",
    "adapter_manifest.schema.json",
    "agent_runtime.schema.json",
    "agent_adapter.schema.json",
    "agent_session.schema.json",
    "agent_workspace.schema.json",
    "agent_task.schema.json",
    "agent_task_content_exposure.schema.json",
    "agent_task_id_request.schema.json",
    "agent_task_request.schema.json",
    "agent_cli_runtime_workspace_registration.schema.json",
    "agent_task_workspace_permission_request.schema.json",
    "agent_task_workspace_permission.schema.json",
    "agent_task_owner_approval.schema.json",
    "agent_task_scratch_recovery.schema.json",
    "agent_tool_call.schema.json",
    "agent_diff.schema.json",
    "agent_comparison.schema.json",
    "agent_handoff.schema.json",
    "gui_shell_compose.schema.json",
    "gui_shell_compose_receipt.schema.json",
    "gui_shell_preview.schema.json",
    "gui_shell_preview_receipt.schema.json",
    "gui_shell_edit_proposal.schema.json",
    "gui_shell_edit_proposal_receipt.schema.json",
    "gui_shell_export.schema.json",
    "gui_shell_export_receipt.schema.json",
    "gui_shell_export_manifest.schema.json",
    "gui_shell_module_catalog.schema.json",
    "gui_shell_module_build_evidence.schema.json",
    "gui_shell_module_comparison_evidence.schema.json",
    "gui_shell_windows_export_build.schema.json",
    "host_capability.schema.json",
    "ipc_request.schema.json",
    "ipc_response.schema.json",
    "broker_error.schema.json",
    "broker_endpoint.schema.json",
    "broker_session.schema.json",
    "broker_health.schema.json",
    "broker_command_envelope.schema.json",
    "desktop_broker_channel_request.schema.json",
    "setup_doctor_report.schema.json",
    "first_run_configuration.schema.json",
    "first_run_configuration_request.schema.json",
}

TYPE_MAP = {
    "object": dict,
    "array": list,
    "string": str,
    "integer": int,
    "number": (int, float),
    "boolean": bool,
    "null": type(None),
}


def _reject_nonfinite_json_constant(token: str) -> None:
    raise ValueError(f"JSONの非有限数は許可しない: {token}")


def _assert_finite_json_numbers(value: object, path: str = "$") -> None:
    if isinstance(value, float):
        if not math.isfinite(value):
            raise ValueError(f"{path}: JSON数値は有限でなければならない")
        return
    if isinstance(value, list):
        for index, item in enumerate(value):
            _assert_finite_json_numbers(item, f"{path}[{index}]")
        return
    if isinstance(value, dict):
        for key, item in value.items():
            _assert_finite_json_numbers(item, f"{path}.{key}")


def parse_json_text(text: str) -> object:
    value = json.loads(text, parse_constant=_reject_nonfinite_json_constant)
    _assert_finite_json_numbers(value)
    return value


def load_json(path: Path) -> tuple[object | None, str | None]:
    try:
        return parse_json_text(path.read_text(encoding="utf-8")), None
    except Exception as exc:
        return None, str(exc)


def type_matches(value, expected_type: str) -> bool:
    if expected_type == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if expected_type == "number":
        return (
            isinstance(value, (int, float))
            and not isinstance(value, bool)
            and (not isinstance(value, float) or math.isfinite(value))
        )
    if expected_type == "boolean":
        return isinstance(value, bool)
    return isinstance(value, TYPE_MAP[expected_type])


def validate_instance(value, schema: dict, path: str = "$", root: dict | None = None) -> list[str]:
    errors: list[str] = []
    if isinstance(value, float) and not math.isfinite(value):
        return [f"{path}: JSON数値は有限でなければならない"]
    if root is None:
        root = schema
    reference = schema.get("$ref")
    if reference is not None:
        if not isinstance(reference, str):
            return [f"{path}: 未対応の$ref {reference!r}"]
        if not reference.startswith("#/"):
            resource_name, separator, fragment = reference.partition("#")
            external = Path(resource_name)
            if (
                not resource_name
                or external.name != resource_name
                or external.is_absolute()
                or ".." in external.parts
            ):
                return [f"{path}: 外部$ref pathが不正 {reference!r}"]
            external_path = SPECS / external
            external_schema, error = load_json(external_path)
            if error or not isinstance(external_schema, dict):
                return [f"{path}: 外部$ref {reference!r}を解決できない"]
            if not separator or not fragment:
                return validate_instance(value, external_schema, path)
            pointer = unquote(fragment)
            if not pointer.startswith("/"):
                return [f"{path}: 外部$ref fragmentがJSON Pointerではない {reference!r}"]
            target: object = external_schema
            for part in pointer[1:].split("/"):
                if not isinstance(target, dict):
                    return [f"{path}: 外部$ref {reference!r}を解決できない"]
                target = target.get(part.replace("~1", "/").replace("~0", "~"))
            if not isinstance(target, dict):
                return [f"{path}: 外部$ref {reference!r}を解決できない"]
            return validate_instance(value, target, path, external_schema)
        target: object = root
        for part in reference[2:].split("/"):
            if not isinstance(target, dict):
                return [f"{path}: $ref {reference!r}を解決できない"]
            target = target.get(part.replace("~1", "/").replace("~0", "~"))
        if not isinstance(target, dict):
            return [f"{path}: $ref {reference!r}を解決できない"]
        return validate_instance(value, target, path, root)
    if "allOf" in schema:
        for branch in schema["allOf"]:
            errors.extend(validate_instance(value, branch, path, root))
    if "anyOf" in schema:
        if not any(not validate_instance(value, branch, path, root) for branch in schema["anyOf"]):
            errors.append(f"{path}: anyOfの一致がない")
    if "not" in schema:
        branch = schema["not"]
        if isinstance(branch, dict) and not validate_instance(value, branch, path, root):
            errors.append(f"{path}: notで拒否された値")
    conditional = schema.get("if")
    if isinstance(conditional, dict):
        condition_matches = not validate_instance(value, conditional, path, root)
        branch_name = "then" if condition_matches else "else"
        branch = schema.get(branch_name)
        if isinstance(branch, dict):
            errors.extend(validate_instance(value, branch, path, root))
    if "oneOf" in schema:
        一致数 = sum(not validate_instance(value, 分岐, path, root) for 分岐 in schema["oneOf"])
        if 一致数 != 1:
            errors.append(f"{path}: oneOfの一致数が1ではない")
    expected_type = schema.get("type")

    if isinstance(expected_type, list):
        if not any(type_matches(value, item) for item in expected_type):
            errors.append(f"{path}: 次のいずれかを期待: {expected_type}")
            return errors
    elif isinstance(expected_type, str):
        if not type_matches(value, expected_type):
            errors.append(f"{path}: 期待値: {expected_type}")
            return errors

    if "const" in schema and value != schema["const"]:
            errors.append(f"{path}: const {schema['const']!r}を期待")

    if "enum" in schema and value not in schema["enum"]:
            errors.append(f"{path}: value {value!r}がenumにない")

    if isinstance(value, str):
        if schema.get("format") == "date-time" and not is_date_time(value):
            errors.append(f"{path}: RFC3339 date-timeとして不正")
        if "minLength" in schema and len(value) < schema["minLength"]:
            errors.append(f"{path}: minLength {schema['minLength']}より短い")
        if "maxLength" in schema and len(value) > schema["maxLength"]:
            errors.append(f"{path}: maxLength を超過")
        if "pattern" in schema and re.match(schema["pattern"], value) is None:
            errors.append(f"{path}: pattern {schema['pattern']}と一致しない")

    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in schema and value < schema["minimum"]:
            errors.append(f"{path}: minimum {schema['minimum']}未満")

        if "maximum" in schema and value > schema["maximum"]:
            errors.append(f"{path}: maximum を超過")

    if isinstance(value, list):
        if "minItems" in schema and len(value) < schema["minItems"]:
            errors.append(f"{path}: minItems {schema['minItems']}より少ない")
        if "maxItems" in schema and len(value) > schema["maxItems"]:
            errors.append(f"{path}: maxItems を超過")
        item_schema = schema.get("items")
        if isinstance(item_schema, dict):
            for index, item in enumerate(value):
                errors.extend(validate_instance(item, item_schema, f"{path}[{index}]", root))
        contains_schema = schema.get("contains")
        if isinstance(contains_schema, dict):
            matching = sum(
                not validate_instance(item, contains_schema, f"{path}[{index}]", root)
                for index, item in enumerate(value)
            )
            minimum = schema.get("minContains", 1)
            maximum = schema.get("maxContains")
            if matching < minimum or (maximum is not None and matching > maximum):
                errors.append(f"{path}: containsの一致数が範囲外")

    if isinstance(value, dict):
        required = schema.get("required", [])
        for key in required:
            if key not in value:
                errors.append(f"{path}: 必須key {key}がない")

        properties = schema.get("properties", {})
        additional = schema.get("additionalProperties", True)
        property_names = schema.get("propertyNames")
        if isinstance(property_names, dict):
            for key in value:
                errors.extend(validate_instance(key, property_names, f"{path}.<key:{key}>", root))

        if additional is False:
            for key in value:
                if key not in properties:
                    errors.append(f"{path}: 追加property {key}は許可されない")

        for key, item in value.items():
            if key in properties:
                errors.extend(validate_instance(item, properties[key], f"{path}.{key}", root))
            elif isinstance(additional, dict):
                errors.extend(validate_instance(item, additional, f"{path}.{key}", root))

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
            errors.append(f"schemaがない: {name}")

    schemas: dict[str, dict] = {}
    for path in sorted(SPECS.glob("*.schema.json")):
        data, err = load_json(path)
        if err:
            errors.append(f"{path}: 無効なJSON: {err}")
            continue
        if not isinstance(data, dict):
            errors.append(f"{path}: schema rootはobjectでなければならない")
            continue
        schemas[path.name] = data
        for key in ["$schema", "$id", "title", "type"]:
            if key not in data:
                errors.append(f"{path}: {key}がない")

    for schema_name in sorted(REQUIRED):
        schema = schemas.get(schema_name)
        if not schema:
            continue
        example_path = valid_example_path(schema_name)
        example, err = load_json(example_path)
        if err:
            errors.append(f"{example_path}: 有効exampleが無効または欠落: {err}")
            continue
        for failure in validate_instance(example, schema):
            errors.append(f"{example_path}: {failure}")

    mobile_agent_schema_name = "mobile_agent_list.schema.json"
    mobile_agent_example, mobile_agent_example_error = load_json(
        valid_example_path(mobile_agent_schema_name)
    )
    mobile_agent_schema = schemas.get(mobile_agent_schema_name)
    if (
        mobile_agent_schema
        and mobile_agent_example_error is None
        and isinstance(mobile_agent_example, dict)
        and isinstance(mobile_agent_example.get("Agent"), list)
        and mobile_agent_example["Agent"]
        and isinstance(mobile_agent_example["Agent"][0], dict)
    ):
        mobile_agent = mobile_agent_example["Agent"][0]
        oversized_list = {"Agent": [deepcopy(mobile_agent) for _ in range(65)]}
        if not validate_instance(oversized_list, mobile_agent_schema):
            errors.append("mobile_agent_list.schema.json: maxItems 64の超過を拒否しない")

        oversized_field = deepcopy(mobile_agent_example)
        oversized_field["Agent"][0]["version"] = "v" * 257
        if not validate_instance(oversized_field, mobile_agent_schema):
            errors.append("agent_adapter.schema.json: maxLength 256の超過を拒否しない")

    invalid_count = 0
    invalid_schema_names: set[str] = set()
    for invalid_path in sorted(INVALID_EXAMPLES.glob("*.invalid.json")):
        invalid_count += 1
        schema_name = schema_name_from_invalid_fixture(invalid_path)
        invalid_schema_names.add(schema_name)
        schema = schemas.get(schema_name)
        if not schema:
            errors.append(f"{invalid_path}: schema {schema_name}を解決できない")
            continue
        instance, err = load_json(invalid_path)
        if err:
            errors.append(f"{invalid_path}: 無効なJSON: {err}")
            continue
        failures = validate_instance(instance, schema)
        if not failures:
            errors.append(f"{invalid_path}: 無効fixtureが予期せず{schema_name}に合格")

    missing_invalid = sorted(REQUIRED - invalid_schema_names)
    for schema_name in missing_invalid:
        errors.append(f"{schema_name}のnegative fixtureがない")

    if errors:
        print("schema checkが失敗:")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(f"schema checkが合格: schema {len(schemas)}件、example {len(REQUIRED)}件、negative fixture {invalid_count}件")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
