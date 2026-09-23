import 'dart:async';

import 'package:flutter/services.dart';

import '../models/generated_contracts.dart';
import 'broker_client.dart';
import 'notification_client.dart';

typedef WindowsTrayActionHandler = FutureOr<void> Function(String action);

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

  static Future<WindowsTrayProjection> fromSnapshot(
    ShellSnapshot snapshot,
    BrokerTransport? transport,
  ) async {
    final brokerSnapshot = snapshot.snapshotSource == 'broker';
    Object criticalCount = '不明';
    if (transport != null) {
      try {
        final body = await NotificationClient(transport).list(
          unreadOnly: true,
          limit: 256,
        );
        final value = body['重大件数'];
        if (value is int && value >= 0 && value <= 256) {
          criticalCount = value;
        }
      } on Object {
        criticalCount = '不明';
      }
    }
    return WindowsTrayProjection(
      runtimeStatus: brokerSnapshot
          ? snapshot.operationStatus.runtimeStatus
          : '不明',
      pendingApprovalCount: brokerSnapshot
          ? snapshot.operationStatus.pendingApprovalsCount
          : '不明',
      criticalNotificationCount: criticalCount,
      evidenceSource: brokerSnapshot ? 'INTERNAL_STATE' : '不明',
      stopRequestSupported: transport != null,
    );
  }
}

class WindowsTrayClient {
  WindowsTrayClient({MethodChannel? channel})
      : _channel = channel ?? const MethodChannel('gui_shell/tray');

  final MethodChannel _channel;
  WindowsTrayActionHandler? _actionHandler;
  bool _available = false;

  bool get available => _available;

  Future<void> start({required WindowsTrayActionHandler onAction}) async {
    _actionHandler = onAction;
    _channel.setMethodCallHandler(_handleMethodCall);
    try {
      await _channel.invokeMethod<void>('initialize');
      _available = true;
    } on MissingPluginException {
      _available = false;
    } on PlatformException {
      _available = false;
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

  Future<void> dispose() async {
    _actionHandler = null;
    _channel.setMethodCallHandler(null);
    _available = false;
  }

  Future<Object?> _handleMethodCall(MethodCall call) async {
    if (call.method == 'onTrayAction' && call.arguments is String) {
      await _actionHandler?.call(call.arguments as String);
    }
    return null;
  }
}
