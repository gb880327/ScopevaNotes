use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use macos_notes_lib::database::Database;
use macos_notes_lib::models::AppError;
use macos_notes_lib::repositories::note_repo::{self, ListQuery};
use macos_notes_lib::repositories::tag_repo;

fn temp_db() -> (Database, PathBuf) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("macos-notes-test-{unique}.db"));
    let db = Database::open(&path).expect("open temp database");
    (db, path)
}

#[test]
fn crud_soft_delete_restore_and_search_projection() {
    let (db, path) = temp_db();

    let tag = db
        .with_conn(|conn| tag_repo::create(conn, "  rust   tips ", None))
        .expect("create tag");
    assert_eq!(tag.name, "rust tips");

    let note = db
        .with_conn(|conn| {
            note_repo::create(
                conn,
                "Async notes",
                "```rust\nlet a = 1;\n```",
                &[tag.id.clone()],
            )
        })
        .expect("create note");
    assert!(note.content.contains("let a = 1"));
    assert_eq!(note.tags.len(), 1);

    let page = db
        .with_conn(|conn| note_repo::list(conn, &ListQuery::default()))
        .expect("list notes");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].tags, vec!["rust tips".to_string()]);

    let found = db
        .with_conn(|conn| {
            note_repo::list(
                conn,
                &ListQuery {
                    search: Some("Async".into()),
                    ..Default::default()
                },
            )
        })
        .expect("search notes");
    assert_eq!(found.items.len(), 1);

    // Stale optimistic-concurrency token must be rejected.
    let stale = db.with_conn(|conn| {
        note_repo::update(
            conn,
            &note.id,
            "Stale title",
            "stale",
            &[],
            Some(note.updated_at - 1),
        )
    });
    assert!(matches!(stale, Err(AppError::Conflict(_))));

    let updated = db
        .with_conn(|conn| {
            note_repo::update(
                conn,
                &note.id,
                "Fresh title",
                "fresh body",
                &[tag.id.clone()],
                Some(note.updated_at),
            )
        })
        .expect("update note");
    assert_eq!(updated.title, "Fresh title");

    // "Keep mine and overwrite" skips the token but keeps the same write path.
    let forced = db
        .with_conn(|conn| {
            note_repo::update(
                conn,
                &note.id,
                "Forced title",
                "forced body",
                &[tag.id.clone()],
                None,
            )
        })
        .expect("forced update");
    assert_eq!(forced.title, "Forced title");

    let indexed: i64 = db
        .with_conn(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM notes_fts WHERE note_id = ?1",
                [&note.id],
                |row| row.get(0),
            )
            .map_err(Into::into)
        })
        .unwrap();
    assert_eq!(indexed, 1);

    db.with_conn(|conn| note_repo::soft_delete(conn, &note.id))
        .expect("soft delete");
    let after_delete = db
        .with_conn(|conn| note_repo::list(conn, &ListQuery::default()))
        .unwrap();
    assert!(after_delete.items.is_empty());

    // Trash listing is the only entry point `restore` can be reached from.
    let trash = db
        .with_conn(|conn| note_repo::list_deleted(conn, None))
        .expect("list deleted notes");
    assert_eq!(trash.len(), 1);
    assert_eq!(trash[0].id, note.id);
    assert_eq!(trash[0].tags, vec!["rust tips".to_string()]);
    let indexed: i64 = db
        .with_conn(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM notes_fts WHERE note_id = ?1",
                [&note.id],
                |row| row.get(0),
            )
            .map_err(Into::into)
        })
        .unwrap();
    assert_eq!(indexed, 0, "soft delete must drop the FTS projection");

    db.with_conn(|conn| note_repo::restore(conn, &note.id))
        .expect("restore");
    let after_restore = db
        .with_conn(|conn| note_repo::list(conn, &ListQuery::default()))
        .unwrap();
    assert_eq!(after_restore.items.len(), 1);
    let trash = db
        .with_conn(|conn| note_repo::list_deleted(conn, None))
        .unwrap();
    assert!(trash.is_empty(), "restored note must leave the trash");

    drop(db);
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_missing_note_and_duplicate_tag() {
    let (db, path) = temp_db();

    let missing = db.with_conn(|conn| note_repo::get(conn, "does-not-exist"));
    assert!(matches!(missing, Err(AppError::NotFound(_))));

    db.with_conn(|conn| tag_repo::create(conn, "work", None))
        .expect("first tag");
    let duplicate = db.with_conn(|conn| tag_repo::create(conn, "WORK", None));
    assert!(matches!(duplicate, Err(AppError::Conflict(_))));

    drop(db);
    let _ = std::fs::remove_file(path);
}
