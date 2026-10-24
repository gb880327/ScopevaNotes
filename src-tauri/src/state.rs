use std::sync::{Arc, Mutex, RwLock};

use crate::database::Database;
use crate::services::library_service::LibraryInfo;

pub struct AppState {
    db: RwLock<Arc<Database>>,
    library: Mutex<LibraryInfo>,
    pub window: crate::platform::macos::window_manager::WindowManager,
}

impl AppState {
    pub fn new(db: Database, library: LibraryInfo) -> Self {
        Self {
            db: RwLock::new(Arc::new(db)),
            library: Mutex::new(library),
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
}

const _: fn() = || {
    fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<AppState>();
};
