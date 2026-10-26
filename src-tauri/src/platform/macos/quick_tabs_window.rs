use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, Rect, Runtime, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use objc2::ClassType;
use objc2_app_kit::NSPanel;

use crate::models::{AppError, AppResult};
use crate::platform::macos::monitor;

/// Gap between the menu bar icon and the panel, in logical pixels.
const TRAY_GAP: f64 = 2.0;
/// Gap from the screen top when the menu bar is auto-hidden (fullscreen Space).
const FULLSCREEN_TOP_INSET: f64 = 2.0;

/// Ignore window for the blur/click race (design D5). Clicking the tray icon
/// while the panel is open both blurs the panel *and* sends a click, in an
/// order AppKit does not promise; both orders must end with the panel hidden.
const BLUR_GUARD: Duration = Duration::from_millis(250);

const DEFAULT_SIZE: (f64, f64) = (480.0, 320.0);
const MIN_SIZE: (f64, f64) = (360.0, 220.0);

/// tray-icon on macOS already converts to physical pixels, so scale 1.0 is the
/// correct factor for it; the logical arm only keeps the function total.
fn physical_rect(rect: Rect) -> (f64, f64, f64, f64) {
    let position = rect.position.to_physical::<f64>(1.0);
    let size = rect.size.to_physical::<f64>(1.0);
    (position.x, position.y, size.width, size.height)
}

objc2::define_class!(
    /// `NSPanel` subclass for the 速记 window.
    ///
    /// Swapping away from tao's own window class drops tao's
    /// `canBecomeKeyWindow` override, and a borderless (decorations-off) window
    /// answers `false` there — the panel would show but never take the keyboard.
    /// Answering `true` here is what lets a *non-activating* panel become key,
    /// i.e. accept typing without activating the app.
    #[unsafe(super(NSPanel))]
    #[name = "ScopevaQuickTabsPanel"]
    #[ivars = ()]
    pub struct QuickTabsPanel;

    impl QuickTabsPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            false
        }
    }
);

/// Watch the menu bar for a moment after showing the panel.
///
/// In a fullscreen Space the bar is still revealed while the pointer sits at the
/// top (that is how the user reached the status item), but it slides away as
/// soon as the panel takes focus — leaving the panel floating below a menu bar
/// that no longer exists, which reads as a gap. There is no notification for
/// this, so poll `NSMenu.menuBarVisible` briefly and, the first time it is
/// really gone, snap the panel to the screen top (design D18).
fn follow_menu_bar<R: Runtime>(app: &AppHandle<R>, top_logical: f64) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    const POLL_INTERVAL: Duration = Duration::from_millis(400);
    const POLL_ATTEMPTS: usize = 25;

    let handle = app.clone();
    std::thread::spawn(move || {
        let settled = Arc::new(AtomicBool::new(false));
        for _ in 0..POLL_ATTEMPTS {
            std::thread::sleep(POLL_INTERVAL);
            if settled.load(Ordering::SeqCst) {
                return;
            }
            let inner = handle.clone();
            let flag = Arc::clone(&settled);
            let ok = handle.run_on_main_thread(move || {
                let Some(mtm) = objc2::MainThreadMarker::new() else {
                    return;
                };
                if objc2_app_kit::NSMenu::menuBarVisible(mtm) {
                    return;
                }
                use tauri::Manager as _;
                let state = inner.state::<crate::state::AppState>();
                let Ok(window) = state.quick_tabs.window(&inner) else {
                    return;
                };
                if !window.is_visible().unwrap_or(false) {
                    flag.store(true, Ordering::SeqCst);
                    return;
                }
                let Ok(position) = window.outer_position() else {
                    return;
                };
                let scale = window.scale_factor().unwrap_or(1.0);
                let y = (top_logical * scale).round() as i32;
                if y >= position.y {
                    flag.store(true, Ordering::SeqCst);
                    return;
                }
                if window
                    .set_position(PhysicalPosition::new(position.x, y))
                    .is_ok()
                {
                    flag.store(true, Ordering::SeqCst);
                }
            });
            if ok.is_err() {
                return;
            }
        }
    });
}

/// Turn the panel into a non-activating `NSPanel` that may join other apps'
/// fullscreen Spaces.
///
/// A plain `WebviewWindow` cannot be shown over a fullscreen Space:
/// `visible_on_all_workspaces(true)` only ORs `canJoinAllSpaces` into the
/// collection behaviour — enough to follow the user between ordinary desktops,
/// not enough to enter somebody else's fullscreen Space — and a regular
/// `NSWindow` may not become key there, so the show silently does nothing while
/// activating the app instead yanks the user to the Space the window belongs to
/// (both failure modes were observed on device).
///
/// This is the stock menu-bar-panel recipe:
///
/// * swap the window class to `NSPanel` and add
///   `NSWindowStyleMaskNonactivatingPanel`, so the panel can become key and take
///   keyboard input *without* activating the app (no Space switch);
/// * `canJoinAllSpaces | fullScreenAuxiliary`, so AppKit allows it in that Space;
/// * floating level, so it draws above the fullscreen window;
/// * `hidesOnDeactivate = false`, so losing app activation does not close it.
fn claim_panel_window<R: Runtime>(window: &WebviewWindow<R>) {
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{
        NSPanel, NSStatusWindowLevel, NSWindowCollectionBehavior, NSWindowStyleMask,
    };

    let Ok(handle) = window.ns_window() else {
        eprintln!("[quick-tabs] no NSWindow handle; fullscreen overlay unavailable");
        return;
    };

    // SAFETY: `ns_window()` hands us tao's live window object, we are on the main
    // thread inside `setup`, and `QuickTabsPanel` is an `NSPanel` subclass with no
    // extra ivars, so the in-place class swap keeps the allocation layout valid.
    // Every call below is a plain property write on that object. The raw
    // `object_setClass` is needed because `AnyObject::set_class` insists the two
    // classes report the *same* instance size, while `NSPanel` reports 8 bytes
    // fewer than `NSWindow` (456 vs 464) — i.e. it adds no storage.
    unsafe {
        objc2::ffi::object_setClass(handle as *mut AnyObject, QuickTabsPanel::class());
        let panel = &*(handle as *const NSPanel);
        panel.setStyleMask(panel.styleMask() | NSWindowStyleMask::NonactivatingPanel);
        panel.setFloatingPanel(true);
        panel.setBecomesKeyOnlyIfNeeded(false);
        panel.setHidesOnDeactivate(false);
        panel.setLevel(NSStatusWindowLevel);
        panel.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }
}

/// Owns the 速记 panel. A separate type from `WindowManager` on purpose: the
/// panel's lifecycle (show/hide/size memory, blur guard) has nothing to do with
/// the main window's (design D2).
#[derive(Default)]
pub struct QuickTabsWindow {
    /// Serialises toggles so a fast double click cannot show twice.
    gate: Mutex<()>,
    visible: AtomicBool,
    /// When the panel last hid itself, so the tray click that caused it is not
    /// treated as a new open request (design D5).
    last_blur_hide: Mutex<Option<Instant>>,
    /// Panel size is remembered across shows; the position never is (design D6).
    saved_size: Mutex<Option<(f64, f64)>>,
    /// Physical rect of the tray button, kept for programmatic shows.
    tray_rect: Mutex<Option<(f64, f64, f64, f64)>>,
}

impl QuickTabsWindow {
    pub const LABEL: &'static str = "quick-tabs";
    pub const URL: &'static str = "quick-tabs.html";

    /// Created hidden at startup so the first click is warm. Failure is not
    /// fatal: the rest of the app works, and `toggle` reports the error.
    pub fn create<R: Runtime>(app: &AppHandle<R>) -> AppResult<WebviewWindow<R>> {
        let builder =
            WebviewWindowBuilder::new(app, Self::LABEL, WebviewUrl::App(Self::URL.into()))
                .title("速记")
                .inner_size(DEFAULT_SIZE.0, DEFAULT_SIZE.1)
                .min_inner_size(MIN_SIZE.0, MIN_SIZE.1)
                .decorations(false)
                .resizable(true)
                .always_on_top(true)
                .visible_on_all_workspaces(true)
                .focused(false)
                .visible(false);
        let window = builder.build().map_err(AppError::from)?;
        claim_panel_window(&window);
        Ok(window)
    }

    pub fn window<R: Runtime>(&self, app: &AppHandle<R>) -> AppResult<WebviewWindow<R>> {
        app.get_webview_window(Self::LABEL)
            .ok_or_else(|| AppError::Other("quick tabs window is missing".into()))
    }

    pub fn is_visible<R: Runtime>(&self, app: &AppHandle<R>) -> bool {
        self.window(app)
            .and_then(|window| Ok(window.is_visible()?))
            .unwrap_or(false)
    }

    /// Explicit user action: open when hidden, close when open.
    pub fn toggle<R: Runtime>(&self, app: &AppHandle<R>, icon: Option<Rect>) -> AppResult<bool> {
        let _gate = self.gate.lock().expect("quick tabs gate poisoned");
        // An explicit toggle is the user's decision; never let a stale blur
        // guard swallow it.
        *self.last_blur_hide.lock().expect("blur lock poisoned") = None;
        let window = self.window(app)?;
        if window.is_visible().unwrap_or(false) {
            self.hide_window(&window)?;
            Ok(false)
        } else {
            self.show_window(&window, app, icon)?;
            Ok(true)
        }
    }

    /// Tray left click. If the panel just hid because it lost focus (the click
    /// that reached us *was* what stole it), do nothing — that click already
    /// got the result the user wanted.
    pub fn toggle_from_tray<R: Runtime>(&self, app: &AppHandle<R>, icon: Rect) -> AppResult<bool> {
        if self.blur_hid_recently() {
            return Ok(false);
        }
        self.toggle(app, Some(icon))
    }

    pub fn show<R: Runtime>(&self, app: &AppHandle<R>, icon: Option<Rect>) -> AppResult<()> {
        let _gate = self.gate.lock().expect("quick tabs gate poisoned");
        let window = self.window(app)?;
        self.show_window(&window, app, icon)
    }

    pub fn hide<R: Runtime>(&self, app: &AppHandle<R>) -> AppResult<()> {
        let _gate = self.gate.lock().expect("quick tabs gate poisoned");
        let window = self.window(app)?;
        self.hide_window(&window)
    }

    /// Lost focus (clicked another app, another window, or the menu bar).
    pub fn hide_on_blur<R: Runtime>(&self, app: &AppHandle<R>) -> AppResult<()> {
        let _gate = self.gate.lock().expect("quick tabs gate poisoned");
        let window = self.window(app)?;
        if !window.is_visible().unwrap_or(false) {
            return Ok(());
        }
        self.hide_window(&window)?;
        *self.last_blur_hide.lock().expect("blur lock poisoned") = Some(Instant::now());
        Ok(())
    }

    fn blur_hid_recently(&self) -> bool {
        let guard = self.last_blur_hide.lock().expect("blur lock poisoned");
        guard.map(|at| at.elapsed() < BLUR_GUARD).unwrap_or(false)
    }

    fn show_window<R: Runtime>(
        &self,
        window: &WebviewWindow<R>,
        app: &AppHandle<R>,
        icon: Option<Rect>,
    ) -> AppResult<()> {
        // Size comes from us, never from `outer_size()`: a window that has not
        // been shown yet reports a size that is off by its scale factor, and
        // anchoring with a half-width panel puts it under the wrong part of the
        // menu bar (caught by the runtime probe, not by the unit tests).
        let panel = self
            .saved_size
            .lock()
            .expect("size lock poisoned")
            .unwrap_or(DEFAULT_SIZE);
        let top_logical = self.place_under_tray(window, app, icon, panel)?;
        window.show()?;
        window.set_focus()?;
        self.visible.store(true, Ordering::SeqCst);
        if let Some(top_logical) = top_logical {
            follow_menu_bar(app, top_logical);
        }
        Ok(())
    }

    fn hide_window<R: Runtime>(&self, window: &WebviewWindow<R>) -> AppResult<()> {
        // Only a window that was actually on screen has a trustworthy size.
        if window.is_visible().unwrap_or(false) {
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
        }
        window.hide()?;
        self.visible.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Anchor to the menu bar icon (design D6). Falls back to the last known
    /// rect, then to the monitor under the pointer, so a programmatic show
    /// still lands somewhere sensible. Coordinates are never persisted.
    fn place_under_tray<R: Runtime>(
        &self,
        window: &WebviewWindow<R>,
        app: &AppHandle<R>,
        icon: Option<Rect>,
        panel: (f64, f64),
    ) -> AppResult<Option<f64>> {
        if let Some(rect) = icon {
            *self.tray_rect.lock().expect("tray rect lock poisoned") = Some(physical_rect(rect));
        }
        let icon = *self.tray_rect.lock().expect("tray rect lock poisoned");

        // The monitor that owns the icon; the panel must never jump displays.
        let monitor = icon
            .map(|(x, y, width, height)| {
                app.monitor_from_point(x + width / 2.0, y + height / 2.0)
                    .ok()
                    .flatten()
            })
            .flatten()
            .or_else(|| {
                app.cursor_position()
                    .ok()
                    .and_then(|cursor| app.monitor_from_point(cursor.x, cursor.y).ok().flatten())
            })
            .or_else(|| app.primary_monitor().ok().flatten());

        let Some(monitor) = monitor else {
            window.center()?;
            return Ok(None);
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

        let (width, height) = monitor::clamp_to_area(area, panel);
        window.set_size(LogicalSize::new(width, height))?;

        let logical_icon = icon
            .map(|(x, y, w, h)| (x / scale, y / scale, w / scale, h / scale))
            // No icon yet: treat the top-centre of the screen as the anchor.
            .unwrap_or((area.0 + area.2 / 2.0, area.1, 1.0, 0.0));
        let (x, y) = monitor::tray_anchor_origin(area, logical_icon, (width, height), TRAY_GAP);
        let top_logical = area.1 + FULLSCREEN_TOP_INSET;
        window.set_position(PhysicalPosition::new(
            (x * scale).round() as i32,
            (y * scale).round() as i32,
        ))?;
        Ok(Some(top_logical))
    }
}
