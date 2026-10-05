import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { BOARD_KINDS } from '../content/boards';
import { COMING_SOON } from '../content/comingSoon';
import { LEVELS } from '../engine/types';
import { PLAYABLE_VARIANTS } from '../game/session';
import en from '../generated/i18n/en.json';
import i18n, { SUPPORTED_LOCALES } from '../i18n';
import { TUTORIAL_STEPS } from '../tutorial/steps';

const keys = new Set(Object.keys(en));
const has = (k: string) => keys.has(k);

/** Every literal t('…') key used in src/ exists in shared/i18n/en.json. */
describe('i18n keys', () => {
  const root = join(__dirname, '..');
  const files: string[] = [];
  const walk = (dir: string) => {
    for (const name of readdirSync(dir)) {
      const p = join(dir, name);
      if (name === 'generated' || name === 'test') continue;
      if (statSync(p).isDirectory()) walk(p);
      else if (/\.tsx?$/.test(name) && !/\.test\.tsx?$/.test(name)) files.push(p);
    }
  };
  walk(root);

  it('has no missing literal keys', () => {
    const missing: string[] = [];
    for (const f of files) {
      const src = readFileSync(f, 'utf8');
      for (const m of src.matchAll(/\bt\(\s*'([a-z0-9_.]+)'/g)) {
        if (!has(m[1]!)) missing.push(`${m[1]} (${f})`);
      }
    }
    expect(missing).toEqual([]);
  });

  it('has every key built at runtime from a template (t(`prefix.${x}`))', () => {
    const families: Record<string, readonly string[]> = {
      'game.level': LEVELS,
      'home.starter': ['me', 'ai', 'random'],
      'game.over': ['win', 'loss', 'draw'],
      'game.over.reason': ['threshold', 'no_feed', 'endless_cycle', 'no_moves', 'resigned'],
      'game.over.tagline': ['win', 'loss', 'draw'],
      'stats.result': ['win', 'loss', 'draw'],
      'sync.status': ['idle', 'syncing', 'offline', 'error', 'auth_required', 'upgrade_required'],
      'settings.language': SUPPORTED_LOCALES,
      menu: ['tutorial', 'rules', 'stats', 'settings', 'about'],
      variant: PLAYABLE_VARIANTS.flatMap((v) => [`${v}.short`, `${v}.about`]),
      'home.tile': ['tutorial', 'rules', 'stats', 'settings', 'about'],
      shop: BOARD_KINDS.flatMap((k) => [`${k}.title`, `${k}.body`]),
      'about.coffee': ['paypal', 'venmo', 'cashapp'],
      'about.social': [
        'email',
        'linkedin',
        'github',
        'instagram',
        'facebook',
        'twitch',
        'website',
        'x',
      ],
      'home.soon': COMING_SOON.flatMap((id) => [`${id}.title`, `${id}.body`]),
      'home.steps': [1, 2, 3, 4].flatMap((n) => [`${n}.title`, `${n}.body`]),
      'home.features': ['offline', 'levels', 'guest'].flatMap((k) => [`${k}.title`, `${k}.body`]),
      // Error codes the client can produce itself (others fall back to error.internal).
      error: ['internal', 'network'],
    };
    const missing: string[] = [];
    for (const [prefix, values] of Object.entries(families))
      for (const v of values) if (!has(`${prefix}.${v}`)) missing.push(`${prefix}.${v}`);
    for (const step of TUTORIAL_STEPS) {
      // `.next` is only shown between the moves of a multi-move step.
      const parts = ['title', 'intro', 'done', ...(step.moves.length > 1 ? ['next'] : [])];
      for (const part of parts)
        if (!has(`tutorial.${step.id}.${part}`)) missing.push(`tutorial.${step.id}.${part}`);
    }
    // Rules sections (ids + paragraph counts are declared in content/rules.ts).
    const rulesSrc = readFileSync(join(root, 'content/rules.ts'), 'utf8');
    const sections = [...rulesSrc.matchAll(/\{ id: '([a-z_]+)', paragraphs: (\d+) \}/g)];
    expect(sections.length).toBeGreaterThan(0);
    for (const [, id, n] of sections) {
      for (const part of ['title', ...Array.from({ length: Number(n) }, (_, i) => `${i + 1}`)])
        if (!has(`rules.${id}.${part}`)) missing.push(`rules.${id}.${part}`);
    }
    for (const intro of rulesSrc.matchAll(/intro: '([a-z0-9_.]+)'/g))
      if (!has(intro[1]!)) missing.push(intro[1]!);
    expect(missing).toEqual([]);
  });

  it('every template prefix used in src/ is covered above', () => {
    const covered = [
      'game.level.',
      'home.starter.',
      'game.over.',
      'game.over.reason.',
      'game.over.tagline.',
      'stats.result.',
      'sync.status.',
      'settings.language.',
      'menu.',
      'home.tile.',
      'variant.',
      'home.steps.',
      'home.features.',
      'about.social.',
      'about.coffee.',
      'shop.',
      'home.soon.',
      'error.',
      'tutorial.',
      'rules.',
    ];
    const unknown: string[] = [];
    for (const f of files) {
      const src = readFileSync(f, 'utf8');
      for (const m of src.matchAll(/\bt\(\s*`([a-z0-9_.]*)\$\{/g)) {
        if (!covered.includes(m[1]!)) unknown.push(`${m[1]} (${f})`);
      }
    }
    expect(unknown).toEqual([]);
  });

  it('every message formats (ICU) without falling back to its key', () => {
    const sample = {
      count: 2,
      number: 3,
      name: 'Ana',
      label: 'Pit',
      level: 'Easy',
      you: 1,
      ai: 2,
      step: 1,
      total: 7,
      code: 'x',
    };
    const broken: string[] = [];
    for (const k of keys) {
      const out = i18n.t(k, sample);
      if (typeof out !== 'string' || out === k || out.trim() === '') broken.push(k);
    }
    expect(broken).toEqual([]);
  });

  it('plural messages pick singular and plural forms', () => {
    expect(i18n.t('board.pit.yours', { number: 1, count: 1 })).toBe('Your pit 1, 1 seed');
    expect(i18n.t('board.pit.yours', { number: 1, count: 0 })).toBe('Your pit 1, 0 seeds');
    expect(i18n.t('board.pit.yours', { number: 1, count: 12 })).toBe('Your pit 1, 12 seeds');
  });
});
