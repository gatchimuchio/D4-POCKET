import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/settings.dart';
import 'package:gui_shell_desktop/services/profile_client.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

class _ProfileTransport implements BrokerTransport {
  _ProfileTransport({
    this.rejectedOperations = const {},
    this.rejectionError,
    this.acceptedError,
    this.throwOnOperation,
    this.transportError,
  });

  final Set<String> rejectedOperations;
  final Object? rejectionError;
  final Object? acceptedError;
  final String? throwOnOperation;
  final Object? transportError;
  final operations = <String>[];
  final payloads = <Map<String, Object?>?>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    payloads.add(payload);
    if (operation == throwOnOperation) {
      throw transportError ?? StateError('secret-marker transport detail');
    }
    if (rejectedOperations.contains(operation)) {
      return {
        'status': 'rejected',
        'error': rejectionError ??
            {'code': 'owner_denied', 'message': 'secret-marker rejection'},
      };
    }
    if (operation == 'プロファイル一覧') {
      return {
        'status': 'accepted',
        'body': {
          '版': 1,
          'Profiles': [
            {
              '版': 1,
              'ProfileID': 'default.local',
              '表示名': '標準',
              'Runtime': 'runtime.local',
              'Adapter': 'adapter.local',
            },
          ],
        },
      };
    }
    return {
      'status': 'accepted',
      if (acceptedError != null) 'error': acceptedError,
      'body': {'状態': 'accepted'}
    };
  }
}

void main() {
  test('Profile操作はBroker operationへ限定されたpayloadを送る', () async {
    final transport = _ProfileTransport();
    final client = ProfileClient(transport);
    final profiles = await client.list();
    expect(profiles.single['ProfileID'], 'default.local');
    expect(profiles.single['profile_hash'], startsWith('sha256:'));

    await client.copy(
      sourceProfileId: 'default.local',
      profileId: 'copy.local',
      displayName: '複製',
    );
    await client.apply(
      profileId: 'default.local',
      profileHash: 'sha256:${'a' * 64}',
    );
    await client.delete(profileId: 'copy.local');
    await client.export('default.local');
    await client.importProfile({'版': 1, 'ProfileID': 'import.local'});

    expect(
      transport.operations,
      [
        'プロファイル一覧',
        'プロファイル複製',
        'プロファイル適用要求',
        'プロファイル削除',
        'プロファイルexport',
        'プロファイルimport',
      ],
    );
    expect(transport.payloads[2], {
      '版': 1,
      'ProfileID': 'default.local',
      'profile_hash': 'sha256:${'a' * 64}',
    });
  });

  test('Broker拒否・不正error・transport例外をProfile例外へ露出しない', () async {
    const marker = 'secret-marker path profile payload';
    for (final error in <Object?>[
      {'code': 'owner_denied', 'message': marker},
      marker,
    ]) {
      final client = ProfileClient(
        _ProfileTransport(
          rejectedOperations: const {'プロファイル作成'},
          rejectionError: error,
        ),
      );
      await expectLater(
        client.create(const {'版': 1}),
        throwsA(
          isA<BrokerClientException>().having(
            (exception) => exception.message,
            'message',
            isNot(contains(marker)),
          ),
        ),
      );
    }

    final acceptedErrorClient = ProfileClient(
      _ProfileTransport(acceptedError: {'message': marker}),
    );
    await expectLater(
      acceptedErrorClient.create(const {'版': 1}),
      throwsA(
        isA<BrokerClientException>().having(
          (exception) => exception.message,
          'message',
          isNot(contains(marker)),
        ),
      ),
    );

    final transportFailureClient = ProfileClient(
      _ProfileTransport(
        throwOnOperation: 'プロファイル作成',
        transportError: StateError(marker),
      ),
    );
    await expectLater(
      transportFailureClient.create(const {'版': 1}),
      throwsA(
        isA<BrokerClientException>().having(
          (exception) => exception.message,
          'message',
          isNot(contains(marker)),
        ),
      ),
    );

    final listTransportFailureClient = ProfileClient(
      _ProfileTransport(
        throwOnOperation: 'プロファイル一覧',
        transportError: StateError(marker),
      ),
    );
    await expectLater(
      listTransportFailureClient.list(),
      throwsA(
        isA<BrokerClientException>().having(
          (exception) => exception.message,
          'message',
          isNot(contains(marker)),
        ),
      ),
    );
  });

  testWidgets('Profile拒否の自由記述を設定画面へ表示しない', (tester) async {
    const marker = 'secret-marker path profile payload';
    final transport = _ProfileTransport(
      rejectedOperations: const {'プロファイル作成'},
      rejectionError: {'code': 'owner_denied', 'message': marker},
    );
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: SettingsScreen(
          client: ShellCoreClient.mock(transport: transport),
        ),
      ),
    ));
    await tester.pumpAndSettle();

    final createButton = find.widgetWithText(FilledButton, '作成');
    await tester.ensureVisible(createButton);
    await tester.tap(createButton);
    await tester.pumpAndSettle();

    expect(transport.operations, contains('プロファイル作成'));
    expect(find.textContaining(marker), findsNothing);
    expect(find.textContaining('Profile作成に失敗しました。'), findsOneWidget);
  });

  testWidgets('Update CenterはBroker拒否の自由記述を表示しない', (tester) async {
    const marker = 'secret-marker path update package';
    final transport = _ProfileTransport(
      rejectedOperations: const {'更新一覧'},
      rejectionError: {'code': 'denied', 'message': marker},
    );
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: SettingsScreen(
          client: ShellCoreClient.mock(transport: transport),
        ),
      ),
    ));
    await tester.pumpAndSettle();

    expect(find.textContaining(marker), findsNothing);
    expect(
      find.text('更新一覧を取得できません。Broker接続と更新状態を確認してください。'),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('malformed Profile import本文を例外表示へ含めない', (tester) async {
    const marker = 'secret-marker malformed profile';
    final transport = _ProfileTransport();
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: SettingsScreen(
          client: ShellCoreClient.mock(transport: transport),
        ),
      ),
    ));
    await tester.pumpAndSettle();

    final importButton = find.widgetWithText(OutlinedButton, 'import');
    await tester.ensureVisible(importButton);
    await tester.tap(importButton);
    await tester.pumpAndSettle();
    final dialog = find.byType(AlertDialog);
    final field = find.descendant(of: dialog, matching: find.byType(TextField));
    await tester.enterText(field, '$marker{');
    await tester.tap(find.descendant(
      of: dialog,
      matching: find.widgetWithText(FilledButton, 'import'),
    ));
    await tester.pumpAndSettle();

    expect(find.textContaining(marker), findsNothing);
    expect(find.textContaining('Profile importに失敗しました。'), findsOneWidget);
  });
}
