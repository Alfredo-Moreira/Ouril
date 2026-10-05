/**
 * Interactive tutorial: fixed positions from docs/game/rules.md (worked examples) for
 * cv.standard, then the continuous-sowing rules (cv.continuous, its spec in
 * docs/game/variants/cape-verde/continuous.md). The engine decides every outcome; these
 * positions only set up the lesson.
 * Text: `tutorial.<id>.title|intro|done` (and `.next` between moves) in shared/i18n/en.json.
 *
 * Each position has 48 seeds in total (pits + stores). tutorial/steps.test.ts checks every
 * step against the real engine.
 */
import type { GameState } from '../engine/types';
import type { PlayableVariant } from '../game/session';

export interface TutorialStep {
  id: string;
  /** The rules the step is played with; absent = standard. */
  variant?: PlayableVariant;
  position: GameState;
  /** One entry per player move: the pits the lesson allows, or 'any' legal pit. */
  moves: (number[] | 'any')[];
}

function pos(south: number[], north: number[], stores: [number, number]): GameState {
  return {
    pits: [...south, ...north],
    stores,
    to_move: 'south',
    moves_since_capture: 0,
    status: 'playing',
  };
}

export const TUTORIAL_STEPS: TutorialStep[] = [
  // Example 1: simple sowing.
  { id: 'sowing', position: pos([4, 4, 4, 4, 4, 4], [4, 4, 4, 4, 4, 4], [0, 0]), moves: [[2]] },
  // A single capture: the last seed makes 2 in North's pit 6.
  { id: 'capture', position: pos([4, 4, 4, 4, 2, 3], [1, 4, 4, 4, 4, 4], [3, 3]), moves: [[4]] },
  // Example 2: capture with a chain.
  { id: 'chain', position: pos([4, 4, 4, 4, 3, 0], [1, 2, 4, 4, 4, 4], [5, 5]), moves: [[4]] },
  // Example 3: a lap of 12+ seeds skips the origin pit.
  { id: 'lap', position: pos([12, 0, 0, 0, 0, 0], [9, 9, 9, 9, 0, 0], [0, 0]), moves: [[0]] },
  // Example 4: the single-seed rule.
  {
    id: 'single_seed',
    position: pos([1, 0, 3, 1, 0, 2], [4, 4, 4, 4, 4, 4], [9, 8]),
    moves: ['any'],
  },
  // Example 6: must feed a starving opponent.
  {
    id: 'feeding',
    position: pos([0, 2, 0, 0, 0, 1], [0, 0, 0, 0, 0, 0], [21, 24]),
    moves: ['any'],
  },
  // Example 5: grand slam, then the extra move must feed.
  {
    id: 'grand_slam',
    position: pos([0, 0, 0, 1, 3, 0], [1, 2, 0, 0, 0, 0], [18, 23]),
    moves: [[4], 'any'],
  },
  // Continuous sowing: the opening move carries on from an occupied own pit.
  {
    id: 'continuous',
    variant: 'cv.continuous',
    position: pos([4, 4, 4, 4, 4, 4], [4, 4, 4, 4, 4, 4], [0, 0]),
    moves: [[0]],
  },
  // Continuous sowing: a lone seed starts a relay that ends with a capture.
  {
    id: 'continuous_capture',
    variant: 'cv.continuous',
    position: pos([2, 3, 4, 1, 1, 3], [1, 4, 4, 4, 4, 4], [6, 7]),
    moves: [[3]],
  },
  // Capture across: three seeds land in empty pits 2-4 and capture across, with the chain.
  {
    id: 'across',
    variant: 'cv.across',
    position: pos([3, 0, 0, 0, 4, 4], [4, 4, 4, 4, 4, 4], [6, 7]),
    moves: [[0]],
  },
];
