//! Data-layer stress run for task 2.10: 1,000 random notes, CRUD timings and
//! cursor-pagination correctness.
//!
//! Run with:
//!   cargo test --release --test crud_bench -- --ignored --nocapture

use std::collections::HashSet;
use std::time::{Duration, Instant};

use macos_notes_lib::database::Database;
use macos_notes_lib::repositories::note_repo::{self, Cursor, ListQuery};
use macos_notes_lib::repositories::tag_repo;

const NOTES: usize = 1_000;

/// Deterministic LCG so a rerun produces the same corpus.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() >> 33) as usize % bound
    }
}

const CJK: &[&str] = &[
    "笔记", "搜索", "标签", "回滚", "索引", "数据库", "同步", "预览", "快捷键", "回收站",
];
const ASCII: &[&str] = &[
    "sqlite", "fts5", "async", "tauri", "sql", "index", "cursor", "markdown", "shiki", "wry",
];

fn body(rng: &mut Rng, words: usize) -> String {
    let mut out = String::with_capacity(words * 6);
    for _ in 0..words {
        if rng.below(2) == 0 {
            out.push_str(CJK[rng.below(CJK.len())]);
        } else {
            out.push_str(ASCII[rng.below(ASCII.len())]);
        }
        out.push(' ');
    }
    out
}

fn percentile(samples: &[Duration], p: f64) -> Duration {
    if samples.is_empty() {
        return Duration::ZERO;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[index]
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn report(label: &str, samples: &[Duration]) {
    println!(
        "{label:<28} n={:<5} p50={:>7.2}ms p95={:>7.2}ms max={:>7.2}ms",
        samples.len(),
        ms(percentile(samples, 0.50)),
        ms(percentile(samples, 0.95)),
        samples.iter().copied().max().unwrap_or_default().as_secs_f64() * 1000.0,
    );
}

#[test]
#[ignore = "stress run; use --release --ignored --nocapture"]
fn thousand_notes_crud_and_pagination() {
    let dir = std::env::temp_dir().join(format!(
        "macos-notes-crud-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let db = Database::open(dir.join("notes.db")).unwrap();

    let tags: Vec<String> = db
        .with_conn(|conn| {
            (0..20)
                .map(|index| {
                    tag_repo::create(conn, &format!("tag-{index}"), None).map(|tag| tag.id)
                })
                .collect::<Result<_, _>>()
        })
        .unwrap();

    let mut rng = Rng(42);
    let mut created = Vec::with_capacity(NOTES);
    let mut create_samples = Vec::with_capacity(NOTES);
    for index in 0..NOTES {
        let title = format!("笔记 {index} {}", CJK[rng.below(CJK.len())]);
        let words = 40 + rng.below(80);
        let content = body(&mut rng, words);
        let mut ids = Vec::new();
        for _ in 0..rng.below(3) {
            ids.push(tags[rng.below(tags.len())].clone());
        }
        let started = Instant::now();
        let note = db
            .with_conn(|conn| note_repo::create(conn, &title, &content, &ids))
            .unwrap();
        create_samples.push(started.elapsed());
        created.push(note);
    }
    report("create", &create_samples);

    // Reads
    let mut get_samples = Vec::with_capacity(200);
    for _ in 0..200 {
        let note = &created[rng.below(created.len())];
        let started = Instant::now();
        let fetched = db.with_conn(|conn| note_repo::get(conn, &note.id)).unwrap();
        get_samples.push(started.elapsed());
        assert_eq!(fetched.title, note.title);
    }
    report("get", &get_samples);

    // Updates
    let mut update_samples = Vec::with_capacity(200);
    for _ in 0..200 {
        let note = &created[rng.below(created.len())];
        let words = 40 + rng.below(80);
        let content = body(&mut rng, words);
        let started = Instant::now();
        db.with_conn(|conn| {
            note_repo::update(conn, &note.id, &note.title, &content, &[], None)
        })
        .unwrap();
        update_samples.push(started.elapsed());
    }
    report("update", &update_samples);

    // Cursor pagination over the whole library: no duplicates, no gaps, stable order.
    let paged = db
        .with_conn(|conn| {
            let mut cursor: Option<String> = None;
            let mut seen: Vec<(i64, String)> = Vec::new();
            let mut pages = 0;
            let mut samples = Vec::new();
            loop {
                let started = Instant::now();
                let page = note_repo::list(
                    conn,
                    &ListQuery {
                        cursor: cursor.as_deref().map(Cursor::decode).transpose()?,
                        ..Default::default()
                    },
                )?;
                samples.push(started.elapsed());
                pages += 1;
                assert!(page.items.len() <= 50, "page exceeded the default limit");
                for item in &page.items {
                    seen.push((item.updated_at, item.id.clone()));
                }
                match page.next_cursor {
                    Some(next) => cursor = Some(next),
                    None => break,
                }
                assert!(pages <= 100, "pagination did not terminate");
            }
            Ok::<_, macos_notes_lib::models::AppError>((seen, pages, samples))
        })
        .unwrap();
    let (seen, pages, page_samples) = paged;
    report("list page (50)", &page_samples);
    println!("pages={pages} rows={}", seen.len());

    assert_eq!(seen.len(), NOTES, "pagination must return every note exactly once");
    let unique: HashSet<&str> = seen.iter().map(|(_, id)| id.as_str()).collect();
    assert_eq!(unique.len(), NOTES, "pagination returned a duplicate note");
    let mut sorted = seen.clone();
    sorted.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    assert_eq!(seen, sorted, "rows must follow updated_at DESC, id DESC");

    // Search
    let mut search_samples = Vec::with_capacity(40);
    let terms = ["笔记 3", "sqlite", "索引", "markdown", "标签 1", "fts5", "数据库", "cursor"];
    for _ in 0..40 {
        let term = terms[rng.below(terms.len())];
        let started = Instant::now();
        let page = db
            .with_conn(|conn| {
                note_repo::list(
                    conn,
                    &ListQuery {
                        search: Some(term.to_string()),
                        ..Default::default()
                    },
                )
            })
            .unwrap();
        search_samples.push(started.elapsed());
        assert!(page.items.len() <= 50);
    }
    report("search (1k notes)", &search_samples);

    // Deletes + trash listing + restores (unique targets: deleting twice is a
    // NotFound on purpose, see `soft_delete`'s `deleted_at IS NULL` guard).
    let mut deleted_targets = Vec::with_capacity(100);
    let mut used = HashSet::new();
    while deleted_targets.len() < 100 {
        let index = rng.below(created.len());
        if used.insert(index) {
            deleted_targets.push(created[index].id.clone());
        }
    }
    let started = Instant::now();
    for id in &deleted_targets {
        db.with_conn(|conn| note_repo::soft_delete(conn, id)).unwrap();
    }
    println!("soft_delete x100            {:>7.2}ms", ms(started.elapsed()));

    let deleted = db
        .with_conn(|conn| note_repo::list_deleted(conn, Some(200)))
        .unwrap();
    assert_eq!(deleted.len(), 100, "trash listing must show the deleted notes");

    let started = Instant::now();
    for id in deleted_targets.iter().take(50) {
        db.with_conn(|conn| note_repo::restore(conn, id)).unwrap();
    }
    println!("restore x50                 {:>7.2}ms", ms(started.elapsed()));

    let live = db
        .with_conn(|conn| note_repo::list(conn, &ListQuery::default()))
        .unwrap();
    println!("first page after churn      {} items", live.items.len());
    assert_eq!(
        db.with_conn(|conn| note_repo::list_deleted(conn, Some(200)))
            .unwrap()
            .len(),
        50
    );

    let size = ["", "-wal", "-shm"]
        .iter()
        .filter_map(|suffix| {
            std::fs::metadata(format!("{}{suffix}", dir.join("notes.db").to_string_lossy())).ok()
        })
        .map(|meta| meta.len())
        .sum::<u64>();
    println!("library size                {:.1} MB", size as f64 / 1_048_576.0);

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
