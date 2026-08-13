use axum::extract::{Path, State};
use axum::Json;
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::handlers::{delete_by_id, ensure_found};
use crate::models::{Lease, Tenant, TenantInput, TenantWithLeases};
use crate::state::AppState;

const COLUMNS: &str =
    "id, property_id, first_name, last_name, email, phone, is_current, notifications_enabled, notes, driver_license_id, created_at";

pub async fn list(
    State(st): State<AppState>,
    Path(property_id): Path<String>,
) -> AppResult<Json<Vec<TenantWithLeases>>> {
    let tenants = sqlx::query_as::<_, Tenant>(&format!(
        "SELECT {COLUMNS} FROM tenants WHERE property_id = ? ORDER BY is_current DESC, last_name, first_name"
    ))
    .bind(&property_id)
    .fetch_all(&st.pool)
    .await?;

    let leases = sqlx::query_as::<_, Lease>(
        "SELECT l.id, l.tenant_id, l.monthly_rent, l.start_date, l.end_date, l.payment_date, l.late_fee, l.notes, l.created_at \
         FROM leases l JOIN tenants t ON t.id = l.tenant_id \
         WHERE t.property_id = ? ORDER BY l.start_date DESC, l.created_at DESC",
    )
    .bind(&property_id)
    .fetch_all(&st.pool)
    .await?;

    let mut by_tenant: HashMap<String, Vec<Lease>> = HashMap::new();
    for l in leases {
        by_tenant.entry(l.tenant_id.clone()).or_default().push(l);
    }

    let rows = tenants
        .into_iter()
        .map(|t| {
            let leases = by_tenant.remove(&t.id).unwrap_or_default();
            TenantWithLeases::new(t, leases)
        })
        .collect();
    Ok(Json(rows))
}

async fn get_one(st: &AppState, id: &str) -> AppResult<TenantWithLeases> {
    let tenant = sqlx::query_as::<_, Tenant>(&format!("SELECT {COLUMNS} FROM tenants WHERE id = ?"))
        .bind(id)
        .fetch_optional(&st.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let leases = sqlx::query_as::<_, Lease>(
        "SELECT id, tenant_id, monthly_rent, start_date, end_date, payment_date, late_fee, notes, created_at \
         FROM leases WHERE tenant_id = ? ORDER BY start_date DESC, created_at DESC",
    )
    .bind(id)
    .fetch_all(&st.pool)
    .await?;

    Ok(TenantWithLeases::new(tenant, leases))
}

pub async fn create(
    State(st): State<AppState>,
    Path(property_id): Path<String>,
    Json(input): Json<TenantInput>,
) -> AppResult<Json<TenantWithLeases>> {
    if input.first_name.trim().is_empty() {
        return Err(AppError::BadRequest("first name is required".into()));
    }
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO tenants (id, property_id, first_name, last_name, email, phone, is_current, notifications_enabled, notes, driver_license_id, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&property_id)
    .bind(input.first_name.trim())
    .bind(input.last_name.trim())
    .bind(&input.email)
    .bind(&input.phone)
    .bind(input.is_current)
    .bind(input.notifications_enabled)
    .bind(&input.notes)
    .bind(&input.driver_license_id)
    .bind(&now)
    .execute(&st.pool)
    .await?;
    if input.is_current {
        clear_other_current(&st, &property_id, &id).await?;
    }
    Ok(Json(get_one(&st, &id).await?))
}

pub async fn update(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<TenantInput>,
) -> AppResult<Json<TenantWithLeases>> {
    let affected = sqlx::query(
        "UPDATE tenants SET first_name = ?, last_name = ?, email = ?, phone = ?, is_current = ?, notifications_enabled = ?, notes = ?, driver_license_id = ? WHERE id = ?",
    )
    .bind(input.first_name.trim())
    .bind(input.last_name.trim())
    .bind(&input.email)
    .bind(&input.phone)
    .bind(input.is_current)
    .bind(input.notifications_enabled)
    .bind(&input.notes)
    .bind(&input.driver_license_id)
    .bind(&id)
    .execute(&st.pool)
    .await?
    .rows_affected();
    ensure_found(affected)?;
    let row = get_one(&st, &id).await?;
    if row.tenant.is_current {
        clear_other_current(&st, &row.tenant.property_id, &id).await?;
    }
    Ok(Json(row))
}

/// Ensures only one tenant per property is flagged as current.
async fn clear_other_current(st: &AppState, property_id: &str, keep_id: &str) -> AppResult<()> {
    sqlx::query("UPDATE tenants SET is_current = 0 WHERE property_id = ? AND id != ?")
        .bind(property_id)
        .bind(keep_id)
        .execute(&st.pool)
        .await?;
    Ok(())
}

pub async fn delete(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<axum::http::StatusCode> {
    delete_by_id(&st, "tenants", &id).await
}
