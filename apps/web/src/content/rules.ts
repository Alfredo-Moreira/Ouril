/**
 * Rules text per playable variant, shown on the Rules page and in the game's rules dialog.
 * Text lives in shared/i18n/en.json (`rules.<section>.title` and `rules.<section>.<n>`);
 * section ids + paragraph counts must match those keys (checked by i18n/keys.test.ts).
 * Standard mirrors docs/game/rules.md; a regional variant lists only what differs from the
 * variant it's based on (its spec in docs/game/variants/).
 */
import type { PlayableVariant } from '../game/session';

export interface RulesSection {
  id: string;
  paragraphs: number;
}

export interface VariantRules {
  /** i18n key of the short introduction. */
  intro: string;
  /** For a variant that only lists its differences: the variant it otherwise follows. */
  basedOn?: PlayableVariant;
  sections: RulesSection[];
}

export const VARIANT_RULES: Record<PlayableVariant, VariantRules> = {
  'cv.standard': {
    intro: 'rules.intro',
    sections: [
      { id: 'setup', paragraphs: 3 },
      { id: 'sowing', paragraphs: 3 },
      { id: 'single_seed', paragraphs: 1 },
      { id: 'lap', paragraphs: 1 },
      { id: 'capture', paragraphs: 3 },
      { id: 'feeding', paragraphs: 3 },
      { id: 'grand_slam', paragraphs: 4 },
      { id: 'end', paragraphs: 5 },
    ],
  },
  'cv.continuous': {
    intro: 'rules.continuous.intro',
    basedOn: 'cv.standard',
    sections: [
      { id: 'continuous_relay', paragraphs: 2 },
      { id: 'continuous_stop', paragraphs: 1 },
      { id: 'continuous_other', paragraphs: 3 },
    ],
  },
  'cv.across': {
    intro: 'rules.across.intro',
    basedOn: 'cv.standard',
    sections: [
      { id: 'across_capture', paragraphs: 3 },
      { id: 'across_strategy', paragraphs: 1 },
    ],
  },
  'cv.across-continuous': {
    intro: 'rules.across_continuous.intro',
    basedOn: 'cv.continuous',
    sections: [
      { id: 'across_capture', paragraphs: 3 },
      { id: 'across_continuous_end', paragraphs: 1 },
      { id: 'across_strategy', paragraphs: 1 },
    ],
  },
};
