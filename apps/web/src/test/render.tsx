/**
 * Test helpers: render the app (or a screen) with the real WASM core, an inline AI and a
 * fresh in-memory database. Animations run with no delays.
 */
import { render } from '@testing-library/react';
import { MemoryRouter } from 'react-router';

import type { ApiClient } from '../api/client';
import { AppShell } from '../App';
import { inlineAi, type AiPlayer } from '../engine/ai';
import type { CoreApi } from '../engine/types';
import { OurilDb, setDb } from '../storage/db';
import { loadCoreSync } from './wasm';

let dbCounter = 0;

/** A fresh, empty local database for each test. */
export function freshDb(): OurilDb {
  const db = new OurilDb(`ouril-test-${++dbCounter}-${Math.random().toString(36).slice(2)}`);
  setDb(db);
  return db;
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
