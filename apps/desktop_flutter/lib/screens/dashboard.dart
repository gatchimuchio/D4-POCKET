import 'package:flutter/material.dart';

import '../services/shell_core_client.dart';
import 'shared.dart';

class Dashboard extends StatelessWidget {
  const Dashboard({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final snapshot = client.getSnapshot();
    final phase = snapshot.phaseStatus;
    final operation = snapshot.operationStatus;
    final evidence = snapshot.evidenceSummary;
    return ShellPage(
      title: '概要',
      evidenceTitle: 'Dashboard',
      children: [
        SectionList(
          title: '段階状態',
          rows: [
            '段階A: ${phase.phaseAStatus}',
            '段階B: ${phase.phaseBStatus}',
            '段階C: ${phase.phaseCStatus}',
            '段階D: ${phase.phaseDStatus}',
            '段階E: ${phase.phaseEStatus}',
            '段階F: ${phase.phaseFStatus}',
            '所有者利用: 稼働中',
            '完成製品リリース: ${phase.completedProductReleaseClaimed ? '主張済み' : '未主張'}',
            'Windowsインストール先の厳格証拠: 保留',
          ],
        ),
        MetricRow(
          items: [
            MetricItem(
              label: '実行系状態',
              evidenceLabel: 'Runtime Status',
              value: operation.runtimeStatus,
            ),
            MetricItem(
              label: '不変条件状態',
              evidenceLabel: 'Invariant Status',
              value: operation.invariantStatus,
            ),
            MetricItem(
              label: '保留中の承認',
              value: '${operation.pendingApprovalsCount}',
            ),
            MetricItem(label: '問題', value: '${operation.problemsCount}'),
            MetricItem(label: '証拠', value: evidence.evidenceBundle),
            MetricItem(
              label: '復旧',
              value: '${snapshot.recoveryPlaybook.length}件',
            ),
          ],
        ),
        const SectionList(
          title: '所有者操作の境界',
          rows: [
            '段階Bの所有者利用機能は完了しています。',
            '完成製品としてのリリースは主張していません。',
            'リリース証拠の不足は完成製品リリースを遮断しますが、段階Bの所有者利用は遮断しません。',
          ],
        ),
        SnapshotInfoPanel(snapshot: snapshot),
        if (snapshot.snapshotSource == 'fallback')
          const EmptyStatePanel(
            title: 'ローカルスナップショットなし',
            meaning: 'GUI-Shellは所有者のローカルスナップショットではなく、安全な代替射影を使用しています。',
            phaseBBlocked: false,
            nextAction: 'ローカル状態を確認する前に、開発診断スナップショットを更新してください。',
          ),
        SectionList(
          title: '信頼状態',
          rows: [
            for (final trust in snapshot.trustRecords)
              '${trust.scope}: ${trust.state} (${trust.source})',
          ],
        ),
        SectionList(
          title: '問題／遮断要因',
          rows: snapshot.problems.isEmpty
              ? [
                  '現在のスナップショットに問題はありません。',
                  '段階Bの所有者利用は遮断されていません。',
                  'ローカル状態が変わった場合は、証拠センターを開くか診断を更新してください。',
                ]
              : [
                  for (final problem in snapshot.problems)
                    '${problem.item}: ${problem.classification}; リリース遮断: ${problem.blocksRelease ? 'はい' : 'いいえ'}',
                ],
        ),
        SectionList(
          title: '証拠概要',
          rows: [
            'schema_check: ${evidence.schemaCheck}',
            'conformance_skeleton: 通過、${evidence.conformanceCheckCount}件の check',
            'release_smoke: ${evidence.releaseSmoke}',
            'release_gate_check: ${evidence.releaseGateCheck}',
            'evidence_bundle: ${evidence.evidenceBundle}',
            'validate_all: ${evidence.validateAll}',
            'strict_windows_release: ${evidence.strictWindowsRelease}',
          ],
        ),
        SectionList(
          title: '最近の監査事象',
          rows: [
            for (final event in snapshot.auditEvents)
              '${event.eventId}: ${event.action} ${event.result}',
          ],
        ),
        SectionList(
          title: '実行系状態',
          rows: [
            for (final runtime in snapshot.runtimes)
              '${runtime.name}  ${runtime.status}  ${runtime.adapterId}',
          ],
        ),
        SectionList(
          title: '不変条件状態',
          rows: [
            for (final entry in snapshot.invariantFlags.entries)
              '${entry.key}: ${entry.value ? '違反' : '正常'}',
          ],
        ),
      ],
    );
  }
}
