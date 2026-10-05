/**
 * Game-music preferences, per device (kv `music`): on or off, and the volume. Not part of the
 * synced settings: like a device's volume, it's local. The game's pause button and Settings
 * both change `on`.
 */
import { useCallback, useSyncExternalStore } from 'react';

import { getKv, setKv } from '../storage/repo';

export interface MusicPrefs {
  on: boolean;
  /** 0..1 */
  volume: number;
}

export const DEFAULT_MUSIC: MusicPrefs = { on: true, volume: 0.5 };

const KEY = 'music';
let current: MusicPrefs = DEFAULT_MUSIC;
let loaded = false;
const listeners = new Set<() => void>();
const notify = () => listeners.forEach((l) => l());

function subscribe(fn: () => void) {
  listeners.add(fn);
  if (!loaded) {
    loaded = true;
    void getKv<Partial<MusicPrefs> & { game?: boolean }>(KEY).then((p) => {
      // Earlier builds stored `{ site, game, volume }`: `game` was this switch.
      current = { ...DEFAULT_MUSIC, ...p, on: p?.on ?? p?.game ?? DEFAULT_MUSIC.on };
      notify();
    });
  }
  return () => listeners.delete(fn);
}

export function useMusicPrefs(): [MusicPrefs, (patch: Partial<MusicPrefs>) => Promise<void>] {
  const prefs = useSyncExternalStore(
    subscribe,
    () => current,
    () => current,
  );
  const update = useCallback(async (patch: Partial<MusicPrefs>) => {
    current = { ...current, ...patch };
    notify();
    await setKv(KEY, { on: current.on, volume: current.volume });
  }, []);
  return [prefs, update];
}
