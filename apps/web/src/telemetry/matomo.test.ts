import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  flush,
  MAX_AGE_MS,
  MAX_QUEUE,
  matomoConfig,
  matomoEvent,
  matomoPageView,
  queuedHits,
  sanitizePath,
  setMatomo,
  visitorId,
} from './matomo';

const config = { endpoint: 'https://analytics.invalid/matomo.php', siteId: '1' };
const installId = '0192b3c4-d5e6-7f80-9a1b-2c3d4e5f6a7b';

function sentRequests(fetchMock: ReturnType<typeof vi.fn>): URLSearchParams[] {
  return fetchMock.mock.calls.flatMap(([, init]) =>
    (JSON.parse((init as RequestInit).body as string) as { requests: string[] }).requests.map(
      (r) => new URLSearchParams(r.slice(1)),
    ),
  );
}

describe('Matomo usage statistics (ADR 0026)', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn(() => Promise.resolve(new Response(null, { status: 204 })));
    vi.stubGlobal('fetch', fetchMock);
    setMatomo(config, installId);
  });
  afterEach(() => {
    setMatomo(null, '');
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
  });

  it('reads the endpoint from the build variables', () => {
    vi.stubEnv('VITE_MATOMO_URL', 'https://analytics.example.org');
    vi.stubEnv('VITE_MATOMO_SITE_ID', '7');
    expect(matomoConfig()).toEqual({
      endpoint: 'https://analytics.example.org/matomo.php',
      siteId: '7',
    });
    vi.stubEnv('VITE_MATOMO_URL', '');
    expect(matomoConfig()).toBeNull();
  });

  it('derives a 16-hex visitor ID from the random part of the install ID', () => {
    expect(visitorId(installId)).toBe('9a1b2c3d4e5f6a7b');
  });

  it('strips IDs, queries and fragments from paths', () => {
    expect(sanitizePath('/replay/0192b3c4?x=1#y')).toBe('/replay/:id');
    expect(sanitizePath('/rules?from=home')).toBe('/rules');
  });

  it('sends page views and events in one bulk request, with no cookies or referrer', async () => {
    matomoPageView('/play');
    matomoEvent('game_finished', { outcome: 'win', level: 'easy', plies: 42 }, '/play');
    await flush();

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0]! as [string, RequestInit];
    expect(url).toBe(config.endpoint);
    expect(init).toMatchObject({
      method: 'POST',
      credentials: 'omit',
      referrerPolicy: 'no-referrer',
    });
    const [view, event] = sentRequests(fetchMock);
    expect(view!.get('idsite')).toBe('1');
    expect(view!.get('rec')).toBe('1');
    expect(view!.get('_id')).toBe('9a1b2c3d4e5f6a7b');
    expect(view!.get('url')).toBe(`${window.location.origin}/play`);
    expect(view!.has('e_c')).toBe(false);
    expect(event!.get('e_c')).toBe('game_finished');
    expect(event!.get('e_a')).toBe('level:easy outcome:win');
    expect(event!.get('e_n')).toBe('plies');
    expect(event!.get('e_v')).toBe('42');
    expect(queuedHits()).toHaveLength(0);
  });

  it('keeps hits queued while offline or when the request fails', async () => {
    matomoPageView('/');
    vi.spyOn(navigator, 'onLine', 'get').mockReturnValueOnce(false);
    await flush();
    expect(fetchMock).not.toHaveBeenCalled();

    fetchMock.mockImplementationOnce(() => Promise.reject(new Error('offline')));
    await flush();
    expect(queuedHits()).toHaveLength(1);

    await flush();
    expect(queuedHits()).toHaveLength(0);
  });

  it('caps the queue and drops hits too old for Matomo to backdate', async () => {
    for (let i = 0; i < MAX_QUEUE + 10; i++) matomoPageView(`/p${i}`);
    expect(queuedHits()).toHaveLength(MAX_QUEUE);
    expect(queuedHits()[0]!.params.url).toMatch(/\/p10$/);

    await flush(Date.now() + MAX_AGE_MS + 1);
    expect(fetchMock).not.toHaveBeenCalled();
    expect(queuedHits()).toHaveLength(0);
  });

  it('stopping (consent withdrawn) discards the queue and records nothing more', async () => {
    matomoPageView('/');
    setMatomo(null, '');
    expect(queuedHits()).toHaveLength(0);
    matomoPageView('/rules');
    await flush();
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
