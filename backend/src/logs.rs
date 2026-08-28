//! Application event log. Persists error/warning events — especially unattended
//! background-job failures — so they are both troubleshootable on the admin Logs
//! page and, for failures, surfaced in the notification tray.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::LogEntry;

pub const ERROR: &str = "error";
pub const WARNING: &str = "warning";

/// Persists a log entry. Best-effort: a logging failure must never mask the
/// original error, so it is only traced, never propagated.
pub async fn record(pool: &SqlitePool, level: &str, source: &str, message: &str) {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = crate::db::execute(
        pool,
        sqlx::query(
            "INSERT INTO app_logs (id, level, source, message, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(level)
        .bind(source)
        .bind(message)
        .bind(&now),
    )
    .await
    {
        tracing::error!("failed to persist log entry: {e}");
    }
}

/// Records an error and raises a notification for it, so an unattended failure
/// (e.g. a daily job) is visible in the tray as well as the Logs page. Repeated
/// failures of the same `source` collapse into a single active notification that
/// reflects the latest message and re-surfaces even if a prior one was dismissed.
pub async fn record_failure(pool: &SqlitePool, source: &str, message: &str) {
    tracing::error!(source, "{message}");
    record(pool, ERROR, source, message).await;

    let dedup_key = format!("job_error:{source}");
    clear_notification(pool, &dedup_key).await;
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = crate::db::execute(
        pool,
        sqlx::query(
            "INSERT INTO notifications \
             (id, kind, severity, title, body, link, property_id, dedup_key, auto_resolve, created_at, dismissed_at) \
             VALUES (?, 'system_error', 'error', ?, ?, '/admin', NULL, ?, 0, ?, NULL)",
        )
        .bind(&id)
        .bind(format!("{source} failed"))
        .bind(message)
        .bind(&dedup_key)
        .bind(&now),
    )
    .await
    {
        tracing::error!("failed to raise failure notification: {e}");
    }
}

/// Clears the active failure notification for `source` once it succeeds again,
/// so a recovered job no longer shows an error in the tray.
pub async fn clear_failure(pool: &SqlitePool, source: &str) {
    clear_notification(pool, &format!("job_error:{source}")).await;
}

async fn clear_notification(pool: &SqlitePool, dedup_key: &str) {
    if let Err(e) = crate::db::execute(
        pool,
        sqlx::query("DELETE FROM notifications WHERE dedup_key = ?").bind(dedup_key),
    )
    .await
    {
        tracing::error!("failed to clear failure notification: {e}");
    }
}

/// Recent log entries, newest first, capped so the admin page stays light.
pub async fn list(pool: &SqlitePool, limit: i64) -> Result<Vec<LogEntry>, sqlx::Error> {
    crate::db::fetch_all(
        pool,
        sqlx::query_as::<_, LogEntry>(
            "SELECT id, level, source, message, created_at FROM app_logs \
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit),
    )
    .await
}

/// Deletes every log entry.
pub async fn clear(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    crate::db::execute(pool, sqlx::query("DELETE FROM app_logs")).await?;
    Ok(())
}
