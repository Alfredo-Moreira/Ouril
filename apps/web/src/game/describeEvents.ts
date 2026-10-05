import type { MoveEvent } from '../engine/types';

/** Turns a move's events into short sentences (shown, and read by screen readers). */
export function describeEvents(
  t: (key: string, opts?: Record<string, unknown>) => string,
  events: MoveEvent[],
  byHuman: boolean,
): string[] {
  const lines: string[] = [];
  const captured = events.reduce((sum, e) => (e.type === 'capture' ? sum + e.seeds : sum), 0);
  if (events.some((e) => e.type === 'skip_origin')) lines.push(t('game.event.skip_origin'));
  const relays = events.filter((e) => e.type === 'relay').length;
  if (relays > 0) lines.push(t('game.event.relay', { count: relays }));
  if (captured > 0)
    lines.push(t(byHuman ? 'game.captured' : 'game.ai_captured', { count: captured }));
  if (events.some((e) => e.type === 'grand_slam')) lines.push(t('game.event.grand_slam'));
  if (events.some((e) => e.type === 'extra_turn'))
    lines.push(t(byHuman ? 'game.event.extra_turn_you' : 'game.event.extra_turn_ai'));
  return lines;
}
