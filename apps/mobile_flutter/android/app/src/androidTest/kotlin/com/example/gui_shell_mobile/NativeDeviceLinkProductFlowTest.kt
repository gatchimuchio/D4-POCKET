package com.example.gui_shell_mobile

import android.content.Intent
import android.graphics.Rect
import android.os.Bundle
import android.os.SystemClock
import android.view.accessibility.AccessibilityNodeInfo
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.ByteArrayOutputStream
import java.net.InetSocketAddress
import java.net.Socket
import java.security.KeyStore

@RunWith(AndroidJUnit4::class)
class NativeDeviceLinkProductFlowTest {
    private var stage = "開始"

    @Test
    fun installedProductUiUsesNativeDeviceLinkAndExistingRustBroker() {
        try {
            runProductFlow()
        } catch (failure: Throwable) {
            val kind = when (failure.message) {
                "expected UI state unavailable" -> "ui_state_missing"
                "UI action unavailable" -> "ui_action_missing"
                "navigation menu unavailable" -> "navigation_menu_missing"
                "native input accepts test text" -> "native_input_rejected"
                "not_connected" -> "broker_not_connected"
                "loading" -> "runtime_still_loading"
                "empty" -> "broker_runtime_list_empty"
                "status_error" -> "runtime_status_error"
                "runtime_absent" -> "runtime_tile_missing"
                else -> failure.javaClass.simpleName
            }
            throw AssertionError("Android native Device Link E2E failed at ${stage}_$kind")
        }
    }

    private fun runProductFlow() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val target = instrumentation.targetContext
        val bridgePort = InstrumentationRegistry.getArguments().getString("bridge_port")?.toIntOrNull()
        assertTrue("bridge port", bridgePort != null && bridgePort in 1..65535)

        stage = "製品画面起動"
        val launch = target.packageManager.getLaunchIntentForPackage(target.packageName)
        assertNotNull("launch intent", launch)
        instrumentation.startActivitySync(launch!!.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        instrumentation.waitForIdleSync()
        waitForText("D4 Pocket", 45_000)

        stage = "navigation_drawer"
        openDrawer()
        stage = "connection_destination"
        clickText("接続先", exact = false)
        stage = "native_pairing_entry"
        clickText("native画面で招待を入力して端末結合", exact = false)
        stage = "native_device_id"
        val deviceIdText = waitForNodeText("Desktop ownerへ伝える端末ID", 20_000)
        val deviceId = Regex("[a-f0-9]{32}").find(deviceIdText)?.value
        assertTrue("native device id shape", deviceId != null)

        stage = "一時招待を取得"
        val invitationText = receiveInvitation(bridgePort!!, deviceId!!)
        val invitation = JSONObject(invitationText)
        val invitationSecret = invitation.getString("招待秘密")
        val hostId = invitation.getString("HostID")
        val wrongPin = "0".repeat(64)
        assertFalse("test pin must differ", invitation.getString("証明書hash") == wrongPin)

        stage = "誤った証明書pinを拒否"
        val alteredInvitation = JSONObject(invitationText).put("証明書hash", wrongPin).toString()
        enterInvitationAndConfirm(
            alteredInvitation,
            alreadyOpen = true,
            stagePrefix = "wrong_pin",
            expectedHostId = hostId,
            expectedCertificateHash = wrongPin,
        )
        stage = "wrong_pin_rejected"
        waitForText("招待を再利用せず、Desktopで端末状態を確認して新しい招待を発行してください。", 20_000)
        stage = "wrong_pin_unpaired"
        assertTrue("rejected attempt remains unpaired", containsText("native画面で招待を入力して端末結合"))

        stage = "valid_pairing_open"
        clickText("native画面で招待を入力して端末結合", exact = false)
        stage = "valid_pairing_dialog"
        waitForNodeDescription("Desktopの端末招待JSON", 15_000)
        enterInvitationAndConfirm(invitationText, alreadyOpen = true, stagePrefix = "valid_pairing", expectedHostId = hostId,
            expectedCertificateHash = invitation.getString("証明書hash"))
        stage = "paired_ui_state"
        waitForText("Desktop資格を再確認", 30_000)

        stage = "Keystore保護とChannel投影"
        val preferences = target.getSharedPreferences("gui_shell_device_link_native_v1", 0)
        val encryptedState = preferences.getString("encrypted_state", null)
        assertTrue("encrypted state encoding", encryptedState?.startsWith("aesgcm1.") == true)
        assertFalse("invitation secret not stored in clear", encryptedState!!.contains(invitationSecret))
        val keystore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        assertTrue("Android Keystore key", keystore.containsAlias("gui-shell-device-link-state-v1"))
        assertFalse("invitation secret not displayed after pairing", visibleTextContains(invitationSecret))

        stage = "runtime_drawer"
        openDrawer()
        stage = "runtime_destination"
        clickText("実行系", exact = false)
        stage = "broker_runtime_left"
        waitForRuntime("left", 30_000)
        stage = "broker_runtime_right"
        waitForRuntime("right", 30_000)

        stage = "background_home"
        instrumentation.uiAutomation.executeShellCommand("input keyevent KEYCODE_HOME").close()
        SystemClock.sleep(800)
        stage = "foreground_restart"
        val relaunch = instrumentation.uiAutomation.executeShellCommand(
            "am start -W -n ${target.packageName}/.MainActivity",
        )
        val relaunchOutput = android.os.ParcelFileDescriptor.AutoCloseInputStream(relaunch)
            .bufferedReader().use { it.readText() }
        assertTrue("launcher activity start", relaunchOutput.contains("Status: ok"))
        instrumentation.waitForIdleSync()
        stage = "foreground_runtime_drawer"
        openDrawer()
        stage = "foreground_runtime_destination"
        clickText("実行系", exact = false)
        stage = "foreground_runtime_left"
        waitForRuntime("left", 30_000)
        stage = "foreground_runtime_right"
        waitForRuntime("right", 30_000)

        stage = "disconnect_drawer"
        openDrawer()
        stage = "disconnect_settings_destination"
        clickText("設定", exact = false)
        stage = "disconnect_settings_page"
        waitForText("設定と端末資格", 15_000)
        stage = "disconnect_control"
        scrollAndClickText("Desktopの結合を解除して端末資格を削除")
        stage = "disconnect_result"
        waitForText("Desktop側の結合解除と端末内資格の削除を確認しました。送信済み処理の停止は保証しません。", 30_000)
        stage = "post_disconnect_drawer"
        openDrawer()
        stage = "post_disconnect_connection_destination"
        clickText("接続先", exact = false)
        stage = "post_disconnect_unpaired_state"
        waitForText("native画面で招待を入力して端末結合", 15_000)

        stage = "完了"
    }

    private fun enterInvitationAndConfirm(
        invitation: String,
        alreadyOpen: Boolean = false,
        stagePrefix: String,
        expectedHostId: String? = null,
        expectedCertificateHash: String? = null,
    ) {
        if (!alreadyOpen) {
            stage = "${stagePrefix}_open"
            clickText("native画面で招待を入力して端末結合", exact = false)
        }
        stage = "${stagePrefix}_input"
        val input = waitForNodeDescription("Desktopの端末招待JSON", 15_000)
        val values = Bundle().apply {
            putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, invitation)
        }
        assertTrue("native input accepts test text", input.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, values))
        stage = "${stagePrefix}_invite_review"
        clickText("招待を確認", exact = true)
        if (expectedHostId != null) {
            stage = "${stagePrefix}_host_display"
            waitForText("HostID: $expectedHostId", 10_000)
        }
        if (expectedCertificateHash != null) {
            stage = "${stagePrefix}_certificate_display"
            waitForText("証明書hash: $expectedCertificateHash", 10_000)
        }
        stage = "${stagePrefix}_owner_confirmation"
        clickText("一致を確認して結合", exact = true)
    }

    private fun receiveInvitation(port: Int, deviceId: String): String {
        Socket().use { socket ->
            socket.connect(InetSocketAddress("127.0.0.1", port), 5_000)
            socket.soTimeout = 20_000
            socket.getOutputStream().write("$deviceId\n".toByteArray(Charsets.US_ASCII))
            socket.getOutputStream().flush()
            val output = ByteArrayOutputStream()
            while (output.size() <= 8_192) {
                val byte = socket.getInputStream().read()
                if (byte < 0) throw AssertionError("bridge closed")
                if (byte == '\n'.code) return output.toString(Charsets.UTF_8.name())
                output.write(byte)
            }
        }
        throw AssertionError("bridge response exceeds limit")
    }

    private fun openDrawer() {
        val deadline = SystemClock.uptimeMillis() + 10_000
        while (SystemClock.uptimeMillis() < deadline) {
            val node = findNode { candidate ->
                val description = candidate.contentDescription?.toString()?.lowercase().orEmpty()
                description.contains("menu") || description.contains("メニュー") || description.contains("navigation")
            } ?: findNode { candidate ->
                if (!candidate.isClickable) return@findNode false
                val bounds = Rect().also(candidate::getBoundsInScreen)
                bounds.top < 300 && bounds.left < 240 && bounds.width() in 1..180
            }
            if (node != null && clickNode(node)) {
                waitForNodeText("D4 Pocket（GUI Shell基盤）", 5_000)
                return
            }
            SystemClock.sleep(200)
        }
        throw AssertionError("navigation menu unavailable")
    }

    private fun clickText(text: String, exact: Boolean) {
        val deadline = SystemClock.uptimeMillis() + 20_000
        while (SystemClock.uptimeMillis() < deadline) {
            val node = findNode { candidate ->
                listOfNotNull(candidate.text?.toString(), candidate.contentDescription?.toString())
                    .filter(String::isNotBlank)
                    .any { label -> if (exact) label == text else label.contains(text) }
            }
            if (node != null && clickNode(node)) return
            SystemClock.sleep(200)
        }
        throw AssertionError("UI action unavailable")
    }

    private fun scrollAndClickText(text: String) {
        val metrics = InstrumentationRegistry.getInstrumentation().targetContext.resources.displayMetrics
        val x = metrics.widthPixels / 2
        val fromY = (metrics.heightPixels * 0.82).toInt()
        val toY = (metrics.heightPixels * 0.34).toInt()
        repeat(6) {
            val node = findNode { candidate ->
                listOfNotNull(candidate.text?.toString(), candidate.contentDescription?.toString())
                    .any { it.contains(text) }
            }
            if (node != null && clickNode(node)) return
            InstrumentationRegistry.getInstrumentation().uiAutomation
                .executeShellCommand("input swipe $x $fromY $x $toY 350").close()
            SystemClock.sleep(300)
        }
        throw AssertionError("UI action unavailable")
    }

    private fun clickNode(node: AccessibilityNodeInfo): Boolean {
        var current: AccessibilityNodeInfo? = node
        repeat(8) {
            val candidate = current ?: return false
            if (candidate.isClickable && candidate.performAction(AccessibilityNodeInfo.ACTION_CLICK)) return true
            current = candidate.parent
        }
        return false
    }

    private fun waitForNodeDescription(description: String, timeoutMillis: Long): AccessibilityNodeInfo =
        waitForNode({ it.contentDescription?.toString() == description }, timeoutMillis)

    private fun waitForNodeText(text: String, timeoutMillis: Long): String {
        val node = waitForNode({ candidate ->
            listOfNotNull(candidate.text?.toString(), candidate.contentDescription?.toString())
                .any { it.contains(text) }
        }, timeoutMillis)
        return listOfNotNull(node.text?.toString(), node.contentDescription?.toString())
            .firstOrNull { it.contains(text) }.orEmpty()
    }

    private fun waitForText(text: String, timeoutMillis: Long) {
        waitForNode({
            it.text?.toString()?.contains(text) == true || it.contentDescription?.toString()?.contains(text) == true
        }, timeoutMillis)
    }

    private fun waitForRuntime(runtime: String, timeoutMillis: Long) {
        try {
            waitForText(runtime, timeoutMillis)
        } catch (_: AssertionError) {
            val state = when {
                containsText("未接続。現在の実行系状態は未確認です。") ||
                    containsText("Desktop接続を確認できません") -> "not_connected"
                containsText("状態を観測中") -> "loading"
                containsText("登録実行系なし") -> "empty"
                containsText("Broker応答を確認できません") -> "status_error"
                else -> "runtime_absent"
            }
            throw AssertionError(state)
        }
    }

    private fun containsText(text: String): Boolean = findNode {
        it.text?.toString()?.contains(text) == true || it.contentDescription?.toString()?.contains(text) == true
    } != null

    private fun visibleTextContains(text: String): Boolean = findNode {
        it.text?.toString()?.contains(text) == true || it.contentDescription?.toString()?.contains(text) == true
    } != null

    private fun waitForNode(predicate: (AccessibilityNodeInfo) -> Boolean, timeoutMillis: Long): AccessibilityNodeInfo {
        val deadline = SystemClock.uptimeMillis() + timeoutMillis
        while (SystemClock.uptimeMillis() < deadline) {
            findNode(predicate)?.let { return it }
            SystemClock.sleep(200)
        }
        throw AssertionError("expected UI state unavailable")
    }

    private fun findNode(predicate: (AccessibilityNodeInfo) -> Boolean): AccessibilityNodeInfo? {
        val root = InstrumentationRegistry.getInstrumentation().uiAutomation.rootInActiveWindow ?: return null
        val pending = ArrayDeque<AccessibilityNodeInfo>()
        pending.add(root)
        while (pending.isNotEmpty()) {
            val node = pending.removeFirst()
            if (predicate(node)) return node
            for (index in 0 until node.childCount) node.getChild(index)?.let(pending::addLast)
        }
        return null
    }
}
