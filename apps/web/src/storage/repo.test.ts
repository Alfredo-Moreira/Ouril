import Dexie from 'dexie';
import { beforeEach, describe, expect, it } from 'vitest';

import { freshDb } from '../test/render';
import { getDb, OurilDb, setDb } from './db';
import {
  applyRemoteSettings,
  claimForUser,
  DEFAULT_SETTINGS,
  ensureGuestOwner,
  getActiveOwner,
  getCurrentGame,
  getKv,
  getSettings,
  KV,
  saveCurrentGame,
  setKv,
  updateSettings,
  type CurrentGame,
} from './repo';

/** Only the fields these tests look at; the rest of the shape doesn't matter here. */
function game(id: string, moves: number[] = []): CurrentGame {
  return { id, moves } as unknown as CurrentGame;
}

describe('settings and the in-progress game are per owner', () => {
  beforeEach(() => {
    freshDb();
  });

  it('signing in claims the guest settings and in-progress game', async () => {
    await ensureGuestOwner();
    await updateSettings({ sound: false });
    await saveCurrentGame(game('g1', [2]));

    expect(await claimForUser('user-1')).toBe('claimed');

    expect((await getSettings()).sound).toBe(false);
    expect((await getCurrentGame())?.id).toBe('g1');
  });

  it("a different account doesn't see the previous user's settings or game", async () => {
    await claimForUser('user-a');
    await updateSettings({ hints: true, sound: false });
    await saveCurrentGame(game('game-of-a', [0, 6]));

    expect(await claimForUser('user-b')).toBe('switched');

    expect(await getSettings()).toEqual(DEFAULT_SETTINGS);
    expect(await getCurrentGame()).toBeUndefined();
    // B's settings change is queued for B only, never for A.
    await updateSettings({ language: 'pt' });
    const outbox = await getDb().outbox.toArray();
    const settingsRows = outbox.filter((r) => r.mutation.type === 'settings_updated');
    expect(settingsRows.map((r) => r.owner).sort()).toEqual(['user-a', 'user-b']);

    // A's data was left aside, untouched, and comes back when A signs in again.
    expect(await claimForUser('user-a')).toBe('switched');
    expect(await getSettings()).toEqual({ ...DEFAULT_SETTINGS, hints: true, sound: false });
    expect((await getCurrentGame())?.id).toBe('game-of-a');
  });

  it("claiming keeps the account's own settings if this device already has them", async () => {
    await setKv(KV.settings('user-1'), { ...DEFAULT_SETTINGS, language: 'pt' });
    await ensureGuestOwner();
    await updateSettings({ language: 'en', sound: false });

    await claimForUser('user-1');

    expect((await getSettings()).language).toBe('pt');
    // The guest's own copy is not deleted.
    const guest = (await getKv<string>(KV.guestId))!;
    expect((await getKv<{ sound: boolean }>(KV.settings(guest)))?.sound).toBe(false);
  });

  it('remote settings are applied to the owner being synced, not the active one', async () => {
    await claimForUser('user-a');
    await claimForUser('user-b');

    await applyRemoteSettings('user-a', { sound: false });

    expect((await getSettings('user-a')).sound).toBe(false);
    expect(await getSettings()).toEqual(DEFAULT_SETTINGS); // user-b
  });
});

describe('local schema v1 → v2 migration', () => {
  it('moves the global settings and in-progress game to the active owner', async () => {
    const name = `ouril-migration-${Math.random().toString(36).slice(2)}`;
    const v1 = new Dexie(name);
    v1.version(1).stores({ games: 'id, owner, [owner+endedAt]', outbox: 'id, owner', kv: 'key' });
    await v1.table('kv').bulkPut([
      { key: 'guest_id', value: 'guest:abc' },
      { key: 'owner', value: 'user-1' },
      { key: 'settings', value: { sound: false, language: 'en', hints: true } },
      { key: 'current_game', value: { id: 'old-game', moves: [3] } },
    ]);
    v1.close();

    setDb(new OurilDb(name));

    expect(await getActiveOwner()).toBe('user-1');
    expect(await getSettings()).toEqual({ sound: false, language: 'en', hints: true });
    expect((await getCurrentGame())?.id).toBe('old-game');
    expect(await getKv('settings')).toBeUndefined();
    expect(await getKv('current_game')).toBeUndefined();
  });
});
