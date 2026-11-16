package org.moreiralabs.ouril.ui.board3d

import android.graphics.Bitmap
import android.graphics.BlurMaskFilter
import android.graphics.Canvas
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.graphics.Shader
import androidx.core.graphics.createBitmap
import kotlin.math.PI
import kotlin.math.sin

/**
 * The 3D board's textures, drawn in code exactly as the web's `board3d/textures.ts` draws them on a
 * canvas (the same random sequence, the same strokes): no image files, works offline.
 */
object BoardTextures {
    /** Warm hardwood: long grain lines with gentle waves, a few darker streaks and pores. */
    fun wood(base: Int, dark: Int, light: Int, seed: Int): Bitmap {
        val w = 1024
        val h = 512
        val bitmap = createBitmap(w, h)
        val canvas = Canvas(bitmap)
        val rand = rng(seed)
        canvas.drawColor(base)

        // Broad colour bands.
        for (i in 0 until 14) {
            val y = (rand() * h).toFloat()
            val color = if (rand() > 0.5) dark else light
            val paint = Paint().apply {
                shader = LinearGradient(0f, y - 40, 0f, y + 40, intArrayOf(color and 0xffffff, color, color and 0xffffff), floatArrayOf(0f, 0.5f, 1f), Shader.TileMode.CLAMP)
                alpha = alpha8(0.12 + rand() * 0.12)
            }
            canvas.drawRect(0f, y - 40, w.toFloat(), y + 40, paint)
        }

        // Grain lines: long, slightly wavy, varying weight.
        val stroke = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE }
        for (i in 0 until 220) {
            val y0 = rand() * h
            val amp = 2 + rand() * 9
            val freq = 0.002 + rand() * 0.006
            val phase = rand() * PI * 2
            val a = 0.05 + rand() * 0.16
            stroke.color = if (rand() > 0.3) dark else light
            stroke.alpha = alpha8(a)
            stroke.strokeWidth = (0.5 + rand() * 1.8).toFloat()
            val path = Path()
            var x = 0
            while (x <= w) {
                val y = y0 + sin(x * freq + phase) * amp + sin(x * freq * 3.1) * amp * 0.25
                if (x == 0) path.moveTo(0f, y.toFloat()) else path.lineTo(x.toFloat(), y.toFloat())
                x += 16
            }
            canvas.drawPath(path, stroke)
        }

        // Pores.
        val pore = Paint().apply {
            color = dark
            alpha = alpha8(0.18)
        }
        repeat(1800) {
            val x = (rand() * w).toFloat()
            val y = (rand() * h).toFloat()
            val width = (1 + rand() * 3).toFloat()
            canvas.drawRect(x, y, x + width, y + 1, pore)
        }
        return bitmap
    }

    /** The board: warm hardwood. */
    fun boardWood(): Bitmap = wood(0xff7a4524.toInt(), 0xff3e1f0e.toInt(), 0xffb07745.toInt(), 11)

    /** The bowls: darker wood. */
    fun bowlWood(): Bitmap = wood(0xff4a2814.toInt(), 0xff24110a.toInt(), 0xff73452a.toInt(), 23)

    /**
     * One tile of the panu cloth under the board (5 world units square): indigo strips, a soft
     * weave, lozenges on every other strip, stitched seams. As on the web, the canvas's opacity
     * carries from one strip into the next, and the tile shows opaque in the strokes' colours.
     */
    fun clothTile(): Bitmap {
        val size = 512
        val strips = 4
        val h = (size / strips).toFloat()
        val bitmap = createBitmap(size, size)
        val canvas = Canvas(bitmap)
        val rand = rng(31)
        var globalAlpha = 1.0
        val cotton = 0xfff5f1e8.toInt()

        for (s in 0 until strips) {
            val y = s * h
            // Each strip a slightly different indigo, as hand-dyed strips are.
            val shade = if (s % 2 == 0) intArrayOf(0xff1d2b70.toInt(), 0xff17235e.toInt()) else intArrayOf(0xff223280.toInt(), 0xff1b296c.toInt())
            canvas.drawRect(0f, y, size.toFloat(), y + h, Paint().apply {
                shader = LinearGradient(0f, y, 0f, y + h, shade, null, Shader.TileMode.CLAMP)
                alpha = alpha8(globalAlpha)
            })

            // A soft weave: faint lengthwise threads.
            globalAlpha = 0.06
            val thread = Paint(Paint.ANTI_ALIAS_FLAG).apply {
                style = Paint.Style.STROKE
                color = cotton
                alpha = alpha8(globalAlpha)
                strokeWidth = 1f
            }
            repeat(10) {
                val ty = (y + 8 + rand() * (h - 16)).toFloat()
                canvas.drawLine(0f, ty, size.toFloat(), ty, thread)
            }

            // Lozenges on every other strip, outlined in cotton.
            if (s % 2 == 1) {
                globalAlpha = 0.22
                val line = Paint(Paint.ANTI_ALIAS_FLAG).apply {
                    style = Paint.Style.STROKE
                    color = cotton
                    alpha = alpha8(globalAlpha)
                    strokeWidth = 5f
                }
                val r = h * 0.3f
                var x = h / 2
                while (x < size) {
                    val cy = y + h / 2
                    canvas.drawPath(Path().apply {
                        moveTo(x, cy - r); lineTo(x + r, cy); lineTo(x, cy + r); lineTo(x - r, cy); close()
                    }, line)
                    x += h
                }
            }

            // The seam between strips: a darker line with cotton stitches across it.
            globalAlpha = 0.5
            canvas.drawRect(0f, y, size.toFloat(), y + 3, Paint().apply {
                color = 0xff0d1640.toInt()
                alpha = alpha8(globalAlpha)
            })
            globalAlpha = 0.3
            val stitch = Paint(Paint.ANTI_ALIAS_FLAG).apply {
                style = Paint.Style.STROKE
                color = cotton
                alpha = alpha8(globalAlpha)
                strokeWidth = 3f
            }
            var x = 6f
            while (x < size) {
                canvas.drawLine(x, y - 4, x + 6, y + 7, stitch)
                x += 16f
            }
        }
        return opaque(bitmap)
    }

    /**
     * The soft shadow the board casts on the cloth (the web's ContactShadows): its footprint,
     * blurred, in a [width] × [depth] world-unit rectangle centred under it.
     */
    fun contactShadow(width: Double, depth: Double): Bitmap {
        val pxPerUnit = 512 / width
        val w = 512
        val h = (depth * pxPerUnit).toInt()
        val bitmap = createBitmap(w, h)
        val canvas = Canvas(bitmap)
        val hw = BoardGeometry.HALF_W * pxPerUnit
        val hd = BoardGeometry.HALF_D * pxPerUnit
        val r = (BoardGeometry.CORNER * pxPerUnit).toFloat()
        val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = 0xff050816.toInt()
            // How dark the board's footprint renders from below (its top at 0.69 of the 1.6 range),
            // times the shadow's opacity.
            alpha = alpha8((1 - 0.69 / 1.6) * 0.55)
            maskFilter = BlurMaskFilter((0.35 * pxPerUnit).toFloat(), BlurMaskFilter.Blur.NORMAL)
        }
        canvas.drawRoundRect(RectF((w / 2 - hw).toFloat(), (h / 2 - hd).toFloat(), (w / 2 + hw).toFloat(), (h / 2 + hd).toFloat()), r, r, paint)
        return bitmap
    }

    private fun alpha8(a: Double) = (a.coerceIn(0.0, 1.0) * 255 + 0.5).toInt()

    /** Every pixel at full opacity in its own colour, as a canvas uploaded to WebGL shows. */
    private fun opaque(bitmap: Bitmap): Bitmap {
        val pixels = IntArray(bitmap.width * bitmap.height)
        bitmap.getPixels(pixels, 0, bitmap.width, 0, 0, bitmap.width, bitmap.height)
        for (i in pixels.indices) pixels[i] = pixels[i] or (0xff shl 24)
        bitmap.setPixels(pixels, 0, bitmap.width, 0, 0, bitmap.width, bitmap.height)
        return bitmap
    }
}
