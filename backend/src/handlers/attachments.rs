use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::Attachment;
use crate::state::AppState;

const MAX_SIZE: usize = 20 * 1024 * 1024; // 20 MB

fn allowed_extension(content_type: &str, original: &str) -> Option<&'static str> {
    let ext = std::path::Path::new(original)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match (content_type, ext.as_str()) {
        ("image/jpeg", _) | (_, "jpg") | (_, "jpeg") => Some("jpg"),
        ("image/png", _) | (_, "png") => Some("png"),
        ("image/gif", _) | (_, "gif") => Some("gif"),
        ("image/webp", _) | (_, "webp") => Some("webp"),
        ("application/pdf", _) | (_, "pdf") => Some("pdf"),
        _ => None,
    }
}

/// Persists raw bytes as an attachment (file on disk + database row), returning
/// the new attachment id. Shared by uploads and inbound email ingestion.
pub(crate) async fn store_bytes(
    st: &AppState,
    original_name: &str,
    content_type: &str,
    data: &[u8],
) -> AppResult<String> {
    let ext = allowed_extension(content_type, original_name).ok_or_else(|| {
        AppError::BadRequest("only JPG, PNG, GIF, WEBP or PDF files are allowed".into())
    })?;
    if data.len() > MAX_SIZE {
        return Err(AppError::BadRequest("file exceeds 20 MB limit".into()));
    }

    let id = Uuid::new_v4().to_string();
    let stored_name = format!("{id}.{ext}");
    tokio::fs::write(st.uploads.join(&stored_name), data).await?;

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO attachments (id, stored_name, original_name, content_type, size, uploaded_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&stored_name)
    .bind(original_name)
    .bind(content_type)
    .bind(data.len() as i64)
    .bind(&now)
    .execute(&st.pool)
    .await?;
    Ok(id)
}

pub async fn upload(
    State(st): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<Attachment>> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let original = field.file_name().unwrap_or("upload").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        let id = store_bytes(&st, &original, &content_type, &data).await?;

        let attachment = sqlx::query_as::<_, Attachment>(
            "SELECT id, stored_name, original_name, content_type, size, uploaded_at \
             FROM attachments WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&st.pool)
        .await?;
        return Ok(Json(attachment));
    }
    Err(AppError::BadRequest("no file field provided".into()))
}

pub async fn download(State(st): State<AppState>, Path(id): Path<String>) -> AppResult<Response> {
    let attachment = sqlx::query_as::<_, Attachment>(
        "SELECT id, stored_name, original_name, content_type, size, uploaded_at \
         FROM attachments WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&st.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let path = st.uploads.join(&attachment.stored_name);
    let bytes = tokio::fs::read(&path).await?;

    let disposition = format!("inline; filename=\"{}\"", sanitize(&attachment.original_name));
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, attachment.content_type),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        Body::from(bytes),
    )
        .into_response())
}

fn sanitize(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '"' && *c != '\r' && *c != '\n')
        .collect()
}
