//! Profile reads/writes and account deletion.

use ouril_protocol::Me;
use serde_json::{Map, Value};
use sqlx::{FromRow, PgConnection, PgPool, Row};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

/// Name shown for deleted accounts (kept so future online opponents' history stays intact).
pub const DELETED_DISPLAY_NAME: &str = "Deleted player";

#[derive(Debug, FromRow)]
struct UserRow {
    id: Uuid,
    display_name: String,
    handle: Option<String>,
    avatar_url: Option<String>,
    locale: Option<String>,
    country: Option<String>,
    created_at: OffsetDateTime,
}

impl From<UserRow> for Me {
    fn from(r: UserRow) -> Me {
        Me {
            id: r.id.to_string(),
            display_name: r.display_name,
            handle: r.handle,
            avatar_url: r.avatar_url,
            locale: r.locale,
            country: r.country,
            created_at: rfc3339(r.created_at),
        }
    }
}

/// RFC 3339 in UTC (`…Z`).
pub fn rfc3339(t: OffsetDateTime) -> String {
    t.to_offset(time::UtcOffset::UTC)
        .format(&Rfc3339)
        .unwrap_or_default()
}

/// The profile of a non-deleted user.
pub async fn load_me(conn: &mut PgConnection, user_id: Uuid) -> Result<Option<Me>, sqlx::Error> {
    let row: Option<UserRow> = sqlx::query_as(
        "SELECT id, display_name, handle, avatar_url, locale, country, created_at
         FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .fetch_optional(conn)
    .await?;
    Ok(row.map(Me::from))
}

/// Validated profile changes (`None` = unchanged).
#[derive(Debug, Default, Clone)]
pub struct ProfileChanges {
    pub display_name: Option<String>,
    pub handle: Option<String>,
    pub avatar_url: Option<String>,
    pub locale: Option<String>,
    pub country: Option<String>,
}

impl ProfileChanges {
    pub fn is_empty(&self) -> bool {
        self.display_name.is_none()
            && self.handle.is_none()
            && self.avatar_url.is_none()
            && self.locale.is_none()
            && self.country.is_none()
    }
}

#[derive(Debug)]
pub enum ProfileError {
    HandleTaken,
    Db(sqlx::Error),
}

impl From<sqlx::Error> for ProfileError {
    fn from(e: sqlx::Error) -> Self {
        ProfileError::Db(e)
    }
}

/// Apply profile changes made at `at`, field-level last-writer-wins by that time: each field
/// remembers when it was set (`users.profile_field_times`) and an older change never
/// overwrites a newer value. Callers pass the clamped device time for synced changes and
/// `now` for direct edits. Fields that lose are dropped silently (the change still counts as
/// applied). A handle someone else has (case-insensitive) is `HandleTaken`. Run inside a
/// transaction or savepoint: a unique violation aborts it.
pub async fn update_profile_at(
    conn: &mut PgConnection,
    user_id: Uuid,
    c: &ProfileChanges,
    at: OffsetDateTime,
) -> Result<(), ProfileError> {
    if c.is_empty() {
        return Ok(());
    }
    // Lock the row so concurrent changes merge one at a time.
    let Some(row) = sqlx::query(
        "SELECT profile_field_times FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(user_id)
    .fetch_optional(&mut *conn)
    .await?
    else {
        return Ok(()); // deleted account: nothing to update
    };
    let mut times = match row.try_get::<Value, _>("profile_field_times")? {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    let newer = |field: &str| {
        times
            .get(field)
            .and_then(Value::as_str)
            .and_then(|s| OffsetDateTime::parse(s, &Rfc3339).ok())
            .is_none_or(|prev| at >= prev)
    };
    let keep = |field: &str, v: &Option<String>| v.clone().filter(|_| newer(field));
    let winning = ProfileChanges {
        display_name: keep("display_name", &c.display_name),
        handle: keep("handle", &c.handle),
        avatar_url: keep("avatar_url", &c.avatar_url),
        locale: keep("locale", &c.locale),
        country: keep("country", &c.country),
    };
    if winning.is_empty() {
        return Ok(());
    }
    update_profile(conn, user_id, &winning).await?;

    let stamp =
        Value::String(at.format(&Rfc3339).map_err(|_| {
            ProfileError::Db(sqlx::Error::Protocol("unformattable timestamp".into()))
        })?);
    for (field, set) in [
        ("display_name", winning.display_name.is_some()),
        ("handle", winning.handle.is_some()),
        ("avatar_url", winning.avatar_url.is_some()),
        ("locale", winning.locale.is_some()),
        ("country", winning.country.is_some()),
    ] {
        if set {
            times.insert(field.to_string(), stamp.clone());
        }
    }
    sqlx::query("UPDATE users SET profile_field_times = $2 WHERE id = $1")
        .bind(user_id)
        .bind(Value::Object(times))
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Write the given fields and bump `change_seq` (no ordering check; see
/// [`update_profile_at`]).
async fn update_profile(
    conn: &mut PgConnection,
    user_id: Uuid,
    c: &ProfileChanges,
) -> Result<(), ProfileError> {
    if c.is_empty() {
        return Ok(());
    }
    let res = sqlx::query(
        "UPDATE users SET
            display_name = COALESCE($2, display_name),
            handle       = COALESCE($3, handle),
            avatar_url   = COALESCE($4, avatar_url),
            locale       = COALESCE($5, locale),
            country      = COALESCE($6, country),
            updated_at   = now(),
            change_seq   = nextval('change_seq')
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .bind(&c.display_name)
    .bind(&c.handle)
    .bind(&c.avatar_url)
    .bind(&c.locale)
    .bind(&c.country)
    .execute(&mut *conn)
    .await;
    match res {
        Ok(_) => Ok(()),
        Err(e) if is_unique_violation(&e, "users_handle_lower_key") => {
            Err(ProfileError::HandleTaken)
        }
        Err(e) => Err(ProfileError::Db(e)),
    }
}

pub fn is_unique_violation(e: &sqlx::Error, constraint: &str) -> bool {
    e.as_database_error()
        .is_some_and(|d| d.code().as_deref() == Some("23505") && d.constraint() == Some(constraint))
}

/// `(client_id, refresh_token)` of the user's Apple identities, to revoke on deletion.
pub async fn apple_tokens(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT provider_client_id, provider_refresh_token FROM auth_identities
         WHERE user_id = $1 AND provider = 'apple'
           AND provider_client_id IS NOT NULL AND provider_refresh_token IS NOT NULL",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
}

/// Delete an account: revoke every session, remove identities, settings, games and sync
/// history, and anonymize the user row ("Deleted player"). Signing in again with the same
/// provider creates a new, empty account. Apple tokens are revoked by the caller
/// (`routes::me::delete_me`) with [`apple_tokens`] read before this runs.
pub async fn delete_account(db: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE sessions SET revoked_at = COALESCE(revoked_at, now()) WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "DELETE FROM session_refresh_tokens WHERE session_id IN (SELECT id FROM sessions WHERE user_id = $1)",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;
    for sql in [
        "DELETE FROM auth_identities WHERE user_id = $1",
        "DELETE FROM user_settings WHERE user_id = $1",
        "DELETE FROM games WHERE user_id = $1",
        "DELETE FROM sync_mutation WHERE user_id = $1",
    ] {
        sqlx::query(sql).bind(user_id).execute(&mut *tx).await?;
    }
    sqlx::query(
        "UPDATE users SET display_name = $2, handle = NULL, avatar_url = NULL, locale = NULL,
                country = NULL, deleted_at = now(), updated_at = now(),
                change_seq = nextval('change_seq')
         WHERE id = $1",
    )
    .bind(user_id)
    .bind(DELETED_DISPLAY_NAME)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
