use axum::extract::{Path, State};
use axum::Json;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::categories;
use crate::error::{AppError, AppResult};
use crate::handlers::{delete_attachment, delete_by_id};
use crate::models::{Transaction, TransactionInput};
use crate::state::AppState;

/// Reads join the category so every transaction carries its display label.
const SELECT: &str =
    "SELECT t.id, t.property_id, t.kind, t.category_id, c.label AS category_label, \
     t.amount, t.date, t.description, t.tenant_name, t.borne_by, t.receipt_id, t.created_at \
     FROM transactions t JOIN categories c ON c.id = t.category_id";

async fn fetch_transaction(pool: &SqlitePool, id: &str) -> AppResult<Transaction> {
    sqlx::query_as::<_, Transaction>(&format!("{SELECT} WHERE t.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

pub async fn list_for_property(
    State(st): State<AppState>,
    Path(property_id): Path<String>,
) -> AppResult<Json<Vec<Transaction>>> {
    let rows = sqlx::query_as::<_, Transaction>(&format!(
        "{SELECT} WHERE t.property_id = ? ORDER BY t.date DESC, t.created_at DESC"
    ))
    .bind(&property_id)
    .fetch_all(&st.pool)
    .await?;
    Ok(Json(rows))
}

/// The income/expense kind and who bore the cost, both derived from the chosen
/// leaf category — the single source of truth. A tenant only "bears" (and thus
/// deducts from rent) a category that is marked deductible.
async fn resolve(
    pool: &SqlitePool,
    input: &TransactionInput,
    property_kind: &str,
) -> AppResult<(String, String)> {
    let category = categories::get(pool, &input.category_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("unknown category".into()))?;
    if !category.selectable {
        return Err(AppError::BadRequest(
            "category is a group and can't be recorded against".into(),
        ));
    }
    if !category.applies_to(property_kind) {
        return Err(AppError::BadRequest(
            "category is not available for this property".into(),
        ));
    }
    // A tenant only "bears" (and deducts) a cost on a rental that collects rent.
    let deductible = category.deductible && property_kind == "rental";
    let borne_by = if input.borne_by == "tenant" && deductible {
        "tenant".to_string()
    } else {
        "landlord".to_string()
    };
    Ok((category.kind, borne_by))
}

pub async fn create(
    State(st): State<AppState>,
    Path(property_id): Path<String>,
    Json(input): Json<TransactionInput>,
) -> AppResult<Json<Transaction>> {
    Ok(Json(create_from(&st.pool, &property_id, &input).await?))
}

/// Inserts a transaction under a property, deriving kind/borne_by from the
/// category. Shared by the HTTP handler and inbox-invoice assignment.
pub(crate) async fn create_from(
    pool: &SqlitePool,
    property_id: &str,
    input: &TransactionInput,
) -> AppResult<Transaction> {
    if input.date.trim().is_empty() {
        return Err(AppError::BadRequest("date is required".into()));
    }
    let property_kind = sqlx::query_scalar::<_, String>("SELECT kind FROM properties WHERE id = ?")
        .bind(property_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;
    let (kind, borne_by) = resolve(pool, input, &property_kind).await?;

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO transactions (id, property_id, kind, category_id, amount, date, description, tenant_name, borne_by, receipt_id, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(property_id)
    .bind(&kind)
    .bind(&input.category_id)
    .bind(input.amount)
    .bind(&input.date)
    .bind(&input.description)
    .bind(&input.tenant_name)
    .bind(&borne_by)
    .bind(&input.receipt_id)
    .bind(&now)
    .execute(pool)
    .await?;
    fetch_transaction(pool, &id).await
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
    let (kind, borne_by) = resolve(&st.pool, &input, &property_kind).await?;

    let old_receipt =
        sqlx::query_scalar::<_, Option<String>>("SELECT receipt_id FROM transactions WHERE id = ?")
            .bind(&id)
            .fetch_optional(&st.pool)
            .await?
            .flatten();

    let affected = sqlx::query(
        "UPDATE transactions SET kind = ?, category_id = ?, amount = ?, date = ?, description = ?, tenant_name = ?, borne_by = ?, receipt_id = ? \
         WHERE id = ?",
    )
    .bind(&kind)
    .bind(&input.category_id)
    .bind(input.amount)
    .bind(&input.date)
    .bind(&input.description)
    .bind(&input.tenant_name)
    .bind(&borne_by)
    .bind(&input.receipt_id)
    .bind(&id)
    .execute(&st.pool)
    .await?
    .rows_affected();
    if affected == 0 {
        return Err(AppError::NotFound);
    }
    // Drop the previous receipt once it's no longer referenced.
    if let Some(old) = old_receipt {
        if input.receipt_id.as_deref() != Some(old.as_str()) {
            delete_attachment(&st, &old).await?;
        }
    }
    Ok(Json(fetch_transaction(&st.pool, &id).await?))
}

pub async fn delete(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<axum::http::StatusCode> {
    let receipt_id =
        sqlx::query_scalar::<_, Option<String>>("SELECT receipt_id FROM transactions WHERE id = ?")
            .bind(&id)
            .fetch_optional(&st.pool)
            .await?
            .flatten();
    let status = delete_by_id(&st, "transactions", &id).await?;
    if let Some(rid) = receipt_id {
        delete_attachment(&st, &rid).await?;
    }
    Ok(status)
}
