import 'dart:convert';

import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

const Map<String, String> guiShellOptionalExportModules = {
  'shell.setup_doctor': '環境診断',
  'shell.history': '実行履歴',
  'shell.evaluation_lab': '評価ラボ',
  'shell.host_capabilities': 'ホスト能力',
  'shell.notifications': '通知',
  'shell.observability': '観測',
  'shell.trace_inspector': '追跡情報',
  'shell.host_operations': 'Host操作',
};

class ExportClient {
  const ExportClient(this._transport);

  final BrokerTransport _transport;

  Future<Map<String, Object?>> export({
    required String exportId,
    required Map<String, Object?> composeManifest,
    String distributionChannel = 'local',
    List<String>? optionalModuleIds,
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
        if (optionalModuleIds != null)
          'module_selection': {
            'optional_module_ids': optionalModuleIds,
          },
      },
    );
    if (response['status'] != 'accepted') {
      final error = response['error'];
      if (error is Map && error['code'] == 'owner_required') {
        throw const BrokerClientException(
          'Owner確認が完了しなかったため、書出しは拒否されました。',
        );
      }
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
