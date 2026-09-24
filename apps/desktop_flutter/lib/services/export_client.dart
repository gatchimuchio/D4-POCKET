import 'dart:convert';

import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

class ExportClient {
  const ExportClient(this._transport);

  final BrokerTransport _transport;

  Future<Map<String, Object?>> export({
    required String exportId,
    required Map<String, Object?> composeManifest,
    String distributionChannel = 'local',
  }) async {
    final response = await _transport.request(
      'GUI Shell書出し',
      payload: {
        'version': 1,
        'export_id': exportId,
        'compose_manifest': composeManifest,
        'target_platform': 'windows',
        'export_mode': 'manifest_only',
        'distribution_channel': distributionChannel,
      },
    );
    if (response['status'] != 'accepted') {
      final error = response['error'];
      final message = error is Map
          ? error['message']?.toString() ?? response.toString()
          : response.toString();
      throw BrokerClientException('GUI Shell書出しが拒否されました: $message');
    }
    final body = response['body'];
    if (body is! Map) {
      throw const BrokerClientException('GUI Shell書出し応答bodyがobjectではありません');
    }
    return Map<String, Object?>.from(body);
  }
}

String exportJson(Map<String, Object?> value) =>
    const JsonEncoder.withIndent('  ').convert(value);
