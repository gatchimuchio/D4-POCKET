import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/models/generated_contracts.dart';
import 'package:gui_shell_desktop/services/global_search_index.dart';
import 'package:gui_shell_desktop/services/workspace_client.dart';
import 'package:gui_shell_ui/runtime_resource_client.dart' show BrokerTransport;

void main() {
  final now = DateTime.utc(2026, 11, 16, 12);

  test('検索対象は現在Host・Workspace・期限内Workspace読取許可だけ', () {
    final entries = GlobalSearchIndex.build(
      hosts: const [
        HostRegistryRecord(
          hostId: 'host-linux-lab',
          displayName: 'Linux lab',
          platform: 'linux',
          connectionState: 'pending_review',
          trustState: 'pending_review',
          runtimeCount: 0,
          agentCount: 0,
          evidenceSource: 'INTERNAL_STATE',
          visibility: 'metadata_only',
          authorityStrip: true,
        ),
      ],
      workspaces: [
        _workspace('workspace-live',
            approved: true, expiresAt: now.add(const Duration(minutes: 5))),
        _workspace('workspace-expired',
            approved: true,
            expiresAt: now.subtract(const Duration(minutes: 1))),
        _workspace('workspace-denied', approved: false),
      ],
      now: now,
    );

    expect(entries.map((entry) => entry.category).toSet(),
        {'Host', 'Workspace', '許可済み操作'});
    expect(GlobalSearchIndex.search(entries, 'Linux lab').single.target,
        GlobalSearchTarget.host);
    expect(GlobalSearchIndex.search(entries, 'workspace-live'), hasLength(2));
    expect(
        GlobalSearchIndex.search(entries, 'workspace-expired'), hasLength(1));
    expect(GlobalSearchIndex.search(entries, 'workspace-denied'), hasLength(1));
    expect(GlobalSearchIndex.search(entries, 'Workspace読取').single.target,
        GlobalSearchTarget.grantedAction);
    for (final oldCategory in [
      'MCP',
      'A2A',
      'Agent',
      'Audit',
      'Approval',
      'Permission',
      '問題',
      '復旧',
      '診断',
      'Profile',
    ]) {
      expect(GlobalSearchIndex.search(entries, oldCategory), isEmpty,
          reason: '$oldCategoryは検索対象に含めない');
    }
  });

  test('検索用Broker取得はHost一覧と作業領域一覧だけを現在再取得する', () async {
    final transport = _SearchTransport();
    final entries = await GlobalSearchIndex.loadCurrent(transport, now: now);

    expect(transport.operations.toSet(), {'Host一覧', '作業領域一覧'});
    expect(transport.requests, hasLength(2));
    expect(
      transport.requests.singleWhere(
        (request) => request['operation'] == 'Host一覧',
      )['payload'],
      {'版': 1},
    );
    expect(
      transport.requests.singleWhere(
        (request) => request['operation'] == '作業領域一覧',
      )['payload'],
      <String, Object?>{},
    );
    expect(entries.map((entry) => entry.evidenceSource).toSet(),
        {'INTERNAL_STATE'});
    expect(GlobalSearchIndex.search(entries, 'fixture-secret-marker'), isEmpty);
    expect(
      transport.operations.any((operation) =>
          operation.contains('Permission') ||
          operation.contains('Approval') ||
          operation.contains('実行')),
      isFalse,
    );
  });

  test('検索取得が失敗したら古い値へfallbackしない', () async {
    final transport = _SearchTransport(failWorkspaceList: true);

    await expectLater(
      GlobalSearchIndex.loadCurrent(transport, now: now),
      throwsA(isA<StateError>()),
    );
    expect(transport.operations.toSet(), {'Host一覧', '作業領域一覧'});
  });

  test('検索を再生成すると更新されたBroker Hostを読み直す', () async {
    final transport = _SearchTransport();
    final first = await GlobalSearchIndex.loadCurrent(transport, now: now);
    transport.hostDisplayName = 'Updated Linux';
    final second = await GlobalSearchIndex.loadCurrent(transport, now: now);

    expect(GlobalSearchIndex.search(first, 'Current Linux'), hasLength(1));
    expect(GlobalSearchIndex.search(first, 'Updated Linux'), isEmpty);
    expect(GlobalSearchIndex.search(second, 'Current Linux'), isEmpty);
    expect(GlobalSearchIndex.search(second, 'Updated Linux'), hasLength(1));
    expect(transport.operations, hasLength(4));
  });

  test('検索結果とqueryをboundedにし、未許可のSnapshot分類を混ぜない', () {
    final entries = GlobalSearchIndex.build(
      hosts: List<HostRegistryRecord>.generate(
        80,
        (index) => HostRegistryRecord(
          hostId: 'host-$index',
          displayName: 'Host $index',
          platform: 'linux',
          connectionState: 'pending_review',
          trustState: 'pending_review',
          runtimeCount: 0,
          agentCount: 0,
          evidenceSource: 'INTERNAL_STATE',
          visibility: 'metadata_only',
          authorityStrip: true,
        ),
      ),
      workspaces: List<WorkspaceRegistration>.generate(
        16,
        (index) => _workspace(
          'workspace-$index',
          approved: true,
          expiresAt: now.add(const Duration(hours: 1)),
        ),
      ),
      now: now,
    );

    expect(entries.length, lessThanOrEqualTo(GlobalSearchIndex.maxEntries));
    expect(GlobalSearchIndex.search(entries, 'a' * 129), isEmpty);
    expect(
      GlobalSearchIndex.search(entries, '').length,
      lessThanOrEqualTo(GlobalSearchIndex.maxResults),
    );
    expect(GlobalSearchIndex.search(entries, 'private-secret-marker'), isEmpty);
  });

  test('Host表示metadataから不可視・双方向制御文字を除去する', () {
    final entries = GlobalSearchIndex.build(
      hosts: const [
        HostRegistryRecord(
          hostId: 'host-safe',
          displayName: 'Li\u200Bnx\u202Elab',
          platform: 'linux',
          connectionState: 'pending_review',
          trustState: 'pending_review',
          runtimeCount: 0,
          agentCount: 0,
          evidenceSource: 'INTERNAL_STATE',
          visibility: 'metadata_only',
          authorityStrip: true,
        ),
      ],
      workspaces: const [],
      now: now,
    );

    expect(entries.single.title, 'Linxlab');
    expect(entries.single.title, isNot(contains('\u200B')));
    expect(entries.single.title, isNot(contains('\u202E')));
  });
}

WorkspaceRegistration _workspace(
  String id, {
  required bool approved,
  DateTime? expiresAt,
}) {
  final expiredAt =
      expiresAt == null ? null : expiresAt.millisecondsSinceEpoch ~/ 1000;
  return WorkspaceRegistration.parse({
    '作業領域ID': id,
    '実行系ID': 'runtime-codex',
    '登録hash': 'sha256:${'a' * 64}',
    '承認状態': approved ? 'approved' : 'denied',
    '有効期限': approved ? expiredAt : null,
    '表示範囲': approved ? 'summary' : 'none',
    'approval_id': approved ? 'approval-$id' : null,
    'workspace_root': r'C:\work\fixture',
  });
}

class _SearchTransport implements BrokerTransport {
  _SearchTransport({this.failWorkspaceList = false});

  final bool failWorkspaceList;
  String hostDisplayName = 'Current Linux';
  final List<String> operations = [];
  final List<Map<String, Object?>> requests = [];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    requests.add({'operation': operation, 'payload': payload ?? {}});
    if (operation == 'Host一覧') {
      return _accepted(operation, {
        '版': 1,
        'Host一覧': [_host('host-current', hostDisplayName)],
        '件数': 1,
        '公開範囲': 'metadata_only',
        '証拠種別': 'INTERNAL_STATE',
      });
    }
    if (operation == '作業領域一覧') {
      if (failWorkspaceList) {
        throw StateError('fixture transport failure');
      }
      return _accepted(operation, {
        '作業領域': [
          {
            '作業領域ID': 'workspace-current',
            '実行系ID': 'runtime-codex',
            '登録hash': 'sha256:${'b' * 64}',
            '承認状態': 'approved',
            '有効期限': 1900000000,
            '表示範囲': 'summary',
            'approval_id': 'approval-current',
            'workspace_root': null,
          },
        ],
      });
    }
    throw StateError('unexpected search operation: $operation');
  }

  Map<String, Object?> _accepted(String operation, Object body) => {
        'operation': operation,
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'audit-search-${operations.length}',
        'error': null,
        'body': body,
      };
}

Map<String, Object?> _host(String id, String displayName) => {
      '版': 1,
      'Host ID': id,
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
        'hash': 'sha256:${'c' * 64}',
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
      '登録監査ID': 'audit-host-register-$id',
    };
