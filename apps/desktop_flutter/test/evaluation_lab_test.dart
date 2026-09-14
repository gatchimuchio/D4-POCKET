import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/evaluation_lab.dart';
import 'package:gui_shell_desktop/services/evaluation_client.dart';
import 'package:gui_shell_ui/evaluation_client.dart' show BrokerTransport;

const _datasetId = '11111111111111111111111111111111';
const _caseId = '22222222222222222222222222222222';
const _experimentId = '44444444444444444444444444444444';

void main() {
  Finder selectableTextContaining(String value) => find.byWidgetPredicate(
    (widget) =>
        widget is SelectableText && (widget.data?.contains(value) ?? false),
    description: '指定文言を含むSelectableText: $value',
  );

  testWidgets('評価ラボは四tabを持ちowner資格の入力面を作らない', (tester) async {
    final transport = _FixtureTransport({
      '評価実験開始': [
        _accepted(
          '評価実験開始',
          _experiment(),
          auditId: 'audit.evaluation.experiment.1',
        ),
      ],
    });
    await tester.pumpWidget(_testApp(EvaluationClient(transport)));

    expect(find.text('Dataset'), findsOneWidget);
    expect(find.text('Run'), findsOneWidget);
    expect(find.text('Result'), findsOneWidget);
    expect(find.text('Compare'), findsOneWidget);
    expect(
      find.textContaining('権限、Permission、Approval、監査、release判定'),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('evaluation-owner-credential')),
      findsNothing,
    );
    expect(
      find.byKey(const ValueKey('evaluation-owner-endpoint')),
      findsNothing,
    );

    await tester.tap(find.text('Run'));
    await tester.pumpAndSettle();
    final labels = tester
        .widgetList<TextField>(find.byType(TextField))
        .map((field) => field.decoration?.labelText)
        .toList();
    expect(labels, contains('評価Dataset ID'));
    expect(labels, contains('実行系ID（comma-separated）'));
    expect(labels.join(' '), isNot(contains('owner')));
    expect(labels.join(' '), isNot(contains('資格')));

    await tester.enterText(
      find.byKey(const ValueKey('evaluation-run-dataset-id')),
      _datasetId,
    );
    await tester.enterText(
      find.byKey(const ValueKey('evaluation-run-runtime-ids')),
      'runtime-a, runtime-b',
    );
    final runSubmit = find.byKey(const ValueKey('evaluation-run-submit'));
    await tester.ensureVisible(runSubmit);
    await tester.tap(runSubmit);
    await tester.pumpAndSettle();

    expect(transport.requests, hasLength(1));
    expect(transport.requests.single.operation, '評価実験開始');
    expect(transport.requests.single.payload, {
      '版': 1,
      '評価DatasetID': _datasetId,
      '対象Runtime一覧': const ['runtime-a', 'runtime-b'],
    });
    expect(find.textContaining('owner承認を代理しません'), findsOneWidget);
    expect(
      selectableTextContaining('計画監査ID: audit.evaluation.experiment.plan.1'),
      findsOneWidget,
    );
  });

  testWidgets('公開metadataと安全な集計だけを表示する', (tester) async {
    final transport = _FixtureTransport({
      '評価Dataset一覧': [_accepted('評価Dataset一覧', _listingBody())],
      '評価実験状態': [_accepted('評価実験状態', _statusBody())],
      '評価比較': [
        _accepted(
          '評価比較',
          _comparison(),
          auditId: 'audit.evaluation.comparison.1',
        ),
      ],
    });
    await tester.pumpWidget(_testApp(EvaluationClient(transport)));

    await tester.tap(find.byKey(const ValueKey('evaluation-dataset-refresh')));
    await tester.pumpAndSettle();
    expect(find.text('基本対話評価'), findsOneWidget);
    expect(selectableTextContaining(_datasetId), findsOneWidget);
    expect(
      selectableTextContaining('99999999999999999999999999999999'),
      findsNothing,
    );
    expect(selectableTextContaining(_hash('b')), findsNothing);

    final resultTab = find.text('Result');
    await tester.ensureVisible(resultTab);
    await tester.tap(resultTab);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('evaluation-result-experiment-id')),
      _experimentId,
    );
    final resultRefresh = find.byKey(
      const ValueKey('evaluation-result-refresh'),
    );
    await tester.ensureVisible(resultRefresh);
    await tester.tap(resultRefresh);
    await tester.pumpAndSettle();
    expect(find.text('公開評価結果'), findsOneWidget);
    expect(selectableTextContaining('総合hash:'), findsOneWidget);
    expect(selectableTextContaining('評価監査ID:'), findsOneWidget);
    expect(find.text('private result body'), findsNothing);
    expect(find.text('66666666666666666666666666666666'), findsNothing);

    final comparisonTab = find.text('Compare');
    await tester.ensureVisible(comparisonTab);
    await tester.tap(comparisonTab);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('evaluation-compare-experiment-id')),
      _experimentId,
    );
    final comparisonRefresh = find.byKey(
      const ValueKey('evaluation-compare-refresh'),
    );
    await tester.ensureVisible(comparisonRefresh);
    await tester.tap(comparisonRefresh);
    await tester.pumpAndSettle();
    expect(find.text('評価比較'), findsOneWidget);
    expect(find.text('実験別の数値集計'), findsOneWidget);
    expect(find.text('private result body'), findsNothing);
    expect(transport.requests.map((request) => request.operation), const [
      '評価Dataset一覧',
      '評価実験状態',
      '評価比較',
    ]);
  });
}

Widget _testApp(EvaluationClient client) => MaterialApp(
  home: Scaffold(body: EvaluationLab(client: client)),
);

class _FixtureTransport implements BrokerTransport {
  _FixtureTransport(this.responses);

  final Map<String, List<Map<String, Object?>>> responses;
  final List<_Request> requests = [];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    requests.add(_Request(operation, Map<String, Object?>.from(payload ?? {})));
    final queue = responses[operation];
    if (queue == null || queue.isEmpty) {
      throw StateError('$operation のfixtureがありません');
    }
    return queue.removeAt(0);
  }
}

class _Request {
  const _Request(this.operation, this.payload);

  final String operation;
  final Map<String, Object?> payload;
}

Map<String, Object?> _accepted(
  String operation,
  Map<String, Object?> body, {
  String auditId = 'audit.evaluation.fixture',
}) {
  return {
    'request_id': 'request.$operation',
    'operation': operation,
    'status': 'accepted',
    'evidence_source': 'INTERNAL_STATE',
    'audit_event_id': auditId,
    'error': null,
    'health': null,
    'body': body,
    'shutdown_requested': false,
  };
}

Map<String, Object?> _listingBody() => {
  '版': 1,
  '評価Dataset一覧': [_dataset()],
};

Map<String, Object?> _dataset() => {
  '版': 1,
  '評価DatasetID': _datasetId,
  'revision': 1,
  '定義hash': _hash('0'),
  '非公開保管ID': '99999999999999999999999999999999',
  '暗号文hash': _hash('b'),
  '公開表示名': '基本対話評価',
  'Case数': 1,
  'Case一覧': [
    {'評価CaseID': _caseId, '定義hash': _hash('d')},
  ],
  '公開範囲': 'hash_only',
  '作成時刻UnixMillis': 1726300000000,
  '作成監査ID': 'audit.evaluation.dataset.create.1',
  '証拠種別': 'INTERNAL_STATE',
};

Map<String, Object?> _experiment({int resultCount = 0}) {
  final completed = resultCount == 2;
  return {
    '版': 1,
    '評価ExperimentID': _experimentId,
    '評価DatasetID': _datasetId,
    'Dataset定義hash': _hash('0'),
    '対象Runtime一覧': ['runtime-a', 'runtime-b'],
    '状態': resultCount == 0 ? '承認待ち' : (completed ? '完了' : '実行中'),
    '計画Case数': 1,
    '結果数': resultCount,
    '作成時刻UnixMillis': 1726300000000,
    '開始時刻UnixMillis': resultCount == 0 ? null : 1726300000100,
    '終了時刻UnixMillis': completed ? 1726300000200 : null,
    '計画監査ID': 'audit.evaluation.experiment.plan.1',
    '実験監査ID': 'audit.evaluation.experiment.1',
    '証拠種別': 'INTERNAL_STATE',
  };
}

Map<String, Object?> _statusBody() => {
  '版': 1,
  '評価実験': _experiment(resultCount: 1),
  '公開結果一覧': [_safeResult()],
};

Map<String, Object?> _safeResult() => {
  '評価ExperimentID': _experimentId,
  '評価DatasetID': _datasetId,
  '評価CaseID': _caseId,
  '実行系ID': 'runtime-a',
  '判定': '成立',
  '評価器判定一覧': [
    {
      '評価器ID': 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      '種類': 'exact',
      '判定': '成立',
      '理由code': '一致',
    },
  ],
  'LatencyMillis': 12,
  '総合hash': _hash('e'),
  '評価監査ID': 'audit.evaluation.result.1',
};

Map<String, Object?> _comparison() => {
  '版': 1,
  '比較ID': '88888888888888888888888888888888',
  '評価DatasetID': _datasetId,
  'Dataset定義hash': _hash('0'),
  '実験一覧': [
    _comparisonExperiment(_experimentId, 'runtime-a', passedCount: 1),
    _comparisonExperiment(_experimentId, 'runtime-b', passedCount: 1),
  ],
  '計画Case数': 1,
  '成立数': 2,
  '不成立数': 0,
  '評価不能数': 0,
  '中断数': 0,
  '比較時刻UnixMillis': 1726300000300,
  '経路差': 'same',
  '参照差': 'same',
  '能力差': 'unknown',
  '比較監査ID': 'audit.evaluation.comparison.1',
  '証拠種別': 'INTERNAL_STATE',
};

Map<String, Object?> _comparisonExperiment(
  String experimentId,
  String runtimeId, {
  int passedCount = 0,
  int failedCount = 0,
  int indeterminateCount = 0,
  int interruptedCount = 0,
}
) => {
  '評価ExperimentID': experimentId,
  '実行系ID': runtimeId,
  'Dataset定義hash': _hash('0'),
  '成立数': passedCount,
  '不成立数': failedCount,
  '評価不能数': indeterminateCount,
  '中断数': interruptedCount,
  '平均LatencyMillis': null,
};

String _hash(String character) => 'sha256:${character * 64}';
