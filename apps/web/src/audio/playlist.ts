/** Random order for the game playlist, never the same track twice in a row. */
export function nextTrack(
  count: number,
  current: number | null,
  random: () => number = Math.random,
): number | null {
  if (count <= 0) return null;
  if (count === 1) return 0;
  // Pick among the others: shift past the current one so it's never repeated.
  const pick = Math.floor(random() * (count - (current === null ? 0 : 1)));
  return current !== null && pick >= current ? pick + 1 : pick;
}
