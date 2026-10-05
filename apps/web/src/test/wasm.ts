/**
 * Test helper: initialise the WASM core synchronously from disk (Vitest runs in Node, where
 * the default fetch-based `init()` can't load a file URL).
 */
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

import * as core from 'ouril-wasm';

let initialised = false;

export function loadCoreSync(): typeof core {
  if (!initialised) {
    const require = createRequire(import.meta.url);
    const wasmPath = require.resolve('ouril-wasm/ouril_wasm_bg.wasm');
    core.initSync({ module: readFileSync(wasmPath) });
    initialised = true;
  }
  return core;
}
