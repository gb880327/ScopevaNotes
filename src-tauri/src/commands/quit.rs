use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::state::AppState;

/// Called by the frontend *after* it flushed pending edits, so the dialog never
/// appears before the save finished. Confirming flips the state flag and exits;
/// cancelling leaves the app running with its window state untouched.
#[tauri::command]
pub fn request_quit(app: AppHandle) -> Result<(), String> {
    if let Some(state) = app.try_state::<AppState>() {
        if !state.begin_quit_prompt() {
            return Ok(());
        }
    }
    let handle = app.clone();
    app.dialog()
        .message("所有编辑已保存。退出后 Notes 将不再驻留菜单栏。")
        .title("退出 Notes？")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "退出".into(),
            "取消".into(),
        ))
        .show(move |confirmed| {
            if !confirmed {
                if let Some(state) = handle.try_state::<AppState>() {
                    state.end_quit_prompt();
                }
                return;
            }
            if let Some(state) = handle.try_state::<AppState>() {
                state.allow_exit();
            }
            handle.exit(0);
        });
    Ok(())
}
