import Flutter
import UIKit
import XCTest
@testable import Runner

class RunnerTests: XCTestCase {
  func testApplicationDelegateUsesFlutter() {
    // native起動経路の接続だけを検査し、端末連携の証拠とはしない。
    XCTAssertNotNil(UIApplication.shared.delegate as? AppDelegate)
  }
}
