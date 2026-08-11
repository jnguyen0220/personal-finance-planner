pub mod attachments;
pub mod categories;
pub mod insurance;
pub mod leases;
pub mod messages;
pub mod notifications;
pub mod properties;
pub mod providers;
pub mod settings;
pub mod states;
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
