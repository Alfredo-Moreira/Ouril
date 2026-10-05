/**
 * "Play on a real board": where to buy a physical mancala board. Shown at the bottom of the
 * About page (`/about#buy`, linked from the header) and on the home page. A board with an
 * empty `url` is hidden in production (development shows a placeholder).
 */
export const BOARD_KINDS = ['classic', 'premium'] as const;
export type BoardKind = (typeof BOARD_KINDS)[number];

export const BOARDS = {
  /** Set to true if these are Amazon Associates (affiliate) links: the page then shows the
   *  required disclosure and marks the links `rel="sponsored"`. */
  affiliate: true,
  links: {
    /** The default recommendation: Classic Mancala (Amazon B01ECIKH8A). */
    classic: 'https://link.amazon/B0aObbq83',
    /** The premium recommendation: African Oware seed board (Amazon B01MSO63IV). */
    premium: 'https://link.amazon/B0hlhvh2c',
  } satisfies Record<BoardKind, string>,
};

/** Whether there's a board to show (development always shows placeholders). Hides the
 *  header's "Buy a board" button until a link is filled in. */
export function hasBoardLinks(): boolean {
  return import.meta.env.DEV || BOARD_KINDS.some((k) => BOARDS.links[k]);
}
