import { useCallback, useEffect, useState, type RefObject } from 'react';

/**
 * The Fullscreen API for one element. `supported` is false where the browser doesn't allow it
 * for page elements (iPhone Safari): there the game's full-viewport layout is the full screen.
 */
export function useFullscreen(ref: RefObject<HTMLElement | null>) {
  const supported =
    typeof document !== 'undefined' &&
    !!document.fullscreenEnabled &&
    typeof HTMLElement !== 'undefined' &&
    typeof HTMLElement.prototype.requestFullscreen === 'function';
  const [active, setActive] = useState(false);

  useEffect(() => {
    if (!supported) return;
    const onChange = () => setActive(document.fullscreenElement === ref.current);
    document.addEventListener('fullscreenchange', onChange);
    return () => {
      document.removeEventListener('fullscreenchange', onChange);
      // Leaving the game screen also leaves full screen.
      if (document.fullscreenElement) void document.exitFullscreen().catch(() => {});
    };
  }, [supported, ref]);

  const toggle = useCallback(() => {
    if (!supported) return;
    if (document.fullscreenElement) void document.exitFullscreen().catch(() => {});
    else void ref.current?.requestFullscreen({ navigationUI: 'hide' }).catch(() => {});
  }, [supported, ref]);

  return { supported, active, toggle };
}
