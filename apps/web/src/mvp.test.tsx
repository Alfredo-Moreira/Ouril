import { screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { freshDb, renderApp } from './test/render';

// The MVP as shipped: no telemetry service configured (VITE_SENTRY_DSN empty). The other
// tests run with one, to cover the consent flow.
vi.mock('./telemetry', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./telemetry')>()),
  telemetryConfigured: () => false,
}));

describe('MVP without telemetry', () => {
  beforeEach(() => {
    freshDb();
  });

  it('asks for no consent: there is nothing to send', async () => {
    renderApp();
    expect(await screen.findByRole('heading', { level: 1, name: 'Ouril' })).toBeInTheDocument();
    expect(screen.queryByRole('dialog', { name: 'Help improve Ouril?' })).toBeNull();
  });

  it('Settings has no privacy section', async () => {
    renderApp({ route: '/settings' });
    await screen.findByRole('heading', { level: 1, name: 'Settings' });
    expect(screen.queryByText('Crash reports')).toBeNull();
    expect(screen.queryByText('Usage statistics')).toBeNull();
  });
});
