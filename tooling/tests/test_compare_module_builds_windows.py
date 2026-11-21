from __future__ import annotations

import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from tooling.build_environment import build_child_environment
from tooling.build_module_pruned_windows import (
    EXPECTED_CORE_IDS,
    EXPECTED_REQUIRED_IDS,
    _load_catalog,
    resolve_module_plan_for_optional_ids,
)
from tooling.compare_module_builds_windows import (
    COMPARISON_EVIDENCE_VERSION,
    _load_selection_input,
    _aot_package_libraries,
    _parse_linker_map_symbols,
    _pruning_variant_plans,
    _runtime_dependency_tree_from_json,
    package_dependency_comparison,
    resource_inventory_comparison,
    _symbol_inventory_record,
    symbol_set_comparison,
    validate_comparison_evidence,
)


ROOT = Path(__file__).resolve().parents[2]


class ModuleComparisonMeasurementTests(unittest.TestCase):
    def test_evidence_producer_version_matches_schema(self) -> None:
        schema = json.loads(
            (ROOT / "specs/gui_shell_module_comparison_evidence.schema.json")
            .read_text(encoding="utf-8")
        )
        self.assertEqual(
            COMPARISON_EVIDENCE_VERSION,
            schema["properties"]["version"]["const"],
        )

    def test_pruning_variants_cover_required_distinct_optional_configurations(self) -> None:
        catalog = _load_catalog()
        variants = _pruning_variant_plans(catalog)
        optional_ids = {module["module_id"] for module in catalog["optional_modules"]}

        self.assertEqual(
            [label for label, _plan, _omitted in variants],
            ["no_optional_modules", "without_setup_doctor", "without_history"],
        )
        configurations = [
            tuple(module["module_id"] for module in catalog["optional_modules"]),
            *(plan.included_optional_ids for _label, plan, _omitted in variants),
        ]
        self.assertEqual(len(configurations), len(set(configurations)))
        for _label, plan, omitted in variants:
            self.assertEqual(set(plan.included_optional_ids) | set(omitted), optional_ids)
            self.assertFalse(set(plan.included_optional_ids) & set(omitted))
            self.assertEqual(
                plan.included_module_ids[: len(EXPECTED_CORE_IDS)], EXPECTED_CORE_IDS
            )
            self.assertEqual(
                plan.included_module_ids[
                    len(EXPECTED_CORE_IDS) : len(EXPECTED_CORE_IDS) + len(EXPECTED_REQUIRED_IDS)
                ],
                EXPECTED_REQUIRED_IDS,
            )
        self.assertEqual(variants[0][1].included_optional_ids, ())

    def test_optional_plan_resolver_applies_dependency_closure_and_rejects_unknown_ids(self) -> None:
        catalog = _load_catalog()
        plan = resolve_module_plan_for_optional_ids(["shell.trace_inspector"], catalog)
        self.assertEqual(
            plan.included_optional_ids,
            ("shell.observability", "shell.trace_inspector"),
        )
        self.assertEqual(plan.excluded_optional_ids[0], "shell.setup_doctor")
        with self.assertRaisesRegex(ValueError, "未知の任意Module"):
            resolve_module_plan_for_optional_ids(["shell.unknown"], catalog)

    def test_build_environment_filters_credentials_and_applies_case_insensitive_overrides(self) -> None:
        environment = build_child_environment(
            {"PATH": "approved-path", "CARGO_HOME": "C:\\cargo"},
            inherited={
                "path": "untrusted-old-path",
                "GITHUB_TOKEN": "must-not-pass",
                "USERPROFILE": "C:\\Users\\tester",
                "CARGO_HOME": "must-not-pass-through",
            },
        )
        self.assertEqual(environment["PATH"], "approved-path")
        self.assertEqual(environment["CARGO_HOME"], "C:\\cargo")
        self.assertEqual(environment["USERPROFILE"], "C:\\Users\\tester")
        self.assertNotIn("path", environment)
        self.assertNotIn("GITHUB_TOKEN", environment)

    def test_linker_map_extracts_unique_public_symbols_only(self) -> None:
        raw = b"""Start Length Name Class
Address Publics by Value Rva+Base Lib:Object
 0001:00000000 ?task_entry@@YAXXZ 0000000140001000 f helper.obj
 0001:00000010 ?broker_call@@YAXXZ 0000000140001010 f helper.obj
 0001:00000010 ?broker_call@@YAXXZ 0000000140001010 f helper.obj
Static symbols
 0001:00000020 local_symbol 0000000140001020 f helper.obj
"""
        self.assertEqual(
            _parse_linker_map_symbols(raw),
            ["?broker_call@@YAXXZ", "?task_entry@@YAXXZ"],
        )

    def test_linker_map_without_public_symbols_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "Public symbolを採取できない"):
            _parse_linker_map_symbols(b"Start Length Name Class\nStatic symbols\n")

    def test_symbol_comparison_reports_removed_and_added_names(self) -> None:
        self.assertEqual(
            symbol_set_comparison(["core", "optional_old"], ["core", "optional_new"]),
            {"removed_symbols": ["optional_old"], "added_symbols": ["optional_new"]},
        )

    def test_symbol_inventory_record_matches_nested_contract_shape(self) -> None:
        symbol_file = {
            "path": "baseline/rust/public-symbols.txt",
            "size_bytes": 12,
            "sha256": "a" * 64,
        }
        self.assertEqual(
            _symbol_inventory_record(symbol_file, ["symbol_a", "symbol_b"]),
            {"file": symbol_file, "symbol_count": 2},
        )

    def test_resource_comparison_uses_packaged_flutter_asset_files(self) -> None:
        baseline = [
            {"path": "data/app.so", "size_bytes": 100, "sha256": "a" * 64},
            {
                "path": "data/flutter_assets/FontManifest.json",
                "size_bytes": 82,
                "sha256": "b" * 64,
            },
        ]
        selected = [
            {"path": "data/app.so", "size_bytes": 90, "sha256": "c" * 64},
            {
                "path": "data/flutter_assets/FontManifest.json",
                "size_bytes": 82,
                "sha256": "b" * 64,
            },
            {
                "path": "data/flutter_assets/extra.bin",
                "size_bytes": 4,
                "sha256": "d" * 64,
            },
        ]

        self.assertEqual(
            resource_inventory_comparison(baseline, selected),
            {
                "baseline_resource_bytes": 82,
                "selected_resource_bytes": 86,
                "changed_resource_paths": ["data/flutter_assets/extra.bin"],
                "resource_inventory_identical": False,
            },
        )

    def test_runtime_dependency_tree_uses_root_runtime_closure_only(self) -> None:
        deps = {
            "root": "gui_shell_desktop",
            "packages": [
                {
                    "name": "gui_shell_desktop",
                    "kind": "root",
                    "directDependencies": ["flutter", "gui_shell_ui"],
                    "devDependencies": ["flutter_test"],
                },
                {
                    "name": "flutter",
                    "version": "0.0.0",
                    "source": "sdk",
                    "dependencies": ["characters"],
                },
                {
                    "name": "gui_shell_ui",
                    "version": "0.1.0",
                    "source": "path",
                    "dependencies": ["broker_contract", "flutter"],
                },
                {
                    "name": "broker_contract",
                    "version": "0.1.0",
                    "source": "path",
                    "dependencies": [],
                },
                {
                    "name": "characters",
                    "version": "1.0.0",
                    "source": "hosted",
                    "dependencies": [],
                },
                {
                    "name": "flutter_test",
                    "version": "0.0.0",
                    "source": "sdk",
                    "dependencies": [],
                },
            ],
        }

        tree = _runtime_dependency_tree_from_json(deps)

        self.assertEqual(tree["runtime_package_count"], 4)
        self.assertEqual(
            [item["name"] for item in tree["runtime_packages"]],
            ["broker_contract", "characters", "flutter", "gui_shell_ui"],
        )
        self.assertNotIn("flutter_test", {item["name"] for item in tree["runtime_packages"]})
        self.assertEqual(len(tree["runtime_tree_sha256"]), 64)

    def test_runtime_dependency_tree_rejects_missing_transitive_package(self) -> None:
        deps = {
            "root": "app",
            "packages": [
                {"name": "app", "kind": "root", "directDependencies": ["missing"]}
            ],
        }
        with self.assertRaisesRegex(ValueError, "runtime依存packageがpub deps graphに存在しない"):
            _runtime_dependency_tree_from_json(deps)

    def test_runtime_dependency_tree_rejects_duplicate_package_nodes(self) -> None:
        deps = {
            "root": "app",
            "packages": [
                {"name": "app", "kind": "root", "directDependencies": ["flutter"]},
                {"name": "flutter", "version": "0.0.0", "source": "sdk", "dependencies": []},
                {"name": "flutter", "version": "0.0.0", "source": "sdk", "dependencies": []},
            ],
        }
        with self.assertRaisesRegex(ValueError, "重複package node"):
            _runtime_dependency_tree_from_json(deps)

    def test_package_comparison_reports_graph_and_aot_package_changes(self) -> None:
        baseline = {
            "runtime_dependency_tree": {"runtime_tree_sha256": "a" * 64},
            "aot_package_libraries": ["package:shared/core.dart", "package:optional/page.dart"],
        }
        selected = {
            "runtime_dependency_tree": {"runtime_tree_sha256": "a" * 64},
            "aot_package_libraries": ["package:shared/core.dart"],
        }

        self.assertEqual(
            package_dependency_comparison(baseline, selected),
            {
                "baseline_runtime_tree_sha256": "a" * 64,
                "selected_runtime_tree_sha256": "a" * 64,
                "runtime_package_tree_identical": True,
                "removed_aot_package_libraries": ["package:optional/page.dart"],
                "added_aot_package_libraries": [],
            },
        )

    def test_aot_package_library_inventory_is_unique_and_sorted(self) -> None:
        report = {
            "n": "report",
            "children": [
                {
                    "n": "app.so (Dart AOT)",
                    "children": [
                        {
                            "n": "package:gui_shell_ui",
                            "children": [
                                {
                                    "n": "package:gui_shell_ui/src",
                                    "children": [
                                        {"n": "runtime_dialogue_client.dart"},
                                        {"n": "history_client.dart"},
                                    ],
                                }
                            ],
                        },
                        {"n": "package:flutter/material.dart"},
                        {"n": "package:flutter/material.dart"},
                    ],
                }
            ],
        }
        self.assertEqual(
            _aot_package_libraries(report),
            [
                "package:flutter/material.dart",
                "package:gui_shell_ui/src/history_client.dart",
                "package:gui_shell_ui/src/runtime_dialogue_client.dart",
            ],
        )

    def test_schema_validation_rejects_dependency_tree_tampering(self) -> None:
        evidence = json.loads(
            (ROOT / "examples/contracts/gui_shell_module_comparison_evidence.valid.json")
            .read_text(encoding="utf-8")
        )
        schema = json.loads(
            (ROOT / "specs/gui_shell_module_comparison_evidence.schema.json")
            .read_text(encoding="utf-8")
        )
        self.assertEqual(
            validate_comparison_evidence(evidence, schema),
            [],
        )

        tampered = copy.deepcopy(evidence)
        tampered["selected"]["runtime_dependency_tree"]["runtime_packages"][0][
            "version"
        ] = "tampered"
        errors = validate_comparison_evidence(tampered, schema)
        self.assertTrue(any("dependency tree hash" in error for error in errors))

    def test_schema_validation_rejects_native_binary_metric_tampering(self) -> None:
        evidence = json.loads(
            (ROOT / "examples/contracts/gui_shell_module_comparison_evidence.valid.json")
            .read_text(encoding="utf-8")
        )
        schema = json.loads(
            (ROOT / "specs/gui_shell_module_comparison_evidence.schema.json")
            .read_text(encoding="utf-8")
        )
        evidence["rust_binary_comparison"]["comparison"]["binary_bytes_reduced"] = 0
        errors = validate_comparison_evidence(evidence, schema)
        self.assertTrue(any("byte削減量" in error for error in errors))

    def test_schema_validation_rejects_native_command_feature_mismatch(self) -> None:
        evidence = json.loads(
            (ROOT / "examples/contracts/gui_shell_module_comparison_evidence.valid.json")
            .read_text(encoding="utf-8")
        )
        schema = json.loads(
            (ROOT / "specs/gui_shell_module_comparison_evidence.schema.json")
            .read_text(encoding="utf-8")
        )
        evidence["rust_binary_comparison"]["selected"]["cargo_command"] = [
            "module_history" if item == "module_observability,module_trace_inspector" else item
            for item in evidence["rust_binary_comparison"]["selected"]["cargo_command"]
        ]
        errors = validate_comparison_evidence(evidence, schema)
        self.assertTrue(any("build command" in error for error in errors))

    def test_schema_validation_rejects_launcher_command_feature_mismatch(self) -> None:
        evidence = json.loads(
            (ROOT / "examples/contracts/gui_shell_module_comparison_evidence.valid.json")
            .read_text(encoding="utf-8")
        )
        schema = json.loads(
            (ROOT / "specs/gui_shell_module_comparison_evidence.schema.json")
            .read_text(encoding="utf-8")
        )
        evidence["rust_binary_comparison"]["selected"][
            "launcher_cargo_command"
        ] = [
            "module_history"
            if item == "module_observability,module_trace_inspector"
            else item
            for item in evidence["rust_binary_comparison"]["selected"][
                "launcher_cargo_command"
            ]
        ]
        errors = validate_comparison_evidence(evidence, schema)
        self.assertTrue(any("launcher build command" in error for error in errors))

    def test_schema_validation_rejects_desktop_launcher_measurement_mismatch(self) -> None:
        evidence = json.loads(
            (ROOT / "examples/contracts/gui_shell_module_comparison_evidence.valid.json")
            .read_text(encoding="utf-8")
        )
        schema = json.loads(
            (ROOT / "specs/gui_shell_module_comparison_evidence.schema.json")
            .read_text(encoding="utf-8")
        )
        evidence["rust_binary_comparison"]["desktop_launcher_comparison"][
            "binary_bytes_reduced"
        ] = 0
        errors = validate_comparison_evidence(evidence, schema)
        self.assertTrue(any("launcher byte削減量" in error for error in errors))

    def test_schema_validation_rejects_removed_security_core_or_required_module(self) -> None:
        schema = json.loads(
            (ROOT / "specs/gui_shell_module_comparison_evidence.schema.json")
            .read_text(encoding="utf-8")
        )
        example_path = ROOT / "examples/contracts/gui_shell_module_comparison_evidence.valid.json"
        for module_id in ("core.security_broker", "shell.settings"):
            with self.subTest(module_id=module_id):
                evidence = json.loads(example_path.read_text(encoding="utf-8"))
                evidence["rust_binary_comparison"]["always_included_module_ids"].remove(
                    module_id
                )
                errors = validate_comparison_evidence(evidence, schema)
                self.assertTrue(
                    any("always_included_module_ids" in error for error in errors)
                )

    def test_schema_validation_requires_distinct_measured_pruning_variants(self) -> None:
        schema = json.loads(
            (ROOT / "specs/gui_shell_module_comparison_evidence.schema.json")
            .read_text(encoding="utf-8")
        )
        example_path = ROOT / "examples/contracts/gui_shell_module_comparison_evidence.valid.json"
        evidence = json.loads(example_path.read_text(encoding="utf-8"))

        missing = copy.deepcopy(evidence)
        missing["pruning_variants"].pop()
        self.assertTrue(validate_comparison_evidence(missing, schema))

        tampered = copy.deepcopy(evidence)
        tampered["pruning_variants"][0]["included_optional_module_ids"].append(
            "shell.setup_doctor"
        )
        errors = validate_comparison_evidence(tampered, schema)
        self.assertTrue(any("包含Module一覧" in error for error in errors))

        mismatched_native = copy.deepcopy(evidence)
        mismatched_native["rust_binary_comparison"]["pruning_variants"][0]["run"][
            "included_optional_module_ids"
        ].append("shell.setup_doctor")
        errors = validate_comparison_evidence(mismatched_native, schema)
        self.assertTrue(any("Rust no_optional_modules" in error for error in errors))

    def test_manifest_file_is_a_supported_untrusted_comparison_selection(self) -> None:
        manifest_path = ROOT / "examples/contracts/gui_shell_export_manifest.valid.json"
        manifest_raw = manifest_path.read_bytes()
        manifest = json.loads(manifest_raw.decode("utf-8"))
        file_name = manifest["manifest"]["app_identity"]["app_id"] + ".json"
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / file_name
            path.write_bytes(manifest_raw)
            selection_input = _load_selection_input(manifest_path=path)

        self.assertEqual(
            selection_input.selection_input_trust,
            "unverified_manifest_json_selection_only",
        )
        self.assertIsNone(selection_input.source_receipt_sha256)
        self.assertEqual(
            selection_input.source_manifest_sha256,
            hashlib.sha256(manifest_raw).hexdigest(),
        )
        self.assertIsNone(selection_input.receipt)
        self.assertEqual(selection_input.manifest_file["export_id"], manifest["export_id"])

        evidence = json.loads(
            (ROOT / "examples/contracts/gui_shell_module_comparison_evidence.valid.json")
            .read_text(encoding="utf-8")
        )
        evidence["selection_input_trust"] = selection_input.selection_input_trust
        evidence["source_receipt_sha256"] = None
        evidence["source_manifest_sha256"] = selection_input.source_manifest_sha256
        schema = json.loads(
            (ROOT / "specs/gui_shell_module_comparison_evidence.schema.json")
            .read_text(encoding="utf-8")
        )
        self.assertEqual(
            validate_comparison_evidence(
                evidence, schema, selection_input=selection_input
            ),
            [],
        )
        evidence["authority_verified"] = True
        self.assertTrue(
            validate_comparison_evidence(
                evidence, schema, selection_input=selection_input
            )
        )

    def test_receipt_selection_remains_supported_and_binds_manifest_hash(self) -> None:
        receipt_path = ROOT / "examples/contracts/gui_shell_export_receipt.valid.json"
        selection_input = _load_selection_input(receipt_path=receipt_path)
        evidence = json.loads(
            (ROOT / "examples/contracts/gui_shell_module_comparison_evidence.valid.json")
            .read_text(encoding="utf-8")
        )
        evidence["source_receipt_sha256"] = selection_input.source_receipt_sha256
        evidence["source_manifest_sha256"] = selection_input.source_manifest_sha256
        schema = json.loads(
            (ROOT / "specs/gui_shell_module_comparison_evidence.schema.json")
            .read_text(encoding="utf-8")
        )
        self.assertEqual(
            validate_comparison_evidence(
                evidence, schema, selection_input=selection_input
            ),
            [],
        )
        evidence["source_manifest_sha256"] = "f" * 64
        self.assertTrue(
            validate_comparison_evidence(
                evidence, schema, selection_input=selection_input
            )
        )

    def test_manifest_selection_rejects_bad_name_and_inconsistent_module_plan(self) -> None:
        manifest_raw = (
            ROOT / "examples/contracts/gui_shell_export_manifest.valid.json"
        ).read_bytes()
        manifest = json.loads(manifest_raw.decode("utf-8"))
        expected_name = manifest["manifest"]["app_identity"]["app_id"] + ".json"
        with tempfile.TemporaryDirectory() as temporary:
            wrong_name = Path(temporary) / "other.json"
            wrong_name.write_bytes(manifest_raw)
            with self.assertRaisesRegex(ValueError, "file名がApp IDと一致"):
                _load_selection_input(manifest_path=wrong_name)

            changed = copy.deepcopy(manifest)
            changed["manifest"]["module_plan"]["included_module_ids"].remove(
                "shell.observability"
            )
            inconsistent = Path(temporary) / expected_name
            inconsistent.write_text(
                json.dumps(changed, ensure_ascii=False), encoding="utf-8"
            )
            with self.assertRaisesRegex(ValueError, "必須Moduleと依存閉包に一致しない"):
                _load_selection_input(manifest_path=inconsistent)

    def test_selection_input_requires_exactly_one_source(self) -> None:
        with self.assertRaisesRegex(ValueError, "どちらか一方"):
            _load_selection_input()
        with self.assertRaisesRegex(ValueError, "どちらか一方"):
            _load_selection_input(
                receipt_path=Path("receipt.json"), manifest_path=Path("manifest.json")
            )


if __name__ == "__main__":
    unittest.main()
