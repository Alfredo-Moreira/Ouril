import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useParams } from 'react-router';

import { ApiError } from '../api/client';
import type { OnlineGame as Game } from '../api/types';
import { BoardView } from '../components/BoardView';
import { useCore } from '../engine/CoreContext';
import type { GameState, VariantRef } from '../engine/types';
import { displayPosition, flipPit, resultFor } from '../online/perspective';
import { usePolling } from '../online/usePolling';
import { useApp } from '../state/useApp';

/** While waiting for the opponent's move: how often to ask the server. */
const WAITING_POLL_MS = 2_000;
/** On your turn or before the game starts: only to notice a resign, timeout or acceptance. */
const IDLE_POLL_MS = 15_000;

/**
 * `/online/:id`: a game against another player (ADR 0031). The server holds the moves; the
 * engine replays them to draw the board, from your side (your pits along the bottom). You
 * play on your turn; while it's the opponent's turn the page polls for their move.
 */
export function OnlineGame() {
  const { t, i18n } = useTranslation();
  const { id = '' } = useParams();
  const { core } = useCore();
  const { api, notifyLocalChange } = useApp();
  const [game, setGame] = useState<Game | null>(null);
  const [missing, setMissing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirmResign, setConfirmResign] = useState(false);
  const wasFinished = useRef<boolean | null>(null);

  const refresh = useCallback(() => {
    void api()
      .getGame(id)
      .then((g) => {
        setGame(g);
        setError(null);
      })
      .catch((e: unknown) => {
        if (e instanceof ApiError && e.status === 404) setMissing(true);
        else setError(e instanceof ApiError ? e.code : 'network');
      });
  }, [api, id]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // A game that just ended reaches the player's history through sync: ask for one.
  useEffect(() => {
    if (!game) return;
    const finished = game.status === 'finished';
    if (wasFinished.current === false && finished) notifyLocalChange();
    wasFinished.current = finished;
  }, [game, notifyLocalChange]);

  const live = game?.status === 'active' || game?.status === 'invited';
  usePolling(
    refresh,
    !live ? null : game?.status === 'active' && !game.your_turn ? WAITING_POLL_MS : IDLE_POLL_MS,
  );

  const variant: VariantRef | null = game
    ? { id: game.variant_id, version: game.variant_version }
    : null;
  const state = useMemo<GameState | null>(() => {
    if (!game || !variant) return null;
    const replay = core.replay(variant, game.first_player, game.moves);
    return replay.steps.at(-1)?.state ?? replay.initial;
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `variant` follows `game`.
  }, [core, game]);

  const act = async (action: () => Promise<Game>) => {
    setBusy(true);
    setError(null);
    try {
      setGame(await action());
    } catch (e) {
      setError(e instanceof ApiError ? e.code : 'network');
      refresh();
    } finally {
      setBusy(false);
    }
  };

  if (missing) {
    return (
      <section>
        <h1>{t('online.title')}</h1>
        <p>{t('online.game.not_found')}</p>
        <Link to="/online">{t('online.back')}</Link>
      </section>
    );
  }
  if (!game || !state || !variant) return <p role="status">{t('common.loading')}</p>;

  const perSide = state.pits.length / 2;
  const legal =
    game.status === 'active' && game.your_turn && !busy
      ? core.legalMoves(variant, state).map((p) => flipPit(p, game.you, perSide))
      : [];
  const shown = displayPosition(state.pits, state.stores, game.you);
  const opponentName = game.opponent?.display_name ?? t('online.invite_link.anyone');
  const deadline = game.turn_deadline
    ? new Intl.DateTimeFormat(i18n.language, { dateStyle: 'medium', timeStyle: 'short' }).format(
        new Date(game.turn_deadline),
      )
    : null;

  const play = (shownPit: number) =>
    void act(() =>
      api().playOnlineMove(game.id, game.moves.length, flipPit(shownPit, game.you, perSide)),
    );

  return (
    <section className="online-game">
      <h1>{t('online.game.title', { name: opponentName })}</h1>

      <p className="online-game__status" role="status" aria-live="polite">
        {game.status === 'active' &&
          (game.your_turn
            ? t('online.game.your_turn')
            : t('online.game.their_turn', { name: opponentName }))}
        {game.status === 'invited' &&
          (game.invited_by_you
            ? t('online.game.waiting_accept', { name: opponentName })
            : t('online.game.invited_you', { name: opponentName }))}
        {game.status === 'finished' &&
          game.result &&
          t(`online.result.${resultFor(game.result.outcome, game.you)}`, {
            reason: t(`online.reason.${game.result.reason}`, {
              defaultValue: t(`game.over.reason.${game.result.reason}`, { defaultValue: '' }),
            }),
          })}
        {['declined', 'cancelled', 'expired'].includes(game.status) &&
          t(`online.status.${game.status}`)}
      </p>
      {game.status === 'active' && deadline && (
        <p className="note">{t('online.game.deadline', { date: deadline })}</p>
      )}

      <BoardView
        pits={shown.pits}
        stores={shown.stores}
        legal={legal}
        onPlay={play}
        southLabel={t('online.you')}
        northLabel={opponentName}
        finished={game.status === 'finished'}
      />

      {error && (
        <p className="error" role="alert">
          {t(`online.error.${error}`, {
            defaultValue: t(`error.${error}`, { defaultValue: t('error.internal') }),
          })}
        </p>
      )}

      <div className="row">
        {game.status === 'invited' && !game.invited_by_you && (
          <>
            <button
              type="button"
              className="button"
              disabled={busy}
              onClick={() => void act(() => api().gameAction(game.id, 'accept'))}
            >
              {t('online.accept')}
            </button>
            <button
              type="button"
              className="button button--ghost"
              disabled={busy}
              onClick={() => void act(() => api().gameAction(game.id, 'decline'))}
            >
              {t('online.decline')}
            </button>
          </>
        )}
        {game.status === 'active' &&
          (confirmResign ? (
            <div className="confirm" role="group" aria-label={t('online.resign')}>
              <p>{t('online.resign.confirm')}</p>
              <button
                type="button"
                className="button button--danger"
                disabled={busy}
                onClick={() =>
                  void act(() => api().gameAction(game.id, 'resign')).then(() =>
                    setConfirmResign(false),
                  )
                }
              >
                {t('online.resign.yes')}
              </button>
              <button type="button" className="button" onClick={() => setConfirmResign(false)}>
                {t('common.cancel')}
              </button>
            </div>
          ) : (
            <button
              type="button"
              className="button button--ghost"
              onClick={() => setConfirmResign(true)}
            >
              {t('online.resign')}
            </button>
          ))}
        <Link className="button button--ghost" to="/online">
          {t('online.back')}
        </Link>
      </div>
    </section>
  );
}
