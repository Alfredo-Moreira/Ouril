/**
 * Rating pools (ADR 0034): `overall`, or `<variant_id>:<mode>`. Only turn-by-turn games
 * exist so far, so the boards offer the overall pool and each variant's `async` pool.
 */
import { RATING_POOL_OVERALL } from '../api/types';
import { PLAYABLE_VARIANTS } from '../game/session';

export type RatingMode = 'async' | 'live';

/** The modes players can be rated in today (`live` comes with live games). */
export const RATED_MODES: readonly RatingMode[] = ['async'];

/** Every pool a board can show: overall first, then each variant and mode. */
export const BOARD_POOLS: readonly string[] = [
  RATING_POOL_OVERALL,
  ...PLAYABLE_VARIANTS.flatMap((v) => RATED_MODES.map((m) => `${v}:${m}`)),
];

/** A pool's display name: "Overall", or "Standard · Turn by turn". */
export function poolLabel(t: (key: string) => string, pool: string): string {
  if (pool === RATING_POOL_OVERALL) return t('leaderboard.pool.overall');
  const [variant, mode] = pool.split(':');
  return `${t(`variant.${variant}.short`)} · ${t(`leaderboard.mode.${mode}`)}`;
}
