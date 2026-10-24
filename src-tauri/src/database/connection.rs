use std::{
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use rusqlite::Connection;

use crate::models::{AppError, AppResult};

/// Single writer connection guarded by a mutex. Commands hop onto a blocking
/// thread before touching it (see plan §3): never share raw `Connection`
/// across async tasks.
pub struct Database {
    path: PathBuf,
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        Self::open_with_busy_timeout(path, Duration::from_secs(5))
    }

    /// Tests need a short timeout; the app keeps a 5 s window so a cloud sync
    /// client releasing its lock does not surface as an error.
    pub fn open_with_busy_timeout(path: impl AsRef<Path>, busy: Duration) -> AppResult<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(busy)?;

        let db = Self {
            path,
            conn: Mutex::new(conn),
        };
        db.with_conn(|conn| {
            super::migrations::run(conn)?;
            super::migrations::ensure_not_newer(conn)
        })?;
        Ok(db)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn with_conn<T>(&self, f: impl FnOnce(&mut Connection) -> AppResult<T>) -> AppResult<T> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Other("database lock poisoned".into()))?;
        f(&mut guard).map_err(AppError::classify)
    }

    pub fn quick_check(&self) -> AppResult<String> {
        self.with_conn(|conn| {
            let result: String = conn.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
            Ok(result)
        })
    }
}
