use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{auth::AuthenticatedUser, db, error::AppError, state::AppState};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub api_key: String,
    pub user_id: String,
    pub username: String,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub user_id: String,
    pub username: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, AppError> {
    let username = payload.username.trim().to_string();

    if username.is_empty() || username.len() > 50 {
        return Err(AppError::BadRequest(
            "Username must be between 1 and 50 characters".into(),
        ));
    }

    if !username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(AppError::BadRequest(
            "Username may only contain letters, numbers, underscores, and hyphens".into(),
        ));
    }

    if db::username_exists(&state.db, &username)
        .await
        .map_err(AppError::Internal)?
    {
        return Err(AppError::Conflict(format!(
            "Username '{}' is already taken",
            username
        )));
    }

    let (user, key) = db::create_user(&state.db, &username)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(RegisterResponse {
        api_key: key.plaintext,
        user_id: user.id,
        username: user.username,
    }))
}

pub async fn me(user: AuthenticatedUser) -> Result<Json<MeResponse>, AppError> {
    Ok(Json(MeResponse {
        user_id: user.user_id,
        username: user.username,
    }))
}
