import 'broker_transport.dart';
import 'runtime_dialogue_client.dart';

/// 現在承認に限定する履歴client。承認発行操作を公開しない。
class HistoryClient {
  HistoryClient(this.transport);
  final BrokerTransport transport;
  Future<Map> _call(String op, Map<String, Object?> payload) async {
    final r = await transport.request(op, payload: payload);
    if (r['operation'] != op ||
        r['status'] != 'accepted' ||
        r['evidence_source'] != 'INTERNAL_STATE' ||
        !_text(r['audit_event_id'])) {
      _reject();
    }
    return _shape(r['body'], {'grant', 'page'});
  }

  Future<Map> _contentCall(String op, Map<String, Object?> payload) async {
    final r = await transport.request(op, payload: payload);
    if (r['operation'] != op ||
        r['status'] != 'accepted' ||
        r['evidence_source'] != 'INTERNAL_STATE' ||
        !_text(r['audit_event_id'])) {
      _reject();
    }
    return _shape(r['body'], {'grant', 'content'});
  }

  Future<HistoryContentGrant?> contentStatus() async {
    final body = await _contentCall('対話内容閲覧状態', {});
    if (body['content'] != null) _reject();
    return body['grant'] == null
        ? null
        : HistoryContentGrant.parse(body['grant']);
  }

  Future<HistoryContent> content(
      HistoryEntry entry, HistoryContentGrant grant) async {
    if (!grant.current || !grant.matches(entry)) _reject();
    final before = await contentStatus();
    if (before == null || !grant.same(before) || !grant.current) _reject();
    final body = await _contentCall('対話内容閲覧', {'approval_id': grant.id});
    final returned = HistoryContentGrant.parse(body['grant']);
    if (!grant.same(returned) || !grant.current) _reject();
    final m =
        _shape(body['content'], {'版', '要求', '要求hash', '結果', '実行記録', '結果証跡'});
    final request = _shape(m['要求'], {'要求ID', '実行系ID', '対話セッションID', '入力'});
    final record = _shape(m['実行記録'], entry.record.fields.keys.toSet());
    if (m['版'] != 1 ||
        m['要求hash'] != entry.context.requestHash ||
        request['入力'] is! String ||
        (request['入力'] as String).isEmpty ||
        (request['入力'] as String).runes.length > 4096) {
      _reject();
    }
    for (final key in entry.record.fields.keys) {
      if (record[key] != entry.record.fields[key]) _reject();
    }
    for (final key in ['要求ID', '実行系ID', '対話セッションID']) {
      if (request[key] != entry.record.fields[key]) _reject();
    }
    final proof = HistoryResultEvidence.parse(
        m['結果証跡'], entry.record, entry.context, entry.state);
    if (proof == null ||
        entry.evidence == null ||
        proof.responseHash != entry.evidence!.responseHash ||
        proof.capabilityHash != entry.evidence!.capabilityHash ||
        proof.routeHash != entry.evidence!.routeHash ||
        proof.traceHash != entry.evidence!.traceHash) {
      _reject();
    }
    if (m['結果'] is! Map) _reject();
    final result = DialogueResult.parse(
        (m['結果'] as Map).cast<String, Object?>(),
        request['要求ID'] as String,
        request['実行系ID'] as String,
        request['対話セッションID'] as String);
    if (result.text('表示範囲') != 'full' ||
        result.text('状態') != entry.state ||
        result.text('応答hash') != proof.responseHash ||
        result.text('失敗分類') != '' ||
        result.text('復旧') != '') {
      _reject();
    }
    final after = await contentStatus();
    if (after == null || !grant.same(after) || !grant.current) _reject();
    return HistoryContent(request['入力'] as String, result, grant);
  }

  Future<HistoryGrant?> status() async {
    final body = await _call('対話履歴閲覧状態', {});
    if (body['page'] != null) {
      _reject();
    }
    return body['grant'] == null ? null : HistoryGrant.parse(body['grant']);
  }

  Future<HistoryPage> page(HistoryGrant grant,
      {int after = 0,
      String? state,
      String? requestId,
      String? sessionId}) async {
    if (!grant.current ||
        after < 0 ||
        (requestId != null && !RuntimeDialogueClient.validId(requestId)) ||
        (sessionId != null && !RuntimeDialogueClient.validId(sessionId))) {
      _reject();
    }
    final body = await _call('対話履歴閲覧', {
      'approval_id': grant.id,
      'query': {
        'after': after,
        'limit': 50,
        'latest_per_request': true,
        'include_audit_context': true,
        'include_result_evidence': true,
        'include_content_receipt': true,
        'filter': {
          '実行系ID': grant.runtime,
          if (state != null) '状態': state,
          if (requestId != null) '要求ID': requestId,
          if (sessionId != null) '対話セッションID': sessionId
        }
      }
    });
    final returned = HistoryGrant.parse(body['grant']);
    if (!grant.same(returned) || !grant.current) {
      _reject();
    }
    final page = _shape(body['page'],
        {'version', 'entries', 'next_cursor', 'has_more', 'head_hash'});
    final entries = page['entries'];
    final next = page['next_cursor'];
    final more = page['has_more'];
    if (page['version'] != 1 ||
        entries is! List ||
        entries.length > 50 ||
        next is! int ||
        next < after ||
        more is! bool ||
        (more && (next <= after || entries.isEmpty)) ||
        (page['head_hash'] != null && !_hash(page['head_hash']))) {
      _reject();
    }
    final parsed = <HistoryEntry>[];
    final ids = <String>{};
    final requests = <String>{};
    for (final raw in entries) {
      final e = _shape(raw, {
        'audit_event_id',
        'event_hash',
        'record',
        'audit_context',
        'result_evidence',
        'content_receipt'
      });
      if (!_text(e['audit_event_id']) ||
          !_hash(e['event_hash']) ||
          !ids.add(e['audit_event_id'] as String)) {
        _reject();
      }
      final saved = _shape(e['record'], {'版', '状態', '失敗分類', '実行記録'});
      final record = saved['実行記録'];
      if (saved['版'] != 1 ||
          record is! Map ||
          !RuntimeDialogueClient.validId(record['要求ID']) ||
          !RuntimeDialogueClient.validId(record['対話セッションID'])) {
        _reject();
      }
      final r = record;
      if ((requestId != null && r['要求ID'] != requestId) ||
          (sessionId != null && r['対話セッションID'] != sessionId) ||
          !requests.add(r['要求ID'] as String)) {
        _reject();
      }
      final detail = DialogueExecutionRecord.parse(
          r, r['要求ID'] as String, grant.runtime, r['対話セッションID'] as String);
      final s = saved['状態'];
      final failure = saved['失敗分類'];
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
      final start = detail.fields['開始時刻'] != null;
      final end = detail.fields['終了時刻'] != null;
      final valid = switch (s) {
        '承認待ち' => !start && !end && failure == null,
        '実行中' => start && !end && failure == null,
        '成功' || '保留' => start && end && failure == null,
        '失敗' => end && failures.contains(failure) && failure != '取消',
        '中止' => failure == '取消',
        _ => false
      };
      if (!valid || (state != null && s != state)) {
        _reject();
      }
      final context = HistoryAuditContext.parse(e['audit_context'], detail);
      parsed.add(HistoryEntry(
          s as String,
          failure as String?,
          e['audit_event_id'] as String,
          e['event_hash'] as String,
          detail,
          context,
          HistoryResultEvidence.parse(e['result_evidence'], detail, context, s),
          HistoryContentReceipt.parse(
              e['content_receipt'], detail, context, s)));
    }
    // 取得中の失効・承認置換を表示前に再照合する。
    final current = await status();
    if (current == null || !grant.same(current) || !grant.current) {
      _reject();
    }
    return HistoryPage(List.unmodifiable(parsed), next, more, grant);
  }

  Future<Map<String, Object?>> replay(
      HistoryGrant grant, HistoryEntry parent, String input,
      {required bool branch}) async {
    if (!grant.current ||
        input.trim().isEmpty ||
        input.runes.length > 4096 ||
        parent.record.fields['実行系ID'] != grant.runtime) {
      _reject();
    }
    final before = await status();
    if (before == null || !grant.same(before) || !grant.current) {
      _reject();
    }
    final op = branch ? '対話分岐' : '対話再実行';
    final response = await transport.request(op, payload: {
      'approval_id': grant.id,
      '実行系ID': grant.runtime,
      '参照監査ID': parent.auditId,
      '参照event_hash': parent.eventHash,
      '入力': input
    });
    if (response['operation'] != op ||
        response['status'] != 'accepted' ||
        response['evidence_source'] != 'INTERNAL_STATE' ||
        !_text(response['audit_event_id'])) {
      _reject();
    }
    final body = _shape(response['body'], {
      '要求ID',
      '要求hash',
      '状態',
      '期限',
      '対話セッションID',
      '実行系ID',
      '参照監査ID',
      '参照event_hash',
      '種別'
    });
    if (!RuntimeDialogueClient.validId(body['要求ID']) ||
        !RuntimeDialogueClient.validId(body['対話セッションID']) ||
        body['要求ID'] == parent.record.fields['要求ID'] ||
        body['対話セッションID'] == parent.record.fields['対話セッションID'] ||
        !_hash(body['要求hash']) ||
        body['状態'] != '承認待ち' ||
        body['期限'] is! int ||
        body['実行系ID'] != grant.runtime ||
        body['種別'] != op ||
        body['参照監査ID'] != parent.auditId ||
        body['参照event_hash'] != parent.eventHash) {
      _reject();
    }
    final after = await status();
    if (after == null || !grant.same(after) || !grant.current) {
      _reject();
    }
    return Map<String, Object?>.unmodifiable(body.cast<String, Object?>());
  }
}

class HistoryGrant {
  HistoryGrant._(this.id, this.runtime, this.expires)
      : _clock = Stopwatch()..start(),
        _remaining = expires * 1000 - DateTime.now().millisecondsSinceEpoch;
  final String id, runtime;
  final int expires;
  final Stopwatch _clock;
  final int _remaining;
  bool get current =>
      DateTime.now().millisecondsSinceEpoch < expires * 1000 &&
      _clock.elapsedMilliseconds < _remaining;
  bool same(HistoryGrant other) =>
      id == other.id && runtime == other.runtime && expires == other.expires;
  factory HistoryGrant.parse(Object? raw) {
    final m = _shape(raw, {'approval_id', 'runtime_id', 'expires_at'});
    final runtime = m['runtime_id'];
    final expires = m['expires_at'];
    if (!RuntimeDialogueClient.validId(m['approval_id']) ||
        runtime is! String ||
        runtime.length > 128 ||
        !RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.-]*$').hasMatch(runtime) ||
        expires is! int ||
        expires < 0 ||
        expires > 8640000000000) {
      _reject();
    }
    final g = HistoryGrant._(m['approval_id'] as String, runtime, expires);
    if (!g.current || g._remaining > 300000) {
      _reject();
    }
    return g;
  }
}

class HistoryEntry {
  const HistoryEntry(this.state, this.failure, this.auditId, this.eventHash,
      this.record, this.context, this.evidence, this.receipt);
  final String state, auditId, eventHash;
  final String? failure;
  final DialogueExecutionRecord record;
  final HistoryAuditContext context;
  final HistoryResultEvidence? evidence;
  final HistoryContentReceipt? receipt;
}

class HistoryAuditContext {
  const HistoryAuditContext(this.requestHash, this.approvalId, this.scope);
  final String requestHash;
  final String? approvalId, scope;
  factory HistoryAuditContext.parse(
      Object? raw, DialogueExecutionRecord record) {
    final m =
        _shape(raw, {'版', '要求hash', '作成操作', '承認', '承認能力', '復旧対応', '現在権限'});
    if (m['版'] != 1 ||
        !_hash(m['要求hash']) ||
        m['作成操作'] != '対話送信' ||
        m['現在権限'] != false ||
        m['承認能力'] is! List) {
      _reject();
    }
    final caps = m['承認能力'] as List;
    if (m['承認'] == null) {
      if (record.fields['開始監査ID'] != null ||
          caps.isNotEmpty ||
          m['復旧対応'] != null) {
        _reject();
      }
      return HistoryAuditContext(m['要求hash'] as String, null, null);
    }
    final a = _shape(m['承認'], {'監査ID', '操作', '要求hash', '内容表示範囲'});
    if (!_text(a['監査ID']) ||
        a['監査ID'] != record.fields['開始監査ID'] ||
        a['操作'] != '対話承認' ||
        a['要求hash'] != m['要求hash'] ||
        !['none', 'hash_only', 'summary', 'redacted', 'full']
            .contains(a['内容表示範囲']) ||
        caps.length != 1 ||
        caps.single != '対話送信' ||
        m['復旧対応'] != '接続再確認') {
      _reject();
    }
    return HistoryAuditContext(
        m['要求hash'] as String, a['監査ID'] as String, a['内容表示範囲'] as String);
  }
}

class HistoryPage {
  const HistoryPage(this.entries, this.next, this.more, this.grant);
  final List<HistoryEntry> entries;
  final int next;
  final bool more;
  final HistoryGrant grant;
}

Map _shape(Object? raw, Set<String> keys) {
  if (raw is! Map || raw.length != keys.length || !keys.containsAll(raw.keys)) {
    _reject();
  }
  return raw;
}

bool _text(Object? v) =>
    v is String &&
    v.isNotEmpty &&
    v.length <= 256 &&
    !v.runes.any((c) => c < 32 || c == 127);
bool _hash(Object? v) =>
    v is String &&
    v.length == 71 &&
    RegExp(r'^sha256:[0-9a-f]{64}$').hasMatch(v);
Never _reject() => throw const BrokerClientException('履歴の現在承認または応答を確認できません');

/// 過去のAdapter申告hash。現在権限と実使用証明は持たない。
class HistoryResultEvidence {
  const HistoryResultEvidence(
      this.responseHash, this.capabilityHash, this.routeHash, this.traceHash);
  final String responseHash;
  final String? capabilityHash, routeHash, traceHash;
  static HistoryResultEvidence? parse(
      Object? raw,
      DialogueExecutionRecord record,
      HistoryAuditContext context,
      String state) {
    if (raw == null) return null;
    final m = _shape(raw, {
      '版',
      '要求ID',
      '対話セッションID',
      '実行系ID',
      '要求hash',
      '終了監査ID',
      '表示範囲',
      '応答hash',
      '能力申告hash',
      '経路申告hash',
      '追跡参照hash',
      '証拠種別'
    });
    if (m['版'] != 1 ||
        m['証拠種別'] != 'INTERNAL_STATE' ||
        !['成功', '保留'].contains(state) ||
        m['要求hash'] != context.requestHash ||
        m['表示範囲'] != context.scope ||
        !['full', 'hash_only', 'summary', 'redacted'].contains(m['表示範囲']) ||
        !_hash(m['応答hash'])) {
      _reject();
    }
    for (final key in ['要求ID', '対話セッションID', '実行系ID', '終了監査ID']) {
      if (m[key] != record.fields[key]) _reject();
    }
    for (final key in ['能力申告hash', '経路申告hash', '追跡参照hash']) {
      if (context.scope == 'full' ? !_hash(m[key]) : m[key] != null) _reject();
    }
    return HistoryResultEvidence(
        m['応答hash'] as String,
        m['能力申告hash'] as String?,
        m['経路申告hash'] as String?,
        m['追跡参照hash'] as String?);
  }
}

class HistoryContentReceipt {
  const HistoryContentReceipt(this.auditId, this.eventHash, this.cipherHash);
  final String auditId, eventHash, cipherHash;
  static HistoryContentReceipt? parse(
      Object? raw,
      DialogueExecutionRecord record,
      HistoryAuditContext context,
      String state) {
    if (raw == null) return null;
    final m = _shape(raw, {'audit_event_id', 'event_hash', 'receipt'});
    final r = _shape(m['receipt'], {
      '版',
      '要求ID',
      '対話セッションID',
      '実行系ID',
      '要求hash',
      '終了監査ID',
      '保存承認監査ID',
      '暗号文hash',
      '証拠種別'
    });
    if (!_text(m['audit_event_id']) ||
        !_hash(m['event_hash']) ||
        r['版'] != 1 ||
        r['証拠種別'] != 'INTERNAL_STATE' ||
        !['成功', '保留'].contains(state) ||
        context.scope != 'full' ||
        r['要求hash'] != context.requestHash ||
        !_text(r['保存承認監査ID']) ||
        !_hash(r['暗号文hash'])) {
      _reject();
    }
    for (final key in ['要求ID', '対話セッションID', '実行系ID', '終了監査ID']) {
      if (r[key] != record.fields[key]) _reject();
    }
    return HistoryContentReceipt(m['audit_event_id'] as String,
        m['event_hash'] as String, r['暗号文hash'] as String);
  }
}

class HistoryContentGrant {
  HistoryContentGrant._(
      this.id, this.request, this.auditId, this.eventHash, this.expires)
      : _clock = Stopwatch()..start(),
        _started = DateTime.now().millisecondsSinceEpoch;
  final String id, request, auditId, eventHash;
  final int expires, _started;
  final Stopwatch _clock;
  bool get current {
    final now = DateTime.now().millisecondsSinceEpoch;
    return now >= _started &&
        now < expires * 1000 &&
        _clock.elapsedMilliseconds < expires * 1000 - _started;
  }

  bool same(HistoryContentGrant other) =>
      id == other.id &&
      request == other.request &&
      auditId == other.auditId &&
      eventHash == other.eventHash &&
      expires == other.expires;
  bool matches(HistoryEntry e) =>
      request == e.record.fields['要求ID'] &&
      auditId == e.receipt?.auditId &&
      eventHash == e.receipt?.eventHash;
  factory HistoryContentGrant.parse(Object? raw) {
    final m = _shape(
        raw, {'approval_id', '要求ID', '保存監査ID', '保存監査hash', 'expires_at'});
    final expires = m['expires_at'];
    if (!RuntimeDialogueClient.validId(m['approval_id']) ||
        !RuntimeDialogueClient.validId(m['要求ID']) ||
        !_text(m['保存監査ID']) ||
        !_hash(m['保存監査hash']) ||
        expires is! int ||
        expires < 0 ||
        expires > 8640000000000) {
      _reject();
    }
    final g = HistoryContentGrant._(
        m['approval_id'] as String,
        m['要求ID'] as String,
        m['保存監査ID'] as String,
        m['保存監査hash'] as String,
        expires);
    if (!g.current || expires * 1000 - g._started > 60000) _reject();
    return g;
  }
}

class HistoryContent {
  const HistoryContent(this.input, this.result, this.grant);
  final String input;
  final DialogueResult result;
  final HistoryContentGrant grant;
}
