import Cocoa
import Darwin
import FlutterMacOS

/// 固定同梱helperへのtransport。資格・権限・Owner判断を保持しない。
final class BrokerProcessChannel {
  private let channel: FlutterMethodChannel
  private let worker = Process()
  private let requests = Pipe()
  private let responses = Pipe()
  private let queue = DispatchQueue(label: "org.gatchimuchio.gui-shell.macos-broker")
  private var available = false
  private var launched = false
  private var closing = false
  private var pending = 0
  private var closeCallbacks = [(Bool) -> Void]()
  private var graceful = true
  #if DEBUG
  private var bootstrap = Set<String>()
  private(set) var productBootstrapObserved = false
  var completedBootstrapOperations: [String] { bootstrap.sorted() }
  #endif

  init(messenger: FlutterBinaryMessenger) {
    channel = FlutterMethodChannel(name: "gui_shell/broker", binaryMessenger: messenger)
    channel.setMethodCallHandler { [weak self] call, result in
      guard let self else { result(Self.failure()); return }
      guard call.method == "request" else { result(FlutterMethodNotImplemented); return }
      guard let frame = call.arguments as? String, !frame.contains("\n"), !frame.contains("\r"),
            frame.utf8.count < 64 * 1024 else { result(Self.failure()); return }
      self.request(frame, result: result)
    }
    do {
      let helper = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/gui_shell_macos_broker")
      let attributes = try helper.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey])
      guard attributes.isRegularFile == true, attributes.isSymbolicLink != true else { return }
      worker.executableURL = helper
      worker.arguments = []
      worker.environment = ["HOME": NSHomeDirectory(), "TMPDIR": NSTemporaryDirectory()]
      worker.standardInput = requests
      worker.standardOutput = responses
      worker.standardError = FileHandle.nullDevice
      worker.terminationHandler = { [weak self] process in
        DispatchQueue.main.async {
          guard let self else { return }
          self.available = false
          let completed = self.closing && self.graceful && process.terminationStatus == 0
          let callbacks = self.closeCallbacks
          self.closeCallbacks.removeAll()
          callbacks.forEach { $0(completed) }
        }
      }
      try worker.run()
      launched = true
      try requests.fileHandleForReading.close()
      try responses.fileHandleForWriting.close()
      guard fcntl(requests.fileHandleForWriting.fileDescriptor, F_SETNOSIGPIPE, 1) == 0 else {
        worker.terminate()
        return
      }
      available = true
    } catch {
      available = false
    }
  }

  private static func failure() -> FlutterError {
    FlutterError(code: "broker_transport_unavailable", message: "同梱の安全Broker接続を確認できません。自動再送しません。", details: nil)
  }

  private func request(_ frame: String, result: @escaping FlutterResult) {
    guard available, !closing, pending < 4 else { result(Self.failure()); return }
    pending += 1
    // 完了判定はmain queueだけ。timeout時は一回だけ失敗し、以後をfail-closedにする。
    var finished = false
    let complete: (String?) -> Void = { [weak self] response in
      guard let self, !finished else { return }
      finished = true
      self.pending -= 1
      guard let response else {
        self.available = false
        self.graceful = false
        if self.worker.isRunning { self.worker.terminate() }
        result(Self.failure())
        return
      }
      #if DEBUG
      if let data = response.data(using: .utf8),
         let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
         let operation = object["operation"] as? String {
        let initial: Set<String> = ["health", "初回設定取得", "ホスト能力", "Host一覧", "アダプター一覧", "Agent一覧", "対話セッション一覧", "normalize_payload", "content_projection", "approval_edit", "command_envelope"]
        let expected = operation == "command_envelope" ? "suspended" : "accepted"
        if initial.contains(operation), object["status"] as? String == expected {
          self.bootstrap.insert(operation)
        }
        #if D4_MACOS_OWNER_UI_TEST
        let becameReady = !self.productBootstrapObserved && initial.isSubset(of: self.bootstrap)
        #endif
        self.productBootstrapObserved = initial.isSubset(of: self.bootstrap)
        #if D4_MACOS_OWNER_UI_TEST
        if becameReady {
          // Dartの初期要求完了によりEngine起動後だと分かる時点で表示補助を一回だけ接続する。
          // 要求、Owner選択、資格は変更しない。通常buildには含めない。
          NotificationCenter.default.post(
            name: NSNotification.Name("NSApplicationDidChangeAccessibilityEnhancedUserInterfaceNotification"),
            object: nil, userInfo: ["AXEnhancedUserInterface": true])
        }
        #endif
      }
      #endif
      result(response)
    }
    // 待機期限の分類だけ。承認の判定・返信はRustのOS確認画面が所有する。
    let envelope = frame.data(using: .utf8).flatMap { try? JSONSerialization.jsonObject(with: $0) as? [String: Any] }
    let operation = envelope?["operation"] as? String
    let ownerOperations: Set<String> = ["アダプター導入", "アダプター更新", "アダプター検証",
      "アダプター有効化", "アダプター無効化", "アダプター隔離", "アダプター削除"]
    let timeout = operation == "AgentCLI実行系作業領域登録" ? 320 : (operation == "作業領域OS選択" || ownerOperations.contains(operation ?? "") ? 305 : 5)
    DispatchQueue.main.asyncAfter(deadline: .now() + .seconds(timeout)) { complete(nil) }
    queue.async { [weak self] in
      guard let self else { return }
      var output: String?
      do {
        let bytes = Data((frame + "\n").utf8)
        try bytes.withUnsafeBytes { raw in
          var sent = 0
          while sent < raw.count {
            let count = Darwin.write(self.requests.fileHandleForWriting.fileDescriptor, raw.baseAddress!.advanced(by: sent), raw.count - sent)
            if count < 0 && errno == EINTR { continue }
            guard count > 0 else { throw PipeFailure.closed }
            sent += count
          }
        }
        var buffer = Data()
        var chunk = [UInt8](repeating: 0, count: 4096)
        while buffer.count <= 4 * 1024 * 1024 {
          let count = Darwin.read(self.responses.fileHandleForReading.fileDescriptor, &chunk, chunk.count)
          if count < 0 && errno == EINTR { continue }
          guard count > 0 else { break }
          buffer.append(contentsOf: chunk.prefix(count))
          if let newline = buffer.firstIndex(of: 0x0A) {
            guard buffer.index(after: newline) == buffer.endIndex, buffer.count <= 4 * 1024 * 1024 else { break }
            output = String(data: buffer[..<newline], encoding: .utf8)
            break
          }
        }
      } catch { output = nil }
      let reply = output
      DispatchQueue.main.async { complete(reply) }
    }
  }

  func close(completion: @escaping (Bool) -> Void) {
    guard launched else { completion(false); return }
    guard worker.isRunning else { completion(closing && graceful && worker.terminationStatus == 0); return }
    closeCallbacks.append(completion)
    guard !closing else { return }
    closing = true
    available = false
    queue.async { [weak self] in try? self?.requests.fileHandleForWriting.close() }
    DispatchQueue.main.asyncAfter(deadline: .now() + .seconds(5)) { [weak self] in
      guard let self, self.worker.isRunning else { return }
      self.graceful = false
      self.worker.terminate()
    }
  }
}

private enum PipeFailure: Error { case closed }
