import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/profile_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

class _ProfileTransport implements BrokerTransport {
  final operations = <String>[];
  final payloads = <Map<String, Object?>?>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    payloads.add(payload);
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
}
