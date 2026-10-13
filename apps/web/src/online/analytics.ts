/**
 * Usage-statistics events for games against other players (ADR 0026, ADR 0031). They say what
 * happened, never who: no account, handle, game ID or invite code. Like every event, they're
 * sent only after the player opted in (`trackEvent` is a no-op otherwise).
 */
import type { OnlineGame } from '../api/types';
import { trackEvent } from '../telemetry';
import { resultFor } from './perspective';

/** How an invitation was made. */
export type InviteVia = 'handle' | 'link' | 'opponent';

export function trackInvite(via: InviteVia): void {
  trackEvent('online_invite', { via });
}

export function trackInviteAnswer(answer: 'accept' | 'decline' | 'cancel'): void {
  trackEvent('online_invite_answer', { answer });
}

/** An invite link was used to join a game. */
export function trackJoin(): void {
  trackEvent('online_join');
}

/** A game ended while this player had it open (each player's app counts its own result). */
export function trackOnlineFinished(game: OnlineGame): void {
  if (!game.result) return;
  trackEvent('online_game_finished', {
    outcome: resultFor(game.result.outcome, game.you),
    reason: game.result.reason,
    plies: game.moves.length,
  });
}
