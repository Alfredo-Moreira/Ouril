//! The server's side of the live-game relay on the homelab (ADR 0032, 0033).
//!
//! - **Tokens:** players read relay channels with a short-lived HS256 token signed with
//!   `RELAY_TOKEN_SECRET` (never `JWT_SECRET`), listing exactly the channels they may read.
//! - **Publishing:** after a change is committed, the server tells the relay which channel
//!   changed. Events carry only IDs and counts; players fetch the details from this server.
//!   Publishing never blocks or fails a request: it runs in the background, and if the relay
//!   is down, players notice changes by polling instead.

use std::{sync::Arc, time::Duration};

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use ouril_protocol::{
    game_channel, user_channel, LiveToken, OnlineGame, RelayClaims, RelayEvent, RelayPublish,
    RELAY_AUDIENCE, RELAY_ISSUER, RELAY_TOKEN_TTL_SECS,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    config::RelayConfig,
    error::{AppError, AppResult},
};

/// How long a publish may take before it's abandoned.
const PUBLISH_TIMEOUT: Duration = Duration::from_secs(3);

pub struct Relay {
    config: RelayConfig,
    encoding: EncodingKey,
    http: reqwest::Client,
}

impl Relay {
    pub fn new(config: RelayConfig) -> Self {
        let encoding = EncodingKey::from_secret(config.token_secret.as_bytes());
        let http = reqwest::Client::builder()
            .timeout(PUBLISH_TIMEOUT)
            .build()
            .expect("HTTP client");
        Relay {
            config,
            encoding,
            http,
        }
    }

    /// A token for `user` to read `channels`, valid for `RELAY_TOKEN_TTL_SECS`.
    pub fn token(&self, user: Uuid, channels: Vec<String>) -> AppResult<LiveToken> {
        let now = OffsetDateTime::now_utc().unix_timestamp().max(0) as u64;
        let claims = RelayClaims {
            iss: RELAY_ISSUER.into(),
            aud: RELAY_AUDIENCE.into(),
            sub: user.to_string(),
            iat: now,
            exp: now + u64::from(RELAY_TOKEN_TTL_SECS),
            ch: channels,
        };
        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|_| AppError::internal())?;
        Ok(LiveToken {
            token,
            url: self.config.ws_url(),
            expires_in: RELAY_TOKEN_TTL_SECS,
        })
    }

    /// Deliver `event` to `channel`, in the background. Failures are only logged.
    pub fn publish(self: &Arc<Self>, channel: String, event: RelayEvent) {
        let relay = Arc::clone(self);
        tokio::spawn(async move {
            let body = RelayPublish { channel, event };
            let res = relay
                .http
                .post(relay.config.publish_url())
                .bearer_auth(&relay.config.api_key)
                .json(&body)
                .send()
                .await;
            match res {
                Ok(r) if r.status().is_success() => {}
                // Never log the key or the body; the channel and status are enough.
                Ok(r) => tracing::warn!(status = %r.status(), "relay refused a publish"),
                Err(e) => tracing::warn!(error = %e.without_url(), "relay unreachable"),
            }
        });
    }
}

/// Tell both players a game changed: its channel (for whoever has it open) and the
/// opponent's own channel (their games list).
pub fn game_changed(relay: Option<&Arc<Relay>>, game: &OnlineGame) {
    let Some(relay) = relay else { return };
    relay.publish(
        game_channel(&game.id),
        RelayEvent::GameChanged {
            game_id: game.id.clone(),
            moves: game.moves.len() as u32,
        },
    );
    if let Some(opponent) = &game.opponent {
        relay.publish(user_channel(&opponent.user_id), RelayEvent::GamesChanged);
    }
}

/// Tell a waiting player their seek was paired.
pub fn matched(relay: Option<&Arc<Relay>>, user: Uuid, game: Uuid) {
    if let Some(relay) = relay {
        relay.publish(
            user_channel(&user.to_string()),
            RelayEvent::Matched {
                game_id: game.to_string(),
            },
        );
    }
}
