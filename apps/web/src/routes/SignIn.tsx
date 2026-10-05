import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate } from 'react-router';

import { ApiError } from '../api/client';
import { useApp } from '../state/useApp';

/**
 * Optional sign-in (never required to play).
 *
 * Google and Apple are placeholders until real client IDs exist: the buttons are disabled and
 * no request is made (the server would answer 501 `not_configured`). In dev builds
 * (`import.meta.env.DEV`) a "Dev sign-in" button calls POST /v1/auth/dev (server feature
 * `dev-auth`, debug builds only). Production bundles don't include that button.
 */
// A compile-time constant, so production bundles drop the dev sign-in UI entirely.
const DEV_SIGN_IN = import.meta.env.DEV;

export function SignIn() {
  const { t } = useTranslation();
  const app = useApp();
  const navigate = useNavigate();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (app.auth === 'signed_in') {
    return (
      <section>
        <h1>{t('account.sign_in')}</h1>
        <p>{t('account.signed_in_as', { name: app.user?.display_name ?? '' })}</p>
        <Link className="button" to="/settings">
          {t('menu.settings')}
        </Link>
      </section>
    );
  }

  const dev = async () => {
    setBusy(true);
    setError(null);
    try {
      await app.signInDev();
      navigate('/stats');
    } catch (e) {
      setError(e instanceof ApiError ? e.code : 'network');
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="sign-in">
      <h1>{t('account.sign_in')}</h1>
      <p>{t('account.sign_in.why')}</p>
      {app.auth === 'expired' && <p className="note">{t('account.expired')}</p>}
      <div className="stack">
        <button type="button" className="button" disabled aria-describedby="sign-in-unavailable">
          {t('account.sign_in.google')} <span className="badge">{t('common.coming_soon')}</span>
        </button>
        <button type="button" className="button" disabled aria-describedby="sign-in-unavailable">
          {t('account.sign_in.apple')} <span className="badge">{t('common.coming_soon')}</span>
        </button>
      </div>
      <p id="sign-in-unavailable" className="note">
        {t('account.sign_in.unavailable')}
      </p>

      {DEV_SIGN_IN && (
        <div className="card dev-only">
          <p className="note">{t('account.sign_in.dev.note')}</p>
          <button type="button" className="button" onClick={() => void dev()} disabled={busy}>
            {t('account.sign_in.dev')}
          </button>
        </div>
      )}
      {error && (
        <p className="error" role="alert">
          {t(`error.${error}`, { defaultValue: t('error.internal') })}
        </p>
      )}
      <Link to="/">{t('account.keep_playing')}</Link>
    </section>
  );
}
