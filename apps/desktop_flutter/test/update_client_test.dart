import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/update_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

class _UpdateTransport implements BrokerTransport {
  final operations = <String>[];
  final payloads = <Map<String, Object?>?>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    payloads.add(payload);
    if (operation == '更新一覧') {
      return {
        'status': 'accepted',
        'body': {
          '版': 1,
          '更新一覧': <Object?>[],
          '件数': 0,
          '署名信頼設定': 'unconfigured',
          'download実行': 'suspended',
          '適用実行': 'suspended',
          'rollback実行': 'suspended',
          '証拠種別': 'INTERNAL_STATE',
        },
      };
    }
    return {
      'status': operation == '更新適用要求' ? 'suspended' : 'accepted',
      'body': {'版': 1, '実行状態': 'suspended'},
    };
  }
}

void main() {
  test('署名状態は現在trustの再検証意味を日本語表示する', () {
    expect(
      UpdateClient.signatureStatusLabel('verified'),
      '現在のBroker trustで検証済み',
    );
    expect(
      UpdateClient.signatureStatusLabel('verification_stale'),
      '現在のBroker trustで未検証',
    );
    expect(
      UpdateClient.signatureStatusLabel('legacy_unbound'),
      '旧候補（package未結合）',
    );
    expect(UpdateClient.signatureStatusLabel('unknown'), '不明');
  });

  test('配布元表示はBrokerの導出状態だけを使い実行可能と表示しない', () {
    expect(
      UpdateClient.packageSourceLabel({
        '状態': 'configured',
        'URL': 'https://updates.example.invalid/d4/stable/update-1.pkg',
      }),
      '配布元=updates.example.invalid（実downloadは保留）',
    );
    expect(
      UpdateClient.packageSourceLabel({'状態': 'unconfigured', 'URL': null}),
      '配布元未設定（実downloadは保留）',
    );
    expect(
      UpdateClient.packageSourceLabel({
        '状態': 'configured',
        'URL': 'https://user@updates.example.invalid/update-1.pkg',
      }),
      '配布元状態不明',
    );
    expect(UpdateClient.packageSourceLabel(null), '配布元状態不明');
  });

  test('Update操作はBrokerの監査済みoperationへ限定される', () async {
    final transport = _UpdateTransport();
    final client = UpdateClient(transport);
    final body = await client.list();
    expect(body['署名信頼設定'], 'unconfigured');
    await client.verify({'版': 1});
    await client.requestDownload(
      updateId: 'update-1',
      candidateHash: 'sha256:${'a' * 64}',
    );
    await client.requestApply(
      updateId: 'update-1',
      candidateHash: 'sha256:${'a' * 64}',
    );
    await client.defer(
      updateId: 'update-1',
      candidateHash: 'sha256:${'a' * 64}',
      deferredUntil: '2026-09-24T00:00:00Z',
    );
    await client.requestRollback(
      updateId: 'update-1',
      candidateHash: 'sha256:${'a' * 64}',
    );
    expect(transport.operations, [
      '更新一覧',
      '更新署名検査',
      '更新download要求',
      '更新適用要求',
      '更新延期',
      '更新rollback要求',
    ]);
  });
}
