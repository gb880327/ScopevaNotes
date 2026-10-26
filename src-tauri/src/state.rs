use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::database::Database;
use crate::services::library_service::LibraryInfo;

pub struct AppState {
    db: RwLock<Arc<Database>>,
    library: Mutex<LibraryInfo>,
    shortcut: Mutex<String>,
    /// Set once the user confirmed quitting, so the exit interception in
    /// `lib.rs` lets the process die instead of asking again.
    quitting: AtomicBool,
    quit_pending: AtomicBool,
    /// One import at a time; the flag is also the cancel signal the background
    /// thread polls between files and batches (design D12).
    import_running: Arc<AtomicBool>,
    import_cancel: Arc<AtomicBool>,
    pub window: crate::platform::macos::window_manager::WindowManager,
    /// The 速记 panel. Kept beside the main window manager rather than inside
    /// it: the two have unrelated visibility rules (design D2).
    pub quick_tabs: crate::platform::macos::quick_tabs_window::QuickTabsWindow,
}

impl AppState {
    pub fn new(db: Database, library: LibraryInfo, shortcut: String) -> Self {
        Self {
            db: RwLock::new(Arc::new(db)),
            library: Mutex::new(library),
            shortcut: Mutex::new(shortcut),
            quitting: AtomicBool::new(false),
            quit_pending: AtomicBool::new(false),
            import_running: Arc::new(AtomicBool::new(false)),
            import_cancel: Arc::new(AtomicBool::new(false)),
            window: Default::default(),
            quick_tabs: Default::default(),
        }
    }

    /// Cheap snapshot so blocking DB work owns an `Arc` instead of holding the
    /// state lock across an await point.
    pub fn db(&self) -> Arc<Database> {
        self.db.read().expect("state lock poisoned").clone()
    }

    pub fn library(&self) -> LibraryInfo {
        self.library.lock().expect("state lock poisoned").clone()
    }

    pub fn replace_library(&self, db: Database, info: LibraryInfo) {
        *self.db.write().expect("state lock poisoned") = Arc::new(db);
        *self.library.lock().expect("state lock poisoned") = info;
    }

    /// Hide the current library notice (the user read it or switched to the
    /// default library). Returns the refreshed snapshot.
    pub fn clear_notice(&self) -> LibraryInfo {
        let mut info = self.library.lock().expect("state lock poisoned");
        info.notice = None;
        info.clone()
    }

    /// The accelerator currently registered with the global shortcut plugin.
    pub fn shortcut(&self) -> String {
        self.shortcut.lock().expect("state lock poisoned").clone()
    }

    pub fn set_shortcut(&self, accelerator: &str) {
        *self.shortcut.lock().expect("state lock poisoned") = accelerator.to_string();
    }

    /// Marks the confirmed quit. Every later exit request passes through.
    pub fn allow_exit(&self) {
        self.quitting.store(true, Ordering::SeqCst);
    }

    pub fn exit_allowed(&self) -> bool {
        self.quitting.load(Ordering::SeqCst)
    }

    /// One dialog at a time, even if the shortcut and the tray fire together.
    pub fn begin_quit_prompt(&self) -> bool {
        !self
            .quit_pending
            .swap(true, Ordering::SeqCst)
    }

    pub fn end_quit_prompt(&self) {
        self.quit_pending.store(false, Ordering::SeqCst);
    }

    /// Claim the single import slot. `None` means an import is already running.
    pub fn begin_import(&self) -> Option<ImportGuard> {
        if self
            .import_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return None;
        }
        self.import_cancel.store(false, Ordering::SeqCst);
        Some(ImportGuard {
            running: self.import_running.clone(),
            cancel: self.import_cancel.clone(),
        })
    }

    pub fn request_import_cancel(&self) {
        self.import_cancel.store(true, Ordering::SeqCst);
    }
}

/// Owns the import slot for the lifetime of one import. `Drop` clears both
/// flags, so an early error or a panic cannot leave the app stuck on
/// "an import is running".
pub struct ImportGuard {
    running: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
}

impl ImportGuard {
    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }
}

impl Drop for ImportGuard {
    fn drop(&mut self) {
        self.cancel.store(false, Ordering::SeqCst);
        self.running.store(false, Ordering::SeqCst);
    }
}

const _: fn() = || {
    fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<AppState>();
};
