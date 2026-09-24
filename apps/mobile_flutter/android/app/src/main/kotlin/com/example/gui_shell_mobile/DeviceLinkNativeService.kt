package com.example.gui_shell_mobile

import android.app.Activity
import android.content.DialogInterface
import android.graphics.Color
import android.text.InputType
import android.view.WindowManager
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import android.app.AlertDialog
import android.os.Handler
import android.os.Looper
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicLong
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.math.roundToInt

internal class DeviceLinkNativeService(
    private val activity: Activity,
) {
    private val store = DeviceLinkNativeStore(activity.applicationContext)
    private val transport = DeviceLinkTlsClient()
    private val worker: ExecutorService = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())
    private val generation = AtomicLong(0)
    private val pairingInProgress = AtomicBoolean(false)
    @Volatile private var foreground = true
    @Volatile private var disposed = false
    @Volatile private var dismissActivePairing: (() -> Unit)? = null

    fun handle(call: MethodCall, result: MethodChannel.Result): Boolean {
        if (disposed) {
            result.error("device_link_unavailable", GENERIC_FAILURE, null)
            return true
        }
        return when (call.method) {
            "read_state" -> {
                if (arguments(call, setOf("version")) == null) fail(result)
                else submit(result) { readState() }
                true
            }
            "pair" -> {
                if (arguments(call, setOf("version")) == null) fail(result)
                else beginPairing(result)
                true
            }
            "broker_request" -> {
                val args = arguments(call, setOf("version", "broker_operation", "payload"))
                val operation = args?.get("broker_operation") as? String
                val payload = toStringMap(args?.get("payload"))
                if (args == null || operation == null || payload == null) fail(result)
                else submit(result) { brokerRequest(operation, payload) }
                true
            }
            "disconnect" -> {
                if (arguments(call, setOf("version")) == null) fail(result)
                else submit(result) { disconnect() }
                true
            }
            "local_delete" -> {
                if (arguments(call, setOf("version")) == null) fail(result)
                else submit(result) { localDelete() }
                true
            }
            else -> false
        }
    }

    fun setForeground(value: Boolean) {
        if (foreground == value) return
        foreground = value
        generation.incrementAndGet()
        if (!value) {
            transport.cancelAll()
            main.post { dismissActivePairing?.invoke() }
        }
    }

    fun close() {
        disposed = true
        setForeground(false)
        transport.cancelAll()
        main.post { dismissActivePairing?.invoke() }
        worker.shutdownNow()
    }

    private fun arguments(call: MethodCall, keys: Set<String>): Map<String, Any?>? {
        val raw = call.arguments as? Map<*, *> ?: return null
        if (raw.keys.any { it !is String } || raw.keys.toSet() != keys) return null
        val version = raw["version"] ?: return null
        if ((version !is Int && version !is Long) || (version as Number).toLong() != 1L) return null
        if (raw.any { (key, value) -> key is String && key != "version" && value == null }) return null
        return raw.entries.associate { (key, value) -> (key as String) to value }
    }

    private fun toStringMap(raw: Any?): Map<String, Any?>? {
        val map = raw as? Map<*, *> ?: return null
        if (map.keys.any { it !is String }) return null
        return try {
            @Suppress("UNCHECKED_CAST")
            map as Map<String, Any?>
        } catch (_: Throwable) {
            null
        }
    }

    private fun submit(result: MethodChannel.Result, operation: () -> Any?) {
        try {
            worker.execute {
                val outcome = runCatching(operation)
                main.post {
                    if (disposed) {
                        result.error("device_link_unavailable", GENERIC_FAILURE, null)
                    } else {
                        outcome.fold(
                            onSuccess = { result.success(it) },
                            onFailure = { result.error("device_link_failed", GENERIC_FAILURE, null) },
                        )
                    }
                }
            }
        } catch (_: Throwable) {
            fail(result)
        }
    }

    private fun fail(result: MethodChannel.Result) {
        result.error("device_link_invalid_request", GENERIC_FAILURE, null)
    }

    private fun readState(): Map<String, Any?> {
        val state = store.loadOrCreate()
        if (pairingInProgress.get()) {
            return snapshot(
                paired = state.credential != null,
                connected = false,
                storageReady = true,
                status = "端末結合処理中のため、通常接続確認を停止しています。",
            )
        }
        val raw = state.credential ?: return snapshot(
            paired = false,
            connected = false,
            storageReady = true,
            status = "未接続。native画面からDesktopの招待を登録してください。",
        )
        val credential = DeviceCredential.stored(raw, state.deviceId)
        if (!foreground || disposed) return snapshot(true, false, true, "バックグラウンド中は接続を停止しています。資格はnative保管にあります。")
        if (credential.isExpired()) return snapshot(true, false, true, "端末資格の期限が切れています。Desktopで旧結合を失効し、新しい招待を発行してください。")
        val epoch = generation.get()
        val response = runCatching {
            transport.exchange(credential, "端末確認", emptyMap(), { canSend(epoch) })
        }.getOrNull()
        val connected = response?.get("status") == "accepted"
        return snapshot(
            paired = true,
            connected = connected && foreground && !disposed,
            storageReady = true,
            status = if (connected) "Desktop端末資格を確認しました。" else "Desktop接続を確認できません。資格は削除せず、通信を停止しています。",
        )
    }

    private fun localSnapshot(): Map<String, Any?> {
        val state = store.loadOrCreate()
        val paired = state.credential != null
        return snapshot(
            paired = paired,
            connected = false,
            storageReady = true,
            status = if (foreground) {
                if (paired) "接続確認が必要です。" else "未接続。native画面からDesktopの招待を登録してください。"
            } else {
                "バックグラウンド中は接続を停止しています。"
            },
        )
    }

    private fun brokerRequest(operation: String, payload: Map<String, Any?>): Map<String, Any?> {
        DeviceLinkPayloadPolicy.validate(operation, payload)
        check(!pairingInProgress.get())
        if (!foreground || disposed) throw IllegalStateException(GENERIC_FAILURE)
        val epoch = generation.get()
        val state = store.loadOrCreate()
        val raw = state.credential ?: throw IllegalStateException(GENERIC_FAILURE)
        val credential = DeviceCredential.stored(raw, state.deviceId)
        if (credential.isExpired()) throw IllegalStateException(GENERIC_FAILURE)
        return transport.exchange(credential, operation, payload, { canSend(epoch) })
    }

    private fun disconnect(): Map<String, Any?> {
        check(!pairingInProgress.get())
        if (!foreground || disposed) throw IllegalStateException(GENERIC_FAILURE)
        val epoch = generation.get()
        val state = store.loadOrCreate()
        val raw = state.credential ?: return localSnapshot()
        val credential = DeviceCredential.stored(raw, state.deviceId)
        require(!credential.isExpired())
        val response = transport.exchange(credential, "端末離脱", emptyMap(), { canSend(epoch) })
        require(response["status"] == "accepted" && response["body"] == mapOf("状態" to "失効"))
        val deleted = store.deleteCredential()
        require(deleted.credential == null)
        return snapshot(false, false, true, "Desktop側の結合解除と端末内資格の削除を確認しました。送信済み処理の停止は保証しません。")
    }

    private fun localDelete(): Map<String, Any?> {
        check(!pairingInProgress.get())
        require(foreground && !disposed)
        val deleted = store.deleteCredential()
        require(deleted.credential == null)
        return snapshot(false, false, true, "端末内資格を削除しました。Desktop側の失効は未確認です。ownerに失効を依頼してください。")
    }

    private fun beginPairing(result: MethodChannel.Result): Boolean {
        if (!pairingInProgress.compareAndSet(false, true)) {
            fail(result)
            return true
        }
        val returned = AtomicBoolean(false)
        try {
            worker.execute {
                val state = runCatching { store.loadOrCreate() }.getOrNull()
                main.post {
                    if (disposed || !foreground) {
                        pairingInProgress.set(false)
                        returnSnapshot(result, returned, localSnapshotSafely())
                    } else if (state == null) {
                        pairingInProgress.set(false)
                        returnSnapshot(result, returned, storageFailureSnapshot())
                    } else if (state.credential != null) {
                        pairingInProgress.set(false)
                        returnSnapshot(result, returned, localSnapshotSafely())
                    } else {
                        runCatching { showPairDialog(state, result, returned) }
                            .onFailure {
                                pairingInProgress.set(false)
                                val dismiss = dismissActivePairing
                                dismissActivePairing = null
                                runCatching { dismiss?.invoke() }
                                returnSnapshot(result, returned, localSnapshotSafely())
                            }
                    }
                }
            }
        } catch (_: Throwable) {
            pairingInProgress.set(false)
            returnSnapshot(result, returned, storageFailureSnapshot())
        }
        return true
    }

    private fun showPairDialog(
        state: NativeDeviceState,
        result: MethodChannel.Result,
        returned: AtomicBoolean,
    ) {
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(24), dp(12), dp(24), dp(4))
        }
        val deviceIdView = TextView(activity).apply {
            text = "Desktop ownerへ伝える端末ID:\n${state.deviceId}"
            setTextIsSelectable(true)
            setTextColor(Color.DKGRAY)
        }
        val instructions = TextView(activity).apply {
            text = "受け取った招待JSONを貼り付けてください。招待秘密はこのnative画面からFlutterへ渡りません。"
            setPadding(0, dp(10), 0, dp(6))
        }
        val input = EditText(activity).apply {
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE or InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD
            isSingleLine = false
            maxLines = 8
            importantForAutofill = android.view.View.IMPORTANT_FOR_AUTOFILL_NO
            contentDescription = "Desktopの端末招待JSON"
        }
        val feedback = TextView(activity).apply { setTextColor(Color.DKGRAY) }
        root.addView(deviceIdView)
        root.addView(instructions)
        root.addView(input)
        root.addView(feedback)
        var invitation: DeviceCredential? = null
        var detailsView: TextView? = null
        var pairingCompleted = false
        var pairRequestActive = false
        val dialog = AlertDialog.Builder(activity)
            .setTitle("端末を結合")
            .setView(root)
            .setNegativeButton("キャンセル", null)
            .setPositiveButton("招待を確認", null)
            .create()
        val dismissAction: () -> Unit = {
            input.text.clear()
            invitation = null
            dialog.dismiss()
        }
        dismissActivePairing = dismissAction
        dialog.setOnDismissListener {
            input.text.clear()
            invitation = null
            dialog.window?.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
            if (dismissActivePairing === dismissAction) dismissActivePairing = null
            if (!pairRequestActive) pairingInProgress.set(false)
            if (!pairingCompleted && !pairRequestActive) {
                returnSnapshot(result, returned, localSnapshotSafely())
            }
        }
        dialog.setOnShowListener {
            dialog.window?.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
            dialog.getButton(DialogInterface.BUTTON_POSITIVE).setOnClickListener { button ->
                if (invitation == null) {
                    val parsed = runCatching {
                        DeviceCredential.invitation(input.text.toString(), state.deviceId)
                    }.getOrNull()
                    input.text.clear()
                    if (parsed == null) {
                        feedback.text = "招待の形式・期限・端末IDを確認してください。"
                    } else {
                        invitation = parsed
                        detailsView = TextView(activity).apply {
                            text = "接続先: ${parsed.host}:${parsed.port}\nHostID: ${parsed.hostId}\n証明書hash: ${parsed.certificateHash}\n\nDesktop画面のHostと証明書hashが一致することを操作者が照合してください。"
                            setTextIsSelectable(true)
                            setPadding(dp(24), dp(12), dp(24), dp(12))
                        }
                        instructions.text = "招待を解析しました。画面の接続先をDesktop ownerと照合してください。"
                        root.removeView(input)
                        root.removeView(feedback)
                        root.addView(detailsView)
                        dialog.getButton(DialogInterface.BUTTON_POSITIVE).text = "一致を確認して結合"
                    }
                } else {
                    if (!foreground || disposed) {
                        invitation = null
                        input.text.clear()
                        dialog.dismiss()
                        returnSnapshot(result, returned, localSnapshotSafely())
                    } else {
                        button.isEnabled = false
                        feedback.text = "Desktopへ一度だけ安全接続しています。再送しないでください。"
                        val candidate = invitation
                        invitation = null
                        pairRequestActive = true
                        try {
                            worker.execute {
                                val finalState = if (candidate == null) pairingFailure()
                                else runCatching { pair(candidate, state) }.getOrElse { pairingFailure() }
                                main.post {
                                    input.text.clear()
                                    pairRequestActive = false
                                    pairingInProgress.set(false)
                                    pairingCompleted = true
                                    dialog.dismiss()
                                    returnSnapshot(result, returned, finalState)
                                }
                            }
                        } catch (_: Throwable) {
                            pairRequestActive = false
                            pairingInProgress.set(false)
                            input.text.clear()
                            dialog.dismiss()
                            returnSnapshot(result, returned, localSnapshotSafely())
                        }
                    }
                }
            }
        }
        dialog.window?.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        // native招待画面とFlutter間で入力値が共有されないよう、dismiss時にも入力bufferを消去する。
        dialog.show()
    }

    private fun pair(invitation: DeviceCredential, state: NativeDeviceState): Map<String, Any?> {
        val epoch = generation.get()
        if (!canSend(epoch)) return localSnapshot()
        val reply = transport.exchange(invitation, "端末結合", emptyMap(), { canSend(epoch) }, allowCredentialResponse = true)
        require(reply["status"] == "accepted")
        @Suppress("UNCHECKED_CAST")
        val body = reply["body"] as? Map<String, Any?> ?: throw IllegalArgumentException("pair response")
        val paired = DeviceCredential.paired(body, invitation)
        val persisted = runCatching { store.storeCredential(paired) }.getOrNull()
        if (persisted == null || persisted.credential == null) {
            val revoked = runCatching {
                val unlink = transport.exchange(paired, "端末離脱", emptyMap(), { canSend(epoch) })
                unlink["status"] == "accepted" && unlink["body"] == mapOf("状態" to "失効")
            }.getOrDefault(false)
            runCatching { store.deleteCredential() }
            return snapshot(
                paired = false,
                connected = false,
                storageReady = false,
                status = if (revoked) "安全保管に失敗しDesktop結合を解除しました。新しい招待でやり直してください。" else "安全保管に失敗しDesktop失効を確認できません。ownerが端末を失効させてください。",
            )
        }
        val verified = runCatching {
            transport.exchange(paired, "端末確認", emptyMap(), { canSend(epoch) })["status"] == "accepted"
        }.getOrDefault(false)
        return snapshot(
            paired = true,
            connected = verified && canSend(epoch),
            storageReady = true,
            status = if (verified && canSend(epoch)) "Desktop端末資格を確認し、OS安全保管への保存を再読しました。" else "結合資格はOS安全保管に保存しましたが、Desktop接続を確認できません。再確認してください。",
        )
    }

    private fun pairingFailure(): Map<String, Any?> {
        val stored = runCatching { store.loadOrCreate() }.getOrNull()
        return if (stored?.credential != null) {
            snapshot(true, false, true, "結合後の保存状態を確認できません。Desktopで端末一覧を確認してください。")
        } else {
            snapshot(false, false, stored != null, "結合を確認できません。招待を再利用せず、Desktopで端末状態を確認して新しい招待を発行してください。")
        }
    }

    private fun localSnapshotSafely(): Map<String, Any?> = runCatching { localSnapshot() }
        .getOrElse { storageFailureSnapshot() }

    private fun storageFailureSnapshot(): Map<String, Any?> = snapshot(
        paired = false,
        connected = false,
        storageReady = false,
        status = "Android Keystore保護の状態を確認できません。平文保存や別保管へfallbackせず停止しています。",
    )

    private fun snapshot(
        paired: Boolean,
        connected: Boolean,
        storageReady: Boolean,
        status: String,
    ): Map<String, Any?> = DeviceLinkSnapshot(
        storageReady = storageReady,
        paired = paired,
        connected = connected,
        foreground = foreground && !disposed,
        status = status,
    ).asChannelMap()

    private fun canSend(epoch: Long): Boolean = foreground && !disposed && generation.get() == epoch

    private fun dp(value: Int): Int = (value * activity.resources.displayMetrics.density).roundToInt()

    private fun returnSnapshot(
        result: MethodChannel.Result,
        returned: AtomicBoolean,
        value: Map<String, Any?>,
    ) {
        if (returned.compareAndSet(false, true)) result.success(value)
    }

    companion object {
        private const val GENERIC_FAILURE = "native端末連携を完了できません。入力や資格の詳細は返しません。"
    }
}
