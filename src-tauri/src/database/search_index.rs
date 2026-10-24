use rusqlite::{params, Connection, OptionalExtension};

use crate::models::AppResult;

/// Rebuild the FTS projection for exactly one note inside the caller's write
/// transaction. Soft-deleted notes are removed from the index.
pub fn reindex_note(conn: &Connection, note_id: &str) -> AppResult<()> {
    let row = conn
        .query_row(
            "SELECT title, content, deleted_at FROM notes WHERE id = ?1",
            [note_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .optional()?;

    conn.execute("DELETE FROM notes_fts WHERE note_id = ?1", [note_id])?;

    let Some((title, content, deleted_at)) = row else {
        return Ok(());
    };
    if deleted_at.is_some() {
        return Ok(());
    }

    let tags_text: String = conn.query_row(
        "SELECT COALESCE(group_concat(t.name, ' '), '')
         FROM note_tags nt JOIN tags t ON t.id = nt.tag_id
         WHERE nt.note_id = ?1",
        [note_id],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT INTO notes_fts(note_id, title, content, tags_text) VALUES (?1, ?2, ?3, ?4)",
        params![note_id, title, content, tags_text],
    )?;
    Ok(())
}

/// Escape free-form user input before handing it to `MATCH`.
/// See plan §4.2: user input must never be concatenated raw into FTS syntax.
#[allow(dead_code)] // wired up when FTS search lands (M4)
pub fn to_match_query(input: &str) -> String {
    input
        .split_whitespace()
        .map(|token| format!("\"{}\"", token.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ")
}
