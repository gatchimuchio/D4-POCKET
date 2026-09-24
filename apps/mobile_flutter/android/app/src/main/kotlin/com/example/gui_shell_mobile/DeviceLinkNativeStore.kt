package com.example.gui_shell_mobile

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

internal data class NativeDeviceState(
    val deviceId: String,
    val credential: Map<String, Any?>?,
)

internal class DeviceLinkNativeStore(context: Context) {
    private val preferences = context.applicationContext.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE)

    @Synchronized
    fun loadOrCreate(): NativeDeviceState {
        val encoded = preferences.getString(STATE_KEY, null)
        if (encoded == null) {
            val keyStore = KeyStore.getInstance(ANDROID_KEY_STORE).apply { load(null) }
            require(!keyStore.containsAlias(KEY_ALIAS))
            val created = NativeDeviceState(randomId(), null)
            persist(created)
            return created
        }
        val blob = decodeBlob(encoded)
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(TAG_BITS, blob.first))
        cipher.updateAAD(AAD)
        val clear = cipher.doFinal(blob.second).toString(Charsets.UTF_8)
        val state = StrictJson.parseObject(clear, MAX_STATE_BYTES)
        require(state.keys == setOf("version", "device_id", "credential"))
        require(state["version"] == 1L)
        val deviceId = state["device_id"] as? String ?: throw IllegalArgumentException("device id")
        require(Regex("^[a-f0-9]{32}$").matches(deviceId))
        val credential = when (val value = state["credential"]) {
            null -> null
            is Map<*, *> -> {
                require(value.keys.all { it is String })
                @Suppress("UNCHECKED_CAST")
                value as Map<String, Any?>
            }
            else -> throw IllegalArgumentException("credential state")
        }
        return NativeDeviceState(deviceId, credential)
    }

    @Synchronized
    fun storeCredential(credential: DeviceCredential): NativeDeviceState {
        val before = loadOrCreate()
        require(before.deviceId == credential.deviceId)
        val next = NativeDeviceState(before.deviceId, credential.asJson())
        persist(next)
        val verified = loadOrCreate()
        require(verified == next)
        return verified
    }

    @Synchronized
    fun deleteCredential(): NativeDeviceState {
        val before = loadOrCreate()
        val next = NativeDeviceState(before.deviceId, null)
        persist(next)
        val verified = loadOrCreate()
        require(verified == next)
        return verified
    }

    private fun persist(state: NativeDeviceState) {
        val clear = StrictJson.encode(
            linkedMapOf(
                "version" to 1L,
                "device_id" to state.deviceId,
                "credential" to state.credential,
            ),
        ).toByteArray(Charsets.UTF_8)
        require(clear.size <= MAX_STATE_BYTES)
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key())
        cipher.updateAAD(AAD)
        val ciphertext = cipher.doFinal(clear)
        val encoded = ENCODING + Base64.encodeToString(cipher.iv, Base64.NO_WRAP) + "." +
            Base64.encodeToString(ciphertext, Base64.NO_WRAP)
        require(preferences.edit().putString(STATE_KEY, encoded).commit())
    }

    private fun decodeBlob(encoded: String): Pair<ByteArray, ByteArray> {
        require(encoded.startsWith(ENCODING))
        val parts = encoded.removePrefix(ENCODING).split('.')
        require(parts.size == 2 && parts.all { Regex("^[A-Za-z0-9+/]+={0,2}$").matches(it) })
        val iv = Base64.decode(parts[0], Base64.NO_WRAP)
        val ciphertext = Base64.decode(parts[1], Base64.NO_WRAP)
        require(iv.size == GCM_IV_BYTES && ciphertext.size in 16..(MAX_STATE_BYTES + 16))
        return iv to ciphertext
    }

    private fun key(): SecretKey {
        val store = KeyStore.getInstance(ANDROID_KEY_STORE).apply { load(null) }
        val existing = store.getKey(KEY_ALIAS, null) as? SecretKey
        if (existing != null) return existing
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEY_STORE)
        generator.init(
            KeyGenParameterSpec.Builder(
                KEY_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setKeySize(256)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setRandomizedEncryptionRequired(true)
                .build(),
        )
        return generator.generateKey()
    }

    private fun randomId(): String {
        val bytes = ByteArray(16)
        SecureRandom().nextBytes(bytes)
        return bytes.toHex()
    }

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }

    companion object {
        private const val PREFERENCES = "gui_shell_device_link_native_v1"
        private const val STATE_KEY = "encrypted_state"
        private const val ANDROID_KEY_STORE = "AndroidKeyStore"
        private const val KEY_ALIAS = "gui-shell-device-link-state-v1"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val ENCODING = "aesgcm1."
        private const val MAX_STATE_BYTES = 16 * 1024
        private const val TAG_BITS = 128
        private const val GCM_IV_BYTES = 12
        private val AAD = "gui-shell/mobile-device-link/state/v1".toByteArray(Charsets.UTF_8)
    }
}
