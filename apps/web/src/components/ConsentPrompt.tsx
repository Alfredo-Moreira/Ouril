import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useApp } from '../state/useApp';
import { Dialog } from './Dialog';

/**
 * First-launch telemetry choices (ADR 0014): two separate options, both off by default.
 * Declining is one tap and as prominent as accepting.
 */
export function ConsentPrompt() {
  const { t } = useTranslation();
  const { consent, saveConsent } = useApp();
  const [crashReports, setCrash] = useState(false);
  const [usageStats, setUsage] = useState(false);

  if (consent.decided) return null;

  return (
    <Dialog title={t('telemetry.prompt.title')}>
      <p>{t('telemetry.prompt.body')}</p>
      <label className="toggle">
        <input
          type="checkbox"
          checked={crashReports}
          onChange={(e) => setCrash(e.target.checked)}
        />
        <span>
          <strong>{t('telemetry.crash.title')}</strong>
          <br />
          {t('telemetry.crash.body')}
        </span>
      </label>
      <label className="toggle">
        <input type="checkbox" checked={usageStats} onChange={(e) => setUsage(e.target.checked)} />
        <span>
          <strong>{t('telemetry.usage.title')}</strong>
          <br />
          {t('telemetry.usage.body')}
        </span>
      </label>
      <p className="note">{t('telemetry.prompt.later')}</p>
      <div className="dialog__actions dialog__actions--equal">
        <button
          type="button"
          className="button"
          onClick={() => void saveConsent({ crashReports: false, usageStats: false })}
        >
          {t('telemetry.prompt.decline')}
        </button>
        <button
          type="button"
          className="button"
          onClick={() => void saveConsent({ crashReports, usageStats })}
        >
          {t('telemetry.prompt.save')}
        </button>
      </div>
    </Dialog>
  );
}
