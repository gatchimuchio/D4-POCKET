package com.example.gui_shell_mobile

import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class NativeDeviceLinkLifecycleTest {
    @Test
    fun queuedRequestsAndUndeliveredRepliesCannotCrossNativeResume() {
        val worker = Executors.newSingleThreadExecutor()
        val releaseWorker = CountDownLatch(1)
        worker.execute { releaseWorker.await(10, TimeUnit.SECONDS) }
        val method = MethodCall("read_recovery_audit", mapOf("version" to 1))
        val queued = ChannelResult()
        val awaitingDelivery = ChannelResult()
        val current = ChannelResult()
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            lateinit var service: DeviceLinkNativeService
            try {
                scenario.onActivity { activity ->
                    service = DeviceLinkNativeService(activity, worker)
                    service.setForeground(true)
                    assertTrue(service.handle(method, queued))
                    service.setForeground(false)
                    service.setForeground(true)
                }
                releaseWorker.countDown()
                queued.await()
                assertEquals("device_link_failed", queued.errorCode)

                scenario.onActivity {
                    assertTrue(service.handle(method, awaitingDelivery))
                    worker.submit {}.get(3, TimeUnit.SECONDS)
                    service.setForeground(false)
                    service.setForeground(true)
                    assertTrue(service.handle(method, current))
                }
                awaitingDelivery.await()
                assertEquals("device_link_failed", awaitingDelivery.errorCode)
                current.await()
                assertNull(current.errorCode)
                assertTrue(current.value is List<*>)
            } finally {
                releaseWorker.countDown()
                scenario.onActivity { service.close() }
            }
        }
    }

    private class ChannelResult : MethodChannel.Result {
        private val completed = CountDownLatch(1)
        var value: Any? = null
            private set
        var errorCode: String? = null
            private set

        override fun success(result: Any?) {
            value = result
            completed.countDown()
        }

        override fun error(errorCode: String, errorMessage: String?, errorDetails: Any?) {
            this.errorCode = errorCode
            completed.countDown()
        }

        override fun notImplemented() = error("not_implemented", null, null)

        fun await() = assertTrue("native channel callback timed out", completed.await(10, TimeUnit.SECONDS))
    }
}
