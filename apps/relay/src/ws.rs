//! One player's WebSocket: authenticate with the first message, then receive the events of
//! the token's channels until the token expires, the player leaves, or they misbehave.
//!
//! Close codes: 4001 token expired (fetch a new one and reconnect), 4003 bad or missing
//! token, 1008 a message that isn't allowed, 1001 idle.

use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use ouril_protocol::{
    is_valid_channel, RelayClaims, RELAY_AUDIENCE, RELAY_ISSUER, RELAY_MAX_CHANNELS,
};
use serde::Deserialize;
use tokio::sync::{broadcast, mpsc};

use crate::{limits::Slot, routes::AppState};

/// Largest message a player may send (the token is the biggest).
pub const MAX_MESSAGE_BYTES: usize = 4 * 1024;
/// Events waiting for a slow client before it's dropped (it refetches on reconnect).
const OUTBOX: usize = 64;

pub const CLOSE_TOKEN_EXPIRED: u16 = 4001;
pub const CLOSE_BAD_TOKEN: u16 = 4003;
const CLOSE_POLICY: u16 = 1008;
const CLOSE_GOING_AWAY: u16 = 1001;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Auth {
    token: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ClientMessage {
    Ping,
}

/// Check a token: signature, issuer, audience, expiry and channels.
pub fn verify(secret: &str, token: &str) -> Option<RelayClaims> {
    let mut v = Validation::new(Algorithm::HS256);
    v.set_audience(&[RELAY_AUDIENCE]);
    v.set_issuer(&[RELAY_ISSUER]);
    v.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    v.leeway = 0;
    let claims = decode::<RelayClaims>(token, &DecodingKey::from_secret(secret.as_bytes()), &v)
        .ok()?
        .claims;
    let channels_ok = !claims.ch.is_empty()
        && claims.ch.len() <= RELAY_MAX_CHANNELS
        && claims.ch.iter().all(|c| is_valid_channel(c));
    channels_ok.then_some(claims)
}

async fn close(socket: &mut WebSocket, code: u16, reason: &'static str) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: Utf8Bytes::from_static(reason),
        })))
        .await;
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub async fn serve(mut socket: WebSocket, state: AppState, slot: Slot) {
    let _slot = slot; // freed when this connection ends
    let config = Arc::clone(&state.config);

    // 1. The token, as the first message, in time.
    let first = tokio::time::timeout(config.auth_timeout, socket.recv()).await;
    let claims = match first {
        Ok(Some(Ok(Message::Text(text)))) => serde_json::from_str::<Auth>(&text)
            .ok()
            .and_then(|a| verify(&config.token_secret, &a.token)),
        _ => None,
    };
    let Some(claims) = claims else {
        close(&mut socket, CLOSE_BAD_TOKEN, "token required").await;
        return;
    };

    // 2. Subscribe: each channel forwards into one outbox.
    let (tx, mut outbox) = mpsc::channel::<Arc<str>>(OUTBOX);
    let forwarders: Vec<_> = claims
        .ch
        .iter()
        .map(|ch| {
            let mut rx = state.hub.subscribe(ch);
            let tx = tx.clone();
            tokio::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            if tx.try_send(event).is_err() {
                                break; // slow or gone: the connection ends below
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            })
        })
        .collect();
    drop(tx);
    if socket
        .send(Message::Text(Utf8Bytes::from_static(r#"{"type":"ready"}"#)))
        .await
        .is_err()
    {
        forwarders.iter().for_each(|f| f.abort());
        return;
    }

    // 3. Until the token expires, the player leaves, or something goes wrong.
    let expires_in = Duration::from_secs(claims.exp.saturating_sub(unix_now()));
    let expiry = tokio::time::sleep(expires_in);
    tokio::pin!(expiry);
    let mut ping = tokio::time::interval(config.ping_interval);
    ping.tick().await; // the first tick is immediate
    let mut last_heard = Instant::now();

    loop {
        tokio::select! {
            _ = &mut expiry => {
                close(&mut socket, CLOSE_TOKEN_EXPIRED, "token expired").await;
                break;
            }
            event = outbox.recv() => match event {
                Some(event) => {
                    if socket.send(Message::Text(event.as_ref().into())).await.is_err() {
                        break;
                    }
                }
                // Every forwarder stopped: this client fell too far behind.
                None => {
                    close(&mut socket, CLOSE_POLICY, "too slow").await;
                    break;
                }
            },
            _ = ping.tick() => {
                if last_heard.elapsed() > config.idle_timeout {
                    close(&mut socket, CLOSE_GOING_AWAY, "idle").await;
                    break;
                }
                if socket.send(Message::Ping(Default::default())).await.is_err() {
                    break;
                }
            }
            incoming = socket.recv() => {
                last_heard = Instant::now();
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if matches!(serde_json::from_str::<ClientMessage>(&text), Ok(ClientMessage::Ping)) {
                            let pong = Message::Text(Utf8Bytes::from_static(r#"{"type":"pong"}"#));
                            if socket.send(pong).await.is_err() {
                                break;
                            }
                        } else {
                            close(&mut socket, CLOSE_POLICY, "not allowed").await;
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_) | Message::Ping(_))) => {}
                    Some(Ok(Message::Binary(_))) => {
                        close(&mut socket, CLOSE_POLICY, "not allowed").await;
                        break;
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                }
            }
        }
    }
    forwarders.iter().for_each(|f| f.abort());
}
