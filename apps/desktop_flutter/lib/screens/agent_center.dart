import 'package:flutter/material.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import '../services/agent_coordination.dart';
import 'shared.dart';
import 'workspace_inspector.dart';

class AgentCenter extends StatelessWidget {
  const AgentCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final snapshot = client.getSnapshot();
    final adapters = snapshot.agentAdapters;
    final sessions = client.mode == 'broker'
        ? snapshot.agentSessions
        : const <AgentSessionRecord>[];
    final comparison = AgentComparisonProjection.fromSessions(sessions);
    return ShellPage(
      title: 'エージェントセンター',
      children: [
        if (client.workspaceClient != null)
          BorderedPanel(
              child: WorkspaceInspector(client: client.workspaceClient!)),
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'Agent Adapter状態',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 8),
              if (adapters.isEmpty)
                const SectionList(
                  title: '状態',
                  rows: ['Agent Adapterの実物登録はありません。unknownを利用可能へ昇格しません。'],
                )
              else
                for (final adapter in adapters)
                  _AgentAdapterPanel(adapter: adapter),
            ],
          ),
        ),
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                '複数Agent比較',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 8),
              SectionList(
                title: '状態',
                rows: [
                  if (client.mode != 'broker')
                    'Local／mock snapshotはAgent実行結果ではないため比較対象にしません。'
                  else if (comparison.available)
                    'Broker metadata上の識別子重複はありませんが、実Agent比較・実行時隔離は未接続です。'
                  else
                    comparison.statusMessage,
                ],
              ),
              SectionList(
                title: '対象セッション',
                rows: comparison.sessionIds.isEmpty
                    ? ['なし']
                    : comparison.sessionIds,
              ),
              SectionList(
                title: '安全境界',
                rows: [
                  comparison.available
                      ? 'Agent runtime ID／Workspace参照の重複なし（実行時の隔離は未検証）'
                      : '比較条件が成立していません',
                  'Authority・Approval・Credentialは共有しない',
                ],
              ),
            ],
          ),
        ),
        const BorderedPanel(
          child: SectionList(
            title: 'Agent引き継ぎ',
            rows: [
              '未接続です。Task成果・diff・試験結果のBroker経路が成立するまで引き継ぎ概要を生成しません。',
              '接続後もAuthority・Permission・Approval・Credential・hidden contextは引き継がず、target条件で再評価します。',
            ],
          ),
        ),
        if (client.mode != 'broker' && snapshot.agentSessions.isNotEmpty)
          const BorderedPanel(
            child: SectionList(
              title: 'エージェント実行',
              rows: ['ローカルの模擬データは、Broker上のエージェント実行として表示しません。'],
            ),
          ),
        for (final session in sessions)
          BorderedPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  session.sessionId,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                SectionList(
                    title: 'Agent実行系ID', rows: [session.agentRuntimeId]),
                SectionList(title: '対話状態', rows: [session.status]),
                const SectionList(
                  title: '証拠種別',
                  rows: ['INTERNAL_STATE（Broker対話Session一覧）'],
                ),
                SectionList(title: '作成監査ID', rows: [session.auditEventId]),
                if (session.workspace.trim().isEmpty)
                  const Text('Workspace結合は未確認です。')
                else ...[
                  SectionList(
                    title: 'Broker登録Workspace ID',
                    rows: [session.workspace],
                  ),
                  SectionList(
                    title: 'Workspace結合監査ID',
                    rows: [session.workspaceAuditEventId],
                  ),
                  const Text(
                    'Runtime一致はBroker内で検証済みです。実Agentの実行Session・書込み隔離は未検証です。',
                  ),
                ],
                const Text(
                  'Task・diff・Tool・command内容は現在のBroker contractにないため表示しません。',
                ),
                const SectionList(
                  title: '未接続の実行情報',
                  rows: [
                    '作業領域: 実行directoryは未確認',
                    'タスク: Brokerから未取得',
                    '変更ファイル: Brokerから未取得',
                    '道具呼出し: Brokerから未取得',
                    'シェルコマンド: Brokerから未取得',
                    '試験状態: Brokerから未取得',
                    '差分概要: Brokerから未取得',
                    '保留中の承認: Brokerから未取得（承認がないことを意味しません）',
                    '巻戻し候補: Brokerから未取得',
                    '監査リンク: 上記の監査ID参照のみ',
                  ],
                ),
              ],
            ),
          ),
      ],
    );
  }
}

class _AgentAdapterPanel extends StatelessWidget {
  const _AgentAdapterPanel({required this.adapter});

  final AgentAdapterRecord adapter;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('${adapter.agentId} (${adapter.adapterId})'),
          SectionList(
            title: 'プロバイダー / モデル',
            rows: ['${adapter.provider} / ${adapter.model}'],
          ),
          SectionList(title: '状態', rows: [adapter.status]),
          SectionList(title: 'バージョン', rows: [adapter.version]),
          SectionList(title: '証拠種別', rows: [adapter.evidenceSource]),
          SectionList(title: '理由', rows: [adapter.evidenceReason]),
          SectionList(
            title: '能力',
            rows: adapter.capabilities.isEmpty
                ? ['unknown']
                : [
                    for (final capability in adapter.capabilities)
                      '${capability.capabilityId}: ${capability.status}',
                  ],
          ),
          SectionList(
            title: '境界',
            rows: [
              '作業領域: ${adapter.workspaceBoundary}',
              'プロセス起動: ${adapter.processSpawnStatus}',
              '認証方式: ${adapter.authenticationMethod}',
            ],
          ),
        ],
      ),
    );
  }
}
