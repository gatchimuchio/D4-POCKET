import 'broker_transport.dart';
import 'runtime_dialogue_client.dart';

/// 現在承認を読むだけの履歴client。承認発行操作を公開しない。
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

  Future<HistoryGrant?> status() async {
    final body = await _call('対話履歴閲覧状態', {});
    if (body['page'] != null) {
      _reject();
    }
    return body['grant'] == null ? null : HistoryGrant.parse(body['grant']);
  }

  Future<HistoryPage> page(HistoryGrant grant,
      {int after = 0, String? state}) async {
    if (!grant.current || after < 0) {
      _reject();
    }
    final body = await _call('対話履歴閲覧', {
      'approval_id': grant.id,
      'query': {
        'after': after,
        'limit': 50,
        'latest_per_request': true,
        'filter': {'実行系ID': grant.runtime, if (state != null) '状態': state}
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
      final e = _shape(raw, {'audit_event_id', 'event_hash', 'record'});
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
      if (!requests.add(r['要求ID'] as String)) {
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
      parsed.add(HistoryEntry(s as String, failure as String?,
          e['audit_event_id'] as String, detail));
    }
    // 取得中の失効・承認置換を表示前に再照合する。
    final current = await status();
    if (current == null || !grant.same(current) || !grant.current) {
      _reject();
    }
    return HistoryPage(List.unmodifiable(parsed), next, more, grant);
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
  const HistoryEntry(this.state, this.failure, this.auditId, this.record);
  final String state, auditId;
  final String? failure;
  final DialogueExecutionRecord record;
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
