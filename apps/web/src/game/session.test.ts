import { describe, expect, it } from 'vitest';

import { loadCoreSync } from '../test/wasm';
import { outcomeFor, playMove, startGame, toRecord, undoLastHumanMove } from './session';

const core = loadCoreSync();
const fixed = { coinFlip: () => true, seed: () => 42 };

describe('game session', () => {
  it('starts with the chosen first player', () => {
    expect(startGame(core, { level: 'easy', starter: 'me' }, fixed).state.to_move).toBe('south');
    expect(startGame(core, { level: 'easy', starter: 'ai' }, fixed).state.to_move).toBe('north');
  });

  it('undo removes the human move and the AI reply', () => {
    let g = startGame(core, { level: 'easy', starter: 'me' }, fixed);
    g = playMove(core, g, 2).game;
    const ai = core.aiMove(g.variant, g.state, 'easy', 1)!;
    g = playMove(core, g, ai).game;
    const undone = undoLastHumanMove(core, g)!;
    expect(undone.moves).toEqual([]);
    expect(undone.state).toEqual(core.newGame(g.variant, 'south'));
  });

  it('plays a full game vs AI to a valid record', () => {
    let g = startGame(core, { level: 'easy', starter: 'random' }, fixed);
    let last: ReturnType<typeof playMove>['result'] | null = null;
    for (let i = 0; i < 1000 && g.state.status === 'playing'; i++) {
      const pit = core.aiMove(g.variant, g.state, 'easy', i)!;
      ({ game: g, result: last } = playMove(core, g, pit));
    }
    expect(g.state.status).not.toBe('playing');
    const over = last!.events.find((e) => e.type === 'game_over');
    expect(over).toBeDefined();
    const record = toRecord(core, g, over!.type === 'game_over' ? over!.reason : 'no_moves');
    expect(core.replay(record.variant, record.first_player, record.moves).steps).toHaveLength(
      record.moves.length,
    );
    expect(['win', 'loss', 'draw']).toContain(outcomeFor(record.result.outcome, 'south'));
    expect(core.deriveStats([record]).overall.played).toBe(1);
  });
});
