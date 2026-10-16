import { act, fireEvent, screen, waitFor, within } from '@testing-library/react';
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
    liveLobby: vi.fn(async () => ({ seeks: [] })),
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

  it("offers online play in the landing page's quick start", async () => {
    await signInLocally();
    renderApp({ route: '/', createApi: () => onlineApi([]) });
    await screen.findByRole('heading', { level: 1, name: 'Ouril' });
    const quick = document.getElementById('play')!;
    const computer = await within(quick).findByRole('tab', { name: 'Against the computer' });
    expect(computer).toHaveAttribute('aria-selected', 'true');
    expect(within(quick).getByRole('button', { name: 'Start game' })).toBeInTheDocument();

    fireEvent.click(within(quick).getByRole('tab', { name: 'Online' }));
    expect(await within(quick).findByLabelText('Invite a player by handle')).toBeInTheDocument();
    expect(within(quick).queryByRole('button', { name: 'Start game' })).toBeNull();
  });

  it("asks a guest to sign in from the landing page's Online tab", async () => {
    renderApp({ route: '/' });
    await screen.findByRole('heading', { level: 1, name: 'Ouril' });
    const quick = document.getElementById('play')!;
    fireEvent.click(await within(quick).findByRole('tab', { name: 'Online' }));
    expect(await within(quick).findByText('Play other people online')).toBeInTheDocument();
    fireEvent.click(within(quick).getByRole('link', { name: 'Sign in' }));
    expect(sessionStorage.getItem('ouril.return_to')).toBe('/game?tab=online');
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
    await waitFor(() =>
      expect(api.createGame).toHaveBeenCalledWith({ opponent_handle: 'Nobody', rated: true }),
    );
    expect(await screen.findByRole('alert')).toHaveTextContent('No player with that handle.');
  });

  it('draws the board from your side and sends the real pit (playing North)', async () => {
    await signInLocally();
    // South played pit 0 first; now it's North's (your) turn.
    const north = game({ you: 'north', moves: [0], your_turn: true, invited_by_you: false });
    const api = onlineApi([north]);
    renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => api });
    expect(await screen.findByRole('heading', { name: 'You vs Ana' })).toBeInTheDocument();
    expect(screen.getByText('Your turn')).toBeInTheDocument();
    // "Your pit 1" is drawn where South's first pit is: it's really North's pit 6.
    fireEvent.click(screen.getByRole('button', { name: /^Your pit 1,/ }));
    await waitFor(() => expect(api.playOnlineMove).toHaveBeenCalledWith('g1', 1, 6));
  });

  it("waits during the opponent's turn and resigns from the menu after confirming", async () => {
    await signInLocally();
    // North moves first, so it's Ana's turn on the board too.
    const api = onlineApi([game({ your_turn: false, first_player: 'north' })], {
      gameAction: vi.fn(async () =>
        game({
          status: 'finished',
          your_turn: false,
          result: { outcome: 'north_wins', reason: 'resigned', stores: [0, 0] },
        }),
      ),
    });
    renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => api });
    expect(await screen.findByText('Ana’s turn')).toBeInTheDocument();
    expect(screen.getByText(/Next move due by/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Game menu' }));
    fireEvent.click(screen.getByRole('button', { name: 'Resign' }));
    fireEvent.click(screen.getByRole('button', { name: 'Yes, resign' }));
    await waitFor(() => expect(api.gameAction).toHaveBeenCalledWith('g1', 'resign'));
    const result = await screen.findByRole('dialog', { name: 'You lost' });
    expect(within(result).getByText('You resigned.')).toBeInTheDocument();
  });

  it('is the same screen as a game against the computer', async () => {
    await signInLocally();
    renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => onlineApi([game()]) });
    expect(await screen.findByRole('heading', { name: 'You vs Ana' })).toBeInTheDocument();
    // The whole viewport: no menu bar, like /play.
    expect(screen.queryByRole('link', { name: 'Game' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Game menu' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Rules' }));
    expect(await screen.findByRole('dialog', { name: /rules/i })).toBeInTheDocument();
  });

  it('sows your move at once, before the server answers', async () => {
    await signInLocally();
    const api = onlineApi([game({ moves: [], your_turn: true })], {
      playOnlineMove: vi.fn(() => new Promise<OnlineGame>(() => undefined)),
    });
    renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => api });
    fireEvent.click(await screen.findByRole('button', { name: /^Your pit 1, 4 seeds/ }));
    await waitFor(() => expect(api.playOnlineMove).toHaveBeenCalledWith('g1', 0, 0));
    expect(await screen.findByRole('button', { name: /^Your pit 1, 0 seeds/ })).toBeInTheDocument();
  });

  it("sows the opponent's move when it arrives", async () => {
    await signInLocally();
    // The answer to your move already holds Ana's reply: both are sown, one after the other.
    const api = onlineApi([game({ moves: [], your_turn: true })], {
      playOnlineMove: vi.fn(async () => game({ moves: [0, 9], your_turn: true })),
    });
    renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => api });
    fireEvent.click(await screen.findByRole('button', { name: /^Your pit 1, 4 seeds/ }));
    // Ana's pit 4 (real pit 9) sowed round into yours: your emptied pit 1 holds 1 seed again.
    expect(
      await screen.findByRole('button', { name: /^Your pit 1, 1 seed\b/ }),
    ).toBeInTheDocument();
    expect(screen.getByText('Your turn')).toBeInTheDocument();
  });

  it('shows the result of a finished game, with who resigned, and offers a rematch', async () => {
    await signInLocally();
    const finished = game({
      status: 'finished',
      your_turn: false,
      result: { outcome: 'south_wins', reason: 'resigned', stores: [0, 0] },
    });
    const api = onlineApi([finished]);
    renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => api });
    const result = await screen.findByRole('dialog', { name: 'You won!' });
    expect(within(result).getByText('Ana resigned.')).toBeInTheDocument();
    fireEvent.click(within(result).getByRole('button', { name: 'Rematch' }));
    await waitFor(() =>
      expect(api.createGame).toHaveBeenCalledWith({ opponent_id: 'u-ana', rated: true }),
    );
  });

  it('makes casual invitations when Rated games is turned off', async () => {
    await signInLocally();
    const api = onlineApi([]);
    renderApp({ route: '/online', createApi: () => api });
    const rated = await screen.findByRole('checkbox', { name: 'Rated games' });
    expect(rated).toBeChecked();
    fireEvent.click(rated);
    expect(screen.getByText(/New invitations are casual/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Create an invite link' }));
    await waitFor(() => expect(api.createGame).toHaveBeenCalledWith({ rated: false }));
  });

  it('shows how a rated game moved your ratings', async () => {
    await signInLocally();
    const won = game({
      status: 'finished',
      your_turn: false,
      rated: true,
      result: { outcome: 'south_wins', reason: 'resigned', stores: [0, 0] },
      rating_changes: [
        { pool: 'overall', before: 1500, after: 1512 },
        { pool: 'cv.standard:async', before: 1480, after: 1495 },
      ],
    });
    renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => onlineApi([won]) });
    const result = await screen.findByRole('dialog', { name: 'You won!' });
    expect(within(result).getByText('Overall')).toBeInTheDocument();
    expect(within(result).getByText('1500 → 1512')).toBeInTheDocument();
    expect(within(result).getByText('(+12)')).toBeInTheDocument();
    expect(within(result).getByText('Standard · Turn by turn')).toBeInTheDocument();
  });

  it('says a casual game left ratings alone', async () => {
    await signInLocally();
    const casual = game({
      status: 'finished',
      your_turn: false,
      rated: false,
      result: { outcome: 'north_wins', reason: 'threshold', stores: [10, 25] },
    });
    renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => onlineApi([casual]) });
    const result = await screen.findByRole('dialog', { name: 'You lost' });
    expect(within(result).getByText("A casual game: ratings didn't change.")).toBeInTheDocument();
  });

  it('plays now: looks for an opponent, shows who else is looking, and can stop', async () => {
    await signInLocally();
    const api = onlineApi([], {
      seek: vi.fn(async () => ({ state: 'waiting', players_seeking: 3 })),
      cancelSeek: vi.fn(async () => ({ state: 'idle', players_seeking: 2 })),
    });
    renderApp({ route: '/online', createApi: () => api });
    const playNow = await screen.findByRole('region', { name: 'Play now' });
    fireEvent.click(within(playNow).getByRole('button', { name: '5 + 3' }));
    await waitFor(() =>
      expect(api.seek).toHaveBeenCalledWith({
        time_control: { initial_secs: 300, increment_secs: 3 },
        rated: true,
      }),
    );
    expect(await within(playNow).findByText('Looking for an opponent for 5 + 3…')).toBeDefined();
    expect(within(playNow).getByText('3 players looking')).toBeInTheDocument();
    fireEvent.click(within(playNow).getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(api.cancelSeek).toHaveBeenCalled());
    expect(within(playNow).getByRole('button', { name: '3 + 2' })).toBeInTheDocument();
  });

  it('opens the game when a live opponent is found', async () => {
    await signInLocally();
    const live = game({ id: 'live1', mode: 'live' });
    const api = onlineApi([live], {
      seek: vi.fn(async () => ({ state: 'matched', game_id: 'live1', players_seeking: 0 })),
      cancelSeek: vi.fn(async () => ({ state: 'idle', players_seeking: 0 })),
    });
    renderApp({ route: '/online', core: loadCoreSync(), createApi: () => api });
    fireEvent.click(
      within(await screen.findByRole('region', { name: 'Play now' })).getByRole('button', {
        name: '10 + 5',
      }),
    );
    expect(await screen.findByRole('heading', { name: 'You vs Ana' })).toBeInTheDocument();
    expect(api.cancelSeek).toHaveBeenCalled();
  });

  it('invites to a live game with the chosen clock', async () => {
    await signInLocally();
    const api = onlineApi([]);
    renderApp({ route: '/online', createApi: () => api });
    fireEvent.change(await screen.findByLabelText('Game pace'), { target: { value: '180+2' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create an invite link' }));
    await waitFor(() =>
      expect(api.createGame).toHaveBeenCalledWith({
        rated: true,
        time_control: { initial_secs: 180, increment_secs: 2 },
      }),
    );
  });

  it('shows both clocks in a live game and can call it off before both have moved', async () => {
    await signInLocally();
    const now = new Date().toISOString();
    const live = game({
      mode: 'live',
      time_control: { initial_secs: 300, increment_secs: 3 },
      clocks: { south_ms: 300_000, north_ms: 254_000, turn_started_at: now, running: false },
      server_now: now,
    });
    const api = onlineApi([live], {
      gameAction: vi.fn(async () => ({ ...live, status: 'aborted' })),
    });
    renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => api });
    const timers = await screen.findAllByRole('timer');
    expect(timers.map((t) => t.textContent)).toEqual(['4:14', '5:00']);
    expect(screen.getByText(/Make your first move within 30 seconds/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Game menu' }));
    fireEvent.click(screen.getByRole('button', { name: 'Call off the game' }));
    await waitFor(() => expect(api.gameAction).toHaveBeenCalledWith('g1', 'abort'));
    expect(
      await screen.findByText("Game called off: a player didn't make their first move in time."),
    ).toBeInTheDocument();
  });

  it('asks the server when a live clock reaches zero', async () => {
    await signInLocally();
    const now = Date.now();
    const live = game({
      mode: 'live',
      moves: [0, 9],
      time_control: { initial_secs: 180, increment_secs: 2 },
      // Your clock had 1 s left when your turn started 5 s ago.
      clocks: {
        south_ms: 1_000,
        north_ms: 90_000,
        turn_started_at: new Date(now - 5_000).toISOString(),
        running: true,
      },
      server_now: new Date(now).toISOString(),
    });
    const api = onlineApi([live]);
    renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => api });
    expect(await screen.findByText('0:00.0')).toBeInTheDocument();
    await waitFor(() => expect(vi.mocked(api.getGame).mock.calls.length).toBeGreaterThanOrEqual(2));
  });

  it('refetches the game the moment the relay says it changed', async () => {
    await signInLocally();
    const sockets: FakeSocket[] = [];
    class FakeSocket {
      onopen: (() => void) | null = null;
      onmessage: ((m: { data: string }) => void) | null = null;
      onclose: ((e: { code: number }) => void) | null = null;
      sent: string[] = [];
      constructor(readonly url: string) {
        sockets.push(this);
        setTimeout(() => this.onopen?.(), 0);
      }
      send(data: string) {
        this.sent.push(data);
      }
      close() {}
    }
    vi.stubGlobal('WebSocket', FakeSocket);
    try {
      const api = onlineApi([game({ your_turn: false })], {
        liveToken: vi.fn(async () => ({ token: 'tok', url: 'wss://relay/ws', expires_in: 600 })),
      });
      renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => api });
      await waitFor(() => expect(sockets[0]?.sent).toEqual(['{"token":"tok"}']));
      expect(api.liveToken).toHaveBeenCalledWith('g1');
      const before = vi.mocked(api.getGame).mock.calls.length;
      act(() => sockets[0]!.onmessage?.({ data: '{"type":"ready"}' }));
      act(() =>
        sockets[0]!.onmessage?.({
          data: '{"type":"game_changed","game_id":"g1","moves":1}',
        }),
      );
      await waitFor(() => expect(vi.mocked(api.getGame).mock.calls.length).toBe(before + 1));
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it("answers the opponent's draw offer, and offers one from the menu", async () => {
    await signInLocally();
    const offered = game({ moves: [0, 9], draw_offer: 'north' });
    const api = onlineApi([offered], {
      draw: vi.fn(async () =>
        game({
          moves: [0, 9],
          status: 'finished',
          your_turn: false,
          result: { outcome: 'draw', reason: 'agreement', stores: [0, 0] },
        }),
      ),
    });
    renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => api });
    expect(await screen.findByText('Ana offers a draw.')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Accept the draw' }));
    await waitFor(() => expect(api.draw).toHaveBeenCalledWith('g1', 'accept'));
    const result = await screen.findByRole('dialog', { name: 'It’s a draw' });
    expect(within(result).getByText('Draw agreed.')).toBeInTheDocument();
  });

  it('offers a draw from the menu once both players have moved', async () => {
    await signInLocally();
    const api = onlineApi([game({ moves: [0, 9] })], {
      draw: vi.fn(async () => game({ moves: [0, 9], draw_offer: 'south' })),
    });
    renderApp({ route: '/online/g1', core: loadCoreSync(), createApi: () => api });
    fireEvent.click(await screen.findByRole('button', { name: 'Game menu' }));
    fireEvent.click(screen.getByRole('button', { name: 'Offer a draw' }));
    await waitFor(() => expect(api.draw).toHaveBeenCalledWith('g1', 'offer'));
    expect(await screen.findByText(/You offered a draw/)).toBeInTheDocument();
  });

  it('joins a player waiting in the lobby', async () => {
    await signInLocally();
    const api = onlineApi([game({ id: 'joined', mode: 'live' })], {
      liveLobby: vi.fn(async () => ({
        seeks: [
          {
            player: { user_id: 'u-rui', display_name: 'Rui' },
            rating: 1620,
            variant_id: 'cv.standard',
            time_control: { initial_secs: 300, increment_secs: 3 },
            rated: true,
            waiting_secs: 12,
          },
        ],
      })),
      joinSeek: vi.fn(async () => ({ state: 'matched', game_id: 'joined', players_seeking: 0 })),
    });
    renderApp({ route: '/online', core: loadCoreSync(), createApi: () => api });
    const waiting = await screen.findByRole('region', { name: 'Waiting for an opponent' });
    expect(within(waiting).getByText(/1620/)).toBeInTheDocument();
    fireEvent.click(within(waiting).getByRole('button', { name: 'Play' }));
    await waitFor(() => expect(api.joinSeek).toHaveBeenCalledWith('u-rui'));
    expect(await screen.findByRole('heading', { name: 'You vs Ana' })).toBeInTheDocument();
  });

  it('keeps looking while you play the computer, and says when an opponent is found', async () => {
    await signInLocally();
    let calls = 0;
    const api = onlineApi([game({ id: 'live1', mode: 'live' })], {
      // Waiting at first, then paired on a later heartbeat.
      seek: vi.fn(async () =>
        ++calls < 2
          ? { state: 'waiting', players_seeking: 1 }
          : { state: 'matched', game_id: 'live1', players_seeking: 0 },
      ),
      cancelSeek: vi.fn(async () => ({ state: 'idle', players_seeking: 0 })),
    });
    renderApp({ route: '/online', core: loadCoreSync(), createApi: () => api });
    const playNow = await screen.findByRole('region', { name: 'Play now' });
    fireEvent.click(within(playNow).getByRole('button', { name: '5 + 3' }));
    fireEvent.click(
      await within(playNow).findByRole('link', { name: 'Play the computer while you wait' }),
    );
    expect(await screen.findByText('Looking for an opponent for 5 + 3…')).toBeInTheDocument();
    expect(await screen.findByText('Opponent found!', {}, { timeout: 5_000 })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Open the game' }));
    expect(await screen.findByRole('heading', { name: 'You vs Ana' })).toBeInTheDocument();
  }, 10_000);

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
