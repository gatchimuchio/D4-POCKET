import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/main.dart';

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
