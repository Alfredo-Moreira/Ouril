import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiClient, ApiError } from '../api/client';
import type { OnlineGame } from '../api/types';
import { setConsent } from '../storage/repo';
import { freshDb, offlineApi, renderApp, signInLocally } from '../test/render';
import { loadCoreSync } from '../test/wasm';

const ANA = { user_id: 'u-ana', display_name: 'Ana', handle: 'ana' };

function game(overrides: Partial<OnlineGame> = {}): OnlineGame {
  return {
    id: 'g1',
    variant_id: 'cv.standard',
    variant_version: 1,
    status: 'active',
    first_player: 'south',
    you: 'south',
    opponent: ANA,
    moves: [],
    your_turn: true,
    turn_deadline: '2026-10-20T10:00:00Z',
    invited_by_you: true,
    created_at: '2026-10-13T10:00:00Z',
    updated_at: '2026-10-13T10:00:00Z',
    ...overrides,
  };
}

/** The offline API plus online-game methods, all spies. */
function onlineApi(games: OnlineGame[], extra: Record<string, unknown> = {}) {
  return Object.assign(offlineApi(), {
    listGames: vi.fn(async () => ({ games })),
    opponents: vi.fn(async () => ({ opponents: [] })),
    getGame: vi.fn(async (id: string) => games.find((g) => g.id === id) ?? games[0]!),
    createGame: vi.fn(async () => game({ id: 'new', status: 'invited' })),
    gameAction: vi.fn(async (id: string) => games.find((g) => g.id === id)!),
    playOnlineMove: vi.fn(async () => game()),
    joinGame: vi.fn(async () => game({ id: 'joined' })),
    ...extra,
  }) as unknown as ApiClient & Record<string, ReturnType<typeof vi.fn>>;
}

describe('Play online', () => {
  beforeEach(async () => {
    freshDb();
    await setConsent({ decided: true, crashReports: false, usageStats: false, installId: 'x' });
  });
  afterEach(() => sessionStorage.clear());

  it('lives under Game: a tab next to the computer, with no menu item of its own', async () => {
    await signInLocally();
    renderApp({ route: '/', createApi: () => onlineApi([]) });
    await screen.findByRole('heading', { level: 1 });
    const game = screen.getAllByRole('link', { name: 'Game' })[0]!;
    expect(game).toHaveAttribute('href', '/game');
    expect(screen.queryByRole('link', { name: 'Online' })).toBeNull();

    fireEvent.click(game);
    expect(await screen.findByRole('heading', { level: 1, name: 'Play' })).toBeInTheDocument();
    const computer = screen.getByRole('tab', { name: 'Against the computer' });
    expect(computer).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('button', { name: 'Start game' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: 'Online' }));
    expect(await screen.findByLabelText('Invite a player by handle')).toBeInTheDocument();
  });

  it('asks a guest to sign in on the Online tab, and comes back to it', async () => {
    renderApp({ route: '/game?tab=online' });
    expect(await screen.findByText('Play other people online')).toBeInTheDocument();
    fireEvent.click(within(screen.getByRole('tabpanel')).getByRole('link', { name: 'Sign in' }));
    expect(sessionStorage.getItem('ouril.return_to')).toBe('/game?tab=online');
  });

  it('keeps old /online links working', async () => {
    await signInLocally();
    renderApp({ route: '/online', createApi: () => onlineApi([]) });
    expect(await screen.findByRole('tab', { name: 'Online' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
  });

  it('lists games by what they need from you, and accepts an invitation', async () => {
    await signInLocally();
    const api = onlineApi([
      game({ id: 'inv', status: 'invited', invited_by_you: false, you: 'north' }),
      game({ id: 'mine', your_turn: true }),
      game({ id: 'theirs', your_turn: false, opponent: { user_id: 'u-rui', display_name: 'Rui' } }),
      game({ id: 'link', status: 'invited', opponent: undefined, invite_code: 'abc123' }),
    ]);
    renderApp({ route: '/online', createApi: () => api });
    const invitations = await screen.findByRole('region', { name: 'Invitations for you' });
    expect(within(invitations).getByText('Ana')).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Your turn' })).toBeInTheDocument();
    expect(
      within(screen.getByRole('region', { name: 'Their turn' })).getByText('Rui'),
    ).toBeVisible();
    const sent = screen.getByRole('region', { name: 'Invitations you sent' });
    expect(within(sent).getByText(/\/join\/abc123$/)).toBeInTheDocument();

    fireEvent.click(within(invitations).getByRole('button', { name: 'Accept' }));
    await waitFor(() => expect(api.gameAction).toHaveBeenCalledWith('inv', 'accept'));
  });

  it('invites by handle and explains an unknown handle', async () => {
    await signInLocally();
    const api = onlineApi([], {
      createGame: vi.fn(async () => {
        throw new ApiError(404, 'not_found', 'nope');
      }),
    });
    renderApp({ route: '/online', createApi: () => api });
    fireEvent.change(await screen.findByLabelText('Invite a player by handle'), {
      target: { value: '@Nobody' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Invite' }));
    await waitFor(() => expect(api.createGame).toHaveBeenCalledWith({ opponent_handle: 'Nobody' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('No player with that handle.');
  });

  it('draws the board from your side and sends the real pit (playing North)', async () => {
    await signInLocally();
    // South played pit 0 first; now it's North's (your) turn.
    const north = game({ you: 'north', moves: [0], your_turn: true, invited_by_you: false });
    const api = onlineApi([north]);
    renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => api });
    expect(await screen.findByRole('heading', { name: 'You vs Ana' })).toBeInTheDocument();
    expect(screen.getByText('Your turn.')).toBeInTheDocument();
    // "Your pit 1" is drawn where South's first pit is: it's really North's pit 6.
    fireEvent.click(screen.getByRole('button', { name: /^Your pit 1,/ }));
    await waitFor(() => expect(api.playOnlineMove).toHaveBeenCalledWith('g1', 1, 6));
  });

  it("waits during the opponent's turn and resigns after confirming", async () => {
    await signInLocally();
    const api = onlineApi([game({ your_turn: false })], {
      gameAction: vi.fn(async () =>
        game({
          status: 'finished',
          your_turn: false,
          result: { outcome: 'north_wins', reason: 'resigned', stores: [0, 0] },
        }),
      ),
    });
    renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => api });
    expect(await screen.findByText('Waiting for Ana to move…')).toBeInTheDocument();
    expect(screen.getByText(/Next move due by/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Resign' }));
    fireEvent.click(screen.getByRole('button', { name: 'Yes, resign' }));
    await waitFor(() => expect(api.gameAction).toHaveBeenCalledWith('g1', 'resign'));
    expect(await screen.findByText('You lost by resignation.')).toBeInTheDocument();
  });

  it('an invite link asks a guest to sign in, then joins once signed in', async () => {
    const guest = renderApp({ route: '/join/abc123' });
    // The page's own button (the header has a Sign in link too).
    const signIn = await within(await screen.findByRole('main')).findByRole('link', {
      name: 'Sign in',
    });
    expect(screen.getByText(/Sign in to accept this invitation/)).toBeInTheDocument();
    fireEvent.click(signIn);
    expect(sessionStorage.getItem('ouril.return_to')).toBe('/join/abc123');
    guest.unmount();

    await signInLocally();
    const api = onlineApi([game({ id: 'joined' })]);
    renderApp({ route: '/join/abc123', core: loadCoreSync(), createApi: () => api });
    await waitFor(() => expect(api.joinGame).toHaveBeenCalledWith('abc123'));
    expect(api.joinGame).toHaveBeenCalledTimes(1);
    expect(await screen.findByRole('heading', { name: 'You vs Ana' })).toBeInTheDocument();
  });
});
