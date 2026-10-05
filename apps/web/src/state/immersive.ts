/**
 * Whether a screen has taken the whole viewport (no header or tab bar). Only the game view
 * does, and only while a game is on screen: the empty "New game" screen keeps the usual
 * layout and navigation.
 */
import { useEffect, useSyncExternalStore } from 'react';

let holders = 0;
const listeners = new Set<() => void>();
const notify = () => listeners.forEach((l) => l());

function subscribe(fn: () => void) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

/** True while some mounted screen has asked for the whole viewport. */
export function useImmersive(): boolean {
  return useSyncExternalStore(
    subscribe,
    () => holders > 0,
    () => false,
  );
}

/** Call from a screen that takes the whole viewport while it's mounted. */
export function useImmersiveScreen(): void {
  useEffect(() => {
    holders++;
    notify();
    return () => {
      holders--;
      notify();
    };
  }, []);
}
