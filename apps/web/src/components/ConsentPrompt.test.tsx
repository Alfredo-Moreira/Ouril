import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { getConsent } from '../storage/repo';
import {
  configureTelemetry,
  getTelemetryState,
  telemetryConfigured,
  trackEvent,
  usageStatsConfigured,
} from '../telemetry';
import { freshDb, renderApp } from '../test/render';

describe('first-launch telemetry consent (ADR 0014)', () => {
  beforeEach(() => {
    freshDb();
  });

  it('nothing is stored as consented before the player chooses', async () => {
    renderApp();
    await screen.findByRole('dialog', { name: 'Help improve Ouril?' });
    const c = await getConsent();
    expect(c).toMatchObject({ decided: false, crashReports: false, usageStats: false });
  });

  it('"No thanks" is one tap, as prominent as "Save choices", and saves both off', async () => {
    renderApp();
    const dialog = await screen.findByRole('dialog', { name: 'Help improve Ouril?' });
    const decline = within(dialog).getByRole('button', { name: 'No thanks' });
    const save = within(dialog).getByRole('button', { name: 'Save choices' });
    expect(decline.className).toBe(save.className);

    fireEvent.click(decline);
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(await getConsent()).toMatchObject({
      decided: true,
      crashReports: false,
      usageStats: false,
    });
  });

  it('"No thanks" wins even if a box was ticked', async () => {
    renderApp();
    const dialog = await screen.findByRole('dialog', { name: 'Help improve Ouril?' });
    fireEvent.click(within(dialog).getByRole('checkbox', { name: /Crash reports/ }));
    fireEvent.click(within(dialog).getByRole('button', { name: 'No thanks' }));
    await waitFor(async () => expect((await getConsent()).decided).toBe(true));
    expect((await getConsent()).crashReports).toBe(false);
  });

  it('"Save choices" stores exactly what was ticked, and the dialog never returns', async () => {
    const first = renderApp();
    const dialog = await screen.findByRole('dialog', { name: 'Help improve Ouril?' });
    fireEvent.click(within(dialog).getByRole('checkbox', { name: /Usage statistics/ }));
    fireEvent.click(within(dialog).getByRole('button', { name: 'Save choices' }));
    await waitFor(async () => expect((await getConsent()).decided).toBe(true));
    expect(await getConsent()).toMatchObject({ crashReports: false, usageStats: true });

    first.unmount();
    renderApp();
    expect(await screen.findByText('Playing as a guest')).toBeInTheDocument();
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});

describe('each telemetry option needs its service configured', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    configureTelemetry({ decided: false, crashReports: false, usageStats: false, installId: '' });
  });

  it('is inactive even with full consent when no service is configured', () => {
    vi.stubEnv('VITE_SENTRY_DSN', '');
    vi.stubEnv('VITE_MATOMO_URL', '');
    vi.stubEnv('VITE_MATOMO_SITE_ID', '');
    expect(telemetryConfigured()).toBe(false);
    const s = configureTelemetry({
      decided: true,
      crashReports: true,
      usageStats: true,
      installId: 'i',
    });
    expect(s).toEqual({ crashReportsActive: false, usageStatsActive: false });
    trackEvent('game_finished', { level: 'easy' });
    expect(getTelemetryState()).toEqual(s);
  });

  it('follows each choice separately and only after a decision', () => {
    expect(telemetryConfigured()).toBe(true);
    expect(
      configureTelemetry({ decided: false, crashReports: true, usageStats: true, installId: 'i' }),
    ).toEqual({ crashReportsActive: false, usageStatsActive: false });
    expect(
      configureTelemetry({ decided: true, crashReports: true, usageStats: false, installId: 'i' }),
    ).toEqual({ crashReportsActive: true, usageStatsActive: false });
    expect(
      configureTelemetry({ decided: true, crashReports: false, usageStats: true, installId: 'i' }),
    ).toEqual({ crashReportsActive: false, usageStatsActive: true });
  });

  it('usage statistics need both the Matomo URL and the site ID', () => {
    vi.stubEnv('VITE_SENTRY_DSN', '');
    vi.stubEnv('VITE_MATOMO_SITE_ID', '');
    expect(usageStatsConfigured()).toBe(false);
    expect(telemetryConfigured()).toBe(false);
  });

  it('the prompt shows only the options whose service is configured', async () => {
    freshDb();
    vi.stubEnv('VITE_SENTRY_DSN', '');
    renderApp();
    const dialog = await screen.findByRole('dialog', { name: 'Help improve Ouril?' });
    expect(within(dialog).getByRole('checkbox', { name: /Usage statistics/ })).toBeInTheDocument();
    expect(within(dialog).queryByRole('checkbox', { name: /Crash reports/ })).toBeNull();
  });
});
