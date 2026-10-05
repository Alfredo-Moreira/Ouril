import { useRef } from 'react';

/**
 * Seconds since the previous call, from the scene clock (the clock the seed and hand timings
 * use), capped so a stalled tab doesn't jump. Call once per rendered frame.
 */
export function useClockDelta() {
  const last = useRef<number | null>(null);
  return (now: number) => {
    const dt = last.current === null ? 0 : Math.min(0.05, Math.max(0, now - last.current));
    last.current = now;
    return dt;
  };
}
