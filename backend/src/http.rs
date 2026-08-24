//! Single outbound-HTTP choke point. Every request the backend makes to an
//! external service (Telnyx, Gmail, …) is built with [`get`]/[`post`] and sent
//! through [`send`], so request/response logging, timing, and future
//! instrumentation live in exactly one place. Callers never touch a
//! `reqwest::Client` directly.

use std::sync::OnceLock;

/// The process-wide HTTP client. Reusing one client shares the connection pool
/// across every outbound call.
fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

/// Begins a GET request. Add headers/query/body on the returned builder, then
/// pass it to [`send`].
pub fn get(url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder {
    client().get(url)
}

/// Begins a POST request. Add headers/body on the returned builder, then pass
/// it to [`send`].
pub fn post(url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder {
    client().post(url)
}

/// Sends a prepared request, logging its method, URL and outcome. Returns the
/// response (whatever its status) or the transport error as a string. This is
/// the only function in the codebase that performs an outbound HTTP call.
pub async fn send(request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let request = request.build().map_err(|e| e.to_string())?;
    let method = request.method().clone();
    let url = request.url().clone();
    tracing::debug!(target: "http", %method, %url, "outbound request");
    match client().execute(request).await {
        Ok(resp) => {
            tracing::debug!(target: "http", %method, %url, status = %resp.status(), "outbound response");
            Ok(resp)
        }
        Err(e) => {
            tracing::error!(target: "http", %method, %url, "outbound request failed: {e}");
            Err(e.to_string())
        }
    }
}
