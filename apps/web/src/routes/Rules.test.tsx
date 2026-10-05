import { fireEvent, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';

import { freshDb, renderApp } from '../test/render';

describe('Rules: one tab per variant, collapsible sections', () => {
  beforeEach(() => {
    freshDb();
  });

  it('opens on Standard with only the first section expanded; tabs switch variant', async () => {
    renderApp({ route: '/rules' });
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    const standard = screen.getByRole('tab', { name: 'Standard' });
    expect(standard).toHaveAttribute('aria-selected', 'true');
    const panel = screen.getByRole('tabpanel');
    const details = panel.querySelectorAll('details');
    expect(details.length).toBe(8);
    expect([...details].filter((d) => d.open)).toHaveLength(1);

    fireEvent.click(within(panel).getByRole('button', { name: 'Expand all' }));
    expect([...panel.querySelectorAll('details')].every((d) => d.open)).toBe(true);

    fireEvent.click(screen.getByRole('tab', { name: 'Continuous sowing' }));
    expect(screen.getByRole('tab', { name: 'Continuous sowing' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
    const continuous = screen.getByRole('tabpanel');
    expect(continuous).toHaveTextContent('Relay sowing');
    expect(continuous).toHaveTextContent('Everything else follows Standard.');
    fireEvent.click(within(continuous).getByRole('button', { name: 'See the Standard rules' }));
    expect(screen.getByRole('tab', { name: 'Standard' })).toHaveAttribute('aria-selected', 'true');
  });

  it('a tab can be linked: /rules?variant=cv.continuous', async () => {
    renderApp({ route: '/rules?variant=cv.continuous' });
    expect(await screen.findByRole('tab', { name: 'Continuous sowing' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
  });

  it('in a game, the rules button shows the rules of the variant being played', async () => {
    renderApp();
    fireEvent.click(await screen.findByRole('button', { name: 'No thanks' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Continuous sowing' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Me' }));
    fireEvent.click(screen.getByRole('button', { name: 'Start game' }));
    await screen.findByRole('group', { name: 'Game board' });

    fireEvent.click(screen.getByRole('button', { name: 'Rules' }));
    const dialog = await screen.findByRole('dialog', { name: 'Rules: Continuous sowing' });
    expect(dialog).toHaveTextContent('Relay sowing');
    fireEvent.click(within(dialog).getByRole('button', { name: 'Back to the game' }));
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});
