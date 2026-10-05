//! `AuthUser`: the extractor for endpoints that need `Authorization: Bearer <access token>`.

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use uuid::Uuid;

use super::session::session_active;
use crate::{error::AppError, state::AppState};

/// The signed-in caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub session_id: Uuid,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| {
                let (scheme, token) = v.split_once(' ')?;
                scheme.eq_ignore_ascii_case("bearer").then(|| token.trim())
            })
            .ok_or_else(|| AppError::unauthorized("missing bearer token"))?;
        let (user_id, session_id) = state
            .tokens
            .verify(token)
            .ok_or_else(|| AppError::unauthorized("invalid or expired access token"))?;
        if !session_active(&state.db, user_id, session_id).await? {
            return Err(AppError::unauthorized("session is no longer valid"));
        }
        tracing::Span::current().record("user_id", tracing::field::display(user_id));
        Ok(AuthUser {
            user_id,
            session_id,
        })
    }
}
