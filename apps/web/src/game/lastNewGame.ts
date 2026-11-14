/**
 * The choices of the last game started from the new-game form, remembered on this device. The
 * form opens with them, and the home page's Play now starts a game with them at once.
 */
import type { NewGameRequest } from '../components/NewGameForm';
import { LEVELS, type Level } from '../engine/types';
import { PLAYABLE_VARIANTS, type PlayableVariant, type Starter } from './session';

export const STARTERS: Starter[] = ['me', 'ai', 'random'];

const KEY = 'ouril_new_game';

const DEFAULTS: NewGameRequest = { level: 'easy', starter: 'random', variant: 'cv.standard' };

/** The last choices, or the defaults (nothing saved, storage unavailable, or a stale value). */
export function lastNewGame(): NewGameRequest {
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? 'null') as Partial<NewGameRequest>;
    if (!saved || typeof saved !== 'object') return DEFAULTS;
    return {
      level: LEVELS.includes(saved.level as Level) ? (saved.level as Level) : DEFAULTS.level,
      starter: STARTERS.includes(saved.starter as Starter)
        ? (saved.starter as Starter)
        : DEFAULTS.starter,
      variant: PLAYABLE_VARIANTS.includes(saved.variant as PlayableVariant)
        ? saved.variant
        : DEFAULTS.variant,
    };
  } catch {
    return DEFAULTS;
  }
}

export function rememberNewGame(request: NewGameRequest) {
  try {
    localStorage.setItem(KEY, JSON.stringify(request));
  } catch {
    // Storage unavailable: nothing to remember.
  }
}
