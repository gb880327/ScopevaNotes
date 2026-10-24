mod commands;
pub mod database;
pub mod models;
pub mod platform;
pub mod repositories;
pub mod services;
pub mod state;

use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_global_shortcut::ShortcutState;

use crate::platform::macos::{shortcuts, tray};
use crate::services::library_service;
use crate::state::AppState;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(state) = app.try_state::<AppState>() {
                let _ = state.window.show(app);
            }
        }))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        shortcuts::on_pressed(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();

            let (db, info) = match library_service::resolve(&handle) {
                Ok(pair) => pair,
                Err(err) => {
                    // Never leave the user without a window: fall back to the
                    // default library and surface the failure once the library
                    // UI exists (M1/M5).
                    eprintln!("[library] falling back to default library: {err}");
                    let path = library_service::default_library_path(&handle)?;
                    let db = database::Database::open(&path)?;
                    let info = library_service::info_for(&db, true)?;
                    (db, info)
                }
            };
            app.manage(AppState::new(
                db,
                info,
                shortcuts::DEFAULT_SHORTCUT.to_string(),
            ));

            tray::setup(&handle)?;
            if let Err(err) = shortcuts::register(&handle, shortcuts::DEFAULT_SHORTCUT) {
                // A taken shortcut must not abort startup.
                eprintln!("[shortcut] registration failed: {err}");
            }

            // Silent start: the window stays hidden until the tray, the
            // shortcut or Dock activation asks for it (spec: 启动不显示主窗口).
            show_first_run_tip(&handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Close button hides; only the tray "Quit" exits (plan §1.3).
                api.prevent_close();
                let app = window.app_handle();
                if let Some(state) = app.try_state::<AppState>() {
                    if let Err(err) = state.window.hide(app) {
                        eprintln!("[window] hide failed: {err}");
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::notes::list_notes,
            commands::notes::list_deleted_notes,
            commands::notes::get_note,
            commands::notes::create_note,
            commands::notes::update_note,
            commands::notes::set_note_favorite,
            commands::notes::soft_delete_note,
            commands::notes::restore_note,
            commands::tags::list_tags,
            commands::tags::create_tag,
            commands::tags::rename_tag,
            commands::tags::delete_tag,
            commands::libraries::get_library_info,
            commands::libraries::create_library,
            commands::libraries::open_library,
            commands::libraries::move_library,
            commands::libraries::backup_library,
            commands::window::toggle_main_window,
            commands::shortcuts::get_global_shortcut,
            commands::shortcuts::set_global_shortcut,
            commands::quit::request_quit,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // ⌘Q, Dock quit and the tray item all arrive here (or emit the
            // event themselves). Block the exit until the user confirmed;
            // `allow_exit` is set by the confirmation dialog.
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if let Some(state) = app.try_state::<AppState>() {
                    if !state.exit_allowed() {
                        api.prevent_exit();
                        // If no webview is listening (early startup or a
                        // crashed frontend) there is nothing to save or ask.
                        if app.emit("ui://quit-requested", ()).is_err() {
                            state.allow_exit();
                            app.exit(0);
                        }
                    }
                }
            }
        });
}

/// Shown once, while the window stays hidden, so a silent start is not
/// mistaken for a failed launch. The marker lives outside the settings store
/// so it cannot race the frontend's writes.
fn show_first_run_tip(app: &tauri::AppHandle) {
    let Ok(dir) = app.path().app_config_dir() else {
        return;
    };
    let marker = dir.join("onboarding-shortcut-tip");
    if marker.exists() {
        return;
    }
    let accelerator = app
        .try_state::<AppState>()
        .map(|state| shortcuts::display(&state.shortcut()))
        .unwrap_or_else(|| shortcuts::display(shortcuts::DEFAULT_SHORTCUT));
    app.dialog()
        .message(format!(
            "Notes 已在菜单栏运行，不会自动打开窗口。\n\n按 {accelerator} 显示或隐藏窗口；也可以在菜单栏图标或设置的「全局快捷键」里换成其他组合。"
        ))
        .title("Notes")
        .kind(MessageDialogKind::Info)
        .show(|_| {});
    if std::fs::create_dir_all(&dir).is_ok() {
        let _ = std::fs::write(&marker, b"1");
    }
}
