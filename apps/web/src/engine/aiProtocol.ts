import type { GameState, Level, VariantRef } from './types';

export interface AiRequest {
  id: number;
  variant: VariantRef;
  state: GameState;
  level: Level;
  seed: number;
}

export type AiResponse = { id: number; pit: number | null } | { id: number; error: string };
