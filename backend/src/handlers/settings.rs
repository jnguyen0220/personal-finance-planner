use axum::extract::State;
use axum::Json;

use crate::error::AppResult;
use crate::models::{Settings, SettingsUpdate};
use crate::settings;
use crate::state::AppState;

pub async fn get(State(st): State<AppState>) -> AppResult<Json<Settings>> {
    let messaging_enabled = settings::get_bool(&st.pool, settings::MESSAGING_ENABLED, true).await?;
    let signature = settings::get_string(&st.pool, settings::SIGNATURE)
        .await?
        .unwrap_or_default();
    Ok(Json(Settings {
        messaging_enabled,
        signature,
    }))
}

pub async fn update(
    State(st): State<AppState>,
    Json(input): Json<SettingsUpdate>,
) -> AppResult<Json<Settings>> {
    if let Some(enabled) = input.messaging_enabled {
        settings::set_bool(&st.pool, settings::MESSAGING_ENABLED, enabled).await?;
    }
    if let Some(signature) = input.signature {
        settings::set_string(&st.pool, settings::SIGNATURE, signature.trim()).await?;
    }
    get(State(st)).await
}
