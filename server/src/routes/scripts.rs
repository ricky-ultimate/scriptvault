use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use serde_json::Value;

use crate::{auth::AuthenticatedUser, db, error::AppError, state::AppState};

pub async fn list_scripts(
    user: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, AppError> {
    let metas = db::list_script_meta(&state.db, &user.user_id)
        .await
        .map_err(AppError::Internal)?;

    let values = metas
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "name": m.name,
                "version": m.version,
                "hash": m.hash,
                "updated_at": m.updated_at,
            })
        })
        .collect();

    Ok(Json(values))
}

pub async fn get_script(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(script_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let exists = db::script_meta_exists(&state.db, &user.user_id, &script_id)
        .await
        .map_err(AppError::Internal)?;

    if !exists {
        return Err(AppError::NotFound);
    }

    let (script, etag) = state
        .r2
        .get_script(&user.user_id, &script_id)
        .await
        .map_err(|e| {
            if e.to_string().contains("script not found") {
                AppError::NotFound
            } else {
                AppError::Internal(e)
            }
        })?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "ETag",
        HeaderValue::from_str(&format!("\"{}\"", etag)).unwrap(),
    );
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

    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let version = payload
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let hash = payload
        .get("metadata")
        .and_then(|m| m.get("hash"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let updated_at = payload
        .get("updated_at")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok())
        .unwrap_or_else(chrono::Utc::now);

    if name.is_empty() {
        return Err(AppError::BadRequest("missing name field".into()));
    }
    if version.is_empty() {
        return Err(AppError::BadRequest("missing version field".into()));
    }
    if hash.is_empty() {
        return Err(AppError::BadRequest("missing metadata.hash field".into()));
    }

    let etag = match state
        .r2
        .put_script(&user.user_id, &script_id, &payload, if_match.as_deref())
        .await
    {
        Ok(e) => e,
        Err(e) if e.to_string() == "etag_mismatch" => return Err(AppError::PreconditionFailed),
        Err(e) => return Err(AppError::Internal(e)),
    };

    if let Err(e) = db::upsert_script_meta(
        &state.db,
        &user.user_id,
        &script_id,
        &name,
        &version,
        &hash,
        updated_at,
    )
    .await
    {
        if let Err(r2_err) = state.r2.delete_script(&user.user_id, &script_id).await {
            tracing::error!(
                script_id = %script_id,
                user_id = %user.user_id,
                r2_err = %r2_err,
                "R2 write succeeded but Postgres failed and R2 rollback also failed"
            );
        }
        return Err(AppError::Internal(e));
    }

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "ETag",
        HeaderValue::from_str(&format!("\"{}\"", etag)).unwrap(),
    );
    Ok((StatusCode::OK, resp_headers))
}

pub async fn delete_script(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(script_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let exists = db::script_meta_exists(&state.db, &user.user_id, &script_id)
        .await
        .map_err(AppError::Internal)?;

    if !exists {
        return Err(AppError::NotFound);
    }

    state
        .r2
        .delete_script(&user.user_id, &script_id)
        .await
        .map_err(AppError::Internal)?;

    db::delete_script_meta(&state.db, &user.user_id, &script_id)
        .await
        .map_err(AppError::Internal)?;

    Ok(StatusCode::NO_CONTENT)
}
