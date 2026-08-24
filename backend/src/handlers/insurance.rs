use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::handlers::delete_by_id;
use crate::models::{InsuranceInput, InsurancePolicy, InsurancePolicyView};
use crate::state::AppState;

const COLUMNS: &str =
    "id, property_id, provider, policy_number, premium, start_date, expiry_date, notes, created_at";

pub async fn list_for_property(
    State(st): State<AppState>,
    Path(property_id): Path<String>,
) -> AppResult<Json<Vec<InsurancePolicyView>>> {
    let rows = crate::db::fetch_all(
        &st.pool,
        sqlx::query_as::<_, InsurancePolicy>(&format!(
            "SELECT {COLUMNS} FROM insurance_policies WHERE property_id = ? ORDER BY expiry_date"
        ))
        .bind(&property_id),
    )
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(InsurancePolicyView::from_policy)
            .collect(),
    ))
}

pub async fn create(
    State(st): State<AppState>,
    Path(property_id): Path<String>,
    Json(input): Json<InsuranceInput>,
) -> AppResult<Json<InsurancePolicyView>> {
    if input.provider.trim().is_empty() {
        return Err(AppError::BadRequest("provider is required".into()));
    }
    if input.expiry_date.trim().is_empty() {
        return Err(AppError::BadRequest("expiry_date is required".into()));
    }
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let policy = crate::db::fetch_one(
        &st.pool,
        sqlx::query_as::<_, InsurancePolicy>(&format!(
            "INSERT INTO insurance_policies (id, property_id, provider, policy_number, premium, start_date, expiry_date, notes, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING {COLUMNS}"
        ))
        .bind(&id)
        .bind(&property_id)
        .bind(input.provider.trim())
        .bind(&input.policy_number)
        .bind(input.premium)
        .bind(&input.start_date)
        .bind(&input.expiry_date)
        .bind(&input.notes)
        .bind(&now),
    )
    .await?;
    Ok(Json(InsurancePolicyView::from_policy(policy)))
}

pub async fn update(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<InsuranceInput>,
) -> AppResult<Json<InsurancePolicyView>> {
    let policy = crate::db::fetch_optional(
        &st.pool,
        sqlx::query_as::<_, InsurancePolicy>(&format!(
            "UPDATE insurance_policies SET provider = ?, policy_number = ?, premium = ?, start_date = ?, expiry_date = ?, notes = ? \
             WHERE id = ? RETURNING {COLUMNS}"
        ))
        .bind(input.provider.trim())
        .bind(&input.policy_number)
        .bind(input.premium)
        .bind(&input.start_date)
        .bind(&input.expiry_date)
        .bind(&input.notes)
        .bind(&id),
    )
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(InsurancePolicyView::from_policy(policy)))
}

pub async fn delete(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<axum::http::StatusCode> {
    delete_by_id(&st, "insurance_policies", &id).await
}
