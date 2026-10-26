use serde::{Deserialize, Serialize};

/// A 速记 (Quick Tab): plain text, no Markdown, no tags, no search index.
///
/// Deliberately *not* a `Note` variant — see design D1. The struct carries the
/// stored color *key* (e.g. `"yellow"`), never a CSS value, so the palette can
/// change without touching existing rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickTab {
    pub id: String,
    pub title: String,
    pub color: String,
    pub content: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
