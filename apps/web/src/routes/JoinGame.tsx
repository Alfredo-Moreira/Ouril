import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate, useParams } from 'react-router';

import { ApiError } from '../api/client';
import { rememberReturnTo } from '../auth/returnTo';
import { trackJoin } from '../online/analytics';
import { useApp } from '../state/useApp';

/**
 * `/join/:code`: an invite link to a game against another player (ADR 0031). A signed-in
 * player joins straight away and lands on the game; a guest is asked to sign in first and
 * comes back here afterwards.
 */
export function JoinGame() {
  const { t } = useTranslation();
  const { code = '' } = useParams();
  const { auth, api } = useApp();
  const navigate = useNavigate();
  const [error, setError] = useState<string | null>(null);
  // Joining uses the link up: never twice, even when React runs effects twice in development.
  const sent = useRef(false);

  useEffect(() => {
    if (auth === 'guest' || sent.current) return;
    sent.current = true;
    void api()
      .joinGame(code)
      .then((game) => {
        trackJoin();
        navigate(`/online/${game.id}`, { replace: true });
      })
      .catch((e: unknown) => setError(e instanceof ApiError ? e.code : 'network'));
  }, [api, auth, code, navigate]);

  if (auth === 'guest') {
    return (
      <section className="sign-in">
        <h1>{t('online.join.title')}</h1>
        <p>{t('online.join.sign_in')}</p>
        <Link
          className="button"
          to="/sign-in"
          onClick={() => rememberReturnTo(`/join/${encodeURIComponent(code)}`)}
        >
          {t('account.sign_in')}
        </Link>
      </section>
    );
  }

  return (
    <section className="sign-in">
      <h1>{t('online.join.title')}</h1>
      {error ? (
        <>
          <p className="error" role="alert">
            {t(`online.join.error.${error}`, {
              defaultValue: t(`error.${error}`, { defaultValue: t('error.internal') }),
            })}
          </p>
          <Link to="/game?tab=online">{t('online.back')}</Link>
        </>
      ) : (
        <p role="status">{t('online.join.working')}</p>
      )}
    </section>
  );
}
