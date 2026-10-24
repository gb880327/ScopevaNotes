//! IPC adapters for the batch Markdown importer. All logic lives in
//! `services::import_service` so it can be tested without a Tauri runtime.

use tauri::{Emitter, State};

use crate::commands::blocking;
use crate::models::{AppError, AppResult};
use crate::services::import_service::{self, ImportOptions, ImportPreview, ImportReport};
use crate::state::AppState;

pub const PROGRESS_EVENT: &str = "ui://import-progress";

#[tauri::command]
pub async fn import_scan(state: State<'_, AppState>, path: String) -> AppResult<ImportPreview> {
    let db = state.db();
    blocking(move || import_service::scan(&db, &path)).await
}

#[tauri::command]
pub async fn import_notes(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
    options: Option<ImportOptions>,
) -> AppResult<ImportReport> {
    let db = state.db();
    let guard = state
        .begin_import()
        .ok_or_else(|| AppError::Busy("已有导入在进行中".into()))?;
    let cancel = guard.cancel_flag();
    let options = options.unwrap_or_default();

    blocking(move || {
        // The guard lives in the blocking thread and clears the flags on drop.
        let _guard = guard;
        import_service::run_import(&db, &cancel, &path, options, &mut |progress| {
            let _ = app.emit(PROGRESS_EVENT, progress);
        })
    })
    .await
}

/// Asks a running import to stop at the next file / batch boundary. No-op when
/// nothing is running.
#[tauri::command]
pub async fn cancel_import(state: State<'_, AppState>) -> AppResult<()> {
    state.request_import_cancel();
    Ok(())
}

#[tauri::command]
pub async fn delete_import_batch(state: State<'_, AppState>, tag: String) -> AppResult<usize> {
    if !tag.starts_with(import_service::BATCH_TAG_PREFIX) {
        return Err(AppError::Invalid(format!("not an import batch tag: {tag}")));
    }
    let db = state.db();
    blocking(move || {
        db.with_conn(|conn| crate::repositories::note_repo::soft_delete_by_tag(conn, &tag))
    })
    .await
}
