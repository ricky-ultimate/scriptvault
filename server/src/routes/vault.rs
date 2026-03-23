use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::Value;

use crate::{auth::AuthenticatedUser, error::AppError, state::AppState};

pub async fn get_vault(
    user: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let vault = state.r2.get_vault(&user.user_id).await?;
    Ok(Json(vault))
}

pub async fn put_vault(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, AppError> {
    state.r2.put_vault(&user.user_id, &payload).await?;
    Ok(StatusCode::OK)
}
