use sqlx::SqlitePool;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub uploads: PathBuf,
    /// When the process started, anchoring the once-a-day scheduler.
    pub started_at: chrono::DateTime<chrono::Utc>,
}
