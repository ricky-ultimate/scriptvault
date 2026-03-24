use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::Value;

use crate::{auth::AuthenticatedUser, error::AppError, state::AppState};

pub async fn list_scripts(
    user: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, AppError> {
    let metas = state.r2.list_script_metas(&user.user_id).await?;
    Ok(Json(metas))
}

pub async fn get_script(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(script_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let (script, etag) = state.r2.get_script(&user.user_id, &script_id).await?;
    let mut headers = HeaderMap::new();
    headers.insert("ETag", HeaderValue::from_str(&format!("\"{}\"", etag)).unwrap());
    Ok((StatusCode::OK, headers, Json(script)))
}

pub async fn put_script(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(script_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, AppError> {
    let if_match = headers
        .get("If-Match")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim_matches('"').to_string());

    let etag = state
        .r2
        .put_script(&user.user_id, &script_id, &payload, if_match.as_deref())
        .await?;

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert("ETag", HeaderValue::from_str(&format!("\"{}\"", etag)).unwrap());
    Ok((StatusCode::OK, resp_headers))
}

pub async fn delete_script(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(script_id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.r2.delete_script(&user.user_id, &script_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
