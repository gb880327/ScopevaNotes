use tauri::{AppHandle, State};

use crate::models::AppResult;
use crate::state::AppState;

/// Every entry point (shortcut, tray, UI) funnels through the same manager.
#[tauri::command]
pub async fn toggle_main_window(app: AppHandle, state: State<'_, AppState>) -> AppResult<bool> {
    state.window.toggle(&app)
}
