import { describe, expect, it } from 'vitest';

import {
  GOOGLE_AUTH_ENDPOINT,
  GOOGLE_PENDING_KEY,
  GOOGLE_PENDING_MAX_AGE_MS,
  googleSignInUrl,
  readGoogleCallback,
} from './google';

function memoryStorage() {
  const items = new Map<string, string>();
  return {
    items,
    getItem: (k: string) => items.get(k) ?? null,
    setItem: (k: string, v: string) => void items.set(k, v),
    removeItem: (k: string) => void items.delete(k),
  };
}

const ORIGIN = 'http://localhost:5173';

function start(storage = memoryStorage(), now = 1_000) {
  const url = new URL(googleSignInUrl({ clientId: 'client-1', origin: ORIGIN, storage, now }));
  return {
    storage,
    url,
    state: url.searchParams.get('state')!,
    nonce: url.searchParams.get('nonce')!,
  };
}

describe('Google sign-in redirect', () => {
  it('asks Google for an ID token with a fresh state and nonce', () => {
    const { url, state, nonce, storage } = start();
    expect(`${url.origin}${url.pathname}`).toBe(GOOGLE_AUTH_ENDPOINT);
    expect(Object.fromEntries(url.searchParams)).toMatchObject({
      client_id: 'client-1',
      response_type: 'id_token',
      redirect_uri: `${ORIGIN}/auth/google`,
      scope: 'openid email profile',
      prompt: 'select_account',
    });
    expect(state).toMatch(/^[A-Za-z0-9_-]{43}$/);
    expect(nonce).toMatch(/^[A-Za-z0-9_-]{43}$/);
    expect(state).not.toBe(nonce);
    expect(JSON.parse(storage.items.get(GOOGLE_PENDING_KEY)!)).toEqual({
      state,
      nonce,
      createdAt: 1_000,
    });
    expect(start().state).not.toBe(state);
  });

  it('accepts the matching answer once and returns the nonce for the server', () => {
    const { storage, state, nonce } = start();
    const hash = `#state=${state}&id_token=header.payload.sig&authuser=0`;
    expect(readGoogleCallback(hash, { storage, now: 2_000 })).toEqual({
      kind: 'ok',
      idToken: 'header.payload.sig',
      nonce,
    });
    // Used up: the same fragment can't sign in again.
    expect(readGoogleCallback(hash, { storage, now: 2_000 })).toEqual({ kind: 'invalid' });
  });

  it('refuses a wrong state, a missing token, an expired or unknown sign-in', () => {
    let s = start();
    expect(readGoogleCallback(`#state=other&id_token=t`, { storage: s.storage })).toEqual({
      kind: 'invalid',
    });
    s = start();
    expect(readGoogleCallback(`#state=${s.state}`, { storage: s.storage })).toEqual({
      kind: 'invalid',
    });
    s = start(memoryStorage(), 0);
    expect(
      readGoogleCallback(`#state=${s.state}&id_token=t`, {
        storage: s.storage,
        now: GOOGLE_PENDING_MAX_AGE_MS + 1,
      }),
    ).toEqual({ kind: 'invalid' });
    expect(readGoogleCallback(`#state=x&id_token=t`, { storage: memoryStorage() })).toEqual({
      kind: 'invalid',
    });
  });

  it('reports a declined sign-in as cancelled, other errors as invalid', () => {
    const s = start();
    expect(
      readGoogleCallback(`#error=access_denied&state=${s.state}`, { storage: s.storage }),
    ).toEqual({ kind: 'cancelled' });
    expect(s.storage.items.has(GOOGLE_PENDING_KEY)).toBe(false);
    expect(readGoogleCallback('#error=invalid_request', { storage: memoryStorage() })).toEqual({
      kind: 'invalid',
    });
  });
});
