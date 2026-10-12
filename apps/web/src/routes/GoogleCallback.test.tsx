import { screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiClient, ApiError } from '../api/client';
import type { Me } from '../api/types';
import { GOOGLE_PENDING_KEY } from '../auth/google';
import { getActiveOwner, setConsent } from '../storage/repo';
import { freshDb, renderApp } from '../test/render';

const ME = { id: 'user-g', display_name: 'Ana Lopes' } as Me;

/** As if the sign-in started in this tab and Google redirected back with `fragment`. */
function returnFromGoogle(fragment: string, pending = { state: 's1', nonce: 'n1' }) {
  sessionStorage.setItem(GOOGLE_PENDING_KEY, JSON.stringify({ ...pending, createdAt: Date.now() }));
  window.history.replaceState(null, '', `/auth/google#${fragment}`);
}

function fakeApi(signIn: ApiClient['googleSignIn']) {
  return {
    googleSignIn: vi.fn(signIn),
    // Sync starts after sign-in; keep it off the network.
    syncPush: vi.fn(async () => ({ results: [] })),
    syncPull: vi.fn(async () => ({ changes: [], cursor: 0, has_more: false })),
    meta: vi.fn(async () => {
      throw new Error('offline');
    }),
  } as unknown as ApiClient & { googleSignIn: ReturnType<typeof vi.fn> };
}

describe('Google sign-in callback (/auth/google)', () => {
  beforeEach(async () => {
    freshDb();
    vi.stubGlobal('fetch', vi.fn());
    await setConsent({ decided: true, crashReports: false, usageStats: false, installId: 'x' });
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    sessionStorage.clear();
    window.history.replaceState(null, '', '/');
  });

  it('signs in with the token and nonce, removes the token from the URL and claims guest data', async () => {
    const guest = await getActiveOwner();
    returnFromGoogle('state=s1&id_token=the.id.token');
    const api = fakeApi(async () => ({
      tokens: { access_token: 'a', token_type: 'Bearer', expires_in: 900 },
      user: ME,
      is_new_user: true,
    }));
    renderApp({ route: '/auth/google', createApi: () => api });
    await waitFor(() =>
      expect(api.googleSignIn).toHaveBeenCalledWith({ id_token: 'the.id.token', nonce: 'n1' }),
    );
    expect(window.location.hash).toBe('');
    expect(sessionStorage.getItem(GOOGLE_PENDING_KEY)).toBeNull();
    await waitFor(async () => expect(await getActiveOwner()).toBe('user-g'));
    expect(guest).toMatch(/^guest:/);
    expect(await screen.findByRole('link', { name: 'Ana Lopes' })).toBeInTheDocument();
    expect(api.googleSignIn).toHaveBeenCalledTimes(1);
  });

  it('a cancelled sign-in makes no request and offers to try again', async () => {
    returnFromGoogle('error=access_denied&state=s1');
    const api = fakeApi(async () => {
      throw new Error('should not be called');
    });
    renderApp({ route: '/auth/google', createApi: () => api });
    expect(await screen.findByText(/Google sign-in was cancelled/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Try again' })).toHaveAttribute('href', '/sign-in');
    expect(api.googleSignIn).not.toHaveBeenCalled();
  });

  it('a wrong state makes no request', async () => {
    returnFromGoogle('state=forged&id_token=t');
    const api = fakeApi(async () => {
      throw new Error('should not be called');
    });
    renderApp({ route: '/auth/google', createApi: () => api });
    expect(await screen.findByRole('alert')).toHaveTextContent(/didn't finish/);
    expect(api.googleSignIn).not.toHaveBeenCalled();
  });

  it('shows the server error, for example while Google sign-in is off on the server', async () => {
    returnFromGoogle('state=s1&id_token=t');
    const api = fakeApi(async () => {
      throw new ApiError(501, 'not_configured', 'nope');
    });
    renderApp({ route: '/auth/google', createApi: () => api });
    expect(await screen.findByRole('alert')).toHaveTextContent(/isn’t available yet/);
    expect(await getActiveOwner()).toMatch(/^guest:/);
  });
});
