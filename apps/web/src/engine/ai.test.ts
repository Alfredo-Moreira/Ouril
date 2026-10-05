import { describe, expect, it, vi } from 'vitest';

import { createFakeCore, FAKE_VARIANT } from '../test/fakeCore';
import { createAi, inlineAi } from './ai';

describe('AI adapter', () => {
  it('inlineAi passes variant, state, level and seed straight to the core', async () => {
    const core = createFakeCore();
    const ai = inlineAi(core);
    const state = core.newGame(FAKE_VARIANT, 'north');
    await expect(ai.move(FAKE_VARIANT, state, 'hard', 123)).resolves.toBe(6);
    expect(core.aiMove).toHaveBeenCalledWith(FAKE_VARIANT, state, 'hard', 123);
  });

  it('inlineAi maps "no legal move" (undefined) to null', async () => {
    const ai = inlineAi({ aiMove: vi.fn(() => undefined) });
    await expect(
      ai.move(FAKE_VARIANT, createFakeCore().newGame(FAKE_VARIANT, 'south'), 'easy', 1),
    ).resolves.toBeNull();
  });

  it('createAi falls back to the inline AI where Workers are unavailable', async () => {
    expect(typeof Worker).toBe('undefined');
    const core = createFakeCore();
    const ai = createAi(core);
    await ai.move(FAKE_VARIANT, core.newGame(FAKE_VARIANT, 'north'), 'easy', 5);
    expect(core.aiMove).toHaveBeenCalledOnce();
    ai.dispose();
  });
});
