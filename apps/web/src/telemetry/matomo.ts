/**
 * Usage statistics through self-hosted Matomo (ADR 0026), used only after the player opts in
 * (ADR 0014). This is a small client for Matomo's HTTP tracking API, not `matomo.js`: no
 * third-party script, no cookies, no referrer, no screen size. Each hit carries the site ID,
 * the page path (IDs stripped), the event, and a visitor ID derived from the random install ID.
 *
 * Hits are queued in localStorage (capped, oldest dropped) and sent in bulk when online, so
 * gameplay never waits on the network. Hits older than Matomo's 24-hour backdating limit are
 * dropped rather than sent.
 */

export interface MatomoConfig {
  /** `<base>/matomo.php`. */
  endpoint: string;
  siteId: string;
}

type Params = Record<string, string>;
interface QueuedHit {
  params: Params;
  /** When it happened (ms since epoch); sent as `cdt` so delayed hits keep their time. */
  at: number;
}

const QUEUE_KEY = 'ouril.telemetry.queue';
export const MAX_QUEUE = 200;
/** Matomo only accepts `cdt` up to 24 hours back without an auth token; keep a margin. */
export const MAX_AGE_MS = 23 * 60 * 60 * 1000;
const FLUSH_DELAY_MS = 2000;

/** The configured Matomo instance, or null when the build has none (then nothing is sent). */
export function matomoConfig(): MatomoConfig | null {
  const base = import.meta.env.VITE_MATOMO_URL?.trim();
  const siteId = import.meta.env.VITE_MATOMO_SITE_ID?.trim();
  if (!base || !siteId) return null;
  try {
    return { endpoint: new URL('matomo.php', base.endsWith('/') ? base : `${base}/`).href, siteId };
  } catch {
    return null;
  }
}

/** Matomo's `_id`: 16 hex characters, from the random part of the install ID (a UUIDv7). */
export function visitorId(installId: string): string {
  return installId
    .replace(/[^0-9a-f]/gi, '')
    .slice(-16)
    .toLowerCase()
    .padStart(16, '0');
}

/** Routes with a parameter: the parameter is replaced by its name. */
const ROUTE_PARAMS: readonly [RegExp, string][] = [
  [/^\/replay\/[^/]+/, '/replay/:id'],
  [/^\/online\/[^/]+/, '/online/:id'],
  // An invite code would let anyone join the game: it must never leave the device.
  [/^\/join\/[^/]+/, '/join/:code'],
];

/**
 * A path segment that looks like an ID or a secret: a UUID, or 16+ base64url characters with
 * at least one digit (so long route names made of words aren't mistaken for one).
 */
const ID_LIKE =
  /^(?:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}|(?=[^/]*\d)[A-Za-z0-9_-]{16,})$/i;

/**
 * The route without anything that identifies a game, a player or an invitation, and without
 * the query string or fragment (`/online/<id>` → `/online/:id`, `/join/<code>` → `/join/:code`).
 * Any other segment that looks like an ID is replaced too, so a new route can't leak one.
 */
export function sanitizePath(path: string): string {
  const clean = path.split(/[?#]/)[0] || '/';
  for (const [pattern, name] of ROUTE_PARAMS) {
    if (pattern.test(clean)) return clean.replace(pattern, name);
  }
  return clean
    .split('/')
    .map((segment) => (ID_LIKE.test(segment) ? ':id' : segment))
    .join('/');
}

let active: { config: MatomoConfig; visitor: string } | null = null;
let timer: ReturnType<typeof setTimeout> | null = null;
let inFlight = false;
let listening = false;

function readQueue(): QueuedHit[] {
  try {
    const raw = localStorage.getItem(QUEUE_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? (parsed as QueuedHit[]) : [];
  } catch {
    return [];
  }
}

function writeQueue(queue: QueuedHit[]): void {
  try {
    if (queue.length === 0) localStorage.removeItem(QUEUE_KEY);
    else localStorage.setItem(QUEUE_KEY, JSON.stringify(queue));
  } catch {
    // Storage full or blocked: drop the hits rather than affect the game.
  }
}

function onOnline() {
  void flush();
}
function onHidden() {
  if (document.visibilityState === 'hidden') void flush();
}

/** Start (with the player's install ID) or stop. Stopping discards anything not yet sent. */
export function setMatomo(config: MatomoConfig | null, installId: string): void {
  if (!config) {
    active = null;
    if (timer) clearTimeout(timer);
    timer = null;
    writeQueue([]);
    if (listening) {
      window.removeEventListener('online', onOnline);
      document.removeEventListener('visibilitychange', onHidden);
      listening = false;
    }
    return;
  }
  active = { config, visitor: visitorId(installId) };
  if (!listening) {
    window.addEventListener('online', onOnline);
    document.addEventListener('visibilitychange', onHidden);
    listening = true;
  }
  if (readQueue().length > 0) schedule();
}

function enqueue(params: Params): void {
  if (!active) return;
  const queue = readQueue();
  queue.push({ params, at: Date.now() });
  writeQueue(queue.slice(-MAX_QUEUE));
  schedule();
}

function schedule(): void {
  if (timer) return;
  timer = setTimeout(() => {
    timer = null;
    void flush();
  }, FLUSH_DELAY_MS);
}

function basePath(path: string): Params {
  return { url: `${window.location.origin}${sanitizePath(path)}` };
}

export function matomoPageView(path: string): void {
  enqueue(basePath(path));
}

/**
 * An event: category = the event name, action = its text props (`level:easy outcome:loss`),
 * value = its one numeric prop, if exactly one. Sorted so the same event always reads the same.
 */
export function matomoEvent(
  name: string,
  props: Record<string, string | number>,
  path = window.location.pathname,
): void {
  const entries = Object.entries(props).sort(([a], [b]) => a.localeCompare(b));
  const text = entries.filter(([, v]) => typeof v === 'string').map(([k, v]) => `${k}:${v}`);
  const numbers = entries.filter(([, v]) => typeof v === 'number');
  const params: Params = { ...basePath(path), e_c: name, e_a: text.join(' ') || name };
  if (numbers.length === 1) {
    const [k, v] = numbers[0]!;
    params.e_n = k;
    params.e_v = String(v);
  }
  enqueue(params);
}

/** Send what's queued. Hits stay queued if offline or the request fails. */
export async function flush(now = Date.now()): Promise<void> {
  if (!active || inFlight) return;
  if (typeof navigator !== 'undefined' && navigator.onLine === false) return;
  const fresh = readQueue().filter((h) => now - h.at < MAX_AGE_MS);
  writeQueue(fresh);
  if (fresh.length === 0) return;
  const { config, visitor } = active;
  const requests = fresh.map((h) => {
    const q = new URLSearchParams({
      idsite: config.siteId,
      rec: '1',
      apiv: '1',
      _id: visitor,
      cdt: String(Math.floor(h.at / 1000)),
      ...h.params,
    });
    return `?${q.toString()}`;
  });
  inFlight = true;
  try {
    const res = await fetch(config.endpoint, {
      method: 'POST',
      // A "simple" content type, as matomo.js uses: no CORS preflight. Matomo reads the JSON body.
      headers: { 'Content-Type': 'application/x-www-form-urlencoded; charset=UTF-8' },
      body: JSON.stringify({ requests }),
      credentials: 'omit',
      referrerPolicy: 'no-referrer',
      keepalive: true,
    });
    if (res.ok) {
      // Remove only what was sent; hits queued meanwhile stay.
      const sent = new Set(fresh.map((h) => h.at + JSON.stringify(h.params)));
      writeQueue(readQueue().filter((h) => !sent.has(h.at + JSON.stringify(h.params))));
    }
  } catch {
    // Offline or blocked: try again later.
  } finally {
    inFlight = false;
  }
}

/** Test seam. */
export function queuedHits(): readonly QueuedHit[] {
  return readQueue();
}
