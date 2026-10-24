use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};

use crate::models::AppResult;
use crate::state::AppState;

/// macOS tray. TODO(M0): replace the default app icon with a monochrome
/// template icon (32x32 @2x, `isTemplate = true`) before release.
pub fn setup<R: Runtime>(app: &AppHandle<R>) -> AppResult<()> {
    let toggle = MenuItem::with_id(
        app,
        "toggle",
        "显示 / 隐藏窗口",
        true,
        Some("CommandOrControl+Shift+Space"),
    )?;
    let new_note = MenuItem::with_id(app, "new_note", "新建笔记", true, Some("CommandOrControl+N"))?;
    let library = MenuItem::with_id(app, "library", "笔记库…", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置…", true, Some("CommandOrControl+,"))?;
    let quit = MenuItem::with_id(app, "quit", "退出 Notes", true, Some("CommandOrControl+Q"))?;
    let separator = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[&toggle, &new_note, &separator, &library, &settings, &separator, &quit],
    )?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| crate::models::AppError::Other("missing default window icon".into()))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => click_toggle(app),
            "new_note" => {
                let _ = app.emit("ui://new-note", ());
                click_toggle(app);
            }
            "library" => {
                let _ = app.emit("ui://open-library", ());
                click_toggle(app);
            }
            "settings" => {
                let _ = app.emit("ui://open-settings", ());
                click_toggle(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                click_toggle(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn click_toggle<R: Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Err(err) = state.window.toggle(app) {
            eprintln!("[tray] toggle failed: {err}");
        }
    }
}
