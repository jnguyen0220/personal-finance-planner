use sqlx::SqlitePool;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub uploads: PathBuf,
}
