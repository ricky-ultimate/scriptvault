use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use serde_json::Value;

use crate::{auth::AuthenticatedUser, db, error::AppError, state::AppState};

const MAX_SCRIPT_NAME_LEN: usize = 100;
const MAX_SCRIPT_VERSION_LEN: usize = 50;
const MAX_TAG_COUNT: usize = 20;
const MAX_TAG_LEN: usize = 50;
const MAX_DESCRIPTION_LEN: usize = 500;
const MAX_CONTENT_BYTES: usize = 1_048_576;

fn validate_script_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::BadRequest("name must not be empty".into()));
    }
    if name.len() > MAX_SCRIPT_NAME_LEN {
        return Err(AppError::BadRequest(format!(
            "name exceeds maximum length of {}",
            MAX_SCRIPT_NAME_LEN
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(AppError::BadRequest(
            "name may only contain letters, numbers, hyphens, underscores, and dots".into(),
        ));
    }
    Ok(())
}

fn validate_script_version(version: &str) -> Result<(), AppError> {
    if version.is_empty() {
        return Err(AppError::BadRequest("version must not be empty".into()));
    }
    if version.len() > MAX_SCRIPT_VERSION_LEN {
        return Err(AppError::BadRequest(format!(
            "version exceeds maximum length of {}",
            MAX_SCRIPT_VERSION_LEN
        )));
    }
    Ok(())
}

fn validate_tags(tags: &[String]) -> Result<(), AppError> {
    if tags.len() > MAX_TAG_COUNT {
        return Err(AppError::BadRequest(format!(
            "too many tags: maximum is {}",
            MAX_TAG_COUNT
        )));
    }
    for tag in tags {
        if tag.len() > MAX_TAG_LEN {
            return Err(AppError::BadRequest(format!(
                "tag '{}' exceeds maximum length of {}",
                tag, MAX_TAG_LEN
            )));
        }
        if tag.is_empty() {
            return Err(AppError::BadRequest(
                "tags must not be empty strings".into(),
            ));
        }
    }
    Ok(())
}

fn validate_payload_size(payload: &Value) -> Result<(), AppError> {
    let estimated = serde_json::to_vec(payload)
        .map(|b| b.len())
        .unwrap_or(usize::MAX);
    if estimated > MAX_CONTENT_BYTES {
        return Err(AppError::BadRequest(format!(
            "payload exceeds maximum size of {} bytes",
            MAX_CONTENT_BYTES
        )));
    }
    Ok(())
}

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
                "tags": m.tags,
                "description": m.description,
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
    validate_payload_size(&payload)?;

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
    let tags: Vec<String> = payload
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let description = payload
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    validate_script_name(&name)?;
    validate_script_version(&version)?;
    validate_tags(&tags)?;

    if hash.is_empty() {
        return Err(AppError::BadRequest("missing metadata.hash field".into()));
    }

    if let Some(ref desc) = description {
        if desc.len() > MAX_DESCRIPTION_LEN {
            return Err(AppError::BadRequest(format!(
                "description exceeds maximum length of {}",
                MAX_DESCRIPTION_LEN
            )));
        }
    }

    let if_match_raw = headers
        .get("If-Match")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim_matches('"').to_string());

    let already_exists = db::script_meta_exists(&state.db, &user.user_id, &script_id)
        .await
        .map_err(AppError::Internal)?;

    let effective_if_match = if if_match_raw.is_some() && !already_exists {
        None
    } else {
        if_match_raw.as_deref()
    };

    db::upsert_script_meta(
        &state.db,
        &user.user_id,
        &script_id,
        &name,
        &version,
        &hash,
        updated_at,
        &tags,
        description.as_deref(),
    )
    .await
    .map_err(AppError::Internal)?;

    let etag = match state
        .r2
        .put_script(&user.user_id, &script_id, &payload, effective_if_match)
        .await
    {
        Ok(e) => e,
        Err(e) if e.to_string() == "etag_mismatch" => {
            if let Err(rollback_err) =
                db::delete_script_meta(&state.db, &user.user_id, &script_id).await
            {
                tracing::error!(
                    script_id = %script_id,
                    user_id = %user.user_id,
                    rollback_err = %rollback_err,
                    "etag mismatch on R2 write; Postgres rollback also failed — manual cleanup required"
                );
            }
            return Err(AppError::PreconditionFailed);
        }
        Err(e) => {
            if let Err(rollback_err) =
                db::delete_script_meta(&state.db, &user.user_id, &script_id).await
            {
                tracing::error!(
                    script_id = %script_id,
                    user_id = %user.user_id,
                    r2_err = %e,
                    rollback_err = %rollback_err,
                    "R2 write failed; Postgres rollback also failed — manual cleanup required"
                );
            }
            return Err(AppError::Internal(e));
        }
    };

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
