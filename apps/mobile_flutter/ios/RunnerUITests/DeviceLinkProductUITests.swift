import XCTest

final class DeviceLinkProductUITests: XCTestCase {
  func testProductPairRuntimeResumeAndDisconnect() throws {
    guard let port = ProcessInfo.processInfo.environment["D4_IOS_PRODUCT_BRIDGE_PORT"], UInt16(port) != nil else {
      throw XCTSkip("専用Simulatorの製品接続harnessからだけ実行する")
    }
    let app = XCUIApplication(bundleIdentifier: "com.example.guiShellMobile")
    app.launchEnvironment["D4_IOS_PRODUCT_BRIDGE_PORT"] = port
    var stage = "initial_screen"
    defer { app.terminate() }
    do {
      app.launch()
      try check(app.staticTexts["D4 Pocket・概要"].waitForExistence(timeout: 20))
      try navigate(app, "接続先")
      stage = "open_native_dialog"
      try tap(app.buttons["native画面で招待を入力して端末結合"])
      try check(app.alerts["端末を結合"].waitForExistence(timeout: 10))
      stage = "native_invitation"
      // 招待実値をUI runnerへ取得せず、native入力完了の非秘密identifierだけ確認する。
      try check(app.secureTextFields["D4ProductInvitationReady"].waitForExistence(timeout: 25))
      try tap(app.alerts.buttons["招待を確認"])
      stage = "host_confirmation"
      try check(app.alerts["接続先を照合"].waitForExistence(timeout: 10))
      try tap(app.alerts.buttons["一致を確認して結合"])
      stage = "pair_complete"
      try check(app.staticTexts["Desktop端末資格を確認し、ThisDeviceOnly Keychainへ保存を再読しました。"].waitForExistence(timeout: 20))
      stage = "runtime_projection"
      try navigate(app, "実行系")
      try runtimeVisible(app)
      stage = "os_background_resume"
      XCUIDevice.shared.press(.home)
      try check(app.wait(for: .runningBackground, timeout: 10))
      app.activate()
      try check(app.wait(for: .runningForeground, timeout: 10))
      try runtimeVisible(app)
      stage = "product_disconnect"
      try navigate(app, "設定")
      try tap(app.buttons["Desktopの結合を解除して端末資格を削除"])
      try check(app.staticTexts["Desktop側の結合解除と端末内資格の削除を確認しました。送信済み処理の停止は保証しません。"].waitForExistence(timeout: 20))
      print("D4_IOS_PRODUCT_PASS")
    } catch {
      XCTFail("D4_IOS_PRODUCT_FAIL \(stage)")
    }
  }

  private func check(_ value: Bool) throws {
    if !value { throw ProductUIFailure.condition }
  }

  private func tap(_ element: XCUIElement) throws {
    try check(element.waitForExistence(timeout: 15))
    try check(element.isEnabled && element.isHittable)
    element.tap()
  }

  private func navigate(_ app: XCUIApplication, _ destination: String) throws {
    try tap(app.buttons["Open navigation menu"])
    let item = app.descendants(matching: .any).matching(NSPredicate(format: "label == %@ OR label BEGINSWITH %@", destination, destination + "\n")).firstMatch
    if item.exists && !item.isHittable { app.swipeUp() }
    try tap(item)
    try check(app.staticTexts["D4 Pocket・\(destination)"].waitForExistence(timeout: 10))
  }

  private func runtimeVisible(_ app: XCUIApplication) throws {
    for name in ["left", "right"] {
      let item = app.descendants(matching: .any).matching(NSPredicate(format: "label == %@ OR label BEGINSWITH %@", name, name + "\n")).firstMatch
      try check(item.waitForExistence(timeout: 20))
    }
  }
}

private enum ProductUIFailure: Error { case condition }
