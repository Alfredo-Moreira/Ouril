import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useSearchParams } from 'react-router';

import { rememberReturnTo } from '../auth/returnTo';
import { ArrowIcon } from '../components/Icons';
import { NewGameForm } from '../components/NewGameForm';
import { PlayModeTabs } from '../components/PlayModeTabs';
import { PLAY_MODES, type PlayMode } from '../game/playModes';
import { useApp, useOnlineVisible } from '../state/useApp';
import { getCurrentGame, type CurrentGame } from '../storage/repo';
import { OnlineLobby } from './Online';

/**
 * `/game`, the menu's "Game": choose who to play. **Against the computer** continues the game in
 * progress or starts a new one (played at `/play`). **Online** (when accounts are on) is the
 * lobby for games against other players for signed-in players, and an invitation to sign in for
 * guests. The tab lives in the URL (`?tab=online`), so links and "back" land on it.
 */
export function GameHub() {
  const { t } = useTranslation();
  const [params, setParams] = useSearchParams();
  const tab = params.get('tab') as PlayMode;
  const mode: PlayMode = PLAY_MODES.includes(tab) ? tab : 'computer';

  const select = (next: PlayMode) =>
    setParams(next === 'computer' ? {} : { tab: next }, { replace: true });

  return (
    <section className="game-hub">
      <h1>{t('play.title')}</h1>
      <PlayModeTabs mode={mode} onSelect={select} idPrefix="play">
        {mode === 'computer' ? <VsComputer /> : <OnlinePanel returnTo="/game?tab=online" />}
      </PlayModeTabs>
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

/**
 * The online lobby for signed-in players; guests are asked to sign in first, and come back to
 * `returnTo` afterwards.
 */
export function OnlinePanel({ returnTo }: { returnTo: string }) {
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
        onClick={() => rememberReturnTo(returnTo)}
      >
        {t('account.sign_in')}
      </Link>
    </div>
  );
}
