import 'dart:convert';

import 'package:broker_contract/broker_request_envelope.dart' show brokerTextHash;

import 'ai_edit_client.dart';
import 'broker_client.dart';

const _visibilities = {'none', 'hash_only', 'summary', 'redacted', 'full'};
const workspaceEditReviewOperation = 'GUI Shell編集提案審査';
const workspaceEditApplyTargetOperation = 'GUI Shell編集適用先照合';
const workspaceEditApplyOperation = 'GUI Shell編集適用';
const workspaceEditRollbackOperation = 'GUI Shell編集Rollback';
final _hash = RegExp(r'^sha256:[a-f0-9]{64}$');
final _id = RegExp(r'^[a-zA-Z0-9][a-zA-Z0-9_.-]{0,127}$');
final _auditEventId = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$');
const _unavailableAuditEventId = 'broker-audit-unavailable';
const _workspaceFailureCodes = {
  '作業領域拒否',
  '権限拒否',
  'broker_persistence_unavailable',
  '監査失敗',
  'broker_audit_append_failed',
  '作業領域期限超過',
  'ai_edit_review_provenance_invalid',
  'ai_edit_apply_binding_invalid',
  'ai_edit_apply_rejected',
  'ai_edit_rollback_unsupported',
  'ai_edit_rollback_binding_invalid',
  'ai_edit_rollback_rejected',
};
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

/// Broker拒否から、秘密を含み得る自由記述messageを除いて画面へ渡す情報。
class WorkspaceClientFailure implements Exception {
  const WorkspaceClientFailure({
    required this.operation,
    required this.status,
    required this.code,
    required this.recoverable,
    required this.auditEventId,
  });

  final String operation;
  final String status;
  final String code;
  final bool recoverable;
  final String? auditEventId;

  String get recoveryAction {
    if (status == 'suspended') {
      return 'Broker状態が復旧しAudit可用性を確認するまで作業領域操作を停止してください。再送はせず、復旧後に別要求として再開してください。';
    }
    if (auditEventId == null ||
        code == 'broker_persistence_unavailable' ||
        code == '監査失敗' ||
        code == 'broker_audit_append_failed') {
      return 'Broker永続監査を復旧し、状態を確認するまで内容操作を停止してください。再送はせず、確認後に別要求として再開してください。';
    }
    if (!recoverable) {
      return 'Brokerがこの操作を復旧可能としていません。自動再試行せず、Broker状態とAuditを確認してから操作を判断してください。';
    }
    return switch (code) {
      '権限拒否' => 'native Owner確認を完了した後、登録状態を更新してください。自動再試行はしていません。',
      '作業領域期限超過' => '登録状態を更新し、必要なら新しい期限付きWorkspace読取Approvalを要求してください。',
      '作業領域拒否' =>
        '現在の登録・Workspace範囲・読取Approvalを更新して確認してください。再試行は状態確認後に別要求として行ってください。',
      _ => '登録・Approval状態を更新して確認してください。再試行は状態確認後に別要求として行ってください。',
    };
  }

  String get userMessage => [
        status == 'suspended'
            ? 'Brokerが作業領域操作を一時停止しました。'
            : 'Brokerが作業領域操作を拒否しました。',
        '操作: $operation',
        'Broker code: $code',
        '復旧可能: ${recoverable ? 'yes' : 'no'}',
        if (auditEventId == null)
          'Audit: 利用不可（Brokerの監査状態を確認してください）'
        else
          'Audit: $auditEventId',
        '復旧: $recoveryAction',
      ].join('\n');

  @override
  String toString() => userMessage;
}

WorkspaceClientFailure? _workspaceFailure(
    String operation, Map<String, Object?> response) {
  final status = response['status'];
  if (status != 'rejected' && status != 'suspended') return null;
  if (response['operation'] != operation ||
      response['evidence_source'] != 'INTERNAL_STATE') {
    _invalid();
  }
  final error = _map(response['error']);
  final rawCode = error['code'];
  final rawMessage = error['message'];
  final recoverable = error['recoverable'];
  if (rawCode is! String ||
      rawCode.isEmpty ||
      rawCode.length > 128 ||
      rawMessage is! String ||
      utf8.encode(rawMessage).length > 4096 ||
      recoverable is! bool ||
      error['audit_event_required'] != true ||
      error['fail_closed'] != true) {
    _invalid();
  }

  final rawAuditId = response['audit_event_id'];
  final String? auditId;
  if (rawAuditId == _unavailableAuditEventId) {
    auditId = null;
  } else if (rawAuditId is String && _auditEventId.hasMatch(rawAuditId)) {
    auditId = rawAuditId;
  } else {
    _invalid();
  }

  return WorkspaceClientFailure(
    operation: operation,
    status: status as String,
    code: _workspaceFailureCodes.contains(rawCode)
        ? rawCode
        : 'unknown_broker_error',
    recoverable: recoverable,
    auditEventId: auditId,
  );
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
      [this.baselineHash, this.sourceRegistration, this.editTargetRequest,
      this.appliedRequest]);
  final WorkspaceRegistration registration;
  final String operation, path, auditId;
  final String? baselineHash;
  final WorkspaceRegistration? sourceRegistration;
  final Map<String, Object?>? projection;
  final Map<String, Object?>? editTargetRequest;
  final Map<String, Object?>? appliedRequest;
}

/// Brokerのprojectionだけを消費する。owner資格と権限付与を保持しない。
class WorkspaceClient {
  WorkspaceClient(this.transport);
  final BrokerTransport transport;

  Future<Map<String, Object?>> _request(
      String operation, Map<String, Object?> payload) async {
    final response = await transport.request(operation, payload: payload);
    if (response['operation'] != operation) _invalid();
    if (response['status'] != 'accepted') {
      final failure = _workspaceFailure(operation, response);
      if (failure != null) throw failure;
      _invalid();
    }
    if (response['error'] != null ||
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

  Future<WorkspaceRegistration> approveContent(
      WorkspaceRegistration selected, String visibility) async {
    if (!_visibilities.contains(visibility)) _invalid();
    final payload = {
      '作業領域ID': selected.id,
      '登録hash': selected.hash,
      '表示範囲': visibility,
    };
    final response = await _request('作業領域承認', payload);
    if (response['evidence_source'] != 'INTERNAL_STATE') _invalid();
    final body = _map(response['body']);
    _keys(body, [
      '作業領域ID',
      'approval_id',
      'permission_id',
      'capability_id',
      'recovery_id',
      '有効期限',
      '表示範囲',
    ]);
    final approvalId = _string(body['approval_id']);
    if (body['作業領域ID'] != selected.id ||
        body['permission_id'] != 'workspace.inspect.$approvalId' ||
        body['capability_id'] != 'workspace.inspect' ||
        body['recovery_id'] != 'workspace.reapprove' ||
        body['有効期限'] is! int ||
        (body['有効期限'] as int) <=
            DateTime.now().millisecondsSinceEpoch ~/ 1000 ||
        body['表示範囲'] != visibility) {
      _invalid();
    }
    final refreshed = await list();
    final current =
        refreshed.where((entry) => entry.id == selected.id).toList();
    if (current.length != 1 ||
        current.single.runtime != selected.runtime ||
        current.single.hash != selected.hash ||
        current.single.approval != approvalId ||
        current.single.visibility != visibility) {
      _invalid();
    }
    return current.single;
  }

  Future<void> revokeContent(WorkspaceRegistration selected) async {
    final payload = {'作業領域ID': selected.id, '登録hash': selected.hash};
    final response = await _request('作業領域失効', payload);
    if (response['evidence_source'] != 'INTERNAL_STATE') _invalid();
    final body = _map(response['body']);
    _keys(body, ['作業領域ID', '承認状態']);
    if (body['作業領域ID'] != selected.id || body['承認状態'] != 'revoked') {
      _invalid();
    }
    final refreshed = await list();
    final current =
        refreshed.where((entry) => entry.id == selected.id).toList();
    if (current.length != 1 ||
        current.single.runtime != selected.runtime ||
        current.single.hash != selected.hash ||
        current.single.approval != null ||
        current.single.visibility != 'none') {
      _invalid();
    }
  }

  Future<WorkspaceView> captureWholeBaseline(
      WorkspaceRegistration selected) async {
    if (!selected.current(DateTime.now()) || selected.visibility != 'full') {
      _invalid();
    }
    final payload = {'作業領域ID': selected.id, '登録hash': selected.hash};
    final response = await _request('作業領域全体基準点保存', payload);
    if (response['evidence_source'] != 'LIVE_RUNTIME') _invalid();
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
      'projection',
    ]);
    if (body['version'] != 1 ||
        body['operation'] != '作業領域全体基準点保存' ||
        body['要求hash'] != brokerPayloadHash(payload) ||
        body['作業領域ID'] != selected.id ||
        body['実行系ID'] != selected.runtime ||
        body['登録hash'] != selected.hash ||
        body['approval_id'] != selected.approval ||
        body['有効期限'] != selected.expires ||
        body['表示範囲'] != 'full') {
      _invalid();
    }
    final projection = _map(body['projection']);
    _keys(projection, ['基準点hash', '対象数']);
    final baselineHash = _string(projection['基準点hash']);
    final entryCount = projection['対象数'];
    if (!_hash.hasMatch(baselineHash) ||
        entryCount is! int ||
        entryCount < 0 ||
        entryCount > 4096) {
      _invalid();
    }
    final current = await list();
    if (!selected.current(DateTime.now()) ||
        !current.any((entry) => entry.sameGrant(selected))) {
      _invalid();
    }
    return WorkspaceView(selected, '作業領域全体基準点保存', '', projection,
        _string(response['audit_event_id']), baselineHash);
  }

  Future<WorkspaceView> reviewEditProposal(
    WorkspaceRegistration selected,
    AiEditProposalReview proposal,
    String baselineHash,
  ) async {
    const operation = workspaceEditReviewOperation;
    final original = proposal.toBrokerProposal();
    if (selected.visibility != 'full' ||
        !selected.current(DateTime.now()) ||
        !_hash.hasMatch(baselineHash) ||
        !_auditEventId.hasMatch(proposal.auditId) ||
        brokerPayloadHash(original) != proposal.instructionHash) {
      _invalid();
    }
    final payload = {
      'version': 1,
      'proposal': original,
      'proposal_audit_id': proposal.auditId,
      'workspace_id': selected.id,
      'registration_hash': selected.hash,
      'baseline_hash': baselineHash,
      'approval_id': selected.approval,
    };
    final response = await _request(operation, payload);
    final body = _map(response['body']);
    _keys(body, [
      'version', 'operation', '要求hash', '作業領域ID', '実行系ID', '登録hash',
      'approval_id', '有効期限', '表示範囲', 'projection',
    ]);
    final auditId = _string(response['audit_event_id']);
    if (response['evidence_source'] != 'LIVE_RUNTIME' ||
        !_auditEventId.hasMatch(auditId) ||
        auditId == _unavailableAuditEventId ||
        body['version'] != 1 ||
        body['operation'] != operation ||
        body['要求hash'] != brokerPayloadHash(payload) ||
        body['作業領域ID'] != selected.id ||
        body['実行系ID'] != selected.runtime ||
        body['登録hash'] != selected.hash ||
        body['approval_id'] != selected.approval ||
        body['有効期限'] != selected.expires ||
        body['表示範囲'] != 'full') {
      _invalid();
    }
    final projection = _map(body['projection']);
    _keys(projection, [
      'proposal_id', 'proposal_audit_id', 'proposal_hash', '基準点hash',
      'diff_hash', 'changes', 'excluded_secrets',
    ]);
    final changes = projection['changes'];
    final excluded = projection['excluded_secrets'];
    if (projection['proposal_id'] != proposal.editId ||
        projection['proposal_audit_id'] != proposal.auditId ||
        projection['proposal_hash'] != proposal.instructionHash ||
        projection['基準点hash'] != baselineHash ||
        excluded is! int ||
        excluded < 0 ||
        excluded > 4096 ||
        changes is! List ||
        projection['diff_hash'] != brokerPayloadHash({'changes': changes}) ||
        changes.isEmpty ||
        changes.length > proposal.targetPaths.length) {
      _invalid();
    }
    final paths = <String>{};
    for (final raw in changes) {
      final change = _map(raw);
      _keys(change, ['path', 'status', 'diff']);
      final path = _string(change['path']);
      if (!proposal.targetPaths.contains(path) || !paths.add(path)) {
        _invalid();
      }
      _validateDiff(change['diff']);
      final diff = _map(change['diff']);
      final before = diff['before'];
      final after = diff['after'];
      final status = before == null
          ? 'added'
          : after == null
              ? 'deleted'
              : 'modified';
      if (diff['kind'] == 'unchanged' ||
          (before == null && after == null) ||
          change['status'] != status) {
        _invalid();
      }
    }
    final current = await list();
    if (!selected.current(DateTime.now()) ||
        !current.any((entry) => entry.sameGrant(selected))) {
      _invalid();
    }
    return WorkspaceView(selected, operation, '', projection, auditId, baselineHash);
  }

  Future<WorkspaceView> prepareEditTarget(
    WorkspaceView review,
    WorkspaceRegistration target,
  ) async {
    const operation = workspaceEditApplyTargetOperation;
    final source = review.registration;
    final now = DateTime.now();
    if (review.operation != workspaceEditReviewOperation ||
        review.auditId == _unavailableAuditEventId ||
        !_auditEventId.hasMatch(review.auditId) ||
        target.id == source.id ||
        target.visibility != 'full' ||
        source.visibility != 'full' ||
        !target.current(now) ||
        !source.current(now)) {
      _invalid();
    }
    final reviewed = _map(review.projection);
    final changes = reviewed['changes'];
    if (changes is! List ||
        changes.isEmpty ||
        changes.length > 32 ||
        reviewed['diff_hash'] != brokerPayloadHash({'changes': changes})) {
      _invalid();
    }
    final files = changes.map((raw) {
      final change = _map(raw);
      final diff = _map(change['diff']);
      return {
        'path': change['path'],
        'before_hash': diff['before'] == null ? null : _map(diff['before'])['sha256'],
        'after_hash': diff['after'] == null ? null : _map(diff['after'])['sha256'],
      };
    }).toList(growable: false);
    final binding = {
      'proposal_id': reviewed['proposal_id'],
      'diff_hash': reviewed['diff_hash'],
      'workspace_id': source.id,
      'registration_hash': source.hash,
      'approval_id': source.approval,
      'expires_at': source.expires,
      'baseline_hash': reviewed['基準点hash'],
      'files': files,
    };
    final payload = {
      'version': 1,
      'review_audit_id': review.auditId,
      'review_binding': binding,
      'workspace_id': target.id,
      'registration_hash': target.hash,
      'approval_id': target.approval,
    };
    final response = await _request(operation, payload);
    final auditId = _string(response['audit_event_id']);
    final body = _map(response['body']);
    _keys(body, [
      'version', 'operation', '要求hash', '作業領域ID', '実行系ID', '登録hash',
      'approval_id', '有効期限', '表示範囲', 'projection',
    ]);
    final projection = _map(body['projection']);
    final expected = {
      'review_audit_id': review.auditId,
      'proposal_id': reviewed['proposal_id'],
      'diff_hash': reviewed['diff_hash'],
      'source_workspace_id': source.id,
      'source_approval_id': source.approval,
      'files': files,
    };
    if (response['evidence_source'] != 'LIVE_RUNTIME' ||
        !_auditEventId.hasMatch(auditId) ||
        auditId == _unavailableAuditEventId ||
        body['version'] != 1 ||
        body['operation'] != operation ||
        body['要求hash'] != brokerPayloadHash(payload) ||
        body['作業領域ID'] != target.id ||
        body['実行系ID'] != target.runtime ||
        body['登録hash'] != target.hash ||
        body['approval_id'] != target.approval ||
        body['有効期限'] != target.expires ||
        body['表示範囲'] != 'full' ||
        brokerPayloadHash(projection) != brokerPayloadHash(expected)) {
      _invalid();
    }
    final current = await list();
    final observed = DateTime.now();
    if (!source.current(observed) ||
        !target.current(observed) ||
        !current.any((entry) => entry.sameGrant(source)) ||
        !current.any((entry) => entry.sameGrant(target))) {
      _invalid();
    }
    return WorkspaceView(target, operation, '', projection, auditId, null,
        source, Map<String, Object?>.unmodifiable(payload));
  }

  Future<WorkspaceView> applyReviewedEdit(WorkspaceView prepared) async {
    const operation = workspaceEditApplyOperation;
    final targetRequest = prepared.editTargetRequest;
    final projection = _map(prepared.projection);
    final files = projection['files'];
    final now = DateTime.now();
    if (prepared.operation != workspaceEditApplyTargetOperation ||
        targetRequest == null ||
        !prepared.registration.current(now) ||
        prepared.sourceRegistration?.current(now) != true ||
        files is! List ||
        files.isEmpty ||
        files.length > 32 ||
        !_auditEventId.hasMatch(prepared.auditId) ||
        prepared.auditId == _unavailableAuditEventId) {
      _invalid();
    }
    final payload = {
      'version': 1,
      'target_audit_id': prepared.auditId,
      'target_request': targetRequest,
    };
    final response = await _request(operation, payload);
    final body = _map(response['body']);
    _keys(body, [
      'version', 'operation', 'request_hash', 'workspace_id', 'review_audit_id',
      'diff_hash', 'transaction_id', 'changed_files', 'status',
    ]);
    final auditId = _string(response['audit_event_id']);
    if (response['evidence_source'] != 'LIVE_RUNTIME' ||
        !_auditEventId.hasMatch(auditId) ||
        auditId == _unavailableAuditEventId ||
        body['version'] != 1 ||
        body['operation'] != operation ||
        body['request_hash'] != brokerPayloadHash(payload) ||
        body['workspace_id'] != prepared.registration.id ||
        body['review_audit_id'] != projection['review_audit_id'] ||
        body['diff_hash'] != projection['diff_hash'] ||
        !RegExp(r'^[a-f0-9]{64}$').hasMatch(_string(body['transaction_id'])) ||
        body['changed_files'] != files.length ||
        body['status'] != 'applied') {
      _invalid();
    }
    return WorkspaceView(
      prepared.registration,
      operation,
      '',
      body,
      auditId,
      null,
      prepared.sourceRegistration,
      prepared.editTargetRequest,
      Map<String, Object?>.unmodifiable(payload),
    );
  }

  Future<WorkspaceView> rollbackAppliedEdit(WorkspaceView applied) async {
    const operation = workspaceEditRollbackOperation;
    final appliedRequest = applied.appliedRequest;
    final body = _map(applied.projection);
    final selected = applied.registration;
    final now = DateTime.now();
    if (applied.operation != workspaceEditApplyOperation ||
        appliedRequest == null ||
        !selected.current(now) ||
        selected.visibility != 'full' ||
        selected.approval == null ||
        !_auditEventId.hasMatch(applied.auditId) ||
        applied.auditId == _unavailableAuditEventId ||
        body['operation'] != workspaceEditApplyOperation ||
        body['request_hash'] != brokerPayloadHash(appliedRequest) ||
        body['status'] != 'applied' ||
        !_hash.hasMatch(_string(body['request_hash'])) ||
        !_id.hasMatch(_string(body['transaction_id'])) ||
        body['changed_files'] is! int ||
        (body['changed_files'] as int) < 1 ||
        (body['changed_files'] as int) > 32) {
      _invalid();
    }
    final current = await list();
    final matches = current.where((entry) => entry.id == selected.id).toList();
    if (matches.length != 1 || !matches.single.sameGrant(selected)) {
      _invalid();
    }
    final payload = <String, Object?>{
      'version': 1,
      'transaction_id': body['transaction_id'],
      'applied_request_hash': body['request_hash'],
      'applied_audit_id': applied.auditId,
      'applied_request': appliedRequest,
      'workspace_id': selected.id,
      'registration_hash': selected.hash,
      'approval_id': selected.approval,
    };
    final response = await _request(operation, payload);
    final result = _map(response['body']);
    _keys(result, [
      'version', 'operation', 'request_hash', 'applied_request_hash',
      'applied_audit_id', 'workspace_id', 'review_audit_id', 'diff_hash',
      'transaction_id', 'changed_files', 'status',
    ]);
    final auditId = _string(response['audit_event_id']);
    if (response['evidence_source'] != 'LIVE_RUNTIME' ||
        !_auditEventId.hasMatch(auditId) ||
        auditId == _unavailableAuditEventId ||
        result['version'] != 1 ||
        result['operation'] != operation ||
        result['request_hash'] != brokerPayloadHash(payload) ||
        result['applied_request_hash'] != body['request_hash'] ||
        result['applied_audit_id'] != applied.auditId ||
        result['workspace_id'] != selected.id ||
        result['review_audit_id'] != body['review_audit_id'] ||
        result['diff_hash'] != body['diff_hash'] ||
        result['transaction_id'] != body['transaction_id'] ||
        result['changed_files'] != body['changed_files'] ||
        result['status'] != 'rolled_back') {
      _invalid();
    }
    return WorkspaceView(matches.single, operation, '', result, auditId);
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
                  utf8.encode(text).length != count ||
                  brokerTextHash(text) != projection['sha256']) {
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
  final contents = {'before': StringBuffer(), 'after': StringBuffer()};
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
      final bytes = utf8.encode(text).length;
      final hasNewline = line['newline'] == true;
      sizes[side] = sizes[side]! + bytes + (hasNewline ? 1 : 0);
      ended[side] = !hasNewline;
      totalBytes += bytes;
      if (totalBytes > 131072) {
        _invalid();
      }
      contents[side]!.write(text);
      if (hasNewline) contents[side]!.write('\n');
    }
    if (row['kind'] == 'same' &&
        ((row['before'] as Map)['text'] != (row['after'] as Map)['text'] ||
            (row['before'] as Map)['newline'] !=
                (row['after'] as Map)['newline'])) {
      _invalid();
    }
  }
  for (final side in ['before', 'after']) {
    final version = diff[side] as Map?;
    if (version == null
        ? numbers[side] != 0
        : version['bytes'] != sizes[side] ||
            version['sha256'] != brokerTextHash(contents[side]!.toString())) {
      _invalid();
    }
    if (numbers[side]! > 2000 || sizes[side]! > 65536) {
      _invalid();
    }
  }
  _validateUnifiedDiff(
    unified,
    beforeExists: diff['before'] != null,
    afterExists: diff['after'] != null,
    beforeLines: numbers['before']!,
    afterLines: numbers['after']!,
    beforeText: contents['before']!.toString(),
    afterText: contents['after']!.toString(),
  );
}

/// Brokerの固定headerと全体hunkを照合し、両版を復元する。
void _validateUnifiedDiff(
  String unified, {
  required bool beforeExists,
  required bool afterExists,
  required int beforeLines,
  required int afterLines,
  required String beforeText,
  required String afterText,
}) {
  final header = StringBuffer('diff --git a/file b/file\n');
  if (!beforeExists) header.writeln('new file mode 100644');
  if (!afterExists) header.writeln('deleted file mode 100644');
  if (beforeLines != 0 || afterLines != 0) {
    header.writeln('--- ${beforeExists ? 'a/file' : '/dev/null'}');
    header.writeln('+++ ${afterExists ? 'b/file' : '/dev/null'}');
    header.writeln('@@ -${beforeLines == 0 ? 0 : 1},$beforeLines '
        '+${afterLines == 0 ? 0 : 1},$afterLines @@');
  }
  final prefix = header.toString();
  if (!unified.startsWith(prefix)) _invalid();

  final lines = unified.substring(prefix.length).split('\n');
  if (lines.last.isNotEmpty) _invalid();
  final before = StringBuffer();
  final after = StringBuffer();
  var beforeCount = 0;
  var afterCount = 0;
  for (var index = 0; index < lines.length - 1; index++) {
    final line = lines[index];
    if (line.isEmpty || !{' ', '-', '+'}.contains(line[0])) _invalid();
    final noNewline = lines[index + 1] == r'\ No newline at end of file';
    final text = line.substring(1) + (noNewline ? '' : '\n');
    if (line[0] != '+') {
      if (noNewline && beforeCount != beforeLines - 1) _invalid();
      beforeCount++;
      before.write(text);
    }
    if (line[0] != '-') {
      if (noNewline && afterCount != afterLines - 1) _invalid();
      afterCount++;
      after.write(text);
    }
    if (noNewline) index++;
  }
  if (beforeCount != beforeLines ||
      afterCount != afterLines ||
      before.toString() != beforeText ||
      after.toString() != afterText) {
    _invalid();
  }
}
