//! Runtime-toggleable application settings, persisted in the `settings` table.

use sqlx::SqlitePool;

/// Master switch for the automated tenant messaging job.
pub const MESSAGING_ENABLED: &str = "messaging_enabled";

pub async fn get_bool(pool: &SqlitePool, key: &str, default: bool) -> Result<bool, sqlx::Error> {
    let value = sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(match value.as_deref() {
        Some("true") | Some("1") => true,
        Some(_) => false,
        None => default,
    })
}

pub async fn set_bool(pool: &SqlitePool, key: &str, value: bool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(if value { "true" } else { "false" })
    .execute(pool)
    .await?;
    Ok(())
}
