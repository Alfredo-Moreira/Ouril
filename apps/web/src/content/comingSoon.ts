/**
 * The home page's "Coming soon" section: what's next on the roadmap (docs/product/roadmap.md),
 * one id per upcoming feature, in display order. Each needs `home.soon.<id>.title` and
 * `home.soon.<id>.body` in `shared/i18n/en.json`. Take an item off this list in the same change
 * that ships it. While the list is empty the section is hidden in production (development shows
 * placeholders).
 */
export const COMING_SOON: readonly string[] = ['apps', 'pass_play', 'languages'];

/** What's still coming in this build. */
export function comingSoon(): readonly string[] {
  return COMING_SOON;
}
