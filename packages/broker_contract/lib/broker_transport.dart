/// Request port implemented by an authenticated host transport.
///
/// The interface carries requests to the existing Broker. It does not grant
/// capability, permission, approval, or access to credentials by itself.
abstract class BrokerTransport {
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  });
}

class BrokerClientException implements Exception {
  const BrokerClientException(this.message);

  final String message;

  @override
  String toString() => message;
}
