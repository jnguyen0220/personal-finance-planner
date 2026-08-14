pub mod attachments;
pub mod categories;
pub mod inbox;
pub mod insurance;
pub mod leases;
pub mod messages;
pub mod notifications;
pub mod options;
pub mod properties;
pub mod providers;
pub mod settings;
pub mod states;
pub mod templates;
pub mod tenants;
pub mod transactions;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Maps a zero affected-row count to a 404, for update/delete guards.
pub(crate) fn ensure_found(affected: u64) -> AppResult<()> {
    if affected == 0 {
        Err(AppError::NotFound)
    } else {
        Ok(())
    }
}

/// Deletes a row by id from `table`, returning 204 or 404. `table` must be a
/// trusted, hard-coded literal — never user input.
pub(crate) async fn delete_by_id(
    st: &AppState,
    table: &str,
    id: &str,
) -> AppResult<axum::http::StatusCode> {
    let affected = sqlx::query(&format!("DELETE FROM {table} WHERE id = ?"))
        .bind(id)
        .execute(&st.pool)
        .await?
        .rows_affected();
    ensure_found(affected)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Removes an attachment's row and its file from disk, so deleting or replacing
/// a receipt or document doesn't leave an orphaned upload behind. Best-effort on
/// the file: a missing one won't fail the caller.
pub(crate) async fn delete_attachment(st: &AppState, id: &str) -> AppResult<()> {
    let stored_name =
        sqlx::query_scalar::<_, String>("SELECT stored_name FROM attachments WHERE id = ?")
            .bind(id)
            .fetch_optional(&st.pool)
            .await?;
    if let Some(stored_name) = stored_name {
        sqlx::query("DELETE FROM attachments WHERE id = ?")
            .bind(id)
            .execute(&st.pool)
            .await?;
        let _ = tokio::fs::remove_file(st.uploads.join(&stored_name)).await;
    }
    Ok(())
}
