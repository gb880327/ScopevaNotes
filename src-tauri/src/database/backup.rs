use std::path::Path;

use rusqlite::Connection;

use crate::models::{AppError, AppResult};

/// Consistent snapshot via `VACUUM INTO` (plan §4.3). Refuses to overwrite an
/// existing file so a typo cannot destroy a backup.
pub fn backup_to(conn: &Connection, destination: &Path) -> AppResult<u64> {
    if destination.exists() {
        return Err(AppError::Invalid(format!(
            "destination already exists: {}",
            destination.display()
        )));
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let escaped = destination.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{escaped}'"))?;

    Ok(std::fs::metadata(destination).map(|m| m.len()).unwrap_or(0))
}
