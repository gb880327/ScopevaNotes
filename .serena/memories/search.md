# Search subsystem

Pipeline (`note_repo::list` + `database/search_index.rs`): FTS5 `MATCH` coarse filter -> literal `LIKE` refine.

## Invariants that break easily
- `notes_fts.note_id` is **UNINDEXED**. Any correlated subquery on it (`EXISTS (... f.note_id = n.id)`) scans the
  whole FTS table per outer row: 10k notes -> 54 s for one query. Use `n.id IN (SELECT note_id FROM notes_fts WHERE notes_fts MATCH ?)`.
  Same trap in rebuilds: deleting each row with `DELETE ... WHERE note_id = ?` makes a rebuild O(n^2) (34 s vs 1.07 s), so
  `reindex_all` empties the table once and only inserts.
- Tags are matched via `note_tags`/`tags` joins, never through `notes_fts.tags_text` in a correlated EXISTS.
- `unicode61` treats a whole CJK run as **one token**: `"笔记"` does not match `笔记管理的要点`, and prefix `"笔记"*`
  only matches at the run start. Indexing therefore stores CJK text with a space between every character
  (`explode_cjk`); queries turn CJK runs into adjacent-character phrases `("管" "理")`.
- `to_match_query` may only emit quoted phrases and the prefix operator; operators, quotes and `%` must never reach
  the FTS parser. ASCII runs become `"sql"*` (word-prefix), so searching `sql` hits `sqlite`.
- The literal `LIKE` refine is mandatory: the prefix query makes `100%` candidate-match `1000`, and the spec requires
  `%` to be literal. Refine checks every whitespace-separated user token against title, content and tag names.
- Accepted semantic limit: matching is word-prefix (Latin) / contiguous characters (CJK), **not** arbitrary infix;
  `lite` will not find `sqlite`. Written into the spec as a contract - do not "fix" it with bm25 reordering or
  trigram FTS without revisiting the spec.
- Ordering stays `updated_at DESC` with a keyset cursor (stable pagination); no relevance re-ranking.

## Measurements (10k notes, 14.6 MB library, release build)
- FTS + refine: p50 16.5 ms, p95 18.5 ms, worst 19.7 ms.
- Pure `LIKE` scan: 1.5 ms on frequent terms, 92-117 ms on rare/zero-hit terms -> exceeds the 100 ms p95 budget.
- Rebuild of the whole projection: ~1.1 s.

## Projection maintenance
- `search_index::reindex_note` runs inside the caller's write transaction for create/update/soft-delete/restore/tag changes;
  a failed index write must roll back the note change.
- Index format changes require a new migration **and** an entry in `REINDEX_VERSIONS` (`migrations.rs`).
- Benchmark: `mem:suggested_commands` (the `search_bench` ignored test asserts the p95 budget).
