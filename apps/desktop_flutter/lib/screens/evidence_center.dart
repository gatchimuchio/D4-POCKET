import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class EvidenceCenter extends StatelessWidget {
  const EvidenceCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final snapshot = client.getSnapshot();
    final summary = snapshot.evidenceSummary;
    return ShellPage(
      title: '証拠センター',
      children: [
        const SectionList(
          title: 'リリース境界',
          rows: [
            '段階Bの証拠センターは表示専用です。',
            'リリース証拠の不足は完成製品リリースを遮断しますが、段階Bの所有者利用は遮断しません。',
            'この画面はリリース証拠を生成しません。',
          ],
        ),
        SnapshotInfoPanel(snapshot: snapshot),
        _EvidenceBundleExportPanel(snapshot: snapshot),
        _SnapshotExchangePanel(snapshot: snapshot),
        SectionList(
          title: '検証概要',
          rows: [
            'schema_check: ${summary.schemaCheck}',
            'conformance_skeleton: 通過、${summary.conformanceCheckCount}件の check',
            'release_smoke: ${summary.releaseSmoke}',
            'release_gate_check: ${summary.releaseGateCheck}',
            'evidence_bundle: ${summary.evidenceBundle}',
            'validate_all: ${summary.validateAll}',
            'strict_windows_release: ${summary.strictWindowsRelease}',
            'Windows実測証拠の不足: ${summary.missingMeasuredWindowsEvidence ? 'release_blocker' : 'なし'}',
            '非合成の環境診断証拠の不足: ${summary.missingSetupDoctorEvidence ? 'release_blocker' : 'なし'}',
            '所有者GO: ${summary.ownerGo}',
          ],
        ),
        if (snapshot.evidence.isEmpty)
          const EmptyStatePanel(
            title: '証拠なし',
            meaning: '現在のスナップショットに表示対象の証拠記録がありません。',
            phaseBBlocked: false,
            nextAction: '証拠状態が必要なときは標準検証コマンドを実行してください。',
          )
        else
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: const [
                DataColumn(label: Text('証拠')),
                DataColumn(label: Text('種別')),
                DataColumn(label: Text('状態')),
                DataColumn(label: Text('パス')),
                DataColumn(label: Text('書出し可能')),
              ],
              rows: [
                for (final evidence in snapshot.evidence)
                  DataRow(
                    cells: [
                      DataCell(Text(evidence.evidenceId)),
                      DataCell(Text(evidence.kind)),
                      DataCell(Text(evidence.status)),
                      DataCell(Text(evidence.path)),
                      DataCell(Text(evidence.exportable ? 'はい' : 'いいえ')),
                    ],
                  ),
              ],
            ),
          ),
      ],
    );
  }
}

class _EvidenceBundleExportPanel extends StatelessWidget {
  const _EvidenceBundleExportPanel({required this.snapshot});

  final ShellSnapshot snapshot;

  @override
  Widget build(BuildContext context) {
    final summaryText = _validationSummaryText(snapshot);
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('証拠束の書き出し', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          const Text('表示専用の書出し補助です。既存のパス、概要、コマンドをコピーするだけで、リリース証拠は収集しません。'),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              OutlinedButton.icon(
                onPressed: () => Clipboard.setData(
                  const ClipboardData(
                    text: 'python3 tooling/evidence_bundle.py --check',
                  ),
                ),
                icon: const Icon(Icons.copy),
                label: const Text('検査コマンドをコピー'),
              ),
              OutlinedButton.icon(
                onPressed: () =>
                    Clipboard.setData(ClipboardData(text: summaryText)),
                icon: const Icon(Icons.summarize_outlined),
                label: const Text('検証概要をコピー'),
              ),
              OutlinedButton.icon(
                onPressed: () => Clipboard.setData(
                  const ClipboardData(text: 'release_evidence'),
                ),
                icon: const Icon(Icons.folder_copy_outlined),
                label: const Text('証拠フォルダーをコピー'),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(summaryText),
        ],
      ),
    );
  }
}

class _SnapshotExchangePanel extends StatelessWidget {
  const _SnapshotExchangePanel({required this.snapshot});

  final ShellSnapshot snapshot;

  @override
  Widget build(BuildContext context) {
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'スナップショットの読込み／書出し',
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          const Text(
            'Flutterでのスナップショット読込みは事前確認専用です。読み込んだ状態の適用はShell Coreの責任です。',
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              OutlinedButton.icon(
                onPressed: () => Clipboard.setData(
                  ClipboardData(text: _snapshotExportJson(snapshot)),
                ),
                icon: const Icon(Icons.ios_share_outlined),
                label: const Text('スナップショットJSONをコピー'),
              ),
              OutlinedButton.icon(
                onPressed: () => Clipboard.setData(
                  ClipboardData(text: snapshot.snapshotPath),
                ),
                icon: const Icon(Icons.folder_copy_outlined),
                label: const Text('スナップショットのパスをコピー'),
              ),
              OutlinedButton.icon(
                onPressed: () => showDialog<void>(
                  context: context,
                  builder: (context) =>
                      _SnapshotImportPreviewDialog(current: snapshot),
                ),
                icon: const Icon(Icons.compare_arrows_outlined),
                label: const Text('読込み前確認／比較'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _SnapshotImportPreviewDialog extends StatefulWidget {
  const _SnapshotImportPreviewDialog({required this.current});

  final ShellSnapshot current;

  @override
  State<_SnapshotImportPreviewDialog> createState() =>
      _SnapshotImportPreviewDialogState();
}

class _SnapshotImportPreviewDialogState
    extends State<_SnapshotImportPreviewDialog> {
  final TextEditingController _controller = TextEditingController();
  String _preview = 'スナップショットJSONを貼り付けると、出所、リリース状態、件数を事前確認できます。';

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 760, maxHeight: 680),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            children: [
              Text(
                'スナップショット読込みの事前確認',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 8),
              TextField(
                controller: _controller,
                maxLines: 8,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  labelText: 'スナップショットJSON',
                ),
                onChanged: _updatePreview,
              ),
              const SizedBox(height: 12),
              Expanded(
                child: SingleChildScrollView(
                  child: Align(
                    alignment: Alignment.topLeft,
                    child: Text(_preview),
                  ),
                ),
              ),
              const SizedBox(height: 8),
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  TextButton(
                    onPressed: () => Navigator.of(context).pop(),
                    child: const Text('閉じる'),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _updatePreview(String value) {
    if (value.trim().isEmpty) {
      setState(() {
        _preview = 'スナップショットJSONを貼り付けると、出所、リリース状態、件数を事前確認できます。';
      });
      return;
    }
    try {
      final json = jsonDecode(value) as Map<String, Object?>;
      final imported = ShellSnapshot.fromJson(json);
      setState(() {
        _preview = [
          '事前確認専用です。読み込んだデータは適用しません。',
          '現在の出所: ${widget.current.snapshotSource}',
          '読込み側の出所: ${imported.snapshotSource}',
          '現在のリリース状態: ${widget.current.operationStatus.releaseState}',
          '読込み側のリリース状態: ${imported.operationStatus.releaseState}',
          '現在の問題数: ${widget.current.problems.length}',
          '読込み側の問題数: ${imported.problems.length}',
          '現在の復旧行数: ${widget.current.recoveryPlaybook.length}',
          '読込み側の復旧行数: ${imported.recoveryPlaybook.length}',
          '現在の証拠行数: ${widget.current.evidence.length}',
          '読込み側の証拠行数: ${imported.evidence.length}',
          '読込み前確認で段階Bを遮断: ${imported.problems.any((problem) => problem.blocksOwnerUse) ? 'はい' : 'いいえ'}',
        ].join('\n');
      });
    } on Object catch (error) {
      setState(() {
        _preview = 'スナップショットJSONが不正です: $error';
      });
    }
  }
}

String _validationSummaryText(ShellSnapshot snapshot) {
  final summary = snapshot.evidenceSummary;
  return [
    'スキーマ検査(schema_check)=${summary.schemaCheck}',
    '適合検査数(conformance_checks)=${summary.conformanceCheckCount}',
    'リリース簡易検査(release_smoke)=${summary.releaseSmoke}',
    'リリース関門検査(release_gate_check)=${summary.releaseGateCheck}',
    '証拠束(evidence_bundle)=${summary.evidenceBundle}',
    '全検証(validate_all)=${summary.validateAll}',
    'Windows厳格リリース(strict_windows_release)=${summary.strictWindowsRelease}',
    'リリース状態(release_state)=${snapshot.operationStatus.releaseState}',
  ].join('\n');
}

String _snapshotExportJson(ShellSnapshot snapshot) {
  final json = {
    'snapshot_source': snapshot.snapshotSource,
    'snapshot_path': snapshot.snapshotPath,
    'snapshot_generated_at': snapshot.snapshotGeneratedAt,
    'snapshot_freshness': snapshot.snapshotFreshness,
    'operation_status': {
      'runtime_status': snapshot.operationStatus.runtimeStatus,
      'invariant_status': snapshot.operationStatus.invariantStatus,
      'trust_status': snapshot.operationStatus.trustStatus,
      'pending_approvals_count': snapshot.operationStatus.pendingApprovalsCount,
      'audit_chain_status': snapshot.operationStatus.auditChainStatus,
      'problems_count': snapshot.operationStatus.problemsCount,
      'release_state': snapshot.operationStatus.releaseState,
    },
    'phase_status': {
      'phase_a_status': snapshot.phaseStatus.phaseAStatus,
      'phase_b_status': snapshot.phaseStatus.phaseBStatus,
      'phase_c_status': snapshot.phaseStatus.phaseCStatus,
      'phase_d_status': snapshot.phaseStatus.phaseDStatus,
      'phase_e_status': snapshot.phaseStatus.phaseEStatus,
      'phase_f_status': snapshot.phaseStatus.phaseFStatus,
      'completed_product_release_claimed':
          snapshot.phaseStatus.completedProductReleaseClaimed,
    },
    'counts': {
      'runtimes': snapshot.runtimes.length,
      'problems': snapshot.problems.length,
      'evidence': snapshot.evidence.length,
      'recovery_playbook': snapshot.recoveryPlaybook.length,
      'pending_approvals': snapshot.pendingApprovals.length,
    },
    'release_ready_claimed': false,
  };
  return const JsonEncoder.withIndent('  ').convert(json);
}
