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
