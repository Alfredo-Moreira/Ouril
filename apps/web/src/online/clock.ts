/**
 * Live-game clocks (ADR 0033) on the client: the server sends each side's time left when the
 * turn started, plus its own time; the side to move counts down locally from there. The
 * server stays the judge: when a clock reaches zero the screen asks it, and it settles the
 * flag fall.
 */
import type { Clocks, Seat } from '../api/types';

/** Server time minus device time, from an answer's `server_now` and when it arrived. */
export function clockOffset(serverNow: string | undefined, receivedAt: number): number {
  if (!serverNow) return 0;
  const server = Date.parse(serverNow);
  return Number.isNaN(server) ? 0 : server - receivedAt;
}

/** Each side's time left right now (ms), never below zero. */
export function timeLeft(
  clocks: Clocks,
  toMove: Seat,
  deviceNow: number,
  offset: number,
): { south: number; north: number } {
  let south = clocks.south_ms;
  let north = clocks.north_ms;
  if (clocks.running && clocks.turn_started_at) {
    const elapsed = deviceNow + offset - Date.parse(clocks.turn_started_at);
    if (toMove === 'south') south -= elapsed;
    else north -= elapsed;
  }
  return { south: Math.max(0, south), north: Math.max(0, north) };
}

/** `4:05`, or `0:09.4` in the last ten seconds. */
export function formatClock(ms: number): string {
  const total = Math.max(0, ms);
  const minutes = Math.floor(total / 60_000);
  const seconds = Math.floor((total % 60_000) / 1000);
  const base = `${minutes}:${String(seconds).padStart(2, '0')}`;
  return total < 10_000 ? `${base}.${Math.floor((total % 1000) / 100)}` : base;
}
