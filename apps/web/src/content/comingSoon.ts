/**
 * The home page's "Coming soon" section. One id per upcoming feature, in display order; each
 * needs `home.soon.<id>.title` and `home.soon.<id>.body` in `shared/i18n/en.json`. While the
 * list is empty the section is hidden in production (development shows placeholders).
 */
export const COMING_SOON: readonly string[] = ['accounts', 'leaderboards', 'online'];
