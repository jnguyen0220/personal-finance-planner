//! Runtime-toggleable application settings, persisted in the `settings` table.

use sqlx::SqlitePool;

/// Master switch for the automated tenant messaging job.
pub const MESSAGING_ENABLED: &str = "messaging_enabled";

/// Master switch for the lease/insurance expiry texts sent to contact phones.
pub const PROPERTY_MESSAGING_ENABLED: &str = "property_messaging_enabled";

/// Signature/sign-off appended to automated messages via the `{signature}` token.
pub const SIGNATURE: &str = "signature";

/// Seeded as the signature on a fresh database.
pub const SIGNATURE_DEFAULT: &str = "Landlord";

/// Days before a lease ends to start reminding, applied to every lease.
pub const LEASE_NOTIFY_DAYS: &str = "lease_notify_days";

/// Days before a policy expires to start reminding, applied to every policy.
pub const INSURANCE_NOTIFY_DAYS: &str = "insurance_notify_days";

/// Default reminder lead time when the operator hasn't chosen one.
pub const NOTIFY_DAYS_DEFAULT: i64 = 30;

/// Phone numbers that receive SMS reminders for lease and insurance expiry.
pub const CONTACT_PHONES: &str = "contact_phones";

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

pub async fn get_string(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
}
pub async fn set_string(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, key: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM settings WHERE key = ?")
        .bind(key)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_i64(pool: &SqlitePool, key: &str, default: i64) -> Result<i64, sqlx::Error> {
    let value = get_string(pool, key).await?;
    Ok(value.and_then(|v| v.trim().parse().ok()).unwrap_or(default))
}

pub async fn set_i64(pool: &SqlitePool, key: &str, value: i64) -> Result<(), sqlx::Error> {
    set_string(pool, key, &value.to_string()).await
}

/// Reads a JSON string-array setting, yielding an empty list when unset or malformed.
pub async fn get_list(pool: &SqlitePool, key: &str) -> Result<Vec<String>, sqlx::Error> {
    let value = get_string(pool, key).await?;
    Ok(value
        .and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
        .unwrap_or_default())
}

pub async fn set_list(pool: &SqlitePool, key: &str, values: &[String]) -> Result<(), sqlx::Error> {
    let json = serde_json::to_string(values).unwrap_or_else(|_| "[]".to_string());
    set_string(pool, key, &json).await
}
