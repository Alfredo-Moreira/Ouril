import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';

import { useBoardPace } from '../board3d/pace';
import { BoardView } from '../components/BoardView';
import { MoveAnnouncer } from '../components/MoveAnnouncer';
import { useCore } from '../engine/CoreContext';
import type { GameState, MoveEvent, VariantRef } from '../engine/types';
import { buildFrames, frameDelay, type Frame } from '../game/animation';
import { variantRef } from '../game/session';
import { TUTORIAL_STEPS } from '../tutorial/steps';

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

export function Tutorial({ animationSpeed: baseSpeed = 1 }: { animationSpeed?: number }) {
  const { t } = useTranslation();
  const animationSpeed = baseSpeed * useBoardPace();
  const { core } = useCore();
  const [index, setIndex] = useState(0);
  const step = TUTORIAL_STEPS[index]!;
  const variant = useMemo<VariantRef>(() => variantRef(core, step.variant), [core, step.variant]);
  const [state, setState] = useState<GameState>(step.position);
  const [moveNo, setMoveNo] = useState(0);
  const [frame, setFrame] = useState<Frame | null>(null);
  const [events, setEvents] = useState<MoveEvent[] | null>(null);
  const alive = useRef(true);
  const headingRef = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  const reset = (i: number) => {
    setIndex(i);
    setState(TUTORIAL_STEPS[i]!.position);
    setMoveNo(0);
    setFrame(null);
    setEvents(null);
    headingRef.current?.focus();
  };

  const done = moveNo >= step.moves.length;
  const allowed = done || frame ? [] : step.moves[moveNo]!;
  const legal = core.legalMoves(variant, state);
  const playable = allowed === 'any' ? legal : legal.filter((p) => allowed.includes(p));

  const play = async (pit: number) => {
    if (!playable.includes(pit)) return;
    const result = core.applyMove(variant, state, pit);
    for (const f of buildFrames(state, pit, result.events, result.state)) {
      if (!alive.current) return;
      setFrame(f);
      const d = frameDelay(f.kind, animationSpeed);
      if (d > 0) await sleep(d);
    }
    if (!alive.current) return;
    setFrame(null);
    setState(result.state);
    setEvents(result.events);
    setMoveNo((n) => n + 1);
  };

  let text: string;
  if (done) text = t(`tutorial.${step.id}.done`);
  else if (moveNo > 0) text = t(`tutorial.${step.id}.next`);
  else text = t(`tutorial.${step.id}.intro`);

  const last = index === TUTORIAL_STEPS.length - 1;

  return (
    <section className="tutorial">
      <h1>{t('tutorial.title')}</h1>
      <p className="note">
        {t('tutorial.progress', { step: index + 1, total: TUTORIAL_STEPS.length })}
      </p>
      <h2 ref={headingRef} tabIndex={-1}>
        {t(`tutorial.${step.id}.title`)}
      </h2>
      <p className="tutorial__text" aria-live="polite">
        {text}
      </p>
      <div className="stage">
        <BoardView
          pits={frame?.pits ?? state.pits}
          stores={frame?.stores ?? state.stores}
          legal={playable}
          onPlay={(p) => void play(p)}
          highlight={frame && frame.kind !== 'end' ? { pit: frame.pit, kind: frame.kind } : null}
          southLabel={t('game.side.you')}
          northLabel={t('tutorial.opponent')}
          speed={animationSpeed}
        />
      </div>
      <MoveAnnouncer events={events} byHuman />
      <div className="game-actions">
        <button type="button" className="button" onClick={() => reset(index)}>
          {t('tutorial.retry')}
        </button>
        {index > 0 && (
          <button type="button" className="button" onClick={() => reset(index - 1)}>
            {t('tutorial.back')}
          </button>
        )}
        {done && !last && (
          <button type="button" className="button button--primary" onClick={() => reset(index + 1)}>
            {t('tutorial.continue')}
          </button>
        )}
        {done && last && (
          <Link className="button button--primary" to="/">
            {t('tutorial.finish')}
          </Link>
        )}
      </div>
    </section>
  );
}
