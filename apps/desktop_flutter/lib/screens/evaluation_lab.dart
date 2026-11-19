import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../services/broker_client.dart';
import '../services/evaluation_client.dart';
import '../services/regression_case_client.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

/// C5の通常資格向け評価操作面。
///
/// 非公開Dataset payload、期待値、評価器設定、owner操作はこの画面に渡さない。
class EvaluationLab extends StatefulWidget {
  const EvaluationLab({
    super.key,
    this.client,
    this.connect,
    this.shellCoreClient,
    this.hostContextGeneration,
    this.regressionCaseClient,
    this.connectRegressionCases,
    this.regressionCaseOwnerClient,
    this.connectRegressionCaseOwner,
  });

  final EvaluationClient? client;
  final EvaluationClientConnector? connect;
  final ShellCoreClient? shellCoreClient;
  final ValueListenable<int>? hostContextGeneration;
  final RegressionCaseClient? regressionCaseClient;
  final RegressionCaseClientConnector? connectRegressionCases;
  final RegressionCaseOwnerClient? regressionCaseOwnerClient;
  final RegressionCaseOwnerClientConnector? connectRegressionCaseOwner;

  @override
  State<EvaluationLab> createState() => _EvaluationLabState();
}

class _EvaluationLabState extends State<EvaluationLab>
    with SingleTickerProviderStateMixin {
  late final TabController _tabs;
  final _runDatasetId = TextEditingController();
  final _resultExperimentId = TextEditingController();
  final _comparisonExperimentId = TextEditingController();
  final _recoveryCaseId = TextEditingController();

  EvaluationClient? _client;
  RegressionCaseClient? _regressionClient;
  RegressionCaseOwnerClient? _regressionOwnerClient;
  EvaluationDatasetListing? _datasets;
  EvaluationDataset? _selectedRunDataset;
  List<RegressionCaseSummary> _regressionCases = const [];
  int? _regressionNextCursor;
  int _regressionTotalCount = 0;
  String? _regressionAuditId;
  List<String>? _runRuntimeCatalog;
  final Set<String> _selectedRunRuntimeIds = <String>{};
  EvaluationExperiment? _startedExperiment;
  EvaluationExperimentStatus? _status;
  EvaluationComparison? _comparison;
  bool _busy = false;
  int _generation = 0;
  int _runtimeCatalogGeneration = 0;
  int _lastRuntimeCatalogTabIndex = 0;
  bool _runtimeCatalogBusy = false;
  String _datasetMessage = 'Dataset一覧は自動更新しません。更新要求を作成してください。';
  String _regressionMessage = '回帰Caseの公開metadataだけを手動更新します。';
  String _regressionOwnerActionMessage =
      'Owner操作はWindowsのnative確認後にBrokerへ送られます。';
  String _runMessage = 'Dataset IDとBroker登録済みRuntimeを選択してください。';
  String _runtimeCatalogMessage = 'Broker登録済みRuntime一覧は未取得です。';
  String _resultMessage = '評価Experiment IDを指定して、公開済みの結果projectionだけを更新します。';
  String _comparisonMessage = '評価Experiment IDを指定して、集計比較だけを更新します。';

  @override
  void initState() {
    super.initState();
    _client = widget.client;
    _regressionClient = widget.regressionCaseClient;
    _regressionOwnerClient = widget.regressionCaseOwnerClient;
    _tabs = TabController(length: 5, vsync: this);
    _tabs.addListener(_handleTabSelection);
    widget.hostContextGeneration?.addListener(_hostContextChanged);
  }

  @override
  void didUpdateWidget(covariant EvaluationLab oldWidget) {
    super.didUpdateWidget(oldWidget);
    final hostScopeChanged =
        oldWidget.hostContextGeneration != widget.hostContextGeneration;
    if (hostScopeChanged) {
      oldWidget.hostContextGeneration?.removeListener(_hostContextChanged);
      widget.hostContextGeneration?.addListener(_hostContextChanged);
    }
    if (hostScopeChanged ||
        oldWidget.client != widget.client ||
        oldWidget.connect != widget.connect ||
        oldWidget.shellCoreClient != widget.shellCoreClient ||
        oldWidget.regressionCaseClient != widget.regressionCaseClient ||
        oldWidget.connectRegressionCases != widget.connectRegressionCases ||
        oldWidget.regressionCaseOwnerClient !=
            widget.regressionCaseOwnerClient ||
        oldWidget.connectRegressionCaseOwner !=
            widget.connectRegressionCaseOwner) {
      _resetForContextChange();
    }
  }

  void _hostContextChanged() => setState(_resetForContextChange);

  void _resetForContextChange() {
    _generation++;
    _client = widget.client;
    _regressionClient = widget.regressionCaseClient;
    _regressionOwnerClient = widget.regressionCaseOwnerClient;
    _runDatasetId.clear();
    _resultExperimentId.clear();
    _comparisonExperimentId.clear();
    _recoveryCaseId.clear();
    _datasets = null;
    _selectedRunDataset = null;
    _regressionCases = const [];
    _regressionNextCursor = null;
    _regressionTotalCount = 0;
    _regressionAuditId = null;
    _runtimeCatalogGeneration++;
    _runRuntimeCatalog = null;
    _selectedRunRuntimeIds.clear();
    _runtimeCatalogBusy = false;
    _runtimeCatalogMessage = '接続条件が変わりました。Runtime一覧を再取得してください。';
    _startedExperiment = null;
    _status = null;
    _comparison = null;
    _busy = false;
    _datasetMessage = '接続条件が変わりました。Dataset一覧を更新してください。';
    _regressionMessage = '接続条件が変わりました。回帰Case一覧を更新してください。';
    _regressionOwnerActionMessage = '接続条件が変わりました。';
    _runMessage = '接続条件が変わりました。実験要求を作成し直してください。';
    _resultMessage = '接続条件が変わりました。結果を更新してください。';
    _comparisonMessage = '接続条件が変わりました。比較を更新してください。';
  }

  @override
  void dispose() {
    _generation += 1;
    _runtimeCatalogGeneration++;
    widget.hostContextGeneration?.removeListener(_hostContextChanged);
    _tabs.removeListener(_handleTabSelection);
    _tabs.dispose();
    _runDatasetId.dispose();
    _resultExperimentId.dispose();
    _comparisonExperimentId.dispose();
    _recoveryCaseId.dispose();
    super.dispose();
  }

  void _handleTabSelection() {
    if (_tabs.indexIsChanging || _tabs.index == _lastRuntimeCatalogTabIndex) {
      return;
    }
    _lastRuntimeCatalogTabIndex = _tabs.index;
    if (_tabs.index == 1) {
      unawaited(_refreshRuntimeCatalog());
    }
  }

  @override
  Widget build(BuildContext context) {
    return ShellPage(
      title: '評価ラボ',
      children: [
        const BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('運用観測の境界'),
              SizedBox(height: 8),
              Text(
                '評価ラボは運用観測であり、権限、Permission、Approval、監査、release判定を生成または変更しません。Brokerが既に記録した公開監査IDを表示するだけです。',
              ),
              SizedBox(height: 6),
              Text(
                '各CaseのRuntime送信には既存のowner対話承認が個別に必要です。実験の実行ボタンは要求を作成するだけで、owner承認を代理しません。',
              ),
              SizedBox(height: 6),
              Text('この画面は本文、期待値、正規表現、JSON Schema、応答内容を表示しません。'),
            ],
          ),
        ),
        Material(
          color: Colors.transparent,
          child: TabBar(
            controller: _tabs,
            tabs: const [
              Tab(text: 'Dataset'),
              Tab(text: 'Run'),
              Tab(text: 'Result'),
              Tab(text: 'Compare'),
              Tab(text: '回帰Case'),
            ],
          ),
        ),
        AnimatedBuilder(
          animation: _tabs,
          builder: (context, _) {
            switch (_tabs.index) {
              case 0:
                return _datasetPanel();
              case 1:
                return _runPanel();
              case 2:
                return _resultPanel();
              case 3:
                return _comparisonPanel();
              default:
                return _regressionCasePanel();
            }
          },
        ),
      ],
    );
  }

  Widget _datasetPanel() {
    final listing = _datasets;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('評価Dataset', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              const Text('公開表示名、ID、revision、定義hash、Case数、Case IDだけを一覧表示します。'),
              const SizedBox(height: 12),
              FilledButton.icon(
                key: const ValueKey('evaluation-dataset-refresh'),
                onPressed: _busy ? null : _refreshDatasets,
                icon: const Icon(Icons.refresh),
                label: const Text('Dataset一覧を更新'),
              ),
              const SizedBox(height: 8),
              Text(_datasetMessage),
              if (_busy) ...[
                const SizedBox(height: 8),
                const LinearProgressIndicator(),
              ],
            ],
          ),
        ),
        if (listing != null) ...[
          const SizedBox(height: 16),
          Text('一覧監査ID: ${listing.auditId}'),
          const SizedBox(height: 8),
          if (listing.datasets.isEmpty)
            const BorderedPanel(child: Text('公開可能な評価Datasetはありません。'))
          else
            for (final dataset in listing.datasets) ...[
              _DatasetSummaryPanel(
                dataset: dataset,
                enabled: !_busy,
                onUseForRun: () => _useDatasetForRun(dataset),
              ),
              const SizedBox(height: 12),
            ],
        ],
      ],
    );
  }

  Widget _regressionCasePanel() {
    final nextCursor = _regressionNextCursor;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('回帰Case一覧', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              const Text(
                'C6の公開metadataだけを通常Broker経路から取得します。入力本文・条件・参照は表示せず、C5 Datasetへ自動追加しません。',
              ),
              const SizedBox(height: 6),
              const Text(
                  '削除と中断照合はWindowsのnative確認を通り、Owner資格はFlutterへ渡しません。登録、private内容の閲覧、実行、C5へのimportはこの画面から行いません。'),
              const SizedBox(height: 12),
              Wrap(
                spacing: 8,
                children: [
                  FilledButton.icon(
                    key: const ValueKey('regression-case-refresh'),
                    onPressed: _busy ? null : () => _refreshRegressionCases(),
                    icon: const Icon(Icons.refresh),
                    label: const Text('一覧を更新'),
                  ),
                  OutlinedButton.icon(
                    key: const ValueKey('regression-case-next-page'),
                    onPressed: _busy || nextCursor == null
                        ? null
                        : () => _refreshRegressionCases(
                              after: nextCursor,
                              append: true,
                            ),
                    icon: const Icon(Icons.navigate_next),
                    label: const Text('次の一覧'),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              Text(_regressionMessage),
              const SizedBox(height: 4),
              Text(_regressionOwnerActionMessage),
              if (_busy) ...[
                const SizedBox(height: 8),
                const LinearProgressIndicator(),
              ],
            ],
          ),
        ),
        if (_regressionAuditId case final auditId?) ...[
          const SizedBox(height: 12),
          Text('一覧監査ID: $auditId'),
          Text('表示 ${_regressionCases.length} / $_regressionTotalCount 件'),
        ],
        if (_regressionCases.isEmpty && _regressionAuditId != null)
          const BorderedPanel(child: Text('公開可能な回帰Caseはありません。'))
        else
          for (final item in _regressionCases) ...[
            const SizedBox(height: 10),
            _RegressionCaseSummaryPanel(
              item: item,
              enabled: !_busy,
              onDelete: () => _deleteRegressionCase(item),
            ),
          ],
        const SizedBox(height: 12),
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('削除中断の照合', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 6),
              const Text('Case IDを指定して、Brokerが保管状態を再観測します。削除は実行せず、自動再試行もしません。'),
              const SizedBox(height: 10),
              SizedBox(
                width: 440,
                child: TextField(
                  key: const ValueKey('regression-case-recovery-id'),
                  controller: _recoveryCaseId,
                  enabled: !_busy,
                  autocorrect: false,
                  enableSuggestions: false,
                  maxLength: 32,
                  decoration: const InputDecoration(
                    labelText: '回帰Case ID',
                    helperText: '32桁の小文字hex。監査承認IDは入力しません。',
                  ),
                ),
              ),
              OutlinedButton.icon(
                key: const ValueKey('regression-case-recovery-submit'),
                onPressed: _busy ? null : _recoverRegressionCase,
                icon: const Icon(Icons.fact_check_outlined),
                label: const Text('中断状態を照合'),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _runPanel() {
    final experiment = _startedExperiment;
    final selectedDataset = _selectedRunDataset;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('評価実験を要求', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              const Text(
                '評価Dataset IDを指定し、Broker登録済みRuntimeを1〜8件選択します。要求ごとに現在のBrokerが再検証します。',
              ),
              const SizedBox(height: 12),
              SizedBox(
                width: 440,
                child: TextField(
                  key: const ValueKey('evaluation-run-dataset-id'),
                  controller: _runDatasetId,
                  enabled: !_busy,
                  readOnly: selectedDataset != null,
                  autocorrect: false,
                  enableSuggestions: false,
                  maxLength: 32,
                  onChanged: (_) => setState(() {}),
                  decoration: InputDecoration(
                    labelText: '評価Dataset ID',
                    helperText: selectedDataset == null
                        ? 'IDを直接入力すると、開始時点の現行版を使用します。'
                        : '一覧で選んだ版を使用します。',
                  ),
                ),
              ),
              if (selectedDataset != null) ...[
                SelectableText(
                  '使用する版: ${selectedDataset.revision}\n'
                  '定義hash: ${selectedDataset.definitionHash}',
                ),
                TextButton(
                  onPressed: _busy
                      ? null
                      : () => setState(() {
                            _runDatasetId.clear();
                            _selectedRunDataset = null;
                          }),
                  child: const Text('Dataset選択を解除'),
                ),
              ],
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  const Text('Broker登録済みRuntime（最大8件）'),
                  OutlinedButton.icon(
                    key: const ValueKey('evaluation-run-runtime-refresh'),
                    onPressed: _busy || _runtimeCatalogBusy
                        ? null
                        : () => unawaited(_refreshRuntimeCatalog()),
                    icon: _runtimeCatalogBusy
                        ? const SizedBox(
                            width: 16,
                            height: 16,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.refresh),
                    label: Text(
                      _runtimeCatalogBusy ? '一覧取得中…' : 'Runtime一覧を更新',
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 6),
              Text(_runtimeCatalogMessage),
              if (_runRuntimeCatalog case final runtimeIds?) ...[
                if (runtimeIds.isEmpty)
                  const Padding(
                    padding: EdgeInsets.symmetric(vertical: 8),
                    child: Text('Brokerに登録済みRuntimeはありません。'),
                  )
                else ...[
                  const SizedBox(height: 6),
                  Wrap(
                    spacing: 8,
                    runSpacing: 4,
                    children: [
                      for (final runtimeId in runtimeIds)
                        FilterChip(
                          key: ValueKey('evaluation-run-runtime-$runtimeId'),
                          label: Text(runtimeId),
                          selected: _selectedRunRuntimeIds.contains(runtimeId),
                          onSelected: _busy ||
                                  _runtimeCatalogBusy ||
                                  (!_selectedRunRuntimeIds
                                          .contains(runtimeId) &&
                                      _selectedRunRuntimeIds.length >= 8)
                              ? null
                              : (selected) =>
                                  _selectRunRuntime(runtimeId, selected),
                        ),
                    ],
                  ),
                ],
                const SizedBox(height: 4),
                Text('選択中: ${_selectedRunRuntimeIds.length} / 8'),
              ],
              const SizedBox(height: 4),
              FilledButton.icon(
                key: const ValueKey('evaluation-run-submit'),
                onPressed:
                    _busy || !_canStartExperiment ? null : _startExperiment,
                icon: const Icon(Icons.play_arrow),
                label: const Text('評価実験を要求'),
              ),
              const SizedBox(height: 8),
              Text(_runMessage),
              if (_busy) ...[
                const SizedBox(height: 8),
                const LinearProgressIndicator(),
              ],
            ],
          ),
        ),
        if (experiment != null) ...[
          const SizedBox(height: 16),
          _ExperimentSummaryPanel(experiment: experiment),
        ],
      ],
    );
  }

  Widget _resultPanel() {
    final status = _status;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _experimentIdRequestPanel(
          key: const ValueKey('evaluation-result-experiment-id'),
          controller: _resultExperimentId,
          buttonKey: const ValueKey('evaluation-result-refresh'),
          buttonLabel: 'Resultを更新',
          message: _resultMessage,
          onPressed: _busy ? null : _refreshResult,
        ),
        if (status != null) ...[
          const SizedBox(height: 16),
          _ExperimentSummaryPanel(experiment: status.experiment),
          const SizedBox(height: 12),
          Text('結果照会監査ID: ${status.auditId}'),
          const SizedBox(height: 8),
          if (status.results.isEmpty)
            const BorderedPanel(child: Text('公開可能な評価結果はまだありません。'))
          else
            for (final result in status.results) ...[
              _ResultSummaryPanel(result: result),
              const SizedBox(height: 12),
            ],
        ],
      ],
    );
  }

  Widget _comparisonPanel() {
    final comparison = _comparison;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _experimentIdRequestPanel(
          key: const ValueKey('evaluation-compare-experiment-id'),
          controller: _comparisonExperimentId,
          buttonKey: const ValueKey('evaluation-compare-refresh'),
          buttonLabel: 'Compareを更新',
          message: _comparisonMessage,
          onPressed: _busy ? null : _refreshComparison,
        ),
        if (comparison != null) ...[
          const SizedBox(height: 16),
          _ComparisonSummaryPanel(comparison: comparison),
        ],
      ],
    );
  }

  Widget _experimentIdRequestPanel({
    required Key key,
    required TextEditingController controller,
    required Key buttonKey,
    required String buttonLabel,
    required String message,
    required VoidCallback? onPressed,
  }) {
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(buttonLabel, style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          const Text('評価Experiment IDだけを指定し、本文や個別の対話追跡値は取得しません。'),
          const SizedBox(height: 12),
          SizedBox(
            width: 440,
            child: TextField(
              key: key,
              controller: controller,
              enabled: !_busy,
              autocorrect: false,
              enableSuggestions: false,
              maxLength: 32,
              decoration: const InputDecoration(
                labelText: '評価Experiment ID',
                helperText: '32桁の小文字hex IDだけを指定します。',
              ),
            ),
          ),
          const SizedBox(height: 4),
          FilledButton.icon(
            key: buttonKey,
            onPressed: onPressed,
            icon: const Icon(Icons.refresh),
            label: Text(buttonLabel),
          ),
          const SizedBox(height: 8),
          Text(message),
          if (_busy) ...[
            const SizedBox(height: 8),
            const LinearProgressIndicator(),
          ],
        ],
      ),
    );
  }

  bool _isCurrent(int generation) => mounted && generation == _generation;

  Future<T> _resolveClient<T>({
    required T? cached,
    required Future<T> Function()? connect,
    required Future<T> Function() connectStandalone,
    required T Function(BrokerTransport) bind,
    required void Function(T) cache,
  }) async {
    if (cached != null) return cached;
    final generation = _generation;
    final shell = widget.shellCoreClient;
    final T client;
    if (connect != null || shell == null) {
      client = await (connect ?? connectStandalone)();
    } else {
      final transport = shell.brokerTransport;
      if (transport == null) {
        throw const BrokerClientException('評価ラボの現在Brokerに接続できません。');
      }
      client = bind(transport);
    }
    if (_isCurrent(generation)) cache(client);
    return client;
  }

  Future<EvaluationClient> _clientForRequest() => _resolveClient(
        cached: _client,
        connect: widget.connect,
        connectStandalone: connectEvaluationClient,
        bind: EvaluationClient.new,
        cache: (value) => _client = value,
      );

  Future<RegressionCaseClient> _regressionClientForRequest() async =>
      _resolveClient(
        cached: _regressionClient,
        connect: widget.connectRegressionCases,
        connectStandalone: connectRegressionCaseClient,
        bind: RegressionCaseClient.new,
        cache: (value) => _regressionClient = value,
      );

  Future<RegressionCaseOwnerClient> _regressionOwnerClientForRequest() =>
      _resolveClient(
        cached: _regressionOwnerClient,
        connect: widget.connectRegressionCaseOwner,
        connectStandalone: connectRegressionCaseOwnerClient,
        bind: RegressionCaseOwnerClient.new,
        cache: (value) => _regressionOwnerClient = value,
      );

  Future<void> _deleteRegressionCase(RegressionCaseSummary item) async {
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _regressionOwnerActionMessage =
          'Windows native確認を待っています。拒否・期限切れなら操作は成立しません。';
    });
    try {
      final client = await _regressionOwnerClientForRequest();
      if (!_isCurrent(generation)) return;
      final receipt = await client.delete(
        caseId: item.caseId,
        definitionHash: item.definitionHash,
        ciphertextHash: item.ciphertextHash,
      );
      if (!mounted || generation != _generation) return;
      setState(() {
        _regressionOwnerActionMessage =
            'Case ${receipt.caseId} の暗号文不在と結果Auditを確認しました。削除監査ID: ${receipt.auditId}。物理消去は主張しません。';
      });
      await _refreshRegressionCases();
    } catch (_) {
      if (mounted && generation == _generation) {
        setState(() {
          _regressionOwnerActionMessage =
              '削除は確定していません。Owner確認、Audit、ProtectedStore状態を再確認してください。';
        });
      }
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _recoverRegressionCase() async {
    final caseId = _recoveryCaseId.text.trim();
    if (!RegExp(r'^[a-f0-9]{32}$').hasMatch(caseId)) {
      setState(() {
        _regressionOwnerActionMessage = '回帰Case IDは32桁の小文字hexで指定してください。';
      });
      return;
    }
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _regressionOwnerActionMessage = 'Windows native確認後に、指定Caseの現在状態だけを照合します。';
    });
    try {
      final client = await _regressionOwnerClientForRequest();
      if (!_isCurrent(generation)) return;
      final receipt = await client.recover(caseId: caseId);
      if (!mounted || generation != _generation) return;
      setState(() {
        _regressionOwnerActionMessage =
            'Case ${receipt.caseId}: ${receipt.state}。観測Audit: ${receipt.auditId}。再削除はしていません。';
      });
      await _refreshRegressionCases();
    } catch (_) {
      if (mounted && generation == _generation) {
        setState(() {
          _regressionOwnerActionMessage =
              '照合できませんでした。未確定削除、Audit chain、ProtectedStore状態を確認してください。';
        });
      }
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _refreshRegressionCases(
      {int after = 0, bool append = false}) async {
    final generation = ++_generation;
    setState(() {
      _busy = true;
      if (!append) {
        _regressionCases = const [];
        _regressionNextCursor = null;
        _regressionTotalCount = 0;
        _regressionAuditId = null;
      }
      _regressionMessage = 'Brokerへ回帰Case metadata一覧を要求しています。';
    });
    try {
      final client = await _regressionClientForRequest();
      if (!_isCurrent(generation)) return;
      final page = await client.list(
        after: after,
        limit: 50,
      );
      if (!mounted || generation != _generation) return;
      final combined =
          append ? [..._regressionCases, ...page.cases] : page.cases;
      if (combined.map((item) => item.caseId).toSet().length !=
          combined.length) {
        throw StateError('回帰Case cursor pageに重複があります');
      }
      setState(() {
        _regressionCases = List.unmodifiable(combined);
        _regressionNextCursor = page.nextCursor;
        _regressionTotalCount = page.totalCount;
        _regressionAuditId = page.auditId;
        _regressionMessage =
            'metadataのみを監査ID ${page.auditId} で確認しました。権限・承認・Case内容は取得していません。';
      });
    } catch (_) {
      if (mounted && generation == _generation) {
        setState(() {
          _regressionMessage = '回帰Caseの監査、暗号文照合、cursor、または安全な応答を確認できません。';
        });
      }
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _refreshDatasets() async {
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _datasets = null;
      _datasetMessage = 'BrokerへDataset一覧を要求しています。';
    });
    try {
      final client = await _clientForRequest();
      if (!_isCurrent(generation)) return;
      final listing = await client.listDatasets();
      if (!mounted || generation != _generation) return;
      setState(() {
        _datasets = listing;
        _datasetMessage = '公開Dataset一覧を監査ID ${listing.auditId} で確認しました。';
      });
    } catch (_) {
      if (mounted && generation == _generation) {
        setState(() => _datasetMessage = 'Dataset一覧の監査記録、結合状態、または応答を確認できません。');
      }
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _refreshRuntimeCatalog() async {
    if (_busy || _runtimeCatalogBusy) return;
    final coreClient = widget.shellCoreClient;
    if (coreClient == null) {
      setState(() {
        _runRuntimeCatalog = null;
        _selectedRunRuntimeIds.clear();
        _startedExperiment = null;
        _runtimeCatalogMessage = 'Broker Runtime一覧を取得できません。評価要求を停止しています。';
        _runMessage = 'Runtime一覧を確認できないため評価要求を送信できません。';
      });
      return;
    }

    final generation = ++_generation;
    final catalogGeneration = ++_runtimeCatalogGeneration;
    setState(() {
      _busy = true;
      _runtimeCatalogBusy = true;
      _runtimeCatalogMessage = 'Brokerから登録済みRuntime一覧を取得しています。';
    });
    try {
      final runtimeIds = await coreClient.refreshRuntimeCatalog();
      if (!mounted ||
          generation != _generation ||
          catalogGeneration != _runtimeCatalogGeneration) {
        return;
      }
      final previousSelection = Set<String>.from(_selectedRunRuntimeIds);
      final retainedSelection =
          previousSelection.where(runtimeIds.contains).take(8).toSet();
      final selectionChanged =
          retainedSelection.length != previousSelection.length;
      setState(() {
        _runRuntimeCatalog = runtimeIds;
        _selectedRunRuntimeIds
          ..clear()
          ..addAll(retainedSelection);
        _runtimeCatalogMessage = runtimeIds.isEmpty
            ? 'Brokerに登録済みRuntimeはありません。'
            : 'Broker登録済みRuntime ${runtimeIds.length}件を監査付きで確認しました。';
        if (selectionChanged) {
          _startedExperiment = null;
          _runMessage = 'Runtime一覧が変わりました。残っている選択を確認してください。';
        }
      });
    } catch (_) {
      if (!mounted ||
          generation != _generation ||
          catalogGeneration != _runtimeCatalogGeneration) {
        return;
      }
      setState(() {
        _runRuntimeCatalog = null;
        _selectedRunRuntimeIds.clear();
        _startedExperiment = null;
        _runtimeCatalogMessage = 'Broker Runtime一覧を確認できません。権限状態を推測せず停止しています。';
        _runMessage = 'Runtime一覧を確認できないため評価要求を停止しています。';
      });
    } finally {
      if (mounted &&
          generation == _generation &&
          catalogGeneration == _runtimeCatalogGeneration) {
        setState(() {
          _runtimeCatalogBusy = false;
          _busy = false;
        });
      }
    }
  }

  void _selectRunRuntime(String runtimeId, bool selected) {
    final runtimeCatalog = _runRuntimeCatalog;
    if (_busy ||
        _runtimeCatalogBusy ||
        runtimeCatalog == null ||
        !runtimeCatalog.contains(runtimeId)) {
      return;
    }
    setState(() {
      if (selected) {
        if (_selectedRunRuntimeIds.length >= 8) return;
        _selectedRunRuntimeIds.add(runtimeId);
      } else {
        _selectedRunRuntimeIds.remove(runtimeId);
      }
      _startedExperiment = null;
      _runMessage = '選択したRuntimeで評価実験を要求できます。Brokerが現在条件を再検証します。';
    });
  }

  bool get _canStartExperiment {
    final runtimeCatalog = _runRuntimeCatalog;
    if (!EvaluationClient.validDatasetId(_runDatasetId.text.trim()) ||
        runtimeCatalog == null ||
        _runtimeCatalogBusy) {
      return false;
    }
    final selected = runtimeCatalog
        .where(_selectedRunRuntimeIds.contains)
        .toList(growable: false);
    return selected.length == _selectedRunRuntimeIds.length &&
        EvaluationClient.validRuntimeIds(selected);
  }

  Future<void> _startExperiment() async {
    final datasetId = _runDatasetId.text.trim();
    final selectedDataset = _selectedRunDataset;
    final runtimeCatalog = _runRuntimeCatalog;
    final runtimeIds = runtimeCatalog == null
        ? <String>[]
        : runtimeCatalog
            .where(_selectedRunRuntimeIds.contains)
            .toList(growable: false);
    if (!EvaluationClient.validDatasetId(datasetId) ||
        runtimeCatalog == null ||
        runtimeIds.length != _selectedRunRuntimeIds.length ||
        !EvaluationClient.validRuntimeIds(runtimeIds)) {
      setState(() {
        _startedExperiment = null;
        _runMessage = 'Dataset IDとBroker登録済みRuntimeを1〜8件選択してください。';
      });
      return;
    }
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _startedExperiment = null;
      _runMessage = 'Brokerへ評価実験の要求を作成しています。owner承認は代理しません。';
    });
    try {
      final client = await _clientForRequest();
      if (!_isCurrent(generation)) return;
      final experiment = await client.startExperiment(
        datasetId,
        runtimeIds,
        expectedDatasetDefinitionHash: selectedDataset?.definitionHash,
      );
      if (!mounted || generation != _generation) return;
      setState(() {
        _startedExperiment = experiment;
        _resultExperimentId.text = experiment.experimentId;
        _comparisonExperimentId.text = experiment.experimentId;
        _runMessage =
            '評価Experiment ${experiment.experimentId} の要求を作成しました。各Case/Runtimeの送信には既存owner対話承認が個別に必要です。';
      });
    } on EvaluationDatasetRevisionChanged catch (error) {
      if (mounted && generation == _generation) {
        setState(() {
          _runDatasetId.clear();
          _selectedRunDataset = null;
          _datasets = null;
          _datasetMessage = 'Dataset一覧を更新し、使用する版を選び直してください。';
          _runMessage = '選択後にDatasetの版が変わりました。'
              '一覧を更新して選び直してください。拒否監査ID: ${error.auditId}';
        });
      }
    } catch (_) {
      if (mounted && generation == _generation) {
        setState(() => _runMessage = '評価実験の監査記録、結合状態、または応答を確認できません。');
      }
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _refreshResult() async {
    final experimentId = _resultExperimentId.text.trim();
    if (!EvaluationClient.validExperimentId(experimentId)) {
      setState(() {
        _status = null;
        _resultMessage = '評価Experiment IDは32桁の小文字hexで指定してください。';
      });
      return;
    }
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _status = null;
      _resultMessage = 'Brokerへ評価実験状態を要求しています。';
    });
    try {
      final client = await _clientForRequest();
      if (!_isCurrent(generation)) return;
      final status = await client.experimentStatus(experimentId);
      if (!mounted || generation != _generation) return;
      setState(() {
        _status = status;
        _resultMessage = '公開結果projectionを監査ID ${status.auditId} で確認しました。';
      });
    } catch (_) {
      if (mounted && generation == _generation) {
        setState(() => _resultMessage = '評価実験状態の監査記録、結合状態、または安全な応答を確認できません。');
      }
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _refreshComparison() async {
    final experimentId = _comparisonExperimentId.text.trim();
    if (!EvaluationClient.validExperimentId(experimentId)) {
      setState(() {
        _comparison = null;
        _comparisonMessage = '評価Experiment IDは32桁の小文字hexで指定してください。';
      });
      return;
    }
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _comparison = null;
      _comparisonMessage = 'Brokerへ評価比較を要求しています。';
    });
    try {
      final client = await _clientForRequest();
      if (!_isCurrent(generation)) return;
      final comparison = await client.compare(experimentId);
      if (!mounted || generation != _generation) return;
      setState(() {
        _comparison = comparison;
        _comparisonMessage = '集計比較を監査ID ${comparison.auditId} で確認しました。';
      });
    } catch (_) {
      if (mounted && generation == _generation) {
        setState(() => _comparisonMessage = '評価比較の監査記録、結合状態、または安全な応答を確認できません。');
      }
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _busy = false);
      }
    }
  }

  void _useDatasetForRun(EvaluationDataset dataset) {
    setState(() {
      _selectedRunDataset = dataset;
      _runDatasetId.text = dataset.datasetId;
      _tabs.animateTo(1);
    });
  }
}

class _RegressionCaseSummaryPanel extends StatelessWidget {
  const _RegressionCaseSummaryPanel({
    required this.item,
    required this.enabled,
    required this.onDelete,
  });

  final RegressionCaseSummary item;
  final bool enabled;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) => BorderedPanel(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(item.displayName,
                style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 6),
            SelectableText('回帰事例ID: ${item.caseId}'),
            Text('実行系: ${item.runtimeId}　結果: ${item.resultStatus}'),
            Text(
              '必要条件 ${item.requiredConditionCount}　禁止条件 ${item.forbiddenConditionCount}　必要参照 ${item.requiredReferenceCount}',
            ),
            Text('作成監査ID: ${item.createdAuditId}　終了監査ID: ${item.endAuditId}'),
            const Text('公開範囲: metadata_only　証拠種別: INTERNAL_STATE'),
            const SizedBox(height: 8),
            OutlinedButton.icon(
              key: ValueKey('regression-case-delete-${item.caseId}'),
              onPressed: enabled ? onDelete : null,
              icon: const Icon(Icons.delete_outline),
              label: const Text('このCaseの保存暗号文を削除'),
            ),
          ],
        ),
      );
}

class _DatasetSummaryPanel extends StatelessWidget {
  const _DatasetSummaryPanel({
    required this.dataset,
    required this.enabled,
    required this.onUseForRun,
  });

  final EvaluationDataset dataset;
  final bool enabled;
  final VoidCallback onUseForRun;

  @override
  Widget build(BuildContext context) {
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            dataset.displayName,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          SelectableText(
            '評価Dataset ID: ${dataset.datasetId}\n'
            '改訂番号: ${dataset.revision}\n'
            '定義hash: ${dataset.definitionHash}\n'
            'Case数: ${dataset.caseCount}\n'
            '作成時刻UnixMillis: ${dataset.createdAtUnixMillis}\n'
            '作成監査ID: ${dataset.creationAuditId}',
          ),
          const SizedBox(height: 8),
          const Text('Case一覧（IDと定義hashのみ）'),
          for (final item in dataset.cases)
            SelectableText(
              '評価Case ID: ${item.caseId}\n定義hash: ${item.definitionHash}',
            ),
          const SizedBox(height: 8),
          TextButton(
            onPressed: enabled ? onUseForRun : null,
            child: const Text('この版で評価実験を設定'),
          ),
        ],
      ),
    );
  }
}

class _ExperimentSummaryPanel extends StatelessWidget {
  const _ExperimentSummaryPanel({required this.experiment});

  final EvaluationExperiment experiment;

  @override
  Widget build(BuildContext context) {
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('評価Experiment', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          SelectableText(
            '評価Experiment ID: ${experiment.experimentId}\n'
            '評価Dataset ID: ${experiment.datasetId}\n'
            'Dataset定義hash: ${experiment.datasetDefinitionHash}\n'
            '対象Runtime: ${experiment.runtimeIds.join(', ')}\n'
            '状態: ${experiment.state}\n'
            '計画Case数: ${experiment.plannedCaseCount}\n'
            '結果数: ${experiment.resultCount}\n'
            '作成時刻UnixMillis: ${experiment.createdAtUnixMillis}\n'
            '開始時刻UnixMillis: ${experiment.startedAtUnixMillis ?? '未開始'}\n'
            '終了時刻UnixMillis: ${experiment.finishedAtUnixMillis ?? '未終了'}\n'
            '計画監査ID: ${experiment.planAuditId}\n'
            '実験監査ID: ${experiment.auditId}',
          ),
        ],
      ),
    );
  }
}

class _ResultSummaryPanel extends StatelessWidget {
  const _ResultSummaryPanel({required this.result});

  final EvaluationResultSummary result;

  @override
  Widget build(BuildContext context) {
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('公開評価結果', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          SelectableText(
            '評価Case ID: ${result.caseId}\n'
            '実行系ID: ${result.runtimeId}\n'
            '判定: ${result.judgement}\n'
            '遅延Millis: ${result.latencyMillis ?? '未観測'}\n'
            '総合hash: ${result.aggregateHash}\n'
            '評価監査ID: ${result.auditId}',
          ),
          const SizedBox(height: 8),
          const Text('評価器判定（ID、種類、判定、理由codeのみ）'),
          for (final evaluator in result.evaluators)
            SelectableText(
              '評価器ID: ${evaluator.evaluatorId}\n'
              '種類: ${evaluator.kind}\n'
              '判定: ${evaluator.judgement}\n'
              '理由code: ${evaluator.reasonCode}',
            ),
        ],
      ),
    );
  }
}

class _ComparisonSummaryPanel extends StatelessWidget {
  const _ComparisonSummaryPanel({required this.comparison});

  final EvaluationComparison comparison;

  @override
  Widget build(BuildContext context) {
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('評価比較', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          SelectableText(
            '比較ID: ${comparison.comparisonId}\n'
            '評価Dataset ID: ${comparison.datasetId}\n'
            'Dataset定義hash: ${comparison.datasetDefinitionHash}\n'
            '計画Case数: ${comparison.plannedCaseCount}\n'
            '成立数: ${comparison.passedCount}\n'
            '不成立数: ${comparison.failedCount}\n'
            '評価不能数: ${comparison.indeterminateCount}\n'
            '中断数: ${comparison.interruptedCount}\n'
            '比較時刻UnixMillis: ${comparison.comparedAtUnixMillis}\n'
            '経路差: ${comparison.routeDifference}\n'
            '参照差: ${comparison.referenceDifference}\n'
            '能力差: ${comparison.capabilityDifference}\n'
            '比較監査ID: ${comparison.auditId}',
          ),
          const SizedBox(height: 8),
          const Text('実験別の数値集計'),
          for (final experiment in comparison.experiments)
            SelectableText(
              '評価Experiment ID: ${experiment.experimentId}\n'
              '実行系ID: ${experiment.runtimeId}\n'
              '成立数: ${experiment.passedCount}\n'
              '不成立数: ${experiment.failedCount}\n'
              '評価不能数: ${experiment.indeterminateCount}\n'
              '中断数: ${experiment.interruptedCount}\n'
              '平均遅延Millis: ${experiment.averageLatencyMillis ?? '未観測'}',
            ),
        ],
      ),
    );
  }
}
