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
  bool foreground = true;
  bool storageReady = false;
  bool hasStoredCredential = false;
  bool _resumePending = false;
  bool _disposed = false;
  int _foregroundGeneration = 0;
  int connectionGeneration = 0;
  String status = 'native安全保管を確認中';
  final List<String> events = [];
  List<String> runtimes = [];
  RuntimeDialogueClient get dialogue => RuntimeDialogueClient(this);

  void _report(String message) {
    status = message;
    events.insert(0, message);
    if (events.length > 20) events.removeLast();
    if (!_disposed) notifyListeners();
  }

  void _apply(DeviceLinkSnapshot snapshot) {
    final wasReady = ready;
    storageReady = snapshot.storageReady;
    hasStoredCredential = snapshot.paired;
    foreground = snapshot.foreground;
    ready = snapshot.connected && snapshot.foreground;
    status = snapshot.status;
    if (!wasReady && ready) connectionGeneration++;
    if (events.isEmpty || events.first != status) {
      events.insert(0, status);
      if (events.length > 20) events.removeLast();
    }
    if (!_disposed) notifyListeners();
  }

  Future<void> initialize() async {
    if (busy || storageReady || _disposed) return;
    busy = true;
    notifyListeners();
    try {
      _apply(await _native.readState());
      if (ready) await _refreshRuntimes();
    } catch (_) {
      ready = false;
      runtimes = [];
      _report('native安全保管またはDesktop接続を確認できません。通信を停止しました。');
    } finally {
      _finish();
    }
  }

  Future<void> _refreshRuntimes() async {
    final values = await dialogue.runtimes();
    if (!ready || !foreground || _disposed) return;
    runtimes = values;
  }

  Future<void> verify() async {
    if (busy || !foreground || !storageReady || !hasStoredCredential) return;
    busy = true;
    ready = false;
    notifyListeners();
    final generation = _foregroundGeneration;
    try {
      final snapshot = await _native.readState();
      if (generation != _foregroundGeneration || !foreground || _disposed)
        return;
      _apply(snapshot);
      if (ready) await _refreshRuntimes();
    } catch (_) {
      ready = false;
      runtimes = [];
      _report('資格・期限・証明書またはDesktop接続を確認できません。操作を停止しました。');
    } finally {
      _finish();
    }
  }

  Future<void> pair() async {
    if (busy ||
        !foreground ||
        !storageReady ||
        hasStoredCredential ||
        _disposed) {
      return;
    }
    busy = true;
    _report('端末結合をnative画面で確認しています');
    try {
      final snapshot = await _native.pair();
      _apply(snapshot);
      if (ready) await _refreshRuntimes();
    } catch (_) {
      ready = false;
      runtimes = [];
      _report('結合を確認できません。招待を再利用せず、Desktop側で端末状態を確認してください。');
    } finally {
      _finish();
    }
  }

  void setForeground(bool value) {
    if (_disposed || foreground == value) return;
    foreground = value;
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
    if (value && storageReady && hasStoredCredential) {
      unawaited(_resumeNativeConnection());
    } else if (!_disposed) {
      notifyListeners();
    }
  }

  Future<void> _resumeNativeConnection() async {
    busy = true;
    notifyListeners();
    final generation = _foregroundGeneration;
    try {
      final snapshot = await _native.readState();
      if (generation != _foregroundGeneration || !foreground || _disposed)
        return;
      _apply(snapshot);
      if (ready) await _refreshRuntimes();
    } catch (_) {
      ready = false;
      runtimes = [];
      _report('復帰後のnative通信を確認できません。操作を停止しました。');
    } finally {
      _finish();
    }
  }

  Future<void> disconnect({bool localOnly = false}) async {
    if (busy || !storageReady || !hasStoredCredential) return;
    busy = true;
    ready = false;
    runtimes = [];
    notifyListeners();
    try {
      _apply(
        localOnly ? await _native.localDelete() : await _native.disconnect(),
      );
    } catch (_) {
      ready = false;
      _report(
        localOnly
            ? '端末内資格の削除を確認できません。通信を停止しました。'
            : 'Desktop失効または端末内資格の削除を確認できません。owner側の失効を確認してください。',
      );
    } finally {
      _finish();
    }
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
    try {
      final response = await _native.request(operation, payload: payload);
      if (generation != _foregroundGeneration || !foreground || !ready) {
        throw const BrokerClientException('画面状態が変化したため操作を停止しました。');
      }
      return response;
    } catch (_) {
      ready = false;
      runtimes = [];
      _report('操作結果を確認できません。自動再送せず、接続先と監査状態を確認してください。');
      rethrow;
    }
  }

  @override
  void dispose() {
    _disposed = true;
    foreground = false;
    _foregroundGeneration++;
    ready = false;
    runtimes = [];
    super.dispose();
  }

  void _finish() {
    busy = false;
    if (!_disposed) {
      notifyListeners();
      if (_resumePending && foreground && storageReady && hasStoredCredential) {
        _resumePending = false;
        unawaited(_resumeNativeConnection());
      } else {
        _resumePending = false;
      }
    }
  }
}
