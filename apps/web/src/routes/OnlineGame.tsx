import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate, useParams } from 'react-router';

import { ApiError } from '../api/client';
import type { OnlineGame as Game } from '../api/types';
import { hasMusic, skipTrack } from '../audio/music';
import { useMusicPrefs } from '../audio/musicPrefs';
import { useBoardPace } from '../board3d/pace';
import { BoardView } from '../components/BoardView';
import { Dialog } from '../components/Dialog';
import { Celebration, FinalScore, ScoreValue } from '../components/GameFx';
import {
  ExitFullscreenIcon,
  FlagIcon,
  FullscreenIcon,
  HomeIcon,
  InfoIcon,
  MusicIcon,
  MusicOffIcon,
  PauseIcon,
  SeedIcon,
  SkipIcon,
  StarIcon,
} from '../components/Icons';
import { MoveAnnouncer } from '../components/MoveAnnouncer';
import { RulesView } from '../components/RulesView';
import { useFullscreen } from '../components/useFullscreen';
import { useCore } from '../engine/CoreContext';
import type { CoreApi, GameState, MoveEvent, Player, VariantRef } from '../engine/types';
import { buildFrames, defaultSpeed, frameDelay, type Frame } from '../game/animation';
import { PLAYABLE_VARIANTS, type PlayableVariant } from '../game/session';
import { playEndSound, playFrameSound } from '../game/sound';
import { trackInviteAnswer, trackOnlineFinished } from '../online/analytics';
import { newer } from '../online/freshness';
import { displayPosition, flipPit, resultFor } from '../online/perspective';
import { poolLabel } from '../online/pools';
import { usePolling } from '../online/usePolling';
import { useImmersiveScreen } from '../state/immersive';
import { useApp, useStatsVisible } from '../state/useApp';

/** While waiting for the opponent's move: how often to ask the server. */
const WAITING_POLL_MS = 2_000;
/** On your turn or before the game starts: only to notice a resign, timeout or acceptance. */
const IDLE_POLL_MS = 15_000;

export interface OnlineGameProps {
  /** 1 = normal speed, 0 = no animation delays (tests). Defaults from prefers-reduced-motion. */
  animationSpeed?: number;
}

type Act = (action: () => Promise<Game>, onDone?: () => void) => Promise<boolean>;

/**
 * `/online/:id`: a game against another player (ADR 0031). The server holds the moves; the
 * engine replays them to draw the board, from your side (your pits along the bottom). Once the
 * game has started it's the same screen as a game against the computer: the whole viewport,
 * music, every move sown seed by seed with its sounds, the scoreboard, the menu, the rules and
 * the result. Before that (an invitation) it's a plain page to accept or decline.
 */
export function OnlineGame(props: OnlineGameProps) {
  const { id = '' } = useParams();
  // A new game (a rematch) starts from scratch: no moves to animate from the old one.
  return <OnlineGameScreen key={id} id={id} {...props} />;
}

function OnlineGameScreen({ id, animationSpeed }: OnlineGameProps & { id: string }) {
  const { t } = useTranslation();
  const { api, notifyLocalChange } = useApp();
  const [game, setGame] = useState<Game | null>(null);
  const [missing, setMissing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const wasFinished = useRef<boolean | null>(null);
  const [speed] = useState(() => animationSpeed ?? defaultSpeed());

  const refresh = useCallback(() => {
    void api()
      .getGame(id)
      .then((g) => {
        setGame((prev) => newer(prev, g));
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
    if (wasFinished.current === false && finished) {
      notifyLocalChange();
      trackOnlineFinished(game);
    }
    wasFinished.current = finished;
  }, [game, notifyLocalChange]);

  const live = game?.status === 'active' || game?.status === 'invited';
  usePolling(
    refresh,
    !live ? null : game?.status === 'active' && !game.your_turn ? WAITING_POLL_MS : IDLE_POLL_MS,
  );

  const act: Act = async (action, onDone) => {
    setBusy(true);
    setError(null);
    try {
      const next = await action();
      setGame((prev) => newer(prev, next));
      onDone?.();
      return true;
    } catch (e) {
      setError(e instanceof ApiError ? e.code : 'network');
      refresh();
      return false;
    } finally {
      setBusy(false);
    }
  };

  if (missing) {
    return (
      <section>
        <h1>{t('play.title')}</h1>
        <p>{t('online.game.not_found')}</p>
        <Link to="/game?tab=online">{t('online.back')}</Link>
      </section>
    );
  }
  if (!game) return <p role="status">{t('common.loading')}</p>;
  if (game.status === 'active' || game.status === 'finished') {
    return <OnlineGameView game={game} act={act} busy={busy} error={error} speed={speed} />;
  }
  return <Invitation game={game} act={act} busy={busy} error={error} />;
}

/** An invitation (yours or theirs) or a game that never started: a plain page. */
function Invitation({
  game,
  act,
  busy,
  error,
}: {
  game: Game;
  act: Act;
  busy: boolean;
  error: string | null;
}) {
  const { t } = useTranslation();
  const { api } = useApp();
  const opponentName = game.opponent?.display_name ?? t('online.invite_link.anyone');
  return (
    <section className="online-game">
      <h1>{t('online.game.title', { name: opponentName })}</h1>
      <p className="online-game__status" role="status" aria-live="polite">
        {game.status === 'invited' &&
          (game.invited_by_you
            ? t('online.game.waiting_accept', { name: opponentName })
            : t('online.game.invited_you', { name: opponentName }))}
        {['declined', 'cancelled', 'expired'].includes(game.status) &&
          t(`online.status.${game.status}`)}
      </p>
      <ErrorLine error={error} />
      <div className="row">
        {game.status === 'invited' && !game.invited_by_you && (
          <>
            <button
              type="button"
              className="button button--primary"
              disabled={busy}
              onClick={() =>
                void act(
                  () => api().gameAction(game.id, 'accept'),
                  () => trackInviteAnswer('accept'),
                )
              }
            >
              {t('online.accept')}
            </button>
            <button
              type="button"
              className="button button--ghost"
              disabled={busy}
              onClick={() =>
                void act(
                  () => api().gameAction(game.id, 'decline'),
                  () => trackInviteAnswer('decline'),
                )
              }
            >
              {t('online.decline')}
            </button>
          </>
        )}
        <Link className="button button--ghost" to="/game?tab=online">
          {t('online.back')}
        </Link>
      </div>
    </section>
  );
}

function ErrorLine({ error }: { error: string | null }) {
  const { t } = useTranslation();
  if (!error) return null;
  return (
    <p className="error" role="alert">
      {t(`online.error.${error}`, {
        defaultValue: t(`error.${error}`, { defaultValue: t('error.internal') }),
      })}
    </p>
  );
}

/** A game in progress or finished, laid out like a game against the computer. */
function OnlineGameView({
  game,
  act,
  busy,
  error,
  speed: baseSpeed,
}: {
  game: Game;
  act: Act;
  busy: boolean;
  error: string | null;
  speed: number;
}) {
  useImmersiveScreen();
  const { t, i18n } = useTranslation();
  const { core } = useCore();
  const { api, settings } = useApp();
  const statsVisible = useStatsVisible();
  const navigate = useNavigate();
  const speed = baseSpeed * useBoardPace();
  const variant: VariantRef = { id: game.variant_id, version: game.variant_version };

  // Your move is sown at once, while it's on its way to the server (`pending`).
  const [pending, setPending] = useState<number[] | null>(null);
  const target = pending ?? game.moves;
  const anim = useMoveAnimation(core, variant, game.first_player, target, speed, settings.sound);
  const { state, frame } = anim;

  const opponentName = game.opponent?.display_name ?? t('online.invite_link.anyone');
  const finished = game.status === 'finished' && !anim.animating && anim.caughtUp;
  const perSide = state.pits.length / 2;
  const canPlay =
    game.status === 'active' &&
    game.your_turn &&
    pending === null &&
    !busy &&
    !anim.animating &&
    anim.caughtUp;
  const legal = canPlay
    ? core.legalMoves(variant, state).map((p) => flipPit(p, game.you, perSide))
    : [];
  const shown = displayPosition(frame?.pits ?? state.pits, frame?.stores ?? state.stores, game.you);
  const highlight =
    frame && frame.kind !== 'end'
      ? {
          pit: frame.pit === undefined ? undefined : flipPit(frame.pit, game.you, perSide),
          kind: frame.kind,
        }
      : null;

  const play = (shownPit: number) => {
    if (!legal.includes(shownPit)) return;
    const moves = [...game.moves, flipPit(shownPit, game.you, perSide)];
    setPending(moves);
    void act(() => api().playOnlineMove(game.id, game.moves.length, moves.at(-1)!)).then(() =>
      setPending(null),
    );
  };

  const playing = !finished && game.status === 'active';
  const myTurn = playing && state.to_move === game.you;
  let status: string;
  // From the board as shown: your move counts as played as soon as it's sown, and a game
  // that has ended is still "sowing" until its last move has been shown.
  if (finished) status = t('game.status.over');
  else if (anim.animating || game.status === 'finished') status = t('game.status.sowing');
  else if (myTurn && anim.caughtUp && pending === null) status = t('game.status.your_turn');
  else status = t('online.status.their_turn', { name: opponentName });

  const root = useRef<HTMLElement>(null);
  const fullscreen = useFullscreen(root);
  const [music, setMusicPrefs] = useMusicPrefs();
  const [menu, setMenu] = useState<'closed' | 'menu' | 'confirm'>('closed');
  const [rulesOpen, setRulesOpen] = useState(false);
  const [resultOpen, setResultOpen] = useState(true);
  const rulesVariant: PlayableVariant =
    PLAYABLE_VARIANTS.find((v) => v === game.variant_id) ?? PLAYABLE_VARIANTS[0];

  const outcome = game.result ? resultFor(game.result.outcome, game.you) : null;
  // Opened while still being played, so an ending is seen live: celebrate and play the end
  // sound. A game opened after it ended just shows its result.
  const [endedHere] = useState(game.status === 'active');
  // The end sound, once, when the game ends on screen (after the last move is sown).
  const endSounded = useRef(false);
  useEffect(() => {
    if (!finished || !outcome || !endedHere || endSounded.current) return;
    endSounded.current = true;
    if (settings.sound) playEndSound(outcome === 'win');
  }, [finished, outcome, endedHere, settings.sound]);

  const resign = async () => {
    if (await act(() => api().gameAction(game.id, 'resign'))) setMenu('closed');
  };
  const rematch = () => {
    const opponent = game.opponent;
    if (!opponent) return;
    void act(async () => {
      const next = await api().createGame({
        opponent_id: opponent.user_id,
        rated: game.rated !== false,
      });
      navigate(`/online/${next.id}`);
      return next;
    });
  };

  const deadline =
    playing && game.turn_deadline
      ? new Intl.DateTimeFormat(i18n.language, { dateStyle: 'medium', timeStyle: 'short' }).format(
          new Date(game.turn_deadline),
        )
      : null;
  const finalStores = game.result
    ? displayPosition(state.pits, game.result.stores as [number, number], game.you).stores
    : shown.stores;

  return (
    <section className="game" ref={root}>
      <h1 className="visually-hidden">{t('online.game.title', { name: opponentName })}</h1>
      <div className="game__bar">
        <button
          type="button"
          className="icon-button"
          onClick={() => setMenu('menu')}
          aria-label={t('game.menu.open')}
          aria-haspopup="dialog"
        >
          <PauseIcon />
        </button>
        <div className="scoreboard" aria-label={t('game.score.label')}>
          <div className={`score score--ai${playing && !myTurn ? ' score--turn' : ''}`}>
            <span className="score__avatar" aria-hidden="true">
              {Array.from(opponentName)[0]}
            </span>
            <span className="score__name">{opponentName}</span>
            <ScoreValue value={shown.stores[1]} />
          </div>
          <p className="game-status" role="status">
            {status}
          </p>
          <div className={`score score--you${myTurn ? ' score--turn' : ''}`}>
            <span className="score__avatar" aria-hidden="true">
              {Array.from(t('online.you'))[0]}
            </span>
            <span className="score__name">{t('online.you')}</span>
            <ScoreValue value={shown.stores[0]} />
          </div>
        </div>
        <div className="game__tools">
          {hasMusic() && (
            <>
              <button
                type="button"
                className="icon-button"
                onClick={() => void setMusicPrefs({ on: !music.on })}
                aria-label={t(music.on ? 'game.music.pause' : 'game.music.play')}
                aria-pressed={music.on}
              >
                {music.on ? <MusicIcon /> : <MusicOffIcon />}
              </button>
              {music.on && (
                <button
                  type="button"
                  className="icon-button"
                  onClick={skipTrack}
                  aria-label={t('game.music.skip')}
                >
                  <SkipIcon />
                </button>
              )}
            </>
          )}
          <button
            type="button"
            className="icon-button"
            onClick={() => setRulesOpen(true)}
            aria-label={t('game.rules')}
            aria-haspopup="dialog"
          >
            <InfoIcon />
          </button>
          {fullscreen.supported ? (
            <button
              type="button"
              className="icon-button"
              onClick={fullscreen.toggle}
              aria-label={t(fullscreen.active ? 'game.fullscreen.exit' : 'game.fullscreen.enter')}
              aria-pressed={fullscreen.active}
            >
              {fullscreen.active ? <ExitFullscreenIcon /> : <FullscreenIcon />}
            </button>
          ) : null}
        </div>
      </div>

      <div className="stage stage--game">
        <BoardView
          pits={shown.pits}
          stores={shown.stores}
          legal={legal}
          onPlay={play}
          highlight={highlight}
          southLabel={t('online.you')}
          northLabel={opponentName}
          speed={speed}
          finished={game.status === 'finished'}
        />
      </div>

      <MoveAnnouncer
        events={anim.announcement?.events ?? null}
        byHuman={anim.announcement ? anim.announcement.mover === game.you : true}
        opponent={opponentName}
      />
      <ErrorLine error={error} />

      <div className="game-actions">
        {finished ? (
          <>
            {game.opponent && (
              <button
                type="button"
                className="button button--primary"
                onClick={rematch}
                disabled={busy}
              >
                {t('online.over.rematch')}
              </button>
            )}
            <Link className="button button--ghost" to="/game?tab=online">
              {t('online.menu.back')}
            </Link>
          </>
        ) : (
          <button
            type="button"
            className="button button--ghost"
            onClick={() => setMenu('menu')}
            aria-haspopup="dialog"
          >
            <PauseIcon />
            {t('game.leave')}
          </button>
        )}
      </div>

      {rulesOpen && (
        <Dialog
          title={t('game.rules.title', { name: t(`variant.${rulesVariant}.short`) })}
          onDismiss={() => setRulesOpen(false)}
          className="dialog--rules"
        >
          <RulesView variant={rulesVariant} headingLevel={3} />
          <div className="dialog__actions">
            <button
              type="button"
              className="button button--primary"
              onClick={() => setRulesOpen(false)}
            >
              {t('game.rules.close')}
            </button>
            <Link className="button button--ghost" to={`/rules?variant=${rulesVariant}`}>
              {t('game.rules.all')}
            </Link>
          </div>
        </Dialog>
      )}
      {menu === 'menu' && !finished && (
        <Dialog title={t('game.menu.title')} onDismiss={() => setMenu('closed')}>
          <p>{t('online.menu.body')}</p>
          <div className="dialog__actions dialog__actions--stack">
            <button
              type="button"
              className="button button--primary"
              onClick={() => setMenu('closed')}
            >
              {t('game.menu.resume')}
            </button>
            <Link className="button" to="/game?tab=online">
              <HomeIcon />
              {t('online.menu.back')}
            </Link>
            <button
              type="button"
              className="button button--ghost button--danger-text"
              onClick={() => setMenu('confirm')}
              disabled={game.status !== 'active'}
            >
              <FlagIcon />
              {t('online.resign')}
            </button>
          </div>
        </Dialog>
      )}
      {menu === 'confirm' && !finished && (
        <Dialog title={t('online.resign.title')} onDismiss={() => setMenu('menu')}>
          <p>{t('online.resign.body', { name: opponentName })}</p>
          <ErrorLine error={error} />
          <div className="dialog__actions dialog__actions--equal">
            <button type="button" className="button" onClick={() => setMenu('closed')}>
              {t('game.menu.resume')}
            </button>
            <button
              type="button"
              className="button button--danger"
              onClick={() => void resign()}
              disabled={busy || game.status !== 'active'}
            >
              <FlagIcon />
              {t('online.resign.yes')}
            </button>
          </div>
        </Dialog>
      )}
      {deadline && (
        <p className="note game__note">{t('online.game.deadline', { date: deadline })}</p>
      )}

      {finished && outcome && game.result && (
        <>
          {outcome === 'win' && endedHere && <Celebration />}
          {resultOpen && (
            <Dialog
              title={t(`game.over.${outcome}`)}
              onDismiss={() => setResultOpen(false)}
              className={`result result--${outcome}`}
              header={
                <span className="result__emblem" aria-hidden="true">
                  {outcome === 'win' ? <StarIcon /> : <SeedIcon />}
                </span>
              }
            >
              <p className="result__tagline">
                {outcome === 'loss'
                  ? t('online.over.tagline.loss', { name: opponentName })
                  : t(`game.over.tagline.${outcome}`)}
              </p>
              <FinalScore you={finalStores[0]} them={finalStores[1]} themLabel={opponentName} />
              <p className="visually-hidden">
                {t('online.over.score', {
                  you: finalStores[0],
                  them: finalStores[1],
                  name: opponentName,
                })}
              </p>
              <p className="note">
                {endReasonText(t, game.result.reason, outcome === 'loss', opponentName)}
              </p>
              <RatingChanges game={game} />
              <ErrorLine error={error} />
              <div className="dialog__actions">
                {game.opponent && (
                  <button
                    type="button"
                    className="button button--primary"
                    onClick={rematch}
                    disabled={busy}
                  >
                    {t('online.over.rematch')}
                  </button>
                )}
                {statsVisible && (
                  <Link className="button" to="/stats">
                    {t('menu.stats')}
                  </Link>
                )}
                <Link className="button button--ghost" to="/game?tab=online">
                  {t('online.menu.back')}
                </Link>
              </div>
            </Dialog>
          )}
        </>
      )}
    </section>
  );
}

/** How the game moved your ratings, or why it didn't. */
function RatingChanges({ game }: { game: Game }) {
  const { t } = useTranslation();
  const changes = game.rating_changes ?? [];
  if (changes.length === 0) {
    return (
      <p className="note">
        {t(game.rated === false ? 'online.over.casual' : 'online.over.unrated')}
      </p>
    );
  }
  return (
    <ul className="rating-changes">
      {changes.map((c) => {
        const diff = c.after - c.before;
        return (
          <li key={c.pool}>
            <span>{poolLabel(t, c.pool)}</span>
            <span>
              {t('online.over.rating', { before: c.before, after: c.after })}{' '}
              <strong className={diff >= 0 ? 'rating-up' : 'rating-down'}>
                {t('online.over.rating_diff', { diff: diff >= 0 ? `+${diff}` : `${diff}` })}
              </strong>
            </span>
          </li>
        );
      })}
    </ul>
  );
}

/** Why the game ended, from your side: a resign or timeout says who. */
function endReasonText(
  t: (key: string, opts?: Record<string, unknown>) => string,
  reason: string,
  youLost: boolean,
  name: string,
): string {
  switch (reason) {
    case 'resigned':
      return youLost ? t('online.over.resigned_you') : t('online.over.resigned_them', { name });
    case 'timeout':
      return youLost ? t('online.over.timeout_you') : t('online.over.timeout_them', { name });
    default:
      return t(`game.over.reason.${reason}`, { defaultValue: '' });
  }
}

// --- Sowing moves as they arrive ------------------------------------------------------------------

interface Announcement {
  id: number;
  events: MoveEvent[];
  mover: Player;
}

function positionAfter(core: CoreApi, variant: VariantRef, first: Player, moves: number[]) {
  const replay = core.replay(variant, first, moves);
  return replay.steps.at(-1)?.state ?? replay.initial;
}

const sameMoves = (a: readonly number[], b: readonly number[]) =>
  a.length === b.length && a.every((m, i) => b[i] === m);

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/**
 * The position on screen, catching up with `target` (the game's moves) one move at a time:
 * every new move, yours or the opponent's, is sown frame by frame with its sounds, exactly as
 * in a game against the computer. Opening a game shows its position at once. If `target` stops
 * extending what's shown (a move the server refused), the board snaps to it.
 */
function useMoveAnimation(
  core: CoreApi,
  variant: VariantRef,
  first: Player,
  target: number[],
  speed: number,
  sound: boolean,
) {
  const [shown, setShown] = useState<{ moves: number[]; state: GameState }>(() => ({
    moves: target,
    state: positionAfter(core, variant, first, target),
  }));
  const [frame, setFrame] = useState<Frame | null>(null);
  const [animating, setAnimating] = useState(false);
  const [announcement, setAnnouncement] = useState<Announcement | null>(null);
  const running = useRef(false);
  const alive = useRef(true);
  const soundOn = useRef(sound);
  useEffect(() => {
    soundOn.current = sound;
  }, [sound]);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  const { id: variantId, version } = variant;
  useEffect(() => {
    if (running.current || sameMoves(target, shown.moves)) return;
    const v = { id: variantId, version };
    running.current = true;
    void (async () => {
      const extends_ =
        target.length > shown.moves.length && shown.moves.every((m, i) => target[i] === m);
      let next: { moves: number[]; state: GameState };
      let events: MoveEvent[] | null = null;
      try {
        if (!extends_) throw new Error('snap');
        const pit = target[shown.moves.length]!;
        const result = core.applyMove(v, shown.state, pit);
        setAnimating(true);
        for (const f of buildFrames(shown.state, pit, result.events, result.state)) {
          if (!alive.current) return;
          setFrame(f);
          if (soundOn.current) playFrameSound(f.kind);
          const delay = frameDelay(f.kind, speed);
          if (delay > 0) await sleep(delay);
        }
        next = { moves: [...shown.moves, pit], state: result.state };
        events = result.events;
      } catch {
        next = { moves: target, state: positionAfter(core, v, first, target) };
      }
      if (!alive.current) return;
      const mover = shown.state.to_move;
      running.current = false;
      setFrame(null);
      setAnimating(false);
      setShown(next);
      if (events) setAnnouncement((a) => ({ id: (a?.id ?? 0) + 1, events, mover }));
    })();
  }, [core, variantId, version, first, target, shown, speed]);

  return {
    state: shown.state,
    frame,
    animating,
    announcement,
    /** The board shows every move in `target`. */
    caughtUp: sameMoves(target, shown.moves),
  };
}
