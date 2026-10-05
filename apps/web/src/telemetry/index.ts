/**
 * Telemetry (ADR 0014). Both crash reports and usage statistics are off until the player
 * opts in, and each is a no-op unless the build configures its service:
 *
 * - **Usage statistics:** self-hosted Matomo (ADR 0026), `VITE_MATOMO_URL` + `VITE_MATOMO_SITE_ID`.
 * - **Crash reports:** Sentry, `VITE_SENTRY_DSN`. Placeholder: wiring the Sentry browser SDK
 *   (lazy-loaded only after consent + DSN) is a follow-up (human review).
 *
 * The install ID is random, resettable in Settings, and never linked to the account.
 */
import type { TelemetryConsent } from '../storage/repo';
import { matomoConfig, matomoEvent, matomoPageView, setMatomo } from './matomo';

export interface TelemetryState {
  /** True only when consent was given AND the service is configured. */
  crashReportsActive: boolean;
  usageStatsActive: boolean;
}

let state: TelemetryState = { crashReportsActive: false, usageStatsActive: false };

/** A crash-report service (Sentry) is configured in this build. */
export function crashReportsConfigured(): boolean {
  return Boolean(import.meta.env.VITE_SENTRY_DSN);
}

/** A usage-statistics service (Matomo) is configured in this build. */
export function usageStatsConfigured(): boolean {
  return matomoConfig() !== null;
}

/** Any telemetry service is configured; otherwise there's nothing to ask consent for. */
export function telemetryConfigured(): boolean {
  return crashReportsConfigured() || usageStatsConfigured();
}

/** Apply the player's choices. Never makes a request when nothing is configured or allowed. */
export function configureTelemetry(consent: TelemetryConsent): TelemetryState {
  state = {
    crashReportsActive: crashReportsConfigured() && consent.decided && consent.crashReports,
    usageStatsActive: usageStatsConfigured() && consent.decided && consent.usageStats,
  };
  // TODO(telemetry): when crashReportsActive, lazy-import @sentry/browser and init it with the
  // DSN, `sendDefaultPii: false`, and the install ID as the anonymous user ID.
  setMatomo(state.usageStatsActive ? matomoConfig() : null, consent.installId);
  return state;
}

/** Record an anonymous usage event (e.g. `game_finished`). No-op unless active. */
export function trackEvent(name: string, props: Record<string, string | number> = {}): void {
  if (!state.usageStatsActive) return;
  matomoEvent(name, props);
}

/** Record a screen view (the route only; IDs are stripped). No-op unless active. */
export function trackPageView(path: string): void {
  if (!state.usageStatsActive) return;
  matomoPageView(path);
}

export function getTelemetryState(): TelemetryState {
  return state;
}
