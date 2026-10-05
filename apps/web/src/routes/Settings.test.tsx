import { fireEvent, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';

import { getDb } from '../storage/db';
import { DEFAULT_SETTINGS, getConsent, getSettings, setConsent } from '../storage/repo';
import { freshDb, renderApp } from '../test/render';

describe('Settings screen', () => {
  beforeEach(async () => {
    freshDb();
    await setConsent({ decided: true, crashReports: false, usageStats: false, installId: 'id-1' });
  });

  it('starts from the defaults: sound on, hints off, English', async () => {
    renderApp({ route: '/settings' });
    expect(await screen.findByRole('checkbox', { name: 'Sound effects' })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: 'Show the hint button' })).not.toBeChecked();
    expect(screen.getByRole('combobox', { name: 'Language' })).toHaveValue('en');
    expect(DEFAULT_SETTINGS).toEqual({ sound: true, language: 'en', hints: false });
  });

  it('persists game settings locally and keeps them after a reload', async () => {
    const first = renderApp({ route: '/settings' });
    fireEvent.click(await screen.findByRole('checkbox', { name: 'Sound effects' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Show the hint button' }));
    await waitFor(async () =>
      expect(await getSettings()).toEqual({ sound: false, hints: true, language: 'en' }),
    );

    // Each change is queued as a settings_updated mutation (uploaded only if signed in).
    const outbox = await getDb().outbox.toArray();
    expect(outbox.map((r) => r.mutation)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ type: 'settings_updated', payload: { sound: false } }),
        expect.objectContaining({ type: 'settings_updated', payload: { hints: true } }),
      ]),
    );
    expect(outbox.every((r) => r.owner.startsWith('guest:'))).toBe(true);

    first.unmount();
    renderApp({ route: '/settings' });
    expect(await screen.findByRole('checkbox', { name: 'Sound effects' })).not.toBeChecked();
    expect(screen.getByRole('checkbox', { name: 'Show the hint button' })).toBeChecked();
  });

  it('persists telemetry choices (when a telemetry service is configured)', async () => {
    renderApp({ route: '/settings' });
    const crash = await screen.findByRole('checkbox', { name: 'Crash reports' });
    const usage = screen.getByRole('checkbox', { name: 'Usage statistics' });
    expect(crash).not.toBeChecked();
    expect(usage).not.toBeChecked();

    fireEvent.click(crash);
    await waitFor(async () => expect((await getConsent()).crashReports).toBe(true));
    expect((await getConsent()).usageStats).toBe(false);
    // Consent is never queued for sync.
    expect(
      (await getDb().outbox.toArray()).filter((r) => r.mutation.type !== 'settings_updated'),
    ).toEqual([]);
  });

  it('resets the anonymous install ID', async () => {
    renderApp({ route: '/settings' });
    fireEvent.click(await screen.findByRole('button', { name: 'Reset anonymous ID' }));
    await waitFor(async () => expect((await getConsent()).installId).not.toBe('id-1'));
    expect((await getConsent()).installId).toMatch(/^[0-9a-f-]{36}$/);
  });

  it('a guest sees "Sign in", never sign out or delete account', async () => {
    renderApp({ route: '/settings' });
    expect(await screen.findByText('Playing as a guest')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Sign out' })).toBeNull();
    expect(screen.queryByRole('button', { name: /Delete account/i })).toBeNull();
  });
});
