/**
 * Engine types, re-exported from the generated WASM package (core/wasm/API.md). The web app
 * contains no rule logic: every rule question is answered by these functions.
 */
import type * as wasm from 'ouril-wasm';

export type {
  EndReason,
  GameRecord,
  GameResult,
  GameState,
  Level,
  MoveEvent,
  MoveResult,
  Player,
  Replay,
  Stats,
  Status,
  VariantConfig,
  VariantRef,
  WinLoss,
} from 'ouril-wasm';

/** The subset of the WASM API the UI uses. Tests can pass a fake or the real module. */
export type CoreApi = Pick<
  typeof wasm,
  | 'coreVersion'
  | 'variants'
  | 'defaultVariant'
  | 'newGame'
  | 'legalMoves'
  | 'applyMove'
  | 'aiMove'
  | 'replay'
  | 'deriveStats'
>;

export const LEVELS = ['easy', 'medium', 'hard'] as const;
