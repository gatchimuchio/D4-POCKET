package com.example.gui_shell_mobile

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceLinkPolicyTest {
    @Test
    fun acceptsOnlyBoundedHistoryGrantReferenceException() {
        val payload = mapOf(
            "approval_id" to "a".repeat(32),
            "query" to mapOf(
                "after" to 0,
                "limit" to 25,
                "filter" to mapOf("実行系ID" to "runtime_1"),
            ),
        )

        DeviceLinkPayloadPolicy.validate("対話履歴閲覧", payload)
        assertThrows(IllegalArgumentException::class.java) {
            DeviceLinkPayloadPolicy.validate("通知一覧", payload)
        }
        assertThrows(IllegalArgumentException::class.java) {
            DeviceLinkPayloadPolicy.validate(
                "対話履歴閲覧",
                payload + ("owner" to "true"),
            )
        }
        assertThrows(IllegalArgumentException::class.java) {
            DeviceLinkPayloadPolicy.validate(
                "対話履歴閲覧",
                payload + ("query" to mapOf("after" to -1, "limit" to 25)),
            )
        }
    }

    @Test
    fun rejectsAuthorityOrCredentialFieldsRecursivelyButAllowsTaskText() {
        DeviceLinkPayloadPolicy.validate(
            "対話送信",
            mapOf("対話セッションID" to "b".repeat(32), "入力" to "check approval status"),
        )
        for (key in listOf("permission", "client_credential", "PRIVATE-token", "招待秘密")) {
            assertThrows(IllegalArgumentException::class.java) {
                DeviceLinkPayloadPolicy.validate("対話送信", mapOf("nested" to mapOf(key to "x")))
            }
        }
    }

    @Test
    fun responseProjectionRejectsCredentialKeysAndValues() {
        DeviceLinkPayloadPolicy.validateResponseTree(
            mapOf("authentication" to mapOf("secret_value_present" to true, "secret_paths" to listOf(".env"))),
            setOf("f".repeat(64)),
        )
        assertThrows(IllegalArgumentException::class.java) {
            DeviceLinkPayloadPolicy.validateResponseTree(
                mapOf("nested" to mapOf("device_credential" to "redacted")),
                emptySet(),
            )
        }
        assertThrows(IllegalArgumentException::class.java) {
            DeviceLinkPayloadPolicy.validateResponseTree(
                mapOf("text" to "echo ${"f".repeat(64)}"),
                setOf("f".repeat(64)),
            )
        }
    }

    @Test
    fun invitationAndPairedCredentialAreBoundToPrivateHostAndExistingDevice() {
        val now = 1_800_000_000L
        val device = "a".repeat(32)
        val invite = invitation(device, host = "192.168.1.20", expires = now + 120)
        val parsed = DeviceCredential.invitation(StrictJson.encode(invite), device, now)
        assertEquals("192.168.1.20", parsed.host)
        assertTrue(!parsed.isExpired(now))

        val paired = mapOf(
            "版" to 1L,
            "HostID" to "b".repeat(32),
            "接続先Host" to "192.168.1.20",
            "port" to 43210L,
            "証明書hash" to "c".repeat(64),
            "端末ID" to device,
            "有効期限" to now + 7_200,
            "結合ID" to "d".repeat(32),
            "端末秘密" to "f".repeat(64),
        )
        assertEquals("f".repeat(64), DeviceCredential.paired(paired, parsed, now).secret)
        assertThrows(IllegalArgumentException::class.java) {
            DeviceCredential.paired(paired + ("接続先Host" to "192.168.1.21"), parsed, now)
        }
        assertThrows(IllegalArgumentException::class.java) {
            DeviceCredential.paired(paired + ("端末秘密" to "e".repeat(64)), parsed, now)
        }
        assertThrows(IllegalArgumentException::class.java) {
            DeviceCredential.invitation(StrictJson.encode(invitation(device, host = "203.0.113.7", expires = now + 120)), device, now)
        }
        assertThrows(IllegalArgumentException::class.java) {
            DeviceCredential.invitation(StrictJson.encode(invitation("f".repeat(32), host = "192.168.1.20", expires = now + 120)), device, now)
        }
    }

    private fun invitation(device: String, host: String, expires: Long): Map<String, Any?> = mapOf(
        "版" to 1L,
        "HostID" to "b".repeat(32),
        "接続先Host" to host,
        "port" to 43210L,
        "証明書hash" to "c".repeat(64),
        "端末ID" to device,
        "有効期限" to expires,
        "招待ID" to "d".repeat(32),
        "招待秘密" to "e".repeat(64),
    )
}
