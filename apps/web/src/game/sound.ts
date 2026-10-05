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

export function playFrameSound(kind: FrameKind): void {
  switch (kind) {
    case 'sow':
      tone(520 + Math.random() * 80, 70, 'triangle');
      break;
    case 'capture':
      tone(330, 180, 'sine', 0.12);
      break;
    case 'grand_slam':
      tone(660, 300, 'square', 0.06);
      break;
    case 'collect':
      tone(260, 220);
      break;
    default:
      break;
  }
}

export function playEndSound(win: boolean): void {
  tone(win ? 784 : 220, 400, 'sine', 0.1);
}
