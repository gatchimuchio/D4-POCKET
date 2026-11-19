package com.example.gui_shell_mobile

import android.content.Context
import android.content.SharedPreferences
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.security.GeneralSecurityException
import java.security.KeyStore
import java.util.UUID
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class NativeDeviceLinkStoreRecoveryTest {
    @Test
    fun resumesInitializationWhenOnlyTheKeyWasSaved() = withFixture { fixture ->
        val previous = fixture.store().loadOrCreate()
        assertTrue(fixture.preferences.edit().remove(STATE_KEY).commit())
        assertTrue(fixture.keyStore.containsAlias(fixture.alias))

        val fresh = fixture.store().loadOrCreate()

        assertNotEquals(previous.deviceId, fresh.deviceId)
        assertTrue(fresh.credential == null)
        assertTrue(fresh.localRecoveryAudit.isEmpty())
        assertTrue(fixture.store().loadOrCreate() == fresh)
    }

    @Test
    fun retainsEncryptedStateWithoutRegeneratingAMissingKey() = withFixture { fixture ->
        val paired = fixture.pair()
        assertTrue(fixture.store().loadOrCreate() == paired)
        val encrypted = fixture.encryptedState()
        fixture.keyStore.deleteEntry(fixture.alias)

        assertThrows(IllegalArgumentException::class.java) { fixture.store().loadOrCreate() }

        assertFalse(fixture.keyStore.containsAlias(fixture.alias))
        assertTrue(fixture.encryptedState() == encrypted)
    }

    @Test
    fun refusesCorruptedCiphertextAndKeepsItForRecovery() = withFixture { fixture ->
        fixture.pair()
        val encrypted = fixture.encryptedState()
        val firstByte = encrypted.lastIndexOf('.') + 1
        val replacement = if (encrypted[firstByte] == 'A') 'B' else 'A'
        val corrupted = encrypted.replaceRange(firstByte, firstByte + 1, replacement.toString())
        assertTrue(fixture.preferences.edit().putString(STATE_KEY, corrupted).commit())

        assertThrows(GeneralSecurityException::class.java) { fixture.store().loadOrCreate() }

        assertTrue(fixture.keyStore.containsAlias(fixture.alias))
        assertTrue(fixture.encryptedState() == corrupted)
    }

    private fun withFixture(test: (Fixture) -> Unit) {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val fixture = Fixture(context)
        try {
            test(fixture)
        } finally {
            try {
                fixture.keyStore.deleteEntry(fixture.alias)
            } finally {
                assertTrue(context.deleteSharedPreferences(fixture.name))
            }
        }
    }

    private class Fixture(private val context: Context) {
        val name = "device-link-store-recovery-${UUID.randomUUID()}"
        val alias = name
        val preferences: SharedPreferences = context.getSharedPreferences(name, Context.MODE_PRIVATE)
        val keyStore: KeyStore
            get() = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }

        fun store() = DeviceLinkNativeStore(context, name, alias)

        fun encryptedState(): String = requireNotNull(preferences.getString(STATE_KEY, null))

        fun pair(): NativeDeviceState {
            val store = store()
            val initial = store.loadOrCreate()
            return store.storeCredential(
                DeviceCredential(
                    hostId = "1".repeat(32),
                    host = "10.0.0.2",
                    port = 44321,
                    certificateHash = "b".repeat(64),
                    deviceId = initial.deviceId,
                    expiresAt = System.currentTimeMillis() / 1_000 + 3_600,
                    credentialId = "2".repeat(32),
                    secret = UUID.randomUUID().toString().replace("-", "").repeat(2),
                ),
            )
        }
    }

    private companion object {
        const val STATE_KEY = "encrypted_state"
    }
}
