import XCTest

final class AdapterOwnerUITests: XCTestCase {
  func testProductAdapterOwnerDenialAndInstall() throws {
    let app = XCUIApplication()
    let notice = XCUIApplication(bundleIdentifier: "com.apple.UserNotificationCenter")
    continueAfterFailure = false
    app.launch()
    defer { app.terminate() }
    let runtime = element(app, "実行系")
    let ready = runtime.waitForExistence(timeout: 20)
    if !ready {
      // 新規runnerの入力前画面だけ。起動先や描画不能の診断を推測で補わない。
      print("D4_MACOS_INITIAL \(app.debugDescription)")
      let screen = XCTAttachment(screenshot: app.screenshot())
      screen.name = "D4-macOS-initial"
      screen.lifetime = .keepAlways
      add(screen)
    }
    XCTAssertTrue(ready)
    runtime.click()
    try presentManifest(app)
    let deny = notice.buttons["承認しない"]
    XCTAssertTrue(deny.waitForExistence(timeout: 15))
    deny.click()
    XCTAssertTrue(element(app, "Adapter catalogなし").waitForExistence(timeout: 10))
    try presentManifest(app)
    let approve = notice.buttons["今回の操作を承認"]
    XCTAssertTrue(approve.waitForExistence(timeout: 15))
    approve.click()
    XCTAssertTrue(element(app, "Adapter管理 / アダプター台帳: macos_owner_fixture").waitForExistence(timeout: 15))
    print("D4_MACOS_OWNER_PRODUCT_PASS")
    // 終了はOSの通常quit。helperを強制終了して成功扱いしない。
    app.typeKey("q", modifierFlags: .command)
    XCTAssertTrue(app.wait(for: .notRunning, timeout: 10))
  }

  private func element(_ app: XCUIApplication, _ label: String) -> XCUIElement {
    app.descendants(matching: .any).matching(NSPredicate(format: "label == %@ OR label BEGINSWITH %@ OR value == %@ OR value BEGINSWITH %@", label, label + "\n", label, label + "\n")).firstMatch
  }

  private func presentManifest(_ app: XCUIApplication) throws {
    let install = app.buttons["Adapter Manifestを導入"]
    XCTAssertTrue(install.waitForExistence(timeout: 10))
    let available = XCTNSPredicateExpectation(predicate: NSPredicate(format: "enabled == true"), object: install)
    XCTAssertEqual(XCTWaiter.wait(for: [available], timeout: 10), .completed)
    // 初期800x600窓ではRuntime情報の下にあるため、実際のスクロールで表示する。
    for _ in 0..<8 {
      if install.isHittable { break }
      app.windows.firstMatch.scroll(byDeltaX: 0, deltaY: -250)
    }
    XCTAssertTrue(install.isHittable)
    install.click()
    let input = element(app, "Adapter定義書（JSON）")
    XCTAssertTrue(input.waitForExistence(timeout: 10))
    input.click()
    // 合成公開Manifestのみ。秘密・実署名鍵・外部作用を含まない。
    let value: [String: Any] = [
      "版": 1, "Adapter ID": "macos_owner_fixture", "Runtime ID": "macos_owner_runtime",
      "発行者": "macOS製品試験", "source": "owner_manifest", "version": "1.0.0",
      "transport": "mock", "Content Exposure": "redacted", "要求Capability": ["runtime.read"],
      "許可差分": ["none"], "既知の危険": ["試験用metadata"], "互換性": "compatible",
      "authority_strip": true, "signed_manifest": true, "署名対象": "7b",
      "署名": String(repeating: "0", count: 128), "署名者fingerprint": "sha256:" + String(repeating: "0", count: 64)
    ]
    let data = try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    app.typeText(String(data: data, encoding: .utf8)!)
    app.buttons["native Owner確認へ"].click()
  }
}
