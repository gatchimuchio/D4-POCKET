import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/models/generated_contracts.dart';
import 'package:gui_shell_desktop/services/global_search_index.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';

void main() {
  test('全体検索はMCPとA2Aを表示専用のsurface結果として見つける', () {
    final snapshot = ShellCoreClient.mock().getSnapshot();

    final mcp = GlobalSearchIndex.search(snapshot, 'MCP');
    final a2a = GlobalSearchIndex.search(snapshot, 'A2A');

    expect(mcp, isNotEmpty);
    expect(mcp.first.title, 'MCP接続');
    expect(mcp.first.pageIndex, 11);
    expect(a2a.first.title, 'A2A接続');
    expect(a2a.first.pageIndex, 20);
    expect(a2a.first.evidenceSource, '不明');
  });

  test('全体検索はqueryと結果件数をboundedにしraw contentを射影しない', () {
    final snapshot = ShellCoreClient.mock().getSnapshot();

    expect(
      GlobalSearchIndex.search(snapshot, 'a' * 129),
      isEmpty,
    );
    final results = GlobalSearchIndex.search(snapshot, '');
    expect(results.length, lessThanOrEqualTo(GlobalSearchIndex.maxResults));
    for (final result in results) {
      expect(result.detail, isNot(contains('projected_content')));
    }
  });

  test('全体検索はBrokerのAgent Adapter一覧を権限なしの表示metadataとして検索する', () {
    final snapshot = ShellCoreClient.mock().getSnapshot().copyWith(
      snapshotSource: 'broker',
      agentAdapters: const [
        AgentAdapterRecord(
          adapterId: 'anthropic-adapter',
          agentId: 'claude-agent',
          provider: 'Anthropic',
          version: '1.2',
          model: 'claude-sonnet',
          status: 'ready',
          capabilities: [
            AgentCapabilitySupportRecord(
              capabilityId: 'task_execution',
              status: 'supported',
              reason: 'free-form capability reason must not enter search',
            ),
          ],
          evidenceSource: 'LIVE_RUNTIME',
          evidenceReason: 'credential-secret-marker',
          authenticationMethod: 'credential-reference-secret-marker',
          authenticationStatus: 'configured',
        ),
      ],
    );

    final results = GlobalSearchIndex.search(snapshot, 'claude-sonnet');

    expect(results, hasLength(1));
    expect(results.single.category, 'Agent Adapter');
    expect(results.single.title, 'claude-agent');
    expect(results.single.recordId, 'anthropic-adapter');
    expect(results.single.pageIndex, 5);
    expect(results.single.evidenceSource, 'INTERNAL_STATE');
    expect(results.single.detail, contains('能力metadataは権限ではありません'));
    expect(results.single.detail, isNot(contains('credential-secret-marker')));
    expect(
      GlobalSearchIndex.search(snapshot, 'credential-secret-marker'),
      isEmpty,
    );
    expect(
      GlobalSearchIndex.search(snapshot, 'free-form capability reason'),
      isEmpty,
    );
  });

  test('全体検索はBrokerのHost能力を表示専用で検索し理由本文を除外する', () {
    final snapshot = ShellCoreClient.mock().getSnapshot().copyWith(
      snapshotSource: 'broker',
      hostCapabilities: const [
        HostCapabilityRecord(
          hostId: 'host-lab',
          displayName: 'Linux lab',
          platform: 'linux',
          status: 'connected',
          capabilities: [
            HostCapabilityItemRecord(
              capabilityId: 'workspace.read',
              status: 'available',
              evidenceSource: 'LIVE_RUNTIME',
              reason: 'host capability reason-secret-marker',
            ),
          ],
        ),
      ],
    );

    final results = GlobalSearchIndex.search(snapshot, 'workspace.read');

    expect(results, hasLength(1));
    expect(results.single.category, 'Host能力');
    expect(results.single.title, 'workspace.read');
    expect(results.single.recordId, 'host-lab');
    expect(results.single.pageIndex, 19);
    expect(results.single.evidenceSource, 'INTERNAL_STATE');
    expect(results.single.detail,
        contains('能力metadataはPermission／Authorityではありません'));
    expect(results.single.detail, isNot(contains('reason-secret-marker')));
    expect(
      GlobalSearchIndex.search(snapshot, 'reason-secret-marker'),
      isEmpty,
    );
  });

  test('Host能力の検索metadataから不可視・双方向制御文字を除去する', () {
    final snapshot = ShellCoreClient.mock().getSnapshot().copyWith(
      hostCapabilities: const [
        HostCapabilityRecord(
          hostId: 'host-safe',
          displayName: 'Linux',
          platform: 'linux',
          status: 'connected',
          capabilities: [
            HostCapabilityItemRecord(
              capabilityId: 'workspace.\u200Bread',
              status: 'available\u202E',
              evidenceSource: 'FIXTURE',
              reason: '',
            ),
          ],
        ),
      ],
    );

    final results = GlobalSearchIndex.search(snapshot, 'workspace.read');

    expect(results, hasLength(1));
    expect(results.single.title, 'workspace.read');
    expect(results.single.detail, isNot(contains('\u202E')));
    expect(results.single.detail, isNot(contains('\u200B')));
  });

  test('Agent Adapterの表示metadataから不可視・双方向制御文字を除去する', () {
    final snapshot = ShellCoreClient.mock().getSnapshot().copyWith(
      agentAdapters: const [
        AgentAdapterRecord(
          adapterId: 'adapter-safe',
          agentId: 'agent-safe',
          provider: 'Anthro\u202Epic',
          version: '1',
          model: 'claude\u200Bsonnet',
          status: 'ready',
          capabilities: [],
          evidenceSource: 'FIXTURE',
          evidenceReason: '',
        ),
      ],
    );

    final results = GlobalSearchIndex.search(snapshot, 'claudesonnet');

    expect(results, hasLength(1));
    expect(results.single.detail, contains('Anthropic'));
    expect(results.single.detail, isNot(contains('\u202E')));
    expect(results.single.detail, isNot(contains('\u200B')));
  });

  test('取込JSONのsnapshot_source自己申告はBroker証拠へ昇格しない', () {
    final snapshot = ShellSnapshot.fromJson({
      'snapshot_source': 'broker',
      'runtimes': [
        {
          'runtime_id': 'forged-runtime',
          'name': '未検証Runtime',
          'status': 'ready',
          'adapter_id': 'unverified-adapter',
        },
      ],
    });

    expect(snapshot.snapshotSource, 'unverified');
    final evidenceSources = GlobalSearchIndex.build(snapshot)
        .map((result) => result.evidenceSource)
        .toSet();
    expect(evidenceSources, {'不明'});
  });
}
