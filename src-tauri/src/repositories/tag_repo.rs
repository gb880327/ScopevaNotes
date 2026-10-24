use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::database::{now_ms, search_index};
use crate::models::{AppError, AppResult, Tag, TagWithCount};

/// Case-insensitive trim + collapse of inner whitespace (plan §4.1).
pub fn normalize_name(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn list_with_counts(conn: &Connection) -> AppResult<Vec<TagWithCount>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.color, COUNT(n.id)
         FROM tags t
         LEFT JOIN note_tags nt ON nt.tag_id = t.id
         LEFT JOIN notes n ON n.id = nt.note_id AND n.deleted_at IS NULL
         GROUP BY t.id
         ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TagWithCount {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            note_count: row.get(3)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn create(conn: &Connection, name: &str, color: Option<String>) -> AppResult<Tag> {
    let name = normalize_name(name);
    if name.is_empty() {
        return Err(AppError::Invalid("tag name must not be empty".into()));
    }
    let id = Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO tags(id, name, color, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, name, color, now_ms()],
    )
    .map_err(|err| match err {
        rusqlite::Error::SqliteFailure(e, _) if e.code == rusqlite::ErrorCode::ConstraintViolation => {
            AppError::Conflict(format!("tag already exists: {name}"))
        }
        other => AppError::Database(other),
    })?;
    Ok(Tag {
        id,
        name,
        color,
    })
}

pub fn rename(conn: &mut Connection, id: &str, name: &str) -> AppResult<Tag> {
    let name = normalize_name(name);
    if name.is_empty() {
        return Err(AppError::Invalid("tag name must not be empty".into()));
    }
    let tx = conn.transaction()?;
    let changed = tx.execute(
        "UPDATE tags SET name = ?2 WHERE id = ?1",
        params![id, name],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("tag {id}")));
    }

    // Tag names are part of the FTS projection.
    let mut stmt = tx.prepare("SELECT note_id FROM note_tags WHERE tag_id = ?1")?;
    let note_ids: Vec<String> = stmt
        .query_map([id], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    drop(stmt);
    for note_id in note_ids {
        search_index::reindex_note(&tx, &note_id)?;
    }

    let color = tx
        .query_row("SELECT color FROM tags WHERE id = ?1", [id], |row| row.get(0))
        .optional()?
        .flatten();
    tx.commit()?;
    Ok(Tag {
        id: id.to_string(),
        name,
        color,
    })
}

pub fn delete(conn: &mut Connection, id: &str) -> AppResult<()> {
    let tx = conn.transaction()?;
    let mut stmt = tx.prepare("SELECT note_id FROM note_tags WHERE tag_id = ?1")?;
    let note_ids: Vec<String> = stmt
        .query_map([id], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    let changed = tx.execute("DELETE FROM tags WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("tag {id}")));
    }
    for note_id in note_ids {
        search_index::reindex_note(&tx, &note_id)?;
    }
    tx.commit()?;
    Ok(())
}
