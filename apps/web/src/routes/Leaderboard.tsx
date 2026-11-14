import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useSearchParams } from 'react-router';

import { ApiError } from '../api/client';
import {
  LEADERBOARD_ACTIVE_DAYS,
  LEADERBOARD_MIN_GAMES,
  POINTS_AI_DAILY_CAP,
  POINTS_AI_WIN,
  POINTS_ONLINE,
  type PointsBoard,
  type PointsEntry,
  type Leaderboard as Board,
  type LeaderboardEntry,
  type LeaderboardScope,
  type MyRatings,
} from '../api/types';
import { countryFlag, countryName } from '../content/countries';
import { BOARD_POOLS, poolLabel } from '../online/pools';
import { useApp } from '../state/useApp';

/**
 * `/leaderboard` (signed-in players, ADR 0034): your ratings, then a board for a pool (overall,
 * or a variant and mode), worldwide or for your country. The pool and scope are in the URL.
 */
export function Leaderboard() {
  const { t } = useTranslation();
  const [params, setParams] = useSearchParams();
  const kind = params.get('board') === 'points' ? 'points' : 'ratings';
  return (
    <section className="leaderboard">
      <h1>{t('leaderboard.title')}</h1>
      <div className="choice-row leaderboard__kind" role="group" aria-label={t('leaderboard.kind')}>
        {(['ratings', 'points'] as const).map((k) => (
          <button
            key={k}
            type="button"
            className="choice rules__tab"
            aria-pressed={kind === k}
            onClick={() => setParams(k === 'points' ? { board: 'points' } : {}, { replace: true })}
          >
            {t(`leaderboard.kind.${k}`)}
          </button>
        ))}
      </div>
      {kind === 'points' ? <PointsView /> : <RatingsBoard />}
    </section>
  );
}

/** The rating boards: a pool, worldwide or for your country. */
function RatingsBoard() {
  const { t, i18n } = useTranslation();
  const { api, user } = useApp();
  const [params, setParams] = useSearchParams();
  const pool = BOARD_POOLS.includes(params.get('pool') ?? '')
    ? params.get('pool')!
    : BOARD_POOLS[0]!;
  const scope: LeaderboardScope = params.get('scope') === 'country' ? 'country' : 'world';
  const [mine, setMine] = useState<MyRatings | null>(null);
  // The last answer, tagged with the board it's for: another pool or scope is still loading.
  const [loaded, setLoaded] = useState<{ key: string; board?: Board; error?: string } | null>(null);
  const country = user?.country ?? null;
  const needsCountry = scope === 'country' && !country;
  // The country goes with the request: the server's copy may not have synced yet.
  const boardCountry = scope === 'country' ? (country ?? undefined) : undefined;
  const key = `${pool}|${scope}|${boardCountry ?? ''}`;
  const board = loaded?.key === key ? (loaded.board ?? null) : null;
  const error = loaded?.key === key ? (loaded.error ?? null) : null;

  const choose = (next: { pool?: string; scope?: LeaderboardScope }) => {
    const p = next.pool ?? pool;
    const s = next.scope ?? scope;
    setParams(
      {
        ...(p === BOARD_POOLS[0] ? {} : { pool: p }),
        ...(s === 'world' ? {} : { scope: s }),
      },
      { replace: true },
    );
  };

  useEffect(() => {
    let cancelled = false;
    void api()
      .myRatings()
      .then((r) => !cancelled && setMine(r))
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [api]);

  useEffect(() => {
    if (needsCountry) return;
    let cancelled = false;
    void api()
      .leaderboard(pool, scope, boardCountry)
      .then((b) => !cancelled && setLoaded({ key, board: b }))
      .catch(
        (e: unknown) =>
          !cancelled &&
          setLoaded({
            key,
            error: e instanceof ApiError ? e.code : 'network',
          }),
      );
    return () => {
      cancelled = true;
    };
  }, [api, pool, scope, boardCountry, key, needsCountry]);

  const youInTop = !!board?.you && board.entries.some((e) => e.rank === board.you!.rank);

  return (
    <>
      {mine && mine.ratings.length > 0 && (
        <div className="card">
          <h2 className="card__title">{t('leaderboard.yours')}</h2>
          <ul className="leaderboard__mine">
            {mine.ratings.map((r) => (
              <li key={r.pool}>
                <span>{poolLabel(t, r.pool)}</span>
                <strong>
                  {r.provisional
                    ? t('leaderboard.rating.provisional', { rating: r.rating })
                    : r.rating}
                </strong>
                <span className="note">
                  {t('leaderboard.record', { wins: r.wins, draws: r.draws, losses: r.losses })}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="leaderboard__controls">
        <label className="field">
          <span>{t('leaderboard.pool')}</span>
          <select value={pool} onChange={(e) => choose({ pool: e.target.value })}>
            {BOARD_POOLS.map((p) => (
              <option key={p} value={p}>
                {poolLabel(t, p)}
              </option>
            ))}
          </select>
        </label>
        <div className="choice-row" role="group" aria-label={t('leaderboard.scope')}>
          {(['world', 'country'] as const).map((s) => (
            <button
              key={s}
              type="button"
              className="choice rules__tab"
              aria-pressed={scope === s}
              onClick={() => choose({ scope: s })}
            >
              {s === 'country' && country
                ? countryName(country, i18n.language)
                : t(`leaderboard.scope.${s}`)}
            </button>
          ))}
        </div>
      </div>

      {needsCountry ? (
        <div className="card">
          <p>{t('leaderboard.no_country')}</p>
          <Link className="button" to="/settings">
            {t('leaderboard.set_country')}
          </Link>
        </div>
      ) : error ? (
        <p className="error" role="alert">
          {t(`error.${error}`, { defaultValue: t('error.internal') })}
        </p>
      ) : !board ? (
        <p role="status">{t('common.loading')}</p>
      ) : board.entries.length === 0 ? (
        <p className="note">
          {t('leaderboard.empty', { games: LEADERBOARD_MIN_GAMES, days: LEADERBOARD_ACTIVE_DAYS })}
        </p>
      ) : (
        <table className="leaderboard__table">
          <caption className="visually-hidden">
            {t('leaderboard.caption', {
              pool: poolLabel(t, pool),
              scope:
                scope === 'country' && country
                  ? countryName(country, i18n.language)
                  : t('leaderboard.scope.world'),
            })}
          </caption>
          <thead>
            <tr>
              <th scope="col">{t('leaderboard.col.rank')}</th>
              <th scope="col">{t('leaderboard.col.player')}</th>
              <th scope="col">{t('leaderboard.col.rating')}</th>
              <th scope="col">{t('leaderboard.col.games')}</th>
            </tr>
          </thead>
          <tbody>
            {board.entries.map((e) => (
              <Row
                key={e.player.user_id}
                entry={e}
                you={e.player.user_id === board.you?.player.user_id}
                movement={!!board.movement_since}
              />
            ))}
            {board.you && !youInTop && (
              <Row entry={board.you} you separated movement={!!board.movement_since} />
            )}
          </tbody>
        </table>
      )}
      {board?.your_best_rank !== undefined && (
        <p className="note">{t('leaderboard.best_rank', { rank: board.your_best_rank })}</p>
      )}
      {board && !board.you && board.entries.length > 0 && (
        <p className="note">
          {t('leaderboard.not_listed', {
            games: LEADERBOARD_MIN_GAMES,
            days: LEADERBOARD_ACTIVE_DAYS,
          })}
        </p>
      )}
    </>
  );
}

function Row({
  entry,
  you = false,
  separated = false,
  movement = false,
}: {
  entry: LeaderboardEntry;
  you?: boolean;
  separated?: boolean;
  /** Whether the board has an earlier snapshot to compare with. */
  movement?: boolean;
}) {
  const { t, i18n } = useTranslation();
  const classes = [you && 'leaderboard__you', separated && 'leaderboard__gap']
    .filter(Boolean)
    .join(' ');
  return (
    <tr className={classes || undefined} aria-current={you ? 'true' : undefined}>
      <td>
        {entry.rank}
        {movement && <Movement change={entry.change} />}
      </td>
      <td>
        <Link className="leaderboard__player" to={`/players/${entry.player.user_id}`}>
          <strong>{entry.player.display_name}</strong>
        </Link>
        {entry.player.handle && <span className="note"> @{entry.player.handle}</span>}
        {entry.country && (
          <span className="leaderboard__country">
            <span aria-hidden="true">{countryFlag(entry.country)} </span>
            {countryName(entry.country, i18n.language)}
          </span>
        )}
        {you && <span className="note"> · {t('leaderboard.you')}</span>}
      </td>
      <td>{entry.rating}</td>
      <td>{entry.games}</td>
    </tr>
  );
}

/** Places gained or lost since the last snapshot, or "new". */
function Movement({ change }: { change?: number }) {
  const { t } = useTranslation();
  if (change === undefined) {
    return (
      <span className="leaderboard__move leaderboard__move--new">{t('leaderboard.move.new')}</span>
    );
  }
  if (change === 0) return null;
  const up = change > 0;
  return (
    <span
      className={`leaderboard__move leaderboard__move--${up ? 'up' : 'down'}`}
      aria-label={t(up ? 'leaderboard.move.up' : 'leaderboard.move.down', {
        count: Math.abs(change),
      })}
    >
      <span aria-hidden="true">{up ? '▲' : '▼'}</span>
      <span aria-hidden="true">{Math.abs(change)}</span>
    </span>
  );
}

/**
 * The seasonal points board: a calendar month (UTC), worldwide or for your country. Points
 * come from online games that counted for ratings, and (a few, capped per day) from wins
 * against the computer.
 */
function PointsView() {
  const { t, i18n } = useTranslation();
  const { api, user } = useApp();
  const [params, setParams] = useSearchParams();
  const season = params.get('season') ?? undefined;
  const scope: LeaderboardScope = params.get('scope') === 'country' ? 'country' : 'world';
  const country = user?.country ?? null;
  const needsCountry = scope === 'country' && !country;
  const boardCountry = scope === 'country' ? (country ?? undefined) : undefined;
  const key = `${season ?? ''}|${scope}|${boardCountry ?? ''}`;
  const [loaded, setLoaded] = useState<{ key: string; board?: PointsBoard; error?: string } | null>(
    null,
  );
  const board = loaded?.key === key ? (loaded.board ?? null) : null;
  const error = loaded?.key === key ? (loaded.error ?? null) : null;
  const months = new Intl.DateTimeFormat(i18n.language, {
    month: 'long',
    year: 'numeric',
    timeZone: 'UTC',
  });
  const seasonName = (s: string) => months.format(new Date(`${s}-01T00:00:00Z`));

  const choose = (next: { season?: string; scope?: LeaderboardScope }) => {
    const s = next.scope ?? scope;
    const se = next.season ?? season;
    setParams(
      {
        board: 'points',
        ...(se ? { season: se } : {}),
        ...(s === 'world' ? {} : { scope: s }),
      },
      { replace: true },
    );
  };

  useEffect(() => {
    if (needsCountry) return;
    let cancelled = false;
    void api()
      .points(scope, season, boardCountry)
      .then((b) => !cancelled && setLoaded({ key, board: b }))
      .catch(
        (e: unknown) =>
          !cancelled && setLoaded({ key, error: e instanceof ApiError ? e.code : 'network' }),
      );
    return () => {
      cancelled = true;
    };
  }, [api, season, scope, boardCountry, key, needsCountry]);

  const youInTop = !!board?.you && board.entries.some((e) => e.rank === board.you!.rank);

  return (
    <>
      <p className="note">
        {t('leaderboard.points.how', {
          win: POINTS_ONLINE[0],
          draw: POINTS_ONLINE[1],
          loss: POINTS_ONLINE[2],
          easy: POINTS_AI_WIN[0],
          medium: POINTS_AI_WIN[1],
          hard: POINTS_AI_WIN[2],
          cap: POINTS_AI_DAILY_CAP,
        })}
      </p>
      <div className="leaderboard__controls">
        <label className="field">
          <span>{t('leaderboard.points.season')}</span>
          <select
            value={board?.season ?? season ?? ''}
            onChange={(e) => choose({ season: e.target.value })}
            disabled={!board}
          >
            {(board?.seasons ?? (season ? [season] : [])).map((s) => (
              <option key={s} value={s}>
                {seasonName(s)}
              </option>
            ))}
          </select>
        </label>
        <div className="choice-row" role="group" aria-label={t('leaderboard.scope')}>
          {(['world', 'country'] as const).map((s) => (
            <button
              key={s}
              type="button"
              className="choice rules__tab"
              aria-pressed={scope === s}
              onClick={() => choose({ scope: s })}
            >
              {s === 'country' && country
                ? countryName(country, i18n.language)
                : t(`leaderboard.scope.${s}`)}
            </button>
          ))}
        </div>
      </div>

      {needsCountry ? (
        <div className="card">
          <p>{t('leaderboard.no_country')}</p>
          <Link className="button" to="/settings">
            {t('leaderboard.set_country')}
          </Link>
        </div>
      ) : error ? (
        <p className="error" role="alert">
          {t(`error.${error}`, { defaultValue: t('error.internal') })}
        </p>
      ) : !board ? (
        <p role="status">{t('common.loading')}</p>
      ) : board.entries.length === 0 ? (
        <p className="note">{t('leaderboard.points.empty')}</p>
      ) : (
        <table className="leaderboard__table">
          <caption className="visually-hidden">
            {t('leaderboard.points.caption', { season: seasonName(board.season) })}
          </caption>
          <thead>
            <tr>
              <th scope="col">{t('leaderboard.col.rank')}</th>
              <th scope="col">{t('leaderboard.col.player')}</th>
              <th scope="col">{t('leaderboard.col.points')}</th>
              <th scope="col">{t('leaderboard.col.games')}</th>
            </tr>
          </thead>
          <tbody>
            {board.entries.map((e) => (
              <PointsRow
                key={e.player.user_id}
                entry={e}
                you={e.player.user_id === board.you?.player.user_id}
              />
            ))}
            {board.you && !youInTop && <PointsRow entry={board.you} you separated />}
          </tbody>
        </table>
      )}
    </>
  );
}

function PointsRow({
  entry,
  you = false,
  separated = false,
}: {
  entry: PointsEntry;
  you?: boolean;
  separated?: boolean;
}) {
  const { t, i18n } = useTranslation();
  const classes = [you && 'leaderboard__you', separated && 'leaderboard__gap']
    .filter(Boolean)
    .join(' ');
  return (
    <tr className={classes || undefined} aria-current={you ? 'true' : undefined}>
      <td>{entry.rank}</td>
      <td>
        <Link className="leaderboard__player" to={`/players/${entry.player.user_id}`}>
          <strong>{entry.player.display_name}</strong>
        </Link>
        {entry.player.handle && <span className="note"> @{entry.player.handle}</span>}
        {entry.country && (
          <span className="leaderboard__country">
            <span aria-hidden="true">{countryFlag(entry.country)} </span>
            {countryName(entry.country, i18n.language)}
          </span>
        )}
        {you && <span className="note"> · {t('leaderboard.you')}</span>}
      </td>
      <td>{entry.points}</td>
      <td>{entry.online_games + entry.ai_wins}</td>
    </tr>
  );
}
