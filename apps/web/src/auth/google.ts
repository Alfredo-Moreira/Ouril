/**
 * Sign in with Google on the web, as an OpenID Connect redirect (accounts.md).
 *
 * Tapping the button navigates to Google's authorization endpoint with `response_type=id_token`
 * and a fresh `state` and `nonce`. Google sends the player back to {@link GOOGLE_CALLBACK_PATH}
 * with the ID token in the URL fragment; the callback route checks `state` and posts the token
 * and nonce to `POST /v1/auth/google`, where the server verifies it (API.md).
 *
 * Why a redirect and not the Google Identity Services script: nothing loads until the player
 * taps the button, so the sign-in screen makes no network request for guests (ADR 0014), and
 * the CSP needs no third-party script, frame or connect source.
 */

export const GOOGLE_AUTH_ENDPOINT = 'https://accounts.google.com/o/oauth2/v2/auth';
/** Where Google sends the player back. Register `<origin>/auth/google` in Google Cloud. */
export const GOOGLE_CALLBACK_PATH = '/auth/google';
/** sessionStorage key for the pending sign-in (survives the redirect, this tab only). */
export const GOOGLE_PENDING_KEY = 'ouril.google_sign_in';
/** A callback for a sign-in started longer ago than this is refused. */
export const GOOGLE_PENDING_MAX_AGE_MS = 10 * 60 * 1000;

interface Pending {
  state: string;
  nonce: string;
  createdAt: number;
}

export type GoogleCallback =
  { kind: 'ok'; idToken: string; nonce: string } | { kind: 'cancelled' } | { kind: 'invalid' };

type Storage = Pick<globalThis.Storage, 'getItem' | 'setItem' | 'removeItem'>;

/** The OAuth client ID (`VITE_GOOGLE_CLIENT_ID`), or null while Google sign-in isn't set up. */
export function googleClientId(): string | null {
  const id = import.meta.env.VITE_GOOGLE_CLIENT_ID?.trim();
  return id ? id : null;
}

/** 32 random bytes, base64url. */
function randomToken(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  let s = '';
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

/**
 * Remember a new `state` and `nonce` and return Google's sign-in URL. The caller navigates to
 * it (`window.location.assign`).
 */
export function googleSignInUrl(options: {
  clientId: string;
  origin: string;
  storage?: Storage;
  now?: number;
}): string {
  const storage = options.storage ?? window.sessionStorage;
  const pending: Pending = {
    state: randomToken(),
    nonce: randomToken(),
    createdAt: options.now ?? Date.now(),
  };
  storage.setItem(GOOGLE_PENDING_KEY, JSON.stringify(pending));
  const params = new URLSearchParams({
    client_id: options.clientId,
    response_type: 'id_token',
    redirect_uri: options.origin + GOOGLE_CALLBACK_PATH,
    scope: 'openid email profile',
    state: pending.state,
    nonce: pending.nonce,
    prompt: 'select_account',
  });
  return `${GOOGLE_AUTH_ENDPOINT}?${params.toString()}`;
}

/** Page navigation, separate so tests can replace it (jsdom can't navigate). */
export const browser = {
  assign(url: string): void {
    window.location.assign(url);
  },
};

/** Start Google sign-in in this tab: remember `state` and `nonce`, then go to Google. */
export function startGoogleSignIn(clientId: string): void {
  browser.assign(googleSignInUrl({ clientId, origin: window.location.origin }));
}

/**
 * Read Google's answer from the URL fragment. The pending sign-in is used up either way, so a
 * fragment can't be replayed in this tab.
 */
export function readGoogleCallback(
  hash: string,
  options: { storage?: Storage; now?: number } = {},
): GoogleCallback {
  const storage = options.storage ?? window.sessionStorage;
  let pending: Pending | null;
  try {
    const raw = storage.getItem(GOOGLE_PENDING_KEY);
    storage.removeItem(GOOGLE_PENDING_KEY);
    pending = raw ? (JSON.parse(raw) as Pending) : null;
  } catch {
    pending = null;
  }
  const params = new URLSearchParams(hash.replace(/^#/, ''));
  const error = params.get('error');
  if (error) return error === 'access_denied' ? { kind: 'cancelled' } : { kind: 'invalid' };

  const idToken = params.get('id_token');
  const now = options.now ?? Date.now();
  if (
    !pending ||
    !idToken ||
    typeof pending.state !== 'string' ||
    typeof pending.nonce !== 'string' ||
    params.get('state') !== pending.state ||
    !(now - pending.createdAt <= GOOGLE_PENDING_MAX_AGE_MS)
  ) {
    return { kind: 'invalid' };
  }
  return { kind: 'ok', idToken, nonce: pending.nonce };
}

let taken: { hash: string; result: GoogleCallback } | null = null;

/**
 * {@link readGoogleCallback} for the page's own fragment, remembered per fragment: reading it
 * again (React renders twice in development) returns the same answer instead of "invalid".
 */
export function takeGoogleCallback(hash: string): GoogleCallback {
  if (taken?.hash !== hash) taken = { hash, result: readGoogleCallback(hash) };
  return taken.result;
}
