use axum::extract::State;
use axum::Json;

use crate::error::AppResult;
use crate::models::{Settings, SettingsUpdate};
use crate::settings;
use crate::state::AppState;

pub async fn get(State(st): State<AppState>) -> AppResult<Json<Settings>> {
    let messaging_enabled = settings::get_bool(&st.pool, settings::MESSAGING_ENABLED, true).await?;
    let property_messaging_enabled =
        settings::get_bool(&st.pool, settings::PROPERTY_MESSAGING_ENABLED, true).await?;
    let signature = settings::get_string(&st.pool, settings::SIGNATURE)
        .await?
        .unwrap_or_default();
    let lease_notify_days = settings::get_i64(
        &st.pool,
        settings::LEASE_NOTIFY_DAYS,
        settings::NOTIFY_DAYS_DEFAULT,
    )
    .await?;
    let insurance_notify_days = settings::get_i64(
        &st.pool,
        settings::INSURANCE_NOTIFY_DAYS,
        settings::NOTIFY_DAYS_DEFAULT,
    )
    .await?;
    let contact_phones = settings::get_list(&st.pool, settings::CONTACT_PHONES).await?;
    Ok(Json(Settings {
        messaging_enabled,
        property_messaging_enabled,
        signature,
        lease_notify_days,
        insurance_notify_days,
        contact_phones,
    }))
}

pub async fn update(
    State(st): State<AppState>,
    Json(input): Json<SettingsUpdate>,
) -> AppResult<Json<Settings>> {
    if let Some(enabled) = input.messaging_enabled {
        settings::set_bool(&st.pool, settings::MESSAGING_ENABLED, enabled).await?;
    }
    if let Some(enabled) = input.property_messaging_enabled {
        settings::set_bool(&st.pool, settings::PROPERTY_MESSAGING_ENABLED, enabled).await?;
    }
    if let Some(signature) = input.signature {
        settings::set_string(&st.pool, settings::SIGNATURE, signature.trim()).await?;
    }
    if let Some(days) = input.lease_notify_days {
        settings::set_i64(&st.pool, settings::LEASE_NOTIFY_DAYS, days.max(0)).await?;
    }
    if let Some(days) = input.insurance_notify_days {
        settings::set_i64(&st.pool, settings::INSURANCE_NOTIFY_DAYS, days.max(0)).await?;
    }
    if let Some(phones) = input.contact_phones {
        // Keep only non-empty, trimmed numbers.
        let cleaned: Vec<String> = phones
            .into_iter()
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();
        settings::set_list(&st.pool, settings::CONTACT_PHONES, &cleaned).await?;
    }
    get(State(st)).await
}
