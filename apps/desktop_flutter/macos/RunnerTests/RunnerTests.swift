import Cocoa
import FlutterMacOS
import XCTest
@testable import gui_shell_desktop

class RunnerTests: XCTestCase {
  func testApplicationDelegateIsInstalled() {
    XCTAssertTrue(NSApplication.shared.delegate is AppDelegate)
  }
}
