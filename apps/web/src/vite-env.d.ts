/// <reference types="vite/client" />
/// <reference types="vite-plugin-pwa/client" />

interface ImportMetaEnv {
  readonly VITE_APP_VERSION?: string;
  readonly VITE_APP_BUILD?: string;
  /** Sentry DSN for the web app. Empty/absent = telemetry is a no-op (ADR 0014). */
  readonly VITE_SENTRY_DSN?: string;
  /**
   * Origin of the API in production, e.g. `https://api.ouril.example` (same site as the web
   * app, ADR 0020). Empty/absent = same origin (the Vite dev proxy, or a `/v1` reverse proxy).
   */
  readonly VITE_API_BASE_URL?: string;
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
