//! Editable, ordered string lists used to populate dropdowns (provider kinds).
//! Values live in the `option_lists` table; the whole list is replaced on save
//! so ordering and membership stay exactly as the operator set.

use sqlx::SqlitePool;

/// Utility categories offered when recording a property's providers.
pub const PROVIDER_KINDS: &str = "provider_kinds";

pub const PROVIDER_KIND_DEFAULTS: &[&str] =
    &["electricity", "water", "gas", "trash", "internet", "other"];

/// The editable lists exposed to the API, with their default seed values.
pub const LISTS: &[(&str, &[&str])] = &[(PROVIDER_KINDS, PROVIDER_KIND_DEFAULTS)];

/// Whether `list` is a known, editable option list.
pub fn is_known(list: &str) -> bool {
    LISTS.iter().any(|(name, _)| *name == list)
}

/// The values of `list`, in stored order.
pub async fn values(pool: &SqlitePool, list: &str) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        "SELECT value FROM option_lists WHERE list = ? ORDER BY position, value",
    )
    .bind(list)
    .fetch_all(pool)
    .await
}

/// Replaces every value of `list` with `values`, trimming blanks and dropping
/// case-insensitive duplicates while preserving the given order.
pub async fn replace(
    pool: &SqlitePool,
    list: &str,
    values: &[String],
) -> Result<Vec<String>, sqlx::Error> {
    let cleaned = dedupe(values);
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM option_lists WHERE list = ?")
        .bind(list)
        .execute(&mut *tx)
        .await?;
    for (position, value) in cleaned.iter().enumerate() {
        sqlx::query("INSERT INTO option_lists (list, value, position) VALUES (?, ?, ?)")
            .bind(list)
            .bind(value)
            .bind(position as i64)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(cleaned)
}

fn dedupe(values: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for raw in values {
        let value = raw.trim().to_string();
        if value.is_empty() {
            continue;
        }
        if seen.insert(value.to_lowercase()) {
            out.push(value);
        }
    }
    out
}
