package org.moreiralabs.ouril.game

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.math.abs

class SoundsTest {
    @Test
    fun aToneIsAWavThatFades() {
        val wav = renderWav(Tone(330.0, 180, gain = 0.12), rate = 8_000)
        val samples = 8_000 * 180 / 1000
        assertEquals(44 + samples * 2, wav.size)
        assertEquals("RIFF", String(wav, 0, 4))
        assertEquals("WAVE", String(wav, 8, 4))
        assertEquals("data", String(wav, 36, 4))

        val pcm = ByteBuffer.wrap(wav, 44, samples * 2).order(ByteOrder.LITTLE_ENDIAN).asShortBuffer()
        val levels = ShortArray(samples).also { pcm.get(it) }.map { abs(it.toInt()) }
        assertTrue(levels.max() <= 0.12 * Short.MAX_VALUE + 1)
        // Loud at the start, nearly silent at the end, as the web's exponential ramp.
        assertTrue(levels.take(samples / 10).max() > 10 * levels.takeLast(samples / 10).max())
    }
}
