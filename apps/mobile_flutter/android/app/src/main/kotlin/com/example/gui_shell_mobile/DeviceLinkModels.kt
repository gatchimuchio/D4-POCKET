package com.example.gui_shell_mobile

internal data class DeviceCredential(
    val hostId: String,
    val host: String,
    val port: Int,
    val certificateHash: String,
    val deviceId: String,
    val expiresAt: Long,
    val credentialId: String,
    val secret: String,
) {
    fun isExpired(nowSeconds: Long = System.currentTimeMillis() / 1000): Boolean = expiresAt <= nowSeconds

    fun asJson(): Map<String, Any?> = linkedMapOf(
        "版" to 1L,
        "HostID" to hostId,
        "接続先Host" to host,
        "port" to port.toLong(),
        "証明書hash" to certificateHash,
        "端末ID" to deviceId,
        "有効期限" to expiresAt,
        "結合ID" to credentialId,
        "端末秘密" to secret,
    )

    companion object {
        private val hex32 = Regex("^[a-f0-9]{32}$")
        private val hex64 = Regex("^[a-f0-9]{64}$")
        private val runtimeId = Regex("^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$")

        fun invitation(raw: String, expectedDeviceId: String, now: Long = System.currentTimeMillis() / 1000): DeviceCredential {
            val data = StrictJson.parseObject(raw, 8192)
            val expected = setOf("版", "HostID", "接続先Host", "port", "証明書hash", "端末ID", "有効期限", "招待ID", "招待秘密")
            require(data.keys == expected)
            require(data["招待ID"] is String && data["招待秘密"] is String)
            return parse(
                data = data,
                deviceId = expectedDeviceId,
                invitation = true,
                now = now,
            )
        }

        fun paired(data: Map<String, Any?>, expected: DeviceCredential, now: Long = System.currentTimeMillis() / 1000): DeviceCredential {
            val keys = setOf("版", "HostID", "接続先Host", "port", "証明書hash", "端末ID", "有効期限", "結合ID", "端末秘密")
            require(data.keys == keys)
            val parsed = parse(data, expected.deviceId, invitation = false, now = now)
            require(parsed.hostId == expected.hostId)
            require(parsed.host == expected.host)
            require(parsed.port == expected.port)
            require(parsed.certificateHash == expected.certificateHash)
            require(parsed.secret != expected.secret)
            return parsed
        }

        fun stored(data: Map<String, Any?>, expectedDeviceId: String, now: Long = System.currentTimeMillis() / 1000): DeviceCredential {
            val keys = setOf("版", "HostID", "接続先Host", "port", "証明書hash", "端末ID", "有効期限", "結合ID", "端末秘密")
            require(data.keys == keys)
            return parse(data, expectedDeviceId, invitation = false, now = now)
        }

        private fun parse(
            data: Map<String, Any?>,
            deviceId: String,
            invitation: Boolean,
            now: Long,
        ): DeviceCredential {
            require(data["版"] == 1L)
            val hostId = data["HostID"] as? String ?: throw IllegalArgumentException("HostID")
            val host = data["接続先Host"] as? String ?: throw IllegalArgumentException("host")
            val port = (data["port"] as? Long)?.takeIf { it in 1L..65535L }?.toInt()
                ?: throw IllegalArgumentException("port")
            val hash = data["証明書hash"] as? String ?: throw IllegalArgumentException("hash")
            val receivedDeviceId = data["端末ID"] as? String ?: throw IllegalArgumentException("device")
            val expiry = data["有効期限"] as? Long ?: throw IllegalArgumentException("expiry")
            val idKey = if (invitation) "招待ID" else "結合ID"
            val secretKey = if (invitation) "招待秘密" else "端末秘密"
            val id = data[idKey] as? String ?: throw IllegalArgumentException("credential id")
            val secret = data[secretKey] as? String ?: throw IllegalArgumentException("secret")
            require(hostId.matches(hex32) && receivedDeviceId.matches(hex32) && receivedDeviceId == deviceId)
            require(hash.matches(hex64) && id.matches(hex32) && secret.matches(hex64))
            require(isPrivateIpv4(host))
            val maximum = now + if (invitation) 300L else 8L * 60L * 60L
            require(expiry > now && expiry <= maximum + 60L)
            return DeviceCredential(hostId, host, port, hash, receivedDeviceId, expiry, id, secret)
        }

        fun isPrivateIpv4(host: String): Boolean {
            val parts = host.split('.')
            if (parts.size != 4 || parts.any { it.isEmpty() || (it.length > 1 && it.startsWith('0')) }) return false
            val octets = parts.map { it.toIntOrNull()?.takeIf { n -> n in 0..255 } ?: return false }
            if (host != octets.joinToString(".")) return false
            val first = octets[0]
            val second = octets[1]
            return first == 127 || first == 10 || (first == 172 && second in 16..31) || (first == 192 && second == 168)
        }
    }
}

internal data class DeviceLinkSnapshot(
    val storageReady: Boolean,
    val paired: Boolean,
    val connected: Boolean,
    val foreground: Boolean,
    val status: String,
) {
    fun asChannelMap(): Map<String, Any?> = linkedMapOf(
        "storage_ready" to storageReady,
        "paired" to paired,
        "connected" to connected,
        "foreground" to foreground,
        "status" to status.take(160),
    )
}

internal object DeviceLinkPayloadPolicy {
    private val operations = setOf(
        "実行系列挙", "Agent一覧", "対話開始", "対話送信", "対話取得", "対話中止", "対話終了",
        "実行系ライフサイクル状態", "実行系資源観測", "通知一覧", "全Runtime停止要求",
        "対話履歴閲覧状態", "対話履歴閲覧",
    )
    private val forbidden = listOf("owner", "authority", "permission", "approval", "audit", "credential", "secret", "token", "private", "password")

    fun validate(operation: String, payload: Map<String, Any?>) {
        require(operation in operations)
        if (operation == "対話履歴閲覧") {
            validateHistory(payload)
        } else {
            validateTree(payload, 0)
        }
        require(StrictJson.encode(payload).toByteArray(Charsets.UTF_8).size <= 64 * 1024)
    }

    fun validateResponseTree(value: Any?, secrets: Set<String>, depth: Int = 0) {
        require(depth <= 32)
        when (value) {
            null, is Boolean, is Number -> Unit
            is String -> require(secrets.none { it.isNotEmpty() && value.contains(it) })
            is List<*> -> {
                require(value.size <= 4096)
                value.forEach { validateResponseTree(it, secrets, depth + 1) }
            }
            is Map<*, *> -> {
                require(value.size <= 4096)
                value.forEach { (key, item) ->
                    val name = key as? String ?: throw IllegalArgumentException("response key")
                    val normalized = name.lowercase().filter { it.isLetterOrDigit() }
                    val safeMarker = normalized == "secretpaths" || normalized == "secretvaluepresent"
                    require(safeMarker || listOf("credential", "token", "password", "privatekey").none { normalized.contains(it) })
                    require(name !in setOf("招待秘密", "端末秘密", "資格秘密", "端末資格"))
                    if (!safeMarker) require(!normalized.contains("secret"))
                    validateResponseTree(item, secrets, depth + 1)
                }
            }
            else -> throw IllegalArgumentException("response value")
        }
    }

    private fun validateHistory(payload: Map<String, Any?>) {
        require(payload.keys == setOf("approval_id", "query"))
        val approvalId = payload["approval_id"] as? String ?: throw IllegalArgumentException("grant reference")
        require(Regex("^[a-f0-9]{32}$").matches(approvalId))
        val query = payload["query"] as? Map<*, *> ?: throw IllegalArgumentException("history query")
        val queryKeys = setOf("after", "limit", "latest_per_request", "include_audit_context", "include_result_evidence", "include_content_receipt", "filter")
        require(query.keys.all { it is String && it in queryKeys } && query.keys.containsAll(setOf("after", "limit")))
        val after = integer(query["after"]) ?: throw IllegalArgumentException("history cursor")
        val limit = integer(query["limit"]) ?: throw IllegalArgumentException("history limit")
        require(after >= 0 && limit in 1L..100L)
        for (key in setOf("latest_per_request", "include_audit_context", "include_result_evidence", "include_content_receipt")) {
            if (key in query) require(query[key] is Boolean)
        }
        val filter = query["filter"] ?: return
        val fields = filter as? Map<*, *> ?: throw IllegalArgumentException("history filter")
        require(fields.keys.all { it in setOf("要求ID", "対話セッションID", "実行系ID", "状態") })
        fields.forEach { (key, value) ->
            require(
                when (key) {
                    "要求ID", "対話セッションID" -> value is String && Regex("^[a-f0-9]{32}$").matches(value)
                    "実行系ID" -> value is String && Regex("^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$").matches(value)
                    "状態" -> value in setOf("承認待ち", "実行中", "成功", "保留", "失敗", "中止")
                    else -> false
                },
            )
        }
    }

    private fun integer(value: Any?): Long? = when (value) {
        is Int -> value.toLong()
        is Long -> value
        else -> null
    }

    private fun validateTree(value: Any?, depth: Int) {
        require(depth <= 32)
        when (value) {
            null, is Boolean, is String, is Long, is Int -> Unit
            is Number -> require(value.toDouble().isFinite())
            is List<*> -> {
                require(value.size <= 4096)
                value.forEach { validateTree(it, depth + 1) }
            }
            is Map<*, *> -> {
                require(value.size <= 4096)
                value.forEach { (key, item) ->
                    val name = key as? String ?: throw IllegalArgumentException("payload key")
                    val normalized = name.lowercase().filter { it.isLetterOrDigit() }
                    require(forbidden.none { normalized.contains(it) })
                    require(name !in setOf("招待秘密", "端末秘密", "資格秘密", "端末資格"))
                    validateTree(item, depth + 1)
                }
            }
            else -> throw IllegalArgumentException("payload value")
        }
    }
}
