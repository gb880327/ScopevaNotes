use tauri::{AppHandle, State};

use crate::models::AppResult;
use crate::platform::macos::shortcuts;
use crate::state::AppState;

#[tauri::command]
pub async fn get_global_shortcut(state: State<'_, AppState>) -> AppResult<String> {
    Ok(state.shortcut())
}

/// Applies and persists (via the frontend store) a new accelerator.
#[tauri::command]
pub async fn set_global_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
    accelerator: String,
) -> AppResult<String> {
    shortcuts::apply(&app, &state, &accelerator)
}
