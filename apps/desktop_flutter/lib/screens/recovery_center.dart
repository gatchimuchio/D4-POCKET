import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class RecoveryCenter extends StatefulWidget {
  const RecoveryCenter({super.key, required this.client, this.active = true});

  final ShellCoreClient client;
  final bool active;

  @override
  State<RecoveryCenter> createState() => _RecoveryCenterState();
}

class _RecoveryCenterState extends State<RecoveryCenter> {
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
  void didUpdateWidget(covariant RecoveryCenter oldWidget) {
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
        _refreshFailure = '最新の復旧状態を取得できません。Broker接続を確認して再試行してください。';
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
    final recoveries = snapshot.recoveryActions;
    final report = _report;
    final unresolvedChecks = report?.checks
        .where((check) => check.status != 'pass')
        .toList(growable: false);
    return ShellPage(
      title: '復旧手順',
      children: [
        if (brokerMode) ...[
          Align(
            alignment: Alignment.centerLeft,
            child: OutlinedButton.icon(
              key: const ValueKey('recovery-center-refresh'),
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
              nextAction: '取得後に現在の復旧案内を表示します。',
              loading: _isRefreshing,
            ),
          if (report != null && unresolvedChecks!.isEmpty)
            const EmptyStatePanel(
              title: '復旧対応なし',
              meaning: '最新のBroker診断に未解決の復旧項目はありません。',
              phaseBBlocked: false,
              nextAction: 'Broker状態を継続して使用してください。',
            )
          else if (unresolvedChecks != null)
            for (final check in unresolvedChecks)
              _LiveRecoveryCheck(check: check),
        ] else if (snapshot.recoveryPlaybook.isEmpty)
          const EmptyStatePanel(
            title: '復旧対応なし',
            meaning: '表示用Snapshotに復旧手順がありません。',
            phaseBBlocked: false,
            nextAction: 'Brokerへ接続して最新診断を取得してください。',
          )
        else if (!brokerMode)
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: const [
                DataColumn(label: Text('復旧')),
                DataColumn(label: Text('項目')),
                DataColumn(label: Text('重大度')),
                DataColumn(label: Text('分類')),
                DataColumn(label: Text('段階Bで継続可能')),
                DataColumn(label: Text('所有者利用を遮断')),
                DataColumn(label: Text('製品リリースを遮断')),
                DataColumn(label: Text('必要な対応')),
                DataColumn(label: Text('コマンド')),
                DataColumn(label: Text('パス')),
                DataColumn(label: Text('コピー')),
              ],
              rows: [
                for (final item in snapshot.recoveryPlaybook)
                  DataRow(
                    cells: [
                      DataCell(Text(item.recoveryId)),
                      DataCell(Text(item.item)),
                      DataCell(Text(item.severity)),
                      DataCell(Text(item.classification)),
                      DataCell(Text(item.safeToIgnoreForPhaseB ? 'はい' : 'いいえ')),
                      DataCell(Text(item.blocksOwnerUse ? 'はい' : 'いいえ')),
                      DataCell(
                        Text(item.blocksCompletedProductRelease ? 'はい' : 'いいえ'),
                      ),
                      DataCell(Text(item.requiredAction)),
                      DataCell(Text(item.command)),
                      DataCell(Text(item.path)),
                      DataCell(_RecoveryCopyActions(item: item)),
                    ],
                  ),
              ],
            ),
          ),
        if (!brokerMode)
          for (final recovery in recoveries)
            BorderedPanel(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  ListTile(
                    contentPadding: EdgeInsets.zero,
                    leading: const Icon(Icons.health_and_safety_outlined),
                    title: Text(recovery.recoveryId),
                    subtitle: Text(recovery.message),
                    trailing: Text(recovery.safeToRetry ? '再試行可能' : '遮断中'),
                  ),
                  SectionList(
                    title: '手順',
                    rows: [
                      '重大度: ${recovery.severity}',
                      '自動修正: 不可',
                      '事前確認: Shell Coreの許可を検証',
                      '対応手順: コマンドをコピー／ログを開く／承認済み能力を通じて再試行',
                      '事後確認: 検証または環境診断を再実行',
                      '巻戻し: 関連する監査事象と復旧対応を使用',
                    ],
                  ),
                ],
              ),
            ),
      ],
    );
  }
}

class _LiveRecoveryCheck extends StatelessWidget {
  const _LiveRecoveryCheck({required this.check});

  final SetupDoctorCheckRecord check;

  @override
  Widget build(BuildContext context) {
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          ListTile(
            contentPadding: EdgeInsets.zero,
            leading: const Icon(Icons.health_and_safety_outlined),
            title: Text(check.checkId),
            subtitle: Text(check.message),
            trailing: Text(_statusLabel(check.status)),
          ),
          SectionList(
            title: 'Brokerの復旧案内',
            rows: [
              '証拠種別: ${check.evidenceClass ?? 'unknown'}',
              check.recoveryInstruction?.isNotEmpty == true
                  ? check.recoveryInstruction!
                  : 'Brokerから復旧案内は返されていません。',
              '診断表示は再試行の安全性や実行許可を意味しません。',
            ],
          ),
        ],
      ),
    );
  }

  String _statusLabel(String status) => switch (status) {
        'fail' => '失敗',
        'warning' => '警告',
        'unknown' => '不明',
        _ => status,
      };
}

class _RecoveryCopyActions extends StatelessWidget {
  const _RecoveryCopyActions({required this.item});

  final RecoveryPlaybookRecord item;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 4,
      children: [
        if (item.command.isNotEmpty)
          IconButton(
            tooltip: 'コマンドをコピー',
            icon: const Icon(Icons.copy, size: 18),
            onPressed: () =>
                Clipboard.setData(ClipboardData(text: item.command)),
          ),
        if (item.path.isNotEmpty)
          IconButton(
            tooltip: 'パスをコピー',
            icon: const Icon(Icons.folder_copy_outlined, size: 18),
            onPressed: () => Clipboard.setData(ClipboardData(text: item.path)),
          ),
      ],
    );
  }
}
