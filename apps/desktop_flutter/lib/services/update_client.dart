import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;

class UpdateClient {
  const UpdateClient(this._transport);

  final BrokerTransport _transport;

  static String signatureStatusLabel(Object? status) => switch (status) {
        'verified' => '現在のBroker trustで検証済み',
        'verification_stale' => '現在のBroker trustで未検証',
        'legacy_unbound' => '旧候補（package未結合）',
        _ => '不明',
      };

  static bool matchesDownloadedCandidate(
    Object? value, {
    required String updateId,
    required String candidateHash,
    required String packageSha256,
  }) =>
      value is Map &&
      value['状態'] == 'downloaded' &&
      value['更新ID'] == updateId &&
      value['候補hash'] == candidateHash &&
      value['package_sha256'] == packageSha256;

  static bool canRepairActiveVersion({
    required bool alreadyActive,
    required bool downloaded,
    required bool operationInProgress,
    required Object? signatureStatus,
    required Object? packageSha256,
    required Object? packageSize,
  }) =>
      alreadyActive &&
      downloaded &&
      !operationInProgress &&
      signatureStatus == 'verified' &&
      packageSha256 is String &&
      RegExp(r'^[a-f0-9]{64}$').hasMatch(packageSha256) &&
      packageSize is num &&
      packageSize > 0;

  static bool canChangeActiveVersion(Map<String, Object?>? rollbackState) =>
      rollbackState?['状態'] == 'available' ||
      rollbackState?['状態'] == 'unavailable';

  static bool canRequestActivation({
    required Map<String, Object?>? rollbackState,
    required bool staged,
    required bool alreadyActive,
    required bool rollbackTarget,
    required Object? signatureStatus,
    required Object? packageSha256,
    required Object? packageSize,
  }) =>
      staged &&
      !alreadyActive &&
      !rollbackTarget &&
      canChangeActiveVersion(rollbackState) &&
      signatureStatus == 'verified' &&
      packageSha256 is String &&
      RegExp(r'^[a-f0-9]{64}$').hasMatch(packageSha256) &&
      packageSize is num &&
      packageSize > 0;

  static bool isFirstInstall(Map<String, Object?>? rollbackState) =>
      rollbackState?['状態'] == 'unavailable' && rollbackState?['現在版'] == null;

  static bool shouldLaunchInstalledVersionAfterExit(
    Map<String, Object?> body,
  ) =>
      body['起動'] == 'after_current_exit';

  static String activationActionLabel(
    Map<String, Object?>? rollbackState, {
    required bool alreadyActive,
  }) {
    if (alreadyActive) return '現在の有効版';
    if (isFirstInstall(rollbackState)) return 'インストール';
    if (canChangeActiveVersion(rollbackState)) return '更新を有効化';
    return '導入状態不明';
  }

  static String packageSourceLabel(Object? value) {
    if (value is! Map) return '配布元状態不明';
    final status = value['状態'];
    final urlValue = value['URL'];
    if (status == 'unconfigured' && urlValue == null) {
      return '配布元未設定';
    }
    if (status == 'ineligible' && urlValue == null) {
      return '取得元なし（候補を現在trustで再検証できません）';
    }
    if (status != 'configured' || urlValue is! String) {
      return '配布元状態不明';
    }
    final uri = Uri.tryParse(urlValue);
    if (uri == null ||
        uri.scheme != 'https' ||
        uri.host.isEmpty ||
        uri.userInfo.isNotEmpty ||
        uri.hasQuery ||
        uri.hasFragment) {
      return '配布元状態不明';
    }
    return '配布元=${uri.host}';
  }

  static String downloadJobLabel(Object? value) {
    if (value is! Map) return 'download jobなし';
    final state = value['状態'];
    final received = value['受信byte数'];
    final total = value['全byte数'];
    if (state == 'downloading' &&
        received is num &&
        total is num &&
        total > 0) {
      final percent = (received / total * 100).clamp(0, 100).round();
      return 'download中 $percent% ($received / $total bytes)';
    }
    return switch (state) {
      'downloaded' => 'download済み（installは別途保留）',
      'failed' => 'download失敗: ${_downloadFailureLabel(value['失敗code'])}',
      'audit_failed' => 'download結果のAudit失敗。復旧確認が必要',
      _ => 'download状態不明',
    };
  }

  static String _downloadFailureLabel(Object? code) => switch (code) {
        'update_download_request_invalid' => '要求が不正',
        'update_download_dns_failed' => '配布元の名前解決に失敗',
        'update_download_address_blocked' => '許可されない接続先を検出',
        'update_download_network_failed' => 'HTTPS通信に失敗',
        'update_download_response_rejected' => '応答状態を拒否',
        'update_download_headers_invalid' => '応答headerが契約外',
        'update_download_size_mismatch' => '受信byte長が不一致',
        'update_download_digest_mismatch' => 'SHA-256が不一致',
        'update_download_cancelled' => 'downloadを中断',
        'update_download_timeout' => '通信期限を超過',
        'update_download_storage_failed' => 'Broker保管処理に失敗',
        'update_download_result_mismatch' => '完了結果が不一致',
        'update_download_audit_failed' => 'Audit確定に失敗',
        _ => '失敗code不明',
      };

  Future<Map<String, Object?>> list() async {
    final response = await _transport.request('更新一覧', payload: const {'版': 1});
    return _acceptedBody(response, '更新一覧');
  }

  Future<Map<String, Object?>> fetchCandidates() =>
      _request('更新候補取得', const {'版': 1});

  Future<Map<String, Object?>> verify(
    Map<String, Object?> candidate,
  ) =>
      _request('更新署名検査', {'版': 1, '候補': candidate});

  Future<Map<String, Object?>> requestDownload({
    required String updateId,
    required String candidateHash,
  }) =>
      _request('更新download要求', {
        '版': 1,
        '更新ID': updateId,
        '候補hash': candidateHash,
      });

  Future<Map<String, Object?>> requestApply({
    required String updateId,
    required String candidateHash,
  }) =>
      _request('更新適用要求', {
        '版': 1,
        '更新ID': updateId,
        '候補hash': candidateHash,
      });

  Future<Map<String, Object?>> requestActivation({
    required String updateId,
    required String candidateHash,
  }) =>
      _request('更新有効版切替要求', {
        '版': 1,
        '更新ID': updateId,
        '候補hash': candidateHash,
      });

  Future<Map<String, Object?>> defer({
    required String updateId,
    required String candidateHash,
    required String deferredUntil,
  }) =>
      _request('更新延期', {
        '版': 1,
        '更新ID': updateId,
        '候補hash': candidateHash,
        '延期期限': deferredUntil,
      });

  Future<Map<String, Object?>> requestRollback({
    required String updateId,
    required String candidateHash,
  }) =>
      _request('更新rollback要求', {
        '版': 1,
        '更新ID': updateId,
        '候補hash': candidateHash,
      });

  Future<Map<String, Object?>> requestUninstall() =>
      _request('製品アンインストール要求', const {'版': 1});

  Future<Map<String, Object?>> requestProductRepair() =>
      _request('製品起動項目修復要求', const {'版': 1});

  Future<Map<String, Object?>> _request(
    String operation,
    Map<String, Object?> payload,
  ) async {
    final response = await _transport.request(operation, payload: payload);
    if (response['status'] != 'accepted' && response['status'] != 'suspended') {
      final error = response['error'];
      final message = error is Map
          ? error['message']?.toString() ?? response.toString()
          : response.toString();
      throw BrokerClientException('$operation が拒否されました: $message');
    }
    final body = response['body'];
    if (body is! Map) {
      throw BrokerClientException('$operation 応答bodyがobjectではありません');
    }
    return Map<String, Object?>.from(body);
  }
}

Map<String, Object?> _acceptedBody(
  Map<String, Object?> response,
  String operation,
) {
  if (response['status'] != 'accepted') {
    final error = response['error'];
    final message = error is Map
        ? error['message']?.toString() ?? response.toString()
        : response.toString();
    throw BrokerClientException('$operation が拒否されました: $message');
  }
  final body = response['body'];
  if (body is! Map) {
    throw BrokerClientException('$operation 応答bodyがobjectではありません');
  }
  return Map<String, Object?>.from(body);
}
