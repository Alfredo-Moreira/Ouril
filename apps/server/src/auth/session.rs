//! Users, linked identities and sessions in Postgres.

use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use super::tokens::{hash_refresh_token, new_refresh_token};

/// A verified identity from a provider (`dev`, `google`, `apple`).
#[derive(Debug, Clone)]
pub struct Identity {
    pub provider: &'static str,
    pub subject: String,
    pub email: Option<String>,
    pub email_is_private_relay: bool,
    /// Used only when the account is created (Apple sends the name only once).
    pub display_name: Option<String>,
}

/// Display name for accounts created without one.
pub const DEFAULT_DISPLAY_NAME: &str = "Player";

/// Find the user linked to `(provider, subject)`, or create the user and identity.
/// Returns `(user_id, is_new_user)`. Email is stored but never used to merge accounts.
pub async fn find_or_create_user(db: &PgPool, id: &Identity) -> Result<(Uuid, bool), sqlx::Error> {
    if let Some(user_id) = find_identity(db, id).await? {
        return Ok((user_id, false));
    }
    let mut tx = db.begin().await?;
    let user_id = Uuid::now_v7();
    let name = id
        .display_name
        .clone()
        .unwrap_or_else(|| DEFAULT_DISPLAY_NAME.to_string());
    sqlx::query("INSERT INTO users (id, display_name) VALUES ($1, $2)")
        .bind(user_id)
        .bind(&name)
        .execute(&mut *tx)
        .await?;
    let linked = sqlx::query(
        "INSERT INTO auth_identities (id, user_id, provider, provider_subject, email, email_is_private_relay)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (provider, provider_subject) DO NOTHING
         RETURNING user_id",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(id.provider)
    .bind(&id.subject)
    .bind(&id.email)
    .bind(id.email_is_private_relay)
    .fetch_optional(&mut *tx)
    .await?;
    if linked.is_none() {
        // A concurrent first sign-in won the race: use its user.
        tx.rollback().await?;
        let user_id = find_identity(db, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;
        return Ok((user_id, false));
    }
    tx.commit().await?;
    Ok((user_id, true))
}

/// Keep a provider's refresh token for an identity (Apple: only to revoke it on deletion).
pub async fn store_provider_token(
    db: &PgPool,
    provider: &str,
    subject: &str,
    client_id: &str,
    refresh_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE auth_identities SET provider_refresh_token = $3, provider_client_id = $4
         WHERE provider = $1 AND provider_subject = $2",
    )
    .bind(provider)
    .bind(subject)
    .bind(refresh_token)
    .bind(client_id)
    .execute(db)
    .await?;
    Ok(())
}

async fn find_identity(db: &PgPool, id: &Identity) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT i.user_id FROM auth_identities i JOIN users u ON u.id = i.user_id
         WHERE i.provider = $1 AND i.provider_subject = $2 AND u.deleted_at IS NULL",
    )
    .bind(id.provider)
    .bind(&id.subject)
    .fetch_optional(db)
    .await
}

/// A new session plus its first refresh token (returned in clear exactly once).
pub async fn create_session(
    db: &PgPool,
    user_id: Uuid,
    device_name: Option<&str>,
    ttl_days: u32,
) -> Result<(Uuid, String), sqlx::Error> {
    let session_id = Uuid::now_v7();
    let token = new_refresh_token();
    let mut tx = db.begin().await?;
    sqlx::query(
        "INSERT INTO sessions (id, user_id, device_name, expires_at)
         VALUES ($1, $2, $3, now() + make_interval(days => $4))",
    )
    .bind(session_id)
    .bind(user_id)
    .bind(device_name)
    .bind(ttl_days as i32)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO session_refresh_tokens (token_hash, session_id) VALUES ($1, $2)")
        .bind(hash_refresh_token(&token))
        .bind(session_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok((session_id, token))
}

#[derive(Debug, PartialEq, Eq)]
pub enum RotateError {
    /// Unknown, expired or revoked token / session.
    Invalid,
    /// A token that was already rotated was presented again: the session is now revoked.
    Reused,
}

/// Rotate a refresh token: the old one stops working, a new one is returned. Presenting an
/// already-rotated token revokes the whole session (refresh-token reuse detection).
/// Each successful refresh also extends the session by `ttl_days` (sliding expiry).
pub async fn rotate_refresh_token(
    db: &PgPool,
    token: &str,
    ttl_days: u32,
) -> Result<Result<(Uuid, Uuid, String), RotateError>, sqlx::Error> {
    let hash = hash_refresh_token(token);
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "SELECT t.session_id, t.rotated_at, s.user_id, s.revoked_at, s.expires_at, u.deleted_at
         FROM session_refresh_tokens t
         JOIN sessions s ON s.id = t.session_id
         JOIN users u ON u.id = s.user_id
         WHERE t.token_hash = $1
         FOR UPDATE OF t, s",
    )
    .bind(&hash)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row else {
        return Ok(Err(RotateError::Invalid));
    };
    let session_id: Uuid = row.try_get("session_id")?;
    let user_id: Uuid = row.try_get("user_id")?;
    let rotated_at: Option<OffsetDateTime> = row.try_get("rotated_at")?;
    let revoked_at: Option<OffsetDateTime> = row.try_get("revoked_at")?;
    let expires_at: OffsetDateTime = row.try_get("expires_at")?;
    let deleted_at: Option<OffsetDateTime> = row.try_get("deleted_at")?;

    if rotated_at.is_some() {
        sqlx::query("UPDATE sessions SET revoked_at = COALESCE(revoked_at, now()) WHERE id = $1")
            .bind(session_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        tracing::warn!(%session_id, "refresh token reuse detected: session revoked");
        return Ok(Err(RotateError::Reused));
    }
    if revoked_at.is_some() || deleted_at.is_some() || expires_at <= OffsetDateTime::now_utc() {
        return Ok(Err(RotateError::Invalid));
    }

    let new_token = new_refresh_token();
    sqlx::query("UPDATE session_refresh_tokens SET rotated_at = now() WHERE token_hash = $1")
        .bind(&hash)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO session_refresh_tokens (token_hash, session_id) VALUES ($1, $2)")
        .bind(hash_refresh_token(&new_token))
        .bind(session_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE sessions SET last_used_at = now(), expires_at = now() + make_interval(days => $2)
         WHERE id = $1",
    )
    .bind(session_id)
    .bind(ttl_days as i32)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Ok((user_id, session_id, new_token)))
}

/// Revoke the session a refresh token belongs to (current or already-rotated token).
/// Unknown tokens are ignored (logout is idempotent).
pub async fn revoke_by_refresh_token(db: &PgPool, token: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE sessions SET revoked_at = COALESCE(revoked_at, now())
         WHERE id = (SELECT session_id FROM session_refresh_tokens WHERE token_hash = $1)",
    )
    .bind(hash_refresh_token(token))
    .execute(db)
    .await?;
    Ok(())
}

/// Whether an access token's session is still usable (not revoked or expired, user not
/// deleted). Checked on every authenticated request so logout and deletion take effect
/// immediately instead of after the access token's TTL.
pub async fn session_active(
    db: &PgPool,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let found: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.id = $1 AND s.user_id = $2 AND s.revoked_at IS NULL AND s.expires_at > now()
           AND u.deleted_at IS NULL",
    )
    .bind(session_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?;
    Ok(found.is_some())
}
