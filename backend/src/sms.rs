//! Outbound SMS seam. Swap the body of `send` for a real provider (e.g. Twilio)
//! without touching the handlers or storage that record each message.

/// Delivers a text message. Returns `Err` with a human-readable reason on
/// failure so it can be stored against the message row.
///
/// This stub validates the recipient and logs the message; wire a provider
/// (credentials from env) here to actually send.
pub async fn send(to: &str, body: &str) -> Result<(), String> {
    if to.trim().is_empty() {
        return Err("tenant has no phone number".into());
    }
    tracing::info!(to, body, "SMS send (simulated — no provider configured)");
    Ok(())
}
