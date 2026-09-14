import 'broker_transport.dart';

const _runtimeResourceOperation = '実行系資源観測';
const _metricKeys = <String>[
  '稼働時間Millis',
  'CPU累積時間Millis',
  'CPU利用率Percent',
  'RAMWorkingSetBytes',
  'RAMPrivateBytes',
  'DiskIOBytes',
  'NetworkIOBytes',
  'GPU利用率Percent',
  'VRAMBytes',
  '処理中要求数',
  '平均応答Millis',
  '失敗要求数',
];

const _historyMetricKeys = <String>[
  'CPU利用率Percent',
  'RAMWorkingSetBytes',
  'NetworkIOBytes',
  '平均応答Millis',
  'エラー率Percent',
];

const _percentMetricKeys = <String>{
  'CPU利用率Percent',
  'GPU利用率Percent',
  'エラー率Percent',
};

final _runtimeId = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$');
final _auditId = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.-]{0,255}$');

/// Broker所有のread-only資源観測だけを受け取る表示用client。
/// PID、接続先、能力、権限、承認、履歴長を要求から受け取らない。
class RuntimeResourceClient {
  RuntimeResourceClient(this.transport);

  final BrokerTransport transport;

  static bool validRuntimeId(Object? value) =>
      value is String && _runtimeId.hasMatch(value);

  Future<RuntimeResourceObservation> observe(String runtimeId) async {
    if (!validRuntimeId(runtimeId)) {
      _reject();
    }
    final response = await transport.request(
      _runtimeResourceOperation,
      payload: {'版': 1, '実行系ID': runtimeId},
    );
    final envelope = _object(response);
    _exactKeys(envelope, const {
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
    final responseAuditId = envelope['audit_event_id'];
    if (!_safeText(envelope['request_id'], maxLength: 256) ||
        envelope['operation'] != _runtimeResourceOperation ||
        envelope['status'] != 'accepted' ||
        !_identifier(responseAuditId) ||
        envelope['error'] != null ||
        envelope['health'] != null ||
        envelope['shutdown_requested'] != false ||
        !_evidenceSource(envelope['evidence_source'])) {
      _reject();
    }

    final observation = RuntimeResourceObservation._parse(
      envelope['body'],
      runtimeId,
      responseAuditId as String,
    );
    final expectedEvidence = observation.binding.isBound
        ? 'LIVE_RUNTIME'
        : 'INTERNAL_STATE';
    if (envelope['evidence_source'] != expectedEvidence) {
      _reject();
    }
    return observation;
  }
}

/// 監査付きの一回の資源観測。過去のsnapshotやUI状態から生成しない。
class RuntimeResourceObservation {
  RuntimeResourceObservation._(
    this.runtimeId,
    this.observedAtUnixMillis,
    this.auditId,
    this.binding,
    this.governance,
    Map<String, RuntimeResourceMetric> metrics,
    List<RuntimeResourceHistorySample> history,
  )   : metrics = Map.unmodifiable(metrics),
        history = List.unmodifiable(history);

  final String runtimeId;
  final int observedAtUnixMillis;
  final String auditId;
  final RuntimeResourceBinding binding;
  final RuntimeResourceGovernance governance;
  final Map<String, RuntimeResourceMetric> metrics;
  final List<RuntimeResourceHistorySample> history;

  RuntimeResourceMetric metric(String key) {
    final result = metrics[key];
    if (result == null) {
      _reject();
    }
    return result;
  }

  static RuntimeResourceObservation _parse(
    Object? raw,
    String requestedRuntimeId,
    String responseAuditId,
  ) {
    final body = _object(raw);
    _exactKeys(body, const {
      '版',
      '実行系ID',
      '観測時刻UnixMillis',
      '観測監査ID',
      '結合',
      '統治',
      '計測',
      '短期履歴',
    });
    final observationAuditId = body['観測監査ID'];
    if (body['版'] != 1 ||
        body['実行系ID'] != requestedRuntimeId ||
        !_timestampMillis(body['観測時刻UnixMillis']) ||
        observationAuditId != responseAuditId ||
        !_identifier(observationAuditId)) {
      _reject();
    }
    final binding = RuntimeResourceBinding._parse(body['結合']);
    final governance = RuntimeResourceGovernance._parse(body['統治']);
    final rawMetrics = _object(body['計測']);
    _exactKeys(rawMetrics, _metricKeys.toSet());
    final metrics = <String, RuntimeResourceMetric>{
      for (final key in _metricKeys)
        key: RuntimeResourceMetric._parse(
          rawMetrics[key],
          percent: _percentMetricKeys.contains(key),
        ),
    };

    final rawHistory = body['短期履歴'];
    if (rawHistory is! List || rawHistory.length > 60) {
      _reject();
    }
    final history = rawHistory
        .map(RuntimeResourceHistorySample._parse)
        .toList(growable: false);
    _validateCurrentC3EvidenceBoundary(binding, metrics, history);
    return RuntimeResourceObservation._(
      requestedRuntimeId,
      body['観測時刻UnixMillis'] as int,
      responseAuditId,
      binding,
      governance,
      metrics,
      history,
    );
  }

  /// C3で接続済みの観測根拠だけを受理する。構造Schemaの許容範囲を、
  /// 未接続metricを実測値へ見せる根拠に使わない。
  static void _validateCurrentC3EvidenceBoundary(
    RuntimeResourceBinding binding,
    Map<String, RuntimeResourceMetric> metrics,
    List<RuntimeResourceHistorySample> history,
  ) {
    if (!binding.isBound) {
      if (history.isNotEmpty ||
          metrics.values.any(
            (metric) =>
                metric.isMeasured || metric.evidenceSource != 'INTERNAL_STATE',
          )) {
        _reject();
      }
      return;
    }

    for (final key in const <String>['RAMWorkingSetBytes', 'RAMPrivateBytes']) {
      _requireMetric(
        metrics[key]!,
        evidenceSource: 'LIVE_RUNTIME',
        measured: true,
      );
    }
    for (final key in const <String>[
      '稼働時間Millis',
      'CPU累積時間Millis',
      'CPU利用率Percent',
    ]) {
      _requireMetric(metrics[key]!, evidenceSource: 'LIVE_RUNTIME');
    }

    for (final key in const <String>[
      'DiskIOBytes',
      'NetworkIOBytes',
      'GPU利用率Percent',
      'VRAMBytes',
      '平均応答Millis',
    ]) {
      _requireMetric(
        metrics[key]!,
        evidenceSource: 'INTERNAL_STATE',
        measured: false,
      );
    }
    for (final key in const <String>['処理中要求数', '失敗要求数']) {
      _requireMetric(
        metrics[key]!,
        evidenceSource: 'INTERNAL_STATE',
        measured: true,
      );
    }

    for (final sample in history) {
      _requireMetric(
        sample.metric('CPU利用率Percent'),
        evidenceSource: 'LIVE_RUNTIME',
      );
      _requireMetric(
        sample.metric('RAMWorkingSetBytes'),
        evidenceSource: 'LIVE_RUNTIME',
        measured: true,
      );
      _requireMetric(
        sample.metric('NetworkIOBytes'),
        evidenceSource: 'INTERNAL_STATE',
        measured: false,
      );
      _requireMetric(
        sample.metric('平均応答Millis'),
        evidenceSource: 'INTERNAL_STATE',
        measured: false,
      );
      _requireMetric(
        sample.metric('エラー率Percent'),
        evidenceSource: 'INTERNAL_STATE',
      );
    }
  }

  static void _requireMetric(
    RuntimeResourceMetric metric, {
    required String evidenceSource,
    bool? measured,
  }) {
    if (metric.evidenceSource != evidenceSource ||
        (measured != null && metric.isMeasured != measured)) {
      _reject();
    }
  }
}

/// Brokerが固定したprocess binding。PIDは入力値でなくloopback listenerの所有者だけである。
class RuntimeResourceBinding {
  const RuntimeResourceBinding._(
    this.status,
    this.basis,
    this.pid,
    this.processCreationAtUnixMillis,
    this.registeredAtUnixMillis,
    this.registrationAuditId,
    this.reason,
  );

  final String status;
  final String basis;
  final int? pid;
  final int? processCreationAtUnixMillis;
  final int registeredAtUnixMillis;
  final String registrationAuditId;
  final String? reason;

  bool get isBound => status == 'bound';

  static RuntimeResourceBinding _parse(Object? raw) {
    final value = _object(raw);
    const common = {
      '状態',
      '根拠',
      'PID',
      'PID作成時刻UnixMillis',
      '登録時刻UnixMillis',
      '登録監査ID',
    };
    final status = value['状態'];
    if (status == 'bound') {
      _exactKeys(value, common);
      final registrationAuditId = value['登録監査ID'];
      if (value['根拠'] != 'loopback_tcp_listener_owner_pid' ||
          value['PID'] is! int ||
          (value['PID'] as int) < 1 ||
          !_timestampMillis(value['PID作成時刻UnixMillis']) ||
          !_timestampMillis(value['登録時刻UnixMillis']) ||
          !_identifier(registrationAuditId)) {
        _reject();
      }
      return RuntimeResourceBinding._(
        status as String,
        value['根拠'] as String,
        value['PID'] as int,
        value['PID作成時刻UnixMillis'] as int,
        value['登録時刻UnixMillis'] as int,
        registrationAuditId as String,
        null,
      );
    }

    _exactKeys(value, {...common, '理由'});
    final unsupported = status == 'unsupported';
    final registrationAuditId = value['登録監査ID'];
    if (!{'unbound', 'binding_mismatch', 'exited', 'unsupported'}
            .contains(status) ||
        value['根拠'] !=
            (unsupported ? 'unsupported' : 'loopback_tcp_listener_owner_pid') ||
        value['PID'] != null ||
        value['PID作成時刻UnixMillis'] != null ||
        !_timestampMillis(value['登録時刻UnixMillis']) ||
        !_identifier(registrationAuditId) ||
        !_safeText(value['理由'], maxLength: 256)) {
      _reject();
    }
    return RuntimeResourceBinding._(
      status as String,
      value['根拠'] as String,
      null,
      null,
      value['登録時刻UnixMillis'] as int,
      registrationAuditId as String,
      value['理由'] as String,
    );
  }
}

/// read-only資源観測にBrokerが固定した統治対応。UI入力から作らない。
class RuntimeResourceGovernance {
  const RuntimeResourceGovernance._(
    this.capabilityId,
    this.permissionId,
    this.approvalState,
    this.recoveryId,
  );

  final String capabilityId;
  final String permissionId;
  final String approvalState;
  final String recoveryId;

  static RuntimeResourceGovernance _parse(Object? raw) {
    final value = _object(raw);
    _exactKeys(value, const {'能力ID', '権限ID', '承認状態', '復旧ID'});
    if (value['能力ID'] != 'runtime.resource.observe' ||
        value['権限ID'] != 'permission.runtime.resource.observe' ||
        value['承認状態'] != 'not_required' ||
        value['復旧ID'] != 'recover-runtime-resource-binding') {
      _reject();
    }
    return RuntimeResourceGovernance._(
      value['能力ID'] as String,
      value['権限ID'] as String,
      value['承認状態'] as String,
      value['復旧ID'] as String,
    );
  }
}

/// 計測値または明示的な未取得理由。unknownを数値0へ置き換えない。
class RuntimeResourceMetric {
  const RuntimeResourceMetric._(
    this.state,
    this.value,
    this.evidenceSource,
    this.reason,
  );

  final String state;
  final num? value;
  final String evidenceSource;
  final String? reason;

  bool get isMeasured => state == 'measured';

  static RuntimeResourceMetric _parse(Object? raw, {required bool percent}) {
    final value = _object(raw);
    final state = value['状態'];
    if (state == 'measured') {
      _exactKeys(value, const {'状態', '値', '証拠種別'});
      final measured = value['値'];
      if (!_evidenceSource(value['証拠種別']) ||
          measured is! num ||
          !measured.isFinite ||
          measured < 0 ||
          (percent && measured > 100)) {
        _reject();
      }
      return RuntimeResourceMetric._(
        'measured',
        measured,
        value['証拠種別'] as String,
        null,
      );
    }
    if (state != 'unknown') {
      _reject();
    }
    _exactKeys(value, const {'状態', '値', '証拠種別', '理由'});
    if (value['値'] != null ||
        !_evidenceSource(value['証拠種別']) ||
        !_safeText(value['理由'], maxLength: 256)) {
      _reject();
    }
    return RuntimeResourceMetric._(
      'unknown',
      null,
      value['証拠種別'] as String,
      value['理由'] as String,
    );
  }
}

/// Broker内の短期履歴の一点。永続監査や現在の権限を示さない。
class RuntimeResourceHistorySample {
  RuntimeResourceHistorySample._(
    this.observedAtUnixMillis,
    Map<String, RuntimeResourceMetric> metrics,
  ) : metrics = Map.unmodifiable(metrics);

  final int observedAtUnixMillis;
  final Map<String, RuntimeResourceMetric> metrics;

  RuntimeResourceMetric metric(String key) {
    final result = metrics[key];
    if (result == null) {
      _reject();
    }
    return result;
  }

  static RuntimeResourceHistorySample _parse(Object? raw) {
    final value = _object(raw);
    _exactKeys(value, {'観測時刻UnixMillis', ..._historyMetricKeys});
    if (!_timestampMillis(value['観測時刻UnixMillis'])) {
      _reject();
    }
    return RuntimeResourceHistorySample._(
      value['観測時刻UnixMillis'] as int,
      {
        for (final key in _historyMetricKeys)
          key: RuntimeResourceMetric._parse(
            value[key],
            percent: _percentMetricKeys.contains(key),
          ),
      },
    );
  }
}

Map<String, Object?> _object(Object? raw) {
  if (raw is! Map) {
    _reject();
  }
  final result = <String, Object?>{};
  for (final entry in raw.entries) {
    if (entry.key is! String) {
      _reject();
    }
    result[entry.key as String] = entry.value;
  }
  return result;
}

void _exactKeys(Map<String, Object?> value, Set<String> keys) {
  if (value.length != keys.length || !keys.every(value.containsKey)) {
    _reject();
  }
}

bool _evidenceSource(Object? value) =>
    value == 'LIVE_RUNTIME' || value == 'INTERNAL_STATE';

bool _identifier(Object? value) => value is String && _auditId.hasMatch(value);

bool _timestampMillis(Object? value) => value is int && value >= 0;

bool _safeText(Object? value, {required int maxLength}) =>
    value is String &&
    value.isNotEmpty &&
    value.length <= maxLength &&
    !value.runes.any((codePoint) => codePoint < 32 || codePoint == 127);

Never _reject() =>
    throw const BrokerClientException('実行系資源観測の応答を確認できません');
