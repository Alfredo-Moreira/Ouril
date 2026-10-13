import type { OnlineGame } from '../api/types';

/**
 * The fresher of two copies of the same game. Answers can arrive out of order (a poll that
 * started before your move can land after it), and an older copy would undo the move on
 * screen. A different game (after a rematch) always wins.
 */
export function newer(prev: OnlineGame | null, next: OnlineGame): OnlineGame {
  if (!prev || prev.id !== next.id) return next;
  if (next.moves.length < prev.moves.length) return prev;
  return Date.parse(next.updated_at) < Date.parse(prev.updated_at) ? prev : next;
}
