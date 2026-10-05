/**
 * App-wide state: settings, telemetry consent, the optional account and sync.
 *
 * Guest-first: on startup nothing here touches the network. An ApiClient and SyncEngine are
 * only created once the player signs in (or was signed in before this launch).
 */
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';

import { ApiClient } from '../api/client';
import type { Me } from '../api/types';
import { getDb } from '../storage/db';
import {
  claimForUser,
  clearStoredAccount,
  DEFAULT_SETTINGS,
  ensureGuestOwner,
  getActiveOwner,
  getConsent,
  getSettings,
  getStoredAccount,
  KV,
  queueProfileChange,
  setConsent,
  setKv,
  setStoredAccount,
  updateSettings,
  type Settings,
  type TelemetryConsent,
} from '../storage/repo';
import { SyncEngine, type SyncStatus } from '../sync/syncEngine';
import { configureTelemetry } from '../telemetry';
import { uuidv7 } from '../util/uuid';
import { normalizeDisplayName, normalizeHandle } from '../util/profile';
import { clientHeader, updateStatus, type UpdateStatus } from '../util/version';
import i18n from '../i18n';
import { AppContext, type AppContextValue, type AuthState, type ProfileNotice } from './useApp';

export interface AppProviderProps {
  children: ReactNode;
  /** Creates the API client on first sign-in. Tests pass a fake. */
  createApi?: () => ApiClient;
  /** Core version for `X-Ouril-Client`. */
  coreVersion?: string;
}

export function AppProvider({ children, createApi, coreVersion = '0.0.0' }: AppProviderProps) {
  const [loaded, setLoaded] = useState(false);
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [consent, setConsentState] = useState<TelemetryConsent>({
    decided: false,
    crashReports: false,
    usageStats: false,
    installId: '',
  });
  const [auth, setAuth] = useState<AuthState>('guest');
  const [user, setUser] = useState<Me | null>(null);
  const [syncStatus, setSyncStatus] = useState<SyncStatus | null>(null);
  const [dataVersion, setDataVersion] = useState(0);
  const [update, setUpdate] = useState<UpdateStatus | null>(null);
  const [pendingHandle, setPendingHandle] = useState<string | null>(null);
  const [profileNotice, setProfileNotice] = useState<ProfileNotice | null>(null);

  const apiRef = useRef<ApiClient | null>(null);
  const syncRef = useRef<SyncEngine | null>(null);

  // Props are read once: the API client is created lazily, on the first signed-in action.
  const factoryRef = useRef({ createApi, coreVersion });
  const api = useCallback((): ApiClient => {
    const { createApi: create, coreVersion: version } = factoryRef.current;
    apiRef.current ??=
      create?.() ??
      new ApiClient({
        baseUrl: import.meta.env.VITE_API_BASE_URL ?? '',
        clientHeader: () => clientHeader(version),
      });
    return apiRef.current;
  }, []);

  const startSync = useCallback(() => {
    syncRef.current?.stop();
    const engine = new SyncEngine({
      api: api(),
      onStatus: (s) => {
        setSyncStatus(s);
        if (s === 'idle') setDataVersion((v) => v + 1);
      },
      onRemoteSettings: setSettings,
      onRemoteProfile: (profile) => {
        setUser((u) => {
          if (!u) return u;
          const next = { ...u, ...profile };
          void setStoredAccount({ user: next });
          return next;
        });
        if (profile.handle) setPendingHandle((p) => (p === profile.handle ? null : p));
      },
      onAuthLost: () => setAuth('expired'),
      onUpgradeRequired: () => setUpdate('required'),
      onRejected: (rejections) => {
        for (const r of rejections) {
          if (r.type === 'handle_requested') setPendingHandle(null);
          if (r.type !== 'handle_requested' && r.type !== 'profile_updated') continue;
          setProfileNotice(r.reason === 'handle_taken' ? 'handle_taken' : 'invalid');
        }
      },
    });
    syncRef.current = engine;
    engine.start();
    // Signed in only (guests never reach this): check the app version (versioning.md). A
    // failure just means "unknown"; the sync engine still reacts to a 426.
    void Promise.resolve()
      .then(() => api().meta())
      .then((meta) => {
        const status = updateStatus(meta);
        setUpdate(status);
        if (status === 'required') {
          engine.stop();
          setSyncStatus('upgrade_required');
        }
      })
      .catch(() => undefined);
  }, [api]);

  // Startup: local reads only.
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      await ensureGuestOwner();
      const [s, c, account] = await Promise.all([getSettings(), getConsent(), getStoredAccount()]);
      if (cancelled) return;
      setSettings(s);
      setConsentState(c);
      configureTelemetry(c);
      if (account) {
        setUser(account.user);
        setAuth('signed_in');
        startSync(); // a signed-in player may use the network
      }
      setLoaded(true);
    })();
    return () => {
      cancelled = true;
      syncRef.current?.stop();
    };
  }, [startSync]);

  useEffect(() => {
    if (i18n.language !== settings.language) void i18n.changeLanguage(settings.language);
    document.documentElement.lang = settings.language;
    // The page description (search engines, link previews) follows the language too.
    document
      .querySelector('meta[name="description"]')
      ?.setAttribute('content', i18n.t('app.description'));
  }, [settings.language]);

  const value = useMemo<AppContextValue>(
    () => ({
      settings,
      async updateSettings(patch) {
        setSettings(await updateSettings(patch));
        syncRef.current?.requestSync();
      },
      consent,
      async saveConsent(choice) {
        const next = {
          ...consent,
          ...choice,
          decided: true,
          installId: consent.installId || uuidv7(),
        };
        await setConsent(next);
        setConsentState(next);
        configureTelemetry(next);
      },
      async resetInstallId() {
        const next = { ...consent, installId: uuidv7() };
        await setConsent(next);
        setConsentState(next);
      },
      auth,
      user,
      syncStatus,
      async signInDev(name) {
        const res = await api().devSignIn(name ? { user: name } : {});
        await claimForUser(res.user.id);
        await setStoredAccount({ user: res.user });
        setSettings(await getSettings()); // the account's own settings (or the claimed guest's)
        setUser(res.user);
        setAuth('signed_in');
        setDataVersion((v) => v + 1);
        startSync();
      },
      async signOut() {
        syncRef.current?.stop();
        syncRef.current = null;
        try {
          await api().logout();
        } catch {
          // Offline or already expired: signing out locally is enough.
        }
        await clearStoredAccount();
        setUser(null);
        setAuth('guest');
        setSyncStatus(null);
        setPendingHandle(null);
        setProfileNotice(null);
      },
      async deleteAccount() {
        await api().deleteMe();
        syncRef.current?.stop();
        syncRef.current = null;
        // The account and its server data are gone: drop its local copy and start a fresh guest.
        const db = getDb();
        const owner = await getActiveOwner();
        await db.transaction('rw', db.games, db.outbox, db.kv, async () => {
          await db.games.where('owner').equals(owner).delete();
          await db.outbox.where('owner').equals(owner).delete();
          await db.kv.delete(KV.syncCursor(owner));
          await db.kv.delete(KV.settings(owner));
          await db.kv.delete(KV.currentGame(owner));
          await db.kv.delete(KV.guestId);
          await db.kv.delete(KV.owner);
          await clearStoredAccount();
        });
        const guest = await ensureGuestOwner();
        await setKv(KV.owner, guest);
        setSettings(await getSettings(guest));
        setPendingHandle(null);
        setProfileNotice(null);
        setUser(null);
        setAuth('guest');
        setSyncStatus(null);
        setDataVersion((v) => v + 1);
      },
      notifyLocalChange() {
        syncRef.current?.requestSync();
      },
      dataVersion,
      update,
      async updateProfile(change) {
        if (auth === 'guest' || !user) return false;
        const name =
          change.display_name === undefined ? undefined : normalizeDisplayName(change.display_name);
        const handle = change.handle === undefined ? undefined : normalizeHandle(change.handle);
        if (name === null || handle === null) return false;
        const queued = {
          display_name: name !== undefined && name !== user.display_name ? name : undefined,
          handle: handle !== undefined && handle !== user.handle ? handle : undefined,
        };
        if (queued.display_name === undefined && queued.handle === undefined) return true;
        await queueProfileChange(queued);
        setProfileNotice(null);
        if (queued.display_name !== undefined) {
          // Optimistic: the display name has no uniqueness rule.
          const next = { ...user, display_name: queued.display_name };
          setUser(next);
          await setStoredAccount({ user: next });
        }
        if (queued.handle !== undefined) setPendingHandle(queued.handle);
        syncRef.current?.requestSync();
        return true;
      },
      pendingHandle,
      profileNotice,
      clearProfileNotice() {
        setProfileNotice(null);
      },
    }),
    [
      settings,
      consent,
      auth,
      user,
      syncStatus,
      dataVersion,
      api,
      startSync,
      update,
      pendingHandle,
      profileNotice,
    ],
  );

  if (!loaded) return null;
  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}
