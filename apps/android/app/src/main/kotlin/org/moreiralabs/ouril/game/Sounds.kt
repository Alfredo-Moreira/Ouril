package org.moreiralabs.ouril.game

import android.content.Context
import android.media.AudioAttributes
import android.media.SoundPool
import java.io.File
import java.util.concurrent.ConcurrentHashMap
import kotlin.math.PI
import kotlin.math.asin
import kotlin.math.pow
import kotlin.math.sign
import kotlin.math.sin
import kotlin.random.Random

/**
 * The web's sound effects (`game/sound.ts`): tiny tones made on the device, no audio assets.
 * Each tone is rendered once to a WAV file in the cache and played through a [SoundPool].
 */
class SoundEffects(private val context: Context) {
    private val pool = SoundPool.Builder()
        .setMaxStreams(4)
        .setAudioAttributes(
            AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_GAME)
                .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                .build(),
        )
        .build()
    private val loaded = ConcurrentHashMap<String, Int>()
    private val lock = Any()
    private var released = false

    /** Renders and loads every tone. Writes files: call it off the main thread. */
    fun load() {
        val dir = File(context.cacheDir, "sounds").apply { mkdirs() }
        for ((name, tone) in TONES) {
            val file = File(dir, "$name.wav").apply { writeBytes(renderWav(tone)) }
            synchronized(lock) {
                if (released) return
                loaded[name] = pool.load(file.path, 1)
            }
        }
    }

    fun frame(kind: FrameKind) {
        when (kind) {
            FrameKind.SOW -> play("sow${Random.nextInt(SOW_PITCHES)}")
            FrameKind.CAPTURE -> play("capture")
            FrameKind.GRAND_SLAM -> play("grand_slam")
            FrameKind.COLLECT -> play("collect")
            else -> Unit
        }
    }

    fun end(win: Boolean) = play(if (win) "win" else "loss")

    fun release() = synchronized(lock) {
        released = true
        pool.release()
    }

    /** Before [load] finishes, a tone simply isn't played. */
    private fun play(name: String) {
        loaded[name]?.let { pool.play(it, 1f, 1f, 1, 0, 1f) }
    }

    private companion object {
        /** The web picks a sowing pitch from 520–600 Hz at random; four steps sound the same. */
        const val SOW_PITCHES = 4
        val TONES: Map<String, Tone> =
            (0 until SOW_PITCHES).associate { "sow$it" to Tone(520.0 + it * 80.0 / (SOW_PITCHES - 1), 70, Wave.TRIANGLE) } +
                mapOf(
                    "capture" to Tone(330.0, 180, gain = 0.12),
                    "grand_slam" to Tone(660.0, 300, Wave.SQUARE, 0.06),
                    "collect" to Tone(260.0, 220),
                    "win" to Tone(784.0, 400, gain = 0.1),
                    "loss" to Tone(220.0, 400, gain = 0.1),
                )
    }
}

enum class Wave { SINE, TRIANGLE, SQUARE }

/** An oscillator note that starts at `gain` and fades exponentially, as the web's `tone()`. */
data class Tone(val freq: Double, val ms: Int, val wave: Wave = Wave.SINE, val gain: Double = 0.08)

/** A [Tone] as a 16-bit mono PCM WAV file. */
fun renderWav(tone: Tone, rate: Int = 44_100): ByteArray {
    val samples = rate * tone.ms / 1000
    val data = ByteArray(samples * 2)
    for (i in 0 until samples) {
        val t = i.toDouble() / rate
        val phase = sin(2 * PI * tone.freq * t)
        val wave = when (tone.wave) {
            Wave.SINE -> phase
            Wave.TRIANGLE -> 2 / PI * asin(phase)
            Wave.SQUARE -> sign(phase)
        }
        // exponentialRampToValueAtTime(0.0001, end), as on the web.
        val envelope = tone.gain * (0.0001 / tone.gain).pow(t * 1000 / tone.ms)
        val v = (wave * envelope * Short.MAX_VALUE).toInt()
        data[2 * i] = v.toByte()
        data[2 * i + 1] = (v shr 8).toByte()
    }
    return wavHeader(data.size, rate) + data
}

private fun wavHeader(dataBytes: Int, rate: Int): ByteArray {
    fun le32(v: Int) = byteArrayOf(v.toByte(), (v shr 8).toByte(), (v shr 16).toByte(), (v shr 24).toByte())
    fun le16(v: Int) = byteArrayOf(v.toByte(), (v shr 8).toByte())
    return "RIFF".toByteArray() + le32(36 + dataBytes) + "WAVE".toByteArray() +
        "fmt ".toByteArray() + le32(16) + le16(1) + le16(1) + le32(rate) + le32(rate * 2) + le16(2) + le16(16) +
        "data".toByteArray() + le32(dataBytes)
}
