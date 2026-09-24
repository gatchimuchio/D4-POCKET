import CryptoKit
import Foundation
import Network
import Security

enum DeviceLinkTransportError: Error {
  case failed
}

private final class DeviceLinkResultLatch<Value> {
  private let lock = NSLock()
  private let semaphore = DispatchSemaphore(value: 0)
  private var result: Result<Value, Error>?

  func resolve(_ result: Result<Value, Error>) {
    lock.lock()
    guard self.result == nil else { lock.unlock(); return }
    self.result = result
    lock.unlock()
    semaphore.signal()
  }

  func wait(until deadline: DispatchTime) throws -> Value {
    guard semaphore.wait(timeout: deadline) == .success else { throw DeviceLinkTransportError.failed }
    lock.lock()
    let value = result
    lock.unlock()
    guard let value else { throw DeviceLinkTransportError.failed }
    return try value.get()
  }
}

final class DeviceLinkTLSClient {
  private let lock = NSLock()
  private var active: [ObjectIdentifier: NWConnection] = [:]
  private let queue = DispatchQueue(label: "org.gatchimuchio.gui-shell.device-link.tls", qos: .utility)

  func cancelAll() {
    lock.lock()
    let connections = Array(active.values)
    lock.unlock()
    connections.forEach { $0.forceCancel() }
  }

  func exchange(_ credential: DeviceLinkCredential,
                operation: String,
                payload: [String: Any],
                canSend: @escaping () -> Bool,
                allowCredentialResponse: Bool = false) throws -> [String: Any] {
    guard !credential.isExpired(), canSend(),
          let port = NWEndpoint.Port(rawValue: UInt16(credential.port)) else {
      throw DeviceLinkTransportError.failed
    }
    let nonce = try Self.randomID()
    let request: [String: Any] = [
      "版": 1,
      "HostID": credential.hostID,
      "端末ID": credential.deviceID,
      "資格ID": credential.credentialID,
      "資格秘密": credential.secret,
      "nonce": nonce,
      "発行時刻": Int64(Date().timeIntervalSince1970),
      "操作": operation,
      "内容": payload,
    ]
    var wire = try DeviceLinkStrictJSON.encodeObject(request, maximumBytes: 64 * 1024)
    wire.append(0x0A)
    let deadline = DispatchTime.now() + .seconds(5)
    let tls = NWProtocolTLS.Options()
    let security = tls.securityProtocolOptions
    sec_protocol_options_set_min_tls_protocol_version(security, .TLSv12)
    sec_protocol_options_set_max_tls_protocol_version(security, .TLSv13)
    sec_protocol_options_set_tls_resumption_enabled(security, false)
    sec_protocol_options_set_tls_tickets_enabled(security, false)
    sec_protocol_options_set_peer_authentication_required(security, true)
    Self.installPin(security, expectedHash: credential.certificateHash)
    let parameters = NWParameters(tls: tls, tcp: NWProtocolTCP.Options())
    let connection = NWConnection(host: NWEndpoint.Host(credential.host), port: port, using: parameters)
    let identity = ObjectIdentifier(connection)
    lock.lock()
    active[identity] = connection
    lock.unlock()
    let timeout = DispatchSource.makeTimerSource(queue: .global(qos: .utility))
    timeout.schedule(deadline: deadline)
    timeout.setEventHandler { connection.forceCancel() }
    timeout.resume()
    defer {
      timeout.cancel()
      lock.lock()
      active.removeValue(forKey: identity)
      lock.unlock()
      connection.cancel()
    }

    let ready = DeviceLinkResultLatch<Void>()
    connection.stateUpdateHandler = { state in
      switch state {
      case .ready:
        ready.resolve(.success(()))
      case .failed, .cancelled:
        ready.resolve(.failure(DeviceLinkTransportError.failed))
      case .setup, .waiting, .preparing:
        break
      @unknown default:
        ready.resolve(.failure(DeviceLinkTransportError.failed))
      }
    }
    connection.start(queue: queue)
    try ready.wait(until: deadline)
    guard canSend(), !credential.isExpired() else { throw DeviceLinkTransportError.failed }

    let sent = DeviceLinkResultLatch<Void>()
    connection.send(content: wire, contentContext: .defaultMessage, isComplete: true) { result in
      switch result {
      case .contentProcessed(let error):
        if error == nil { sent.resolve(.success(())) }
        else { sent.resolve(.failure(DeviceLinkTransportError.failed)) }
      @unknown default:
        sent.resolve(.failure(DeviceLinkTransportError.failed))
      }
    }
    try sent.wait(until: deadline)
    guard canSend(), !credential.isExpired() else { throw DeviceLinkTransportError.failed }
    let frame = try receiveSingleFrame(connection, deadline: deadline, canSend: canSend)
    let response = try DeviceLinkStrictJSON.parseObject(frame, maximumBytes: 4 * 1024 * 1024)
    try validateResponse(response, nonce: nonce, operation: operation, credential: credential,
                         allowCredentialResponse: allowCredentialResponse)
    return try sanitize(response)
  }

  private func receiveSingleFrame(_ connection: NWConnection, deadline: DispatchTime,
                                  canSend: () -> Bool) throws -> Data {
    var output = Data()
    var newlineSeen = false
    while true {
      guard canSend() else { throw DeviceLinkTransportError.failed }
      let received = DeviceLinkResultLatch<(Data, Bool)>()
      connection.receive(minimumIncompleteLength: 1, maximumLength: 8192) { data, _, complete, error in
        if error != nil { received.resolve(.failure(DeviceLinkTransportError.failed)) }
        else { received.resolve(.success((data ?? Data(), complete))) }
      }
      let (chunk, complete) = try received.wait(until: deadline)
      if !chunk.isEmpty {
        if let newline = chunk.firstIndex(of: 0x0A) {
          guard !newlineSeen, chunk.index(after: newline) == chunk.endIndex else {
            throw DeviceLinkTransportError.failed
          }
          let body = chunk[..<newline]
          guard output.count + body.count <= 4 * 1024 * 1024 else { throw DeviceLinkTransportError.failed }
          output.append(contentsOf: body)
          newlineSeen = true
        } else {
          guard !newlineSeen, output.count + chunk.count <= 4 * 1024 * 1024 else {
            throw DeviceLinkTransportError.failed
          }
          output.append(chunk)
        }
      }
      if complete {
        guard newlineSeen, !output.isEmpty else { throw DeviceLinkTransportError.failed }
        return output
      }
    }
  }

  private func validateResponse(_ response: [String: Any], nonce: String, operation: String,
                                credential: DeviceLinkCredential,
                                allowCredentialResponse: Bool) throws {
    let keys: Set<String> = ["request_id", "operation", "status", "evidence_source", "audit_event_id", "error", "health", "body", "shutdown_requested"]
    guard Set(response.keys) == keys,
          response["request_id"] as? String == nonce,
          response["operation"] as? String == operation,
          let status = response["status"] as? String,
          Set(["accepted", "rejected", "suspended"]).contains(status),
          let auditID = response["audit_event_id"] as? String,
          (1...256).contains(auditID.utf8.count),
          auditID.unicodeScalars.allSatisfy({ CharacterSet.alphanumerics.contains($0) || "_.:-".unicodeScalars.contains($0) }),
          let evidence = response["evidence_source"] as? String,
          Set(["LIVE_RUNTIME", "INTERNAL_STATE", "CONFIG", "EXTERNAL_EVIDENCE", "FIXTURE"]).contains(evidence),
          let shutdown = response["shutdown_requested"], DeviceLinkStrictJSON.isBoolean(shutdown),
          (shutdown as? NSNumber)?.boolValue == false,
          response["health"] is NSNull else { throw DeviceLinkTransportError.failed }
    let body = response["body"]
    let error = response["error"]
    if status == "accepted" {
      guard body is [String: Any], error is NSNull else { throw DeviceLinkTransportError.failed }
    } else {
      guard error is [String: Any] else { throw DeviceLinkTransportError.failed }
    }
    if let errorObject = error as? [String: Any] {
      let expected: Set<String> = ["code", "message", "recoverable", "audit_event_required", "fail_closed"]
      guard Set(errorObject.keys) == expected,
            let code = errorObject["code"] as? String,
            (1...96).contains(code.utf8.count),
            code.utf8.allSatisfy({ (65...90).contains($0) || (97...122).contains($0) || (48...57).contains($0) || [45, 46, 95].contains($0) }) else {
        throw DeviceLinkTransportError.failed
      }
    }
    if !allowCredentialResponse {
      guard let body else { throw DeviceLinkTransportError.failed }
      try DeviceLinkPayloadPolicy.validateResponseTree(body, secrets: [credential.secret])
    }
    if operation == "端末確認", status == "accepted" {
      guard NSDictionary(dictionary: body as? [String: Any] ?? [:]).isEqual(to: ["状態": "接続中"]) else {
        throw DeviceLinkTransportError.failed
      }
    }
    if operation == "端末離脱", status == "accepted" {
      guard NSDictionary(dictionary: body as? [String: Any] ?? [:]).isEqual(to: ["状態": "失効"]) else {
        throw DeviceLinkTransportError.failed
      }
    }
  }

  private func sanitize(_ response: [String: Any]) throws -> [String: Any] {
    guard let operation = response["operation"] as? String,
          let status = response["status"] as? String,
          let auditID = response["audit_event_id"] as? String,
          let evidence = response["evidence_source"] as? String,
          let body = response["body"] else { throw DeviceLinkTransportError.failed }
    let safeError: Any
    if let error = response["error"] as? [String: Any], let code = error["code"] as? String {
      safeError = ["code": code]
    } else if response["error"] is NSNull {
      safeError = NSNull()
    } else {
      throw DeviceLinkTransportError.failed
    }
    return [
      "operation": operation,
      "status": status,
      "audit_event_id": auditID,
      "evidence_source": evidence,
      "body": body,
      "error": safeError,
    ]
  }

  private static func installPin(_ options: sec_protocol_options_t, expectedHash: String) {
    let verifyQueue = DispatchQueue(label: "org.gatchimuchio.gui-shell.device-link.pin")
    sec_protocol_options_set_verify_block(options, { metadata, trust, complete in
      var leaf: SecCertificate?
      let available = sec_protocol_metadata_access_peer_certificate_chain(metadata) { certificate in
        if leaf == nil { leaf = sec_certificate_copy_ref(certificate).takeRetainedValue() }
      }
      guard available, let leaf,
            SHA256.hash(data: SecCertificateCopyData(leaf) as Data)
              .map({ String(format: "%02x", $0) }).joined() == expectedHash else {
        complete(false)
        return
      }
      let secTrust = sec_trust_copy_ref(trust).takeRetainedValue()
      guard SecTrustSetPolicies(secTrust, SecPolicyCreateBasicX509()) == errSecSuccess,
            SecTrustSetAnchorCertificates(secTrust, [leaf] as CFArray) == errSecSuccess,
            SecTrustSetAnchorCertificatesOnly(secTrust, true) == errSecSuccess,
            SecTrustSetNetworkFetchAllowed(secTrust, false) == errSecSuccess else {
        complete(false)
        return
      }
      var trustError: CFError?
      complete(SecTrustEvaluateWithError(secTrust, &trustError))
    }, verifyQueue)
  }

  private static func randomID() throws -> String {
    var bytes = [UInt8](repeating: 0, count: 16)
    guard SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) == errSecSuccess else {
      throw DeviceLinkTransportError.failed
    }
    return bytes.map { String(format: "%02x", $0) }.joined()
  }
}
