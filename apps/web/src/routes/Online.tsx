import { useCallback, useEffect, useState, type FormEvent, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate } from 'react-router';

import { ApiError } from '../api/client';
import {
  LIVE_TIME_CONTROLS,
  type OnlineGame,
  type OpenSeek,
  type OpponentSummary,
  type RelayEvent,
  type TimeControl,
} from '../api/types';
import { trackInvite, trackInviteAnswer } from '../online/analytics';
import { inviteLink } from '../online/links';
import { resultFor } from '../online/perspective';
import { usePolling } from '../online/usePolling';
import { useLiveSeek } from '../online/liveSeek';
import { useRelay } from '../online/useRelay';
import { useApp } from '../state/useApp';

/** How often the games list refreshes while it's open. */
const LIST_POLL_MS = 10_000;
const tcKey = (tc: TimeControl) => `${tc.initial_secs}+${tc.increment_secs}`;

/** `5 + 3`, translated. */
function useClockLabel() {
  const { t } = useTranslation();
  return (tc: TimeControl) =>
    t('online.time_control', { minutes: tc.initial_secs / 60, increment: tc.increment_secs });
}

/**
 * The Online tab of the Game page (`/game?tab=online`, signed-in players, ADR 0031): invite
 * someone by handle, make an invite link, challenge a past opponent, and see your games:
 * invitations for you, games waiting for your move or theirs, invitations you sent, and
 * recent finished games.
 */
export function OnlineLobby() {
  const { t } = useTranslation();
  const { api } = useApp();
  const navigate = useNavigate();
  const [games, setGames] = useState<OnlineGame[] | null>(null);
  const [opponents, setOpponents] = useState<OpponentSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // New invitations are rated unless the player turns it off (ADR 0034).
  const [rated, setRated] = useState(true);
  // Turn by turn (`async`), or a live clock's key (ADR 0033).
  const [pace, setPace] = useState('async');
  const timeControl = LIVE_TIME_CONTROLS.find((tc) => tcKey(tc) === pace);
  const clockLabel = useClockLabel();

  const refresh = useCallback(() => {
    void api()
      .listGames()
      .then((list) => {
        setGames(list.games);
        setError(null);
      })
      .catch((e: unknown) => setError(e instanceof ApiError ? e.code : 'network'));
  }, [api]);

  useEffect(() => {
    refresh();
    void api()
      .opponents()
      .then((list) => setOpponents(list.opponents))
      .catch(() => undefined);
  }, [api, refresh]);
  usePolling(refresh, LIST_POLL_MS);
  // Invitations and answers show up at once through the relay, when it's there.
  useRelay(
    null,
    useCallback(
      (e: RelayEvent) => {
        if (e.type === 'games_changed') refresh();
      },
      [refresh],
    ),
  );

  /** Run an action; on success track it (`onDone`) and open the game or refresh the list. */
  const run = async (action: () => Promise<OnlineGame>, onDone?: () => void, open = false) => {
    setBusy(true);
    setError(null);
    try {
      const game = await action();
      onDone?.();
      if (open) navigate(`/online/${game.id}`);
      else refresh();
    } catch (e) {
      setError(e instanceof ApiError ? e.code : 'network');
    } finally {
      setBusy(false);
    }
  };

  const invitations = games?.filter((g) => g.status === 'invited' && !g.invited_by_you) ?? [];
  const yourTurn = games?.filter((g) => g.status === 'active' && g.your_turn) ?? [];
  const theirTurn = games?.filter((g) => g.status === 'active' && !g.your_turn) ?? [];
  const sent = games?.filter((g) => g.status === 'invited' && g.invited_by_you) ?? [];
  const finished = games?.filter((g) => !['invited', 'active'].includes(g.status)) ?? [];

  return (
    <div className="online">
      <p className="note">{t('online.intro')}</p>

      <div className="card">
        <label className="toggle">
          <input type="checkbox" checked={rated} onChange={(e) => setRated(e.target.checked)} />
          <span>{t('online.rated')}</span>
        </label>
        <p className="note">{t(rated ? 'online.rated.on' : 'online.rated.off')}</p>
        <label className="field">
          <span>{t('online.pace')}</span>
          <select value={pace} onChange={(e) => setPace(e.target.value)}>
            <option value="async">{t('online.pace.async')}</option>
            {LIVE_TIME_CONTROLS.map((tc) => (
              <option key={tcKey(tc)} value={tcKey(tc)}>
                {t('online.pace.live', { clock: clockLabel(tc) })}
              </option>
            ))}
          </select>
        </label>
      </div>

      <PlayNow rated={rated} />

      <InviteForm
        busy={busy}
        onInvite={(handle) =>
          run(
            () => api().createGame({ opponent_handle: handle, rated, time_control: timeControl }),
            () => trackInvite('handle'),
          )
        }
      />
      <div className="row">
        <button
          type="button"
          className="button"
          disabled={busy}
          onClick={() =>
            void run(
              () => api().createGame({ rated, time_control: timeControl }),
              () => trackInvite('link'),
            )
          }
        >
          {t('online.invite_link.create')}
        </button>
      </div>
      {error && (
        <p className="error" role="alert">
          {t(`online.error.${error}`, {
            defaultValue: t(`error.${error}`, { defaultValue: t('error.internal') }),
          })}
        </p>
      )}

      {games === null ? (
        <p role="status">{t('common.loading')}</p>
      ) : (
        <>
          <GameSection title={t('online.section.invitations')} games={invitations}>
            {(g) => (
              <>
                <button
                  type="button"
                  className="button"
                  disabled={busy}
                  onClick={() =>
                    void run(
                      () => api().gameAction(g.id, 'accept'),
                      () => trackInviteAnswer('accept'),
                      true,
                    )
                  }
                >
                  {t('online.accept')}
                </button>
                <button
                  type="button"
                  className="button button--ghost"
                  disabled={busy}
                  onClick={() =>
                    void run(
                      () => api().gameAction(g.id, 'decline'),
                      () => trackInviteAnswer('decline'),
                    )
                  }
                >
                  {t('online.decline')}
                </button>
              </>
            )}
          </GameSection>
          <GameSection title={t('online.section.your_turn')} games={yourTurn}>
            {(g) => (
              <Link className="button" to={`/online/${g.id}`}>
                {t('online.play')}
              </Link>
            )}
          </GameSection>
          <GameSection title={t('online.section.their_turn')} games={theirTurn}>
            {(g) => (
              <Link className="button button--ghost" to={`/online/${g.id}`}>
                {t('online.view')}
              </Link>
            )}
          </GameSection>
          <GameSection title={t('online.section.sent')} games={sent}>
            {(g) => (
              <>
                {g.invite_code && <CopyLink code={g.invite_code} />}
                <button
                  type="button"
                  className="button button--ghost"
                  disabled={busy}
                  onClick={() =>
                    void run(
                      () => api().gameAction(g.id, 'cancel'),
                      () => trackInviteAnswer('cancel'),
                    )
                  }
                >
                  {t('online.cancel')}
                </button>
              </>
            )}
          </GameSection>
          <GameSection title={t('online.section.finished')} games={finished}>
            {(g) =>
              g.status === 'finished' ? (
                <Link className="button button--ghost" to={`/online/${g.id}`}>
                  {t('online.view')}
                </Link>
              ) : null
            }
          </GameSection>
          {games.length === 0 && <p>{t('online.empty')}</p>}
        </>
      )}

      {opponents.length > 0 && (
        <section aria-labelledby="online-opponents">
          <h2 id="online-opponents">{t('online.section.opponents')}</h2>
          <ul className="online__list">
            {opponents.map((o) => (
              <li key={o.player.user_id} className="online__item">
                <span>
                  <Link to={`/players/${o.player.user_id}`}>
                    <strong>{o.player.display_name}</strong>
                  </Link>
                  {o.player.handle && <span className="note"> @{o.player.handle}</span>}
                </span>
                <span className="note">
                  {t('online.opponent.games', { count: o.games_played })}
                </span>
                <button
                  type="button"
                  className="button"
                  disabled={busy}
                  onClick={() =>
                    void run(
                      () =>
                        api().createGame({
                          opponent_id: o.player.user_id,
                          rated,
                          time_control: timeControl,
                        }),
                      () => trackInvite('opponent'),
                    )
                  }
                >
                  {t('online.challenge')}
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

/**
 * Play now (ADR 0033): pick a clock and be paired with someone near your rating, or join
 * someone already waiting. The search itself lives in `LiveSeekProvider`, so it keeps going
 * while you play the computer.
 */
function PlayNow({ rated }: { rated: boolean }) {
  const { t } = useTranslation();
  const seek = useLiveSeek();
  const clockLabel = useClockLabel();

  return (
    <section className="card play-now" aria-labelledby="play-now-title">
      <h2 id="play-now-title" className="card__title">
        {t('online.play_now')}
      </h2>
      {seek.seeking ? (
        <>
          <p role="status">
            {t('online.play_now.looking', { clock: clockLabel(seek.seeking.timeControl) })}{' '}
            <span className="note">{t('online.play_now.players', { count: seek.players })}</span>
          </p>
          <div className="row">
            <button type="button" className="button" onClick={seek.cancel}>
              {t('common.cancel')}
            </button>
            <Link className="button button--ghost" to="/play">
              {t('online.play_now.computer')}
            </Link>
          </div>
        </>
      ) : (
        <>
          <p className="note">{t('online.play_now.intro')}</p>
          <div className="row">
            {LIVE_TIME_CONTROLS.map((tc) => (
              <button
                key={tcKey(tc)}
                type="button"
                className="button"
                onClick={() => seek.start(tc, rated)}
              >
                {clockLabel(tc)}
              </button>
            ))}
          </div>
        </>
      )}
      {seek.error && (
        <p className="error" role="alert">
          {t(`online.error.${seek.error}`, {
            defaultValue: t(`error.${seek.error}`, { defaultValue: t('error.internal') }),
          })}
        </p>
      )}
      <OpenChallenges />
    </section>
  );
}

/** How often the open-challenges list refreshes. */
const LOBBY_POLL_MS = 5_000;

/** Players waiting for a live game right now, each with a Join button. */
function OpenChallenges() {
  const { t } = useTranslation();
  const { api } = useApp();
  const seek = useLiveSeek();
  const clockLabel = useClockLabel();
  const [seeks, setSeeks] = useState<OpenSeek[]>([]);
  const refresh = useCallback(() => {
    void api()
      .liveLobby()
      .then((l) => setSeeks(l.seeks))
      .catch(() => undefined);
  }, [api]);
  useEffect(refresh, [refresh]);
  usePolling(refresh, LOBBY_POLL_MS);
  if (seeks.length === 0) return null;
  return (
    <section aria-labelledby="open-challenges">
      <h3 id="open-challenges">{t('online.open_challenges')}</h3>
      <ul className="online__list">
        {seeks.map((s) => (
          <li key={s.player.user_id} className="online__item">
            <span>
              <Link to={`/players/${s.player.user_id}`}>
                <strong>{s.player.display_name}</strong>
              </Link>
              <span className="note">
                {' '}
                ({s.rating}) · {clockLabel(s.time_control)} · {t(`variant.${s.variant_id}.short`)}
                {' · '}
                {t(s.rated ? 'online.rated.short' : 'online.casual')}
              </span>
            </span>
            <button
              type="button"
              className="button"
              onClick={() => void seek.join(s.player.user_id)}
            >
              {t('online.join_seek')}
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}

function InviteForm({
  busy,
  onInvite,
}: {
  busy: boolean;
  onInvite: (handle: string) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [handle, setHandle] = useState('');
  const submit = (e: FormEvent) => {
    e.preventDefault();
    const h = handle.trim().replace(/^@/, '');
    if (!h) return;
    void onInvite(h).then(() => setHandle(''));
  };
  return (
    <form className="card online__invite" onSubmit={submit}>
      <label className="field">
        <span>{t('online.invite.label')}</span>
        <input
          value={handle}
          onChange={(e) => setHandle(e.target.value)}
          placeholder={t('online.invite.placeholder')}
          autoComplete="off"
          autoCapitalize="none"
          spellCheck={false}
          maxLength={21}
        />
      </label>
      <button type="submit" className="button" disabled={busy || !handle.trim()}>
        {t('online.invite.send')}
      </button>
    </form>
  );
}

function CopyLink({ code }: { code: string }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const link = inviteLink(code);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(link);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard blocked: the link is shown and selectable.
    }
  };
  return (
    <span className="online__link">
      <code>{link}</code>
      <button type="button" className="button button--ghost" onClick={() => void copy()}>
        {copied ? t('online.invite_link.copied') : t('online.invite_link.copy')}
      </button>
    </span>
  );
}

function GameSection({
  title,
  games,
  children,
}: {
  title: string;
  games: OnlineGame[];
  children: (game: OnlineGame) => ReactNode;
}) {
  if (games.length === 0) return null;
  const id = `online-${title.replace(/\W+/g, '-').toLowerCase()}`;
  return (
    <section aria-labelledby={id}>
      <h2 id={id}>{title}</h2>
      <ul className="online__list">
        {games.map((g) => (
          <li key={g.id} className="online__item">
            <GameSummary game={g} />
            <span className="row">{children(g)}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}

function GameSummary({ game }: { game: OnlineGame }) {
  const { t } = useTranslation();
  const name = game.opponent?.display_name ?? t('online.invite_link.anyone');
  let detail: string;
  if (game.status === 'finished' && game.result) {
    detail = t(`stats.result.${resultFor(game.result.outcome, game.you)}`);
  } else if (game.status === 'active') {
    detail = t('online.moves', { count: game.moves.length });
  } else {
    detail = t(`online.status.${game.status}`, { defaultValue: '' });
  }
  return (
    <span>
      {game.opponent ? (
        <Link to={`/players/${game.opponent.user_id}`}>
          <strong>{name}</strong>
        </Link>
      ) : (
        <strong>{name}</strong>
      )}
      {game.opponent?.handle && <span className="note"> @{game.opponent.handle}</span>}
      {detail && <span className="note"> · {detail}</span>}
      {game.rated === false && <span className="note"> · {t('online.casual')}</span>}
      {game.mode === 'live' && game.time_control && (
        <span className="note">
          {' · '}
          {t('online.pace.live', {
            clock: t('online.time_control', {
              minutes: game.time_control.initial_secs / 60,
              increment: game.time_control.increment_secs,
            }),
          })}
        </span>
      )}
    </span>
  );
}
