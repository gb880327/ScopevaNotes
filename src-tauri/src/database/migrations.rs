use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{AppError, AppResult};

/// Ordered, append-only migrations. `include_str!` keeps the SQL next to the
/// code that applies it and ships it inside the binary.
const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../../migrations/0001_init.sql")),
    (2, include_str!("../../migrations/0002_fts.sql")),
];

pub const LATEST_VERSION: i64 = MIGRATIONS[MIGRATIONS.len() - 1].0;

const META_DDL: &str = "CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);";

pub fn run(conn: &mut Connection) -> AppResult<()> {
    conn.execute_batch(META_DDL)?;
    for (version, sql) in MIGRATIONS {
        let applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [version],
            |row| row.get(0),
        )?;
        if applied {
            continue;
        }
        // One transaction per migration: a failed step leaves the DB untouched.
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
            params![version, super::now_ms()],
        )?;
        tx.commit()?;
    }
    Ok(())
}

pub fn current_version(conn: &Connection) -> AppResult<i64> {
    let version = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0);
    Ok(version)
}

/// A library written by a newer build must not be downgraded silently.
pub fn ensure_not_newer(conn: &Connection) -> AppResult<()> {
    let version = current_version(conn)?;
    if version > LATEST_VERSION {
        return Err(AppError::Invalid(format!(
            "library schema v{version} is newer than this build (v{LATEST_VERSION})"
        )));
    }
    Ok(())
}
