//! `ouril-relay`: the live-game relay on the homelab (ADR 0032, 0033).
//!
//! It only fans out the server's events to players: it validates no moves, stores nothing,
//! and events carry only IDs and counts ([`ouril_protocol::RelayEvent`]). It assumes the
//! internet is hostile (it runs on a home network):
//!
//! - **Three paths only:** `GET /ws` (players), `POST /publish` (the server), `GET /healthz`.
//! - **Players need a token** from the server: HS256 with `RELAY_TOKEN_SECRET`, audience
//!   `ouril-relay`, listing the channels they may read. It's the first WebSocket message
//!   (never in the URL, so it can't land in proxy logs), within `auth_timeout`. The
//!   connection closes when the token expires.
//! - **Players can't publish:** after the token, only `{"type":"ping"}` is accepted; anything
//!   else closes the connection.
//! - **The server publishes with a key** (`Authorization: Bearer`, compared in constant time).
//! - **Limits:** allowed origins, total and per-IP connections, upgrades per IP per minute,
//!   small frames and bodies, heartbeats and an idle timeout.

pub mod config;
mod hub;
mod limits;
mod routes;
mod ws;

pub use config::Config;
pub use routes::app;
