import { screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';

import { freshDb, renderApp } from './test/render';
import { loadCoreSync } from './test/wasm';

describe('App', () => {
  beforeEach(() => {
    freshDb();
  });

  it('renders the home screen as a guest', async () => {
    renderApp();
    expect(await screen.findByText('Playing as a guest')).toBeInTheDocument();
  });

  it('asks for telemetry consent on first launch, both options off', async () => {
    renderApp();
    const dialog = await screen.findByRole('dialog', { name: 'Help improve Ouril?' });
    const boxes = dialog.querySelectorAll('input[type="checkbox"]');
    expect(boxes).toHaveLength(2);
    boxes.forEach((b) => expect(b).not.toBeChecked());
  });

  it('loads the WASM core', () => {
    const core = loadCoreSync();
    expect(core.variants().map((v) => v.id)).toContain('cv.standard');
    expect(core.defaultVariant('cv')?.id).toBe('cv.standard');
  });
});
