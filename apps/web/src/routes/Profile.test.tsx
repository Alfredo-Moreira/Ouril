import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiClient, ApiError } from '../api/client';
import type { Leaderboard, PlayerProfile } from '../api/types';
import { setConsent } from '../storage/repo';
import { freshDb, offlineApi, renderApp, signInLocally } from '../test/render';

const ANA: PlayerProfile = {
  player: { user_id: 'u-ana', display_name: 'Ana Lopes', handle: 'ana' },
  country: 'cv',
  member_since: '2026-10-04T10:00:00Z',
  you: false,
  ratings: [
    {
      pool: 'overall',
      rating: 1712,
      deviation: 70,
      provisional: false,
      games: 30,
      wins: 20,
      draws: 2,
      losses: 8,
      peak: 1730,
    },
  ],
  history: [
    {
      pool: 'overall',
      points: [
        { at: '2026-10-10T10:00:00Z', rating: 1600 },
        { at: '2026-10-12T10:00:00Z', rating: 1650 },
        { at: '2026-10-14T10:00:00Z', rating: 1712 },
      ],
    },
  ],
  badges: [
    { pool: 'overall', scope: 'cv', rank: 1, achieved_on: '2026-10-14' },
    { pool: 'cv.standard:live', scope: 'world', rank: 4, achieved_on: '2026-10-13' },
  ],
  head_to_head: { wins: 3, draws: 1, losses: 2 },
};

function profileApi(extra: Record<string, unknown> = {}) {
  return Object.assign(offlineApi(), {
    player: vi.fn(async () => ANA),
    createGame: vi.fn(async () => ({ id: 'g1' })),
    listGames: vi.fn(async () => ({ games: [] })),
    opponents: vi.fn(async () => ({ opponents: [] })),
    ...extra,
  }) as unknown as ApiClient & Record<string, ReturnType<typeof vi.fn>>;
}

describe('Player profiles', () => {
  beforeEach(async () => {
    freshDb();
    await setConsent({ decided: true, crashReports: false, usageStats: false, installId: 'x' });
  });

  it('shows ratings with their history, badges, and your record against them', async () => {
    await signInLocally();
    const api = profileApi();
    renderApp({ route: '/players/u-ana', createApi: () => api });
    expect(await screen.findByRole('heading', { level: 1, name: 'Ana Lopes' })).toBeDefined();
    expect(api.player).toHaveBeenCalledWith('u-ana');
    expect(screen.getByText(/Playing since October 2026/).textContent).toContain('Cape Verde');
    expect(screen.getByText('You against Ana Lopes: 3 won, 1 drawn, 2 lost')).toBeDefined();
    expect(screen.getByText('#1 in Cape Verde · Overall')).toBeInTheDocument();
    expect(screen.getByText('#4 in the world · Standard · Live')).toBeInTheDocument();
    const overall = screen.getByRole('heading', { level: 3, name: 'Overall' }).parentElement!;
    expect(within(overall).getAllByText('1712')[0]!.tagName).toBe('STRONG');
    const chart = screen.getByRole('img', { name: 'Overall rating over time' });
    expect(chart).toBeInTheDocument();
    // The same points as a table, for screen readers.
    const table = screen.getByRole('table', { name: 'Overall rating over time' });
    expect(within(table).getAllByRole('row')).toHaveLength(4);
  });

  it('challenges the player to a rated game', async () => {
    await signInLocally();
    const api = profileApi();
    renderApp({ route: '/players/u-ana', createApi: () => api });
    fireEvent.click(await screen.findByRole('button', { name: 'Challenge' }));
    await waitFor(() =>
      expect(api.createGame).toHaveBeenCalledWith({ opponent_id: 'u-ana', rated: true }),
    );
  });

  it('has no challenge on your own profile, and explains a missing player', async () => {
    await signInLocally();
    const own = profileApi({
      player: vi.fn(async () => ({ ...ANA, you: true, head_to_head: undefined })),
    });
    const first = renderApp({ route: '/players/me', createApi: () => own });
    await screen.findByRole('heading', { level: 1, name: 'Ana Lopes' });
    expect(screen.queryByRole('button', { name: 'Challenge' })).toBeNull();
    first.unmount();

    const missing = profileApi({
      player: vi.fn(async () => {
        throw new ApiError(404, 'not_found', 'no');
      }),
    });
    renderApp({ route: '/players/@nobody', createApi: () => missing });
    expect(
      await screen.findByRole('heading', { name: "This player isn't available." }),
    ).toBeDefined();
  });

  it('opens from a leaderboard row', async () => {
    await signInLocally();
    const board: Leaderboard = {
      pool: 'overall',
      scope: 'world',
      entries: [
        {
          rank: 1,
          player: { user_id: 'u-ana', display_name: 'Ana Lopes' },
          rating: 1712,
          games: 30,
        },
      ],
      you: undefined,
    };
    const api = profileApi({
      myRatings: vi.fn(async () => ({ ratings: [] })),
      leaderboard: vi.fn(async () => board),
    });
    renderApp({ route: '/leaderboard', createApi: () => api });
    // In the table (the header's account link has the signed-in player's name too).
    const table = await screen.findByRole('table');
    fireEvent.click(within(table).getByRole('link', { name: 'Ana Lopes' }));
    expect(await screen.findByRole('heading', { level: 1, name: 'Ana Lopes' })).toBeDefined();
  });
});
