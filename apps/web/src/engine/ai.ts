/**
 * AI move provider. In the browser the search runs in a Web Worker; where Workers aren't
 * available (Vitest/jsdom) it falls back to calling the core directly.
 */
import type { AiRequest, AiResponse } from './aiProtocol';
import type { CoreApi, GameState, Level, VariantRef } from './types';

export interface AiPlayer {
  /** The AI's pit for `state`, or `null` if there is no legal move. */
  move(variant: VariantRef, state: GameState, level: Level, seed: number): Promise<number | null>;
  dispose(): void;
}

/** Runs the AI on the calling thread. For tests and as a fallback. */
export function inlineAi(core: Pick<CoreApi, 'aiMove'>): AiPlayer {
  return {
    move: async (variant, state, level, seed) => core.aiMove(variant, state, level, seed) ?? null,
    dispose: () => {},
  };
}

/** Runs the AI in a dedicated module Worker (one search at a time is enough for vs-AI play). */
export function workerAi(): AiPlayer {
  const worker = new Worker(new URL('./ai.worker.ts', import.meta.url), { type: 'module' });
  let nextId = 1;
  const pending = new Map<
    number,
    { resolve: (p: number | null) => void; reject: (e: Error) => void }
  >();

  worker.onmessage = (event: MessageEvent<AiResponse>) => {
    const msg = event.data;
    const entry = pending.get(msg.id);
    if (!entry) return;
    pending.delete(msg.id);
    if ('error' in msg) entry.reject(new Error(msg.error));
    else entry.resolve(msg.pit);
  };
  worker.onerror = (event) => {
    for (const entry of pending.values())
      entry.reject(new Error(event.message || 'ai_worker_error'));
    pending.clear();
  };

  return {
    move(variant, state, level, seed) {
      const id = nextId++;
      const request: AiRequest = { id, variant, state, level, seed };
      return new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject });
        worker.postMessage(request);
      });
    },
    dispose() {
      worker.terminate();
      pending.clear();
    },
  };
}

/** Picks the Worker implementation when available. */
export function createAi(core: Pick<CoreApi, 'aiMove'>): AiPlayer {
  return typeof Worker === 'undefined' ? inlineAi(core) : workerAi();
}
