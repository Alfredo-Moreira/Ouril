//! `ouril-protocol`: request/response types for the HTTP API (`/v1`), shared by the server
//! and every client. The human-readable contract is `apps/server/API.md`.
//!
//! Compatibility rules (`docs/architecture/versioning.md`): within `/v1` only additive changes.
//! Clients ignore unknown fields and must handle unknown enum values (error codes, mutation
//! types, change entities) gracefully.
//!
//! TypeScript: `just protocol-ts` exports every type with `#[ts(export)]` to
//! `apps/web/src/generated/protocol/` (build output, never committed or hand-edited).

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "ts")]
use ts_rs::TS;

/// Current major API version (URL prefix).
pub const API_VERSION: &str = "v1";
/// Realtime protocol version (Phase 4; published in `/v1/meta` from the MVP).
pub const REALTIME_PROTO_CURRENT: u32 = 1;
pub const REALTIME_PROTO_MIN: u32 = 1;
/// Header every signed-in client request carries: `ios/1.4.0 (52); core/0.7.2`.
pub const CLIENT_HEADER: &str = "X-Ouril-Client";
/// Web only: name of the httpOnly refresh-token cookie (`Path=/v1/auth`, `SameSite=Lax`).
pub const REFRESH_COOKIE: &str = "ouril_refresh";

// ---------------------------------------------------------------------------------------
// GET /v1/meta
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct PlatformVersions {
    pub ios: String,
    pub android: String,
    pub web: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct ApiVersions {
    pub current: String,
    pub deprecated: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct RealtimeProto {
    pub current: u32,
    pub min: u32,
}

/// `GET /v1/meta` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Meta {
    pub min_supported: PlatformVersions,
    pub recommended: PlatformVersions,
    pub api: ApiVersions,
    pub realtime_proto: RealtimeProto,
}

// ---------------------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------------------

/// `POST /v1/auth/dev` (debug builds with the `dev-auth` feature only).
/// Signs in as a seeded test user; `user` picks which one (default `"dev"`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct DevSignInRequest {
    /// Stable key of the seeded test user, `[a-z0-9_-]{1,32}`. Default `"dev"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub user: Option<String>,
    /// Display name used only when the test user is created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub display_name: Option<String>,
}

/// `POST /v1/auth/google`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct GoogleSignInRequest {
    pub id_token: String,
    pub nonce: String,
}

/// `POST /v1/auth/apple`. Apple sends the name only on the first sign-in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct AppleSignInRequest {
    pub id_token: String,
    pub nonce: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub given_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub family_name: Option<String>,
    /// The authorization code from the same Apple sign-in. The server exchanges it for Apple's
    /// refresh token so it can revoke the token when the account is deleted (ADR 0009).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub authorization_code: Option<String>,
}

/// `POST /v1/auth/refresh` and `POST /v1/auth/logout`. Native apps send the refresh token in
/// the body; the web sends `{}` and relies on the `ouril_refresh` cookie.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct RefreshRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub refresh_token: Option<String>,
}

/// Session tokens. `refresh_token` is present only for native clients; web clients get it
/// as an httpOnly cookie instead (never readable from JS).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct TokenPair {
    /// Short-lived JWT (default 900 s). Send as `Authorization: Bearer <token>`.
    pub access_token: String,
    /// Always `"Bearer"`.
    pub token_type: String,
    /// Seconds until `access_token` expires.
    pub expires_in: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub refresh_token: Option<String>,
}

/// Response of every sign-in endpoint (`/v1/auth/{dev,google,apple}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct SignInResponse {
    pub tokens: TokenPair,
    pub user: Me,
    /// True when this sign-in created the account (the app may prompt for a handle).
    pub is_new_user: bool,
}

// ---------------------------------------------------------------------------------------
// Profile: GET / PATCH / DELETE /v1/me
// ---------------------------------------------------------------------------------------

/// The signed-in user's profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Me {
    /// UUID.
    pub id: String,
    pub display_name: String,
    pub handle: Option<String>,
    pub avatar_url: Option<String>,
    /// BCP 47, e.g. `en`, `pt`, `kea`.
    pub locale: Option<String>,
    /// ISO 3166-1 alpha-2, lowercase.
    pub country: Option<String>,
    /// RFC 3339.
    pub created_at: String,
}

/// `PATCH /v1/me`. Absent fields are unchanged. (Clearing a field is not supported in v1.)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct ProfilePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub avatar_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub locale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub country: Option<String>,
}

// ---------------------------------------------------------------------------------------
// Sync: POST /v1/sync/push, GET /v1/sync/pull
// ---------------------------------------------------------------------------------------

/// Known mutation types. Mutation `type` is a string so new types are additive.
pub mod mutation_types {
    /// Payload: `ouril_sync::GameRecord` (schema 1). Append-only, union by game ID.
    pub const GAME_FINISHED: &str = "game_finished";
    /// Payload: `ouril_sync::ProfileUpdate` (schema 1). Field-level last-writer-wins by
    /// `client_time`.
    pub const PROFILE_UPDATED: &str = "profile_updated";
    /// Payload: `ouril_sync::SettingsUpdate` (schema 1). Field-level last-writer-wins by
    /// `client_time`.
    pub const SETTINGS_UPDATED: &str = "settings_updated";
    /// Payload: `ouril_sync::HandleRequest` (schema 1). Applied only if the handle is free.
    pub const HANDLE_REQUESTED: &str = "handle_requested";
}

/// Machine-readable `MutationResult.reason` values for `rejected` (permanent). String-valued
/// so new reasons are additive; clients treat unknown reasons like `invalid_payload`.
pub mod rejection_reasons {
    /// Payload doesn't match the type's schema, or a field fails validation.
    pub const INVALID_PAYLOAD: &str = "invalid_payload";
    /// `game_finished`: replaying the moves fails, or the stored result doesn't match.
    pub const ILLEGAL_GAME: &str = "illegal_game";
    /// `handle_requested`: someone else has the handle.
    pub const HANDLE_TAKEN: &str = "handle_taken";
}

/// Machine-readable `MutationResult.reason` values for `deferred`: the mutation may be valid,
/// but this server can't process it yet (typically a client newer than the server, e.g. during
/// a rolling deploy). Nothing is recorded; the client keeps it and pushes it again later.
pub mod deferral_reasons {
    /// `type` isn't known to this server.
    pub const UNKNOWN_TYPE: &str = "unknown_type";
    /// `schema` isn't supported for this `type`.
    pub const UNSUPPORTED_SCHEMA: &str = "unsupported_schema";
    /// `game_finished`: the record's `variant` `id@version` isn't known to this server.
    pub const UNSUPPORTED_VARIANT: &str = "unsupported_variant";
    /// `game_finished`: the record's `format` is newer than this server supports.
    pub const UNSUPPORTED_RECORD_FORMAT: &str = "unsupported_record_format";
}

/// Most mutations accepted in one `POST /v1/sync/push` (413 `payload_too_large` above).
pub const MAX_MUTATIONS_PER_PUSH: usize = 100;
/// Default and maximum page size of `GET /v1/sync/pull`.
pub const PULL_DEFAULT_LIMIT: u32 = 200;
pub const PULL_MAX_LIMIT: u32 = 500;

/// One queued local change from the device outbox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Mutation {
    /// Client-generated UUID (v7). Repeats are ignored, so retries are always safe.
    pub id: String,
    /// One of [`mutation_types`].
    #[serde(rename = "type")]
    pub kind: String,
    /// Payload schema version for this type (`game_finished@1` → 1).
    pub schema: u16,
    pub payload: Value,
    /// Device time when the change was made (RFC 3339). Orders `settings_updated`,
    /// `profile_updated` and `handle_requested` fields (clamped to the server's clock).
    pub client_time: String,
}

/// `POST /v1/sync/push` body. At most 100 mutations per request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct SyncPush {
    pub mutations: Vec<Mutation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum MutationStatus {
    /// Applied now, or already applied earlier (repeat).
    Applied,
    /// Not applied, permanently; the client rolls it back locally. See `reason`
    /// ([`rejection_reasons`]). Repeating the mutation ID returns the same rejection.
    Rejected,
    /// Not applied yet: this server can't process it (see `reason`, [`deferral_reasons`]).
    /// Nothing is recorded; the client keeps the mutation and pushes it again later.
    Deferred,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct MutationResult {
    pub id: String,
    pub status: MutationStatus,
    /// Machine-readable reason when rejected ([`rejection_reasons`]) or deferred
    /// ([`deferral_reasons`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reason: Option<String>,
}

/// `POST /v1/sync/push` response: one result per mutation, in request order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct SyncPushResult {
    pub results: Vec<MutationResult>,
}

/// `GET /v1/sync/pull` query string.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct SyncPullQuery {
    /// Last cursor received; omit or 0 for a full pull.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "number"))]
    pub cursor: Option<u64>,
    /// Page size, 1..=500 (default 200).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub limit: Option<u32>,
}

/// Kind of synced row. String-valued so new kinds are additive.
pub mod change_entities {
    pub const GAME: &str = "game";
    pub const PROFILE: &str = "profile";
    pub const SETTINGS: &str = "settings";
}

/// One changed row since the cursor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Change {
    /// One of [`change_entities`].
    pub entity: String,
    /// Row ID (game UUID; the user ID for `profile` and `settings`).
    pub id: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub change_seq: u64,
    /// Deletion marker: the client deletes the row locally.
    pub deleted: bool,
    /// Full current row (absent when `deleted`): a `GameRecord`, `Me`, or settings object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub data: Option<Value>,
}

/// `GET /v1/sync/pull` response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct SyncPull {
    pub changes: Vec<Change>,
    /// Store this and send it as `cursor` next time.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub cursor: u64,
    /// More changes are waiting: pull again immediately with the new cursor.
    pub has_more: bool,
}

// ---------------------------------------------------------------------------------------
// Games against other players (ADR 0031)
// ---------------------------------------------------------------------------------------

/// Longest a player has to make a move in an active game, before they lose on time.
pub const TURN_DEADLINE_DAYS: i64 = 3;
/// How long an invitation stays open before it expires.
pub const INVITE_TTL_DAYS: i64 = 7;
/// Most invitations a player can have open at once.
pub const MAX_OPEN_INVITES: i64 = 20;

/// Lifecycle of a game against another player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum OnlineGameStatus {
    /// Waiting for the invited player (or anyone with the invite link) to accept.
    Invited,
    Active,
    Finished,
    /// The invited player said no.
    Declined,
    /// The inviter withdrew the invitation.
    Cancelled,
    /// Nobody accepted within `INVITE_TTL_DAYS`.
    Expired,
}

/// A seat at the board. The inviter plays South.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum Seat {
    South,
    North,
}

/// Another player, as shown to you.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct PlayerSummary {
    pub user_id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub handle: Option<String>,
}

/// How a finished game ended. `outcome` is `south_wins`, `north_wins` or `draw`; `reason` is
/// a record end reason (`threshold`, …, `resigned`, `timeout`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct OnlineGameResult {
    pub outcome: String,
    pub reason: String,
    /// Final `[south, north]` stores.
    pub stores: [u8; 2],
}

/// A game against another player, from the requesting player's point of view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct OnlineGame {
    pub id: String,
    pub variant_id: String,
    pub variant_version: u16,
    pub status: OnlineGameStatus,
    pub first_player: Seat,
    /// Your seat.
    pub you: Seat,
    /// The other player; absent for an invite link nobody has accepted yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub opponent: Option<PlayerSummary>,
    /// Moves so far (pit indices); replay them with the engine.
    pub moves: Vec<u8>,
    /// Active, and you are to move.
    pub your_turn: bool,
    /// Active: when the player to move loses on time (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub turn_deadline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub result: Option<OnlineGameResult>,
    /// The code for the invite link, shown to the inviter while nobody has accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub invite_code: Option<String>,
    /// Whether you sent the invitation.
    pub invited_by_you: bool,
    /// Whether the game was created as rated (ADR 0034). A rated game may still not count
    /// (see `rating_changes`).
    #[serde(default)]
    pub rated: bool,
    /// Finished and counted: how your ratings changed (the overall pool and the game's own).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rating_changes: Vec<RatingChange>,
    pub created_at: String,
    pub updated_at: String,
}

/// `POST /v1/games`. With `opponent_handle` or `opponent_id`, an invitation for that player;
/// with neither, an invite link anyone signed in can accept.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct CreateGameRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub opponent_handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub opponent_id: Option<String>,
    /// Defaults to `cv.standard`; the server pins its latest version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub variant_id: Option<String>,
    /// Whether the game counts for ratings (ADR 0034). Defaults to `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rated: Option<bool>,
}

/// `POST /v1/games/join`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct JoinGameRequest {
    pub invite_code: String,
}

/// `POST /v1/games/{id}/moves`. `ply` is the number of moves already played; a repeat of the
/// last accepted move is answered with the current game (safe retries).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct MoveRequest {
    pub ply: u32,
    pub pit: u8,
}

/// `GET /v1/games`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct GameList {
    pub games: Vec<OnlineGame>,
}

/// Someone you've played against (`GET /v1/opponents`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct OpponentSummary {
    pub player: PlayerSummary,
    pub games_played: u32,
    pub last_played_at: String,
}

/// `GET /v1/opponents`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct OpponentList {
    pub opponents: Vec<OpponentSummary>,
}

// ---------------------------------------------------------------------------------------
// Ratings and leaderboards (ADR 0034)
// ---------------------------------------------------------------------------------------

/// The pool every rated game counts in.
pub const RATING_POOL_OVERALL: &str = "overall";
/// Game modes with their own pools: `<variant_id>:<mode>`. `live` comes with live games.
pub const RATING_MODES: [&str; 2] = ["async", "live"];
/// Rated games a player needs in a pool before they appear on its boards.
pub const LEADERBOARD_MIN_GAMES: u32 = 10;
/// ...and how recently they must have played a rated game in it.
pub const LEADERBOARD_ACTIVE_DAYS: i64 = 60;
/// Most entries on a board.
pub const LEADERBOARD_SIZE: u32 = 100;
/// Rated games between the same two players that count in any 24 hours.
pub const RATED_GAMES_PER_PAIR_PER_DAY: i64 = 3;

/// One rating change from a finished game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct RatingChange {
    /// `overall` or `<variant_id>:<mode>`.
    pub pool: String,
    pub before: i32,
    pub after: i32,
}

/// A player's rating in one pool, rounded for display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct PoolRating {
    pub pool: String,
    pub rating: i32,
    /// Rating deviation now (it grows while the player doesn't play rated games).
    pub deviation: i32,
    /// Too uncertain to rank yet (shown as `1520?`).
    pub provisional: bool,
    pub games: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub peak: i32,
}

/// `GET /v1/ratings/me`: every pool you have played rated games in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct MyRatings {
    pub ratings: Vec<PoolRating>,
}

/// A leaderboard's scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum LeaderboardScope {
    World,
    Country,
}

/// One row of a leaderboard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct LeaderboardEntry {
    pub rank: u32,
    pub player: PlayerSummary,
    /// ISO 3166-1 alpha-2, lowercase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub country: Option<String>,
    pub rating: i32,
    pub games: u32,
}

/// `GET /v1/leaderboard?pool=…&scope=world|country[&country=cv]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Leaderboard {
    pub pool: String,
    pub scope: LeaderboardScope,
    /// The country of a country board.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub country: Option<String>,
    /// The top `LEADERBOARD_SIZE` eligible players.
    pub entries: Vec<LeaderboardEntry>,
    /// You, when you're eligible for this board (whether or not you're in the top).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub you: Option<LeaderboardEntry>,
}

// ---------------------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------------------

/// Error codes. Clients must treat unknown codes like `internal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// 400: malformed JSON or invalid field.
    BadRequest,
    /// 401: missing, invalid or expired access/refresh token, or a bad ID token.
    Unauthorized,
    /// 403: authenticated but not allowed (e.g. origin check failed).
    Forbidden,
    /// 404: unknown route or resource.
    NotFound,
    /// 409: handle already taken.
    HandleTaken,
    /// 409: the request doesn't fit the current state (e.g. a move for the wrong turn, or
    /// accepting a game that's no longer an invitation). Refresh and try again.
    Conflict,
    /// 413: request body or batch too large.
    PayloadTooLarge,
    /// 426: client below `min_supported` (from `X-Ouril-Client`).
    UpgradeRequired,
    /// 429: rate limited.
    RateLimited,
    /// 501: feature not configured on this server (e.g. Google/Apple sign-in without keys).
    NotConfigured,
    /// 503: a provider the request depends on is unreachable (e.g. Google/Apple signing keys
    /// couldn't be fetched). Retry later.
    ProviderUnavailable,
    /// 500: unexpected server error.
    Internal,
}

/// Body of every non-2xx response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct ApiError {
    pub code: ErrorCode,
    /// English, for logs and developers. UIs show localized text keyed by `code`.
    pub message: String,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        ApiError {
            code,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_uses_type_key() {
        let m: Mutation = serde_json::from_str(
            r#"{"id":"0192f1c2-0000-7000-8000-000000000000","type":"game_finished","schema":1,"payload":{},"client_time":"2026-10-03T18:31:00Z"}"#,
        )
        .unwrap();
        assert_eq!(m.kind, mutation_types::GAME_FINISHED);
    }

    #[test]
    fn api_error_shape() {
        let e = ApiError::new(ErrorCode::NotConfigured, "Google sign-in is not configured");
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"code":"not_configured","message":"Google sign-in is not configured"}"#
        );
    }

    use serde::de::DeserializeOwned;
    use serde_json::json;

    /// Serialize, compare with the expected JSON, deserialize back to the same value.
    fn round_trip<T>(value: &T, expected: Value)
    where
        T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        let v = serde_json::to_value(value).unwrap();
        assert_eq!(v, expected);
        let back: T = serde_json::from_value(v).unwrap();
        assert_eq!(&back, value);
    }

    #[test]
    fn error_codes_are_snake_case_and_match_api_md() {
        let all = [
            (ErrorCode::BadRequest, "bad_request"),
            (ErrorCode::Unauthorized, "unauthorized"),
            (ErrorCode::Forbidden, "forbidden"),
            (ErrorCode::NotFound, "not_found"),
            (ErrorCode::HandleTaken, "handle_taken"),
            (ErrorCode::Conflict, "conflict"),
            (ErrorCode::PayloadTooLarge, "payload_too_large"),
            (ErrorCode::UpgradeRequired, "upgrade_required"),
            (ErrorCode::RateLimited, "rate_limited"),
            (ErrorCode::NotConfigured, "not_configured"),
            (ErrorCode::ProviderUnavailable, "provider_unavailable"),
            (ErrorCode::Internal, "internal"),
        ];
        for (code, s) in all {
            round_trip(&code, json!(s));
        }
    }

    #[test]
    fn meta_round_trip() {
        let p = PlatformVersions {
            ios: "0.1.0".into(),
            android: "0.1.0".into(),
            web: "0.2.0".into(),
        };
        let m = Meta {
            min_supported: p.clone(),
            recommended: p,
            api: ApiVersions {
                current: API_VERSION.into(),
                deprecated: vec![],
            },
            realtime_proto: RealtimeProto {
                current: REALTIME_PROTO_CURRENT,
                min: REALTIME_PROTO_MIN,
            },
        };
        let platforms = json!({"ios": "0.1.0", "android": "0.1.0", "web": "0.2.0"});
        round_trip(
            &m,
            json!({
                "min_supported": platforms,
                "recommended": platforms,
                "api": {"current": "v1", "deprecated": []},
                "realtime_proto": {"current": 1, "min": 1}
            }),
        );
    }

    #[test]
    fn sign_in_requests_round_trip() {
        round_trip(&DevSignInRequest::default(), json!({}));
        let d: DevSignInRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(d, DevSignInRequest::default());
        round_trip(
            &DevSignInRequest {
                user: Some("dev".into()),
                display_name: Some("Dev Player".into()),
            },
            json!({"user": "dev", "display_name": "Dev Player"}),
        );
        round_trip(
            &GoogleSignInRequest {
                id_token: "t".into(),
                nonce: "n".into(),
            },
            json!({"id_token": "t", "nonce": "n"}),
        );
        assert!(serde_json::from_value::<GoogleSignInRequest>(json!({"id_token": "t"})).is_err());
        round_trip(
            &AppleSignInRequest {
                id_token: "t".into(),
                nonce: "n".into(),
                given_name: None,
                family_name: Some("Lopes".into()),
                authorization_code: None,
            },
            json!({"id_token": "t", "nonce": "n", "family_name": "Lopes"}),
        );
        round_trip(&RefreshRequest::default(), json!({}));
    }

    #[test]
    fn token_pair_omits_refresh_token_for_web() {
        let web = TokenPair {
            access_token: "jwt".into(),
            token_type: "Bearer".into(),
            expires_in: 900,
            refresh_token: None,
        };
        round_trip(
            &web,
            json!({"access_token": "jwt", "token_type": "Bearer", "expires_in": 900}),
        );
        let native = TokenPair {
            refresh_token: Some("opaque".into()),
            ..web
        };
        round_trip(
            &native,
            json!({"access_token": "jwt", "token_type": "Bearer", "expires_in": 900, "refresh_token": "opaque"}),
        );
    }

    fn me() -> Me {
        Me {
            id: "0192f1c2-0000-7000-8000-000000000000".into(),
            display_name: "Ana".into(),
            handle: None,
            avatar_url: None,
            locale: Some("pt".into()),
            country: Some("cv".into()),
            created_at: "2026-10-04T10:00:00Z".into(),
        }
    }

    #[test]
    fn me_and_sign_in_response_round_trip() {
        // `Me` keeps nullable fields as explicit nulls (API.md example).
        let me_json = json!({
            "id": "0192f1c2-0000-7000-8000-000000000000", "display_name": "Ana",
            "handle": null, "avatar_url": null, "locale": "pt", "country": "cv",
            "created_at": "2026-10-04T10:00:00Z"
        });
        round_trip(&me(), me_json.clone());
        round_trip(
            &SignInResponse {
                tokens: TokenPair {
                    access_token: "jwt".into(),
                    token_type: "Bearer".into(),
                    expires_in: 900,
                    refresh_token: None,
                },
                user: me(),
                is_new_user: true,
            },
            json!({
                "tokens": {"access_token": "jwt", "token_type": "Bearer", "expires_in": 900},
                "user": me_json,
                "is_new_user": true
            }),
        );
        round_trip(&ProfilePatch::default(), json!({}));
        round_trip(
            &ProfilePatch {
                handle: Some("ana".into()),
                ..Default::default()
            },
            json!({"handle": "ana"}),
        );
    }

    #[test]
    fn unknown_fields_are_ignored() {
        // Additive-only API: clients and server ignore fields they don't know.
        let p: ProfilePatch =
            serde_json::from_value(json!({"display_name": "A", "future": 1})).unwrap();
        assert_eq!(p.display_name.as_deref(), Some("A"));
        let pull: SyncPull = serde_json::from_value(
            json!({"changes": [], "cursor": 3, "has_more": false, "server_time": "x"}),
        )
        .unwrap();
        assert_eq!(pull.cursor, 3);
    }

    #[test]
    fn sync_push_round_trip() {
        let push = SyncPush {
            mutations: vec![Mutation {
                id: "0192f1c3-0000-7000-8000-000000000000".into(),
                kind: mutation_types::SETTINGS_UPDATED.into(),
                schema: 1,
                payload: json!({"sound": true}),
                client_time: "2026-10-03T18:31:00Z".into(),
            }],
        };
        round_trip(
            &push,
            json!({"mutations": [{
                "id": "0192f1c3-0000-7000-8000-000000000000",
                "type": "settings_updated",
                "schema": 1,
                "payload": {"sound": true},
                "client_time": "2026-10-03T18:31:00Z"
            }]}),
        );
        let result = SyncPushResult {
            results: vec![
                MutationResult {
                    id: "a".into(),
                    status: MutationStatus::Applied,
                    reason: None,
                },
                MutationResult {
                    id: "b".into(),
                    status: MutationStatus::Rejected,
                    reason: Some(rejection_reasons::HANDLE_TAKEN.into()),
                },
            ],
        };
        round_trip(
            &result,
            json!({"results": [
                {"id": "a", "status": "applied"},
                {"id": "b", "status": "rejected", "reason": "handle_taken"}
            ]}),
        );
    }

    #[test]
    fn sync_pull_round_trip() {
        round_trip(&SyncPullQuery::default(), json!({}));
        round_trip(
            &SyncPullQuery {
                cursor: Some(41),
                limit: Some(PULL_DEFAULT_LIMIT),
            },
            json!({"cursor": 41, "limit": 200}),
        );
        let pull = SyncPull {
            changes: vec![
                Change {
                    entity: change_entities::GAME.into(),
                    id: "g".into(),
                    change_seq: 41,
                    deleted: false,
                    data: Some(json!({"format": 1})),
                },
                Change {
                    entity: change_entities::GAME.into(),
                    id: "h".into(),
                    change_seq: 42,
                    deleted: true,
                    data: None,
                },
            ],
            cursor: 42,
            has_more: false,
        };
        round_trip(
            &pull,
            json!({
                "changes": [
                    {"entity": "game", "id": "g", "change_seq": 41, "deleted": false, "data": {"format": 1}},
                    {"entity": "game", "id": "h", "change_seq": 42, "deleted": true}
                ],
                "cursor": 42,
                "has_more": false
            }),
        );
    }

    #[test]
    fn constants_match_api_md() {
        assert_eq!(MAX_MUTATIONS_PER_PUSH, 100);
        assert_eq!(PULL_DEFAULT_LIMIT, 200);
        assert_eq!(PULL_MAX_LIMIT, 500);
        assert_eq!(REFRESH_COOKIE, "ouril_refresh");
        assert_eq!(CLIENT_HEADER, "X-Ouril-Client");
        assert_eq!(
            [
                rejection_reasons::INVALID_PAYLOAD,
                rejection_reasons::ILLEGAL_GAME,
                rejection_reasons::HANDLE_TAKEN,
            ],
            ["invalid_payload", "illegal_game", "handle_taken"]
        );
        assert_eq!(
            [
                deferral_reasons::UNKNOWN_TYPE,
                deferral_reasons::UNSUPPORTED_SCHEMA,
                deferral_reasons::UNSUPPORTED_VARIANT,
                deferral_reasons::UNSUPPORTED_RECORD_FORMAT,
            ],
            [
                "unknown_type",
                "unsupported_schema",
                "unsupported_variant",
                "unsupported_record_format"
            ]
        );
        assert_eq!(
            serde_json::to_value(MutationStatus::Deferred).unwrap(),
            serde_json::json!("deferred")
        );
    }
}
