import { useEffect } from 'react';
import { BrowserRouter, Link, NavLink, Route, Routes, useLocation } from 'react-router';
import { useTranslation } from 'react-i18next';

import type { ApiClient } from './api/client';
import { preloadBoard3D } from './board3d/load';
import { detectDevice } from './board3d/mode';
import { ConsentPrompt } from './components/ConsentPrompt';
import { hasBoardLinks } from './content/boards';
import { cacheMusic, setMusic } from './audio/music';
import { useMusicPrefs } from './audio/musicPrefs';
import { ScreenErrorBoundary } from './components/ErrorBoundary';
import { telemetryConfigured, trackPageView } from './telemetry';
import {
  BagIcon,
  InfoIcon,
  LearnIcon,
  LogoMark,
  PlayIcon,
  RulesIcon,
  SettingsIcon,
  StatsIcon,
  UserIcon,
} from './components/Icons';
import type { AiPlayer } from './engine/ai';
import { useCore } from './engine/CoreContext';
import { CoreProvider } from './engine/CoreProvider';
import type { CoreApi } from './engine/types';
import { FEATURES } from './features';
import { About } from './routes/About';
import { GameRoute } from './routes/Game';
import { Home } from './routes/Home';
import { ReplayRoute } from './routes/Replay';
import { NotFound } from './routes/NotFound';
import { Rules } from './routes/Rules';
import { Settings } from './routes/Settings';
import { SignIn } from './routes/SignIn';
import { Stats } from './routes/Stats';
import { Tutorial } from './routes/Tutorial';
import { AppProvider } from './state/AppProvider';
import { useImmersive } from './state/immersive';
import { useApp } from './state/useApp';

export interface AppProps {
  /** Test seams: inject the core/AI/API client. Production uses the defaults. */
  core?: CoreApi;
  ai?: AiPlayer;
  createApi?: () => ApiClient;
  animationSpeed?: number;
}

/**
 * The web app. The player starts as an unsigned-in guest and nothing here talks to the
 * network unless they opt into usage statistics (ADR 0014): the core is a same-origin WASM
 * asset and data lives in IndexedDB.
 */
export function App(props: AppProps) {
  return (
    <BrowserRouter>
      <AppShell {...props} />
    </BrowserRouter>
  );
}

/** Everything below the router (tests render it inside a MemoryRouter). */
export function AppShell({ core, ai, createApi, animationSpeed }: AppProps) {
  return (
    <CoreProvider core={core} ai={ai}>
      <WithCore createApi={createApi} animationSpeed={animationSpeed} />
    </CoreProvider>
  );
}

function WithCore({ createApi, animationSpeed }: Pick<AppProps, 'createApi' | 'animationSpeed'>) {
  const { core } = useCore();
  return (
    <AppProvider createApi={createApi} coreVersion={core.coreVersion()}>
      <Layout animationSpeed={animationSpeed} />
    </AppProvider>
  );
}

function Layout({ animationSpeed }: { animationSpeed?: number }) {
  const { t } = useTranslation();
  const { auth, user, update, consent } = useApp();
  const { pathname } = useLocation();
  // A screen view per route, only with usage-statistics consent (ADR 0014). Re-runs when the
  // choice is made, so the screen the player is on counts too.
  useEffect(() => {
    trackPageView(pathname);
  }, [pathname, consent.decided, consent.usageStats]);
  useEffect(() => {
    // Fetch the 3D board early (idle time) where it will be used, so it's ready on first show.
    if (detectDevice().webgl) preloadBoard3D();
  }, []);
  // A game on screen takes the whole viewport (no header or tab bar; it has its own menu), and
  // the landing page runs its sections edge to edge.
  const immersive = useImmersive();
  // Game music: only while a game is on screen, if the player hasn't turned it off.
  const [music] = useMusicPrefs();
  useEffect(() => {
    setMusic(immersive && music.on, music.volume);
    if (music.on) cacheMusic();
  }, [immersive, music]);
  const wide = pathname === '/';
  return (
    <>
      <a className="skip-link" href="#main">
        {t('common.skip_to_content')}
      </a>
      {!immersive && (
        <header className="app-header">
          <div className="app-header__inner">
            <NavLink to="/" className="brand" end>
              <LogoMark className="brand__mark" />
              <span className="brand__name">{t('app.title')}</span>
            </NavLink>
            {/* One nav for every screen size: a top bar on wide screens, a bottom tab bar on phones. */}
            <nav className="app-nav" aria-label={t('menu.label')}>
              <NavLink to="/play">
                <PlayIcon />
                <span>{t('menu.game')}</span>
              </NavLink>
              <NavLink to="/tutorial">
                <LearnIcon />
                <span>{t('menu.tutorial')}</span>
              </NavLink>
              <NavLink to="/rules" className="app-nav__secondary">
                <RulesIcon />
                <span>{t('menu.rules')}</span>
              </NavLink>
              {FEATURES.stats && (
                <NavLink to="/stats">
                  <StatsIcon />
                  <span>{t('menu.stats')}</span>
                </NavLink>
              )}
              <NavLink to="/settings">
                <SettingsIcon />
                <span>{t('menu.settings')}</span>
              </NavLink>
              <NavLink to="/about">
                <InfoIcon />
                <span>{t('menu.about')}</span>
              </NavLink>
            </nav>
            {hasBoardLinks() && (
              <Link to="/about#buy" className="account-link shop-link">
                <BagIcon />
                <span>{t('menu.shop')}</span>
              </Link>
            )}
            {FEATURES.accounts && (
              <NavLink to={auth === 'guest' ? '/sign-in' : '/settings'} className="account-link">
                <UserIcon />
                <span>
                  {auth === 'guest'
                    ? t('account.sign_in')
                    : (user?.display_name ?? t('menu.account'))}
                </span>
              </NavLink>
            )}
          </div>
        </header>
      )}
      {update === 'required' && (
        <div className="banner banner--warning" role="alert">
          <p>{t('update.required')}</p>
          <button type="button" className="button" onClick={() => window.location.reload()}>
            {t('update.reload')}
          </button>
        </div>
      )}
      {update === 'recommended' && (
        <div className="banner" role="status">
          <p>{t('update.recommended')}</p>
          <button type="button" className="button" onClick={() => window.location.reload()}>
            {t('update.reload')}
          </button>
        </div>
      )}
      <main
        id="main"
        tabIndex={-1}
        className={immersive ? 'main--immersive' : wide ? 'main--wide' : undefined}
      >
        <ScreenErrorBoundary resetKey={pathname}>
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/play" element={<GameRoute animationSpeed={animationSpeed} />} />
            <Route path="/tutorial" element={<Tutorial animationSpeed={animationSpeed} />} />
            {FEATURES.stats && <Route path="/stats" element={<Stats />} />}
            {FEATURES.stats && <Route path="/replay/:id" element={<ReplayRoute />} />}
            <Route path="/rules" element={<Rules />} />
            <Route path="/settings" element={<Settings />} />
            {FEATURES.accounts && <Route path="/sign-in" element={<SignIn />} />}
            <Route path="/about" element={<About />} />
            <Route path="*" element={<NotFound />} />
          </Routes>
        </ScreenErrorBoundary>
      </main>
      {/* Asked only when a telemetry service is configured (ADR 0014): nothing to consent to otherwise. */}
      {telemetryConfigured() && <ConsentPrompt />}
    </>
  );
}
