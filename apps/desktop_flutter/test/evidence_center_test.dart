import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/evidence_center.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';

void main() {
  testWidgets('証拠書出しは未取得のApproval件数をnullで記録する', (WidgetTester tester) async {
    final clipboardText = _captureClipboard(tester);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: EvidenceCenter(client: ShellCoreClient.local())),
      ),
    );

    final exportButton = find.text('スナップショットJSONをコピー');
    await tester.ensureVisible(exportButton);
    await tester.tap(exportButton);
    await tester.pump();

    final exported = jsonDecode(clipboardText()!) as Map<String, dynamic>;
    final operationStatus =
        exported['operation_status'] as Map<String, dynamic>;
    final counts = exported['counts'] as Map<String, dynamic>;
    expect(operationStatus['pending_approvals_count'], isNull);
    expect(operationStatus['pending_approvals_count_known'], isFalse);
    expect(counts['pending_approvals'], isNull);
    expect(counts['pending_approvals_inventory_known'], isFalse);
  });

  testWidgets('証拠書出しは既知のBroker Approval件数を数値で記録する',
      (WidgetTester tester) async {
    final clipboardText = _captureClipboard(tester);
    final source = ShellCoreClient.local().getSnapshot();
    final known = source.copyWith(
      snapshotSource: 'broker',
      operationStatus: source.operationStatus.copyWith(
        pendingApprovalsCountKnown: true,
      ),
      pendingApprovalsInventoryKnown: true,
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: EvidenceCenter(client: ShellCoreClient.mock(snapshot: known)),
        ),
      ),
    );

    final exportButton = find.text('スナップショットJSONをコピー');
    await tester.ensureVisible(exportButton);
    await tester.tap(exportButton);
    await tester.pump();

    final exported = jsonDecode(clipboardText()!) as Map<String, dynamic>;
    final operationStatus =
        exported['operation_status'] as Map<String, dynamic>;
    final counts = exported['counts'] as Map<String, dynamic>;
    expect(operationStatus['pending_approvals_count'], 1);
    expect(operationStatus['pending_approvals_count_known'], isTrue);
    expect(counts['pending_approvals'], 1);
    expect(counts['pending_approvals_inventory_known'], isTrue);
  });

  testWidgets('証拠書出しは既知の件数と未取得の承認一覧を分けて記録する', (WidgetTester tester) async {
    final clipboardText = _captureClipboard(tester);
    final source = ShellCoreClient.local().getSnapshot();
    final countKnownButInventoryUnknown = source.copyWith(
      snapshotSource: 'broker',
      operationStatus: source.operationStatus.copyWith(
        pendingApprovalsCountKnown: true,
      ),
      pendingApprovalsInventoryKnown: false,
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: EvidenceCenter(
            client:
                ShellCoreClient.mock(snapshot: countKnownButInventoryUnknown),
          ),
        ),
      ),
    );

    final exportButton = find.text('スナップショットJSONをコピー');
    await tester.ensureVisible(exportButton);
    await tester.tap(exportButton);
    await tester.pump();

    final exported = jsonDecode(clipboardText()!) as Map<String, dynamic>;
    final operationStatus =
        exported['operation_status'] as Map<String, dynamic>;
    final counts = exported['counts'] as Map<String, dynamic>;
    expect(operationStatus['pending_approvals_count'], 1);
    expect(operationStatus['pending_approvals_count_known'], isTrue);
    expect(counts['pending_approvals'], isNull);
    expect(counts['pending_approvals_inventory_known'], isFalse);
  });

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

  testWidgets('有効snapshotの事前確認は適用や権限変更を行わない', (
    WidgetTester tester,
  ) async {
    final clipboardText = _captureClipboard(tester);
    final client = ShellCoreClient.local();
    final originalReleaseState =
        client.getSnapshot().operationStatus.releaseState;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: EvidenceCenter(client: client)),
      ),
    );

    final exportButton = find.text('スナップショットJSONをコピー');
    await tester.ensureVisible(exportButton);
    await tester.tap(exportButton);
    await tester.pump();
    final imported = jsonDecode(clipboardText()!) as Map<String, Object?>;
    final operationStatus = Map<String, Object?>.from(
      imported['operation_status']! as Map,
    )..['release_state'] = 'imported-release-marker';
    imported['operation_status'] = operationStatus;

    final compareButton = find.text('読込み前確認／比較');
    await tester.ensureVisible(compareButton);
    await tester.tap(compareButton);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.descendant(
        of: find.byType(Dialog),
        matching: find.byType(TextField),
      ),
      jsonEncode(imported),
    );
    await tester.pump();

    final dialogText = tester
        .widgetList<Text>(
          find.descendant(
            of: find.byType(Dialog),
            matching: find.byType(Text),
          ),
        )
        .map((text) => text.data ?? '')
        .join('\n');
    expect(dialogText, contains('事前確認専用です。読み込んだデータは適用しません。'));
    expect(dialogText, contains('読込み側のリリース状態: imported-release-marker'));
    expect(
      find.descendant(
        of: find.byType(Dialog),
        matching: find.textContaining('Approval'),
      ),
      findsNothing,
    );
    expect(
      find.descendant(
        of: find.byType(Dialog),
        matching: find.text('適用'),
      ),
      findsNothing,
    );

    await tester.tap(find.descendant(
      of: find.byType(Dialog),
      matching: find.text('閉じる'),
    ));
    await tester.pumpAndSettle();
    expect(client.getSnapshot().operationStatus.releaseState,
        originalReleaseState);
    expect(find.textContaining('imported-release-marker'), findsNothing);
  });
}

String? Function() _captureClipboard(WidgetTester tester) {
  String? capturedText;
  final messenger = tester.binding.defaultBinaryMessenger;
  messenger.setMockMethodCallHandler(
    SystemChannels.platform,
    (call) async {
      if (call.method == 'Clipboard.setData') {
        final data = Map<String, Object?>.from(call.arguments as Map);
        capturedText = data['text'] as String?;
      }
      return null;
    },
  );
  addTearDown(() {
    messenger.setMockMethodCallHandler(SystemChannels.platform, null);
  });
  return () => capturedText;
}
