import Flutter
import UIKit
import XCTest
import Security
import Network
@testable import Runner

class RunnerTests: XCTestCase {
  func testApplicationDelegateUsesFlutter() {
    // native起動経路の接続だけを検査し、端末連携の証拠とはしない。
    XCTAssertNotNil(UIApplication.shared.delegate as? AppDelegate)
  }

  func testStrictJSONRejectsDuplicateAndEscapedDuplicateKeys() {
    XCTAssertThrowsError(try DeviceLinkStrictJSON.parseObject("{\"key\":1,\"key\":2}", maximumBytes: 128))
    XCTAssertThrowsError(try DeviceLinkStrictJSON.parseObject("{\"key\":1,\"\\u006bey\":2}", maximumBytes: 128))
    XCTAssertNoThrow(try DeviceLinkStrictJSON.parseObject("{\"key\":1,\"other\":[true,null]}", maximumBytes: 128))
  }

  func testErrorCodeAcceptsJapaneseBrokerIdentifierAndRejectsInvalidText() {
    XCTAssertTrue(DeviceLinkTLSClient.isValidErrorCode("端末要求拒否"))
    XCTAssertTrue(DeviceLinkTLSClient.isValidErrorCode("broker_audit_append_failed"))
    for invalid in ["", "端末 要求拒否", "端末要求拒否\n", "<code>", String(repeating: "拒", count: 33)] {
      XCTAssertFalse(DeviceLinkTLSClient.isValidErrorCode(invalid))
    }
  }

  func testInvitationParsingBindsHostDeviceAndExpiry() throws {
    let deviceID = try XCTUnwrap(randomHex(byteCount: 16))
    let invitation: [String: Any] = [
      "版": 1,
      "HostID": String(repeating: "1", count: 32),
      "接続先Host": "192.168.1.20",
      "port": 44321,
      "証明書hash": String(repeating: "a", count: 64),
      "端末ID": deviceID,
      "有効期限": 1_300,
      "招待ID": String(repeating: "2", count: 32),
      "招待秘密": try XCTUnwrap(randomHex(byteCount: 32)),
    ]
    let data = try JSONSerialization.data(withJSONObject: invitation, options: [.sortedKeys])
    let parsed = try DeviceLinkCredential.invitation(String(decoding: data, as: UTF8.self),
                                                      expectedDeviceID: deviceID, now: 1_000)
    XCTAssertEqual(parsed.host, "192.168.1.20")
    XCTAssertTrue(DeviceLinkCredential.isPrivateIPv4("127.0.0.1"))
    XCTAssertFalse(DeviceLinkCredential.isPrivateIPv4("8.8.8.8"))
    XCTAssertFalse(DeviceLinkCredential.isPrivateIPv4("192.168.001.20"))
    XCTAssertThrowsError(try DeviceLinkCredential.invitation(String(decoding: data, as: UTF8.self),
                                                             expectedDeviceID: try XCTUnwrap(randomHex(byteCount: 16)),
                                                             now: 1_000))
  }

  func testHistoryGrantIsLimitedToBoundedHistoryQuery() {
    let history: [String: Any] = [
      "approval_id": String(repeating: "a", count: 32),
      "query": ["after": 0, "limit": 50],
    ]
    XCTAssertNoThrow(try DeviceLinkPayloadPolicy.validate("対話履歴閲覧", payload: history))
    XCTAssertThrowsError(try DeviceLinkPayloadPolicy.validate("対話履歴閲覧状態", payload: history))
    XCTAssertThrowsError(try DeviceLinkPayloadPolicy.validate("対話開始", payload: ["token": "x"]))
  }

  func testWorkspaceSelectionIsAnEmptyReadOnlyRequest() {
    XCTAssertNoThrow(try DeviceLinkPayloadPolicy.validate("作業領域一覧", payload: [:]))
    XCTAssertThrowsError(try DeviceLinkPayloadPolicy.validate("作業領域一覧", payload: ["実行系ID": "runtime-a"]))
    XCTAssertNoThrow(try DeviceLinkPayloadPolicy.validateWorkspaceSelectionResponse(
      ["作業領域": [["作業領域ID": "workspace-a", "実行系ID": "runtime-a"]]]))
    XCTAssertThrowsError(try DeviceLinkPayloadPolicy.validateWorkspaceSelectionResponse(
      ["作業領域": [["作業領域ID": "workspace-a", "approval_id": "grant"]]]))
  }

  func testLocalRecoveryAuditIsClosedSecretFreeAndBounded() throws {
    let first = DeviceLinkLocalRecoveryAudit.localCredentialDeleted(
      eventID: String(repeating: "a", count: 32), timestamp: "2026-09-25T12:00:00Z")
    XCTAssertTrue(DeviceLinkLocalRecoveryAudit.isValid(first))
    let milliseconds = DeviceLinkLocalRecoveryAudit.localCredentialDeleted(
      eventID: String(repeating: "b", count: 32), timestamp: "2026-09-25T12:00:00.123Z")
    XCTAssertTrue(DeviceLinkLocalRecoveryAudit.isValid(milliseconds))
    let metadata = try XCTUnwrap(first["metadata"] as? [String: Any])
    XCTAssertEqual(metadata["evidence_source"] as? String, "INTERNAL_STATE")
    XCTAssertEqual(metadata["desktop_revocation"] as? String, "unconfirmed")
    XCTAssertEqual(first["payload_hash"] as? String,
                   "sha256:2f0b366fb62efa9e741f0127c676e648d81b263a471c35d0dd00814a174e5a92")
    XCTAssertFalse(DeviceLinkLocalRecoveryAudit.isValid(first.merging(["credential_secret": "secret"]) { _, new in new }))
    for timestamp in ["2026-02-30T12:00:00Z", "2026-09-25T12:00:00Z\n",
                      "2026-09-25T12:00:00+00:00", "2026-09-25T12:00:00.1Z",
                      "2026-09-25T12:00:00.1234Z"] {
      XCTAssertFalse(DeviceLinkLocalRecoveryAudit.isValid(first.merging(["timestamp": timestamp]) { _, new in new }))
    }

    let history = (0...DeviceLinkLocalRecoveryAudit.maximumEvents).map { index in
      DeviceLinkLocalRecoveryAudit.localCredentialDeleted(
        eventID: String(format: "%032x", index), timestamp: "2026-09-25T12:00:00Z")
    }
    var retained = [[String: Any]]()
    for item in history.dropLast() {
      retained = DeviceLinkLocalRecoveryAudit.appendBounded(retained, event: item)
    }
    XCTAssertEqual(retained.count, DeviceLinkLocalRecoveryAudit.maximumEvents)
    retained = DeviceLinkLocalRecoveryAudit.appendBounded(retained, event: try XCTUnwrap(history.last))
    XCTAssertEqual(retained.count, DeviceLinkLocalRecoveryAudit.maximumEvents)
    let retainedFirst = try XCTUnwrap(retained.first)
    let retainedLast = try XCTUnwrap(retained.last)
    let expectedLast = try XCTUnwrap(history.last)
    XCTAssertTrue(NSDictionary(dictionary: retainedFirst).isEqual(to: history[1]))
    XCTAssertTrue(NSDictionary(dictionary: retainedLast).isEqual(to: expectedLast))
  }

  func testThisDeviceOnlyKeychainReadWriteDeleteAndReadback() throws {
    let service = "org.gatchimuchio.gui-shell.device-link.test.\(UUID().uuidString)"
    let store = DeviceLinkNativeStore(service: service)
    defer { try? store.removeForTest() }
    let initial = try store.loadOrCreate()
    let credential = DeviceLinkCredential(
      hostID: String(repeating: "1", count: 32),
      host: "10.0.0.2",
      port: 44321,
      certificateHash: String(repeating: "b", count: 64),
      deviceID: initial.deviceID,
      expiresAt: Int64(Date().timeIntervalSince1970) + 3_600,
      credentialID: String(repeating: "2", count: 32),
      secret: try XCTUnwrap(randomHex(byteCount: 32)))
    let saved = try store.storeCredential(credential)
    XCTAssertTrue(saved.credential != nil)
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: service,
      kSecAttrAccount as String: "native-state",
      kSecAttrSynchronizable as String: kCFBooleanFalse as Any,
      kSecReturnAttributes as String: true,
      kSecMatchLimit as String: kSecMatchLimitOne,
    ]
    var attributes: CFTypeRef?
    XCTAssertEqual(SecItemCopyMatching(query as CFDictionary, &attributes), errSecSuccess)
    let storedAttributes = attributes as? [String: Any]
    XCTAssertEqual(storedAttributes?[kSecAttrAccessible as String] as? String,
                   kSecAttrAccessibleWhenUnlockedThisDeviceOnly as String)
    if let synchronizable = storedAttributes?[kSecAttrSynchronizable as String] {
      XCTAssertEqual((synchronizable as? NSNumber)?.boolValue, false)
    }
    let deleted = try store.deleteCredentialWithLocalRecoveryAudit()
    XCTAssertNil(deleted.credential)
    XCTAssertEqual(deleted.localRecoveryAudit.count, 1)
    XCTAssertTrue(DeviceLinkLocalRecoveryAudit.isValid(try XCTUnwrap(deleted.localRecoveryAudit.last)))
    XCTAssertEqual(try store.readLocalRecoveryAudit().count, 1)
    let pairedAgain = try store.storeCredential(credential)
    XCTAssertEqual(pairedAgain.localRecoveryAudit.count, 1)
    let disconnected = try store.deleteCredential()
    XCTAssertNil(disconnected.credential)
    XCTAssertEqual(disconnected.localRecoveryAudit.count, 1)
  }

  func testVersionOneKeychainStateMigratesOnNextWrite() throws {
    let service = "org.gatchimuchio.gui-shell.device-link.migration.\(UUID().uuidString)"
    let store = DeviceLinkNativeStore(service: service)
    defer { try? store.removeForTest() }
    let deviceID = String(repeating: "c", count: 32)
    let legacy = try JSONSerialization.data(withJSONObject: [
      "version": 1,
      "device_id": deviceID,
      "credential": NSNull(),
    ], options: [.sortedKeys])
    let identity: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: service,
      kSecAttrAccount as String: "native-state",
      kSecAttrSynchronizable as String: kCFBooleanFalse as Any,
      kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
      kSecValueData as String: legacy,
    ]
    XCTAssertEqual(SecItemAdd(identity as CFDictionary, nil), errSecSuccess)
    let restored = try store.loadOrCreate()
    XCTAssertEqual(restored.deviceID, deviceID)
    XCTAssertTrue(restored.localRecoveryAudit.isEmpty)

    let credential = DeviceLinkCredential(
      hostID: String(repeating: "1", count: 32),
      host: "10.0.0.2",
      port: 44321,
      certificateHash: String(repeating: "b", count: 64),
      deviceID: deviceID,
      expiresAt: Int64(Date().timeIntervalSince1970) + 3_600,
      credentialID: String(repeating: "2", count: 32),
      secret: try XCTUnwrap(randomHex(byteCount: 32)))
    _ = try store.storeCredential(credential)
    var query = identity
    query.removeValue(forKey: kSecAttrAccessible as String)
    query.removeValue(forKey: kSecValueData as String)
    query[kSecReturnData as String] = true
    query[kSecMatchLimit as String] = kSecMatchLimitOne
    var stored: CFTypeRef?
    XCTAssertEqual(SecItemCopyMatching(query as CFDictionary, &stored), errSecSuccess)
    let upgraded = try DeviceLinkStrictJSON.parseObject(try XCTUnwrap(stored as? Data), maximumBytes: 16 * 1024)
    XCTAssertEqual(DeviceLinkStrictJSON.integer(upgraded["version"]), 2)
    XCTAssertEqual((upgraded["local_recovery_audit"] as? [[String: Any]])?.count, 0)
  }

  private func randomHex(byteCount: Int) -> String? {
    var bytes = [UInt8](repeating: 0, count: byteCount)
    guard SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) == errSecSuccess else { return nil }
    return bytes.map { String(format: "%02x", $0) }.joined()
  }
}

// 実Brokerとの接続だけを検証する。Flutter UI／native確認dialog／OS lifecycleの証拠ではない。
class NativeDeviceLinkLiveTests: XCTestCase {
  func testNativeTLSAndKeychainUseRealBroker() throws {
#if targetEnvironment(simulator)
    guard let rawPort = ProcessInfo.processInfo.environment["D4_IOS_NATIVE_BRIDGE_PORT"],
          let port = UInt16(rawPort), port > 0 else {
      throw XCTSkip("専用native接続harnessからだけ実行する")
    }
    let store = DeviceLinkNativeStore(service: "org.gatchimuchio.gui-shell.native-live.\(UUID().uuidString)")
    let transport = DeviceLinkTLSClient()
    defer {
      transport.cancelAll()
      do { try store.removeForTest() }
      catch { XCTFail("D4_IOS_NATIVE_LIVE_FAIL keychain_cleanup") }
    }
    var stage = "keychain_initial"
    do {
      let initial = try store.loadOrCreate()
      try require(initial.credential == nil)
      stage = "invitation_bridge"
      let raw = try receiveInvitation(port: port, deviceID: initial.deviceID)
      let invitation = try DeviceLinkCredential.invitation(raw, expectedDeviceID: initial.deviceID)
      stage = "wrong_pin"
      let wrongHash = String(repeating: "0", count: 64)
      try require(invitation.certificateHash != wrongHash)
      let wrongPin = DeviceLinkCredential(
        hostID: invitation.hostID, host: invitation.host, port: invitation.port,
        certificateHash: wrongHash, deviceID: invitation.deviceID,
        expiresAt: invitation.expiresAt, credentialID: invitation.credentialID,
        secret: invitation.secret)
      var rejected = false
      do {
        _ = try transport.exchange(wrongPin, operation: "端末結合", payload: [:],
                                   canSend: { true }, allowCredentialResponse: true)
      } catch { rejected = true }
      try require(rejected)
      stage = "pair"
      let response = try transport.exchange(invitation, operation: "端末結合", payload: [:],
                                            canSend: { true }, allowCredentialResponse: true)
      try require(response["status"] as? String == "accepted")
      guard let body = response["body"] as? [String: Any] else { throw DeviceLinkTransportError.failed }
      let paired = try DeviceLinkCredential.paired(body, invitation: invitation)
      stage = "keychain_readback"
      _ = try store.storeCredential(paired)
      guard let stored = try store.loadOrCreate().credential else { throw DeviceLinkTransportError.failed }
      let reloaded = try DeviceLinkCredential.stored(stored, expectedDeviceID: initial.deviceID)
      try require(reloaded == paired)
      stage = "confirm"
      let confirmed = try transport.exchange(reloaded, operation: "端末確認", payload: [:], canSend: { true })
      try require(confirmed["status"] as? String == "accepted")
      stage = "runtime_list"
      try DeviceLinkPayloadPolicy.validate("実行系列挙", payload: [:])
      let runtime = try transport.exchange(reloaded, operation: "実行系列挙", payload: [:], canSend: { true })
      try require(runtime["status"] as? String == "accepted")
      let runtimes = (runtime["body"] as? [String: Any])?["実行系"] as? [String]
      try require(runtimes == ["left", "right"])
      stage = "disconnect"
      let disconnected = try transport.exchange(reloaded, operation: "端末離脱", payload: [:], canSend: { true })
      try require(disconnected["status"] as? String == "accepted")
      _ = try store.deleteCredential()
      try require(try store.loadOrCreate().credential == nil)
      stage = "revoked_credential"
      let revoked = try transport.exchange(reloaded, operation: "端末確認", payload: [:], canSend: { true })
      try require(revoked["status"] as? String == "rejected")
      print("D4_IOS_NATIVE_LIVE_PASS")
    } catch {
      // assertionのactual/expectedや例外本文に資格値を出さない。
      XCTFail("D4_IOS_NATIVE_LIVE_FAIL \(stage)")
    }
#else
    throw XCTSkip("物理端末をこのharnessの対象にしない")
#endif
  }

  private func require(_ condition: Bool) throws {
    if !condition { throw DeviceLinkTransportError.failed }
  }

  private func receiveInvitation(port: UInt16, deviceID: String) throws -> String {
    guard let endpointPort = NWEndpoint.Port(rawValue: port) else { throw DeviceLinkTransportError.failed }
    let connection = NWConnection(host: "127.0.0.1", port: endpointPort, using: .tcp)
    let result = NativeInvitationResult()
    let queue = DispatchQueue(label: "org.gatchimuchio.gui-shell.test.invitation")
    var buffer = Data()
    func receive() {
      connection.receive(minimumIncompleteLength: 1, maximumLength: 8192) { data, _, complete, error in
        guard error == nil else { result.finish(nil); return }
        if let data { buffer.append(data) }
        guard buffer.count <= 8193 else { result.finish(nil); return }
        if let newline = buffer.firstIndex(of: 0x0A) {
          guard buffer.index(after: newline) == buffer.endIndex else { result.finish(nil); return }
          result.finish(String(data: buffer[..<newline], encoding: .utf8))
        } else if complete { result.finish(nil) }
        else { receive() }
      }
    }
    connection.stateUpdateHandler = { state in
      switch state {
      case .ready:
        connection.send(content: Data((deviceID + "\n").utf8), completion: .contentProcessed { error in
          if error == nil { receive() } else { result.finish(nil) }
        })
      case .failed, .cancelled: result.finish(nil)
      default: break
      }
    }
    connection.start(queue: queue)
    defer { connection.forceCancel() }
    return try result.wait()
  }
}


private final class NativeInvitationResult {
  private let lock = NSLock()
  private let semaphore = DispatchSemaphore(value: 0)
  private var finished = false
  private var value: String?

  func finish(_ value: String?) {
    lock.lock()
    guard !finished else { lock.unlock(); return }
    finished = true
    self.value = value
    lock.unlock()
    semaphore.signal()
  }

  func wait() throws -> String {
    guard semaphore.wait(timeout: .now() + .seconds(20)) == .success else { throw DeviceLinkTransportError.failed }
    lock.lock()
    let result = value
    lock.unlock()
    guard let result else { throw DeviceLinkTransportError.failed }
    return result
  }
}
