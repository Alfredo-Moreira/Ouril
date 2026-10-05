/** App-wide state context (provided by AppProvider). */
import { createContext, useContext } from 'react';

import type { Me } from '../api/types';
import type { Settings, TelemetryConsent } from '../storage/repo';
import type { SyncStatus } from '../sync/syncEngine';
import type { UpdateStatus } from '../util/version';

export type AuthState = 'guest' | 'signed_in' | 'expired';

/** Why the last profile change didn't go through (from the server's rejection). */
export type ProfileNotice = 'handle_taken' | 'invalid';

export interface AppContextValue {
  settings: Settings;
  updateSettings(patch: Partial<Settings>): Promise<void>;
  consent: TelemetryConsent;
  saveConsent(choice: { crashReports: boolean; usageStats: boolean }): Promise<void>;
  resetInstallId(): Promise<void>;
  auth: AuthState;
  user: Me | null;
  syncStatus: SyncStatus | null;
  /** Dev sign-in (debug builds with server feature `dev-auth`). */
  signInDev(user?: string): Promise<void>;
  signOut(): Promise<void>;
  deleteAccount(): Promise<void>;
  /** Call after a local change (finished game, settings) so a signed-in player syncs soon. */
  notifyLocalChange(): void;
  /** Increments when local data changed from elsewhere (pull, sign-in); re-read lists then. */
  dataVersion: number;
  /**
   * This build against the server's `/v1/meta` (only known once signed in; guests never ask).
   * `required`: online features are off until the app updates.
   */
  update: UpdateStatus | null;
  /**
   * Queue a profile change (signed in only). Values are normalized here; returns false and
   * changes nothing if one is invalid. The handle stays pending until the server confirms it.
   */
  updateProfile(change: { display_name?: string; handle?: string }): Promise<boolean>;
  /** A requested handle the server hasn't confirmed yet. */
  pendingHandle: string | null;
  profileNotice: ProfileNotice | null;
  clearProfileNotice(): void;
}

export const AppContext = createContext<AppContextValue | null>(null);

export function useApp(): AppContextValue {
  const value = useContext(AppContext);
  if (!value) throw new Error('useApp() outside <AppProvider>');
  return value;
}
