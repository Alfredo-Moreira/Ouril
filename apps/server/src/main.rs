//! `ouril-server` binary.
//!
//! Usage:
//!   ouril-server            serve HTTP on $HOST:$PORT
//!   ouril-server migrate    run pending database migrations and exit (Fly release command)
//!
//! Debug builds also apply pending migrations on startup (local dev convenience); release
//! builds rely on the release command (ADR 0019).

use std::{net::SocketAddr, time::Duration};

use ouril_server::{config::Config, AppState, MIGRATOR};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    ouril_server::install_crypto_provider();

    let config = Config::from_env()?;
    let pool = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect_lazy(&config.database_url)?;

    if std::env::args().nth(1).as_deref() == Some("migrate") {
        MIGRATOR.run(&pool).await?;
        tracing::info!("migrations applied");
        return Ok(());
    }

    if cfg!(debug_assertions) {
        MIGRATOR.run(&pool).await?;
        tracing::info!("debug build: migrations applied on startup");
    }

    if ouril_server::DEV_AUTH_ENABLED {
        tracing::warn!("dev-auth is ENABLED: POST /v1/auth/dev signs in without Google/Apple");
    }
    tracing::info!(
        google = config.google_client_id.is_some(),
        apple = config.apple.is_some(),
        origins = ?config.allowed_origins,
        "sign-in providers configured"
    );

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let state = AppState::new(config, pool);
    let app = ouril_server::app(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "ouril-server listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    tracing::info!("ouril-server stopped");
    Ok(())
}

/// `RUST_LOG` filters (default `info`); `LOG_FORMAT=json` switches to JSON lines (production).
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let json = std::env::var("LOG_FORMAT")
        .map(|v| v.eq_ignore_ascii_case("json"))
        .unwrap_or(false);
    if json {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_current_span(true)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received, draining connections");
}
