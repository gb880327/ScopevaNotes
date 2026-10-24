use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use macos_notes_lib::database::Database;
use macos_notes_lib::models::AppError;
use macos_notes_lib::services::library_service::{self, NoticeKind};

fn unique(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("macos-notes-{tag}-{nanos}"))
}

fn cleanup(paths: &[&Path]) {
    for path in paths {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.to_string_lossy()));
        }
    }
}

#[test]
fn missing_configured_library_falls_back_with_visible_notice() {
    let default = unique("fallback-default.db");
    let configured = unique("does-not-exist").join("notes.db");

    let (db, info) = library_service::resolve_from(
        Some(configured.to_string_lossy().into_owned()),
        &default,
    )
    .expect("fallback must succeed");

    assert_eq!(info.path, db.path().to_string_lossy());
    assert_eq!(info.path, default.to_string_lossy());
    assert!(info.is_default);
    let notice = info.notice.expect("fallback must be surfaced");
    assert_eq!(notice.kind, NoticeKind::Fallback);
    assert_eq!(notice.path.as_deref(), Some(configured.to_string_lossy().as_ref()));
    assert!(notice.message.contains("默认库"), "message: {}", notice.message);

    cleanup(&[&default]);
}

#[test]
fn corrupt_configured_library_falls_back_with_reason() {
    let default = unique("corrupt-default.db");
    let corrupt = unique("corrupt.db");
    let mut file = std::fs::File::create(&corrupt).unwrap();
    file.write_all(b"this is not a sqlite database, not even close").unwrap();
    drop(file);

    let (db, info) =
        library_service::resolve_from(Some(corrupt.to_string_lossy().into_owned()), &default)
            .expect("fallback must succeed");

    assert_eq!(db.path(), default);
    let notice = info.notice.expect("fallback must be surfaced");
    assert_eq!(notice.kind, NoticeKind::Fallback);
    assert!(
        notice.message.contains("无法打开"),
        "message: {}",
        notice.message
    );

    cleanup(&[&default, &corrupt]);
}

#[test]
fn healthy_configured_library_has_no_fallback_notice() {
    let default = unique("unused-default.db");
    let configured = unique("custom.db");
    Database::open(&configured).unwrap();

    let (db, info) =
        library_service::resolve_from(Some(configured.to_string_lossy().into_owned()), &default)
            .expect("configured library opens");

    assert_eq!(db.path(), configured);
    assert!(!info.is_default);
    assert_eq!(info.notice, None);

    cleanup(&[&configured]);
}

#[test]
fn cloud_sync_folders_are_detected_from_path_components() {
    let cases: &[(&str, Option<&str>)] = &[
        ("/Users/me/Dropbox/notes/notes.db", Some("Dropbox")),
        ("/Users/me/Nextcloud/Notes/notes.db", Some("Nextcloud")),
        (
            "/Users/me/Library/Mobile Documents/com~apple~CloudDocs/notes.db",
            Some("iCloud Drive"),
        ),
        (
            "/Users/me/Library/CloudStorage/GoogleDrive-me/My Drive/notes.db",
            Some("Google Drive"),
        ),
        ("/Users/me/OneDrive/notes.db", Some("OneDrive")),
        (
            "/Users/me/Library/CloudStorage/OneDrive-Acme/docs/notes.db",
            Some("OneDrive"),
        ),
        ("/Users/me/Documents/notes.db", None),
        // Only whole path components count: a file named like a provider is not a folder.
        (
            "/Users/me/Documents/Dropbox/../notes.db",
            Some("Dropbox"),
        ),
    ];
    for (raw, expected) in cases {
        assert_eq!(
            library_service::cloud_sync_provider(Path::new(raw)),
            *expected,
            "path: {raw}"
        );
    }
    assert_eq!(
        library_service::cloud_sync_provider(Path::new("/Users/me/Documents/dropbox-notes.db")),
        None
    );
}

#[test]
fn cloud_library_reports_advisory_notice() {
    let dir = unique("cloud");
    std::fs::create_dir_all(dir.join("Nextcloud")).unwrap();
    let path = dir.join("Nextcloud").join("notes.db");
    let db = Database::open(&path).unwrap();

    let info = library_service::info_for(&db, false).unwrap();
    let notice = info.notice.expect("cloud notice expected");
    assert_eq!(notice.kind, NoticeKind::CloudSync);
    assert!(notice.message.contains("Nextcloud"));

    cleanup(&[&path]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn locked_database_is_reported_as_busy() {
    let path = unique("locked.db");
    let db = Database::open_with_busy_timeout(&path, Duration::from_millis(150)).unwrap();

    // A second connection takes the write lock and never releases it while the
    // assertion runs, which is what a cloud-sync client or a second copy does.
    let holder = rusqlite::Connection::open(&path).unwrap();
    holder.execute_batch("BEGIN EXCLUSIVE").unwrap();

    let err = db
        .with_conn(|conn| {
            conn.execute("INSERT INTO notes (id, title, content, created_at, updated_at) \
                          VALUES ('x', 't', 'c', 0, 0)", [])
                .map_err(Into::into)
        })
        .expect_err("write must fail while locked");

    match err {
        AppError::Busy(message) => assert!(
            message.contains("locked") || message.contains("busy"),
            "message: {message}"
        ),
        other => panic!("expected Busy, got {other:?}"),
    }

    holder.execute_batch("ROLLBACK").unwrap();
    drop(holder);
    cleanup(&[&path]);
}
