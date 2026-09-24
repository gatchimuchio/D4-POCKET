package com.example.gui_shell_mobile

import java.nio.charset.StandardCharsets

internal object StrictJson {
    fun parseObject(source: String, byteLimit: Int): Map<String, Any?> {
        require(source.toByteArray(StandardCharsets.UTF_8).size <= byteLimit)
        val parser = Parser(source)
        val value = parser.readValue(0)
        parser.skipSpace()
        require(parser.atEnd())
        @Suppress("UNCHECKED_CAST")
        return value as? Map<String, Any?> ?: throw IllegalArgumentException("object required")
    }

    fun encode(value: Any?): String = buildString { appendValue(value, 0) }

    private fun StringBuilder.appendValue(value: Any?, depth: Int) {
        require(depth <= 32)
        when (value) {
            null -> append("null")
            is Boolean -> append(if (value) "true" else "false")
            is String -> appendString(value)
            is Byte, is Short, is Int, is Long -> append(value.toString())
            is Float -> {
                require(value.isFinite())
                append(value.toString())
            }
            is Double -> {
                require(value.isFinite())
                append(value.toString())
            }
            is Map<*, *> -> {
                require(value.size <= 4096 && value.keys.all { it is String })
                append('{')
                value.entries.forEachIndexed { index, entry ->
                    if (index != 0) append(',')
                    appendString(entry.key as String)
                    append(':')
                    appendValue(entry.value, depth + 1)
                }
                append('}')
            }
            is Iterable<*> -> {
                val items = value.toList()
                require(items.size <= 4096)
                append('[')
                items.forEachIndexed { index, item ->
                    if (index != 0) append(',')
                    appendValue(item, depth + 1)
                }
                append(']')
            }
            is Array<*> -> {
                require(value.size <= 4096)
                append('[')
                value.forEachIndexed { index, item ->
                    if (index != 0) append(',')
                    appendValue(item, depth + 1)
                }
                append(']')
            }
            else -> throw IllegalArgumentException("unsupported JSON value")
        }
    }

    private fun StringBuilder.appendString(value: String) {
        validateUnicode(value)
        append('"')
        value.forEach { char ->
            when (char) {
                '"' -> append("\\\"")
                '\\' -> append("\\\\")
                '\b' -> append("\\b")
                '\u000c' -> append("\\f")
                '\n' -> append("\\n")
                '\r' -> append("\\r")
                '\t' -> append("\\t")
                else -> if (char.code < 0x20) {
                    append("\\u%04x".format(char.code))
                } else {
                    append(char)
                }
            }
        }
        append('"')
    }

    private fun validateUnicode(value: String) {
        var index = 0
        while (index < value.length) {
            val char = value[index]
            if (char.isHighSurrogate()) {
                require(index + 1 < value.length && value[index + 1].isLowSurrogate())
                index += 2
            } else {
                require(!char.isLowSurrogate())
                index++
            }
        }
    }

    private class Parser(private val source: String) {
        private var offset = 0
        fun atEnd(): Boolean = offset == source.length

        fun skipSpace() {
            while (offset < source.length && source[offset] in charArrayOf(' ', '\r', '\n', '\t')) {
                offset++
            }
        }

        fun readValue(depth: Int): Any? {
            require(depth <= 32)
            skipSpace()
            require(offset < source.length)
            return when (source[offset]) {
                '{' -> readObject(depth + 1)
                '[' -> readArray(depth + 1)
                '"' -> readString()
                't' -> readLiteral("true", true)
                'f' -> readLiteral("false", false)
                'n' -> readLiteral("null", null)
                '-', in '0'..'9' -> readNumber()
                else -> throw IllegalArgumentException("invalid JSON")
            }
        }

        private fun readObject(depth: Int): Map<String, Any?> {
            offset++
            skipSpace()
            val result = LinkedHashMap<String, Any?>()
            if (consume('}')) return result
            while (true) {
                skipSpace()
                require(offset < source.length && source[offset] == '"')
                val key = readString()
                require(!result.containsKey(key))
                skipSpace()
                require(consume(':'))
                result[key] = readValue(depth)
                require(result.size <= 4096)
                skipSpace()
                if (consume('}')) return result
                require(consume(','))
            }
        }

        private fun readArray(depth: Int): List<Any?> {
            offset++
            skipSpace()
            val result = ArrayList<Any?>()
            if (consume(']')) return result
            while (true) {
                result.add(readValue(depth))
                require(result.size <= 4096)
                skipSpace()
                if (consume(']')) return result
                require(consume(','))
            }
        }

        private fun readString(): String {
            require(consume('"'))
            val result = StringBuilder()
            while (offset < source.length) {
                val char = source[offset++]
                when {
                    char == '"' -> {
                        val value = result.toString()
                        validateUnicode(value)
                        return value
                    }
                    char == '\\' -> {
                        require(offset < source.length)
                        when (val escaped = source[offset++]) {
                            '"', '\\', '/' -> result.append(escaped)
                            'b' -> result.append('\b')
                            'f' -> result.append('\u000c')
                            'n' -> result.append('\n')
                            'r' -> result.append('\r')
                            't' -> result.append('\t')
                            'u' -> result.append(readHexChar())
                            else -> throw IllegalArgumentException("invalid escape")
                        }
                    }
                    char.code < 0x20 -> throw IllegalArgumentException("control character")
                    else -> result.append(char)
                }
                require(result.length <= 4 * 1024 * 1024)
            }
            throw IllegalArgumentException("unterminated string")
        }

        private fun readHexChar(): Char {
            require(offset + 4 <= source.length)
            val value = source.substring(offset, offset + 4).toIntOrNull(16)
                ?: throw IllegalArgumentException("invalid unicode escape")
            offset += 4
            return value.toChar()
        }

        private fun readNumber(): Number {
            val start = offset
            consume('-')
            require(offset < source.length)
            if (consume('0')) {
                require(offset == source.length || source[offset] !in '0'..'9')
            } else {
                require(source[offset] in '1'..'9')
                while (offset < source.length && source[offset] in '0'..'9') offset++
            }
            if (consume('.')) {
                require(offset < source.length && source[offset] in '0'..'9')
                while (offset < source.length && source[offset] in '0'..'9') offset++
            }
            if (offset < source.length && source[offset] in charArrayOf('e', 'E')) {
                offset++
                if (offset < source.length && source[offset] in charArrayOf('+', '-')) offset++
                require(offset < source.length && source[offset] in '0'..'9')
                while (offset < source.length && source[offset] in '0'..'9') offset++
            }
            val raw = source.substring(start, offset)
            return raw.toLongOrNull() ?: raw.toDoubleOrNull()?.takeIf { it.isFinite() }
            ?: throw IllegalArgumentException("invalid number")
        }

        private fun <T> readLiteral(value: String, result: T): T {
            require(source.startsWith(value, offset))
            offset += value.length
            return result
        }

        private fun consume(char: Char): Boolean {
            if (offset < source.length && source[offset] == char) {
                offset++
                return true
            }
            return false
        }
    }
}
