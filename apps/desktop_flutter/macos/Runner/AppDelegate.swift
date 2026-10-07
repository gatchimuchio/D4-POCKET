import Cocoa
import FlutterMacOS

@main
class AppDelegate: FlutterAppDelegate {
  override func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
    guard let channel = sender.windows.compactMap({ $0 as? MainFlutterWindow }).first?.brokerChannel else {
      return .terminateNow
    }
    DispatchQueue.main.async {
      channel.close { _ in sender.reply(toApplicationShouldTerminate: true) }
    }
    return .terminateLater
  }

  override func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
    return true
  }

  override func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool {
    return true
  }
}
