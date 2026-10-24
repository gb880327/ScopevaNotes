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
    pub window: crate::platform::macos::window_manager::WindowManager,
}

impl AppState {
    pub fn new(db: Database, library: LibraryInfo, shortcut: String) -> Self {
        Self {
            db: RwLock::new(Arc::new(db)),
            library: Mutex::new(library),
            shortcut: Mutex::new(shortcut),
            quitting: AtomicBool::new(false),
            quit_pending: AtomicBool::new(false),
            window: Default::default(),
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
}

const _: fn() = || {
    fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<AppState>();
};
