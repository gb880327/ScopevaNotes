use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, Runtime, WebviewWindow};

use crate::models::{AppError, AppResult};
use crate::platform::macos::monitor;

/// Single owner of show/hide state. Every trigger (shortcut, tray, IPC,
/// close button) goes through `toggle`, and a mutex serialises rapid presses
/// so the state machine cannot flip out of order (plan §6.1).
#[derive(Default)]
pub struct WindowManager {
    visible: AtomicBool,
    saved_size: Mutex<Option<(f64, f64)>>,
    gate: Mutex<()>,
}

impl WindowManager {
    pub const LABEL: &'static str = "main";

    pub fn toggle<R: Runtime>(&self, app: &AppHandle<R>) -> AppResult<bool> {
        let _gate = self.gate.lock().expect("window gate poisoned");
        let window = self.window(app)?;
        if window.is_visible().unwrap_or(false) {
            self.hide_window(&window)?;
            Ok(false)
        } else {
            self.show_window(&window, app)?;
            Ok(true)
        }
    }

    pub fn show<R: Runtime>(&self, app: &AppHandle<R>) -> AppResult<()> {
        let _gate = self.gate.lock().expect("window gate poisoned");
        let window = self.window(app)?;
        self.show_window(&window, app)
    }

    pub fn hide<R: Runtime>(&self, app: &AppHandle<R>) -> AppResult<()> {
        let _gate = self.gate.lock().expect("window gate poisoned");
        let window = self.window(app)?;
        self.hide_window(&window)
    }

    fn window<R: Runtime>(&self, app: &AppHandle<R>) -> AppResult<WebviewWindow<R>> {
        app.get_webview_window(Self::LABEL)
            .ok_or_else(|| AppError::Other("main window is missing".into()))
    }

    fn show_window<R: Runtime>(
        &self,
        window: &WebviewWindow<R>,
        app: &AppHandle<R>,
    ) -> AppResult<()> {
        if let Some((width, height)) = *self.saved_size.lock().expect("size lock poisoned") {
            window.set_size(LogicalSize::new(width, height))?;
        }

        self.place_on_pointer_monitor(window, app)?;

        // Strictly on top while visible; the plan accepts the trade-off.
        window.set_always_on_top(true)?;
        window.show()?;
        window.set_focus()?;
        self.visible.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn hide_window<R: Runtime>(&self, window: &WebviewWindow<R>) -> AppResult<()> {
        if let Ok(size) = window.inner_size() {
            let scale = window
                .current_monitor()
                .ok()
                .flatten()
                .map(|monitor| monitor.scale_factor())
                .unwrap_or(1.0);
            *self.saved_size.lock().expect("size lock poisoned") =
                Some((size.width as f64 / scale, size.height as f64 / scale));
        }
        window.set_always_on_top(false)?;
        window.hide()?;
        self.visible.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn place_on_pointer_monitor<R: Runtime>(
        &self,
        window: &WebviewWindow<R>,
        app: &AppHandle<R>,
    ) -> AppResult<()> {
        let monitor = app
            .cursor_position()
            .ok()
            .and_then(|cursor| app.monitor_from_point(cursor.x, cursor.y).ok().flatten())
            .or_else(|| app.primary_monitor().ok().flatten());

        let Some(monitor) = monitor else {
            window.center()?;
            return Ok(());
        };

        let scale = monitor.scale_factor();
        let position = monitor.position();
        let size = monitor.size();
        let area = (
            position.x as f64,
            position.y as f64,
            size.width as f64,
            size.height as f64,
        );

        let logical_window = window
            .outer_size()
            .map(|size| {
                (
                    size.width as f64 / scale,
                    size.height as f64 / scale,
                )
            })
            .unwrap_or((1100.0, 720.0));
        let (width, height) = monitor::clamp_to_area(
            (area.0 / scale, area.1 / scale, area.2 / scale, area.3 / scale),
            logical_window,
        );
        window.set_size(LogicalSize::new(width, height))?;

        let (x, y) = monitor::centered_origin(
            (area.0 / scale, area.1 / scale, area.2 / scale, area.3 / scale),
            (width, height),
        );
        window.set_position(PhysicalPosition::new(
            (x * scale).round() as i32,
            (y * scale).round() as i32,
        ))?;
        Ok(())
    }
}
