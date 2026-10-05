//! `/v1/me`: profile read, update and account deletion (auth required).

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use ouril_protocol::{Me, ProfilePatch};
use time::OffsetDateTime;

use crate::{
    auth::{
        cookie::{self, ClientKind},
        AuthUser,
    },
    error::{ApiJson, AppError, AppResult},
    state::AppState,
    users::{self, ProfileChanges, ProfileError},
    validate,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/me", get(get_me).patch(patch_me).delete(delete_me))
}

async fn get_me(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<Me>> {
    let mut conn = state.db.acquire().await?;
    users::load_me(&mut conn, auth.user_id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::unauthorized("account no longer exists"))
}

fn field(
    name: &str,
    v: Option<String>,
    f: fn(&str) -> Result<String, &'static str>,
) -> AppResult<Option<String>> {
    v.map(|s| f(&s).map_err(|e| AppError::bad_request(format!("{name}: {e}"))))
        .transpose()
}

async fn patch_me(
    State(state): State<AppState>,
    auth: AuthUser,
    ApiJson(patch): ApiJson<ProfilePatch>,
) -> AppResult<Json<Me>> {
    let changes = ProfileChanges {
        display_name: field("display_name", patch.display_name, validate::display_name)?,
        handle: field("handle", patch.handle, validate::handle)?,
        avatar_url: field("avatar_url", patch.avatar_url, validate::avatar_url)?,
        locale: field("locale", patch.locale, validate::locale)?,
        country: field("country", patch.country, validate::country)?,
    };
    let mut tx = state.db.begin().await?;
    // A direct edit happens now: it beats any older change still queued on a device.
    match users::update_profile_at(&mut tx, auth.user_id, &changes, OffsetDateTime::now_utc()).await
    {
        Ok(()) => {}
        Err(ProfileError::HandleTaken) => return Err(AppError::handle_taken()),
        Err(ProfileError::Db(e)) => return Err(e.into()),
    }
    let me = users::load_me(&mut tx, auth.user_id)
        .await?
        .ok_or_else(|| AppError::unauthorized("account no longer exists"))?;
    tx.commit().await?;
    Ok(Json(me))
}

async fn delete_me(
    State(state): State<AppState>,
    headers: HeaderMap,
    auth: AuthUser,
) -> AppResult<Response> {
    // Read Apple's tokens before the identities are deleted, revoke them after (ADR 0009).
    let apple_tokens = users::apple_tokens(&state.db, auth.user_id).await?;
    users::delete_account(&state.db, auth.user_id).await?;
    tracing::info!(user_id = %auth.user_id, "account deleted");
    for (client_id, token) in &apple_tokens {
        match state.apple_client.as_ref() {
            Some(apple) => {
                if let Err(e) = apple.revoke(client_id, token).await {
                    // The account is already deleted; Apple's token expires on its own, but
                    // this needs attention (docs/review/human-review.md).
                    tracing::error!(error = %e, "Apple token revocation failed");
                }
            }
            None => tracing::error!("Apple token not revoked: Apple sign-in is not configured"),
        }
    }
    let mut res = StatusCode::NO_CONTENT.into_response();
    // Clear the web cookie too (it no longer works anyway). Bearer auth already proved the
    // caller, so a missing/unknown Origin just means "not a browser".
    if let Ok(ClientKind::Web { secure }) = cookie::client_kind(&state.config, &headers) {
        res.headers_mut()
            .append(header::SET_COOKIE, cookie::clear_refresh_cookie(secure));
    }
    Ok(res)
}
