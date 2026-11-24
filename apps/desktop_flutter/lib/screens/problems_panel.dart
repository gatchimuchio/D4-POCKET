import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../models/generated_contracts.dart';
import '../services/runtime_resource_client.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class ProblemsPanel extends StatefulWidget {
  const ProblemsPanel({
    super.key,
    required this.client,
    this.resourceClient,
    this.connectResource = connectRuntimeResourceClient,
    this.stopReceipt,
    this.onStopReceiptChanged,
    this.active = true,
  });

  final ShellCoreClient client;
  final RuntimeResourceClient? resourceClient;
  final RuntimeResourceClientConnector connectResource;
  final TrayStopResult? stopReceipt;
  final ValueChanged<TrayStopResult?>? onStopReceiptChanged;
  final bool active;

  @override
  State<ProblemsPanel> createState() => _ProblemsPanelState();
}

class _ProblemsPanelState extends State<ProblemsPanel> {
  int _requestGeneration = 0;
  bool _isRefreshing = false;
  SetupDoctorReportRecord? _report;
  String? _refreshFailure;
  int _stopRequestGeneration = 0;
  bool _isSubmittingStopRequest = false;
  TrayStopResult? _stopReceipt;
  String? _stopRequestMessage;

  @override
  void initState() {
    super.initState();
    if (widget.active) _scheduleRefresh();
  }

  @override
  void didUpdateWidget(covariant ProblemsPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    final brokerChanged =
        !identical(oldWidget.client, widget.client) ||
        !identical(
          oldWidget.client.brokerTransport,
          widget.client.brokerTransport,
        );
    if (brokerChanged) {
      _requestGeneration++;
      _isRefreshing = false;
      _report = null;
      _refreshFailure = null;
      _stopRequestGeneration++;
      _isSubmittingStopRequest = false;
      _stopReceipt = null;
      _stopRequestMessage = 'Broker接続先が変わりました。前の接続の停止要求結果は破棄しました。';
      if (widget.active) _scheduleRefresh();
    } else if (oldWidget.active != widget.active) {
      if (widget.active) {
        _scheduleRefresh();
      } else {
        _requestGeneration++;
        _isRefreshing = false;
      }
    }
    if (oldWidget.stopReceipt != widget.stopReceipt &&
        widget.stopReceipt == null) {
      _stopReceipt = null;
    }
  }

  bool _isCurrentStopRequest(
    ShellCoreClient client,
    Object? transport,
    int generation,
  ) =>
      mounted &&
      generation == _stopRequestGeneration &&
      identical(client, widget.client) &&
      identical(transport, widget.client.brokerTransport);

  Future<void> _requestAllRuntimeStop() async {
    final client = widget.client;
    final transport = client.brokerTransport;
    if (!widget.active ||
        client.mode != 'broker' ||
        transport == null ||
        _isSubmittingStopRequest) {
      return;
    }

    final generation = ++_stopRequestGeneration;
    setState(() {
      _isSubmittingStopRequest = true;
      _stopReceipt = null;
      _stopRequestMessage = '現在のBrokerへ停止要求を送信しています。停止自体は実行しません。';
    });
    widget.onStopReceiptChanged?.call(null);
    try {
      final receipt = await client.requestAllRuntimeStop();
      if (!_isCurrentStopRequest(client, transport, generation)) return;
      setState(() {
        _stopReceipt = receipt;
        _stopRequestMessage = '現在のBrokerから停止要求Receiptを受け取りました。';
      });
      widget.onStopReceiptChanged?.call(receipt);
    } on Object {
      if (!_isCurrentStopRequest(client, transport, generation)) return;
      setState(() {
        _stopReceipt = null;
        _stopRequestMessage =
            '停止要求の結果を確認できません。現在のBroker状態を確認してください。自動再送はしていません。';
      });
      widget.onStopReceiptChanged?.call(null);
    } finally {
      if (_isCurrentStopRequest(client, transport, generation)) {
        setState(() => _isSubmittingStopRequest = false);
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
          _RuntimeResourceProblemsSection(
            client: client,
            resourceClient: widget.resourceClient,
            connectResource: widget.connectResource,
            active: widget.active,
          ),
        if (brokerMode) _stopRequestSection(),
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
                  DataColumn(label: Text('診断範囲')),
                  DataColumn(label: Text('状態')),
                  DataColumn(label: Text('重大度')),
                  DataColumn(label: Text('証拠種別')),
                  DataColumn(label: Text('Brokerの観測')),
                  DataColumn(label: Text('必要な対応')),
                ],
                rows: [
                  for (final check in unresolvedChecks)
                    DataRow(
                      cells: [
                        DataCell(Text(check.checkId)),
                        DataCell(Text(_scopeLabel(check.scope))),
                        DataCell(Text(_statusLabel(check.status))),
                        DataCell(Text(_severityLabel(check.severity))),
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

  Widget _stopRequestSection() {
    final receipt = widget.stopReceipt ?? _stopReceipt;
    final canRequest =
        widget.active &&
        widget.client.brokerTransport != null &&
        !_isSubmittingStopRequest;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionList(
          title: 'Runtime停止要求',
          rows: [
            '現在のBrokerに登録されたRuntime全体へ停止要求を送ります。',
            'この要求はRuntimeを停止せず、個別のOwner再承認を必要とします。',
          ],
        ),
        Align(
          alignment: Alignment.centerLeft,
          child: OutlinedButton.icon(
            key: const ValueKey('problems-panel-stop-request'),
            onPressed: canRequest
                ? () => unawaited(_requestAllRuntimeStop())
                : null,
            icon: _isSubmittingStopRequest
                ? const SizedBox(
                    width: 18,
                    height: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.stop_circle_outlined),
            label: Text(
              _isSubmittingStopRequest ? 'Broker応答を確認中…' : '全Runtime停止要求を送信',
            ),
          ),
        ),
        if (_stopRequestMessage != null)
          BorderedPanel(child: Text(_stopRequestMessage!)),
        if (receipt != null)
          SectionList(
            title: '現在のBroker接続の停止要求Receipt',
            rows: [
              '対象範囲: Brokerに登録された全Runtime（個別Host／Taskには結合しません）',
              '応答状態: ${receipt.status}',
              '対象Runtime数: ${receipt.targetCount}',
              '停止実行済み: ${receipt.stopPerformed ? 'はい' : 'いいえ'}',
              '承認状態: ${receipt.approvalState}',
              'Audit ID: ${receipt.auditId.isEmpty ? '未取得' : receipt.auditId}',
              'Recovery ID: ${receipt.recoveryId}',
            ],
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

  String _scopeLabel(String scope) => switch (scope) {
        'installation' => '導入',
        'runtime' => '実行環境',
        'authority' => '権限境界',
        'network' => '通信',
        'recovery' => '復旧',
        'audit' => '監査',
        'configuration' => '設定',
        _ => '不明',
      };

  String _severityLabel(String severity) => switch (severity) {
        'info' => '情報',
        'warning' => '注意',
        'critical' => '重大',
        _ => '不明',
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

const _runtimeResourceMetrics = <String, String>{
  'CPU利用率Percent': 'CPU利用率 (%)',
  'RAMWorkingSetBytes': 'RAM Working Set (bytes)',
  'RAMPrivateBytes': 'RAM Private (bytes)',
  'DiskIOBytes': 'Disk I/O (bytes)',
  'NetworkIOBytes': 'Network I/O (bytes)',
  '平均応答Millis': '平均応答 (ms)',
  '失敗要求数': '失敗要求数 (件)',
};

class _RuntimeResourceProblemsSection extends StatefulWidget {
  const _RuntimeResourceProblemsSection({
    required this.client,
    required this.resourceClient,
    required this.connectResource,
    required this.active,
  });

  final ShellCoreClient client;
  final RuntimeResourceClient? resourceClient;
  final RuntimeResourceClientConnector connectResource;
  final bool active;

  @override
  State<_RuntimeResourceProblemsSection> createState() =>
      _RuntimeResourceProblemsSectionState();
}

class _RuntimeResourceProblemsSectionState
    extends State<_RuntimeResourceProblemsSection> {
  RuntimeResourceClient? _resourceClient;
  List<String>? _runtimeIds;
  String? _selectedRuntimeId;
  RuntimeResourceObservation? _observation;
  String _message = 'Broker登録Runtime一覧を取得してください。';
  bool _loadingRuntimes = false;
  bool _observing = false;
  int _generation = 0;

  @override
  void initState() {
    super.initState();
    _resourceClient = widget.resourceClient;
  }

  @override
  void didUpdateWidget(covariant _RuntimeResourceProblemsSection oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.client, widget.client) ||
        oldWidget.resourceClient != widget.resourceClient ||
        oldWidget.connectResource != widget.connectResource) {
      _generation++;
      _resourceClient = widget.resourceClient;
      _runtimeIds = null;
      _selectedRuntimeId = null;
      _observation = null;
      _loadingRuntimes = false;
      _observing = false;
      _message = 'Broker接続条件が変わりました。Runtime一覧を再取得してください。';
    } else if (oldWidget.active && !widget.active) {
      _generation++;
      _loadingRuntimes = false;
      _observing = false;
      _observation = null;
      _message = '画面を離れたため、前回の資源観測を破棄しました。';
    }
  }

  Future<void> _refreshRuntimeIds() async {
    if (!widget.active || _loadingRuntimes || _observing) return;
    final client = widget.client;
    final generation = ++_generation;
    final previousSelection = _selectedRuntimeId;
    setState(() {
      _loadingRuntimes = true;
      _observation = null;
      _message = 'Brokerから登録済みRuntime一覧を取得しています。';
    });
    try {
      final runtimeIds = await client.refreshRuntimeCatalog();
      if (!mounted ||
          generation != _generation ||
          !identical(client, widget.client)) {
        return;
      }
      setState(() {
        _runtimeIds = runtimeIds;
        _selectedRuntimeId = runtimeIds.contains(previousSelection)
            ? previousSelection
            : (runtimeIds.isEmpty ? null : runtimeIds.first);
        _message = runtimeIds.isEmpty
            ? 'Brokerに登録済みRuntimeはありません。'
            : 'Broker登録済みRuntime ${runtimeIds.length}件を取得しました。';
      });
    } catch (_) {
      if (!mounted ||
          generation != _generation ||
          !identical(client, widget.client)) {
        return;
      }
      setState(() {
        _runtimeIds = null;
        _selectedRuntimeId = null;
        _observation = null;
        _message = 'Runtime一覧を確認できません。権限状態を推測せず停止しています。';
      });
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _loadingRuntimes = false);
      }
    }
  }

  void _selectRuntime(String? runtimeId) {
    if (runtimeId == null || !_runtimeIds!.contains(runtimeId)) return;
    _generation++;
    setState(() {
      _selectedRuntimeId = runtimeId;
      _observation = null;
      _message = 'Runtimeを変更しました。現在の対象を一回観測してください。';
    });
  }

  Future<void> _observeResources() async {
    final runtimeId = _selectedRuntimeId;
    final runtimeIds = _runtimeIds;
    if (!widget.active ||
        runtimeId == null ||
        runtimeIds == null ||
        !runtimeIds.contains(runtimeId) ||
        !RuntimeResourceClient.validRuntimeId(runtimeId) ||
        _loadingRuntimes ||
        _observing) {
      return;
    }
    final client = widget.client;
    final generation = ++_generation;
    setState(() {
      _observing = true;
      _observation = null;
      _message = '選択Runtimeの資源をBrokerへ一回要求しています。';
    });
    try {
      var resourceClient = _resourceClient ?? widget.resourceClient;
      if (resourceClient == null) {
        resourceClient = await widget.connectResource();
        if (!_isCurrent(client, generation, runtimeId)) return;
        _resourceClient = resourceClient;
      }
      final observation = await resourceClient.observe(runtimeId);
      if (!_isCurrent(client, generation, runtimeId)) return;
      setState(() {
        _observation = observation;
        _message = '一回観測を受信しました。';
      });
    } catch (_) {
      if (_isCurrent(client, generation, runtimeId)) {
        setState(() {
          _observation = null;
          _message = '資源観測の応答または監査結合を確認できません。状態は不明です。';
        });
      }
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _observing = false);
      }
    }
  }

  bool _isCurrent(
    ShellCoreClient client,
    int generation,
    String runtimeId,
  ) =>
      mounted &&
      widget.active &&
      generation == _generation &&
      identical(client, widget.client) &&
      runtimeId == _selectedRuntimeId &&
      _runtimeIds?.contains(runtimeId) == true;

  @override
  Widget build(BuildContext context) {
    final runtimeIds = _runtimeIds;
    final observation = _observation;
    final busy = _loadingRuntimes || _observing;
    final rows = observation == null
        ? <String>[]
        : <String>[
            'Runtime ID: ${observation.runtimeId}',
            '観測時刻: ${DateTime.fromMillisecondsSinceEpoch(observation.observedAtUnixMillis, isUtc: true).toLocal().toIso8601String()}',
            'PID結合: ${observation.binding.status} / 根拠=${observation.binding.basis}',
            if (observation.binding.pid != null)
              'Broker観測PID: ${observation.binding.pid}',
            '登録監査ID: ${observation.binding.registrationAuditId}',
            '資源観測監査ID: ${observation.auditId}',
            for (final entry in _runtimeResourceMetrics.entries)
              _metricRow(observation, entry.key, entry.value),
          ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionList(
          title: 'Runtime資源の現在観測',
          rows: [
            'Broker登録Runtimeに対する監査付き一回観測です。常時pollingは行わず、取得できない値は0ではなく不明として表示します。',
          ],
        ),
        if (runtimeIds != null && runtimeIds.isNotEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: SizedBox(
              width: 420,
              child: DropdownButtonFormField<String>(
                key: const ValueKey('problems-resource-runtime-selection'),
                initialValue: runtimeIds.contains(_selectedRuntimeId)
                    ? _selectedRuntimeId
                    : runtimeIds.first,
                decoration: const InputDecoration(
                  labelText: 'Broker登録済みRuntime',
                ),
                items: [
                  for (final id in runtimeIds)
                    DropdownMenuItem(value: id, child: Text(id)),
                ],
                onChanged: busy ? null : _selectRuntime,
              ),
            ),
          ),
        Wrap(
          spacing: 12,
          runSpacing: 8,
          children: [
            OutlinedButton.icon(
              key: const ValueKey('problems-resource-runtime-refresh'),
              onPressed: !widget.active || busy
                  ? null
                  : () => unawaited(_refreshRuntimeIds()),
              icon: _loadingRuntimes
                  ? const SizedBox(
                      width: 18,
                      height: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.refresh),
              label: const Text('登録Runtimeを取得'),
            ),
            FilledButton.icon(
              key: const ValueKey('problems-resource-observe'),
              onPressed: runtimeIds == null ||
                      runtimeIds.isEmpty ||
                      _selectedRuntimeId == null ||
                      !widget.active ||
                      busy
                  ? null
                  : () => unawaited(_observeResources()),
              icon: _observing
                  ? const SizedBox(
                      width: 18,
                      height: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.monitor_heart_outlined),
              label: const Text('資源を一回観測'),
            ),
          ],
        ),
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 8),
          child: Text(_message),
        ),
        if (observation != null) SectionList(title: 'Broker観測結果', rows: rows),
      ],
    );
  }

  String _metricRow(
    RuntimeResourceObservation observation,
    String key,
    String label,
  ) {
    final metric = observation.metric(key);
    if (!metric.isMeasured) {
      return '$label: 不明 / 証拠=${metric.evidenceSource} / 理由=${metric.reason ?? '未提示'}';
    }
    return '$label: ${metric.value} / 証拠=${metric.evidenceSource}';
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
