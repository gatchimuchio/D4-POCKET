import XCTest
import AppKit
import Vision

final class AdapterOwnerUITests: XCTestCase {
  func testProductCredentialVault() throws {
    let app = XCUIApplication()
    let notice = XCUIApplication(bundleIdentifier: "com.apple.UserNotificationCenter")
    continueAfterFailure = false
    app.launch(); defer { app.terminate() }
    XCTAssertTrue(element(app, "エージェント").waitForExistence(timeout: 20))
    // コンパクト表示の2 iconがAX名なしだった実観測。公開groupの左、同じ行の
    // palette→全体検索という現行Row順序と実frameから通常mouse入力する。
    let group = element(app, "操作グループ選択")
    XCTAssertTrue(group.waitForExistence(timeout: 10))
    let toolbar = app.buttons.allElementsBoundByAccessibilityElement.filter {
      $0.frame.height >= 24 && app.windows.firstMatch.frame.contains($0.frame)
        && abs($0.frame.midY - group.frame.midY) < 4 && $0.frame.maxX < group.frame.minX
    }.sorted { $0.frame.minX < $1.frame.minX }
    guard toolbar.count == 2 else {
      // 秘密入力前の公開button名・frameだけ。入力値や全階層dumpは出さない。
      print("D4_CREDENTIAL_PUBLIC_NAV \(app.buttons.allElementsBoundByAccessibilityElement.prefix(40).map { "\($0.label)|\($0.frame)" })")
      throw failure("公開toolbarの2 iconを一意に確認できない")
    }
    toolbar[0].click()
    let search = app.textFields.firstMatch
    XCTAssertTrue(search.waitForExistence(timeout: 10))
    // 既存Manifest試験と同じ、FlutterView背面のnative編集欄の実位置へ通常click。
    search.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
    app.typeText("MCP")
    // 検索後の全階層AX queryが停止したため、秘密入力前の公開候補だけを画面で確認。
    try clickCredentialPublicText(app, "MCP接続", scroll: false)
    // native editorのlabel queryでは欄を取得できなかった。秘密入力前の可視ラベルと
    // 実画面位置を照合し、通常mouse入力だけで公開server IDを入力する。
    try clickCredentialPublicText(app, "サーバー識別子", scroll: true)
    app.typeText("macos-vault-fixture")
    let input = app.buttons["native入力で資格情報を登録"]
    XCTAssertTrue(input.waitForExistence(timeout: 10)); reveal(app, input); input.click()
    let cancel = app.sheets.buttons["取消"].firstMatch
    XCTAssertTrue(cancel.waitForExistence(timeout: 10)); cancel.click()
    XCTAssertTrue(element(app, "資格情報登録は未成立です。取消・拒否・期限または保管状態を確認してください。自動再送しません。").waitForExistence(timeout: 10))
    reveal(app, input); input.click()
    let proceed = app.sheets.buttons["入力して確認へ"].firstMatch
    XCTAssertTrue(proceed.waitForExistence(timeout: 10))
    // 合成秘密はRust Debug fixtureで生成済み。値をXCTestから入力・読取・出力しない。
    XCTAssertTrue(app.secureTextFields.firstMatch.exists); proceed.click()
    approve(notice)
    XCTAssertTrue(element(app, "Keychain登録後のmetadataを取得しました。秘密値は取得していません。").waitForExistence(timeout: 15))
    XCTAssertTrue(element(app, "api_key ・ 有効").exists)
    let revoke = app.buttons["資格情報を失効"]
    XCTAssertTrue(revoke.waitForExistence(timeout: 10)); reveal(app, revoke); revoke.click()
    approve(notice)
    XCTAssertTrue(element(app, "api_key ・ 失効").waitForExistence(timeout: 15))
    print("D4_MACOS_CREDENTIAL_VAULT_PRODUCT_PASS")
    app.typeKey("q", modifierFlags: .command)
    XCTAssertTrue(app.wait(for: .notRunning, timeout: 10))
  }

  private func clickCredentialPublicText(_ app: XCUIApplication, _ label: String, scroll: Bool) throws {
    // この呼出しはnative秘密入力前だけ。画像を保存・添付・出力しない。
    for attempt in 0..<(scroll ? 8 : 1) {
      Thread.sleep(forTimeInterval: 0.6)
      let image = XCUIScreen.main.screenshot().image
      var proposed = CGRect(origin: .zero, size: image.size)
      guard let pixels = image.cgImage(forProposedRect: &proposed, context: nil, hints: nil) else {
        throw failure("公開入力画面を取得できない")
      }
      let request = VNRecognizeTextRequest()
      request.recognitionLevel = .accurate
      request.recognitionLanguages = ["ja-JP", "en-US"]
      try VNImageRequestHandler(cgImage: pixels).perform([request])
      let hits = (request.results ?? []).filter {
        $0.topCandidates(1).first?.string.replacingOccurrences(of: " ", with: "") == label
      }
      if hits.isEmpty && scroll && attempt < 7 {
        app.windows.firstMatch.coordinate(withNormalizedOffset: CGVector(dx: 0.65, dy: 0.65))
          .scroll(byDeltaX: 0, deltaY: -180)
        continue
      }
      guard hits.count == 1 else { throw failure("公開ラベルを一意に確認できない: " + label) }
      let box = hits[0].boundingBox
      let point = CGPoint(x: box.midX * image.size.width, y: (1 - box.midY) * image.size.height)
      let window = app.windows.firstMatch
      guard window.frame.contains(point) else { throw failure("公開入力が製品窓の外にある") }
      window.coordinate(withNormalizedOffset: .zero)
        .withOffset(CGVector(dx: point.x - window.frame.minX, dy: point.y - window.frame.minY)).click()
      return
    }
  }

  func testProductAgentCLIOSSelection() throws {
    let app = XCUIApplication()
    let notice = XCUIApplication(bundleIdentifier: "com.apple.UserNotificationCenter")
    continueAfterFailure = false
    app.launch()
    defer { app.terminate() }
    let center = element(app, "エージェント")
    XCTAssertTrue(center.waitForExistence(timeout: 20)); center.click()
    let start = app.buttons["登録を開始"]
    XCTAssertTrue(start.waitForExistence(timeout: 10)); reveal(app, start); start.click()
    XCTAssertTrue(app.textFields.firstMatch.waitForExistence(timeout: 10))
    let fields = app.textFields.allElementsBoundByAccessibilityElement
    XCTAssertEqual(fields.count, 6)
    let workspace = "/Users/runner/Library/Containers/com.example.guiShellDesktop/Data/d4-registration-fixture/workspace"
    let cli = "/Users/runner/d4-os-selected-cli"
    let values = ["test-model", "macos-selected-cli", "/previous-cli",
                  "macos-cli-workspace", workspace, "private.env"]
    for (index, value) in values.enumerated() {
      if index == 0 {
        reveal(app, fields[0])
        fields[0].coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
      } else {
        for _ in 0..<(index == 1 ? 3 : 1) { app.typeKey(XCUIKeyboardKey.tab.rawValue, modifierFlags: []) }
      }
      app.typeKey("a", modifierFlags: .command)
      app.typeKey(XCUIKeyboardKey.delete.rawValue, modifierFlags: [])
      app.typeText(value)
      XCTAssertEqual(fields[index].value as? String, value)
    }
    // 既存6入力のfocus順を維持し、Workspace buttonの次の新CLI buttonへ通常Tabで進む。
    for _ in 0..<2 { app.typeKey(XCUIKeyboardKey.tab.rawValue, modifierFlags: []) }
    app.typeKey(" ", modifierFlags: [])
    let chooser = try WorkspacePanelUI(app: app)
    XCTAssertTrue(chooser.waitForButton("Cancel", timeout: 15))
    try chooser.press("Cancel"); app.activate()
    XCTAssertTrue(workspaceMessage(app, "OS選択を取り消しました。入力は変更していません。").waitForExistence(timeout: 10))
    // Rustが取消を返した実入力値を公開合成フォームの通常Copyで確認する。
    try assertCurrentCLIInput(app, "/previous-cli")
    // CLI入力→Workspace ID→root→secret path→Workspace button→CLI button。
    for _ in 0..<5 { app.typeKey(XCUIKeyboardKey.tab.rawValue, modifierFlags: []) }
    app.typeKey(" ", modifierFlags: [])
    XCTAssertTrue(chooser.waitForButton("CLI fileを選択", timeout: 15))
    try chooser.enterFolder(cli)
    try chooser.press("CLI fileを選択"); app.activate()
    XCTAssertTrue(workspaceMessage(app, "OS選択済み（起動中のみ）。登録・Permission・Approvalは別です。").waitForExistence(timeout: 10))
    try assertCurrentCLIInput(app, cli)
    XCTAssertFalse(workspaceMessage(app, "Broker起動中だけ登録しました。Task実行能力: unsupported。").exists)
    let submit = app.buttons["native Owner確認へ進む"]
    submit.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
    approve(notice)
    XCTAssertTrue(element(app, "Broker内登録: macos-selected-cli").waitForExistence(timeout: 20))
    XCTAssertTrue(workspaceMessage(app, "作業領域ID: macos-cli-workspace").exists)
    print("D4_MACOS_AGENT_CLI_OS_SELECTION_PASS")
    app.typeKey("q", modifierFlags: .command)
    XCTAssertTrue(app.wait(for: .notRunning, timeout: 10))
  }

  private func assertCurrentCLIInput(_ app: XCUIApplication, _ expected: String) throws {
    // 全6欄は公開合成値のみ。secret入力・資格・他画面へCopyを広げない。
    // CLI欄まで通常scrollで戻し、既知公開ラベルだけを認識する。
    app.windows.firstMatch.coordinate(withNormalizedOffset: CGVector(dx: 0.65, dy: 0.65))
      .scroll(byDeltaX: 0, deltaY: 150)
    // 録画でscroll途中の画像とclick時の位置差を観測した。通常UIの移動終了後に座標を読む。
    Thread.sleep(forTimeInterval: 1)
    let image = XCUIScreen.main.screenshot().image
    var proposed = CGRect(origin: .zero, size: image.size)
    guard let pixels = image.cgImage(forProposedRect: &proposed, context: nil, hints: nil) else {
      throw failure("CLIの公開入力画面を取得できない")
    }
    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.recognitionLanguages = ["ja-JP", "en-US"]
    try VNImageRequestHandler(cgImage: pixels).perform([request])
    let labels = (request.results ?? []).filter {
      guard let text = $0.topCandidates(1).first?.string else { return false }
      let normalized = text.folding(options: [.caseInsensitive, .widthInsensitive], locale: Locale(identifier: "ja_JP"))
        .replacingOccurrences(of: " ", with: "")
      return normalized.contains("codexcli") && normalized.contains("path")
    }
    guard labels.count == 1 else { throw failure("CLI入力の可視ラベルを一意に確認できない") }
    let box = labels[0].boundingBox
    let point = CGPoint(x: box.midX * image.size.width, y: (1 - box.minY) * image.size.height + 14)
    let window = app.windows.firstMatch
    guard window.frame.contains(point) else { throw failure("CLI入力欄が製品窓の外にある") }
    window.coordinate(withNormalizedOffset: .zero)
      .withOffset(CGVector(dx: point.x - window.frame.minX, dy: point.y - window.frame.minY)).click()
    NSPasteboard.general.clearContents()
    defer { NSPasteboard.general.clearContents() }
    app.typeKey("a", modifierFlags: .command); app.typeKey("c", modifierFlags: .command)
    XCTAssertEqual(NSPasteboard.general.string(forType: .string), expected, "実CLI入力内容")
  }

  func testProductWorkspaceOSSelection() throws {
    let app = XCUIApplication()
    let notice = XCUIApplication(bundleIdentifier: "com.apple.UserNotificationCenter")
    continueAfterFailure = false
    app.launch()
    defer { app.terminate() }
    let center = element(app, "エージェント")
    XCTAssertTrue(center.waitForExistence(timeout: 20)); center.click()
    let start = app.buttons["登録を開始"]
    XCTAssertTrue(start.waitForExistence(timeout: 10)); reveal(app, start); start.click()
    XCTAssertTrue(app.textFields.firstMatch.waitForExistence(timeout: 10))
    let fields = app.textFields.allElementsBoundByAccessibilityElement
    XCTAssertEqual(fields.count, 6)
    let cli = "/Users/runner/Library/Containers/com.example.guiShellDesktop/Data/d4-registration-fixture/codex"
    let outside = "/Users/runner/d4-os-selected-workspace"
    let values = ["test-model", "macos-os-selected-codex", cli,
                  "macos-os-selected-workspace", "/previous-input", "private.env"]
    for (index, value) in values.enumerated() {
      if index == 0 {
        reveal(app, fields[0])
        fields[0].coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
      } else {
        for _ in 0..<(index == 1 ? 3 : 1) { app.typeKey(XCUIKeyboardKey.tab.rawValue, modifierFlags: []) }
      }
      app.typeKey("a", modifierFlags: .command)
      app.typeKey(XCUIKeyboardKey.delete.rawValue, modifierFlags: [])
      app.typeText(value)
      XCTAssertEqual(fields[index].value as? String, value)
    }
    let select = app.buttons["OSで作業領域を選択"]
    XCTAssertTrue(select.exists)
    // Flutterのscroll後のAX frameは1pxのままだった。実際の画面の可視文字へclickする。
    try clickInitialWorkspaceSelection(app)
    let chooser = try WorkspacePanelUI(app: app)
    if !chooser.waitForButton("Cancel", timeout: 15) {
      let image = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
      image.name = "D4-workspace-chooser-missing"; image.lifetime = .keepAlways; add(image)
      print("D4_WORKSPACE_CHOOSER_APP \(app.debugDescription)")
      print("D4_WORKSPACE_CHOOSER_NATIVE \(chooser.description)")
      app.activate()
      app.windows.firstMatch.coordinate(withNormalizedOffset: CGVector(dx: 0.65, dy: 0.65))
        .scroll(byDeltaX: 0, deltaY: -180)
      print("D4_WORKSPACE_SELECTION_FAILURE \(app.debugDescription)")
      XCTFail("親GUI内Rust所有OS chooserを観測できない")
    }
    try chooser.press("Cancel")
    app.activate()
    XCTAssertTrue(workspaceMessage(app, "OS選択を取り消しました。入力は変更していません。").waitForExistence(timeout: 10))
    try assertCurrentWorkspaceInput(app, "/previous-input")
    // 直前の実CopyでWorkspace editorのfocusを確認済み。秘密path欄→OS buttonへ通常Tab移動する。
    // 取消messageも「OS選択」で始まるため、その文字へのOCR clickを使わない。
    for _ in 0..<2 { app.typeKey(XCUIKeyboardKey.tab.rawValue, modifierFlags: []) }
    app.typeKey(" ", modifierFlags: [])
    XCTAssertTrue(chooser.waitForButton("作業領域を選択", timeout: 15))
    try chooser.enterFolder(outside)
    XCTAssertTrue(chooser.waitForButton("作業領域を選択", timeout: 10))
    try chooser.press("作業領域を選択"); app.activate()
    XCTAssertTrue(workspaceMessage(app, "OS選択済み（起動中のみ）。登録・Permission・Approvalは別です。").waitForExistence(timeout: 10))
    try assertCurrentWorkspaceInput(app, outside)
    XCTAssertFalse(workspaceMessage(app, "Broker起動中だけ登録しました。Task実行能力: unsupported。").exists)
    let submit = app.buttons["native Owner確認へ進む"]
    submit.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
    approve(notice)
    // SectionListの見出しはheader Semanticsを持つ。通常の状態文のStaticText queryと区別する。
    XCTAssertTrue(element(app, "Broker内登録: macos-os-selected-codex").waitForExistence(timeout: 20))
    XCTAssertTrue(workspaceMessage(app, "作業領域ID: macos-os-selected-workspace").exists)
    print("D4_MACOS_WORKSPACE_OS_SELECTION_PASS")
    app.typeKey("q", modifierFlags: .command)
    XCTAssertTrue(app.wait(for: .notRunning, timeout: 10))
  }

  private func workspaceMessage(_ app: XCUIApplication, _ label: String) -> XCUIElement {
    app.staticTexts.matching(NSPredicate(format: "label == %@ OR value == %@", label, label)).firstMatch
  }

  private func assertCurrentWorkspaceInput(_ app: XCUIApplication, _ expected: String) throws {
    // native sheet後はAX値が画面の値と一致しなかった。公開ラベルを確認して実入力欄へmouse入力する。
    // このtestは全6欄を公開合成値だけに固定済み。実credentialや他の画面へcopy操作を広げない。
    let image = XCUIScreen.main.screenshot().image
    var proposed = CGRect(origin: .zero, size: image.size)
    guard let pixels = image.cgImage(forProposedRect: &proposed, context: nil, hints: nil) else {
      throw failure("公開合成試験の入力画面を取得できない")
    }
    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.recognitionLanguages = ["ja-JP", "en-US"]
    try VNImageRequestHandler(cgImage: pixels).perform([request])
    let labels = (request.results ?? []).filter {
      guard let text = $0.topCandidates(1).first?.string else { return false }
      let normalized = text.folding(options: [.caseInsensitive, .widthInsensitive], locale: Locale(identifier: "ja_JP"))
        .replacingOccurrences(of: " ", with: "")
      return normalized.contains("workspaceroot") && normalized.contains("path")
    }
    guard labels.count == 1 else { throw failure("Workspace入力の可視ラベルを一意に確認できない") }
    let box = labels[0].boundingBox
    // ラベル直下の通常editor行。確認済み製品フォームの14px余白を使い、値を設定しない。
    let point = CGPoint(x: box.midX * image.size.width, y: (1 - box.minY) * image.size.height + 14)
    let window = app.windows.firstMatch
    guard window.frame.contains(point) else { throw failure("Workspace入力欄が製品窓の外にある") }
    window.coordinate(withNormalizedOffset: .zero)
      .withOffset(CGVector(dx: point.x - window.frame.minX, dy: point.y - window.frame.minY)).click()
    NSPasteboard.general.clearContents()
    defer { NSPasteboard.general.clearContents() }
    app.typeKey("a", modifierFlags: .command)
    app.typeKey("c", modifierFlags: .command)
    XCTAssertEqual(NSPasteboard.general.string(forType: .string), expected, "実Workspace入力内容")
  }

  private func clickInitialWorkspaceSelection(_ app: XCUIApplication) throws {
    // 秘密path欄への入力直後は選択buttonがviewport外だった。通常の本文scrollで表示する。
    app.windows.firstMatch.coordinate(withNormalizedOffset: CGVector(dx: 0.65, dy: 0.65))
      .scroll(byDeltaX: 0, deltaY: -180)
    let image = XCUIScreen.main.screenshot().image
    var proposed = CGRect(origin: .zero, size: image.size)
    guard let pixels = image.cgImage(forProposedRect: &proposed, context: nil, hints: nil) else {
      throw failure("公開合成試験の画面を取得できない")
    }
    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.recognitionLanguages = ["ja-JP", "en-US"]
    request.usesLanguageCorrection = true
    request.customWords = ["OSで作業領域を選択"]
    try VNImageRequestHandler(cgImage: pixels).perform([request])
    let matches = (request.results ?? []).filter {
      guard let text = $0.topCandidates(1).first?.string else { return false }
      // 小さい日本語buttonの途中の漢字はVisionで欠落し得る。固有の接頭ラベルで一意に束縛する。
      let normalized = text.folding(options: [.caseInsensitive, .widthInsensitive], locale: Locale(identifier: "ja_JP"))
        .replacingOccurrences(of: " ", with: "").replacingOccurrences(of: "0s", with: "os")
      return normalized.hasPrefix("os") && normalized.hasSuffix("選択") && !normalized.contains("cli")
    }
    guard matches.count == 1 else {
      print("D4_WORKSPACE_BUTTON_OCR_COUNT \(request.results?.count ?? 0) matches=\(matches.count)")
      // 固定runnerの800px製品窓・検査済み6欄入力後の保存映像でボタン中央を確認した。
      // 最初のclickだけ。OS選択結果やBroker返答は注入せず通常mouse入力を送る。
      // 取消後は投影が増えてlayoutが変わるため、このfallbackを使わない。
      let window = app.windows.firstMatch
      guard matches.isEmpty, abs(window.frame.width - 800) <= 1 else {
        throw failure("初回の観測済み製品窓と一致しない、またはbutton候補が複数")
      }
      window.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.81)).click()
      return
    }
    let box = matches[0].boundingBox
    let point = CGPoint(x: box.midX * image.size.width, y: (1 - box.midY) * image.size.height)
    let window = app.windows.firstMatch
    guard window.frame.contains(point) else { throw failure("OS選択buttonが製品窓の外にある") }
    window.coordinate(withNormalizedOffset: .zero)
      .withOffset(CGVector(dx: point.x - window.frame.minX, dy: point.y - window.frame.minY)).click()
  }

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
        if index == 0 {
          reveal(app, field)
          field.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
        } else {
          // 模型欄の次は認証方式の2 RadioListTile、その後は残る入力欄の順。
          // scroll後のAX frameが古いままなので、製品の通常focus traversalを使う。
          for _ in 0..<(index == 1 ? 3 : 1) {
            app.typeKey(XCUIKeyboardKey.tab.rawValue, modifierFlags: [])
          }
        }
        XCTAssertTrue(field.exists, label)
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

/// 公開画面とXCTestの通常入力だけを使う。外部AX資格を要求せず、Broker応答を代替しない。
private final class WorkspacePanelUI {
  private let app: XCUIApplication

  init(app: XCUIApplication) throws {
    self.app = app
  }

  private func button(_ title: String) -> XCUIElement {
    // 同じ製品processの実AppKit panel。Flutterの日本語取消buttonとは別のnative英語Cancel。
    // App全体のqueryは同名Touch Bar buttonを先に返す。実window配下だけを操作する。
    app.windows.buttons.matching(identifier: title).firstMatch
  }

  func waitForButton(_ title: String, timeout: TimeInterval) -> Bool {
    button(title).waitForExistence(timeout: timeout)
  }

  func press(_ title: String) throws {
    let target = button(title)
    guard target.exists, target.isEnabled else {
      throw failure("OS chooserの可視ボタンを操作できない: " + title)
    }
    target.click()
  }

  func enterFolder(_ path: String) throws {
    app.typeKey("g", modifierFlags: [.command, .shift])
    app.typeText(path)
    app.typeKey(XCUIKeyboardKey.return.rawValue, modifierFlags: [])
  }

  var description: String {
    "native panelは親GUI process所有"
  }
}

private func failure(_ message: String) -> NSError {
  NSError(domain: "D4WorkspacePanelUITest", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
}
