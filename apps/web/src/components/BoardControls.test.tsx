import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { BoardControls } from './BoardControls';

describe('BoardControls (the 3D board’s keyboard and screen-reader interface)', () => {
  const props = {
    pits: [4, 4, 4, 4, 4, 0, 4, 4, 4, 4, 4, 4],
    stores: [0, 0] as [number, number],
    legal: [0, 1, 2, 3, 4],
    southLabel: 'You',
    northLabel: 'Computer',
  };

  it('labels every pit and store, and plays legal pits with Enter or a click', () => {
    const onPlay = vi.fn();
    const onFocusPit = vi.fn();
    render(<BoardControls {...props} onPlay={onPlay} onFocusPit={onFocusPit} />);
    const board = screen.getByRole('group', { name: 'Game board' });
    expect(within(board).getAllByRole('img')).toHaveLength(8); // 6 opponent pits + 2 stores

    const pit3 = within(board).getByRole('button', { name: 'Your pit 3, 4 seeds' });
    fireEvent.focus(pit3);
    expect(onFocusPit).toHaveBeenCalledWith(2);
    fireEvent.keyDown(pit3, { key: 'Enter' });
    expect(onPlay).toHaveBeenCalledWith(2);

    const empty = within(board).getByRole('button', { name: /pit 6/ });
    expect(empty).toHaveAttribute('aria-disabled', 'true');
    fireEvent.click(empty);
    expect(onPlay).toHaveBeenCalledTimes(1);
  });

  it('draws nothing: the whole interface is visually hidden', () => {
    render(<BoardControls {...props} />);
    expect(screen.getByRole('group', { name: 'Game board' })).toHaveClass('visually-hidden');
    expect(document.querySelector('svg')).toBeNull();
  });
});
