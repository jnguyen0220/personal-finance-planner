use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::categories::{canonical_kind, is_deductible};
use crate::error::{AppError, AppResult};
use crate::handlers::delete_by_id;
use crate::models::{Transaction, TransactionInput};
use crate::state::AppState;

const COLUMNS: &str =
    "id, property_id, kind, category, amount, date, description, tenant_name, borne_by, receipt_id, created_at";

pub async fn list_for_property(
    State(st): State<AppState>,
    Path(property_id): Path<String>,
) -> AppResult<Json<Vec<Transaction>>> {
    let rows = sqlx::query_as::<_, Transaction>(&format!(
        "SELECT {COLUMNS} FROM transactions WHERE property_id = ? ORDER BY date DESC, created_at DESC"
    ))
    .bind(&property_id)
    .fetch_all(&st.pool)
    .await?;
    Ok(Json(rows))
}

/// Standard categories dictate their own income/expense kind, so the client
/// value is only trusted for unknown (legacy) categories.
fn resolve_kind(category: &str, property_kind: &str, requested: &str) -> AppResult<String> {
    match canonical_kind(category, property_kind) {
        Some(k) => Ok(k.to_string()),
        None if requested == "income" || requested == "expense" => Ok(requested.to_string()),
        None => Err(AppError::BadRequest(
            "kind must be 'income' or 'expense'".into(),
        )),
    }
}

/// Only categories the backend marks deductible can be borne by the tenant (a
/// credit against rent owed); every other transaction is attributed to the landlord.
fn resolve_borne_by(category: &str, property_kind: &str, requested: &str) -> String {
    if requested == "tenant" && is_deductible(category, property_kind) {
        "tenant".to_string()
    } else {
        "landlord".to_string()
    }
}

pub async fn create(
    State(st): State<AppState>,
    Path(property_id): Path<String>,
    Json(input): Json<TransactionInput>,
) -> AppResult<Json<Transaction>> {
    if input.date.trim().is_empty() {
        return Err(AppError::BadRequest("date is required".into()));
    }
    let property_kind = sqlx::query_scalar::<_, String>("SELECT kind FROM properties WHERE id = ?")
        .bind(&property_id)
        .fetch_optional(&st.pool)
        .await?
        .ok_or(AppError::NotFound)?;
    let kind = resolve_kind(&input.category, &property_kind, &input.kind)?;
    let borne_by = resolve_borne_by(&input.category, &property_kind, &input.borne_by);

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let row = sqlx::query_as::<_, Transaction>(&format!(
        "INSERT INTO transactions (id, property_id, kind, category, amount, date, description, tenant_name, borne_by, receipt_id, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING {COLUMNS}"
    ))
    .bind(&id)
    .bind(&property_id)
    .bind(&kind)
    .bind(&input.category)
    .bind(input.amount)
    .bind(&input.date)
    .bind(&input.description)
    .bind(&input.tenant_name)
    .bind(&borne_by)
    .bind(&input.receipt_id)
    .bind(&now)
    .fetch_one(&st.pool)
    .await?;
    Ok(Json(row))
}

pub async fn update(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<TransactionInput>,
) -> AppResult<Json<Transaction>> {
    let property_kind = sqlx::query_scalar::<_, String>(
        "SELECT p.kind FROM transactions t JOIN properties p ON p.id = t.property_id WHERE t.id = ?",
    )
    .bind(&id)
    .fetch_optional(&st.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let kind = resolve_kind(&input.category, &property_kind, &input.kind)?;
    let borne_by = resolve_borne_by(&input.category, &property_kind, &input.borne_by);

    let row = sqlx::query_as::<_, Transaction>(&format!(
        "UPDATE transactions SET kind = ?, category = ?, amount = ?, date = ?, description = ?, tenant_name = ?, borne_by = ?, receipt_id = ? \
         WHERE id = ? RETURNING {COLUMNS}"
    ))
    .bind(&kind)
    .bind(&input.category)
    .bind(input.amount)
    .bind(&input.date)
    .bind(&input.description)
    .bind(&input.tenant_name)
    .bind(&borne_by)
    .bind(&input.receipt_id)
    .bind(&id)
    .fetch_optional(&st.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

pub async fn delete(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<axum::http::StatusCode> {
    delete_by_id(&st, "transactions", &id).await
}
