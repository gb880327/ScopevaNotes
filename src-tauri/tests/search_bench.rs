//! Search latency benchmark for the 10,000-note target in
//! `specs/search/spec.md` (P95 < 100ms for the first page).
//!
//! Ignored by default so `cargo test` stays fast:
//!
//! ```sh
//! cargo test --release --test search_bench -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use macos_notes_lib::database::{search_index, Database};
use macos_notes_lib::repositories::note_repo::{self, ListQuery};

const NOTES: usize = 10_000;
const QUERIES: usize = 40;
const P95_BUDGET_MS: f64 = 100.0;

const EN: [&str; 12] = [
    "sqlite", "async", "writer", "parser", "index", "query", "cache", "journal", "thread",
    "mutex", "cursor", "transaction",
];
const ZH: [&str; 10] = [
    "笔记", "搜索", "索引", "事务", "性能", "标签", "分页", "迁移", "同步", "备份",
];

fn temp_db() -> (Database, PathBuf) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("macos-notes-bench-{unique}.db"));
    let db = Database::open(&path).expect("open bench database");
    (db, path)
}

/// Deterministic pseudo-random so runs are comparable.
fn pick(seed: usize, len: usize, modulo: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(len);
    let mut x = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    for _ in 0..len {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        out.push((x >> 33) % modulo);
    }
    out
}

fn seed_notes(db: &Database) {
    // Direct SQL, one transaction: seeding through the repository would add
    // 10k commit cycles and say nothing about query latency.
    db.with_conn(|conn| {
        let tx = conn.transaction()?;
        for i in 0..NOTES {
            let base = (i * 7919) % 1_000_003;
            let en = pick(base + 1, 12, EN.len());
            let zh = pick(base + 2, 8, ZH.len());
            let words: Vec<&str> = en.iter().map(|i| EN[*i]).collect();
            let chinese: Vec<&str> = zh.iter().map(|i| ZH[*i]).collect();
            let body = format!(
                "备注 {i}：{zh} 的 {en} 记录。\n\nThe {a} {b} and {c} {d} live here.\n\n{pad}",
                zh = chinese.join("与"),
                en = words.join("、"),
                a = words[0],
                b = words[1],
                c = words[2],
                d = words[3],
                pad = "filler text for a realistic note size. ".repeat(6),
            );
            let title = format!("{i} {}", words[5]);
            let id = format!("note-{i:06}");
            let stamp = 1_760_000_000_000i64 + i as i64;
            tx.execute(
                "INSERT INTO notes (id, title, content, is_favorite, deleted_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 0, NULL, ?4, ?4)",
                (&id, &title, &body, stamp),
            )?;
        }
        tx.commit()?;
        Ok(())
    })
    .expect("seed notes");

    // Real indexing path, so the projection is in the CJK-aware format.
    db.with_conn(search_index::reindex_all).expect("reindex");
}

#[test]
#[ignore = "benchmark: run explicitly with --release --ignored"]
fn search_first_page_latency_at_10k_notes() {
    let (db, path) = temp_db();
    let seed_start = Instant::now();
    seed_notes(&db);
    println!(
        "seeded {NOTES} notes in {:?}",
        seed_start.elapsed()
    );

    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    println!("library size: {:.1} MB", size as f64 / 1_048_576.0);

    let mut samples: Vec<f64> = Vec::with_capacity(QUERIES + 2);
    // The two spec corner cases: a rare word (nothing matches) and a word that
    // only exists as the middle of a longer token.
    let mut terms: Vec<&str> = Vec::new();
    for i in 0..QUERIES {
        // Alternate English / Chinese / mixed.
        terms.push(match i % 3 {
            0 => EN[i % EN.len()],
            1 => ZH[i % ZH.len()],
            // CJK substring in the middle of a run, plus a Latin word.
            _ => "索引 sqlite",
        });
    }
    terms.push("nosuchkeywordanywhere");
    terms.push("索引 sqlite");

    for (i, term) in terms.iter().copied().enumerate() {
        let query = ListQuery {
            search: Some(term.to_string()),
            ..Default::default()
        };
        // Warm-up once so the first sample is not a cold page-cache outlier.
        db.with_conn(|conn| note_repo::list(conn, &query)).unwrap();
        let start = Instant::now();
        let page = db
            .with_conn(|conn| note_repo::list(conn, &query))
            .expect("search");
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        samples.push(elapsed);
        if i < 5 {
            println!("  {term:>16} -> {:>4} hits in {elapsed:.1} ms", page.items.len());
        }
    }

    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = samples[samples.len() / 2];
    let p95 = samples[(samples.len() * 95 / 100).min(samples.len() - 1)];
    let max = *samples.last().unwrap();
    let len = samples.len();
    println!("p50 {p50:.1} ms · p95 {p95:.1} ms · max {max:.1} ms over {len} queries");

    drop(db);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{}-wal", path.to_string_lossy()));
    let _ = std::fs::remove_file(format!("{}-shm", path.to_string_lossy()));

    assert!(
        p95 < P95_BUDGET_MS,
        "p95 {p95:.1} ms exceeds the {P95_BUDGET_MS} ms budget"
    );
}
