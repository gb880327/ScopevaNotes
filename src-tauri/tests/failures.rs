//! Failure drills for task 5.9: read-only destinations, corrupt files,
//! interrupted migrations and libraries written by a newer build.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use macos_notes_lib::database::{backup, Database};
use macos_notes_lib::models::AppError;
use macos_notes_lib::repositories::note_repo::{self, ListQuery};

fn unique(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("macos-notes-fail-{tag}-{nanos}"))
}

fn cleanup(paths: &[&Path]) {
    for path in paths {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.to_string_lossy()));
        }
    }
}

/// `Database` has no `Debug`, so `expect_err` is unavailable.
fn open_error(path: &Path) -> AppError {
    match Database::open(path) {
        Ok(_) => panic!("expected open to fail: {}", path.display()),
        Err(err) => err,
    }
}

fn set_mode(path: &Path, mode: u32) {
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    perms.set_mode(mode);
    std::fs::set_permissions(path, perms).unwrap();
}

#[test]
fn open_in_read_only_directory_fails_without_creating_a_library() {
    let dir = unique("readonly");
    std::fs::create_dir_all(&dir).unwrap();
    set_mode(&dir, 0o555);
    let path = dir.join("notes.db");

    let err = open_error(&path);
    assert!(
        matches!(err, AppError::Database(_) | AppError::Io(_)),
        "unexpected error: {err:?}"
    );
    assert!(!path.exists(), "no file should be left behind");

    set_mode(&dir, 0o755);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn backup_into_read_only_directory_fails_and_keeps_source_intact() {
    let dir = unique("backup-src");
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("notes.db");
    let db = Database::open(&source).unwrap();
    db.with_conn(|conn| note_repo::create(conn, "保留我", "内容", &[]))
        .unwrap();

    let target_dir = unique("backup-ro");
    std::fs::create_dir_all(&target_dir).unwrap();
    set_mode(&target_dir, 0o555);

    let err = db
        .with_conn(|conn| backup::backup_to(conn, &target_dir.join("backup.db")))
        .expect_err("read-only destination must fail");
    assert!(
        matches!(err, AppError::Database(_) | AppError::Io(_)),
        "unexpected error: {err:?}"
    );
    assert!(!target_dir.join("backup.db").exists());
    assert_eq!(db.with_conn(|conn| note_repo::list(conn, &ListQuery::default()))
        .unwrap()
        .items
        .len(), 1, "source library stays usable after the failed backup");

    set_mode(&target_dir, 0o755);
    cleanup(&[&source]);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&target_dir);
}

#[test]
fn backup_refuses_to_overwrite_an_existing_file() {
    let dir = unique("backup-existing");
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("notes.db");
    let db = Database::open(&source).unwrap();
    let destination = dir.join("backup.db");
    std::fs::write(&destination, b"do not clobber").unwrap();

    let err = db
        .with_conn(|conn| backup::backup_to(conn, &destination))
        .expect_err("existing destination must be refused");
    assert!(matches!(err, AppError::Invalid(_)), "{err:?}");
    assert_eq!(std::fs::read(&destination).unwrap(), b"do not clobber");

    cleanup(&[&source, &destination]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn corrupt_file_is_reported_instead_of_migrated() {
    let dir = unique("corrupt");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("notes.db");
    std::fs::write(&path, vec![0u8; 4096]).unwrap();

    let err = open_error(&path);
    assert!(matches!(err, AppError::Database(_)), "{err:?}");

    cleanup(&[&path]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn library_from_a_newer_build_is_refused() {
    let dir = unique("newer");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("notes.db");
    {
        let db = Database::open(&path).unwrap();
        db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (999, 0)",
                [],
            )
            .map_err(Into::into)
        })
        .unwrap();
    }

    let err = open_error(&path);
    assert!(matches!(err, AppError::Invalid(_)), "{err:?}");
    assert!(
        err.to_string().contains("newer"),
        "message should name the version gap: {err}"
    );

    cleanup(&[&path]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn interrupted_migration_leaves_no_partial_schema() {
    let dir = unique("interrupted");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("notes.db");

    // A half-written `notes` table (older, partial schema) makes migration 0001
    // fail after it already created `tags` / `note_tags`: the index step
    // references columns that do not exist.
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE notes (id TEXT PRIMARY KEY);")
            .unwrap();
    }

    let err = open_error(&path);
    assert!(matches!(err, AppError::Database(_)), "{err:?}");

    // The failed transaction rolled back: no tags table, no applied version.
    let conn = rusqlite::Connection::open(&path).unwrap();
    let tags: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'tags'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tags, 0, "failed migration must not leave the tags table behind");
    let applied: i64 = conn
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| row.get(0))
        .unwrap();
    assert_eq!(applied, 0, "no version may be recorded for a failed migration");
    drop(conn);

    cleanup(&[&path]);
    let _ = std::fs::remove_dir_all(&dir);
}
