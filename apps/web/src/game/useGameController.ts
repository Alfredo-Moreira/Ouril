/**
 * Drives one game vs AI: human moves, AI moves (Web Worker), event animation, persistence of
 * the in-progress game, undo, and recording the finished game. No rule logic: legality,
 * results and positions all come from the engine.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { useCore } from '../engine/CoreContext';
import type { GameRecord, MoveEvent, MoveResult } from '../engine/types';
import { useApp } from '../state/useApp';
import { saveCurrentGame, saveFinishedGame, type CurrentGame } from '../storage/repo';
import { trackEvent } from '../telemetry';
import { buildFrames, frameDelay, type Frame } from './animation';
import { forfeitGame } from './forfeit';
import {
  aiSeedFor,
  endReason,
  gameOverEvent,
  isOver,
  outcomeFor,
  playMove,
  toRecord,
  undoLastHumanMove,
} from './session';
import { playEndSound, playFrameSound } from './sound';

export interface Announcement {
  id: number;
  events: MoveEvent[];
  byHuman: boolean;
}

export interface FinishedGame {
  record: GameRecord;
  outcome: 'win' | 'loss' | 'draw';
}

export interface GameController {
  game: CurrentGame;
  frame: Frame | null;
  legalMoves: number[];
  canPlay: boolean;
  aiThinking: boolean;
  animating: boolean;
  canUndo: boolean;
  finished: FinishedGame | null;
  announcement: Announcement | null;
  hintPit: number | null;
  error: string | null;
  play(pit: number): void;
  undo(): void;
  requestHint(): void;
  /** False while a move is being animated or once the game is over. */
  canForfeit: boolean;
  /** End the game now as a loss for the player (recorded and synced like any game). */
  forfeit(): Promise<void>;
}

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

export function useGameController(initial: CurrentGame, animationSpeed: number): GameController {
  const { core, ai } = useCore();
  const { settings, notifyLocalChange } = useApp();
  const [game, setGame] = useState<CurrentGame>(initial);
  const [frame, setFrame] = useState<Frame | null>(null);
  const [animating, setAnimating] = useState(false);
  const [finished, setFinished] = useState<FinishedGame | null>(null);
  const [announcement, setAnnouncement] = useState<Announcement | null>(null);
  const [hintPit, setHintPit] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const alive = useRef(true);
  // Incremented by undo/new moves so stale AI answers are ignored.
  const epoch = useRef(0);
  const soundOn = useRef(settings.sound);
  useEffect(() => {
    soundOn.current = settings.sound;
  }, [settings.sound]);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  const legalMoves = useMemo(() => core.legalMoves(game.variant, game.state), [core, game]);
  const humanTurn = !isOver(game.state) && game.state.to_move === game.humanPlayer;
  // The AI is searching whenever it's its turn and no animation is running.
  const aiThinking = !isOver(game.state) && !humanTurn && !animating && finished === null;
  const canPlay = humanTurn && !animating && finished === null;

  const finish = useCallback(
    async (g: CurrentGame, events: MoveEvent[] | null) => {
      const reason = (events && gameOverEvent(events)?.reason) ?? endReason(core, g) ?? 'no_moves';
      const record = toRecord(core, g, reason);
      await saveFinishedGame(record);
      notifyLocalChange();
      const outcome = outcomeFor(record.result.outcome, g.humanPlayer);
      trackEvent('game_finished', { level: g.level, outcome, plies: g.moves.length });
      if (soundOn.current) playEndSound(outcome === 'win');
      if (alive.current) setFinished({ record, outcome });
    },
    [core, notifyLocalChange],
  );

  const commit = useCallback(
    async (before: CurrentGame, pit: number, byHuman: boolean) => {
      let next: { game: CurrentGame; result: MoveResult };
      try {
        next = playMove(core, before, pit);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
        return;
      }
      epoch.current++;
      setHintPit(null);
      setError(null);
      setAnimating(true);
      // Persist first: a reload mid-animation resumes from the new position.
      await saveCurrentGame(next.game);
      const frames = buildFrames(before.state, pit, next.result.events, next.result.state);
      for (const f of frames) {
        if (!alive.current) return;
        setFrame(f);
        if (soundOn.current) playFrameSound(f.kind);
        const delay = frameDelay(f.kind, animationSpeed);
        if (delay > 0) await sleep(delay);
      }
      if (!alive.current) return;
      setFrame(null);
      setGame(next.game);
      setAnnouncement((a) => ({ id: (a?.id ?? 0) + 1, events: next.result.events, byHuman }));
      setAnimating(false);
      if (isOver(next.game.state)) await finish(next.game, next.result.events);
    },
    [core, animationSpeed, finish],
  );

  // A resumed game that already ended (closed between the last move and saving the record).
  useEffect(() => {
    if (isOver(initial.state)) void finish(initial, null);
    // Only for the initial game.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // AI turn.
  useEffect(() => {
    if (isOver(game.state) || game.state.to_move === game.humanPlayer || animating || finished)
      return;
    const myEpoch = epoch.current;
    const started = Date.now();
    let cancelled = false;
    ai.move(game.variant, game.state, game.level, aiSeedFor(game))
      .then(async (pit) => {
        // A short pause so the AI's move is readable even when the search is instant.
        const wait = Math.max(0, 350 * animationSpeed - (Date.now() - started));
        if (wait > 0) await sleep(wait);
        if (cancelled || !alive.current || epoch.current !== myEpoch || pit === null) return;
        await commit(game, pit, false);
      })
      .catch((e: unknown) => {
        if (cancelled) return;
        setError(e instanceof Error ? e.message : String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [ai, game, animating, finished, commit, animationSpeed]);

  const play = useCallback(
    (pit: number) => {
      if (!canPlay || !legalMoves.includes(pit)) return;
      void commit(game, pit, true);
    },
    [canPlay, legalMoves, commit, game],
  );

  const canUndo = canPlay && undoPossible(game);

  const undo = useCallback(() => {
    if (!canPlay) return;
    const prev = undoLastHumanMove(core, game);
    if (!prev) return;
    epoch.current++;
    setHintPit(null);
    setGame(prev);
    void saveCurrentGame(prev);
  }, [canPlay, core, game]);

  const requestHint = useCallback(() => {
    if (!canPlay) return;
    // Hints reuse the AI search for the player's side (docs/architecture/ai.md, "Later").
    const myEpoch = epoch.current;
    ai.move(game.variant, game.state, 'medium', aiSeedFor(game))
      .then((pit) => {
        if (alive.current && epoch.current === myEpoch) setHintPit(pit);
      })
      .catch(() => {});
  }, [ai, canPlay, game]);

  const canForfeit = finished === null && !animating && !isOver(game.state);

  const forfeit = useCallback(async () => {
    if (!canForfeit) return;
    epoch.current++; // ignore any AI search still running
    setHintPit(null);
    const record = await forfeitGame(core, game);
    notifyLocalChange();
    if (soundOn.current) playEndSound(false);
    if (alive.current) setFinished({ record, outcome: 'loss' });
  }, [canForfeit, core, game, notifyLocalChange]);

  return {
    game,
    frame,
    legalMoves,
    canPlay,
    aiThinking,
    animating,
    canUndo,
    finished,
    announcement,
    hintPit,
    error,
    play,
    undo,
    requestHint,
    canForfeit,
    forfeit,
  };
}

/** The player has made at least one move (the AI may have opened). */
function undoPossible(game: CurrentGame): boolean {
  return game.firstPlayer === game.humanPlayer ? game.moves.length >= 1 : game.moves.length >= 2;
}
