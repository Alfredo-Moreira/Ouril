/**
 * A game vs AI as plain data plus pure helpers. Every rule question goes to the engine
 * (`newGame`, `legalMoves`, `applyMove`, `replay`); this file only tracks moves and metadata.
 */
import type {
  CoreApi,
  EndReason,
  GameRecord,
  GameResult,
  GameState,
  Level,
  MoveEvent,
  MoveResult,
  Player,
  VariantRef,
} from '../engine/types';
import type { CurrentGame } from '../storage/repo';
import { coinFlip, randomSeed, uuidv7 } from '../util/uuid';

export type Starter = 'me' | 'ai' | 'random';

export interface NewGameOptions {
  level: Level;
  starter: Starter;
  /** Variant ID (latest version is used); absent = the Cape Verde default. */
  variant?: string;
}

/** The Cape Verdean variants players can choose when starting a game, default first. */
export const PLAYABLE_VARIANTS = [
  'cv.standard',
  'cv.continuous',
  'cv.across',
  'cv.across-continuous',
] as const;
export type PlayableVariant = (typeof PLAYABLE_VARIANTS)[number];

/** The local player always sits South (bottom of the board) in vs-AI games. */
export const HUMAN: Player = 'south';
export const AI: Player = 'north';

export function opponent(p: Player): Player {
  return p === 'south' ? 'north' : 'south';
}

/** The default Cape Verde variant, latest version. */
export function defaultVariantRef(core: Pick<CoreApi, 'defaultVariant'>): VariantRef {
  const v = core.defaultVariant('cv');
  if (!v) throw new Error('unknown_variant');
  return { id: v.id, version: v.version };
}

/** Variant IDs renamed before release, old → current (same rules; ADR 0023 note). */
const RENAMED_VARIANTS: Record<string, string> = { 'cv.santiago-relay': 'cv.continuous' };

/**
 * The variant a saved game should be played with: its own `id@version` if this core knows
 * it, the renamed ID if it was renamed before release, otherwise `null` (the game can't be
 * played here, e.g. from a newer app).
 */
export function resolveSavedVariant(
  core: Pick<CoreApi, 'variants'>,
  ref: VariantRef,
): VariantRef | null {
  const known = (id: string) =>
    core.variants().some((v) => v.id === id && v.version === ref.version);
  if (known(ref.id)) return ref;
  const renamed = RENAMED_VARIANTS[ref.id];
  if (renamed && known(renamed)) return { id: renamed, version: ref.version };
  return null;
}

/** Latest version of a variant by ID; the Cape Verde default when it's absent or unknown. */
export function variantRef(
  core: Pick<CoreApi, 'defaultVariant' | 'variants'>,
  id: string | undefined,
): VariantRef {
  if (id) {
    const latest = core
      .variants()
      .filter((v) => v.id === id)
      .sort((a, b) => b.version - a.version)[0];
    if (latest) return { id: latest.id, version: latest.version };
  }
  return defaultVariantRef(core);
}

export function startGame(
  core: Pick<CoreApi, 'defaultVariant' | 'variants' | 'newGame'>,
  { level, starter, variant: variantId }: NewGameOptions,
  random: { coinFlip: () => boolean; seed: () => number } = { coinFlip, seed: randomSeed },
  now: Date = new Date(),
): CurrentGame {
  const variant = variantRef(core, variantId);
  const humanFirst = starter === 'me' || (starter === 'random' && random.coinFlip());
  const firstPlayer: Player = humanFirst ? HUMAN : AI;
  return {
    id: uuidv7(now.getTime()),
    variant,
    firstPlayer,
    humanPlayer: HUMAN,
    level,
    moves: [],
    state: core.newGame(variant, firstPlayer),
    aiSeed: random.seed(),
    startedAt: now.toISOString(),
  };
}

export function isOver(state: GameState): boolean {
  return state.status !== 'playing';
}

/** Apply `pit` for whoever is to move. Throws the engine's error code if illegal. */
export function playMove(
  core: Pick<CoreApi, 'applyMove'>,
  game: CurrentGame,
  pit: number,
): { game: CurrentGame; result: MoveResult } {
  const result = core.applyMove(game.variant, game.state, pit);
  return { game: { ...game, moves: [...game.moves, pit], state: result.state }, result };
}

/** Deterministic AI seed for the next ply (reproducible games for bug reports). */
export function aiSeedFor(game: CurrentGame): number {
  return (game.aiSeed + game.moves.length) % Number.MAX_SAFE_INTEGER;
}

/**
 * Undo back to before the local player's last move (also removing the AI replies after it).
 * Returns null when there is nothing to undo. Rebuilds the position with the engine's `replay`.
 */
export function undoLastHumanMove(
  core: Pick<CoreApi, 'replay'>,
  game: CurrentGame,
): CurrentGame | null {
  if (game.moves.length === 0) return null;
  const rep = core.replay(game.variant, game.firstPlayer, game.moves);
  // Who moved at ply i: the player to move in the position before it.
  const moverAt = (i: number): Player => (i === 0 ? rep.initial : rep.steps[i - 1]!.state).to_move;
  let ply = game.moves.length - 1;
  while (ply >= 0 && moverAt(ply) !== game.humanPlayer) ply--;
  if (ply < 0) return null;
  const state = ply === 0 ? rep.initial : rep.steps[ply - 1]!.state;
  return { ...game, moves: game.moves.slice(0, ply), state };
}

export function gameOverEvent(
  events: MoveEvent[],
): Extract<MoveEvent, { type: 'game_over' }> | undefined {
  return events.find((e): e is Extract<MoveEvent, { type: 'game_over' }> => e.type === 'game_over');
}

/** Find how a finished game ended by replaying it (for games resumed after the last move). */
export function endReason(core: Pick<CoreApi, 'replay'>, game: CurrentGame): EndReason | undefined {
  const rep = core.replay(game.variant, game.firstPlayer, game.moves);
  const last = rep.steps[rep.steps.length - 1];
  return last ? gameOverEvent(last.events)?.reason : undefined;
}

export function resultOf(state: GameState): GameResult | null {
  return state.status === 'playing' ? null : state.status;
}

/** The finished-game record (core/wasm/API.md, format 1). */
export function toRecord(
  core: Pick<CoreApi, 'coreVersion'>,
  game: CurrentGame,
  reason: EndReason,
  endedAt: Date = new Date(),
): GameRecord {
  const outcome = resultOf(game.state);
  if (!outcome) throw new Error('game_not_over');
  return {
    format: 1,
    id: game.id,
    variant: game.variant,
    first_player: game.firstPlayer,
    moves: game.moves,
    core_version: core.coreVersion(),
    result: { outcome, stores: game.state.stores, reason },
    started_at: game.startedAt,
    ended_at: endedAt.toISOString(),
    mode: 'vs_ai',
    ai_level: game.level,
    human_player: game.humanPlayer,
  };
}

/**
 * The record of a game the local player forfeited (record format 2, ADR 0022): the opponent
 * wins, the stores stay as they are, and the moves replay to a game still in progress.
 */
export function toForfeitRecord(
  core: Pick<CoreApi, 'coreVersion'>,
  game: CurrentGame,
  endedAt: Date = new Date(),
): GameRecord {
  if (game.state.status !== 'playing') throw new Error('game_over');
  return {
    format: 2,
    id: game.id,
    variant: game.variant,
    first_player: game.firstPlayer,
    moves: game.moves,
    core_version: core.coreVersion(),
    result: {
      outcome: game.humanPlayer === 'south' ? 'north_wins' : 'south_wins',
      stores: game.state.stores,
      reason: 'resigned',
    },
    started_at: game.startedAt,
    ended_at: endedAt.toISOString(),
    mode: 'vs_ai',
    ai_level: game.level,
    human_player: game.humanPlayer,
  };
}

/** From the local player's point of view. */
export function outcomeFor(result: GameResult, me: Player): 'win' | 'loss' | 'draw' {
  if (result === 'draw') return 'draw';
  return (result === 'south_wins') === (me === 'south') ? 'win' : 'loss';
}
