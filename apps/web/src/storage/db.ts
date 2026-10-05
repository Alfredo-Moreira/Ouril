/**
 * Local database (IndexedDB via Dexie), per docs/architecture/data-and-sync.md.
 *
 * Every synced row carries an `owner`: the guest ID (`guest:<uuid>`) or a user ID. The
 * active owner is kept in `kv`. Signing in claims the guest's rows; signing in as a
 * different account switches the active owner and leaves the previous user's rows aside.
 * Session tokens are never stored here (access token: memory; refresh token: httpOnly cookie).
 */
import Dexie, { type Table } from 'dexie';

import type { Mutation } from '../api/types';
import type { GameRecord } from '../engine/types';

export interface GameRow {
  id: string;
  owner: string;
  endedAt: string;
  record: GameRecord;
  /** Set when the server rejected the upload (e.g. `illegal_game`). The game stays local. */
  syncRejected?: string;
}

export interface OutboxRow {
  /** Mutation ID (UUIDv7, so ordering by ID is ordering by creation time). */
  id: string;
  owner: string;
  mutation: Mutation;
}

export interface KvRow {
  key: string;
  value: unknown;
}

export class OurilDb extends Dexie {
  games!: Table<GameRow, string>;
  outbox!: Table<OutboxRow, string>;
  kv!: Table<KvRow, string>;

  constructor(name = 'ouril') {
    super(name);
    // Bump the version and add an upgrade() when the shape changes. Every version must keep
    // working from any older one (versioning.md, "Data migrations").
    this.version(1).stores({
      games: 'id, owner, [owner+endedAt]',
      outbox: 'id, owner',
      kv: 'key',
    });
    // v2: settings and the in-progress game are stored per owner (`settings:<owner>`,
    // `current_game:<owner>`) instead of in one global key.
    this.version(2)
      .stores({})
      .upgrade(async (tx) => {
        const kv = tx.table<KvRow, string>('kv');
        const owner =
          ((await kv.get('owner'))?.value as string | undefined) ??
          ((await kv.get('guest_id'))?.value as string | undefined);
        for (const legacy of ['settings', 'current_game']) {
          const row = await kv.get(legacy);
          if (!row) continue;
          if (owner && !(await kv.get(`${legacy}:${owner}`))) {
            await kv.put({ key: `${legacy}:${owner}`, value: row.value });
          }
          await kv.delete(legacy);
        }
      });
  }
}

let instance: OurilDb | null = null;

/** The app's database (lazily opened). Tests can swap it with `setDb`. */
export function getDb(): OurilDb {
  instance ??= new OurilDb();
  return instance;
}

export function setDb(db: OurilDb): void {
  instance = db;
}
