/**
 * Procedural textures, drawn on a canvas at runtime: no image files to download or license,
 * and the board still works offline.
 */
import { CanvasTexture, RepeatWrapping, SRGBColorSpace } from 'three';

/** Deterministic pseudo-random numbers (mulberry32), so the grain is the same every load. */
export function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Warm hardwood: long grain lines with gentle waves, a few darker streaks and pores. */
export function woodTexture(opts: { base: string; dark: string; light: string; seed?: number }) {
  const w = 1024;
  const h = 512;
  const canvas = document.createElement('canvas');
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext('2d')!;
  const rand = rng(opts.seed ?? 7);

  ctx.fillStyle = opts.base;
  ctx.fillRect(0, 0, w, h);

  // Broad colour bands.
  for (let i = 0; i < 14; i++) {
    const y = rand() * h;
    const band = ctx.createLinearGradient(0, y - 40, 0, y + 40);
    band.addColorStop(0, 'transparent');
    band.addColorStop(0.5, rand() > 0.5 ? opts.dark : opts.light);
    band.addColorStop(1, 'transparent');
    ctx.globalAlpha = 0.12 + rand() * 0.12;
    ctx.fillStyle = band;
    ctx.fillRect(0, y - 40, w, 80);
  }

  // Grain lines: long, slightly wavy, varying weight.
  for (let i = 0; i < 220; i++) {
    const y0 = rand() * h;
    const amp = 2 + rand() * 9;
    const freq = 0.002 + rand() * 0.006;
    const phase = rand() * Math.PI * 2;
    ctx.globalAlpha = 0.05 + rand() * 0.16;
    ctx.strokeStyle = rand() > 0.3 ? opts.dark : opts.light;
    ctx.lineWidth = 0.5 + rand() * 1.8;
    ctx.beginPath();
    for (let x = 0; x <= w; x += 16) {
      const y = y0 + Math.sin(x * freq + phase) * amp + Math.sin(x * freq * 3.1) * amp * 0.25;
      if (x === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();
  }

  // Pores.
  ctx.globalAlpha = 0.18;
  ctx.fillStyle = opts.dark;
  for (let i = 0; i < 1800; i++) {
    ctx.fillRect(rand() * w, rand() * h, 1 + rand() * 3, 1);
  }
  ctx.globalAlpha = 1;

  const tex = new CanvasTexture(canvas);
  tex.colorSpace = SRGBColorSpace;
  tex.wrapS = RepeatWrapping;
  tex.wrapT = RepeatWrapping;
  tex.anisotropy = 8;
  return tex;
}
