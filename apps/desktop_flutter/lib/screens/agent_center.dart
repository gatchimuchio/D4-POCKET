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
    final sessions = snapshot.agentSessions;
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
              SectionList(title: '状態', rows: [comparison.statusMessage]),
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
        for (final session in sessions)
          BorderedPanel(
            child: _HandoffPanel(
              projection: AgentHandoffProjection.fromSession(session),
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
                SectionList(title: '作業領域', rows: [session.workspace]),
                SectionList(title: 'タスク', rows: [session.task]),
                SectionList(title: '変更ファイル', rows: session.changedFiles),
                SectionList(title: '道具呼出し', rows: session.toolCalls),
                SectionList(title: 'シェルコマンド', rows: session.shellCommands),
                SectionList(title: '試験状態', rows: [session.testStatus]),
                SectionList(title: '差分概要', rows: [session.diffSummary]),
                SectionList(
                  title: '保留中の承認',
                  rows: ['${session.pendingApprovalCount}'],
                ),
                SectionList(title: '巻戻し候補', rows: [session.rollbackCandidate]),
                SectionList(title: '監査リンク', rows: [session.auditEventId]),
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

class _HandoffPanel extends StatelessWidget {
  const _HandoffPanel({required this.projection});

  final AgentHandoffProjection projection;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Agent Handoff投影: ${projection.sessionId}',
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        SectionList(title: '状態', rows: [projection.statusMessage]),
        SectionList(title: 'Task概要', rows: [projection.taskSummary]),
        SectionList(title: '差分概要', rows: [projection.diffSummary]),
        SectionList(title: '試験状態', rows: [projection.testStatus]),
      ],
    );
  }
}
