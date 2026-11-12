import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/main.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

void main() {
  const allOptionalModulesEnabled = kGuiShellModuleSetupDoctor &&
      kGuiShellModuleHistory &&
      kGuiShellModuleEvaluationLab &&
      kGuiShellModuleHostCapabilities &&
      kGuiShellModuleNotifications &&
      kGuiShellModuleObservability &&
      kGuiShellModuleTraceInspector &&
      kGuiShellModuleHostOperations;
  const allOptionalModulesDisabled = !kGuiShellModuleSetupDoctor &&
      !kGuiShellModuleHistory &&
      !kGuiShellModuleEvaluationLab &&
      !kGuiShellModuleHostCapabilities &&
      !kGuiShellModuleNotifications &&
      !kGuiShellModuleObservability &&
      !kGuiShellModuleTraceInspector &&
      !kGuiShellModuleHostOperations;

  testWidgets('Compile-time Module選択が画面と選択済みconsumerに反映される', (tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());
    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    final labels = rail.destinations
        .map((destination) => (destination.label as Text).data)
        .whereType<String>()
        .toList(growable: false);

    expect(
      labels,
      containsAll([
        '概要',
        '信頼',
        '実行系',
        '権限',
        'エージェント',
        '承認',
        '監査',
        '復旧',
        '問題',
        '証拠',
        '設定',
        '対話',
        'A2A接続',
      ]),
    );

    const optionalModules = <MapEntry<String, bool>>[
      MapEntry('診断', kGuiShellModuleSetupDoctor),
      MapEntry('履歴', kGuiShellModuleHistory),
      MapEntry('評価ラボ', kGuiShellModuleEvaluationLab),
      MapEntry('ホスト能力', kGuiShellModuleHostCapabilities),
      MapEntry('通知', kGuiShellModuleNotifications),
      MapEntry('観測', kGuiShellModuleObservability),
      MapEntry('追跡', kGuiShellModuleTraceInspector),
      MapEntry('Host操作', kGuiShellModuleHostOperations),
    ];
    for (final module in optionalModules) {
      expect(labels.contains(module.key), module.value, reason: module.key);
    }
    expect(rail.destinations,
        hasLength(13 + optionalModules.where((module) => module.value).length));

    if (kGuiShellModuleObservability) {
      await _openNavigation(tester, '観測');
      expect(find.text('観測センター'), findsAtLeastNWidgets(1));
      expect(
        find.text('Rust Broker接続がないため、観測を取得できません。snapshotを実測値の代替にはしません。'),
        findsOneWidget,
      );
    }
    if (kGuiShellModuleTraceInspector) {
      await _openNavigation(tester, '追跡');
      expect(find.text('追跡情報'), findsAtLeastNWidgets(1));
      expect(
          find.text(
              'Trace表示からPermission、Approval、Authority、Capability、Credentialを生成しません。'),
          findsOneWidget);
      expect(
        find.text('Rust Broker接続がないため、Traceを取得できません。snapshotをTraceの根拠にはしません。'),
        findsOneWidget,
      );
    }
  });

  if (kGuiShellModuleObservability) {
    testWidgets('選択されたObservability ModuleはBroker consumerの値を表示する',
        (tester) async {
      final fixture = _ObservationFixtureTransport();
      await tester.pumpWidget(
        GuiShellDesktopApp(client: ShellCoreClient.mock(transport: fixture)),
      );

      expect(
        fixture.operations.where((operation) => operation == '観測一覧'),
        isEmpty,
        reason: '未訪問の観測画面はBrokerへ照会しない',
      );
      await _openNavigation(tester, '観測');
      await tester.pumpAndSettle();

      final observationPayloads = <Map<String, Object?>>[
        for (var index = 0; index < fixture.operations.length; index++)
          if (fixture.operations[index] == '観測一覧') fixture.payloads[index],
      ];
      expect(observationPayloads, hasLength(1));
      expect(observationPayloads.single['上限'], 200);
      expect(observationPayloads.every((payload) => payload['版'] == 1), isTrue);
      expect(find.text('Span: 1'), findsOneWidget);
      expect(find.text('Trace: 1'), findsOneWidget);
      expect(find.text('Metric: 1'), findsOneWidget);
      expect(find.text('Fixture Metric: 3 count (measured)'), findsOneWidget);
      expect(
          find.textContaining('Broker / fixture-observation'), findsOneWidget);
      expect(find.textContaining('Audit=責任・安全・証拠'), findsOneWidget);
      expect(find.textContaining('権限生成: なし'), findsOneWidget);
    });
  }

  if (kGuiShellModuleTraceInspector) {
    testWidgets('選択されたTrace Inspectorはbounded Broker結果を表示する', (tester) async {
      final fixture = _ObservationFixtureTransport();
      await tester.pumpWidget(
        GuiShellDesktopApp(client: ShellCoreClient.mock(transport: fixture)),
      );

      expect(
        fixture.operations.where((operation) => operation == '観測一覧'),
        isEmpty,
        reason: '未訪問の追跡画面はBrokerへ照会しない',
      );
      await _openNavigation(tester, '追跡');
      await tester.pumpAndSettle();

      final observationPayloads = <Map<String, Object?>>[
        for (var index = 0; index < fixture.operations.length; index++)
          if (fixture.operations[index] == '観測一覧') fixture.payloads[index],
      ];
      expect(observationPayloads, hasLength(1));
      expect(observationPayloads.single['上限'], 256);
      expect(observationPayloads.every((payload) => payload['版'] == 1), isTrue);
      expect(find.text('Trace: 1'), findsOneWidget);
      expect(find.text('Span: 1'), findsOneWidget);
      expect(find.text('Broker / fixture-observation / completed'),
          findsOneWidget);
      final traceDropdown = find.byType(DropdownButton<String?>);
      expect(traceDropdown, findsOneWidget);
      final dropdownWidget =
          tester.widget<DropdownButton<String?>>(traceDropdown);
      expect(
        dropdownWidget.items!.map((item) => item.value),
        contains('module-trace-fixture'),
      );
      await tester.tap(traceDropdown);
      await tester.pumpAndSettle();
      expect(find.text('module-trace-fixture'), findsWidgets);
      await tester.tap(find.text('module-trace-fixture').last);
      await tester.pumpAndSettle();
      final filteredPayloads = <Map<String, Object?>>[
        for (var index = 0; index < fixture.operations.length; index++)
          if (fixture.operations[index] == '観測一覧' &&
              fixture.payloads[index]['TraceID'] == 'module-trace-fixture')
            fixture.payloads[index],
      ];
      expect(filteredPayloads, hasLength(1));
      expect(filteredPayloads.single['上限'], 256);
      expect(
        find.textContaining('関連Audit: audit-observation-fixture'),
        findsOneWidget,
      );
      expect(
        find.text(
          'Trace表示からPermission、Approval、Authority、Capability、Credentialを生成しません。',
        ),
        findsOneWidget,
      );
      expect(find.textContaining('Broker接続がないため'), findsNothing);

      await _openNavigation(tester, '概要');
      await _openNavigation(tester, '追跡');
      await tester.pumpAndSettle();
      expect(
        fixture.operations.where((operation) => operation == '観測一覧'),
        hasLength(2),
        reason: '訪問済み画面は保持し、戻るたびに再照会しない',
      );
    });
  }

  if (kGuiShellModuleObservability) {
    testWidgets('観測一覧の失敗後にBroker要求を再試行できる', (tester) async {
      final fixture = _ObservationFixtureTransport(failFirstObservation: true);
      await tester.pumpWidget(
        GuiShellDesktopApp(client: ShellCoreClient.mock(transport: fixture)),
      );

      await _openNavigation(tester, '観測');
      await tester.pumpAndSettle();
      expect(find.textContaining('観測一覧を取得できません:'), findsOneWidget);
      expect(find.text('再試行'), findsOneWidget);
      expect(
        fixture.operations.where((operation) => operation == '観測一覧'),
        hasLength(1),
      );

      await tester.tap(find.text('再試行'));
      await tester.pumpAndSettle();
      expect(find.text('Metric: 1'), findsOneWidget);
      expect(
        fixture.operations.where((operation) => operation == '観測一覧'),
        hasLength(2),
      );
    });
  }

  if (kGuiShellModuleTraceInspector) {
    testWidgets('Trace一覧の失敗後にBroker要求を再試行できる', (tester) async {
      final fixture = _ObservationFixtureTransport(failFirstObservation: true);
      await tester.pumpWidget(
        GuiShellDesktopApp(client: ShellCoreClient.mock(transport: fixture)),
      );

      await _openNavigation(tester, '追跡');
      await tester.pumpAndSettle();
      expect(find.textContaining('Trace一覧を取得できません:'), findsOneWidget);
      expect(find.text('再試行'), findsOneWidget);
      expect(
        fixture.operations.where((operation) => operation == '観測一覧'),
        hasLength(1),
      );

      await tester.tap(find.text('再試行'));
      await tester.pumpAndSettle();
      expect(find.text('Trace: 1'), findsOneWidget);
      expect(
        fixture.operations.where((operation) => operation == '観測一覧'),
        hasLength(2),
      );
    });
  }

  if (allOptionalModulesEnabled) {
    test('全optional Moduleは互換性のため既定で有効', () {
      expect(allOptionalModulesEnabled, isTrue);
    });
    return;
  }

  if (!allOptionalModulesDisabled) {
    return;
  }

  testWidgets('除外Moduleは操作経路から消え必須画面は残る', (
    tester,
  ) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    rail.onDestinationSelected!(10);
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(find.text('この書出し構成には対象Moduleが含まれていません。'), findsNothing);
    expect(find.byType(NavigationRail), findsOneWidget);

    final updatedRail =
        tester.widget<NavigationRail>(find.byType(NavigationRail));
    updatedRail.onDestinationSelected!(12);
    await tester.pumpAndSettle();
    expect(find.text('A2A接続センター'), findsOneWidget);
    expect(find.textContaining('Broker接続がない'), findsOneWidget);

    await tester.tap(find.byTooltip('コマンドパレットを開く（Ctrl+KまたはCtrl+P）'));
    await tester.pumpAndSettle();
    final palette = find.byType(Dialog);
    expect(
      find.descendant(of: palette, matching: find.text('Host切替')),
      findsNothing,
    );
    expect(
      find.descendant(of: palette, matching: find.text('履歴を開く')),
      findsNothing,
    );
  });

  test('Trace InspectorはObservabilityなしでは有効化されない', () {
    expect(kGuiShellModuleObservability, isFalse);
    expect(kGuiShellModuleTraceInspector, isFalse);
  });
}

Future<void> _openNavigation(WidgetTester tester, String label) async {
  final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
  final destinationIndex = rail.destinations.indexWhere(
    (destination) => (destination.label as Text).data == label,
  );
  expect(destinationIndex, greaterThanOrEqualTo(0),
      reason: '$labelは選択Moduleのナビに必要である');
  rail.onDestinationSelected!(destinationIndex);
  await tester.pumpAndSettle();
  expect(tester.takeException(), isNull, reason: '$labelの画面表示');
}

class _ObservationFixtureTransport implements BrokerTransport {
  _ObservationFixtureTransport({this.failFirstObservation = false});

  bool failFirstObservation;
  final operations = <String>[];
  final payloads = <Map<String, Object?>>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    payloads.add(Map<String, Object?>.from(payload ?? const {}));
    if (operation == '観測一覧' && failFirstObservation) {
      failFirstObservation = false;
      throw StateError('fixture observation failure');
    }
    return {
      'status': 'accepted',
      'body': {
        '版': 1,
        'Span一覧': [
          {
            'TraceID': 'module-trace-fixture',
            'SpanID': 'module-span-fixture',
            '対象': 'Broker',
            '操作': 'fixture-observation',
            '状態': 'completed',
            '所要Millis': 12,
            '関連監査ID': 'audit-observation-fixture',
            '証拠種別': 'INTERNAL_STATE',
            '開始EpochMillis': 1700000000000,
            '終了EpochMillis': 1700000000012,
          },
        ],
        'Trace一覧': [
          {'TraceID': 'module-trace-fixture'},
        ],
        'Metric一覧': [
          {
            'MetricID': 'fixture-metric-1',
            '名称': 'Fixture Metric',
            '値': 3,
            '単位': 'count',
            '状態': 'measured',
          },
        ],
        '件数': 1,
        '証拠種別': 'INTERNAL_STATE',
        '観測範囲': 'broker_audit_finalization',
        'Auditとの責任分離': 'Audit=責任・安全・証拠; Observation=性能・挙動・運用状態',
        'OpenTelemetry export': 'unsupported',
        '権限生成': 'なし',
      },
    };
  }
}
