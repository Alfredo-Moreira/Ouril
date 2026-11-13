import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { useLocation, useNavigate } from 'react-router';

import { ApiError } from '../api/client';
import type { RelayEvent, SeekStatus, TimeControl } from '../api/types';
import { useApp } from '../state/useApp';
import { LiveSeekContext, useLiveSeek, type LiveSeek } from './liveSeek';
import { usePolling } from './usePolling';
import { useRelay } from './useRelay';

/** The heartbeat that keeps a seek alive (and pairs it). */
const HEARTBEAT_MS = 3_000;

/** The Game page, where the search shows itself and a match opens straight away. */
const onGamePage = (path: string) => path === '/game';

/**
 * Keeps a Play now search going across screens: heartbeats every 3 s, listens to the relay for
 * "matched", and stops the search when cancelled or when the app closes. A match found on the
 * Game page opens at once; anywhere else (a game against the computer, say) it waits in
 * `found` for the player to open it.
 */
export function LiveSeekProvider({ children }: { children: ReactNode }) {
  const { api } = useApp();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const [seeking, setSeeking] = useState<LiveSeek['seeking']>(null);
  const [players, setPlayers] = useState(0);
  const [found, setFound] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const matched = useCallback(
    (gameId: string) => {
      setSeeking(null);
      if (onGamePage(pathname)) navigate(`/online/${gameId}`);
      else setFound(gameId);
    },
    [navigate, pathname],
  );
  const handle = useCallback(
    (st: SeekStatus) => {
      setPlayers(st.players_seeking);
      if (st.state === 'matched' && st.game_id) matched(st.game_id);
    },
    [matched],
  );
  const send = useCallback(() => {
    if (!seeking) return;
    void api()
      .seek({ time_control: seeking.timeControl, rated: seeking.rated })
      .then(handle)
      .catch((e: unknown) => {
        setSeeking(null);
        setError(e instanceof ApiError ? e.code : 'network');
      });
  }, [api, seeking, handle]);

  useEffect(send, [send]);
  usePolling(send, seeking ? HEARTBEAT_MS : null);
  useRelay(
    null,
    useCallback(
      (e: RelayEvent) => {
        if (e.type === 'matched') matched(e.game_id);
      },
      [matched],
    ),
    seeking !== null,
  );
  // Stopping (cancelled, matched, or the app closing) forgets the seek on the server.
  useEffect(() => {
    if (!seeking) return;
    return () => {
      void api()
        .cancelSeek()
        .catch(() => undefined);
    };
  }, [api, seeking]);

  const value = useMemo<LiveSeek>(
    () => ({
      seeking,
      players,
      found,
      error,
      start(timeControl: TimeControl, rated: boolean) {
        setError(null);
        setFound(null);
        setSeeking({ timeControl, rated });
      },
      cancel: () => setSeeking(null),
      async join(userId: string) {
        setError(null);
        try {
          const st = await api().joinSeek(userId);
          setSeeking(null);
          if (st.game_id) navigate(`/online/${st.game_id}`);
        } catch (e) {
          setError(e instanceof ApiError ? e.code : 'network');
        }
      },
      openFound() {
        if (found) navigate(`/online/${found}`);
        setFound(null);
      },
      dismissFound: () => setFound(null),
    }),
    [seeking, players, found, error, api, navigate],
  );

  return <LiveSeekContext.Provider value={value}>{children}</LiveSeekContext.Provider>;
}

/**
 * A small notice outside the Game page: still looking (with Cancel), or an opponent found
 * (with Open game). It sits over everything, including a game against the computer.
 */
export function SeekToast() {
  const { t } = useTranslation();
  const { pathname } = useLocation();
  const seek = useLiveSeek();
  if (seek.found) {
    return (
      <div className="seek-toast" role="status">
        <span>{t('online.play_now.found')}</span>
        <button type="button" className="button button--primary" onClick={seek.openFound}>
          {t('online.play_now.open')}
        </button>
        <button type="button" className="button button--ghost" onClick={seek.dismissFound}>
          {t('online.play_now.dismiss')}
        </button>
      </div>
    );
  }
  if (!seek.seeking || onGamePage(pathname)) return null;
  return (
    <div className="seek-toast" role="status">
      <span>
        {t('online.play_now.looking', {
          clock: t('online.time_control', {
            minutes: seek.seeking.timeControl.initial_secs / 60,
            increment: seek.seeking.timeControl.increment_secs,
          }),
        })}
      </span>
      <button type="button" className="button button--ghost" onClick={seek.cancel}>
        {t('common.cancel')}
      </button>
    </div>
  );
}
