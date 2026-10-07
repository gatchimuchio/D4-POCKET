import Cocoa
import FlutterMacOS

class MainFlutterWindow: NSWindow {
  private(set) var brokerChannel: BrokerProcessChannel?
  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    let windowFrame = self.frame
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)

    RegisterGeneratedPlugins(registry: flutterViewController)
    brokerChannel = BrokerProcessChannel(messenger: flutterViewController.engine.binaryMessenger)

    super.awakeFromNib()
    #if DEBUG && D4_MACOS_OWNER_UI_TEST
    // 試験表示だけ。固定Flutterが受けるアクセシビリティ通知でsemanticsを有効にする。
    // Broker、要求、Owner選択、資格の状態は変更しない。通常buildには含めない。
    DispatchQueue.main.async {
      NotificationCenter.default.post(
        name: NSNotification.Name("NSApplicationDidChangeAccessibilityEnhancedUserInterfaceNotification"),
        object: nil, userInfo: ["AXEnhancedUserInterface": true])
    }
    #endif
  }
}
