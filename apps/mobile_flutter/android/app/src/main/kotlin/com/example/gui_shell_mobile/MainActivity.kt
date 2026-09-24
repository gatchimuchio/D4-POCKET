package com.example.gui_shell_mobile

import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    private var deviceLink: DeviceLinkNativeService? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        val service = deviceLink ?: DeviceLinkNativeService(this).also { deviceLink = it }
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                if (!service.handle(call, result)) result.notImplemented()
            }
    }

    override fun onResume() {
        super.onResume()
        deviceLink?.setForeground(true)
    }

    override fun onPause() {
        deviceLink?.setForeground(false)
        super.onPause()
    }

    override fun onStop() {
        deviceLink?.setForeground(false)
        super.onStop()
    }

    override fun onDestroy() {
        deviceLink?.close()
        deviceLink = null
        super.onDestroy()
    }

    companion object {
        private const val CHANNEL = "gui_shell/mobile_device_link"
    }
}
