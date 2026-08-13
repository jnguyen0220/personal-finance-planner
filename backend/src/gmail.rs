//! Inbound email via the Gmail REST API. Google no longer issues app passwords,
//! so the backend authenticates with an OAuth2 refresh token, exchanges it for a
//! short-lived access token, and reads messages that match a search query.
//!
//! Configure the `GMAIL_*` variables in `.env` (see `.env.example`). When the
//! credentials are absent the poller is disabled and skipped. `poll` returns the
//! fetched messages; process them where it is called (the scheduler in `main`).

use base64::Engine;
use serde::Deserialize;

const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const GMAIL_API: &str = "https://gmail.googleapis.com/gmail/v1/users/me";

/// OAuth2 credentials plus the inbox query, sourced from the environment.
pub struct Config {
    client_id: String,
    client_secret: String,
    refresh_token: String,
    query: String,
    max_results: u32,
}

impl Config {
    /// Reads the `GMAIL_*` variables. Returns `None` (poller disabled) unless the
    /// three credential values are all present and non-empty.
    pub fn from_env() -> Option<Self> {
        Some(Self {
            client_id: non_empty("GMAIL_CLIENT_ID")?,
            client_secret: non_empty("GMAIL_CLIENT_SECRET")?,
            refresh_token: non_empty("GMAIL_REFRESH_TOKEN")?,
            query: non_empty("GMAIL_QUERY").unwrap_or_else(|| "is:unread".to_string()),
            max_results: std::env::var("GMAIL_MAX_RESULTS")
                .ok()
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(10),
        })
    }
}

/// Whether Gmail credentials are configured, without building a client.
pub fn configured() -> bool {
    Config::from_env().is_some()
}

fn non_empty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// A fetched email reduced to the fields the app cares about.
pub struct Email {
    pub id: String,
    pub thread_id: String,
    pub from: String,
    pub subject: String,
    pub date: String,
    pub snippet: String,
    pub body: String,
}

/// Fetches messages matching the configured query. Returns an empty list when
/// Gmail is not configured so callers need not special-case the disabled state.
pub async fn poll() -> Result<Vec<Email>, String> {
    let Some(cfg) = Config::from_env() else {
        return Ok(Vec::new());
    };
    let client = reqwest::Client::new();
    let token = access_token(&client, &cfg).await?;

    let mut emails = Vec::new();
    for id in list_ids(&client, &token, &cfg).await? {
        match get_message(&client, &token, &id).await {
            Ok(email) => emails.push(email),
            Err(e) => tracing::error!("gmail: failed to fetch message {id}: {e}"),
        }
    }
    Ok(emails)
}

/// Exchanges the refresh token for a short-lived access token.
async fn access_token(client: &reqwest::Client, cfg: &Config) -> Result<String, String> {
    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
    }

    let resp = client
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("client_id", cfg.client_id.as_str()),
            ("client_secret", cfg.client_secret.as_str()),
            ("refresh_token", cfg.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("token refresh failed ({status}): {body}"));
    }
    Ok(resp
        .json::<TokenResponse>()
        .await
        .map_err(|e| e.to_string())?
        .access_token)
}

/// Lists message IDs matching the configured query.
async fn list_ids(
    client: &reqwest::Client,
    token: &str,
    cfg: &Config,
) -> Result<Vec<String>, String> {
    #[derive(Deserialize)]
    struct ListResponse {
        #[serde(default)]
        messages: Vec<MessageRef>,
    }
    #[derive(Deserialize)]
    struct MessageRef {
        id: String,
    }

    let max = cfg.max_results.to_string();
    let resp = client
        .get(format!("{GMAIL_API}/messages"))
        .bearer_auth(token)
        .query(&[("q", cfg.query.as_str()), ("maxResults", max.as_str())])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("list messages failed ({status}): {body}"));
    }
    Ok(resp
        .json::<ListResponse>()
        .await
        .map_err(|e| e.to_string())?
        .messages
        .into_iter()
        .map(|m| m.id)
        .collect())
}

/// Fetches a single message in full and reduces it to an [`Email`].
async fn get_message(client: &reqwest::Client, token: &str, id: &str) -> Result<Email, String> {
    let resp = client
        .get(format!("{GMAIL_API}/messages/{id}"))
        .bearer_auth(token)
        .query(&[("format", "full")])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("get message failed ({status}): {body}"));
    }
    let msg = resp
        .json::<GmailMessage>()
        .await
        .map_err(|e| e.to_string())?;
    Ok(to_email(msg))
}

#[derive(Deserialize)]
struct GmailMessage {
    id: String,
    #[serde(rename = "threadId", default)]
    thread_id: String,
    #[serde(default)]
    snippet: String,
    payload: Option<Payload>,
}

#[derive(Deserialize)]
struct Payload {
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
    #[serde(default)]
    headers: Vec<Header>,
    body: Option<Body>,
    #[serde(default)]
    parts: Vec<Payload>,
}

#[derive(Deserialize)]
struct Header {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct Body {
    data: Option<String>,
}

fn to_email(msg: GmailMessage) -> Email {
    let (from, subject, date, body) = match &msg.payload {
        Some(p) => (
            header(&p.headers, "From").to_string(),
            header(&p.headers, "Subject").to_string(),
            header(&p.headers, "Date").to_string(),
            extract_body(p),
        ),
        None => Default::default(),
    };
    Email {
        id: msg.id,
        thread_id: msg.thread_id,
        from,
        subject,
        date,
        snippet: msg.snippet,
        body,
    }
}

fn header<'a>(headers: &'a [Header], name: &str) -> &'a str {
    headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
        .unwrap_or_default()
}

/// Depth-first search for the message text, preferring `text/plain`.
fn extract_body(payload: &Payload) -> String {
    if payload.mime_type.as_deref() == Some("text/plain") {
        if let Some(text) = payload.body.as_ref().and_then(decode_body) {
            return text;
        }
    }
    for part in &payload.parts {
        let text = extract_body(part);
        if !text.is_empty() {
            return text;
        }
    }
    // Fall back to a single-part message body (e.g. plain text with no parts).
    payload
        .body
        .as_ref()
        .and_then(decode_body)
        .unwrap_or_default()
}

fn decode_body(body: &Body) -> Option<String> {
    let data = body.data.as_ref()?;
    // Gmail encodes body data as base64url; strip padding/whitespace to be lenient.
    let cleaned: String = data
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '=')
        .collect();
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cleaned)
        .ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}
