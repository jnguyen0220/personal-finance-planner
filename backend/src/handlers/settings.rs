use axum::extract::State;
use axum::Json;

use crate::error::AppResult;
use crate::models::Settings;
use crate::settings;
use crate::state::AppState;

pub async fn get(State(st): State<AppState>) -> AppResult<Json<Settings>> {
    let messaging_enabled =
        settings::get_bool(&st.pool, settings::MESSAGING_ENABLED, true).await?;
    Ok(Json(Settings { messaging_enabled }))
}

pub async fn update(
    State(st): State<AppState>,
    Json(input): Json<Settings>,
) -> AppResult<Json<Settings>> {
    settings::set_bool(&st.pool, settings::MESSAGING_ENABLED, input.messaging_enabled).await?;
    Ok(Json(input))
}
