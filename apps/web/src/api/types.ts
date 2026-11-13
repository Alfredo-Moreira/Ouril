/**
 * HTTP API types (apps/server/API.md, v1).
 *
 * Hand-written mirror of the generated types in `src/generated/protocol` (`just protocol-ts`).
 * The app imports these instead of the generated folder so `pnpm build` works on a fresh
 * checkout without the Rust toolchain. Switching to the generated types (once `just
 * protocol-ts` runs before every web build) is flagged for human review.
 * Clients must ignore unknown fields and treat unknown enum values gracefully.
 */
import type { GameRecord } from '../engine/types';

export type ErrorCode =
  | 'bad_request'
  | 'unauthorized'
  | 'forbidden'
  | 'not_found'
  | 'handle_taken'
  | 'payload_too_large'
  | 'upgrade_required'
  | 'rate_limited'
  | 'not_configured'
  | 'provider_unavailable'
  | 'internal';

export interface ApiErrorBody {
  code: ErrorCode | (string & {});
  message: string;
}

export interface TokenPair {
  access_token: string;
  token_type: string;
  expires_in: number;
  refresh_token?: string;
}

export interface Me {
  id: string;
  display_name: string;
  handle: string | null;
  avatar_url: string | null;
  locale: string | null;
  country: string | null;
  created_at: string;
}

/** `GET /v1/meta` (versioning.md, "Old apps"). */
export interface Meta {
  min_supported: { ios: string; android: string; web: string };
  recommended: { ios: string; android: string; web: string };
  api: { current: string; deprecated: string[] };
  realtime_proto: { current: number; min: number };
}

export interface SignInResponse {
  tokens: TokenPair;
  user: Me;
  is_new_user: boolean;
}

/** `POST /v1/auth/google`: the ID token from Google and the raw nonce the app generated. */
export interface GoogleSignInRequest {
  id_token: string;
  nonce: string;
}

export interface DevSignInRequest {
  user?: string;
  display_name?: string;
}

export interface SettingsUpdate {
  sound?: boolean;
  language?: string;
  hints?: boolean;
}

export interface ProfileUpdate {
  display_name?: string;
  avatar_url?: string;
  locale?: string;
  country?: string;
}

export type MutationType =
  'game_finished' | 'profile_updated' | 'settings_updated' | 'handle_requested';

export type Mutation =
  | MutationOf<'game_finished', GameRecord>
  | MutationOf<'settings_updated', SettingsUpdate>
  | MutationOf<'profile_updated', ProfileUpdate>
  | MutationOf<'handle_requested', { handle: string }>;

interface MutationOf<T extends MutationType, P> {
  id: string;
  type: T;
  schema: number;
  payload: P;
  client_time: string;
}

export interface SyncPush {
  mutations: Mutation[];
}

export interface MutationResult {
  id: string;
  /**
   * `applied`; `rejected` (permanent: drop it, see `reason`); `deferred` (this server can't
   * process it yet, e.g. a newer variant version: keep it and push again later). Unknown
   * statuses from a newer server are treated like `deferred`.
   */
  status: 'applied' | 'rejected' | 'deferred' | (string & {});
  reason?: string;
}

export interface SyncPushResult {
  results: MutationResult[];
}

export interface Change {
  entity: 'game' | 'profile' | 'settings' | (string & {});
  id: string;
  change_seq: number;
  deleted: boolean;
  data?: unknown;
}

export interface SyncPull {
  changes: Change[];
  cursor: number;
  has_more: boolean;
}

/** Max mutations per `POST /v1/sync/push`. */
export const MAX_PUSH_BATCH = 100;

// --- Games against other players (ADR 0031) -------------------------------------------------

export type OnlineGameStatus =
  | 'invited'
  | 'active'
  | 'finished'
  | 'declined'
  | 'cancelled'
  | 'expired'
  | 'aborted'
  | (string & {});

export type Seat = 'south' | 'north';

export interface PlayerSummary {
  user_id: string;
  display_name: string;
  handle?: string;
}

export interface OnlineGameResult {
  /** `south_wins` | `north_wins` | `draw`. */
  outcome: string;
  /** A record end reason: `threshold`, …, `resigned`, `timeout`. */
  reason: string;
  stores: [number, number];
}

/** A game against another player, from your point of view (`GET /v1/games/{id}`). */
export interface OnlineGame {
  id: string;
  variant_id: string;
  variant_version: number;
  status: OnlineGameStatus;
  first_player: Seat;
  you: Seat;
  opponent?: PlayerSummary;
  moves: number[];
  your_turn: boolean;
  turn_deadline?: string;
  result?: OnlineGameResult;
  /** The invite link's code (inviter only, while nobody has accepted). */
  invite_code?: string;
  invited_by_you: boolean;
  /** Created as rated (ADR 0034); absent from older servers. */
  rated?: boolean;
  /** Finished and counted: your rating changes, overall pool first. */
  rating_changes?: RatingChange[];
  /** `async` (turn by turn) or `live` (ADR 0033); absent from older servers. */
  mode?: GamePace;
  /** Live games: the clock both players got. */
  time_control?: TimeControl;
  /** Live games: each side's time left when the current turn started. */
  clocks?: Clocks;
  /** Who has an open draw offer (it closes on the next move). */
  draw_offer?: Seat;
  /** The server's time when it answered (RFC 3339), to count down accurately. */
  server_now?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateGameRequest {
  opponent_handle?: string;
  opponent_id?: string;
  variant_id?: string;
  /** Counts for ratings; the server defaults to true. */
  rated?: boolean;
  /** A live game with this clock (one of LIVE_TIME_CONTROLS); absent = turn by turn. */
  time_control?: TimeControl;
}

export interface GameList {
  games: OnlineGame[];
}

export interface OpponentSummary {
  player: PlayerSummary;
  games_played: number;
  last_played_at: string;
}

export interface OpponentList {
  opponents: OpponentSummary[];
}

// --- ratings and leaderboards (ADR 0034) -----------------------------------------------------

/** `overall`, or `<variant_id>:<mode>`. */
export const RATING_POOL_OVERALL = 'overall';
/** Rated games a player needs in a pool before they appear on its boards. */
export const LEADERBOARD_MIN_GAMES = 10;
/** ...and how recently they must have played one. */
export const LEADERBOARD_ACTIVE_DAYS = 60;

export interface RatingChange {
  pool: string;
  before: number;
  after: number;
}

export interface PoolRating {
  pool: string;
  rating: number;
  deviation: number;
  provisional: boolean;
  games: number;
  wins: number;
  draws: number;
  losses: number;
  peak: number;
}

export interface MyRatings {
  ratings: PoolRating[];
}

export type LeaderboardScope = 'world' | 'country';

export interface LeaderboardEntry {
  rank: number;
  player: PlayerSummary;
  country?: string;
  rating: number;
  games: number;
  /** Places gained since `movement_since` (negative: lost); absent = new to the board. */
  change?: number;
}

export interface Leaderboard {
  pool: string;
  scope: LeaderboardScope;
  country?: string;
  entries: LeaderboardEntry[];
  you?: LeaderboardEntry;
  /** The day movement is measured from; absent until the scheduled job has run. */
  movement_since?: string;
  /** Your best rank ever on this board. */
  your_best_rank?: number;
}

// --- live games and the relay (ADR 0033) ------------------------------------------------------

export type GamePace = 'async' | 'live';

export interface TimeControl {
  initial_secs: number;
  increment_secs: number;
}

/** The clocks players can choose: 3+2, 5+3, 10+5. */
export const LIVE_TIME_CONTROLS: readonly TimeControl[] = [
  { initial_secs: 180, increment_secs: 2 },
  { initial_secs: 300, increment_secs: 3 },
  { initial_secs: 600, increment_secs: 5 },
];

/** Each player must make their first move within this, or the game is aborted. */
export const LIVE_FIRST_MOVE_SECS = 30;

export interface Clocks {
  south_ms: number;
  north_ms: number;
  /** When the current turn began; absent once the game is over. */
  turn_started_at?: string;
  /** Whether the side to move is losing time (false until both players have moved). */
  running: boolean;
}

export interface SeekRequest {
  time_control: TimeControl;
  variant_id?: string;
  rated?: boolean;
}

export type SeekState = 'idle' | 'waiting' | 'matched';

export interface SeekStatus {
  state: SeekState;
  game_id?: string;
  players_seeking: number;
}

export interface LiveToken {
  token: string;
  url: string;
  expires_in: number;
}

/** What the relay sends: only IDs and counts; details come from the server. */
export type RelayEvent =
  | { type: 'game_changed'; game_id: string; moves: number }
  | { type: 'matched'; game_id: string }
  | { type: 'games_changed' };

// --- player profiles ---------------------------------------------------------------------

export interface RatingPoint {
  at: string;
  rating: number;
}

export interface RatingHistory {
  pool: string;
  points: RatingPoint[];
}

/** A top placement: `scope` is `world` or a country code. */
export interface Badge {
  pool: string;
  scope: string;
  rank: number;
  achieved_on: string;
}

export interface HeadToHead {
  wins: number;
  draws: number;
  losses: number;
}

export interface PlayerProfile {
  player: PlayerSummary;
  country?: string;
  member_since: string;
  you: boolean;
  ratings: PoolRating[];
  history: RatingHistory[];
  badges: Badge[];
  head_to_head?: HeadToHead;
}

export type DrawAction = 'offer' | 'accept' | 'decline';

/** Someone looking for a live game, as listed in the lobby. */
export interface OpenSeek {
  player: PlayerSummary;
  rating: number;
  variant_id: string;
  time_control: TimeControl;
  rated: boolean;
  waiting_secs: number;
}

export interface LiveLobby {
  seeks: OpenSeek[];
}
