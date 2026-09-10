/// 表示層が使う要求口。実装先はDesktop brokerまたは認証済み端末連携に限る。
abstract class BrokerTransport {
  Future<Map<String, Object?>> request(String operation,
      {Map<String, Object?>? payload});
}

class BrokerClientException implements Exception {
  const BrokerClientException(this.message);
  final String message;
  @override
  String toString() => message;
}
