import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useParams } from 'react-router';

import { ApiError } from '../api/client';
import type { Badge, PlayerProfile } from '../api/types';
import { RatingChart } from '../components/RatingChart';
import { countryFlag, countryName } from '../content/countries';
import { poolLabel } from '../online/pools';
import { useApp } from '../state/useApp';

/**
 * `/players/:id` (signed-in players): a player's ratings with their history, badges for top
 * placements, and your record against them, with a button to challenge them.
 */
export function Profile() {
  const { id = '' } = useParams();
  return <ProfileScreen key={id} id={id} />;
}

function ProfileScreen({ id }: { id: string }) {
  const { t, i18n } = useTranslation();
  const { api } = useApp();
  const navigate = useNavigate();
  const [profile, setProfile] = useState<PlayerProfile | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void api()
      .player(id)
      .then((p) => !cancelled && setProfile(p))
      .catch((e: unknown) => !cancelled && setError(e instanceof ApiError ? e.code : 'network'));
    return () => {
      cancelled = true;
    };
  }, [api, id]);

  if (error === 'not_found') {
    return (
      <section className="profile-page">
        <h1>{t('profile.not_found')}</h1>
      </section>
    );
  }
  if (error) {
    return (
      <p className="error" role="alert">
        {t(`error.${error}`, { defaultValue: t('error.internal') })}
      </p>
    );
  }
  if (!profile) return <p role="status">{t('common.loading')}</p>;

  const since = new Intl.DateTimeFormat(i18n.language, { month: 'long', year: 'numeric' }).format(
    new Date(profile.member_since),
  );
  const h2h = profile.head_to_head;
  const challenge = async () => {
    setBusy(true);
    try {
      await api().createGame({ opponent_id: profile.player.user_id, rated: true });
      navigate('/game?tab=online');
    } catch (e) {
      setError(e instanceof ApiError ? e.code : 'network');
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="profile-page">
      <header className="profile-page__head">
        <h1>{profile.player.display_name}</h1>
        <p className="note">
          {profile.player.handle && <span>@{profile.player.handle} · </span>}
          {profile.country && (
            <span>
              <span aria-hidden="true">{countryFlag(profile.country)} </span>
              {countryName(profile.country, i18n.language)} ·{' '}
            </span>
          )}
          {t('profile.since', { date: since })}
        </p>
        {!profile.you && (
          <button
            type="button"
            className="button button--primary"
            onClick={() => void challenge()}
            disabled={busy}
          >
            {t('profile.challenge')}
          </button>
        )}
      </header>

      {h2h && h2h.wins + h2h.draws + h2h.losses > 0 && (
        <p className="card profile-page__h2h">
          {t('profile.h2h', {
            name: profile.player.display_name,
            wins: h2h.wins,
            draws: h2h.draws,
            losses: h2h.losses,
          })}
        </p>
      )}

      {profile.badges.length > 0 && (
        <section aria-labelledby="profile-badges">
          <h2 id="profile-badges">{t('profile.badges')}</h2>
          <ul className="badges">
            {profile.badges.map((b) => (
              <li key={`${b.pool}|${b.scope}`} className="badge">
                <BadgeLabel badge={b} />
              </li>
            ))}
          </ul>
        </section>
      )}

      <section aria-labelledby="profile-ratings">
        <h2 id="profile-ratings">{t('profile.ratings')}</h2>
        {profile.ratings.length === 0 ? (
          <p className="note">{t('profile.no_ratings')}</p>
        ) : (
          <div className="profile-page__ratings">
            {profile.ratings.map((r) => {
              const points = profile.history.find((h) => h.pool === r.pool)?.points ?? [];
              const title = poolLabel(t, r.pool);
              return (
                <article key={r.pool} className="card">
                  <h3 className="card__title">{title}</h3>
                  <p className="profile-page__rating">
                    <strong>
                      {r.provisional
                        ? t('leaderboard.rating.provisional', { rating: r.rating })
                        : r.rating}
                    </strong>{' '}
                    <span className="note">
                      {t('leaderboard.record', { wins: r.wins, draws: r.draws, losses: r.losses })}{' '}
                      · {t('profile.peak', { rating: r.peak })}
                    </span>
                  </p>
                  {points.length > 0 && <RatingChart points={points} title={title} />}
                </article>
              );
            })}
          </div>
        )}
      </section>
    </section>
  );
}

/** "#1 in Cape Verde · Overall", "#4 in the world · Standard · Live". */
function BadgeLabel({ badge }: { badge: Badge }) {
  const { t, i18n } = useTranslation();
  const pool = poolLabel(t, badge.pool);
  return (
    <>
      <span aria-hidden="true">{badge.rank === 1 ? '★ ' : ''}</span>
      {badge.scope === 'world'
        ? t('profile.badge.world', { rank: badge.rank, pool })
        : t('profile.badge.country', {
            rank: badge.rank,
            pool,
            country: countryName(badge.scope, i18n.language),
          })}
    </>
  );
}
