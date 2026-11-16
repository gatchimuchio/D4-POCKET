import 'dart:async';

import 'package:flutter/services.dart';

import '../module_features.dart';
import 'broker_client.dart';
import 'broker_health_client.dart';
import 'notification_client.dart';

typedef WindowsTrayActionHandler = FutureOr<void> Function(String action);
typedef WindowsTrayVisibilityHandler = void Function(bool visible);

class WindowsTrayProjection {
  const WindowsTrayProjection({
    required this.runtimeStatus,
    required this.pendingApprovalCount,
    required this.criticalNotificationCount,
    required this.evidenceSource,
    required this.stopRequestSupported,
  });

  final String runtimeStatus;
  final Object pendingApprovalCount;
  final Object criticalNotificationCount;
  final String evidenceSource;
  final bool stopRequestSupported;

  Map<String, Object?> toJson() => {
        'version': 1,
        'runtime_status': runtimeStatus,
        'pending_approval_count': pendingApprovalCount,
        'critical_notification_count': criticalNotificationCount,
        'evidence_source': evidenceSource,
        'stop_request_supported': stopRequestSupported,
      };

  static Future<WindowsTrayProjection> fromBroker(
    BrokerTransport? transport, {
    bool windowVisible = true,
  }) async {
    if (!windowVisible) {
      return unavailable(stopRequestSupported: transport != null);
    }
    var runtimeStatus = '不明';
    var evidenceSource = '不明';
    if (transport != null) {
      try {
        final health = await BrokerHealthClient.read(transport);
        runtimeStatus =
            health.uiStatus == BrokerUiStatus.ready ? 'ready' : 'suspend';
        // windows_tray_projection classifies its bounded Broker projection
        // as INTERNAL_STATE; the health envelope's source was validated above.
        evidenceSource = 'INTERNAL_STATE';
      } on Object {
        runtimeStatus = '不明';
        evidenceSource = '不明';
      }
    }
    Object criticalCount = '不明';
    if (transport != null && kGuiShellModuleNotifications) {
      try {
        final body = await NotificationClient(transport).list(
          unreadOnly: true,
          limit: 256,
        );
        final value = body['重大件数'];
        if (value is int && value >= 0 && value <= 256) {
          criticalCount = value;
          evidenceSource = 'INTERNAL_STATE';
        }
      } on Object {
        criticalCount = '不明';
      }
    }
    return WindowsTrayProjection(
      runtimeStatus: runtimeStatus,
      pendingApprovalCount: '不明',
      criticalNotificationCount: criticalCount,
      evidenceSource: evidenceSource,
      stopRequestSupported: transport != null,
    );
  }

  static WindowsTrayProjection unavailable({
    required bool stopRequestSupported,
  }) =>
      WindowsTrayProjection(
        runtimeStatus: '不明',
        pendingApprovalCount: '不明',
        criticalNotificationCount: '不明',
        evidenceSource: '不明',
        stopRequestSupported: stopRequestSupported,
      );
}

class WindowsTrayClient {
  WindowsTrayClient({MethodChannel? channel})
      : _channel = channel ?? const MethodChannel('gui_shell/tray');

  final MethodChannel _channel;
  WindowsTrayActionHandler? _actionHandler;
  WindowsTrayVisibilityHandler? _visibilityHandler;
  bool _available = false;
  int _availabilityGeneration = 0;

  bool get available => _available;

  Future<bool> start({
    required WindowsTrayActionHandler onAction,
    WindowsTrayVisibilityHandler? onVisibilityChanged,
  }) async {
    _actionHandler = onAction;
    _visibilityHandler = onVisibilityChanged;
    _channel.setMethodCallHandler(_handleMethodCall);
    try {
      final generation = _availabilityGeneration;
      final windowVisible = await _channel.invokeMethod<bool>('initialize');
      if (generation == _availabilityGeneration) {
        _available = windowVisible != null;
        if (_available) return windowVisible!;
        return false;
      }
      return await _refreshVisibility();
    } on MissingPluginException {
      _available = false;
      return false;
    } on PlatformException {
      _available = false;
      return false;
    }
  }

  Future<bool?> isWindowVisible() async {
    if (!_available) return null;
    try {
      final windowVisible =
          await _channel.invokeMethod<bool>('isWindowVisible');
      if (windowVisible == null) _available = false;
      return windowVisible;
    } on MissingPluginException {
      _available = false;
      return null;
    } on PlatformException {
      _available = false;
      return null;
    }
  }

  Future<void> publish(WindowsTrayProjection projection) async {
    if (!_available) return;
    try {
      await _channel.invokeMethod<void>('publish', projection.toJson());
    } on MissingPluginException {
      _available = false;
    } on PlatformException {
      _available = false;
    }
  }

  Future<bool> _refreshVisibility() async {
    if (!_available) return false;
    try {
      final visible = await _channel.invokeMethod<bool>('isWindowVisible');
      if (visible == null) {
        _available = false;
        _availabilityGeneration++;
        _visibilityHandler?.call(false);
        return false;
      }
      _visibilityHandler?.call(visible);
      return visible;
    } on MissingPluginException {
      _available = false;
      _availabilityGeneration++;
      _visibilityHandler?.call(false);
      return false;
    } on PlatformException {
      _available = false;
      _availabilityGeneration++;
      _visibilityHandler?.call(false);
      return false;
    }
  }

  Future<void> dispose() async {
    _actionHandler = null;
    _visibilityHandler = null;
    _channel.setMethodCallHandler(null);
    _available = false;
  }

  Future<Object?> _handleMethodCall(MethodCall call) async {
    if (call.method == 'onTrayAction' && call.arguments is String) {
      await _actionHandler?.call(call.arguments as String);
    } else if (call.method == 'onWindowVisibilityChanged' &&
        call.arguments is bool) {
      _visibilityHandler?.call(call.arguments as bool);
    } else if (call.method == 'onTrayAvailabilityChanged' &&
        call.arguments is bool) {
      _availabilityGeneration++;
      _available = call.arguments as bool;
      if (_available) {
        await _refreshVisibility();
      } else {
        _visibilityHandler?.call(false);
      }
    }
    return null;
  }
}
