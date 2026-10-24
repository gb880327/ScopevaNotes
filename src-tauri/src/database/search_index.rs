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

    insert_row(conn, note_id, &title, &content, &tags_text)
}

/// CJK runs are indexed character by character. `unicode61` otherwise keeps a
/// whole Chinese/Japanese run as one token, so searching `管理` inside
/// `笔记管理的要点` would never match.
fn insert_row(
    conn: &Connection,
    note_id: &str,
    title: &str,
    content: &str,
    tags_text: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO notes_fts(note_id, title, content, tags_text) VALUES (?1, ?2, ?3, ?4)",
        params![
            note_id,
            explode_cjk(title),
            explode_cjk(content),
            explode_cjk(tags_text)
        ],
    )?;
    Ok(())
}

/// Rebuilds the whole projection. Needed once after the CJK-aware index format
/// landed (migration v3), and cheap enough for a library rebuild.
pub fn reindex_all(conn: &mut Connection) -> AppResult<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM notes_fts", [])?;
    let mut stmt = tx.prepare("SELECT id FROM notes")?;
    let ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    drop(stmt);
    for id in ids {
        // `reindex_note` deletes by `note_id` first, and that column is
        // UNINDEXED: a rebuild through it becomes an O(n²) scan (34 s at 10k
        // notes). The table was emptied above, so plain inserts suffice.
        let row = tx
            .query_row(
                "SELECT n.title, n.content,
                        COALESCE((SELECT group_concat(t.name, ' ')
                                  FROM note_tags nt JOIN tags t ON t.id = nt.tag_id
                                  WHERE nt.note_id = n.id), '')
                 FROM notes n WHERE n.id = ?1 AND n.deleted_at IS NULL",
                [&id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
            )
            .optional()?;
        if let Some((title, content, tags_text)) = row {
            insert_row(&tx, &id, &title, &content, &tags_text)?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF      // kana
        | 0x3400..=0x4DBF    // CJK extension A
        | 0x4E00..=0x9FFF    // CJK unified
        | 0xF900..=0xFAFF    // compatibility ideographs
        | 0xFF66..=0xFF9F    // half-width kana
        | 0x20000..=0x2FA1F  // extensions B–F
    )
}

/// Separates every CJK character with a space so `unicode61` emits one token
/// per character; everything else is left untouched.
pub fn explode_cjk(text: &str) -> String {
    if !text.chars().any(is_cjk) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + text.len() / 2);
    for c in text.chars() {
        if is_cjk(c) {
            out.push(' ');
            out.push(c);
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

/// Builds the coarse `MATCH` expression for a free-form search string.
///
/// ASCII-ish runs become prefix queries (`"sql"*` finds `sqlite`); CJK runs
/// become adjacent-character phrases, matching the exploded index. Operators,
/// quotes and `%` can never reach the parser. Returns `None` when the input has
/// no searchable character at all (e.g. `++`), and the caller then falls back
/// to a plain scan.
///
/// This is only a *candidate* filter: `note_repo` still refines every hit with
/// literal `LIKE`, because FTS tokenisation alone would let `100%` match `1000`.
pub fn to_match_query(input: &str) -> Option<String> {
    let mut terms: Vec<String> = Vec::new();
    // Split on every non-alphanumeric run: `unicode61` does the same, and it
    // keeps operators/quotes from ever reaching the FTS parser.
    for run in input.split(|c: char| !c.is_alphanumeric()) {
        if run.is_empty() {
            continue;
        }
        if run.chars().any(is_cjk) {
            // Adjacent-character phrase; the index stores one token per CJK char.
            let phrase = run
                .chars()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(" ");
            terms.push(format!("({phrase})"));
        } else {
            // ASCII/other: prefix match so `sql` still finds `sqlite`.
            terms.push(format!("\"{run}\"*"));
        }
    }
    if terms.is_empty() {
        return None;
    }
    Some(terms.join(" AND "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_terms_are_prefix_queries() {
        assert_eq!(
            to_match_query("rust sqlite").as_deref(),
            Some("\"rust\"* AND \"sqlite\"*")
        );
        // Punctuation cannot reach the parser; `100%` becomes the prefix `100`.
        assert_eq!(to_match_query("100%").as_deref(), Some("\"100\"*"));
        assert_eq!(to_match_query("C++").as_deref(), Some("\"C\"*"));
        assert_eq!(
            to_match_query("a_b").as_deref(),
            Some("\"a\"* AND \"b\"*")
        );
    }

    #[test]
    fn cjk_terms_become_adjacent_character_phrases() {
        // Two characters, the spec's "中文短词" case.
        assert_eq!(to_match_query("笔记").as_deref(), Some("(\"笔\" \"记\")"));
        assert_eq!(
            to_match_query("索引 sqlite").as_deref(),
            Some("(\"索\" \"引\") AND \"sqlite\"*")
        );
    }

    #[test]
    fn match_query_is_none_without_searchable_characters() {
        assert_eq!(to_match_query("+++"), None);
        assert_eq!(to_match_query("%_"), None);
        assert_eq!(to_match_query("   "), None);
    }

    #[test]
    fn explode_cjk_only_touches_cjk() {
        assert_eq!(explode_cjk("plain ascii"), "plain ascii");
        assert_eq!(explode_cjk("笔记sqlite"), " 笔  记 sqlite");
    }
}
