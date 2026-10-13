//! `ouril-relay` binary. See `src/lib.rs` for what the relay does.
//!
//! Usage:
//!   ouril-relay              serve on $PORT (8090)
//!   ouril-relay healthcheck  exit 0 if the relay on this machine answers `/healthz`
//!                            (the image has no shell or curl; Docker's HEALTHCHECK uses this)
//!
//! Settings come from the environment and, like the `home-lab` repo's images, from a `.env`
//! baked into the image (`/app/.env`, or `$RELAY_ENV_FILE`). Values already set in the
//! environment (the compose file) win.

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use ouril_relay::{app, Config};
use tracing_subscriber::EnvFilter;

const DEFAULT_ENV_FILE: &str = "/app/.env";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Before any thread exists: setting environment variables is only sound single-threaded.
    load_env_file();
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        std::process::exit(if healthy() { 0 } else { 1 });
    }
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(serve())
}

async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let json = std::env::var("LOG_FORMAT").as_deref() == Ok("json");
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    if json {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }

    let config = Config::from_env()?;
    tracing::info!(?config, "starting ouril-relay");
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app(config).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}

/// `KEY=value` lines from the env file, for keys not already set. No shell expansion; one
/// pair of surrounding quotes is removed.
fn load_env_file() {
    let path = std::env::var("RELAY_ENV_FILE").unwrap_or_else(|_| DEFAULT_ENV_FILE.into());
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or(value);
        if std::env::var_os(key).is_none() {
            std::env::set_var(key, value);
        }
    }
}

/// `GET /healthz` on this machine answers `200`.
fn healthy() -> bool {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8090".into());
    let Ok(mut s) = TcpStream::connect(format!("127.0.0.1:{port}")) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(Duration::from_secs(3)));
    if s.write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut buf = [0u8; 16];
    s.read(&mut buf).is_ok() && buf.starts_with(b"HTTP/1.1 200")
}
