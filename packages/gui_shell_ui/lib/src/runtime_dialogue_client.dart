import 'broker_transport.dart';

/// 実行系対話の表示用client。承認・表示資格の発行や実行系への通信は行わない。
class RuntimeDialogueClient {
  RuntimeDialogueClient(this.transport);
  final BrokerTransport transport;

  Future<Map<String, Object?>> _request(
      String operation, Map<String, Object?> payload) async {
    const allowed = {'実行系列挙', '作業領域一覧', '対話開始', '対話送信', '対話取得', '対話中止', '対話終了'};
    if (!allowed.contains(operation)) {
      throw const BrokerClientException('この操作面では承認操作を送信できません');
    }
    final response = await transport.request(operation, payload: payload);
    if (response['operation'] != operation ||
        response['audit_event_id'] is! String ||
        (response['audit_event_id'] as String).isEmpty) {
      throw const BrokerClientException('対話応答の対応または監査IDが不正です');
    }
    return response;
  }

  Future<Map<String, Object?>> operation(
      String operation, Map<String, Object?> payload) async {
    final response = await _request(operation, payload);
    if (response['status'] != 'accepted') {
      final error = response['error'];
      final code = error is Map ? error['code'] : null;
      // 外部の任意メッセージをそのまま画面へ反映しない。
      const failures = {
        '要求不正',
        '実行系不在',
        '作業領域不在',
        '権限拒否',
        'セッション不一致',
        '通信失敗',
        '期限超過',
        '応答不正',
        '監査失敗',
        '取消'
      };
      throw BrokerClientException(failures.contains(code)
          ? '操作を拒否しました: $code'
          : 'brokerが操作を拒否または保留しました');
    }
    if (response['body'] is! Map) {
      throw const BrokerClientException('対話応答の本文が不正です');
    }
    return Map<String, Object?>.from(response['body'] as Map);
  }

  Future<List<String>> runtimes() async {
    final body = await operation('実行系列挙', {});
    final items = body['実行系'];
    if (items is! List ||
        items.length > 128 ||
        items.any((v) => v is! String || !_runtime.hasMatch(v)) ||
        items.toSet().length != items.length) {
      throw const BrokerClientException('実行系列挙の形式が不正です');
    }
    return List<String>.unmodifiable(items.cast<String>());
  }

  Future<Map<String, List<String>>> workspaceIdsByRuntime() async {
    final response = await _request('作業領域一覧', const {});
    if (response['status'] != 'accepted' ||
        response['error'] != null ||
        response['evidence_source'] != 'INTERNAL_STATE') {
      throw const BrokerClientException('登録Workspace一覧を確認できません');
    }
    final body = response['body'];
    if (body is! Map ||
        body.length != 1 ||
        !body.containsKey('作業領域') ||
        body['作業領域'] is! List ||
        (body['作業領域'] as List).length > 16) {
      throw const BrokerClientException('登録Workspace一覧の形式が不正です');
    }
    final result = <String, List<String>>{};
    final seenWorkspaceIds = <String>{};
    for (final raw in body['作業領域'] as List) {
      if (raw is! Map || raw.keys.any((key) => key is! String)) {
        throw const BrokerClientException('登録Workspace項目の形式が不正です');
      }
      final item = Map<String, Object?>.from(raw);
      const keys = {
        '作業領域ID',
        '実行系ID',
        '登録hash',
        '承認状態',
        '有効期限',
        '表示範囲',
        'approval_id'
      };
      final id = item['作業領域ID'];
      final runtime = item['実行系ID'];
      final hash = item['登録hash'];
      final status = item['承認状態'];
      final visibility = item['表示範囲'];
      if (item.length != keys.length ||
          !keys.every(item.containsKey) ||
          id is! String ||
          !_runtime.hasMatch(id) ||
          runtime is! String ||
          !_runtime.hasMatch(runtime) ||
          hash is! String ||
          !_hash.hasMatch(hash) ||
          !seenWorkspaceIds.add(id)) {
        throw const BrokerClientException('登録Workspace識別情報が不正です');
      }
      if (status == 'approved') {
        if (item['approval_id'] is! String ||
            (item['approval_id'] as String).isEmpty ||
            item['有効期限'] is! int ||
            (item['有効期限'] as int) <= 0 ||
            !_workspaceVisibilities.contains(visibility)) {
          throw const BrokerClientException('Workspace承認metadataが不正です');
        }
      } else if (status != 'denied' ||
          item['approval_id'] != null ||
          item['有効期限'] != null ||
          visibility != 'none') {
        throw const BrokerClientException('Workspace登録状態が不正です');
      }
      if (result[runtime] == null) result[runtime] = <String>[];
      result[runtime]!.add(id);
    }
    return Map.unmodifiable({
      for (final entry in result.entries)
        entry.key: List<String>.unmodifiable(entry.value)
    });
  }

  Future<String> start(String runtime, {String? workspaceId}) async {
    if (workspaceId != null && !_runtime.hasMatch(workspaceId)) {
      throw const BrokerClientException('Workspace IDの形式が不正です');
    }
    final body = await operation('対話開始', {
      '実行系ID': runtime,
      if (workspaceId != null) '作業領域ID': workspaceId,
    });
    if (body['実行系ID'] != runtime ||
        body['状態'] != '利用中' ||
        !validId(body['対話セッションID'])) {
      throw const BrokerClientException('新規セッションの対応が不正です');
    }
    return body['対話セッションID']! as String;
  }

  Future<String> send(String session, String input) async {
    return (await sendReference(session, input)).requestId;
  }

  Future<DialogueRequestReference> sendReference(
      String session, String input) async {
    if (input.trim().isEmpty || input.runes.length > 4096) {
      throw const BrokerClientException('入力は空白以外の1〜4096文字にしてください');
    }
    final body = await operation('対話送信', {'対話セッションID': session, '入力': input});
    final requestHash = body['要求hash'];
    final expiresAt = body['期限'];
    if (!validId(body['要求ID']) ||
        requestHash is! String ||
        !_hash.hasMatch(requestHash) ||
        body['状態'] != '承認待ち' ||
        !_safeInteger(expiresAt) ||
        body.length != 4 ||
        !const {'要求ID', '要求hash', '状態', '期限'}.containsAll(body.keys)) {
      throw const BrokerClientException('送信要求の対応が不正です');
    }
    return DialogueRequestReference(body['要求ID']! as String, requestHash);
  }

  Future<DialogueProgress> poll(
      String request, String runtime, String session) async {
    final body = await operation('対話取得', {'要求ID': request});
    final state = body['状態'];
    if (body['要求ID'] != request ||
        !{'承認待ち', '実行中', '完了', '中止'}.contains(state)) {
      throw const BrokerClientException('取得応答の要求対応が不正です');
    }
    final result = body['結果'];
    final record = body.containsKey('実行記録')
        ? DialogueExecutionRecord.parse(body['実行記録'], request, runtime, session)
        : null;
    if (state == '承認待ち' || state == '実行中') {
      if (result != null) throw const BrokerClientException('未確定の本文は表示できません');
      return DialogueProgress(state! as String, null, record: record);
    }
    if (result is! Map) throw const BrokerClientException('確定応答がありません');
    final parsed = DialogueResult.parse(
        Map<String, Object?>.from(result), request, runtime, session);
    if (state == '中止' && parsed.text('状態') != '中止') {
      throw const BrokerClientException('中止進捗と対話結果の状態が一致しません');
    }
    return DialogueProgress(state! as String, parsed, record: record);
  }

  Future<void> cancel(String request) async {
    final body = await operation('対話中止', {'要求ID': request});
    if (body['要求ID'] != request || body['状態'] != '中止') {
      throw const BrokerClientException('中止を確認できません');
    }
  }

  Future<void> close(String session) async {
    final body = await operation('対話終了', {'対話セッションID': session});
    if (body['対話セッションID'] != session || body['状態'] != '終了') {
      throw const BrokerClientException('セッション終了を確認できません');
    }
  }

  static final _id = RegExp(r'^[a-f0-9]{32}$');
  static final _hash = RegExp(r'^sha256:[a-f0-9]{64}$');
  static const _workspaceVisibilities = {
    'none',
    'hash_only',
    'summary',
    'redacted',
    'full'
  };
  static final _runtime = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$');
  static bool validId(Object? value) =>
      value is String && value.length == 32 && _id.hasMatch(value);
  static bool _safeInteger(Object? value) =>
      value is int && value >= 0 && value <= 9007199254740991;
}

class DialogueRequestReference {
  const DialogueRequestReference(this.requestId, this.requestHash);
  final String requestId;
  final String requestHash;
}

class DialogueProgress {
  const DialogueProgress(this.state, this.result, {this.record});
  final String state;
  final DialogueResult? result;
  final DialogueExecutionRecord? record;
}

class DialogueExecutionRecord {
  DialogueExecutionRecord._(this.fields);
  final Map<String, Object?> fields;

  String time(String key) {
    final value = fields[key] as int?;
    return value == null
        ? '未記録'
        : DateTime.fromMillisecondsSinceEpoch(value * 1000, isUtc: true)
            .toIso8601String();
  }

  String audit(String key) => fields[key] as String? ?? '未記録';

  factory DialogueExecutionRecord.parse(
      Object? value, String request, String runtime, String session) {
    const keys = {
      '要求ID',
      '実行系ID',
      '対話セッションID',
      '作成時刻',
      '開始時刻',
      '終了時刻',
      '作成監査ID',
      '開始監査ID',
      '終了監査ID'
    };
    void reject() => throw const BrokerClientException('実行記録の構造または要求対応が不正です');
    if (value is! Map ||
        value.length != keys.length ||
        !keys.containsAll(value.keys) ||
        value['要求ID'] != request ||
        value['実行系ID'] != runtime ||
        value['対話セッションID'] != session) {
      reject();
    }
    final fields = Map<String, Object?>.from(value as Map);
    for (final stage in ['作成', '開始', '終了']) {
      final time = fields['$stage時刻'];
      final audit = fields['$stage監査ID'];
      if (time == null && audit == null && stage != '作成') continue;
      if (time is! int ||
          time < -8640000000000 ||
          time > 8640000000000 ||
          audit is! String ||
          audit.isEmpty ||
          audit.length > 256 ||
          audit.runes.any((c) => c < 32 || c == 127)) {
        reject();
      }
    }
    return DialogueExecutionRecord._(Map.unmodifiable(fields));
  }
}

class DialogueResult {
  DialogueResult._(this.fields);
  final Map<String, Object?> fields;
  String text(String key) => fields[key]! as String;
  List<String> list(String key) => (fields[key]! as List).cast<String>();

  factory DialogueResult.parse(Map<String, Object?> fields, String request,
      String runtime, String session) {
    const keys = {
      '要求ID',
      '実行系ID',
      '対話セッションID',
      '状態',
      '表示範囲',
      '本文',
      '参照',
      '能力',
      '経路',
      '追跡ID',
      '追跡hash',
      '応答hash',
      '失敗分類',
      '復旧'
    };
    void reject() => throw const BrokerClientException('対話結果の構造・対応・表示境界が不正です');
    if (fields.length != keys.length ||
        !keys.containsAll(fields.keys) ||
        fields['要求ID'] != request ||
        fields['実行系ID'] != runtime ||
        fields['対話セッションID'] != session) {
      reject();
    }
    for (final key in keys.difference({'参照', '能力'})) {
      if (fields[key] is! String) {
        reject();
      }
    }
    final scope = fields['表示範囲'];
    if (!{'none', 'hash_only', 'summary', 'redacted', 'full'}.contains(scope) ||
        !{'成功', '保留', '失敗', '中止'}.contains(fields['状態'])) {
      reject();
    }
    for (final entry in {
      '本文': 65536,
      '経路': 256,
      '追跡ID': 32,
      '追跡hash': 71,
      '応答hash': 71
    }.entries) {
      if ((fields[entry.key]! as String).runes.length > entry.value) {
        reject();
      }
    }
    for (final key in ['参照', '能力']) {
      final values = fields[key];
      if (values is! List ||
          values.length > 64 ||
          values.any((v) =>
              v is! String || v.runes.length > (key == '参照' ? 2048 : 256))) {
        reject();
      }
    }
    if (scope != 'full' &&
        (['本文', '経路', '追跡ID', '追跡hash'].any((k) => fields[k] != '') ||
            (fields['参照']! as List).isNotEmpty ||
            (fields['能力']! as List).isNotEmpty)) {
      reject();
    }
    final trace = fields['追跡ID']! as String;
    if (trace.isNotEmpty && !RuntimeDialogueClient.validId(trace)) {
      reject();
    }
    for (final key in ['追跡hash', '応答hash']) {
      final hash = fields[key]! as String;
      if (hash.isNotEmpty &&
          (hash.length != 71 ||
              !RegExp(r'^sha256:[a-f0-9]{64}$').hasMatch(hash))) {
        reject();
      }
    }
    if (scope == 'none' && fields['応答hash'] != '') {
      reject();
    }
    if (!{
          '',
          '要求不正',
          '実行系不在',
          '権限拒否',
          'セッション不一致',
          '通信失敗',
          '期限超過',
          '応答不正',
          '監査失敗',
          '取消'
        }.contains(fields['失敗分類']) ||
        !{'', '入力修正', '実行系再確認', '権限再確認', '新規セッション', '接続再確認', '監査修復'}
            .contains(fields['復旧'])) {
      reject();
    }
    if ({'失敗', '中止'}.contains(fields['状態']) &&
        (fields['失敗分類'] == '' || fields['復旧'] == '' || fields['本文'] != '')) {
      reject();
    }
    return DialogueResult._(Map.unmodifiable({
      ...fields,
      for (final key in ['参照', '能力'])
        key: List<String>.unmodifiable((fields[key]! as List).cast<String>()),
    }));
  }
}
