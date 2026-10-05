/** The 3D board's lazily loaded chunk (ADR 0021). */
export const loadBoard3D = () => import('./Board3D');

let started = false;

/**
 * Starts downloading the 3D board while the browser is idle, so it's usually cached before a
 * board is shown. Same-origin static asset only (no API call).
 */
export function preloadBoard3D(): void {
  if (started || typeof window === 'undefined') return;
  started = true;
  const go = () => void loadBoard3D().catch(() => {});
  if ('requestIdleCallback' in window) window.requestIdleCallback(go, { timeout: 2000 });
  else setTimeout(go, 600);
}
