import { cleanup, fireEvent, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';

import { getCurrentGame, listGames, saveCurrentGame } from '../storage/repo';
import { freshDb, renderApp } from '../test/render';

describe('Game screen (guest, real engine, inline AI)', () => {
  beforeEach(() => {
    freshDb();
  });

  it('starts a game from Home, plays a move, and the AI replies', async () => {
    renderApp();
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));

    const board = await screen.findByRole('group', { name: 'Game board' });
    const pit3 = await within(board).findByRole('button', { name: 'Your pit 3, 4 seeds' });
    fireEvent.click(pit3);

    await waitFor(async () => expect((await getCurrentGame())?.moves.length).toBe(2));
    expect(await screen.findByText('Your turn')).toBeInTheDocument();
    expect(await listGames()).toEqual([]);
  });

  it('resumes the unfinished game', async () => {
    renderApp();
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));
    const board = await screen.findByRole('group', { name: 'Game board' });
    fireEvent.click(await within(board).findByRole('button', { name: 'Your pit 1, 4 seeds' }));
    await waitFor(async () => expect((await getCurrentGame())?.moves.length).toBe(2));

    fireEvent.click(screen.getByRole('button', { name: 'Menu' }));
    const menu = await screen.findByRole('dialog', { name: 'Game paused' });
    fireEvent.click(within(menu).getByRole('link', { name: 'Save and exit' }));
    fireEvent.click(await screen.findByRole('link', { name: 'Resume game' }));
    const resumed = await screen.findByRole('group', { name: 'Game board' });
    const saved = (await getCurrentGame())!;
    const n = saved.state.pits[0]!;
    const label = new RegExp(`^Your pit 1, ${n} seeds?`);
    expect(within(resumed).getByRole('button', { name: label })).toBeInTheDocument();
  });

  it('forfeits from the game menu: a loss, recorded as resigned, nothing left to resume', async () => {
    renderApp();
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));
    const board = await screen.findByRole('group', { name: 'Game board' });
    fireEvent.click(await within(board).findByRole('button', { name: 'Your pit 2, 4 seeds' }));
    await waitFor(async () => expect((await getCurrentGame())?.moves.length).toBe(2));

    fireEvent.click(screen.getByRole('button', { name: 'Game menu' }));
    const menu = await screen.findByRole('dialog', { name: 'Game paused' });
    fireEvent.click(within(menu).getByRole('button', { name: 'Forfeit game' }));
    const confirm = await screen.findByRole('dialog', { name: 'Forfeit this game?' });
    fireEvent.click(within(confirm).getByRole('button', { name: 'Forfeit' }));

    const result = await screen.findByRole('dialog', { name: 'You lost' });
    expect(result).toHaveTextContent('You forfeited the game.');
    const games = await listGames();
    expect(games).toHaveLength(1);
    expect(games[0]!.record.format).toBe(2);
    expect(games[0]!.record.result).toMatchObject({ outcome: 'north_wins', reason: 'resigned' });
    expect(games[0]!.record.moves).toHaveLength(2);
    expect(await getCurrentGame()).toBeFalsy();
  });

  it('with no game in progress, the game screen offers the options and starts in one step', async () => {
    renderApp({ route: '/play' });
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    expect(await screen.findByRole('heading', { level: 1, name: 'New game' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('radio', { name: 'Medium' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));

    const board = await screen.findByRole('group', { name: 'Game board' });
    expect(
      await within(board).findByRole('button', { name: 'Your pit 1, 4 seeds' }),
    ).toBeInTheDocument();
    await waitFor(async () => expect((await getCurrentGame())?.level).toBe('medium'));
  });

  it('forfeits the paused game from the home page', async () => {
    renderApp();
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));
    const board = await screen.findByRole('group', { name: 'Game board' });
    fireEvent.click(await within(board).findByRole('button', { name: 'Your pit 1, 4 seeds' }));
    await waitFor(async () => expect((await getCurrentGame())?.moves.length).toBe(2));

    fireEvent.click(screen.getByRole('button', { name: 'Menu' }));
    fireEvent.click(
      within(await screen.findByRole('dialog', { name: 'Game paused' })).getByRole('link', {
        name: 'Save and exit',
      }),
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Forfeit game' }));
    const confirm = await screen.findByRole('dialog', { name: 'Forfeit this game?' });
    fireEvent.click(within(confirm).getByRole('button', { name: 'Forfeit' }));

    expect(await screen.findByText(/Game forfeited/)).toBeInTheDocument();
    expect(screen.queryByRole('link', { name: 'Resume game' })).toBeNull();
    const games = await listGames();
    expect(games).toHaveLength(1);
    expect(games[0]!.record.result.reason).toBe('resigned');
    expect(await getCurrentGame()).toBeFalsy();
  });

  it('starting a new game forfeits the unfinished one', async () => {
    renderApp();
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));
    const board = await screen.findByRole('group', { name: 'Game board' });
    fireEvent.click(await within(board).findByRole('button', { name: 'Your pit 1, 4 seeds' }));
    await waitFor(async () => expect((await getCurrentGame())?.moves.length).toBe(2));
    const first = (await getCurrentGame())!.id;

    fireEvent.click(screen.getByRole('button', { name: 'Menu' }));
    fireEvent.click(
      within(await screen.findByRole('dialog', { name: 'Game paused' })).getByRole('link', {
        name: 'Save and exit',
      }),
    );
    expect(
      await screen.findByText(
        'Starting a new game forfeits the unfinished one: it counts as a loss.',
      ),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));

    await waitFor(async () => expect((await getCurrentGame())?.id).not.toBe(first));
    const games = await listGames();
    expect(games).toHaveLength(1);
    expect(games[0]!.id).toBe(first);
    expect(games[0]!.record.result.reason).toBe('resigned');
    expect((await getCurrentGame())!.moves).toHaveLength(0);
  });

  it('starting a new game forfeits the unfinished one even with no moves played', async () => {
    renderApp();
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));
    await screen.findByRole('group', { name: 'Game board' });
    const first = (await getCurrentGame())!;
    expect(first.moves).toHaveLength(0);

    fireEvent.click(screen.getByRole('button', { name: 'Menu' }));
    fireEvent.click(
      within(await screen.findByRole('dialog', { name: 'Game paused' })).getByRole('link', {
        name: 'Save and exit',
      }),
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Start game' }));

    await waitFor(async () => expect((await getCurrentGame())?.id).not.toBe(first.id));
    const games = await listGames();
    expect(games.map((g) => [g.id, g.record.result.reason])).toEqual([[first.id, 'resigned']]);
  });

  it('plays the Continuous sowing rules when chosen, and records that variant', async () => {
    renderApp();
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Continuous sowing' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));
    const board = await screen.findByRole('group', { name: 'Game board' });
    // Opening from pit 1: the last seed lands in pit 5 (occupied), which relays on to North.
    fireEvent.click(await within(board).findByRole('button', { name: 'Your pit 1, 4 seeds' }));
    await waitFor(async () => expect((await getCurrentGame())?.moves.length).toBe(2));
    const game = (await getCurrentGame())!;
    expect(game.variant.id).toBe('cv.continuous');
    expect(game.moves[0]).toBe(0);
  });

  it('a saved game under a renamed variant ID resumes and is upgraded', async () => {
    renderApp();
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Continuous sowing' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));
    await screen.findByRole('group', { name: 'Game board' });
    const saved = (await getCurrentGame())!;
    await saveCurrentGame({ ...saved, variant: { id: 'cv.santiago-relay', version: 1 } });
    cleanup();

    renderApp({ route: '/play' });
    expect(await screen.findByRole('group', { name: 'Game board' })).toBeInTheDocument();
    await waitFor(async () =>
      expect((await getCurrentGame())?.variant).toEqual({ id: 'cv.continuous', version: 1 }),
    );
  });

  it('a saved game with unknown rules is set aside with an explanation, not a blank page', async () => {
    renderApp();
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));
    await screen.findByRole('group', { name: 'Game board' });
    const saved = (await getCurrentGame())!;
    await saveCurrentGame({ ...saved, variant: { id: 'xx.unknown', version: 9 } });
    cleanup();

    renderApp({ route: '/play' });
    expect(await screen.findByText(/used rules this version of the app/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Start game' })).toBeInTheDocument();
    expect(await getCurrentGame()).toBeFalsy();
  });
});
