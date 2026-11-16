import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';

class _RuntimeCatalogFixture implements BrokerTransport {
  _RuntimeCatalogFixture(this.response);

  final Map<String, Object?> response;
  String? operation;
  Map<String, Object?>? payload;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    this.operation = operation;
    this.payload = Map<String, Object?>.from(payload ?? const {});
    return response;
  }
}

Map<String, Object?> _response(
  List<Object?> runtimeIds, {
  String evidenceSource = 'INTERNAL_STATE',
  Object? error,
}) =>
    {
      'request_id': 'runtime-list-request-1',
      'operation': '実行系列挙',
      'status': 'accepted',
      'evidence_source': evidenceSource,
      'audit_event_id': 'runtime-list-audit-1',
      'error': error,
      'health': null,
      'body': {'実行系': runtimeIds},
      'shutdown_requested': false,
    };

void main() {
  test('Broker登録済みRuntimeだけを厳密検証し、安定順に返す', () async {
    final fixture = _RuntimeCatalogFixture(
      _response(['runtime-b', 'runtime-a']),
    );

    final runtimeIds =
        await ShellCoreClient.mock(transport: fixture).refreshRuntimeCatalog();

    expect(fixture.operation, '実行系列挙');
    expect(fixture.payload, isEmpty);
    expect(runtimeIds, ['runtime-a', 'runtime-b']);
  });

  test('重複Runtime IDは一覧全体を拒否する', () async {
    final client = ShellCoreClient.mock(
      transport: _RuntimeCatalogFixture(_response(['runtime-a', 'runtime-a'])),
    );

    await expectLater(
      client.refreshRuntimeCatalog(),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('Broker以外の証拠種別や成功とerrorが混在する一覧を拒否する', () async {
    for (final response in [
      _response(['runtime-a'], evidenceSource: 'FIXTURE'),
      _response(['runtime-a'], error: {'code': 'unexpected'}),
    ]) {
      final client = ShellCoreClient.mock(
        transport: _RuntimeCatalogFixture(response),
      );

      await expectLater(
        client.refreshRuntimeCatalog(),
        throwsA(isA<BrokerClientException>()),
      );
    }
  });
}
