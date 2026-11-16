/**
 * Tiny synthesized sound effects (Web Audio, no asset downloads). Muted via Settings.
 */
import type { FrameKind } from './animation';

let ctx: AudioContext | null = null;

function audio(): AudioContext | null {
  if (typeof window === 'undefined' || typeof window.AudioContext === 'undefined') return null;
  ctx ??= new window.AudioContext();
  if (ctx.state === 'suspended') void ctx.resume();
  return ctx;
}

function tone(freq: number, durationMs: number, type: OscillatorType = 'sine', gain = 0.08): void {
  const ac = audio();
  if (!ac) return;
  const osc = ac.createOscillator();
  const amp = ac.createGain();
  osc.type = type;
  osc.frequency.value = freq;
  amp.gain.setValueAtTime(gain, ac.currentTime);
  amp.gain.exponentialRampToValueAtTime(0.0001, ac.currentTime + durationMs / 1000);
  osc.connect(amp).connect(ac.destination);
  osc.start();
  osc.stop(ac.currentTime + durationMs / 1000);
}

/** A tone: `freq` Hz (plus up to `jitter` Hz at random), fading out over `ms`. */
export interface Tone {
  freq: number;
  jitter: number;
  ms: number;
  wave: OscillatorType;
  gain: number;
}

/** The sound of each frame kind, played as the frame starts. */
export const FRAME_TONES: Partial<Record<FrameKind, Tone>> = {
  sow: { freq: 520, jitter: 80, ms: 70, wave: 'triangle', gain: 0.08 },
  capture: { freq: 330, jitter: 0, ms: 180, wave: 'sine', gain: 0.12 },
  grand_slam: { freq: 660, jitter: 0, ms: 300, wave: 'square', gain: 0.06 },
  collect: { freq: 260, jitter: 0, ms: 220, wave: 'sine', gain: 0.08 },
};

/** The sound of the game's end, for the player. */
export const END_TONES: Record<'win' | 'loss', Tone> = {
  win: { freq: 784, jitter: 0, ms: 400, wave: 'sine', gain: 0.1 },
  loss: { freq: 220, jitter: 0, ms: 400, wave: 'sine', gain: 0.1 },
};

function play(t: Tone): void {
  tone(t.freq + (t.jitter ? Math.random() * t.jitter : 0), t.ms, t.wave, t.gain);
}

export function playFrameSound(kind: FrameKind): void {
  const t = FRAME_TONES[kind];
  if (t) play(t);
}

export function playEndSound(win: boolean): void {
  play(END_TONES[win ? 'win' : 'loss']);
}
