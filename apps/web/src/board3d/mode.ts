/**
 * What the browser can do for the board (ADR 0021): the board is 3D, so the question is
 * whether WebGL2 is there.
 */

export interface DeviceInfo {
  webgl: boolean;
}

/** What the browser can do. Cheap, synchronous, safe to call in tests (jsdom: no WebGL). */
export function detectDevice(): DeviceInfo {
  if (typeof window === 'undefined') return { webgl: false };
  return { webgl: hasWebGL2() };
}

let webglCache: boolean | undefined;

function hasWebGL2(): boolean {
  if (webglCache !== undefined) return webglCache;
  try {
    const canvas = document.createElement('canvas');
    const gl = canvas.getContext('webgl2');
    webglCache = !!gl;
    gl?.getExtension('WEBGL_lose_context')?.loseContext();
  } catch {
    webglCache = false;
  }
  return webglCache;
}
