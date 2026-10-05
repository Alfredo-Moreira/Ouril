//! Offline-first sync (ADR 0011, docs/architecture/data-and-sync.md): applying pushed
//! mutations idempotently, validating game records with `ouril-engine`, and pulling changes
//! after a cursor.

use ouril_engine::{replay, variant_version, MoveEvent, Status};
use ouril_protocol::{
    change_entities, deferral_reasons as deferrals, mutation_types, rejection_reasons as reasons,
    Change, Me, Mutation, MutationResult, MutationStatus,
};
use ouril_sync::{
    GameRecord, HandleRequest, ProfileUpdate, RecordEndReason, SettingsUpdate, GAME_RECORD_FORMAT,
};
use serde_json::{Map, Value};
use sqlx::{Acquire, PgConnection, PgPool, Row};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    users::{self, ProfileChanges, ProfileError},
    validate,
};

/// Longest move list accepted in a game record (far above any real game; bounds replay CPU).
pub const MAX_MOVES_PER_GAME: usize = 4096;
/// AI levels accepted in `GameRecord.ai_level`.
pub const AI_LEVELS: [&str; 3] = ["easy", "medium", "hard"];
/// Payload schema supported for every MVP mutation type.
pub const SUPPORTED_SCHEMA: u16 = 1;

/// Mutation types this server can apply.
pub const KNOWN_TYPES: [&str; 4] = [
    mutation_types::GAME_FINISHED,
    mutation_types::PROFILE_UPDATED,
    mutation_types::SETTINGS_UPDATED,
    mutation_types::HANDLE_REQUESTED,
];

/// Why a mutation wasn't applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotApplied {
    /// Permanently invalid; recorded under the mutation ID ([`reasons`]).
    Rejected(&'static str),
    /// Possibly valid but not supported by this server yet (a newer client). Never recorded,
    /// so the client can push it again once the server is updated ([`deferrals`]).
    Deferred(&'static str),
}

fn deferred(id: &str, reason: &str) -> MutationResult {
    MutationResult {
        id: id.to_string(),
        status: MutationStatus::Deferred,
        reason: Some(reason.to_string()),
    }
}

fn rejected(id: &str, reason: &str) -> MutationResult {
    MutationResult {
        id: id.to_string(),
        status: MutationStatus::Rejected,
        reason: Some(reason.to_string()),
    }
}

fn applied(id: &str) -> MutationResult {
    MutationResult {
        id: id.to_string(),
        status: MutationStatus::Applied,
        reason: None,
    }
}

fn parse_time(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).ok()
}

/// Check a finished game record without touching the database. Replays the moves with the
/// exact variant version and compares the outcome, stores and end reason. A forfeit
/// (`reason: "resigned"`, format 2, ADR 0022) must replay to a game still in progress, and the
/// side that didn't resign wins.
/// A record from a newer client (newer `format`, unknown variant `id@version`) is
/// [`NotApplied::Deferred`]; anything else wrong is [`NotApplied::Rejected`] with
/// `invalid_payload` or `illegal_game`.
///
/// CPU-bound: call through [`validate_game_record_blocking`] from async code.
pub fn validate_game_record(r: &GameRecord) -> Result<(), NotApplied> {
    if r.format > GAME_RECORD_FORMAT {
        return Err(NotApplied::Deferred(deferrals::UNSUPPORTED_RECORD_FORMAT));
    }
    let variant = variant_version(&r.variant.id, r.variant.version)
        .ok_or(NotApplied::Deferred(deferrals::UNSUPPORTED_VARIANT))?;
    check_game_record(r, variant).map_err(NotApplied::Rejected)
}

fn check_game_record(
    r: &GameRecord,
    variant: &ouril_engine::VariantConfig,
) -> Result<(), &'static str> {
    if r.format == 0 || r.format > GAME_RECORD_FORMAT {
        return Err(reasons::INVALID_PAYLOAD);
    }
    Uuid::parse_str(&r.id).map_err(|_| reasons::INVALID_PAYLOAD)?;
    if r.moves.len() > MAX_MOVES_PER_GAME || r.core_version.len() > 64 {
        return Err(reasons::INVALID_PAYLOAD);
    }
    let (Some(started), Some(ended)) = (parse_time(&r.started_at), parse_time(&r.ended_at)) else {
        return Err(reasons::INVALID_PAYLOAD);
    };
    if started > ended {
        return Err(reasons::INVALID_PAYLOAD);
    }
    if let Some(level) = &r.ai_level {
        if !AI_LEVELS.contains(&level.as_str()) {
            return Err(reasons::INVALID_PAYLOAD);
        }
    }
    let replayed = replay(variant, r.first_player, &r.moves).map_err(|_| reasons::ILLEGAL_GAME)?;
    let last = replayed.final_state();
    if r.result.reason == RecordEndReason::Resigned {
        // Forfeits arrived with format 2, and need to know who resigned (the local side).
        let Some(resigned) = r.human_player.filter(|_| r.format >= 2) else {
            return Err(reasons::INVALID_PAYLOAD);
        };
        let winner = match resigned {
            ouril_engine::Player::South => ouril_engine::GameResult::NorthWins,
            ouril_engine::Player::North => ouril_engine::GameResult::SouthWins,
        };
        if last.status != Status::Playing
            || r.result.outcome != winner
            || last.stores != r.result.stores
        {
            return Err(reasons::ILLEGAL_GAME);
        }
        return Ok(());
    }
    let outcome_matches = matches!(
        (last.status, r.result.outcome),
        (
            Status::Won(ouril_engine::Player::South),
            ouril_engine::GameResult::SouthWins
        ) | (
            Status::Won(ouril_engine::Player::North),
            ouril_engine::GameResult::NorthWins
        ) | (Status::Draw, ouril_engine::GameResult::Draw)
    );
    if !outcome_matches || last.stores != r.result.stores {
        return Err(reasons::ILLEGAL_GAME);
    }
    let reason = replayed
        .steps
        .last()
        .and_then(|s| {
            s.events.iter().find_map(|e| match e {
                MoveEvent::GameOver { reason, .. } => Some(*reason),
                _ => None,
            })
        })
        .ok_or(reasons::ILLEGAL_GAME)?;
    if RecordEndReason::from(reason) != r.result.reason {
        return Err(reasons::ILLEGAL_GAME);
    }
    Ok(())
}

/// Run [`validate_game_record`] on the blocking pool. An engine panic is a server bug: it
/// becomes a 500 so the client keeps the mutation and retries later (it is never rejected).
pub async fn validate_game_record_blocking(
    r: GameRecord,
) -> AppResult<(GameRecord, Result<(), NotApplied>)> {
    tokio::task::spawn_blocking(move || {
        let res = validate_game_record(&r);
        (r, res)
    })
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "engine panicked while validating a game record");
        AppError::internal()
    })
}

/// Work done before the mutation's transaction opens. Game records are parsed and replayed
/// here (CPU-bound, up to [`MAX_MOVES_PER_GAME`] moves) so no pool connection or row lock is
/// held during validation. Pure: a repeated mutation ID may be validated again, harmlessly.
enum Prepared {
    /// A `game_finished` mutation of a supported schema: the validated record, or why it
    /// isn't applied.
    Game(Result<GameRecord, NotApplied>),
    /// Everything else is checked inside the transaction.
    Other,
}

async fn prepare(m: &Mutation) -> AppResult<Prepared> {
    if m.kind != mutation_types::GAME_FINISHED || m.schema != SUPPORTED_SCHEMA {
        return Ok(Prepared::Other);
    }
    let record: GameRecord = match payload(&m.payload) {
        Ok(r) => r,
        Err(reason) => return Ok(Prepared::Game(Err(NotApplied::Rejected(reason)))),
    };
    let (record, check) = validate_game_record_blocking(record).await?;
    Ok(Prepared::Game(check.map(|()| record)))
}

/// Why this server can't process `m` yet, if so (checked before anything is recorded).
fn deferral(m: &Mutation, prepared: &Prepared) -> Option<&'static str> {
    if !KNOWN_TYPES.contains(&m.kind.as_str()) {
        return Some(deferrals::UNKNOWN_TYPE);
    }
    if m.schema != SUPPORTED_SCHEMA {
        return Some(deferrals::UNSUPPORTED_SCHEMA);
    }
    match prepared {
        Prepared::Game(Err(NotApplied::Deferred(reason))) => Some(reason),
        _ => None,
    }
}

/// Apply one mutation in its own transaction. Repeated mutation IDs return the outcome
/// recorded the first time without changing anything. Deferred mutations are not recorded,
/// so the same ID can be applied once the server supports it.
pub async fn apply_mutation(db: &PgPool, user_id: Uuid, m: &Mutation) -> AppResult<MutationResult> {
    let Ok(mutation_id) = Uuid::parse_str(&m.id) else {
        return Ok(rejected(&m.id, reasons::INVALID_PAYLOAD));
    };
    // Validation first, outside the transaction (see `Prepared`).
    let prepared = prepare(m).await?;
    if let Some(reason) = deferral(m, &prepared) {
        return Ok(deferred(&m.id, reason));
    }
    let mut tx = db.begin().await?;

    // Claim the mutation ID first. A concurrent push of the same ID waits here until the
    // other transaction finishes, then sees its row.
    let claimed = sqlx::query(
        "INSERT INTO sync_mutation (user_id, id, type, outcome) VALUES ($1, $2, $3, 'applied')
         ON CONFLICT (user_id, id) DO NOTHING RETURNING id",
    )
    .bind(user_id)
    .bind(mutation_id)
    .bind(truncate(&m.kind, 64))
    .fetch_optional(&mut *tx)
    .await?;
    if claimed.is_none() {
        let row =
            sqlx::query("SELECT outcome, reason FROM sync_mutation WHERE user_id = $1 AND id = $2")
                .bind(user_id)
                .bind(mutation_id)
                .fetch_one(&mut *tx)
                .await?;
        let outcome: String = row.try_get("outcome")?;
        let reason: Option<String> = row.try_get("reason")?;
        return Ok(if outcome == "applied" {
            applied(&m.id)
        } else {
            rejected(&m.id, reason.as_deref().unwrap_or(reasons::INVALID_PAYLOAD))
        });
    }

    let outcome = apply_kind(&mut tx, user_id, m, prepared).await?;
    if let Err(reason) = outcome {
        sqlx::query("UPDATE sync_mutation SET outcome = 'rejected', reason = $3 WHERE user_id = $1 AND id = $2")
            .bind(user_id)
            .bind(mutation_id)
            .bind(reason)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(match outcome {
        Ok(()) => applied(&m.id),
        Err(reason) => rejected(&m.id, reason),
    })
}

/// `Ok(Err(reason))` = rejected; the caller records it. Changes made by a rejected mutation
/// are rolled back to a savepoint.
async fn apply_kind(
    conn: &mut PgConnection,
    user_id: Uuid,
    m: &Mutation,
    prepared: Prepared,
) -> AppResult<Result<(), &'static str>> {
    // Unknown types, unsupported schemas and deferred records never get here (`deferral`).
    let mut sp = conn.begin().await?;
    let res = match (m.kind.as_str(), prepared) {
        (mutation_types::GAME_FINISHED, Prepared::Game(record)) => match record {
            Ok(record) => game_finished(&mut sp, user_id, &record).await?,
            Err(NotApplied::Rejected(reason)) => Err(reason),
            Err(NotApplied::Deferred(_)) => unreachable!("deferred before the transaction"),
        },
        (mutation_types::GAME_FINISHED, Prepared::Other) => {
            unreachable!("prepare() validates every supported game_finished")
        }
        (mutation_types::PROFILE_UPDATED, _) => {
            profile_updated(&mut sp, user_id, &m.payload, device_time(&m.client_time)).await?
        }
        (mutation_types::SETTINGS_UPDATED, _) => {
            settings_updated(&mut sp, user_id, &m.payload, &m.client_time).await?
        }
        (mutation_types::HANDLE_REQUESTED, _) => {
            handle_requested(&mut sp, user_id, &m.payload, device_time(&m.client_time)).await?
        }
        _ => unreachable!("unknown types are deferred in apply_mutation"),
    };
    if res.is_ok() {
        sp.commit().await?;
    } else {
        sp.rollback().await?;
    }
    Ok(res)
}

fn payload<T: serde::de::DeserializeOwned>(v: &Value) -> Result<T, &'static str> {
    serde_json::from_value(v.clone()).map_err(|_| reasons::INVALID_PAYLOAD)
}

/// Store a record already checked by [`validate_game_record`] (see `prepare`).
async fn game_finished(
    conn: &mut PgConnection,
    user_id: Uuid,
    record: &GameRecord,
) -> AppResult<Result<(), &'static str>> {
    let game_id = Uuid::parse_str(&record.id).expect("validated");
    let started = parse_time(&record.started_at).expect("validated");
    let ended = parse_time(&record.ended_at).expect("validated");
    let canonical = serde_json::to_value(record).map_err(|_| AppError::internal())?;

    let inserted = sqlx::query(
        "INSERT INTO games (id, user_id, variant_id, variant_version, first_player, moves, mode,
                            ai_level, human_player, result_outcome, result_reason, store_south,
                            store_north, core_version, record_format, record, started_at, ended_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
         ON CONFLICT (id) DO NOTHING
         RETURNING id",
    )
    .bind(game_id)
    .bind(user_id)
    .bind(&record.variant.id)
    .bind(i32::from(record.variant.version))
    .bind(enum_str(&record.first_player))
    .bind(
        record
            .moves
            .iter()
            .map(|&m| i32::from(m))
            .collect::<Vec<i32>>(),
    )
    .bind(enum_str(&record.mode))
    .bind(&record.ai_level)
    .bind(record.human_player.map(|p| enum_str(&p)))
    .bind(enum_str(&record.result.outcome))
    .bind(enum_str(&record.result.reason))
    .bind(i32::from(record.result.stores[0]))
    .bind(i32::from(record.result.stores[1]))
    .bind(&record.core_version)
    .bind(i32::from(record.format))
    .bind(&canonical)
    .bind(started)
    .bind(ended)
    .fetch_optional(&mut *conn)
    .await?;
    if inserted.is_some() {
        return Ok(Ok(()));
    }
    // Same game pushed again (union by ID): fine if it's ours, rejected if the ID belongs to
    // someone else.
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM games WHERE id = $1")
        .bind(game_id)
        .fetch_optional(&mut *conn)
        .await?;
    Ok(if owner == Some(user_id) {
        Ok(())
    } else {
        Err(reasons::INVALID_PAYLOAD)
    })
}

/// The serde string of a unit enum (`Player::South` -> `"south"`).
fn enum_str<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn validated<T>(
    v: Option<String>,
    f: fn(&str) -> Result<T, &'static str>,
) -> Result<Option<T>, &'static str> {
    v.map(|s| f(&s).map_err(|_| reasons::INVALID_PAYLOAD))
        .transpose()
}

/// When a synced change was made: its `client_time`, clamped to the server's clock so a device
/// with a clock set in the future can't win forever. Unparsable means now.
fn device_time(client_time: &str) -> OffsetDateTime {
    let now = OffsetDateTime::now_utc();
    parse_time(client_time).map_or(now, |t| t.min(now))
}

async fn profile_updated(
    conn: &mut PgConnection,
    user_id: Uuid,
    v: &Value,
    at: OffsetDateTime,
) -> AppResult<Result<(), &'static str>> {
    let p: ProfileUpdate = match payload(v) {
        Ok(p) => p,
        Err(reason) => return Ok(Err(reason)),
    };
    let changes = (|| {
        Ok::<_, &'static str>(ProfileChanges {
            display_name: validated(p.display_name, validate::display_name)?,
            handle: None,
            avatar_url: validated(p.avatar_url, validate::avatar_url)?,
            locale: validated(p.locale, validate::locale)?,
            country: validated(p.country, validate::country)?,
        })
    })();
    let changes = match changes {
        Ok(c) => c,
        Err(reason) => return Ok(Err(reason)),
    };
    match users::update_profile_at(conn, user_id, &changes, at).await {
        Ok(()) => Ok(Ok(())),
        Err(ProfileError::HandleTaken) => Ok(Err(reasons::HANDLE_TAKEN)),
        Err(ProfileError::Db(e)) => Err(e.into()),
    }
}

async fn handle_requested(
    conn: &mut PgConnection,
    user_id: Uuid,
    v: &Value,
    at: OffsetDateTime,
) -> AppResult<Result<(), &'static str>> {
    let req: HandleRequest = match payload(v) {
        Ok(r) => r,
        Err(reason) => return Ok(Err(reason)),
    };
    let Ok(handle) = validate::handle(&req.handle) else {
        return Ok(Err(reasons::INVALID_PAYLOAD));
    };
    let changes = ProfileChanges {
        handle: Some(handle),
        ..Default::default()
    };
    match users::update_profile_at(conn, user_id, &changes, at).await {
        Ok(()) => Ok(Ok(())),
        Err(ProfileError::HandleTaken) => Ok(Err(reasons::HANDLE_TAKEN)),
        Err(ProfileError::Db(e)) => Err(e.into()),
    }
}

/// Field-level last-writer-wins by **device time**: each field remembers the `client_time` of
/// the change that set it, and an older change never overwrites a newer one. This keeps a
/// mutation that was deferred and applied later (or delivered late by an offline device) from
/// undoing a newer setting. Device times later than the server's clock are clamped to now, so
/// a device with a clock set in the future can't win forever. Ties apply in arrival order.
async fn settings_updated(
    conn: &mut PgConnection,
    user_id: Uuid,
    v: &Value,
    client_time: &str,
) -> AppResult<Result<(), &'static str>> {
    let s: SettingsUpdate = match payload(v) {
        Ok(s) => s,
        Err(reason) => return Ok(Err(reason)),
    };
    let mut patch = Map::new();
    if let Some(sound) = s.sound {
        patch.insert("sound".into(), Value::Bool(sound));
    }
    if let Some(hints) = s.hints {
        patch.insert("hints".into(), Value::Bool(hints));
    }
    if let Some(language) = s.language {
        match validate::locale(&language) {
            Ok(l) => patch.insert("language".into(), Value::String(l)),
            Err(_) => return Ok(Err(reasons::INVALID_PAYLOAD)),
        };
    }
    if patch.is_empty() {
        return Ok(Ok(()));
    }
    let at = device_time(client_time);
    let at_str = at.format(&Rfc3339).map_err(|_| AppError::internal())?;

    // Make sure the row exists, then lock it so concurrent pushes merge one at a time.
    sqlx::query("INSERT INTO user_settings (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING")
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
    let row = sqlx::query(
        "SELECT settings, field_times FROM user_settings WHERE user_id = $1 FOR UPDATE",
    )
    .bind(user_id)
    .fetch_one(&mut *conn)
    .await?;
    let as_map = |v: Value| match v {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    let mut settings = as_map(row.try_get("settings")?);
    let mut times = as_map(row.try_get("field_times")?);

    let mut changed = false;
    for (field, value) in patch {
        let newer = times
            .get(&field)
            .and_then(Value::as_str)
            .and_then(parse_time)
            .is_none_or(|prev| at >= prev);
        if newer {
            settings.insert(field.clone(), value);
            times.insert(field, Value::String(at_str.clone()));
            changed = true;
        }
    }
    if changed {
        sqlx::query(
            "UPDATE user_settings
                SET settings = $2, field_times = $3, updated_at = now(),
                    change_seq = nextval('change_seq')
              WHERE user_id = $1",
        )
        .bind(user_id)
        .bind(Value::Object(settings))
        .bind(Value::Object(times))
        .execute(&mut *conn)
        .await?;
    }
    // A change that is entirely older than what's stored is still `applied`: it was merged,
    // and every one of its fields lost to a newer change.
    Ok(Ok(()))
}

/// Up to `limit` changes after `cursor`, ordered by `change_seq`, plus whether more exist.
pub async fn pull(
    db: &PgPool,
    user_id: Uuid,
    cursor: i64,
    limit: i64,
) -> AppResult<(Vec<Change>, bool)> {
    let mut conn = db.acquire().await?;
    let rows = sqlx::query(
        "SELECT entity, id, change_seq, deleted, data FROM (
            SELECT 'profile' AS entity, id, change_seq, false AS deleted, NULL::jsonb AS data
              FROM users WHERE id = $1 AND deleted_at IS NULL AND change_seq > $2
            UNION ALL
            SELECT 'settings', user_id, change_seq, false, settings
              FROM user_settings WHERE user_id = $1 AND change_seq > $2
            UNION ALL
            SELECT 'game', id, change_seq, deleted_at IS NOT NULL,
                   CASE WHEN deleted_at IS NULL THEN record END
              FROM games WHERE user_id = $1 AND change_seq > $2
         ) c
         ORDER BY change_seq
         LIMIT $3",
    )
    .bind(user_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&mut *conn)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let mut me: Option<Me> = None;
    let mut changes = Vec::with_capacity(rows.len().min(limit as usize));
    for row in rows.into_iter().take(limit as usize) {
        let entity: String = row.try_get("entity")?;
        let id: Uuid = row.try_get("id")?;
        let seq: i64 = row.try_get("change_seq")?;
        let deleted: bool = row.try_get("deleted")?;
        let mut data: Option<Value> = row.try_get("data")?;
        if entity == change_entities::PROFILE {
            if me.is_none() {
                me = users::load_me(&mut conn, user_id).await?;
            }
            data = me.as_ref().and_then(|m| serde_json::to_value(m).ok());
        }
        changes.push(Change {
            entity,
            id: id.to_string(),
            change_seq: seq.max(0) as u64,
            deleted,
            data: if deleted { None } else { data },
        });
    }
    Ok((changes, has_more))
}

fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ouril_engine::{apply_move, legal_moves, new_game, Player};
    use ouril_sync::{GameMode, GameRecordResult, VariantRef, GAME_RECORD_FORMAT_RULES};

    /// Play a whole cv.standard@1 game with a fixed policy (always the first legal move) and
    /// build its record from the engine's own result.
    pub(crate) fn played_record() -> GameRecord {
        let v = variant_version("cv.standard", 1).expect("cv.standard@1");
        let mut s = new_game(v, Player::South);
        let mut moves = Vec::new();
        let mut reason = None;
        while s.status == Status::Playing {
            let m = legal_moves(v, &s)[0];
            let r = apply_move(v, &s, m).expect("legal");
            moves.push(m);
            for e in &r.events {
                if let MoveEvent::GameOver { reason: why, .. } = e {
                    reason = Some(*why);
                }
            }
            s = r.state;
            assert!(moves.len() < MAX_MOVES_PER_GAME, "game did not end");
        }
        let outcome = match s.status {
            Status::Won(Player::South) => ouril_engine::GameResult::SouthWins,
            Status::Won(Player::North) => ouril_engine::GameResult::NorthWins,
            _ => ouril_engine::GameResult::Draw,
        };
        GameRecord {
            format: GAME_RECORD_FORMAT_RULES,
            id: Uuid::now_v7().to_string(),
            variant: VariantRef {
                id: "cv.standard".into(),
                version: 1,
            },
            first_player: Player::South,
            moves,
            core_version: "0.1.0".into(),
            result: GameRecordResult {
                outcome,
                stores: s.stores,
                reason: reason.expect("game over event").into(),
            },
            started_at: "2026-10-03T18:20:00Z".into(),
            ended_at: "2026-10-03T18:31:00Z".into(),
            mode: GameMode::VsAi,
            ai_level: Some("easy".into()),
            human_player: Some(Player::South),
        }
    }

    #[test]
    fn accepts_a_real_game() {
        let r = played_record();
        assert_eq!(validate_game_record(&r), Ok(()));
        // Handy for manual curl tests: cargo test -p ouril-server played -- --nocapture
        println!("{}", serde_json::to_string(&r).unwrap());
    }

    /// South forfeits after the first `n` moves of the played game.
    fn forfeit_record(n: usize) -> GameRecord {
        let mut r = played_record();
        let v = variant_version("cv.standard", 1).expect("cv.standard@1");
        r.moves.truncate(n);
        let last = replay(v, r.first_player, &r.moves).unwrap();
        r.format = GAME_RECORD_FORMAT;
        r.result = GameRecordResult {
            outcome: ouril_engine::GameResult::NorthWins,
            stores: last.final_state().stores,
            reason: RecordEndReason::Resigned,
        };
        r
    }

    #[test]
    fn accepts_a_forfeit() {
        assert_eq!(validate_game_record(&forfeit_record(6)), Ok(()));
        assert_eq!(
            validate_game_record(&forfeit_record(0)),
            Ok(()),
            "before any move"
        );
    }

    #[test]
    fn rejects_bad_forfeits() {
        let finished = played_record();
        let mut r = forfeit_record(finished.moves.len());
        r.result.stores = finished.result.stores;
        assert_eq!(
            validate_game_record(&r),
            Err(NotApplied::Rejected(reasons::ILLEGAL_GAME)),
            "forfeit after the game ended"
        );

        let mut r = forfeit_record(6);
        r.result.outcome = ouril_engine::GameResult::SouthWins;
        assert_eq!(
            validate_game_record(&r),
            Err(NotApplied::Rejected(reasons::ILLEGAL_GAME)),
            "the side that resigned can't win"
        );

        let mut r = forfeit_record(6);
        r.result.stores = [r.result.stores[0] + 1, r.result.stores[1]];
        assert_eq!(
            validate_game_record(&r),
            Err(NotApplied::Rejected(reasons::ILLEGAL_GAME))
        );

        let mut r = forfeit_record(6);
        r.format = GAME_RECORD_FORMAT_RULES;
        assert_eq!(
            validate_game_record(&r),
            Err(NotApplied::Rejected(reasons::INVALID_PAYLOAD)),
            "forfeits need format 2"
        );

        let mut r = forfeit_record(6);
        r.human_player = None;
        assert_eq!(
            validate_game_record(&r),
            Err(NotApplied::Rejected(reasons::INVALID_PAYLOAD)),
            "who resigned is unknown"
        );
    }

    #[test]
    fn rejects_tampered_games() {
        let good = played_record();

        let mut r = good.clone();
        r.result.stores = [r.result.stores[1], r.result.stores[0]];
        if r.result.stores != good.result.stores {
            assert_eq!(
                validate_game_record(&r),
                Err(NotApplied::Rejected(reasons::ILLEGAL_GAME))
            );
        }

        let mut r = good.clone();
        r.moves.pop();
        assert_eq!(
            validate_game_record(&r),
            Err(NotApplied::Rejected(reasons::ILLEGAL_GAME)),
            "unfinished game"
        );

        let mut r = good.clone();
        r.moves.push(0);
        assert_eq!(
            validate_game_record(&r),
            Err(NotApplied::Rejected(reasons::ILLEGAL_GAME)),
            "move after game over"
        );

        let mut r = good.clone();
        r.moves[0] = 11; // North's pit on South's first move
        assert_eq!(
            validate_game_record(&r),
            Err(NotApplied::Rejected(reasons::ILLEGAL_GAME))
        );

        let mut r = good.clone();
        r.result.outcome = match r.result.outcome {
            ouril_engine::GameResult::SouthWins => ouril_engine::GameResult::NorthWins,
            _ => ouril_engine::GameResult::SouthWins,
        };
        assert_eq!(
            validate_game_record(&r),
            Err(NotApplied::Rejected(reasons::ILLEGAL_GAME))
        );
    }

    #[test]
    fn rejects_bad_metadata() {
        let good = played_record();
        type Tamper = Box<dyn Fn(&mut GameRecord)>;
        let cases: Vec<Tamper> = vec![
            Box::new(|r| r.format = 0),
            Box::new(|r| r.id = "not-a-uuid".into()),
            Box::new(|r| r.started_at = "yesterday".into()),
            Box::new(|r| std::mem::swap(&mut r.started_at, &mut r.ended_at)),
            Box::new(|r| r.ai_level = Some("impossible".into())),
        ];
        for (i, tamper) in cases.iter().enumerate() {
            let mut r = good.clone();
            tamper(&mut r);
            assert_eq!(
                validate_game_record(&r),
                Err(NotApplied::Rejected(reasons::INVALID_PAYLOAD)),
                "case {i}"
            );
        }
    }

    #[test]
    fn defers_records_from_newer_clients() {
        let good = played_record();
        type Tamper = Box<dyn Fn(&mut GameRecord)>;
        let cases: Vec<(Tamper, &str)> = vec![
            (
                Box::new(|r| r.format = GAME_RECORD_FORMAT + 1),
                deferrals::UNSUPPORTED_RECORD_FORMAT,
            ),
            (
                Box::new(|r| r.variant.version = 99),
                deferrals::UNSUPPORTED_VARIANT,
            ),
            (
                Box::new(|r| r.variant.id = "xx.unknown".into()),
                deferrals::UNSUPPORTED_VARIANT,
            ),
        ];
        for (i, (tamper, reason)) in cases.iter().enumerate() {
            let mut r = good.clone();
            tamper(&mut r);
            assert_eq!(
                validate_game_record(&r),
                Err(NotApplied::Deferred(reason)),
                "case {i}"
            );
        }
    }

    #[test]
    fn enum_strings() {
        assert_eq!(enum_str(&ouril_engine::Player::North), "north");
        assert_eq!(enum_str(&GameMode::VsAi), "vs_ai");
    }
}
