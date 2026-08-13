use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::Json;
use serde::Deserialize;

use crate::categories::{self, Category};
use crate::error::{AppError, AppResult};
use crate::etag;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CategoriesQuery {
    pub kind: Option<String>,
}

/// Categories resolved for a property kind, used to drive the transaction forms.
pub async fn list(
    State(st): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CategoriesQuery>,
) -> AppResult<Response> {
    let kind = q.kind.as_deref().unwrap_or("rental");
    let resolved = categories::resolved_for(&st.pool, kind).await?;
    let body = serde_json::to_string(&resolved).expect("serialize categories");
    Ok(etag::respond_json(&headers, body))
}

/// Every category with its raw, editable attributes, for the Admin page.
pub async fn list_all(State(st): State<AppState>) -> AppResult<Json<Vec<Category>>> {
    Ok(Json(categories::all(&st.pool).await?))
}

fn normalize(mut c: Category) -> AppResult<Category> {
    c.id = c.id.trim().to_string();
    c.label = c.label.trim().to_string();
    if c.id.is_empty() {
        return Err(AppError::BadRequest("id is required".into()));
    }
    if c.label.is_empty() {
        return Err(AppError::BadRequest("label is required".into()));
    }
    if c.kind != "income" && c.kind != "expense" {
        return Err(AppError::BadRequest(
            "kind must be 'income' or 'expense'".into(),
        ));
    }
    c.parent_id = c
        .parent_id
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty());
    c.fields = c
        .fields
        .into_iter()
        .map(|f| f.trim().to_string())
        .filter(|f| !f.is_empty())
        .collect();
    Ok(c)
}

pub async fn create(
    State(st): State<AppState>,
    Json(input): Json<Category>,
) -> AppResult<Json<Category>> {
    let category = normalize(input)?;
    if categories::exists(&st.pool, &category.id).await? {
        return Err(AppError::BadRequest(
            "a category with that id already exists".into(),
        ));
    }
    if let Some(parent) = &category.parent_id {
        if !categories::exists(&st.pool, parent).await? {
            return Err(AppError::BadRequest("parent category does not exist".into()));
        }
    }
    Ok(Json(categories::insert(&st.pool, &category).await?))
}

pub async fn update(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<Category>,
) -> AppResult<Json<Category>> {
    let category = normalize(input)?;
    if categories::update(&st.pool, &id, &category).await? == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(category))
}

pub async fn delete(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<axum::http::StatusCode> {
    let in_use = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM transactions WHERE category_id = ? LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&st.pool)
    .await?
    .is_some();
    if in_use {
        return Err(AppError::BadRequest(
            "category is in use by transactions and can't be deleted".into(),
        ));
    }
    if categories::remove(&st.pool, &id).await? == 0 {
        return Err(AppError::NotFound);
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}
