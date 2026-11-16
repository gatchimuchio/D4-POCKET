import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/models/generated_contracts.dart';
import 'package:gui_shell_desktop/screens/settings.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';

void main() {
  testWidgets('Broker以外のSnapshotではPermissionとApprovalをunknown表示する',
      (tester) async {
    final client = ShellCoreClient.mock(
      snapshot: ShellSnapshot.fromJson({
        'permissions': [
          {
            'permission_id': 'do-not-show-permission',
            'capability_id': 'workspace.write',
            'decision': 'allow',
            'source': 'owner',
          },
        ],
        'pending_approvals': [
          {
            'approval_id': 'do-not-show-approval',
            'operation': 'Task',
            'status': 'pending',
          },
        ],
        'permission_inventory_known': true,
        'pending_approvals_inventory_known': true,
      }),
    );

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: SettingsScreen(client: client, active: false)),
    ));

    expect(find.textContaining('Permission／Approvalはunknown'), findsOneWidget);
    expect(find.text('Permission一覧: unknown'), findsOneWidget);
    expect(find.text('Pending Approval一覧: unknown'), findsOneWidget);
    expect(find.textContaining('do-not-show-permission'), findsNothing);
    expect(find.textContaining('do-not-show-approval'), findsNothing);

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: SettingsScreen(
          client: _client(source: 'broker', freshness: 'unknown'),
          active: false,
        ),
      ),
    ));
    expect(find.text('Permission一覧: unknown'), findsOneWidget);
    expect(find.text('Pending Approval一覧: unknown'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Broker Snapshotのpermissionとpending approvalを本文なしで表示する',
      (tester) async {
    final client = ShellCoreClient.mock(
      snapshot: ShellSnapshot.fromJson({
        'permissions': [
          {
            'permission_id': 'permission-workspace-read',
            'capability_id': 'workspace.read',
            'decision': 'allow',
            'source': 'owner',
          },
        ],
        'pending_approvals': [
          {
            'approval_id': 'approval-task-1',
            'operation': 'AgentTask',
            'status': 'pending',
            'content_visibility': 'redacted',
            'projected_content': {'task': 'SENSITIVE_APPROVAL_BODY'},
          },
        ],
        'permission_inventory_known': true,
        'pending_approvals_inventory_known': true,
        'snapshot_freshness': '2026-11-17T00:00:00Z',
      }).copyWith(snapshotSource: 'broker'),
    );

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: SettingsScreen(client: client, active: false)),
    ));

    expect(find.text('Permission一覧: 1件'), findsOneWidget);
    expect(
      find.textContaining(
          'permission-workspace-read / workspace.read / decision=allow'),
      findsOneWidget,
    );
    expect(find.text('Pending Approval一覧: 1件'), findsOneWidget);
    expect(
      find.textContaining(
          'approval-task-1 / operation=AgentTask / status=pending'),
      findsOneWidget,
    );
    expect(find.textContaining('本文は非表示'), findsOneWidget);
    expect(find.textContaining('SENSITIVE_APPROVAL_BODY'), findsNothing);
    expect(find.textContaining('発行・消費・変更しません'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

ShellCoreClient _client({required String source, required String freshness}) =>
    ShellCoreClient.mock(
      snapshot: ShellSnapshot.fromJson({
        'permission_inventory_known': true,
        'pending_approvals_inventory_known': true,
        'snapshot_freshness': freshness,
      }).copyWith(snapshotSource: source),
    );
