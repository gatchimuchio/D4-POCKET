import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/evidence_center.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';

void main() {
  testWidgets('証拠センターが不正JSONの詳細と入力内容を表示しない', (
    WidgetTester tester,
  ) async {
    const secretMarker = 'SNAPSHOT_IMPORT_SECRET_MARKER';
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: EvidenceCenter(client: ShellCoreClient.mock())),
      ),
    );

    final compareButton = find.text('読込み前確認／比較');
    await tester.ensureVisible(compareButton);
    await tester.tap(compareButton);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byType(TextField),
      '{"snapshot_source":"$secretMarker",}',
    );
    await tester.pump();

    final visibleDialogText = tester
        .widgetList<Text>(
          find.descendant(
            of: find.byType(Dialog),
            matching: find.byType(Text),
          ),
        )
        .map((text) => text.data ?? '')
        .join('\n');
    expect(
      visibleDialogText,
      contains('スナップショットの形式を確認できません。JSONの構造と項目の型を確認してください。'),
    );
    expect(visibleDialogText, isNot(contains(secretMarker)));
  });
}
