import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useSearchParams } from 'react-router';

import { ApiError } from '../api/client';
import {
  LEADERBOARD_ACTIVE_DAYS,
  LEADERBOARD_MIN_GAMES,
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
    <section className="leaderboard">
      <h1>{t('leaderboard.title')}</h1>

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
              />
            ))}
            {board.you && !youInTop && <Row entry={board.you} you separated />}
          </tbody>
        </table>
      )}
      {board && !board.you && board.entries.length > 0 && (
        <p className="note">
          {t('leaderboard.not_listed', {
            games: LEADERBOARD_MIN_GAMES,
            days: LEADERBOARD_ACTIVE_DAYS,
          })}
        </p>
      )}
    </section>
  );
}

function Row({
  entry,
  you = false,
  separated = false,
}: {
  entry: LeaderboardEntry;
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
        <strong>{entry.player.display_name}</strong>
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
