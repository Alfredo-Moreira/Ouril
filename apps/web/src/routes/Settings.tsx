import { useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';

import { ApiError } from '../api/client';
import { hasMusic } from '../audio/music';
import { useMusicPrefs } from '../audio/musicPrefs';
import { FEATURES } from '../features';
import { SUPPORTED_LOCALES } from '../i18n';
import { useApp } from '../state/useApp';
import { crashReportsConfigured, telemetryConfigured, usageStatsConfigured } from '../telemetry';

export function Settings() {
  const { t } = useTranslation();
  const app = useApp();
  const { settings, consent } = app;
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(e instanceof ApiError ? e.code : 'network');
    } finally {
      setBusy(false);
    }
  };

  const [music, setMusicPrefs] = useMusicPrefs();

  return (
    <section className="settings">
      <h1>{t('settings.title')}</h1>

      <fieldset className="card">
        <legend>{t('settings.game')}</legend>
        <label className="toggle">
          <input
            type="checkbox"
            checked={settings.sound}
            onChange={(e) => void app.updateSettings({ sound: e.target.checked })}
          />
          <span>{t('settings.sound')}</span>
        </label>
        <label className="toggle">
          <input
            type="checkbox"
            checked={settings.hints}
            onChange={(e) => void app.updateSettings({ hints: e.target.checked })}
          />
          <span>{t('settings.hints')}</span>
        </label>
        <label className="field">
          <span>{t('settings.language')}</span>
          <select
            value={settings.language}
            onChange={(e) => void app.updateSettings({ language: e.target.value })}
          >
            {SUPPORTED_LOCALES.map((l) => (
              <option key={l} value={l}>
                {t(`settings.language.${l}`)}
              </option>
            ))}
          </select>
        </label>
      </fieldset>

      <fieldset className="card">
        <legend>{t('settings.music')}</legend>
        <label className="toggle">
          <input
            type="checkbox"
            checked={music.on}
            onChange={(e) => void setMusicPrefs({ on: e.target.checked })}
          />
          <span>{t('settings.music.game')}</span>
        </label>
        <label className="field">
          <span>{t('settings.music.volume')}</span>
          <input
            type="range"
            min={0}
            max={100}
            step={5}
            value={Math.round(music.volume * 100)}
            disabled={!music.on}
            onChange={(e) => void setMusicPrefs({ volume: Number(e.target.value) / 100 })}
          />
        </label>
        <p className="note">{t(hasMusic() ? 'settings.music.note' : 'settings.music.none')}</p>
      </fieldset>

      {/* Only when a telemetry service is configured (ADR 0014). */}
      {telemetryConfigured() && (
        <fieldset className="card">
          <legend>{t('settings.privacy')}</legend>
          {crashReportsConfigured() && (
            <>
              <label className="toggle">
                <input
                  type="checkbox"
                  checked={consent.crashReports}
                  onChange={(e) =>
                    void app.saveConsent({
                      crashReports: e.target.checked,
                      usageStats: consent.usageStats,
                    })
                  }
                />
                <span>{t('telemetry.crash.title')}</span>
              </label>
              <p className="note">{t('telemetry.crash.body')}</p>
            </>
          )}
          {usageStatsConfigured() && (
            <>
              <label className="toggle">
                <input
                  type="checkbox"
                  checked={consent.usageStats}
                  onChange={(e) =>
                    void app.saveConsent({
                      crashReports: consent.crashReports,
                      usageStats: e.target.checked,
                    })
                  }
                />
                <span>{t('telemetry.usage.title')}</span>
              </label>
              <p className="note">{t('telemetry.usage.body')}</p>
            </>
          )}
          <button type="button" className="button" onClick={() => void app.resetInstallId()}>
            {t('settings.reset_install_id')}
          </button>
        </fieldset>
      )}

      {FEATURES.accounts && (
        <fieldset className="card">
          <legend>{t('settings.account')}</legend>
          {app.auth === 'guest' ? (
            <>
              <p>{t('account.guest')}</p>
              <Link className="button" to="/sign-in">
                {t('account.sign_in')}
              </Link>
            </>
          ) : (
            <>
              <p>{t('account.signed_in_as', { name: app.user?.display_name ?? '' })}</p>
              {app.auth === 'expired' && <p className="note">{t('account.expired')}</p>}
              {app.user && <ProfileForm key={app.user.id} />}
              {app.syncStatus && <p className="note">{t(`sync.status.${app.syncStatus}`)}</p>}
              <div className="row">
                <button
                  type="button"
                  className="button"
                  disabled={busy}
                  onClick={() => void run(app.signOut)}
                >
                  {t('account.sign_out')}
                </button>
                {!confirmDelete ? (
                  <button
                    type="button"
                    className="button button--danger"
                    onClick={() => setConfirmDelete(true)}
                  >
                    {t('account.delete')}
                  </button>
                ) : (
                  <div className="confirm" role="group" aria-label={t('account.delete')}>
                    <p>{t('account.delete.confirm')}</p>
                    <button
                      type="button"
                      className="button button--danger"
                      disabled={busy}
                      onClick={() =>
                        void run(app.deleteAccount).then(() => setConfirmDelete(false))
                      }
                    >
                      {t('account.delete.yes')}
                    </button>
                    <button
                      type="button"
                      className="button"
                      onClick={() => setConfirmDelete(false)}
                    >
                      {t('common.cancel')}
                    </button>
                  </div>
                )}
              </div>
            </>
          )}
          {error && (
            <p className="error" role="alert">
              {t(`error.${error}`, { defaultValue: t('error.internal') })}
            </p>
          )}
        </fieldset>
      )}
    </section>
  );
}

/** Display name and handle. Changes are queued and synced (offline-first, data-and-sync.md). */
function ProfileForm() {
  const { t } = useTranslation();
  const app = useApp();
  const user = app.user!;
  const [name, setName] = useState(user.display_name);
  const [handle, setHandle] = useState(user.handle ?? '');
  const [invalid, setInvalid] = useState(false);
  const [saved, setSaved] = useState(false);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setSaved(false);
    const ok = await app.updateProfile({
      display_name: name,
      handle: handle.trim() === '' ? undefined : handle,
    });
    setInvalid(!ok);
    setSaved(ok);
  };

  return (
    <form className="profile" onSubmit={(e) => void submit(e)} noValidate>
      <h3>{t('profile.title')}</h3>
      <label>
        <span>{t('profile.display_name')}</span>
        <input
          type="text"
          value={name}
          maxLength={50}
          autoComplete="nickname"
          onChange={(e) => setName(e.target.value)}
        />
      </label>
      <label>
        <span>{t('profile.handle')}</span>
        <input
          type="text"
          value={handle}
          maxLength={21}
          autoComplete="username"
          spellCheck={false}
          aria-describedby="handle-help"
          onChange={(e) => setHandle(e.target.value)}
        />
      </label>
      <p id="handle-help" className="note">
        {t('profile.handle.help')}
      </p>
      <button type="submit" className="button">
        {t('profile.save')}
      </button>
      <div aria-live="polite">
        {invalid && (
          <p className="error" role="alert">
            {t('profile.invalid')}
          </p>
        )}
        {saved && !invalid && <p className="note">{t('profile.saved')}</p>}
        {app.pendingHandle && (
          <p className="note">{t('profile.handle.pending', { handle: app.pendingHandle })}</p>
        )}
        {app.profileNotice && (
          <p className="error" role="alert">
            {t(app.profileNotice === 'handle_taken' ? 'profile.handle.taken' : 'profile.rejected')}
          </p>
        )}
      </div>
    </form>
  );
}
