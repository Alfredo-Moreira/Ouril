/**
 * Features that exist in the code but aren't released yet. Each is off unless its build-time
 * flag is `"true"` (see `.env.example`). The MVP ships without them: no sign-in, no Stats page.
 * The code behind them (accounts, sync, stats, replays) stays in place, ready to turn on.
 */
const on = (v: string | undefined) => v === 'true';

export const FEATURES = {
  /** Sign-in, accounts and sync (`/sign-in`, the account link, Settings → Account). */
  accounts: on(import.meta.env.VITE_FEATURE_ACCOUNTS),
  /** The Stats page and its game history and replays (`/stats`, `/replay/:id`). */
  stats: on(import.meta.env.VITE_FEATURE_STATS),
} as const;
