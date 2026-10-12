import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';

import { PRIVACY } from '../content/privacy';
import { useApp } from '../state/useApp';

/**
 * `/delete-account`: how to delete an Ouril account, and what's deleted or kept. Google Play
 * requires a web page like this one; the deletion itself happens in Settings (`DELETE /v1/me`)
 * or by email.
 */
export function DeleteAccount() {
  const { t } = useTranslation();
  const app = useApp();
  const values = { email: PRIVACY.email, days: PRIVACY.backupDays };

  return (
    <article className="privacy">
      <h1>{t('delete_account.title')}</h1>
      <p>{t('delete_account.intro')}</p>

      <h2>{t('delete_account.how.title')}</h2>
      <ol>
        <li>{t('delete_account.how.1')}</li>
        <li>{t('delete_account.how.2')}</li>
      </ol>
      {app.auth === 'guest' ? (
        <Link className="button" to="/sign-in">
          {t('account.sign_in')}
        </Link>
      ) : (
        <Link className="button" to="/settings">
          {t('delete_account.go_settings')}
        </Link>
      )}

      <h2>{t('delete_account.email.title')}</h2>
      <p>{t('delete_account.email.body', values)}</p>
      <p>
        <a href={`mailto:${PRIVACY.email}`}>{PRIVACY.email}</a>
      </p>

      <h2>{t('delete_account.what.title')}</h2>
      <p>{t('delete_account.what.1')}</p>
      <p>{t('delete_account.what.2')}</p>

      <h2>{t('delete_account.kept.title')}</h2>
      <p>{t('delete_account.kept.1', values)}</p>

      <p>
        <Link to="/privacy">{t('delete_account.privacy_link')}</Link>
      </p>
    </article>
  );
}
