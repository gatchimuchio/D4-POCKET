import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/evaluation_client.dart';

const _datasetId = '11111111111111111111111111111111';
const _caseId = '22222222222222222222222222222222';
const _secondCaseId = '33333333333333333333333333333333';
const _experimentId = '44444444444444444444444444444444';
const _comparisonId = '88888888888888888888888888888888';
const _evaluatorId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

void main() {
  test('通常操作だけが安全なpayloadでBrokerへ渡る', () async {
    final transport = _FixtureTransport({
      '評価Dataset一覧': [_accepted('評価Dataset一覧', _listingBody())],
      '評価実験開始': [
        _accepted(
          '評価実験開始',
          _experiment(),
          auditId: 'audit.evaluation.experiment.1',
        ),
      ],
      '評価実験状態': [_accepted('評価実験状態', _statusBody())],
      '評価比較': [
        _accepted(
          '評価比較',
          _comparison(),
          auditId: 'audit.evaluation.comparison.1',
        ),
      ],
    });
    final client = EvaluationClient(transport);

    final datasets = await client.listDatasets();
    final experiment = await client.startExperiment(_datasetId, const [
      'runtime-a',
      'runtime-b',
    ]);
    final status = await client.experimentStatus(_experimentId);
    final comparison = await client.compare(_experimentId);

    expect(datasets.datasets.single.displayName, '基本対話評価');
    expect(experiment.experimentId, _experimentId);
    expect(experiment.planAuditId, 'audit.evaluation.experiment.plan.1');
    expect(status.results, isEmpty);
    expect(comparison.experiments, hasLength(2));
    expect(transport.requests.map((request) => request.operation), const [
      '評価Dataset一覧',
      '評価実験開始',
      '評価実験状態',
      '評価比較',
    ]);
    expect(transport.requests[0].payload, const {'版': 1});
    expect(transport.requests[1].payload, {
      '版': 1,
      '評価DatasetID': _datasetId,
      '対象Runtime一覧': const ['runtime-a', 'runtime-b'],
    });
    expect(transport.requests[2].payload, const {
      '版': 1,
      '評価ExperimentID': _experimentId,
    });
    expect(transport.requests[3].payload, const {
      '版': 1,
      '評価ExperimentID': _experimentId,
    });
    for (final request in transport.requests) {
      expect(
        request.payload.keys,
        isNot(contains('owner')),
        reason: '通常clientはowner操作や資格をpayloadへ含めない',
      );
    }
  });

  test('operation、監査ID、status、bodyが不正な応答を閉鎖側で拒否する', () async {
    final wrongOperation = EvaluationClient(
      _FixtureTransport({
        '評価Dataset一覧': [_accepted('評価実験開始', _listingBody())],
      }),
    );
    await expectLater(
      wrongOperation.listDatasets(),
      throwsA(isA<BrokerClientException>()),
    );

    final noAudit = _accepted('評価Dataset一覧', _listingBody())
      ..['audit_event_id'] = '';
    final wrongAudit = EvaluationClient(
      _FixtureTransport({
        '評価Dataset一覧': [noAudit],
      }),
    );
    await expectLater(
      wrongAudit.listDatasets(),
      throwsA(isA<BrokerClientException>()),
    );

    final mismatchedStartAudit = EvaluationClient(
      _FixtureTransport({
        '評価実験開始': [
          _accepted(
            '評価実験開始',
            _experiment(),
            auditId: 'audit.evaluation.other',
          ),
        ],
      }),
    );
    await expectLater(
      mismatchedStartAudit.startExperiment(
        _datasetId,
        const ['runtime-a', 'runtime-b'],
      ),
      throwsA(isA<BrokerClientException>()),
    );

    final missingPlanAudit = _experiment()..remove('計画監査ID');
    final noPlanAudit = EvaluationClient(
      _FixtureTransport({
        '評価実験開始': [
          _accepted(
            '評価実験開始',
            missingPlanAudit,
            auditId: 'audit.evaluation.experiment.1',
          ),
        ],
      }),
    );
    await expectLater(
      noPlanAudit.startExperiment(
        _datasetId,
        const ['runtime-a', 'runtime-b'],
      ),
      throwsA(isA<BrokerClientException>()),
    );

    final invalidPlanAudit = _experiment()..['計画監査ID'] = '';
    final malformedPlanAudit = EvaluationClient(
      _FixtureTransport({
        '評価実験開始': [
          _accepted(
            '評価実験開始',
            invalidPlanAudit,
            auditId: 'audit.evaluation.experiment.1',
          ),
        ],
      }),
    );
    await expectLater(
      malformedPlanAudit.startExperiment(
        _datasetId,
        const ['runtime-a', 'runtime-b'],
      ),
      throwsA(isA<BrokerClientException>()),
    );

    final mismatchedComparisonAudit = EvaluationClient(
      _FixtureTransport({
        '評価比較': [
          _accepted(
            '評価比較',
            _comparison(),
            auditId: 'audit.evaluation.other',
          ),
        ],
      }),
    );
    await expectLater(
      mismatchedComparisonAudit.compare(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );

    final bodyWithUnknownField = _listingBody()..['unexpected'] = true;
    final unknownBody = EvaluationClient(
      _FixtureTransport({
        '評価Dataset一覧': [_accepted('評価Dataset一覧', bodyWithUnknownField)],
      }),
    );
    await expectLater(
      unknownBody.listDatasets(),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('raw Dataset keyと結果の本文・追跡値・応答hashを拒否する', () async {
    final rawDataset = _dataset()
      ..['入力'] = 'private input'
      ..['JSONスキーマ'] = {'type': 'object'}
      ..['期待本文'] = 'private expected body';
    final datasetClient = EvaluationClient(
      _FixtureTransport({
        '評価Dataset一覧': [
          _accepted('評価Dataset一覧', {
            '版': 1,
            '評価Dataset一覧': [rawDataset],
          }),
        ],
      }),
    );
    await expectLater(
      datasetClient.listDatasets(),
      throwsA(isA<BrokerClientException>()),
    );

    final unsafeResult = _safeResult()
      ..['本文'] = 'private result body'
      ..['対話要求ID'] = '66666666666666666666666666666666'
      ..['作成監査ID'] = 'audit.evaluation.create.1'
      ..['応答hash'] = _hash('5');
    final resultClient = EvaluationClient(
      _FixtureTransport({
        '評価実験状態': [
          _accepted('評価実験状態', {
            '版': 1,
            '評価実験': _experiment(resultCount: 1),
            '公開結果一覧': [unsafeResult],
          }),
        ],
      }),
    );
    await expectLater(
      resultClient.experimentStatus(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('評価器理由codeは固定9値以外を拒否する', () async {
    const allowedReasonCodes = <String>{
      '一致',
      '不一致',
      '入力不足',
      '応答不正',
      '根拠不足',
      '閾値超過',
      '中止',
      '監査失敗',
      '評価器不正',
    };

    for (final reasonCode in allowedReasonCodes) {
      final safeResult = _safeResult()
        ..['評価器判定一覧'] = [
          {
            '評価器ID': _evaluatorId,
            '種類': 'exact',
            '判定': '成立',
            '理由code': reasonCode,
          },
        ];
      final client = EvaluationClient(
        _FixtureTransport({
          '評価実験状態': [
            _accepted('評価実験状態', {
              '版': 1,
              '評価実験': _experiment(resultCount: 1),
              '公開結果一覧': [safeResult],
            }),
          ],
        }),
      );

      final status = await client.experimentStatus(_experimentId);
      expect(status.results.single.evaluators.single.reasonCode, reasonCode);
    }

    final unsafeResult = _safeResult()
      ..['評価器判定一覧'] = [
        {
          '評価器ID': _evaluatorId,
          '種類': 'exact',
          '判定': '成立',
          '理由code': '任意の安全そうな説明文',
        },
      ];
    final unsafeClient = EvaluationClient(
      _FixtureTransport({
        '評価実験状態': [
          _accepted('評価実験状態', {
            '版': 1,
            '評価実験': _experiment(resultCount: 1),
            '公開結果一覧': [unsafeResult],
          }),
        ],
      }),
    );
    await expectLater(
      unsafeClient.experimentStatus(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('Dataset一覧は同一IDの異なるrevisionを許容し、同じ対だけを拒否する', () async {
    final firstRevision = _dataset();
    final secondRevision = _dataset()
      ..['revision'] = 2
      ..['定義hash'] = _hash('1')
      ..['非公開保管ID'] = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab'
      ..['暗号文hash'] = _hash('c')
      ..['作成監査ID'] = 'audit.evaluation.dataset.create.2';
    final acceptedClient = EvaluationClient(
      _FixtureTransport({
        '評価Dataset一覧': [
          _accepted('評価Dataset一覧', {
            '版': 1,
            '評価Dataset一覧': [firstRevision, secondRevision],
          }),
        ],
      }),
    );

    final listing = await acceptedClient.listDatasets();
    expect(listing.datasets.map((dataset) => dataset.revision), [1, 2]);

    final duplicateClient = EvaluationClient(
      _FixtureTransport({
        '評価Dataset一覧': [
          _accepted('評価Dataset一覧', {
            '版': 1,
            '評価Dataset一覧': [_dataset(), _dataset()],
          }),
        ],
      }),
    );
    await expectLater(
      duplicateClient.listDatasets(),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('Dataset revisionはJSONとDart Webで安全な正整数だけを受け取る', () async {
    for (final invalidRevision in <Object?>[
      0,
      -1,
      1.0,
      9007199254740992,
    ]) {
      final dataset = _dataset()..['revision'] = invalidRevision;
      final client = EvaluationClient(
        _FixtureTransport({
          '評価Dataset一覧': [
            _accepted('評価Dataset一覧', {
              '版': 1,
              '評価Dataset一覧': [dataset],
            }),
          ],
        }),
      );
      await expectLater(
        client.listDatasets(),
        throwsA(isA<BrokerClientException>()),
      );
    }
  });

  test('比較の全aggregateを要求Experimentへ結び、実行系ID重複を拒否する', () async {
    final mismatchedExperiment = _comparison()
      ..['実験一覧'] = [
        _comparisonExperiment(_experimentId, 'runtime-a', passedCount: 1),
        _comparisonExperiment(
          '55555555555555555555555555555555',
          'runtime-b',
          interruptedCount: 1,
        ),
      ];
    final mismatchedClient = EvaluationClient(
      _FixtureTransport({
        '評価比較': [
          _accepted(
            '評価比較',
            mismatchedExperiment,
            auditId: 'audit.evaluation.comparison.1',
          ),
        ],
      }),
    );
    await expectLater(
      mismatchedClient.compare(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );

    final mismatchedDataset = _comparison()
      ..['実験一覧'] = [
        _comparisonExperiment(_experimentId, 'runtime-a', passedCount: 1),
        _comparisonExperiment(
          _experimentId,
          'runtime-b',
          interruptedCount: 1,
        )..['Dataset定義hash'] = _hash('1'),
      ];
    final mismatchedDatasetClient = EvaluationClient(
      _FixtureTransport({
        '評価比較': [
          _accepted(
            '評価比較',
            mismatchedDataset,
            auditId: 'audit.evaluation.comparison.1',
          ),
        ],
      }),
    );
    await expectLater(
      mismatchedDatasetClient.compare(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );

    final duplicateRuntime = _comparison()
      ..['実験一覧'] = [
        _comparisonExperiment(_experimentId, 'runtime-a', passedCount: 1),
        _comparisonExperiment(_experimentId, 'runtime-a', interruptedCount: 1),
      ];
    final duplicateRuntimeClient = EvaluationClient(
      _FixtureTransport({
        '評価比較': [
          _accepted(
            '評価比較',
            duplicateRuntime,
            auditId: 'audit.evaluation.comparison.1',
          ),
        ],
      }),
    );
    await expectLater(
      duplicateRuntimeClient.compare(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('完了Experimentと比較集計の完結条件を検証する', () async {
    final completeClient = EvaluationClient(
      _FixtureTransport({
        '評価実験状態': [
          _accepted('評価実験状態', {
            '版': 1,
            '評価実験': _experiment(resultCount: 2),
            '公開結果一覧': _completeResults(),
          }),
        ],
      }),
    );
    final completeStatus = await completeClient.experimentStatus(_experimentId);
    expect(completeStatus.experiment.state, '完了');
    expect(completeStatus.results, hasLength(2));

    final incompleteComplete = _experiment(resultCount: 1)..['状態'] = '完了';
    final incompleteCompleteClient = EvaluationClient(
      _FixtureTransport({
        '評価実験状態': [
          _accepted('評価実験状態', {
            '版': 1,
            '評価実験': incompleteComplete,
            '公開結果一覧': [_safeResult()],
          }),
        ],
      }),
    );
    await expectLater(
      incompleteCompleteClient.experimentStatus(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );

    final overflowComplete = _experiment(resultCount: 3)..['状態'] = '完了';
    final overflowCompleteClient = EvaluationClient(
      _FixtureTransport({
        '評価実験状態': [
          _accepted('評価実験状態', {
            '版': 1,
            '評価実験': overflowComplete,
            '公開結果一覧': const [],
          }),
        ],
      }),
    );
    await expectLater(
      overflowCompleteClient.experimentStatus(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );

    final completeWithoutFinishedAt = _experiment(resultCount: 2)
      ..['終了時刻UnixMillis'] = null;
    final completeWithoutFinishedAtClient = EvaluationClient(
      _FixtureTransport({
        '評価実験状態': [
          _accepted('評価実験状態', {
            '版': 1,
            '評価実験': completeWithoutFinishedAt,
            '公開結果一覧': [_safeResult(), _safeResult()..['実行系ID'] = 'runtime-b'],
          }),
        ],
      }),
    );
    await expectLater(
      completeWithoutFinishedAtClient.experimentStatus(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );

    final completeWithoutStartedAt = _experiment(resultCount: 2)
      ..['開始時刻UnixMillis'] = null;
    final completeWithoutStartedAtClient = EvaluationClient(
      _FixtureTransport({
        '評価実験状態': [
          _accepted('評価実験状態', {
            '版': 1,
            '評価実験': completeWithoutStartedAt,
            '公開結果一覧': _completeResults(),
          }),
        ],
      }),
    );
    await expectLater(
      completeWithoutStartedAtClient.experimentStatus(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );

    final entryOverflow = _comparison();
    final overflowEntries = entryOverflow['実験一覧'] as List<Object?>;
    (overflowEntries.first as Map<String, Object?>)['成立数'] = 2;
    (overflowEntries.last as Map<String, Object?>)['中断数'] = 0;
    entryOverflow['成立数'] = 2;
    entryOverflow['中断数'] = 0;
    final entryOverflowClient = EvaluationClient(
      _FixtureTransport({
        '評価比較': [
          _accepted(
            '評価比較',
            entryOverflow,
            auditId: 'audit.evaluation.comparison.1',
          ),
        ],
      }),
    );
    await expectLater(
      entryOverflowClient.compare(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );

    final entryUnderflow = _comparison();
    final underflowEntries = entryUnderflow['実験一覧'] as List<Object?>;
    (underflowEntries.first as Map<String, Object?>)['成立数'] = 0;
    entryUnderflow['成立数'] = 0;
    final entryUnderflowClient = EvaluationClient(
      _FixtureTransport({
        '評価比較': [
          _accepted(
            '評価比較',
            entryUnderflow,
            auditId: 'audit.evaluation.comparison.1',
          ),
        ],
      }),
    );
    await expectLater(
      entryUnderflowClient.compare(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );

    final rootMismatch = _comparison()..['成立数'] = 0;
    final rootMismatchClient = EvaluationClient(
      _FixtureTransport({
        '評価比較': [
          _accepted(
            '評価比較',
            rootMismatch,
            auditId: 'audit.evaluation.comparison.1',
          ),
        ],
      }),
    );
    await expectLater(
      rootMismatchClient.compare(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('Case数とRuntime数の乗算で完了結果と比較集計を検証する', () async {
    final fourResultClient = EvaluationClient(
      _FixtureTransport({
        '評価実験状態': [
          _accepted('評価実験状態', {
            '版': 1,
            '評価実験': _experiment(
              plannedCaseCount: 2,
              resultCount: 4,
            ),
            '公開結果一覧': _twoCaseCompleteResults(),
          }),
        ],
      }),
    );
    final fourResultStatus = await fourResultClient.experimentStatus(
      _experimentId,
    );
    expect(fourResultStatus.experiment.resultCount, 4);
    expect(fourResultStatus.results, hasLength(4));

    final incompleteFourResult = _experiment(
      plannedCaseCount: 2,
      resultCount: 2,
    )..['状態'] = '完了';
    final incompleteFourResultClient = EvaluationClient(
      _FixtureTransport({
        '評価実験状態': [
          _accepted('評価実験状態', {
            '版': 1,
            '評価実験': incompleteFourResult,
            '公開結果一覧': _completeResults(),
          }),
        ],
      }),
    );
    await expectLater(
      incompleteFourResultClient.experimentStatus(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );

    final twoCaseComparison = _comparison();
    final twoCaseEntries = twoCaseComparison['実験一覧'] as List<Object?>;
    twoCaseComparison['計画Case数'] = 2;
    (twoCaseEntries[0] as Map<String, Object?>)['成立数'] = 2;
    (twoCaseEntries[1] as Map<String, Object?>)['中断数'] = 2;
    twoCaseComparison['成立数'] = 2;
    twoCaseComparison['中断数'] = 2;
    final twoCaseComparisonClient = EvaluationClient(
      _FixtureTransport({
        '評価比較': [
          _accepted(
            '評価比較',
            twoCaseComparison,
            auditId: 'audit.evaluation.comparison.1',
          ),
        ],
      }),
    );
    final comparison = await twoCaseComparisonClient.compare(_experimentId);
    expect(comparison.plannedCaseCount, 2);
    expect(comparison.passedCount + comparison.interruptedCount, 4);

    final underfilledTwoCaseComparison = _comparison();
    final underfilledEntries =
        underfilledTwoCaseComparison['実験一覧'] as List<Object?>;
    underfilledTwoCaseComparison['計画Case数'] = 2;
    (underfilledEntries[0] as Map<String, Object?>)['成立数'] = 1;
    (underfilledEntries[1] as Map<String, Object?>)['中断数'] = 2;
    underfilledTwoCaseComparison['成立数'] = 1;
    underfilledTwoCaseComparison['中断数'] = 2;
    final underfilledTwoCaseComparisonClient = EvaluationClient(
      _FixtureTransport({
        '評価比較': [
          _accepted(
            '評価比較',
            underfilledTwoCaseComparison,
            auditId: 'audit.evaluation.comparison.1',
          ),
        ],
      }),
    );
    await expectLater(
      underfilledTwoCaseComparisonClient.compare(_experimentId),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('Dataset IDと実行系ID以外のRun入力をBrokerへ送らない', () async {
    final transport = _FixtureTransport({
      '評価実験開始': [_accepted('評価実験開始', _experiment())],
    });
    final client = EvaluationClient(transport);

    await expectLater(
      client.startExperiment('raw input', const ['runtime-a']),
      throwsA(isA<BrokerClientException>()),
    );
    await expectLater(
      client.startExperiment(_datasetId, const ['runtime-a', 'runtime-a']),
      throwsA(isA<BrokerClientException>()),
    );

    expect(
      EvaluationClient.validRuntimeIds(const [
        'runtime-1',
        'runtime-2',
        'runtime-3',
        'runtime-4',
        'runtime-5',
        'runtime-6',
        'runtime-7',
        'runtime-8',
      ]),
      isTrue,
    );
    expect(
      EvaluationClient.validRuntimeIds(const [
        'runtime-1',
        'runtime-2',
        'runtime-3',
        'runtime-4',
        'runtime-5',
        'runtime-6',
        'runtime-7',
        'runtime-8',
        'runtime-9',
      ]),
      isFalse,
    );

    expect(transport.requests, isEmpty);
  });
}

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
      throw BrokerClientException('$operation のfixtureがありません');
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

Map<String, Object?> _experiment({
  int plannedCaseCount = 1,
  int resultCount = 0,
}) {
  final completed = resultCount == plannedCaseCount * 2;
  return {
    '版': 1,
    '評価ExperimentID': _experimentId,
    '評価DatasetID': _datasetId,
    'Dataset定義hash': _hash('0'),
    '対象Runtime一覧': ['runtime-a', 'runtime-b'],
    '状態': resultCount == 0 ? '承認待ち' : (completed ? '完了' : '実行中'),
    '計画Case数': plannedCaseCount,
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
  '評価実験': _experiment(),
  '公開結果一覧': const [],
};

Map<String, Object?> _safeResult() => {
  '評価ExperimentID': _experimentId,
  '評価DatasetID': _datasetId,
  '評価CaseID': _caseId,
  '実行系ID': 'runtime-a',
  '判定': '成立',
  '評価器判定一覧': [
    {'評価器ID': _evaluatorId, '種類': 'exact', '判定': '成立', '理由code': '一致'},
  ],
  'LatencyMillis': 12,
  '総合hash': _hash('e'),
  '評価監査ID': 'audit.evaluation.result.1',
};

List<Map<String, Object?>> _completeResults() => [
  _safeResult(),
  _safeResult()..['実行系ID'] = 'runtime-b',
];

List<Map<String, Object?>> _twoCaseCompleteResults() => [
  _safeResult(),
  _safeResult()..['実行系ID'] = 'runtime-b',
  _safeResult()..['評価CaseID'] = _secondCaseId,
  _safeResult()
    ..['評価CaseID'] = _secondCaseId
    ..['実行系ID'] = 'runtime-b',
];

Map<String, Object?> _comparison() => {
  '版': 1,
  '比較ID': _comparisonId,
  '評価DatasetID': _datasetId,
  'Dataset定義hash': _hash('0'),
  '実験一覧': [
    _comparisonExperiment(_experimentId, 'runtime-a', passedCount: 1),
    _comparisonExperiment(_experimentId, 'runtime-b', interruptedCount: 1),
  ],
  '計画Case数': 1,
  '成立数': 1,
  '不成立数': 0,
  '評価不能数': 0,
  '中断数': 1,
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
}) =>
    {
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
