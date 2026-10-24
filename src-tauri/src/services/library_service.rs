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
    Ok(LibraryInfo {
        path: db.path().to_string_lossy().into_owned(),
        schema_version,
        is_default,
    })
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
