//! Trash scale check (task 4.1). Ignored by default; run with
//!
//! ```sh
//! cargo test --release --test trash_bench -- --ignored --nocapture
//! ```
//!
//! "Empty trash" is a single transaction on the shared connection, so its
//! duration is the whole time autosave is blocked. The seeding below writes the
//! rows directly (bypassing `soft_delete`'s per-note reindex) so the number
//! measures `purge_all`, not the setup.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use macos_notes_lib::database::Database;
use macos_notes_lib::repositories::note_repo;

fn temp_db() -> (Database, std::path::PathBuf) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("trash-bench-{unique}.db"));
    let db = Database::open(&path).expect("open temp database");
    (db, path)
}

fn seed_deleted(db: &Database, count: usize) {
    db.with_conn(|conn| {
        let tx = conn.transaction()?;
        for i in 0..count {
            tx.execute(
                "INSERT INTO notes(id, title, content, created_at, updated_at, deleted_at)
                 VALUES (?1, ?2, ?3, 1000, 1000, 2000)",
                rusqlite::params![
                    format!("n{i}"),
                    format!("废稿 {i}"),
                    format!("回收站里的正文 {i} 关键词")
                ],
            )?;
            tx.execute(
                "INSERT INTO notes_fts(note_id, title, content, tags_text) VALUES (?1, ?2, ?3, '')",
                rusqlite::params![format!("n{i}"), format!("废稿 {i}"), format!("正文 {i}")],
            )?;
        }
        tx.commit()?;
        Ok(())
    })
    .expect("seed trash");
}

#[test]
#[ignore]
fn empty_trash_scale() {
    for count in [1_000usize, 10_000] {
        let (db, path) = temp_db();
        seed_deleted(&db, count);

        let before = db.with_conn(|conn| note_repo::count_deleted(conn)).unwrap();
        assert_eq!(before as usize, count);

        let started = Instant::now();
        let removed = db.with_conn(|conn| note_repo::purge_all(conn)).unwrap();
        let elapsed = started.elapsed();

        let notes: i64 = db
            .with_conn(|conn| Ok(conn.query_row("SELECT count(*) FROM notes", [], |r| r.get(0))?))
            .unwrap();
        let fts: i64 = db
            .with_conn(|conn| Ok(conn.query_row("SELECT count(*) FROM notes_fts", [], |r| r.get(0))?))
            .unwrap();
        println!(
            "purge_all deleted={count} removed={removed} elapsed={:.2}ms notes_left={notes} fts_left={fts}",
            elapsed.as_secs_f64() * 1000.0
        );
        assert_eq!(removed as usize, count);
        assert_eq!((notes, fts), (0, 0), "no note or index row survives");

        drop(db);
        let _ = std::fs::remove_file(&path);
    }
}
