/**
 * The live-game relay (ADR 0033), for signed-in screens only: guests never reach it.
 *
 * - Asks the server for a token (`POST /v1/live/token`), opens the relay's WebSocket and sends
 *   the token as the first message (never in the URL).
 * - Events carry only IDs and counts; the screen refetches from the server when one arrives.
 * - Heartbeat every 25 s (Cloudflare closes WebSockets idle for 100 s). An expired token
 *   (close 4001) reconnects at once with a new one; other drops back off up to 30 s.
 * - No relay configured (501), or an older server: gives up quietly. Screens keep polling, so
 *   the relay only ever makes things faster.
 */
import { useEffect, useRef, useState } from 'react';

import { ApiError } from '../api/client';
import type { RelayEvent } from '../api/types';
import { useApp } from '../state/useApp';

const HEARTBEAT_MS = 25_000;
const MAX_BACKOFF_MS = 30_000;
const CLOSE_TOKEN_EXPIRED = 4001;

/** Connect while `enabled`; `gameId` adds that game's channel. Returns whether it's live. */
export function useRelay(
  gameId: string | null,
  onEvent: (event: RelayEvent) => void,
  enabled = true,
): boolean {
  const { api } = useApp();
  const [connected, setConnected] = useState(false);
  const handler = useRef(onEvent);
  useEffect(() => {
    handler.current = onEvent;
  }, [onEvent]);

  useEffect(() => {
    if (!enabled || typeof WebSocket === 'undefined') return;
    let stopped = false;
    let socket: WebSocket | null = null;
    let heartbeat: ReturnType<typeof setInterval> | undefined;
    let retry: ReturnType<typeof setTimeout> | undefined;
    let backoff = 1_000;

    const again = (delay: number) => {
      if (stopped) return;
      retry = setTimeout(() => void connect(), delay);
    };

    const connect = async () => {
      let token;
      try {
        token = await api().liveToken(gameId ?? undefined);
      } catch (e) {
        // A network blip: try again later. No relay, not allowed, or an older server: stop.
        if (e instanceof ApiError && e.status >= 500 && e.status !== 501) again(backoff);
        return;
      }
      if (stopped) return;
      const ws = new WebSocket(token.url);
      socket = ws;
      ws.onopen = () => ws.send(JSON.stringify({ token: token.token }));
      ws.onmessage = (m: MessageEvent) => {
        let msg: { type?: string };
        try {
          msg = JSON.parse(String(m.data)) as { type?: string };
        } catch {
          return;
        }
        if (msg.type === 'ready') {
          backoff = 1_000;
          setConnected(true);
          heartbeat = setInterval(() => ws.send(JSON.stringify({ type: 'ping' })), HEARTBEAT_MS);
        } else if (msg.type !== 'pong') {
          handler.current(msg as RelayEvent);
        }
      };
      ws.onclose = (ev: CloseEvent) => {
        clearInterval(heartbeat);
        setConnected(false);
        if (socket === ws) socket = null;
        if (ev.code === CLOSE_TOKEN_EXPIRED) {
          again(0);
        } else {
          again(backoff);
          backoff = Math.min(backoff * 2, MAX_BACKOFF_MS);
        }
      };
    };

    void connect();
    return () => {
      stopped = true;
      clearTimeout(retry);
      clearInterval(heartbeat);
      socket?.close(1000);
    };
  }, [api, gameId, enabled]);

  return enabled && connected;
}
