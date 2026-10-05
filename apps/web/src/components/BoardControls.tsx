/**
 * The 3D board's keyboard and screen-reader interface (ADR 0021): the pits and stores as
 * visually hidden controls. Nothing here is drawn. Focusing a pit lights the same pit in the
 * 3D view, and Enter or Space plays it.
 */
import { useTranslation } from 'react-i18next';
import type { KeyboardEvent } from 'react';

import type { FrameKind } from '../game/animation';

/** What every board screen passes to the board (pit numbering: docs/game/rules.md). */
export interface BoardProps {
  pits: number[];
  stores: [number, number];
  /** Pits the local player may play now (empty when it isn't their turn). */
  legal: readonly number[];
  onPlay?: (pit: number) => void;
  /** Pit + frame kind being animated. */
  highlight?: { pit?: number; kind: FrameKind } | null;
  hintPit?: number | null;
  /** Labels for the two sides (e.g. "You" / "Computer"). */
  southLabel: string;
  northLabel: string;
  /** Keyboard focus moved to a pit (or left the board): the 3D view mirrors it. */
  onFocusPit?: (pit: number | null) => void;
}

export function BoardControls({
  pits,
  stores,
  legal,
  onPlay,
  hintPit,
  southLabel,
  northLabel,
  onFocusPit,
}: BoardProps) {
  const { t } = useTranslation();
  const n = pits.length / 2;

  const pit = (index: number) => {
    const count = pits[index] ?? 0;
    const south = index < n;
    const number = south ? index + 1 : index - n + 1;
    const label = t(south ? 'board.pit.yours' : 'board.pit.opponent', { number, count });
    if (!south || !onPlay) {
      return (
        <li key={index} role="img" aria-label={label}>
          {label}
        </li>
      );
    }
    const isLegal = legal.includes(index);
    const hinted = hintPit === index;
    const playable = hinted ? t('board.pit.hint', { label }) : label;
    const activate = () => {
      if (isLegal) onPlay(index);
    };
    const onKeyDown = (e: KeyboardEvent<HTMLLIElement>) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        activate();
      }
    };
    return (
      <li
        key={index}
        role="button"
        tabIndex={isLegal ? 0 : -1}
        aria-label={isLegal ? playable : t('board.pit.not_playable', { label })}
        data-pit={index}
        data-hint={hinted || undefined}
        aria-disabled={!isLegal}
        onClick={activate}
        onKeyDown={onKeyDown}
        onFocus={() => onFocusPit?.(index)}
        onBlur={() => onFocusPit?.(null)}
      >
        {label}
      </li>
    );
  };

  return (
    <ul className="visually-hidden" role="group" aria-label={t('board.label')}>
      <li role="img" aria-label={t('board.store', { name: northLabel, count: stores[1] })} />
      {Array.from({ length: n }, (_, c) => pit(2 * n - 1 - c))}
      {Array.from({ length: n }, (_, c) => pit(c))}
      <li role="img" aria-label={t('board.store', { name: southLabel, count: stores[0] })} />
    </ul>
  );
}
