import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/main.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('Broker切替中に旧検索応答が返ってもpaletteを開かない', (tester) async {
    final oldTransport = _BrokerSearchTransport(
      displayName: 'Old Linux',
      deferHost: true,
    );
    await tester.pumpWidget(
      GuiShellDesktopApp(
        client: ShellCoreClient.mock(transport: oldTransport),
      ),
    );

    await _openPalette(tester);
    await tester.pump();
    await oldTransport.hostRequestStarted.future;

    await tester.pumpWidget(
      GuiShellDesktopApp(
        client: ShellCoreClient.mock(
          transport: _BrokerSearchTransport(displayName: 'New Linux'),
        ),
      ),
    );
    await tester.pumpAndSettle();
    oldTransport.releaseHost();
    await tester.pumpAndSettle();

    expect(find.text('コマンドパレット'), findsNothing);
    expect(find.text('Old Linux'), findsNothing);
    expect(find.textContaining('Broker接続先が切り替わったため'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Broker切替後に旧paletteの選択結果を画面遷移へ使わない', (tester) async {
    final oldTransport = _BrokerSearchTransport(displayName: 'Old Linux');
    await tester.pumpWidget(
      GuiShellDesktopApp(
        client: ShellCoreClient.mock(transport: oldTransport),
      ),
    );
    await _openPalette(tester);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byWidgetPredicate(
        (widget) =>
            widget is TextField && widget.decoration?.labelText == 'コマンドパレット',
      ),
      'Old Linux',
    );
    await tester.pumpAndSettle();
    final oldHostResult = find.byWidgetPredicate(
      (widget) => widget is Text && widget.data == 'Old Linux',
    );
    expect(oldHostResult, findsOneWidget);

    await tester.pumpWidget(
      GuiShellDesktopApp(
        client: ShellCoreClient.mock(
          transport: _BrokerSearchTransport(displayName: 'New Linux'),
        ),
      ),
    );
    await tester.pump();
    await tester.tap(oldHostResult);
    await tester.pumpAndSettle();

    expect(find.text('D4 Pocket Host操作面'), findsNothing);
    expect(find.textContaining('Broker接続先が切り替わったため'), findsOneWidget);
    expect(oldTransport.operations, ['Host一覧', '作業領域一覧']);
    expect(tester.takeException(), isNull);
  });
}

Future<void> _openPalette(WidgetTester tester) async {
  await tester.sendKeyDownEvent(LogicalKeyboardKey.control);
  await tester.sendKeyEvent(LogicalKeyboardKey.keyK);
  await tester.sendKeyUpEvent(LogicalKeyboardKey.control);
}

class _BrokerSearchTransport implements BrokerTransport {
  _BrokerSearchTransport({required this.displayName, bool deferHost = false})
      : _pendingHost = deferHost ? Completer<Map<String, Object?>>() : null;

  final String displayName;
  final Completer<Map<String, Object?>>? _pendingHost;
  final Completer<void> hostRequestStarted = Completer<void>();
  final List<String> operations = [];

  void releaseHost() => _pendingHost!.complete(_accepted(
        'Host一覧',
        {
          '版': 1,
          'Host一覧': [_host()],
          '件数': 1,
          '公開範囲': 'metadata_only',
          '証拠種別': 'INTERNAL_STATE',
        },
      ));

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    if (operation == 'Host一覧') {
      final pending = _pendingHost;
      if (pending != null) {
        if (!hostRequestStarted.isCompleted) hostRequestStarted.complete();
        return pending.future;
      }
      return _accepted('Host一覧', {
        '版': 1,
        'Host一覧': [_host()],
        '件数': 1,
        '公開範囲': 'metadata_only',
        '証拠種別': 'INTERNAL_STATE',
      });
    }
    if (operation == '作業領域一覧') {
      return _accepted(operation, {'作業領域': <Object?>[]});
    }
    throw StateError('検索fixtureでは未対応の操作です');
  }

  Map<String, Object?> _accepted(String operation, Object body) => {
        'operation': operation,
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'audit-search-${operations.length}',
        'error': null,
        'body': body,
      };

  Map<String, Object?> _host() => {
        '版': 1,
        'Host ID': 'host-${displayName.toLowerCase().replaceAll(' ', '-')}',
        '表示名': displayName,
        'Platform': 'linux',
        '接続状態': 'pending_review',
        'Trust': {
          'state': 'pending_review',
          'evidence_source': 'INTERNAL_STATE',
          'requires_operator_review': true,
        },
        '証明書/identity': {
          '種別': 'identity_hash',
          'hash': 'sha256:${'a' * 64}',
        },
        'Runtime summary': {
          'runtime_count': 0,
          'agent_count': 0,
          'evidence_source': 'INTERNAL_STATE',
        },
        '最終接続': null,
        '公開範囲': 'metadata_only',
        '証拠種別': 'INTERNAL_STATE',
        '権限生成': 'なし',
        'authority_strip': true,
        '能力ID': 'host.registry.register',
        '権限ID': 'permission.host.registry.register',
        '承認状態': 'owner_control_approved',
        '復旧ID': 'recover-host-registration',
        '登録監査ID': 'audit-host-register-search',
      };
}
