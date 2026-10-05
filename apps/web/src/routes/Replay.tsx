import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useParams } from 'react-router';

import { BoardView } from '../components/BoardView';
import { useCore } from '../engine/CoreContext';
import type { Replay } from '../engine/types';
import { describeEvents } from '../game/describeEvents';
import type { GameRow } from '../storage/db';
import { getGame } from '../storage/repo';
import { useApp } from '../state/useApp';

/**
 * Step through a finished game move by move. The positions come from the engine replaying the
 * stored move list (versioning.md: `variant@version` + moves rebuild every position), so the
 * web app never re-implements a rule.
 */
export function ReplayRoute() {
  const { t } = useTranslation();
  const { id = '' } = useParams();
  const { dataVersion } = useApp();
  const [row, setRow] = useState<GameRow | null | undefined>(undefined);

  useEffect(() => {
    let cancelled = false;
    void getGame(id).then((r) => {
      if (!cancelled) setRow(r ?? null);
    });
    return () => {
      cancelled = true;
    };
  }, [id, dataVersion]);

  if (row === undefined) return <p role="status">{t('common.loading')}</p>;
  if (row === null)
    return (
      <section>
        <h1>{t('replay.title')}</h1>
        <p>{t('replay.not_found')}</p>
        <Link className="button" to="/stats">
          {t('replay.back')}
        </Link>
      </section>
    );
  return <ReplayView row={row} />;
}

function ReplayView({ row }: { row: GameRow }) {
  const { t } = useTranslation();
  const { core } = useCore();
  const { record } = row;
  const [ply, setPly] = useState(0);

  const replay = useMemo<Replay | null>(() => {
    try {
      return core.replay(record.variant, record.first_player, record.moves);
    } catch {
      return null;
    }
  }, [core, record]);

  if (!replay) {
    return (
      <section>
        <h1>{t('replay.title')}</h1>
        <p className="error" role="alert">
          {t('replay.unavailable')}
        </p>
        <Link className="button" to="/stats">
          {t('replay.back')}
        </Link>
      </section>
    );
  }

  const total = replay.steps.length;
  const state = ply === 0 ? replay.initial : replay.steps[ply - 1]!.state;
  const human = record.human_player ?? 'south';
  const you = t('game.side.you');
  const ai = record.ai_level
    ? t('game.side.ai', { level: t(`game.level.${record.ai_level}`) })
    : t('replay.opponent');
  // The previous position decides who played the move shown at `ply`.
  const mover =
    ply === 0 ? null : (ply === 1 ? replay.initial : replay.steps[ply - 2]!.state).to_move;
  const lines = ply === 0 ? [] : describeEvents(t, replay.steps[ply - 1]!.events, mover === human);

  return (
    <section className="replay">
      <h1>{t('replay.title')}</h1>
      <p className="note">
        {t('replay.ply', { ply, total })}
        {ply > 0 && ` · ${t('replay.pit', { pit: record.moves[ply - 1]! + 1 })}`}
      </p>
      <div className="stage">
        <BoardView
          pits={state.pits}
          stores={state.stores}
          legal={[]}
          southLabel={human === 'south' ? you : ai}
          northLabel={human === 'south' ? ai : you}
        />
      </div>
      <p role="status" aria-live="polite" className="replay__events">
        {ply === 0 ? t('replay.start') : lines.join(' ')}
      </p>
      <div className="row" role="group" aria-label={t('replay.controls')}>
        <button type="button" className="button" disabled={ply === 0} onClick={() => setPly(0)}>
          {t('replay.first')}
        </button>
        <button
          type="button"
          className="button"
          disabled={ply === 0}
          onClick={() => setPly((p) => Math.max(0, p - 1))}
        >
          {t('replay.prev')}
        </button>
        <button
          type="button"
          className="button button--primary"
          disabled={ply === total}
          onClick={() => setPly((p) => Math.min(total, p + 1))}
        >
          {t('replay.next')}
        </button>
        <button
          type="button"
          className="button"
          disabled={ply === total}
          onClick={() => setPly(total)}
        >
          {t('replay.last')}
        </button>
      </div>
      <Link to="/stats">{t('replay.back')}</Link>
    </section>
  );
}
