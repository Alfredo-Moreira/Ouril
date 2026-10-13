import { fireEvent, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiClient, ApiError } from '../api/client';
import type { OnlineGame } from '../api/types';
import { setConsent } from '../storage/repo';
import { trackEvent } from '../telemetry';
import { freshDb, offlineApi, renderApp, signInLocally } from '../test/render';
import { trackOnlineFinished } from './analytics';

// Record events instead of sending them; everything else in telemetry stays real.
vi.mock('../telemetry', async (original) => ({
  ...(await original<typeof import('../telemetry')>()),
  trackEvent: vi.fn(),
}));
const track = vi.mocked(trackEvent);

const finished: OnlineGame = {
  id: '0192b3c4-d5e6-7f80-9a1b-2c3d4e5f6a7b',
  variant_id: 'cv.standard',
  variant_version: 1,
  status: 'finished',
  first_player: 'south',
  you: 'north',
  opponent: { user_id: 'u-ana', display_name: 'Ana', handle: 'ana' },
  moves: [0, 6, 1],
  your_turn: false,
  result: { outcome: 'north_wins', reason: 'resigned', stores: [0, 0] },
  invited_by_you: false,
  created_at: '2026-10-13T10:00:00Z',
  updated_at: '2026-10-13T10:00:00Z',
};

function api(extra: Record<string, unknown>) {
  return Object.assign(offlineApi(), {
    listGames: vi.fn(async () => ({ games: [] })),
    opponents: vi.fn(async () => ({ opponents: [] })),
    ...extra,
  }) as unknown as ApiClient & Record<string, ReturnType<typeof vi.fn>>;
}

describe('usage events for online play', () => {
  beforeEach(async () => {
    freshDb();
    track.mockClear();
    await setConsent({ decided: true, crashReports: false, usageStats: false, installId: 'x' });
  });

  it('a finished game says how it went, never who played or which game', () => {
    trackOnlineFinished(finished);
    expect(track).toHaveBeenCalledWith('online_game_finished', {
      outcome: 'win',
      reason: 'resigned',
      plies: 3,
    });
    const sent = JSON.stringify(track.mock.calls);
    for (const secret of [finished.id, 'u-ana', 'Ana', 'ana']) {
      expect(sent).not.toContain(secret);
    }
  });

  it('counts an invitation only once the server accepted it', async () => {
    await signInLocally();
    const ok = api({ createGame: vi.fn(async () => ({ ...finished, status: 'invited' })) });
    const first = renderApp({ route: '/online', createApi: () => ok });
    fireEvent.click(await screen.findByRole('button', { name: 'Create an invite link' }));
    await waitFor(() => expect(track).toHaveBeenCalledWith('online_invite', { via: 'link' }));
    first.unmount();

    track.mockClear();
    const failing = api({
      createGame: vi.fn(async () => {
        throw new ApiError(404, 'not_found', 'nope');
      }),
    });
    renderApp({ route: '/online', createApi: () => failing });
    fireEvent.change(await screen.findByLabelText('Invite a player by handle'), {
      target: { value: 'nobody' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Invite' }));
    expect(await screen.findByRole('alert')).toBeInTheDocument();
    expect(track).not.toHaveBeenCalled();
  });

  it('counts joining by link and signing out, without identifiers', async () => {
    await signInLocally();
    const joiner = api({
      joinGame: vi.fn(async () => ({ ...finished, status: 'active' })),
      getGame: vi.fn(async () => ({ ...finished, status: 'active' })),
    });
    const join = renderApp({
      route: '/join/8f14e45fceea167a5a36dedd4bea2543',
      createApi: () => joiner,
    });
    await waitFor(() => expect(track).toHaveBeenCalledWith('online_join'));
    expect(JSON.stringify(track.mock.calls)).not.toContain('8f14e45f');
    join.unmount();

    track.mockClear();
    renderApp({
      route: '/settings',
      createApi: () => api({ logout: vi.fn(async () => undefined) }),
    });
    fireEvent.click(await screen.findByRole('button', { name: 'Sign out' }));
    await waitFor(() => expect(track).toHaveBeenCalledWith('sign_out'));
  });
});
