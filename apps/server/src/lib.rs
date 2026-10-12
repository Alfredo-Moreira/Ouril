//! `ouril-server`: the Ouril API (MVP: auth, profile, offline-first sync).
//! Contract: `apps/server/API.md`. Design: `docs/architecture/backend.md`.
//!
//! The binary (`src/main.rs`) only loads [`config::Config`], connects to Postgres and serves
//! [`app`]. Everything else lives here so integration tests can build the same router.
//!
//! **Guests never call this server.** Every endpoint except `/healthz` and `/v1/meta` is used
//! only after the player chose to sign in.

#[cfg(all(feature = "dev-auth", not(debug_assertions)))]
compile_error!("the `dev-auth` feature must never be compiled into release builds (ADR 0017)");

pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod ratelimit;
pub mod routes;
pub mod state;
pub mod sync;
pub mod users;
pub mod validate;

pub use routes::router as app;
pub use state::AppState;

/// Migrations embedded at compile time from `apps/server/migrations`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Fail unless `db` accepts writes. A read replica's connection string in `DATABASE_URL` (Neon
/// shows one per compute, ADR 0027) would otherwise surface later as failed sign-ins and syncs;
/// `ouril-server migrate` calls this first, so such a deploy stops at the release command.
pub async fn ensure_writable(db: &sqlx::PgPool) -> Result<(), String> {
    let read_only: bool = sqlx::query_scalar(
        "SELECT pg_is_in_recovery() OR current_setting('transaction_read_only') = 'on'",
    )
    .fetch_one(db)
    .await
    .map_err(|e| format!("database check failed: {e}"))?;
    if read_only {
        return Err(
            "DATABASE_URL points at a read-only database (a read replica?): use the \
                    primary compute's connection string"
                .into(),
        );
    }
    Ok(())
}

/// Select rustls' process-wide crypto provider (aws-lc-rs). Idempotent; called by
/// [`AppState::new`] before any TLS connection (Postgres with TLS, JWKS fetches).
pub fn install_crypto_provider() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

/// True when this binary was built with the development identity provider.
pub const DEV_AUTH_ENABLED: bool = cfg!(feature = "dev-auth");
