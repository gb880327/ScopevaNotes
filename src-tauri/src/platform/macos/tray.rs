use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};

use tauri_plugin_autostart::ManagerExt;

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
    // Left click now owns the 速记 panel, so this item must say what it does:
    // the main window (design D4). The id stays `toggle` for one reason only —
    // its handler is unchanged.
    let toggle = MenuItem::with_id(app, "toggle", "显示主窗口", true, None::<&str>)?;
    let new_note = MenuItem::with_id(
        app,
        "new_note",
        "新建笔记",
        true,
        Some("CommandOrControl+N"),
    )?;
    let import = MenuItem::with_id(app, "import", "导入 Markdown…", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置…", true, Some("CommandOrControl+,"))?;
    let quit = MenuItem::with_id(app, "quit", "退出 Notes", true, Some("CommandOrControl+Q"))?;
    let separator = PredefinedMenuItem::separator(app)?;
    // Mirrors the real LaunchAgent state instead of a stored preference.
    let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "开机启动",
        true,
        autostart_enabled,
        None::<&str>,
    )?;
    let autostart_item = autostart.clone();

    let menu = Menu::with_items(
        app,
        &[
            &toggle, &new_note, &separator, &autostart, &separator, &import, &settings, &separator,
            &quit,
        ],
    )?;

    let icon = Image::from_bytes(TRAY_ICON)
        .map_err(|err| AppError::Other(format!("tray icon decode failed: {err}")))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(false)
        .tooltip("Notes")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "toggle" => click_toggle(app),
            "new_note" => {
                let _ = app.emit("ui://new-note", ());
                click_toggle(app);
            }
            // Import is not a toggle: the dialog needs a visible window.
            "import" => {
                if let Some(state) = app.try_state::<AppState>() {
                    if let Err(err) = state.window.show(app) {
                        eprintln!("[tray] show failed: {err}");
                    }
                }
                let _ = app.emit("ui://import-markdown", ());
            }
            "settings" => {
                let _ = app.emit("ui://open-settings", ());
                click_toggle(app);
            }
            "autostart" => {
                let manager = app.autolaunch();
                let enabled = manager.is_enabled().unwrap_or(false);
                let result = if enabled {
                    manager.disable()
                } else {
                    manager.enable()
                };
                if let Err(err) = result {
                    eprintln!("[autostart] toggle failed: {err}");
                }
                // Trust the system after the call, not the click.
                let actual = manager.is_enabled().unwrap_or(!enabled);
                let _ = autostart_item.set_checked(actual);
            }
            // Same path as ⌘Q: the frontend flushes, then asks for confirmation.
            "quit" => {
                let _ = app.emit("ui://quit-requested", ());
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // `rect` is the icon's own rect: it is the only way to hang the
            // panel off the right spot on a multi-display setup (design D6).
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(state) = app.try_state::<AppState>() {
                    if let Err(err) = state.quick_tabs.toggle_from_tray(app, rect) {
                        eprintln!("[tray] quick tabs toggle failed: {err}");
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// Menu items and the shortcut still target the main window; only the plain
/// left click was re-pointed at the panel.
fn click_toggle<R: Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Err(err) = state.window.toggle(app) {
            eprintln!("[tray] toggle failed: {err}");
        }
    }
}
