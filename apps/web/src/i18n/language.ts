/**
 * The default language for a first visit: the first of the browser's preferred languages the
 * app has strings for (exact match like `pt-BR`, then its base `pt`), otherwise English.
 * No side effects (doesn't initialise i18next), so storage code can use it.
 */
import { locales } from '../generated/i18n';

export function browserLanguage(
  preferred: readonly string[] = typeof navigator === 'undefined'
    ? []
    : navigator.languages?.length
      ? navigator.languages
      : [navigator.language],
  supported: readonly string[] = locales,
): string {
  for (const tag of preferred) {
    if (!tag) continue;
    const exact = supported.find((l) => l.toLowerCase() === tag.toLowerCase());
    if (exact) return exact;
    const base = tag.split('-')[0]!.toLowerCase();
    const byBase = supported.find((l) => l.toLowerCase() === base);
    if (byBase) return byBase;
  }
  return 'en';
}
