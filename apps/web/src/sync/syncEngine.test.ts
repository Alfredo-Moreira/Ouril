import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiClient } from '../api/client';
import { MAX_PUSH_BATCH, type SyncPull, type SyncPush } from '../api/types';
import type { GameRecord } from '../engine/types';
import { claimForUser, getActiveOwner, listGames, saveFinishedGame } from '../storage/repo';
import { getDb } from '../storage/db';
import { freshDb } from '../test/render';
import { SyncEngine } from './syncEngine';

function record(id: string, endedAt: string): GameRecord {
  return {
    format: 1,
    id,
    variant: { id: 'cv.standard', version: 1 },
    first_player: 'south',
    moves: [2],
    core_version: '0.1.0',
    result: { outcome: 'south_wins', stores: [25, 10], reason: 'threshold' },
    started_at: endedAt,
    ended_at: endedAt,
    mode: 'vs_ai',
    ai_level: 'easy',
    human_player: 'south',
  };
}

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

describe('SyncEngine', () => {
  beforeEach(() => {
    freshDb();
  });

  it('never pushes guest data', async () => {
    await saveFinishedGame(record('g1', '2026-10-01T10:00:00Z'));
    const fetch = vi.fn();
    const api = new ApiClient({ fetch, clientHeader: () => 'web/test' });
    await new SyncEngine({ api }).syncNow();
    expect(fetch).not.toHaveBeenCalled();
  });

  it('claims guest games on sign-in, pushes the outbox, then pulls', async () => {
    await saveFinishedGame(record('g1', '2026-10-01T10:00:00Z'));
    await claimForUser('user-1');
    expect(await getActiveOwner()).toBe('user-1');

    const pushed: SyncPush[] = [];
    const fetch = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
      const u = String(url);
      if (u.endsWith('/v1/auth/refresh'))
        return json({ access_token: 'a', token_type: 'Bearer', expires_in: 900 });
      if (u.endsWith('/v1/sync/push')) {
        const body = JSON.parse(String(init?.body)) as SyncPush;
        pushed.push(body);
        return json({ results: body.mutations.map((m) => ({ id: m.id, status: 'applied' })) });
      }
      if (u.includes('/v1/sync/pull')) {
        const page: SyncPull = {
          changes: [
            {
              entity: 'game',
              id: 'g2',
              change_seq: 5,
              deleted: false,
              data: record('g2', '2026-10-02T10:00:00Z'),
            },
          ],
          cursor: 5,
          has_more: false,
        };
        return json(page);
      }
      return json({ code: 'not_found', message: '' }, 404);
    });
    const api = new ApiClient({
      fetch: fetch as typeof globalThis.fetch,
      clientHeader: () => 'web/test',
    });
    await new SyncEngine({ api }).syncNow();

    expect(pushed).toHaveLength(1);
    expect(pushed[0]!.mutations.map((m) => m.type)).toEqual(['game_finished']);
    expect(await getDb().outbox.count()).toBe(0);
    expect((await listGames()).map((g) => g.id)).toEqual(['g2', 'g1']);
  });

  it('reports auth loss when the refresh token is gone', async () => {
    await claimForUser('user-1');
    await saveFinishedGame(record('g1', '2026-10-01T10:00:00Z'));
    const fetch = vi.fn(async () => json({ code: 'unauthorized', message: '' }, 401));
    const onAuthLost = vi.fn();
    const api = new ApiClient({
      fetch: fetch as typeof globalThis.fetch,
      clientHeader: () => 'web/test',
    });
    const engine = new SyncEngine({ api, onAuthLost });
    await engine.syncNow();
    expect(onAuthLost).toHaveBeenCalled();
    expect(engine.getStatus()).toBe('auth_required');
    expect(await getDb().outbox.count()).toBe(1);
  });

  it('keeps deferred mutations queued without blocking the ones behind them', async () => {
    await claimForUser('user-1');
    // A full batch of games the server can't process yet, then one more it can.
    for (let i = 0; i < MAX_PUSH_BATCH; i++) {
      await saveFinishedGame(record(`future-${i}`, '2026-10-01T10:00:00Z'));
    }
    await saveFinishedGame(record('supported', '2026-10-02T10:00:00Z'));

    const pushes: string[][] = [];
    const fetch = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
      const u = String(url);
      if (u.endsWith('/v1/auth/refresh'))
        return json({ access_token: 'a', token_type: 'Bearer', expires_in: 900 });
      if (u.endsWith('/v1/sync/push')) {
        const body = JSON.parse(String(init?.body)) as SyncPush;
        pushes.push(body.mutations.map((m) => (m.payload as GameRecord).id));
        return json({
          results: body.mutations.map((m) =>
            (m.payload as GameRecord).id.startsWith('future-')
              ? { id: m.id, status: 'deferred', reason: 'unsupported_variant' }
              : { id: m.id, status: 'applied' },
          ),
        });
      }
      if (u.includes('/v1/sync/pull')) return json({ changes: [], cursor: 0, has_more: false });
      return json({ code: 'not_found', message: '' }, 404);
    });
    const api = new ApiClient({
      fetch: fetch as typeof globalThis.fetch,
      clientHeader: () => 'web/test',
    });
    const onDeferred = vi.fn();
    const onRejected = vi.fn();
    const engine = new SyncEngine({ api, onDeferred, onRejected });
    await engine.syncNow();

    // The second push skipped the deferred batch and reached the supported game.
    expect(pushes).toHaveLength(2);
    expect(pushes[1]).toEqual(['supported']);
    expect(onRejected).not.toHaveBeenCalled();
    expect(onDeferred).toHaveBeenCalledTimes(1);
    expect(onDeferred.mock.calls[0]![0]).toHaveLength(MAX_PUSH_BATCH);
    // Deferred games stay queued and aren't marked as rejected.
    expect(await getDb().outbox.count()).toBe(MAX_PUSH_BATCH);
    expect((await getDb().games.get('future-0'))?.syncRejected).toBeUndefined();

    // The next sync pushes them again (e.g. after the server is updated).
    await engine.syncNow();
    expect(pushes).toHaveLength(3);
    expect(pushes[2]).toHaveLength(MAX_PUSH_BATCH);
    expect(pushes[2]!.every((id) => id.startsWith('future-'))).toBe(true);
  });
});
