package com.example.gui_shell_mobile

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class StrictJsonTest {
    @Test
    fun parsesAndEncodesStrictNestedJson() {
        val parsed = StrictJson.parseObject(
            """{"text":"\ud83d\ude00","array":[true,null,-3,2.5e1]}""",
            256,
        )

        assertEquals("😀", parsed["text"])
        assertEquals("""{"text":"😀","array":[true,null,-3,25.0]}""", StrictJson.encode(parsed))
    }

    @Test
    fun rejectsDuplicateKeysMalformedNumbersAndTrailingInput() {
        for (raw in listOf(
            """{"key":1,"key":2}""",
            """{"key":01}""",
            """{"key":NaN}""",
            """{"key":1} false""",
            """{"key":"\ud800"}""",
        )) {
            assertThrows(IllegalArgumentException::class.java) {
                StrictJson.parseObject(raw, 256)
            }
        }
    }

    @Test
    fun enforcesInputAndNestingLimits() {
        assertThrows(IllegalArgumentException::class.java) {
            StrictJson.parseObject("""{"value":"too long"}""", 8)
        }
        assertThrows(IllegalArgumentException::class.java) {
            StrictJson.parseObject("{" + "[".repeat(34) + "0" + "]".repeat(34) + "}", 1024)
        }
    }
}
