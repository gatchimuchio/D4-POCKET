import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;

class ObservationClient {
  const ObservationClient(this._transport);

  final BrokerTransport _transport;

  Future<Map<String, Object?>> list({
    int limit = 200,
    String? traceId,
  }) async {
    final payload = <String, Object?>{
      '版': 1,
      '上限': limit,
      if (traceId != null && traceId.isNotEmpty) 'TraceID': traceId,
    };
    final response = await _transport.request('観測一覧', payload: payload);
    if (response['status'] != 'accepted') {
      final error = response['error'];
      final message = error is Map
          ? error['message']?.toString() ?? response.toString()
          : response.toString();
      throw BrokerClientException('観測一覧が拒否されました: $message');
    }
    final body = response['body'];
    if (body is! Map) {
      throw const BrokerClientException('観測一覧応答bodyがobjectではありません');
    }
    return Map<String, Object?>.from(body);
  }
}
