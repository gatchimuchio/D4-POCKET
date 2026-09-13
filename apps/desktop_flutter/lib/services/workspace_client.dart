import 'dart:convert';

import 'broker_client.dart';

const _visibilities = {'none', 'hash_only', 'summary', 'redacted', 'full'};
final _hash = RegExp(r'^sha256:[a-f0-9]{64}$');
final _id = RegExp(r'^[a-zA-Z0-9][a-zA-Z0-9_.-]{0,127}$');
Never _invalid() => throw const BrokerClientException('作業領域の応答を確認できません');
Map<String, Object?> _map(Object? value) {
  if (value is! Map<String, Object?>) {
    _invalid();
  }
  return value;
}

void _keys(Map<String, Object?> value, List<String> keys) {
  if (value.length != keys.length || !keys.every(value.containsKey)) {
    _invalid();
  }
}

String _string(Object? value) {
  if (value is! String) {
    _invalid();
  }
  return value;
}

class WorkspaceRegistration {
  WorkspaceRegistration._(this.id, this.runtime, this.hash, this.visibility,
      this.approval, this.expires);
  final String id, runtime, hash, visibility;
  final String? approval;
  final int? expires;

  factory WorkspaceRegistration.parse(Object? value) {
    final data = _map(value);
    _keys(data,
        ['作業領域ID', '実行系ID', '登録hash', '承認状態', '有効期限', '表示範囲', 'approval_id']);
    final id = _string(data['作業領域ID']);
    final runtime = _string(data['実行系ID']);
    final hash = _string(data['登録hash']);
    final visibility = _string(data['表示範囲']);
    if (!_id.hasMatch(id) ||
        !_id.hasMatch(runtime) ||
        !_hash.hasMatch(hash) ||
        !_visibilities.contains(visibility)) {
      _invalid();
    }
    final approval = data['approval_id'];
    final expires = data['有効期限'];
    if (data['承認状態'] == 'approved') {
      if (approval is! String ||
          approval.isEmpty ||
          expires is! int ||
          expires <= 0) {
        _invalid();
      }
      return WorkspaceRegistration._(
          id, runtime, hash, visibility, approval, expires);
    }
    if (data['承認状態'] != 'denied' ||
        approval != null ||
        expires != null ||
        visibility != 'none') {
      _invalid();
    }
    return WorkspaceRegistration._(id, runtime, hash, visibility, null, null);
  }

  bool current(DateTime now) =>
      approval != null && expires! * 1000 > now.millisecondsSinceEpoch;
  bool sameGrant(WorkspaceRegistration other) =>
      id == other.id &&
      runtime == other.runtime &&
      hash == other.hash &&
      approval == other.approval &&
      expires == other.expires &&
      visibility == other.visibility;
}

class WorkspaceView {
  WorkspaceView(this.registration, this.operation, this.path, this.projection,
      this.auditId);
  final WorkspaceRegistration registration;
  final String operation, path, auditId;
  final Map<String, Object?>? projection;
}

/// Brokerのprojectionだけを消費する。owner資格と権限付与を保持しない。
class WorkspaceClient {
  WorkspaceClient(this.transport);
  final BrokerTransport transport;

  Future<Map<String, Object?>> _request(
      String operation, Map<String, Object?> payload) async {
    final response = await transport.request(operation, payload: payload);
    if (response['status'] != 'accepted' ||
        response['operation'] != operation ||
        response['error'] != null ||
        response['audit_event_id'] is! String ||
        (response['audit_event_id'] as String).isEmpty) {
      _invalid();
    }
    return response;
  }

  Future<List<WorkspaceRegistration>> list() async {
    final response = await _request('作業領域一覧', {});
    if (response['evidence_source'] != 'INTERNAL_STATE') {
      _invalid();
    }
    final body = _map(response['body']);
    _keys(body, ['作業領域']);
    final entries = body['作業領域'];
    if (entries is! List || entries.length > 16) {
      _invalid();
    }
    final result =
        entries.map(WorkspaceRegistration.parse).toList(growable: false);
    if (result.map((v) => v.id).toSet().length != result.length) {
      _invalid();
    }
    return List.unmodifiable(result);
  }

  Future<WorkspaceView> read(WorkspaceRegistration selected, String path,
      {required bool tree}) async {
    if (!selected.current(DateTime.now())) {
      _invalid();
    }
    final operation = tree ? '作業領域ツリー' : '作業領域読取';
    final response =
        await _request(operation, {'作業領域ID': selected.id, '相対path': path});
    final body = _map(response['body']);
    _keys(body, [
      'version',
      'operation',
      '要求hash',
      '作業領域ID',
      '実行系ID',
      '登録hash',
      'approval_id',
      '有効期限',
      '表示範囲',
      'projection'
    ]);
    if (body['version'] != 1 ||
        body['operation'] != operation ||
        body['要求hash'] !=
            brokerPayloadHash({'作業領域ID': selected.id, '相対path': path}) ||
        body['作業領域ID'] != selected.id ||
        body['実行系ID'] != selected.runtime ||
        body['登録hash'] != selected.hash ||
        body['approval_id'] != selected.approval ||
        body['有効期限'] != selected.expires ||
        body['表示範囲'] != selected.visibility) {
      _invalid();
    }
    final live =
        selected.visibility == 'full' || selected.visibility == 'hash_only';
    if (response['evidence_source'] !=
        (live ? 'LIVE_RUNTIME' : 'INTERNAL_STATE')) {
      _invalid();
    }
    final raw = body['projection'];
    Map<String, Object?>? projection;
    if (selected.visibility == 'none') {
      if (raw != null) {
        _invalid();
      }
    } else {
      projection = _map(raw);
      switch (selected.visibility) {
        case 'hash_only':
          _keys(projection, ['sha256']);
          if (!_hash.hasMatch(_string(projection['sha256']))) {
            _invalid();
          }
        case 'summary':
        case 'redacted':
          _keys(projection, ['説明']);
          if (projection['説明'] != 'この表示範囲に提供できる承認済み内容はありません') {
            _invalid();
          }
        case 'full':
          if (tree) {
            _keys(projection, ['entries']);
            final entries = projection['entries'];
            if (entries is! List || entries.length > 1024) {
              _invalid();
            }
            final seen = <String>{};
            for (final rawEntry in entries) {
              final entry = _map(rawEntry);
              _keys(entry, ['path', 'kind', 'bytes']);
              final entryPath = _string(entry['path']);
              final prefix = path.isEmpty ? '' : '$path/';
              if (!entryPath.startsWith(prefix) ||
                  entryPath.length <= prefix.length ||
                  entryPath.substring(prefix.length).contains('/') ||
                  !seen.add(entryPath) ||
                  !{'file', 'directory'}.contains(entry['kind']) ||
                  entry['bytes'] is! int ||
                  (entry['bytes'] as int) < 0) {
                _invalid();
              }
            }
          } else {
            _keys(projection, ['path', 'bytes', 'sha256', 'binary', 'text']);
            final count = projection['bytes'];
            if (projection['path'] != path ||
                count is! int ||
                count < 0 ||
                count > 65536 ||
                !_hash.hasMatch(_string(projection['sha256']))) {
              _invalid();
            }
            if (projection['binary'] == true) {
              if (projection['text'] != null) {
                _invalid();
              }
            } else if (projection['binary'] == false) {
              final text = _string(projection['text']);
              if (text.contains(String.fromCharCode(0)) ||
                  utf8.encode(text).length != count) {
                _invalid();
              }
            } else {
              _invalid();
            }
          }
        default:
          _invalid();
      }
    }
    // 取得中の失効・再承認・再登録を、描画前に現在一覧と照合する。
    final current = await list();
    if (!selected.current(DateTime.now()) ||
        !current.any((v) => v.sameGrant(selected))) {
      _invalid();
    }
    return WorkspaceView(selected, operation, path, projection,
        response['audit_event_id'] as String);
  }
}
