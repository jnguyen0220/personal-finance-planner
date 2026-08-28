use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::error::AppResult;
use crate::logs;
use crate::models::LogEntry;
use crate::state::AppState;

/// Cap on how many recent entries the admin Logs page loads at once.
const LIMIT: i64 = 500;

pub async fn list(State(st): State<AppState>) -> AppResult<Json<Vec<LogEntry>>> {
    Ok(Json(logs::list(&st.pool, LIMIT).await?))
}

pub async fn clear(State(st): State<AppState>) -> AppResult<StatusCode> {
    logs::clear(&st.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}
