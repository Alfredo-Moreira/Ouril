import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useLocation, useNavigate } from 'react-router';

import { hasMusic, skipTrack } from '../audio/music';
import { useMusicPrefs } from '../audio/musicPrefs';
import { useBoardPace } from '../board3d/pace';
import { BoardView } from '../components/BoardView';
import { Celebration, ScoreValue } from '../components/GameFx';
import {
  BulbIcon,
  ExitFullscreenIcon,
  FlagIcon,
  FullscreenIcon,
  HomeIcon,
  InfoIcon,
  MusicIcon,
  MusicOffIcon,
  PauseIcon,
  SkipIcon,
  SeedIcon,
  StarIcon,
  UndoIcon,
} from '../components/Icons';
import { useCountUp } from '../components/useCountUp';
import { useFullscreen } from '../components/useFullscreen';
import { Dialog } from '../components/Dialog';
import { MoveAnnouncer } from '../components/MoveAnnouncer';
import { useCore } from '../engine/CoreContext';
import { forfeitGame } from '../game/forfeit';
import {
  PLAYABLE_VARIANTS,
  resolveSavedVariant,
  startGame,
  type PlayableVariant,
} from '../game/session';
import { RulesView } from '../components/RulesView';
import { useGameController } from '../game/useGameController';
import { useImmersiveScreen } from '../state/immersive';
import { useApp } from '../state/useApp';
import {
  clearCurrentGame,
  getCurrentGame,
  saveCurrentGame,
  type CurrentGame,
} from '../storage/repo';
import { NewGameForm, type NewGameRequest } from '../components/NewGameForm';
import { FEATURES } from '../features';

export interface GameRouteProps {
  /** 1 = normal speed, 0 = no animation delays (tests). Defaults from prefers-reduced-motion. */
  animationSpeed?: number;
}

function defaultSpeed(): number {
  if (typeof window === 'undefined' || !window.matchMedia) return 1;
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 0.4 : 1;
}

/** `/play`: starts a new game (from Home) or resumes the unfinished one. */
export function GameRoute({ animationSpeed }: GameRouteProps) {
  const { t } = useTranslation();
  const { core } = useCore();
  const { notifyLocalChange } = useApp();
  const location = useLocation();
  const navigate = useNavigate();
  const [game, setGame] = useState<CurrentGame | null | undefined>(undefined);
  const handled = useRef<NewGameRequest | null>(null);
  const loaded = useRef(false);
  const [speed] = useState(() => animationSpeed ?? defaultSpeed());

  const request = (location.state as { newGame?: NewGameRequest } | null)?.newGame;
  const [unplayable, setUnplayable] = useState(false);

  /**
   * The saved game, with its variant checked: a renamed variant is upgraded in place; a game
   * whose rules this app doesn't have is cleared (and explained) instead of crashing.
   */
  const loadSavedGame = useCallback(async (): Promise<CurrentGame | null> => {
    const g = await getCurrentGame();
    if (!g) return null;
    const v = resolveSavedVariant(core, g.variant);
    if (!v) {
      await clearCurrentGame();
      setUnplayable(true);
      return null;
    }
    if (v.id === g.variant.id) return g;
    const upgraded = { ...g, variant: v };
    await saveCurrentGame(upgraded);
    return upgraded;
  }, [core]);

  useEffect(() => {
    if (request) {
      // A new game from Home or "Play again" (handled once, also under StrictMode).
      if (handled.current === request) return;
      handled.current = request;
      loaded.current = true;
      const g = startGame(core, request);
      void (async () => {
        // Starting over forfeits any unfinished game, even one with no moves yet (a loss,
        // ADR 0022), so a new game can't be used to dodge one.
        const previous = await loadSavedGame();
        if (previous && previous.state.status === 'playing') {
          await forfeitGame(core, previous);
          notifyLocalChange();
        }
        await saveCurrentGame(g);
        // Drop the request from history so a reload resumes instead of restarting.
        navigate('/play', { replace: true, state: null });
        setGame(g);
      })();
    } else if (!loaded.current) {
      loaded.current = true;
      void loadSavedGame().then((g) => setGame(g));
    }
  }, [core, request, navigate, notifyLocalChange, loadSavedGame]);

  if (game === undefined) return <p role="status">{t('common.loading')}</p>;
  if (game === null)
    return (
      <section className="new-game-screen">
        {unplayable && (
          <p className="banner banner--warning" role="status">
            {t('game.unplayable')}
          </p>
        )}
        <NewGameForm title={t('game.new')} intro={t('game.none.body')} headingLevel={1} />
      </section>
    );
  return <GameView key={game.id} initial={game} speed={speed} />;
}

function GameView({ initial, speed: baseSpeed }: { initial: CurrentGame; speed: number }) {
  useImmersiveScreen();
  const { t } = useTranslation();
  const speed = baseSpeed * useBoardPace();
  const navigate = useNavigate();
  const { settings } = useApp();
  const c = useGameController(initial, speed);
  const { game, frame } = c;

  const pits = frame?.pits ?? game.state.pits;
  const stores = frame?.stores ?? game.state.stores;
  const you = t('game.side.you');
  const ai = t('game.side.ai', { level: t(`game.level.${game.level}`) });

  let status: string;
  if (c.finished) status = t('game.status.over');
  else if (c.animating) status = t('game.status.sowing');
  else if (c.aiThinking) status = t('game.status.ai_thinking');
  else if (game.state.to_move === game.humanPlayer) status = t('game.status.your_turn');
  else status = t('game.status.ai_turn');

  const root = useRef<HTMLElement>(null);
  // The pause menu: keep playing, save and exit (resume later from Home), or forfeit.
  const [menu, setMenu] = useState<'closed' | 'menu' | 'confirm'>('closed');
  const [rulesOpen, setRulesOpen] = useState(false);
  const [music, setMusicPrefs] = useMusicPrefs();
  const playing: PlayableVariant =
    PLAYABLE_VARIANTS.find((v) => v === game.variant.id) ?? PLAYABLE_VARIANTS[0];
  const forfeit = async () => {
    await c.forfeit();
    setMenu('closed');
  };
  const fullscreen = useFullscreen(root);
  const aiTurn = !c.finished && game.state.to_move !== game.humanPlayer;
  const youTurn = !c.finished && game.state.to_move === game.humanPlayer;

  return (
    <section className="game" ref={root}>
      <h1 className="visually-hidden">{t('game.title')}</h1>
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
          <div className={`score score--ai${aiTurn ? ' score--turn' : ''}`}>
            <span className="score__avatar" aria-hidden="true">
              {ai.slice(0, 1)}
            </span>
            <span className="score__name">{ai}</span>
            <ScoreValue value={stores[1]} />
          </div>
          <p className="game-status" role="status">
            {status}
          </p>
          <div className={`score score--you${youTurn ? ' score--turn' : ''}`}>
            <span className="score__avatar" aria-hidden="true">
              {you.slice(0, 1)}
            </span>
            <span className="score__name">{you}</span>
            <ScoreValue value={stores[0]} />
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
          pits={pits}
          stores={stores}
          legal={c.canPlay ? c.legalMoves : []}
          onPlay={c.play}
          highlight={frame && frame.kind !== 'end' ? { pit: frame.pit, kind: frame.kind } : null}
          hintPit={c.hintPit}
          southLabel={you}
          northLabel={ai}
          speed={speed}
          finished={!!c.finished}
        />
      </div>

      <MoveAnnouncer
        events={c.announcement?.events ?? null}
        byHuman={c.announcement?.byHuman ?? true}
      />
      {c.error && (
        <p className="error" role="alert">
          {t('game.error', { code: c.error })}
        </p>
      )}

      <div className="game-actions">
        <button type="button" className="button" onClick={c.undo} disabled={!c.canUndo}>
          <UndoIcon />
          {t('game.undo')}
        </button>
        {settings.hints && (
          <button type="button" className="button" onClick={c.requestHint} disabled={!c.canPlay}>
            <BulbIcon />
            {t('game.hint')}
          </button>
        )}
        <button
          type="button"
          className="button button--ghost"
          onClick={() => setMenu('menu')}
          aria-haspopup="dialog"
        >
          <PauseIcon />
          {t('game.leave')}
        </button>
      </div>

      {rulesOpen && (
        <Dialog
          title={t('game.rules.title', { name: t(`variant.${playing}.short`) })}
          onDismiss={() => setRulesOpen(false)}
          className="dialog--rules"
        >
          <RulesView variant={playing} headingLevel={3} />
          <div className="dialog__actions">
            <button
              type="button"
              className="button button--primary"
              onClick={() => setRulesOpen(false)}
            >
              {t('game.rules.close')}
            </button>
            <Link className="button button--ghost" to={`/rules?variant=${playing}`}>
              {t('game.rules.all')}
            </Link>
          </div>
        </Dialog>
      )}
      {menu === 'menu' && !c.finished && (
        <Dialog title={t('game.menu.title')} onDismiss={() => setMenu('closed')}>
          <p>{t('game.menu.body')}</p>
          <div className="dialog__actions dialog__actions--stack">
            <button
              type="button"
              className="button button--primary"
              onClick={() => setMenu('closed')}
            >
              {t('game.menu.resume')}
            </button>
            <Link className="button" to="/">
              <HomeIcon />
              {t('game.menu.save_exit')}
            </Link>
            <button
              type="button"
              className="button button--ghost button--danger-text"
              onClick={() => setMenu('confirm')}
              disabled={!c.canForfeit}
            >
              <FlagIcon />
              {t('game.menu.forfeit')}
            </button>
          </div>
        </Dialog>
      )}
      {menu === 'confirm' && !c.finished && (
        <Dialog title={t('game.forfeit.title')} onDismiss={() => setMenu('menu')}>
          <p>{t('game.forfeit.body')}</p>
          <div className="dialog__actions dialog__actions--equal">
            <button type="button" className="button" onClick={() => setMenu('closed')}>
              {t('game.menu.resume')}
            </button>
            <button
              type="button"
              className="button button--danger"
              onClick={() => void forfeit()}
              disabled={!c.canForfeit}
            >
              <FlagIcon />
              {t('game.forfeit.confirm')}
            </button>
          </div>
        </Dialog>
      )}
      <p className="note game__note">{t('game.autosave')}</p>

      {c.finished && (
        <>
          {c.finished.outcome === 'win' && <Celebration />}
          <Dialog
            title={t(`game.over.${c.finished.outcome}`)}
            onDismiss={() => navigate('/')}
            className={`result result--${c.finished.outcome}`}
            header={
              <span className="result__emblem" aria-hidden="true">
                {c.finished.outcome === 'win' ? <StarIcon /> : <SeedIcon />}
              </span>
            }
          >
            <p className="result__tagline">{t(`game.over.tagline.${c.finished.outcome}`)}</p>
            <FinalScore
              you={c.finished.record.result.stores[0]}
              ai={c.finished.record.result.stores[1]}
            />
            <p className="visually-hidden">
              {t('game.over.score', {
                you: c.finished.record.result.stores[0],
                ai: c.finished.record.result.stores[1],
              })}
            </p>
            <p className="note">{t(`game.over.reason.${c.finished.record.result.reason}`)}</p>
            <div className="dialog__actions">
              <button
                type="button"
                className="button button--primary"
                onClick={() =>
                  navigate('/play', {
                    state: {
                      newGame: {
                        level: game.level,
                        starter: 'random',
                        variant: PLAYABLE_VARIANTS.find((v) => v === game.variant.id),
                      } satisfies NewGameRequest,
                    },
                  })
                }
              >
                {t(c.finished.outcome === 'loss' ? 'game.over.rematch' : 'game.over.play_again')}
              </button>
              {FEATURES.stats && (
                <Link className="button" to="/stats">
                  {t('menu.stats')}
                </Link>
              )}
              <Link className="button button--ghost" to="/">
                {t('game.over.home')}
              </Link>
            </div>
          </Dialog>
        </>
      )}
    </section>
  );
}

/** The final score, counting up once when the result appears. */
function FinalScore({ you, ai }: { you: number; ai: number }) {
  const { t } = useTranslation();
  const y = useCountUp(you);
  const a = useCountUp(ai);
  return (
    <div className="result__score" aria-hidden="true">
      <span className="result__side">
        <span className="result__num">{y}</span>
        <span className="result__label">{t('game.over.you')}</span>
      </span>
      <span className="result__dash">–</span>
      <span className="result__side">
        <span className="result__num">{a}</span>
        <span className="result__label">{t('game.over.ai')}</span>
      </span>
    </div>
  );
}
