import Foundation
import CryptoKit

struct DeviceLinkCredential: Equatable {
  let hostID: String
  let host: String
  let port: Int
  let certificateHash: String
  let deviceID: String
  let expiresAt: Int64
  let credentialID: String
  let secret: String

  func isExpired(now: Int64 = Int64(Date().timeIntervalSince1970)) -> Bool {
    expiresAt <= now
  }

  var json: [String: Any] {
    [
      "版": 1,
      "HostID": hostID,
      "接続先Host": host,
      "port": port,
      "証明書hash": certificateHash,
      "端末ID": deviceID,
      "有効期限": expiresAt,
      "結合ID": credentialID,
      "端末秘密": secret,
    ]
  }

  static func invitation(_ raw: String, expectedDeviceID: String,
                         now: Int64 = Int64(Date().timeIntervalSince1970)) throws -> DeviceLinkCredential {
    let data = try DeviceLinkStrictJSON.parseObject(raw, maximumBytes: 8192)
    let expected = Set(["版", "HostID", "接続先Host", "port", "証明書hash", "端末ID", "有効期限", "招待ID", "招待秘密"])
    guard Set(data.keys) == expected else { throw DeviceLinkJSONError.invalid }
    return try parse(data, expectedDeviceID: expectedDeviceID, invitation: true, now: now)
  }

  static func paired(_ data: [String: Any], invitation: DeviceLinkCredential,
                     now: Int64 = Int64(Date().timeIntervalSince1970)) throws -> DeviceLinkCredential {
    let expected = Set(["版", "HostID", "接続先Host", "port", "証明書hash", "端末ID", "有効期限", "結合ID", "端末秘密"])
    guard Set(data.keys) == expected else { throw DeviceLinkJSONError.invalid }
    let credential = try parse(data, expectedDeviceID: invitation.deviceID, invitation: false, now: now)
    guard credential.hostID == invitation.hostID,
          credential.host == invitation.host,
          credential.port == invitation.port,
          credential.certificateHash == invitation.certificateHash,
          credential.secret != invitation.secret else { throw DeviceLinkJSONError.invalid }
    return credential
  }

  static func stored(_ data: [String: Any], expectedDeviceID: String,
                     now: Int64 = Int64(Date().timeIntervalSince1970)) throws -> DeviceLinkCredential {
    let expected = Set(["版", "HostID", "接続先Host", "port", "証明書hash", "端末ID", "有効期限", "結合ID", "端末秘密"])
    guard Set(data.keys) == expected else { throw DeviceLinkJSONError.invalid }
    return try parse(data, expectedDeviceID: expectedDeviceID, invitation: false, now: now)
  }

  private static func parse(_ data: [String: Any], expectedDeviceID: String,
                            invitation: Bool, now: Int64) throws -> DeviceLinkCredential {
    let version = DeviceLinkStrictJSON.integer(data["版"])
    let hostID = data["HostID"] as? String
    let host = data["接続先Host"] as? String
    let portValue = DeviceLinkStrictJSON.integer(data["port"])
    let certificateHash = data["証明書hash"] as? String
    let deviceID = data["端末ID"] as? String
    let expiresAt = DeviceLinkStrictJSON.integer(data["有効期限"])
    let idKey = invitation ? "招待ID" : "結合ID"
    let secretKey = invitation ? "招待秘密" : "端末秘密"
    let credentialID = data[idKey] as? String
    let secret = data[secretKey] as? String
    guard version == 1,
          let hostID, isHex(hostID, length: 32),
          let host, isPrivateIPv4(host),
          let portValue, (1...65535).contains(portValue),
          let certificateHash, isHex(certificateHash, length: 64),
          let deviceID, isHex(deviceID, length: 32), deviceID == expectedDeviceID,
          let expiresAt,
          let credentialID, isHex(credentialID, length: 32),
          let secret, isHex(secret, length: 64) else { throw DeviceLinkJSONError.invalid }
    let duration: Int64 = invitation ? 300 : 8 * 60 * 60
    guard expiresAt > now, expiresAt <= now + duration + 60 else { throw DeviceLinkJSONError.invalid }
    return DeviceLinkCredential(hostID: hostID, host: host, port: Int(portValue),
                                certificateHash: certificateHash, deviceID: deviceID,
                                expiresAt: expiresAt, credentialID: credentialID, secret: secret)
  }

  static func isPrivateIPv4(_ host: String) -> Bool {
    let parts = host.split(separator: ".", omittingEmptySubsequences: false)
    guard parts.count == 4 else { return false }
    var octets = [Int]()
    for part in parts {
      guard !part.isEmpty, !(part.count > 1 && part.first == "0"),
            let value = Int(part), (0...255).contains(value), String(value) == part else { return false }
      octets.append(value)
    }
    return octets[0] == 127 || octets[0] == 10 ||
      (octets[0] == 172 && (16...31).contains(octets[1])) ||
      (octets[0] == 192 && octets[1] == 168)
  }

  private static func isHex(_ value: String, length: Int) -> Bool {
    value.utf8.count == length && value.utf8.allSatisfy { (48...57).contains($0) || (97...102).contains($0) }
  }
}

struct DeviceLinkNativeState: Equatable {
  let deviceID: String
  let credential: [String: Any]?
  let localRecoveryAudit: [[String: Any]]

  init(deviceID: String, credential: [String: Any]?, localRecoveryAudit: [[String: Any]] = []) {
    self.deviceID = deviceID
    self.credential = credential
    self.localRecoveryAudit = localRecoveryAudit
  }

  static func == (lhs: DeviceLinkNativeState, rhs: DeviceLinkNativeState) -> Bool {
    lhs.deviceID == rhs.deviceID &&
      NSDictionary(dictionary: ["credential": lhs.credential ?? [:], "audit": lhs.localRecoveryAudit])
        .isEqual(to: ["credential": rhs.credential ?? [:], "audit": rhs.localRecoveryAudit])
  }
}

enum DeviceLinkLocalRecoveryAudit {
  static let maximumEvents = 32
  private static let payloadMarker = "gui-shell/mobile/local-credential-delete/v1"

  static var payloadHash: String {
    "sha256:" + SHA256.hash(data: Data(payloadMarker.utf8)).map { String(format: "%02x", $0) }.joined()
  }

  static func localCredentialDeleted(eventID: String, timestamp: String) -> [String: Any] {
    precondition(isHex(eventID, length: 32))
    precondition(isCanonicalTimestamp(timestamp))
    return [
      "event_id": eventID,
      "timestamp": timestamp,
      "actor": "shell",
      "action": "mobile_local_credential_delete",
      "target": "mobile_device_link_credential",
      "result": "success",
      "payload_hash": payloadHash,
      "metadata": [
        "audit_scope": "mobile_local_recovery",
        "evidence_source": "INTERNAL_STATE",
        "desktop_revocation": "unconfirmed",
        "authority_effect": "none",
        "operator_identity": "unverified",
      ],
    ]
  }

  static func appendBounded(_ existing: [[String: Any]], event: [String: Any]) -> [[String: Any]] {
    precondition(existing.count <= maximumEvents && existing.allSatisfy { isValid($0) } && isValid(event))
    let newEventID = event["event_id"] as? String
    precondition(newEventID != nil && !existing.contains { ($0["event_id"] as? String) == newEventID })
    return Array((existing + [event]).suffix(maximumEvents))
  }

  static func isValid(_ event: [String: Any]) -> Bool {
    guard Set(event.keys) == Set(["event_id", "timestamp", "actor", "action", "target", "result", "payload_hash", "metadata"]),
          let eventID = event["event_id"] as? String, isHex(eventID, length: 32),
          let timestamp = event["timestamp"] as? String, isCanonicalTimestamp(timestamp),
          let metadata = event["metadata"] as? [String: Any],
          Set(metadata.keys) == Set(["audit_scope", "evidence_source", "desktop_revocation", "authority_effect", "operator_identity"]) else {
      return false
    }
    return event["actor"] as? String == "shell" &&
      event["action"] as? String == "mobile_local_credential_delete" &&
      event["target"] as? String == "mobile_device_link_credential" &&
      event["result"] as? String == "success" &&
      event["payload_hash"] as? String == payloadHash &&
      metadata["audit_scope"] as? String == "mobile_local_recovery" &&
      metadata["evidence_source"] as? String == "INTERNAL_STATE" &&
      metadata["desktop_revocation"] as? String == "unconfirmed" &&
      metadata["authority_effect"] as? String == "none" &&
      metadata["operator_identity"] as? String == "unverified"
  }

  private static func isHex(_ value: String, length: Int) -> Bool {
    value.utf8.count == length && value.utf8.allSatisfy { (48...57).contains($0) || (97...102).contains($0) }
  }

  private static func isCanonicalTimestamp(_ value: String) -> Bool {
    guard let utc = TimeZone(secondsFromGMT: 0) else { return false }
    let seconds = ISO8601DateFormatter()
    seconds.formatOptions = [.withInternetDateTime]
    seconds.timeZone = utc
    if let date = seconds.date(from: value), seconds.string(from: date) == value { return true }

    let milliseconds = ISO8601DateFormatter()
    milliseconds.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    milliseconds.timeZone = utc
    return milliseconds.date(from: value).map { milliseconds.string(from: $0) == value } ?? false
  }
}

struct DeviceLinkSnapshot {
  let storageReady: Bool
  let paired: Bool
  let connected: Bool
  let foreground: Bool
  let status: String

  var channelValue: [String: Any] {
    [
      "storage_ready": storageReady,
      "paired": paired,
      "connected": connected,
      "foreground": foreground,
      "status": String(status.prefix(160)),
    ]
  }
}

enum DeviceLinkPayloadPolicy {
  private static let operations: Set<String> = [
    "実行系列挙", "Agent一覧", "対話開始", "対話送信", "対話取得", "対話中止", "対話終了",
    "実行系ライフサイクル状態", "実行系資源観測", "通知一覧", "全Runtime停止要求",
    "対話履歴閲覧状態", "対話履歴閲覧",
  ]
  private static let forbidden = ["owner", "authority", "permission", "approval", "audit", "credential", "secret", "token", "private", "password"]

  static func validate(_ operation: String, payload: [String: Any]) throws {
    guard operations.contains(operation) else { throw DeviceLinkJSONError.invalid }
    if operation == "対話履歴閲覧" { try validateHistory(payload) }
    else { try validateTree(payload, depth: 0) }
    _ = try DeviceLinkStrictJSON.encodeObject(payload, maximumBytes: 64 * 1024)
  }

  static func validateResponseTree(_ value: Any, secrets: Set<String>, depth: Int = 0) throws {
    guard depth <= 32 else { throw DeviceLinkJSONError.invalid }
    if value is NSNull || DeviceLinkStrictJSON.isBoolean(value) { return }
    if let text = value as? String {
      guard secrets.allSatisfy({ $0.isEmpty || !text.contains($0) }) else { throw DeviceLinkJSONError.invalid }
      return
    }
    if let number = value as? NSNumber {
      guard number.doubleValue.isFinite else { throw DeviceLinkJSONError.invalid }
      return
    }
    if let list = value as? [Any] {
      guard list.count <= 4096 else { throw DeviceLinkJSONError.invalid }
      for item in list { try validateResponseTree(item, secrets: secrets, depth: depth + 1) }
      return
    }
    if let map = value as? [String: Any] {
      guard map.count <= 4096 else { throw DeviceLinkJSONError.invalid }
      for (key, item) in map {
        let normalized = normalize(key)
        let safeMarker = normalized == "secretpaths" || normalized == "secretvaluepresent"
        guard safeMarker || !["credential", "token", "password", "privatekey"].contains(where: normalized.contains),
              ["招待秘密", "端末秘密", "資格秘密", "端末資格"].contains(key) == false,
              safeMarker || !normalized.contains("secret") else { throw DeviceLinkJSONError.invalid }
        try validateResponseTree(item, secrets: secrets, depth: depth + 1)
      }
      return
    }
    throw DeviceLinkJSONError.invalid
  }

  private static func validateHistory(_ payload: [String: Any]) throws {
    guard Set(payload.keys) == Set(["approval_id", "query"]),
          let approvalID = payload["approval_id"] as? String, isHex(approvalID, length: 32),
          let query = payload["query"] as? [String: Any] else { throw DeviceLinkJSONError.invalid }
    let keys: Set<String> = ["after", "limit", "latest_per_request", "include_audit_context", "include_result_evidence", "include_content_receipt", "filter"]
    guard Set(query.keys).isSubset(of: keys), query["after"] != nil, query["limit"] != nil,
          let after = DeviceLinkStrictJSON.integer(query["after"]), after >= 0,
          let limit = DeviceLinkStrictJSON.integer(query["limit"]), (1...100).contains(limit) else {
      throw DeviceLinkJSONError.invalid
    }
    for key in ["latest_per_request", "include_audit_context", "include_result_evidence", "include_content_receipt"] {
      if let value = query[key], !DeviceLinkStrictJSON.isBoolean(value) { throw DeviceLinkJSONError.invalid }
    }
    guard let filterValue = query["filter"] else { return }
    guard let filter = filterValue as? [String: Any],
          Set(filter.keys).isSubset(of: Set(["要求ID", "対話セッションID", "実行系ID", "状態"])) else {
      throw DeviceLinkJSONError.invalid
    }
    for (key, value) in filter {
      switch key {
      case "要求ID", "対話セッションID":
        guard let text = value as? String, isHex(text, length: 32) else { throw DeviceLinkJSONError.invalid }
      case "実行系ID":
        guard let text = value as? String, isRuntimeID(text) else { throw DeviceLinkJSONError.invalid }
      case "状態":
        guard let text = value as? String,
              Set(["承認待ち", "実行中", "成功", "保留", "失敗", "中止"]).contains(text) else {
          throw DeviceLinkJSONError.invalid
        }
      default:
        throw DeviceLinkJSONError.invalid
      }
    }
  }

  private static func validateTree(_ value: Any, depth: Int) throws {
    guard depth <= 32 else { throw DeviceLinkJSONError.invalid }
    if value is NSNull || DeviceLinkStrictJSON.isBoolean(value) || value is String { return }
    if let number = value as? NSNumber {
      guard number.doubleValue.isFinite else { throw DeviceLinkJSONError.invalid }
      return
    }
    if let list = value as? [Any] {
      guard list.count <= 4096 else { throw DeviceLinkJSONError.invalid }
      for item in list { try validateTree(item, depth: depth + 1) }
      return
    }
    if let map = value as? [String: Any] {
      guard map.count <= 4096 else { throw DeviceLinkJSONError.invalid }
      for (key, item) in map {
        let normalized = normalize(key)
        guard !forbidden.contains(where: normalized.contains),
              ["招待秘密", "端末秘密", "資格秘密", "端末資格"].contains(key) == false else {
          throw DeviceLinkJSONError.invalid
        }
        try validateTree(item, depth: depth + 1)
      }
      return
    }
    throw DeviceLinkJSONError.invalid
  }

  private static func normalize(_ value: String) -> String {
    value.lowercased().filter { $0.isLetter || $0.isNumber }
  }

  private static func isHex(_ value: String, length: Int) -> Bool {
    value.utf8.count == length && value.utf8.allSatisfy { (48...57).contains($0) || (97...102).contains($0) }
  }

  private static func isRuntimeID(_ value: String) -> Bool {
    guard let first = value.utf8.first,
          (65...90).contains(first) || (97...122).contains(first) || (48...57).contains(first) else { return false }
    return value.utf8.count <= 128 && value.utf8.allSatisfy {
      (65...90).contains($0) || (97...122).contains($0) || (48...57).contains($0) || [45, 46, 95].contains($0)
    }
  }
}
