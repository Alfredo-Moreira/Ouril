import { screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { freshDb, renderApp } from './test/render';

// The MVP: sign-in and Stats are off (the other tests run with them on, see vite.config.ts).
vi.mock('./features', () => ({ FEATURES: { accounts: false, stats: false } }));

describe('unreleased features are hidden (MVP)', () => {
  beforeEach(() => {
    freshDb();
  });

  it('no sign-in or Stats in the navigation; About is there', async () => {
    renderApp();
    await screen.findByRole('button', { name: 'No thanks' });
    expect(screen.queryByRole('link', { name: /Sign in/ })).toBeNull();
    expect(screen.queryByRole('link', { name: 'Stats' })).toBeNull();
    expect(screen.queryByText('Playing as a guest')).toBeNull();
    expect(screen.getAllByRole('link', { name: 'About' }).length).toBeGreaterThan(0);
  });

  it('the hidden pages are not reachable by URL', async () => {
    renderApp({ route: '/sign-in' });
    expect(await screen.findByRole('heading', { level: 1 })).not.toHaveTextContent('Sign in');
  });

  it('Settings has no account section', async () => {
    renderApp({ route: '/settings' });
    await screen.findByRole('heading', { level: 1, name: 'Settings' });
    expect(screen.queryByText('Account')).toBeNull();
  });

  it('the About page shows the developer', async () => {
    renderApp({ route: '/about' });
    expect(await screen.findByRole('heading', { level: 1, name: 'Alfredo Moreira' })).toBeVisible();
    expect(screen.getByRole('img', { name: 'Photo of Alfredo Moreira' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'GitHub' })).toHaveAttribute(
      'href',
      'https://github.com/Alfredo-Moreira',
    );
  });

  it('buy me a coffee: each option is pre-filled with $5 where the service allows it', async () => {
    renderApp({ route: '/about' });
    await screen.findByRole('heading', { level: 1, name: 'Alfredo Moreira' });
    const href = (name: RegExp) => screen.getByRole('link', { name }).getAttribute('href');
    expect(href(/PayPal/)).toBe('https://paypal.me/devopsmoreira/5USD');
    expect(href(/Venmo/)).toBe(
      'https://venmo.com/cv_badiu?txn=pay&amount=5&note=A%20coffee%20for%20Ouril',
    );
    expect(href(/Cash App/)).toBe('https://cash.app/$cvbadiu007/5');
    expect(screen.getByText('amoreira20149@gmail.com')).toBeInTheDocument();
    expect(screen.getAllByText('Send $5')).toHaveLength(4);
  });

  it('the header’s “Buy a board” opens the About page at the boards section', async () => {
    renderApp();
    await screen.findByRole('button', { name: 'No thanks' });
    expect(screen.getByRole('link', { name: 'Buy a board' })).toHaveAttribute('href', '/about#buy');
  });
});
