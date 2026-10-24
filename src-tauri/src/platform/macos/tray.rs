use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};

use crate::models::{AppError, AppResult};
use crate::state::AppState;

/// Brand mark used in the menu bar. Rendered by AppKit at 18pt height, so a
/// high-resolution source stays sharp on Retina displays. The logo is a
/// colored rounded square, hence `icon_as_template(false)`: a template icon
/// would collapse it into a solid silhouette.
const TRAY_ICON: &[u8] = include_bytes!("../../../icons/tray-logo.png");

pub fn setup<R: Runtime>(app: &AppHandle<R>) -> AppResult<()> {
    // No accelerator label here: the shortcut is user-configurable, and a
    // stale hint is worse than none. Settings shows the live value.
    let toggle = MenuItem::with_id(app, "toggle", "显示 / 隐藏窗口", true, None::<&str>)?;
    let new_note = MenuItem::with_id(app, "new_note", "新建笔记", true, Some("CommandOrControl+N"))?;
    let library = MenuItem::with_id(app, "library", "笔记库…", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置…", true, Some("CommandOrControl+,"))?;
    let quit = MenuItem::with_id(app, "quit", "退出 Notes", true, Some("CommandOrControl+Q"))?;
    let separator = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[&toggle, &new_note, &separator, &library, &settings, &separator, &quit],
    )?;

    let icon = Image::from_bytes(TRAY_ICON)
        .map_err(|err| AppError::Other(format!("tray icon decode failed: {err}")))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(false)
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
