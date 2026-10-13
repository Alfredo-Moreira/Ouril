import type { MoveEvent } from '../engine/types';

/**
 * Turns a move's events into short sentences (shown, and read by screen readers). The other
 * side is the computer, or, when `opponent` is given, that player (online games).
 */
export function describeEvents(
  t: (key: string, opts?: Record<string, unknown>) => string,
  events: MoveEvent[],
  byHuman: boolean,
  opponent?: string,
): string[] {
  const lines: string[] = [];
  const captured = events.reduce((sum, e) => (e.type === 'capture' ? sum + e.seeds : sum), 0);
  if (events.some((e) => e.type === 'skip_origin')) lines.push(t('game.event.skip_origin'));
  const relays = events.filter((e) => e.type === 'relay').length;
  if (relays > 0) lines.push(t('game.event.relay', { count: relays }));
  if (captured > 0)
    lines.push(
      byHuman
        ? t('game.captured', { count: captured })
        : opponent !== undefined
          ? t('online.event.captured', { count: captured, name: opponent })
          : t('game.ai_captured', { count: captured }),
    );
  if (events.some((e) => e.type === 'grand_slam')) lines.push(t('game.event.grand_slam'));
  if (events.some((e) => e.type === 'extra_turn'))
    lines.push(
      byHuman
        ? t('game.event.extra_turn_you')
        : opponent !== undefined
          ? t('online.event.extra_turn', { name: opponent })
          : t('game.event.extra_turn_ai'),
    );
  return lines;
}
