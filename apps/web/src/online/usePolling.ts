import { useEffect, useRef } from 'react';

/**
 * Call `tick` every `ms` milliseconds while the page is visible, and once when it becomes
 * visible again. `ms = null` stops polling. Online games use this instead of a live
 * connection (ADR 0031).
 */
export function usePolling(tick: () => void, ms: number | null): void {
  const latest = useRef(tick);
  useEffect(() => {
    latest.current = tick;
  });
  useEffect(() => {
    if (ms === null) return;
    const run = () => {
      if (typeof document === 'undefined' || !document.hidden) latest.current();
    };
    const timer = setInterval(run, ms);
    const onVisible = () => {
      if (!document.hidden) latest.current();
    };
    document.addEventListener('visibilitychange', onVisible);
    return () => {
      clearInterval(timer);
      document.removeEventListener('visibilitychange', onVisible);
    };
  }, [ms]);
}
