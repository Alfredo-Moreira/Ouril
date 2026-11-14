/**
 * A player's nameplate on the game screen: a seed-shaped avatar with the initial, the name, the
 * score, and the clock in live games. The opponent's sits above the board and yours below it.
 * Whose turn it is shows as the plate's outline; the words for it stay in the screen's one
 * status line (`role="status"`), so it's announced once.
 */
import type { ReactNode } from 'react';

import { ScoreValue } from './GameFx';

export function Nameplate({
  side,
  name,
  score,
  turn,
  clock,
}: {
  side: 'you' | 'them';
  name: string;
  score: number;
  /** This player is to move. */
  turn: boolean;
  /** A live game's clock (`role="timer"`). */
  clock?: ReactNode;
}) {
  return (
    <div className={`plate plate--${side}${turn ? ' plate--turn' : ''}`}>
      <span className="plate__avatar" aria-hidden="true">
        {Array.from(name)[0]}
      </span>
      <span className="plate__name">{name}</span>
      <ScoreValue value={score} />
      {clock}
    </div>
  );
}
