import 'dart:convert';
import 'dart:io';

import '../screens/shared.dart';

const String kSurfaceSemanticsExportPathEnv =
    'GUI_SHELL_SURFACE_SEMANTICS_EXPORT_JSON';

const List<String> kRequiredSurfaceSemanticsLabels = [
  'Dashboard',
  'NavigationRail',
  'Runtime Status',
  'Invariant Status',
];

Future<void> writeSurfaceSemanticsExportIfRequested({
  Map<String, String>? environment,
}) async {
  final env = environment ?? Platform.environment;
  final exportPath = env[kSurfaceSemanticsExportPathEnv];
  if (exportPath == null || exportPath.trim().isEmpty) {
    return;
  }
  final export = buildSurfaceSemanticsExport(path: exportPath);
  final output = File(exportPath);
  await output.parent.create(recursive: true);
  await output.writeAsString(
    const JsonEncoder.withIndent('  ').convert(export),
  );
}

Map<String, Object?> buildSurfaceSemanticsExport({String path = ''}) {
  // build時の登録履歴。描画・可視性・現在のSemantics treeは観測していない。
  final registered = SurfaceSemanticsRegistry.observed;
  return {
    'source': 'flutter_semantics_runtime_export',
    'path': path,
    'captured_at': DateTime.now().toUtc().toIso8601String(),
    'process_id': pid,
    'evidence_class': 'INTERNAL_STATE',
    'expected_surfaces': kRequiredSurfaceSemanticsLabels,
    'registered_surfaces': registered.keys.toList(),
    'registered_identifiers': registered,
    'visible_surfaces': <String>[],
    'surface_matches': <String, Object?>{},
    'aggregate_surface_shortcut_detected': false,
    'surface_match_requirements_met': false,
    'diagnostic_tree': {
      'mode': 'flutter_dart_surface_semantics_runtime_export',
      'observed_element_count': 0,
      'observed_elements': <Map<String, Object?>>[],
      'tree_edges': <Map<String, Object?>>[],
      'capture_limit': 'build_registry_only',
      'failure_diagnostic': true,
    },
    'evidence_source': {
      'source_kind': 'installed_app_flutter_semantics_export',
      'product_generated': true,
      'synthetic': false,
      'collector_derives_surfaces': false,
      'visibility_measured': false,
      'formal_release_input': false,
    },
  };
}
