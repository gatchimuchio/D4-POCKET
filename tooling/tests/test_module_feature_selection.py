from __future__ import annotations

import tomllib
import unittest
from pathlib import Path

from tooling.build_module_pruned_windows import (
    MODULE_CARGO_FEATURE_NAMES,
    ResolvedModulePlan,
    _load_catalog,
    _validate_dart_define_bindings,
    cargo_feature_arguments,
)


ROOT = Path(__file__).resolve().parents[2]


def _plan(*included: str) -> ResolvedModulePlan:
    return ResolvedModulePlan(
        selection_mode="explicit_optional",
        requested_optional_ids=tuple(included),
        included_optional_ids=tuple(included),
        included_module_ids=tuple(included),
        excluded_optional_ids=(),
    )


class ModuleFeatureSelectionTests(unittest.TestCase):
    def test_cargo_manifest_declares_every_catalog_feature_and_full_default(self) -> None:
        manifest = tomllib.loads(
            (ROOT / "native/rust_helper/Cargo.toml").read_text(encoding="utf-8")
        )
        features = manifest["features"]
        expected = set(MODULE_CARGO_FEATURE_NAMES.values())

        self.assertEqual(set(features["default"]), expected)
        self.assertTrue(expected.issubset(features))
        self.assertEqual(features["module_trace_inspector"], ["module_observability"])

    def test_dart_declarations_and_real_surface_references_match_the_catalog(self) -> None:
        _validate_dart_define_bindings(_load_catalog())

    def test_windows_selected_features_disable_defaults_and_include_only_selection(self) -> None:
        arguments = cargo_feature_arguments(
            _plan("shell.notifications", "shell.observability"),
            target_platform="windows",
        )

        self.assertEqual(
            arguments,
            [
                "--no-default-features",
                "--features",
                "module_notifications,module_observability",
            ],
        )
        self.assertEqual(
            cargo_feature_arguments(_plan(), target_platform="windows"),
            ["--no-default-features"],
        )

    def test_trace_feature_selection_requires_and_includes_observability(self) -> None:
        with self.assertRaisesRegex(ValueError, "依存を満たさない"):
            cargo_feature_arguments(
                _plan("shell.trace_inspector"), target_platform="macos"
            )

        arguments = cargo_feature_arguments(
            _plan("shell.observability", "shell.trace_inspector"),
            target_platform="macos",
        )
        self.assertEqual(
            arguments,
            [
                "--no-default-features",
                "--features",
                "module_observability,module_trace_inspector",
            ],
        )

    def test_linux_keeps_native_broker_feature_and_limits_test_feature(self) -> None:
        self.assertEqual(
            cargo_feature_arguments(_plan(), target_platform="linux"),
            ["--no-default-features", "--features", "linux-desktop-broker"],
        )
        self.assertEqual(
            cargo_feature_arguments(
                _plan("shell.notifications"),
                target_platform="linux",
                r2_e2e_test_build=True,
            ),
            [
                "--no-default-features",
                "--features",
                "linux-desktop-broker,module_notifications,r2-e2e",
            ],
        )
        with self.assertRaisesRegex(ValueError, "Linux検証buildだけ"):
            cargo_feature_arguments(
                _plan(), target_platform="windows", r2_e2e_test_build=True
            )


if __name__ == "__main__":
    unittest.main()
