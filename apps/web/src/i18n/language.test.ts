import { describe, expect, it } from 'vitest';

import { browserLanguage } from './language';

describe('browserLanguage (first-visit default)', () => {
  it('picks the first preferred language the app supports, exact or by base', () => {
    expect(browserLanguage(['pt-BR', 'en'], ['en', 'pt'])).toBe('pt');
    expect(browserLanguage(['fr', 'kea', 'en'], ['en', 'kea'])).toBe('kea');
    expect(browserLanguage(['pt-PT'], ['en', 'pt-PT', 'pt'])).toBe('pt-PT');
  });

  it('falls back to English', () => {
    expect(browserLanguage(['de', 'fr'], ['en', 'pt'])).toBe('en');
    expect(browserLanguage([], ['en'])).toBe('en');
  });
});
