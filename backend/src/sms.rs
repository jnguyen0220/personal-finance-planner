//! Outbound SMS via the Telnyx Messaging API. When the `TELNYX_*` variables are
//! configured messages are delivered through Telnyx; otherwise `send` logs the
//! message so the app still works without a provider.

use serde::Serialize;

const TELNYX_API: &str = "https://api.telnyx.com/v2/messages";

/// Telnyx credentials sourced from the environment.
struct Config {
    api_key: String,
    from: Option<String>,
    messaging_profile_id: Option<String>,
}

impl Config {
    /// Reads the `TELNYX_*` variables. Returns `None` (provider disabled) unless
    /// an API key and at least one sender (`TELNYX_FROM` phone number or
    /// `TELNYX_MESSAGING_PROFILE_ID`) are present.
    fn from_env() -> Option<Self> {
        let api_key = non_empty("TELNYX_API_KEY")?;
        let from = non_empty("TELNYX_FROM");
        let messaging_profile_id = non_empty("TELNYX_MESSAGING_PROFILE_ID");
        if from.is_none() && messaging_profile_id.is_none() {
            return None;
        }
        Some(Self {
            api_key,
            from,
            messaging_profile_id,
        })
    }
}

fn non_empty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Delivers a text message. Returns `Err` with a human-readable reason on
/// failure so it can be stored against the message row. Falls back to logging
/// when Telnyx is not configured.
pub async fn send(to: &str, body: &str) -> Result<(), String> {
    let to = to.trim();
    if to.is_empty() {
        return Err("tenant has no phone number".into());
    }

    let Some(cfg) = Config::from_env() else {
        tracing::info!(to, body, "SMS send (simulated — no provider configured)");
        return Ok(());
    };

    send_via_telnyx(&cfg, to, body).await
}

#[derive(Serialize)]
struct MessageRequest<'a> {
    to: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    messaging_profile_id: Option<&'a str>,
}

async fn send_via_telnyx(cfg: &Config, to: &str, body: &str) -> Result<(), String> {
    let payload = MessageRequest {
        to,
        text: body,
        from: cfg.from.as_deref(),
        messaging_profile_id: cfg.messaging_profile_id.as_deref(),
    };
    let resp = crate::http::send(
        crate::http::post(TELNYX_API)
            .bearer_auth(&cfg.api_key)
            .json(&payload),
    )
    .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp.text().await.unwrap_or_default();
        return Err(format!("telnyx send failed ({status}): {detail}"));
    }
    Ok(())
}
