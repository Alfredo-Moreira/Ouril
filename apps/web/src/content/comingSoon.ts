/**
 * The home page's "Coming soon" section. One id per upcoming feature, in display order; each
 * needs `home.soon.<id>.title` and `home.soon.<id>.body` in `shared/i18n/en.json`. While the
 * list is empty the section is hidden in production (development shows placeholders).
 */
export const COMING_SOON: readonly string[] = ['accounts', 'leaderboards', 'online'];

/** What's still coming in this build: a feature that is switched on (`features.ts`) is not. */
export function comingSoon(features: { accounts: boolean }): readonly string[] {
  return COMING_SOON.filter((id) => !(id === 'accounts' && features.accounts));
}
