import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/models/generated_contracts.dart';
import 'package:gui_shell_desktop/screens/approval_center.dart';
import 'package:gui_shell_desktop/screens/dashboard.dart';
import 'package:gui_shell_desktop/screens/permission_center.dart';
import 'package:gui_shell_desktop/screens/shared.dart';
import 'package:gui_shell_desktop/services/global_search_index.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';

void main() {
  testWidgets('承認センターは未取得を承認なしと表示しない', (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ApprovalCenter(client: ShellCoreClient.local())),
      ),
    );

    expect(find.text('承認待ち状態は未取得'), findsOneWidget);
    expect(find.text('保留中の承認なし'), findsNothing);
    expect(find.textContaining('local-approval-redacted'), findsNothing);
  });

  testWidgets('許可センターは未取得のPermission fixtureを表示しない',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: PermissionCenter(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('Permission一覧は未取得'), findsOneWidget);
    expect(find.byType(DataTable), findsNothing);
    expect(find.textContaining('permission.fs.write.workspace'), findsNothing);
  });

  testWidgets('既知の承認だけ表示し、非表示内容を露出しない', (WidgetTester tester) async {
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
          body: ApprovalCenter(client: ShellCoreClient.mock(snapshot: known)),
        ),
      ),
    );

    expect(find.textContaining('可視性: redacted'), findsOneWidget);
    expect(find.textContaining('[redacted]'), findsOneWidget);
    expect(find.textContaining('hello'), findsNothing);
  });

  testWidgets('DashboardとStatus Barは未取得の承認件数を不明で示す',
      (WidgetTester tester) async {
    final snapshot = ShellCoreClient.local().getSnapshot();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Dashboard(client: ShellCoreClient.local(snapshot: snapshot)),
        ),
      ),
    );

    final approvalMetric = find.ancestor(
      of: find.text('保留中の承認'),
      matching: find.byType(SurfaceSemantics),
    );
    expect(
      find.descendant(of: approvalMetric, matching: find.text('不明')),
      findsOneWidget,
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ShellStatusBar(snapshot: snapshot)),
      ),
    );
    expect(find.text('承認: 一覧未取得'), findsOneWidget);
  });

  testWidgets('件数既知でも未取得の承認一覧を承認なしと表示しない', (WidgetTester tester) async {
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
          body: ApprovalCenter(
            client:
                ShellCoreClient.mock(snapshot: countKnownButInventoryUnknown),
          ),
        ),
      ),
    );

    expect(find.text('承認待ち状態は未取得'), findsOneWidget);
    expect(find.text('保留中の承認なし'), findsNothing);
    expect(find.textContaining('local-approval-redacted'), findsNothing);
  });

  test('横断検索は未取得のPermission／Approval fixtureを検索対象にしない', () {
    final snapshot = _snapshot(inventoryKnown: false);

    expect(
      GlobalSearchIndex.search(snapshot, 'fixture-permission-marker'),
      isEmpty,
    );
    expect(
      GlobalSearchIndex.search(snapshot, 'fixture-approval-marker'),
      isEmpty,
    );
  });

  test('横断検索はBrokerが取得済みと示したPermission／Approvalを検索する', () {
    final snapshot = _snapshot(inventoryKnown: true);

    final permission =
        GlobalSearchIndex.search(snapshot, 'fixture-permission-marker');
    final approval =
        GlobalSearchIndex.search(snapshot, 'fixture-approval-marker');

    expect(permission, hasLength(1));
    expect(permission.single.category, '権限');
    expect(approval, hasLength(1));
    expect(approval.single.category, '承認');
  });
}

ShellSnapshot _snapshot({
  required bool inventoryKnown,
  bool countKnown = true,
}) {
  return ShellSnapshot.fromJson({
    'snapshot_source': 'broker',
    'permission_inventory_known': inventoryKnown,
    'operation_status': {
      'pending_approvals_count': 1,
      'pending_approvals_count_known': countKnown,
    },
    'permissions': [
      {
        'permission_id': 'fixture-permission-marker',
        'capability_id': 'filesystem.read',
        'decision': 'allow',
        'source': 'broker',
      },
    ],
    'pending_approvals': [
      {
        'approval_id': 'fixture-approval-marker',
        'operation': 'workspace.read',
        'status': 'pending',
        'content_visibility': 'redacted',
      },
    ],
    'pending_approvals_inventory_known': inventoryKnown,
  }).copyWith(snapshotSource: 'broker');
}
