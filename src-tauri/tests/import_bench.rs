//! Import scale check (task 5.3). Ignored by default; run with
//!
//! ```sh
//! cargo test --release --test import_bench -- --ignored --nocapture
//! ```
//!
//! The import runs on the single connection, so the longest stretch between two
//! progress events is the worst-case wait an autosave request can see: parsing
//! happens before the transaction opens, and the transaction commits before the
//! event fires. That gap is therefore an *upper bound* on the lock hold, not a
//! measurement of the lock alone.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use macos_notes_lib::database::Database;
use macos_notes_lib::services::import_service::{self, ImportOptions};

fn scratch(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("import-bench-{name}-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn vault(files: usize) -> PathBuf {
    let root = scratch(&format!("vault-{files}"));
    fs::create_dir_all(root.join(".obsidian")).unwrap();
    for i in 0..files {
        let sub = root.join(format!("folder-{}", i % 20));
        fs::create_dir_all(&sub).unwrap();
        fs::write(
            sub.join(format!("note-{i:05}.md")),
            format!(
                "---\ntitle: 笔记 {i}\ntags: [bench]\n---\n\n# 笔记 {i}\n\n{} 迁移验证关键字 ![](img/{i}.png)\n",
                "正文内容。".repeat(20)
            ),
        )
        .unwrap();
    }
    root
}

fn run(files: usize, batch: usize) {
    let dir = scratch(&format!("db-{files}"));
    let db = Database::open(dir.join("notes.db")).unwrap();
    let root = vault(files);

    let mut batches = 0usize;
    let mut worst_gap = Duration::ZERO;
    let mut previous = Instant::now();
    let started = Instant::now();
    let cancel = AtomicBool::new(false);

    let report = import_service::run_import(
        &db,
        &cancel,
        &root.to_string_lossy(),
        ImportOptions {
            batch_size: Some(batch),
            ..ImportOptions::default()
        },
        &mut |_progress| {
            let now = Instant::now();
            let gap = now.duration_since(previous);
            previous = now;
            worst_gap = worst_gap.max(gap);
            batches += 1;
        },
    )
    .expect("import");
    let elapsed = started.elapsed();

    let size = fs::metadata(dir.join("notes.db")).unwrap().len();
    let wal = fs::metadata(dir.join("notes.db-wal"))
        .map(|meta| meta.len())
        .unwrap_or(0);
    println!(
        "{files:>5} files | batch {batch:>3} | imported {:>5} | batches {:>3} | {:>7.2}s | \
         db+wal {:>6.1} MB | worst batch gap {:>6.1} ms",
        report.imported,
        batches,
        elapsed.as_secs_f64(),
        (size + wal) as f64 / 1_048_576.0,
        worst_gap.as_secs_f64() * 1000.0,
    );
    assert_eq!(report.imported, files);
    assert_eq!(report.failed.len(), 0);

    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
#[ignore]
fn import_scale() {
    for (files, batch) in [(1000usize, 200usize), (5000, 200), (5000, 500)] {
        run(files, batch);
    }
}
