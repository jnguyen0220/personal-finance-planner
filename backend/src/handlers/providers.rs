use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::handlers::delete_by_id;
use crate::models::{Provider, ProviderInput};
use crate::state::AppState;

const COLUMNS: &str = "id, property_id, kind, name, phone, homepage, created_at";

pub async fn list_for_property(
    State(st): State<AppState>,
    Path(property_id): Path<String>,
) -> AppResult<Json<Vec<Provider>>> {
    let rows = crate::db::fetch_all(
        &st.pool,
        sqlx::query_as::<_, Provider>(&format!(
            "SELECT {COLUMNS} FROM providers WHERE property_id = ? ORDER BY kind, name"
        ))
        .bind(&property_id),
    )
    .await?;
    Ok(Json(rows))
}

pub async fn create(
    State(st): State<AppState>,
    Path(property_id): Path<String>,
    Json(input): Json<ProviderInput>,
) -> AppResult<Json<Provider>> {
    if input.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let row = crate::db::fetch_one(
        &st.pool,
        sqlx::query_as::<_, Provider>(&format!(
            "INSERT INTO providers (id, property_id, kind, name, phone, homepage, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING {COLUMNS}"
        ))
        .bind(&id)
        .bind(&property_id)
        .bind(&input.kind)
        .bind(input.name.trim())
        .bind(&input.phone)
        .bind(&input.homepage)
        .bind(&now),
    )
    .await?;
    Ok(Json(row))
}

pub async fn update(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<ProviderInput>,
) -> AppResult<Json<Provider>> {
    if input.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let row = crate::db::fetch_optional(
        &st.pool,
        sqlx::query_as::<_, Provider>(&format!(
            "UPDATE providers SET kind = ?, name = ?, phone = ?, homepage = ? WHERE id = ? \
             RETURNING {COLUMNS}"
        ))
        .bind(&input.kind)
        .bind(input.name.trim())
        .bind(&input.phone)
        .bind(&input.homepage)
        .bind(&id),
    )
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

pub async fn delete(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<axum::http::StatusCode> {
    delete_by_id(&st, "providers", &id).await
}
