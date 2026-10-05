/**
 * Web Worker that runs the AI search off the UI thread (docs/architecture/ai.md).
 * It loads its own copy of the WASM module (same-origin asset).
 */
import init, { aiMove } from 'ouril-wasm';

import type { AiRequest, AiResponse } from './aiProtocol';

const ready = init();

self.onmessage = async (event: MessageEvent<AiRequest>) => {
  const { id, variant, state, level, seed } = event.data;
  let response: AiResponse;
  try {
    await ready;
    const pit = aiMove(variant, state, level, seed);
    response = { id, pit: pit ?? null };
  } catch (e) {
    response = { id, error: e instanceof Error ? e.message : String(e) };
  }
  // `self` is a DedicatedWorkerGlobalScope here; the project's lib is DOM, hence the cast.
  (self as unknown as { postMessage(message: AiResponse): void }).postMessage(response);
};
