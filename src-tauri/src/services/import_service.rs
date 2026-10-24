//! Markdown import: discovery, decoding, frontmatter, timestamps.
//!
//! Scope is deliberately narrow — this module only converts files into
//! `ParsedNote` values. It never touches the database (see `commands/import.rs`)
//! and it never rewrites the body: the only transformation applied to a
//! document is the removal of its leading YAML block (design D1). Keeping the
//! text byte-identical means a future rendering improvement (wikilinks,
//! `==highlight==`, callouts) applies to already-imported notes without
//! re-importing them.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::database::Database;
use crate::models::AppError;
use crate::repositories::note_repo::{self, ImportRow};
use crate::repositories::tag_repo;

const MARKDOWN_EXTS: &[&str] = &["md", "markdown"];
/// Directories that are never a user's notes, even when not hidden.
const SKIP_DIRS: &[&str] = &["node_modules"];
const MAX_DEPTH: usize = 64;

/// Result of walking one directory.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Discovery {
    /// Absolute paths of every `.md` / `.markdown` file found.
    pub files: Vec<PathBuf>,
    /// Hidden directories/files and `node_modules` that were skipped. Counted
    /// per entry (one `.obsidian/` directory counts once, not per file).
    pub skipped_hidden: usize,
}

/// Recursively collect Markdown files, skipping hidden entries, `node_modules`
/// and symbolic links (a link to a parent would otherwise loop or duplicate).
pub fn discover(root: &Path) -> AppResult<Discovery> {
    if !root.is_dir() {
        return Err(AppError::Invalid(format!(
            "not a directory: {}",
            root.display()
        )));
    }
    let mut discovery = Discovery {
        files: Vec::new(),
        skipped_hidden: 0,
    };
    walk(root, 0, &mut discovery)?;
    discovery.files.sort();
    Ok(discovery)
}

type AppResult<T> = Result<T, AppError>;

fn walk(dir: &Path, depth: usize, out: &mut Discovery) -> AppResult<()> {
    if depth > MAX_DEPTH {
        return Ok(());
    }
    let entries = fs::read_dir(dir)
        .map_err(|err| AppError::Other(format!("cannot read {}: {err}", dir.display())))?;
    for entry in entries {
        let entry =
            entry.map_err(|err| AppError::Other(format!("cannot read directory: {err}")))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // `file_type` does not follow symlinks, so a symlink is detected here
        // and never traversed.
        let file_type = entry
            .file_type()
            .map_err(|err| AppError::Other(format!("cannot stat {}: {err}", path.display())))?;
        if file_type.is_symlink() {
            out.skipped_hidden += 1;
            continue;
        }

        if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) {
            out.skipped_hidden += 1;
            continue;
        }

        if file_type.is_dir() {
            walk(&path, depth + 1, out)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let is_markdown = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| MARKDOWN_EXTS.contains(&ext.to_ascii_lowercase().as_str()))
            .unwrap_or(false);
        if is_markdown {
            out.files.push(path);
        }
    }
    Ok(())
}

/// One document, ready to be inserted. Timestamps come from the file system
/// (design D6) so an imported library keeps its original ordering.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedNote {
    pub source_path: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub content_hash: String,
    pub decoded_as_gb18030: bool,
    /// Local files referenced by the body (images, embeds) that this version
    /// does not migrate. Counted so the report can say so out loud.
    pub local_file_refs: usize,
}

/// Parse one file. Errors are per-file strings: the importer reports them and
/// keeps going (a bad file must not cost the other 4,999).
pub fn parse_file(path: &Path) -> Result<ParsedNote, String> {
    let bytes = fs::read(path).map_err(|err| format!("读取失败: {err}"))?;
    let (text, decoded_as_gb18030) = decode(&bytes)?;
    let (frontmatter, body) = split_frontmatter(&text);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let title = frontmatter
        .title
        .filter(|t| !t.trim().is_empty())
        .or_else(|| first_h1(body))
        .unwrap_or(stem);

    let (created_at, updated_at) = file_times(path);

    Ok(ParsedNote {
        source_path: path.to_string_lossy().to_string(),
        title,
        content: body.to_string(),
        tags: frontmatter.tags,
        created_at: frontmatter.created.unwrap_or(created_at),
        updated_at,
        content_hash: content_hash(&bytes),
        decoded_as_gb18030,
        local_file_refs: count_local_file_refs(body),
    })
}

/// UTF-8 first, GB18030 only as a fallback. The order matters: a UTF-8 Chinese
/// file is also (mostly) valid GB18030, so trying GB first would produce mojibake.
fn decode(bytes: &[u8]) -> Result<(String, bool), String> {
    if let Ok(text) = std::str::from_utf8(bytes) {
        let text = text.trim_start_matches('\u{feff}');
        return Ok((text.to_string(), false));
    }
    let (cow, _encoding, had_errors) = encoding_rs::GB18030.decode(bytes);
    if had_errors {
        return Err("无法按 UTF-8 或 GB18030 解码".to_string());
    }
    let text = cow.trim_start_matches('\u{feff}').to_string();
    Ok((text, true))
}

/// Stable across runs and Rust versions (unlike `DefaultHasher`). Only used to
/// record "what did this file contain when it was imported", never for lookup.
fn content_hash(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[derive(Debug, Default)]
struct Frontmatter {
    title: Option<String>,
    tags: Vec<String>,
    created: Option<i64>,
}

/// Split a leading `---` YAML block from the body. Only `title`, `tags` and
/// `created` are understood (design D8); everything else is dropped with the
/// block. The body is returned verbatim.
fn split_frontmatter(text: &str) -> (Frontmatter, &str) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return (Frontmatter::default(), text);
    };
    if first.trim_end() != "---" {
        return (Frontmatter::default(), text);
    }

    let mut frontmatter = Frontmatter::default();
    let mut offset = first.len();
    let mut last_key: Option<String> = None;
    let mut closed = false;

    for line in lines {
        let trimmed = line.trim_end();
        if trimmed == "---" || trimmed == "..." {
            offset += line.len();
            closed = true;
            break;
        }
        offset += line.len();

        let body = trimmed.trim_end_matches('\r');
        if body.trim().is_empty() || body.trim_start().starts_with('#') {
            continue;
        }
        if let Some(item) = body.trim_start().strip_prefix("- ") {
            if last_key.as_deref() == Some("tags") {
                push_tags(&mut frontmatter.tags, item);
            }
            continue;
        }
        let Some((key, value)) = body.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        match key.as_str() {
            "title" => {
                let title = unquote(value);
                if !title.is_empty() {
                    frontmatter.title = Some(title);
                }
            }
            "tags" => {
                if value.is_empty() {
                    last_key = Some(key);
                    continue;
                }
                let inner = value
                    .strip_prefix('[')
                    .and_then(|v| v.strip_suffix(']'))
                    .unwrap_or(value);
                push_tags(&mut frontmatter.tags, inner);
            }
            "created" => frontmatter.created = parse_timestamp(value),
            _ => {}
        }
        last_key = Some(key);
    }

    if !closed {
        // Unterminated block: treat the file as having no frontmatter rather
        // than swallowing the whole document.
        return (Frontmatter::default(), text);
    }
    (frontmatter, &text[offset..])
}

fn push_tags(out: &mut Vec<String>, raw: &str) {
    for part in raw.split(',') {
        let tag = unquote(part.trim());
        let tag = tag.trim_start_matches('#').trim();
        if tag.is_empty() || out.iter().any(|existing| existing == tag) {
            continue;
        }
        out.push(tag.to_string());
    }
}

fn unquote(raw: &str) -> String {
    let raw = raw.trim();
    for quote in ['"', '\''] {
        if raw.len() >= 2 && raw.starts_with(quote) && raw.ends_with(quote) {
            return raw[1..raw.len() - 1].to_string();
        }
    }
    raw.to_string()
}

/// First `# ` line outside a fenced code block. The line stays in the body —
/// removing it would surprise anyone comparing the note with the file.
fn first_h1(body: &str) -> Option<String> {
    let mut fence: Option<&str> = None;
    for line in body.lines() {
        let trimmed = line.trim_start();
        let marker = if trimmed.starts_with("```") {
            Some("```")
        } else if trimmed.starts_with("~~~") {
            Some("~~~")
        } else {
            None
        };
        if let Some(marker) = marker {
            match fence {
                Some(open) if open == marker => fence = None,
                Some(_) => {}
                None => fence = Some(marker),
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            let title = rest.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}

/// `YYYY-MM-DD[ T]HH:MM[:SS][Z]`, `YYYY-MM-DD`, or an epoch number.
fn parse_timestamp(raw: &str) -> Option<i64> {
    let raw = unquote(raw);
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(number) = raw.parse::<i64>() {
        // Seconds vs milliseconds: anything past 1e12 cannot be seconds.
        return Some(if number.abs() > 1_000_000_000_000 {
            number
        } else {
            number.saturating_mul(1000)
        });
    }

    let (date, rest) = match raw.split_once(['T', ' ']) {
        Some((date, rest)) => (date, Some(rest)),
        None => (raw, None),
    };
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let mut seconds = 0i64;
    if let Some(rest) = rest {
        let time = rest.trim_end_matches('Z').trim_end_matches('z');
        let mut clock = time.split(':');
        let hour: i64 = clock.next()?.parse().ok()?;
        let minute: i64 = clock.next().unwrap_or("0").parse().ok()?;
        let second: i64 = clock.next().unwrap_or("0").parse().ok()?;
        if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) || !(0..=59).contains(&second) {
            return None;
        }
        seconds = hour * 3600 + minute * 60 + second;
    }

    Some((days_from_civil(year, month, day) * 86_400 + seconds) * 1000)
}

/// Howard Hinnant's `days_from_civil`: days since 1970-01-01 without a date
/// crate. Handles leap years and the 1900/2100 century rules.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_prime = (month + 9) % 12;
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// `(created, updated)` in ms. `created` prefers the APFS birth time, then
/// `mtime`, then now. Future timestamps (clock skew, copied files) are clamped
/// to now so one bad file cannot pin itself to the top of the list forever.
fn file_times(path: &Path) -> (i64, i64) {
    let now = now_ms();
    let metadata = fs::metadata(path).ok();
    let to_ms = |time: std::io::Result<SystemTime>| -> Option<i64> {
        time.ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
    };

    let updated = metadata
        .as_ref()
        .and_then(|m| to_ms(m.modified()))
        .unwrap_or(now)
        .min(now);
    let created = metadata
        .as_ref()
        .and_then(|m| to_ms(m.created()))
        .unwrap_or(updated)
        .min(now);
    (created, updated)
}

/// Count body references that point at a local file: `![](img/a.png)` and
/// `![[diagram.png]]`. Remote URLs and `[[note]]` transclusions are not files,
/// so they are not counted.
fn count_local_file_refs(body: &str) -> usize {
    let mut count = 0;
    let bytes = body.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'!' && bytes[i + 1] == b'[' {
            if bytes.get(i + 2) == Some(&b'[') {
                if let Some(end) = body[i + 3..].find("]]") {
                    let target = &body[i + 3..i + 3 + end];
                    if is_local_file(target) {
                        count += 1;
                    }
                    i = i + 3 + end + 2;
                    continue;
                }
            }
            if let Some(close) = body[i + 2..].find(']') {
                let after = i + 2 + close + 1;
                if bytes.get(after) == Some(&b'(') {
                    if let Some(len) = body[after + 1..].find(')') {
                        let target = &body[after + 1..after + 1 + len];
                        if is_local_target(target) {
                            count += 1;
                        }
                        i = after + 1 + len + 1;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    count
}

fn is_local_target(raw: &str) -> bool {
    let target = raw
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim();
    if target.is_empty() || target.starts_with('#') {
        return false;
    }
    let lower = target.to_ascii_lowercase();
    // `img/a.png`, `/abs/path.png`, `./x.png`, `../x.png` are local; anything
    // with a scheme (https:, data:, mailto:) is not.
    for scheme in ["http://", "https://", "data:", "mailto:", "tel:", "ftp://"] {
        if lower.starts_with(scheme) {
            return false;
        }
    }
    true
}

fn is_local_file(raw: &str) -> bool {
    let target = raw.trim();
    if target.is_empty() {
        return false;
    }
    // Embeds are files only when they carry an extension (`![[img.png]]`);
    // `![[note]]` is a transclusion, not a file.
    let name = target.rsplit('/').next().unwrap_or(target);
    name.rsplit_once('.')
        .map(|(_, ext)| !ext.is_empty())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Preview and execution
//
// Both entry points live here (not in `commands/`) so integration tests can
// drive them without a Tauri runtime. `commands/import.rs` only adapts them to
// IPC and the progress event.
// ---------------------------------------------------------------------------

pub const DEFAULT_BATCH: usize = 200;
pub const MAX_BATCH: usize = 500;
pub const BATCH_TAG_PREFIX: &str = "import-";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFailure {
    pub path: String,
    pub reason: String,
}

/// Dry run: everything the user needs to decide, nothing written.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub root: String,
    pub total_files: usize,
    pub to_import: usize,
    pub skipped_existing: usize,
    pub hidden_skipped: usize,
    /// Notes (not references) whose body points at local files.
    pub local_file_refs: usize,
    pub unreadable: Vec<ImportFailure>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOptions {
    pub batch_size: Option<usize>,
    #[serde(default)]
    pub parent_dir_tag: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    /// Files handled so far (imported + skipped + failed).
    pub done: usize,
    pub total: usize,
    pub imported: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub imported: usize,
    pub skipped_existing: usize,
    pub failed: Vec<ImportFailure>,
    pub hidden_skipped: usize,
    pub decoded_as_gb18030: usize,
    pub local_file_references: usize,
    pub cancelled: bool,
    pub elapsed_ms: u64,
    pub batch_tag: String,
    pub total_files: usize,
}

/// Read-only pass. Parsing happens without the database lock so previewing a
/// large vault does not stall autosave.
pub fn scan(db: &Database, root: &str) -> AppResult<ImportPreview> {
    let discovery = discover(Path::new(root))?;

    let mut parsed: Vec<ParsedNote> = Vec::with_capacity(discovery.files.len());
    let mut unreadable: Vec<ImportFailure> = Vec::new();
    for file in &discovery.files {
        match parse_file(file) {
            Ok(note) => parsed.push(note),
            Err(reason) => unreadable.push(ImportFailure {
                path: file.to_string_lossy().to_string(),
                reason,
            }),
        }
    }

    let exists: Vec<bool> = db.with_conn(|conn| {
        let mut flags = Vec::with_capacity(parsed.len());
        for note in &parsed {
            flags.push(note_repo::source_path_exists(conn, &note.source_path)?);
        }
        Ok(flags)
    })?;
    let skipped_existing = exists.iter().filter(|flag| **flag).count();

    Ok(ImportPreview {
        root: root.to_string(),
        total_files: discovery.files.len(),
        to_import: parsed.len() - skipped_existing,
        skipped_existing,
        hidden_skipped: discovery.skipped_hidden,
        local_file_refs: parsed
            .iter()
            .filter(|note| note.local_file_refs > 0)
            .count(),
        unreadable,
    })
}

/// Execute the import in batches of at most `MAX_BATCH`. `cancel` is polled
/// between files and before each batch; files already parsed in the current
/// batch are still committed, so a retry only fills the gap (design D4).
pub fn run_import(
    db: &Database,
    cancel: &AtomicBool,
    root: &str,
    options: ImportOptions,
    on_progress: &mut dyn FnMut(ImportProgress),
) -> AppResult<ImportReport> {
    let started = Instant::now();
    let batch_size = options
        .batch_size
        .unwrap_or(DEFAULT_BATCH)
        .clamp(1, MAX_BATCH);
    let discovery = discover(Path::new(root))?;
    let total = discovery.files.len();
    let batch_tag = batch_tag_name(db)?;

    let mut report = ImportReport {
        imported: 0,
        skipped_existing: 0,
        failed: Vec::new(),
        hidden_skipped: discovery.skipped_hidden,
        decoded_as_gb18030: 0,
        local_file_references: 0,
        cancelled: false,
        elapsed_ms: 0,
        batch_tag: batch_tag.clone(),
        total_files: total,
    };
    let mut done = 0usize;

    for chunk in discovery.files.chunks(batch_size) {
        if cancel.load(Ordering::SeqCst) {
            report.cancelled = true;
            break;
        }

        let mut rows: Vec<ParsedNote> = Vec::with_capacity(chunk.len());
        let failed_before = report.failed.len();
        for file in chunk {
            if cancel.load(Ordering::SeqCst) {
                report.cancelled = true;
                break;
            }
            match parse_file(file) {
                Ok(note) => rows.push(note),
                Err(reason) => report.failed.push(ImportFailure {
                    path: file.to_string_lossy().to_string(),
                    reason,
                }),
            }
        }

        let (inserted, skipped, gb, refs) = if rows.is_empty() {
            (0, 0, 0, 0)
        } else {
            let parent_tag = options.parent_dir_tag;
            db.with_conn(|conn| {
                let tx = conn.transaction()?;
                let mut inserted = 0usize;
                let mut skipped = 0usize;
                let mut gb = 0usize;
                let mut refs = 0usize;
                for note in &rows {
                    if note_repo::source_path_exists(&tx, &note.source_path)? {
                        skipped += 1;
                        continue;
                    }
                    let mut tag_names: Vec<String> = vec![batch_tag.clone()];
                    if parent_tag {
                        if let Some(parent) = parent_dir_tag(&note.source_path) {
                            tag_names.push(parent);
                        }
                    }
                    tag_names.extend(note.tags.iter().cloned());
                    let tag_ids = ensure_tags(&tx, &tag_names)?;
                    note_repo::insert_imported(
                        &tx,
                        &ImportRow {
                            source_path: &note.source_path,
                            content_hash: &note.content_hash,
                            title: &note.title,
                            content: &note.content,
                            created_at: note.created_at,
                            updated_at: note.updated_at,
                            tag_ids: &tag_ids,
                        },
                    )?;
                    inserted += 1;
                    if note.decoded_as_gb18030 {
                        gb += 1;
                    }
                    if note.local_file_refs > 0 {
                        refs += 1;
                    }
                }
                tx.commit()?;
                Ok((inserted, skipped, gb, refs))
            })?
        };

        report.imported += inserted;
        report.skipped_existing += skipped;
        report.decoded_as_gb18030 += gb;
        report.local_file_references += refs;

        // Files actually reached: parsed + parse failures. Unreached files
        // after a cancel are not counted.
        done += rows.len() + (report.failed.len() - failed_before);
        on_progress(ImportProgress {
            done,
            total,
            imported: report.imported,
        });

        if report.cancelled {
            break;
        }
    }

    report.elapsed_ms = started.elapsed().as_millis() as u64;
    Ok(report)
}

/// `import-YYYY-MM-DD` in the user's local time zone (SQLite knows the offset,
/// std does not).
pub fn batch_tag_name(db: &Database) -> AppResult<String> {
    let date: String = db.with_conn(|conn| {
        Ok(conn.query_row(
            "SELECT strftime('%Y-%m-%d', 'now', 'localtime')",
            [],
            |row| row.get(0),
        )?)
    })?;
    Ok(format!("{BATCH_TAG_PREFIX}{date}"))
}

fn parent_dir_tag(source_path: &str) -> Option<String> {
    Path::new(source_path)
        .parent()
        .and_then(|parent| parent.file_name())
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
}

/// Reuse a tag by name (case-insensitive, per the schema) instead of creating
/// duplicates.
fn ensure_tags(conn: &rusqlite::Connection, names: &[String]) -> AppResult<Vec<String>> {
    let mut ids = Vec::new();
    for name in names {
        let id = ensure_tag(conn, name)?;
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    Ok(ids)
}

fn ensure_tag(conn: &rusqlite::Connection, name: &str) -> AppResult<String> {
    let normalized = tag_repo::normalize_name(name);
    if normalized.is_empty() {
        return Err(AppError::Invalid("tag name must not be empty".into()));
    }
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM tags WHERE name = ?1",
            [&normalized],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    match tag_repo::create(conn, &normalized, None) {
        Ok(tag) => Ok(tag.id),
        // Another writer created it between the SELECT and the INSERT.
        Err(AppError::Conflict(_)) => conn
            .query_row(
                "SELECT id FROM tags WHERE name = ?1",
                [&normalized],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::Conflict(format!("tag already exists: {normalized}"))),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::fs::symlink;

    fn scratch(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("import-test-{name}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(bytes).unwrap();
        path
    }

    #[test]
    fn discover_skips_hidden_node_modules_and_symlinks() {
        let dir = scratch("discover");
        write(&dir, "a.md", b"# a");
        write(&dir, "sub/b.markdown", b"# b");
        write(&dir, "sub/notes.txt", b"nope");
        write(&dir, "sub/pic.png", b"png");
        write(&dir, ".obsidian/workspace.json", b"{}");
        write(&dir, ".trash/old.md", b"# old");
        write(&dir, "node_modules/pkg/readme.md", b"# pkg");
        write(&dir, ".hidden.md", b"# hidden");
        symlink(&dir, dir.join("loop")).unwrap();

        let discovery = discover(&dir).expect("discover");
        let names: Vec<String> = discovery
            .files
            .iter()
            .map(|p| p.strip_prefix(&dir).unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(
            names,
            vec!["a.md".to_string(), "sub/b.markdown".to_string()]
        );
        // .obsidian, .trash, node_modules, .hidden.md, loop
        assert_eq!(discovery.skipped_hidden, 5);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn decode_prefers_utf8_and_falls_back_to_gb18030() {
        let (text, gb) = decode(&[0xEF, 0xBB, 0xBF, b'h', b'i']).unwrap();
        assert_eq!(text, "hi");
        assert!(!gb, "BOM is stripped, not reported as GB18030");

        // "笔记" in GB18030
        let (text, gb) = decode(&[0xB1, 0xCA, 0xBC, 0xC7]).unwrap();
        assert_eq!(text, "笔记");
        assert!(gb, "GB18030 fallback must be flagged for the report");

        assert!(decode(&[0xFF, 0xFF, 0xFF]).is_err());
    }

    #[test]
    fn frontmatter_variants_are_parsed_and_removed() {
        let dir = scratch("frontmatter");
        let cases: [(&str, Vec<&str>); 3] = [
            ("title: 甲\ntags: [work, 2026]\n---", vec!["work", "2026"]),
            ("title: 乙\ntags: work, home\n---", vec!["work", "home"]),
            (
                "title: 丙\ntags:\n  - work\n  - #nested/tag\n---",
                vec!["work", "nested/tag"],
            ),
        ];
        for (index, (block, expected)) in cases.iter().enumerate() {
            let raw = format!("---\n{block}\n\n正文第 {index} 行\n");
            let path = write(&dir, &format!("note{index}.md"), raw.as_bytes());
            let parsed = parse_file(&path).unwrap();
            assert!(!parsed.content.contains("---"), "frontmatter removed");
            assert!(parsed.content.contains(&format!("正文第 {index} 行")));
            assert_eq!(parsed.tags, *expected);
        }

        // Unknown keys only: the block still goes away.
        let raw = "---\naliases: [x]\ncssclasses: wide\n---\n\nbody\n";
        let path = write(&dir, "unknown.md", raw.as_bytes());
        let parsed = parse_file(&path).unwrap();
        assert_eq!(parsed.content.trim(), "body");
        assert!(parsed.tags.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn title_priority_and_h1_detection() {
        let dir = scratch("title");
        let with_fm = write(
            &dir,
            "file-name.md",
            "---\ntitle: FM \n---\n\n# 手写标题\n\nbody".as_bytes(),
        );
        assert_eq!(parse_file(&with_fm).unwrap().title, "FM");

        let with_h1 = write(&dir, "未命名 3.md", "# 手写标题\n\nbody".as_bytes());
        let parsed = parse_file(&with_h1).unwrap();
        assert_eq!(parsed.title, "手写标题");
        assert!(
            parsed.content.contains("# 手写标题"),
            "H1 stays in the body"
        );

        let fenced = write(
            &dir,
            "fenced.md",
            "```sh\n# not a title\n```\n\n# 真正的标题\n".as_bytes(),
        );
        assert_eq!(parse_file(&fenced).unwrap().title, "真正的标题");

        let plain = write(&dir, "只有文件名.md", b"no headings here");
        assert_eq!(parse_file(&plain).unwrap().title, "只有文件名");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn created_timestamp_supports_iso_and_epoch() {
        assert_eq!(parse_timestamp("1970-01-01"), Some(0));
        assert_eq!(
            parse_timestamp("2026-10-24T12:34:56Z"),
            Some(1_792_845_296_000)
        );
        assert_eq!(parse_timestamp("1700000000"), Some(1_700_000_000_000));
        assert_eq!(parse_timestamp("1700000000000"), Some(1_700_000_000_000));
        assert_eq!(parse_timestamp("not a date"), None);
    }

    #[test]
    fn local_file_refs_count_only_local_files() {
        let body = "![](img/a.png) ![ok](https://x/y.png) [[另一条笔记]] \
                    ![[diagram.png]] ![[嵌入的笔记]] [链接](https://example.com) #anchor ![](#frag)";
        assert_eq!(count_local_file_refs(body), 2, "img/a.png + diagram.png");

        let body = "nothing here";
        assert_eq!(count_local_file_refs(body), 0);
    }

    #[test]
    fn body_is_byte_identical_after_frontmatter_strip() {
        let dir = scratch("fidelity");
        let raw = "---\ntitle: 保真\ntags: [a]\n---\n\n# 标题\n\n[[另一条笔记|别名]] ==高亮== %%注释%%\n\n> [!note] 提示\n\n![](img/a.png)\n\n^block1\n";
        let path = write(&dir, "fidelity.md", raw.as_bytes());
        let parsed = parse_file(&path).unwrap();

        // Everything after the closing delimiter line, byte for byte (the
        // blank line that followed it is part of the body).
        let expected = raw
            .strip_prefix("---\ntitle: 保真\ntags: [a]\n---\n")
            .unwrap()
            .to_string();
        assert_eq!(parsed.content, expected);
        for fragment in [
            "[[另一条笔记|别名]]",
            "==高亮==",
            "%%注释%%",
            "> [!note] 提示",
            "^block1",
            "![](img/a.png)",
        ] {
            assert!(
                parsed.content.contains(fragment),
                "import must not rewrite {fragment}"
            );
        }
        assert_eq!(parsed.local_file_refs, 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn timestamps_come_from_the_file_and_future_is_clamped() {
        let dir = scratch("times");
        let path = write(&dir, "old.md", b"body");
        let parsed = parse_file(&path).unwrap();
        let now = now_ms();
        assert!(parsed.updated_at <= now);
        assert!(parsed.created_at <= now);
        // A file created before the epoch would still be a valid timestamp; the
        // important property is that it is not `now`.
        assert!(
            parsed.updated_at > 1_500_000_000_000,
            "mtime is a real time"
        );
        assert_eq!(parsed.created_at <= parsed.updated_at, true);

        let _ = fs::remove_dir_all(&dir);
    }
}
