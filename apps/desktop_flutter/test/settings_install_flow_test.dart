import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/settings.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

void main() {
  testWidgets('portable初回InstallはBroker登録後に終了を要求し、拒否時は復旧案内する', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final transport = _InstallFlowTransport();
    final exitRequests = <ui.AppExitType>[];

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: SettingsScreen(
          client: ShellCoreClient.mock(transport: transport),
          requestApplicationExit: (exitType) async {
            exitRequests.add(exitType);
            return ui.AppExitResponse.cancel;
          },
        ),
      ),
    ));
    await tester.pumpAndSettle();

    final stageButton = find.text('未起動版へ展開');
    await tester.ensureVisible(stageButton);
    await tester.tap(stageButton);
    await tester.pumpAndSettle();
    expect(transport.operations, contains('更新適用要求'));

    final installButton = find.widgetWithText(TextButton, 'インストール');
    await tester.ensureVisible(installButton);
    await tester.tap(installButton);
    await tester.pump();
    expect(tester.widget<TextButton>(installButton).onPressed, isNull);
    await tester.tap(installButton, warnIfMissed: false);
    await tester.pump();
    expect(
      transport.operations.where((operation) => operation == '更新有効版切替要求'),
      hasLength(1),
    );
    await tester.pump(const Duration(milliseconds: 601));
    await tester.pumpAndSettle();

    expect(transport.operations, contains('更新有効版切替要求'));
    expect(
      exitRequests,
      [ui.AppExitType.required],
    );
    expect(
      find.text(
        'インストール済み版は登録されましたが、画面を終了できませんでした。Start Menuから起動してください。',
      ),
      findsOneWidget,
    );
    expect(tester.widget<TextButton>(installButton).onPressed, isNotNull);
    expect(find.textContaining('更新操作失敗'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Uninstall中は状態変更を直列化し、終了拒否時は未削除と案内する', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final transport = _InstallFlowTransport(installed: true);
    final exitRequests = <ui.AppExitType>[];

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: SettingsScreen(
          client: ShellCoreClient.mock(transport: transport),
          requestApplicationExit: (exitType) async {
            exitRequests.add(exitType);
            return ui.AppExitResponse.cancel;
          },
        ),
      ),
    ));
    await tester.pumpAndSettle();

    final uninstallButton =
        find.widgetWithText(OutlinedButton, 'D4 Pocketをアンインストール');
    await tester.ensureVisible(uninstallButton);
    await tester.tap(uninstallButton);
    await tester.pump();
    expect(tester.widget<OutlinedButton>(uninstallButton).onPressed, isNull);
    await tester.tap(uninstallButton, warnIfMissed: false);
    await tester.pump();
    expect(
      transport.operations.where((operation) => operation == '製品アンインストール要求'),
      hasLength(1),
    );

    await tester.pump(const Duration(milliseconds: 501));
    await tester.pumpAndSettle();

    expect(exitRequests, [ui.AppExitType.required]);
    expect(
      find.text(
        'アプリの終了がキャンセルされました。製品fileはまだ削除されていません。再度アンインストールを要求してください。',
      ),
      findsOneWidget,
    );
    expect(tester.widget<OutlinedButton>(uninstallButton).onPressed, isNotNull);
    expect(tester.takeException(), isNull);
  });

  testWidgets('起動項目修復はBrokerへ一度だけ要求し、修復結果を表示する', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final repairGate = Completer<void>();
    final transport = _InstallFlowTransport(
      installed: true,
      repairGate: repairGate,
    );
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body:
            SettingsScreen(client: ShellCoreClient.mock(transport: transport)),
      ),
    ));
    await tester.pumpAndSettle();

    final repairButton = find.widgetWithText(OutlinedButton, '起動項目を修復');
    await tester.ensureVisible(repairButton);
    await tester.tap(repairButton);
    await tester.pump();
    expect(tester.widget<OutlinedButton>(repairButton).onPressed, isNull);
    await tester.tap(repairButton, warnIfMissed: false);
    expect(
      transport.operations.where((operation) => operation == '製品起動項目修復要求'),
      hasLength(1),
    );
    repairGate.complete();
    await tester.pumpAndSettle();
    expect(find.text('固定root起動器を復元しました。版本体と利用者dataは変更していません。'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

class _InstallFlowTransport implements BrokerTransport {
  _InstallFlowTransport({this.installed = false, this.repairGate});

  final bool installed;
  final Completer<void>? repairGate;

  static const _packageHash =
      'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
  static const _candidateHash = 'sha256:current-candidate';
  static const _updateId = 'd4-pocket-current';

  final operations = <String>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    if (operation == '更新一覧') {
      return {
        'status': 'accepted',
        'body': {
          '版': 1,
          '更新一覧': [
            {
              '更新ID': _updateId,
              '候補hash': _candidateHash,
              '提供版': '1.2.3',
              'channel': 'stable',
              '署名状態': 'verified',
              'rollback可能': false,
              'package_sha256': _packageHash,
              'package_size_bytes': 4096,
              '取得元': {
                '状態': 'configured',
                'URL': 'https://updates.example.invalid',
              },
            },
          ],
          '件数': 1,
          '署名信頼設定': 'configured',
          'download実行': 'available',
          'download_job': {
            '状態': 'downloaded',
            '更新ID': _updateId,
            '候補hash': _candidateHash,
            'package_sha256': _packageHash,
          },
          '適用実行': 'available',
          'rollback実行': 'suspended',
          'rollback状態': {
            '状態': 'unavailable',
            '現在版': installed
                ? {
                    '更新ID': _updateId,
                    '候補hash': _candidateHash,
                    '提供版': '1.2.3',
                  }
                : null,
            '対象版': null,
          },
          '証拠種別': 'INTERNAL_STATE',
        },
      };
    }
    if (operation == '更新適用要求') {
      return {
        'status': 'accepted',
        'body': {
          '版': 1,
          '導入状態': 'version_staged',
          '有効化': 'suspended',
          '証拠種別': 'INTERNAL_STATE',
        },
      };
    }
    if (operation == '更新有効版切替要求') {
      return {
        'status': 'accepted',
        'body': {
          '版': 1,
          '有効化': 'active_version_recorded',
          '起動': 'after_current_exit',
          'Start Menu': 'registered',
          '証拠種別': 'INTERNAL_STATE',
        },
      };
    }
    if (operation == '製品アンインストール要求') {
      return {
        'status': 'accepted',
        'body': {'版': 1, '状態': 'uninstall_authorized'},
      };
    }
    if (operation == '製品起動項目修復要求') {
      await repairGate?.future;
      return {
        'status': 'accepted',
        'body': {
          '版': 1,
          '状態': 'product_launch_entries_repaired',
          '起動器復元': true,
          'Start Menu復元': false,
          '証拠種別': 'LIVE_RUNTIME',
        },
      };
    }
    return {
      'status': 'accepted',
      'body': <String, Object?>{'版': 1}
    };
  }
}
