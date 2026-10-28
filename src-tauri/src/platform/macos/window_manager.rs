use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use objc2::ClassType;
use objc2_app_kit::NSPanel;

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

objc2::define_class!(
    /// `NSPanel` subclass for the main window.
    ///
    /// A plain `NSWindow` is never drawn inside another app's full-screen Space,
    /// however high its level: AppKit keeps it on the Space where it may become
    /// key, so showing it there silently moves the user back to that desktop
    /// instead (measured on device — see verification.md). The menu-bar-panel
    /// recipe works:
    ///
    /// * class swap to `NSPanel`, which AppKit treats as auxiliary;
    /// * `NSWindowStyleMaskNonactivatingPanel`, so the panel may take keyboard
    ///   input while joining that Space instead of dragging the app out of it;
    /// * `canJoinAllSpaces | fullScreenAuxiliary`, so it is allowed in.
    ///
    /// The activating behaviour a document window needs is restored in
    /// `becomeKeyWindow`: without it a click focuses the window but leaves the
    /// menu bar on the other app.
    #[unsafe(super(NSPanel))]
    #[name = "ScopevaMainWindowPanel"]
    #[ivars = ()]
    struct MainWindowPanel;

    impl MainWindowPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            true
        }

        #[unsafe(method(becomeKeyWindow))]
        fn become_key_window(&self) {
            // SAFETY: AppKit calls this on the main thread; `super(self)` only
            // forwards to NSPanel's own implementation.
            unsafe {
                let _: () = objc2::msg_send![super(self), becomeKeyWindow];
            }
            if let Some(mtm) = objc2::MainThreadMarker::new() {
                // `activate` is macOS 14+; the bundle targets 12.0.
                #[allow(deprecated)]
                objc2_app_kit::NSApplication::sharedApplication(mtm)
                    .activateIgnoringOtherApps(true);
            }
        }
    }
);

/// Turns the main window into a panel that may be shown inside another app's
/// full-screen Space (spec: 全屏应用之上显示主窗口).
///
/// Must be called on the main thread once, right after the window exists: the
/// class, style mask and collection behaviour are properties of the window
/// object, and Tauri's `show` / `hide` / `set_always_on_top` never touch them
/// (design D2).
pub fn prepare_main_window<R: Runtime>(window: &WebviewWindow<R>) -> AppResult<()> {
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{NSWindowCollectionBehavior, NSWindowStyleMask};

    let handle = window
        .ns_window()
        .map_err(|err| AppError::Other(format!("no NSWindow handle: {err}")))?;

    // SAFETY: `ns_window()` hands us tao's live window object and we are on the
    // main thread, the only thread AppKit allows. `object_setClass` is sound
    // here because `NSPanel` adds no instance variables (456 vs 464 bytes of
    // storage) and `MainWindowPanel` declares none; after the swap the object is
    // only used through `NSPanel` / `NSWindow` messages.
    unsafe {
        objc2::ffi::object_setClass(handle as *mut AnyObject, MainWindowPanel::class());

        let panel = &*(handle as *const NSPanel);
        panel.setStyleMask(panel.styleMask() | NSWindowStyleMask::NonactivatingPanel);
        panel.setHidesOnDeactivate(false);
        panel.setBecomesKeyOnlyIfNeeded(false);
        panel.setCollectionBehavior(
            panel.collectionBehavior()
                | NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }
    Ok(())
}
