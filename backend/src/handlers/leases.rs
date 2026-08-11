use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::handlers::delete_by_id;
use crate::models::{Lease, LeaseInput};
use crate::state::AppState;

const COLUMNS: &str =
    "id, tenant_id, monthly_rent, start_date, end_date, payment_date, late_fee, notes, created_at";

pub async fn create(
    State(st): State<AppState>,
    Path(tenant_id): Path<String>,
    Json(input): Json<LeaseInput>,
) -> AppResult<Json<Lease>> {
    let tenant = sqlx::query_scalar::<_, String>("SELECT id FROM tenants WHERE id = ?")
        .bind(&tenant_id)
        .fetch_optional(&st.pool)
        .await?;
    if tenant.is_none() {
        return Err(AppError::NotFound);
    }
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let row = sqlx::query_as::<_, Lease>(&format!(
        "INSERT INTO leases (id, tenant_id, monthly_rent, start_date, end_date, payment_date, late_fee, notify_days, notes, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING {COLUMNS}"
    ))
    .bind(&id)
    .bind(&tenant_id)
    .bind(input.monthly_rent)
    .bind(&input.start_date)
    .bind(&input.end_date)
    .bind(&input.payment_date)
    .bind(input.late_fee)
    .bind(input.notify_days)
    .bind(&input.notes)
    .bind(&now)
    .fetch_one(&st.pool)
    .await?;
    Ok(Json(row))
}

pub async fn update(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<LeaseInput>,
) -> AppResult<Json<Lease>> {
    let row = sqlx::query_as::<_, Lease>(&format!(
        "UPDATE leases SET monthly_rent = ?, start_date = ?, end_date = ?, payment_date = ?, late_fee = ?, notify_days = ?, notes = ? \
         WHERE id = ? RETURNING {COLUMNS}"
    ))
    .bind(input.monthly_rent)
    .bind(&input.start_date)
    .bind(&input.end_date)
    .bind(&input.payment_date)
    .bind(input.late_fee)
    .bind(input.notify_days)
    .bind(&input.notes)
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
    delete_by_id(&st, "leases", &id).await
}
