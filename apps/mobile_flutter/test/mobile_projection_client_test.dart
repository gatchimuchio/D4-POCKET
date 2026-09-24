import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_mobile/services/mobile_projection_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart';

class _Transport implements BrokerTransport {
  _Transport(this.responses);

  final Map<String, Map<String, Object?>> responses;
  final calls = <String>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    calls.add(operation);
    return responses[operation]!;
  }
}

Map<String, Object?> _response(String operation, Map<String, Object?> body) => {
  'operation': operation,
  'status': 'accepted',
  'audit_event_id': 'audit-1',
  'body': body,
};

Map<String, Object?> _agentMetadata({
  Map<String, Object?>? authentication,
  Map<String, Object?>? extra,
}) => {
  'adapter_id': 'codex-cli',
  'agent_id': 'codex',
  'provider': 'OpenAI',
  'version': '1.0.0',
  'model': 'unknown',
  'status': 'degraded',
  'capabilities': [
    {
      'capability_id': 'task_execution',
      'support': {'status': 'unknown', 'reason': '実task未確認'},
    },
  ],
  'workspace_requirements': {
    'mode': 'required',
    'boundary_policy': 'deny_outside_workspace',
    'secret_paths': ['.env', '.ssh'],
  },
  'tool_support': {'status': 'unknown', 'reason': '実動作未確認'},
  'mcp_support': {'status': 'unknown', 'reason': '接続未確認'},
  'session_support': {'status': 'unknown', 'reason': '実動作未確認'},
  'cancellation_support': {'status': 'unknown', 'reason': '実動作未確認'},
  'usage_metrics_support': {'status': 'unknown', 'reason': '実測未確認'},
  'cost_metrics_support': {'status': 'unknown', 'reason': '実測未確認'},
  'authentication':
      authentication ?? {'method': 'unknown', 'secret_value_present': false},
  'host_requirements': {
    'platforms': ['windows'],
    'network_scope': 'unknown',
    'process_spawn': {'status': 'unsupported', 'reason': 'Brokerで停止中'},
  },
  'evidence_source': 'LIVE_RUNTIME',
  'evidence_reason': '起動時interface確認のみ',
  ...?extra,
};

void main() {
  test('Agent一覧はBroker metadataを限定投影し実行・権限を推定しない', () async {
    final transport = _Transport({
      'Agent一覧': {
        ..._response('Agent一覧', {
          'Agent': [_agentMetadata()],
        }),
        'evidence_source': 'INTERNAL_STATE',
      },
    });

    final result = await MobileProjectionClient(transport).agents();

    expect(result.items.single.agentId, 'codex');
    expect(result.items.single.status, 'degraded');
    expect(result.items.single.capabilities.single.status, 'unknown');
    expect(result.auditId, 'audit-1');
    expect(transport.calls, ['Agent一覧']);
  });

  test('Agent metadataの権限fieldと秘密値を拒否する', () async {
    for (final agent in [
      _agentMetadata(extra: {'permission': 'granted'}),
      _agentMetadata(
        authentication: {'method': 'unknown', 'secret_value_present': true},
      ),
    ]) {
      final transport = _Transport({
        'Agent一覧': {
          ..._response('Agent一覧', {
            'Agent': [agent],
          }),
          'evidence_source': 'INTERNAL_STATE',
        },
      });
      await expectLater(
        MobileProjectionClient(transport).agents(),
        throwsA(isA<BrokerClientException>()),
      );
    }
  });

  test('Agent一覧の証拠種別と重複IDを拒否する', () async {
    final invalidResponses = [
      {
        ..._response('Agent一覧', {
          'Agent': [_agentMetadata()],
        }),
        'evidence_source': 'FIXTURE',
      },
      {
        ..._response('Agent一覧', {
          'Agent': [_agentMetadata(), _agentMetadata()],
        }),
        'evidence_source': 'INTERNAL_STATE',
      },
      {
        ..._response('Agent一覧', {
          'Agent': List.generate(65, (_) => _agentMetadata()),
        }),
        'evidence_source': 'INTERNAL_STATE',
      },
    ];
    for (final response in invalidResponses) {
      await expectLater(
        MobileProjectionClient(_Transport({'Agent一覧': response})).agents(),
        throwsA(isA<BrokerClientException>()),
      );
    }
  });

  test('通知はsummaryと件数を検証して本文を受け取らない', () async {
    final transport = _Transport({
      '通知一覧': _response('通知一覧', {
        '版': 1,
        '通知一覧': [
          {
            '版': 1,
            '通知ID': 'notification-1',
            'source': 'Broker',
            'severity': 'warning',
            'タイトル': 'Brokerの状態更新',
            '概要': 'Broker操作を保留しました。',
            '作成順': 1,
            '関連監査ID': 'audit-1',
            '関連監査hash': 'sha256:NaN',
            '遷移先': 'dashboard',
            '状態': 'unread',
            '通知hash': 'sha256:NaN',
            '表示範囲': 'summary',
            '権限生成': 'なし',
          },
        ],
        '件数': 1,
        '未読件数': 1,
        '重大件数': 0,
        '証拠種別': 'INTERNAL_STATE',
        '表示範囲': 'summary',
        '権限生成': 'なし',
        '操作': 'navigation_only',
      }),
    });

    final result = await MobileProjectionClient(transport).notifications();

    expect(result.count, 1);
    expect(result.items.single.summary, contains('保留'));
    expect(transport.calls, ['通知一覧']);
  });

  test('停止要求は実停止済みを受理しない', () async {
    final transport = _Transport({
      '全Runtime停止要求': _response('全Runtime停止要求', {
        '版': 1,
        '要求種別': '全Runtime停止要求',
        '対象': [
          {
            '実行系ID': 'local',
            '状態': 'ready',
            '承認必要': true,
            '停止実行済み': false,
            '再承認状態': 'owner_reapproval_required',
            '復旧ID': 'recover-runtime-stop-request',
          },
        ],
        '承認状態': 'owner_reapproval_required',
        '停止実行済み': false,
        '証拠種別': 'INTERNAL_STATE',
        '権限生成': 'なし',
        '復旧ID': 'recover-runtime-stop-request',
      }),
    });

    final result = await MobileProjectionClient(transport).stopRequest();

    expect(result.stopExecuted, isFalse);
    expect(result.targets.single.approvalRequired, isTrue);
  });

  test('通知の権限生成fieldを改変した応答を拒否する', () async {
    final transport = _Transport({
      '通知一覧': _response('通知一覧', {
        '版': 1,
        '通知一覧': <Object?>[],
        '件数': 0,
        '未読件数': 0,
        '重大件数': 0,
        '証拠種別': 'INTERNAL_STATE',
        '表示範囲': 'summary',
        '権限生成': 'permission-granted',
        '操作': 'navigation_only',
      }),
    });

    await expectLater(
      MobileProjectionClient(transport).notifications(),
      throwsA(isA<BrokerClientException>()),
    );
  });
}
