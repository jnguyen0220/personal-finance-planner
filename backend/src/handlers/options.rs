use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::Json;

use crate::error::{AppError, AppResult};
use crate::etag;
use crate::options;
use crate::state::AppState;

/// The values of an editable dropdown list, used to populate select inputs.
pub async fn get(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(list): Path<String>,
) -> AppResult<Response> {
    if !options::is_known(&list) {
        return Err(AppError::NotFound);
    }
    let values = options::values(&st.pool, &list).await?;
    let body = serde_json::to_string(&values).expect("serialize option list");
    Ok(etag::respond_json(&headers, body))
}

/// Replaces the whole list with the provided values, in order.
pub async fn put(
    State(st): State<AppState>,
    Path(list): Path<String>,
    Json(values): Json<Vec<String>>,
) -> AppResult<Json<Vec<String>>> {
    if !options::is_known(&list) {
        return Err(AppError::NotFound);
    }
    Ok(Json(options::replace(&st.pool, &list, &values).await?))
}
