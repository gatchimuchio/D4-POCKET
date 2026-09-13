import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart';

import 'device_link_client.dart';

abstract class DeviceStore {
  Future<String?> read(String key);
  Future<void> write(String key, String value);
  Future<void> delete(String key);
}

class SecureDeviceStore implements DeviceStore {
  final _storage = const FlutterSecureStorage(
    aOptions: AndroidOptions(resetOnError: false),
    iOptions: IOSOptions(
      accessibility: KeychainAccessibility.unlocked_this_device,
    ),
  );
  void _platform() {
    if (!Platform.isAndroid && !Platform.isIOS) {
      throw const BrokerClientException('この保管経路はAndroidとiOS専用です。');
    }
  }

  @override
  Future<String?> read(String key) {
    _platform();
    return _storage.read(key: key);
  }

  @override
  Future<void> write(String key, String value) {
    _platform();
    return _storage.write(key: key, value: value);
  }

  @override
  Future<void> delete(String key) {
    _platform();
    return _storage.delete(key: key);
  }
}

/// 接続表示とlifecycleの調停。実操作の権限判定は常にDesktopに残す。
class DeviceLinkController extends ChangeNotifier implements BrokerTransport {
  DeviceLinkController({
    DeviceStore? store,
    DeviceLinkClient Function(DeviceCredential)? connect,
  }) : _store = store ?? SecureDeviceStore(),
       _connect = connect ?? DeviceLinkClient.new;
  final DeviceStore _store;
  final DeviceLinkClient Function(DeviceCredential) _connect;
  static const deviceKey = 'gui_shell_device_v1';
  static const credentialKey = 'gui_shell_pair_v1';
  String? deviceId;
  DeviceCredential? credential;
  DeviceLinkClient? _client;
  bool busy = false;
  bool ready = false;
  bool foreground = true;
  bool storageReady = false;
  bool hasStoredCredential = false;
  bool _resumePending = false;
  bool _disposed = false;
  int _foregroundGeneration = 0;
  Timer? _expiry;
  String status = '安全保管を確認中';
  final List<String> events = [];
  List<String> runtimes = [];
  RuntimeDialogueClient get dialogue => RuntimeDialogueClient(this);

  void _report(String message) {
    status = message;
    events.insert(0, message);
    if (events.length > 20) events.removeLast();
    if (!_disposed) notifyListeners();
  }

  void _stop() {
    ready = false;
    _client?.setActive(false);
  }

  void _install(DeviceCredential value) {
    if (_disposed) throw const BrokerClientException('画面終了');
    _client?.setActive(false);
    credential = value;
    _client = _connect(value)..setActive(foreground);
    _expiry?.cancel();
    final remaining =
        value.expiry * 1000 - DateTime.now().millisecondsSinceEpoch;
    _expiry = Timer(Duration(milliseconds: remaining > 0 ? remaining : 0), () {
      _stop();
      _report('端末資格の期限が切れました。Desktopで失効を確認し、新しい招待を発行してください。');
    });
  }

  Future<void> initialize() async {
    if (busy || storageReady || _disposed) return;
    busy = true;
    try {
      var id = await _store.read(deviceKey);
      if (id == null) {
        id = newDeviceId();
        await _store.write(deviceKey, id);
        if (await _store.read(deviceKey) != id) throw const FormatException();
      }
      if (!RegExp(r'^[0-9a-f]{32}$').hasMatch(id))
        throw const FormatException();
      deviceId = id;
      final stored = await _store.read(credentialKey);
      hasStoredCredential = stored != null;
      storageReady = true;
      if (stored == null) {
        _report('未接続。Desktopでこの端末IDへの招待を発行してください。');
      } else {
        _install(
          DeviceCredential.parse(stored, invitation: false, deviceId: id),
        );
        await _verify();
      }
    } catch (_) {
      _stop();
      _report(
        storageReady
            ? '保存資格を使用できません。期限・Hostを確認し、保存資格を削除して再結合してください。'
            : '安全保管を利用できません。保存・接続は停止しました。端末のロック解除と安全保管を確認してください。',
      );
    } finally {
      _finish();
    }
  }

  DeviceCredential invitation(String source) {
    if (!storageReady ||
        deviceId == null ||
        credential != null ||
        hasStoredCredential)
      throw const BrokerClientException('現在の結合を解除してから招待を読み込んでください。');
    return DeviceCredential.parse(
      source,
      invitation: true,
      deviceId: deviceId!,
    );
  }

  Future<void> pair(DeviceCredential invite) async {
    if (busy ||
        !foreground ||
        !storageReady ||
        hasStoredCredential ||
        credential != null ||
        invite.text('端末ID') != deviceId)
      return;
    busy = true;
    _report('確認したHostと結合中');
    final candidate = _connect(invite);
    _client = candidate;
    try {
      final value = await candidate.pair();
      _install(value);
      try {
        await _store.write(credentialKey, jsonEncode(value.data));
        if (await _store.read(credentialKey) != jsonEncode(value.data))
          throw const FormatException();
        hasStoredCredential = true;
      } catch (_) {
        // 保管不成立を接続成功にしない。失効できない場合もownerへ明示する。
        var revoked = false;
        try {
          await _client!.request('端末離脱');
          revoked = true;
        } catch (_) {
          /* 通信不能 */
        }
        _stop();
        _report(
          revoked
              ? '安全保管に失敗し、Desktopの結合を解除しました。保存資格を削除してください。'
              : '安全保管に失敗しました。Desktopでこの端末を失効させ、保存資格を削除してください。',
        );
        return;
      }
      await _verify();
    } catch (_) {
      _stop();
      _report('結合を確認できません。招待を再送せず、Desktopで端末一覧を確認して新しい招待を発行してください。');
    } finally {
      candidate.setActive(false);
      if (identical(_client, candidate)) _client = null;
      _finish();
    }
  }

  Future<void> _verify() async {
    ready = false;
    if (!foreground || _disposed || _client == null) return;
    final generation = _foregroundGeneration;
    bool currentForeground() =>
        foreground && !_disposed && generation == _foregroundGeneration;
    final client = _client!;
    final current = credential!;
    final storedId = await _store.read(deviceKey);
    final stored = await _store.read(credentialKey);
    if (!currentForeground()) return;
    if (storedId != deviceId || stored == null)
      throw const BrokerClientException('安全保管の資格を確認できません');
    final confirmed = DeviceCredential.parse(
      stored,
      invitation: false,
      deviceId: deviceId!,
    );
    if (!mapEquals(confirmed.data, current.data) || current.expired)
      throw const BrokerClientException('保存資格が現在の結合と一致しません');
    client.setActive(true);
    await client.request('端末確認');
    if (!currentForeground()) return;
    final observed = await RuntimeDialogueClient(client).runtimes();
    if (!currentForeground()) return;
    runtimes = observed;
    ready = true;
    _report('Desktopへの端末資格を確認済み');
  }

  Future<void> verify() async {
    if (busy || !foreground || _client == null) return;
    busy = true;
    _stop();
    _report('Desktopに資格を再確認中');
    try {
      await _verify();
    } catch (_) {
      _stop();
      _report('接続・資格を確認できません。Host、期限、Desktopの失効状態を確認してください。');
    } finally {
      _finish();
    }
  }

  void setForeground(bool value) {
    if (_disposed) return;
    foreground = value;
    if (!value) {
      _foregroundGeneration++;
      _stop();
      _report('バックグラウンド中は通信停止');
    } else {
      if (busy) {
        _resumePending = true;
      } else {
        unawaited(storageReady ? verify() : initialize());
      }
    }
  }

  Future<void> disconnect({bool localOnly = false}) async {
    if (busy || !storageReady) return;
    busy = true;
    ready = false;
    try {
      if (!localOnly) {
        if (_client == null) throw const BrokerClientException('失効確認用の資格なし');
        if (!foreground) throw const BrokerClientException('通信停止中');
        _client!.setActive(true);
        await _client!.request('端末離脱');
      }
      _stop();
      await _store.delete(credentialKey);
      if (await _store.read(credentialKey) != null)
        throw const BrokerClientException('保存資格の削除未確認');
      hasStoredCredential = false;
      credential = null;
      _client = null;
      runtimes = [];
      _expiry?.cancel();
      _report(
        localOnly
            ? '端末内の資格を削除しました。Desktop側の失効は未確認です。ownerが端末一覧から失効させてください。'
            : 'Desktopの結合を解除し、保存資格を削除しました。送信済み実行系の処理停止は保証しません。',
      );
    } catch (_) {
      _stop();
      _report('解除または保存資格の削除を確認できません。Desktopで失効を確認してください。');
    } finally {
      _finish();
    }
  }

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    if (!ready || !foreground || _client == null)
      throw const BrokerClientException('接続の確認待ちです。');
    try {
      return await _client!.request(operation, payload: payload);
    } catch (_) {
      _stop();
      _report('操作を確認できません。送信を止めました。接続先画面で資格を再確認してください。');
      rethrow;
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _expiry?.cancel();
    _stop();
    super.dispose();
  }

  void _finish() {
    busy = false;
    if (!_disposed) {
      notifyListeners();
      if (_resumePending && foreground) {
        _resumePending = false;
        unawaited(storageReady ? verify() : initialize());
      }
    }
  }
}
