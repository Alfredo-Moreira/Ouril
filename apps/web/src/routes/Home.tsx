import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
  type RefObject,
} from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';

import { useBoardPace } from '../board3d/pace';
import { BoardView } from '../components/BoardView';
import { BuyBoards } from '../components/BuyBoards';
import { Dialog } from '../components/Dialog';
import { NewGameForm } from '../components/NewGameForm';
import {
  ArrowIcon,
  FlagIcon,
  InfoIcon,
  LearnIcon,
  PlayIcon,
  RulesIcon,
  SeedIcon,
  SettingsIcon,
  SparkleIcon,
  StatsIcon,
  UserIcon,
} from '../components/Icons';
import { COMING_SOON } from '../content/comingSoon';
import { useCore } from '../engine/CoreContext';
import { FEATURES } from '../features';
import type { GameState } from '../engine/types';
import { buildFrames, frameDelay, type Frame } from '../game/animation';
import { forfeitGame } from '../game/forfeit';
import { defaultVariantRef } from '../game/session';
import { useApp } from '../state/useApp';
import { getCurrentGame, type CurrentGame } from '../storage/repo';

export type { NewGameRequest } from '../components/NewGameForm';

/**
 * The landing page: a vibrant hero with a live board (the computer playing itself, with the
 * hand), quick start, how to play, the game's heritage, and links to the rest of the app.
 * Motion follows docs/architecture/web-design.md: the hero reveal plays once per session,
 * sections reveal once as they scroll in, and nothing moves with reduced motion.
 */
export function Home() {
  const { t } = useTranslation();
  const { auth, user, dataVersion, notifyLocalChange } = useApp();
  const [current, setCurrent] = useState<CurrentGame | null | undefined>(undefined);

  useEffect(() => {
    let cancelled = false;
    void getCurrentGame().then((g) => {
      if (!cancelled) setCurrent(g ?? null);
    });
    return () => {
      cancelled = true;
    };
  }, [dataVersion]); // re-read when the active owner changes (sign-in, account deletion)

  const resumable = current && current.state.status === 'playing';
  const { core } = useCore();
  const [confirmForfeit, setConfirmForfeit] = useState(false);
  const [forfeited, setForfeited] = useState(false);
  const forfeit = async () => {
    if (!current || current.state.status !== 'playing') return;
    await forfeitGame(core, current);
    notifyLocalChange();
    setConfirmForfeit(false);
    setCurrent(null);
    setForfeited(true);
  };
  // The hero reveal plays once per session: Home is visited often, and repeating it would
  // only slow the page down (motion frequency rule).
  const [intro] = useState(() => {
    try {
      if (sessionStorage.getItem('ouril_intro')) return false;
      sessionStorage.setItem('ouril_intro', '1');
    } catch {
      // Storage unavailable: skip the reveal.
      return false;
    }
    return true;
  });

  return (
    <div className={`landing${intro ? ' landing--intro' : ''}`}>
      <section className="hero">
        <div className="hero__backdrop" aria-hidden="true">
          <IslandStars />
        </div>
        <div className="hero__inner">
          <div className="hero__copy">
            <p className="hero__kicker">{t('home.tagline')}</p>
            <h1 className="hero__title">{t('app.title')}</h1>
            <p className="hero__lede">{t('home.hero.lede')}</p>
            <div className="hero__ctas">
              {resumable ? (
                <Link className="button button--primary button--large button--glow" to="/play">
                  <PlayIcon />
                  {t('home.cta.resume')}
                </Link>
              ) : (
                <a className="button button--primary button--large button--glow" href="#play">
                  <PlayIcon />
                  {t('home.cta.play')}
                </a>
              )}
              <Link className="button button--large button--on-dark" to="/tutorial">
                <LearnIcon />
                {t('home.cta.learn')}
              </Link>
            </div>
            {FEATURES.accounts && (
              <p className="hero__account">
                <UserIcon />
                {auth === 'guest'
                  ? t('account.guest')
                  : t('account.signed_in_as', { name: user?.display_name ?? '' })}
              </p>
            )}
          </div>
          {/* Decorative: the computer playing itself (3D where it runs well). */}
          <div className="hero__stage stage" aria-hidden="true">
            <LiveBoard />
          </div>
        </div>
      </section>

      <div className="landing__body">
        <div className="home__grid" id="play">
          {resumable && (
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
                <button
                  type="button"
                  className="button button--ghost button--danger-text"
                  onClick={() => setConfirmForfeit(true)}
                >
                  <FlagIcon />
                  {t('game.menu.forfeit')}
                </button>
              </div>
            </div>
          )}
          {forfeited && (
            <p className="card resume-note" role="status">
              {t('home.forfeited')} {FEATURES.stats && <Link to="/stats">{t('menu.stats')}</Link>}
            </p>
          )}
          {confirmForfeit && current && (
            <Dialog title={t('game.forfeit.title')} onDismiss={() => setConfirmForfeit(false)}>
              <p>{t('game.forfeit.body')}</p>
              <div className="dialog__actions dialog__actions--equal">
                <button type="button" className="button" onClick={() => setConfirmForfeit(false)}>
                  {t('game.forfeit.cancel')}
                </button>
                <button
                  type="button"
                  className="button button--danger"
                  onClick={() => void forfeit()}
                >
                  <FlagIcon />
                  {t('game.forfeit.confirm')}
                </button>
              </div>
            </Dialog>
          )}

          <NewGameForm
            title={t('home.quick.title')}
            replaceWarning={!!current && current.state.status === 'playing'}
          />
        </div>

        <Reveal as="section" className="steps" labelledBy="steps-title">
          <h2 id="steps-title" className="section-title">
            {t('home.steps.title')}
          </h2>
          <ol className="steps__list">
            {STEPS.map((n) => (
              <li key={n} className="step">
                <span className="step__num" aria-hidden="true">
                  {n}
                </span>
                <h3 className="step__title">{t(`home.steps.${n}.title`)}</h3>
                <p className="step__body">{t(`home.steps.${n}.body`)}</p>
              </li>
            ))}
          </ol>
        </Reveal>

        <Reveal as="section" className="heritage" labelledBy="heritage-title">
          <div className="heritage__stars" aria-hidden="true">
            <IslandStars />
          </div>
          <div>
            <p className="heritage__kicker">{t('home.heritage.kicker')}</p>
            <h2 id="heritage-title" className="section-title">
              {t('home.heritage.title')}
            </h2>
            <p className="heritage__body">{t('home.heritage.body')}</p>
            <p className="heritage__body heritage__body--next">{t('home.heritage.world_cup')}</p>
          </div>
        </Reveal>

        {(COMING_SOON.length > 0 || import.meta.env.DEV) && (
          <Reveal as="section" className="soon" labelledBy="soon-title">
            <h2 id="soon-title" className="section-title">
              {t('home.soon.title')}
            </h2>
            <ul className="soon__list">
              {COMING_SOON.length > 0
                ? COMING_SOON.map((id) => (
                    <li key={id} className="soon__item">
                      <span className="soon__icon" aria-hidden="true">
                        <SparkleIcon />
                      </span>
                      <h3 className="soon__title">{t(`home.soon.${id}.title`)}</h3>
                      <p className="soon__body">{t(`home.soon.${id}.body`)}</p>
                    </li>
                  ))
                : [0, 1, 2].map((i) => (
                    <li key={i} className="soon__item soon__item--placeholder">
                      <span className="soon__icon" aria-hidden="true">
                        <SparkleIcon />
                      </span>
                      <h3 className="soon__title">{t('home.soon.placeholder.title')}</h3>
                      <p className="soon__body">{t('home.soon.placeholder.body')}</p>
                    </li>
                  ))}
            </ul>
          </Reveal>
        )}

        <Reveal as="section" className="features" labelledBy="features-title">
          <h2 id="features-title" className="section-title">
            {t('home.features.title')}
          </h2>
          <ul className="features__list">
            {FEATURE_CARDS.map(({ key, Icon }) => (
              <li key={key} className="feature">
                <span className="feature__icon" aria-hidden="true">
                  <Icon />
                </span>
                <h3 className="feature__title">{t(`home.features.${key}.title`)}</h3>
                <p className="feature__body">{t(`home.features.${key}.body`)}</p>
              </li>
            ))}
          </ul>
        </Reveal>

        <Reveal as="div" className="home-buy">
          <BuyBoards id="home-buy" />
        </Reveal>

        <nav className="tiles" aria-label={t('home.more')}>
          {TILES.map(({ to, Icon, key }) => (
            <Link key={to} to={to} className="tile">
              <span className="tile__icon">
                <Icon />
              </span>
              <span className="tile__title">{t(`menu.${key}`)}</span>
              <span className="tile__desc">{t(`home.tile.${key}`)}</span>
            </Link>
          ))}
        </nav>
      </div>
    </div>
  );
}

const STEPS = [1, 2, 3, 4] as const;

const FEATURE_CARDS = [
  { key: 'offline', Icon: SeedIcon },
  { key: 'levels', Icon: StatsIcon },
  { key: 'guest', Icon: UserIcon },
] as const;

const TILES = [
  { to: '/tutorial', Icon: LearnIcon, key: 'tutorial' },
  { to: '/rules', Icon: RulesIcon, key: 'rules' },
  ...(FEATURES.stats ? [{ to: '/stats', Icon: StatsIcon, key: 'stats' } as const] : []),
  { to: '/settings', Icon: SettingsIcon, key: 'settings' },
  { to: '/about', Icon: InfoIcon, key: 'about' },
] as const;

/** The flag's circle of ten yellow stars, one per island. */
function IslandStars() {
  return (
    <svg className="island-stars" viewBox="-60 -60 120 120">
      {Array.from({ length: 10 }, (_, i) => {
        const a = (i / 10) * Math.PI * 2 - Math.PI / 2;
        return (
          <g key={i} transform={`translate(${Math.cos(a) * 44} ${Math.sin(a) * 44}) scale(0.42)`}>
            <path
              className="island-stars__star"
              style={{ '--i': i } as CSSProperties}
              d="m0-12 3.5 7.3 8 .9-5.9 5.5 1.6 7.9L0 5.7l-7.2 3.9 1.6-7.9-5.9-5.5 8-.9z"
            />
          </g>
        );
      })}
    </svg>
  );
}

// --- Reveal on scroll ---------------------------------------------------------------------------

/** Fades and lifts its content in once, the first time it scrolls into view. */
function Reveal({
  as: Tag,
  className,
  labelledBy,
  children,
}: {
  as: 'section' | 'div';
  className: string;
  labelledBy?: string;
  children: ReactNode;
}) {
  const ref = useRef<HTMLElement>(null);
  const seen = useInView(ref, { once: true, margin: '-12% 0px' });
  return (
    <Tag
      ref={ref as RefObject<HTMLElement & HTMLDivElement>}
      className={`${className} reveal${seen ? ' reveal--in' : ''}`}
      aria-labelledby={labelledBy}
    >
      {children}
    </Tag>
  );
}

function useInView(
  ref: RefObject<HTMLElement | null>,
  { once = false, margin = '0px' }: { once?: boolean; margin?: string } = {},
): boolean {
  const supported = typeof IntersectionObserver === 'function';
  const [inView, setInView] = useState(!supported);
  useEffect(() => {
    const el = ref.current;
    if (!el || !supported) return;
    const io = new IntersectionObserver(
      ([entry]) => {
        // A section already scrolled past (a jump to the end, an anchor link) counts as seen.
        const passed = once && !!entry && entry.boundingClientRect.bottom < 0;
        const visible = !!entry?.isIntersecting || passed;
        if (visible && once) {
          setInView(true);
          io.disconnect();
        } else if (!once) {
          setInView(visible);
        }
      },
      { rootMargin: margin },
    );
    io.observe(el);
    return () => io.disconnect();
  }, [ref, once, margin, supported]);
  return inView;
}

// --- Live board ---------------------------------------------------------------------------------

/** A mid-game position (48 seeds) the demo starts from, South to move. */
const SHOWCASE: GameState = {
  pits: [3, 0, 5, 6, 2, 1, 4, 7, 0, 2, 3, 1],
  stores: [6, 8],
  to_move: 'south',
  moves_since_capture: 0,
  status: 'playing',
};
/** The demo plays a handful of moves while in view, then rests (no endless loop). */
const DEMO_PLIES = 10;
const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

function prefersReducedMotion(): boolean {
  return (
    typeof window !== 'undefined' &&
    !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
  );
}

/**
 * The computer playing itself on the hero board, with the engine deciding every move (easy
 * AI). It plays only while visible, stops after `DEMO_PLIES`, and stays still with reduced
 * motion.
 */
function LiveBoard() {
  const { core } = useCore();
  const variant = useMemo(() => defaultVariantRef(core), [core]);
  const pace = useBoardPace();
  const [reduced] = useState(prefersReducedMotion);
  const [state, setState] = useState<GameState>(SHOWCASE);
  const [frame, setFrame] = useState<Frame | null>(null);
  const host = useRef<HTMLDivElement>(null);
  const visible = useInView(host);
  const played = useRef(0);
  const current = useRef<GameState>(SHOWCASE);

  useEffect(() => {
    if (!visible || reduced) return;
    let alive = true;
    void (async () => {
      await sleep(900);
      while (alive && played.current < DEMO_PLIES) {
        const s = current.current;
        if (s.status !== 'playing') break;
        const pit = core.aiMove(variant, s, 'easy', played.current + 7);
        if (pit === null || pit === undefined) break;
        const result = core.applyMove(variant, s, pit);
        for (const f of buildFrames(s, pit, result.events, result.state)) {
          if (!alive) return;
          setFrame(f);
          const d = frameDelay(f.kind, pace);
          if (d > 0) await sleep(d);
        }
        if (!alive) return;
        current.current = result.state;
        played.current++;
        setFrame(null);
        setState(result.state);
        await sleep(1100);
      }
    })();
    return () => {
      alive = false;
    };
  }, [visible, reduced, core, variant, pace]);

  return (
    <div ref={host}>
      <BoardView
        pits={frame?.pits ?? state.pits}
        stores={frame?.stores ?? state.stores}
        legal={[]}
        highlight={frame && frame.kind !== 'end' ? { pit: frame.pit, kind: frame.kind } : null}
        southLabel=""
        northLabel=""
        speed={reduced ? 0 : pace}
      />
    </div>
  );
}
