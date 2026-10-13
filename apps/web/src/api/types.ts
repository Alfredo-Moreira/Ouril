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
  'invited' | 'active' | 'finished' | 'declined' | 'cancelled' | 'expired' | (string & {});

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
  created_at: string;
  updated_at: string;
}

export interface CreateGameRequest {
  opponent_handle?: string;
  opponent_id?: string;
  variant_id?: string;
  /** Counts for ratings; the server defaults to true. */
  rated?: boolean;
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
}

export interface Leaderboard {
  pool: string;
  scope: LeaderboardScope;
  country?: string;
  entries: LeaderboardEntry[];
  you?: LeaderboardEntry;
}
