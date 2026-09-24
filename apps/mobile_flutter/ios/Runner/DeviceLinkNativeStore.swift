import Foundation
import Security

final class DeviceLinkNativeStore {
  private let service: String
  private let account: String

  init(service: String = "org.gatchimuchio.gui-shell.device-link.v1",
       account: String = "native-state") {
    self.service = service
    self.account = account
  }

  func loadOrCreate() throws -> DeviceLinkNativeState {
    if let data = try readData() { return try decode(data) }
    let initial = DeviceLinkNativeState(deviceID: try Self.randomID(), credential: nil)
    try write(initial)
    guard let verifiedData = try readData() else { throw DeviceLinkJSONError.invalid }
    let verified = try decode(verifiedData)
    guard verified == initial else { throw DeviceLinkJSONError.invalid }
    return verified
  }

  func storeCredential(_ credential: DeviceLinkCredential) throws -> DeviceLinkNativeState {
    let before = try loadOrCreate()
    guard before.deviceID == credential.deviceID else { throw DeviceLinkJSONError.invalid }
    let next = DeviceLinkNativeState(deviceID: before.deviceID, credential: credential.json)
    try write(next)
    guard let data = try readData() else { throw DeviceLinkJSONError.invalid }
    let verified = try decode(data)
    guard verified == next else { throw DeviceLinkJSONError.invalid }
    return verified
  }

  func deleteCredential() throws -> DeviceLinkNativeState {
    let before = try loadOrCreate()
    let next = DeviceLinkNativeState(deviceID: before.deviceID, credential: nil)
    try write(next)
    guard let data = try readData() else { throw DeviceLinkJSONError.invalid }
    let verified = try decode(data)
    guard verified == next else { throw DeviceLinkJSONError.invalid }
    return verified
  }

  func removeForTest() throws {
    let status = SecItemDelete(identityQuery as CFDictionary)
    guard status == errSecSuccess || status == errSecItemNotFound else { throw DeviceLinkJSONError.invalid }
  }

  private var identityQuery: [String: Any] {
    [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: service,
      kSecAttrAccount as String: account,
      kSecAttrSynchronizable as String: kCFBooleanFalse as Any,
    ]
  }

  private func readData() throws -> Data? {
    var query = identityQuery
    query[kSecReturnData as String] = true
    query[kSecMatchLimit as String] = kSecMatchLimitOne
    var item: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &item)
    if status == errSecItemNotFound { return nil }
    guard status == errSecSuccess, let data = item as? Data else { throw DeviceLinkJSONError.invalid }
    return data
  }

  private func write(_ state: DeviceLinkNativeState) throws {
    let object: [String: Any] = [
      "version": 1,
      "device_id": state.deviceID,
      "credential": state.credential.map { $0 as Any } ?? NSNull(),
    ]
    let data = try DeviceLinkStrictJSON.encodeObject(object, maximumBytes: 16 * 1024)
    let attributes: [String: Any] = [
      kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
      kSecAttrSynchronizable as String: kCFBooleanFalse as Any,
    ]
    let updateStatus = SecItemUpdate(identityQuery as CFDictionary,
                                     [kSecValueData as String: data] as CFDictionary)
    if updateStatus == errSecItemNotFound {
      var add = identityQuery
      for (key, value) in attributes { add[key] = value }
      add[kSecValueData as String] = data
      let status = SecItemAdd(add as CFDictionary, nil)
      guard status == errSecSuccess else { throw DeviceLinkJSONError.invalid }
    } else if updateStatus != errSecSuccess {
      throw DeviceLinkJSONError.invalid
    }
  }

  private func decode(_ data: Data) throws -> DeviceLinkNativeState {
    let object = try DeviceLinkStrictJSON.parseObject(data, maximumBytes: 16 * 1024)
    guard Set(object.keys) == Set(["version", "device_id", "credential"]),
          DeviceLinkStrictJSON.integer(object["version"]) == 1,
          let deviceID = object["device_id"] as? String,
          deviceID.utf8.count == 32,
          deviceID.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) }) else {
      throw DeviceLinkJSONError.invalid
    }
    let credential: [String: Any]?
    if object["credential"] is NSNull { credential = nil }
    else if let value = object["credential"] as? [String: Any] { credential = value }
    else { throw DeviceLinkJSONError.invalid }
    return DeviceLinkNativeState(deviceID: deviceID, credential: credential)
  }

  private static func randomID() throws -> String {
    var bytes = [UInt8](repeating: 0, count: 16)
    let status = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
    guard status == errSecSuccess else { throw DeviceLinkJSONError.invalid }
    return bytes.map { String(format: "%02x", $0) }.joined()
  }
}
