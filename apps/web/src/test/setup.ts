import '@testing-library/jest-dom/vitest';
// IndexedDB for Dexie in jsdom.
import 'fake-indexeddb/auto';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';

import '../i18n';

afterEach(() => {
  cleanup();
  // Remembered choices (the last new game) must not leak from one test into the next.
  localStorage.clear();
});
