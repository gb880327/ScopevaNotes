use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use macos_notes_lib::database::Database;
use macos_notes_lib::models::AppError;
use macos_notes_lib::repositories::note_repo::{self, ListQuery};
use macos_notes_lib::repositories::quick_tab_repo::{self, COLORS, MAX_TABS};
use macos_notes_lib::repositories::tag_repo;

fn temp_db() -> (Database, PathBuf) {
    // Same guard as tests/database.rs: the clock alone collides when two tests
    // start in the same microsecond tick.
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("macos-notes-quick-tabs-{unique}-{seq}.db"));
    let db = Database::open(&path).expect("open temp database");
    (db, path)
}

fn table_rows(db: &Database, table: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    db.with_conn(|conn| {
        let count: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
        Ok(count)
    })
    .expect("count rows")
}

#[test]
fn crud_survives_a_round_trip_and_duplicate_is_independent() {
    let (db, _path) = temp_db();

    let first = db
        .with_conn(|conn| quick_tab_repo::create(conn, "暂存", None))
        .expect("create first tab");
    let second = db
        .with_conn(|conn| quick_tab_repo::create(conn, "草稿", None))
        .expect("create second tab");

    // Default colour rotates through the fixed palette.
    assert_eq!(first.color, COLORS[0]);
    assert_eq!(second.color, COLORS[1]);
    assert!(first.id != second.id, "ids must be distinct");

    let listed = db
        .with_conn(|conn| quick_tab_repo::list(conn))
        .expect("list");
    assert_eq!(
        listed.iter().map(|t| t.id.clone()).collect::<Vec<_>>(),
        vec![first.id.clone(), second.id.clone()],
        "list is ordered by sort_order"
    );

    let updated = db
        .with_conn(|conn| {
            quick_tab_repo::update(
                conn,
                &first.id,
                Some("改名"),
                Some("red"),
                Some("第一行\n第二行"),
            )
        })
        .expect("update");
    assert_eq!(updated.title, "改名");
    assert_eq!(updated.color, "red");
    assert_eq!(updated.content, "第一行\n第二行");
    assert!(updated.updated_at >= first.updated_at);

    // Partial update leaves untouched columns alone.
    let title_only = db
        .with_conn(|conn| quick_tab_repo::update(conn, &first.id, Some("再改名"), None, None))
        .expect("partial update");
    assert_eq!(title_only.color, "red");
    assert_eq!(title_only.content, "第一行\n第二行");

    let copy = db
        .with_conn(|conn| quick_tab_repo::duplicate(conn, &first.id))
        .expect("duplicate");
    assert_ne!(copy.id, first.id);
    assert_eq!(copy.content, title_only.content);
    assert_eq!(copy.color, "red");
    assert_eq!(copy.title, "再改名 副本");

    let edited = db
        .with_conn(|conn| quick_tab_repo::update(conn, &copy.id, None, None, Some("副本内容")))
        .expect("edit copy");
    assert_eq!(edited.content, "副本内容");
    let source_still = db
        .with_conn(|conn| quick_tab_repo::get(conn, &first.id))
        .expect("source still there");
    assert_eq!(
        source_still.content, "第一行\n第二行",
        "editing a copy must not touch the source"
    );

    db.with_conn(|conn| quick_tab_repo::delete(conn, &second.id))
        .expect("delete");
    assert_eq!(db.with_conn(|conn| quick_tab_repo::count(conn)).unwrap(), 2);

    let missing = db
        .with_conn(|conn| quick_tab_repo::delete(conn, &second.id))
        .expect_err("deleting twice must fail");
    assert!(matches!(missing, AppError::NotFound(_)));
    let missing_update = db
        .with_conn(|conn| quick_tab_repo::update(conn, "nope", Some("x"), None, None))
        .expect_err("updating a missing tab must fail");
    assert!(matches!(missing_update, AppError::NotFound(_)));
}

#[test]
fn the_cap_is_twelve_and_is_enforced_in_rust() {
    let (db, _path) = temp_db();

    for index in 0..MAX_TABS {
        db.with_conn(|conn| quick_tab_repo::create(conn, &format!("tab {index}"), None))
            .expect("create within the cap");
    }
    assert_eq!(
        db.with_conn(|conn| quick_tab_repo::count(conn)).unwrap(),
        12
    );

    let rejected = db
        .with_conn(|conn| quick_tab_repo::create(conn, "overflow", None))
        .expect_err("the 13th tab must be rejected");
    match rejected {
        AppError::Invalid(message) => assert!(message.contains("12"), "got: {message}"),
        other => panic!("expected an invalid-input error, got {other:?}"),
    }
    assert_eq!(
        db.with_conn(|conn| quick_tab_repo::count(conn)).unwrap(),
        12,
        "a rejected create must not leave a row behind"
    );

    // Duplicate is a create too, so it hits the same ceiling.
    let source = db
        .with_conn(|conn| quick_tab_repo::list(conn))
        .unwrap()
        .remove(0);
    let dup_rejected = db
        .with_conn(|conn| quick_tab_repo::duplicate(conn, &source.id))
        .expect_err("duplicate must respect the cap");
    assert!(matches!(dup_rejected, AppError::Invalid(_)));

    db.with_conn(|conn| quick_tab_repo::delete(conn, &source.id))
        .expect("delete to free a slot");
    db.with_conn(|conn| quick_tab_repo::create(conn, "back to twelve", None))
        .expect("create after freeing a slot");
    assert_eq!(
        db.with_conn(|conn| quick_tab_repo::count(conn)).unwrap(),
        12
    );
}

#[test]
fn quick_tabs_never_reach_the_note_tables() {
    let (db, _path) = temp_db();

    let tag = db
        .with_conn(|conn| tag_repo::create(conn, "inbox", None))
        .expect("create tag");
    let note = db
        .with_conn(|conn| note_repo::create(conn, "正式笔记", "annote-marker", &[tag.id.clone()]))
        .expect("create note");

    let before = (
        table_rows(&db, "notes"),
        table_rows(&db, "notes_fts"),
        table_rows(&db, "tags"),
        table_rows(&db, "note_tags"),
    );

    let secret = "quicktab-secret-marker";
    let tab = db
        .with_conn(|conn| quick_tab_repo::create(conn, "隔离", None))
        .expect("create tab");
    db.with_conn(|conn| quick_tab_repo::update(conn, &tab.id, None, None, Some(secret)))
        .expect("write content");

    let after = (
        table_rows(&db, "notes"),
        table_rows(&db, "notes_fts"),
        table_rows(&db, "tags"),
        table_rows(&db, "note_tags"),
    );
    assert_eq!(
        before, after,
        "quick tabs must not write any note-side table"
    );

    // Searching for the tab's content finds nothing; the note is still found.
    let hit_for_tab = db
        .with_conn(|conn| {
            note_repo::list(
                conn,
                &ListQuery {
                    search: Some(secret.to_string()),
                    ..Default::default()
                },
            )
        })
        .expect("search for tab content");
    assert!(
        hit_for_tab.items.is_empty(),
        "quick tab content must not be searchable"
    );

    let hit_for_note = db
        .with_conn(|conn| {
            note_repo::list(
                conn,
                &ListQuery {
                    search: Some("annote-marker".to_string()),
                    ..Default::default()
                },
            )
        })
        .expect("search for note content");
    assert_eq!(hit_for_note.items.len(), 1);
    assert_eq!(hit_for_note.items[0].id, note.id);

    // Trash is unaffected: neither the count nor the listing sees the tab.
    assert_eq!(
        db.with_conn(|conn| note_repo::count_deleted(conn)).unwrap(),
        0
    );
    assert!(db
        .with_conn(|conn| note_repo::list_deleted(conn, None))
        .unwrap()
        .is_empty());
    assert_eq!(
        db.with_conn(|conn| tag_repo::list_with_counts(conn))
            .unwrap()[0]
            .note_count,
        1
    );
}

#[test]
fn content_round_trips_byte_for_byte() {
    let (db, _path) = temp_db();

    let raw = "第一行\t带制表符\r\nsecond line\n\n  emoji: 🧪🚀  \n尾行";
    let tab = db
        .with_conn(|conn| quick_tab_repo::create(conn, "保真", None))
        .expect("create tab");
    let stored = db
        .with_conn(|conn| quick_tab_repo::update(conn, &tab.id, None, None, Some(raw)))
        .expect("write content");
    assert_eq!(stored.content, raw);

    let reread = db
        .with_conn(|conn| quick_tab_repo::get(conn, &tab.id))
        .expect("read back");
    assert_eq!(reread.content, raw);
    assert_eq!(
        reread.content.as_bytes(),
        raw.as_bytes(),
        "no normalisation of CRLF, tabs or trailing spaces"
    );
}

#[test]
fn unknown_colors_are_rejected() {
    let (db, _path) = temp_db();
    let err = db
        .with_conn(|conn| quick_tab_repo::create(conn, "bad color", Some("#ff0000")))
        .expect_err("hex values are not colour keys");
    assert!(matches!(err, AppError::Invalid(_)));
    assert_eq!(db.with_conn(|conn| quick_tab_repo::count(conn)).unwrap(), 0);

    let tab = db
        .with_conn(|conn| quick_tab_repo::create(conn, "ok", Some("CYAN")))
        .expect("colour keys are case-insensitive");
    assert_eq!(tab.color, "cyan");
}

#[test]
fn dto_serializes_as_camel_case() {
    let (db, _path) = temp_db();
    let tab = db
        .with_conn(|conn| quick_tab_repo::create(conn, "dto", None))
        .expect("create tab");
    let json = serde_json::to_value(&tab).expect("serialize");
    for key in [
        "id",
        "title",
        "color",
        "content",
        "sortOrder",
        "createdAt",
        "updatedAt",
    ] {
        assert!(json.get(key).is_some(), "missing key {key} in {json}");
    }
    assert!(
        json.get("sort_order").is_none(),
        "snake_case must not leak into the DTO"
    );
}
