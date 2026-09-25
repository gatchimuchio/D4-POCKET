import Flutter
import Foundation
import UIKit

final class DeviceLinkNativeService {
  private let store = DeviceLinkNativeStore()
  private let transport = DeviceLinkTLSClient()
  private let worker = DispatchQueue(label: "org.gatchimuchio.gui-shell.device-link.worker", qos: .userInitiated)
  private let lock = NSLock()
  private var foreground = false
  private var disposed = false
  private var generation: UInt64 = 0
  private var pairing = false
  private var pairNetworkActive = false
  private var pairResult: FlutterResult?
  private var activeDialog: UIAlertController?
  private var invitationField: UITextField?
  private var observers = [NSObjectProtocol]()
  private let genericFailure = "端末連携を完了できません。資格や通信の詳細は返しません。"

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

  private func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) -> Bool {
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
      perform(result) { try self.readState() }
      return true
    case "read_recovery_audit":
      guard arguments(call, exactKeys: ["version"]) != nil else { fail(result); return true }
      perform(result) { try self.readLocalRecoveryAudit() }
      return true
    case "pair":
      guard arguments(call, exactKeys: ["version"]) != nil else { fail(result); return true }
      beginPairing(result)
      return true
    case "broker_request":
      guard let args = arguments(call, exactKeys: ["version", "broker_operation", "payload"]),
            let operation = args["broker_operation"] as? String,
            let payload = args["payload"] as? [String: Any] else { fail(result); return true }
      perform(result) { try self.brokerRequest(operation, payload: payload) }
      return true
    case "disconnect":
      guard arguments(call, exactKeys: ["version"]) != nil else { fail(result); return true }
      perform(result) { try self.disconnect() }
      return true
    case "local_delete":
      guard arguments(call, exactKeys: ["version"]) != nil else { fail(result); return true }
      perform(result) { try self.localDelete() }
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

  private func perform(_ result: @escaping FlutterResult, action: @escaping () throws -> Any) {
    worker.async { [weak self] in
      guard let self else { return }
      let outcome = Result { try action() }
      DispatchQueue.main.async {
        self.lock.lock()
        let isDisposed = self.disposed
        self.lock.unlock()
        if isDisposed { self.fail(result, code: "device_link_unavailable"); return }
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

  private func setForeground(_ value: Bool) {
    lock.lock()
    guard !disposed, foreground != value else { lock.unlock(); return }
    foreground = value
    generation &+= 1
    lock.unlock()
    if !value {
      transport.cancelAll()
      DispatchQueue.main.async { [weak self] in self?.dismissForBackground() }
    }
  }

  private func currentState() -> (foreground: Bool, disposed: Bool, generation: UInt64, pairing: Bool) {
    lock.lock()
    defer { lock.unlock() }
    return (foreground, disposed, generation, pairing)
  }

  private func canSend(_ epoch: UInt64) -> Bool {
    lock.lock()
    defer { lock.unlock() }
    return foreground && !disposed && generation == epoch
  }

  private func readState() throws -> [String: Any] {
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
    guard current.foreground, !current.disposed else {
      return snapshot(paired: true, connected: false, storageReady: true,
                      status: "バックグラウンド中は接続を停止しています。資格はnative保管にあります。")
    }
    guard !credential.isExpired() else {
      return snapshot(paired: true, connected: false, storageReady: true,
                      status: "端末資格の期限が切れています。Desktopで旧結合を失効し、新しい招待を発行してください。")
    }
    let reply = try transport.exchange(credential, operation: "端末確認", payload: [:],
                                       canSend: { self.canSend(current.generation) })
    let connected = reply["status"] as? String == "accepted" && canSend(current.generation)
    return snapshot(paired: true, connected: connected, storageReady: true,
                    status: connected ? "Desktop端末資格を確認しました。" : "Desktop接続を確認できません。資格は削除せず、通信を停止しています。")
  }

  private func readLocalRecoveryAudit() throws -> [[String: Any]] {
    let current = currentState()
    guard current.foreground, !current.disposed, !current.pairing else { throw DeviceLinkTransportError.failed }
    return try store.readLocalRecoveryAudit()
  }

  private func brokerRequest(_ operation: String, payload: [String: Any]) throws -> [String: Any] {
    try DeviceLinkPayloadPolicy.validate(operation, payload: payload)
    let current = currentState()
    guard !current.pairing, current.foreground, !current.disposed else { throw DeviceLinkTransportError.failed }
    let state = try store.loadOrCreate()
    guard let raw = state.credential else { throw DeviceLinkTransportError.failed }
    let credential = try DeviceLinkCredential.stored(raw, expectedDeviceID: state.deviceID)
    guard !credential.isExpired() else { throw DeviceLinkTransportError.failed }
    return try transport.exchange(credential, operation: operation, payload: payload,
                                  canSend: { self.canSend(current.generation) })
  }

  private func disconnect() throws -> [String: Any] {
    let current = currentState()
    guard !current.pairing, current.foreground, !current.disposed else { throw DeviceLinkTransportError.failed }
    let state = try store.loadOrCreate()
    guard let raw = state.credential else { return localSnapshot() }
    let credential = try DeviceLinkCredential.stored(raw, expectedDeviceID: state.deviceID)
    guard !credential.isExpired() else { throw DeviceLinkTransportError.failed }
    let reply = try transport.exchange(credential, operation: "端末離脱", payload: [:],
                                       canSend: { self.canSend(current.generation) })
    guard reply["status"] as? String == "accepted",
          NSDictionary(dictionary: reply["body"] as? [String: Any] ?? [:]).isEqual(to: ["状態": "失効"]) else {
      throw DeviceLinkTransportError.failed
    }
    let deleted = try store.deleteCredential()
    guard deleted.credential == nil else { throw DeviceLinkTransportError.failed }
    return snapshot(paired: false, connected: false, storageReady: true,
                    status: "Desktop側の結合解除と端末内資格の削除を確認しました。送信済み処理の停止は保証しません。")
  }

  private func localDelete() throws -> [String: Any] {
    let current = currentState()
    guard !current.pairing, current.foreground, !current.disposed else { throw DeviceLinkTransportError.failed }
    let deleted = try store.deleteCredentialWithLocalRecoveryAudit()
    guard deleted.credential == nil else { throw DeviceLinkTransportError.failed }
    guard !deleted.localRecoveryAudit.isEmpty else { throw DeviceLinkTransportError.failed }
    return snapshot(paired: false, connected: false, storageReady: true,
                    status: "端末内資格を削除し、端末内回復記録を保存しました。Desktop側の失効は未確認です。ownerに失効を依頼してください。")
  }

  private func beginPairing(_ result: @escaping FlutterResult) {
    lock.lock()
    guard !pairing, foreground, !disposed else { lock.unlock(); fail(result); return }
    pairing = true
    pairResult = result
    lock.unlock()
    worker.async { [weak self] in
      guard let self else { return }
      let state = try? self.store.loadOrCreate()
      DispatchQueue.main.async {
        guard let state else { self.finishPairing(self.storageFailureSnapshot()); return }
        let current = self.currentState()
        guard current.foreground, !current.disposed else {
          self.finishPairing(self.localSnapshotSafely())
          return
        }
        guard state.credential == nil else {
          self.finishPairing(self.localSnapshotSafely())
          return
        }
        self.presentInvitationEntry(state, message: nil)
      }
    }
  }

  private func presentInvitationEntry(_ state: DeviceLinkNativeState, message: String?) {
    guard currentState().foreground, let presenter = topPresenter() else {
      cancelPairing()
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
      self?.cancelPairing()
    })
    alert.addAction(UIAlertAction(title: "招待を確認", style: .default) { [weak self, weak field] _ in
      guard let self else { return }
      var raw = field?.text ?? ""
      field?.text = nil
      defer { raw.removeAll(keepingCapacity: false) }
      guard let invitation = try? DeviceLinkCredential.invitation(raw, expectedDeviceID: state.deviceID) else {
        DispatchQueue.main.async { self.presentInvitationEntry(state, message: "招待の形式・期限・端末IDを確認してください。") }
        return
      }
      DispatchQueue.main.async { self.presentHostConfirmation(invitation, state: state) }
    })
    activeDialog = alert
    presenter.present(alert, animated: true)
  }

  private func presentHostConfirmation(_ invitation: DeviceLinkCredential, state: DeviceLinkNativeState) {
    guard currentState().foreground, let presenter = topPresenter() else {
      cancelPairing()
      return
    }
    let details = "接続先: \(invitation.host):\(invitation.port)\nHostID: \(invitation.hostID)\n証明書hash: \(invitation.certificateHash)\n\nDesktop ownerの画面と一致することを確認してください。"
    let alert = UIAlertController(title: "接続先を照合", message: details, preferredStyle: .alert)
    alert.addAction(UIAlertAction(title: "キャンセル", style: .cancel) { [weak self] _ in self?.cancelPairing() })
    alert.addAction(UIAlertAction(title: "一致を確認して結合", style: .default) { [weak self] _ in
      self?.startPairing(invitation, state: state)
    })
    activeDialog = alert
    invitationField = nil
    presenter.present(alert, animated: true)
  }

  private func startPairing(_ invitation: DeviceLinkCredential, state: DeviceLinkNativeState) {
    lock.lock()
    guard pairing, foreground, !disposed else { lock.unlock(); cancelPairing(); return }
    pairNetworkActive = true
    let epoch = generation
    lock.unlock()
    activeDialog?.dismiss(animated: false)
    activeDialog = nil
    invitationField?.text = nil
    invitationField = nil
    worker.async { [weak self] in
      guard let self else { return }
      let value = self.pair(invitation, state: state, epoch: epoch)
      DispatchQueue.main.async { self.finishPairing(value) }
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

  private func finishPairing(_ value: [String: Any]) {
    lock.lock()
    let completion = pairResult
    pairResult = nil
    pairing = false
    pairNetworkActive = false
    lock.unlock()
    invitationField?.text = nil
    invitationField = nil
    activeDialog?.dismiss(animated: false)
    activeDialog = nil
    completion?(value)
  }

  private func cancelPairing() {
    lock.lock()
    let activeNetwork = pairNetworkActive
    lock.unlock()
    if activeNetwork {
      transport.cancelAll()
      return
    }
    worker.async { [weak self] in
      guard let self else { return }
      let value = self.localSnapshotSafely()
      DispatchQueue.main.async { self.finishPairing(value) }
    }
  }

  private func dismissForBackground() {
    let current = currentState()
    activeDialog?.dismiss(animated: false)
    activeDialog = nil
    invitationField?.text = nil
    invitationField = nil
    guard current.pairing, !currentPairNetworkActive() else { return }
    worker.async { [weak self] in
      guard let self else { return }
      let value = self.localSnapshotSafely()
      DispatchQueue.main.async { self.finishPairing(value) }
    }
  }

  private func currentPairNetworkActive() -> Bool {
    lock.lock()
    defer { lock.unlock() }
    return pairNetworkActive
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
