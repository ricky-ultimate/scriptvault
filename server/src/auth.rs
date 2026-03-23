use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::{db, error::AppError, state::AppState};

pub struct AuthenticatedUser {
    pub user_id: String,
    pub username: String,
}

fn extract_bearer_token(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t.to_string())
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_bearer_token(parts).ok_or(AppError::Unauthorized)?;
        let key_hash = db::hash_key(&token);

        let (user, key_id) = db::find_user_by_key_hash(&state.db, &key_hash)
            .await
            .map_err(AppError::Internal)?
            .ok_or(AppError::Unauthorized)?;

        db::update_key_last_used(&state.db, &key_id)
            .await
            .map_err(AppError::Internal)?;

        Ok(AuthenticatedUser {
            user_id: user.id,
            username: user.username,
        })
    }
}
