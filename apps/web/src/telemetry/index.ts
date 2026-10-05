/**
 * Telemetry (ADR 0014). Both crash reports and usage statistics are off until the player
 * opts in. Even with consent, this is a **no-op until a Sentry DSN is configured**
 * (`VITE_SENTRY_DSN`), so the MVP build sends nothing.
 *
 * Placeholder: wiring the Sentry browser SDK (lazy-loaded only after consent + DSN) and the
 * usage-stats endpoint is a follow-up (human review). The install ID is random, resettable
 * in Settings, and never linked to the account.
 */
import type { TelemetryConsent } from '../storage/repo';

export interface TelemetryState {
  /** True only when consent was given AND a DSN is configured. */
  crashReportsActive: boolean;
  usageStatsActive: boolean;
}

let state: TelemetryState = { crashReportsActive: false, usageStatsActive: false };

export function telemetryConfigured(): boolean {
  return Boolean(import.meta.env.VITE_SENTRY_DSN);
}

/** Apply the player's choices. Never makes a request when nothing is configured. */
export function configureTelemetry(consent: TelemetryConsent): TelemetryState {
  const configured = telemetryConfigured();
  state = {
    crashReportsActive: configured && consent.decided && consent.crashReports,
    usageStatsActive: configured && consent.decided && consent.usageStats,
  };
  // TODO(telemetry): when crashReportsActive, lazy-import @sentry/browser and init it with the
  // DSN, `sendDefaultPii: false`, and the install ID as the anonymous user ID.
  return state;
}

/** Record an anonymous usage event (e.g. `game_finished`). No-op unless active. */
export function trackEvent(name: string, props: Record<string, string | number> = {}): void {
  if (!state.usageStatsActive) return;
  // TODO(telemetry): queue locally and send when online (capped queue, ADR 0014).
  void name;
  void props;
}

export function getTelemetryState(): TelemetryState {
  return state;
}
