use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use std::collections::HashMap;
use uuid::Uuid;

use crate::database::{now_ms, search_index};
use crate::models::{AppError, AppResult, NoteDetail, NoteSummary, NoteSummaryPage, Tag};

const DEFAULT_PAGE: i64 = 50;
const MAX_PAGE: i64 = 200;

#[derive(Debug, Clone)]
pub struct Cursor {
    pub updated_at: i64,
    pub id: String,
}

impl Cursor {
    pub fn decode(raw: &str) -> AppResult<Self> {
        let (ts, id) = raw
            .split_once(':')
            .ok_or_else(|| AppError::Invalid("malformed cursor".into()))?;
        Ok(Self {
            updated_at: ts
                .parse()
                .map_err(|_| AppError::Invalid("malformed cursor".into()))?,
            id: id.to_string(),
        })
    }

    pub fn encode(&self) -> String {
        format!("{}:{}", self.updated_at, self.id)
    }
}

#[derive(Debug, Default)]
pub struct ListQuery {
    pub search: Option<String>,
    pub tag_ids: Vec<String>,
    pub favorites_only: bool,
    pub limit: Option<i64>,
    pub cursor: Option<Cursor>,
}

pub fn list(conn: &Connection, query: &ListQuery) -> AppResult<NoteSummaryPage> {
    let limit = query.limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);
    let mut sql = String::from(
        "SELECT n.id, n.title, substr(n.content, 1, 240), n.updated_at, n.is_favorite
         FROM notes n
         WHERE n.deleted_at IS NULL",
    );
    let mut args: Vec<Value> = Vec::new();

    if let Some(term) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let pattern = format!("%{}%", escape_like(term));
        sql.push_str(" AND (n.title LIKE ? ESCAPE '\\' OR n.content LIKE ? ESCAPE '\\')");
        args.push(Value::Text(pattern.clone()));
        args.push(Value::Text(pattern));
    }
    if query.favorites_only {
        sql.push_str(" AND n.is_favorite = 1");
    }
    if !query.tag_ids.is_empty() {
        sql.push_str(&format!(
            " AND n.id IN (SELECT nt.note_id FROM note_tags nt WHERE nt.tag_id IN ({}))",
            placeholders(query.tag_ids.len())
        ));
        args.extend(query.tag_ids.iter().cloned().map(Value::Text));
    }
    if let Some(cursor) = &query.cursor {
        sql.push_str(" AND (n.updated_at < ? OR (n.updated_at = ? AND n.id < ?))");
        args.push(Value::Integer(cursor.updated_at));
        args.push(Value::Integer(cursor.updated_at));
        args.push(Value::Text(cursor.id.clone()));
    }
    sql.push_str(" ORDER BY n.updated_at DESC, n.id DESC LIMIT ?");
    args.push(Value::Integer(limit + 1));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(args.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;

    let mut raw = Vec::new();
    for row in rows {
        raw.push(row?);
    }
    let has_more = raw.len() as i64 > limit;
    if has_more {
        raw.truncate(limit as usize);
    }

    let ids: Vec<String> = raw.iter().map(|(id, ..)| id.clone()).collect();
    let mut tags = tags_for_notes(conn, &ids)?;

    let items: Vec<NoteSummary> = raw
        .into_iter()
        .map(|(id, title, content, updated_at, is_favorite)| NoteSummary {
            tags: tags.remove(&id).unwrap_or_default(),
            excerpt: excerpt(&content),
            id,
            title,
            updated_at,
            is_favorite: is_favorite != 0,
        })
        .collect();

    let next_cursor = if has_more {
        items.last().map(|item| {
            Cursor {
                updated_at: item.updated_at,
                id: item.id.clone(),
            }
            .encode()
        })
    } else {
        None
    };

    Ok(NoteSummaryPage { items, next_cursor })
}

pub fn get(conn: &Connection, id: &str) -> AppResult<NoteDetail> {
    let mut note = conn
        .query_row(
            "SELECT id, title, content, is_favorite, deleted_at, created_at, updated_at
             FROM notes WHERE id = ?1",
            [id],
            |row| {
                Ok(NoteDetail {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    is_favorite: row.get::<_, i64>(3)? != 0,
                    deleted_at: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    tags: Vec::new(),
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("note {id}")))?;

    note.tags = tags_for_note(conn, id)?;
    Ok(note)
}

pub fn create(
    conn: &mut Connection,
    title: &str,
    content: &str,
    tag_ids: &[String],
) -> AppResult<NoteDetail> {
    let id = Uuid::now_v7().to_string();
    let now = now_ms();
    let tx = conn.transaction()?;

    tx.execute(
        "INSERT INTO notes(id, title, content, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, title, content, now],
    )?;
    replace_tags(&tx, &id, tag_ids)?;
    search_index::reindex_note(&tx, &id)?;
    tx.commit()?;

    get(conn, &id)
}

pub fn update(
    conn: &mut Connection,
    id: &str,
    title: &str,
    content: &str,
    tag_ids: &[String],
    expected_updated_at: i64,
) -> AppResult<NoteDetail> {
    let tx = conn.transaction()?;
    let current: Option<i64> = tx
        .query_row("SELECT updated_at FROM notes WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .optional()?;
    let Some(current) = current else {
        return Err(AppError::NotFound(format!("note {id}")));
    };
    if current != expected_updated_at {
        return Err(AppError::Conflict(
            "note changed in another window; reload before saving".into(),
        ));
    }

    tx.execute(
        "UPDATE notes SET title = ?2, content = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, title, content, now_ms()],
    )?;
    replace_tags(&tx, id, tag_ids)?;
    search_index::reindex_note(&tx, id)?;
    tx.commit()?;

    get(conn, id)
}

pub fn set_favorite(conn: &Connection, id: &str, value: bool) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE notes SET is_favorite = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, i64::from(value), now_ms()],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("note {id}")));
    }
    Ok(())
}

pub fn soft_delete(conn: &mut Connection, id: &str) -> AppResult<()> {
    let tx = conn.transaction()?;
    let changed = tx.execute(
        "UPDATE notes SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
        params![id, now_ms()],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("note {id}")));
    }
    search_index::reindex_note(&tx, id)?;
    tx.commit()?;
    Ok(())
}

pub fn restore(conn: &mut Connection, id: &str) -> AppResult<()> {
    let tx = conn.transaction()?;
    let changed = tx.execute(
        "UPDATE notes SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1",
        params![id, now_ms()],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("note {id}")));
    }
    search_index::reindex_note(&tx, id)?;
    tx.commit()?;
    Ok(())
}

pub fn replace_tags(conn: &Connection, note_id: &str, tag_ids: &[String]) -> AppResult<()> {
    conn.execute("DELETE FROM note_tags WHERE note_id = ?1", [note_id])?;
    for tag_id in tag_ids {
        conn.execute(
            "INSERT OR IGNORE INTO note_tags(note_id, tag_id) VALUES (?1, ?2)",
            params![note_id, tag_id],
        )?;
    }
    Ok(())
}

pub fn tags_for_note(conn: &Connection, note_id: &str) -> AppResult<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.color
         FROM note_tags nt JOIN tags t ON t.id = nt.tag_id
         WHERE nt.note_id = ?1 ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([note_id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn tags_for_notes(conn: &Connection, ids: &[String]) -> AppResult<HashMap<String, Vec<String>>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "SELECT nt.note_id, t.name
         FROM note_tags nt JOIN tags t ON t.id = nt.tag_id
         WHERE nt.note_id IN ({}) ORDER BY t.name COLLATE NOCASE",
        placeholders(ids.len())
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(ids.iter()), |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let (note_id, name) = row?;
        grouped.entry(note_id).or_default().push(name);
    }
    Ok(grouped)
}

fn placeholders(count: usize) -> String {
    std::iter::repeat("?").take(count).collect::<Vec<_>>().join(",")
}

fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn excerpt(content: &str) -> String {
    let collapsed = content.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.chars().take(160).collect()
}
