use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{message_kind, BroadcastRecipient, BroadcastResult, Message, MessageInput};
use crate::sms;
use crate::state::AppState;

const COLUMNS: &str =
    "id, tenant_id, property_id, kind, to_phone, body, status, error, created_at, sent_at";

pub async fn list_for_property(
    State(st): State<AppState>,
    Path(property_id): Path<String>,
) -> AppResult<Json<Vec<Message>>> {
    let rows = sqlx::query_as::<_, Message>(&format!(
        "SELECT {COLUMNS} FROM messages WHERE property_id = ? ORDER BY created_at DESC"
    ))
    .bind(&property_id)
    .fetch_all(&st.pool)
    .await?;
    Ok(Json(rows))
}

pub async fn create(
    State(st): State<AppState>,
    Path(tenant_id): Path<String>,
    Json(input): Json<MessageInput>,
) -> AppResult<Json<Message>> {
    if input.body.trim().is_empty() {
        return Err(AppError::BadRequest("message body is required".into()));
    }

    let tenant = sqlx::query_as::<_, (String, String)>(
        "SELECT property_id, phone FROM tenants WHERE id = ?",
    )
    .bind(&tenant_id)
    .fetch_optional(&st.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let (property_id, phone) = tenant;

    send_and_store(
        &st,
        &tenant_id,
        &property_id,
        &phone,
        &input.kind,
        input.body.trim(),
    )
    .await
}

/// Lists the current tenants (across all properties) a broadcast would reach:
/// those marked current with a phone number on file.
pub async fn recipients(State(st): State<AppState>) -> AppResult<Json<Vec<BroadcastRecipient>>> {
    let rows = sqlx::query_as::<_, BroadcastRecipient>(
        "SELECT t.id, trim(t.first_name || ' ' || t.last_name) AS name, t.phone, p.name AS property_name \
         FROM tenants t JOIN properties p ON p.id = t.property_id \
         WHERE t.is_current = 1 AND t.phone <> '' \
         ORDER BY name",
    )
    .fetch_all(&st.pool)
    .await?;
    Ok(Json(rows))
}

/// Sends a one-off message to every current tenant across all properties who
/// has a phone number, recording each attempt. Returns a delivery summary.
pub async fn broadcast(
    State(st): State<AppState>,
    Json(input): Json<MessageInput>,
) -> AppResult<Json<BroadcastResult>> {
    let body = input.body.trim();
    if body.is_empty() {
        return Err(AppError::BadRequest("message body is required".into()));
    }

    let recipients = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, property_id, phone FROM tenants WHERE is_current = 1 AND phone <> ''",
    )
    .fetch_all(&st.pool)
    .await?;

    if recipients.is_empty() {
        return Err(AppError::BadRequest(
            "no current tenants with a phone number".into(),
        ));
    }

    // Render {signature} where the operator placed it, like the other templates.
    let signature = crate::templates::signature_value(&st.pool).await?;
    let full_body = crate::templates::render(body, &[("signature", signature)]);

    let (mut sent, mut failed) = (0usize, 0usize);
    for (tenant_id, property_id, phone) in recipients {
        let Json(msg) = send_and_store(
            &st,
            &tenant_id,
            &property_id,
            &phone,
            message_kind::BROADCAST,
            &full_body,
        )
        .await?;
        if msg.status == "sent" {
            sent += 1;
        } else {
            failed += 1;
        }
    }

    Ok(Json(BroadcastResult {
        total: sent + failed,
        sent,
        failed,
    }))
}

/// Texts a tenant every utility provider configured for their property.
pub async fn send_providers(
    State(st): State<AppState>,
    Path(tenant_id): Path<String>,
) -> AppResult<Json<Message>> {
    let tenant = sqlx::query_as::<_, (String, String)>(
        "SELECT property_id, phone FROM tenants WHERE id = ?",
    )
    .bind(&tenant_id)
    .fetch_optional(&st.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let (property_id, phone) = tenant;

    let body = compose_providers_message(&st, &property_id).await?;
    send_and_store(
        &st,
        &tenant_id,
        &property_id,
        &phone,
        message_kind::PROVIDERS,
        &body,
    )
    .await
}

/// Returns the utility/HOA message body for a property without sending it.
pub async fn preview_providers(
    State(st): State<AppState>,
    Path(property_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let body = compose_providers_message(&st, &property_id).await?;
    Ok(Json(serde_json::json!({ "body": body })))
}

/// Builds the utility-provider + HOA message for a property, erroring when the
/// property has nothing to share. Shared by preview and send so they never drift.
async fn compose_providers_message(st: &AppState, property_id: &str) -> AppResult<String> {
    let property = sqlx::query_as::<_, (String, String, String, String, String, String, String, String)>(
        "SELECT address, city, state, zip, hoa_name, hoa_phone, hoa_email, hoa_webpage FROM properties WHERE id = ?",
    )
    .bind(property_id)
    .fetch_optional(&st.pool)
    .await?
    .unwrap_or_default();
    let (address, city, state, zip, hoa_name, hoa_phone, hoa_email, hoa_webpage) = property;

    let providers = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT kind, name, phone, homepage FROM providers WHERE property_id = ? ORDER BY kind, name",
    )
    .bind(property_id)
    .fetch_all(&st.pool)
    .await?;

    // Any HOA contact detail worth including.
    let hoa: Vec<&str> = [
        hoa_name.trim(),
        hoa_phone.trim(),
        hoa_email.trim(),
        hoa_webpage.trim(),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect();

    if providers.is_empty() && hoa.is_empty() {
        return Err(AppError::BadRequest(
            "no providers or HOA configured for this property".into(),
        ));
    }

    let provider_lines: Vec<String> = providers
        .into_iter()
        .map(|(kind, name, pphone, homepage)| {
            let mut line = format!("- {}: {}", title_case(&kind), name);
            if !pphone.trim().is_empty() {
                line.push_str(&format!(" — {}", pphone.trim()));
            }
            if !homepage.trim().is_empty() {
                line.push_str(&format!(" — {}", homepage.trim()));
            }
            line
        })
        .collect();
    let providers_block = provider_lines.join("\n");
    let hoa_block = if hoa.is_empty() {
        String::new()
    } else {
        format!("\nHOA: {}", hoa.join(" — "))
    };

    let template = crate::templates::body(&st.pool, "providers").await?;
    let signature = crate::templates::signature_value(&st.pool).await?;
    Ok(crate::templates::render(
        &template,
        &[
            ("address", address),
            ("city", city),
            ("state", state),
            ("zip", zip),
            ("providers", providers_block),
            ("hoa", hoa_block),
            ("signature", signature),
        ],
    ))
}

/// Sends `body` to `phone`, recording the attempt and its outcome as a message.
async fn send_and_store(
    st: &AppState,
    tenant_id: &str,
    property_id: &str,
    phone: &str,
    kind: &str,
    body: &str,
) -> AppResult<Json<Message>> {
    let now = chrono::Utc::now().to_rfc3339();
    let (status, error, sent_at) = match sms::send(phone, body).await {
        Ok(()) => ("sent", None, Some(now.clone())),
        Err(e) => ("failed", Some(e), None),
    };

    let id = Uuid::new_v4().to_string();
    let row = sqlx::query_as::<_, Message>(&format!(
        "INSERT INTO messages (id, tenant_id, property_id, kind, to_phone, body, status, error, created_at, sent_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING {COLUMNS}"
    ))
    .bind(&id)
    .bind(tenant_id)
    .bind(property_id)
    .bind(kind)
    .bind(phone)
    .bind(body)
    .bind(status)
    .bind(&error)
    .bind(&now)
    .bind(&sent_at)
    .fetch_one(&st.pool)
    .await?;

    Ok(Json(row))
}

/// Capitalises the first letter for display (e.g. "electricity" → "Electricity").
fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
