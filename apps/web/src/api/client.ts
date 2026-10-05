/**
 * HTTP client for apps/server (API.md). Only ever used after the player signs in:
 * guests make zero requests (ADR 0014, data-and-sync.md).
 *
 * - The access token lives in memory only. The refresh token is an httpOnly cookie
 *   (`ouril_refresh`, Path=/v1/auth) that JS never sees.
 * - A 401 on an authenticated call triggers one refresh + retry.
 * - Refresh is serialised across tabs with the Web Locks API: two tabs posting the same
 *   refresh cookie at once would look like token reuse to the server and revoke the whole
 *   session (backend.md, session.rs `rotate_refresh_token`). The tab that waits for the lock
 *   sends the already-rotated cookie and succeeds. Without `navigator.locks`, refresh is only
 *   de-duplicated within the tab.
 * - In dev, Vite proxies `/v1` to the server, so requests are same-origin. In production the
 *   API may live on a same-site subdomain (`VITE_API_BASE_URL`, ADR 0020); requests are then
 *   cross-origin with `credentials: 'include'` so the refresh cookie is sent (the server
 *   allows the web origin via CORS with credentials).
 */
import type {
  ApiErrorBody,
  DevSignInRequest,
  Me,
  Meta,
  SignInResponse,
  SyncPull,
  SyncPush,
  SyncPushResult,
  TokenPair,
} from './types';

export class ApiError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.code = code;
  }
}

/**
 * The request never reached the server: offline, DNS, CORS, or a proxy or gateway answering
 * because the server is down or restarting (see `toApiError`).
 */
export class NetworkError extends Error {
  constructor(cause: unknown) {
    super(cause instanceof Error ? cause.message : 'network_error');
    this.name = 'NetworkError';
  }
}

export interface ApiClientOptions {
  /** API origin, e.g. `https://api.ouril.example`. Defaults to '' (same origin). */
  baseUrl?: string;
  fetch?: typeof fetch;
  /** Value for `X-Ouril-Client`. */
  clientHeader: () => string;
  /**
   * Cross-tab lock manager. Defaults to `navigator.locks` when present; pass `null` to
   * disable (refresh then only de-duplicates within this client).
   */
  locks?: Pick<LockManager, 'request'> | null;
}

/** Web Locks name shared by every tab of this origin. */
export const REFRESH_LOCK_NAME = 'ouril-refresh';

function defaultLocks(): Pick<LockManager, 'request'> | null {
  if (typeof navigator === 'undefined') return null;
  return (navigator as Navigator & { locks?: LockManager }).locks ?? null;
}

type Method = 'GET' | 'POST' | 'PATCH' | 'DELETE';

export class ApiClient {
  private accessToken: string | null = null;
  private refreshing: Promise<void> | null = null;
  private readonly baseUrl: string;
  private readonly fetchImpl: typeof fetch;
  private readonly clientHeader: () => string;
  private readonly locks: Pick<LockManager, 'request'> | null;

  constructor(options: ApiClientOptions) {
    this.baseUrl = (options.baseUrl ?? '').replace(/\/+$/, '');
    this.fetchImpl = options.fetch ?? ((...args) => globalThis.fetch(...args));
    this.clientHeader = options.clientHeader;
    this.locks = options.locks === undefined ? defaultLocks() : options.locks;
  }

  get hasAccessToken(): boolean {
    return this.accessToken !== null;
  }

  clearTokens(): void {
    this.accessToken = null;
  }

  // --- auth -----------------------------------------------------------------------------------

  /** Debug builds only (server feature `dev-auth`). 404 elsewhere. */
  async devSignIn(request: DevSignInRequest = {}): Promise<SignInResponse> {
    if (!import.meta.env.DEV)
      throw new ApiError(404, 'not_found', 'dev sign-in is not in production builds');
    const res = await this.request<SignInResponse>('POST', '/v1/auth/dev', request, {
      auth: false,
    });
    this.accessToken = res.tokens.access_token;
    return res;
  }

  /** Rotates the refresh cookie and stores a new access token. Throws ApiError(401) if expired. */
  refresh(): Promise<void> {
    this.refreshing ??= this.refreshUnderLock()
      .then((pair) => {
        this.accessToken = pair.access_token;
      })
      .finally(() => {
        this.refreshing = null;
      });
    return this.refreshing;
  }

  /**
   * Holds the cross-tab lock for the whole round trip, so one tab's rotated Set-Cookie
   * lands before the next tab's request reads the cookie jar.
   */
  private refreshUnderLock(): Promise<TokenPair> {
    const post = () => this.request<TokenPair>('POST', '/v1/auth/refresh', {}, { auth: false });
    if (!this.locks) return post();
    return this.locks.request(REFRESH_LOCK_NAME, post) as Promise<TokenPair>;
  }

  async logout(): Promise<void> {
    try {
      await this.request<void>('POST', '/v1/auth/logout', {}, { auth: false });
    } finally {
      this.accessToken = null;
    }
  }

  /** App version requirements. Unauthenticated; only called while signed in (never as guest). */
  meta(): Promise<Meta> {
    return this.request<Meta>('GET', '/v1/meta', undefined, { auth: false });
  }

  // --- profile --------------------------------------------------------------------------------

  me(): Promise<Me> {
    return this.request<Me>('GET', '/v1/me');
  }

  async deleteMe(): Promise<void> {
    await this.request<void>('DELETE', '/v1/me');
    this.accessToken = null;
  }

  // --- sync -----------------------------------------------------------------------------------

  syncPush(body: SyncPush): Promise<SyncPushResult> {
    return this.request<SyncPushResult>('POST', '/v1/sync/push', body);
  }

  syncPull(cursor: number, limit = 200): Promise<SyncPull> {
    return this.request<SyncPull>('GET', `/v1/sync/pull?cursor=${cursor}&limit=${limit}`);
  }

  // --- core -----------------------------------------------------------------------------------

  private async request<T>(
    method: Method,
    path: string,
    body?: unknown,
    { auth = true, retried = false }: { auth?: boolean; retried?: boolean } = {},
  ): Promise<T> {
    if (auth && !this.accessToken) await this.refresh();

    const headers: Record<string, string> = {
      Accept: 'application/json',
      'X-Ouril-Client': this.clientHeader(),
    };
    if (body !== undefined) headers['Content-Type'] = 'application/json';
    if (auth && this.accessToken) headers.Authorization = `Bearer ${this.accessToken}`;

    let res: Response;
    try {
      res = await this.fetchImpl(`${this.baseUrl}${path}`, {
        method,
        headers,
        body: body === undefined ? undefined : JSON.stringify(body),
        credentials: this.baseUrl ? 'include' : 'same-origin',
      });
    } catch (e) {
      throw new NetworkError(e);
    }

    if (res.status === 401 && auth && !retried) {
      this.accessToken = null;
      await this.refresh();
      return this.request<T>(method, path, body, { auth, retried: true });
    }
    if (!res.ok) throw await toApiError(res);
    if (res.status === 204) return undefined as T;
    const text = await res.text();
    return (text ? JSON.parse(text) : undefined) as T;
  }
}

/** Gateway statuses a proxy or load balancer sends when it can't reach the server. */
const UNREACHABLE_STATUSES = new Set([500, 502, 503, 504]);

/**
 * Our server always answers an error with a JSON body carrying a `code`. A 500/502/503/504
 * without one came from a proxy or gateway (the dev proxy while the server restarts, a load
 * balancer while it's down): the server was never reached, so it's a `NetworkError`, shown as
 * "Could not reach the server" and treated as offline by sync. Any other error without a code
 * stays `internal`.
 */
async function toApiError(res: Response): Promise<ApiError | NetworkError> {
  let body: Partial<ApiErrorBody> = {};
  try {
    body = (await res.json()) as Partial<ApiErrorBody>;
  } catch {
    // Not JSON: not our server's error format.
  }
  if (!body.code && UNREACHABLE_STATUSES.has(res.status)) {
    return new NetworkError(new Error(`server_unreachable_${res.status}`));
  }
  return new ApiError(res.status, body.code ?? 'internal', body.message ?? res.statusText);
}
