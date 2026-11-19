import Flutter
import Foundation
import UIKit

final class DeviceLinkNativeService {
  private struct PairingAttempt: Equatable {
    let epoch: UInt64
    let id = UUID()
  }

  private let store = DeviceLinkNativeStore()
  private let transport = DeviceLinkTLSClient()
  private let worker: DispatchQueue
  private let lock = NSLock()
  private var foreground = false
  private var disposed = false
  private var generation: UInt64 = 0
  private var pairNetworkActive = false
  private var pairAttempt: PairingAttempt?
  private var pairResult: FlutterResult?
  private var activeDialog: UIAlertController?
  private var invitationField: UITextField?
  private var observers = [NSObjectProtocol]()
  private let genericFailure = "端末連携を完了できません。資格や通信の詳細は返しません。"

  init(worker: DispatchQueue = DispatchQueue(label: "org.gatchimuchio.gui-shell.device-link.worker", qos: .userInitiated)) {
    self.worker = worker
  }

  func install(messenger: FlutterBinaryMessenger) {
    let channel = FlutterMethodChannel(name: "gui_shell/mobile_device_link", binaryMessenger: messenger)
    channel.setMethodCallHandler { [weak self] call, result in
      guard let self else {
        result(FlutterError(code: "device_link_unavailable", message: "端末連携を利用できません。", details: nil))
        return
      }
      if !self.handle(call, result: result) { result(FlutterMethodNotImplemented) }
    }
    let center = NotificationCenter.default
    observers.append(center.addObserver(forName: UIApplication.didBecomeActiveNotification,
                                        object: nil, queue: .main) { [weak self] _ in
      self?.setForeground(true)
    })
    observers.append(center.addObserver(forName: UIApplication.willResignActiveNotification,
                                        object: nil, queue: .main) { [weak self] _ in
      self?.setForeground(false)
    })
    setForeground(UIApplication.shared.applicationState == .active)
  }

  func close() {
    lock.lock()
    disposed = true
    foreground = false
    generation &+= 1
    lock.unlock()
    transport.cancelAll()
    DispatchQueue.main.async { [weak self] in self?.dismissForBackground() }
    observers.forEach(NotificationCenter.default.removeObserver)
    observers.removeAll()
  }

  func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) -> Bool {
    lock.lock()
    let isDisposed = disposed
    lock.unlock()
    if isDisposed {
      fail(result, code: "device_link_unavailable")
      return true
    }
    switch call.method {
    case "read_state":
      guard arguments(call, exactKeys: ["version"]) != nil else { fail(result); return true }
      perform(result, requiresForeground: false) { try self.readState(epoch: $0) }
      return true
    case "read_recovery_audit":
      guard arguments(call, exactKeys: ["version"]) != nil else { fail(result); return true }
      perform(result) { try self.readLocalRecoveryAudit(epoch: $0) }
      return true
    case "pair":
      guard arguments(call, exactKeys: ["version"]) != nil else { fail(result); return true }
      beginPairing(result)
      return true
    case "broker_request":
      guard let args = arguments(call, exactKeys: ["version", "broker_operation", "payload"]),
            let operation = args["broker_operation"] as? String,
            let payload = args["payload"] as? [String: Any] else { fail(result); return true }
      perform(result) { try self.brokerRequest(operation, payload: payload, epoch: $0) }
      return true
    case "disconnect":
      guard arguments(call, exactKeys: ["version"]) != nil else { fail(result); return true }
      perform(result) { try self.disconnect(epoch: $0) }
      return true
    case "local_delete":
      guard arguments(call, exactKeys: ["version"]) != nil else { fail(result); return true }
      perform(result) { try self.localDelete(epoch: $0) }
      return true
    default:
      return false
    }
  }

  private func arguments(_ call: FlutterMethodCall, exactKeys: Set<String>) -> [String: Any]? {
    guard let values = call.arguments as? [String: Any], Set(values.keys) == exactKeys,
          DeviceLinkStrictJSON.integer(values["version"]) == 1,
          values.values.allSatisfy({ !($0 is NSNull) }) else { return nil }
    return values
  }

  private func perform(_ result: @escaping FlutterResult, requiresForeground: Bool = true,
                       action: @escaping (UInt64) throws -> Any) {
    let epoch = currentState().generation
    guard isCurrent(epoch, requiresForeground: requiresForeground) else {
      fail(result, code: "device_link_failed")
      return
    }
    worker.async { [weak self] in
      guard let self else { return }
      let outcome = Result {
        guard self.isCurrent(epoch, requiresForeground: requiresForeground) else {
          throw DeviceLinkTransportError.failed
        }
        return try action(epoch)
      }
      DispatchQueue.main.async {
        self.lock.lock()
        let isDisposed = self.disposed
        self.lock.unlock()
        if isDisposed { self.fail(result, code: "device_link_unavailable"); return }
        guard self.isCurrent(epoch, requiresForeground: requiresForeground) else {
          self.fail(result, code: "device_link_failed")
          return
        }
        switch outcome {
        case .success(let value): result(value)
        case .failure: self.fail(result, code: "device_link_failed")
        }
      }
    }
  }

  private func fail(_ result: @escaping FlutterResult, code: String = "device_link_invalid_request") {
    result(FlutterError(code: code, message: genericFailure, details: nil))
  }

  func setForeground(_ value: Bool) {
    lock.lock()
    guard !disposed, foreground != value else { lock.unlock(); return }
    foreground = value
    generation &+= 1
    lock.unlock()
    if !value {
      transport.cancelAll()
      dismissForBackground()
    }
  }

  private func currentState() -> (foreground: Bool, disposed: Bool, generation: UInt64, pairing: Bool) {
    lock.lock()
    defer { lock.unlock() }
    return (foreground, disposed, generation, pairAttempt != nil)
  }

  private func canSend(_ epoch: UInt64) -> Bool {
    isCurrent(epoch, requiresForeground: true)
  }

  private func isCurrent(_ epoch: UInt64, requiresForeground: Bool) -> Bool {
    lock.lock()
    defer { lock.unlock() }
    return !disposed && generation == epoch && (!requiresForeground || foreground)
  }

  private func readState(epoch: UInt64) throws -> [String: Any] {
    let state = try store.loadOrCreate()
    let current = currentState()
    if current.pairing {
      return snapshot(paired: state.credential != nil, connected: false, storageReady: true,
                      status: "端末結合処理中のため、通常接続確認を停止しています。")
    }
    guard let raw = state.credential else {
      return snapshot(paired: false, connected: false, storageReady: true,
                      status: "未接続。native画面からDesktopの招待を登録してください。")
    }
    let credential = try DeviceLinkCredential.stored(raw, expectedDeviceID: state.deviceID)
    guard canSend(epoch) else {
      return snapshot(paired: true, connected: false, storageReady: true,
                      status: "バックグラウンド中は接続を停止しています。資格はnative保管にあります。")
    }
    guard !credential.isExpired() else {
      return snapshot(paired: true, connected: false, storageReady: true,
                      status: "端末資格の期限が切れています。Desktopで旧結合を失効し、新しい招待を発行してください。")
    }
    let reply = try transport.exchange(credential, operation: "端末確認", payload: [:],
                                       canSend: { self.canSend(epoch) })
    let connected = reply["status"] as? String == "accepted" && canSend(epoch)
    return snapshot(paired: true, connected: connected, storageReady: true,
                    status: connected ? "Desktop端末資格を確認しました。" : "Desktop接続を確認できません。資格は削除せず、通信を停止しています。")
  }

  private func readLocalRecoveryAudit(epoch: UInt64) throws -> [[String: Any]] {
    guard canSend(epoch), !currentState().pairing else { throw DeviceLinkTransportError.failed }
    return try store.readLocalRecoveryAudit()
  }

  private func brokerRequest(_ operation: String, payload: [String: Any], epoch: UInt64) throws -> [String: Any] {
    try DeviceLinkPayloadPolicy.validate(operation, payload: payload)
    let current = currentState()
    guard !current.pairing, canSend(epoch) else { throw DeviceLinkTransportError.failed }
    let state = try store.loadOrCreate()
    guard let raw = state.credential else { throw DeviceLinkTransportError.failed }
    let credential = try DeviceLinkCredential.stored(raw, expectedDeviceID: state.deviceID)
    guard !credential.isExpired() else { throw DeviceLinkTransportError.failed }
    return try transport.exchange(credential, operation: operation, payload: payload,
                                  canSend: { self.canSend(epoch) })
  }

  private func disconnect(epoch: UInt64) throws -> [String: Any] {
    let current = currentState()
    guard !current.pairing, canSend(epoch) else { throw DeviceLinkTransportError.failed }
    let state = try store.loadOrCreate()
    guard let raw = state.credential else { return try localSnapshot() }
    let credential = try DeviceLinkCredential.stored(raw, expectedDeviceID: state.deviceID)
    guard !credential.isExpired() else { throw DeviceLinkTransportError.failed }
    let reply = try transport.exchange(credential, operation: "端末離脱", payload: [:],
                                       canSend: { self.canSend(epoch) })
    guard canSend(epoch), reply["status"] as? String == "accepted",
          NSDictionary(dictionary: reply["body"] as? [String: Any] ?? [:]).isEqual(to: ["状態": "失効"]) else {
      throw DeviceLinkTransportError.failed
    }
    let deleted = try store.deleteCredential()
    guard deleted.credential == nil else { throw DeviceLinkTransportError.failed }
    return snapshot(paired: false, connected: false, storageReady: true,
                    status: "Desktop側の結合解除と端末内資格の削除を確認しました。送信済み処理の停止は保証しません。")
  }

  private func localDelete(epoch: UInt64) throws -> [String: Any] {
    guard canSend(epoch), !currentState().pairing else { throw DeviceLinkTransportError.failed }
    let deleted = try store.deleteCredentialWithLocalRecoveryAudit()
    guard deleted.credential == nil else { throw DeviceLinkTransportError.failed }
    guard !deleted.localRecoveryAudit.isEmpty else { throw DeviceLinkTransportError.failed }
    return snapshot(paired: false, connected: false, storageReady: true,
                    status: "端末内資格を削除し、端末内回復記録を保存しました。Desktop側の失効は未確認です。ownerに失効を依頼してください。")
  }

  private func beginPairing(_ result: @escaping FlutterResult) {
    lock.lock()
    guard pairAttempt == nil, foreground, !disposed else { lock.unlock(); fail(result); return }
    let attempt = PairingAttempt(epoch: generation)
    pairAttempt = attempt
    pairResult = result
    lock.unlock()
    worker.async { [weak self] in
      guard let self else { return }
      let state = self.canSend(attempt.epoch) ? (try? self.store.loadOrCreate()) : nil
      DispatchQueue.main.async {
        guard self.canSend(attempt.epoch), self.isCurrentPair(attempt) else {
          self.finishPairing(nil, attempt: attempt)
          return
        }
        guard let state else { self.finishPairing(self.storageFailureSnapshot(), attempt: attempt); return }
        guard state.credential == nil else {
          self.finishPairing(self.localSnapshotSafely(), attempt: attempt)
          return
        }
        self.presentInvitationEntry(state, message: nil, attempt: attempt)
      }
    }
  }

  private func presentInvitationEntry(_ state: DeviceLinkNativeState, message: String?, attempt: PairingAttempt) {
    guard canSend(attempt.epoch), isCurrentPair(attempt), let presenter = topPresenter() else {
      cancelPairing(attempt: attempt)
      return
    }
    let alert = UIAlertController(title: "端末を結合",
                                  message: message ?? "Desktop ownerへ伝える端末ID:\n\(state.deviceID)\n\n受け取った招待JSONを入力してください。招待秘密はnative内で処理します。",
                                  preferredStyle: .alert)
    alert.addTextField { field in
      field.placeholder = "Desktopの端末招待JSON"
      field.isSecureTextEntry = true
      field.autocorrectionType = .no
      field.spellCheckingType = .no
      field.smartQuotesType = .no
      field.smartDashesType = .no
      field.clearButtonMode = .whileEditing
      field.accessibilityLabel = "Desktopの端末招待JSON"
    }
    let field = alert.textFields?.first
    invitationField = field
    alert.addAction(UIAlertAction(title: "キャンセル", style: .cancel) { [weak self, weak field] _ in
      field?.text = nil
      self?.cancelPairing(attempt: attempt)
    })
    alert.addAction(UIAlertAction(title: "招待を確認", style: .default) { [weak self, weak field] _ in
      guard let self else { return }
      var raw = field?.text ?? ""
      field?.text = nil
      defer { raw.removeAll(keepingCapacity: false) }
      guard self.canSend(attempt.epoch), self.isCurrentPair(attempt) else {
        self.cancelPairing(attempt: attempt)
        return
      }
      guard let invitation = try? DeviceLinkCredential.invitation(raw, expectedDeviceID: state.deviceID) else {
        DispatchQueue.main.async {
          self.presentInvitationEntry(state, message: "招待の形式・期限・端末IDを確認してください。", attempt: attempt)
        }
        return
      }
      DispatchQueue.main.async { self.presentHostConfirmation(invitation, state: state, attempt: attempt) }
    })
    activeDialog = alert
    presenter.present(alert, animated: true)
  }

  private func presentHostConfirmation(_ invitation: DeviceLinkCredential, state: DeviceLinkNativeState, attempt: PairingAttempt) {
    guard canSend(attempt.epoch), isCurrentPair(attempt), let presenter = topPresenter() else {
      cancelPairing(attempt: attempt)
      return
    }
    let details = "接続先: \(invitation.host):\(invitation.port)\nHostID: \(invitation.hostID)\n証明書hash: \(invitation.certificateHash)\n\nDesktop ownerの画面と一致することを確認してください。"
    let alert = UIAlertController(title: "接続先を照合", message: details, preferredStyle: .alert)
    alert.addAction(UIAlertAction(title: "キャンセル", style: .cancel) { [weak self] _ in
      self?.cancelPairing(attempt: attempt)
    })
    alert.addAction(UIAlertAction(title: "一致を確認して結合", style: .default) { [weak self] _ in
      self?.startPairing(invitation, state: state, attempt: attempt)
    })
    activeDialog = alert
    invitationField = nil
    presenter.present(alert, animated: true)
  }

  private func startPairing(_ invitation: DeviceLinkCredential, state: DeviceLinkNativeState, attempt: PairingAttempt) {
    lock.lock()
    guard pairAttempt == attempt, generation == attempt.epoch, foreground, !disposed else {
      lock.unlock()
      cancelPairing(attempt: attempt)
      return
    }
    pairNetworkActive = true
    lock.unlock()
    activeDialog?.dismiss(animated: false)
    activeDialog = nil
    invitationField?.text = nil
    invitationField = nil
    worker.async { [weak self] in
      guard let self else { return }
      let value = self.pair(invitation, state: state, epoch: attempt.epoch)
      DispatchQueue.main.async { self.finishPairing(value, attempt: attempt) }
    }
  }

  private func pair(_ invitation: DeviceLinkCredential, state: DeviceLinkNativeState,
                    epoch: UInt64) -> [String: Any] {
    guard canSend(epoch) else { return localSnapshotSafely() }
    do {
      let response = try transport.exchange(invitation, operation: "端末結合", payload: [:],
                                            canSend: { self.canSend(epoch) }, allowCredentialResponse: true)
      guard response["status"] as? String == "accepted",
            let body = response["body"] as? [String: Any] else { throw DeviceLinkTransportError.failed }
      let paired = try DeviceLinkCredential.paired(body, invitation: invitation)
      do {
        let stored = try store.storeCredential(paired)
        guard let raw = stored.credential,
              try DeviceLinkCredential.stored(raw, expectedDeviceID: state.deviceID) == paired else {
          throw DeviceLinkJSONError.invalid
        }
      } catch {
        let revoked = revoke(paired, epoch: epoch)
        _ = try? store.deleteCredential()
        return snapshot(paired: false, connected: false, storageReady: false,
                        status: revoked ? "安全保管に失敗しDesktop結合を解除しました。新しい招待でやり直してください。" : "安全保管に失敗しDesktop失効を確認できません。ownerが端末を失効させてください。")
      }
      let verified = (try? transport.exchange(paired, operation: "端末確認", payload: [:],
                                              canSend: { self.canSend(epoch) })["status"] as? String) == "accepted"
      let connected = verified && canSend(epoch)
      return snapshot(paired: true, connected: connected, storageReady: true,
                      status: connected ? "Desktop端末資格を確認し、ThisDeviceOnly Keychainへ保存を再読しました。" : "結合資格はThisDeviceOnly Keychainへ保存しましたが、Desktop接続を確認できません。再確認してください。")
    } catch {
      return pairingFailure()
    }
  }

  private func revoke(_ credential: DeviceLinkCredential, epoch: UInt64) -> Bool {
    guard canSend(epoch),
          let reply = try? transport.exchange(credential, operation: "端末離脱", payload: [:],
                                              canSend: { self.canSend(epoch) }) else { return false }
    return reply["status"] as? String == "accepted" &&
      NSDictionary(dictionary: reply["body"] as? [String: Any] ?? [:]).isEqual(to: ["状態": "失効"])
  }

  private func pairingFailure() -> [String: Any] {
    guard let stored = try? store.loadOrCreate() else { return storageFailureSnapshot() }
    if stored.credential != nil {
      return snapshot(paired: true, connected: false, storageReady: true,
                      status: "結合後の保存状態を確認できません。Desktopで端末一覧を確認してください。")
    }
    return snapshot(paired: false, connected: false, storageReady: true,
                    status: "結合を確認できません。招待を再利用せず、Desktopで端末状態を確認して新しい招待を発行してください。")
  }

  private func localSnapshot() throws -> [String: Any] {
    let state = try store.loadOrCreate()
    let paired = state.credential != nil
    return snapshot(paired: paired, connected: false, storageReady: true,
                    status: currentState().foreground
                      ? (paired ? "接続確認が必要です。" : "未接続。native画面からDesktopの招待を登録してください。")
                      : "バックグラウンド中は接続を停止しています。")
  }

  private func localSnapshotSafely() -> [String: Any] {
    (try? localSnapshot()) ?? storageFailureSnapshot()
  }

  private func storageFailureSnapshot() -> [String: Any] {
    snapshot(paired: false, connected: false, storageReady: false,
             status: "ThisDeviceOnly Keychainを確認できません。別保管や平文へfallbackせず停止しています。")
  }

  private func snapshot(paired: Bool, connected: Bool, storageReady: Bool,
                        status: String) -> [String: Any] {
    let state = currentState()
    return DeviceLinkSnapshot(storageReady: storageReady, paired: paired, connected: connected,
                              foreground: state.foreground && !state.disposed,
                              status: status).channelValue
  }

  private func finishPairing(_ value: [String: Any]?, attempt: PairingAttempt) {
    lock.lock()
    guard pairAttempt == attempt else { lock.unlock(); return }
    let completion = pairResult
    pairResult = nil
    pairAttempt = nil
    pairNetworkActive = false
    lock.unlock()
    invitationField?.text = nil
    invitationField = nil
    activeDialog?.dismiss(animated: false)
    activeDialog = nil
    if let completion {
      if let value, canSend(attempt.epoch) { completion(value) }
      else { fail(completion, code: "device_link_failed") }
    }
  }

  private func isCurrentPair(_ attempt: PairingAttempt) -> Bool {
    lock.lock()
    defer { lock.unlock() }
    return pairAttempt == attempt
  }

  private func cancelPairing(attempt: PairingAttempt) {
    lock.lock()
    guard pairAttempt == attempt else { lock.unlock(); return }
    let activeNetwork = pairNetworkActive
    lock.unlock()
    if activeNetwork {
      transport.cancelAll()
      return
    }
    worker.async { [weak self] in
      guard let self else { return }
      let value = self.localSnapshotSafely()
      DispatchQueue.main.async { self.finishPairing(value, attempt: attempt) }
    }
  }

  private func dismissForBackground() {
    activeDialog?.dismiss(animated: false)
    activeDialog = nil
    invitationField?.text = nil
    invitationField = nil
    lock.lock()
    let attempt = pairNetworkActive ? nil : pairAttempt
    lock.unlock()
    if let attempt { finishPairing(nil, attempt: attempt) }
  }

  private func topPresenter() -> UIViewController? {
    let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
      .filter { $0.activationState == .foregroundActive }
    guard let root = scenes.flatMap(\.windows).first(where: \.isKeyWindow)?.rootViewController else { return nil }
    var top = root
    while let presented = top.presentedViewController { top = presented }
    return top
  }
}
