use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

use crate::database::{migrations, Database};
use crate::models::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NoticeKind {
    /// The configured library was unusable, so the default one was opened.
    Fallback,
    /// The library lives inside a cloud-sync folder.
    CloudSync,
}

/// Something the user must see about the active library. Rendered as a banner
/// in the settings panel (spec: library / 路径失效与云同步提示).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryNotice {
    pub kind: NoticeKind,
    pub message: String,
    /// Path the notice is about (the configured one for a fallback).
    pub path: Option<String>,
}

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
    #[serde(default)]
    pub notice: Option<LibraryNotice>,
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
///
/// A configured library that cannot be used never blocks startup and never
/// silently turns into an empty library: we open the default one and attach a
/// `Fallback` notice so the UI can offer "choose another" / "use default"
/// (plan §1.8, D12).
pub fn resolve<R: Runtime>(app: &AppHandle<R>) -> AppResult<(Database, LibraryInfo)> {
    let config = load_config(app)?;
    resolve_from(config.library_path, &default_library_path(app)?)
}

/// Path-only half of [`resolve`], so the fallback rules are testable without a
/// Tauri `AppHandle`.
pub fn resolve_from(
    configured: Option<String>,
    default_path: &Path,
) -> AppResult<(Database, LibraryInfo)> {
    let Some(custom) = configured else {
        let db = Database::open(default_path)?;
        let info = info_for(&db, true)?;
        return Ok((db, info));
    };

    let path = PathBuf::from(&custom);
    let failure = if path.exists() {
        match Database::open(&path) {
            Ok(db) => {
                let info = info_for(&db, false)?;
                return Ok((db, info));
            }
            Err(err) => err.to_string(),
        }
    } else {
        format!("笔记库不存在：{}", path.display())
    };

    let db = Database::open(default_path)?;
    let mut info = info_for(&db, true)?;
    info.notice = Some(LibraryNotice {
        kind: NoticeKind::Fallback,
        message: format!("配置的笔记库无法打开，已临时改用默认库。原因：{failure}"),
        path: Some(custom),
    });
    Ok((db, info))
}

/// Cloud-sync clients rewrite the database file behind our back; SQLite is fine
/// with that only while no process holds a lock. Detect the common folders so
/// the settings panel can warn (spec: 云同步目录提示).
pub fn cloud_sync_provider(path: &Path) -> Option<&'static str> {
    // (folder name, provider label, also match `name-…` / `name …`).
    const PROVIDERS: &[(&str, &str, bool)] = &[
        ("Dropbox", "Dropbox", false),
        ("OneDrive", "OneDrive", true),
        ("Google Drive", "Google Drive", true),
        ("GoogleDrive", "Google Drive", true),
        ("iCloud Drive", "iCloud Drive", false),
        ("Mobile Documents", "iCloud Drive", false),
        ("Nextcloud", "Nextcloud", false),
        ("ownCloud", "ownCloud", false),
        ("pCloud", "pCloud", false),
        ("Sync.com", "Sync.com", false),
        ("Box", "Box", false),
    ];
    path.components().find_map(|component| {
        let std::path::Component::Normal(part) = component else {
            return None;
        };
        let part = part.to_string_lossy();
        PROVIDERS
            .iter()
            .find(|(needle, _, allow_prefix)| {
                // Clients rename the folder per account: `GoogleDrive-me@x`,
                // `OneDrive - Acme`. Only those providers opt into the prefix
                // form, so `dropbox-notes.db` is not mistaken for Dropbox.
                part.eq_ignore_ascii_case(needle)
                    || *allow_prefix
                        && part
                            .get(..needle.len())
                            .is_some_and(|head| head.eq_ignore_ascii_case(needle))
                        && part[needle.len()..].starts_with(['-', ' '])
            })
            .map(|(_, label, _)| *label)
    })
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
    let path = db.path().to_string_lossy().into_owned();
    let notice = cloud_sync_provider(db.path()).map(|provider| LibraryNotice {
        kind: NoticeKind::CloudSync,
        message: format!(
            "笔记库位于 {provider} 同步目录。云同步在写入时可能锁住文件，编辑保存或切换笔记库前请确认同步已完成。"
        ),
        path: Some(path.clone()),
    });
    Ok(LibraryInfo {
        path,
        schema_version,
        is_default,
        note_count,
        file_size_bytes: disk_usage(db.path()),
        notice,
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
