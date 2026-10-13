import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { ApiClient } from '../api/client';
import type { Leaderboard, LeaderboardEntry, Me, MyRatings } from '../api/types';
import { getDb } from '../storage/db';
import { setConsent } from '../storage/repo';
import { freshDb, offlineApi, renderApp, signInLocally, TEST_USER } from '../test/render';

function entry(rank: number, name: string, rating: number, country = 'cv'): LeaderboardEntry {
  return {
    rank,
    player: { user_id: `u-${name}`, display_name: name, handle: name.toLowerCase() },
    country,
    rating,
    games: 20,
  };
}

const MINE: MyRatings = {
  ratings: [
    {
      pool: 'overall',
      rating: 1532,
      deviation: 80,
      provisional: false,
      games: 14,
      wins: 9,
      draws: 1,
      losses: 4,
      peak: 1540,
    },
    {
      pool: 'cv.standard:async',
      rating: 1488,
      deviation: 150,
      provisional: true,
      games: 4,
      wins: 2,
      draws: 0,
      losses: 2,
      peak: 1510,
    },
  ],
};

function boardsApi(board: Partial<Leaderboard> = {}) {
  return Object.assign(offlineApi(), {
    myRatings: vi.fn(async () => MINE),
    leaderboard: vi.fn(async (pool: string, scope: 'world' | 'country'): Promise<Leaderboard> => ({
      pool,
      scope,
      entries: [entry(1, 'Rui', 1900), entry(2, 'Lia', 1800, 'pt')],
      you: {
        ...entry(57, 'Ana Lopes', 1532),
        player: { user_id: TEST_USER.id, display_name: 'Ana Lopes' },
      },
      ...board,
    })),
  }) as unknown as ApiClient & Record<string, ReturnType<typeof vi.fn>>;
}

describe('Leaderboard', () => {
  beforeEach(async () => {
    freshDb();
    await setConsent({ decided: true, crashReports: false, usageStats: false, installId: 'x' });
  });

  it('is in the menu for signed-in players, and asks guests to sign in', async () => {
    const guest = renderApp({ route: '/leaderboard' });
    expect(await screen.findByRole('heading', { level: 1, name: 'Sign in' })).toBeInTheDocument();
    expect(screen.queryByRole('link', { name: 'Leaderboard' })).toBeNull();
    guest.unmount();

    await signInLocally();
    renderApp({ route: '/', createApi: () => boardsApi() });
    fireEvent.click(await screen.findByRole('link', { name: 'Leaderboard' }));
    expect(
      await screen.findByRole('heading', { level: 1, name: 'Leaderboard' }),
    ).toBeInTheDocument();
  });

  it('shows your ratings and the world board, with you pinned below the top', async () => {
    await signInLocally();
    const api = boardsApi();
    renderApp({ route: '/leaderboard', createApi: () => api });
    const yours = (await screen.findByRole('heading', { name: 'Your ratings' })).parentElement!;
    expect(within(yours).getByText('1532')).toBeInTheDocument();
    expect(within(yours).getByText('1488?')).toBeInTheDocument();
    expect(within(yours).getByText('Standard · Turn by turn')).toBeInTheDocument();

    const table = await screen.findByRole('table');
    const rows = within(table).getAllByRole('row').slice(1);
    expect(rows.map((r) => within(r).getAllByRole('cell')[0]!.textContent)).toEqual([
      '1',
      '2',
      '57',
    ]);
    expect(rows[2]).toHaveAttribute('aria-current', 'true');
    expect(within(rows[1]!).getByText('Portugal')).toBeInTheDocument();
    expect(api.leaderboard).toHaveBeenCalledWith('overall', 'world');

    fireEvent.change(screen.getByLabelText('Ranking'), {
      target: { value: 'cv.across:async' },
    });
    await waitFor(() => expect(api.leaderboard).toHaveBeenCalledWith('cv.across:async', 'world'));
  });

  it('asks for a country before showing a country board', async () => {
    await signInLocally();
    const api = boardsApi();
    renderApp({ route: '/leaderboard?scope=country', createApi: () => api });
    expect(
      await screen.findByText('Set your country in your profile to see its leaderboard.'),
    ).toBeInTheDocument();
    expect(api.leaderboard).not.toHaveBeenCalled();
  });

  it("shows your country's board, named after it", async () => {
    await signInLocally({ ...TEST_USER, country: 'cv' } as Me);
    const api = boardsApi({ scope: 'country', country: 'cv' });
    renderApp({ route: '/leaderboard', createApi: () => api });
    fireEvent.click(await screen.findByRole('button', { name: 'Cape Verde' }));
    await waitFor(() => expect(api.leaderboard).toHaveBeenCalledWith('overall', 'country'));
  });

  it('explains how to get on an empty board', async () => {
    await signInLocally();
    renderApp({
      route: '/leaderboard',
      createApi: () => boardsApi({ entries: [], you: undefined }),
    });
    expect(await screen.findByText(/Players appear after 10 rated games/)).toBeInTheDocument();
  });

  it('sets the country from Settings, queued like any profile change', async () => {
    await signInLocally();
    renderApp({ route: '/settings', createApi: () => boardsApi() });
    fireEvent.change(await screen.findByLabelText('Country'), { target: { value: 'cv' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save profile' }));
    await waitFor(async () => {
      const outbox = await getDb().outbox.toArray();
      expect(outbox.map((r) => r.mutation)).toContainEqual(
        expect.objectContaining({ type: 'profile_updated', payload: { country: 'cv' } }),
      );
    });
  });
});
