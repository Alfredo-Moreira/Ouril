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

/// Check a token: HS256 only, signature (constant time), issuer, audience, subject, expiry
/// (no leeway) and channels. Anything else is refused.
pub fn verify(secret: &str, token: &str) -> Option<RelayClaims> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use hmac::{Hmac, Mac};

    #[derive(Deserialize)]
    struct JwtHeader {
        alg: String,
    }

    let mut parts = token.split('.');
    let (header, payload, signature) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let header: JwtHeader = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(header).ok()?).ok()?;
    if header.alg != "HS256" {
        return None; // never "none", never another algorithm
    }
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(header_and_payload(token)?.as_bytes());
    mac.verify_slice(&URL_SAFE_NO_PAD.decode(signature).ok()?)
        .ok()?;
    let claims: RelayClaims =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()?;
    let valid = claims.iss == RELAY_ISSUER
        && claims.aud == RELAY_AUDIENCE
        && !claims.sub.is_empty()
        && claims.exp > unix_now()
        && !claims.ch.is_empty()
        && claims.ch.len() <= RELAY_MAX_CHANNELS
        && claims.ch.iter().all(|c| is_valid_channel(c));
    valid.then_some(claims)
}

/// `header.payload`, the signed part of a JWT.
fn header_and_payload(token: &str) -> Option<&str> {
    token.rsplit_once('.').map(|(signed, _)| signed)
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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use hmac::{Hmac, Mac};

    const SECRET: &str = "token-secret-for-unit-tests-0123456789abcdef";
    const GAME: &str = "game:01a137aa-33cf-7667-a7bb-d673a70b3b19";

    fn b64(v: &serde_json::Value) -> String {
        URL_SAFE_NO_PAD.encode(v.to_string())
    }

    fn sign(header: &serde_json::Value, claims: &serde_json::Value, secret: &str) -> String {
        let signed = format!("{}.{}", b64(header), b64(claims));
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signed.as_bytes());
        format!(
            "{signed}.{}",
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        )
    }

    fn claims() -> serde_json::Value {
        serde_json::json!({
            "iss": RELAY_ISSUER, "aud": RELAY_AUDIENCE, "sub": "u",
            "iat": unix_now(), "exp": unix_now() + 60, "ch": [GAME]
        })
    }

    #[test]
    fn a_good_token_passes() {
        let t = sign(
            &serde_json::json!({"alg":"HS256","typ":"JWT"}),
            &claims(),
            SECRET,
        );
        assert_eq!(verify(SECRET, &t).unwrap().ch, [GAME]);
    }

    #[test]
    fn unsigned_other_algorithms_and_tampering_are_refused() {
        let none = format!(
            "{}.{}.",
            b64(&serde_json::json!({"alg":"none"})),
            b64(&claims())
        );
        assert!(verify(SECRET, &none).is_none(), "alg none");
        let hs512 = sign(&serde_json::json!({"alg":"HS512"}), &claims(), SECRET);
        assert!(verify(SECRET, &hs512).is_none(), "only HS256");
        let good = sign(&serde_json::json!({"alg":"HS256"}), &claims(), SECRET);
        let mut more = claims();
        more["ch"] = serde_json::json!([GAME, "user:01a137aa-33cb-764c-8eb5-409f541e78da"]);
        let (head, rest) = good.split_once('.').unwrap();
        let sig = rest.split_once('.').unwrap().1;
        let tampered = format!("{head}.{}.{sig}", b64(&more));
        assert!(
            verify(SECRET, &tampered).is_none(),
            "payload changed after signing"
        );
        assert!(verify(SECRET, &format!("{good}.extra")).is_none());
        assert!(verify("another-secret-0123456789abcdefghijklmnop", &good).is_none());
    }
}
