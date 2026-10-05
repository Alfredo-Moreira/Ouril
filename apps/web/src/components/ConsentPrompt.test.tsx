import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { getConsent } from '../storage/repo';
import {
  configureTelemetry,
  getTelemetryState,
  telemetryConfigured,
  trackEvent,
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

describe('telemetry is a no-op without a DSN', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    configureTelemetry({ decided: false, crashReports: false, usageStats: false, installId: '' });
  });

  it('is inactive even with full consent when VITE_SENTRY_DSN is not set', () => {
    vi.stubEnv('VITE_SENTRY_DSN', '');
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

  it('with a DSN, follows each choice separately and only after a decision', () => {
    vi.stubEnv('VITE_SENTRY_DSN', 'https://public@example.invalid/1');
    expect(telemetryConfigured()).toBe(true);
    expect(
      configureTelemetry({ decided: false, crashReports: true, usageStats: true, installId: 'i' }),
    ).toEqual({ crashReportsActive: false, usageStatsActive: false });
    expect(
      configureTelemetry({ decided: true, crashReports: true, usageStats: false, installId: 'i' }),
    ).toEqual({ crashReportsActive: true, usageStatsActive: false });
  });
});
