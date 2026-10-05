import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';

import { useCore } from '../engine/CoreContext';
import { LEVELS, type Stats as CoreStats, type WinLoss } from '../engine/types';
import { outcomeFor } from '../game/session';
import { useApp } from '../state/useApp';
import type { GameRow } from '../storage/db';
import { listGames } from '../storage/repo';

const RECENT = 20;

/** Local stats (derived by the core from game records, never stored as counters) + history. */
export function Stats() {
  const { t, i18n } = useTranslation();
  const { core } = useCore();
  const { auth, dataVersion } = useApp();
  const [rows, setRows] = useState<GameRow[] | null>(null);

  useEffect(() => {
    let cancelled = false;
    void listGames().then((r) => {
      if (!cancelled) setRows(r);
    });
    return () => {
      cancelled = true;
    };
  }, [dataVersion]);

  if (!rows) return <p role="status">{t('common.loading')}</p>;

  const stats: CoreStats = core.deriveStats(rows.map((r) => r.record));
  const dateFmt = new Intl.DateTimeFormat(i18n.language, {
    dateStyle: 'medium',
    timeStyle: 'short',
  });

  return (
    <section className="stats">
      <h1>{t('stats.title')}</h1>
      {auth === 'guest' && (
        <p className="note">
          {t('stats.guest_note')} <Link to="/sign-in">{t('account.sign_in')}</Link>
        </p>
      )}

      {stats.overall.played === 0 ? (
        <p>{t('stats.empty')}</p>
      ) : (
        <>
          <dl className="stat-tiles">
            <Tile label={t('stats.played')} value={stats.overall.played} />
            <Tile label={t('stats.wins')} value={stats.overall.wins} />
            <Tile label={t('stats.losses')} value={stats.overall.losses} />
            <Tile label={t('stats.draws')} value={stats.overall.draws} />
            <Tile label={t('stats.current_streak')} value={stats.current_streak} />
            <Tile label={t('stats.best_streak')} value={stats.best_streak} />
          </dl>

          <h2>{t('stats.by_level')}</h2>
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th scope="col">{t('stats.level')}</th>
                  <th scope="col">{t('stats.played')}</th>
                  <th scope="col">{t('stats.wins')}</th>
                  <th scope="col">{t('stats.losses')}</th>
                  <th scope="col">{t('stats.draws')}</th>
                </tr>
              </thead>
              <tbody>
                {LEVELS.map((level) => {
                  const wl: WinLoss = stats.by_level[level] ?? {
                    played: 0,
                    wins: 0,
                    losses: 0,
                    draws: 0,
                  };
                  return (
                    <tr key={level}>
                      <th scope="row">{t(`game.level.${level}`)}</th>
                      <td>{wl.played}</td>
                      <td>{wl.wins}</td>
                      <td>{wl.losses}</td>
                      <td>{wl.draws}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          <h2>{t('stats.recent')}</h2>
          <ol className="history">
            {rows.slice(0, RECENT).map(({ record, syncRejected }) => {
              const me = record.human_player ?? 'south';
              const outcome = outcomeFor(record.result.outcome, me);
              const [south, north] = record.result.stores;
              const mine = me === 'south' ? south : north;
              const theirs = me === 'south' ? north : south;
              return (
                <li key={record.id} className={`history__item history__item--${outcome}`}>
                  <span className="history__result">{t(`stats.result.${outcome}`)}</span>
                  <span>{t('stats.score', { you: mine, ai: theirs })}</span>
                  <span>{record.ai_level ? t(`game.level.${record.ai_level}`) : ''}</span>
                  <time dateTime={record.ended_at}>
                    {dateFmt.format(new Date(record.ended_at))}
                  </time>
                  {syncRejected && <span className="note">{t('stats.not_backed_up')}</span>}
                  <Link to={`/replay/${record.id}`} className="history__replay">
                    {t('replay.open')}
                  </Link>
                </li>
              );
            })}
          </ol>
        </>
      )}
    </section>
  );
}

function Tile({ label, value }: { label: string; value: number }) {
  return (
    <div className="stat-tile">
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}
