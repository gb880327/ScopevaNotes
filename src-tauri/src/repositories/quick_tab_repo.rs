use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::database::now_ms;
use crate::models::{AppError, AppResult, QuickTab};

/// Hard cap, enforced here rather than in the UI (design D8). The dot bar is
/// sized for this many; anything beyond it would need scrolling or an overflow
/// menu, both of which were dropped on purpose.
pub const MAX_TABS: i64 = 12;

/// Fixed colour keys. Stored verbatim; the frontend maps them to hex values, so
/// the palette can change without a migration. Order doubles as the default
/// rotation for new tabs.
pub const COLORS: &[&str] = &["yellow", "orange", "red", "purple", "blue", "cyan", "green"];

const SELECT: &str =
    "SELECT id, title, color, content, sort_order, created_at, updated_at FROM quick_tabs";

fn row_to_tab(row: &rusqlite::Row<'_>) -> rusqlite::Result<QuickTab> {
    Ok(QuickTab {
        id: row.get(0)?,
        title: row.get(1)?,
        color: row.get(2)?,
        content: row.get(3)?,
        sort_order: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn normalize_color(raw: &str) -> AppResult<String> {
    let key = raw.trim().to_ascii_lowercase();
    if COLORS.contains(&key.as_str()) {
        Ok(key)
    } else {
        Err(AppError::Invalid(format!("unknown quick tab color {raw}")))
    }
}

/// Ordered by the explicit sort column; `created_at` breaks ties so the dot bar
/// order is stable even if two rows share a sort value.
pub fn list(conn: &Connection) -> AppResult<Vec<QuickTab>> {
    let mut stmt = conn.prepare(&format!(
        "{SELECT} ORDER BY sort_order ASC, created_at ASC, id ASC"
    ))?;
    let rows = stmt.query_map([], row_to_tab)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn count(conn: &Connection) -> AppResult<i64> {
    let total = conn.query_row("SELECT COUNT(*) FROM quick_tabs", [], |row| row.get(0))?;
    Ok(total)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<QuickTab> {
    conn.query_row(&format!("{SELECT} WHERE id = ?1"), [id], row_to_tab)
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("quick tab {id}")))
}

/// Insert one row. Split out so `create` and `duplicate` share the same
/// cap-checked path; caller decides inside which transaction it runs.
fn insert(
    conn: &Connection,
    title: &str,
    color: Option<&str>,
    content: &str,
) -> AppResult<QuickTab> {
    let existing = count(conn)?;
    if existing >= MAX_TABS {
        return Err(AppError::Invalid(format!("速记最多 {MAX_TABS} 个")));
    }
    let color = match color {
        Some(raw) => normalize_color(raw)?,
        // Rotate the palette by current count so new tabs do not all look alike.
        None => COLORS[(existing as usize) % COLORS.len()].to_string(),
    };
    let now = now_ms();
    let tab = QuickTab {
        id: Uuid::now_v7().to_string(),
        title: title.to_string(),
        color,
        content: content.to_string(),
        sort_order: existing,
        created_at: now,
        updated_at: now,
    };
    conn.execute(
        "INSERT INTO quick_tabs(id, title, color, content, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            tab.id,
            tab.title,
            tab.color,
            tab.content,
            tab.sort_order,
            tab.created_at,
            tab.updated_at
        ],
    )?;
    Ok(tab)
}

/// Create a tab. The cap check and the insert share one transaction so two
/// concurrent creates cannot both slip past the limit.
pub fn create(conn: &mut Connection, title: &str, color: Option<&str>) -> AppResult<QuickTab> {
    let tx = conn.transaction()?;
    let tab = insert(&tx, title, color, "")?;
    tx.commit()?;
    Ok(tab)
}

/// Partial update: every field is optional, `None` leaves the column alone.
/// Returns the row as stored so the caller can pick up the new `updated_at`.
pub fn update(
    conn: &mut Connection,
    id: &str,
    title: Option<&str>,
    color: Option<&str>,
    content: Option<&str>,
) -> AppResult<QuickTab> {
    let color = color.map(normalize_color).transpose()?;
    let tx = conn.transaction()?;
    let changed = tx.execute(
        "UPDATE quick_tabs SET
           title = COALESCE(?2, title),
           color = COALESCE(?3, color),
           content = COALESCE(?4, content),
           updated_at = ?5
         WHERE id = ?1",
        params![id, title, color, content, now_ms()],
    )?;
    if changed == 0 {
        // Dropping `tx` rolls back; nothing was written anyway.
        return Err(AppError::NotFound(format!("quick tab {id}")));
    }
    let tab = tx.query_row(&format!("{SELECT} WHERE id = ?1"), [id], row_to_tab)?;
    tx.commit()?;
    Ok(tab)
}

/// Permanent by design: no `deleted_at`, no trash, no undo (spec: 速记与笔记体系隔离).
pub fn delete(conn: &mut Connection, id: &str) -> AppResult<()> {
    let removed = conn.execute("DELETE FROM quick_tabs WHERE id = ?1", [id])?;
    if removed == 0 {
        return Err(AppError::NotFound(format!("quick tab {id}")));
    }
    Ok(())
}

/// Copy a tab: new id, identical content, title suffixed so the two are
/// distinguishable in the dot bar. Never shares a row, so editing one copy
/// cannot touch the other (spec scenario 复制得到独立副本).
pub fn duplicate(conn: &mut Connection, id: &str) -> AppResult<QuickTab> {
    let tx = conn.transaction()?;
    let source = tx
        .query_row(&format!("{SELECT} WHERE id = ?1"), [id], row_to_tab)
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("quick tab {id}")))?;
    let title = if source.title.trim().is_empty() {
        String::new()
    } else {
        format!("{} 副本", source.title)
    };
    let tab = insert(&tx, &title, Some(&source.color), &source.content)?;
    tx.commit()?;
    Ok(tab)
}
