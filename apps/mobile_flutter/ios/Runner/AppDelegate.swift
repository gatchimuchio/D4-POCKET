import Flutter
import UIKit

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate {
  private var deviceLinkService: DeviceLinkNativeService?

  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  func didInitializeImplicitFlutterEngine(_ engineBridge: FlutterImplicitEngineBridge) {
    GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)
    let service = DeviceLinkNativeService()
    deviceLinkService = service
    service.install(messenger: engineBridge.applicationRegistrar.messenger())
#if D4_IOS_PRODUCT_TEST
    DeviceLinkProductTestInput.install()
#endif
  }

  override func applicationWillTerminate(_ application: UIApplication) {
    deviceLinkService?.close()
    deviceLinkService = nil
  }
}
