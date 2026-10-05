/**
 * Local data access: settings, consent, the in-progress game, finished games and the outbox.
 * All writes are local and instant. Nothing here touches the network.
 */
import Dexie from 'dexie';

import type { Me, Mutation, MutationType } from '../api/types';
import type { GameRecord, GameState, Level, Player, VariantRef } from '../engine/types';
import { browserLanguage } from '../i18n/language';
import { uuidv7 } from '../util/uuid';
import { getDb, type GameRow } from './db';

// --- kv -------------------------------------------------------------------------------------

export const KV = {
  guestId: 'guest_id',
  owner: 'owner',
  /** Settings and the in-progress game belong to an owner, like games and the outbox. */
  settings: (owner: string) => `settings:${owner}`,
  consent: 'telemetry_consent',
  currentGame: (owner: string) => `current_game:${owner}`,
  account: 'account',
  syncCursor: (owner: string) => `sync_cursor:${owner}`,
} as const;

export async function getKv<T>(key: string): Promise<T | undefined> {
  const row = await getDb().kv.get(key);
  return row?.value as T | undefined;
}

export async function setKv(key: string, value: unknown): Promise<void> {
  await getDb().kv.put({ key, value });
}

export async function deleteKv(key: string): Promise<void> {
  await getDb().kv.delete(key);
}

// --- owner ----------------------------------------------------------------------------------

/** The local guest profile ID, created on first launch. */
export async function ensureGuestOwner(): Promise<string> {
  const db = getDb();
  return db.transaction('rw', db.kv, async () => {
    let id = await getKv<string>(KV.guestId);
    if (!id) {
      id = `guest:${uuidv7()}`;
      await setKv(KV.guestId, id);
    }
    return id;
  });
}

/** Whose data the app shows and writes: the guest ID or the last signed-in user ID. */
export async function getActiveOwner(): Promise<string> {
  return (await getKv<string>(KV.owner)) ?? ensureGuestOwner();
}

export function isGuestOwner(owner: string): boolean {
  return owner.startsWith('guest:');
}

/**
 * Bind local data to `userId` on sign-in (data-and-sync.md, "Guests and sign-in").
 * - Guest data is claimed by the account: games and outbox rows are relabelled (the guest's
 *   outbox gets uploaded), and the guest's settings and in-progress game move to the account
 *   unless this device already holds the account's own.
 * - Same user as before: nothing to do.
 * - A different user: switch to that user's own data (empty on first sign-in). The previous
 *   user's games, outbox, settings and in-progress game stay aside, untouched and unsynced.
 */
export async function claimForUser(userId: string): Promise<'claimed' | 'same' | 'switched'> {
  const db = getDb();
  return db.transaction('rw', db.games, db.outbox, db.kv, async () => {
    const owner = await getActiveOwner();
    if (owner === userId) return 'same';
    if (isGuestOwner(owner)) {
      await db.games.where('owner').equals(owner).modify({ owner: userId });
      await db.outbox.where('owner').equals(owner).modify({ owner: userId });
      await moveKv(KV.settings(owner), KV.settings(userId));
      await moveKv(KV.currentGame(owner), KV.currentGame(userId));
      await setKv(KV.owner, userId);
      return 'claimed';
    }
    await setKv(KV.owner, userId);
    return 'switched';
  });
}

/** Move a kv value to another key, unless the destination already has one (then keep both). */
async function moveKv(from: string, to: string): Promise<void> {
  const value = await getKv<unknown>(from);
  if (value === undefined || (await getKv<unknown>(to)) !== undefined) return;
  await setKv(to, value);
  await deleteKv(from);
}

// --- account (profile only; never tokens) ---------------------------------------------------

export interface StoredAccount {
  user: Me;
}

export const getStoredAccount = () => getKv<StoredAccount>(KV.account);
export const setStoredAccount = (account: StoredAccount) => setKv(KV.account, account);
export const clearStoredAccount = () => deleteKv(KV.account);

// --- settings -------------------------------------------------------------------------------

export interface Settings {
  sound: boolean;
  language: string;
  hints: boolean;
}

/** The language defaults to the browser's, when the app has it (i18n/language.ts). */
export const DEFAULT_SETTINGS: Settings = {
  sound: true,
  language: browserLanguage(),
  hints: false,
};

/** Settings of the active owner (defaults for an owner who never changed them). */
export async function getSettings(owner?: string): Promise<Settings> {
  const key = KV.settings(owner ?? (await getActiveOwner()));
  return { ...DEFAULT_SETTINGS, ...(await getKv<Partial<Settings>>(key)) };
}

/** Save a settings change and queue a `settings_updated` mutation in the same transaction. */
export async function updateSettings(patch: Partial<Settings>): Promise<Settings> {
  const db = getDb();
  return db.transaction('rw', db.kv, db.outbox, async () => {
    const owner = await getActiveOwner();
    const next = { ...(await getSettings(owner)), ...patch };
    await setKv(KV.settings(owner), next);
    await enqueue('settings_updated', patch);
    return next;
  });
}

/** Apply settings pulled from the server to `owner`'s settings (no mutation is queued). */
export async function applyRemoteSettings(
  owner: string,
  data: Partial<Settings>,
): Promise<Settings> {
  const current = await getSettings(owner);
  const next: Settings = {
    sound: typeof data.sound === 'boolean' ? data.sound : current.sound,
    language: typeof data.language === 'string' ? data.language : current.language,
    hints: typeof data.hints === 'boolean' ? data.hints : current.hints,
  };
  await setKv(KV.settings(owner), next);
  return next;
}

// --- telemetry consent (ADR 0014; never synced, never linked to the account) ---------------

export interface TelemetryConsent {
  decided: boolean;
  crashReports: boolean;
  usageStats: boolean;
  installId: string;
}

export async function getConsent(): Promise<TelemetryConsent> {
  const stored = await getKv<TelemetryConsent>(KV.consent);
  if (stored) return stored;
  return { decided: false, crashReports: false, usageStats: false, installId: uuidv7() };
}

export async function setConsent(consent: TelemetryConsent): Promise<void> {
  await setKv(KV.consent, consent);
}

// --- in-progress game (stays on the device in the MVP) --------------------------------------

export interface CurrentGame {
  id: string;
  variant: VariantRef;
  firstPlayer: Player;
  humanPlayer: Player;
  level: Level;
  moves: number[];
  state: GameState;
  /** Base seed for the AI; the seed for each AI move is derived from it and the ply. */
  aiSeed: number;
  startedAt: string;
}

export const getCurrentGame = async () =>
  getKv<CurrentGame>(KV.currentGame(await getActiveOwner()));
export const saveCurrentGame = async (game: CurrentGame) =>
  setKv(KV.currentGame(await getActiveOwner()), game);
export const clearCurrentGame = async () => deleteKv(KV.currentGame(await getActiveOwner()));

// --- finished games + outbox ----------------------------------------------------------------

/** Queue a mutation for the active owner. Call inside a transaction that includes `outbox`. */
export async function enqueue<T extends MutationType>(
  type: T,
  payload: Extract<Mutation, { type: T }>['payload'],
): Promise<void> {
  const owner = await getActiveOwner();
  const mutation = {
    id: uuidv7(),
    type,
    schema: 1,
    payload,
    client_time: new Date().toISOString(),
  } as Mutation;
  await getDb().outbox.put({ id: mutation.id, owner, mutation });
}

/** Store a finished game, queue its upload, and clear the in-progress game: one transaction. */
export async function saveFinishedGame(record: GameRecord): Promise<void> {
  const db = getDb();
  await db.transaction('rw', db.games, db.outbox, db.kv, async () => {
    const owner = await getActiveOwner();
    await db.games.put({ id: record.id, owner, endedAt: record.ended_at, record });
    await enqueue('game_finished', record);
    await deleteKv(KV.currentGame(owner));
  });
}

/** Finished games of the active owner, newest first. */
export async function listGames(): Promise<GameRow[]> {
  const owner = await getActiveOwner();
  const rows = await getDb()
    .games.where('[owner+endedAt]')
    .between([owner, Dexie.minKey], [owner, Dexie.maxKey])
    .toArray();
  return rows.reverse();
}

// --- profile (queued like every other change; data-and-sync.md) ------------------------------

/**
 * Queue profile changes for the signed-in account: a `profile_updated` for the display name and
 * a `handle_requested` for the handle (applied only if the server confirms it's free).
 * Values must already be normalized (util/profile.ts).
 */
export async function queueProfileChange(change: {
  display_name?: string;
  handle?: string;
}): Promise<void> {
  const db = getDb();
  await db.transaction('rw', db.outbox, db.kv, async () => {
    if (change.display_name !== undefined) {
      await enqueue('profile_updated', { display_name: change.display_name });
    }
    if (change.handle !== undefined) await enqueue('handle_requested', { handle: change.handle });
  });
}

/** A finished game of the active owner, for replay. */
export async function getGame(id: string): Promise<GameRow | undefined> {
  const [owner, row] = await Promise.all([getActiveOwner(), getDb().games.get(id)]);
  return row?.owner === owner ? row : undefined;
}
