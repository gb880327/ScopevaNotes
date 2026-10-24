pub mod libraries;
pub mod notes;
pub mod shortcuts;
pub mod tags;
pub mod window;

use crate::models::{AppError, AppResult};

/// Keep every SQLite call off the async runtime (plan §3).
pub(crate) async fn blocking<T, F>(work: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|err| AppError::Other(format!("background task failed: {err}")))?
}
