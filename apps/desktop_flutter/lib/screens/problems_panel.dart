import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class ProblemsPanel extends StatefulWidget {
  const ProblemsPanel({super.key, required this.client, this.active = true});

  final ShellCoreClient client;
  final bool active;

  @override
  State<ProblemsPanel> createState() => _ProblemsPanelState();
}

class _ProblemsPanelState extends State<ProblemsPanel> {
  int _requestGeneration = 0;
  bool _isRefreshing = false;
  SetupDoctorReportRecord? _report;
  String? _refreshFailure;

  @override
  void initState() {
    super.initState();
    if (widget.active) _scheduleRefresh();
  }

  @override
  void didUpdateWidget(covariant ProblemsPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.client, widget.client)) {
      _requestGeneration++;
      _isRefreshing = false;
      _report = null;
      _refreshFailure = null;
      if (widget.active) _scheduleRefresh();
    } else if (oldWidget.active != widget.active) {
      if (widget.active) {
        _scheduleRefresh();
      } else {
        _requestGeneration++;
        _isRefreshing = false;
      }
    }
  }

  void _scheduleRefresh() {
    final generation = _requestGeneration;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted &&
          generation == _requestGeneration &&
          !_isRefreshing &&
          widget.active &&
          widget.client.mode == 'broker') {
        unawaited(_refresh());
      }
    });
  }

  Future<void> _refresh() async {
    if (!widget.active || widget.client.mode != 'broker') return;
    final client = widget.client;
    final generation = ++_requestGeneration;
    setState(() {
      _isRefreshing = true;
      _report = null;
      _refreshFailure = null;
    });
    try {
      final report = await client.refreshSetupDoctorReport();
      if (!mounted ||
          generation != _requestGeneration ||
          !identical(client, widget.client)) {
        return;
      }
      setState(() => _report = report);
    } on Object {
      if (!mounted ||
          generation != _requestGeneration ||
          !identical(client, widget.client)) {
        return;
      }
      setState(() {
        _report = null;
        _refreshFailure = '最新の問題状態を取得できません。Broker接続を確認して再試行してください。';
      });
    } finally {
      if (mounted && generation == _requestGeneration) {
        setState(() => _isRefreshing = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final client = widget.client;
    final snapshot = client.getSnapshot();
    final brokerMode = client.mode == 'broker';
    final problems = snapshot.problems;
    final report = _report;
    final unresolvedChecks = report?.checks
        .where((check) => check.status != 'pass')
        .toList(growable: false);
    final recoveryById = {
      for (final recovery in snapshot.recoveryPlaybook)
        if (recovery.recoveryId.isNotEmpty) recovery.recoveryId: recovery,
    };
    final authorityByRecoveryId = {
      for (final authority in snapshot.authorityMap)
        if (authority.recoveryId.isNotEmpty) authority.recoveryId: authority,
    };
    return ShellPage(
      title: '問題一覧',
      children: [
        if (brokerMode)
          const SectionList(
            title: 'Broker診断の表示範囲',
            rows: [
              '監査付きSetup Doctor報告のうち、正常以外のcheckを表示します。',
              'この画面は全製品Release blockerの一覧ではなく、診断表示から権限や復旧実行を生成しません。',
            ],
          )
        else ...[
          const SectionList(
            title: '段階Bの境界',
            rows: [
              'リリース遮断要因は、段階Bの所有者利用を失敗扱いにせず、ここに表示します。',
              '各行は表示専用です。権限判断と復旧実行は引き続きShell Coreの責任です。',
            ],
          ),
          const SectionList(
            title: '問題と復旧の対応',
            rows: [
              '`safe_to_ignore_for_phase_b=true`は、所有者が段階Bの日常操作を継続できることを示します。',
              '`blocks_owner_use=true`は、段階Bの完了状態を維持する前に所有者利用循環への対応が必要であることを示します。',
              '`blocks_completed_product_release=true`は、後続の厳格リリースを遮断します。',
            ],
          ),
        ],
        if (brokerMode) ...[
          Align(
            alignment: Alignment.centerLeft,
            child: OutlinedButton.icon(
              key: const ValueKey('problems-panel-refresh'),
              onPressed: widget.active && !_isRefreshing
                  ? () => unawaited(_refresh())
                  : null,
              icon: _isRefreshing
                  ? const SizedBox(
                      width: 18,
                      height: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.refresh),
              label: Text(_isRefreshing ? '取得中…' : 'Brokerから最新状態を取得'),
            ),
          ),
          if (report != null)
            SectionList(
              title: '現在のBroker診断',
              rows: [
                '状態: ${report.status}',
                if (report.reportId != null) '診断報告ID: ${report.reportId}',
                if (report.generatedAt != null)
                  'Broker報告時刻: ${report.generatedAt}',
                '監査イベントID: ${report.auditEventId}',
              ],
            )
          else if (_refreshFailure != null)
            BrokerReportStatusPanel(
              title: '最新状態が不明',
              meaning: _refreshFailure!,
              nextAction: 'Broker接続を確認し、最新状態を再取得してください。',
            )
          else
            BrokerReportStatusPanel(
              title: _isRefreshing ? '取得中' : 'Broker状態を未取得',
              meaning: _isRefreshing
                  ? 'Brokerの監査付き診断報告を取得しています。'
                  : 'この画面を開くとBrokerから現在の診断報告を取得します。',
              nextAction: '取得後に現在の問題だけを表示します。',
              loading: _isRefreshing,
            ),
          if (report != null && unresolvedChecks!.isEmpty)
            const EmptyStatePanel(
              title: '問題なし',
              meaning: '最新のBroker診断に未解決の問題はありません。',
              phaseBBlocked: false,
              nextAction: 'Broker状態を継続して使用してください。',
            )
          else if (unresolvedChecks != null)
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: DataTable(
                columns: const [
                  DataColumn(label: Text('診断ID')),
                  DataColumn(label: Text('状態')),
                  DataColumn(label: Text('証拠種別')),
                  DataColumn(label: Text('Brokerの観測')),
                  DataColumn(label: Text('必要な対応')),
                ],
                rows: [
                  for (final check in unresolvedChecks)
                    DataRow(
                      cells: [
                        DataCell(Text(check.checkId)),
                        DataCell(Text(_statusLabel(check.status))),
                        DataCell(Text(check.evidenceClass ?? 'unknown')),
                        DataCell(Text(check.message)),
                        DataCell(Text(
                          check.recoveryInstruction?.isNotEmpty == true
                              ? check.recoveryInstruction!
                              : 'Brokerから復旧案内は返されていません。',
                        )),
                      ],
                    ),
                ],
              ),
            ),
        ] else if (problems.isEmpty)
          const EmptyStatePanel(
            title: '問題なし',
            meaning: '現在のスナップショットには、所有者利用またはリリースの問題がありません。',
            phaseBBlocked: false,
            nextAction: '所有者利用を継続するか、ローカル変更後に診断を更新してください。',
          )
        else if (!brokerMode)
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: const [
                DataColumn(label: Text('項目')),
                DataColumn(label: Text('分類')),
                DataColumn(label: Text('復旧')),
                DataColumn(label: Text('段階Bで継続可能')),
                DataColumn(label: Text('所有者利用を遮断')),
                DataColumn(label: Text('製品リリースを遮断')),
                DataColumn(label: Text('理由')),
                DataColumn(label: Text('必要な対応')),
                DataColumn(label: Text('関連')),
                DataColumn(label: Text('コピー')),
              ],
              rows: [
                for (final problem in problems)
                  _problemRow(problem, recoveryById, authorityByRecoveryId),
              ],
            ),
          ),
      ],
    );
  }

  String _statusLabel(String status) => switch (status) {
        'fail' => '失敗',
        'warning' => '警告',
        'unknown' => '不明',
        _ => status,
      };

  DataRow _problemRow(
    ProblemRecord problem,
    Map<String, RecoveryPlaybookRecord> recoveryById,
    Map<String, AuthorityMapRecord> authorityByRecoveryId,
  ) {
    final recovery = recoveryById[problem.recoveryId];
    final authority = authorityByRecoveryId[problem.recoveryId];
    final safeForPhaseB = problem.safeToIgnoreForPhaseB ||
        recovery?.safeToIgnoreForPhaseB == true;
    final blocksOwnerUse =
        problem.blocksOwnerUse || recovery?.blocksOwnerUse == true;
    final blocksProductRelease = problem.blocksCompletedProductRelease ||
        problem.blocksRelease ||
        recovery?.blocksCompletedProductRelease == true;
    final commandOrPath = [
      if (recovery?.command.isNotEmpty == true) recovery!.command,
      if (recovery?.path.isNotEmpty == true) recovery!.path,
      if (recovery == null) problem.target,
    ].join(' | ');
    final related = [
      '復旧ID: ${problem.recoveryId}',
      if (problem.target.isNotEmpty) '証拠／パス: ${problem.target}',
      if (authority != null) '実行系ID: ${authority.runtimeId}',
      if (authority != null) '権限: ${authority.capabilityId}',
    ].join('\n');
    return DataRow(
      cells: [
        DataCell(Text(problem.item.isEmpty ? problem.message : problem.item)),
        DataCell(Text(problem.classification)),
        DataCell(Text(problem.recoveryId)),
        DataCell(Text(safeForPhaseB ? 'はい' : 'いいえ')),
        DataCell(Text(blocksOwnerUse ? 'はい' : 'いいえ')),
        DataCell(Text(blocksProductRelease ? 'はい' : 'いいえ')),
        DataCell(Text(problem.reason)),
        DataCell(Text(problem.requiredAction)),
        DataCell(Text(related)),
        DataCell(
          _CopyActions(commandOrPath: commandOrPath, path: problem.target),
        ),
      ],
    );
  }
}

class _CopyActions extends StatelessWidget {
  const _CopyActions({required this.commandOrPath, required this.path});

  final String commandOrPath;
  final String path;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 4,
      children: [
        if (commandOrPath.isNotEmpty)
          IconButton(
            tooltip: 'コマンドまたはパスをコピー',
            icon: const Icon(Icons.copy, size: 18),
            onPressed: () =>
                Clipboard.setData(ClipboardData(text: commandOrPath)),
          ),
        if (path.isNotEmpty)
          IconButton(
            tooltip: '関連パスをコピー',
            icon: const Icon(Icons.folder_copy_outlined, size: 18),
            onPressed: () => Clipboard.setData(ClipboardData(text: path)),
          ),
      ],
    );
  }
}
