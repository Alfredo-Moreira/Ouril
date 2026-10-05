/**
 * Game flow against a scripted fake engine: checks that the UI asks the engine every rule
 * question (legal moves, applying moves, AI, undo via replay, records) and has no rule logic
 * of its own. See src/test/fakeCore.ts for the fake rules.
 */
import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';

import { inlineAi } from '../engine/ai';
import type { GameState } from '../engine/types';
import { getDb } from '../storage/db';
import { getCurrentGame, listGames } from '../storage/repo';
import { createFakeCore, FAKE_VARIANT, type FakeCore } from '../test/fakeCore';
import { freshDb, renderApp } from '../test/render';

let core: FakeCore;

async function startFromHome(level: 'Easy' | 'Medium' | 'Hard', starter: 'Me' | 'Computer') {
  renderApp({ core, ai: inlineAi(core) });
  fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
  fireEvent.click(screen.getByRole('radio', { name: level }));
  fireEvent.click(screen.getByRole('radio', { name: starter }));
  fireEvent.click(screen.getByRole('button', { name: 'Start game' }));
  return screen.findByRole('group', { name: 'Game board' });
}

const humanMoves = () =>
  core.applyMove.mock.calls
    .filter(([, s]) => (s as GameState).to_move === 'south')
    .map(([, , p]) => p);

describe('game flow (fake engine)', () => {
  beforeEach(() => {
    freshDb();
    core = createFakeCore();
  });

  it('starts a new game through the engine with the chosen first player and variant', async () => {
    await startFromHome('Medium', 'Me');
    expect(core.defaultVariant).toHaveBeenCalledWith('cv');
    expect(core.newGame).toHaveBeenCalledWith(FAKE_VARIANT, 'south');
    expect(await screen.findByText('Your turn')).toBeInTheDocument();
    const saved = await getCurrentGame();
    expect(saved).toMatchObject({ level: 'medium', firstPlayer: 'south', variant: FAKE_VARIANT });
  });

  it('only engine-legal pits are playable; clicking an illegal pit does not call the engine', async () => {
    // The engine says only pit index 3 is legal, even though other pits have seeds.
    core.legalMoves.mockImplementation((_v, s) =>
      s.status === 'playing' && s.to_move === 'south' ? [3] : s.status === 'playing' ? [6] : [],
    );
    const board = await startFromHome('Easy', 'Me');
    await screen.findByText('Your turn');

    const legalNames = within(board)
      .getAllByRole('button')
      .filter((b) => b.getAttribute('aria-disabled') === 'false')
      .map((b) => b.getAttribute('aria-label'));
    expect(legalNames).toEqual(['Your pit 4, 4 seeds']);

    fireEvent.click(within(board).getByRole('button', { name: /^Your pit 1, 4 seeds/ }));
    fireEvent.click(within(board).getByRole('button', { name: /^Your pit 2, 0 seeds/ }));
    fireEvent.click(within(board).getByRole('img', { name: 'Opponent pit 1, 4 seeds' }));
    expect(core.applyMove).not.toHaveBeenCalled();
    expect((await getCurrentGame())!.moves).toEqual([]);

    fireEvent.click(within(board).getByRole('button', { name: 'Your pit 4, 4 seeds' }));
    await waitFor(() => expect(core.applyMove).toHaveBeenCalled());
    expect(core.applyMove.mock.calls[0]![0]).toEqual(FAKE_VARIANT);
    expect(core.applyMove.mock.calls[0]![2]).toBe(3);
  });

  it('plays: your move -> AI reply -> your winning move -> end dialog + saved record', async () => {
    const board = await startFromHome('Hard', 'Me');
    fireEvent.click(await within(board).findByRole('button', { name: 'Your pit 1, 4 seeds' }));

    // AI replies with the fake AI's choice (pit index 6), at the chosen level.
    await waitFor(async () => expect((await getCurrentGame())?.moves).toEqual([0, 6]));
    expect(core.aiMove).toHaveBeenCalledTimes(1);
    const [, aiState, aiLevel, aiSeed] = core.aiMove.mock.calls[0]!;
    expect(aiState.to_move).toBe('north');
    expect(aiLevel).toBe('hard');
    expect(Number.isSafeInteger(aiSeed) && aiSeed >= 0).toBe(true);
    expect(await screen.findByText('Your turn')).toBeInTheDocument();
    // Scores come from the engine's stores.
    expect(within(board).getByRole('img', { name: 'You: 4 seeds captured' })).toBeInTheDocument();

    fireEvent.click(within(board).getByRole('button', { name: 'Your pit 3, 4 seeds' }));
    const dialog = await screen.findByRole('dialog', { name: 'You won!' });
    expect(dialog).toHaveTextContent('8');
    expect(screen.getByText('Game over')).toBeInTheDocument();

    const games = await listGames();
    expect(games).toHaveLength(1);
    const record = games[0]!.record;
    expect(record).toMatchObject({
      format: 1,
      variant: FAKE_VARIANT,
      first_player: 'south',
      moves: [0, 6, 2],
      core_version: '9.9.9-fake',
      mode: 'vs_ai',
      ai_level: 'hard',
      human_player: 'south',
      result: { outcome: 'south_wins', stores: [8, 4], reason: 'threshold' },
    });
    expect(humanMoves()).toEqual([0, 2]);
    // The in-progress game is cleared in the same transaction, and the upload is queued
    // locally (a guest never sends it).
    expect(await getCurrentGame()).toBeUndefined();
    const outbox = await getDb().outbox.toArray();
    expect(outbox.map((r) => r.mutation.type)).toContain('game_finished');
  });

  it('a loss shows the loss dialog', async () => {
    core = createFakeCore({ pits: [1, 0, 0, 0, 0, 0, 9, 4, 4, 4, 4, 4] });
    const board = await startFromHome('Easy', 'Me');
    fireEvent.click(await within(board).findByRole('button', { name: 'Your pit 1, 1 seed' }));
    expect(await screen.findByRole('dialog', { name: /lost/i })).toBeInTheDocument();
    expect((await listGames())[0]!.record.result.outcome).toBe('north_wins');
  });

  it('when the computer starts, it moves first', async () => {
    await startFromHome('Easy', 'Computer');
    await waitFor(async () => expect((await getCurrentGame())?.moves).toEqual([6]));
    expect(core.newGame).toHaveBeenCalledWith(FAKE_VARIANT, 'north');
    expect(await screen.findByText('Your turn')).toBeInTheDocument();
  });

  it('undo rebuilds the position with the engine’s replay', async () => {
    const board = await startFromHome('Easy', 'Me');
    const undo = screen.getByRole('button', { name: 'Undo' });
    expect(undo).toBeDisabled();
    fireEvent.click(await within(board).findByRole('button', { name: 'Your pit 1, 4 seeds' }));
    await waitFor(async () => expect((await getCurrentGame())?.moves).toEqual([0, 6]));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Undo' })).toBeEnabled());

    fireEvent.click(screen.getByRole('button', { name: 'Undo' }));
    expect(core.replay).toHaveBeenCalledWith(FAKE_VARIANT, 'south', [0, 6]);
    expect(
      await within(board).findByRole('button', { name: 'Your pit 1, 4 seeds' }),
    ).toBeInTheDocument();
    await waitFor(async () => expect((await getCurrentGame())?.moves).toEqual([]));
  });

  it('shows the hint button only when hints are on, and the hint comes from the AI', async () => {
    renderApp({ core, ai: inlineAi(core), route: '/settings' });
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Show the hint button' }));
    await waitFor(() =>
      expect(screen.getByRole('checkbox', { name: 'Show the hint button' })).toBeChecked(),
    );
    fireEvent.click(screen.getAllByRole('link', { name: 'Ouril' })[0]!);
    fireEvent.click(await screen.findByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));
    await screen.findByText('Your turn');
    fireEvent.click(screen.getByRole('button', { name: 'Hint' }));
    await waitFor(() => expect(document.querySelector('[data-hint]')).not.toBeNull());
    expect(document.querySelector('[data-hint]')!.getAttribute('data-pit')).toBe('0');
    expect(core.aiMove.mock.calls[0]![1].to_move).toBe('south');
  });

  it('without hints enabled there is no hint button', async () => {
    await startFromHome('Easy', 'Me');
    await screen.findByText('Your turn');
    expect(screen.queryByRole('button', { name: 'Hint' })).toBeNull();
  });
});
