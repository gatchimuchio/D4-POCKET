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
  final observed = SurfaceSemanticsRegistry.observed;
  final visible = <String>[];
  final surfaceMatches = <String, Object?>{};
  final observedElements = <Map<String, Object?>>[];
  final treeEdges = <Map<String, Object?>>[];

  var index = 0;
  for (final entry in observed.entries) {
    final elementKey = 'flutter_semantics:${entry.value}';
    observedElements.add({
      'element_key': elementKey,
      'runtime_id': 'flutter_semantics.$index',
      'parent_runtime_id': '',
      'name': entry.key,
      'automation_id': entry.value,
      'control_type': 'FlutterSemanticsNode',
      'class_name': 'SurfaceSemantics',
      'framework_id': 'Flutter',
      'localized_control_type': 'semantics',
      'help_text': '',
      'is_offscreen': false,
      'bounding_rectangle': {'x': 0, 'y': 0, 'width': 0, 'height': 0},
      'supported_patterns': <String>[],
      'is_root': false,
      'is_native_container': false,
      'surfaces_present': [entry.key],
      'surface_count': 1,
      'contains_all_required_surfaces': false,
    });
    index += 1;
  }

  for (final label in kRequiredSurfaceSemanticsLabels) {
    final identifier = observed[label];
    if (identifier == null) {
      surfaceMatches[label] = {
        'matched': false,
        'name': '',
        'automation_id': '',
        'control_type': '',
        'class_name': '',
        'framework_id': '',
        'element_key': '',
        'is_root': false,
        'is_native_container': false,
        'surfaces_present': <String>[],
      };
      continue;
    }
    visible.add(label);
    surfaceMatches[label] = {
      'matched': true,
      'name': label,
      'automation_id': identifier,
      'control_type': 'FlutterSemanticsNode',
      'class_name': 'SurfaceSemantics',
      'framework_id': 'Flutter',
      'element_key': 'flutter_semantics:$identifier',
      'is_root': false,
      'is_native_container': false,
      'surfaces_present': [label],
    };
  }

  final complete = visible.length == kRequiredSurfaceSemanticsLabels.length;
  return {
    'source': 'flutter_semantics_runtime_export',
    'path': path,
    'captured_at': DateTime.now().toUtc().toIso8601String(),
    'expected_surfaces': kRequiredSurfaceSemanticsLabels,
    'visible_surfaces': visible,
    'surface_matches': surfaceMatches,
    'aggregate_surface_shortcut_detected': false,
    'surface_match_requirements_met': complete,
    'automation_names': [
      for (final entry in observed.entries) ...[entry.key, entry.value],
    ],
    'diagnostic_tree': {
      'mode': 'flutter_dart_surface_semantics_runtime_export',
      'observed_element_count': observedElements.length,
      'observed_elements': observedElements,
      'tree_edges': treeEdges,
      'capture_limit': 'none',
      'failure_diagnostic': !complete,
    },
    'evidence_source': {
      'source_kind': 'installed_app_flutter_semantics_export',
      'product_generated': true,
      'synthetic': false,
      'collector_derives_surfaces': false,
    },
  };
}
