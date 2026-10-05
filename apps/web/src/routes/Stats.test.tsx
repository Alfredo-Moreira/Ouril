import { screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';

import { getDb } from '../storage/db';
import { saveFinishedGame, setConsent } from '../storage/repo';
import { createFakeCore, makeRecord } from '../test/fakeCore';
import { freshDb, renderApp } from '../test/render';
import { loadCoreSync } from '../test/wasm';

const tile = (label: string) => {
  const dt = screen.getByText(label, { selector: 'dt' });
  return dt.parentElement!.querySelector('dd')!.textContent;
};

describe('Stats screen', () => {
  beforeEach(async () => {
    freshDb();
    // Skip the first-launch consent dialog.
    await setConsent({ decided: true, crashReports: false, usageStats: false, installId: 'x' });
  });

  it('shows the empty state and the guest note when there are no games', async () => {
    renderApp({ route: '/stats' });
    expect(await screen.findByRole('heading', { level: 1 })).toBeInTheDocument();
    expect(screen.getByText(/No finished games yet/)).toBeInTheDocument();
    expect(screen.getByText(/Your stats are stored on this device/)).toBeInTheDocument();
  });

  it('renders exactly the stats the engine derives from the stored records', async () => {
    await saveFinishedGame(makeRecord({ id: 'a' }));
    await saveFinishedGame(makeRecord({ id: 'b', ended_at: '2026-01-03T10:00:00.000Z' }));
    const core = createFakeCore({
      stats: {
        overall: { played: 11, wins: 7, losses: 3, draws: 1 },
        by_level: {
          easy: { played: 5, wins: 4, losses: 1, draws: 0 },
          hard: { played: 6, wins: 3, losses: 2, draws: 1 },
        },
        current_streak: 2,
        best_streak: 4,
      },
    });
    renderApp({ route: '/stats', core });
    await screen.findByText('Played', { selector: 'dt' });

    // Derived by the engine from the records, never counted by the UI.
    expect(core.deriveStats).toHaveBeenCalled();
    const passed = core.deriveStats.mock.calls.at(-1)![0];
    expect(passed.map((r) => r.id).sort()).toEqual(['a', 'b']);

    expect(tile('Played')).toBe('11');
    expect(tile('Wins')).toBe('7');
    expect(tile('Losses')).toBe('3');
    expect(tile('Draws')).toBe('1');

    const rows = within(screen.getByRole('table')).getAllByRole('row');
    const cells = (name: string) =>
      Array.from(rows.find((r) => r.textContent?.startsWith(name))!.querySelectorAll('td')).map(
        (td) => td.textContent,
      );
    expect(cells('Easy')).toEqual(['5', '4', '1', '0']);
    // A level with no games shows zeros.
    expect(cells('Medium')).toEqual(['0', '0', '0', '0']);
    expect(cells('Hard')).toEqual(['6', '3', '2', '1']);
  });

  it('lists recent games newest first with result, score and level (real engine)', async () => {
    await saveFinishedGame(
      makeRecord({ id: 'old', ended_at: '2026-01-01T10:00:00.000Z', ai_level: 'hard' }),
    );
    await saveFinishedGame(
      makeRecord({
        id: 'new',
        ended_at: '2026-02-01T10:00:00.000Z',
        ai_level: 'medium',
        result: { outcome: 'north_wins', stores: [12, 30], reason: 'threshold' },
      }),
    );
    renderApp({ route: '/stats', core: loadCoreSync() });
    const items = within(await screen.findByRole('list')).getAllByRole('listitem');
    expect(items).toHaveLength(2);
    expect(items[0]).toHaveTextContent('Loss');
    expect(items[0]).toHaveTextContent('12–30');
    expect(items[0]).toHaveTextContent('Medium');
    expect(items[1]).toHaveTextContent('Win');
    expect(items[1]).toHaveTextContent('25–10');
    expect(tile('Played')).toBe('2');
  });

  it('shows the score from the player’s side when they played North', async () => {
    await saveFinishedGame(
      makeRecord({
        human_player: 'north',
        result: { outcome: 'north_wins', stores: [10, 26], reason: 'threshold' },
      }),
    );
    renderApp({ route: '/stats' });
    const item = within(await screen.findByRole('list')).getByRole('listitem');
    expect(item).toHaveTextContent('Win');
    expect(item).toHaveTextContent('26–10');
  });

  it('marks games the server rejected as not backed up', async () => {
    await saveFinishedGame(makeRecord({ id: 'bad' }));
    await getDb().games.update('bad', { syncRejected: 'illegal_game' });
    renderApp({ route: '/stats' });
    expect(await screen.findByText(/not backed up/i)).toBeInTheDocument();
  });

  it('only shows the active owner’s games', async () => {
    await saveFinishedGame(makeRecord({ id: 'mine' }));
    await getDb().games.put({
      id: 'someone-else',
      owner: 'user-2',
      endedAt: '2026-03-01T00:00:00.000Z',
      record: makeRecord({ id: 'someone-else' }),
    });
    renderApp({ route: '/stats' });
    await screen.findByRole('list');
    expect(within(screen.getByRole('list')).getAllByRole('listitem')).toHaveLength(1);
    expect(tile('Played')).toBe('1');
  });
});
