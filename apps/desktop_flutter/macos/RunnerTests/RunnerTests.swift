import Cocoa
import FlutterMacOS
import XCTest
@testable import gui_shell_desktop

class RunnerTests: XCTestCase {
  func testApplicationDelegateIsInstalled() {
    XCTAssertTrue(NSApplication.shared.delegate is AppDelegate)
  }

  func testProductBootstrapAndBrokerExit() {
    guard let window = NSApplication.shared.windows.compactMap({ $0 as? MainFlutterWindow }).first,
          let channel = window.brokerChannel else {
      XCTFail("macOS製品窓のBroker接続がない")
      return
    }
    let ready = expectation(description: "Flutter製品起動から実Brokerへ到達")
    let timer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { timer in
      if channel.productBootstrapObserved { timer.invalidate(); ready.fulfill() }
    }
    wait(for: [ready], timeout: 20)
    timer.invalidate()
    XCTAssertTrue(channel.productBootstrapObserved, "完了した固定操作: \(channel.completedBootstrapOperations)")
    let stopped = expectation(description: "Brokerの正常終了とAudit確定")
    channel.close { success in
      XCTAssertTrue(success)
      stopped.fulfill()
    }
    wait(for: [stopped], timeout: 10)
  }
}
