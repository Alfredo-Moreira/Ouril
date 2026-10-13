/**
 * Where to go after signing in, for a page that sent a guest to sign in (an invite link).
 * Kept in sessionStorage, this tab only, and used once.
 */
const KEY = 'ouril.return_to';

export function rememberReturnTo(path: string): void {
  try {
    sessionStorage.setItem(KEY, path);
  } catch {
    // Storage blocked: the player lands on the default page instead.
  }
}

/** The remembered path (only same-site paths), cleared, or `fallback`. */
export function takeReturnTo(fallback: string): string {
  try {
    const path = sessionStorage.getItem(KEY);
    sessionStorage.removeItem(KEY);
    if (path && path.startsWith('/') && !path.startsWith('//')) return path;
  } catch {
    // Storage blocked.
  }
  return fallback;
}
