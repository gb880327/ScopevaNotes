use tauri::{AppHandle, State};

use crate::models::AppResult;
use crate::state::AppState;

/// Esc / panel close button. Kept separate from `toggle_quick_tabs` so the
/// frontend can hide unconditionally without reading visibility state first.
#[tauri::command]
pub async fn hide_quick_tabs(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.quick_tabs.hide(&app)
}

/// Every entry point (shortcut, tray, UI) funnels through the same manager.
#[tauri::command]
pub async fn toggle_main_window(app: AppHandle, state: State<'_, AppState>) -> AppResult<bool> {
    state.window.toggle(&app)
}
