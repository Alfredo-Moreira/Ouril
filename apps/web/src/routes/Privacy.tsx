import { useTranslation } from 'react-i18next';

import { PRIVACY, PRIVACY_SECTIONS } from '../content/privacy';

/**
 * `/privacy`: the privacy policy. Available to everyone, offline too: it's part of the app, so
 * reading it makes no request. Content: `content/privacy.ts` + `privacy.*` strings.
 */
export function Privacy() {
  const { t, i18n } = useTranslation();
  const date = new Intl.DateTimeFormat(i18n.language, { dateStyle: 'long' }).format(
    new Date(`${PRIVACY.updated}T12:00:00Z`),
  );
  const values = { name: PRIVACY.controller, email: PRIVACY.email, days: PRIVACY.backupDays };

  return (
    <article className="privacy">
      <h1>{t('privacy.title')}</h1>
      <p className="note">{t('privacy.updated', { date })}</p>
      {PRIVACY_SECTIONS.map((section) => (
        <section key={section.id} aria-labelledby={`privacy-${section.id}`}>
          <h2 id={`privacy-${section.id}`}>{t(`privacy.${section.id}.title`)}</h2>
          {Array.from({ length: section.paragraphs }, (_, i) => (
            <p key={i}>{t(`privacy.${section.id}.${i + 1}`, values)}</p>
          ))}
        </section>
      ))}
      <p>
        {t('privacy.contact')} <a href={`mailto:${PRIVACY.email}`}>{PRIVACY.email}</a>
      </p>
    </article>
  );
}
