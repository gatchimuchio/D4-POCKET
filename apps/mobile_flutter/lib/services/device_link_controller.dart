import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart';

import 'device_link_client.dart';

/// 状態調停だけを行う。招待・資格・TLS・保管はnativeに閉じる。
class DeviceLinkController extends ChangeNotifier implements BrokerTransport {
  DeviceLinkController({DeviceLinkNativePort? nativePort})
    : _native = nativePort ?? MethodChannelDeviceLink();

  final DeviceLinkNativePort _native;
  bool busy = false;
  bool ready = false;
  bool _uiForeground = true;
  bool _nativeForeground = true;
  bool get foreground => _uiForeground && _nativeForeground;
  bool storageReady = false;
  bool hasStoredCredential = false;
  bool _resumePending = false;
  bool _disposed = false;
  int _foregroundGeneration = 0;
  int connectionGeneration = 0;
  String status = 'native安全保管を確認中';
  final List<String> events = [];
  List<String> runtimes = [];
  RuntimeDialogueClient get dialogue =>
      RuntimeDialogueClient(this, deviceLinkWorkspaceProjection: true);

  void _report(String message) {
    status = message;
    events.insert(0, message);
    if (events.length > 20) events.removeLast();
    if (!_disposed) notifyListeners();
  }

  void _apply(DeviceLinkSnapshot snapshot) {
    storageReady = snapshot.storageReady;
    hasStoredCredential = snapshot.paired;
    _nativeForeground = snapshot.foreground;
    ready = snapshot.connected && foreground;
    status = snapshot.status;
    if (ready) connectionGeneration++;
    if (events.isEmpty || events.first != status) {
      events.insert(0, status);
      if (events.length > 20) events.removeLast();
    }
    if (!_disposed) notifyListeners();
  }

  bool _isCurrentForeground(int generation) =>
      !_disposed && _uiForeground && generation == _foregroundGeneration;

  Future<void> _runStateOperation(
    Future<DeviceLinkSnapshot> Function() operation,
    String failureMessage, {
    String? progressMessage,
    bool refreshRuntimes = true,
  }) async {
    final generation = _foregroundGeneration;
    busy = true;
    ready = false;
    runtimes = [];
    if (progressMessage == null) {
      notifyListeners();
    } else {
      _report(progressMessage);
    }
    try {
      if (!_isCurrentForeground(generation)) return;
      final snapshot = await operation();
      if (!_isCurrentForeground(generation)) return;
      _apply(snapshot);
      if (refreshRuntimes && ready) {
        final values = await dialogue.runtimes();
        if (_isCurrentForeground(generation) && ready) runtimes = values;
      }
    } catch (_) {
      if (_isCurrentForeground(generation)) {
        ready = false;
        runtimes = [];
        _report(failureMessage);
      }
    } finally {
      _finish();
    }
  }

  Future<void> initialize() async {
    if (busy || storageReady || !_uiForeground || _disposed) return;
    await _runStateOperation(
      _native.readState,
      'native安全保管またはDesktop接続を確認できません。通信を停止しました。',
    );
  }

  Future<void> verify() async {
    if (busy ||
        !foreground ||
        !storageReady ||
        !hasStoredCredential ||
        _disposed) {
      return;
    }
    await _runStateOperation(
      _native.readState,
      '資格・期限・証明書またはDesktop接続を確認できません。操作を停止しました。',
    );
  }

  Future<void> pair() async {
    if (busy ||
        !foreground ||
        !storageReady ||
        hasStoredCredential ||
        _disposed) {
      return;
    }
    await _runStateOperation(
      _native.pair,
      '結合を確認できません。招待を再利用せず、Desktop側で端末状態を確認してください。',
      progressMessage: '端末結合をnative画面で確認しています',
    );
  }

  void setForeground(bool value) {
    if (_disposed ||
        (value == _uiForeground && (!value || _nativeForeground))) {
      return;
    }
    _uiForeground = value;
    _foregroundGeneration++;
    if (!value) {
      ready = false;
      runtimes = [];
      _report('バックグラウンド中はnative通信を停止しています');
    }
    if (busy) {
      _resumePending = value;
      return;
    }
    if (value) {
      unawaited(_resumeNativeConnection());
    }
  }

  Future<void> _resumeNativeConnection() =>
      _runStateOperation(_native.readState, '復帰後のnative通信を確認できません。操作を停止しました。');

  Future<void> disconnect({bool localOnly = false}) async {
    if (busy ||
        !foreground ||
        !storageReady ||
        !hasStoredCredential ||
        _disposed) {
      return;
    }
    await _runStateOperation(
      localOnly ? _native.localDelete : _native.disconnect,
      localOnly
          ? '端末内資格の削除を確認できません。通信を停止しました。'
          : 'Desktop失効または端末内資格の削除を確認できません。owner側の失効を確認してください。',
      refreshRuntimes: false,
    );
  }

  Future<List<DeviceLinkLocalRecoveryAuditEvent>>
  readLocalRecoveryAudit() async {
    if (busy || !storageReady || !foreground || _disposed) {
      throw const BrokerClientException('端末内回復記録を読み取れません。状態を確認してください。');
    }
    return _native.readLocalRecoveryAudit();
  }

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    if (!ready || !foreground || _disposed) {
      throw const BrokerClientException('接続の確認待ちです。');
    }
    final generation = _foregroundGeneration;
    final connection = connectionGeneration;
    bool isCurrentConnection() =>
        _isCurrentForeground(generation) &&
        foreground &&
        ready &&
        connection == connectionGeneration;
    try {
      final response = await _native.request(operation, payload: payload);
      if (!isCurrentConnection()) {
        throw const BrokerClientException('画面状態が変化したため操作を停止しました。');
      }
      return response;
    } catch (_) {
      if (isCurrentConnection()) {
        ready = false;
        runtimes = [];
        _report('操作結果を確認できません。自動再送せず、接続先と監査状態を確認してください。');
      }
      rethrow;
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _uiForeground = false;
    _foregroundGeneration++;
    ready = false;
    runtimes = [];
    super.dispose();
  }

  void _finish() {
    busy = false;
    if (_disposed) return;
    final resumePending = _resumePending && _uiForeground;
    _resumePending = false;
    if (resumePending) {
      unawaited(_resumeNativeConnection());
    } else {
      notifyListeners();
    }
  }
}
