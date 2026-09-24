import Flutter
import UIKit
import XCTest
import Security
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
    let deleted = try store.deleteCredential()
    XCTAssertNil(deleted.credential)
    XCTAssertNil(try store.loadOrCreate().credential)
  }

  private func randomHex(byteCount: Int) -> String? {
    var bytes = [UInt8](repeating: 0, count: byteCount)
    guard SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) == errSecSuccess else { return nil }
    return bytes.map { String(format: "%02x", $0) }.joined()
  }
}
