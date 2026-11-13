/**
 * The player's Play now search (ADR 0033), kept app-wide by `LiveSeekProvider`, so it keeps
 * going while they play the computer and they hear when an opponent is found.
 */
import { createContext, useContext } from 'react';

import type { TimeControl } from '../api/types';

export interface LiveSeek {
  /** The clock being searched for, or null. */
  seeking: { timeControl: TimeControl; rated: boolean } | null;
  /** Players looking for a game right now (from the last heartbeat). */
  players: number;
  /** A game found while the player was elsewhere, waiting to be opened. */
  found: string | null;
  error: string | null;
  start(timeControl: TimeControl, rated: boolean): void;
  cancel(): void;
  /** Play a waiting player from the lobby now. */
  join(userId: string): Promise<void>;
  openFound(): void;
  dismissFound(): void;
}

const inert: LiveSeek = {
  seeking: null,
  players: 0,
  found: null,
  error: null,
  start: () => undefined,
  cancel: () => undefined,
  join: async () => undefined,
  openFound: () => undefined,
  dismissFound: () => undefined,
};

export const LiveSeekContext = createContext<LiveSeek>(inert);

export function useLiveSeek(): LiveSeek {
  return useContext(LiveSeekContext);
}
