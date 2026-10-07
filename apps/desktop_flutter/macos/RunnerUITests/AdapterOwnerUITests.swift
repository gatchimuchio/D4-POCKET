import XCTest

final class AdapterOwnerUITests: XCTestCase {
  func testProductAgentRegistration() throws {
    let app = XCUIApplication()
    let notice = XCUIApplication(bundleIdentifier: "com.apple.UserNotificationCenter")
    continueAfterFailure = false
    app.launch()
    defer { app.terminate() }
    let center = element(app, "エージェント")
    XCTAssertTrue(center.waitForExistence(timeout: 20))
    center.click()
    let fixture = "/Users/runner/Library/Containers/com.example.guiShellDesktop/Data/d4-registration-fixture"
    for approveRegistration in [false, true] {
      let start = app.buttons["登録を開始"]
      XCTAssertTrue(start.waitForExistence(timeout: 10))
      reveal(app, start)
      start.click()
      XCTAssertTrue(app.textFields.firstMatch.waitForExistence(timeout: 10))
      // 可視欄のindexはscrollで変わる。表示後のAX identityへ一度だけ束縛する。
      let fields = app.textFields.allElementsBoundByAccessibilityElement
      XCTAssertEqual(fields.count, 6)
      for (index, entry) in [
        ("模型識別子", "test-model"),
        ("実行系ID（Runtime ID）", "macos-product-codex"),
        ("Codex CLI実行fileの絶対path", fixture + "/codex"),
        ("作業領域ID（Workspace ID）", "macos-product-workspace"),
        ("Workspace rootの絶対path", fixture + "/workspace"),
        ("除外する秘密path（相対path、1行に1件）", "private.env")
      ].enumerated() {
        let (label, value) = entry
        let field = fields[index]
        reveal(app, field)
        XCTAssertTrue(field.exists, label)
        field.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
        // Flutter editorはAX TextFieldの子ではない。実際の窓へkeyを送り欄の値で照合する。
        app.typeKey("a", modifierFlags: .command)
        app.typeKey(XCUIKeyboardKey.delete.rawValue, modifierFlags: [])
        XCTAssertEqual(field.value as? String, "", label + "の既定値除去")
        app.typeText(value)
        XCTAssertEqual(field.value as? String, value, label + "の入力照合")
      }
      let submit = app.buttons["native Owner確認へ進む"]
      XCTAssertTrue(submit.exists)
      submit.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
      let decision = notice.dialogs.firstMatch.buttons[approveRegistration ? "今回の操作を承認" : "承認しない"]
      if !decision.waitForExistence(timeout: 15) {
        let screen = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        screen.name = "D4-macOS-registration-missing"; screen.lifetime = .keepAlways; add(screen)
        print("D4_MACOS_REGISTRATION_UI \(app.debugDescription)")
        XCTFail("登録Owner確認が表示されない")
      }
      decision.click()
      if !approveRegistration {
        XCTAssertTrue(element(app, "Codex実行系とWorkspaceの登録はRust Desktopのnative Owner確認だけで許可します").waitForExistence(timeout: 10))
        XCTAssertFalse(element(app, "Broker内登録: macos-product-codex").exists)
      }
    }
    let registered = element(app, "Broker内登録: macos-product-codex")
    if !registered.waitForExistence(timeout: 20) {
      print("D4_MACOS_REGISTRATION_RESULT \(app.debugDescription)")
      XCTFail("実Codex登録が成立しない")
    }
    XCTAssertTrue(element(app, "Broker起動中だけ登録しました。Task実行能力: unsupported。").exists)
    print("D4_MACOS_AGENT_REGISTRATION_PASS")
    app.typeKey("q", modifierFlags: .command)
    XCTAssertTrue(app.wait(for: .notRunning, timeout: 10))
  }

  private func reveal(_ app: XCUIApplication, _ target: XCUIElement) {
    XCTAssertTrue(target.waitForExistence(timeout: 10))
    for _ in 0..<10 {
      if target.frame.height >= 30 && app.windows.firstMatch.frame.contains(target.frame) { break }
      app.windows.firstMatch.coordinate(withNormalizedOffset: CGVector(dx: 0.65, dy: 0.65))
        .scroll(byDeltaX: 0, deltaY: -180)
    }
    XCTAssertGreaterThanOrEqual(target.frame.height, 30)
    XCTAssertTrue(app.windows.firstMatch.frame.contains(target.frame))
  }

  func testProductAdapterLifecycle() throws {
    let app = XCUIApplication()
    let notice = XCUIApplication(bundleIdentifier: "com.apple.UserNotificationCenter")
    continueAfterFailure = false
    app.launch()
    defer { app.terminate() }
    let runtime = element(app, "実行系")
    XCTAssertTrue(runtime.waitForExistence(timeout: 20))
    runtime.click()
    // 新しい状態管理試験の前提だけを用意する。導入Acceptanceの再試験ではない。
    try presentManifest(app)
    approve(notice)
    XCTAssertTrue(element(app, "Adapter管理 / アダプター台帳: macos_owner_fixture").waitForExistence(timeout: 15))
    for (operation, result) in [
      ("検証", "rejected / 署名対象がmanifestの正本byteと一致しない"),
      ("有効化", "rejected / 署名検証済みAdapterだけを有効化できる"),
      ("無効化", "管理: disabled / 有効: inactive"),
      ("隔離", "管理: quarantined / 有効: blocked")
    ] {
      manage(app, notice, operation)
      let projection = app.staticTexts.matching(NSPredicate(format: "value CONTAINS %@ OR label CONTAINS %@", result, result)).firstMatch
      XCTAssertTrue(projection.waitForExistence(timeout: 10), operation)
    }
    manage(app, notice, "削除")
    XCTAssertTrue(element(app, "Adapter catalogなし").waitForExistence(timeout: 10))
    print("D4_MACOS_ADAPTER_LIFECYCLE_PASS")
    app.typeKey("q", modifierFlags: .command)
    XCTAssertTrue(app.wait(for: .notRunning, timeout: 10))
  }

  private func approve(_ notice: XCUIApplication) {
    let button = notice.dialogs.firstMatch.buttons["今回の操作を承認"]
    XCTAssertTrue(button.waitForExistence(timeout: 15))
    button.click()
  }

  private func manage(_ app: XCUIApplication, _ notice: XCUIApplication, _ operation: String) {
    let button = app.buttons["アダプター" + operation]
    XCTAssertTrue(button.waitForExistence(timeout: 10))
    let available = XCTNSPredicateExpectation(predicate: NSPredicate(format: "enabled == true"), object: button)
    XCTAssertEqual(XCTWaiter.wait(for: [available], timeout: 10), .completed)
    for _ in 0..<8 {
      if button.frame.height >= 30 && button.isHittable { break }
      app.windows.firstMatch.coordinate(withNormalizedOffset: CGVector(dx: 0.7, dy: 0.7))
        .scroll(byDeltaX: 0, deltaY: -250)
    }
    XCTAssertGreaterThanOrEqual(button.frame.height, 30)
    XCTAssertTrue(button.isHittable)
    button.click()
    approve(notice)
  }

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
    // OSはTouch Barにも同名ボタンを出す。実際の確認dialog内だけを操作する。
    let deny = notice.dialogs.firstMatch.buttons["承認しない"]
    let ownerVisible = deny.waitForExistence(timeout: 15)
    if !ownerVisible {
      // 専用runner・合成公開Manifestだけの試験。UIの既存拒否理由を観測する。
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
    let approve = notice.dialogs.firstMatch.buttons["今回の操作を承認"]
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
    // typeText経路で「既」「署」が互換漢字へ変化した。JSONの意味を保ち、
    // 入力文字だけをASCIIのUnicode escapeへ固定する。製品の正本化は変更しない。
    let json = String(data: data, encoding: .utf8)!
    let asciiJSON = json.utf16.map { unit in
      unit < 0x80 ? String(UnicodeScalar(Int(unit))!) : String(format: "\\u%04x", Int(unit))
    }.joined()
    XCTAssertTrue(asciiJSON.utf8.allSatisfy { $0 < 0x80 })
    app.typeText(asciiJSON)
    let submit = app.buttons["native Owner確認へ"]
    XCTAssertTrue(app.windows.firstMatch.frame.contains(submit.frame))
    submit.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
  }
}
