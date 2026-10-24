//! Task 8.5: crash recovery and a backup/restore round-trip.
//!
//! The crash drill is real: a child process commits (or half-writes) notes and
//! then leaves through `process::exit`, so no destructor runs and SQLite's hot
//! journal is left on disk for the next open to recover.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use macos_notes_lib::database::{backup, Database};
use macos_notes_lib::repositories::note_repo::{self, ListQuery};

const CHILD_ENV: &str = "NOTES_RECOVERY_CHILD";

fn unique(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("macos-notes-recovery-{tag}-{nanos}"))
}

fn cleanup(paths: &[&Path]) {
    for path in paths {
        for suffix in ["", "-wal", "-shm", "-journal"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.to_string_lossy()));
        }
    }
}

fn seed(db: &Database, prefix: &str, count: usize) {
    for index in 0..count {
        db.with_conn(|conn| {
            note_repo::create(
                conn,
                &format!("{prefix} {index}"),
                &format!("正文 {prefix} {index}"),
                &[],
            )
        })
        .unwrap();
    }
}

fn count_titled(db: &Database, prefix: &str) -> usize {
    let page = db
        .with_conn(|conn| {
            note_repo::list(
                conn,
                &ListQuery {
                    search: Some(prefix.to_string()),
                    limit: Some(200),
                    ..Default::default()
                },
            )
        })
        .unwrap();
    page.items
        .iter()
        .filter(|item| item.title.starts_with(prefix))
        .count()
}

/// Runs in the child process and dies without unwinding.
fn crash_child(mode: &str, path: &Path) -> ! {
    let db = Database::open(path).unwrap();
    match mode {
        "committed" => {
            seed(&db, "committed", 20);
        }
        _ => {
            // Leave a transaction open: the rows must be rolled back by recovery.
            db.with_conn(|conn| {
                conn.execute_batch("BEGIN")?;
                for index in 0..50 {
                    conn.execute(
                        "INSERT INTO notes(id, title, content, created_at, updated_at)
                         VALUES (?1, ?2, 'x', 0, 0)",
                        [format!("uncommitted-{index}"), format!("uncommitted {index}")],
                    )?;
                }
                Ok(())
            })
            .unwrap();
        }
    }
    // No Drop, no checkpoint, no rollback — the process simply disappears.
    std::process::exit(42);
}

#[test]
fn hard_kill_recovery_keeps_committed_rows_and_drops_open_transaction() {
    if let Ok(mode) = std::env::var(CHILD_ENV) {
        let path = PathBuf::from(std::env::var("NOTES_RECOVERY_DB").unwrap());
        crash_child(&mode, &path);
    }

    let dir = unique("kill");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("notes.db");
    Database::open(&path).unwrap();

    for mode in ["committed", "mid-transaction"] {
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "hard_kill_recovery_keeps_committed_rows_and_drops_open_transaction",
                "--exact",
                "--test-threads=1",
            ])
            .env(CHILD_ENV, mode)
            .env("NOTES_RECOVERY_DB", &path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(42), "child must exit without cleanup");

        if mode == "mid-transaction" {
            // Proof that the crash really left unrecovered state behind.
            let journal = PathBuf::from(format!("{}-journal", path.to_string_lossy()));
            println!("journal after crash: {}", journal.exists());
        }

        // Reopening runs SQLite recovery before our migrations do anything.
        let db = Database::open(&path).unwrap();
        assert_eq!(db.quick_check().unwrap(), "ok");
        if mode == "committed" {
            assert_eq!(count_titled(&db, "committed"), 20, "committed writes must survive");
        }
        assert_eq!(
            db.with_conn(|conn| {
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM notes WHERE id LIKE 'uncommitted-%'",
                    [],
                    |row| row.get::<_, i64>(0),
                )?)
            })
            .unwrap(),
            0,
            "uncommitted rows must never become visible"
        );
        drop(db);
    }

    let db = Database::open(&path).unwrap();
    assert_eq!(db.quick_check().unwrap(), "ok");
    assert_eq!(count_titled(&db, "committed"), 20);
    drop(db);

    cleanup(&[&path]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn backup_restore_round_trip_preserves_notes_tags_and_search() {
    let dir = unique("roundtrip");
    std::fs::create_dir_all(&dir).unwrap();
    let live = dir.join("notes.db");
    let snapshot = dir.join("snapshot.db");

    let db = Database::open(&live).unwrap();
    let tag_id = db
        .with_conn(|conn| macos_notes_lib::repositories::tag_repo::create(conn, "工作", None))
        .unwrap()
        .id;
    for index in 0..30 {
        db.with_conn(|conn| {
            note_repo::create(
                conn,
                &format!("笔记 {index}"),
                if index % 2 == 0 { "提到 sqlite 的正文" } else { "普通正文" },
                if index % 3 == 0 {
                    std::slice::from_ref(&tag_id)
                } else {
                    &[]
                },
            )
        })
        .unwrap();
    }
    let before_search = db
        .with_conn(|conn| {
            note_repo::list(
                conn,
                &ListQuery {
                    search: Some("sqlite".into()),
                    limit: Some(200),
                    ..Default::default()
                },
            )
        })
        .unwrap()
        .items
        .len();
    assert_eq!(before_search, 15);

    let size = db
        .with_conn(|conn| backup::backup_to(conn, &snapshot))
        .unwrap();
    assert!(size > 0);
    drop(db);

    // Simulate losing the live file (disk failure / bad sync) and restoring it.
    std::fs::remove_file(&live).unwrap();
    std::fs::copy(&snapshot, &live).unwrap();

    let restored = Database::open(&live).unwrap();
    assert_eq!(restored.quick_check().unwrap(), "ok");
    assert_eq!(
        restored
            .with_conn(|conn| note_repo::list(conn, &ListQuery { limit: Some(200), ..Default::default() }))
            .unwrap()
            .items
            .len(),
        30
    );
    assert_eq!(
        restored
            .with_conn(|conn| {
                note_repo::list(
                    conn,
                    &ListQuery {
                        search: Some("sqlite".into()),
                        limit: Some(200),
                        ..Default::default()
                    },
                )
            })
            .unwrap()
            .items
            .len(),
        15,
        "the FTS projection must survive the round-trip"
    );
    assert_eq!(
        restored
            .with_conn(|conn| macos_notes_lib::repositories::tag_repo::list_with_counts(conn))
            .unwrap()
            .iter()
            .map(|tag| tag.note_count)
            .sum::<i64>(),
        10,
        "tags and their counts must survive the round-trip"
    );
    drop(restored);

    cleanup(&[&live, &snapshot]);
    let _ = std::fs::remove_dir_all(&dir);
}
