use std::path::{Path, PathBuf};
use std::sync::Mutex;

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

/// A window size in logical pixels. Coordinates are deliberately absent: both
/// windows are positioned fresh on every show (design D2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSize {
    pub width: f64,
    pub height: f64,
}

impl WindowSize {
    pub fn from_tuple(size: (f64, f64)) -> Self {
        Self {
            width: size.0,
            height: size.1,
        }
    }

    pub fn as_tuple(self) -> (f64, f64) {
        (self.width, self.height)
    }
}

/// Which remembered size a read or write is about (design D2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeSlot {
    Main,
    QuickTabs,
}

/// Only the library pointer and UI preferences live here — never note data
/// (plan §4.3).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub library_path: Option<String>,
    /// Remembered main-window size; `None` means "never resized" (spec:
    /// macos-shell / 显示时定位与置顶).
    pub main_window_size: Option<WindowSize>,
    /// Remembered 速记 panel size (spec: quick-tabs / 速记面板的定位).
    pub quick_tabs_size: Option<WindowSize>,
}

/// The single in-memory copy of `config.json` and its only writer.
///
/// The library pointer and the two window sizes share one file, so a
/// `load → modify → save` in every caller would let a size update and a
/// library switch silently drop each other (design D5). Memory is the truth
/// here; the file is its projection, which is why a failed write keeps the
/// in-memory value and only logs.
pub struct ConfigCache {
    inner: Mutex<AppConfig>,
}

impl Default for ConfigCache {
    fn default() -> Self {
        Self::new(AppConfig::default())
    }
}

impl ConfigCache {
    pub fn new(config: AppConfig) -> Self {
        Self {
            inner: Mutex::new(config),
        }
    }

    /// Reads the file once at startup. A missing, unreadable or corrupt file
    /// MUST NOT abort launch (spec: 尺寸配置损坏不阻断启动), so the error is
    /// logged and defaults are used.
    pub fn load<R: Runtime>(app: &AppHandle<R>) -> Self {
        match load_config(app) {
            Ok(config) => Self::new(config),
            Err(err) => {
                eprintln!("[config] falling back to defaults: {err}");
                Self::default()
            }
        }
    }

    pub fn snapshot(&self) -> AppConfig {
        self.inner.lock().expect("config lock poisoned").clone()
    }

    /// The size remembered for `slot`, if the user ever resized that window.
    pub fn size(&self, slot: SizeSlot) -> Option<(f64, f64)> {
        let config = self.inner.lock().expect("config lock poisoned");
        let size = match slot {
            SizeSlot::Main => config.main_window_size,
            SizeSlot::QuickTabs => config.quick_tabs_size,
        };
        size.map(WindowSize::as_tuple)
    }

    pub fn set_size<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        slot: SizeSlot,
        size: (f64, f64),
    ) -> AppResult<()> {
        self.update(app, |config| {
            let value = Some(WindowSize::from_tuple(size));
            match slot {
                SizeSlot::Main => config.main_window_size = value,
                SizeSlot::QuickTabs => config.quick_tabs_size = value,
            }
        })
    }

    pub fn set_library_path<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        path: Option<String>,
    ) -> AppResult<()> {
        self.update(app, |config| config.library_path = path)
    }

    fn update<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        change: impl FnOnce(&mut AppConfig),
    ) -> AppResult<()> {
        let mut config = self.inner.lock().expect("config lock poisoned");
        change(&mut config);
        save_config(app, &config)
    }
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

/// Private on purpose: `ConfigCache` is the only writer, so a caller cannot
/// read-modify-write the file and drop a field it did not know about.
fn save_config<R: Runtime>(app: &AppHandle<R>, config: &AppConfig) -> AppResult<()> {
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
        return Err(AppError::Invalid(format!(
            "integrity check failed: {check}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_config_without_sizes_still_deserializes() {
        let config: AppConfig = serde_json::from_str(r#"{"libraryPath":"/tmp/notes.db"}"#)
            .expect("a config written before sizes existed must still load");
        assert_eq!(config.library_path.as_deref(), Some("/tmp/notes.db"));
        assert!(config.main_window_size.is_none());
        assert!(config.quick_tabs_size.is_none());
    }

    #[test]
    fn config_round_trips_both_sizes() {
        let config = AppConfig {
            library_path: Some("/tmp/notes.db".into()),
            main_window_size: Some(WindowSize::from_tuple((1320.0, 860.0))),
            quick_tabs_size: Some(WindowSize::from_tuple((640.0, 420.0))),
        };
        let raw = serde_json::to_string(&config).expect("serialize");
        // camelCase keys are the contract with the file a human may edit.
        assert!(raw.contains("\"mainWindowSize\""));
        assert!(raw.contains("\"quickTabsSize\""));
        let back: AppConfig = serde_json::from_str(&raw).expect("deserialize");
        assert_eq!(
            back.main_window_size.map(WindowSize::as_tuple),
            Some((1320.0, 860.0))
        );
        assert_eq!(
            back.quick_tabs_size.map(WindowSize::as_tuple),
            Some((640.0, 420.0))
        );
    }

    #[test]
    fn config_cache_keeps_both_fields_across_updates() {
        let cache = ConfigCache::new(AppConfig {
            library_path: Some("/tmp/old.db".into()),
            main_window_size: Some(WindowSize::from_tuple((1111.0, 777.0))),
            quick_tabs_size: None,
        });
        // What `update` does minus the file write: memory must not lose the
        // other field, which is the bug class design D5 guards against.
        {
            let mut config = cache.inner.lock().expect("config lock poisoned");
            config.library_path = Some("/tmp/new.db".into());
        }
        assert_eq!(cache.size(SizeSlot::Main), Some((1111.0, 777.0)));
        assert_eq!(cache.size(SizeSlot::QuickTabs), None);
        assert_eq!(
            cache.snapshot().library_path.as_deref(),
            Some("/tmp/new.db")
        );
    }
}
