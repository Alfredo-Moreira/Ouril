//! `/v1/auth/*`: sign-in (dev, Google, Apple), refresh and logout.
//!
//! Google and Apple are **placeholders**: 501 `not_configured` until their credentials are set.
//! The dev identity provider exists only in debug builds with the `dev-auth` feature.

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Extension, Json, Router,
};
use ouril_protocol::{
    AppleSignInRequest, GoogleSignInRequest, RefreshRequest, SignInResponse, TokenPair,
};
use uuid::Uuid;

use crate::{
    auth::{
        cookie::{self, ClientKind},
        oidc::VerifyError,
        session::{self, Identity, RotateError},
        tokens::looks_like_refresh_token,
    },
    client::ClientInfo,
    error::{ApiJson, AppError, AppResult},
    state::AppState,
    users, validate,
};

pub fn router() -> Router<AppState> {
    let r = Router::new()
        .route("/auth/google", post(google))
        .route("/auth/apple", post(apple))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout));
    #[cfg(feature = "dev-auth")]
    let r = r.route("/auth/dev", post(dev));
    r
}

fn refresh_max_age(state: &AppState) -> u64 {
    u64::from(state.config.refresh_token_ttl_days) * 86_400
}

/// Build the token pair. Web clients get the refresh token as a cookie, native ones in JSON.
fn tokens_response<T: serde::Serialize>(
    state: &AppState,
    kind: &ClientKind,
    user_id: Uuid,
    session_id: Uuid,
    refresh_token: String,
    body: impl FnOnce(TokenPair) -> T,
) -> AppResult<Response> {
    let access_token = state.tokens.issue(user_id, session_id).map_err(|e| {
        tracing::error!(error = %e, "failed to sign access token");
        AppError::internal()
    })?;
    let pair = TokenPair {
        access_token,
        token_type: "Bearer".into(),
        expires_in: state.tokens.ttl_secs,
        refresh_token: (!kind.is_web()).then(|| refresh_token.clone()),
    };
    let mut res = Json(body(pair)).into_response();
    if let ClientKind::Web { secure } = kind {
        res.headers_mut().append(
            header::SET_COOKIE,
            cookie::set_refresh_cookie(&refresh_token, refresh_max_age(state), *secure),
        );
    }
    Ok(res)
}

/// Shared tail of every sign-in: find/create the user, open a session, answer.
async fn complete_sign_in(
    state: &AppState,
    headers: &HeaderMap,
    client: Option<&ClientInfo>,
    identity: Identity,
) -> AppResult<Response> {
    let kind = cookie::client_kind(&state.config, headers)?;
    let (user_id, is_new_user) = session::find_or_create_user(&state.db, &identity).await?;
    let device = client.map(ClientInfo::label);
    let (session_id, refresh_token) = session::create_session(
        &state.db,
        user_id,
        device.as_deref(),
        state.config.refresh_token_ttl_days,
    )
    .await?;
    let mut conn = state.db.acquire().await?;
    let user = users::load_me(&mut conn, user_id)
        .await?
        .ok_or_else(AppError::internal)?;
    tracing::info!(provider = identity.provider, %user_id, is_new_user, "signed in");
    tokens_response(state, &kind, user_id, session_id, refresh_token, |tokens| {
        SignInResponse {
            tokens,
            user,
            is_new_user,
        }
    })
}

/// `POST /v1/auth/dev`: debug builds with `dev-auth` only (otherwise the route doesn't exist
/// and the fallback answers 404). Signs in as the seeded test user `user` (default `dev`).
#[cfg(feature = "dev-auth")]
async fn dev(
    State(state): State<AppState>,
    headers: HeaderMap,
    client: Option<Extension<ClientInfo>>,
    body: Option<ApiJson<ouril_protocol::DevSignInRequest>>,
) -> AppResult<Response> {
    let req = body.map(|ApiJson(b)| b).unwrap_or_default();
    let user =
        validate::dev_user(req.user.as_deref().unwrap_or("dev")).map_err(AppError::bad_request)?;
    let display_name = match req.display_name.as_deref() {
        Some(n) => validate::display_name(n).map_err(AppError::bad_request)?,
        None => "Dev Player".to_string(),
    };
    let identity = Identity {
        provider: "dev",
        subject: user,
        email: None,
        email_is_private_relay: false,
        display_name: Some(display_name),
    };
    complete_sign_in(&state, &headers, client.as_ref().map(|e| &e.0), identity).await
}

fn verify_error(provider: &str, e: VerifyError) -> AppError {
    match e {
        VerifyError::Invalid(why) => {
            tracing::info!(provider, why, "ID token rejected");
            AppError::unauthorized("ID token verification failed")
        }
        VerifyError::KeysUnavailable => {
            tracing::error!(provider, "provider keys unavailable");
            AppError::provider_unavailable(match provider {
                "google" => "Google sign-in",
                "apple" => "Apple sign-in",
                _ => "Sign-in",
            })
        }
    }
}

/// `POST /v1/auth/google`. Placeholder until `GOOGLE_CLIENT_ID` is set.
async fn google(
    State(state): State<AppState>,
    headers: HeaderMap,
    client: Option<Extension<ClientInfo>>,
    ApiJson(req): ApiJson<GoogleSignInRequest>,
) -> AppResult<Response> {
    let Some(verifier) = state.google.clone() else {
        return Err(AppError::not_configured("Google sign-in"));
    };
    let token = verifier
        .verify(&req.id_token, &req.nonce)
        .await
        .map_err(|e| verify_error("google", e))?;
    let identity = Identity {
        provider: "google",
        subject: token.subject,
        // Keep only verified emails; email is never used to link accounts.
        email: token.email.filter(|_| token.email_verified),
        email_is_private_relay: false,
        display_name: token.name.and_then(|n| validate::display_name(&n).ok()),
    };
    complete_sign_in(&state, &headers, client.as_ref().map(|e| &e.0), identity).await
}

/// `POST /v1/auth/apple`. Placeholder until every `APPLE_*` variable is set.
async fn apple(
    State(state): State<AppState>,
    headers: HeaderMap,
    client: Option<Extension<ClientInfo>>,
    ApiJson(req): ApiJson<AppleSignInRequest>,
) -> AppResult<Response> {
    let Some(verifier) = state.apple.clone() else {
        return Err(AppError::not_configured("Apple sign-in"));
    };
    let token = verifier
        .verify(&req.id_token, &req.nonce)
        .await
        .map_err(|e| verify_error("apple", e))?;
    // Apple sends the name only on the first sign-in, in the request body: save it then.
    let name = [req.given_name.as_deref(), req.family_name.as_deref()]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    // Keep an email only if Apple verified it or it's a private relay address; email is never
    // used to link accounts.
    let email = token
        .email
        .filter(|_| token.email_verified || token.is_private_email);
    let identity = Identity {
        provider: "apple",
        subject: token.subject.clone(),
        email,
        email_is_private_relay: token.is_private_email,
        display_name: validate::display_name(&name).ok(),
    };
    let res = complete_sign_in(&state, &headers, client.as_ref().map(|e| &e.0), identity).await?;
    // Exchange the authorization code for Apple's refresh token, kept only to revoke it on
    // account deletion. A failure doesn't fail the sign-in: it's logged, and deletion then
    // has nothing to revoke for this identity (docs/review/human-review.md).
    if let (Some(code), Some(apple), Some(client_id)) = (
        req.authorization_code
            .as_deref()
            .filter(|c| !c.is_empty() && c.len() <= 2048),
        state.apple_client.as_ref(),
        token.audience.as_deref(),
    ) {
        match apple.exchange_code(client_id, code).await {
            Ok(Some(refresh)) => {
                session::store_provider_token(
                    &state.db,
                    "apple",
                    &token.subject,
                    client_id,
                    &refresh,
                )
                .await?;
            }
            Ok(None) => tracing::warn!("Apple code exchange returned no refresh token"),
            Err(e) => tracing::warn!(error = %e, "Apple code exchange failed"),
        }
    }
    Ok(res)
}

/// Where the refresh token came from. Cookie use requires an allowed `Origin` (CSRF).
fn refresh_token_from(
    state: &AppState,
    headers: &HeaderMap,
    body: Option<RefreshRequest>,
) -> AppResult<(ClientKind, Option<String>)> {
    let kind = cookie::client_kind(&state.config, headers)?;
    let token = match (&kind, body.and_then(|b| b.refresh_token)) {
        (_, Some(t)) => Some(t),
        (ClientKind::Web { .. }, None) => cookie::read_refresh_cookie(headers),
        // A cookie without an allowed Origin is ignored (cross-site or non-browser).
        (ClientKind::Native, None) => None,
    };
    Ok((kind, token))
}

/// `POST /v1/auth/refresh`: rotate the refresh token, return a new access token.
async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Option<ApiJson<RefreshRequest>>,
) -> AppResult<Response> {
    let (kind, token) = refresh_token_from(&state, &headers, body.map(|ApiJson(b)| b))?;
    let token = token
        .filter(|t| looks_like_refresh_token(t))
        .ok_or_else(|| AppError::unauthorized("missing or invalid refresh token"))?;
    match session::rotate_refresh_token(&state.db, &token, state.config.refresh_token_ttl_days)
        .await?
    {
        Ok((user_id, session_id, new_token)) => {
            tokens_response(&state, &kind, user_id, session_id, new_token, |t| t)
        }
        Err(RotateError::Invalid) | Err(RotateError::Reused) => {
            let mut res = AppError::unauthorized("refresh token is invalid, expired or revoked")
                .into_response();
            if let ClientKind::Web { secure } = kind {
                res.headers_mut()
                    .append(header::SET_COOKIE, cookie::clear_refresh_cookie(secure));
            }
            Ok(res)
        }
    }
}

/// `POST /v1/auth/logout`: revoke the session. Always 204 (idempotent), clears the cookie.
async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Option<ApiJson<RefreshRequest>>,
) -> AppResult<Response> {
    let (kind, token) = refresh_token_from(&state, &headers, body.map(|ApiJson(b)| b))?;
    if let Some(token) = token.filter(|t| looks_like_refresh_token(t)) {
        session::revoke_by_refresh_token(&state.db, &token).await?;
    }
    let mut res = StatusCode::NO_CONTENT.into_response();
    if let ClientKind::Web { secure } = kind {
        res.headers_mut()
            .append(header::SET_COOKIE, cookie::clear_refresh_cookie(secure));
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use axum::{http::StatusCode, response::IntoResponse};

    use super::*;

    #[test]
    fn unreachable_provider_keys_are_503_not_500() {
        let res = verify_error("google", VerifyError::KeysUnavailable).into_response();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
        let res = verify_error("apple", VerifyError::Invalid("nonce mismatch")).into_response();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}
