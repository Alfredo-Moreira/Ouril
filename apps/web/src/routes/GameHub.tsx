import { useEffect, useRef, useState, type KeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useSearchParams } from 'react-router';

import { rememberReturnTo } from '../auth/returnTo';
import { ArrowIcon } from '../components/Icons';
import { NewGameForm } from '../components/NewGameForm';
import { FEATURES } from '../features';
import { useApp, useOnlineVisible } from '../state/useApp';
import { getCurrentGame, type CurrentGame } from '../storage/repo';
import { OnlineLobby } from './Online';

type Mode = 'computer' | 'online';

/**
 * `/game`, the menu's "Game": choose who to play. **Against the computer** continues the game in
 * progress or starts a new one (played at `/play`). **Online** (when accounts are on) is the
 * lobby for games against other players for signed-in players, and an invitation to sign in for
 * guests. The tab lives in the URL (`?tab=online`), so links and "back" land on it.
 */
export function GameHub() {
  const { t } = useTranslation();
  const [params, setParams] = useSearchParams();
  const modes: Mode[] = FEATURES.accounts ? ['computer', 'online'] : ['computer'];
  const mode: Mode = modes.includes(params.get('tab') as Mode)
    ? (params.get('tab') as Mode)
    : 'computer';
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);

  const select = (next: Mode) =>
    setParams(next === 'computer' ? {} : { tab: next }, { replace: true });

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const step = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    const next = modes[(modes.indexOf(mode) + step + modes.length) % modes.length]!;
    select(next);
    tabs.current[modes.indexOf(next)]?.focus();
  };

  return (
    <section className="game-hub">
      <h1>{t('play.title')}</h1>
      {modes.length > 1 && (
        <div
          className="rules__tabs choice-row"
          role="tablist"
          aria-label={t('play.mode')}
          onKeyDown={onKeyDown}
        >
          {modes.map((m, i) => (
            <button
              key={m}
              ref={(el) => {
                tabs.current[i] = el;
              }}
              type="button"
              role="tab"
              id={`play-tab-${m}`}
              aria-selected={m === mode}
              aria-controls="play-panel"
              tabIndex={m === mode ? 0 : -1}
              className="choice rules__tab"
              onClick={() => select(m)}
            >
              {t(`play.mode.${m}`)}
            </button>
          ))}
        </div>
      )}
      <div
        id="play-panel"
        role={modes.length > 1 ? 'tabpanel' : undefined}
        aria-labelledby={modes.length > 1 ? `play-tab-${mode}` : undefined}
        className="game-hub__panel"
      >
        {mode === 'computer' ? <VsComputer /> : <Online />}
      </div>
    </section>
  );
}

/** Continue the game in progress, or start a new one. */
function VsComputer() {
  const { t } = useTranslation();
  const { dataVersion } = useApp();
  const [current, setCurrent] = useState<CurrentGame | null>(null);

  useEffect(() => {
    let cancelled = false;
    void getCurrentGame().then((g) => {
      if (!cancelled) setCurrent(g ?? null);
    });
    return () => {
      cancelled = true;
    };
  }, [dataVersion]);

  const inProgress = current?.state.status === 'playing';
  return (
    <>
      {inProgress && current && (
        <div className="card card--accent resume">
          <div>
            <h2 className="card__title">{t('home.resume')}</h2>
            <p>{t('home.resume.body', { level: t(`game.level.${current.level}`) })}</p>
          </div>
          <div className="resume__actions">
            <Link className="button button--primary" to="/play">
              {t('home.resume')}
              <ArrowIcon />
            </Link>
          </div>
        </div>
      )}
      <NewGameForm title={t('game.new')} replaceWarning={inProgress} />
    </>
  );
}

/** The online lobby for signed-in players; guests are asked to sign in first. */
function Online() {
  const { t } = useTranslation();
  const signedIn = useOnlineVisible();
  if (signedIn) return <OnlineLobby />;
  return (
    <div className="card">
      <h2 className="card__title">{t('play.online.sign_in.title')}</h2>
      <p>{t('play.online.sign_in.body')}</p>
      <Link
        className="button button--primary"
        to="/sign-in"
        onClick={() => rememberReturnTo('/game?tab=online')}
      >
        {t('account.sign_in')}
      </Link>
    </div>
  );
}
