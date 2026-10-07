// Simulatorの明示的な試験buildだけで、招待をnative secure fieldへ渡す。
// UI操作・確認・資格保存・Broker要求は既存の製品経路に任せる。
#if D4_IOS_PRODUCT_TEST
#if !targetEnvironment(simulator)
#error("端末招待の試験入力はSimulator専用")
#endif
import Foundation
import Network
import UIKit

enum DeviceLinkProductTestInput {
  private static var observer: NSObjectProtocol?
  private static var attempted = false

  static func install() {
    guard let rawPort = ProcessInfo.processInfo.environment["D4_IOS_PRODUCT_BRIDGE_PORT"],
          let port = UInt16(rawPort), let endpoint = NWEndpoint.Port(rawValue: port), port > 0 else { return }
    observer = NotificationCenter.default.addObserver(
      forName: UITextField.textDidBeginEditingNotification, object: nil, queue: .main
    ) { notification in
      guard !attempted, let field = notification.object as? UITextField,
            field.isSecureTextEntry, field.placeholder == "Desktopの端末招待JSON" else { return }
      let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
      var controller = scenes.flatMap(\.windows).first(where: \.isKeyWindow)?.rootViewController
      while let presented = controller?.presentedViewController { controller = presented }
      guard let alert = controller as? UIAlertController, alert.title == "端末を結合",
            let message = alert.message,
            let range = message.range(of: "[a-f0-9]{32}", options: .regularExpression) else { return }
      attempted = true
      let deviceID = String(message[range])
      let connection = NWConnection(host: "127.0.0.1", port: endpoint, using: .tcp)
      let queue = DispatchQueue(label: "org.gatchimuchio.gui-shell.test.input")
      var buffer = Data()
      var finished = false
      func finish(_ value: String?) {
        guard !finished else { return }
        finished = true
        connection.stateUpdateHandler = nil
        connection.forceCancel()
        buffer.removeAll(keepingCapacity: false)
        DispatchQueue.main.async { [weak field, weak alert] in
          guard let field, field.window != nil, alert?.title == "端末を結合" else { return }
          guard let value,
                (try? DeviceLinkCredential.invitation(value, expectedDeviceID: deviceID)) != nil else {
            field.accessibilityIdentifier = "D4ProductInvitationFailed"
            return
          }
          field.text = value
          field.accessibilityIdentifier = "D4ProductInvitationReady"
        }
      }
      func receive() {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 8192) { data, _, complete, error in
          guard error == nil else { finish(nil); return }
          if let data { buffer.append(data) }
          guard buffer.count <= 8193 else { finish(nil); return }
          if let newline = buffer.firstIndex(of: 0x0A) {
            guard buffer.index(after: newline) == buffer.endIndex else { finish(nil); return }
            finish(String(data: buffer[..<newline], encoding: .utf8))
          } else if complete { finish(nil) }
          else { receive() }
        }
      }
      connection.stateUpdateHandler = { state in
        switch state {
        case .ready:
          connection.send(content: Data((deviceID + "\n").utf8), completion: .contentProcessed { error in
            if error == nil { receive() } else { finish(nil) }
          })
        case .failed, .cancelled: finish(nil)
        default: break
        }
      }
      connection.start(queue: queue)
      queue.asyncAfter(deadline: .now() + .seconds(20)) { finish(nil) }
    }
  }
}
#endif
