import 'package:flutter/material.dart';

import '../services/evaluation_client.dart';
import '../services/regression_case_client.dart';
import 'shared.dart';

/// C5の通常資格向け評価操作面。
///
/// 非公開Dataset payload、期待値、評価器設定、owner操作はこの画面に渡さない。
class EvaluationLab extends StatefulWidget {
  const EvaluationLab({
    super.key,
    this.client,
    this.connect = connectEvaluationClient,
    this.regressionCaseClient,
    this.connectRegressionCases = connectRegressionCaseClient,
    this.regressionCaseOwnerClient,
    this.connectRegressionCaseOwner = connectRegressionCaseOwnerClient,
  });

  final EvaluationClient? client;
  final EvaluationClientConnector connect;
  final RegressionCaseClient? regressionCaseClient;
  final RegressionCaseClientConnector connectRegressionCases;
  final RegressionCaseOwnerClient? regressionCaseOwnerClient;
  final RegressionCaseOwnerClientConnector connectRegressionCaseOwner;

  @override
  State<EvaluationLab> createState() => _EvaluationLabState();
}

class _EvaluationLabState extends State<EvaluationLab>
    with SingleTickerProviderStateMixin {
  late final TabController _tabs;
  final _runDatasetId = TextEditingController();
  final _runRuntimeIds = TextEditingController();
  final _resultExperimentId = TextEditingController();
  final _comparisonExperimentId = TextEditingController();
  final _recoveryCaseId = TextEditingController();

  EvaluationClient? _client;
  RegressionCaseClient? _regressionClient;
  RegressionCaseOwnerClient? _regressionOwnerClient;
  EvaluationDatasetListing? _datasets;
  List<RegressionCaseSummary> _regressionCases = const [];
  int? _regressionNextCursor;
  int _regressionTotalCount = 0;
  String? _regressionAuditId;
  EvaluationExperiment? _startedExperiment;
  EvaluationExperimentStatus? _status;
  EvaluationComparison? _comparison;
  bool _busy = false;
  int _generation = 0;
  String _datasetMessage = 'Dataset一覧は自動更新しません。更新要求を作成してください。';
  String _regressionMessage = '回帰Caseの公開metadataだけを手動更新します。';
  String _regressionOwnerActionMessage =
      'Owner操作はWindowsのnative確認後にBrokerへ送られます。';
  String _runMessage = 'Dataset IDと1〜8件の実行系IDだけをBrokerへ送ります。';
  String _resultMessage = '評価Experiment IDを指定して、公開済みの結果projectionだけを更新します。';
  String _comparisonMessage = '評価Experiment IDを指定して、集計比較だけを更新します。';

  @override
  void initState() {
    super.initState();
    _client = widget.client;
    _regressionClient = widget.regressionCaseClient;
    _regressionOwnerClient = widget.regressionCaseOwnerClient;
    _tabs = TabController(length: 5, vsync: this);
  }

  @override
  void didUpdateWidget(covariant EvaluationLab oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.client != widget.client ||
        oldWidget.connect != widget.connect ||
        oldWidget.regressionCaseClient != widget.regressionCaseClient ||
        oldWidget.connectRegressionCases != widget.connectRegressionCases ||
        oldWidget.regressionCaseOwnerClient !=
            widget.regressionCaseOwnerClient ||
        oldWidget.connectRegressionCaseOwner !=
            widget.connectRegressionCaseOwner) {
      _generation += 1;
      _client = widget.client;
      _regressionClient = widget.regressionCaseClient;
      _regressionOwnerClient = widget.regressionCaseOwnerClient;
      _datasets = null;
      _regressionCases = const [];
      _regressionNextCursor = null;
      _regressionTotalCount = 0;
      _regressionAuditId = null;
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
  }

  @override
  void dispose() {
    _generation += 1;
    _tabs.dispose();
    _runDatasetId.dispose();
    _runRuntimeIds.dispose();
    _resultExperimentId.dispose();
    _comparisonExperimentId.dispose();
    _recoveryCaseId.dispose();
    super.dispose();
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
                onUseForRun: () => _useDatasetForRun(dataset.datasetId),
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
                '入力できるのは評価Dataset IDとcomma-separatedの実行系ID（1〜8件、重複なし）だけです。',
              ),
              const SizedBox(height: 12),
              SizedBox(
                width: 440,
                child: TextField(
                  key: const ValueKey('evaluation-run-dataset-id'),
                  controller: _runDatasetId,
                  enabled: !_busy,
                  autocorrect: false,
                  enableSuggestions: false,
                  maxLength: 32,
                  decoration: const InputDecoration(
                    labelText: '評価Dataset ID',
                    helperText: '32桁の小文字hex IDだけを指定します。',
                  ),
                ),
              ),
              const SizedBox(height: 8),
              SizedBox(
                width: 640,
                child: TextField(
                  key: const ValueKey('evaluation-run-runtime-ids'),
                  controller: _runRuntimeIds,
                  enabled: !_busy,
                  autocorrect: false,
                  enableSuggestions: false,
                  maxLength: 1031,
                  decoration: const InputDecoration(
                    labelText: '実行系ID（comma-separated）',
                    helperText: '例: runtime-a, runtime-b。PID、接続先、本文は入力できません。',
                  ),
                ),
              ),
              const SizedBox(height: 4),
              FilledButton.icon(
                key: const ValueKey('evaluation-run-submit'),
                onPressed: _busy ? null : _startExperiment,
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

  Future<EvaluationClient> _clientForRequest() async {
    return _client ??= widget.client ?? await widget.connect();
  }

  Future<RegressionCaseClient> _regressionClientForRequest() async =>
      _regressionClient ??=
          widget.regressionCaseClient ?? await widget.connectRegressionCases();

  Future<RegressionCaseOwnerClient> _regressionOwnerClientForRequest() async =>
      _regressionOwnerClient ??= widget.regressionCaseOwnerClient ??
          await widget.connectRegressionCaseOwner();

  Future<void> _deleteRegressionCase(RegressionCaseSummary item) async {
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _regressionOwnerActionMessage =
          'Windows native確認を待っています。拒否・期限切れなら操作は成立しません。';
    });
    try {
      final receipt = await (await _regressionOwnerClientForRequest()).delete(
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
      final receipt = await (await _regressionOwnerClientForRequest())
          .recover(caseId: caseId);
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
      final page = await (await _regressionClientForRequest()).list(
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
      final listing = await (await _clientForRequest()).listDatasets();
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

  Future<void> _startExperiment() async {
    final datasetId = _runDatasetId.text.trim();
    final rawRuntimeIds = _runRuntimeIds.text.split(',');
    final runtimeIds = rawRuntimeIds.map((value) => value.trim()).toList();
    if (rawRuntimeIds.any((value) => value.trim().isEmpty) ||
        !EvaluationClient.validDatasetId(datasetId) ||
        !EvaluationClient.validRuntimeIds(runtimeIds)) {
      setState(() {
        _startedExperiment = null;
        _runMessage = '評価Dataset IDと、重複しない1〜8件の実行系IDを指定してください。';
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
      final experiment = await (await _clientForRequest()).startExperiment(
        datasetId,
        runtimeIds,
      );
      if (!mounted || generation != _generation) return;
      setState(() {
        _startedExperiment = experiment;
        _resultExperimentId.text = experiment.experimentId;
        _comparisonExperimentId.text = experiment.experimentId;
        _runMessage =
            '評価Experiment ${experiment.experimentId} の要求を作成しました。各Case/Runtimeの送信には既存owner対話承認が個別に必要です。';
      });
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
      final status = await (await _clientForRequest()).experimentStatus(
        experimentId,
      );
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
      final comparison = await (await _clientForRequest()).compare(
        experimentId,
      );
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

  void _useDatasetForRun(String datasetId) {
    setState(() {
      _runDatasetId.text = datasetId;
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
            child: const Text('このDataset IDでRunを作成'),
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
