/**
 * Forfeiting a saved game (ADR 0022): record it as a loss (record format 2, reason
 * `resigned`), queue it for sync and clear the game in progress. Used by the game screen and by
 * the home page's resume card.
 */
import type { CoreApi, GameRecord } from '../engine/types';
import { saveFinishedGame, type CurrentGame } from '../storage/repo';
import { trackEvent } from '../telemetry';
import { resolveSavedVariant, toForfeitRecord } from './session';

export async function forfeitGame(
  core: Pick<CoreApi, 'coreVersion' | 'variants'>,
  game: CurrentGame,
): Promise<GameRecord> {
  // A game saved under a renamed variant is recorded under its current ID.
  const variant = resolveSavedVariant(core, game.variant) ?? game.variant;
  const record = toForfeitRecord(core, { ...game, variant });
  await saveFinishedGame(record);
  trackEvent('game_finished', { level: game.level, outcome: 'loss', plies: game.moves.length });
  return record;
}
