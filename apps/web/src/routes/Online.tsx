import { useCallback, useEffect, useState, type FormEvent, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate } from 'react-router';

import { ApiError } from '../api/client';
import type { OnlineGame, OpponentSummary } from '../api/types';
import { trackInvite, trackInviteAnswer } from '../online/analytics';
import { inviteLink } from '../online/links';
import { resultFor } from '../online/perspective';
import { usePolling } from '../online/usePolling';
import { useApp } from '../state/useApp';

/** How often the games list refreshes while it's open. */
const LIST_POLL_MS = 10_000;

/**
 * `/online` (signed-in players, ADR 0031): invite someone by handle, make an invite link,
 * challenge a past opponent, and see your games: invitations for you, games waiting for your
 * move or theirs, invitations you sent, and recent finished games.
 */
export function Online() {
  const { t } = useTranslation();
  const { api } = useApp();
  const navigate = useNavigate();
  const [games, setGames] = useState<OnlineGame[] | null>(null);
  const [opponents, setOpponents] = useState<OpponentSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

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
    <section className="online">
      <h1>{t('online.title')}</h1>
      <p className="note">{t('online.intro')}</p>

      <InviteForm
        busy={busy}
        onInvite={(handle) =>
          run(
            () => api().createGame({ opponent_handle: handle }),
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
              () => api().createGame({}),
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
                  <strong>{o.player.display_name}</strong>
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
                      () => api().createGame({ opponent_id: o.player.user_id }),
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
      <strong>{name}</strong>
      {game.opponent?.handle && <span className="note"> @{game.opponent.handle}</span>}
      {detail && <span className="note"> · {detail}</span>}
    </span>
  );
}
