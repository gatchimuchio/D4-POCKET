import 'broker_client.dart';

/// 実行系対話の表示用client。承認・表示資格の発行や実行系への通信は行わない。
class RuntimeDialogueClient {
  RuntimeDialogueClient(this.transport);
  final BrokerTransport transport;

  static Future<RuntimeDialogueClient> connect() async =>
      RuntimeDialogueClient(await BrokerClient.connect());

  Future<Map<String, Object?>> operation(
      String operation, Map<String, Object?> payload) async {
    const allowed = {'実行系列挙', '対話開始', '対話送信', '対話取得', '対話中止', '対話終了'};
    if (!allowed.contains(operation)) {
      throw const BrokerClientException('この操作面では承認操作を送信できません');
    }
    final response = await transport.request(operation, payload: payload);
    if (response['operation'] != operation ||
        response['audit_event_id'] is! String ||
        (response['audit_event_id'] as String).isEmpty) {
      throw const BrokerClientException('対話応答の対応または監査IDが不正です');
    }
    if (response['status'] != 'accepted') {
      final error = response['error'];
      final code = error is Map ? error['code'] : null;
      // 外部の任意メッセージをそのまま画面へ反映しない。
      const failures = {
        '要求不正',
        '実行系不在',
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
        items.any((v) => v is! String || !_runtime.hasMatch(v))) {
      throw const BrokerClientException('実行系列挙の形式が不正です');
    }
    return List<String>.unmodifiable(items.cast<String>());
  }

  Future<String> start(String runtime) async {
    final body = await operation('対話開始', {'実行系ID': runtime});
    if (body['実行系ID'] != runtime ||
        body['状態'] != '利用中' ||
        !validId(body['対話セッションID'])) {
      throw const BrokerClientException('新規セッションの対応が不正です');
    }
    return body['対話セッションID']! as String;
  }

  Future<String> send(String session, String input) async {
    if (input.trim().isEmpty || input.runes.length > 4096) {
      throw const BrokerClientException('入力は空白以外の1〜4096文字にしてください');
    }
    final body = await operation('対話送信', {'対話セッションID': session, '入力': input});
    if (!validId(body['要求ID']) || body['状態'] != '承認待ち') {
      throw const BrokerClientException('送信要求の対応が不正です');
    }
    return body['要求ID']! as String;
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
    if (state == '承認待ち' || state == '実行中') {
      if (result != null) throw const BrokerClientException('未確定の本文は表示できません');
      return DialogueProgress(state! as String, null);
    }
    if (result is! Map) throw const BrokerClientException('確定応答がありません');
    return DialogueProgress(
        state! as String,
        DialogueResult.parse(
            Map<String, Object?>.from(result), request, runtime, session));
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
  static final _runtime = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$');
  static bool validId(Object? value) =>
      value is String && value.length == 32 && _id.hasMatch(value);
}

class DialogueProgress {
  const DialogueProgress(this.state, this.result);
  final String state;
  final DialogueResult? result;
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
    return DialogueResult._(Map.unmodifiable(fields));
  }
}
