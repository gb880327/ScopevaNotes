use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::models::{AppError, AppResult};
use crate::state::AppState;

pub const DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+Space";

pub fn register<R: Runtime>(app: &AppHandle<R>, accelerator: &str) -> AppResult<Shortcut> {
    let shortcut: Shortcut = accelerator
        .parse()
        .map_err(|_| AppError::Invalid(format!("invalid shortcut: {accelerator}")))?;
    app.global_shortcut()
        .register(shortcut)
        .map_err(|err| AppError::Conflict(format!("shortcut unavailable: {err}")))?;
    Ok(shortcut)
}

/// Called from the plugin handler for every registered accelerator.
pub fn on_pressed<R: Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Err(err) = state.window.toggle(app) {
            eprintln!("[window] toggle failed: {err}");
        }
    }
}
