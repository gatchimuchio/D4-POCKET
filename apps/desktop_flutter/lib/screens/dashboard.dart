import 'dart:async';

import 'package:flutter/material.dart';

import '../services/broker_client.dart' show BrokerTransport;
import '../services/broker_health_client.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class Dashboard extends StatefulWidget {
  const Dashboard({
    super.key,
    required this.client,
    this.active = true,
    this.onOpenProductSetup,
    this.onOpenAgentSetup,
  });

  final ShellCoreClient client;
  final bool active;
  final VoidCallback? onOpenProductSetup;
  final VoidCallback? onOpenAgentSetup;

  @override
  State<Dashboard> createState() => _DashboardState();
}

class _DashboardState extends State<Dashboard> {
  int _generation = 0;
  BrokerHealthObservation? _health;
  BrokerUiStatus? _unavailableStatus;
  bool _loading = false;
  bool _requestInFlight = false;
  bool _refreshQueued = false;

  @override
  void initState() {
    super.initState();
    _startHealthRead();
  }

  @override
  void didUpdateWidget(covariant Dashboard oldWidget) {
    super.didUpdateWidget(oldWidget);
    final clientChanged = !identical(oldWidget.client, widget.client) ||
        !identical(
          oldWidget.client.brokerTransport,
          widget.client.brokerTransport,
        );
    final activeChanged = oldWidget.active != widget.active;
    if (!clientChanged && !activeChanged) return;

    _generation++;
    _refreshQueued = false;
    if (clientChanged) {
      _health = null;
      _unavailableStatus = null;
      _loading = false;
    }
    if (!widget.active) {
      _loading = false;
      return;
    }
    _startHealthRead();
  }

  void _startHealthRead({bool notify = false}) {
    if (!widget.active) return;
    final transport = widget.client.brokerTransport;
    if (transport == null) {
      void showUnavailable() {
        _health = null;
        _unavailableStatus = widget.client.mode == 'broker_unavailable'
            ? BrokerUiStatus.recoverableError
            : BrokerUiStatus.unsupported;
        _loading = false;
      }

      if (notify && mounted) {
        setState(showUnavailable);
      } else {
        showUnavailable();
      }
      return;
    }
    if (_requestInFlight) {
      _refreshQueued = true;
      if (notify && mounted) {
        setState(() => _loading = true);
      } else {
        _loading = true;
      }
      return;
    }

    _refreshQueued = false;
    final generation = _generation;
    final client = widget.client;
    void markLoading() {
      _health = null;
      _unavailableStatus = null;
      _loading = true;
    }

    if (notify && mounted) {
      setState(markLoading);
    } else {
      markLoading();
    }
    _requestInFlight = true;
    unawaited(_readHealth(client, transport, generation));
  }

  Future<void> _readHealth(
    ShellCoreClient client,
    BrokerTransport transport,
    int generation,
  ) async {
    BrokerHealthObservation? observation;
    const unavailableStatus = BrokerUiStatus.recoverableError;
    try {
      observation = await BrokerHealthClient.read(transport);
    } on Object {
      // Error details can contain Broker or transport data; expose only state.
    }

    _requestInFlight = false;
    if (!mounted) return;
    final isCurrent = widget.active &&
        generation == _generation &&
        identical(widget.client, client) &&
        identical(widget.client.brokerTransport, transport);
    if (isCurrent) {
      setState(() {
        _health = observation;
        _unavailableStatus = observation == null ? unavailableStatus : null;
        _loading = false;
      });
    }

    final refreshQueued = _refreshQueued;
    _refreshQueued = false;
    if (refreshQueued && widget.active) _startHealthRead();
  }

  void _retryHealth() => _startHealthRead(notify: true);

  String get _displayedStatus => _loading
      ? '確認中'
      : _health?.uiStatus.label ?? _unavailableStatus?.label ?? '不明';

  String get _healthMessage {
    final health = _health;
    if (health != null) return health.safeSummary;
    if (_loading) return '現在のBroker状態を確認しています。';
    if (_unavailableStatus == BrokerUiStatus.unsupported) {
      return 'この接続状態ではライブBroker Healthを取得できません。';
    }
    if (_unavailableStatus == BrokerUiStatus.recoverableError) {
      return 'ライブBroker Healthを取得できません。接続後に再取得できます。';
    }
    return '現在のBroker Healthは未取得です。';
  }

  @override
  Widget build(BuildContext context) {
    final snapshot = widget.client.getSnapshot();
    final phase = snapshot.phaseStatus;
    final operation = snapshot.operationStatus;
    final evidence = snapshot.evidenceSummary;
    return ShellPage(
      title: '概要',
      evidenceTitle: 'Dashboard',
      children: [
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'D4 Pocketの導入・更新',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 4),
              const Text(
                'Windowsへの初回導入や更新は、署名済み候補をBrokerが検証する更新センターから行えます。候補取得は明示操作、導入・更新の承認はRustの確認画面で行います。',
              ),
              const SizedBox(height: 8),
              FilledButton.icon(
                key: const ValueKey('open-product-setup'),
                onPressed: widget.onOpenProductSetup,
                icon: const Icon(Icons.install_desktop_outlined),
                label: const Text('インストール・更新センターを開く'),
              ),
            ],
          ),
        ),
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                '次の準備: 実行系と作業領域',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 4),
              const Text(
                '起動後はAgentと作業領域を登録して利用を始めます。登録はRust Brokerのnative Owner確認を通り、PermissionとTask Approvalは別に判断されます。',
              ),
              const SizedBox(height: 8),
              OutlinedButton.icon(
                key: const ValueKey('open-agent-setup'),
                onPressed: widget.onOpenAgentSetup,
                icon: const Icon(Icons.smart_toy_outlined),
                label: const Text('実行系・Agent管理を開く'),
              ),
            ],
          ),
        ),
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
              label: 'Broker状態',
              evidenceLabel: 'Broker Health Status',
              value: _displayedStatus,
            ),
            MetricItem(
              label: '実行系状態（スナップショット）',
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
              value: snapshot.snapshotSource == 'broker' &&
                      operation.pendingApprovalsCountKnown
                  ? '${operation.pendingApprovalsCount}'
                  : '不明',
            ),
            MetricItem(label: '問題', value: '${operation.problemsCount}'),
            MetricItem(label: '証拠', value: evidence.evidenceBundle),
            MetricItem(
              label: '復旧',
              value: '${snapshot.recoveryPlaybook.length}件',
            ),
          ],
        ),
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                _healthMessage,
                key: const ValueKey('dashboard-broker-health-summary'),
              ),
              if (_unavailableStatus == BrokerUiStatus.recoverableError &&
                  !_loading) ...[
                const SizedBox(height: 8),
                OutlinedButton.icon(
                  key: const ValueKey('dashboard-broker-health-retry'),
                  onPressed: _retryHealth,
                  icon: const Icon(Icons.refresh),
                  label: const Text('Broker状態を再取得'),
                ),
              ],
            ],
          ),
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
