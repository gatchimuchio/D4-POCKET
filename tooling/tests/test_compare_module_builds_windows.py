from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from tooling.compare_module_builds_windows import (
    _aot_package_libraries,
    _runtime_dependency_tree_from_json,
    package_dependency_comparison,
    resource_inventory_comparison,
    validate_comparison_evidence,
)


ROOT = Path(__file__).resolve().parents[2]


class ModuleComparisonMeasurementTests(unittest.TestCase):
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


if __name__ == "__main__":
    unittest.main()
