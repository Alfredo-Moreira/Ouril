/// <reference types="vite/client" />
/// <reference types="vite-plugin-pwa/client" />

interface ImportMetaEnv {
  readonly VITE_APP_VERSION?: string;
  readonly VITE_APP_BUILD?: string;
  /** Sentry DSN for the web app. Empty/absent = telemetry is a no-op (ADR 0014). */
  readonly VITE_SENTRY_DSN?: string;
  /**
   * Base URL of the self-hosted Matomo for usage statistics, e.g. `https://analytics.example/`
   * (ADR 0026). Empty/absent, or no site ID = usage statistics are a no-op.
   */
  readonly VITE_MATOMO_URL?: string;
  /** Matomo site ID for this app. */
  readonly VITE_MATOMO_SITE_ID?: string;
  /**
   * Origin of the API, only if it isn't same-origin. Empty/absent = same origin: the Vite dev
   * proxy locally, the `/v1` proxy in vercel.json in production (ADR 0029).
   */
  readonly VITE_API_BASE_URL?: string;
  /**
   * Google OAuth client ID (public). Empty/absent = the Google button stays a disabled
   * placeholder. Register `<origin>/auth/google` as a redirect URI for it (`src/auth/google.ts`).
   */
  readonly VITE_GOOGLE_CLIENT_ID?: string;
  /** `"true"` turns on sign-in, accounts and sync (off for the MVP; `src/features.ts`). */
  readonly VITE_FEATURE_ACCOUNTS?: string;
  /** `"true"` turns on the Stats page (off for the MVP; `src/features.ts`). */
  readonly VITE_FEATURE_STATS?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

/** URLs of the game music files in public/audio/game/ (vite.config.ts `musicTracks`). */
declare module 'virtual:music-tracks' {
  const tracks: readonly string[];
  export default tracks;
}
