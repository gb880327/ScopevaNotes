//! Cost of deleting one tag shared by many notes (change `add-tag-management`,
//! task 1.2).
//!
//! `tag_repo::delete` rebuilds the search projection for every affected note
//! with `reindex_note`, and `notes_fts.note_id` is UNINDEXED, so each rebuild
//! is a full scan of the FTS table. This measures how bad it gets at 1,000
//! notes — the number quoted in the change's `verification.md`.
//!
//! Run with:
//!   cargo test --release --test tag_bench -- --ignored --nocapture

use std::time::Instant;

use macos_notes_lib::database::Database;
use macos_notes_lib::repositories::{note_repo, tag_repo};

const NOTES: usize = 1_000;

#[test]
#[ignore = "stress run; use --release --ignored --nocapture"]
fn delete_tag_shared_by_a_thousand_notes() {
    let dir = std::env::temp_dir().join(format!(
        "macos-notes-tag-bench-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let db = Database::open(dir.join("notes.db")).unwrap();

    let tag = db
        .with_conn(|conn| tag_repo::create(conn, "shared", None))
        .expect("create tag");
    let ids = vec![tag.id.clone()];

    for index in 0..NOTES {
        let title = format!("笔记 {index}");
        let content = format!("正文 {index} sqlite 标签 搜索");
        db.with_conn(|conn| note_repo::create(conn, &title, &content, &ids))
            .expect("create note");
    }

    let before = db
        .with_conn(|conn| tag_repo::list_with_counts(conn))
        .expect("list before");
    assert_eq!(before[0].note_count as usize, NOTES);

    let started = Instant::now();
    db.with_conn(|conn| tag_repo::delete(conn, &tag.id))
        .expect("delete tag");
    let elapsed = started.elapsed();

    let after = db
        .with_conn(|conn| tag_repo::list_with_counts(conn))
        .expect("list after");
    let live = db
        .with_conn(|conn| {
            note_repo::list(
                conn,
                &note_repo::ListQuery {
                    limit: Some(5),
                    ..Default::default()
                },
            )
        })
        .expect("list notes");

    println!(
        "delete_tag notes={NOTES:<5} elapsed={:>8.2}ms remaining_tags={} live_notes={}",
        elapsed.as_secs_f64() * 1000.0,
        after.len(),
        live.items.len()
    );
    assert!(after.is_empty(), "the tag row is gone");
    assert_eq!(live.items.len(), 5, "notes survive a tag delete");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
