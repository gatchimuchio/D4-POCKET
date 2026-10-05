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

  if (allOptionalModulesEnabled) {
    test('全optional Moduleは互換性のため既定で有効', () {
      expect(allOptionalModulesEnabled, isTrue);
    });
    return;
  }

  testWidgets('除外Moduleはナビゲーションから消え必須画面は残る', (
    tester,
  ) async {
    expect(kGuiShellModuleSetupDoctor, isFalse);
    expect(kGuiShellModuleHistory, isFalse);
    expect(kGuiShellModuleEvaluationLab, isFalse);
    expect(kGuiShellModuleHostCapabilities, isFalse);
    expect(kGuiShellModuleNotifications, isFalse);
    expect(kGuiShellModuleObservability, isFalse);
    expect(kGuiShellModuleTraceInspector, isFalse);
    expect(kGuiShellModuleHostOperations, isFalse);

    await tester.pumpWidget(const GuiShellDesktopApp());
    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    final labels = rail.destinations
        .map((destination) => (destination.label as Text).data)
        .whereType<String>()
        .toList(growable: false);

    expect(rail.destinations, hasLength(13));
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
    for (final excluded in [
      '診断',
      '履歴',
      '評価ラボ',
      'ホスト能力',
      '通知',
      '観測',
      '追跡',
      'Host操作',
    ]) {
      expect(labels, isNot(contains(excluded)));
    }

    rail.onDestinationSelected!(10);
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(find.text('この書出し構成には対象Moduleが含まれていません。'), findsNothing);
    expect(find.byType(NavigationRail), findsOneWidget);

    rail.onDestinationSelected!(12);
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
  });

  test('Trace InspectorはObservabilityなしでは有効化されない', () {
    expect(kGuiShellModuleObservability, isFalse);
    expect(kGuiShellModuleTraceInspector, isFalse);
  });
}
