package org.moreiralabs.ouril.ui.board3d

import android.app.ActivityManager
import android.content.Context
import android.graphics.Bitmap
import android.util.Half
import android.util.Log
import android.view.Choreographer
import android.view.Surface
import android.view.TextureView
import com.google.android.filament.Box
import com.google.android.filament.Camera
import com.google.android.filament.ColorGrading
import com.google.android.filament.Engine
import com.google.android.filament.EntityManager
import com.google.android.filament.Filament
import com.google.android.filament.IndexBuffer
import com.google.android.filament.IndirectLight
import com.google.android.filament.LightManager
import com.google.android.filament.Material
import com.google.android.filament.MaterialInstance
import com.google.android.filament.RenderableManager
import com.google.android.filament.Renderer
import com.google.android.filament.SurfaceOrientation
import com.google.android.filament.SwapChain
import com.google.android.filament.Texture
import com.google.android.filament.TextureSampler
import com.google.android.filament.ToneMapper
import com.google.android.filament.VertexBuffer
import com.google.android.filament.View
import com.google.android.filament.Viewport
import com.google.android.filament.android.TextureHelper
import com.google.android.filament.android.UiHelper
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.FloatBuffer
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.ln
import kotlin.math.max
import kotlin.math.min
import kotlin.math.sin

/** Whether this device can show the 3D board (OpenGL ES 3 and Filament); otherwise the 2D board shows. */
object Board3DSupport {
    @Volatile private var result: Boolean? = null

    /** Why the 3D board isn't showing, once it has failed (for test builds and the log). */
    @Volatile var failure: String? = null
        private set

    fun available(context: Context): Boolean = result ?: check(context).also { result = it }

    /** The 3D board couldn't start: the 2D board shows from now on. */
    fun failed(reason: String) {
        result = false
        failure = reason
        Log.w("Ouril", "3D board off: $reason")
    }

    private fun check(context: Context): Boolean {
        val gles = context.getSystemService(ActivityManager::class.java)?.deviceConfigurationInfo?.reqGlEsVersion ?: 0
        if (gles < 0x30000) {
            failed("OpenGL ES ${gles shr 16}.${gles and 0xffff}, needs 3.0")
            return false
        }
        return try {
            Filament.init()
            true
        } catch (e: Throwable) {
            failed("Filament.init: $e")
            false
        }
    }
}

/**
 * The 3D board drawn with Filament (ADR 0036): the web's scene (`Board3D.tsx`, `Sowing.tsx`) built
 * from the same geometry, colours and lights, posed every frame from the shared choreography
 * ([Choreography], [CameraRig], rings). It draws into a transparent [TextureView]; the cloth under
 * it and the seed counts over it are Compose (`Board3D.kt`).
 *
 * Renders only while something moves: [request] asks for frames, and [onFrame] (stepping the
 * motion) says whether to keep going.
 */
class BoardScene(private val context: Context, density: Float) {
    private val engine: Engine = Engine.create()
    private val renderer: Renderer = engine.createRenderer()
    private val scene = engine.createScene()
    private val view: View = engine.createView()
    private val cameraEntity = EntityManager.get().create()
    private val camera: Camera = engine.createCamera(cameraEntity)
    private val uiHelper = UiHelper(UiHelper.ContextErrorPolicy.DONT_CHECK)
    private var swapChain: SwapChain? = null

    /** Scratch for [setTransform] (used while the scene is built, so declared first). */
    private val matrix = FloatArray(16)

    /** The view's size in pixels (0 until the surface is laid out). */
    var width = 0
        private set
    var height = 0
        private set

    /** Steps the motion for a frame and returns whether anything still moves. */
    var onFrame: (Long) -> Boolean = { false }

    /** Drawing failed: the board falls back to 2D (`GameBoard`). */
    var onFailure: (Throwable) -> Unit = {}

    private val entities = mutableListOf<Int>()
    private val buffers = mutableListOf<Pair<VertexBuffer, IndexBuffer>>()
    private val instances = mutableListOf<MaterialInstance>()
    private val textures = mutableListOf<Texture>()
    private val litColor = material("lit_color")
    private val litTexture = material("lit_texture")
    private val ringMaterial = material("ring")
    private val ibl: IndirectLight

    private val seedMesh: Mesh
    private val seeds = mutableListOf<Int>()
    private var orientations: List<Quat> = emptyList()
    private val ringInstances = mutableListOf<MaterialInstance>()
    private val hand: Hand
    private val sleeve: MaterialInstance

    init {
        view.camera = camera
        view.scene = scene
        // Transparent: the cloth shows through, drawn behind by Compose in its own colours.
        view.blendMode = View.BlendMode.TRANSLUCENT
        renderer.clearOptions = Renderer.ClearOptions().apply {
            clearColor = doubleArrayOf(0.0, 0.0, 0.0, 0.0)
            clear = true
        }
        // three.js's ACES filmic tone mapping, as the web renders.
        view.colorGrading = ColorGrading.Builder().toneMapper(ToneMapper.ACESLegacy()).build(engine)
        view.multiSampleAntiAliasingOptions = View.MultiSampleAntiAliasingOptions().apply {
            enabled = true
            sampleCount = 4
        }
        // At most 1.75 pixels per dp, as the web caps its canvas.
        val scale = min(1f, 1.75f / density)
        if (scale < 1f) {
            view.dynamicResolutionOptions = View.DynamicResolutionOptions().apply {
                enabled = true
                homogeneousScaling = true
                minScale = scale
                maxScale = scale
            }
        }
        // Exposure 1: light intensities are three.js's.
        camera.setExposure(1f, 1.2f, 100f)

        directional("#ffe2bf", 2.1f, Vec3(-4.0, 9.0, 6.0), shadows = true)
        directional("#9cc7ff", 0.6f, Vec3(6.0, 4.0, -6.0), shadows = false)
        ibl = IndirectLight.Builder()
            .reflections(environment(32))
            .irradiance(3, BoardLight.irradiance())
            .intensity(1f)
            .build(engine)
        scene.indirectLight = ibl

        buildBoard()
        seedMesh = mesh(sphere(0.105, 20, 14))
        sleeve = colored("#3f56d6", roughness = 0.9)
        hand = Hand()
        val ring = mesh(ring(BoardGeometry.PIT_R + 0.02, BoardGeometry.PIT_R + 0.085, 64))
        for (i in 0 until 12) {
            val mi = ringMaterial.createInstance().also { instances += it }
            ringInstances += mi
            val e = renderable(ring, mi, cast = false, receive = false)
            val c = pitCenter(i, 6)
            setTransform(e, translation(c.x, BoardGeometry.TOP + 0.006, c.z))
        }

        uiHelper.isOpaque = false
        uiHelper.renderCallback = object : UiHelper.RendererCallback {
            override fun onNativeWindowChanged(surface: Surface) {
                swapChain?.let { engine.destroySwapChain(it) }
                swapChain = engine.createSwapChain(surface, uiHelper.swapChainFlags)
                request()
            }

            override fun onDetachedFromSurface() {
                swapChain?.let {
                    engine.destroySwapChain(it)
                    engine.flushAndWait()
                }
                swapChain = null
            }

            override fun onResized(width: Int, height: Int) {
                this@BoardScene.width = width
                this@BoardScene.height = height
                view.viewport = Viewport(0, 0, width, height)
                request()
            }
        }
    }

    fun attach(textureView: TextureView) {
        try {
            uiHelper.attachTo(textureView)
        } catch (e: Throwable) {
            destroyed = true
            onFailure(e)
        }
    }

    // --- Frames -------------------------------------------------------------------------------

    private val choreographer = Choreographer.getInstance()
    private var pending = false
    private var destroyed = false
    private val frameCallback = Choreographer.FrameCallback { nanos ->
        pending = false
        if (destroyed) return@FrameCallback
        try {
            val active = onFrame(nanos)
            render(nanos)
            if (active) request()
        } catch (e: Throwable) {
            destroyed = true
            onFailure(e)
        }
    }

    /** Draws the next frame (and keeps drawing while [onFrame] says something moves). */
    fun request() {
        if (pending || destroyed) return
        pending = true
        choreographer.postFrameCallback(frameCallback)
    }

    private fun render(nanos: Long) {
        val chain = swapChain ?: return
        if (!uiHelper.isReadyToRender || width == 0 || height == 0) return
        if (renderer.beginFrame(chain, nanos)) {
            renderer.render(view)
            renderer.endFrame()
        }
    }

    // --- Posing the scene ---------------------------------------------------------------------

    /** Places the camera, every seed, the hand and the rings for this frame. */
    fun update(choreo: Choreography, rings: List<RingState>, eye: Vec3, now: Double) {
        if (width > 0 && height > 0) {
            camera.setProjection(CameraLens.FOV, width.toDouble() / height, CameraLens.NEAR, CameraLens.FAR, Camera.Fov.VERTICAL)
        }
        val look = CameraLens.LOOK_AT
        camera.lookAt(eye.x, eye.y, eye.z, look.x, look.y, look.z, 0.0, 1.0, 0.0)

        val motions = choreo.motions
        ensureSeeds(motions.size)
        val scale = doubleArrayOf(1.18, 0.78, 0.95)
        for (i in motions.indices) {
            val m = motions[i]
            val q = if (m.tumble != 0.0) quatMul(orientations[i], quatFromEuler(m.tumble, 0.0, 0.0)) else orientations[i]
            setTransform(seeds[i], compose(m.pos.x, m.pos.y, m.pos.z, q, scale))
            setVisible(seeds[i], true)
        }
        for (i in motions.size until seeds.size) setVisible(seeds[i], false)

        hand.pose(choreo.hand, now < choreo.hand.hideAt)

        rings.forEachIndexed { i, r ->
            // The web's ring isn't tone-mapped and glows (emissive 1.6 over its lit colour);
            // brighter here, so it reads the same after Filament's tone mapping.
            val gain = 3f
            ringInstances[i].setParameter("color", r.color[0].toFloat() * gain, r.color[1].toFloat() * gain, r.color[2].toFloat() * gain)
            ringInstances[i].setParameter("opacity", r.opacity.toFloat())
        }
    }

    private fun ensureSeeds(count: Int) {
        if (seeds.size >= count) return
        orientations = seedOrientations(count)
        val colors = seedColors(count)
        for (i in seeds.size until count) {
            val mi = litColor.createInstance().also { instances += it }
            mi.setParameter("baseColor", colors[i * 3].toFloat(), colors[i * 3 + 1].toFloat(), colors[i * 3 + 2].toFloat())
            mi.setParameter("roughness", 0.32f)
            mi.setParameter("clearCoat", 0.7f)
            mi.setParameter("clearCoatRoughness", 0.25f)
            seeds += renderable(seedMesh, mi, cast = true, receive = true)
        }
    }

    // --- The board ----------------------------------------------------------------------------

    private fun buildBoard() {
        val sampler = TextureSampler(TextureSampler.MinFilter.LINEAR_MIPMAP_LINEAR, TextureSampler.MagFilter.LINEAR, TextureSampler.WrapMode.REPEAT)
            .apply { anisotropy = 8f }
        val wood = litTexture.createInstance().also { instances += it }
        wood.setParameter("baseColorMap", texture(BoardTextures.boardWood()), sampler)
        // The web's repeat (0.085, 0.24) and offset (0.5, 0.5), V flipped as three.js flips canvases.
        wood.setParameter("uvTransform", 0.085f, -0.24f, 0.5f, 0.5f)
        wood.setParameter("roughness", 0.48f)
        wood.setParameter("clearCoat", 0.35f)
        wood.setParameter("clearCoatRoughness", 0.4f)
        renderable(mesh(slab(6)), wood, cast = true, receive = true)

        val bowlWood = litTexture.createInstance().also { instances += it }
        bowlWood.setParameter("baseColorMap", texture(BoardTextures.bowlWood()), sampler)
        bowlWood.setParameter("uvTransform", 0.5f, -0.5f, 0f, 1f)
        bowlWood.setParameter("roughness", 0.85f)
        bowlWood.setParameter("clearCoat", 0f)
        bowlWood.setParameter("clearCoatRoughness", 0f)
        val bowl = mesh(bowl())
        for (i in 0 until 12) {
            val c = pitCenter(i, 6)
            setTransform(renderable(bowl, bowlWood, cast = false, receive = true), translation(c.x, 0.0, c.z))
        }
        for (x in listOf(BoardGeometry.STORE_X, -BoardGeometry.STORE_X)) {
            val e = renderable(bowl, bowlWood, cast = false, receive = true)
            setTransform(e, compose(x, 0.0, 0.0, IDENTITY, doubleArrayOf(BoardGeometry.STORE_RX / BoardGeometry.PIT_R, 1.0, BoardGeometry.STORE_RZ / BoardGeometry.PIT_R)))
        }
        // The underside, so the board reads as solid.
        val under = renderable(
            mesh(box(BoardGeometry.HALF_W * 2 - 0.4, 0.04, BoardGeometry.HALF_D * 2 - 0.4)),
            colored("#2c170b", roughness = 0.9),
            cast = false,
            receive = true,
        )
        setTransform(under, translation(0.0, -BoardGeometry.THICK - 0.02, 0.0))
    }

    // --- The hand -----------------------------------------------------------------------------

    /** The web's procedural right hand (`Sowing.tsx`): a palm, four fingers in two parts, a thumb, a sleeve. */
    private inner class Hand {
        private val skin = colored("#7a4a2f", roughness = 0.58)
        private val palm = renderable(mesh(roundedBox(0.48, 0.13, 0.46, 0.06, 4)), skin)
        private val back = renderable(mesh(sphere(1.0, 20, 12)), skin)
        private val knuckleMesh = mesh(sphere(FINGER_R * 1.12, 12, 10))
        private val knuckles = FINGERS.map { renderable(knuckleMesh, skin) }
        private val proximal = FINGERS.map { renderable(mesh(capsule(FINGER_R, it.len0 - FINGER_R, 4, 10)), skin) }
        private val distal = FINGERS.map { renderable(mesh(capsule(FINGER_R * 0.92, it.len1 - FINGER_R, 4, 10)), skin) }
        private val thumb = renderable(mesh(capsule(0.062, 0.17, 4, 10)), skin)
        private val forearm = renderable(mesh(capsule(0.15, 0.8, 4, 14)), skin)
        private val sleeveEntity = renderable(mesh(cylinder(0.24, 0.19, 3.0, 20)), sleeve)
        private val all = listOf(palm, back, thumb, forearm, sleeveEntity) + knuckles + proximal + distal
        private var side = 1

        fun pose(h: HandState, shown: Boolean) {
            all.forEach { setVisible(it, shown) }
            if (!shown) return
            if (h.side != side) {
                side = h.side
                val c = srgbHexToLinear(if (side > 0) "#3f56d6" else "#c8202a")
                sleeve.setParameter("baseColor", c[0].toFloat(), c[1].toFloat(), c[2].toFloat())
            }
            val g = h.grip
            // South's hand reaches in from the camera side, fingers pointing away; North's mirrors it.
            val root = trs(h.pos.x, h.pos.y, h.pos.z, 0.08 + g * 0.1, if (h.side > 0) 0.0 else PI, 0.0)
            val model = mul(root, scale(HAND_SCALE))
            setTransform(palm, mul(model, translation(0.0, PALM_Y, 0.0)))
            setTransform(back, mul(model, compose(0.0, PALM_Y + 0.05, 0.02, IDENTITY, doubleArrayOf(0.25, 0.07, 0.24))))
            FINGERS.forEachIndexed { i, f ->
                val knuckle = mul(model, trs(f.x, PALM_Y - 0.01, -0.2, -(0.35 + g * 0.95), 0.0, 0.0))
                setTransform(knuckles[i], knuckle)
                setTransform(proximal[i], mul(knuckle, trs(0.0, 0.0, -f.len0 / 2, PI / 2, 0.0, 0.0)))
                val tip = mul(knuckle, trs(0.0, 0.0, -f.len0 + 0.015, -(0.25 + g * 1.05), 0.0, 0.0))
                setTransform(distal[i], mul(tip, trs(0.0, 0.0, -f.len1 / 2, PI / 2, 0.0, 0.0)))
            }
            val thumbGroup = mul(model, trs(-0.25, PALM_Y - 0.02, 0.06, -g * 0.5, 0.6 - g * 0.8, 0.0))
            setTransform(thumb, mul(thumbGroup, trs(0.0, 0.0, -0.12, PI / 2, 0.0, 0.0)))
            // Wrist, forearm and a long sleeve: a right arm coming in from the player's right.
            val arm = mul(model, trs(0.04, PALM_Y + 0.01, 0.2, -0.62, 0.62, 0.0, yxz = true))
            setTransform(forearm, mul(arm, trs(0.0, 0.0, 0.45, PI / 2, 0.0, 0.0)))
            setTransform(sleeveEntity, mul(arm, trs(0.0, 0.0, 2.35, PI / 2, 0.0, 0.0)))
        }
    }

    // --- Filament plumbing --------------------------------------------------------------------

    private class Mesh(val vertices: VertexBuffer, val indices: IndexBuffer, val count: Int)

    private fun material(name: String): Material {
        val bytes = context.assets.open("materials/$name.filamat").use { it.readBytes() }
        val buffer = ByteBuffer.allocateDirect(bytes.size).order(ByteOrder.nativeOrder()).put(bytes)
        buffer.flip()
        return Material.Builder().payload(buffer, bytes.size).build(engine)
    }

    private fun colored(hex: String, roughness: Double): MaterialInstance {
        val c = srgbHexToLinear(hex)
        return litColor.createInstance().also {
            it.setParameter("baseColor", c[0].toFloat(), c[1].toFloat(), c[2].toFloat())
            it.setParameter("roughness", roughness.toFloat())
            it.setParameter("clearCoat", 0f)
            it.setParameter("clearCoatRoughness", 0f)
            instances += it
        }
    }

    private fun texture(bitmap: Bitmap): Texture {
        val levels = (ln(max(bitmap.width, bitmap.height).toDouble()) / ln(2.0)).toInt() + 1
        val t = Texture.Builder()
            .width(bitmap.width)
            .height(bitmap.height)
            .levels(levels)
            .sampler(Texture.Sampler.SAMPLER_2D)
            .format(Texture.InternalFormat.SRGB8_A8)
            .usage(Texture.Usage.DEFAULT or Texture.Usage.GEN_MIPMAPPABLE)
            .build(engine)
        TextureHelper.setBitmap(engine, t, 0, bitmap)
        t.generateMipmaps(engine)
        textures += t
        return t
    }

    /**
     * The light panels as a small cubemap (half floats), for the glaze's reflections. Its smaller
     * levels are averaged here, not on the GPU: not every phone can render to half floats.
     */
    private fun environment(size: Int): Texture {
        val levels = (ln(size.toDouble()) / ln(2.0)).toInt() + 1
        val t = Texture.Builder()
            .width(size)
            .height(size)
            .levels(levels)
            .sampler(Texture.Sampler.SAMPLER_CUBEMAP)
            .format(Texture.InternalFormat.RGBA16F)
            .build(engine)
        var rgb = BoardLight.cubemap(size)
        var s = size
        for (level in 0 until levels) {
            val texels = 6 * s * s
            val buffer = ByteBuffer.allocateDirect(texels * 4 * 2).order(ByteOrder.nativeOrder())
            for (i in 0 until texels) {
                buffer.putShort(Half.toHalf(rgb[i * 3]))
                buffer.putShort(Half.toHalf(rgb[i * 3 + 1]))
                buffer.putShort(Half.toHalf(rgb[i * 3 + 2]))
                buffer.putShort(Half.toHalf(1f))
            }
            buffer.flip()
            t.setImage(engine, level, 0, 0, 0, s, s, 6, Texture.PixelBufferDescriptor(buffer, Texture.Format.RGBA, Texture.Type.HALF, 1))
            if (s > 1) {
                rgb = halve(rgb, s)
                s /= 2
            }
        }
        textures += t
        return t
    }

    /** A cubemap's next level: each face's texels averaged in twos. */
    private fun halve(rgb: FloatArray, size: Int): FloatArray {
        val h = size / 2
        val out = FloatArray(6 * h * h * 3)
        for (f in 0 until 6) for (y in 0 until h) for (x in 0 until h) for (c in 0 until 3) {
            fun at(xx: Int, yy: Int) = rgb[((f * size + yy) * size + xx) * 3 + c]
            out[((f * h + y) * h + x) * 3 + c] = (at(2 * x, 2 * y) + at(2 * x + 1, 2 * y) + at(2 * x, 2 * y + 1) + at(2 * x + 1, 2 * y + 1)) / 4
        }
        return out
    }

    private fun directional(color: String, intensity: Float, from: Vec3, shadows: Boolean) {
        val e = EntityManager.get().create()
        val c = srgbHexToLinear(color)
        val d = Vec3(-from.x, -from.y, -from.z).normalize()
        val builder = LightManager.Builder(LightManager.Type.DIRECTIONAL)
            .color(c[0].toFloat(), c[1].toFloat(), c[2].toFloat())
            .intensity(intensity)
            .direction(d.x.toFloat(), d.y.toFloat(), d.z.toFloat())
            .castShadows(shadows)
        if (shadows) {
            builder.shadowOptions(
                LightManager.ShadowOptions().apply {
                    mapSize = 1024
                    // The board is within 40 units of the camera on any screen: the shadow map covers that, not the far plane.
                    shadowFar = 40f
                    shadowFarHint = 40f
                },
            )
        }
        builder.build(engine, e)
        scene.addEntity(e)
        entities += e
    }

    private fun mesh(data: MeshData): Mesh {
        val n = data.vertexCount
        val uvs = data.uvs
        val builder = VertexBuffer.Builder()
            .bufferCount(if (uvs != null) 3 else 2)
            .vertexCount(n)
            .attribute(VertexBuffer.VertexAttribute.POSITION, 0, VertexBuffer.AttributeType.FLOAT3, 0, 12)
            .attribute(VertexBuffer.VertexAttribute.TANGENTS, 1, VertexBuffer.AttributeType.FLOAT4, 0, 16)
        if (uvs != null) builder.attribute(VertexBuffer.VertexAttribute.UV0, 2, VertexBuffer.AttributeType.FLOAT2, 0, 8)
        val vb = builder.build(engine)
        vb.setBufferAt(engine, 0, floats(data.positions))
        val orientation = SurfaceOrientation.Builder().vertexCount(n).normals(floats(data.normals)).build()
        val tangents = ByteBuffer.allocateDirect(n * 16).order(ByteOrder.nativeOrder()).asFloatBuffer()
        orientation.getQuatsAsFloat(tangents)
        orientation.destroy()
        tangents.rewind()
        vb.setBufferAt(engine, 1, tangents)
        if (uvs != null) vb.setBufferAt(engine, 2, floats(uvs))
        val ib = IndexBuffer.Builder().indexCount(data.indices.size).bufferType(IndexBuffer.Builder.IndexType.UINT).build(engine)
        val idx = ByteBuffer.allocateDirect(data.indices.size * 4).order(ByteOrder.nativeOrder()).asIntBuffer()
        idx.put(data.indices)
        idx.rewind()
        ib.setBuffer(engine, idx)
        buffers += vb to ib
        return Mesh(vb, ib, data.indices.size)
    }

    private fun floats(values: FloatArray): FloatBuffer {
        val b = ByteBuffer.allocateDirect(values.size * 4).order(ByteOrder.nativeOrder()).asFloatBuffer()
        b.put(values)
        b.rewind()
        return b
    }

    private fun renderable(mesh: Mesh, material: MaterialInstance, cast: Boolean = true, receive: Boolean = false): Int {
        val e = EntityManager.get().create()
        engine.transformManager.create(e)
        RenderableManager.Builder(1)
            .boundingBox(Box(0f, 0f, 0f, 8f, 8f, 8f))
            .geometry(0, RenderableManager.PrimitiveType.TRIANGLES, mesh.vertices, mesh.indices, 0, mesh.count)
            .material(0, material)
            .castShadows(cast)
            .receiveShadows(receive)
            .culling(false)
            .build(engine, e)
        scene.addEntity(e)
        entities += e
        return e
    }

    private fun setTransform(entity: Int, m: DoubleArray) {
        for (i in 0 until 16) matrix[i] = m[i].toFloat()
        val tm = engine.transformManager
        tm.setTransform(tm.getInstance(entity), matrix)
    }

    private fun setVisible(entity: Int, visible: Boolean) {
        val rm = engine.renderableManager
        rm.setLayerMask(rm.getInstance(entity), 0xff, if (visible) 0x1 else 0x0)
    }

    fun destroy() {
        destroyed = true
        choreographer.removeFrameCallback(frameCallback)
        uiHelper.detach()
        swapChain?.let { engine.destroySwapChain(it) }
        swapChain = null
        for (e in entities) {
            engine.destroyEntity(e)
            EntityManager.get().destroy(e)
        }
        instances.forEach(engine::destroyMaterialInstance)
        listOf(litColor, litTexture, ringMaterial).forEach(engine::destroyMaterial)
        engine.destroyIndirectLight(ibl)
        textures.forEach(engine::destroyTexture)
        for ((vb, ib) in buffers) {
            engine.destroyVertexBuffer(vb)
            engine.destroyIndexBuffer(ib)
        }
        engine.destroyRenderer(renderer)
        engine.destroyView(view)
        engine.destroyScene(scene)
        engine.destroyCameraComponent(cameraEntity)
        EntityManager.get().destroy(cameraEntity)
        engine.destroy()
    }

    private companion object {
        class Finger(val x: Double, val len0: Double, val len1: Double)

        /** Index to little finger: x across the palm and the two segment lengths (model units). */
        val FINGERS = listOf(Finger(-0.17, 0.21, 0.17), Finger(-0.06, 0.24, 0.19), Finger(0.055, 0.23, 0.18), Finger(0.165, 0.18, 0.14))
        const val FINGER_R = 0.052
        const val HAND_SCALE = 1.7

        /** The hand's anchor is the centre of the handful; the palm sits above it (model units). */
        const val PALM_Y = 0.18
        val IDENTITY = Quat(0.0, 0.0, 0.0, 1.0)
    }
}

// --- Transforms, as three.js composes them (column-major) ---------------------------------------

/** three.js's `Quaternion.setFromEuler`, orders XYZ (its default) and YXZ. */
fun quatFromEuler(x: Double, y: Double, z: Double, yxz: Boolean = false): Quat {
    val c1 = cos(x / 2)
    val c2 = cos(y / 2)
    val c3 = cos(z / 2)
    val s1 = sin(x / 2)
    val s2 = sin(y / 2)
    val s3 = sin(z / 2)
    return if (!yxz) {
        Quat(s1 * c2 * c3 + c1 * s2 * s3, c1 * s2 * c3 - s1 * c2 * s3, c1 * c2 * s3 + s1 * s2 * c3, c1 * c2 * c3 - s1 * s2 * s3)
    } else {
        Quat(s1 * c2 * c3 + c1 * s2 * s3, c1 * s2 * c3 - s1 * c2 * s3, c1 * c2 * s3 - s1 * s2 * c3, c1 * c2 * c3 + s1 * s2 * s3)
    }
}

/** three.js's `multiplyQuaternions(a, b)`: b's rotation, then a's. */
fun quatMul(a: Quat, b: Quat) = Quat(
    a.x * b.w + a.w * b.x + a.y * b.z - a.z * b.y,
    a.y * b.w + a.w * b.y + a.z * b.x - a.x * b.z,
    a.z * b.w + a.w * b.z + a.x * b.y - a.y * b.x,
    a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
)

/** three.js's `Matrix4.compose`: translation × rotation × scale. */
fun compose(px: Double, py: Double, pz: Double, q: Quat, s: DoubleArray): DoubleArray {
    val x2 = q.x + q.x
    val y2 = q.y + q.y
    val z2 = q.z + q.z
    val xx = q.x * x2
    val xy = q.x * y2
    val xz = q.x * z2
    val yy = q.y * y2
    val yz = q.y * z2
    val zz = q.z * z2
    val wx = q.w * x2
    val wy = q.w * y2
    val wz = q.w * z2
    return doubleArrayOf(
        (1 - (yy + zz)) * s[0], (xy + wz) * s[0], (xz - wy) * s[0], 0.0,
        (xy - wz) * s[1], (1 - (xx + zz)) * s[1], (yz + wx) * s[1], 0.0,
        (xz + wy) * s[2], (yz - wx) * s[2], (1 - (xx + yy)) * s[2], 0.0,
        px, py, pz, 1.0,
    )
}

fun trs(px: Double, py: Double, pz: Double, rx: Double, ry: Double, rz: Double, yxz: Boolean = false): DoubleArray =
    compose(px, py, pz, quatFromEuler(rx, ry, rz, yxz), doubleArrayOf(1.0, 1.0, 1.0))

fun translation(x: Double, y: Double, z: Double): DoubleArray = compose(x, y, z, Quat(0.0, 0.0, 0.0, 1.0), doubleArrayOf(1.0, 1.0, 1.0))

fun scale(s: Double): DoubleArray = compose(0.0, 0.0, 0.0, Quat(0.0, 0.0, 0.0, 1.0), doubleArrayOf(s, s, s))

/** a × b (column-major): b applied first. */
fun mul(a: DoubleArray, b: DoubleArray): DoubleArray {
    val out = DoubleArray(16)
    for (col in 0 until 4) for (row in 0 until 4) {
        var sum = 0.0
        for (k in 0 until 4) sum += a[k * 4 + row] * b[col * 4 + k]
        out[col * 4 + row] = sum
    }
    return out
}
