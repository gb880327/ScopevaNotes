use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State};

use crate::commands::blocking;
use crate::database::{backup, Database};
use crate::models::AppResult;
use crate::services::library_service::{self, AppConfig, LibraryInfo, NoticeKind};
use crate::state::AppState;

#[tauri::command]
pub async fn get_library_info(state: State<'_, AppState>) -> AppResult<LibraryInfo> {
    Ok(state.library())
}

#[tauri::command]
pub async fn create_library(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<LibraryInfo> {
    let target = PathBuf::from(path);
    library_service::validate_new_path(&target)?;
    let db = blocking(move || Database::open(&target)).await?;
    let info = library_service::info_for(&db, false)?;
    library_service::save_config(
        &app,
        &AppConfig {
            library_path: Some(info.path.clone()),
        },
    )?;
    state.replace_library(db, info.clone());
    // Broadcast: the 速记 panel holds its own copy and must reload, otherwise
    // it keeps writing drafts into the previous library (design D13).
    let _ = app.emit("ui://library-changed", &info);
    Ok(info)
}

#[tauri::command]
pub async fn open_library(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<LibraryInfo> {
    let target = PathBuf::from(path);
    let checked = target.clone();
    blocking(move || library_service::validate_existing_path(&checked)).await?;
    let db = blocking(move || Database::open(&target)).await?;
    let info = library_service::info_for(&db, false)?;
    library_service::save_config(
        &app,
        &AppConfig {
            library_path: Some(info.path.clone()),
        },
    )?;
    state.replace_library(db, info.clone());
    // Broadcast: the 速记 panel holds its own copy and must reload, otherwise
    // it keeps writing drafts into the previous library (design D13).
    let _ = app.emit("ui://library-changed", &info);
    Ok(info)
}

/// Migrates the current library into `path` as a consistent snapshot, then
/// switches to it. The old file is left untouched for recovery (plan §1.8).
#[tauri::command]
pub async fn move_library(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<LibraryInfo> {
    let target = PathBuf::from(path);
    library_service::validate_new_path(&target)?;

    let current = state.db();
    let snapshot = target.clone();
    blocking(move || current.with_conn(|conn| backup::backup_to(conn, &snapshot))).await?;

    let db = blocking(move || Database::open(&target)).await?;
    let info = library_service::info_for(&db, false)?;
    library_service::save_config(
        &app,
        &AppConfig {
            library_path: Some(info.path.clone()),
        },
    )?;
    state.replace_library(db, info.clone());
    // Broadcast: the 速记 panel holds its own copy and must reload, otherwise
    // it keeps writing drafts into the previous library (design D13).
    let _ = app.emit("ui://library-changed", &info);
    Ok(info)
}

#[tauri::command]
pub async fn backup_library(state: State<'_, AppState>, destination: String) -> AppResult<u64> {
    let db = state.db();
    let target = PathBuf::from(destination);
    blocking(move || db.with_conn(|conn| backup::backup_to(conn, &target))).await
}

/// Acknowledge the library notice. `use_default` also rewrites `config.json` to
/// the default location, which is what the fallback banner offers; it is ignored
/// for advisory notices (cloud sync) so the user's path is never discarded.
#[tauri::command]
pub async fn dismiss_library_notice(
    app: AppHandle,
    state: State<'_, AppState>,
    use_default: bool,
) -> AppResult<LibraryInfo> {
    let is_fallback = matches!(
        state.library().notice.map(|notice| notice.kind),
        Some(NoticeKind::Fallback)
    );
    if use_default && is_fallback {
        library_service::save_config(
            &app,
            &AppConfig {
                library_path: None,
            },
        )?;
    }
    Ok(state.clear_notice())
}
