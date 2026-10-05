import { fireEvent, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiClient, ApiError } from '../api/client';
import type { Me } from '../api/types';
import { getActiveOwner, setConsent } from '../storage/repo';
import { freshDb, renderApp } from '../test/render';

const ME = { id: 'user-1', display_name: 'Dev Player' } as Me;

describe('Sign-in screen', () => {
  const fetchSpy = vi.fn();
  beforeEach(async () => {
    freshDb();
    fetchSpy.mockReset();
    vi.stubGlobal('fetch', fetchSpy);
    await setConsent({ decided: true, crashReports: false, usageStats: false, installId: 'x' });
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it('Google and Apple are disabled placeholders that make no request', async () => {
    renderApp({ route: '/sign-in' });
    const google = await screen.findByRole('button', { name: /Sign in with Google/ });
    const apple = screen.getByRole('button', { name: /Sign in with Apple/ });
    expect(google).toBeDisabled();
    expect(apple).toBeDisabled();
    expect(google).toHaveAccessibleDescription(/isn't available yet/);
    fireEvent.click(google);
    fireEvent.click(apple);
    expect(fetchSpy).not.toHaveBeenCalled();
    expect(screen.getAllByText('Coming soon')).toHaveLength(2);
    // Playing on as a guest is always offered.
    expect(screen.getByRole('link', { name: /keep playing/i })).toBeInTheDocument();
  });

  it('dev builds show "Dev sign-in", which signs in and claims guest data', async () => {
    const guest = await getActiveOwner();
    const api = { devSignIn: vi.fn(async () => ({ user: ME })) } as unknown as ApiClient;
    // Sync starts after sign-in; keep it from touching the (stubbed) network.
    Object.assign(api, {
      syncPush: vi.fn(async () => ({ results: [] })),
      syncPull: vi.fn(async () => ({ changes: [], cursor: 0, has_more: false })),
    });
    renderApp({ route: '/sign-in', createApi: () => api });
    fireEvent.click(await screen.findByRole('button', { name: 'Dev sign-in' }));
    await waitFor(() => expect(api.devSignIn).toHaveBeenCalledWith({}));
    await waitFor(async () => expect(await getActiveOwner()).toBe('user-1'));
    expect(guest).toMatch(/^guest:/);
    expect(await screen.findByRole('link', { name: 'Dev Player' })).toBeInTheDocument();
  });

  it('shows the error code from the server when dev sign-in fails', async () => {
    const api = {
      devSignIn: vi.fn(async () => {
        throw new ApiError(501, 'not_configured', 'nope');
      }),
    } as unknown as ApiClient;
    renderApp({ route: '/sign-in', createApi: () => api });
    fireEvent.click(await screen.findByRole('button', { name: 'Dev sign-in' }));
    expect(await screen.findByRole('alert')).toBeInTheDocument();
    expect(screen.getByRole('alert').textContent).not.toMatch(/^error\./);
  });

  it('production builds (import.meta.env.DEV = false) do not render Dev sign-in', async () => {
    vi.stubEnv('DEV', false);
    vi.stubEnv('PROD', true);
    vi.resetModules();
    // Re-import so the module-level DEV constant is re-evaluated, with matching React copies.
    const rtl = await import('@testing-library/react');
    const { MemoryRouter } = await import('react-router');
    await import('../i18n');
    const { SignIn } = await import('./SignIn');
    const { AppContext } = await import('../state/useApp');
    const value = { auth: 'guest', user: null, signInDev: vi.fn() } as never;
    rtl.render(
      <MemoryRouter>
        <AppContext.Provider value={value}>
          <SignIn />
        </AppContext.Provider>
      </MemoryRouter>,
    );
    expect(await rtl.screen.findByRole('button', { name: /Sign in with Google/ })).toBeDisabled();
    expect(rtl.screen.queryByRole('button', { name: 'Dev sign-in' })).toBeNull();
    expect(rtl.screen.queryByText(/Development build/)).toBeNull();
    rtl.cleanup();
  });
});
