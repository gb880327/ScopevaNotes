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

fn seed(db: &Database, title: &str, content: &str, tags: &[String]) -> String {
    db.with_conn(|conn| note_repo::create(conn, title, content, tags))
        .expect("create note")
        .id
}

fn search(db: &Database, term: &str) -> Vec<String> {
    db.with_conn(|conn| {
        note_repo::list(
            conn,
            &ListQuery {
                search: Some(term.into()),
                ..Default::default()
            },
        )
    })
    .expect("search")
    .items
    .into_iter()
    .map(|item| item.title)
    .collect()
}

#[test]
fn search_hits_body_tags_chinese_and_stays_literal() {
    let (db, path) = temp_db();

    let tag = db
        .with_conn(|conn| tag_repo::create(conn, "rust", None))
        .expect("create tag");

    seed(&db, "Async", "sqlite writer lives here", &[]);
    seed(&db, "Tagged", "no keyword in the body", &[tag.id.clone()]);
    seed(&db, "Percent", "usage reached 100% yesterday", &[]);
    seed(&db, "Rounded", "usage reached 1000 yesterday", &[]);
    seed(&db, "Chinese", "笔记管理的要点", &[]);
    seed(&db, "MidRun", "工程管理实践", &[]);
    seed(&db, "Underscore", "identifier a_b in code", &[]);

    assert_eq!(search(&db, "sqlite"), vec!["Async".to_string()]);
    assert_eq!(search(&db, "rust"), vec!["Tagged".to_string()], "tag hit");
    assert_eq!(
        search(&db, "100%"),
        vec!["Percent".to_string()],
        "% stays literal"
    );
    assert_eq!(search(&db, "笔记"), vec!["Chinese".to_string()], "2 CJK chars");
    let mut managed = search(&db, "管理");
    managed.sort();
    assert_eq!(
        managed,
        vec!["Chinese".to_string(), "MidRun".to_string()],
        "CJK substring in the middle of a run"
    );
    // UTF-8 ordering inside the same millisecond is by id, not insertion order.
    assert!(search(&db, "sql").contains(&"Async".to_string()), "prefix match");
    assert!(
        search(&db, "sqlitewriter").is_empty(),
        "prefix must not stretch past the token"
    );
    assert_eq!(search(&db, "a_b"), vec!["Underscore".to_string()]);
    assert!(search(&db, "foo AND \"bar").is_empty(), "bad syntax must not error");
    assert!(search(&db, "++").is_empty(), "unindexable input falls back cleanly");

    assert!(search(&db, "sqlite notasuchword").is_empty(), "tokens AND together");

    drop(db);
    let _ = std::fs::remove_file(path);
}

#[test]
fn search_composes_with_filters_and_skips_deleted() {
    let (db, path) = temp_db();

    let rust = db
        .with_conn(|conn| tag_repo::create(conn, "rust", None))
        .expect("create tag");
    let kotlin = db
        .with_conn(|conn| tag_repo::create(conn, "kotlin", None))
        .expect("create tag");

    let sqlite_rust = seed(&db, "SQLite rust", "sqlite writer", &[rust.id.clone()]);
    let sqlite_kotlin = seed(&db, "SQLite kotlin", "sqlite coroutines", &[kotlin.id.clone()]);
    let prose_rust = seed(&db, "Prose rust", "no database here", &[rust.id.clone()]);
    let deleted = seed(&db, "SQLite gone", "sqlite writer deleted later", &[rust.id.clone()]);

    db.with_conn(|conn| note_repo::set_favorite(conn, &sqlite_kotlin, true))
        .expect("favorite");
    db.with_conn(|conn| note_repo::soft_delete(conn, &deleted))
        .expect("soft delete");

    let titles = |query: ListQuery| {
        db.with_conn(|conn| note_repo::list(conn, &query))
            .expect("list")
            .items
            .into_iter()
            .map(|item| item.title)
            .collect::<Vec<_>>()
    };

    // Keyword + tag: intersection, not union.
    let mut both = titles(ListQuery {
        search: Some("sqlite".into()),
        tag_ids: vec![rust.id.clone()],
        ..Default::default()
    });
    both.sort();
    assert_eq!(both, vec!["SQLite rust".to_string()]);

    // Keyword + favorites.
    let mut favorite = titles(ListQuery {
        search: Some("sqlite".into()),
        favorites_only: true,
        ..Default::default()
    });
    favorite.sort();
    assert_eq!(favorite, vec!["SQLite kotlin".to_string()]);

    // Soft-deleted notes never surface, with or without a keyword.
    let with_keyword = titles(ListQuery {
        search: Some("sqlite".into()),
        ..Default::default()
    });
    assert!(!with_keyword.contains(&"SQLite gone".to_string()));
    let without_keyword = titles(ListQuery::default());
    assert!(!without_keyword.contains(&"SQLite gone".to_string()));
    assert_eq!(without_keyword.len(), 3);

    // Dropping the keyword keeps the filters applied.
    let mut tags_only = titles(ListQuery {
        tag_ids: vec![rust.id.clone()],
        ..Default::default()
    });
    tags_only.sort();
    assert_eq!(tags_only, vec!["Prose rust".to_string(), "SQLite rust".to_string()]);

    drop(db);
    let _ = std::fs::remove_file(path);
}
