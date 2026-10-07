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
    let ownerVisible = deny.waitForExistence(timeout: 15)
    if !ownerVisible {
      // 専用runner・合成公開Manifestだけの試験。UIの既存拒否理由を観測する。
      print(String(app.windows.firstMatch.label.prefix(2048)))
      let status = app.staticTexts.matching(NSPredicate(format: "value BEGINSWITH %@", "macos_owner_fixture:")).firstMatch
      if status.exists, let message = status.value as? String {
        print("D4_MACOS_OWNER_FAILURE \(message.prefix(1024))")
      } else {
        print("D4_MACOS_OWNER_FAILURE 結果表示なし")
      }
      let screen = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
      screen.name = "D4-macOS-owner-missing"
      screen.lifetime = .keepAlways
      add(screen)
    }
    XCTAssertTrue(ownerVisible)
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
      // Flutterは画面外のボタンも高さ1pxのAX要素とし、isHittableがtrueになり得る。
      if install.frame.height >= 30 && install.isHittable { break }
      // Windowの既定hit pointはtitle barだったため、観測済み本文領域へ送る。
      app.windows.firstMatch.coordinate(withNormalizedOffset: CGVector(dx: 0.7, dy: 0.7))
        .scroll(byDeltaX: 0, deltaY: -250)
    }
    XCTAssertGreaterThanOrEqual(install.frame.height, 30)
    XCTAssertTrue(install.isHittable)
    install.click()
    // Flutter 3.44 macOSは編集欄をNSTextFieldとして公開する。
    // 全要素のvalueを評価せず、このdialogで唯一の編集欄だけを取得する。
    let input = app.textFields.firstMatch
    XCTAssertTrue(input.waitForExistence(timeout: 10))
    XCTAssertTrue(app.windows.firstMatch.frame.contains(input.frame))
    let screen = XCTAttachment(screenshot: app.screenshot())
    screen.name = "D4-macOS-manifest-entry"
    screen.lifetime = .keepAlways
    add(screen)
    // native編集欄はFlutterViewの背面に置かれる。取得した入力欄中央へ通常clickを送る。
    input.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
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
    let submit = app.buttons["native Owner確認へ"]
    XCTAssertTrue(app.windows.firstMatch.frame.contains(submit.frame))
    submit.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
  }
}
