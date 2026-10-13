/**
 * Test helpers: render the app (or a screen) with the real WASM core, an inline AI and a
 * fresh in-memory database. Animations run with no delays.
 */
import { render } from '@testing-library/react';
import { MemoryRouter } from 'react-router';

import type { ApiClient } from '../api/client';
import type { Me } from '../api/types';
import { AppShell } from '../App';
import { inlineAi, type AiPlayer } from '../engine/ai';
import type { CoreApi } from '../engine/types';
import { OurilDb, setDb } from '../storage/db';
import { claimForUser, setStoredAccount } from '../storage/repo';
import { loadCoreSync } from './wasm';

let dbCounter = 0;

/** A fresh, empty local database for each test. */
export function freshDb(): OurilDb {
  const db = new OurilDb(`ouril-test-${++dbCounter}-${Math.random().toString(36).slice(2)}`);
  setDb(db);
  return db;
}

export const TEST_USER = { id: 'user-1', display_name: 'Ana Lopes' } as Me;

/** A player already signed in on this device (as after a sign-in), for the next render. */
export async function signInLocally(user: Me = TEST_USER): Promise<void> {
  await claimForUser(user.id);
  await setStoredAccount({ user });
}

/** An API client for signed-in tests that never touches the network: sync finds nothing. */
export function offlineApi(): ApiClient {
  return {
    syncPush: async () => ({ results: [] }),
    syncPull: async () => ({ changes: [], cursor: 0, has_more: false }),
    meta: async () => {
      throw new Error('offline in tests');
    },
  } as unknown as ApiClient;
}

export interface RenderAppOptions {
  route?: string;
  core?: CoreApi;
  ai?: AiPlayer;
  createApi?: () => ApiClient;
}

export function renderApp({ route = '/', core, ai, createApi }: RenderAppOptions = {}) {
  const c = core ?? loadCoreSync();
  return render(
    <MemoryRouter initialEntries={[route]}>
      <AppShell core={c} ai={ai ?? inlineAi(c)} createApi={createApi} animationSpeed={0} />
    </MemoryRouter>,
  );
}
