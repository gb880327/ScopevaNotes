use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, Runtime, WebviewWindow};

use crate::models::{AppError, AppResult};
use crate::platform::macos::monitor;
use crate::services::library_service::{ConfigCache, SizeSlot};

/// Used until the user resizes the window, and again if the remembered size is
/// unusable (spec: 显示时定位与置顶 / 尺寸配置损坏不阻断启动).
const DEFAULT_SIZE: (f64, f64) = (1100.0, 720.0);
const MIN_SIZE: (f64, f64) = (760.0, 480.0);

/// Single owner of show/hide state. Every trigger (shortcut, tray, IPC,
/// close button) goes through `toggle`, and a mutex serialises rapid presses
/// so the state machine cannot flip out of order (plan §6.1).
#[derive(Default)]
pub struct WindowManager {
    visible: AtomicBool,
    saved_size: Mutex<Option<(f64, f64)>>,
    /// Present once the app is wired up; the manager itself stays usable
    /// without it (tests, early startup) and then just skips persistence.
    config: Option<Arc<ConfigCache>>,
    gate: Mutex<()>,
}

impl WindowManager {
    pub const LABEL: &'static str = "main";

    /// Seeded with the size remembered in `config.json`, so the first show
    /// after a restart uses what the user dragged to (spec: 调整尺寸后重启仍是
    /// 上次的尺寸).
    pub fn new(config: Arc<ConfigCache>, size: Option<(f64, f64)>) -> Self {
        Self {
            config: Some(config),
            saved_size: Mutex::new(size),
            ..Default::default()
        }
    }

    pub fn toggle<R: Runtime>(&self, app: &AppHandle<R>) -> AppResult<bool> {
        let _gate = self.gate.lock().expect("window gate poisoned");
        let window = self.window(app)?;
        if window.is_visible().unwrap_or(false) {
            self.hide_window(&window, app)?;
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
        self.hide_window(&window, app)
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
        // The size comes from us, never from `outer_size()`: a window that has
        // never been shown reports a size that is off by its scale factor, so
        // remembering it would compound the error on every show.
        let remembered = *self.saved_size.lock().expect("size lock poisoned");
        self.place_on_pointer_monitor(window, app, remembered)?;

        // Strictly on top while visible; the plan accepts the trade-off.
        window.set_always_on_top(true)?;
        window.show()?;
        window.set_focus()?;
        self.visible.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn hide_window<R: Runtime>(
        &self,
        window: &WebviewWindow<R>,
        app: &AppHandle<R>,
    ) -> AppResult<()> {
        if let Ok(size) = window.inner_size() {
            let scale = window
                .current_monitor()
                .ok()
                .flatten()
                .map(|monitor| monitor.scale_factor())
                .unwrap_or(1.0);
            let logical = (size.width as f64 / scale, size.height as f64 / scale);
            *self.saved_size.lock().expect("size lock poisoned") = Some(logical);
            // Persisting is a convenience: a failed write must not stop the
            // window from hiding (design D3).
            if let Some(config) = &self.config {
                if let Err(err) = config.set_size(app, SizeSlot::Main, logical) {
                    eprintln!("[window] size not persisted: {err}");
                }
            }
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
        remembered: Option<(f64, f64)>,
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
            position.x as f64 / scale,
            position.y as f64 / scale,
            size.width as f64 / scale,
            size.height as f64 / scale,
        );

        let (width, height) = monitor::clamp_window_size(remembered, DEFAULT_SIZE, MIN_SIZE, area);
        window.set_size(LogicalSize::new(width, height))?;

        let (x, y) = monitor::centered_origin(area, (width, height));
        window.set_position(PhysicalPosition::new(
            (x * scale).round() as i32,
            (y * scale).round() as i32,
        ))?;
        Ok(())
    }
}

/// Lets the main window appear on top of another app's full-screen Space
/// (spec: 全屏应用之上显示主窗口).
///
/// The level set by `set_always_on_top` is irrelevant on its own: a window that
/// is not a member of the full-screen Space is simply not drawn there, no matter
/// how high it floats. `CanJoinAllSpaces` puts it on every desktop and
/// `FullScreenAuxiliary` lets it be an auxiliary window over some other app's
/// full-screen window (design D1).
///
/// Must be called on the main thread; ignores the failure of the underlying
/// window handle, because a window that cannot join is still usable, it just
/// stays on its own desktop.
pub fn join_fullscreen_spaces<R: Runtime>(window: &WebviewWindow<R>) -> AppResult<()> {
    use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};

    let handle = window
        .ns_window()
        .map_err(|err| AppError::Other(format!("no NSWindow handle: {err}")))?;

    // SAFETY: `ns_window()` hands us tao's live window object and we are on the
    // main thread, which is the only thread AppKit allows here. We only read and
    // write `collectionBehavior` — a plain property on that object — and never
    // change its class or its level.
    unsafe {
        let ns_window = &*(handle as *const NSWindow);
        let behavior = ns_window.collectionBehavior()
            | NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary;
        ns_window.setCollectionBehavior(behavior);
    }
    Ok(())
}
