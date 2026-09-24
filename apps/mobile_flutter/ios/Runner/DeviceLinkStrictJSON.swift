import Foundation
import CoreFoundation

enum DeviceLinkJSONError: Error {
  case invalid
}

enum DeviceLinkStrictJSON {
  static func parseObject(_ text: String, maximumBytes: Int) throws -> [String: Any] {
    try parseObject(Data(text.utf8), maximumBytes: maximumBytes)
  }

  static func parseObject(_ data: Data, maximumBytes: Int) throws -> [String: Any] {
    guard !data.isEmpty, data.count <= maximumBytes else { throw DeviceLinkJSONError.invalid }
    var scanner = Scanner(bytes: Array(data))
    try scanner.scanDocument()
    let value = try JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed])
    guard let object = value as? [String: Any] else { throw DeviceLinkJSONError.invalid }
    try validateTree(object, depth: 0)
    return object
  }

  static func encodeObject(_ object: [String: Any], maximumBytes: Int) throws -> Data {
    guard JSONSerialization.isValidJSONObject(object) else { throw DeviceLinkJSONError.invalid }
    let data = try JSONSerialization.data(withJSONObject: object, options: [.sortedKeys, .fragmentsAllowed])
    guard data.count <= maximumBytes else { throw DeviceLinkJSONError.invalid }
    _ = try parseObject(data, maximumBytes: maximumBytes)
    return data
  }

  static func isBoolean(_ value: Any) -> Bool {
    guard let number = value as? NSNumber else { return false }
    return CFGetTypeID(number) == CFBooleanGetTypeID()
  }

  static func integer(_ value: Any?) -> Int64? {
    guard let value, !isBoolean(value), let number = value as? NSNumber,
          let integer = Int64(number.stringValue), String(integer) == number.stringValue else { return nil }
    return integer
  }

  private static func validateTree(_ value: Any, depth: Int) throws {
    guard depth <= 32 else { throw DeviceLinkJSONError.invalid }
    if value is NSNull || isBoolean(value) || value is String { return }
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
      for item in map.values { try validateTree(item, depth: depth + 1) }
      return
    }
    throw DeviceLinkJSONError.invalid
  }

  private struct Scanner {
    let bytes: [UInt8]
    var index = 0
    var members = 0

    mutating func scanDocument() throws {
      skipWhitespace()
      try scanValue(depth: 0)
      skipWhitespace()
      guard index == bytes.count else { throw DeviceLinkJSONError.invalid }
    }

    private mutating func scanValue(depth: Int) throws {
      guard depth <= 32, index < bytes.count else { throw DeviceLinkJSONError.invalid }
      skipWhitespace()
      guard index < bytes.count else { throw DeviceLinkJSONError.invalid }
      switch bytes[index] {
      case 0x7B:
        try scanObject(depth: depth + 1)
      case 0x5B:
        try scanArray(depth: depth + 1)
      case 0x22:
        _ = try scanString()
      default:
        try scanPrimitive()
      }
    }

    private mutating func scanObject(depth: Int) throws {
      guard depth <= 32 else { throw DeviceLinkJSONError.invalid }
      index += 1
      skipWhitespace()
      if consume(0x7D) { return }
      var keys = Set<String>()
      while true {
        skipWhitespace()
        let token = try scanString()
        let wrapped = Data([0x5B] + token + [0x5D])
        guard let decoded = try JSONSerialization.jsonObject(with: wrapped) as? [String],
              decoded.count == 1, keys.insert(decoded[0]).inserted else {
          throw DeviceLinkJSONError.invalid
        }
        members += 1
        guard members <= 4096 else { throw DeviceLinkJSONError.invalid }
        skipWhitespace()
        guard consume(0x3A) else { throw DeviceLinkJSONError.invalid }
        try scanValue(depth: depth)
        skipWhitespace()
        if consume(0x7D) { return }
        guard consume(0x2C) else { throw DeviceLinkJSONError.invalid }
      }
    }

    private mutating func scanArray(depth: Int) throws {
      guard depth <= 32 else { throw DeviceLinkJSONError.invalid }
      index += 1
      skipWhitespace()
      if consume(0x5D) { return }
      var count = 0
      while true {
        count += 1
        guard count <= 4096 else { throw DeviceLinkJSONError.invalid }
        try scanValue(depth: depth)
        skipWhitespace()
        if consume(0x5D) { return }
        guard consume(0x2C) else { throw DeviceLinkJSONError.invalid }
      }
    }

    private mutating func scanString() throws -> [UInt8] {
      skipWhitespace()
      guard consume(0x22) else { throw DeviceLinkJSONError.invalid }
      let start = index - 1
      var escaped = false
      while index < bytes.count {
        let byte = bytes[index]
        index += 1
        if escaped {
          if byte == 0x75 {
            guard index + 4 <= bytes.count else { throw DeviceLinkJSONError.invalid }
            index += 4
          }
          escaped = false
        } else if byte == 0x5C {
          escaped = true
        } else if byte == 0x22 {
          return Array(bytes[start..<index])
        }
      }
      throw DeviceLinkJSONError.invalid
    }

    private mutating func scanPrimitive() throws {
      let start = index
      while index < bytes.count && ![0x20, 0x09, 0x0A, 0x0D, 0x2C, 0x5D, 0x7D].contains(bytes[index]) {
        index += 1
      }
      guard index > start else { throw DeviceLinkJSONError.invalid }
    }

    private mutating func skipWhitespace() {
      while index < bytes.count && [0x20, 0x09, 0x0A, 0x0D].contains(bytes[index]) { index += 1 }
    }

    private mutating func consume(_ byte: UInt8) -> Bool {
      guard index < bytes.count, bytes[index] == byte else { return false }
      index += 1
      return true
    }
  }
}
