/**
 * Loads the Rust core (core/wasm/pkg, the `ouril-wasm` workspace package) once.
 * Contract: core/wasm/API.md. Build the package first with `just bindings`.
 */
import init, * as core from 'ouril-wasm';

import type { CoreApi } from './types';

export type Core = typeof core;

let ready: Promise<CoreApi> | null = null;

/** Initialise the WASM module (same-origin asset; no other network access). */
export function loadCore(): Promise<CoreApi> {
  ready ??= init().then(() => core);
  return ready;
}
