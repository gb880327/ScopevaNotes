use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

use crate::database::{migrations, Database};
use crate::models::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInfo {
    pub path: String,
    pub schema_version: i64,
    pub is_default: bool,
    /// Visible notes only; the trash is reported separately by the UI.
    pub note_count: i64,
    /// Main database file plus its WAL, so the number matches Finder after writes.
    pub file_size_bytes: i64,
}

/// Only the library pointer and UI preferences live here — never note data
/// (plan §4.3).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub library_path: Option<String>,
}

fn config_path<R: Runtime>(app: &AppHandle<R>) -> AppResult<PathBuf> {
    let dir = app.path().app_config_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("config.json"))
}

pub fn load_config<R: Runtime>(app: &AppHandle<R>) -> AppResult<AppConfig> {
    let path = config_path(app)?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = std::fs::read_to_string(&path)?;
    serde_json::from_str(&raw).map_err(|err| AppError::Invalid(format!("config.json: {err}")))
}

pub fn save_config<R: Runtime>(app: &AppHandle<R>, config: &AppConfig) -> AppResult<()> {
    let path = config_path(app)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(config)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn default_library_path<R: Runtime>(app: &AppHandle<R>) -> AppResult<PathBuf> {
    Ok(app.path().app_data_dir()?.join("notes.db"))
}

/// Resolve the active library, creating the default one on first run.
pub fn resolve<R: Runtime>(app: &AppHandle<R>) -> AppResult<(Database, LibraryInfo)> {
    let config = load_config(app)?;
    let default_path = default_library_path(app)?;

    if let Some(custom) = config.library_path {
        let path = PathBuf::from(custom);
        if path.exists() {
            let db = Database::open(&path)?;
            let info = info_for(&db, false)?;
            return Ok((db, info));
        }
        // Never silently fall back to an empty library (plan §1.8).
        return Err(AppError::NotFound(format!(
            "library not found: {}",
            path.display()
        )));
    }

    let db = Database::open(&default_path)?;
    let info = info_for(&db, true)?;
    Ok((db, info))
}

pub fn info_for(db: &Database, is_default: bool) -> AppResult<LibraryInfo> {
    let schema_version = db.with_conn(|conn| migrations::current_version(conn))?;
    let note_count = db.with_conn(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM notes WHERE deleted_at IS NULL",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(Into::into)
    })?;
    Ok(LibraryInfo {
        path: db.path().to_string_lossy().into_owned(),
        schema_version,
        is_default,
        note_count,
        file_size_bytes: disk_usage(db.path()),
    })
}

/// SQLite spreads writes into `-wal` until a checkpoint, so counting the main
/// file alone under-reports the library by up to a few MB.
fn disk_usage(path: &std::path::Path) -> i64 {
    let mut total = std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0);
    for suffix in ["-wal", "-shm"] {
        let side = std::path::PathBuf::from(format!("{}{suffix}", path.to_string_lossy()));
        if let Ok(meta) = std::fs::metadata(side) {
            total += meta.len() as i64;
        }
    }
    total
}

/// Reject paths that would overwrite an existing file on "create".
pub fn validate_new_path(path: &Path) -> AppResult<()> {
    if path.exists() {
        return Err(AppError::Conflict(format!(
            "file already exists: {}",
            path.display()
        )));
    }
    match path.parent() {
        Some(parent) if parent.as_os_str().is_empty() || parent.is_dir() => Ok(()),
        Some(parent) => Err(AppError::Invalid(format!(
            "parent directory does not exist: {}",
            parent.display()
        ))),
        None => Err(AppError::Invalid("path has no parent directory".into())),
    }
}

/// Reject libraries that are missing, unreadable, or corrupt before we touch
/// the running state.
pub fn validate_existing_path(path: &Path) -> AppResult<()> {
    if !path.is_file() {
        return Err(AppError::NotFound(format!(
            "library not found: {}",
            path.display()
        )));
    }
    let db = Database::open(path)?;
    let check = db.quick_check()?;
    if check != "ok" {
        return Err(AppError::Invalid(format!("integrity check failed: {check}")));
    }
    Ok(())
}
