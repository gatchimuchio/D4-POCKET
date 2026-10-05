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
          'download_job': null,
          '適用実行': 'suspended',
          'rollback実行': 'suspended',
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
          'file数': 14,
          'total_bytes': 4096,
          '復旧ID': 'recover-update-install-activation',
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
          '起動': 'not_started',
          'Start Menu': 'unchanged',
          '証拠種別': 'INTERNAL_STATE',
        },
      };
    }
    return {
      'status': 'accepted',
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

  test('配布元表示はBrokerの導出状態だけを使う', () {
    expect(
      UpdateClient.packageSourceLabel({
        '状態': 'configured',
        'URL': 'https://updates.example.invalid/d4/stable/update-1.pkg',
      }),
      '配布元=updates.example.invalid',
    );
    expect(
      UpdateClient.packageSourceLabel({'状態': 'unconfigured', 'URL': null}),
      '配布元未設定',
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

  test('download job表示は進捗・終了・失敗を境界付きで射影する', () {
    expect(
      UpdateClient.downloadJobLabel({
        '状態': 'downloading',
        '受信byte数': 25,
        '全byte数': 100,
      }),
      'download中 25% (25 / 100 bytes)',
    );
    expect(
      UpdateClient.downloadJobLabel({'状態': 'downloaded'}),
      'download済み（installは別途保留）',
    );
    expect(
      UpdateClient.downloadJobLabel({
        '状態': 'failed',
        '失敗code': 'update_download_digest_mismatch',
      }),
      'download失敗: SHA-256が不一致',
    );
    expect(
      UpdateClient.downloadJobLabel({
        '状態': 'failed',
        '失敗code': 'secret path C:/private/update.pkg',
      }),
      'download失敗: 失敗code不明',
    );
    expect(UpdateClient.downloadJobLabel(null), 'download jobなし');
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
    final apply = await client.requestApply(
      updateId: 'update-1',
      candidateHash: 'sha256:${'a' * 64}',
    );
    expect(apply['導入状態'], 'version_staged');
    expect(apply['有効化'], 'suspended');
    final activation = await client.requestActivation(
      updateId: 'update-1',
      candidateHash: 'sha256:${'a' * 64}',
    );
    expect(activation['有効化'], 'active_version_recorded');
    expect(activation['起動'], 'not_started');
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
      '更新有効版切替要求',
      '更新延期',
      '更新rollback要求',
    ]);
    expect(transport.payloads[4], {
      '版': 1,
      '更新ID': 'update-1',
      '候補hash': 'sha256:${'a' * 64}',
    });
  });
}
