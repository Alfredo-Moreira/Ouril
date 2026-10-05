//! Sign in with Apple REST calls (ADR 0009): exchanging an authorization code for Apple's
//! refresh token at sign-in, and **revoking** that token when the account is deleted (Apple
//! requires apps that offer account deletion to revoke the user's tokens).
//!
//! Every call authenticates with a short-lived *client secret*: an ES256 JWT signed with the
//! `.p8` key (`APPLE_PRIVATE_KEY_PATH`), issued by the team (`APPLE_TEAM_ID`) for the client the
//! token belongs to: the Services ID for the web, the bundle ID for iOS.
//!
//! **Placeholder status:** only built when every `APPLE_*` variable is set, and not exercised
//! against Apple yet. Tests run against a local mock of the endpoints.

use std::time::Duration;

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::config::AppleConfig;

/// Apple's REST API base URL.
pub const APPLE_BASE_URL: &str = "https://appleid.apple.com";
/// Client secrets live 5 minutes (Apple allows up to 6 months).
const CLIENT_SECRET_TTL_SECS: i64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppleError {
    /// The private key couldn't be read or parsed (configuration problem).
    Key(String),
    /// Apple couldn't be reached, or answered with an error.
    Request(String),
}

impl std::fmt::Display for AppleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppleError::Key(e) => write!(f, "Apple key: {e}"),
            AppleError::Request(e) => write!(f, "Apple request: {e}"),
        }
    }
}

#[derive(Serialize)]
struct ClientSecretClaims<'a> {
    iss: &'a str,
    iat: i64,
    exp: i64,
    aud: &'a str,
    sub: &'a str,
}

#[derive(Deserialize)]
struct TokenResponse {
    #[serde(default)]
    refresh_token: Option<String>,
}

pub struct AppleClient {
    team_id: String,
    key_id: String,
    key: EncodingKey,
    redirect_uri: Option<String>,
    base_url: String,
    http: reqwest::Client,
}

impl AppleClient {
    /// Build from config, reading the `.p8` key. Fails closed: callers disable Apple sign-in
    /// when this fails, so tokens are never issued that couldn't later be revoked.
    pub fn from_config(cfg: &AppleConfig) -> Result<Self, AppleError> {
        let pem = std::fs::read(&cfg.private_key_path)
            .map_err(|e| AppleError::Key(format!("can't read the key file: {e}")))?;
        Self::new(
            cfg.team_id.clone(),
            cfg.key_id.clone(),
            &pem,
            cfg.redirect_uri.clone(),
            APPLE_BASE_URL.into(),
        )
    }

    pub fn new(
        team_id: String,
        key_id: String,
        pem: &[u8],
        redirect_uri: Option<String>,
        base_url: String,
    ) -> Result<Self, AppleError> {
        let key = EncodingKey::from_ec_pem(pem)
            .map_err(|e| AppleError::Key(format!("not an EC (P-256) private key: {e}")))?;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| AppleError::Request(e.to_string()))?;
        Ok(AppleClient {
            team_id,
            key_id,
            key,
            redirect_uri,
            base_url: base_url.trim_end_matches('/').to_string(),
            http,
        })
    }

    /// The ES256 client secret for `client_id` (Services ID or bundle ID).
    pub fn client_secret(
        &self,
        client_id: &str,
        now: OffsetDateTime,
    ) -> Result<String, AppleError> {
        let iat = now.unix_timestamp();
        let claims = ClientSecretClaims {
            iss: &self.team_id,
            iat,
            exp: iat + CLIENT_SECRET_TTL_SECS,
            aud: APPLE_BASE_URL,
            sub: client_id,
        };
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.key_id.clone());
        encode(&header, &claims, &self.key).map_err(|e| AppleError::Key(e.to_string()))
    }

    /// Exchange an authorization code (from the sign-in response) for Apple's refresh token.
    pub async fn exchange_code(
        &self,
        client_id: &str,
        code: &str,
    ) -> Result<Option<String>, AppleError> {
        let secret = self.client_secret(client_id, OffsetDateTime::now_utc())?;
        let mut form = vec![
            ("client_id", client_id),
            ("client_secret", secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
        ];
        if let Some(uri) = &self.redirect_uri {
            form.push(("redirect_uri", uri.as_str()));
        }
        let res = self.post("/auth/token", &form).await?;
        let body: TokenResponse = res
            .json()
            .await
            .map_err(|e| AppleError::Request(format!("bad token response: {e}")))?;
        Ok(body.refresh_token)
    }

    /// Revoke a refresh token (account deletion).
    pub async fn revoke(&self, client_id: &str, refresh_token: &str) -> Result<(), AppleError> {
        let secret = self.client_secret(client_id, OffsetDateTime::now_utc())?;
        self.post(
            "/auth/revoke",
            &[
                ("client_id", client_id),
                ("client_secret", secret.as_str()),
                ("token", refresh_token),
                ("token_type_hint", "refresh_token"),
            ],
        )
        .await?;
        Ok(())
    }

    async fn post(
        &self,
        path: &str,
        form: &[(&str, &str)],
    ) -> Result<reqwest::Response, AppleError> {
        let res = self
            .http
            .post(format!("{}{path}", self.base_url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_encode(form))
            .send()
            .await
            .map_err(|e| AppleError::Request(e.to_string()))?;
        if !res.status().is_success() {
            return Err(AppleError::Request(format!(
                "{path} answered {}",
                res.status()
            )));
        }
        Ok(res)
    }
}

/// `application/x-www-form-urlencoded` body.
fn form_encode(pairs: &[(&str, &str)]) -> String {
    fn enc(s: &str) -> String {
        s.bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    (b as char).to_string()
                }
                b' ' => "+".to_string(),
                _ => format!("%{b:02X}"),
            })
            .collect()
    }
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", enc(k), enc(v)))
        .collect::<Vec<_>>()
        .join("&")
}

#[cfg(test)]
mod tests {
    //! The EC key in `tests/fixtures/apple_test_only_ec.pem` was generated for these tests and
    //! is never used anywhere else. Apple's endpoints are mocked with a local axum server.

    use std::sync::{Arc, Mutex};

    use axum::{extract::State, routing::post, Json, Router};
    use jsonwebtoken::{decode, DecodingKey, Validation};
    use serde_json::{json, Value};

    use super::*;

    const KEY: &[u8] = include_bytes!("../../tests/fixtures/apple_test_only_ec.pem");
    const PUB: &[u8] = include_bytes!("../../tests/fixtures/apple_test_only_ec.pub.pem");

    fn client(base_url: String) -> AppleClient {
        AppleClient::new(
            "TEAM123456".into(),
            "KEY1234567".into(),
            KEY,
            Some("https://ouril.example/auth/apple".into()),
            base_url,
        )
        .unwrap()
    }

    #[test]
    fn client_secret_is_an_es256_jwt_for_the_client() {
        let c = client(APPLE_BASE_URL.into());
        let now = OffsetDateTime::now_utc();
        let secret = c.client_secret("com.example.ouril.web", now).unwrap();
        let header = jsonwebtoken::decode_header(&secret).unwrap();
        assert_eq!(header.alg, Algorithm::ES256);
        assert_eq!(header.kid.as_deref(), Some("KEY1234567"));
        let mut v = Validation::new(Algorithm::ES256);
        v.set_audience(&[APPLE_BASE_URL]);
        v.set_issuer(&["TEAM123456"]);
        let claims = decode::<Value>(&secret, &DecodingKey::from_ec_pem(PUB).unwrap(), &v)
            .unwrap()
            .claims;
        assert_eq!(claims["sub"], "com.example.ouril.web");
        assert_eq!(
            claims["exp"].as_i64().unwrap() - claims["iat"].as_i64().unwrap(),
            CLIENT_SECRET_TTL_SECS
        );
    }

    #[test]
    fn rejects_a_key_that_is_not_ec() {
        let rsa = include_bytes!("../../tests/fixtures/oidc_test_only_rsa.pem");
        let res = AppleClient::new("T".into(), "K".into(), rsa, None, APPLE_BASE_URL.into());
        assert!(matches!(res, Err(AppleError::Key(_))));
    }

    #[test]
    fn form_encoding() {
        assert_eq!(
            form_encode(&[("a b", "x/y=z&w"), ("k", "~ok-1._")]),
            "a+b=x%2Fy%3Dz%26w&k=~ok-1._"
        );
    }

    type Seen = Arc<Mutex<Vec<(String, String)>>>;

    async fn mock_apple() -> (String, Seen) {
        let seen: Seen = Arc::default();
        async fn record(State(seen): State<Seen>, path: &'static str, body: String) {
            seen.lock().unwrap().push((path.to_string(), body));
        }
        let app = Router::new()
            .route(
                "/auth/token",
                post(|s: State<Seen>, body: String| async move {
                    record(s, "/auth/token", body).await;
                    Json(json!({"access_token": "a", "refresh_token": "apple-refresh-1"}))
                }),
            )
            .route(
                "/auth/revoke",
                post(|s: State<Seen>, body: String| async move {
                    record(s, "/auth/revoke", body).await;
                }),
            )
            .with_state(seen.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (format!("http://{addr}"), seen)
    }

    #[tokio::test]
    async fn exchanges_the_code_then_revokes_the_refresh_token() {
        crate::install_crypto_provider();
        let (base, seen) = mock_apple().await;
        let c = client(base);
        let token = c
            .exchange_code("com.example.ouril.web", "auth-code")
            .await
            .unwrap();
        assert_eq!(token.as_deref(), Some("apple-refresh-1"));
        c.revoke("com.example.ouril.web", "apple-refresh-1")
            .await
            .unwrap();

        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        let (path, body) = &seen[0];
        assert_eq!(path, "/auth/token");
        assert!(body.contains("grant_type=authorization_code"));
        assert!(body.contains("code=auth-code"));
        assert!(body.contains("client_id=com.example.ouril.web"));
        assert!(body.contains("redirect_uri=https%3A%2F%2Fouril.example%2Fauth%2Fapple"));
        let (path, body) = &seen[1];
        assert_eq!(path, "/auth/revoke");
        assert!(body.contains("token=apple-refresh-1"));
        assert!(body.contains("token_type_hint=refresh_token"));
        assert!(body.contains("client_secret="));
    }

    #[tokio::test]
    async fn errors_are_reported_not_hidden() {
        crate::install_crypto_provider();
        // Nothing listens here.
        let c = client("http://127.0.0.1:9".into());
        assert!(matches!(
            c.revoke("com.example.ouril.web", "t").await,
            Err(AppleError::Request(_))
        ));
    }
}
