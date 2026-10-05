use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    auth::{apple::AppleClient, oidc::OidcVerifier, tokens::AccessTokens},
    config::Config,
    ratelimit::RateLimiter,
};

/// Shared application state, cheap to clone.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: PgPool,
    pub tokens: Arc<AccessTokens>,
    pub rate_limiter: Arc<RateLimiter>,
    /// `None` until `GOOGLE_CLIENT_ID` is set.
    pub google: Option<Arc<OidcVerifier>>,
    /// `None` until every `APPLE_*` variable is set (and the key loads).
    pub apple: Option<Arc<OidcVerifier>>,
    /// Apple code exchange and token revocation. Present exactly when `apple` is.
    pub apple_client: Option<Arc<AppleClient>>,
}

impl AppState {
    pub fn new(config: Config, db: PgPool) -> Self {
        crate::install_crypto_provider();
        let tokens = AccessTokens::new(&config.jwt_secret, config.access_token_ttl_secs);
        let google = config
            .google_client_id
            .as_ref()
            .map(|id| Arc::new(OidcVerifier::google(id.clone())));
        // Apple sign-in needs a working key for token revocation (account deletion). If the key
        // can't be loaded, keep Apple sign-in off (501) rather than issue tokens we couldn't
        // revoke.
        let (apple, apple_client) = match config.apple.as_ref() {
            Some(a) => match AppleClient::from_config(a) {
                Ok(client) => (
                    Some(Arc::new(OidcVerifier::apple(
                        a.service_id.clone(),
                        a.bundle_id.clone(),
                    ))),
                    Some(Arc::new(client)),
                ),
                Err(e) => {
                    tracing::error!(error = %e, "Apple sign-in disabled: key unusable");
                    (None, None)
                }
            },
            None => (None, None),
        };
        let rate_limiter = Arc::new(RateLimiter::new(
            config.rate_limit_auth_per_minute,
            config.rate_limit_default_per_minute,
        ));
        AppState {
            config: Arc::new(config),
            db,
            tokens: Arc::new(tokens),
            rate_limiter,
            google,
            apple,
            apple_client,
        }
    }
}
