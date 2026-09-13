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
      this.auditId,
      [this.baselineHash]);
  final WorkspaceRegistration registration;
  final String operation, path, auditId;
  final String? baselineHash;
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
      {required bool tree,
      bool scope = false,
      bool changes = false,
      bool preview = false,
      String? baselineHash}) async {
    if (!selected.current(DateTime.now())) {
      _invalid();
    }
    if ((preview &&
            (changes ||
                scope ||
                tree ||
                path.isEmpty ||
                baselineHash == null)) ||
        (changes &&
            (scope || tree || path.isNotEmpty || baselineHash == null)) ||
        (scope && (path.isNotEmpty || baselineHash != null)) ||
        (baselineHash != null && !_hash.hasMatch(baselineHash))) {
      _invalid();
    }
    final operation = preview
        ? '作業領域復旧プレビュー'
        : changes
            ? '作業領域変更一覧'
            : scope
                ? '作業領域比較範囲'
                : baselineHash != null
                    ? '作業領域差分'
                    : tree
                        ? '作業領域ツリー'
                        : '作業領域読取';
    final payload = <String, Object?>{
      '作業領域ID': selected.id,
      '相対path': path,
      if (baselineHash != null) '基準点hash': baselineHash
    };
    final response = await _request(operation, payload);
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
        body['要求hash'] != brokerPayloadHash(payload) ||
        body['作業領域ID'] != selected.id ||
        body['実行系ID'] != selected.runtime ||
        body['登録hash'] != selected.hash ||
        body['approval_id'] != selected.approval ||
        body['有効期限'] != selected.expires ||
        body['表示範囲'] != selected.visibility) {
      _invalid();
    }
    final live = !scope &&
        (selected.visibility == 'full' || selected.visibility == 'hash_only');
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
          if (scope) {
            _keys(projection, ['基準点hash', '相対paths']);
            final hash = projection['基準点hash'];
            final paths = projection['相対paths'];
            if (paths is! List ||
                paths.length > 4096 ||
                paths.toSet().length != paths.length ||
                !paths.every((v) =>
                    v is String &&
                    v.isNotEmpty &&
                    utf8.encode(v).length <= 1024) ||
                (hash == null
                    ? paths.isNotEmpty
                    : hash is! String || !_hash.hasMatch(hash))) {
              _invalid();
            }
          } else if (changes) {
            _keys(projection,
                ['基準点hash', 'changes', 'unchanged', 'excluded_secrets']);
            final items = projection['changes'];
            final unchanged = projection['unchanged'];
            final excluded = projection['excluded_secrets'];
            if (projection['基準点hash'] != baselineHash ||
                items is! List ||
                items.length > 8192 ||
                unchanged is! int ||
                unchanged < 0 ||
                items.length + unchanged > 8192 ||
                excluded is! int ||
                excluded < 0 ||
                excluded > 4096) {
              _invalid();
            }
            final seen = <String>{};
            for (final rawItem in items) {
              final item = _map(rawItem);
              _keys(item, ['path', 'status']);
              final itemPath = _string(item['path']);
              if (itemPath.isEmpty ||
                  utf8.encode(itemPath).length > 1024 ||
                  itemPath
                      .split('/')
                      .any((v) => v.isEmpty || v == '.' || v == '..') ||
                  RegExp(r'[\x00-\x1f\x7f\\:]').hasMatch(itemPath) ||
                  !seen.add(itemPath) ||
                  !{'added', 'modified', 'deleted'}.contains(item['status'])) {
                _invalid();
              }
            }
          } else if (baselineHash != null) {
            _keys(projection, [
              '基準点hash',
              'diff',
              if (preview) ...[
                'action',
                'baseline_content_available',
                'execution_permitted'
              ]
            ]);
            if (projection['基準点hash'] != baselineHash) {
              _invalid();
            }
            _validateDiff(projection['diff']);
            if (preview) {
              final diff = _map(projection['diff']);
              final action = diff['kind'] == 'unchanged'
                  ? 'none'
                  : diff['after'] == null
                      ? 'remove'
                      : diff['before'] == null
                          ? 'recreate'
                          : 'replace';
              if (projection['execution_permitted'] != false ||
                  projection['baseline_content_available'] is! bool ||
                  projection['action'] != action ||
                  (diff['after'] == null &&
                      projection['baseline_content_available'] != true) ||
                  (diff['kind'] == 'text' &&
                      projection['baseline_content_available'] != true)) {
                _invalid();
              }
            }
          } else if (tree) {
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
                  (entry['kind'] == 'directory'
                      ? entry['bytes'] != null
                      : entry['bytes'] is! int ||
                          (entry['bytes'] as int) < 0)) {
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
    if (baselineHash != null && selected.visibility == 'full') {
      final currentScope = await read(selected, '', tree: false, scope: true);
      if (currentScope.projection?['基準点hash'] != baselineHash) {
        _invalid();
      }
    }
    return WorkspaceView(selected, operation, path, projection,
        response['audit_event_id'] as String, baselineHash);
  }
}

void _validateDiff(Object? raw) {
  final diff = _map(raw);
  _keys(diff, ['version', 'kind', 'before', 'after', 'unified', 'rows']);
  if (diff['version'] != 1 ||
      !{'text', 'unchanged', 'binary', 'oversized'}.contains(diff['kind'])) {
    _invalid();
  }
  for (final side in ['before', 'after']) {
    if (diff[side] == null) continue;
    final version = _map(diff[side]);
    _keys(version, ['bytes', 'sha256']);
    if (version['bytes'] is! int ||
        (version['bytes'] as int) < 0 ||
        !_hash.hasMatch(_string(version['sha256']))) {
      _invalid();
    }
  }
  final rows = diff['rows'];
  if (rows is! List || rows.length > 4000) {
    _invalid();
  }
  if (diff['kind'] != 'text') {
    if (diff['unified'] != null || rows.isNotEmpty) {
      _invalid();
    }
    return;
  }
  final unified = _string(diff['unified']);
  if (utf8.encode(unified).length > 262144) {
    _invalid();
  }
  final numbers = <String, int>{'before': 0, 'after': 0};
  var totalBytes = 0;
  final sizes = <String, int>{'before': 0, 'after': 0};
  final ended = <String, bool>{'before': false, 'after': false};
  for (final rawRow in rows) {
    final row = _map(rawRow);
    _keys(row, ['kind', 'before', 'after']);
    if (!{'same', 'added', 'deleted', 'changed'}.contains(row['kind'])) {
      _invalid();
    }
    if ((row['kind'] == 'added' &&
            (row['before'] != null || row['after'] == null)) ||
        (row['kind'] == 'deleted' &&
            (row['before'] == null || row['after'] != null)) ||
        ({'same', 'changed'}.contains(row['kind']) &&
            (row['before'] == null || row['after'] == null))) {
      _invalid();
    }
    for (final side in ['before', 'after']) {
      if (row[side] == null) continue;
      final line = _map(row[side]);
      _keys(line, ['number', 'text', 'newline']);
      if (ended[side]! ||
          line['number'] != numbers[side]! + 1 ||
          line['newline'] is! bool) {
        _invalid();
      }
      numbers[side] = line['number'] as int;
      final text = _string(line['text']);
      if (text.contains('\n') || text.contains(String.fromCharCode(0))) {
        _invalid();
      }
      sizes[side] = sizes[side]! +
          utf8.encode(text).length +
          (line['newline'] == true ? 1 : 0);
      ended[side] = line['newline'] == false;
      totalBytes += utf8.encode(text).length;
      if (totalBytes > 131072) {
        _invalid();
      }
    }
    if (row['kind'] == 'same' &&
        ((row['before'] as Map)['text'] != (row['after'] as Map)['text'] ||
            (row['before'] as Map)['newline'] !=
                (row['after'] as Map)['newline'])) {
      _invalid();
    }
  }
  for (final side in ['before', 'after']) {
    final version = diff[side];
    if (version == null
        ? numbers[side] != 0
        : (version as Map)['bytes'] != sizes[side]) {
      _invalid();
    }
    if (numbers[side]! > 2000 || sizes[side]! > 65536) {
      _invalid();
    }
  }
}
