use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::error::AppResult;
use crate::models::Notification;
use crate::notify;
use crate::state::AppState;

pub async fn list(State(st): State<AppState>) -> AppResult<Json<Vec<Notification>>> {
    notify::reconcile(&st.pool).await?;
    Ok(Json(notify::list_active(&st.pool).await?))
}

pub async fn dismiss(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    notify::dismiss(&st.pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
