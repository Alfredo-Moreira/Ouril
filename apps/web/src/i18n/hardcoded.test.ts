import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

/**
 * Guard for translation: no text shown to players may be written directly in a component.
 * Every word goes through `t()` and lives in shared/i18n/<locale>.json (ADR 0016). Flags:
 * - JSX text containing letters (`<p>Hello</p>`),
 * - literal string children (`{'Hello'}`),
 * - literal accessible or visible attributes (`aria-label="Close"`, `title`, `alt`,
 *   `placeholder`).
 * Proper nouns that are never translated go in ALLOWED.
 */
const ALLOWED = new Set<string>([]);

describe('no hard-coded text in components', () => {
  const root = join(__dirname, '..');
  const files: string[] = [];
  const walk = (dir: string) => {
    for (const name of readdirSync(dir)) {
      const p = join(dir, name);
      if (name === 'generated' || name === 'test') continue;
      if (statSync(p).isDirectory()) walk(p);
      else if (name.endsWith('.tsx') && !name.includes('.test.')) files.push(p);
    }
  };
  walk(root);

  it('every visible or accessible string goes through t()', () => {
    const found: string[] = [];
    for (const f of files) {
      const src = readFileSync(f, 'utf8')
        .replace(/\/\*[\s\S]*?\*\//g, '')
        .replace(/^\s*\/\/.*$/gm, '')
        .replace(/\{\/\*[\s\S]*?\*\/\}/g, '');
      const at = (i: number) => `${f.slice(root.length + 1)}:${src.slice(0, i).split('\n').length}`;
      // JSX text between a tag's end and the next tag or expression.
      for (const m of src.matchAll(/>([^<>{}]*[A-Za-z][^<>{}]*)</g)) {
        const text = m[1]!.trim();
        // `=>` (an arrow, not the end of a tag), e.g. `() => new Promise<void>`.
        if (src[m.index! - 1] === '=') continue;
        // Skip TypeScript generics and arrows (`useRef<HTMLElement>(null)`, `=> {...}`).
        if (/[;=()]|=>|\bconst\b|\breturn\b/.test(text)) continue;
        if (text && !ALLOWED.has(text)) found.push(`${at(m.index!)} text ${JSON.stringify(text)}`);
      }
      // A literal string as a JSX child (after a tag, not an attribute value: `id={`a-${b}`}`).
      for (const m of src.matchAll(/>\s*\{\s*(['"`])([^'"`]*[A-Za-z]{2,}[^'"`]*)\1\s*\}/g)) {
        if (!ALLOWED.has(m[2]!)) found.push(`${at(m.index!)} child ${JSON.stringify(m[2])}`);
      }
      for (const m of src.matchAll(
        /\b(aria-label|aria-description|title|alt|placeholder)=(["'])([^"']*[A-Za-z][^"']*)\2/g,
      )) {
        if (!ALLOWED.has(m[3]!)) found.push(`${at(m.index!)} ${m[1]}=${JSON.stringify(m[3])}`);
      }
    }
    expect(found).toEqual([]);
  });
});
