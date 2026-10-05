/**
 * CRITICAL privacy guarantee (ADR 0014, data-and-sync.md): an unsigned-in guest who hasn't
 * opted into usage statistics makes ZERO network requests — through a full game, stats,
 * settings, tutorial, rules and the sign-in screen. With that consent, the only requests go to
 * the usage-statistics endpoint (Matomo, ADR 0026). Crash reports are a no-op until Sentry is
 * wired up.
 *
 * Every browser networking entry point is replaced by a spy for the whole session.
 */
import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from 'vitest';

import { getDb } from '../storage/db';
import { getCurrentGame, getSettings, listGames } from '../storage/repo';
import { configureTelemetry } from '../telemetry';
import { flush } from '../telemetry/matomo';
import { freshDb, renderApp } from '../test/render';

interface NetSpies {
  fetch: Mock<(...args: unknown[]) => Promise<Response>>;
  xhr: Mock<() => void>;
  webSocket: Mock<(url: string) => void>;
  eventSource: Mock<(url: string) => void>;
  sendBeacon: Mock<(...args: unknown[]) => boolean>;
}

function installNetworkSpies(): NetSpies {
  const spies: NetSpies = {
    fetch: vi.fn<(...args: unknown[]) => Promise<Response>>(() =>
      Promise.reject<Response>(new Error('network disabled in guest test')),
    ),
    xhr: vi.fn<() => void>(),
    webSocket: vi.fn<(url: string) => void>(),
    eventSource: vi.fn<(url: string) => void>(),
    sendBeacon: vi.fn<(...args: unknown[]) => boolean>(() => false),
  };
  vi.stubGlobal('fetch', spies.fetch);
  vi.stubGlobal(
    'XMLHttpRequest',
    class {
      constructor() {
        spies.xhr();
      }
      open() {}
      send() {}
      setRequestHeader() {}
      addEventListener() {}
    },
  );
  vi.stubGlobal(
    'WebSocket',
    class {
      constructor(url: string) {
        spies.webSocket(url);
      }
    },
  );
  vi.stubGlobal(
    'EventSource',
    class {
      constructor(url: string) {
        spies.eventSource(url);
      }
    },
  );
  Object.defineProperty(navigator, 'sendBeacon', {
    value: spies.sendBeacon,
    configurable: true,
    writable: true,
  });
  return spies;
}

function expectNoNetwork(spies: NetSpies) {
  expect(spies.fetch).not.toHaveBeenCalled();
  expect(spies.xhr).not.toHaveBeenCalled();
  expect(spies.webSocket).not.toHaveBeenCalled();
  expect(spies.eventSource).not.toHaveBeenCalled();
  expect(spies.sendBeacon).not.toHaveBeenCalled();
}

const enabledPits = (board: HTMLElement) =>
  within(board)
    .queryAllByRole('button')
    .filter((b) => b.getAttribute('aria-disabled') === 'false');

/** Plays the local side (first legal pit each turn) until the end-of-game dialog appears. */
async function playToEnd(board: HTMLElement): Promise<HTMLElement> {
  for (let turn = 0; turn < 500; turn++) {
    const next = await waitFor(
      () => {
        const dialog = screen.queryByRole('dialog');
        if (dialog) return { dialog };
        const pits = enabledPits(board);
        if (pits.length === 0) throw new Error('waiting for my turn');
        return { pit: pits[0]! };
      },
      { timeout: 5000 },
    );
    if (next.dialog) return next.dialog;
    fireEvent.click(next.pit!);
  }
  throw new Error('game did not finish');
}

describe('guest privacy: zero network requests (ADR 0014)', () => {
  let spies: NetSpies;

  beforeEach(() => {
    freshDb();
    spies = installNetworkSpies();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    delete (navigator as { sendBeacon?: unknown }).sendBeacon;
    configureTelemetry({ decided: false, crashReports: false, usageStats: false, installId: '' });
  });

  it('a full guest session (telemetry declined, every screen, a whole game) never touches the network', async () => {
    renderApp();

    // First launch: the guest declines telemetry.
    const consent = await screen.findByRole('dialog', { name: 'Help improve Ouril?' });
    fireEvent.click(within(consent).getByRole('button', { name: 'No thanks' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(screen.getByText('Playing as a guest')).toBeInTheDocument();

    // Tutorial: play the first step.
    fireEvent.click(screen.getAllByRole('link', { name: 'Tutorial' })[0]!);
    const tutorialBoard = await screen.findByRole('group', { name: 'Game board' });
    fireEvent.click(enabledPits(tutorialBoard)[0]!);
    await screen.findByRole('button', { name: 'Next' });

    // Rules.
    fireEvent.click(screen.getAllByRole('link', { name: 'Rules' })[0]!);
    await screen.findByRole('heading', { level: 1 });

    // Sign-in screen: provider buttons are disabled placeholders.
    fireEvent.click(screen.getAllByRole('link', { name: 'Sign in' })[0]!);
    const google = await screen.findByRole('button', { name: /Sign in with Google/ });
    const apple = screen.getByRole('button', { name: /Sign in with Apple/ });
    expect(google).toBeDisabled();
    expect(apple).toBeDisabled();
    fireEvent.click(google);
    fireEvent.click(apple);

    // A whole game against the real engine (Easy, I start).
    fireEvent.click(screen.getAllByRole('link', { name: 'Ouril' })[0]!);
    fireEvent.click(await screen.findByRole('radio', { name: 'Easy' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));
    const board = await screen.findByRole('group', { name: 'Game board' });
    const end = await playToEnd(board);
    expect(end).toHaveTextContent(/Final score/);
    await waitFor(async () => expect(await listGames()).toHaveLength(1));
    expect(await getCurrentGame()).toBeUndefined();

    // Stats show the finished game.
    fireEvent.click(within(end).getByRole('link', { name: 'Stats' }));
    expect(await screen.findByText('Played', { selector: 'dt' })).toBeInTheDocument();
    expect(within(screen.getByRole('list')).getAllByRole('listitem')).toHaveLength(1);

    // Settings: change everything a guest can change.
    fireEvent.click(screen.getAllByRole('link', { name: 'Settings' })[0]!);
    fireEvent.click(await screen.findByRole('checkbox', { name: 'Sound effects' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Show the hint button' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Crash reports' }));
    fireEvent.click(screen.getByRole('button', { name: 'Reset anonymous ID' }));
    await waitFor(async () => expect((await getSettings()).hints).toBe(true));

    // Start another game with hints on and ask for one (the hint uses the local AI).
    fireEvent.click(screen.getAllByRole('link', { name: 'Ouril' })[0]!);
    fireEvent.click(await screen.findByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));
    await screen.findByText('Your turn');
    fireEvent.click(screen.getByRole('button', { name: 'Hint' }));
    await waitFor(() => expect(document.querySelector('[data-hint]')).not.toBeNull());

    // Give any debounced/background work a chance to run, and force a telemetry flush.
    await new Promise((r) => setTimeout(r, 50));
    await flush();

    expectNoNetwork(spies);

    // The finished game and settings changes are queued locally under the guest owner, but
    // nothing was uploaded.
    const outbox = await getDb().outbox.toArray();
    expect(outbox.length).toBeGreaterThan(0);
    expect(outbox.every((r) => r.owner.startsWith('guest:'))).toBe(true);
    expect(outbox.map((r) => r.mutation.type)).toContain('game_finished');
  });

  it('with usage-statistics consent, the only requests are anonymous hits to Matomo', async () => {
    spies.fetch.mockImplementation(() => Promise.resolve(new Response(null, { status: 204 })));
    renderApp();
    const consent = await screen.findByRole('dialog', { name: 'Help improve Ouril?' });
    fireEvent.click(within(consent).getByRole('checkbox', { name: /Usage statistics/ }));
    fireEvent.click(within(consent).getByRole('button', { name: 'Save choices' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());

    fireEvent.click(screen.getAllByRole('link', { name: 'Rules' })[0]!);
    await screen.findByRole('heading', { level: 1 });
    await waitFor(async () => {
      await flush();
      expect(spies.fetch).toHaveBeenCalled();
    });

    for (const [url, init] of spies.fetch.mock.calls as [string, RequestInit][]) {
      expect(url).toBe('https://analytics.invalid/matomo.php');
      expect(init.credentials).toBe('omit');
      const { requests } = JSON.parse(init.body as string) as { requests: string[] };
      for (const r of requests) {
        const q = new URLSearchParams(r.slice(1));
        expect(q.get('idsite')).toBe('1');
        expect(q.get('_id')).toMatch(/^[0-9a-f]{16}$/);
        expect(q.has('urlref')).toBe(false);
        expect(q.has('uid')).toBe(false);
      }
    }
    expect(spies.xhr).not.toHaveBeenCalled();
    expect(spies.webSocket).not.toHaveBeenCalled();
    expect(spies.eventSource).not.toHaveBeenCalled();
  });

  it('control: the same spies DO see the app’s real API client once the player signs in', async () => {
    // Guards against this suite passing vacuously (e.g. if the client stopped using the global
    // fetch): the real ApiClient's dev sign-in must hit the fetch spy.
    renderApp({ route: '/sign-in' });
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Dev sign-in' }));
    await waitFor(() => expect(spies.fetch).toHaveBeenCalled());
    const [url, init] = spies.fetch.mock.calls[0]! as [string, RequestInit];
    expect(url).toBe('/v1/auth/dev');
    expect(init.method).toBe('POST');
    expect((init.headers as Record<string, string>)['X-Ouril-Client']).toBeTruthy();
    // The rejected request surfaces as an error and the player stays a guest.
    expect(await screen.findByRole('alert')).toBeInTheDocument();
    expect(screen.getAllByRole('link', { name: 'Sign in' }).length).toBeGreaterThan(0);
  });

  it('reloading the app as a returning guest makes no request either', async () => {
    const first = renderApp();
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    first.unmount();
    renderApp({ route: '/stats' });
    await screen.findByRole('heading', { level: 1 });
    await new Promise((r) => setTimeout(r, 50));
    expectNoNetwork(spies);
  });
});
