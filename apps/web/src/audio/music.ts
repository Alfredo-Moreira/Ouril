/**
 * Game music: a random playlist of the MP3s in public/audio/game/ (`virtual:music-tracks`),
 * played only while a game is on screen. One <audio> element at a time; skipping or switching
 * fades over 0.7 s.
 *
 * - Starts only after the visitor's first interaction (browsers block autoplay with sound).
 * - When a track ends, a different random one follows. A file that fails to load is dropped
 *   from the playlist for this session and the next one plays; with none left, it's silent.
 * - Pauses while the tab is hidden.
 * - Offline: `cacheMusic` downloads the tracks into the service worker's music cache while the
 *   browser is idle (vite.config.ts).
 * - Does nothing where media isn't really available (tests run in jsdom).
 *
 * The UI only uses `setMusic`, `skipTrack`, `hasMusic` and `cacheMusic`, so another source
 * (a streaming service, later) can sit behind the same functions.
 */
import tracks from 'virtual:music-tracks';

import { nextTrack } from './playlist';

const FADE_MS = 700;

const supported =
  typeof window !== 'undefined' &&
  typeof window.Audio === 'function' &&
  !/jsdom/i.test(navigator.userAgent);

const playlist: string[] = [...tracks];
let index: number | null = null;
let audio: HTMLAudioElement | null = null;
let wanted = false;
let volume = 0.5;
let unlocked = false;
let hidden = typeof document !== 'undefined' && document.visibilityState === 'hidden';

/** Whether there is any game music to play. */
export function hasMusic(): boolean {
  return playlist.length > 0;
}

function fadeTo(a: HTMLAudioElement, to: number, then?: () => void) {
  const from = a.volume;
  const start = performance.now();
  const step = (now: number) => {
    const t = Math.min(1, (now - start) / FADE_MS);
    a.volume = Math.max(0, Math.min(1, from + (to - from) * t));
    if (t < 1) requestAnimationFrame(step);
    else then?.();
  };
  requestAnimationFrame(step);
}

/** Fade out and drop the current element. */
function stopCurrent() {
  const old = audio;
  audio = null;
  if (old) fadeTo(old, 0, () => old.pause());
}

/** Start the track at `index` (fading in). */
function start() {
  if (index === null || !playlist[index]) return;
  const src = playlist[index]!;
  const a = new Audio(src);
  a.volume = 0;
  a.addEventListener('ended', () => {
    if (audio === a) skipTrack();
  });
  a.addEventListener('error', () => {
    // Missing or unplayable: drop it and move on.
    const i = playlist.indexOf(src);
    if (i >= 0) playlist.splice(i, 1);
    if (audio === a) {
      audio = null;
      index = null;
      apply();
    }
  });
  audio = a;
  a.play().catch(() => {
    // Blocked (no interaction yet) or unplayable: the error handler covers the latter.
  });
  fadeTo(a, volume);
}

function apply() {
  if (!supported) return;
  const shouldPlay = wanted && unlocked && !hidden && playlist.length > 0;
  if (!shouldPlay) {
    if (audio) {
      const a = audio;
      fadeTo(a, 0, () => a.pause());
    }
    return;
  }
  if (audio) {
    // Resume (after a pause or a hidden tab) and follow volume changes.
    if (audio.paused) void audio.play().catch(() => {});
    fadeTo(audio, volume);
    return;
  }
  index = nextTrack(playlist.length, index);
  start();
}

/** Play game music or not (null/false = silence), at this volume (0..1). */
export function setMusic(on: boolean, vol: number) {
  wanted = on;
  volume = vol;
  apply();
}

/** Skip to another random track (no-op with fewer than two tracks or when not playing). */
export function skipTrack() {
  if (!supported || !wanted || playlist.length === 0) return;
  stopCurrent();
  index = nextTrack(playlist.length, index);
  if (unlocked && !hidden) start();
}

let cached = false;

/**
 * Download every game track into the offline music cache while the browser is idle.
 * Production only (the service worker isn't registered in development). Same-origin static
 * files, like the app's own code: no API call.
 */
export function cacheMusic() {
  if (cached || !supported || !navigator.serviceWorker?.controller || playlist.length === 0) return;
  cached = true;
  const run = () => {
    for (const url of playlist) void fetch(url).catch(() => {});
  };
  if ('requestIdleCallback' in window) window.requestIdleCallback(run, { timeout: 10_000 });
  else setTimeout(run, 3000);
}

if (supported) {
  const unlock = () => {
    if (unlocked) return;
    unlocked = true;
    apply();
    window.removeEventListener('pointerdown', unlock);
    window.removeEventListener('keydown', unlock);
  };
  window.addEventListener('pointerdown', unlock);
  window.addEventListener('keydown', unlock);
  document.addEventListener('visibilitychange', () => {
    hidden = document.visibilityState === 'hidden';
    apply();
  });
}
