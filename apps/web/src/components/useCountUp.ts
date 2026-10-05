import { useEffect, useState } from 'react';

function prefersReducedMotion(): boolean {
  return (
    typeof window !== 'undefined' &&
    !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
  );
}

/** Counts from 0 up to `target` with a strong ease-out (rare, end-of-game moment only). */
export function useCountUp(target: number, durationMs = 900): number {
  const [reduced] = useState(
    () => prefersReducedMotion() || typeof requestAnimationFrame !== 'function',
  );
  const [value, setValue] = useState(reduced ? target : 0);
  useEffect(() => {
    if (reduced) return;
    let raf = 0;
    const start = performance.now();
    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / durationMs);
      setValue(Math.round(target * (1 - Math.pow(1 - t, 4))));
      if (t < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [target, durationMs, reduced]);
  return reduced ? target : value;
}
