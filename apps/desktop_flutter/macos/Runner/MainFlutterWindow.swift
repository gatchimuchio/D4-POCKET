import Cocoa
import CoreFoundation
import FlutterMacOS
import UserNotifications

class MainFlutterWindow: NSWindow, NSWindowDelegate {
  private(set) var brokerChannel: BrokerProcessChannel?
  private var trayController: DesktopTrayController?

  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    let windowFrame = self.frame
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)

    RegisterGeneratedPlugins(registry: flutterViewController)
    brokerChannel = BrokerProcessChannel(messenger: flutterViewController.engine.binaryMessenger)

    super.awakeFromNib()
    delegate = self
    if let closeButton = standardWindowButton(.closeButton) {
      closeButton.target = self
      closeButton.action = #selector(hideWindowFromCloseButton(_:))
    }
    trayController = DesktopTrayController(
      window: self,
      messenger: flutterViewController.engine.binaryMessenger
    )
  }

  @objc private func hideWindowFromCloseButton(_ sender: Any?) {
    trayController?.hideWindowForTray()
  }

  func windowDidMiniaturize(_ notification: Notification) {
    trayController?.visibilityDidChange()
  }

  func windowDidDeminiaturize(_ notification: Notification) {
    trayController?.visibilityDidChange()
  }
}

private final class DesktopTrayController: NSObject, UNUserNotificationCenterDelegate {
  private weak var window: NSWindow?
  private let channel: FlutterMethodChannel
  private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
  private let runtimeItem = NSMenuItem(title: "実行系状態: 不明", action: nil, keyEquivalent: "")
  private let approvalItem = NSMenuItem(title: "保留承認: 不明", action: nil, keyEquivalent: "")
  private let notificationItem = NSMenuItem(title: "重大通知: 不明", action: nil, keyEquivalent: "")
  private let stopItem = NSMenuItem(title: "全Runtime停止を要求", action: nil, keyEquivalent: "")
  private var applicationVisibilityObservers: [NSObjectProtocol] = []

  init(window: NSWindow, messenger: FlutterBinaryMessenger) {
    self.window = window
    channel = FlutterMethodChannel(name: "gui_shell/tray", binaryMessenger: messenger)
    super.init()
    UNUserNotificationCenter.current().delegate = self

    let menu = NSMenu()
    let openItem = NSMenuItem(title: "D4 Pocketを開く", action: #selector(openWindow), keyEquivalent: "")
    openItem.target = self
    runtimeItem.isEnabled = false
    approvalItem.isEnabled = false
    notificationItem.isEnabled = false
    stopItem.action = #selector(requestStop)
    stopItem.target = self
    stopItem.isEnabled = false
    let exitItem = NSMenuItem(title: "終了", action: #selector(terminateApplication), keyEquivalent: "")
    exitItem.target = self
    menu.addItem(openItem)
    menu.addItem(runtimeItem)
    menu.addItem(approvalItem)
    menu.addItem(notificationItem)
    menu.addItem(.separator())
    menu.addItem(stopItem)
    menu.addItem(.separator())
    menu.addItem(exitItem)
    statusItem.menu = menu
    statusItem.button?.title = "D4"
    statusItem.button?.toolTip = "D4 Pocket"
    applicationVisibilityObservers = [
      NotificationCenter.default.addObserver(
        forName: NSApplication.didHideNotification,
        object: NSApp,
        queue: .main
      ) { [weak self] _ in self?.visibilityDidChange() },
      NotificationCenter.default.addObserver(
        forName: NSApplication.didUnhideNotification,
        object: NSApp,
        queue: .main
      ) { [weak self] _ in self?.visibilityDidChange() },
    ]

    channel.setMethodCallHandler { [weak self] call, result in
      guard let self else {
        result(FlutterError(code: "tray_unavailable", message: "Trayが利用できません。", details: nil))
        return
      }
      switch call.method {
      case "initialize":
        result(self.windowVisible)
      case "isWindowVisible":
        result(self.windowVisible)
      case "publish":
        self.updateProjection(call.arguments)
        result(nil)
      case "notifyCritical":
        self.showCriticalNotification(result)
      default:
        result(FlutterMethodNotImplemented)
      }
    }
  }

  deinit {
    channel.setMethodCallHandler(nil)
    NSStatusBar.system.removeStatusItem(statusItem)
    applicationVisibilityObservers.forEach { NotificationCenter.default.removeObserver($0) }
  }

  private var windowVisible: Bool {
    guard let window else { return false }
    return window.isVisible && !window.isMiniaturized && !NSApp.isHidden
  }

  func hideWindowForTray() {
    guard let window else { return }
    window.orderOut(nil)
    updateVisibility(false)
    showUnknownProjection()
  }

  func visibilityDidChange() {
    updateVisibility(windowVisible)
    if !windowVisible {
      showUnknownProjection()
    }
  }

  private func updateVisibility(_ visible: Bool) {
    channel.invokeMethod("onWindowVisibilityChanged", arguments: visible)
  }

  private func showUnknownProjection() {
    runtimeItem.title = "実行系状態: 不明"
    approvalItem.title = "保留承認: 不明"
    notificationItem.title = "重大通知: 不明"
  }

  private func boundedString(_ value: Any?) -> String {
    guard let text = value as? String,
          !text.isEmpty,
          text.utf8.count <= 64,
          !text.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) })
    else {
      return "不明"
    }
    return text
  }

  private func boundedCount(_ value: Any?) -> String {
    if let number = value as? NSNumber,
       CFGetTypeID(number) != CFBooleanGetTypeID() {
      let count = number.intValue
      if number.doubleValue == Double(count), (0...256).contains(count) {
        return String(count)
      }
    }
    return boundedString(value)
  }

  private func boundedBoolean(_ value: Any?) -> Bool {
    guard let number = value as? NSNumber,
          CFGetTypeID(number) == CFBooleanGetTypeID()
    else {
      return false
    }
    return number.boolValue
  }

  private func updateProjection(_ value: Any?) {
    guard let projection = value as? [String: Any] else {
      showUnknownProjection()
      stopItem.isEnabled = false
      return
    }
    runtimeItem.title = "実行系状態: \(boundedString(projection["runtime_status"]))"
    approvalItem.title = "保留承認: \(boundedCount(projection["pending_approval_count"]))"
    notificationItem.title = "重大通知: \(boundedCount(projection["critical_notification_count"]))"
    stopItem.isEnabled = boundedBoolean(projection["stop_request_supported"])
  }

  private func showCriticalNotification(_ result: @escaping FlutterResult) {
    let center = UNUserNotificationCenter.current()
    center.getNotificationSettings { [weak self] settings in
      guard let self else {
        DispatchQueue.main.async { result(false) }
        return
      }
      switch settings.authorizationStatus {
      case .authorized, .provisional:
        self.scheduleCriticalNotification(center, result: result)
      case .notDetermined:
        center.requestAuthorization(options: [.alert, .sound]) { [weak self] granted, _ in
          guard granted, let self else {
            DispatchQueue.main.async { result(false) }
            return
          }
          self.scheduleCriticalNotification(center, result: result)
        }
      default:
        DispatchQueue.main.async { result(false) }
      }
    }
  }

  private func scheduleCriticalNotification(
    _ center: UNUserNotificationCenter,
    result: @escaping FlutterResult
  ) {
    let content = UNMutableNotificationContent()
    content.title = "D4 Pocket"
    content.body = "新しい重要通知があります。D4 Pocketを開いて通知センターで確認してください。"
    content.sound = .default
    content.threadIdentifier = "D4 Pocket"
    let request = UNNotificationRequest(
      identifier: Self.criticalNotificationIdentifier,
      content: content,
      trigger: nil
    )
    center.removeDeliveredNotifications(
      withIdentifiers: [Self.criticalNotificationIdentifier]
    )
    center.add(request) { error in
      DispatchQueue.main.async { result(error == nil) }
    }
  }

  func userNotificationCenter(
    _ center: UNUserNotificationCenter,
    willPresent notification: UNNotification,
    withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
  ) {
    if #available(macOS 11.0, *) {
      completionHandler([.banner, .sound])
    } else {
      completionHandler([.alert, .sound])
    }
  }

  func userNotificationCenter(
    _ center: UNUserNotificationCenter,
    didReceive response: UNNotificationResponse,
    withCompletionHandler completionHandler: @escaping () -> Void
  ) {
    guard response.notification.request.identifier == Self.criticalNotificationIdentifier,
          response.actionIdentifier == UNNotificationDefaultActionIdentifier
    else {
      completionHandler()
      return
    }
    DispatchQueue.main.async { [weak self] in
      self?.openNotificationCenter()
      completionHandler()
    }
  }

  private func openNotificationCenter() {
    guard let window else { return }
    NSApp.unhide(nil)
    NSApp.activate(ignoringOtherApps: true)
    window.makeKeyAndOrderFront(nil)
    updateVisibility(windowVisible)
    channel.invokeMethod("onTrayAction", arguments: "open_notifications")
  }

  @objc private func openWindow() {
    guard let window else { return }
    NSApp.unhide(nil)
    NSApp.activate(ignoringOtherApps: true)
    window.makeKeyAndOrderFront(nil)
    updateVisibility(windowVisible)
    channel.invokeMethod("onTrayAction", arguments: "open")
  }

  @objc private func requestStop() {
    guard stopItem.isEnabled else { return }
    channel.invokeMethod("onTrayAction", arguments: "stop_request")
  }

  @objc private func terminateApplication() {
    NSApp.terminate(nil)
  }

  private static let criticalNotificationIdentifier = "d4-pocket-critical-notification"
}
