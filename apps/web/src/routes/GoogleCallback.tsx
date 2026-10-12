import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate } from 'react-router';

import { ApiError } from '../api/client';
import { GOOGLE_CALLBACK_PATH, takeGoogleCallback, type GoogleCallback } from '../auth/google';
import { FEATURES } from '../features';
import { useApp } from '../state/useApp';

type Status = 'working' | 'cancelled' | 'failed';

function initialStatus(): { status: Status; callback: GoogleCallback } {
  const callback = takeGoogleCallback(window.location.hash);
  const status =
    callback.kind === 'ok' ? 'working' : callback.kind === 'cancelled' ? 'cancelled' : 'failed';
  return { status, callback };
}

/**
 * `/auth/google`: Google sends the player back here with the ID token in the URL fragment
 * (`auth/google.ts`). The fragment is removed from the address bar and history straight away,
 * then the token and nonce go to `POST /v1/auth/google`.
 */
export function GoogleCallback() {
  const { t } = useTranslation();
  const app = useApp();
  const navigate = useNavigate();
  const [{ status: initial, callback }] = useState(initialStatus);
  const [status, setStatus] = useState<Status>(initial);
  const [error, setError] = useState<string | null>(null);
  // One request per callback, even when React runs effects twice in development.
  const sent = useRef(false);

  useEffect(() => {
    // Keep the token out of the address bar and history.
    if (window.location.hash) {
      window.history.replaceState(window.history.state, '', GOOGLE_CALLBACK_PATH);
    }
    if (callback.kind !== 'ok' || sent.current) return;
    sent.current = true;
    app
      .signInGoogle(callback.idToken, callback.nonce)
      .then(() => navigate(FEATURES.stats ? '/stats' : '/', { replace: true }))
      .catch((e: unknown) => {
        setError(e instanceof ApiError ? e.code : 'network');
        setStatus('failed');
      });
  }, [app, callback, navigate]);

  return (
    <section className="sign-in">
      <h1>{t('account.sign_in')}</h1>
      {status === 'working' && (
        <p role="status" aria-live="polite">
          {t('account.sign_in.google.working')}
        </p>
      )}
      {status === 'cancelled' && <p className="note">{t('account.sign_in.google.cancelled')}</p>}
      {status === 'failed' && (
        <p className="error" role="alert">
          {error
            ? t(`error.${error}`, { defaultValue: t('error.internal') })
            : t('account.sign_in.google.failed')}
        </p>
      )}
      {status !== 'working' && (
        <div className="stack">
          <Link className="button" to="/sign-in">
            {t('account.sign_in.try_again')}
          </Link>
          <Link to="/">{t('account.keep_playing')}</Link>
        </div>
      )}
    </section>
  );
}
