use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use macos_notes_lib::database::Database;
use macos_notes_lib::models::AppError;
use macos_notes_lib::repositories::note_repo::{self, ListQuery};
use macos_notes_lib::repositories::tag_repo;

fn temp_db() -> (Database, PathBuf) {
    // The clock alone is not enough: macOS resolves it to microseconds, so two
    // tests starting in the same tick would share one file and fight over
    // `schema_migrations`.
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("macos-notes-test-{unique}-{seq}.db"));
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

    let work = db
        .with_conn(|conn| tag_repo::create(conn, "work", None))
        .expect("first tag");
    let duplicate = db.with_conn(|conn| tag_repo::create(conn, "WORK", None));
    assert!(matches!(duplicate, Err(AppError::Conflict(_))));

    // Colour is a presentation field: it changes without touching relations.
    let colored = db
        .with_conn(|conn| tag_repo::set_color(conn, &work.id, Some("#0a66c2".into())))
        .expect("set colour");
    assert_eq!(colored.color.as_deref(), Some("#0a66c2"));
    assert_eq!(colored.name, "work");
    let cleared = db
        .with_conn(|conn| tag_repo::set_color(conn, &work.id, None))
        .expect("clear colour");
    assert!(cleared.color.is_none());
    let missing_tag = db.with_conn(|conn| tag_repo::set_color(conn, "no-such-tag", None));
    assert!(matches!(missing_tag, Err(AppError::NotFound(_))));

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

/// Renaming or deleting a tag is a relation change, never a content change:
/// the notes keep their body, timestamps and their own existence, and the
/// search projection follows the new name.
#[test]
fn renaming_and_deleting_a_tag_keeps_notes_intact() {
    let (db, path) = temp_db();

    let work = db
        .with_conn(|conn| tag_repo::create(conn, "work", None))
        .expect("create tag");
    let ids = ["笔记一", "笔记二", "笔记三"]
        .iter()
        .map(|title| seed(&db, title, "同一段正文", &[work.id.clone()]))
        .collect::<Vec<_>>();
    let before = ids
        .iter()
        .map(|id| {
            let note = db
                .with_conn(|conn| note_repo::get(conn, id))
                .expect("get note");
            (note.updated_at, note.content)
        })
        .collect::<Vec<_>>();
    assert_eq!(search(&db, "work").len(), 3);

    db.with_conn(|conn| tag_repo::rename(conn, &work.id, "job"))
        .expect("rename tag");

    // The old name is gone from the projection, the new one hits all three.
    assert!(search(&db, "work").is_empty());
    assert_eq!(search(&db, "job").len(), 3);
    let renamed = db
        .with_conn(|conn| tag_repo::list_with_counts(conn))
        .expect("list tags");
    assert_eq!(
        renamed
            .iter()
            .find(|tag| tag.id == work.id)
            .map(|tag| (tag.name.as_str(), tag.note_count)),
        Some(("job", 3))
    );

    db.with_conn(|conn| tag_repo::delete(conn, &work.id))
        .expect("delete tag");

    // Nothing was deleted along with the tag: notes, bodies and timestamps hold.
    for (index, id) in ids.iter().enumerate() {
        let note = db
            .with_conn(|conn| note_repo::get(conn, id))
            .expect("note still exists");
        assert_eq!(note.content, before[index].1);
        assert_eq!(note.updated_at, before[index].0);
        assert!(note.tags.is_empty());
    }
    assert!(db
        .with_conn(|conn| tag_repo::list_with_counts(conn))
        .expect("list tags")
        .is_empty());
    assert!(search(&db, "job").is_empty());

    drop(db);
    let _ = std::fs::remove_file(path);
}

/// A library created by v0.1 (schema v3) must survive the v4 upgrade: the
/// import columns are additive, so no row is rewritten and no note is lost.
#[test]
fn upgrade_from_v3_preserves_notes_and_adds_import_columns() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("macos-notes-v3-{unique}.db"));

    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);",
        )
        .unwrap();
        for (version, sql) in [
            (1i64, include_str!("../migrations/0001_init.sql")),
            (2, include_str!("../migrations/0002_fts.sql")),
            (3, include_str!("../migrations/0003_search_reindex.sql")),
        ] {
            conn.execute_batch(sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, 0)",
                [version],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO notes(id, title, content, created_at, updated_at, is_favorite)
             VALUES ('legacy', '旧笔记', '升级前写入的正文', 1000, 2000, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tags(id, name, created_at) VALUES ('t1', 'work', 1000)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO note_tags(note_id, tag_id) VALUES ('legacy', 't1')",
            [],
        )
        .unwrap();
    }

    let db = Database::open(&path).expect("upgrade to v4");
    db.with_conn(|conn| {
        let (title, content, updated, favorite, source): (String, String, i64, i64, Option<String>) =
            conn.query_row(
                "SELECT title, content, updated_at, is_favorite, source_path FROM notes WHERE id = 'legacy'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )?;
        assert_eq!(title, "旧笔记");
        assert_eq!(content, "升级前写入的正文");
        assert_eq!(updated, 2000);
        assert_eq!(favorite, 1);
        assert_eq!(source, None, "hand-written notes keep a NULL source_path");
        Ok(())
    })
    .expect("read legacy row");

    // Two rows may not claim the same source file, but NULLs are unconstrained.
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO notes(id, title, content, created_at, updated_at, source_path)
             VALUES ('dup-a', 'a', '', 1, 1, '/vault/a.md')",
            [],
        )?;
        let dup = conn.execute(
            "INSERT INTO notes(id, title, content, created_at, updated_at, source_path)
             VALUES ('dup-b', 'b', '', 1, 1, '/vault/a.md')",
            [],
        );
        assert!(
            dup.is_err(),
            "partial unique index rejects a second import of the same file"
        );
        Ok(())
    })
    .expect("unique source_path");

    drop(db);
    let _ = std::fs::remove_file(&path);
}

/// Counts a note id across every table permanent deletion must clean up.
fn rows_for(db: &Database, id: &str) -> (i64, i64, i64) {
    db.with_conn(|conn| {
        let notes: i64 = conn.query_row("SELECT count(*) FROM notes WHERE id = ?1", [id], |r| {
            r.get(0)
        })?;
        let links: i64 =
            conn.query_row("SELECT count(*) FROM note_tags WHERE note_id = ?1", [id], |r| {
                r.get(0)
            })?;
        let fts: i64 =
            conn.query_row("SELECT count(*) FROM notes_fts WHERE note_id = ?1", [id], |r| {
                r.get(0)
            })?;
        Ok((notes, links, fts))
    })
    .expect("count rows")
}

#[test]
fn purge_removes_the_note_body_links_and_search_row() {
    let (db, path) = temp_db();
    let tag = db
        .with_conn(|conn| tag_repo::create(conn, "work", None))
        .expect("create tag");
    let id = seed(&db, "投放清单", "只有这一段正文", &[tag.id.clone()]);
    assert_eq!(rows_for(&db, &id), (1, 1, 1));
    assert_eq!(search(&db, "投放清单"), vec!["投放清单".to_string()]);

    // A live note is not purgeable: "permanently delete" must not bypass the trash.
    let rejected = db.with_conn(|conn| note_repo::purge(conn, &id));
    assert!(matches!(rejected, Err(AppError::NotFound(_))), "{rejected:?}");
    assert_eq!(rows_for(&db, &id), (1, 1, 1), "a rejected purge changes nothing");
    assert_eq!(search(&db, "投放清单").len(), 1);

    db.with_conn(|conn| note_repo::soft_delete(conn, &id))
        .expect("soft delete");
    assert_eq!(rows_for(&db, &id), (1, 1, 0), "trashing drops the search row");

    db.with_conn(|conn| note_repo::purge(conn, &id))
        .expect("purge");
    assert_eq!(rows_for(&db, &id), (0, 0, 0), "notes / note_tags / notes_fts");
    assert_eq!(search(&db, "投放清单"), Vec::<String>::new());

    let again = db.with_conn(|conn| note_repo::purge(conn, &id));
    assert!(matches!(again, Err(AppError::NotFound(_))), "{again:?}");

    drop(db);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn empty_trash_keeps_live_notes_and_their_deletion_timestamps_are_real() {
    let (db, path) = temp_db();
    let tag = db
        .with_conn(|conn| tag_repo::create(conn, "work", None))
        .expect("create tag");

    let doomed: Vec<String> = ["废稿一", "废稿二", "废稿三"]
        .iter()
        .map(|title| seed(&db, title, "销毁前正文", &[tag.id.clone()]))
        .collect();
    let kept: Vec<String> = ["留着一", "留着二"]
        .iter()
        .map(|title| seed(&db, title, "保留正文", &[tag.id.clone()]))
        .collect();
    for id in &doomed {
        db.with_conn(|conn| note_repo::soft_delete(conn, id))
            .expect("soft delete");
    }

    let kept_before: Vec<(i64, String)> = kept
        .iter()
        .map(|id| {
            let note = db.with_conn(|conn| note_repo::get(conn, id)).expect("get kept");
            (note.updated_at, note.content)
        })
        .collect();

    // `deleted_at` is the deletion moment, not `updated_at` living a double life.
    let listed = db
        .with_conn(|conn| note_repo::list_deleted(conn, None))
        .expect("list deleted");
    assert_eq!(listed.len(), 3);
    let marked: Vec<i64> = listed.iter().map(|n| n.deleted_at.expect("deletedAt")).collect();
    assert!(marked.iter().all(|ts| *ts > 0));
    let live = db
        .with_conn(|conn| {
            note_repo::list(
                conn,
                &ListQuery {
                    limit: Some(10),
                    ..Default::default()
                },
            )
        })
        .expect("list live")
        .items;
    assert!(
        live.iter().all(|n| n.deleted_at.is_none()),
        "the live listing leaves deletedAt empty"
    );

    assert_eq!(db.with_conn(|conn| note_repo::count_deleted(conn)).expect("count"), 3);
    let removed = db
        .with_conn(|conn| note_repo::purge_all(conn))
        .expect("empty trash");
    assert_eq!(removed, 3);
    assert_eq!(db.with_conn(|conn| note_repo::count_deleted(conn)).expect("count"), 0);
    assert!(db
        .with_conn(|conn| note_repo::list_deleted(conn, None))
        .expect("list deleted")
        .is_empty());

    for id in &doomed {
        assert_eq!(rows_for(&db, id), (0, 0, 0));
    }
    for (id, before) in kept.iter().zip(kept_before) {
        assert_eq!(rows_for(&db, id), (1, 1, 1), "live note untouched");
        let note = db.with_conn(|conn| note_repo::get(conn, id)).expect("get kept");
        assert_eq!((note.updated_at, note.content.clone()), before);
        assert_eq!(
            note.tags.iter().map(|tag| tag.name.as_str()).collect::<Vec<_>>(),
            vec!["work"]
        );
    }
    assert_eq!(search(&db, "保留正文").len(), 2);
    assert_eq!(search(&db, "销毁前正文"), Vec::<String>::new());

    drop(db);
    let _ = std::fs::remove_file(&path);
}
