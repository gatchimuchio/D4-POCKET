package com.example.gui_shell_mobile

import java.io.ByteArrayOutputStream
import java.net.InetSocketAddress
import java.security.MessageDigest
import java.security.SecureRandom
import java.security.cert.CertificateException
import java.security.cert.X509Certificate
import java.util.Collections
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLSocket
import javax.net.ssl.TrustManager
import javax.net.ssl.X509TrustManager

internal class DeviceLinkTlsClient {
    private val activeSockets = Collections.synchronizedSet(mutableSetOf<SSLSocket>())

    fun cancelAll() {
        val sockets = synchronized(activeSockets) { activeSockets.toList() }
        sockets.forEach { runCatching { it.close() } }
    }

    fun exchange(
        credential: DeviceCredential,
        operation: String,
        payload: Map<String, Any?>,
        canSend: () -> Boolean,
        allowCredentialResponse: Boolean = false,
    ): Map<String, Any?> {
        require(!credential.isExpired())
        val nonce = randomId()
        val request = linkedMapOf<String, Any?>(
            "版" to 1L,
            "HostID" to credential.hostId,
            "端末ID" to credential.deviceId,
            "資格ID" to credential.credentialId,
            "資格秘密" to credential.secret,
            "nonce" to nonce,
            "発行時刻" to System.currentTimeMillis() / 1000,
            "操作" to operation,
            "内容" to payload,
        )
        val wire = (StrictJson.encode(request) + "\n").toByteArray(Charsets.UTF_8)
        require(wire.size <= REQUEST_LIMIT)
        val deadline = System.nanoTime() + TIMEOUT_NANOS
        val context = pinnedContext(credential.certificateHash)
        val socket = context.socketFactory.createSocket() as SSLSocket
        activeSockets.add(socket)
        try {
            socket.useClientMode = true
            socket.enabledProtocols = socket.supportedProtocols.filter { it == "TLSv1.3" || it == "TLSv1.2" }.toTypedArray()
            require(socket.enabledProtocols.isNotEmpty())
            socket.tcpNoDelay = true
            socket.connect(InetSocketAddress(credential.host, credential.port), remainingMillis(deadline))
            socket.soTimeout = remainingMillis(deadline)
            socket.startHandshake()
            val peer = socket.session.peerCertificates.firstOrNull() as? X509Certificate
                ?: throw CertificateException("peer certificate")
            peer.checkValidity()
            require(certificateHash(peer) == credential.certificateHash)
            require(canSend() && !credential.isExpired())
            socket.soTimeout = remainingMillis(deadline)
            socket.outputStream.write(wire)
            socket.outputStream.flush()
            val raw = readSingleResponse(socket, deadline)
            val response = StrictJson.parseObject(raw, RESPONSE_LIMIT)
            validateResponse(response, nonce, operation, credential, allowCredentialResponse)
            return sanitizeResponse(response)
        } finally {
            activeSockets.remove(socket)
            runCatching { socket.close() }
        }
    }

    private fun pinnedContext(expectedHash: String): SSLContext {
        val manager = object : X509TrustManager {
            override fun getAcceptedIssuers(): Array<X509Certificate> = emptyArray()
            override fun checkClientTrusted(chain: Array<out X509Certificate>?, authType: String?) {
                throw CertificateException("client authentication unsupported")
            }

            override fun checkServerTrusted(chain: Array<out X509Certificate>?, authType: String?) {
                val leaf = chain?.firstOrNull() ?: throw CertificateException("certificate missing")
                leaf.checkValidity()
                if (certificateHash(leaf) != expectedHash) throw CertificateException("certificate pin mismatch")
            }
        }
        return SSLContext.getInstance("TLS").apply {
            init(null, arrayOf<TrustManager>(manager), SecureRandom())
        }
    }

    private fun readSingleResponse(socket: SSLSocket, deadline: Long): String {
        val output = ByteArrayOutputStream()
        val buffer = ByteArray(8192)
        var newlineSeen = false
        while (true) {
            socket.soTimeout = remainingMillis(deadline)
            val count = socket.inputStream.read(buffer)
            if (count < 0) break
            if (newlineSeen) throw IllegalArgumentException("multiple response frames")
            var newline = -1
            for (index in 0 until count) {
                if (buffer[index] == NEWLINE) {
                    newline = index
                    break
                }
            }
            val bodyCount = if (newline >= 0) newline else count
            if (newline >= 0 && newline != count - 1) throw IllegalArgumentException("response trailing bytes")
            if (output.size() + bodyCount > RESPONSE_LIMIT) throw IllegalArgumentException("response size")
            output.write(buffer, 0, bodyCount)
            if (newline >= 0) newlineSeen = true
            if (System.nanoTime() >= deadline) throw IllegalArgumentException("response timeout")
        }
        require(newlineSeen && output.size() > 0)
        val decoder = Charsets.UTF_8.newDecoder()
            .onMalformedInput(java.nio.charset.CodingErrorAction.REPORT)
            .onUnmappableCharacter(java.nio.charset.CodingErrorAction.REPORT)
        return decoder.decode(java.nio.ByteBuffer.wrap(output.toByteArray())).toString()
    }

    private fun validateResponse(
        response: Map<String, Any?>,
        nonce: String,
        operation: String,
        credential: DeviceCredential,
        allowCredentialResponse: Boolean,
    ) {
        require(response.keys == RESPONSE_KEYS)
        require(response["request_id"] == nonce && response["operation"] == operation)
        val status = response["status"] as? String ?: throw IllegalArgumentException("status")
        require(status in setOf("accepted", "rejected", "suspended"))
        val auditId = response["audit_event_id"] as? String ?: throw IllegalArgumentException("audit id")
        require(auditId.isNotEmpty() && auditId.length <= 256 && auditId.all { it.isLetterOrDigit() || it in "_.:-" })
        require(response["evidence_source"] in setOf("LIVE_RUNTIME", "INTERNAL_STATE", "CONFIG", "EXTERNAL_EVIDENCE", "FIXTURE"))
        require(response["shutdown_requested"] == false)
        require(response["health"] == null)
        val body = response["body"]
        if (status == "accepted") require(body is Map<*, *> && response["error"] == null)
        else require(response["error"] is Map<*, *>)
        val error = response["error"]
        if (error != null) {
            val fields = error as? Map<*, *> ?: throw IllegalArgumentException("error")
            require(fields.keys == ERROR_KEYS)
            val code = fields["code"] as? String ?: throw IllegalArgumentException("error code")
            require(code.length in 1..96 && code.all { it.isLetterOrDigit() || it in "_.-" })
        }
        if (!allowCredentialResponse) {
            DeviceLinkPayloadPolicy.validateResponseTree(
                body,
                setOf(credential.secret),
            )
        }
        if (operation == "端末確認" && status == "accepted") {
            require(body == mapOf("状態" to "接続中"))
        }
        if (operation == "端末離脱" && status == "accepted") {
            require(body == mapOf("状態" to "失効"))
        }
    }

    private fun sanitizeResponse(response: Map<String, Any?>): Map<String, Any?> {
        val error = response["error"] as? Map<*, *>
        return linkedMapOf(
            "operation" to response["operation"],
            "status" to response["status"],
            "audit_event_id" to response["audit_event_id"],
            "evidence_source" to response["evidence_source"],
            "body" to response["body"],
            "error" to error?.let { mapOf("code" to it["code"]) },
        )
    }

    private fun certificateHash(certificate: X509Certificate): String =
        MessageDigest.getInstance("SHA-256").digest(certificate.encoded).toHex()

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }

    private fun randomId(): String = ByteArray(16).also(SecureRandom()::nextBytes).toHex()

    private fun remainingMillis(deadline: Long): Int {
        val remaining = deadline - System.nanoTime()
        if (remaining <= 0) throw java.net.SocketTimeoutException("request deadline")
        return ((remaining + 999_999) / 1_000_000).coerceAtMost(Int.MAX_VALUE.toLong()).toInt()
    }

    companion object {
        private const val REQUEST_LIMIT = 64 * 1024
        private const val RESPONSE_LIMIT = 4 * 1024 * 1024
        private const val TIMEOUT_NANOS = 5_000_000_000L
        private const val NEWLINE: Byte = 10
        private val RESPONSE_KEYS = setOf(
            "request_id", "operation", "status", "evidence_source", "audit_event_id", "error", "health", "body", "shutdown_requested",
        )
        private val ERROR_KEYS = setOf("code", "message", "recoverable", "audit_event_required", "fail_closed")
    }
}
