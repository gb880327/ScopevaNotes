use std::sync::{Arc, Mutex, RwLock};

use crate::database::Database;
use crate::services::library_service::LibraryInfo;

pub struct AppState {
    db: RwLock<Arc<Database>>,
    library: Mutex<LibraryInfo>,
    shortcut: Mutex<String>,
    pub window: crate::platform::macos::window_manager::WindowManager,
}

impl AppState {
    pub fn new(db: Database, library: LibraryInfo, shortcut: String) -> Self {
        Self {
            db: RwLock::new(Arc::new(db)),
            library: Mutex::new(library),
            shortcut: Mutex::new(shortcut),
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

    /// The accelerator currently registered with the global shortcut plugin.
    pub fn shortcut(&self) -> String {
        self.shortcut.lock().expect("state lock poisoned").clone()
    }

    pub fn set_shortcut(&self, accelerator: &str) {
        *self.shortcut.lock().expect("state lock poisoned") = accelerator.to_string();
    }
}

const _: fn() = || {
    fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<AppState>();
};
