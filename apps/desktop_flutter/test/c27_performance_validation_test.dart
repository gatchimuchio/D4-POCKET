import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/models/generated_contracts.dart';
import 'package:gui_shell_desktop/screens/workspace_diff_panel.dart';
import 'package:gui_shell_desktop/services/global_search_index.dart';
import 'package:gui_shell_desktop/services/notification_client.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_resource_client.dart'
    show BrokerTransport, RuntimeResourceClient;

const _watchdog = Duration(seconds: 5);

void main() {
  test('C27のsnapshot・一覧projection・検索・通知・資源観測をboundedに測定する', () async {
    final timings = <String, int>{};
    final base = ShellCoreClient.mock().getSnapshot();

    _measure(timings, 'startup_model_initialization', () {
      ShellCoreClient.mock().getSnapshot();
    });

    final snapshotJson = _snapshotJson(512);
    final encoded = jsonEncode(snapshotJson);
    _measure(timings, 'snapshot_load', () {
      final parsed = ShellSnapshot.fromJson(
        Map<String, Object?>.from(jsonDecode(encoded) as Map),
      );
      expect(parsed.runtimes, hasLength(512));
    });

    final scaled = _scaledSnapshot(base, 512);
    _measure(timings, 'runtime_audit_history_projection', () {
      final entries = GlobalSearchIndex.build(scaled);
      expect(entries.length, lessThanOrEqualTo(GlobalSearchIndex.maxEntries));
    });
    _measure(timings, 'global_search', () {
      final results = GlobalSearchIndex.search(base, 'MCP');
      expect(results, isNotEmpty);
      expect(results.length, lessThanOrEqualTo(GlobalSearchIndex.maxResults));
    });

    final transport = _C27Transport();
    final notifications = NotificationClient(transport);
    final resource = RuntimeResourceClient(transport);
    await _measureAsync(timings, 'notification_count', () async {
      final body = await notifications.list(limit: 200).timeout(_watchdog);
      expect(body['件数'], 200);
    });
    await _measureAsync(timings, 'resource_polling', () async {
      final observation =
          await resource.observe('runtime-a').timeout(_watchdog);
      expect(observation.metrics, hasLength(12));
      expect(observation.binding.isBound, isFalse);
    });

    for (final entry in timings.entries) {
      // これは製品SLAではなく、ローカル開発検証でのハング監視である。
      expect(entry.value, lessThan(5000), reason: '${entry.key}がwatchdogを超過');
      debugPrint('PERF_C27|${entry.key}|${entry.value}ms|開発監視');
    }
  });

  testWidgets('C27の巨大差分はListView遅延描画でUI watchdog内に収まる', (tester) async {
    final diff = _largeTextDiff(4096);
    final stopwatch = Stopwatch()..start();
    await tester
        .pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: WorkspaceDiffPanel(
                diff: diff,
                paired: true,
                onPairedChanged: (_) {},
              ),
            ),
          ),
        )
        .timeout(_watchdog);
    await tester.pump().timeout(_watchdog);
    stopwatch.stop();
    expect(find.text('テキスト差分'), findsOneWidget);
    expect(stopwatch.elapsedMilliseconds, lessThan(5000));
    debugPrint(
      'PERF_C27|large_diff_ui|${stopwatch.elapsedMilliseconds}ms|開発監視',
    );
  });
}

void _measure(Map<String, int> timings, String name, void Function() action) {
  final stopwatch = Stopwatch()..start();
  action();
  stopwatch.stop();
  timings[name] = stopwatch.elapsedMilliseconds;
}

Future<void> _measureAsync(
  Map<String, int> timings,
  String name,
  Future<void> Function() action,
) async {
  final stopwatch = Stopwatch()..start();
  await action();
  stopwatch.stop();
  timings[name] = stopwatch.elapsedMilliseconds;
}

ShellSnapshot _scaledSnapshot(ShellSnapshot base, int count) {
  return ShellSnapshot(
    phaseStatus: base.phaseStatus,
    operationStatus: base.operationStatus,
    runtimes: List<RuntimeRecord>.generate(
      count,
      (index) => RuntimeRecord(
        runtimeId: 'runtime-$index',
        name: 'Runtime $index',
        status: 'ready',
        adapterId: 'adapter-reference',
        diagnosticSummary: '上限付き性能検証用データ',
      ),
    ),
    agentSessions: base.agentSessions,
    permissions: base.permissions,
    pendingApprovals: base.pendingApprovals,
    auditEvents: base.auditEvents,
    recoveryActions: base.recoveryActions,
    invariantFlags: base.invariantFlags,
    setupDoctorChecks: base.setupDoctorChecks,
    setupDoctorStatus: base.setupDoctorStatus,
    installerGrantsAuthority: base.installerGrantsAuthority,
    installerSilentlyApprovesPermissions:
        base.installerSilentlyApprovesPermissions,
    trustRecords: base.trustRecords,
    authorityMap: base.authorityMap,
    adapterCatalog: base.adapterCatalog,
    permissionDiffs: base.permissionDiffs,
    problems: base.problems,
    evidence: base.evidence,
    settings: base.settings,
    auditChainStatus: base.auditChainStatus,
    networkExposure: base.networkExposure,
    releaseBlockerCount: base.releaseBlockerCount,
    evidenceSummary: base.evidenceSummary,
    recoveryPlaybook: base.recoveryPlaybook,
    hosts: base.hosts,
    hostCapabilities: base.hostCapabilities,
    snapshotSource: base.snapshotSource,
    snapshotPath: base.snapshotPath,
    snapshotGeneratedAt: base.snapshotGeneratedAt,
    snapshotFreshness: base.snapshotFreshness,
  );
}

Map<String, Object?> _snapshotJson(int runtimeCount) => {
      'phase_status': <String, Object?>{},
      'runtimes': List.generate(
        runtimeCount,
        (index) => {
          'runtime_id': 'runtime-$index',
          'name': 'Runtime $index',
          'status': 'ready',
          'adapter_id': 'adapter-reference',
          'diagnostic_summary': '上限付き性能検証用データ',
        },
      ),
      'snapshot_source': 'performance_fixture',
      'snapshot_path': 'development-only',
      'snapshot_generated_at': '2026-09-24T00:00:00Z',
      'snapshot_freshness': 'generated',
    };

Map<String, Object?> _largeTextDiff(int rows) => {
      'kind': 'text',
      'before': {'bytes': rows * 8, 'sha256': 'sha256:${'a' * 64}'},
      'after': {'bytes': rows * 8, 'sha256': 'sha256:${'b' * 64}'},
      'unified': '大規模差分の統合表示はboundedな検証用本文',
      'rows': List.generate(
        rows,
        (index) => {
          'kind': 'changed',
          'before': {'number': index + 1, 'text': '前 $index', 'newline': true},
          'after': {'number': index + 1, 'text': '後 $index', 'newline': true},
        },
      ),
    };

class _C27Transport implements BrokerTransport {
  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    if (operation == '通知一覧') {
      final items = List.generate(
        200,
        (index) => <String, Object?>{
          '通知ID': 'notification-$index',
          '通知hash': 'sha256:${'a' * 64}',
          'タイトル': '通知 $index',
          '概要': 'bounded summary',
          'severity': 'info',
          '状態': index.isEven ? 'unread' : 'read',
          'source': 'audit',
          '関連監査ID': 'audit-$index',
          '遷移先': 'dashboard',
        },
      );
      return {
        'status': 'accepted',
        'body': {
          '版': 1,
          '通知一覧': items,
          '件数': items.length,
          '未読件数': 100,
          '重大件数': 0,
          '証拠種別': 'INTERNAL_STATE',
          '表示範囲': 'summary',
          '権限生成': 'なし',
          '操作': 'navigation_only',
        },
      };
    }
    if (operation == '実行系資源観測') {
      final unknownMetrics = <String, Object?>{
        for (final key in const [
          '稼働時間Millis',
          'CPU累積時間Millis',
          'CPU利用率Percent',
          'RAMWorkingSetBytes',
          'RAMPrivateBytes',
          'DiskIOBytes',
          'NetworkIOBytes',
          'GPU利用率Percent',
          'VRAMBytes',
          '処理中要求数',
          '平均応答Millis',
          '失敗要求数',
        ])
          key: {
            '状態': 'unknown',
            '値': null,
            '証拠種別': 'INTERNAL_STATE',
            '理由': '性能fixtureでは実行系が未接続',
          },
      };
      return {
        'request_id': 'resource-request-1',
        'operation': operation,
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'resource-audit-1',
        'error': null,
        'health': null,
        'shutdown_requested': false,
        'body': {
          '版': 1,
          '実行系ID': 'runtime-a',
          '観測時刻UnixMillis': 1700000001000,
          '観測監査ID': 'resource-audit-1',
          '結合': {
            '状態': 'unbound',
            '根拠': 'loopback_tcp_listener_owner_pid',
            'PID': null,
            'PID作成時刻UnixMillis': null,
            '登録時刻UnixMillis': 1700000000000,
            '登録監査ID': 'resource-registration-audit-1',
            '理由': '性能fixtureでは実行系が未接続',
          },
          '統治': {
            '能力ID': 'runtime.resource.observe',
            '権限ID': 'permission.runtime.resource.observe',
            '承認状態': 'not_required',
            '復旧ID': 'recover-runtime-resource-binding',
          },
          '計測': unknownMetrics,
          '短期履歴': <Object?>[],
        },
      };
    }
    throw StateError('C27 fixtureは未対応操作を拒否する: $operation');
  }
}
