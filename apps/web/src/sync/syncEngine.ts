/**
 * Outbox push + cursor pull against /v1/sync (data-and-sync.md, "Sync protocol").
 * Runs only while signed in. Guests never construct or start it.
 *
 * Triggers: start, the browser `online` event, returning to the foreground, after a local
 * change (debounced), and periodically. Failures retry with exponential backoff.
 */
import { ApiError, NetworkError, type ApiClient } from '../api/client';
import { MAX_PUSH_BATCH, type Change, type Me } from '../api/types';
import type { GameRecord } from '../engine/types';
import { getDb } from '../storage/db';
import {
  applyRemoteSettings,
  getActiveOwner,
  getKv,
  isGuestOwner,
  KV,
  setKv,
  type Settings,
} from '../storage/repo';

export type SyncStatus =
  'idle' | 'syncing' | 'offline' | 'error' | 'auth_required' | 'upgrade_required';

export interface SyncCallbacks {
  onStatus?(status: SyncStatus): void;
  onRemoteSettings?(settings: Settings): void;
  onRemoteProfile?(profile: Partial<Me>): void;
  /** The refresh token is gone or expired: uploads pause until the player signs in again. */
  onAuthLost?(): void;
  /** The server answered 426: this app is too old for online features until it updates. */
  onUpgradeRequired?(): void;
  /** Mutations the server rejected (`illegal_game`, `handle_taken`...). */
  onRejected?(rejections: { type: string; reason: string }[]): void;
  /**
   * Mutations the server can't process yet (`unsupported_variant`...). They stay queued and
   * are pushed again on later syncs, e.g. once the server is updated.
   */
  onDeferred?(deferrals: { type: string; reason: string }[]): void;
}

export interface SyncEngineOptions extends SyncCallbacks {
  api: ApiClient;
  debounceMs?: number;
  periodicMs?: number;
  maxBackoffMs?: number;
  pullLimit?: number;
}

export class SyncEngine {
  private status: SyncStatus = 'idle';
  private running: Promise<void> | null = null;
  private again = false;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private interval: ReturnType<typeof setInterval> | null = null;
  private failures = 0;
  private stopped = true;
  private readonly opts: Required<
    Pick<SyncEngineOptions, 'debounceMs' | 'periodicMs' | 'maxBackoffMs' | 'pullLimit'>
  > &
    SyncEngineOptions;

  constructor(options: SyncEngineOptions) {
    this.opts = {
      debounceMs: 2000,
      periodicMs: 5 * 60_000,
      maxBackoffMs: 5 * 60_000,
      pullLimit: 200,
      ...options,
    };
  }

  getStatus(): SyncStatus {
    return this.status;
  }

  start(): void {
    if (!this.stopped) return;
    this.stopped = false;
    window.addEventListener('online', this.onOnline);
    document.addEventListener('visibilitychange', this.onVisibility);
    this.interval = setInterval(() => void this.syncNow(), this.opts.periodicMs);
    void this.syncNow();
  }

  stop(): void {
    this.stopped = true;
    window.removeEventListener('online', this.onOnline);
    document.removeEventListener('visibilitychange', this.onVisibility);
    if (this.timer) clearTimeout(this.timer);
    if (this.interval) clearInterval(this.interval);
    this.timer = null;
    this.interval = null;
  }

  /** Debounced sync after a local change. */
  requestSync(): void {
    if (this.stopped) return;
    this.schedule(this.opts.debounceMs);
  }

  /** Push then pull, now. Concurrent calls coalesce into one extra run. */
  syncNow(): Promise<void> {
    if (this.running) {
      this.again = true;
      return this.running;
    }
    this.running = this.run().finally(() => {
      this.running = null;
      if (this.again && !this.stopped) {
        this.again = false;
        void this.syncNow();
      }
    });
    return this.running;
  }

  private readonly onOnline = () => void this.syncNow();
  private readonly onVisibility = () => {
    if (document.visibilityState === 'visible') void this.syncNow();
  };

  private schedule(ms: number): void {
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      this.timer = null;
      void this.syncNow();
    }, ms);
  }

  private setStatus(status: SyncStatus): void {
    this.status = status;
    this.opts.onStatus?.(status);
  }

  private async run(): Promise<void> {
    if (typeof navigator !== 'undefined' && navigator.onLine === false) {
      this.setStatus('offline');
      return;
    }
    this.setStatus('syncing');
    try {
      await this.push();
      await this.pull();
      this.failures = 0;
      this.setStatus('idle');
    } catch (e) {
      if (e instanceof ApiError && e.status === 401) {
        this.setStatus('auth_required');
        this.stop();
        this.opts.onAuthLost?.();
        return;
      }
      if (e instanceof ApiError && e.status === 426) {
        // Nothing is lost: the outbox keeps everything until the app is updated.
        this.setStatus('upgrade_required');
        this.stop();
        this.opts.onUpgradeRequired?.();
        return;
      }
      this.failures += 1;
      this.setStatus(e instanceof NetworkError ? 'offline' : 'error');
      if (!this.stopped) {
        const base = Math.min(this.opts.maxBackoffMs, 2000 * 2 ** (this.failures - 1));
        this.schedule(base / 2 + Math.random() * (base / 2));
      }
    }
  }

  private async push(): Promise<void> {
    const db = getDb();
    const owner = await getActiveOwner();
    if (isGuestOwner(owner)) return; // never upload guest data
    // Mutations the server can't process yet stay queued for a later sync, but are skipped for
    // the rest of this run so they never block the mutations behind them.
    const waiting = new Set<string>();
    const deferrals: { type: string; reason: string }[] = [];
    for (;;) {
      const rows = await db.outbox.where('owner').equals(owner).sortBy('id');
      const batch = rows.filter((r) => !waiting.has(r.id)).slice(0, MAX_PUSH_BATCH);
      if (batch.length === 0) break;
      const result = await this.opts.api.syncPush({ mutations: batch.map((r) => r.mutation) });
      const byId = new Map(batch.map((r) => [r.id, r]));
      const rejections: { type: string; reason: string }[] = [];
      await db.transaction('rw', db.outbox, db.games, async () => {
        for (const r of result.results) {
          const row = byId.get(r.id);
          if (!row) continue;
          byId.delete(r.id);
          if (r.status === 'applied') {
            await db.outbox.delete(r.id);
          } else if (r.status === 'rejected') {
            const reason = r.reason ?? 'unknown';
            await db.outbox.delete(r.id);
            rejections.push({ type: row.mutation.type, reason });
            // The game stays in local history; mark it so the UI can tell it isn't backed up.
            if (row.mutation.type === 'game_finished') {
              await db.games.update(row.mutation.payload.id, { syncRejected: reason });
            }
          } else {
            // `deferred`, or a status from a newer server: keep it and push it again later.
            waiting.add(r.id);
            deferrals.push({ type: row.mutation.type, reason: r.reason ?? r.status });
          }
        }
      });
      // A mutation the server didn't answer for also waits for the next sync.
      for (const id of byId.keys()) waiting.add(id);
      if (rejections.length > 0) this.opts.onRejected?.(rejections);
      if (batch.length < MAX_PUSH_BATCH) break;
    }
    if (deferrals.length > 0) this.opts.onDeferred?.(deferrals);
  }

  private async pull(): Promise<void> {
    const owner = await getActiveOwner();
    if (isGuestOwner(owner)) return;
    let cursor = (await getKv<number>(KV.syncCursor(owner))) ?? 0;
    for (;;) {
      const page = await this.opts.api.syncPull(cursor, this.opts.pullLimit);
      for (const change of page.changes) await this.apply(owner, change);
      cursor = page.cursor;
      await setKv(KV.syncCursor(owner), cursor);
      if (!page.has_more) return;
    }
  }

  private async apply(owner: string, change: Change): Promise<void> {
    const db = getDb();
    switch (change.entity) {
      case 'game': {
        if (change.deleted) {
          const row = await db.games.get(change.id);
          if (row?.owner === owner) await db.games.delete(change.id);
          return;
        }
        const record = change.data as GameRecord | undefined;
        if (!record || typeof record.id !== 'string') return;
        await db.games.put({ id: record.id, owner, endedAt: record.ended_at, record });
        return;
      }
      case 'settings': {
        if (change.deleted || !change.data) return;
        const settings = await applyRemoteSettings(owner, change.data as Partial<Settings>);
        this.opts.onRemoteSettings?.(settings);
        return;
      }
      case 'profile': {
        if (change.deleted || !change.data) return;
        this.opts.onRemoteProfile?.(change.data as Partial<Me>);
        return;
      }
      default:
        // Unknown entity from a newer server: ignore (versioning.md).
        return;
    }
  }
}
