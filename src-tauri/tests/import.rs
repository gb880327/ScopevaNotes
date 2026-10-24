use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use macos_notes_lib::database::Database;
use macos_notes_lib::repositories::note_repo::{self, ImportRow, ListQuery};
use macos_notes_lib::repositories::tag_repo;

fn temp_db() -> (Database, PathBuf) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("macos-notes-import-{unique}.db"));
    let db = Database::open(&path).expect("open temp database");
    (db, path)
}

/// Task 1.2: the imported row must behave exactly like a written one — tags
/// linked, FTS updated in the same transaction, file timestamps preserved.
#[test]
fn insert_imported_keeps_timestamps_tags_and_search() {
    let (db, path) = temp_db();

    let tag = db
        .with_conn(|conn| tag_repo::create(conn, "work", None))
        .expect("create tag");

    let id = db
        .with_conn(|conn| {
            let tx = conn.transaction()?;
            let id = note_repo::insert_imported(
                &tx,
                &ImportRow {
                    source_path: "/vault/Projects/关键词笔记.md",
                    content_hash: "abc123",
                    title: "关键词笔记",
                    content: "正文包含迁移验证关键字，还有 [[双链]] 与 ![](img/a.png)",
                    created_at: 1_600_000_000_000,
                    updated_at: 1_700_000_000_000,
                    tag_ids: &[tag.id.clone()],
                },
            )?;
            tx.commit()?;
            Ok::<_, macos_notes_lib::models::AppError>(id)
        })
        .expect("insert imported note");

    let detail = db
        .with_conn(|conn| note_repo::get(conn, &id))
        .expect("read back");
    assert_eq!(detail.title, "关键词笔记");
    assert_eq!(detail.created_at, 1_600_000_000_000);
    assert_eq!(detail.updated_at, 1_700_000_000_000);
    assert_eq!(detail.tags.len(), 1);
    assert_eq!(detail.tags[0].name, "work");
    // Byte-level fidelity: nothing in the body was rewritten (design D1).
    assert!(detail.content.contains("[[双链]]"));
    assert!(detail.content.contains("![](img/a.png)"));

    let hits = db
        .with_conn(|conn| {
            note_repo::list(
                conn,
                &ListQuery {
                    search: Some("迁移验证关键字".into()),
                    ..Default::default()
                },
            )
        })
        .expect("search imported note");
    assert_eq!(
        hits.items.len(),
        1,
        "FTS is updated by the same transaction"
    );
    assert_eq!(hits.items[0].id, id);

    drop(db);
    let _ = std::fs::remove_file(path);
}

/// Task 1.3: existence is keyed by the absolute source path, and the partial
/// unique index is the backstop against a double insert.
#[test]
fn source_path_exists_and_is_unique() {
    let (db, path) = temp_db();

    assert!(!db
        .with_conn(|conn| note_repo::source_path_exists(conn, "/vault/a.md"))
        .expect("probe"));

    db.with_conn(|conn| {
        let tx = conn.transaction()?;
        note_repo::insert_imported(
            &tx,
            &ImportRow {
                source_path: "/vault/a.md",
                content_hash: "h1",
                title: "a",
                content: "",
                created_at: 1,
                updated_at: 1,
                tag_ids: &[],
            },
        )?;
        tx.commit()?;
        Ok::<_, macos_notes_lib::models::AppError>(())
    })
    .expect("first insert");

    assert!(db
        .with_conn(|conn| note_repo::source_path_exists(conn, "/vault/a.md"))
        .expect("probe after insert"));

    let duplicate = db.with_conn(|conn| {
        let tx = conn.transaction()?;
        note_repo::insert_imported(
            &tx,
            &ImportRow {
                source_path: "/vault/a.md",
                content_hash: "h2",
                title: "a again",
                content: "",
                created_at: 2,
                updated_at: 2,
                tag_ids: &[],
            },
        )?;
        tx.commit()?;
        Ok::<_, macos_notes_lib::models::AppError>(())
    });
    assert!(
        duplicate.is_err(),
        "same source file may not be imported twice"
    );

    // Hand-written notes carry a NULL source path and are not affected by the
    // uniqueness rule.
    let manual = db
        .with_conn(|conn| note_repo::create(conn, "手动笔记", "", &[]))
        .expect("create manual note");
    let manual2 = db
        .with_conn(|conn| note_repo::create(conn, "手动笔记 2", "", &[]))
        .expect("create second manual note");
    assert_ne!(manual.id, manual2.id);

    drop(db);
    let _ = std::fs::remove_file(path);
}

// ---------------------------------------------------------------------------
// Service-level tests (tasks 3.1 - 3.5). The Tauri commands are thin adapters
// over these functions, so the behaviour is verified here.
// ---------------------------------------------------------------------------

use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};

use macos_notes_lib::services::import_service::{self, ImportOptions, ImportReport};

fn scratch(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("import-svc-{name}-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A vault shaped like the real thing: hidden config, two subdirectories,
/// frontmatter, a searchable keyword and a local image reference.
fn vault(name: &str, count: usize) -> PathBuf {
    let root = scratch(name);
    fs::create_dir_all(root.join(".obsidian")).unwrap();
    fs::write(root.join(".obsidian/workspace.json"), "{}").unwrap();
    for i in 0..count {
        let sub = if i % 2 == 0 { "Projects" } else { "Inbox" };
        fs::create_dir_all(root.join(sub)).unwrap();
        fs::write(
            root.join(sub).join(format!("note-{i}.md")),
            format!(
                "---\ntitle: 笔记 {i}\ntags: [work]\ncreated: 2024-01-02\n---\n\n# 标题 {i}\n\n正文 {i} 迁移验证关键字 ![](img/{i}.png)\n"
            ),
        )
        .unwrap();
    }
    root
}

fn count(db: &Database, sql: &str) -> i64 {
    db.with_conn(|conn| Ok(conn.query_row(sql, [], |row| row.get(0))?))
        .unwrap()
}

fn import(
    db: &Database,
    root: &PathBuf,
    options: ImportOptions,
    cancel_after: Option<usize>,
) -> ImportReport {
    let cancel = AtomicBool::new(false);
    import_service::run_import(
        db,
        &cancel,
        &root.to_string_lossy(),
        options,
        &mut |progress| {
            if let Some(threshold) = cancel_after {
                if progress.done >= threshold {
                    cancel.store(true, Ordering::SeqCst);
                }
            }
        },
    )
    .unwrap()
}

/// Task 3.1: the preview must be a pure read, and its numbers must match what
/// the execution then does.
#[test]
fn scan_is_read_only_and_matches_execution() {
    let (db, path) = temp_db();
    let root = vault("scan", 12);
    let before_notes = count(&db, "SELECT count(*) FROM notes");
    let before_tags = count(&db, "SELECT count(*) FROM tags");
    let before_fts = count(&db, "SELECT count(*) FROM notes_fts");

    // Spec "只读原目录": the whole tree must come out untouched (content and
    // mtime), including the hidden and non-Markdown entries.
    let snapshot = |root: &std::path::Path| -> Vec<(String, u64, std::time::SystemTime)> {
        let mut entries = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).unwrap() {
                let entry = entry.unwrap();
                let meta = entry.metadata().unwrap();
                entries.push((
                    entry.path().to_string_lossy().to_string(),
                    meta.len(),
                    meta.modified().unwrap(),
                ));
                if meta.is_dir() {
                    stack.push(entry.path());
                }
            }
        }
        entries.sort();
        entries
    };
    let tree_before = snapshot(&root);

    let preview = db
        .with_conn(|_| Ok(()))
        .map(|_| import_service::scan(&db, &root.to_string_lossy()).unwrap())
        .unwrap();
    assert_eq!(preview.total_files, 12);
    assert_eq!(preview.to_import, 12);
    assert_eq!(preview.skipped_existing, 0);
    assert_eq!(
        preview.hidden_skipped, 1,
        ".obsidian is reported, not ignored"
    );
    assert_eq!(preview.local_file_refs, 12, "notes with a local image");
    assert!(preview.unreadable.is_empty());

    assert_eq!(count(&db, "SELECT count(*) FROM notes"), before_notes);
    assert_eq!(count(&db, "SELECT count(*) FROM tags"), before_tags);
    assert_eq!(count(&db, "SELECT count(*) FROM notes_fts"), before_fts);

    let report = import(&db, &root, ImportOptions::default(), None);
    assert_eq!(report.imported, preview.to_import);
    assert_eq!(report.hidden_skipped, preview.hidden_skipped);
    assert_eq!(report.local_file_references, preview.local_file_refs);
    assert_eq!(count(&db, "SELECT count(*) FROM notes"), 12);

    // Task 2.4 / spec "时间戳保留原始顺序": `updated_at` is the file mtime,
    // not the import time.
    let file = root.join("Inbox").join("note-1.md");
    let mtime = fs::metadata(&file)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let imported_at: i64 = db
        .with_conn(|conn| {
            Ok(conn.query_row(
                "SELECT updated_at FROM notes WHERE source_path = ?1",
                [file.to_string_lossy().to_string()],
                |row| row.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(imported_at, mtime, "mtime is preserved to the millisecond");
    assert_eq!(snapshot(&root), tree_before, "the source tree is read-only");

    let _ = fs::remove_dir_all(&root);
    drop(db);
    let _ = fs::remove_file(path);
}

/// Tasks 3.2 + 3.4: 1,000 files in 200-row batches, progress per batch, batch
/// tag on exactly the imported notes.
#[test]
fn import_batches_reports_progress_and_tags_the_batch() {
    let (db, path) = temp_db();
    let manual = db
        .with_conn(|conn| note_repo::create(conn, "手动笔记", "原有内容", &[]))
        .unwrap();
    let root = vault("bulk", 1000);

    let cancel = AtomicBool::new(false);
    let mut events: Vec<usize> = Vec::new();
    let report = import_service::run_import(
        &db,
        &cancel,
        &root.to_string_lossy(),
        ImportOptions::default(),
        &mut |progress| events.push(progress.done),
    )
    .unwrap();

    assert_eq!(report.total_files, 1000);
    assert_eq!(report.imported, 1000);
    assert_eq!(report.skipped_existing, 0);
    assert!(!report.cancelled);
    assert!(
        report.elapsed_ms < 60_000,
        "1,000 files must not take minutes"
    );
    assert_eq!(
        events,
        vec![200, 400, 600, 800, 1000],
        "one event per batch"
    );
    assert_eq!(count(&db, "SELECT count(*) FROM notes"), 1001);

    // Batch tag exists once and is attached to the 1,000 imported notes only.
    let batch_tag = report.batch_tag.clone();
    assert!(batch_tag.starts_with("import-"), "tag: {batch_tag}");
    assert_eq!(
        count(&db, "SELECT count(*) FROM tags WHERE name = 'work'"),
        1,
        "the shared frontmatter tag is reused"
    );
    let tagged = db
        .with_conn(|conn| {
            Ok(conn.query_row(
                "SELECT count(*) FROM note_tags nt JOIN tags t ON t.id = nt.tag_id WHERE t.name = ?1",
                [&batch_tag],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .unwrap();
    assert_eq!(tagged, 1000);
    let manual_tagged = db
        .with_conn(|conn| {
            Ok(conn.query_row(
                "SELECT count(*) FROM note_tags nt JOIN tags t ON t.id = nt.tag_id
                 WHERE t.name = ?1 AND nt.note_id = ?2",
                rusqlite::params![batch_tag, manual.id],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .unwrap();
    assert_eq!(manual_tagged, 0, "pre-existing notes are untouched");
    let manual_after = db
        .with_conn(|conn| note_repo::get(conn, &manual.id))
        .unwrap();
    assert_eq!(manual_after.title, "手动笔记");
    assert_eq!(manual_after.content, "原有内容");
    assert_eq!(manual_after.updated_at, manual.updated_at);

    // Imported notes are searchable immediately.
    let hits = db
        .with_conn(|conn| {
            note_repo::list(
                conn,
                &ListQuery {
                    search: Some("迁移验证关键字".into()),
                    limit: Some(1),
                    ..Default::default()
                },
            )
        })
        .unwrap();
    assert_eq!(
        hits.items.len(),
        1,
        "the page is capped, the search still hits"
    );

    let _ = fs::remove_dir_all(&root);
    drop(db);
    let _ = fs::remove_file(path);
}

/// Tasks 3.2 + 3.3 + retry: cancelling keeps the committed batches, and a
/// second run fills only the gap.
#[test]
fn cancel_keeps_committed_batches_and_retry_fills_the_gap() {
    let (db, path) = temp_db();
    let root = vault("cancel", 300);

    let report = import(
        &db,
        &root,
        ImportOptions {
            batch_size: Some(100),
            parent_dir_tag: false,
        },
        Some(100),
    );
    assert!(report.cancelled, "cancel flag set after the first batch");
    assert_eq!(report.imported, 100);
    assert_eq!(count(&db, "SELECT count(*) FROM notes"), 100);

    let retry = import(&db, &root, ImportOptions::default(), None);
    assert!(!retry.cancelled);
    assert_eq!(retry.imported, 200);
    assert_eq!(retry.skipped_existing, 100);
    assert_eq!(count(&db, "SELECT count(*) FROM notes"), 300);

    // Third run: fully idempotent, and the preview already says so.
    let preview = import_service::scan(&db, &root.to_string_lossy()).unwrap();
    assert_eq!(preview.to_import, 0);
    assert_eq!(preview.skipped_existing, 300);
    let again = import(&db, &root, ImportOptions::default(), None);
    assert_eq!(again.imported, 0);
    assert_eq!(again.skipped_existing, 300);
    assert_eq!(count(&db, "SELECT count(*) FROM notes"), 300);

    let _ = fs::remove_dir_all(&root);
    drop(db);
    let _ = fs::remove_file(path);
}

/// Task 3.5: undo goes through the trash, not a hard delete.
#[test]
fn delete_import_batch_moves_notes_to_trash_and_restores() {
    let (db, path) = temp_db();
    let earlier = db
        .with_conn(|conn| note_repo::create(conn, "早前的笔记", "不该被撤销", &[]))
        .unwrap();
    let root = vault("undo", 6);
    let report = import(&db, &root, ImportOptions::default(), None);
    assert_eq!(report.imported, 6);

    let ids: Vec<String> = db
        .with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT n.id FROM notes n JOIN note_tags nt ON nt.note_id = n.id
                 JOIN tags t ON t.id = nt.tag_id WHERE t.name = ?1",
            )?;
            let rows = stmt
                .query_map([report.batch_tag.clone()], |row| row.get(0))?
                .collect::<Result<_, _>>()?;
            Ok(rows)
        })
        .unwrap();

    let removed = db
        .with_conn(|conn| note_repo::soft_delete_by_tag(conn, &report.batch_tag))
        .unwrap();
    assert_eq!(removed, 6);
    assert_eq!(
        count(&db, "SELECT count(*) FROM notes WHERE deleted_at IS NULL"),
        1,
        "only the earlier note stays live"
    );
    let untouched = db
        .with_conn(|conn| note_repo::get(conn, &earlier.id))
        .unwrap();
    assert_eq!(untouched.deleted_at, None);
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM notes WHERE deleted_at IS NOT NULL"
        ),
        6,
        "the earlier note is never soft-deleted by the batch undo"
    );

    db.with_conn(|conn| note_repo::restore(conn, &ids[0]))
        .unwrap();
    let restored = db.with_conn(|conn| note_repo::get(conn, &ids[0])).unwrap();
    assert!(
        restored.title.starts_with("笔记 "),
        "the restored note is one of the batch: {}",
        restored.title
    );
    assert_eq!(
        count(&db, "SELECT count(*) FROM notes WHERE deleted_at IS NULL"),
        2,
        "the earlier note plus the restored one"
    );

    // Repeating the undo only affects what was restored, and never errors.
    assert_eq!(
        db.with_conn(|conn| note_repo::soft_delete_by_tag(conn, &report.batch_tag))
            .unwrap(),
        1
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM notes WHERE deleted_at IS NOT NULL"
        ),
        6,
        "the earlier note is never soft-deleted by the batch undo"
    );

    let _ = fs::remove_dir_all(&root);
    drop(db);
    let _ = fs::remove_file(path);
}

/// Parent-directory labels are opt-in, and frontmatter tags are reused.
#[test]
fn parent_dir_tag_option_adds_one_tag_per_directory() {
    let (db, path) = temp_db();
    let root = vault("parent", 4);
    let report = import(
        &db,
        &root,
        ImportOptions {
            batch_size: None,
            parent_dir_tag: true,
        },
        None,
    );
    assert_eq!(report.imported, 4);

    // `work` comes from every frontmatter block; the directory tags are opt-in
    // and cover only the notes in that directory.
    for (name, expected) in [("Projects", 2), ("Inbox", 2), ("work", 4)] {
        let tagged = db
            .with_conn(|conn| {
                Ok(conn.query_row(
                    "SELECT count(*) FROM note_tags nt JOIN tags t ON t.id = nt.tag_id WHERE t.name = ?1",
                    [name],
                    |row| row.get::<_, i64>(0),
                )?)
            })
            .unwrap();
        assert_eq!(tagged, expected, "tag {name}");
    }

    let _ = fs::remove_dir_all(&root);
    drop(db);
    let _ = fs::remove_file(path);
}

/// A file that cannot be decoded is reported and skipped; nothing partial is
/// written and the rest of the batch still lands.
#[test]
fn undecodable_file_is_reported_and_others_still_import() {
    let (db, path) = temp_db();
    let root = vault("decode", 3);
    fs::write(root.join("broken.md"), [0xFFu8, 0xFF, 0xFF, 0xFF]).unwrap();
    fs::write(root.join("gb.md"), [0xB1u8, 0xCA, 0xBC, 0xC7]).unwrap();

    let report = import(&db, &root, ImportOptions::default(), None);
    assert_eq!(report.imported, 4, "3 valid + 1 GB18030");
    assert_eq!(report.failed.len(), 1);
    assert!(report.failed[0].path.ends_with("broken.md"));
    assert!(!report.failed[0].reason.is_empty());
    assert_eq!(report.decoded_as_gb18030, 1);
    assert_eq!(count(&db, "SELECT count(*) FROM notes"), 4);

    let _ = fs::remove_dir_all(&root);
    drop(db);
    let _ = fs::remove_file(path);
}
