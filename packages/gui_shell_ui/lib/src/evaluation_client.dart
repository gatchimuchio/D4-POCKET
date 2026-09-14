import 'broker_transport.dart';

const _datasetListOperation = '評価Dataset一覧';
const _experimentStartOperation = '評価実験開始';
const _experimentStatusOperation = '評価実験状態';
const _comparisonOperation = '評価比較';

const _allowedOperations = <String>{
  _datasetListOperation,
  _experimentStartOperation,
  _experimentStatusOperation,
  _comparisonOperation,
};

const _experimentStates = <String>{'準備済み', '承認待ち', '実行中', '完了', '中断', '評価不能'};

const _judgements = <String>{'成立', '不成立', '評価不能', '中断'};
const _evaluatorKinds = <String>{
  'exact',
  'contains',
  'regex',
  'json_schema',
  'reference_count',
  'route',
  'status',
  'capability',
  'latency_threshold',
};
const _evaluatorReasonCodes = <String>{
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

// JSONとDart Webの双方で正確に表せる正整数に限定する。
const _maximumSafeInteger = 9007199254740991;

final _opaqueId = RegExp(r'^[a-f0-9]{32}$');
final _runtimeId = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$');
final _auditId = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.:-]{0,255}$');
final _hash = RegExp(r'^sha256:[a-f0-9]{64}$');

const _datasetForbiddenKeys = <String>{
  '入力',
  '本文',
  '期待',
  '期待値',
  '期待本文',
  '期待参照数',
  '期待経路',
  '期待状態',
  '期待能力',
  '上限ミリ秒',
  '正規表現',
  'jsonスキーマ',
  '設定',
  '評価器設定',
  'input',
  'rawinput',
  'rawpayload',
  'expected',
  'expectedbody',
  'expectedreferencecount',
  'expectedroute',
  'expectedstatus',
  'expectedcapability',
  'regex',
  'pattern',
  'jsonschema',
  'schema',
};

const _resultForbiddenKeys = <String>{
  '本文',
  '入力',
  '期待',
  '期待値',
  '参照',
  '能力',
  '経路',
  '追跡',
  '対話要求ID',
  '対話セッションID',
  '開始監査ID',
  '終了監査ID',
  '応答hash',
  'input',
  'rawinput',
  'expected',
  'reference',
  'capability',
  'route',
  'trace',
  'requestid',
  'sessionid',
  'responsehash',
};

/// 通常資格でC5の公開評価projectionだけを受け取るclient。
///
/// owner操作、Datasetの非公開payload、期待値、Evaluator設定はこの操作面に
/// 存在しない。評価は運用観測であり、権限、承認、監査、release判定を
/// 作らない。
class EvaluationClient {
  EvaluationClient(this.transport);

  final BrokerTransport transport;

  static bool validDatasetId(Object? value) => _isOpaqueId(value);

  static bool validExperimentId(Object? value) => _isOpaqueId(value);

  static bool validRuntimeId(Object? value) =>
      value is String && value.length <= 128 && _runtimeId.hasMatch(value);

  static bool validRuntimeIds(Object? value) {
    if (value is! List || value.isEmpty || value.length > 8) {
      return false;
    }
    final ids = value.cast<Object?>();
    return ids.every(validRuntimeId) && ids.toSet().length == ids.length;
  }

  Future<EvaluationDatasetListing> listDatasets() async {
    final response = await _accepted(_datasetListOperation, const {'版': 1});
    return EvaluationDatasetListing._parse(response.body, response.auditId);
  }

  Future<EvaluationExperiment> startExperiment(
    String datasetId,
    List<String> runtimeIds,
  ) async {
    if (!validDatasetId(datasetId) || !validRuntimeIds(runtimeIds)) {
      _reject();
    }
    final response = await _accepted(_experimentStartOperation, {
      '版': 1,
      '評価DatasetID': datasetId,
      '対象Runtime一覧': List<String>.unmodifiable(runtimeIds),
    });
    return EvaluationExperiment._parse(
      response.body,
      expectedDatasetId: datasetId,
      expectedRuntimeIds: runtimeIds,
      expectedAuditId: response.auditId,
    );
  }

  Future<EvaluationExperimentStatus> experimentStatus(
    String experimentId,
  ) async {
    if (!validExperimentId(experimentId)) _reject();
    final response = await _accepted(_experimentStatusOperation, {
      '版': 1,
      '評価ExperimentID': experimentId,
    });
    return EvaluationExperimentStatus._parse(
      response.body,
      requestedExperimentId: experimentId,
      responseAuditId: response.auditId,
    );
  }

  Future<EvaluationComparison> compare(String experimentId) async {
    if (!validExperimentId(experimentId)) _reject();
    final response = await _accepted(_comparisonOperation, {
      '版': 1,
      '評価ExperimentID': experimentId,
    });
    return EvaluationComparison._parse(
      response.body,
      requestedExperimentId: experimentId,
      responseAuditId: response.auditId,
    );
  }

  Future<_AcceptedEnvelope> _accepted(
    String operation,
    Map<String, Object?> payload,
  ) async {
    if (!_allowedOperations.contains(operation) ||
        !_safePayload(operation, payload)) {
      _reject();
    }
    final raw = await transport.request(operation, payload: payload);
    final response = _object(raw);
    _exactKeys(response, const {
      'request_id',
      'operation',
      'status',
      'evidence_source',
      'audit_event_id',
      'error',
      'health',
      'body',
      'shutdown_requested',
    });
    final auditId = response['audit_event_id'];
    if (!_safeText(response['request_id'], maxLength: 256) ||
        response['operation'] != operation ||
        response['status'] != 'accepted' ||
        !_isAuditId(auditId) ||
        response['evidence_source'] != 'INTERNAL_STATE' ||
        response['error'] != null ||
        response['health'] != null ||
        response['shutdown_requested'] != false) {
      _reject();
    }
    return _AcceptedEnvelope(_object(response['body']), auditId as String);
  }
}

class _AcceptedEnvelope {
  const _AcceptedEnvelope(this.body, this.auditId);

  final Map<String, Object?> body;
  final String auditId;
}

/// Dataset一覧の監査済み公開projection。
class EvaluationDatasetListing {
  const EvaluationDatasetListing._(this.datasets, this.auditId);

  final List<EvaluationDataset> datasets;
  final String auditId;

  static EvaluationDatasetListing _parse(
    Map<String, Object?> body,
    String auditId,
  ) {
    _exactKeys(body, const {'版', '評価Dataset一覧'});
    final rawDatasets = body['評価Dataset一覧'];
    if (body['版'] != 1 || rawDatasets is! List || rawDatasets.length > 128) {
      _reject();
    }
    _rejectForbiddenKeys(rawDatasets, _datasetForbiddenKeys);
    final datasets = rawDatasets
        .map((item) => EvaluationDataset._parse(item))
        .toList(growable: false);
    if (datasets
            .map((item) => '${item.datasetId}:${item.revision}')
            .toSet()
            .length !=
        datasets.length) {
      _reject();
    }
    return EvaluationDatasetListing._(List.unmodifiable(datasets), auditId);
  }
}

/// hash_onlyで公開できる一つの不変Dataset revision。
class EvaluationDataset {
  const EvaluationDataset._({
    required this.datasetId,
    required this.revision,
    required this.definitionHash,
    required this.displayName,
    required this.caseCount,
    required this.cases,
    required this.createdAtUnixMillis,
    required this.creationAuditId,
  });

  final String datasetId;
  final int revision;
  final String definitionHash;
  final String displayName;
  final int caseCount;
  final List<EvaluationCaseSummary> cases;
  final int createdAtUnixMillis;
  final String creationAuditId;

  static EvaluationDataset _parse(Object? raw) {
    final value = _object(raw);
    _exactKeys(value, const {
      '版',
      '評価DatasetID',
      'revision',
      '定義hash',
      '非公開保管ID',
      '暗号文hash',
      '公開表示名',
      'Case数',
      'Case一覧',
      '公開範囲',
      '作成時刻UnixMillis',
      '作成監査ID',
      '証拠種別',
    });
    final datasetId = value['評価DatasetID'];
    final revision = value['revision'];
    final definitionHash = value['定義hash'];
    final displayName = value['公開表示名'];
    final caseCount = value['Case数'];
    final rawCases = value['Case一覧'];
    final createdAt = value['作成時刻UnixMillis'];
    final creationAuditId = value['作成監査ID'];
    if (value['版'] != 1 ||
        !_isOpaqueId(datasetId) ||
        !_positiveSafeInteger(revision) ||
        !_isHash(definitionHash) ||
        !_isOpaqueId(value['非公開保管ID']) ||
        !_isHash(value['暗号文hash']) ||
        !_safeText(displayName, maxLength: 128) ||
        caseCount is! int ||
        caseCount < 1 ||
        caseCount > 128 ||
        rawCases is! List ||
        rawCases.length != caseCount ||
        value['公開範囲'] != 'hash_only' ||
        !_timestampMillis(createdAt) ||
        !_isAuditId(creationAuditId) ||
        value['証拠種別'] != 'INTERNAL_STATE') {
      _reject();
    }
    final cases = rawCases
        .map((item) => EvaluationCaseSummary._parse(item))
        .toList(growable: false);
    if (cases.map((item) => item.caseId).toSet().length != cases.length) {
      _reject();
    }
    return EvaluationDataset._(
      datasetId: datasetId as String,
      revision: revision as int,
      definitionHash: definitionHash as String,
      displayName: displayName as String,
      caseCount: caseCount,
      cases: List.unmodifiable(cases),
      createdAtUnixMillis: createdAt as int,
      creationAuditId: creationAuditId as String,
    );
  }
}

class EvaluationCaseSummary {
  const EvaluationCaseSummary._(this.caseId, this.definitionHash);

  final String caseId;
  final String definitionHash;

  static EvaluationCaseSummary _parse(Object? raw) {
    final value = _object(raw);
    _exactKeys(value, const {'評価CaseID', '定義hash'});
    if (!_isOpaqueId(value['評価CaseID']) || !_isHash(value['定義hash'])) {
      _reject();
    }
    return EvaluationCaseSummary._(
      value['評価CaseID'] as String,
      value['定義hash'] as String,
    );
  }
}

/// 実験本体の公開projection。実行、承認、権限、release判断は含まない。
class EvaluationExperiment {
  const EvaluationExperiment._({
    required this.experimentId,
    required this.datasetId,
    required this.datasetDefinitionHash,
    required this.runtimeIds,
    required this.state,
    required this.plannedCaseCount,
    required this.resultCount,
    required this.createdAtUnixMillis,
    required this.startedAtUnixMillis,
    required this.finishedAtUnixMillis,
    required this.planAuditId,
    required this.auditId,
  });

  final String experimentId;
  final String datasetId;
  final String datasetDefinitionHash;
  final List<String> runtimeIds;
  final String state;
  final int plannedCaseCount;
  final int resultCount;
  final int createdAtUnixMillis;
  final int? startedAtUnixMillis;
  final int? finishedAtUnixMillis;
  final String planAuditId;
  final String auditId;

  static EvaluationExperiment _parse(
    Object? raw, {
    String? expectedDatasetId,
    List<String>? expectedRuntimeIds,
    String? expectedAuditId,
  }) {
    final value = _object(raw);
    _exactKeys(value, const {
      '版',
      '評価ExperimentID',
      '評価DatasetID',
      'Dataset定義hash',
      '対象Runtime一覧',
      '状態',
      '計画Case数',
      '結果数',
      '作成時刻UnixMillis',
      '開始時刻UnixMillis',
      '終了時刻UnixMillis',
      '計画監査ID',
      '実験監査ID',
      '証拠種別',
    });
    final experimentId = value['評価ExperimentID'];
    final datasetId = value['評価DatasetID'];
    final definitionHash = value['Dataset定義hash'];
    final rawRuntimes = value['対象Runtime一覧'];
    final state = value['状態'];
    final planned = value['計画Case数'];
    final results = value['結果数'];
    final createdAt = value['作成時刻UnixMillis'];
    final startedAt = value['開始時刻UnixMillis'];
    final finishedAt = value['終了時刻UnixMillis'];
    final planAuditId = value['計画監査ID'];
    final auditId = value['実験監査ID'];
    if (value['版'] != 1 ||
        !_isOpaqueId(experimentId) ||
        !_isOpaqueId(datasetId) ||
        !_isHash(definitionHash) ||
        !EvaluationClient.validRuntimeIds(rawRuntimes) ||
        state is! String ||
        !_experimentStates.contains(state) ||
        planned is! int ||
        planned < 1 ||
        planned > 128 ||
        results is! int ||
        results < 0 ||
        results > 1024 ||
        results > planned * (rawRuntimes as List).length ||
        !_timestampMillis(createdAt) ||
        (startedAt != null && !_timestampMillis(startedAt)) ||
        (finishedAt != null && !_timestampMillis(finishedAt)) ||
        (startedAt != null && (startedAt as int) < (createdAt as int)) ||
        (finishedAt != null &&
            (finishedAt as int) < (startedAt as int? ?? createdAt as int)) ||
        !_isAuditId(planAuditId) ||
        !_isAuditId(auditId) ||
        value['証拠種別'] != 'INTERNAL_STATE' ||
        (expectedDatasetId != null && datasetId != expectedDatasetId) ||
        (expectedRuntimeIds != null &&
            !_sameStrings(rawRuntimes.cast<String>(), expectedRuntimeIds)) ||
        (expectedAuditId != null && auditId != expectedAuditId)) {
      _reject();
    }
    final expectedResultCount = planned * rawRuntimes.length;
    if (state == '完了' &&
        (results != expectedResultCount ||
            startedAt == null ||
            finishedAt == null)) {
      _reject();
    }
    return EvaluationExperiment._(
      experimentId: experimentId as String,
      datasetId: datasetId as String,
      datasetDefinitionHash: definitionHash as String,
      runtimeIds: List.unmodifiable(rawRuntimes.cast<String>()),
      state: state,
      plannedCaseCount: planned,
      resultCount: results,
      createdAtUnixMillis: createdAt as int,
      startedAtUnixMillis: startedAt as int?,
      finishedAtUnixMillis: finishedAt as int?,
      planAuditId: planAuditId as String,
      auditId: auditId as String,
    );
  }
}

/// 評価実験状態と、本文・追跡値を除去した評価結果一覧。
class EvaluationExperimentStatus {
  const EvaluationExperimentStatus._(
    this.experiment,
    this.results,
    this.auditId,
  );

  final EvaluationExperiment experiment;
  final List<EvaluationResultSummary> results;
  final String auditId;

  static EvaluationExperimentStatus _parse(
    Map<String, Object?> body, {
    required String requestedExperimentId,
    required String responseAuditId,
  }) {
    _exactKeys(body, const {'版', '評価実験', '公開結果一覧'});
    final rawResults = body['公開結果一覧'];
    if (body['版'] != 1 || rawResults is! List || rawResults.length > 512) {
      _reject();
    }
    _rejectForbiddenKeys(rawResults, _resultForbiddenKeys);
    final experiment = EvaluationExperiment._parse(body['評価実験']);
    if (experiment.experimentId != requestedExperimentId) _reject();
    final results = rawResults
        .map(
          (item) => EvaluationResultSummary._parse(
            item,
            experimentId: experiment.experimentId,
            datasetId: experiment.datasetId,
            runtimeIds: experiment.runtimeIds,
          ),
        )
        .toList(growable: false);
    if (results.length != experiment.resultCount ||
        results
                .map((item) => '${item.caseId}:${item.runtimeId}')
                .toSet()
                .length !=
            results.length) {
      _reject();
    }
    return EvaluationExperimentStatus._(
      experiment,
      List.unmodifiable(results),
      responseAuditId,
    );
  }
}

/// 一件の評価結果から、本文、対話request/session、応答hashを除いた表示用summary。
class EvaluationResultSummary {
  const EvaluationResultSummary._({
    required this.caseId,
    required this.runtimeId,
    required this.judgement,
    required this.evaluators,
    required this.latencyMillis,
    required this.aggregateHash,
    required this.auditId,
  });

  final String caseId;
  final String runtimeId;
  final String judgement;
  final List<EvaluationEvaluatorJudgement> evaluators;
  final int? latencyMillis;
  final String aggregateHash;
  final String auditId;

  static EvaluationResultSummary _parse(
    Object? raw, {
    required String experimentId,
    required String datasetId,
    required List<String> runtimeIds,
  }) {
    final value = _object(raw);
    _exactKeys(value, const {
      '評価ExperimentID',
      '評価DatasetID',
      '評価CaseID',
      '実行系ID',
      '判定',
      '評価器判定一覧',
      'LatencyMillis',
      '総合hash',
      '評価監査ID',
    });
    final evaluators = value['評価器判定一覧'];
    final latency = value['LatencyMillis'];
    if (value['評価ExperimentID'] != experimentId ||
        value['評価DatasetID'] != datasetId ||
        !_isOpaqueId(value['評価CaseID']) ||
        !EvaluationClient.validRuntimeId(value['実行系ID']) ||
        !runtimeIds.contains(value['実行系ID']) ||
        !_judgements.contains(value['判定']) ||
        evaluators is! List ||
        evaluators.isEmpty ||
        evaluators.length > 16 ||
        (latency != null && !_nonNegativeInt(latency)) ||
        !_isHash(value['総合hash']) ||
        !_isAuditId(value['評価監査ID'])) {
      _reject();
    }
    final parsedEvaluators = evaluators
        .map(EvaluationEvaluatorJudgement._parse)
        .toList(growable: false);
    if (parsedEvaluators.map((item) => item.evaluatorId).toSet().length !=
        parsedEvaluators.length) {
      _reject();
    }
    return EvaluationResultSummary._(
      caseId: value['評価CaseID'] as String,
      runtimeId: value['実行系ID'] as String,
      judgement: value['判定'] as String,
      evaluators: List.unmodifiable(parsedEvaluators),
      latencyMillis: latency as int?,
      aggregateHash: value['総合hash'] as String,
      auditId: value['評価監査ID'] as String,
    );
  }
}

class EvaluationEvaluatorJudgement {
  const EvaluationEvaluatorJudgement._(
    this.evaluatorId,
    this.kind,
    this.judgement,
    this.reasonCode,
  );

  final String evaluatorId;
  final String kind;
  final String judgement;
  final String reasonCode;

  static EvaluationEvaluatorJudgement _parse(Object? raw) {
    final value = _object(raw);
    _exactKeys(value, const {'評価器ID', '種類', '判定', '理由code'});
    if (!_isOpaqueId(value['評価器ID']) ||
        !_evaluatorKinds.contains(value['種類']) ||
        !_judgements.contains(value['判定']) ||
        !_evaluatorReasonCodes.contains(value['理由code'])) {
      _reject();
    }
    return EvaluationEvaluatorJudgement._(
      value['評価器ID'] as String,
      value['種類'] as String,
      value['判定'] as String,
      value['理由code'] as String,
    );
  }
}

/// 同一Dataset revision上の集計比較。raw route/reference/capabilityは返さない。
class EvaluationComparison {
  const EvaluationComparison._({
    required this.comparisonId,
    required this.datasetId,
    required this.datasetDefinitionHash,
    required this.experiments,
    required this.plannedCaseCount,
    required this.passedCount,
    required this.failedCount,
    required this.indeterminateCount,
    required this.interruptedCount,
    required this.comparedAtUnixMillis,
    required this.routeDifference,
    required this.referenceDifference,
    required this.capabilityDifference,
    required this.auditId,
  });

  final String comparisonId;
  final String datasetId;
  final String datasetDefinitionHash;
  final List<EvaluationComparisonExperiment> experiments;
  final int plannedCaseCount;
  final int passedCount;
  final int failedCount;
  final int indeterminateCount;
  final int interruptedCount;
  final int comparedAtUnixMillis;
  final String routeDifference;
  final String referenceDifference;
  final String capabilityDifference;
  final String auditId;

  static EvaluationComparison _parse(
    Map<String, Object?> value, {
    required String requestedExperimentId,
    required String responseAuditId,
  }) {
    _rejectForbiddenKeys(value, _resultForbiddenKeys);
    _exactKeys(value, const {
      '版',
      '比較ID',
      '評価DatasetID',
      'Dataset定義hash',
      '実験一覧',
      '計画Case数',
      '成立数',
      '不成立数',
      '評価不能数',
      '中断数',
      '比較時刻UnixMillis',
      '経路差',
      '参照差',
      '能力差',
      '比較監査ID',
      '証拠種別',
    });
    final experiments = value['実験一覧'];
    final planned = value['計画Case数'];
    final passed = value['成立数'];
    final failed = value['不成立数'];
    final indeterminate = value['評価不能数'];
    final interrupted = value['中断数'];
    if (value['版'] != 1 ||
        !_isOpaqueId(value['比較ID']) ||
        !_isOpaqueId(value['評価DatasetID']) ||
        !_isHash(value['Dataset定義hash']) ||
        experiments is! List ||
        experiments.length < 2 ||
        experiments.length > 8 ||
        planned is! int ||
        planned < 1 ||
        planned > 128 ||
        passed is! int ||
        passed < 0 ||
        passed > 1024 ||
        failed is! int ||
        failed < 0 ||
        failed > 1024 ||
        indeterminate is! int ||
        indeterminate < 0 ||
        indeterminate > 1024 ||
        interrupted is! int ||
        interrupted < 0 ||
        interrupted > 1024 ||
        passed + failed + indeterminate + interrupted >
            planned * experiments.length ||
        !_timestampMillis(value['比較時刻UnixMillis']) ||
        !_difference(value['経路差']) ||
        !_difference(value['参照差']) ||
        !_difference(value['能力差']) ||
        !_isAuditId(value['比較監査ID']) ||
        value['比較監査ID'] != responseAuditId ||
        value['証拠種別'] != 'INTERNAL_STATE') {
      _reject();
    }
    final parsedExperiments = experiments
        .map(
          (item) => EvaluationComparisonExperiment._parse(
            item,
            datasetDefinitionHash: value['Dataset定義hash'] as String,
          ),
        )
        .toList(growable: false);
    if (!parsedExperiments.every(
          (item) => item.experimentId == requestedExperimentId,
        ) ||
        parsedExperiments.map((item) => item.runtimeId).toSet().length !=
            parsedExperiments.length ||
        parsedExperiments.any(
          (item) =>
              item.passedCount +
                  item.failedCount +
                  item.indeterminateCount +
                  item.interruptedCount !=
              planned,
        ) ||
        passed !=
            parsedExperiments.fold<int>(
              0,
              (total, item) => total + item.passedCount,
            ) ||
        failed !=
            parsedExperiments.fold<int>(
              0,
              (total, item) => total + item.failedCount,
            ) ||
        indeterminate !=
            parsedExperiments.fold<int>(
              0,
              (total, item) => total + item.indeterminateCount,
            ) ||
        interrupted !=
            parsedExperiments.fold<int>(
              0,
              (total, item) => total + item.interruptedCount,
            )) {
      _reject();
    }
    return EvaluationComparison._(
      comparisonId: value['比較ID'] as String,
      datasetId: value['評価DatasetID'] as String,
      datasetDefinitionHash: value['Dataset定義hash'] as String,
      experiments: List.unmodifiable(parsedExperiments),
      plannedCaseCount: planned,
      passedCount: passed,
      failedCount: failed,
      indeterminateCount: indeterminate,
      interruptedCount: interrupted,
      comparedAtUnixMillis: value['比較時刻UnixMillis'] as int,
      routeDifference: value['経路差'] as String,
      referenceDifference: value['参照差'] as String,
      capabilityDifference: value['能力差'] as String,
      auditId: value['比較監査ID'] as String,
    );
  }
}

class EvaluationComparisonExperiment {
  const EvaluationComparisonExperiment._({
    required this.experimentId,
    required this.runtimeId,
    required this.passedCount,
    required this.failedCount,
    required this.indeterminateCount,
    required this.interruptedCount,
    required this.averageLatencyMillis,
  });

  final String experimentId;
  final String runtimeId;
  final int passedCount;
  final int failedCount;
  final int indeterminateCount;
  final int interruptedCount;
  final num? averageLatencyMillis;

  static EvaluationComparisonExperiment _parse(
    Object? raw, {
    required String datasetDefinitionHash,
  }) {
    final value = _object(raw);
    _exactKeys(value, const {
      '評価ExperimentID',
      '実行系ID',
      'Dataset定義hash',
      '成立数',
      '不成立数',
      '評価不能数',
      '中断数',
      '平均LatencyMillis',
    });
    final average = value['平均LatencyMillis'];
    if (!_isOpaqueId(value['評価ExperimentID']) ||
        !EvaluationClient.validRuntimeId(value['実行系ID']) ||
        value['Dataset定義hash'] != datasetDefinitionHash ||
        !_nonNegativeInt(value['成立数']) ||
        !_nonNegativeInt(value['不成立数']) ||
        !_nonNegativeInt(value['評価不能数']) ||
        !_nonNegativeInt(value['中断数']) ||
        (average != null &&
            (average is! num || !average.isFinite || average < 0))) {
      _reject();
    }
    return EvaluationComparisonExperiment._(
      experimentId: value['評価ExperimentID'] as String,
      runtimeId: value['実行系ID'] as String,
      passedCount: value['成立数'] as int,
      failedCount: value['不成立数'] as int,
      indeterminateCount: value['評価不能数'] as int,
      interruptedCount: value['中断数'] as int,
      averageLatencyMillis: average as num?,
    );
  }
}

bool _safePayload(String operation, Map<String, Object?> payload) {
  final expected = switch (operation) {
    _datasetListOperation => const {'版'},
    _experimentStartOperation => const {'版', '評価DatasetID', '対象Runtime一覧'},
    _experimentStatusOperation ||
    _comparisonOperation => const {'版', '評価ExperimentID'},
    _ => const <String>{},
  };
  if (payload.length != expected.length ||
      !expected.every(payload.containsKey)) {
    return false;
  }
  if (payload['版'] != 1) return false;
  return switch (operation) {
    _datasetListOperation => true,
    _experimentStartOperation =>
      _isOpaqueId(payload['評価DatasetID']) &&
          EvaluationClient.validRuntimeIds(payload['対象Runtime一覧']),
    _experimentStatusOperation ||
    _comparisonOperation => _isOpaqueId(payload['評価ExperimentID']),
    _ => false,
  };
}

Map<String, Object?> _object(Object? raw) {
  if (raw is! Map) _reject();
  final result = <String, Object?>{};
  for (final entry in raw.entries) {
    if (entry.key is! String || result.containsKey(entry.key)) _reject();
    result[entry.key as String] = entry.value;
  }
  return result;
}

void _exactKeys(Map<String, Object?> value, Set<String> keys) {
  if (value.length != keys.length || !keys.every(value.containsKey)) _reject();
}

void _rejectForbiddenKeys(Object? raw, Set<String> forbidden) {
  if (raw is Map) {
    for (final entry in raw.entries) {
      if (entry.key is! String ||
          forbidden.contains(_normalizedKey(entry.key as String))) {
        _reject();
      }
      _rejectForbiddenKeys(entry.value, forbidden);
    }
  } else if (raw is List) {
    for (final value in raw) {
      _rejectForbiddenKeys(value, forbidden);
    }
  }
}

String _normalizedKey(String key) =>
    key.toLowerCase().replaceAll(RegExp(r'[ _-]'), '');

bool _isOpaqueId(Object? value) =>
    value is String && value.length == 32 && _opaqueId.hasMatch(value);

bool _isAuditId(Object? value) =>
    value is String && value.length <= 256 && _auditId.hasMatch(value);

bool _isHash(Object? value) =>
    value is String && value.length == 71 && _hash.hasMatch(value);

bool _timestampMillis(Object? value) => value is int && value >= 0;

bool _nonNegativeInt(Object? value) => value is int && value >= 0;

bool _positiveSafeInteger(Object? value) =>
    value is int && value >= 1 && value <= _maximumSafeInteger;

bool _safeText(Object? value, {required int maxLength}) =>
    value is String &&
    value.isNotEmpty &&
    value.length <= maxLength &&
    !value.runes.any((codePoint) => codePoint < 32 || codePoint == 127);

bool _difference(Object? value) =>
    value is String && const {'same', 'different', 'unknown'}.contains(value);

bool _sameStrings(List<String> left, List<String> right) =>
    left.length == right.length &&
    left.asMap().entries.every((entry) => entry.value == right[entry.key]);

Never _reject() => throw const BrokerClientException('評価ラボの応答を確認できません');
