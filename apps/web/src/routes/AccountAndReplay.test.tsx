import { fireEvent, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { ApiClient } from '../api/client';
import type { Me, Meta } from '../api/types';
import type { GameRecord } from '../engine/types';
import { getDb } from '../storage/db';
import { saveFinishedGame, setConsent } from '../storage/repo';
import { freshDb, renderApp } from '../test/render';

const ME: Me = {
  id: 'user-1',
  display_name: 'Dev Player',
  handle: null,
  avatar_url: null,
  locale: null,
  country: null,
  created_at: '2026-10-04T00:00:00Z',
};

function meta(minWeb: string): Meta {
  return {
    min_supported: { ios: '0.1.0', android: '0.1.0', web: minWeb },
    recommended: { ios: '0.1.0', android: '0.1.0', web: minWeb },
    api: { current: 'v1', deprecated: [] },
    realtime_proto: { current: 1, min: 1 },
  };
}

function fakeApi(minWeb = '0.0.0') {
  return {
    devSignIn: vi.fn(async () => ({ user: ME })),
    meta: vi.fn(async () => meta(minWeb)),
    syncPush: vi.fn(async () => ({ results: [] })),
    syncPull: vi.fn(async () => ({ changes: [], cursor: 0, has_more: false })),
  } as unknown as ApiClient & { meta: ReturnType<typeof vi.fn> };
}

async function signIn(api: ApiClient) {
  fireEvent.click(await screen.findByRole('button', { name: 'Dev sign-in' }));
  expect(await screen.findByRole('link', { name: 'Dev Player' })).toBeInTheDocument();
  void api;
}

describe('account features', () => {
  beforeEach(async () => {
    freshDb();
    vi.stubGlobal('fetch', vi.fn());
    await setConsent({ decided: true, crashReports: false, usageStats: false, installId: 'x' });
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('shows "update required" when the server needs a newer web app', async () => {
    const api = fakeApi('99.0.0');
    renderApp({ route: '/sign-in', createApi: () => api });
    await signIn(api);
    await waitFor(() => expect(api.meta).toHaveBeenCalled());
    expect(await screen.findByText(/too old for online features/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Reload' })).toBeInTheDocument();
  });

  it('queues a profile change and a handle request, and rejects invalid input', async () => {
    const api = fakeApi();
    renderApp({ route: '/sign-in', createApi: () => api });
    await signIn(api);
    fireEvent.click(screen.getByRole('link', { name: 'Dev Player' }));

    const handle = await screen.findByLabelText('Handle');
    fireEvent.change(handle, { target: { value: 'not ok!' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save profile' }));
    expect(await screen.findByText(/Check the fields/)).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Display name'), { target: { value: '  Ana  ' } });
    fireEvent.change(handle, { target: { value: '@Mindelo_Master' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save profile' }));
    expect(await screen.findByText(/Requested @mindelo_master/)).toBeInTheDocument();

    const outbox = await getDb().outbox.toArray();
    const mine = outbox.filter((r) => r.owner === 'user-1').map((r) => r.mutation);
    expect(mine.find((m) => m.type === 'profile_updated')?.payload).toEqual({
      display_name: 'Ana',
    });
    expect(mine.find((m) => m.type === 'handle_requested')?.payload).toEqual({
      handle: 'mindelo_master',
    });
    // The display name updates right away (it has no uniqueness rule).
    expect(await screen.findByRole('link', { name: 'Ana' })).toBeInTheDocument();
  });
});

describe('replay', () => {
  beforeEach(async () => {
    freshDb();
    await setConsent({ decided: true, crashReports: false, usageStats: false, installId: 'x' });
  });

  it('steps through a stored game with the engine', async () => {
    const record: GameRecord = {
      format: 1,
      id: 'g-replay',
      variant: { id: 'cv.standard', version: 1 },
      first_player: 'south',
      moves: [2, 7],
      core_version: '0.1.0',
      result: { outcome: 'draw', stores: [0, 0], reason: 'endless_cycle' },
      started_at: '2026-10-04T10:00:00Z',
      ended_at: '2026-10-04T10:05:00Z',
      mode: 'vs_ai',
      ai_level: 'easy',
      human_player: 'south',
    };
    await saveFinishedGame(record);
    renderApp({ route: '/replay/g-replay' });

    expect(await screen.findByText(/Move 0 of 2/)).toBeInTheDocument();
    expect(screen.getByText('Starting position')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Previous' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    expect(await screen.findByText(/Move 1 of 2 · pit 3/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Last' }));
    expect(await screen.findByText(/Move 2 of 2/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Next' })).toBeDisabled();
  });

  it('says so when the game is not on this device', async () => {
    renderApp({ route: '/replay/unknown' });
    expect(await screen.findByText("This game isn't on this device.")).toBeInTheDocument();
  });
});
