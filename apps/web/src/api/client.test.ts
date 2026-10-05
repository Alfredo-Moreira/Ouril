import { describe, expect, it } from 'vitest';
import { ApiClient, ApiError, NetworkError, REFRESH_LOCK_NAME } from './client';

/** In-process stand-in for `navigator.locks`: exclusive, FIFO, shared by every "tab". */
function fakeLocks() {
  const tails = new Map<string, Promise<unknown>>();
  const requested: string[] = [];
  return {
    requested,
    request(name: string, cb: () => unknown): Promise<unknown> {
      requested.push(name);
      const prev = tails.get(name) ?? Promise.resolve();
      const result = prev.then(() => cb());
      tails.set(
        name,
        result.catch(() => undefined),
      );
      return result;
    },
  } as unknown as Pick<LockManager, 'request'> & { requested: string[] };
}

/**
 * Simulates the server's refresh rotation over one shared cookie jar: presenting a refresh
 * token that was already rotated out is treated as reuse and revokes the session.
 */
function fakeServer() {
  const state = { cookie: 'r0', generation: 0, revoked: false, refreshCalls: 0 };
  const fetchImpl = async (input: RequestInfo | URL): Promise<Response> => {
    const url = String(input);
    if (!url.endsWith('/v1/auth/refresh')) return new Response(null, { status: 404 });
    state.refreshCalls += 1;
    const presented = state.cookie; // the browser attaches the jar's cookie at send time
    await new Promise((r) => setTimeout(r, 5)); // network latency
    if (state.revoked || presented !== `r${state.generation}`) {
      state.revoked = true;
      return Response.json({ code: 'unauthorized', message: 'reuse' }, { status: 401 });
    }
    state.generation += 1;
    state.cookie = `r${state.generation}`;
    return Response.json({ access_token: `a${state.generation}`, expires_in: 900 });
  };
  return { state, fetchImpl: fetchImpl as typeof fetch };
}

describe('ApiClient.refresh', () => {
  it('serialises refresh across tabs with a shared lock, so no false reuse revokes the session', async () => {
    const server = fakeServer();
    const locks = fakeLocks();
    const tab1 = new ApiClient({ fetch: server.fetchImpl, clientHeader: () => 'web/test', locks });
    const tab2 = new ApiClient({ fetch: server.fetchImpl, clientHeader: () => 'web/test', locks });

    await Promise.all([tab1.refresh(), tab2.refresh()]);

    expect(server.state.revoked).toBe(false);
    expect(server.state.refreshCalls).toBe(2);
    expect(tab1.hasAccessToken).toBe(true);
    expect(tab2.hasAccessToken).toBe(true);
    expect(locks.requested).toEqual([REFRESH_LOCK_NAME, REFRESH_LOCK_NAME]);
  });

  it('without a lock manager, concurrent tabs trip reuse detection (documents the fallback)', async () => {
    const server = fakeServer();
    const tab1 = new ApiClient({
      fetch: server.fetchImpl,
      clientHeader: () => 'web/test',
      locks: null,
    });
    const tab2 = new ApiClient({
      fetch: server.fetchImpl,
      clientHeader: () => 'web/test',
      locks: null,
    });

    const results = await Promise.allSettled([tab1.refresh(), tab2.refresh()]);

    expect(results.some((r) => r.status === 'rejected')).toBe(true);
    expect(server.state.revoked).toBe(true);
  });

  it('still de-duplicates concurrent refreshes within one tab', async () => {
    const server = fakeServer();
    const locks = fakeLocks();
    const tab = new ApiClient({ fetch: server.fetchImpl, clientHeader: () => 'web/test', locks });

    await Promise.all([tab.refresh(), tab.refresh(), tab.refresh()]);

    expect(server.state.refreshCalls).toBe(1);
    expect(locks.requested).toHaveLength(1);
  });

  it('releases the lock when refresh fails so the next attempt can run', async () => {
    const server = fakeServer();
    server.state.revoked = true;
    const locks = fakeLocks();
    const tab = new ApiClient({ fetch: server.fetchImpl, clientHeader: () => 'web/test', locks });

    await expect(tab.refresh()).rejects.toBeInstanceOf(ApiError);
    server.state.revoked = false;
    server.state.cookie = `r${server.state.generation}`;
    await expect(tab.refresh()).resolves.toBeUndefined();
    expect(tab.hasAccessToken).toBe(true);
  });
});

describe('API origin', () => {
  function capture() {
    const calls: { url: string; credentials?: RequestCredentials }[] = [];
    const fetchImpl = async (input: RequestInfo | URL, init?: RequestInit) => {
      calls.push({ url: String(input), credentials: init?.credentials });
      return new Response(JSON.stringify({ code: 'not_found', message: '' }), {
        status: 404,
        headers: { 'Content-Type': 'application/json' },
      });
    };
    return { calls, fetchImpl: fetchImpl as typeof fetch };
  }

  it('is same-origin by default (dev proxy)', async () => {
    const { calls, fetchImpl } = capture();
    const api = new ApiClient({ fetch: fetchImpl, clientHeader: () => 'web/test', locks: null });
    await api.devSignIn({}).catch(() => undefined);
    expect(calls[0]).toEqual({ url: '/v1/auth/dev', credentials: 'same-origin' });
  });

  it('sends the refresh cookie to a same-site API origin', async () => {
    const { calls, fetchImpl } = capture();
    const api = new ApiClient({
      baseUrl: 'https://api.ouril.example/',
      fetch: fetchImpl,
      clientHeader: () => 'web/test',
      locks: null,
    });
    await api.devSignIn({}).catch(() => undefined);
    expect(calls[0]).toEqual({
      url: 'https://api.ouril.example/v1/auth/dev',
      credentials: 'include',
    });
  });
});

describe('server unreachable', () => {
  const api = (res: () => Response) =>
    new ApiClient({
      fetch: (async () => res()) as typeof fetch,
      clientHeader: () => 'web/test',
      locks: null,
    });

  it('a gateway error without our JSON body is a network error (server down or restarting)', async () => {
    for (const status of [500, 502, 503, 504]) {
      await expect(
        api(() => new Response('<html>Bad gateway</html>', { status })).devSignIn({}),
      ).rejects.toBeInstanceOf(NetworkError);
      await expect(api(() => new Response(null, { status })).devSignIn({})).rejects.toBeInstanceOf(
        NetworkError,
      );
    }
  });

  it("the server's own errors keep their code", async () => {
    const err = await api(() =>
      Response.json({ code: 'internal', message: 'boom' }, { status: 500 }),
    )
      .devSignIn({})
      .catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).code).toBe('internal');
  });

  it('other errors without a body stay internal', async () => {
    const err = await api(() => new Response('nope', { status: 418 }))
      .devSignIn({})
      .catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).code).toBe('internal');
  });
});
