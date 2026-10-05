/**
 * Client-side checks mirroring the server's profile rules (apps/server/src/validate.rs), so
 * the form can explain problems before anything is queued. The server stays the authority.
 */

/** Trimmed display name, or null if invalid (1–50 characters, no control/format characters). */
export function normalizeDisplayName(v: string): string | null {
  const s = v.trim();
  const n = [...s].length;
  if (n === 0 || n > 50) return null;
  // Control characters and invisible formatting characters (incl. line/paragraph separators).
  if (/[\p{Cc}\p{Cf}\u2028\u2029]/u.test(s)) return null;
  return s;
}

/** Lowercase handle without a leading `@`, or null if it doesn't match `[a-z0-9_]{3,20}`. */
export function normalizeHandle(v: string): string | null {
  const s = v.trim().replace(/^@+/, '').toLowerCase();
  return /^[a-z0-9_]{3,20}$/.test(s) ? s : null;
}
