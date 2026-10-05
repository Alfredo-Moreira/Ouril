/** Web app version and build (from package.json via Vite `define`). */
export const APP_VERSION: string = import.meta.env.VITE_APP_VERSION ?? '0.0.0';
export const APP_BUILD: string = import.meta.env.VITE_APP_BUILD ?? '1';

/** `X-Ouril-Client` header value (docs/architecture/versioning.md). */
export function clientHeader(coreVersion: string): string {
  return `web/${APP_VERSION} (${APP_BUILD}); core/${coreVersion}`;
}

/** Compare dotted numeric versions (`1.10.0` > `1.9.2`). Missing or non-numeric parts are 0. */
export function compareVersions(a: string, b: string): number {
  const parts = (v: string) =>
    v
      .split(/[.+-]/)
      .slice(0, 3)
      .map((p) => Number.parseInt(p, 10) || 0);
  const [x, y] = [parts(a), parts(b)];
  for (let i = 0; i < 3; i++) {
    const d = (x[i] ?? 0) - (y[i] ?? 0);
    if (d !== 0) return Math.sign(d);
  }
  return 0;
}

/**
 * Where this web build stands against `/v1/meta` (versioning.md, "Old apps"): below
 * `min_supported` online features stop until the app updates (offline play is never blocked);
 * below `recommended` a gentle notice is shown.
 */
export type UpdateStatus = 'current' | 'recommended' | 'required';

export function updateStatus(
  meta: { min_supported: { web: string }; recommended: { web: string } },
  version: string = APP_VERSION,
): UpdateStatus {
  if (compareVersions(version, meta.min_supported.web) < 0) return 'required';
  if (compareVersions(version, meta.recommended.web) < 0) return 'recommended';
  return 'current';
}
