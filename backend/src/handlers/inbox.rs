//! The invoice review queue. The Gmail poller ingests supported attachments as
//! pending items (running OCR to suggest an amount); a reviewer then assigns an
//! item to a property — filing it as a transaction with the file as its receipt
//! — or dismisses it.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::gmail;
use crate::handlers::{attachments, delete_attachment, transactions};
use crate::models::{InboxAssignInput, InboxItem, Transaction, TransactionInput};
use crate::settings;
use crate::state::AppState;

/// Joins the stored attachment so the UI can preview the file.
const SELECT: &str = "SELECT i.id, i.gmail_id, i.thread_id, i.from_addr, i.subject, i.snippet, \
     i.received_at, i.attachment_id, a.original_name AS attachment_name, \
     a.content_type AS attachment_type, i.ocr_amount, i.ocr_status, i.status, i.created_at \
     FROM inbox_items i LEFT JOIN attachments a ON a.id = i.attachment_id";

pub async fn list(State(st): State<AppState>) -> AppResult<Json<Vec<InboxItem>>> {
    let rows = sqlx::query_as::<_, InboxItem>(&format!(
        "{SELECT} WHERE i.status = 'pending' ORDER BY i.received_at DESC, i.created_at DESC"
    ))
    .fetch_all(&st.pool)
    .await?;
    Ok(Json(rows))
}

/// The result of a manual Gmail poll: how many new attachments were queued and
/// when the poll ran.
#[derive(Serialize)]
pub struct PollResult {
    pub ingested: usize,
    pub last_poll: Option<String>,
}

/// Manually triggers a Gmail poll from the invoice UI, queueing any new
/// attachments for review and reporting how many were added.
pub async fn poll(State(st): State<AppState>) -> AppResult<Json<PollResult>> {
    let ingested = poll_and_ingest(&st).await.map_err(AppError::BadRequest)?;
    let last_poll = settings::get_string(&st.pool, settings::GMAIL_LAST_POLL).await?;
    Ok(Json(PollResult {
        ingested,
        last_poll,
    }))
}

/// When Gmail was last polled, for the invoice UI's "last checked" indicator.
#[derive(Serialize)]
pub struct InboxStatus {
    pub last_poll: Option<String>,
}

pub async fn status(State(st): State<AppState>) -> AppResult<Json<InboxStatus>> {
    let last_poll = settings::get_string(&st.pool, settings::GMAIL_LAST_POLL).await?;
    Ok(Json(InboxStatus { last_poll }))
}

/// Re-runs OCR on a pending item's attachment on demand, refreshing the stored
/// text, suggested amount, and status light. Returns the updated item.
pub async fn rerun_ocr(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<InboxItem>> {
    let attachment_id = sqlx::query_scalar::<_, Option<String>>(
        "SELECT attachment_id FROM inbox_items WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&st.pool)
    .await?
    .ok_or(AppError::NotFound)?
    .ok_or_else(|| AppError::BadRequest("item has no attachment to scan".into()))?;

    let (stored_name, content_type) = sqlx::query_as::<_, (String, String)>(
        "SELECT stored_name, content_type FROM attachments WHERE id = ?",
    )
    .bind(&attachment_id)
    .fetch_optional(&st.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let path = st.uploads.join(&stored_name);
    let (ocr_text, ocr_amount, ocr_status) =
        tokio::task::spawn_blocking(move || crate::ocr::extract(&path, &content_type))
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

    sqlx::query("UPDATE inbox_items SET ocr_text = ?, ocr_amount = ?, ocr_status = ? WHERE id = ?")
        .bind(&ocr_text)
        .bind(ocr_amount)
        .bind(ocr_status.as_str())
        .bind(&id)
        .execute(&st.pool)
        .await?;

    let item = sqlx::query_as::<_, InboxItem>(&format!("{SELECT} WHERE i.id = ?"))
        .bind(&id)
        .fetch_one(&st.pool)
        .await?;
    Ok(Json(item))
}

pub async fn assign(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<InboxAssignInput>,
) -> AppResult<Json<Transaction>> {
    let (attachment_id, _status) = fetch_item(&st, &id).await?;

    let tx_input = TransactionInput {
        category_id: input.category_id,
        amount: input.amount,
        date: input.date,
        description: input.description,
        tenant_name: input.tenant_name,
        borne_by: input.borne_by,
        receipt_id: attachment_id,
    };
    let tx = transactions::create_from(&st.pool, &input.property_id, &tx_input).await?;

    sqlx::query("UPDATE inbox_items SET status = 'assigned', transaction_id = ? WHERE id = ?")
        .bind(&tx.id)
        .bind(&id)
        .execute(&st.pool)
        .await?;
    Ok(Json(tx))
}

pub async fn dismiss(State(st): State<AppState>, Path(id): Path<String>) -> AppResult<StatusCode> {
    let (attachment_id, _status) = fetch_item(&st, &id).await?;

    sqlx::query("UPDATE inbox_items SET status = 'dismissed', attachment_id = NULL WHERE id = ?")
        .bind(&id)
        .execute(&st.pool)
        .await?;
    if let Some(aid) = attachment_id {
        delete_attachment(&st, &aid).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Loads a pending item's attachment id, rejecting unknown or already-processed
/// items so an invoice can't be filed or dismissed twice.
async fn fetch_item(st: &AppState, id: &str) -> AppResult<(Option<String>, String)> {
    let row = sqlx::query_as::<_, (Option<String>, String)>(
        "SELECT attachment_id, status FROM inbox_items WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&st.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    if row.1 != "pending" {
        return Err(AppError::BadRequest(
            "item has already been processed".into(),
        ));
    }
    Ok(row)
}

/// Fetches messages from Gmail and ingests any new invoice attachments as
/// pending review items, returning how many were queued. Shared by the daily
/// scheduler and the manual trigger; a no-op when Gmail is not configured.
pub async fn poll_and_ingest(st: &AppState) -> Result<usize, String> {
    if !gmail::configured() {
        return Ok(0);
    }
    let emails = gmail::poll().await?;
    let now = chrono::Utc::now().to_rfc3339();
    settings::set_string(&st.pool, settings::GMAIL_LAST_POLL, &now)
        .await
        .map_err(|e| e.to_string())?;
    let mut total = 0;
    for email in &emails {
        match ingest(st, email).await {
            Ok(0) => {}
            Ok(n) => {
                tracing::info!(
                    id = %email.id,
                    from = %email.from,
                    subject = %email.subject,
                    attachments = n,
                    "gmail: queued invoice attachments for review"
                );
                total += n;
            }
            Err(err) => tracing::error!(id = %email.id, "gmail: failed to ingest email: {err}"),
        }
    }
    Ok(total)
}

/// Stores an email's supported attachments as pending review items, running OCR
/// to suggest an amount. Emails already ingested (by Gmail id) are skipped so a
/// read-only mailbox can be polled repeatedly without duplicating work.
pub async fn ingest(st: &AppState, email: &gmail::Email) -> Result<usize, String> {
    if email.attachments.is_empty() {
        return Ok(0);
    }
    let seen = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM inbox_items WHERE gmail_id = ?")
        .bind(&email.id)
        .fetch_one(&st.pool)
        .await
        .map_err(|e| e.to_string())?;
    if seen > 0 {
        return Ok(0);
    }

    let mut count = 0;
    for att in &email.attachments {
        let attachment_id = attachments::store_bytes(st, &att.filename, &att.mime_type, &att.data)
            .await
            .map_err(|e| e.to_string())?;

        let stored_name =
            sqlx::query_scalar::<_, String>("SELECT stored_name FROM attachments WHERE id = ?")
                .bind(&attachment_id)
                .fetch_one(&st.pool)
                .await
                .map_err(|e| e.to_string())?;
        let path = st.uploads.join(&stored_name);
        let mime = att.mime_type.clone();
        let (ocr_text, ocr_amount, ocr_status) =
            tokio::task::spawn_blocking(move || crate::ocr::extract(&path, &mime))
                .await
                .map_err(|e| e.to_string())?;

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO inbox_items (id, gmail_id, thread_id, from_addr, subject, snippet, \
                                      received_at, attachment_id, ocr_text, ocr_amount, ocr_status, status, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?)",
        )
        .bind(&id)
        .bind(&email.id)
        .bind(&email.thread_id)
        .bind(&email.from)
        .bind(&email.subject)
        .bind(&email.snippet)
        .bind(&email.date)
        .bind(&attachment_id)
        .bind(&ocr_text)
        .bind(ocr_amount)
        .bind(ocr_status.as_str())
        .bind(&now)
        .execute(&st.pool)
        .await
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}
