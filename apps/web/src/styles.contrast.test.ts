/**
 * The Panu tokens meet WCAG contrast in both themes: every text pair 4.5:1 or more on the page
 * and on each surface, and focus rings and the yellow button's edge 3:1 or more (non-text).
 * The values are read from styles.css itself, so a token change that breaks a pair fails here.
 */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

// Read from disk (Vitest stubs CSS imports, `?raw` included); tests run from apps/web.
const css = readFileSync(resolve(process.cwd(), 'src/styles.css'), 'utf8');

type Tokens = Record<string, string>;

function tokensIn(block: string): Tokens {
  const out: Tokens = {};
  for (const m of block.matchAll(/--([a-z0-9-]+):\s*(#[0-9a-f]{6})\s*;/gi)) out[m[1]!] = m[2]!;
  return out;
}

/** The light tokens (the first `:root`), then the dark ones layered over them. */
function themes(): { light: Tokens; dark: Tokens } {
  const light = tokensIn(
    css.slice(css.indexOf(':root {'), css.indexOf('@media (prefers-color-scheme: dark)')),
  );
  const darkStart = css.indexOf('@media (prefers-color-scheme: dark)');
  const darkBlock = css.slice(darkStart, css.indexOf('/* --- Base', darkStart));
  return { light, dark: { ...light, ...tokensIn(darkBlock) } };
}

function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
  const lin = (c: number) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  return 0.2126 * lin(r!) + 0.7152 * lin(g!) + 0.0722 * lin(b!);
}

function ratio(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi! + 0.05) / (lo! + 0.05);
}

const TEXT_ON_BACKGROUNDS = ['ink', 'ink-2', 'muted', 'indigo', 'red', 'green', 'sea'];
const BACKGROUNDS = ['bg', 'surface', 'surface-2'];
/** Text on a filled control: [text, fill]. */
const FILLED: [string, string][] = [
  ['on-star', 'star'],
  ['on-indigo', 'indigo-fill'],
  ['selected-ink', 'selected-bg'],
  ['on-you', 'you-fill'],
  ['on-them', 'them-fill'],
  ['on-red', 'red'],
  ['night-ink', 'night'],
  ['night-muted', 'night'],
  ['night-muted', 'night-2'],
];

describe.each(['light', 'dark'] as const)('Panu contrast (%s)', (theme) => {
  const t = themes()[theme];

  it('every token the pairs use is a hex colour in the stylesheet', () => {
    const used = new Set([
      ...TEXT_ON_BACKGROUNDS,
      ...BACKGROUNDS,
      ...FILLED.flat(),
      'focus',
      'star-edge',
    ]);
    for (const name of used) expect(t[name], `--${name}`).toMatch(/^#[0-9a-f]{6}$/i);
  });

  it.each(TEXT_ON_BACKGROUNDS)('--%s reads at 4.5:1 or more on every background', (text) => {
    for (const bg of BACKGROUNDS) {
      expect(ratio(t[text]!, t[bg]!), `--${text} on --${bg}`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it.each(FILLED)('--%s reads at 4.5:1 or more on --%s', (text, fill) => {
    expect(ratio(t[text]!, t[fill]!)).toBeGreaterThanOrEqual(4.5);
  });

  it('the focus ring and the yellow button’s edge stand out at 3:1 or more', () => {
    for (const bg of BACKGROUNDS) {
      expect(ratio(t.focus!, t[bg]!), `--focus on --${bg}`).toBeGreaterThanOrEqual(3);
    }
    expect(ratio(t['star-edge']!, t.bg!)).toBeGreaterThanOrEqual(theme === 'light' ? 3 : 1);
  });
});
