import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { COMING_SOON, comingSoon } from '../content/comingSoon';
import { PRIVACY, PRIVACY_SECTIONS } from '../content/privacy';
import '../i18n';
import { setConsent } from '../storage/repo';
import { AppContext } from '../state/useApp';
import { freshDb, renderApp } from '../test/render';
import { DeleteAccount } from './DeleteAccount';

describe('privacy policy and account deletion pages', () => {
  const fetchSpy = vi.fn();
  beforeEach(async () => {
    freshDb();
    fetchSpy.mockReset();
    vi.stubGlobal('fetch', fetchSpy);
    await setConsent({ decided: true, crashReports: false, usageStats: false, installId: 'x' });
  });
  afterEach(() => vi.unstubAllGlobals());

  it('/privacy shows every section, the date and the contact, without any request', async () => {
    renderApp({ route: '/privacy' });
    expect(await screen.findByRole('heading', { level: 1, name: 'Privacy policy' })).toBeVisible();
    expect(screen.getAllByRole('heading', { level: 2 })).toHaveLength(PRIVACY_SECTIONS.length);
    expect(screen.getByText(/Last updated: October 13, 2026/)).toBeVisible();
    expect(screen.getByText(new RegExp(`made by ${PRIVACY.controller}`))).toBeVisible();
    expect(screen.getByRole('link', { name: PRIVACY.email })).toHaveAttribute(
      'href',
      `mailto:${PRIVACY.email}`,
    );
    // Values are filled in, never left as ICU placeholders.
    expect(document.body.textContent).not.toMatch(/\{(name|email|days|date)\}/);
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('Settings, About and the sign-in screen link to the privacy policy', async () => {
    for (const route of ['/settings', '/about', '/sign-in']) {
      const { unmount } = renderApp({ route });
      expect((await screen.findAllByRole('link', { name: 'Privacy policy' }))[0]).toHaveAttribute(
        'href',
        '/privacy',
      );
      unmount();
    }
  });

  it('/delete-account explains the steps, the email route and what is kept', async () => {
    renderApp({ route: '/delete-account' });
    expect(
      await screen.findByRole('heading', { level: 1, name: 'Delete your Ouril account' }),
    ).toBeVisible();
    // A guest is sent to sign in first (the page's button, besides the header's link).
    const signIn = screen.getAllByRole('link', { name: 'Sign in' });
    expect(signIn.some((a) => a.classList.contains('button'))).toBe(true);
    for (const a of signIn) expect(a).toHaveAttribute('href', '/sign-in');
    expect(screen.getByText(new RegExp(`Email ${PRIVACY.email}`))).toBeVisible();
    expect(screen.getByText(/replaced within 30 days/)).toBeVisible();
    expect(screen.getByRole('link', { name: 'Read the privacy policy' })).toHaveAttribute(
      'href',
      '/privacy',
    );
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('/delete-account sends a signed-in player to Settings', () => {
    const value = { auth: 'signed_in', user: { id: 'u', display_name: 'Ana' } } as never;
    render(
      <MemoryRouter>
        <AppContext.Provider value={value}>
          <DeleteAccount />
        </AppContext.Provider>
      </MemoryRouter>,
    );
    expect(screen.getByRole('link', { name: 'Open Settings' })).toHaveAttribute(
      'href',
      '/settings',
    );
  });

  it('"Coming soon" drops accounts once they are switched on', async () => {
    expect(comingSoon({ accounts: false })).toEqual(COMING_SOON);
    expect(comingSoon({ accounts: true })).not.toContain('accounts');
    // Tests run with accounts on.
    renderApp({ route: '/' });
    expect(await screen.findByRole('heading', { name: 'Coming soon' })).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Your own account' })).toBeNull();
  });
});
